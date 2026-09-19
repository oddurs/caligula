//! Removing a marking, against a real repository.
//!
//! These run the actual `git worktree remove` and then ask git what is left, so
//! a change that only satisfies the model cannot pass.

mod fixture;

use std::path::{Path, PathBuf};

use caligula::app::App;
use caligula::git::Repo;

fn app_for(repo: Repo) -> App {
    let mut app = App::new();
    app.scanning = false;
    app.add_repo(repo);
    app
}

fn mark(app: &mut App, branch: &str) -> PathBuf {
    let (ri, wi) = app
        .repos
        .iter()
        .enumerate()
        .find_map(|(ri, r)| {
            r.worktrees
                .iter()
                .position(|w| w.label() == branch)
                .map(|wi| (ri, wi))
        })
        .unwrap_or_else(|| panic!("no worktree on {branch}"));
    let path = app.repos[ri].worktrees[wi].path.clone();
    app.marked.insert(path.clone());
    path
}

fn branches_on_disk(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = fixture::reprobe(root)
        .worktrees
        .iter()
        .map(|w| w.label())
        .collect();
    out.sort();
    out
}

#[test]
fn every_marked_worktree_is_removed() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    let clean = mark(&mut app, "chore/clean");
    let ahead = mark(&mut app, "feat/ahead");
    app.ask_remove(false);
    assert!(app.confirm.is_some(), "a marking should open a dialog");
    app.run_confirmed();

    let left = branches_on_disk(&root);
    assert!(!left.contains(&"chore/clean".to_string()), "{left:?}");
    assert!(!left.contains(&"feat/ahead".to_string()), "{left:?}");
    assert!(left.contains(&"main".to_string()), "{left:?}");
    assert!(!clean.exists(), "the directory should be gone");
    assert!(!ahead.exists(), "the directory should be gone");
    assert!(app.marked.is_empty(), "removed worktrees stay marked");
    drop(tmp);
}

#[test]
fn a_branch_survives_unless_it_is_asked_for() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    mark(&mut app, "chore/clean");
    app.ask_remove(false);
    app.run_confirmed();

    let out =
        caligula::git::git(&root, &["branch", "--list", "chore/clean"]).expect("git branch --list");
    assert!(
        out.contains("chore/clean"),
        "removing a worktree must keep its branch: {out:?}"
    );
    drop(tmp);
}

#[test]
fn asking_for_the_branch_deletes_it_too() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    mark(&mut app, "chore/clean");
    app.ask_remove(true);
    app.run_confirmed();

    let out =
        caligula::git::git(&root, &["branch", "--list", "chore/clean"]).expect("git branch --list");
    assert!(out.trim().is_empty(), "the branch should be gone: {out:?}");
    drop(tmp);
}

#[test]
fn uncommitted_work_is_removed_only_because_force_was_used() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    // git refuses this without --force; caligula supplies it because the dialog
    // said in red exactly what was about to be destroyed.
    let dirty = mark(&mut app, "feat/dirty");
    app.ask_remove(false);
    app.run_confirmed();

    assert!(!dirty.exists(), "the dirty worktree should be gone");
    assert!(!branches_on_disk(&root).contains(&"feat/dirty".to_string()));
    drop(tmp);
}

#[test]
fn a_locked_worktree_is_kept_and_said_to_be_kept() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    mark(&mut app, "fix/locked");
    mark(&mut app, "chore/clean");
    app.ask_remove(false);

    let body = app
        .confirm
        .as_ref()
        .map(|c| {
            c.body
                .iter()
                .map(|(t, _)| t.clone())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .expect("a dialog");
    assert!(
        body.contains("fix/locked is kept"),
        "the dialog must say what it is not doing: {body}"
    );

    app.run_confirmed();
    let left = branches_on_disk(&root);
    assert!(left.contains(&"fix/locked".to_string()), "{left:?}");
    assert!(!left.contains(&"chore/clean".to_string()), "{left:?}");
    drop(tmp);
}

#[test]
fn a_marking_of_only_the_main_checkout_removes_nothing() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    mark(&mut app, "main");
    app.ask_remove(false);
    assert!(
        app.confirm.is_none(),
        "there is nothing to confirm, so nothing should be asked"
    );
    assert_eq!(branches_on_disk(&root).len(), 7);
    drop(tmp);
}

#[test]
fn one_failure_does_not_strand_the_rest() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    let clean = mark(&mut app, "chore/clean");
    mark(&mut app, "feat/ahead");
    app.ask_remove(false);

    // Pulled out from under caligula after the dialog opened, so its own
    // removal fails part way through the batch — which is the case a sweep of
    // forty has to survive.
    caligula::git::git_run(&root, &["worktree", "remove", clean.to_str().unwrap()])
        .expect("the fixture worktree is removable");

    app.run_confirmed();

    let left = branches_on_disk(&root);
    assert!(
        !left.contains(&"feat/ahead".to_string()),
        "the rest of the marking must still go: {left:?}"
    );
    drop(tmp);
}

#[test]
fn a_worktree_git_cannot_read_is_not_offered_for_removal() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    mark(&mut app, "docs/gone");
    app.ask_remove(false);
    assert!(
        app.confirm.is_none(),
        "git worktree remove cannot clear a record; prune can"
    );
    assert!(branches_on_disk(&root).contains(&"docs/gone".to_string()));
    drop(tmp);
}

#[test]
fn a_failed_removal_is_reported_in_full() {
    let (tmp, repo) = fixture::build_live();
    let root = repo.root.clone();
    let mut app = app_for(repo);

    let clean = mark(&mut app, "chore/clean");
    app.ask_remove(false);
    // Removed from under caligula after the dialog opened, so its own removal
    // fails and the whole of git's complaint has somewhere to go.
    caligula::git::git_run(&root, &["worktree", "remove", clean.to_str().unwrap()])
        .expect("the fixture worktree is removable");
    app.run_confirmed();

    let failure = app.failure.as_ref().expect("a failure should be reported");
    assert!(
        failure.title.contains("could not be removed"),
        "{}",
        failure.title
    );
    // The command has to be repeatable: the box is useless if it does not say
    // which directory, in which repository.
    assert!(
        failure.body.contains("git -C ") && failure.body.contains("worktree remove"),
        "the command must be there to repeat: {}",
        failure.body
    );
    assert!(
        failure.body.contains("clean"),
        "the failing path must be named: {}",
        failure.body
    );
    assert!(
        failure.body.lines().count() >= 2,
        "git's own words must survive alongside the command: {}",
        failure.body
    );
    drop(tmp);
}
