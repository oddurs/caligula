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
    pub stashes: usize,
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
    pub files: Vec<FileChange>,

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Salvage {
    /// Clean, and everything it contains lives somewhere else too.
    Nothing,
    /// Commits exist only here; removing the worktree keeps the branch.
    Commits,
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
        self.staged + self.unstaged + self.untracked + self.conflicts
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
        if self.is_dirty() {
            Salvage::Uncommitted
        } else if self.unpushed() > 0 {
            Salvage::Commits
        } else {
            Salvage::Nothing
        }
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
        let mut parts = Vec::new();
        if self.changed_files() > 0 {
            parts.push(format!(
                "{} uncommitted file{}",
                self.changed_files(),
                plural(self.changed_files())
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
        if parts.is_empty() {
            "Nothing to salvage — safe to remove".into()
        } else {
            parts.join(", ")
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
    let stashes = git(&root, &["stash", "list"])
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);

    let worktrees = entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| probe_worktree(e, i == 0, base.as_deref()))
        .collect();

    Some(Repo {
        name,
        common_dir: common,
        root,
        remote,
        default_base: base,
        stashes,
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
        files: Vec::new(),
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
        files: Vec::new(),
        last_commit: None,
        recent: Vec::new(),
        unmerged: Vec::new(),
        unmerged_total: 0,
        last_touched: now(),
        broken: false,
    }
}

/// Build a repo with `linked` extra worktrees, for tests in this crate.
#[cfg(test)]
pub(crate) fn test_repo(name: &str, linked: usize) -> Repo {
    let mut worktrees = vec![test_worktree("main")];
    worktrees[0].is_main = true;
    for i in 0..linked {
        worktrees.push(test_worktree(&format!("feat/branch-{i}")));
    }
    Repo {
        name: name.to_string(),
        common_dir: PathBuf::from(format!("/tmp/{name}/.git")),
        root: PathBuf::from(format!("/tmp/{name}")),
        remote: None,
        default_base: Some("origin/main".into()),
        stashes: 0,
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
        assert_eq!(wt.changed_files(), 6);
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
    fn salvage_ranks_uncommitted_above_commits() {
        let mut wt = test_worktree("x");
        assert_eq!(wt.salvage(), Salvage::Nothing);
        wt.upstream = Some("origin/main".into());
        wt.ahead = 3;
        assert_eq!(wt.salvage(), Salvage::Commits);
        wt.untracked = 1;
        assert_eq!(wt.salvage(), Salvage::Uncommitted);
        assert!(Salvage::Uncommitted > Salvage::Commits);
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
