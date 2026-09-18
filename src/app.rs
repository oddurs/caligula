//! Application state: what is on screen, what is selected, and what the keys do.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::git::{self, Repo, Salvage, Staleness};
use crate::scan;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Activity,
    Name,
    Risk,
}

impl Sort {
    pub fn next(self) -> Self {
        match self {
            Sort::Activity => Sort::Name,
            Sort::Name => Sort::Risk,
            Sort::Risk => Sort::Activity,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Sort::Activity => "activity",
            Sort::Name => "name",
            Sort::Risk => "risk",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lens {
    All,
    Dirty,
    Salvage,
    Stale,
    Safe,
}

impl Lens {
    pub fn next(self) -> Self {
        match self {
            Lens::All => Lens::Dirty,
            Lens::Dirty => Lens::Salvage,
            Lens::Salvage => Lens::Stale,
            Lens::Stale => Lens::Safe,
            Lens::Safe => Lens::All,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Lens::All => "all",
            Lens::Dirty => "dirty",
            Lens::Salvage => "salvageable",
            Lens::Stale => "stale",
            Lens::Safe => "safe to remove",
        }
    }
}

#[derive(Clone, Copy)]
pub enum Row {
    Repo { repo: usize },
    Worktree { repo: usize, wt: usize },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Info,
    Good,
    Warn,
    Bad,
}

pub enum Action {
    Remove {
        repo: usize,
        wt: usize,
        force: bool,
        branch: bool,
    },
    Prune {
        repo: usize,
    },
}

pub struct Confirm {
    pub title: String,
    pub body: Vec<(String, Tone)>,
    pub action: Action,
}

pub struct App {
    pub repos: Vec<Repo>,
    pub rows: Vec<Row>,
    pub selected: usize,
    pub offset: usize,
    pub collapsed: HashSet<PathBuf>,
    /// Worktrees the next action applies to, held by path: the row indices move
    /// under sorting, filtering and a re-probe, and the marking must not.
    pub marked: HashSet<PathBuf>,
    pub sort: Sort,
    pub lens: Lens,
    pub filter: String,
    pub filtering: bool,
    pub detail_scroll: u16,
    pub status: Option<(String, Tone, Instant)>,
    pub confirm: Option<Confirm>,
    pub help: bool,
    /// "Fold all" should hold for repos the scan has not reached yet.
    pub fold_new: bool,
    pub scanning: bool,
    pub dirs_seen: usize,
    pub now: u64,
    pub quit: bool,
    /// Set when a key asks for a shell; main drops the TUI and runs it.
    pub shell_request: Option<PathBuf>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        App {
            repos: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            offset: 0,
            collapsed: HashSet::new(),
            marked: HashSet::new(),
            sort: Sort::Activity,
            lens: Lens::All,
            filter: String::new(),
            filtering: false,
            detail_scroll: 0,
            status: None,
            confirm: None,
            help: false,
            fold_new: false,
            scanning: true,
            dirs_seen: 0,
            now: git::now(),
            quit: false,
            shell_request: None,
        }
    }

    // ------------------------------------------------------------- ingestion

    pub fn add_repo(&mut self, repo: Repo) {
        let key = self.selection_key();
        // A repo with a single, clean main checkout is just a repo — the tool is
        // about worktrees, but hiding it entirely would misreport the machine,
        // so keep it and let the lens filter it out.
        if self.fold_new {
            self.collapsed.insert(repo.common_dir.clone());
        }
        self.repos.push(repo);
        self.sort_repos();
        self.rebuild(key);
    }

    /// Re-apply the sort after the user changes it.
    pub fn resort(&mut self) {
        self.sort_repos();
    }

    fn sort_repos(&mut self) {
        let sort = self.sort;
        let now = self.now;
        for repo in &mut self.repos {
            repo.worktrees.sort_by(|a, b| {
                b.is_main
                    .cmp(&a.is_main)
                    .then_with(|| match sort {
                        Sort::Name => a.label().to_lowercase().cmp(&b.label().to_lowercase()),
                        Sort::Activity => b.last_touched.cmp(&a.last_touched),
                        Sort::Risk => b
                            .salvage()
                            .cmp(&a.salvage())
                            .then_with(|| b.changed_files().cmp(&a.changed_files()))
                            .then_with(|| a.age_secs(now).cmp(&b.age_secs(now))),
                    })
                    .then_with(|| a.path.cmp(&b.path))
            });
        }
        self.repos.sort_by(|a, b| match sort {
            Sort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            Sort::Activity => b.last_touched().cmp(&a.last_touched()),
            Sort::Risk => b
                .salvage_count()
                .cmp(&a.salvage_count())
                .then_with(|| b.dirty_count().cmp(&a.dirty_count()))
                .then_with(|| b.linked_count().cmp(&a.linked_count())),
        });
    }

    // ------------------------------------------------------------- row model

    fn matches(&self, repo: &Repo, wt: &git::Worktree) -> bool {
        let lens_ok = match self.lens {
            Lens::All => true,
            Lens::Dirty => wt.is_dirty(),
            Lens::Salvage => wt.salvage() != Salvage::Nothing,
            Lens::Stale => matches!(
                wt.staleness(self.now),
                Staleness::Stale | Staleness::Ancient
            ),
            Lens::Safe => wt.salvage() == Salvage::Nothing && !wt.is_main,
        };
        if !lens_ok {
            return false;
        }
        if self.filter.is_empty() {
            return true;
        }
        let needle = self.filter.to_lowercase();
        repo.name.to_lowercase().contains(&needle)
            || wt.label().to_lowercase().contains(&needle)
            || wt.path.to_string_lossy().to_lowercase().contains(&needle)
    }

    pub fn rebuild(&mut self, keep: Option<PathBuf>) {
        let mut rows = Vec::new();
        for (ri, repo) in self.repos.iter().enumerate() {
            let visible: Vec<usize> = repo
                .worktrees
                .iter()
                .enumerate()
                .filter(|(_, wt)| self.matches(repo, wt))
                .map(|(i, _)| i)
                .collect();
            if visible.is_empty() {
                continue;
            }
            rows.push(Row::Repo { repo: ri });
            if !self.collapsed.contains(&repo.common_dir) {
                rows.extend(visible.into_iter().map(|wt| Row::Worktree { repo: ri, wt }));
            }
        }
        self.rows = rows;

        if let Some(key) = keep
            && let Some(idx) = self.rows.iter().position(|r| self.row_key(*r) == key)
        {
            self.selected = idx;
        }
        if self.selected >= self.rows.len() {
            self.selected = self.rows.len().saturating_sub(1);
        }
    }

    /// Rebuild after the filter text changed: the top hit is what you want.
    pub fn refilter(&mut self) {
        self.selected = 0;
        self.offset = 0;
        self.detail_scroll = 0;
        self.rebuild(None);
    }

    pub fn row_key(&self, row: Row) -> PathBuf {
        match row {
            Row::Repo { repo } => self.repos[repo].common_dir.clone(),
            Row::Worktree { repo, wt } => self.repos[repo].worktrees[wt].path.clone(),
        }
    }

    pub fn selection_key(&self) -> Option<PathBuf> {
        self.rows.get(self.selected).map(|r| self.row_key(*r))
    }

    pub fn current(&self) -> Option<Row> {
        self.rows.get(self.selected).copied()
    }

    pub fn current_repo(&self) -> Option<&Repo> {
        match self.current()? {
            Row::Repo { repo } | Row::Worktree { repo, .. } => Some(&self.repos[repo]),
        }
    }

    // -------------------------------------------------------------- movement

    pub fn move_by(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let last = self.rows.len() as isize - 1;
        let next = (self.selected as isize + delta).clamp(0, last);
        self.selected = next as usize;
        self.detail_scroll = 0;
    }

    pub fn go(&mut self, idx: usize) {
        self.selected = idx.min(self.rows.len().saturating_sub(1));
        self.detail_scroll = 0;
    }

    /// Jump to the next row belonging to a different repo.
    pub fn next_repo(&mut self, forward: bool) {
        let Some(cur) = self.current() else { return };
        let cur_repo = match cur {
            Row::Repo { repo } | Row::Worktree { repo, .. } => repo,
        };
        let positions: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r, Row::Repo { .. }))
            .map(|(i, _)| i)
            .collect();
        let target = if forward {
            positions.iter().find(|&&i| match self.rows[i] {
                Row::Repo { repo } => repo > cur_repo,
                _ => false,
            })
        } else {
            positions.iter().rev().find(|&&i| match self.rows[i] {
                Row::Repo { repo } => repo < cur_repo || (repo == cur_repo && i < self.selected),
                _ => false,
            })
        };
        if let Some(&i) = target {
            self.go(i);
        }
    }

    /// Mark or unmark the row under the cursor, then step down.
    ///
    /// Stepping down is what makes marking a run of worktrees one key held
    /// rather than an alternation of two.
    pub fn toggle_mark(&mut self) {
        // A repo header is stepped over rather than refused: holding space down
        // a list has to carry from one repository into the next.
        let Some(Row::Worktree { repo, wt }) = self.current() else {
            self.move_by(1);
            return;
        };
        let path = self.repos[repo].worktrees[wt].path.clone();
        if !self.marked.remove(&path) {
            self.marked.insert(path);
        }
        self.move_by(1);
    }

    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

    /// Drop marks for worktrees that are no longer there: a re-probe after a
    /// removal must not leave the marking pointing at something gone.
    pub fn prune_marks(&mut self) {
        let live: HashSet<PathBuf> = self
            .repos
            .iter()
            .flat_map(|r| r.worktrees.iter().map(|w| w.path.clone()))
            .collect();
        self.marked.retain(|p| live.contains(p));
    }

    pub fn is_marked(&self, wt: &git::Worktree) -> bool {
        self.marked.contains(&wt.path)
    }

    /// Every marked worktree, in the order they appear on screen.
    pub fn marked_worktrees(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (ri, repo) in self.repos.iter().enumerate() {
            for (wi, wt) in repo.worktrees.iter().enumerate() {
                if self.marked.contains(&wt.path) {
                    out.push((ri, wi));
                }
            }
        }
        out
    }

    /// What the marking adds up to, for the footer: the reason to hesitate.
    pub fn marked_stakes(&self) -> Stakes {
        let mut stakes = Stakes::default();
        for (ri, wi) in self.marked_worktrees() {
            let wt = &self.repos[ri].worktrees[wi];
            stakes.worktrees += 1;
            stakes.files += wt.changed_files();
            stakes.commits += wt.unpushed();
        }
        stakes
    }

    pub fn toggle_collapse(&mut self) {
        let Some(row) = self.current() else { return };
        let repo = match row {
            Row::Repo { repo } | Row::Worktree { repo, .. } => repo,
        };
        let key = self.repos[repo].common_dir.clone();
        if !self.collapsed.remove(&key) {
            self.collapsed.insert(key.clone());
        }
        // Collapsing from inside a group parks the cursor on its header.
        let keep = if matches!(row, Row::Worktree { .. }) && self.collapsed.contains(&key) {
            Some(key)
        } else {
            self.selection_key()
        };
        self.rebuild(keep);
    }

    pub fn collapse_all(&mut self, collapse: bool) {
        let keep = self.current_repo().map(|r| r.common_dir.clone());
        self.fold_new = collapse;
        self.collapsed.clear();
        if collapse {
            for repo in &self.repos {
                self.collapsed.insert(repo.common_dir.clone());
            }
        }
        self.rebuild(keep);
    }

    // --------------------------------------------------------------- actions

    pub fn say(&mut self, msg: impl Into<String>, tone: Tone) {
        self.status = Some((msg.into(), tone, Instant::now()));
    }

    pub fn ask_remove(&mut self, with_branch: bool) {
        let Some(Row::Worktree { repo, wt }) = self.current() else {
            self.say("Select a worktree to remove", Tone::Warn);
            return;
        };
        let r = &self.repos[repo];
        let w = &r.worktrees[wt];
        if w.is_main {
            self.say(
                "That is the main checkout — git will not remove it",
                Tone::Warn,
            );
            return;
        }
        if let Some(reason) = &w.locked {
            self.say(
                format!("Locked: {reason} — press L to unlock first"),
                Tone::Warn,
            );
            return;
        }

        let mut body = vec![
            (scan::shorten_home(&w.path), Tone::Info),
            (format!("branch {}", w.label()), Tone::Info),
        ];
        match w.salvage() {
            Salvage::Nothing => body.push(("Clean. Nothing would be lost.".into(), Tone::Good)),
            Salvage::Commits => body.push((
                format!(
                    "{} commit{} live only here. The branch survives; the checkout does not.",
                    w.unpushed(),
                    if w.unpushed() == 1 { "" } else { "s" }
                ),
                Tone::Warn,
            )),
            Salvage::Uncommitted => body.push((
                format!(
                    "{} uncommitted file(s) will be destroyed.",
                    w.changed_files()
                ),
                Tone::Bad,
            )),
        }
        if with_branch && w.branch.is_some() {
            body.push((
                format!("Branch {} will be deleted too (git branch -D).", w.label()),
                Tone::Bad,
            ));
        }

        // Until the marking itself can be removed, a dialog that appears while
        // one is held has to say which it means, or it reads as the marking.
        if self.marked.len() > 1 || (self.marked.len() == 1 && !self.marked.contains(&w.path)) {
            body.push((
                format!(
                    "{} worktrees are marked. This removes only the one above.",
                    self.marked.len()
                ),
                Tone::Warn,
            ));
        }

        let force = w.is_dirty();
        self.confirm = Some(Confirm {
            title: if with_branch {
                "Remove worktree and branch?".into()
            } else {
                "Remove worktree?".into()
            },
            body,
            action: Action::Remove {
                repo,
                wt,
                force,
                branch: with_branch,
            },
        });
    }

    pub fn ask_prune(&mut self) {
        let Some(row) = self.current() else { return };
        let repo = match row {
            Row::Repo { repo } | Row::Worktree { repo, .. } => repo,
        };
        let r = &self.repos[repo];
        let prunable: Vec<String> = r
            .worktrees
            .iter()
            .filter(|w| w.prunable.is_some() || w.broken)
            .map(|w| scan::shorten_home(&w.path))
            .collect();
        let mut body = vec![(format!("repo {}", r.name), Tone::Info)];
        if prunable.is_empty() {
            body.push(("No stale administrative entries found.".into(), Tone::Info));
            body.push(("Running prune is harmless.".into(), Tone::Good));
        } else {
            for p in &prunable {
                body.push((p.clone(), Tone::Warn));
            }
        }
        self.confirm = Some(Confirm {
            title: "Prune worktree records?".into(),
            body,
            action: Action::Prune { repo },
        });
    }

    pub fn run_confirmed(&mut self) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        match confirm.action {
            Action::Remove {
                repo,
                wt,
                force,
                branch,
            } => {
                let root = self.repos[repo].root.clone();
                let path = self.repos[repo].worktrees[wt].path.clone();
                let label = self.repos[repo].worktrees[wt].label();
                let path_arg = path.to_string_lossy().into_owned();
                let mut args = vec!["worktree", "remove"];
                if force {
                    args.push("--force");
                }
                args.push(&path_arg);
                match git::git_run(&root, &args) {
                    Ok(_) => {
                        let mut msg = format!("Removed {}", scan::shorten_home(&path));
                        if branch {
                            match git::git_run(&root, &["branch", "-D", &label]) {
                                Ok(_) => msg.push_str(&format!(" and branch {label}")),
                                Err(e) => msg.push_str(&format!(" — branch kept: {e}")),
                            }
                        }
                        self.say(msg, Tone::Good);
                        self.refresh_repo(repo);
                    }
                    Err(e) => self.say(format!("git worktree remove failed: {e}"), Tone::Bad),
                }
            }
            Action::Prune { repo } => {
                let root = self.repos[repo].root.clone();
                match git::git_run(&root, &["worktree", "prune", "-v"]) {
                    Ok(out) => {
                        let n = out.lines().filter(|l| !l.trim().is_empty()).count();
                        self.say(
                            if n == 0 {
                                "Nothing to prune".to_string()
                            } else {
                                format!("Pruned {n} record(s)")
                            },
                            Tone::Good,
                        );
                        self.refresh_repo(repo);
                    }
                    Err(e) => self.say(format!("git worktree prune failed: {e}"), Tone::Bad),
                }
            }
        }
    }

    pub fn toggle_lock(&mut self) {
        let Some(Row::Worktree { repo, wt }) = self.current() else {
            self.say("Select a worktree to lock", Tone::Warn);
            return;
        };
        let root = self.repos[repo].root.clone();
        let w = &self.repos[repo].worktrees[wt];
        if w.is_main {
            self.say("The main checkout cannot be locked", Tone::Warn);
            return;
        }
        let path = w.path.to_string_lossy().into_owned();
        let locked = w.locked.is_some();
        let verb = if locked { "unlock" } else { "lock" };
        match git::git_run(&root, &["worktree", verb, &path]) {
            Ok(_) => {
                self.say(if locked { "Unlocked" } else { "Locked" }, Tone::Good);
                self.refresh_repo(repo);
            }
            Err(e) => self.say(format!("git worktree {verb} failed: {e}"), Tone::Bad),
        }
    }

    pub fn refresh_repo(&mut self, idx: usize) {
        let key = self.selection_key();
        let probe_from = self.repos[idx].root.clone();
        match git::probe_repo(&probe_from) {
            Some(fresh) => self.repos[idx] = fresh,
            None => {
                self.repos.remove(idx);
            }
        }
        self.now = git::now();
        self.prune_marks();
        self.sort_repos();
        self.rebuild(key);
    }

    pub fn refresh_current(&mut self) {
        let Some(row) = self.current() else { return };
        let repo = match row {
            Row::Repo { repo } | Row::Worktree { repo, .. } => repo,
        };
        let name = self.repos[repo].name.clone();
        self.refresh_repo(repo);
        self.say(format!("Refreshed {name}"), Tone::Info);
    }

    pub fn copy_path(&mut self) {
        let Some(row) = self.current() else { return };
        let path = self.row_path(row);
        match clipboard(&path) {
            Ok(()) => self.say(format!("Copied {}", scan::shorten_home(&path)), Tone::Good),
            Err(e) => self.say(format!("Clipboard unavailable: {e}"), Tone::Warn),
        }
    }

    pub fn reveal(&mut self) {
        let Some(row) = self.current() else { return };
        let path = self.row_path(row);
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        match Command::new(opener).arg(&path).status() {
            Ok(s) if s.success() => self.say("Opened", Tone::Good),
            Ok(s) => self.say(format!("{opener} exited with {s}"), Tone::Warn),
            Err(e) => self.say(format!("{opener}: {e}"), Tone::Warn),
        }
    }

    pub fn request_shell(&mut self) {
        let Some(row) = self.current() else { return };
        self.shell_request = Some(self.row_path(row));
    }

    fn row_path(&self, row: Row) -> PathBuf {
        match row {
            Row::Repo { repo } => self.repos[repo].root.clone(),
            Row::Worktree { repo, wt } => self.repos[repo].worktrees[wt].path.clone(),
        }
    }

    // --------------------------------------------------------------- totals

    pub fn totals(&self) -> Totals {
        let mut t = Totals {
            repos: self.repos.len(),
            ..Default::default()
        };
        for repo in &self.repos {
            for wt in &repo.worktrees {
                t.worktrees += 1;
                if !wt.is_main {
                    t.linked += 1;
                }
                if wt.is_dirty() {
                    t.dirty += 1;
                }
                if wt.salvage() != Salvage::Nothing {
                    t.salvage += 1;
                }
                if matches!(
                    wt.staleness(self.now),
                    Staleness::Stale | Staleness::Ancient
                ) {
                    t.stale += 1;
                }
                if wt.salvage() == Salvage::Nothing && !wt.is_main {
                    t.safe += 1;
                }
            }
        }
        t
    }
}

/// What removing the current marking would cost.
#[derive(Default, PartialEq, Eq, Debug)]
pub struct Stakes {
    pub worktrees: usize,
    pub files: u32,
    pub commits: u32,
}

#[derive(Default)]
pub struct Totals {
    pub repos: usize,
    pub worktrees: usize,
    pub linked: usize,
    pub dirty: usize,
    pub salvage: usize,
    pub stale: usize,
    pub safe: usize,
}

fn clipboard(path: &Path) -> Result<(), String> {
    use std::io::Write;
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };
    let mut last = String::from("no clipboard tool found");
    for (bin, args) in candidates {
        let child = Command::new(bin)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .spawn();
        match child {
            Ok(mut c) => {
                if let Some(mut stdin) = c.stdin.take() {
                    let _ = stdin.write_all(path.to_string_lossy().as_bytes());
                }
                return c.wait().map(|_| ()).map_err(|e| e.to_string());
            }
            Err(e) => last = format!("{bin}: {e}"),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::test_repo;

    fn app_with(repos: Vec<Repo>) -> App {
        let mut app = App::new();
        for r in repos {
            app.add_repo(r);
        }
        app
    }

    #[test]
    fn worktrees_are_grouped_under_their_repo() {
        let app = app_with(vec![test_repo("alpha", 2), test_repo("beta", 1)]);
        assert_eq!(app.rows.len(), 2 + 3 + 2);
        assert!(matches!(app.rows[0], Row::Repo { .. }));
        assert!(matches!(app.rows[1], Row::Worktree { .. }));
    }

    #[test]
    fn folding_everything_also_folds_what_the_scan_finds_next() {
        let mut app = app_with(vec![test_repo("alpha", 2)]);
        app.collapse_all(true);
        app.add_repo(test_repo("beta", 2));
        assert_eq!(
            app.rows.len(),
            2,
            "a repo found after fold-all should arrive folded"
        );
    }

    #[test]
    fn folding_a_repo_hides_its_worktrees_but_keeps_the_header() {
        let mut app = app_with(vec![test_repo("alpha", 2)]);
        app.toggle_collapse();
        assert_eq!(app.rows.len(), 1);
        assert!(matches!(app.rows[0], Row::Repo { .. }));
        app.toggle_collapse();
        assert_eq!(app.rows.len(), 4);
    }

    #[test]
    fn the_safe_lens_never_offers_the_main_checkout() {
        let mut app = app_with(vec![test_repo("alpha", 2)]);
        app.lens = Lens::Safe;
        app.rebuild(None);
        let labels: Vec<String> = app
            .rows
            .iter()
            .filter_map(|r| match r {
                Row::Worktree { repo, wt } => Some(app.repos[*repo].worktrees[*wt].label()),
                Row::Repo { .. } => None,
            })
            .collect();
        assert_eq!(labels, ["feat/branch-0", "feat/branch-1"]);
    }

    #[test]
    fn a_repo_disappears_when_nothing_in_it_matches() {
        let mut app = app_with(vec![test_repo("alpha", 1), test_repo("beta", 1)]);
        app.filter = "alpha".into();
        app.refilter();
        assert!(app.rows.iter().all(|r| match r {
            Row::Repo { repo } | Row::Worktree { repo, .. } => app.repos[*repo].name == "alpha",
        }));
    }

    #[test]
    fn dirty_worktrees_survive_the_dirty_lens() {
        let mut repo = test_repo("alpha", 2);
        repo.worktrees[2].untracked = 3;
        let mut app = app_with(vec![repo]);
        app.lens = Lens::Dirty;
        app.rebuild(None);
        assert_eq!(app.rows.len(), 2);
        let Row::Worktree { repo, wt } = app.rows[1] else {
            panic!("expected a worktree row")
        };
        assert_eq!(app.repos[repo].worktrees[wt].label(), "feat/branch-1");
    }

    #[test]
    fn marking_steps_down_so_a_run_can_be_marked_with_one_key() {
        let mut app = app_with(vec![test_repo("alpha", 3)]);
        app.go(1);
        app.toggle_mark();
        app.toggle_mark();
        assert_eq!(app.marked.len(), 2);
        assert_eq!(app.selected, 3, "the cursor should have stepped past both");
    }

    #[test]
    fn marking_steps_over_a_repo_header_rather_than_stopping_on_it() {
        let mut app = app_with(vec![test_repo("alpha", 1), test_repo("beta", 1)]);
        // Rows: 0 alpha, 1 main, 2 branch, 3 beta, 4 main, 5 branch.
        app.go(3);
        app.toggle_mark();
        assert!(app.marked.is_empty(), "a repo header is not markable");
        assert_eq!(app.selected, 4, "but holding space must carry past it");
    }

    #[test]
    fn marking_the_same_row_twice_unmarks_it() {
        let mut app = app_with(vec![test_repo("alpha", 3)]);
        app.go(1);
        app.toggle_mark();
        app.go(1);
        app.toggle_mark();
        assert!(app.marked.is_empty());
    }

    #[test]
    fn a_repo_row_cannot_be_marked() {
        let mut app = app_with(vec![test_repo("alpha", 2)]);
        app.go(0);
        app.toggle_mark();
        assert!(app.marked.is_empty());
    }

    #[test]
    fn the_marking_survives_sorting_filtering_and_folding() {
        let mut app = app_with(vec![test_repo("alpha", 3), test_repo("beta", 2)]);
        app.go(2);
        app.toggle_mark();
        let marked = app.marked.clone();

        app.sort = Sort::Name;
        app.resort();
        app.rebuild(None);
        assert_eq!(app.marked, marked, "sorting moved rows, not marks");

        app.filter = "beta".into();
        app.refilter();
        assert_eq!(
            app.marked, marked,
            "a filter hides rows, it does not unmark them"
        );

        app.filter.clear();
        app.refilter();
        app.collapse_all(true);
        assert_eq!(
            app.marked, marked,
            "folding hides rows, it does not unmark them"
        );
    }

    #[test]
    fn a_worktree_that_is_gone_is_dropped_from_the_marking() {
        let mut app = app_with(vec![test_repo("alpha", 2)]);
        app.go(2);
        app.toggle_mark();
        assert_eq!(app.marked.len(), 1);

        app.repos[0].worktrees.remove(1);
        app.prune_marks();
        assert!(app.marked.is_empty(), "the marking outlived the worktree");
    }

    #[test]
    fn the_stakes_add_up_what_the_marking_would_cost() {
        let mut repo = test_repo("alpha", 2);
        repo.worktrees[1].untracked = 3;
        repo.worktrees[2].upstream = Some("origin/main".into());
        repo.worktrees[2].ahead = 2;
        let mut app = app_with(vec![repo]);
        // Rows: 0 the repo, 1 main, 2 and 3 the two feature branches — which are
        // the ones carrying the changes and the commits.
        app.go(2);
        app.toggle_mark();
        app.toggle_mark();

        let stakes = app.marked_stakes();
        assert_eq!(stakes.worktrees, 2);
        assert_eq!(stakes.files, 3);
        assert_eq!(stakes.commits, 2);
    }

    #[test]
    fn selection_follows_the_row_it_was_on() {
        let mut app = app_with(vec![test_repo("alpha", 3)]);
        app.go(3);
        let before = app.selection_key();
        app.sort = Sort::Name;
        app.resort();
        app.rebuild(before.clone());
        assert_eq!(app.selection_key(), before);
    }
}
