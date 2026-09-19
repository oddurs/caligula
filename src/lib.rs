//! caligula — a terminal browser for git worktrees.
//!
//! The binary is a thin shell around these modules: `scan` finds repositories,
//! `git` turns each one into a model of its worktrees, `app` holds what is on
//! screen, and `ui` draws it. Everything is public so the integration tests can
//! build a real repository, probe it, and render the result.

pub mod app;
pub mod git;
pub mod scan;
pub mod text;
pub mod ui;
