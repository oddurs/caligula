//! A real repository with one worktree of each kind caligula has to render.
//!
//! The repository is real — git builds it and caligula probes it through the
//! same code path it uses on a live machine, so a change in git's porcelain
//! output fails here rather than in the field. What is *not* real is what comes
//! out: [`normalize`] replaces the temporary path, the commit hashes and the
//! timestamps with fixed values, because a snapshot that carries any of those
//! records the clock rather than the layout.

// Each integration test compiles this module separately and uses part of it.
#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use caligula::app::App;
use caligula::git::{self, Repo};
use tempfile::TempDir;

/// The instant every fixture age is measured from. Arbitrary, and fixed:
/// 2026-01-01T00:00:00Z.
pub const NOW: u64 = 1_767_225_600;

const DAY: u64 = 86_400;

fn git_in(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .expect("git is on PATH; the suite cannot run without it");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(path: &Path, contents: &str) {
    std::fs::write(path, contents).expect("the fixture directory is writable");
}

/// Build the repository. The returned directory deletes itself when dropped, so
/// hold on to it for as long as the probed model is in use.
pub fn build() -> (TempDir, Repo) {
    let tmp = TempDir::new().expect("a temporary directory");
    let root = tmp.path().join("repo");
    std::fs::create_dir_all(&root).expect("the fixture root");

    git_in(&root, &["init", "-q", "-b", "main", "."]);
    // probe_repo runs git with the ambient environment, so a contributor's
    // global core.excludesFile would decide whether the untracked file in the
    // dirty worktree is seen at all. Local config wins over global and is
    // shared by every worktree through the common dir.
    git_in(&root, &["config", "core.excludesFile", "/dev/null"]);
    write(&root.join("README.md"), "fixture\n");
    git_in(&root, &["add", "-A"]);
    git_in(&root, &["commit", "-qm", "init: the first commit"]);

    let wt = |name: &str| tmp.path().join(name);

    // Clean: nothing to salvage, the case the sweep exists for.
    git_in(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            wt("clean").to_str().unwrap(),
            "-b",
            "chore/clean",
        ],
    );

    // Dirty: a modified tracked file and an untracked one, which are different
    // kinds of risk and are counted separately.
    git_in(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            wt("dirty").to_str().unwrap(),
            "-b",
            "feat/dirty",
        ],
    );
    write(&wt("dirty").join("README.md"), "fixture\nmodified\n");
    write(&wt("dirty").join("scratch.txt"), "untracked\n");

    // Ahead: a commit that exists on no other branch.
    git_in(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            wt("ahead").to_str().unwrap(),
            "-b",
            "feat/ahead",
        ],
    );
    write(&wt("ahead").join("README.md"), "fixture\nahead\n");
    git_in(&wt("ahead"), &["add", "-A"]);
    git_in(
        &wt("ahead"),
        &["commit", "-qm", "feat: a commit that exists only here"],
    );

    // Locked: git refuses to remove it, and so must caligula.
    git_in(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            wt("locked").to_str().unwrap(),
            "-b",
            "fix/locked",
        ],
    );
    git_in(
        &root,
        &[
            "worktree",
            "lock",
            "--reason",
            "in use by a build",
            wt("locked").to_str().unwrap(),
        ],
    );

    // Detached: no branch at all, so the label falls back to the short sha.
    git_in(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            wt("detached").to_str().unwrap(),
        ],
    );

    // Prunable: the directory is gone but the administrative record remains.
    git_in(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            wt("gone").to_str().unwrap(),
            "-b",
            "docs/gone",
        ],
    );
    std::fs::remove_dir_all(wt("gone")).expect("the fixture worktree is removable");

    let mut repo = git::probe_repo(&root).expect("the fixture repository probes");
    normalize(&mut repo);
    (tmp, repo)
}

/// Replace everything that changes between runs with a fixed stand-in.
///
/// Ages are assigned by position rather than measured, so the age column is
/// exercised across all four staleness bands without waiting two months.
pub fn normalize(repo: &mut Repo) {
    let real_root = repo.root.clone();
    let prefix = real_root
        .parent()
        .expect("the fixture root has a parent")
        .to_path_buf();

    repo.name = "fixture".into();
    repo.root = Path::new("/fixture/repo").to_path_buf();
    repo.common_dir = Path::new("/fixture/repo/.git").to_path_buf();
    repo.remote = Some("git@github.com:fixture/fixture.git".into());

    // One worktree per band: hours, days, weeks, months, and one so old it is
    // past anything the interface distinguishes.
    let ages = [
        2 * 3_600,
        DAY,
        9 * DAY,
        40 * DAY,
        90 * DAY,
        400 * DAY,
        700 * DAY,
    ];

    for (i, wt) in repo.worktrees.iter_mut().enumerate() {
        let tail = wt
            .path
            .strip_prefix(&prefix)
            .map(|p| p.to_string_lossy().into_owned())
            .expect("every fixture worktree lives under the fixture root");
        wt.path = Path::new("/fixture").join(tail);

        // Padded on the right: short_sha takes the first eight characters, so
        // left-padding would render every worktree's head as 00000000 and the
        // snapshots could not tell one from another.
        wt.head = format!("{:0<40}", format!("{}head", i));
        wt.last_touched = NOW - ages.get(i).copied().unwrap_or(DAY);

        if let Some(commit) = wt.last_commit.as_mut() {
            commit.sha = format!("{:0<40}", format!("{}c0", i));
            commit.time = wt.last_touched;
        }
        for (j, commit) in wt.recent.iter_mut().enumerate() {
            commit.sha = format!("{:0<40}", format!("{}r{}", i, j));
            commit.time = wt.last_touched - (j as u64 * DAY);
        }
        for (j, commit) in wt.unmerged.iter_mut().enumerate() {
            commit.sha = format!("{:0<40}", format!("{}u{}", i, j));
            commit.time = wt.last_touched - (j as u64 * DAY);
        }
    }
}

/// An app holding the fixture repository, with the clock pinned.
pub fn app(repo: Repo) -> App {
    let mut app = App::new();
    app.now = NOW;
    app.scanning = false;
    app.add_repo(repo);
    app
}
