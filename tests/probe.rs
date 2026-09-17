//! What caligula makes of a real repository.
//!
//! This is the half of the fixture that is *not* snapshotted: the model, asserted
//! field by field. A snapshot would hide which value moved.

mod fixture;

use caligula::git::{Salvage, Staleness};

fn find<'a>(repo: &'a caligula::git::Repo, branch: &str) -> &'a caligula::git::Worktree {
    repo.worktrees
        .iter()
        .find(|w| w.label() == branch)
        .unwrap_or_else(|| panic!("no worktree on {branch}"))
}

#[test]
fn every_kind_of_worktree_is_found() {
    let (_tmp, repo) = fixture::build();
    let mut branches: Vec<String> = repo
        .worktrees
        .iter()
        .filter(|w| !w.detached)
        .map(|w| w.label())
        .collect();
    branches.sort();
    assert_eq!(
        branches,
        [
            "chore/clean",
            "docs/gone",
            "feat/ahead",
            "feat/dirty",
            "fix/locked",
            "main"
        ]
    );
    assert_eq!(repo.worktrees.iter().filter(|w| w.detached).count(), 1);
    assert_eq!(repo.linked_count(), 6);
}

#[test]
fn a_clean_worktree_has_nothing_to_salvage() {
    let (_tmp, repo) = fixture::build();
    let wt = find(&repo, "chore/clean");
    assert_eq!(wt.salvage(), Salvage::Nothing);
    assert_eq!(wt.changed_files(), 0);
    assert_eq!(wt.unpushed(), 0);
    assert_eq!(wt.verdict(), "Nothing to salvage — safe to remove");
}

#[test]
fn a_dirty_worktree_counts_tracked_and_untracked_separately() {
    let (_tmp, repo) = fixture::build();
    let wt = find(&repo, "feat/dirty");
    assert_eq!(wt.salvage(), Salvage::Uncommitted);
    assert_eq!(wt.unstaged, 1, "README.md is modified but not staged");
    assert_eq!(wt.untracked, 1, "scratch.txt is untracked");
    assert_eq!(wt.staged, 0);
    let paths: Vec<&str> = wt.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"README.md"), "{paths:?}");
    assert!(paths.contains(&"scratch.txt"), "{paths:?}");
}

#[test]
fn a_commit_on_no_other_branch_counts_as_unpushed() {
    let (_tmp, repo) = fixture::build();
    let wt = find(&repo, "feat/ahead");
    assert_eq!(wt.salvage(), Salvage::Commits);
    assert_eq!(wt.unpushed(), 1);
    assert_eq!(wt.unmerged.len(), 1);
    assert_eq!(
        wt.unmerged[0].subject,
        "feat: a commit that exists only here"
    );
    assert_eq!(wt.verdict(), "1 off-base commit");
}

#[test]
fn a_locked_worktree_says_why() {
    let (_tmp, repo) = fixture::build();
    let wt = find(&repo, "fix/locked");
    assert_eq!(wt.locked.as_deref(), Some("in use by a build"));
}

#[test]
fn a_worktree_whose_directory_is_gone_is_prunable_and_broken() {
    let (_tmp, repo) = fixture::build();
    let wt = find(&repo, "docs/gone");
    assert!(wt.prunable.is_some(), "git should mark it prunable");
    assert!(
        wt.broken,
        "caligula cannot read a directory that is not there"
    );
}

#[test]
fn the_main_checkout_is_first_and_marked() {
    let (_tmp, repo) = fixture::build();
    assert!(repo.worktrees[0].is_main);
    assert_eq!(repo.worktrees[0].label(), "main");
    assert_eq!(repo.worktrees.iter().filter(|w| w.is_main).count(), 1);
}

#[test]
fn normalized_ages_cover_every_staleness_band() {
    let (_tmp, repo) = fixture::build();
    let bands: Vec<Staleness> = repo
        .worktrees
        .iter()
        .map(|w| w.staleness(fixture::NOW))
        .collect();
    assert!(bands.contains(&Staleness::Active), "{bands:?}");
    assert!(bands.contains(&Staleness::Recent), "{bands:?}");
    assert!(bands.contains(&Staleness::Stale), "{bands:?}");
    assert!(bands.contains(&Staleness::Ancient), "{bands:?}");
}

#[test]
fn a_detached_worktree_is_labelled_by_its_commit() {
    let (_tmp, repo) = fixture::build();
    let wt = repo
        .worktrees
        .iter()
        .find(|w| w.detached)
        .expect("the fixture has a detached worktree");
    assert!(wt.branch.is_none(), "a detached worktree is on no branch");
    // normalize() rewrites the head, so the label is the first eight of that.
    assert_eq!(wt.label(), format!("({})", &wt.head[..8]));
}
