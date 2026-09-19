//! Application state: what is on screen, what is selected, and what the keys do.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::git::{self, Repo, Salvage, Staleness};
use crate::scan;
use crate::text::clip;

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
    /// Remove exactly these, resolved before the dialog opened.
    ///
    /// Resolved rather than indexed on purpose: removing one worktree shifts
    /// every index after it, so a list of indices would be wrong by the second
    /// removal.
    Remove {
        removals: Vec<Removal>,
        branch: bool,
    },
    Prune {
        repo: usize,
    },
}

/// One worktree, resolved to everything its removal needs.
pub struct Removal {
    pub root: PathBuf,
    pub path: PathBuf,
    pub label: String,
    /// The branch to delete with `D`, when there is one. Carried rather than
    /// recovered from the label: `label()` renders a detached head as `(sha)`,
    /// and git permits a branch actually named `(wip)`.
    pub branch: Option<String>,
    pub force: bool,
}

/// A failure worth reading in full.
///
/// The footer is one line and truncates, and git's useful sentence is usually
/// the last of several. A removal that fails is exactly the moment to show all
/// of it, and the command, so it can be repeated by hand.
pub struct Failure {
    pub title: String,
    /// The commands that failed and what git said about each, already in the
    /// order they ran. One string rather than a command and an output, because
    /// a sweep fails one worktree at a time and each failure has its own
    /// command worth repeating.
    pub body: String,
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
    pub failure: Option<Failure>,
    pub failure_scroll: u16,
    pub help: bool,
    /// "Fold all" should hold for repos the scan has not reached yet.
    pub fold_new: bool,
    pub scanning: bool,
    pub scan: ScanProgress,
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
            failure: None,
            failure_scroll: 0,
            help: false,
            fold_new: false,
            scanning: true,
            scan: ScanProgress::default(),
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
            Lens::Safe => wt.is_safe_to_remove(),
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

    /// Mark everything in the current repository that is safe to remove.
    ///
    /// The lens and the filter are respected rather than bypassed: whatever is
    /// being looked at is what gets swept, so the key cannot reach past what is
    /// on screen. Folding is not, because a folded repository is still a
    /// repository — its rows are hidden, not excluded.
    pub fn sweep_repo(&mut self) {
        let Some(row) = self.current() else {
            self.say("Nothing to sweep", Tone::Warn);
            return;
        };
        let repo = match row {
            Row::Repo { repo } | Row::Worktree { repo, .. } => repo,
        };
        let name = self.repos[repo].name.clone();
        let safe: Vec<PathBuf> = self.repos[repo]
            .worktrees
            .iter()
            .filter(|wt| wt.is_safe_to_remove() && self.matches(&self.repos[repo], wt))
            .map(|wt| wt.path.clone())
            .collect();

        if safe.is_empty() {
            self.say(format!("Nothing in {name} is safe to remove"), Tone::Info);
            return;
        }

        // Pressing it twice is a way to change your mind, not a way to mark
        // everything twice.
        if safe.iter().all(|p| self.marked.contains(p)) {
            for path in &safe {
                self.marked.remove(path);
            }
            self.say(format!("Unmarked {} in {name}", safe.len()), Tone::Info);
            return;
        }

        let n = safe.len();
        self.marked.extend(safe);
        self.say(
            format!("Marked {n} safe to remove in {name} — d removes them"),
            Tone::Good,
        );
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
            if wt.salvage() == Salvage::Unknown {
                stakes.unknown += 1;
                continue;
            }
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

    /// Report a failure in full, rather than as much of it as one line holds.
    pub fn fail(&mut self, title: impl Into<String>, body: String) {
        self.failure_scroll = 0;
        self.failure = Some(Failure {
            title: title.into(),
            body,
        });
    }

    /// `d` acts on the marking when there is one, and on the cursor otherwise.
    pub fn ask_remove(&mut self, with_branch: bool) {
        if self.marked.is_empty() {
            self.ask_remove_one(with_branch);
        } else {
            self.ask_remove_marked(with_branch);
        }
    }

    fn ask_remove_one(&mut self, with_branch: bool) {
        let Some(Row::Worktree { repo, wt }) = self.current() else {
            self.say("Select a worktree to remove", Tone::Warn);
            return;
        };
        let w = &self.repos[repo].worktrees[wt];
        if let Err(reason) = removable(w) {
            self.say(reason, Tone::Warn);
            return;
        }

        let mut body = vec![
            (scan::shorten_home(&w.path), Tone::Info),
            (format!("branch {}", w.label()), Tone::Info),
            cost_line(w),
        ];
        if with_branch && w.branch.is_some() {
            body.push((
                format!("Branch {} will be deleted too (git branch -D).", w.label()),
                Tone::Bad,
            ));
        }

        let removals = vec![self.resolve(repo, wt)];
        self.confirm = Some(Confirm {
            title: if with_branch {
                "Remove worktree and branch?".into()
            } else {
                "Remove worktree?".into()
            },
            body,
            action: Action::Remove {
                removals,
                branch: with_branch,
            },
        });
    }

    /// The dialog for a whole marking: what goes, worst first, and what does not.
    fn ask_remove_marked(&mut self, with_branch: bool) {
        const LISTED: usize = 8;

        let mut going: Vec<(usize, usize)> = Vec::new();
        let mut refused: Vec<(String, String)> = Vec::new();
        for (ri, wi) in self.marked_worktrees() {
            let w = &self.repos[ri].worktrees[wi];
            match removable(w) {
                Ok(()) => going.push((ri, wi)),
                Err(reason) => refused.push((w.label(), reason)),
            }
        }

        // Worst first: whoever is about to answer this reads the top of the list.
        going.sort_by(|&(ar, aw), &(br, bw)| {
            let a = &self.repos[ar].worktrees[aw];
            let b = &self.repos[br].worktrees[bw];
            b.salvage()
                .cmp(&a.salvage())
                .then_with(|| b.changed_files().cmp(&a.changed_files()))
                .then_with(|| b.unpushed().cmp(&a.unpushed()))
                .then_with(|| a.label().cmp(&b.label()))
        });

        if going.is_empty() {
            let why = match refused.len() {
                0 => "Nothing is marked".to_string(),
                1 => format!(
                    "The only marked worktree cannot be removed: {}",
                    refused[0].1
                ),
                n => format!("None of the {n} marked worktrees can be removed"),
            };
            self.say(why, Tone::Warn);
            return;
        }

        let mut body = Vec::new();
        for &(ri, wi) in going.iter().take(LISTED) {
            let w = &self.repos[ri].worktrees[wi];
            let tone = match w.salvage() {
                Salvage::Nothing => Tone::Good,
                Salvage::Unknown | Salvage::Commits => Tone::Warn,
                Salvage::Uncommitted => Tone::Bad,
            };
            body.push((
                format!("{:<34}  {}", clip(&w.label(), 34), w.verdict()),
                tone,
            ));
        }
        if going.len() > LISTED {
            body.push((format!("… and {} more", going.len() - LISTED), Tone::Info));
        }

        let forced: Vec<&(usize, usize)> = going
            .iter()
            .filter(|&&(ri, wi)| self.repos[ri].worktrees[wi].is_dirty())
            .collect();
        let files: u32 = going
            .iter()
            .map(|&(ri, wi)| self.repos[ri].worktrees[wi].changed_files())
            .sum();
        let commits: u32 = going
            .iter()
            .map(|&(ri, wi)| self.repos[ri].worktrees[wi].unpushed())
            .sum();

        body.push((String::new(), Tone::Info));
        body.push((
            format!(
                "{} worktree{} across {} repositor{}",
                going.len(),
                if going.len() == 1 { "" } else { "s" },
                repo_count(&going),
                if repo_count(&going) == 1 { "y" } else { "ies" }
            ),
            Tone::Info,
        ));
        if !forced.is_empty() {
            body.push((
                format!(
                    "{} hold{} uncommitted work — {} file{} destroyed, and git will only remove them with --force",
                    forced.len(),
                    if forced.len() == 1 { "s" } else { "" },
                    files,
                    if files == 1 { "" } else { "s" }
                ),
                Tone::Bad,
            ));
        }
        if commits > 0 {
            body.push((
                format!(
                    "{} commit{} exist{} nowhere else{}",
                    commits,
                    if commits == 1 { "" } else { "s" },
                    if commits == 1 { "s" } else { "" },
                    if with_branch {
                        "; deleting the branches loses them"
                    } else {
                        "; the branches survive"
                    }
                ),
                if with_branch { Tone::Bad } else { Tone::Warn },
            ));
        }
        if files == 0 && commits == 0 {
            body.push(("Nothing would be lost from any of them.".into(), Tone::Good));
        }
        for (label, reason) in refused.iter().take(4) {
            body.push((format!("{label} is kept: {reason}"), Tone::Warn));
        }
        if refused.len() > 4 {
            body.push((format!("… and {} more kept", refused.len() - 4), Tone::Warn));
        }

        let removals = going
            .into_iter()
            .map(|(ri, wi)| self.resolve(ri, wi))
            .collect();
        self.confirm = Some(Confirm {
            title: if with_branch {
                "Remove marked worktrees and their branches?".into()
            } else {
                "Remove marked worktrees?".into()
            },
            body,
            action: Action::Remove {
                removals,
                branch: with_branch,
            },
        });
    }

    fn resolve(&self, repo: usize, wt: usize) -> Removal {
        let w = &self.repos[repo].worktrees[wt];
        Removal {
            root: self.repos[repo].root.clone(),
            path: w.path.clone(),
            label: w.label(),
            branch: w.branch.clone(),
            force: w.is_dirty(),
        }
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
            Action::Remove { removals, branch } => {
                let mut removed = 0usize;
                let mut branches = 0usize;
                let mut failures: Vec<String> = Vec::new();
                let mut branch_failures: Vec<String> = Vec::new();
                let mut roots: Vec<PathBuf> = Vec::new();

                for r in &removals {
                    roots.push(r.root.clone());
                    let path_arg = r.path.to_string_lossy().into_owned();
                    let mut args = vec!["worktree", "remove"];
                    if r.force {
                        args.push("--force");
                    }
                    args.push(&path_arg);
                    // One failure must not strand the rest: a sweep that stops
                    // half way leaves the marking and the disk disagreeing.
                    match git::git_run(&r.root, &args) {
                        Ok(_) => {
                            removed += 1;
                            self.marked.remove(&r.path);
                            if let (true, Some(name)) = (branch, r.branch.as_ref()) {
                                match git::git_run(&r.root, &["branch", "-D", name]) {
                                    Ok(_) => branches += 1,
                                    Err(e) => branch_failures.push(format!(
                                        "git -C {} branch -D {name}\n{e}",
                                        scan::shorten_home(&r.root)
                                    )),
                                }
                            }
                        }
                        Err(e) => failures.push(format!(
                            "{}\n{e}",
                            remove_command(&r.root, &r.path, r.force)
                        )),
                    }
                }

                // By root, not by index: refresh_repo re-sorts, so an index
                // taken before the first refresh names a different repository
                // by the second.
                roots.sort_unstable();
                roots.dedup();
                for root in roots {
                    if let Some(idx) = self.repos.iter().position(|r| r.root == root) {
                        self.refresh_repo(idx);
                    }
                }
                self.prune_marks();

                let mut msg = match removed {
                    0 => "Removed nothing".to_string(),
                    1 => "Removed 1 worktree".to_string(),
                    n => format!("Removed {n} worktrees"),
                };
                if branches > 0 {
                    let plural = if branches == 1 { "" } else { "es" };
                    msg.push_str(&format!(" and {branches} branch{plural}"));
                }
                // Kept apart on purpose: a branch that survived is not a
                // worktree that survived, and reporting them as one number
                // reads as though something was left on disk.
                if !branch_failures.is_empty() {
                    msg.push_str(&format!(
                        " — {} branch{} kept: {}",
                        branch_failures.len(),
                        if branch_failures.len() == 1 { "" } else { "es" },
                        branch_failures.join("; ")
                    ));
                }
                // Branch failures belong in the box too: appending them to a
                // one-line footer is the truncation this exists to remove, and
                // the box is drawn over that footer anyway.
                let mut trouble = failures.clone();
                trouble.extend(branch_failures.iter().cloned());
                if trouble.is_empty() {
                    self.say(msg, Tone::Good);
                } else {
                    let title = match (failures.len(), branch_failures.len()) {
                        (0, n) => format!("{n} branch{} could not be deleted", plural_es(n)),
                        (n, 0) => format!("{n} of {} could not be removed", removals.len()),
                        (n, b) => format!(
                            "{n} could not be removed, and {b} branch{} kept",
                            plural_es(b)
                        ),
                    };
                    self.fail(title, trouble.join("\n\n"));
                    self.say(msg, Tone::Bad);
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
                    Err(e) => self.fail(
                        "Prune failed",
                        format!(
                            "git -C {} worktree prune -v\n{e}",
                            scan::shorten_home(&root)
                        ),
                    ),
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
            Err(e) => self.fail(
                format!("Could not {verb} the worktree"),
                format!(
                    "git -C {} worktree {verb} {}\n{e}",
                    scan::shorten_home(&root),
                    scan::shorten_home(std::path::Path::new(&path))
                ),
            ),
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
                if wt.is_safe_to_remove() {
                    t.safe += 1;
                }
            }
        }
        t
    }
}

/// What the scan has got through, as the header reports it.
#[derive(Default, Clone, Copy)]
pub struct ScanProgress {
    pub dirs: usize,
    pub found: usize,
    pub probed: usize,
    pub walking: bool,
}

impl ScanProgress {
    /// One line saying what is happening, rather than a number that stops
    /// moving while the slow part runs.
    pub fn describe(&self) -> String {
        if self.walking && self.found == 0 {
            format!("walking {} directories", self.dirs)
        } else if self.probed < self.found {
            format!("reading {} of {} repositories", self.probed, self.found)
        } else if self.walking {
            format!(
                "{} repositories · walking {} directories",
                self.found, self.dirs
            )
        } else {
            format!("{} repositories", self.found)
        }
    }
}

/// What removing the current marking would cost.
#[derive(Default, PartialEq, Eq, Debug)]
pub struct Stakes {
    pub worktrees: usize,
    pub files: u32,
    pub commits: u32,
    /// Marked worktrees git could not read. Their counters are zero because
    /// nothing was read, so they cannot be added to the two above — and a total
    /// that ignores them would report "nothing to salvage" over work nobody has
    /// looked at.
    pub unknown: usize,
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

/// The command a removal ran, as it could be typed again.
fn remove_command(root: &Path, path: &Path, force: bool) -> String {
    format!(
        "git -C {} worktree remove {}{}",
        scan::shorten_home(root),
        if force { "--force " } else { "" },
        scan::shorten_home(path)
    )
}

fn plural_es(n: usize) -> &'static str {
    if n == 1 { "" } else { "es" }
}

/// Why this worktree cannot be removed, if it cannot.
fn removable(w: &git::Worktree) -> Result<(), String> {
    match w.unremovable() {
        Some(reason) => Err(reason.why()),
        None => Ok(()),
    }
}

/// The one line that says what this removal costs.
fn cost_line(w: &git::Worktree) -> (String, Tone) {
    match w.salvage() {
        Salvage::Nothing => ("Clean. Nothing would be lost.".into(), Tone::Good),
        Salvage::Unknown => (
            "Git cannot read it, so there is no telling what is in it.".into(),
            Tone::Warn,
        ),
        Salvage::Commits => (
            format!(
                "{} commit{} live only here. The branch survives; the checkout does not.",
                w.unpushed(),
                if w.unpushed() == 1 { "" } else { "s" }
            ),
            Tone::Warn,
        ),
        Salvage::Uncommitted => (
            format!(
                "{} uncommitted file{} will be destroyed.",
                w.changed_files(),
                if w.changed_files() == 1 { "" } else { "s" }
            ),
            Tone::Bad,
        ),
    }
}

fn repo_count(going: &[(usize, usize)]) -> usize {
    let mut repos: Vec<usize> = going.iter().map(|&(r, _)| r).collect();
    repos.sort_unstable();
    repos.dedup();
    repos.len()
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
    fn the_scan_says_what_it_is_doing_at_each_stage() {
        let walking_only = ScanProgress {
            dirs: 1204,
            found: 0,
            probed: 0,
            walking: true,
        };
        assert_eq!(walking_only.describe(), "walking 1204 directories");

        // The stage that used to look hung: the directory count has stopped
        // moving and every remaining second is spent reading repositories.
        let probing = ScanProgress {
            dirs: 1204,
            found: 92,
            probed: 47,
            walking: false,
        };
        assert_eq!(probing.describe(), "reading 47 of 92 repositories");

        let done = ScanProgress {
            dirs: 1204,
            found: 92,
            probed: 92,
            walking: false,
        };
        assert_eq!(done.describe(), "92 repositories");
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
    fn a_sweep_marks_only_what_is_safe_to_remove() {
        let mut repo = test_repo("alpha", 4);
        repo.worktrees[1].untracked = 2; // dirty
        repo.worktrees[2].locked = Some("in use".into());
        repo.worktrees[3].broken = true;
        // worktrees[4] is clean, and so is the main checkout.
        let mut app = app_with(vec![repo]);

        app.go(0);
        app.sweep_repo();

        let marked: Vec<String> = app.repos[0]
            .worktrees
            .iter()
            .filter(|w| app.marked.contains(&w.path))
            .map(|w| w.label())
            .collect();
        assert_eq!(
            marked,
            ["feat/branch-3"],
            "the dirty, locked, unreadable and main rows must all be left alone"
        );
    }

    #[test]
    fn a_sweep_is_its_own_undo() {
        let mut app = app_with(vec![test_repo("alpha", 2)]);
        app.go(0);
        app.sweep_repo();
        assert_eq!(app.marked.len(), 2);
        app.sweep_repo();
        assert!(
            app.marked.is_empty(),
            "pressing it again clears its own marking"
        );
    }

    #[test]
    fn a_sweep_reaches_no_further_than_the_lens() {
        let mut repo = test_repo("alpha", 2);
        repo.worktrees[1].untracked = 1;
        let mut app = app_with(vec![repo]);
        app.lens = Lens::Dirty;
        app.rebuild(None);
        app.go(0);
        app.sweep_repo();
        assert!(
            app.marked.is_empty(),
            "nothing the dirty lens shows is safe to remove, so nothing is marked"
        );
    }

    #[test]
    fn a_sweep_touches_only_the_repository_under_the_cursor() {
        let mut app = app_with(vec![test_repo("alpha", 2), test_repo("beta", 2)]);
        let beta = app
            .rows
            .iter()
            .position(|r| matches!(r, Row::Repo { repo } if app.repos[*repo].name == "beta"))
            .expect("beta");
        app.go(beta);
        app.sweep_repo();
        assert!(
            app.marked
                .iter()
                .all(|p| p.to_string_lossy().contains("beta")),
            "a sweep is per repository: {:?}",
            app.marked
        );
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

    /// The counters of an unreadable worktree are zero because nothing was read.
    /// Summing them and calling the total "nothing to salvage" is the same
    /// mistake the safe lens used to make, one screen further on.
    #[test]
    fn an_unreadable_worktree_is_never_totalled_as_nothing() {
        let mut repo = test_repo("alpha", 1);
        repo.worktrees[1].broken = true;
        let mut app = app_with(vec![repo]);
        app.go(2);
        app.toggle_mark();

        let stakes = app.marked_stakes();
        assert_eq!(stakes.worktrees, 1);
        assert_eq!(stakes.unknown, 1, "it must be counted as unreadable");
        assert_eq!(stakes.files, 0);
        assert_eq!(stakes.commits, 0);

        // And the dialog must not claim otherwise either — nor offer to remove
        // something git cannot read.
        app.ask_remove(false);
        assert!(
            app.confirm.is_none(),
            "there is nothing git can remove, so nothing should be asked"
        );
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
