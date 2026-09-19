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
fn the_list_at_sixty_columns() {
    let mut app = fixture_app();
    insta::assert_snapshot!(render(&mut app, 60, 20));
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

/// `d` with a marking held acts on the marking, not on the cursor — the cursor
/// here is deliberately on a worktree that is not marked.
#[test]
fn removing_a_marking_lists_every_worktree_worst_first() {
    let mut app = fixture_app();
    for branch in ["chore/clean", "feat/ahead", "feat/dirty"] {
        select(&mut app, branch);
        app.toggle_mark();
    }
    select(&mut app, "main");
    app.ask_remove(false);
    insta::assert_snapshot!(render(&mut app, 120, 24));
}

#[test]
fn removing_a_marking_with_branches_says_the_commits_go_too() {
    let mut app = fixture_app();
    for branch in ["feat/ahead", "feat/dirty"] {
        select(&mut app, branch);
        app.toggle_mark();
    }
    app.ask_remove(true);
    insta::assert_snapshot!(render(&mut app, 120, 24));
}

#[test]
fn a_marking_of_only_unremovable_worktrees_explains_itself() {
    let mut app = fixture_app();
    select(&mut app, "fix/locked");
    app.toggle_mark();
    app.ask_remove(false);
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

/// The list pane only, so a match in the detail pane cannot stand in for a row
/// that was never drawn.
fn list_pane(screen: &str, width: usize) -> String {
    screen
        .lines()
        .map(|line| {
            let cut = line
                .char_indices()
                .nth(width)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            line[..cut].to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The header row is carved out of the pane, so every row count has to be taken
/// after it. Taken before, the list is one row too long: the bottom row is
/// clipped, and a cursor sitting on it disappears.
#[test]
fn the_selected_row_is_visible_at_every_height() {
    for height in 7..16u16 {
        let mut app = fixture_app();
        app.go(usize::MAX);
        let Some(caligula::app::Row::Worktree { repo, wt }) = app.current() else {
            panic!("the last row should be a worktree");
        };
        let label = app.repos[repo].worktrees[wt].label();
        // Wide enough that a branch name is not truncated: this is a test about
        // height, and a truncated label would fail it for the wrong reason.
        let screen = render(&mut app, 100, height);
        let list = list_pane(&screen, 41);
        assert!(
            list.contains(&label),
            "at height {height} the selected row {label} was not in the list:\n{screen}"
        );
    }
}

/// The scrollbar is drawn into the rows area, not the whole pane. Given the
/// pane it writes one cell past the bottom, and since an off-thumb cell is a
/// blank, that shows up as a hole punched in the block's bottom border.
#[test]
fn the_scrollbar_stays_inside_the_frame() {
    for height in 7..16u16 {
        let mut app = fixture_app();
        let screen = render(&mut app, 60, height);
        let bottom = list_pane(&screen, 33)
            .lines()
            .find(|l| l.contains('\u{256f}') || l.contains('\u{2570}'))
            .map(str::to_string)
            .unwrap_or_default();
        let run: String = bottom.chars().filter(|c| *c != '\u{2570}').collect();
        assert!(
            run.chars().all(|c| c == '\u{2500}'),
            "at height {height} something broke the bottom border: {bottom:?}"
        );
    }
}

#[test]
fn a_swept_repository() {
    let mut app = fixture_app();
    app.go(0);
    app.sweep_repo();
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

/// Filter, then sweep: the path that produced the bug. The filter must name
/// something that *has* safe worktrees, or the sweep marks nothing and the test
/// passes on a marking it did not make.
#[test]
fn a_marking_swept_under_a_filter() {
    let mut app = fixture_app();
    app.filter = "chore".into();
    app.refilter();
    app.go(0);
    app.sweep_repo();
    assert_eq!(
        app.marked.len(),
        1,
        "the sweep itself must make the marking"
    );
    insta::assert_snapshot!(render(&mut app, 120, 20));
}

/// And it has to survive a terminal narrow enough to clip the line.
#[test]
fn a_marking_under_a_filter_on_a_narrow_terminal() {
    let mut app = fixture_app();
    app.filter = "chore".into();
    app.refilter();
    app.go(0);
    app.sweep_repo();
    insta::assert_snapshot!(render(&mut app, 80, 20));
}

/// A row wider than its pane wraps, which strands the age on the next line.
///
/// Asserted by looking for each worktree's age on the *same line* as its label:
/// an earlier version of this test parsed the screen by splitting on the pane
/// borders, landed on the empty string between them, and passed unconditionally
/// at widths that demonstrably wrapped.
#[test]
fn no_detail_row_wraps_at_any_width() {
    for width in [40u16, 50, 60, 80, 120, 200] {
        let mut app = fixture_app();
        app.go(0);
        let rows: Vec<(String, String)> = app.repos[0]
            .worktrees
            .iter()
            .map(|w| (w.label(), w.age_label(fixture::NOW)))
            .collect();
        let screen = render(&mut app, width, 30);

        for (label, age) in rows {
            // A label can be truncated in a narrow pane; its head cannot.
            let head: String = label.chars().take(6).collect();
            let Some(line) = screen.lines().find(|l| {
                l.contains(&head) && (l.contains(" ◆ ") || l.contains(" ● ") || l.contains(" ✗ "))
            }) else {
                continue;
            };
            assert!(
                line.contains(&age),
                "at width {width} the row for {label} lost its age to a wrap:\n{line}"
            );
        }
    }
}

#[test]
fn a_repository_detail_at_eighty_columns() {
    let mut app = fixture_app();
    app.go(0);
    insta::assert_snapshot!(render(&mut app, 80, 24));
}

#[test]
fn a_failure_is_shown_in_full() {
    let mut app = fixture_app();
    app.fail(
        "Could not remove the worktree",
        "git -C ~/Code/quarry worktree remove ~/Code/.worktrees/quarry/feat/x".into(),
        "fatal: '/Users/x/Code/.worktrees/quarry/feat/x' contains modified or \
untracked files, use --force to delete it"
            .into(),
    );
    insta::assert_snapshot!(render(&mut app, 120, 20));
}
