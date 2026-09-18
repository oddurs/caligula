//! What caligula actually puts on the screen.
//!
//! Each test renders a frame through ratatui's test backend and compares it with
//! a recorded screen. When a change moves the layout on purpose, re-record with
//! `INSTA_UPDATE=always scripts/task test` and **read the diff** — that diff is
//! the review of the change to the interface.

mod fixture;

use caligula::app::{App, Lens, Row, Sort};
use caligula::ui;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;

/// The buffer as text. Styles are deliberately dropped: colour is asserted by
/// reading the code, and a snapshot full of escape codes is not reviewable.
fn screen(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The colours of each line, left to right, with runs collapsed.
///
/// `screen` drops styles, and styles are where half of this interface lives:
/// the staleness banding on the age column, the severity of a verdict, and the
/// selected row, which is a background and not a glyph. Without this a
/// regression that paints every row the same colour passes every screen.
fn palette(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| {
            let mut runs: Vec<String> = Vec::new();
            let mut selected = false;
            for x in 0..buf.area.width {
                let cell = &buf[(x, y)];
                if cell.bg != Color::Reset {
                    selected = true;
                }
                if cell.symbol().trim().is_empty() {
                    continue;
                }
                let fg = format!("{:?}", cell.fg);
                if runs.last().map(String::as_str) != Some(fg.as_str()) {
                    runs.push(fg);
                }
            }
            format!(
                "{y:>2}{} {}",
                if selected { " *" } else { "  " },
                runs.join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    terminal
        .draw(|f| ui::draw(f, app))
        .expect("the frame draws");
    screen(terminal.backend().buffer())
}

fn render_palette(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    terminal
        .draw(|f| ui::draw(f, app))
        .expect("the frame draws");
    palette(terminal.backend().buffer())
}

/// Put the cursor on the worktree with this branch.
fn select(app: &mut App, branch: &str) {
    let idx = app
        .rows
        .iter()
        .position(|row| match row {
            Row::Worktree { repo, wt } => app.repos[*repo].worktrees[*wt].label() == branch,
            Row::Repo { .. } => false,
        })
        .unwrap_or_else(|| panic!("no row for {branch}"));
    app.go(idx);
}

fn fixture_app() -> App {
    let (tmp, repo) = fixture::build();
    // The probe is finished; the model no longer refers to the directory.
    drop(tmp);
    fixture::app(repo)
}

#[test]
fn the_list() {
    let mut app = fixture_app();
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn the_list_at_eighty_columns() {
    let mut app = fixture_app();
    insta::assert_snapshot!(render(&mut app, 80, 20));
}

#[test]
fn the_list_at_two_hundred_columns() {
    let mut app = fixture_app();
    insta::assert_snapshot!(render(&mut app, 200, 20));
}

#[test]
fn a_repository_detail() {
    let mut app = fixture_app();
    app.go(0);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn a_worktree_detail_with_changes() {
    let mut app = fixture_app();
    select(&mut app, "feat/dirty");
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn a_worktree_detail_with_unpushed_commits() {
    let mut app = fixture_app();
    select(&mut app, "feat/ahead");
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn a_clean_worktree_shows_its_recent_commits() {
    let mut app = fixture_app();
    select(&mut app, "chore/clean");
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn the_confirmation_dialog_for_a_clean_worktree() {
    let mut app = fixture_app();
    select(&mut app, "chore/clean");
    app.ask_remove(false);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn the_confirmation_dialog_for_uncommitted_work() {
    let mut app = fixture_app();
    select(&mut app, "feat/dirty");
    app.ask_remove(true);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn the_help_overlay() {
    let mut app = fixture_app();
    app.help = true;
    insta::assert_snapshot!(render(&mut app, 120, 30));
}

#[test]
fn the_safe_lens_hides_everything_with_something_in_it() {
    let mut app = fixture_app();
    app.lens = Lens::Safe;
    app.rebuild(None);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn sorted_by_risk() {
    let mut app = fixture_app();
    app.sort = Sort::Risk;
    app.resort();
    app.rebuild(None);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn a_folded_repository() {
    let mut app = fixture_app();
    app.collapse_all(true);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn filtering() {
    let mut app = fixture_app();
    app.filtering = true;
    app.filter = "feat".into();
    app.refilter();
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn a_locked_worktree_detail() {
    let mut app = fixture_app();
    select(&mut app, "fix/locked");
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn a_worktree_git_cannot_read() {
    let mut app = fixture_app();
    select(&mut app, "docs/gone");
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn a_detached_worktree_is_labelled_by_its_commit() {
    let mut app = fixture_app();
    let labels: Vec<String> = app.repos[0].worktrees.iter().map(|w| w.label()).collect();
    let detached = labels
        .iter()
        .find(|l| l.starts_with('('))
        .expect("the fixture has a detached worktree");
    select(&mut app, detached);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn the_colours_of_the_list() {
    let mut app = fixture_app();
    insta::assert_snapshot!(render_palette(&mut app, 120, 20));
}

#[test]
fn the_selected_row_is_the_only_one_highlighted() {
    let mut app = fixture_app();
    select(&mut app, "feat/dirty");
    insta::assert_snapshot!(render_palette(&mut app, 120, 20));
}

#[test]
fn a_marked_worktree() {
    let mut app = fixture_app();
    select(&mut app, "chore/clean");
    app.toggle_mark();
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn the_footer_totals_what_the_marking_would_cost() {
    let mut app = fixture_app();
    for branch in ["feat/dirty", "feat/ahead", "chore/clean"] {
        select(&mut app, branch);
        app.toggle_mark();
    }
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

#[test]
fn the_colours_of_a_marking() {
    let mut app = fixture_app();
    select(&mut app, "feat/dirty");
    app.toggle_mark();
    insta::assert_snapshot!(render_palette(&mut app, 120, 20));
}

#[test]
fn a_dialog_opened_while_a_marking_is_held_says_which_it_means() {
    let mut app = fixture_app();
    for branch in ["feat/dirty", "feat/ahead"] {
        select(&mut app, branch);
        app.toggle_mark();
    }
    select(&mut app, "chore/clean");
    app.ask_remove(false);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}
