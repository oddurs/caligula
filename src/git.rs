//! Everything that shells out to git, and the data it produces.
//!
//! We drive the `git` binary rather than linking libgit2: the porcelain formats
//! are stable, and it keeps the dependency list at one crate.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// A repository: one common `.git` directory and every worktree attached to it.
pub struct Repo {
    pub name: String,
    /// The shared git dir (`/path/to/repo/.git`), the identity of the repo.
    pub common_dir: PathBuf,
    pub root: PathBuf,
    pub remote: Option<String>,
    pub default_base: Option<String>,
    pub stashes: Vec<Stash>,
    pub worktrees: Vec<Worktree>,
}

impl Repo {
    pub fn dirty_count(&self) -> usize {
        self.worktrees.iter().filter(|w| w.is_dirty()).count()
    }

    pub fn salvage_count(&self) -> usize {
        self.worktrees
            .iter()
            .filter(|w| w.salvage() != Salvage::Nothing)
            .count()
    }

    /// Most recent activity across the repo, used for sorting.
    pub fn last_touched(&self) -> u64 {
        self.worktrees
            .iter()
            .map(|w| w.last_touched)
            .max()
            .unwrap_or(0)
    }

    /// What a folded repository still says about itself, column by column.
    ///
    /// Added up where adding up means something, and not where it does not.
    /// Uncommitted files are distinct files in distinct worktrees, and commits
    /// that exist only here are distinct commits on distinct branches, so both
    /// sum. Being behind does not: forty-five worktrees each 79 commits behind
    /// the same upstream are behind by 79, not by 3545. The worst one is the
    /// number that means anything.
    pub fn totals(&self) -> RepoTotals {
        let mut t = RepoTotals::default();
        for w in &self.worktrees {
            t.ahead += w.unpushed();
            t.behind = t.behind.max(w.behind);
            t.files += w.changed_files();
        }
        t
    }

    pub fn staleness(&self, now: u64) -> Staleness {
        let newest = self.last_touched();
        if newest == 0 {
            return Staleness::Ancient;
        }
        Staleness::of(now.saturating_sub(newest))
    }

    pub fn linked_count(&self) -> usize {
        self.worktrees.iter().filter(|w| !w.is_main).count()
    }
}

pub struct Worktree {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
    pub is_main: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,

    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,

    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub conflicts: u32,
    /// Distinct tracked paths that differ from HEAD.
    ///
    /// Not `staged + unstaged`: a file edited, staged, then edited again is one
    /// file but two changes, and counting it twice made the verdict disagree
    /// with the list of files printed under it.
    pub tracked: u32,
    pub files: Vec<FileChange>,

    /// Stashes made on this worktree's branch.
    ///
    /// Shared refs, not the worktree's own: `refs/stash` lives in the common
    /// dir, so removing the worktree leaves these behind. They are attributed
    /// here because standing in front of a worktree is when you need to know
    /// one of them is yours.
    pub stashes: Vec<Stash>,
    pub last_commit: Option<Commit>,
    /// The last few commits, for context when nothing is unmerged.
    pub recent: Vec<Commit>,
    /// Commits on this branch that are not on the repo's base branch.
    pub unmerged: Vec<Commit>,
    pub unmerged_total: u32,
    /// Newest of: last commit, worktree mtime, index mtime. Unix seconds.
    pub last_touched: u64,
    /// Set when git could not read the worktree at all (deleted on disk, etc).
    pub broken: bool,
}

/// One stash entry, and the branch it was made on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stash {
    /// `stash@{0}`.
    ///
    /// Positional, and it shifts on every push and drop — so it is shown next
    /// to the sha rather than on its own. Copying an index off a display that
    /// has gone stale applies a different stash than the one that was read.
    pub id: String,
    /// The stash commit. Unlike the index, this does not move.
    pub sha: String,
    /// The branch HEAD was on. `None` for a stash made on a detached head,
    /// which git records as `(no branch)`.
    pub branch: Option<String>,
    pub message: String,
    pub time: u64,
}

#[derive(Clone)]
pub struct Commit {
    pub sha: String,
    pub time: u64,
    pub author: String,
    pub subject: String,
}

#[derive(Clone)]
pub struct FileChange {
    /// Two-letter porcelain code, e.g. "M.", ".M", "??", "UU".
    pub code: String,
    pub path: String,
}

/// What you would lose by deleting a worktree.
///
/// Ordered by how much it should give you pause, because that is the order a
/// removal dialog lists things in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Salvage {
    /// Clean, and everything it contains lives somewhere else too.
    Nothing,
    /// Commits exist only here; removing the worktree keeps the branch, and the
    /// reflog keeps the commits.
    Commits,
    /// Git could not read it, so nothing can be claimed about it either way.
    ///
    /// Above `Commits` deliberately: an unpushed commit is recoverable, and an
    /// unreadable worktree may hold anything at all. It is its own answer
    /// rather than a default, because the counters of an unread worktree are
    /// all zero, and zero is indistinguishable from clean — saying "nothing to
    /// salvage" on the strength of a failed read is the one mistake this tool
    /// must never make.
    Unknown,
    /// Uncommitted work. Deleting destroys it.
    Uncommitted,
}

/// The column-wise sum of a repository's worktrees.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct RepoTotals {
    pub ahead: u32,
    pub behind: u32,
    pub files: u32,
}

/// Why a worktree cannot be removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unremovable {
    Main,
    Locked(String),
    Unreadable,
}

impl Unremovable {
    /// Front-loaded, for a column that will be clipped: the words that
    /// distinguish this row from its neighbours have to survive the cut.
    pub fn short(&self) -> String {
        match self {
            Unremovable::Main => "Main checkout".into(),
            Unremovable::Locked(reason) => format!("Locked: {reason}"),
            Unremovable::Unreadable => "Unreadable".into(),
        }
    }

    /// The whole sentence, for a dialog explaining what it is not doing.
    pub fn why(&self) -> String {
        match self {
            Unremovable::Main => "it is the main checkout, which git will not remove".into(),
            Unremovable::Locked(reason) => format!("locked — {reason}"),
            Unremovable::Unreadable => "git cannot read it — press p to prune the record".into(),
        }
    }
}

/// How long since anything happened here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    Active,
    Recent,
    Stale,
    Ancient,
}

impl Staleness {
    pub fn of(age: u64) -> Staleness {
        const DAY: u64 = 86_400;
        match age {
            a if a < 3 * DAY => Staleness::Active,
            a if a < 14 * DAY => Staleness::Recent,
            a if a < 60 * DAY => Staleness::Stale,
            _ => Staleness::Ancient,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Staleness::Active => "active",
            Staleness::Recent => "recent",
            Staleness::Stale => "stale",
            Staleness::Ancient => "ancient",
        }
    }
}

impl Worktree {
    pub fn is_dirty(&self) -> bool {
        self.staged + self.unstaged + self.untracked + self.conflicts > 0
    }

    pub fn changed_files(&self) -> u32 {
        self.tracked_changes() + self.untracked
    }

    /// Files git is already following. Losing one of these loses your edits;
    /// losing an untracked file loses the whole file. Both are unrecoverable,
    /// which is why neither is treated as the lesser — but they are different
    /// enough that a single number for both tells you nothing useful.
    pub fn tracked_changes(&self) -> u32 {
        self.tracked
    }

    /// Commits that exist nowhere else: ahead of upstream, or off the base branch.
    pub fn unpushed(&self) -> u32 {
        if self.upstream.is_some() {
            self.ahead
        } else {
            self.unmerged_total
        }
    }

    pub fn salvage(&self) -> Salvage {
        if self.broken {
            Salvage::Unknown
        } else if self.is_dirty() {
            Salvage::Uncommitted
        } else if self.unpushed() > 0 {
            Salvage::Commits
        } else {
            Salvage::Nothing
        }
    }

    /// Whether this can be offered as safe to remove.
    ///
    /// Three different reasons it cannot be, and the lens has to respect all of
    /// them: git will not remove a main checkout, it refuses a locked one, and
    /// nothing can be said about one it could not read.
    /// Why git will not remove this worktree, if it will not.
    ///
    /// One answer, used by the lens, the sweep, the removal guard and the
    /// sentence alike. They drifted apart twice while each decided for itself;
    /// adding a reason here now forces every one of them to account for it.
    pub fn unremovable(&self) -> Option<Unremovable> {
        if self.is_main {
            Some(Unremovable::Main)
        } else if let Some(reason) = &self.locked {
            Some(Unremovable::Locked(reason.clone()))
        } else if self.broken {
            Some(Unremovable::Unreadable)
        } else {
            None
        }
    }

    /// Whether this can be offered as safe to remove: git will do it, and there
    /// is nothing in it worth keeping.
    pub fn is_safe_to_remove(&self) -> bool {
        self.unremovable().is_none() && self.salvage() == Salvage::Nothing
    }

    pub fn age_secs(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_touched)
    }

    /// The age column. Nothing is known about a worktree git cannot read.
    pub fn age_label(&self, now: u64) -> String {
        if self.last_touched == 0 {
            "—".into()
        } else {
            ago(self.age_secs(now))
        }
    }

    pub fn staleness(&self, now: u64) -> Staleness {
        Staleness::of(self.age_secs(now))
    }

    pub fn label(&self) -> String {
        match (&self.branch, self.detached) {
            (Some(b), _) => b.clone(),
            (None, true) => format!("({})", short_sha(&self.head)),
            (None, false) => "(no branch)".into(),
        }
    }

    /// One line saying what is at stake, shown in the detail pane.
    pub fn verdict(&self) -> String {
        if self.broken {
            return "Git cannot read this worktree — it may be gone from disk".into();
        }
        let mut parts = Vec::new();
        let tracked = self.tracked_changes();
        if tracked > 0 {
            parts.push(format!("{tracked} modified file{}", plural(tracked)));
        }
        if self.untracked > 0 {
            parts.push(format!(
                "{} untracked file{}",
                self.untracked,
                plural(self.untracked)
            ));
        }
        let unpushed = self.unpushed();
        if unpushed > 0 {
            let where_ = if self.upstream.is_some() {
                "unpushed"
            } else {
                "off-base"
            };
            parts.push(format!("{unpushed} {where_} commit{}", plural(unpushed)));
        }
        if !parts.is_empty() {
            return parts.join(", ");
        }
        // Nothing to salvage is a statement about content; safe to remove is a
        // statement about whether git will do it. They are not the same, so the
        // reason it will not goes first — this column gets clipped, and the
        // shared half is not the half worth reading.
        match self.unremovable() {
            Some(reason) => format!("{} — nothing to salvage", reason.short()),
            None => "Nothing to salvage — safe to remove".into(),
        }
    }
}

fn plural(n: u32) -> &'static str {
    if n == 1 { "" } else { "s" }
}

pub fn short_sha(sha: &str) -> String {
    sha.chars().take(8).collect()
}

// ---------------------------------------------------------------- git plumbing

/// Variables that choose a repository, all of which outrank `-C`.
///
/// caligula names the repository it means and reads no other, so inheriting any
/// of these can only ever answer the wrong question. It is not hypothetical:
/// anything launched from a git hook has them set, and every worktree on the
/// machine would then be probed against that one repository.
const REPO_ENV: [&str; 6] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
    "GIT_NAMESPACE",
];

/// A git invocation against `dir`, and nothing else.
fn command(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(args);
    for key in REPO_ENV {
        cmd.env_remove(key);
    }
    cmd
}

/// Run git in `dir`, returning stdout on success.
pub fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = command(dir, args)
        // Never take the index lock: browsing must not disturb a running build.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Run git for effect, returning combined output so failures can be shown.
pub fn git_run(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = command(dir, args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(if stdout.is_empty() { stderr } else { stdout })
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

/// The shared git dir for whatever repo `dir` belongs to, or None.
pub fn common_dir(dir: &Path) -> Option<PathBuf> {
    let out = git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let p = PathBuf::from(out.trim());
    if p.as_os_str().is_empty() {
        None
    } else {
        Some(canonical(&p))
    }
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Build the full picture of a repo, given any path inside any of its worktrees.
pub fn probe_repo(member: &Path) -> Option<Repo> {
    let common = common_dir(member)?;
    let list = git(member, &["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&list);
    if entries.is_empty() {
        return None;
    }

    let root = entries[0].path.clone();
    let name = repo_name(&root, &common);
    let remote = git(&root, &["remote", "get-url", "origin"]).map(|s| s.trim().to_string());
    let base = default_base(&root);
    let stashes = git(
        &root,
        &["stash", "list", "--format=%gd%x1f%gs%x1f%ct%x1f%H"],
    )
    .map(|out| parse_stashes(&out))
    .unwrap_or_default();

    let mut worktrees: Vec<Worktree> = entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| probe_worktree(e, i == 0, base.as_deref()))
        .collect();
    for wt in &mut worktrees {
        if let Some(branch) = &wt.branch {
            wt.stashes = stashes
                .iter()
                .filter(|s| s.branch.as_deref() == Some(branch.as_str()))
                .cloned()
                .collect();
        }
    }
    // What is left belongs to the repository: a stash made on a branch that no
    // worktree holds any more is exactly the kind nobody finds again.
    let claimed: Vec<&str> = worktrees
        .iter()
        .filter_map(|w| w.branch.as_deref())
        .collect();
    let orphaned: Vec<Stash> = stashes
        .iter()
        .filter(|s| match &s.branch {
            Some(b) => !claimed.contains(&b.as_str()),
            None => true,
        })
        .cloned()
        .collect();

    Some(Repo {
        name,
        common_dir: common,
        root,
        remote,
        default_base: base,
        stashes: orphaned,
        worktrees,
    })
}

fn repo_name(root: &Path, common: &Path) -> String {
    root.file_name()
        .or_else(|| common.parent().and_then(|p| p.file_name()))
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

/// The branch we measure "off-base" commits against.
fn default_base(root: &Path) -> Option<String> {
    if let Some(head) = git(
        root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    ) {
        let head = head.trim();
        if !head.is_empty() {
            return Some(head.to_string());
        }
    }
    for cand in ["origin/main", "origin/master", "main", "master", "trunk"] {
        if git(root, &["rev-parse", "--verify", "--quiet", cand]).is_some() {
            return Some(cand.to_string());
        }
    }
    None
}

struct Entry {
    path: PathBuf,
    head: String,
    branch: Option<String>,
    detached: bool,
    bare: bool,
    locked: Option<String>,
    prunable: Option<String>,
}

fn parse_worktree_list(out: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut cur: Option<Entry> = None;
    for line in out.lines() {
        let (key, rest) = match line.split_once(' ') {
            Some((k, r)) => (k, r),
            None => (line, ""),
        };
        match key {
            "worktree" => {
                if let Some(e) = cur.take() {
                    entries.push(e);
                }
                cur = Some(Entry {
                    path: PathBuf::from(rest),
                    head: String::new(),
                    branch: None,
                    detached: false,
                    bare: false,
                    locked: None,
                    prunable: None,
                });
            }
            "HEAD" => {
                if let Some(e) = cur.as_mut() {
                    e.head = rest.to_string();
                }
            }
            "branch" => {
                if let Some(e) = cur.as_mut() {
                    e.branch = Some(rest.trim_start_matches("refs/heads/").to_string());
                }
            }
            "detached" => {
                if let Some(e) = cur.as_mut() {
                    e.detached = true;
                }
            }
            "bare" => {
                if let Some(e) = cur.as_mut() {
                    e.bare = true;
                }
            }
            "locked" => {
                if let Some(e) = cur.as_mut() {
                    e.locked = Some(if rest.is_empty() {
                        "locked".into()
                    } else {
                        rest.to_string()
                    });
                }
            }
            "prunable" => {
                if let Some(e) = cur.as_mut() {
                    e.prunable = Some(if rest.is_empty() {
                        "prunable".into()
                    } else {
                        rest.to_string()
                    });
                }
            }
            _ => {}
        }
    }
    if let Some(e) = cur {
        entries.push(e);
    }
    entries
}

fn probe_worktree(e: Entry, is_main: bool, base: Option<&str>) -> Worktree {
    let mut wt = Worktree {
        path: e.path,
        head: e.head,
        branch: e.branch,
        detached: e.detached,
        bare: e.bare,
        is_main,
        locked: e.locked,
        prunable: e.prunable,
        upstream: None,
        ahead: 0,
        behind: 0,
        staged: 0,
        unstaged: 0,
        untracked: 0,
        conflicts: 0,
        tracked: 0,
        files: Vec::new(),
        stashes: Vec::new(),
        last_commit: None,
        recent: Vec::new(),
        unmerged: Vec::new(),
        unmerged_total: 0,
        last_touched: 0,
        broken: false,
    };

    if wt.bare || !wt.path.exists() {
        wt.broken = !wt.bare;
        wt.last_touched = mtime(&wt.path).unwrap_or(0);
        return wt;
    }

    match git(
        &wt.path,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=normal",
        ],
    ) {
        Some(status) => parse_status(&status, &mut wt),
        None => wt.broken = true,
    }

    wt.recent = git(&wt.path, &["log", "-5", "--format=%H%x1f%ct%x1f%an%x1f%s"])
        .map(|s| parse_commits(&s))
        .unwrap_or_default();
    wt.last_commit = wt.recent.first().cloned();

    // Commits that exist only on this branch. With an upstream, `ahead` already
    // answers it; without one, ask how far we have drifted from the base branch.
    if wt.upstream.is_none() {
        if let (Some(base), false) = (base, wt.detached && wt.branch.is_none()) {
            let range = format!("{base}..HEAD");
            wt.unmerged_total = git(&wt.path, &["rev-list", "--count", &range])
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        }
    } else {
        wt.unmerged_total = wt.ahead;
    }
    if wt.unmerged_total > 0 {
        let range = match (&wt.upstream, base) {
            (Some(up), _) => format!("{up}..HEAD"),
            (None, Some(base)) => format!("{base}..HEAD"),
            (None, None) => "HEAD".into(),
        };
        if let Some(out) = git(
            &wt.path,
            &["log", "-20", "--format=%H%x1f%ct%x1f%an%x1f%s", &range],
        ) {
            wt.unmerged = parse_commits(&out);
        }
    }

    // Activity means git activity. The worktree directory's own mtime is not a
    // signal: editors, builds and backup tools touch it constantly, which would
    // report every checkout on the machine as fresh. The reflog records real
    // work (commits, checkouts, merges, resets) and is per-worktree; the index
    // catches staging that has not been committed yet.
    let commit_time = wt.last_commit.as_ref().map(|c| c.time).unwrap_or(0);
    let gitdir = gitdir_of(&wt.path);
    let reflog_time = gitdir
        .as_ref()
        .and_then(|g| mtime(&g.join("logs/HEAD")))
        .unwrap_or(0);
    let index_time = gitdir
        .as_ref()
        .and_then(|g| mtime(&g.join("index")))
        .unwrap_or(0);
    wt.last_touched = commit_time.max(reflog_time).max(index_time);
    if wt.last_touched == 0 {
        // A repo with no commits and no reflog: the directory is all there is.
        wt.last_touched = mtime(&wt.path).unwrap_or(0);
    }

    wt
}

/// The per-worktree git dir: `.git` is a directory in the main worktree and a
/// pointer file in linked ones.
fn gitdir_of(path: &Path) -> Option<PathBuf> {
    let dot = path.join(".git");
    let meta = std::fs::metadata(&dot).ok()?;
    if meta.is_dir() {
        return Some(dot);
    }
    let text = std::fs::read_to_string(&dot).ok()?;
    let rest = text.trim().strip_prefix("gitdir:")?.trim();
    Some(PathBuf::from(rest))
}

fn mtime(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

fn parse_status(out: &str, wt: &mut Worktree) {
    const FILE_CAP: usize = 500;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            wt.upstream = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            let mut it = rest.split_whitespace();
            wt.ahead = it
                .next()
                .and_then(|s| s.trim_start_matches('+').parse().ok())
                .unwrap_or(0);
            wt.behind = it
                .next()
                .and_then(|s| s.trim_start_matches('-').parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("# branch.head ") {
            let head = rest.trim();
            if head != "(detached)" && wt.branch.is_none() {
                wt.branch = Some(head.to_string());
            }
        } else if line.starts_with("# ") {
            continue;
        } else if let Some(rest) = line.strip_prefix("? ") {
            wt.untracked += 1;
            push_file(wt, "??", rest, FILE_CAP);
        } else if let Some(rest) = line.strip_prefix("u ") {
            wt.conflicts += 1;
            wt.tracked += 1;
            let path = rest.split_whitespace().last().unwrap_or("");
            push_file(wt, "UU", path, FILE_CAP);
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            // "<kind> <XY> <sub> <mH> <mI> <mW> <hH> <hI> [<X><score> ] <path>"
            let mut it = line.split_whitespace();
            let kind = it.next().unwrap_or("");
            let xy = it.next().unwrap_or("..");
            let mut chars = xy.chars();
            let x = chars.next().unwrap_or('.');
            let y = chars.next().unwrap_or('.');
            if x != '.' {
                wt.staged += 1;
            }
            if y != '.' {
                wt.unstaged += 1;
            }
            wt.tracked += 1;
            // Renames carry an extra score field before the path, and their
            // path field is "<new>\t<old>". Walk the fixed fields by hand so
            // that spaces and tabs inside the path survive.
            let fields = if kind == "2" { 9 } else { 8 };
            let mut rest = line;
            for _ in 0..fields {
                match rest.split_once(' ') {
                    Some((_, tail)) => rest = tail,
                    None => {
                        rest = "";
                        break;
                    }
                }
            }
            let path = rest.split('\t').next().unwrap_or("");
            push_file(wt, xy, path, FILE_CAP);
        }
    }
}

fn push_file(wt: &mut Worktree, code: &str, path: &str, cap: usize) {
    if wt.files.len() < cap && !path.is_empty() {
        wt.files.push(FileChange {
            code: code.to_string(),
            path: path.to_string(),
        });
    }
}

/// `git stash list --format=%gd%x1f%gs%x1f%ct%x1f%H`.
///
/// The subject is `WIP on <branch>: …` for an automatic stash and
/// `On <branch>: <message>` for `git stash push -m`. Splitting on the first
/// colon is safe: git forbids a colon in a ref name, so the first one always
/// ends the branch.
fn parse_stashes(out: &str) -> Vec<Stash> {
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.split('\x1f');
            let id = fields.next()?.to_string();
            let subject = fields.next()?;
            let time = fields.next().and_then(|t| t.parse().ok()).unwrap_or(0);

            let sha = fields.next().unwrap_or_default().to_string();

            // Only a subject git wrote carries a branch. `git stash store`
            // takes an arbitrary message, and splitting that on its first colon
            // invents a branch out of the first word — "wip: refactor the
            // parser" became branch `wip`, message `refactor the parser`.
            let prefixed = subject
                .strip_prefix("WIP on ")
                .or_else(|| subject.strip_prefix("On "));
            let (branch, message) = match prefixed.and_then(|rest| rest.split_once(": ")) {
                Some((branch, message)) => (
                    (branch != "(no branch)").then(|| branch.to_string()),
                    message.to_string(),
                ),
                None => (None, subject.to_string()),
            };
            Some(Stash {
                id,
                sha,
                branch,
                message,
                time,
            })
        })
        .collect()
}

fn parse_commits(out: &str) -> Vec<Commit> {
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut it = line.split('\x1f');
            Some(Commit {
                sha: it.next()?.to_string(),
                time: it.next()?.parse().unwrap_or(0),
                author: it.next()?.to_string(),
                subject: it.next().unwrap_or("").to_string(),
            })
        })
        .collect()
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// "3d", "5h", "now" — compact, for a column.
pub fn ago(secs: u64) -> String {
    const MIN: u64 = 60;
    const HOUR: u64 = 3_600;
    const DAY: u64 = 86_400;
    match secs {
        s if s < MIN => "now".into(),
        s if s < HOUR => format!("{}m", s / MIN),
        s if s < DAY => format!("{}h", s / HOUR),
        s if s < 365 * DAY => format!("{}d", s / DAY),
        s => format!("{}y", s / (365 * DAY)),
    }
}

/// Absolute-ish timestamp for the detail pane.
pub fn stamp(unix: u64, now: u64) -> String {
    if unix == 0 {
        return "unknown".into();
    }
    format!("{} ago", ago(now.saturating_sub(unix)))
}

/// Build a worktree by hand, for tests in this crate.
#[cfg(test)]
pub(crate) fn test_worktree(label: &str) -> Worktree {
    Worktree {
        path: PathBuf::from(format!("/tmp/{label}")),
        head: "abcdef1234".into(),
        branch: Some(label.to_string()),
        detached: false,
        bare: false,
        is_main: false,
        locked: None,
        prunable: None,
        upstream: None,
        ahead: 0,
        behind: 0,
        staged: 0,
        unstaged: 0,
        untracked: 0,
        conflicts: 0,
        tracked: 0,
        files: Vec::new(),
        stashes: Vec::new(),
        last_commit: None,
        recent: Vec::new(),
        unmerged: Vec::new(),
        unmerged_total: 0,
        last_touched: now(),
        broken: false,
    }
}

/// Build a repo with `linked` extra worktrees, for tests in this crate.
///
/// Worktree paths are namespaced under the repository, because two repositories
/// on one machine do not share a directory — and a marking keyed by path cannot
/// tell them apart if the fixture pretends they do.
#[cfg(test)]
pub(crate) fn test_repo(name: &str, linked: usize) -> Repo {
    let mut worktrees = vec![test_worktree(&format!("{name}/main"))];
    worktrees[0].is_main = true;
    worktrees[0].branch = Some("main".into());
    for i in 0..linked {
        let mut wt = test_worktree(&format!("{name}/feat/branch-{i}"));
        wt.branch = Some(format!("feat/branch-{i}"));
        worktrees.push(wt);
    }
    Repo {
        name: name.to_string(),
        common_dir: PathBuf::from(format!("/tmp/{name}/.git")),
        root: PathBuf::from(format!("/tmp/{name}")),
        remote: None,
        default_base: Some("origin/main".into()),
        stashes: Vec::new(),
        worktrees,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_worktree_list() {
        let out = "worktree /a/main\nHEAD abc\nbranch refs/heads/main\n\n\
                   worktree /a/wt\nHEAD def\nbranch refs/heads/feat/x\nlocked in use\n\n";
        let e = parse_worktree_list(out);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].branch.as_deref(), Some("main"));
        assert_eq!(e[1].path, PathBuf::from("/a/wt"));
        assert_eq!(e[1].branch.as_deref(), Some("feat/x"));
        assert_eq!(e[1].locked.as_deref(), Some("in use"));
    }

    #[test]
    fn parses_status_counts_and_paths() {
        let mut wt = test_worktree("x");
        let out = "# branch.oid abc\n# branch.head feat/x\n# branch.upstream origin/main\n\
                   # branch.ab +2 -1\n\
                   1 M. N... 100644 100644 100644 aaa bbb README.md\n\
                   1 .M N... 100644 100644 100644 aaa bbb src/lib.rs\n\
                   1 MM N... 100644 100644 100644 aaa bbb src/main.rs\n\
                   ? notes.txt\n\
                   u UU N... 100644 100644 100644 100644 aa bb cc merge.rs\n";
        parse_status(out, &mut wt);
        assert_eq!(wt.upstream.as_deref(), Some("origin/main"));
        assert_eq!((wt.ahead, wt.behind), (2, 1));
        assert_eq!(
            (wt.staged, wt.unstaged, wt.untracked, wt.conflicts),
            (2, 2, 1, 1)
        );
        assert_eq!(wt.files[0].path, "README.md");
        assert_eq!(wt.files[3].code, "??");
        assert_eq!(
            wt.changed_files(),
            5,
            "five paths are five files, however many changes each carries"
        );
        assert_eq!(wt.tracked_changes(), 4);
    }

    #[test]
    fn parses_rename_entries() {
        let mut wt = test_worktree("x");
        let out = "2 R. N... 100644 100644 100644 aaa bbb R100 new/name.rs\told/name.rs\n";
        parse_status(out, &mut wt);
        assert_eq!(wt.files.len(), 1);
        assert_eq!(wt.files[0].path, "new/name.rs");
        assert_eq!(wt.staged, 1);
    }

    #[test]
    fn a_worktree_git_cannot_read_is_never_safe_to_remove() {
        let mut wt = test_worktree("x");
        assert!(wt.is_safe_to_remove(), "a clean worktree is safe");

        // Every counter is zero because nothing was ever read, which is exactly
        // how "clean" looks. The two must not be confused.
        wt.broken = true;
        assert_eq!(wt.changed_files(), 0);
        assert_eq!(wt.unpushed(), 0);
        assert_eq!(wt.salvage(), Salvage::Unknown);
        assert!(!wt.is_safe_to_remove());
        assert!(wt.verdict().contains("cannot read"), "{}", wt.verdict());
    }

    #[test]
    fn a_locked_worktree_is_never_safe_to_remove() {
        let mut wt = test_worktree("x");
        wt.locked = Some("in use by a build".into());
        assert_eq!(wt.salvage(), Salvage::Nothing, "it is still clean");
        assert!(!wt.is_safe_to_remove(), "but git refuses to remove it");
        assert_eq!(
            wt.verdict(),
            "Locked: in use by a build — nothing to salvage",
            "and the sentence must not say otherwise"
        );
    }

    #[test]
    fn only_a_worktree_that_can_go_is_told_it_can_go() {
        let clean = test_worktree("x");
        assert!(clean.verdict().contains("safe to remove"));

        let mut main = test_worktree("main");
        main.is_main = true;
        assert!(
            !main.verdict().contains("safe to remove"),
            "{}",
            main.verdict()
        );

        let mut broken = test_worktree("x");
        broken.broken = true;
        assert!(!broken.verdict().contains("safe to remove"));
    }

    #[test]
    fn the_main_checkout_is_never_safe_to_remove() {
        let mut wt = test_worktree("main");
        wt.is_main = true;
        assert!(!wt.is_safe_to_remove());
    }

    /// One number for "uncommitted" answered the wrong question: three modified
    /// tracked files and three untracked ones are different situations, and the
    /// sentence has to say which.
    /// One number for "uncommitted" answered the wrong question: three modified
    /// tracked files and three untracked ones are different situations, and the
    /// sentence has to say which.
    ///
    /// Driven through the parser rather than by setting counters, so the
    /// sentence cannot disagree with what git actually said.
    #[test]
    fn the_verdict_says_which_kind_of_uncommitted() {
        let verdict_of = |status: &str| {
            let mut wt = test_worktree("x");
            parse_status(status, &mut wt);
            wt.verdict()
        };

        let m = |path| format!("1 .M N... 100644 100644 100644 aaa bbb {path}\n");
        assert_eq!(
            verdict_of(&format!("{}{}{}", m("a"), m("b"), m("c"))),
            "3 modified files"
        );
        assert_eq!(verdict_of("? a\n? b\n? c\n? d\n"), "4 untracked files");
        assert_eq!(
            verdict_of(&format!("{}{}? c\n", m("a"), m("b"))),
            "2 modified files, 1 untracked file"
        );
        assert_eq!(
            verdict_of("u UU N... 100644 100644 100644 100644 aa bb cc merge.rs\n"),
            "1 modified file",
            "a conflict is a tracked file"
        );
        assert_eq!(
            verdict_of("1 MM N... 100644 100644 100644 aaa bbb both.rs\n"),
            "1 modified file",
            "edited, staged, and edited again is still one file"
        );
    }

    #[test]
    fn tracked_and_untracked_are_counted_apart_but_both_count() {
        let mut wt = test_worktree("x");
        wt.staged = 1;
        wt.unstaged = 2;
        wt.conflicts = 1;
        wt.tracked = 3;
        wt.untracked = 5;
        assert_eq!(
            wt.tracked_changes(),
            3,
            "two of the changes are to one file"
        );
        assert_eq!(wt.changed_files(), 8);
        assert!(
            !wt.is_safe_to_remove(),
            "untracked files are unrecoverable too; neither kind is the lesser"
        );
    }

    #[test]
    fn salvage_ranks_uncommitted_above_commits() {
        let mut wt = test_worktree("x");
        assert_eq!(wt.salvage(), Salvage::Nothing);
        wt.upstream = Some("origin/main".into());
        wt.ahead = 3;
        assert_eq!(wt.salvage(), Salvage::Commits);
        wt.untracked = 1;
        assert_eq!(wt.salvage(), Salvage::Uncommitted);
        assert!(Salvage::Uncommitted > Salvage::Commits);
        assert!(
            Salvage::Unknown > Salvage::Commits,
            "an unreadable worktree may hold anything; an unpushed commit is in the reflog"
        );
    }

    #[test]
    fn unpushed_falls_back_to_base_when_no_upstream() {
        let mut wt = test_worktree("x");
        wt.unmerged_total = 4;
        assert_eq!(wt.unpushed(), 4);
        wt.upstream = Some("origin/main".into());
        wt.ahead = 1;
        assert_eq!(wt.unpushed(), 1);
    }

    #[test]
    fn repository_selecting_environment_is_never_inherited() {
        // Asserted on the command rather than by setting the variables, which
        // would change them for every other test in this process.
        let cmd = command(Path::new("/tmp"), &["status"]);
        let removed: Vec<String> = cmd
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect();
        for key in REPO_ENV {
            assert!(
                removed.contains(&key.to_string()),
                "{key} is still inherited"
            );
        }
    }

    /// Shapes taken from real `git stash list` output.
    #[test]
    fn parses_the_shapes_git_actually_writes() {
        let out = concat!(
            "stash@{0}\u{1f}WIP on worktree-agent-aba0f691: e08ba70 feat: a thing\u{1f}1775087388\u{1f}aaa\n",
            "stash@{1}\u{1f}On feat/x: work in progress\u{1f}1775087386\u{1f}bbb\n",
            "stash@{2}\u{1f}WIP on (no branch): 242b353 feat: on a detached head\u{1f}1774221743\u{1f}ccc\n",
            "stash@{3}\u{1f}On research/verification: c8aa59f a slash in the branch\u{1f}1774141909\u{1f}ddd\n",
            "stash@{4}\u{1f}wip: refactor the parser\u{1f}1774141900\u{1f}eee\n",
        );
        let stashes = parse_stashes(out);
        assert_eq!(stashes.len(), 5);

        assert_eq!(stashes[0].id, "stash@{0}");
        assert_eq!(
            stashes[0].branch.as_deref(),
            Some("worktree-agent-aba0f691")
        );
        assert_eq!(stashes[0].message, "e08ba70 feat: a thing");
        assert_eq!(stashes[0].time, 1_775_087_388);

        assert_eq!(stashes[1].branch.as_deref(), Some("feat/x"));
        assert_eq!(stashes[1].message, "work in progress");

        assert_eq!(
            stashes[2].branch, None,
            "a stash made on a detached head belongs to no branch"
        );

        assert_eq!(
            stashes[3].branch.as_deref(),
            Some("research/verification"),
            "a slash is ordinary in a ref name; only the colon ends it"
        );

        // `git stash store` takes any message. Splitting one on its first colon
        // invents a branch out of its first word and eats it from the message.
        assert_eq!(stashes[4].branch, None);
        assert_eq!(stashes[4].message, "wip: refactor the parser");
        assert_eq!(stashes[4].sha, "eee");
    }

    #[test]
    fn a_stash_on_a_branch_is_that_worktree_s_stash() {
        let mut wt = test_worktree("feat/x");
        wt.branch = Some("feat/x".into());
        assert!(wt.stashes.is_empty());
        wt.stashes.push(Stash {
            id: "stash@{0}".into(),
            sha: "abc1234".into(),
            branch: Some("feat/x".into()),
            message: "work in progress".into(),
            time: now(),
        });
        // Removing the worktree does not remove the stash — refs/stash lives in
        // the common dir — so this does not change what would be lost.
        assert!(
            wt.is_safe_to_remove(),
            "the stash survives removal; it is surfaced, not counted as loss"
        );
    }

    #[test]
    fn ago_is_compact() {
        assert_eq!(ago(30), "now");
        assert_eq!(ago(3_600), "1h");
        assert_eq!(ago(86_400 * 3), "3d");
        assert_eq!(ago(86_400 * 400), "1y");
    }

    #[test]
    fn unreadable_worktrees_report_no_age() {
        let mut wt = test_worktree("x");
        wt.last_touched = 0;
        assert_eq!(wt.age_label(now()), "—");
    }
}

#[cfg(test)]
mod repo_tests {
    use super::*;

    #[test]
    fn a_repository_sums_what_sums_and_takes_the_worst_of_what_does_not() {
        let mut repo = test_repo("alpha", 2);
        repo.worktrees[0].untracked = 3;
        repo.worktrees[1].untracked = 4;
        repo.worktrees[1].upstream = Some("origin/main".into());
        repo.worktrees[1].ahead = 2;
        repo.worktrees[1].behind = 79;
        repo.worktrees[2].upstream = Some("origin/main".into());
        repo.worktrees[2].ahead = 1;
        repo.worktrees[2].behind = 79;

        let totals = repo.totals();
        assert_eq!(
            totals.files, 7,
            "distinct files in distinct worktrees add up"
        );
        assert_eq!(
            totals.ahead, 3,
            "distinct commits on distinct branches add up"
        );
        assert_eq!(
            totals.behind, 79,
            "two worktrees behind the same upstream are behind by 79, not 158"
        );
    }
}
