//! Rendering. Everything here reads state and draws; nothing here mutates it,
//! apart from the list viewport offset, which only rendering can know.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, Lens, Row, Tone};
use crate::git::{self, Repo, Salvage, Staleness, Worktree};
use crate::scan::shorten_home;
use crate::text::{clip, truncate};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const TEXT: Color = Color::Gray;
const BRIGHT: Color = Color::White;

fn tone_color(t: Tone) -> Color {
    match t {
        Tone::Info => TEXT,
        Tone::Good => Color::Green,
        Tone::Warn => Color::Yellow,
        Tone::Bad => Color::Red,
    }
}

fn stale_color(s: Staleness) -> Color {
    match s {
        Staleness::Active => Color::Green,
        Staleness::Recent => Color::Cyan,
        Staleness::Stale => Color::Yellow,
        Staleness::Ancient => Color::Red,
    }
}

fn salvage_color(s: Salvage) -> Color {
    match s {
        Salvage::Nothing => Color::Green,
        Salvage::Commits => Color::Yellow,
        Salvage::Uncommitted => Color::Red,
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    header(f, chunks[0], app);

    let list_width = (chunks[1].width as f32 * 0.42).round().clamp(34.0, 60.0) as u16;
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(list_width.min(chunks[1].width)),
            Constraint::Min(20),
        ])
        .split(chunks[1]);

    list(f, body[0], app);
    detail(f, body[1], app);
    footer(f, chunks[2], app);

    if app.help {
        help(f, f.area());
    }
    if app.confirm.is_some() {
        confirm(f, f.area(), app);
    }
}

// ------------------------------------------------------------------- header

fn header(f: &mut Frame, area: Rect, app: &App) {
    let t = app.totals();
    let mut spans = vec![
        Span::styled(
            " caligula ",
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{} repo{}", t.repos, if t.repos == 1 { "" } else { "s" }),
            Style::default().fg(BRIGHT),
        ),
        Span::styled(" · ", Style::default().fg(DIM)),
        Span::styled(
            format!("{} worktrees", t.worktrees),
            Style::default().fg(TEXT),
        ),
        Span::styled(format!(" ({} linked)", t.linked), Style::default().fg(DIM)),
    ];
    if t.dirty > 0 {
        spans.push(Span::styled(" · ", Style::default().fg(DIM)));
        spans.push(Span::styled(
            format!("{} dirty", t.dirty),
            Style::default().fg(Color::Red),
        ));
    }
    if t.stale > 0 {
        spans.push(Span::styled(" · ", Style::default().fg(DIM)));
        spans.push(Span::styled(
            format!("{} stale", t.stale),
            Style::default().fg(Color::Yellow),
        ));
    }
    if t.safe > 0 {
        spans.push(Span::styled(" · ", Style::default().fg(DIM)));
        spans.push(Span::styled(
            format!("{} safe to remove", t.safe),
            Style::default().fg(Color::Green),
        ));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);

    if app.scanning {
        const SPIN: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        let phase = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() / 90)
            .unwrap_or(0)) as usize;
        let text = format!("{} scanning {} dirs ", SPIN[phase % 8], app.dirs_seen);
        let w = text.chars().count() as u16;
        if area.width > w {
            let right = Rect {
                x: area.right() - w,
                y: area.y,
                width: w,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(Span::styled(text, Style::default().fg(ACCENT))),
                right,
            );
        }
    }
}

// --------------------------------------------------------------------- list

fn list(f: &mut Frame, area: Rect, app: &mut App) {
    let title = format!(" sort {} · lens {} ", app.sort.label(), app.lens.label());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(title, Style::default().fg(ACCENT)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.rows.is_empty() {
        let msg = if app.scanning {
            "scanning…"
        } else if app.lens != Lens::All || !app.filter.is_empty() {
            "nothing matches this filter"
        } else {
            "no repositories found in the scan roots"
        };
        f.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(DIM)))
                .alignment(Alignment::Center),
            inner.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
        return;
    }

    let height = inner.height as usize;
    // Keep one row of slack above the cursor: the top line is spoken for by the
    // sticky repo header whenever a group is scrolled into.
    if app.selected <= app.offset {
        app.offset = app.selected.saturating_sub(1);
    } else if height > 0 && app.selected >= app.offset + height {
        app.offset = app.selected + 1 - height;
    }
    if app.offset + height > app.rows.len() {
        app.offset = app.rows.len().saturating_sub(height);
    }

    // The last column belongs to the scrollbar, with a blank column before it so
    // the age never sits against the thumb.
    let text = Rect {
        width: inner.width.saturating_sub(2),
        ..inner
    };
    let mut lines = Vec::with_capacity(height);
    for (i, row) in app.rows.iter().enumerate().skip(app.offset).take(height) {
        let selected = i == app.selected;
        lines.push(match *row {
            Row::Repo { repo } => repo_line(&app.repos[repo], app, selected, text.width),
            Row::Worktree { repo, wt } => {
                let worktree = &app.repos[repo].worktrees[wt];
                worktree_line(
                    worktree,
                    app.now,
                    selected,
                    app.is_marked(worktree),
                    text.width,
                )
            }
        });
    }
    f.render_widget(Paragraph::new(lines), text);

    if let (true, Some(Row::Worktree { repo, .. })) = (app.offset > 0, app.rows.get(app.offset)) {
        let head = Rect { height: 1, ..text };
        f.render_widget(Clear, head);
        f.render_widget(
            Paragraph::new(repo_line(&app.repos[*repo], app, false, text.width)),
            head,
        );
    }

    if app.rows.len() > height && height > 0 {
        scrollbar(f, inner, app.offset, height, app.rows.len());
    }
}

fn scrollbar(f: &mut Frame, area: Rect, offset: usize, height: usize, total: usize) {
    let bar_x = area.right().saturating_sub(1);
    let thumb = ((height * height) / total).max(1);
    let pos = if total > height {
        (offset * (height - thumb)) / (total - height)
    } else {
        0
    };
    for y in 0..height {
        let on = y >= pos && y < pos + thumb;
        let cell = Rect {
            x: bar_x,
            y: area.y + y as u16,
            width: 1,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Span::styled(
                if on { "│" } else { " " },
                Style::default().fg(if on { ACCENT } else { DIM }),
            )),
            cell,
        );
    }
}

fn repo_line<'a>(repo: &'a Repo, app: &App, selected: bool, width: u16) -> Line<'a> {
    // "▾" with a space either side.
    const PREFIX: usize = 3;

    let collapsed = app.collapsed.contains(&repo.common_dir);
    let arrow = if collapsed { "▸" } else { "▾" };

    let linked = repo.linked_count();
    let dirty = repo.dirty_count();
    let mut right = String::from(" ");
    if linked > 0 {
        right.push_str(&format!("{linked}wt "));
    }
    if dirty > 0 {
        right.push_str(&format!("{dirty}● "));
    }

    let width = width as usize;
    let avail = width.saturating_sub(PREFIX);
    if right.chars().count() + 4 > avail {
        right = String::new();
    }
    let name_w = avail.saturating_sub(right.chars().count());
    let name = truncate(&repo.name, name_w);
    let pad = name_w.saturating_sub(name.chars().count());

    let base = row_style(selected);
    Line::from(vec![
        Span::styled(
            format!(" {arrow} "),
            base.fg(if selected { ACCENT } else { DIM }),
        ),
        Span::styled(name, base.fg(BRIGHT).add_modifier(Modifier::BOLD)),
        Span::styled(" ".repeat(pad), base),
        Span::styled(right, base.fg(if dirty > 0 { Color::Red } else { DIM })),
    ])
}

fn worktree_line<'a>(
    wt: &'a Worktree,
    now: u64,
    selected: bool,
    marked: bool,
    width: u16,
) -> Line<'a> {
    // mark gutter + "│ " + marker + " "
    const PREFIX: usize = 5;
    const AGE: usize = 5;
    const MIN_LABEL: usize = 6;

    let marker = if wt.broken {
        ("✗", Color::Red)
    } else if wt.is_main {
        ("◆", stale_color(wt.staleness(now)))
    } else {
        ("●", stale_color(wt.staleness(now)))
    };

    // Lay the row out from the fixed columns inwards so it is exactly as wide as
    // the pane: a row that overflows loses its age off the right-hand edge.
    let width = width as usize;
    let body = width.saturating_sub(PREFIX + AGE);
    let mut badges = badge_text(wt);
    if badges.chars().count() + MIN_LABEL > body {
        badges = clip(&badges, body.saturating_sub(MIN_LABEL));
    }
    let label_w = body.saturating_sub(badges.chars().count());
    let label = truncate(&wt.label(), label_w);
    let pad = label_w.saturating_sub(label.chars().count());

    let base = row_style(selected);
    let label_style = if wt.is_main {
        base.fg(TEXT).add_modifier(Modifier::ITALIC)
    } else if wt.salvage() == Salvage::Uncommitted {
        base.fg(BRIGHT)
    } else {
        base.fg(TEXT)
    };

    // A solid block in the first column, not a shade: the marking decides what a
    // removal applies to, so it has to be readable at a glance and on a terminal
    // with no colour at all.
    let (gutter, gutter_style) = if marked {
        ("▌", base.fg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        (" ", base.fg(DIM))
    };

    Line::from(vec![
        Span::styled(gutter, gutter_style),
        Span::styled("│ ", base.fg(DIM)),
        Span::styled(marker.0, base.fg(marker.1)),
        Span::styled(" ", base),
        Span::styled(label, label_style),
        Span::styled(" ".repeat(pad), base),
        Span::styled(badges, base.fg(salvage_color(wt.salvage()))),
        Span::styled(
            format!("{:>AGE$}", wt.age_label(now)),
            base.fg(stale_color(wt.staleness(now))),
        ),
    ])
}

fn row_style(selected: bool) -> Style {
    if selected {
        Style::default().bg(Color::Rgb(40, 44, 52))
    } else {
        Style::default()
    }
}

/// Compact right-hand markers: what is here that is not anywhere else.
fn badge_text(wt: &Worktree) -> String {
    let mut s = String::new();
    if wt.locked.is_some() {
        s.push_str("L ");
    }
    if wt.prunable.is_some() || wt.broken {
        s.push_str("! ");
    }
    if wt.unpushed() > 0 {
        s.push_str(&format!("↑{} ", wt.unpushed()));
    }
    if wt.behind > 0 {
        s.push_str(&format!("↓{} ", wt.behind));
    }
    if wt.changed_files() > 0 {
        s.push_str(&format!("~{} ", wt.changed_files()));
    }
    s
}

// ------------------------------------------------------------------- detail

fn detail(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    f.render_widget(block, area);

    let lines = match app.current() {
        Some(Row::Worktree { repo, wt }) => worktree_detail(
            &app.repos[repo],
            &app.repos[repo].worktrees[wt],
            app.now,
            inner.width,
        ),
        Some(Row::Repo { repo }) => repo_detail(&app.repos[repo], app.now),
        None => vec![Line::from(Span::styled(
            "nothing selected",
            Style::default().fg(DIM),
        ))],
    };

    let max_scroll = (lines.len() as u16).saturating_sub(inner.height);
    app.detail_scroll = app.detail_scroll.min(max_scroll);
    f.render_widget(
        Paragraph::new(lines)
            .scroll((app.detail_scroll, 0))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn kv<'a>(key: &'a str, value: impl Into<String>, color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{key:<10}"), Style::default().fg(DIM)),
        Span::styled(value.into(), Style::default().fg(color)),
    ])
}

fn section(title: impl Into<String>) -> Vec<Line<'static>> {
    vec![
        Line::raw(""),
        Line::from(Span::styled(
            title.into(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
    ]
}

fn worktree_detail<'a>(repo: &'a Repo, wt: &'a Worktree, now: u64, width: u16) -> Vec<Line<'a>> {
    let mut out = Vec::new();
    let stale = wt.staleness(now);

    out.push(Line::from(vec![
        Span::styled(
            wt.label(),
            Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} · last touched {}", stale.label(), wt.age_label(now)),
            Style::default().fg(stale_color(stale)),
        ),
        if wt.is_main {
            Span::styled("  main checkout", Style::default().fg(DIM))
        } else {
            Span::raw("")
        },
    ]));
    out.push(Line::from(Span::styled(
        shorten_home(&wt.path),
        Style::default().fg(DIM),
    )));

    // The verdict is the whole point of the tool: say what is at stake.
    let color = salvage_color(wt.salvage());
    out.push(Line::raw(""));
    out.push(Line::from(vec![
        Span::styled("┃ ", Style::default().fg(color)),
        Span::styled(
            wt.verdict(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ]));
    if wt.broken {
        out.push(Line::from(vec![
            Span::styled("┃ ", Style::default().fg(Color::Red)),
            Span::styled(
                "git cannot read this worktree — it may be gone from disk",
                Style::default().fg(Color::Red),
            ),
        ]));
    }
    out.push(Line::raw(""));

    let tracking = match (&wt.upstream, wt.ahead, wt.behind) {
        (Some(up), 0, 0) => format!("→ {up}  in sync"),
        (Some(up), a, b) => format!("→ {up}  ↑{a} ↓{b}"),
        (None, _, _) => match &repo.default_base {
            Some(base) => format!("no upstream · {} commits off {base}", wt.unmerged_total),
            None => "no upstream".to_string(),
        },
    };
    out.push(kv("branch", format!("{}  {tracking}", wt.label()), TEXT));
    if let Some(c) = &wt.last_commit {
        out.push(kv(
            "head",
            format!(
                "{}  {}",
                git::short_sha(&c.sha),
                clip(&c.subject, width.saturating_sub(22) as usize)
            ),
            TEXT,
        ));
        out.push(kv(
            "",
            format!("{} · {}", c.author, git::stamp(c.time, now)),
            DIM,
        ));
    }
    out.push(kv(
        "repo",
        format!("{}  {}", repo.name, shorten_home(&repo.root)),
        TEXT,
    ));
    if repo.stashes > 0 {
        out.push(kv(
            "stash",
            format!(
                "{} entr{} (shared across this repo)",
                repo.stashes,
                if repo.stashes == 1 { "y" } else { "ies" }
            ),
            Color::Yellow,
        ));
    }
    if let Some(reason) = &wt.locked {
        out.push(kv("locked", reason.clone(), Color::Yellow));
    }
    if let Some(reason) = &wt.prunable {
        out.push(kv("prunable", reason.clone(), Color::Yellow));
    }

    if wt.changed_files() > 0 {
        out.extend(section(format!(
            "Working tree ({} staged · {} unstaged · {} untracked{})",
            wt.staged,
            wt.unstaged,
            wt.untracked,
            if wt.conflicts > 0 {
                format!(" · {} conflicted", wt.conflicts)
            } else {
                String::new()
            }
        )));
        for fc in wt.files.iter().take(40) {
            let color = match fc.code.as_str() {
                "??" => Color::Blue,
                "UU" => Color::Red,
                c if c.starts_with('.') => Color::Yellow,
                _ => Color::Green,
            };
            out.push(Line::from(vec![
                Span::styled(format!("  {:<3}", fc.code), Style::default().fg(color)),
                Span::styled(
                    truncate(&fc.path, width.saturating_sub(6) as usize),
                    Style::default().fg(TEXT),
                ),
            ]));
        }
        if wt.files.len() > 40 {
            out.push(Line::from(Span::styled(
                format!("  … {} more", wt.files.len() - 40),
                Style::default().fg(DIM),
            )));
        }
    }

    if !wt.unmerged.is_empty() {
        let where_ = if wt.upstream.is_some() {
            "Unpushed"
        } else {
            "Off-base"
        };
        out.extend(section(format!("{where_} commits ({})", wt.unmerged_total)));
        for c in wt.unmerged.iter().take(20) {
            out.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", git::short_sha(&c.sha)),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("{:>4}  ", git::ago(now.saturating_sub(c.time))),
                    Style::default().fg(DIM),
                ),
                Span::styled(
                    clip(&c.subject, width.saturating_sub(20) as usize),
                    Style::default().fg(TEXT),
                ),
            ]));
        }
        if wt.unmerged_total as usize > wt.unmerged.len() {
            out.push(Line::from(Span::styled(
                format!(
                    "  … {} more",
                    wt.unmerged_total as usize - wt.unmerged.len()
                ),
                Style::default().fg(DIM),
            )));
        }
    }

    if wt.unmerged.is_empty() && !wt.recent.is_empty() {
        out.extend(section("Recent commits"));
        for c in wt.recent.iter().take(5) {
            out.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", git::short_sha(&c.sha)),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("{:>4}  ", git::ago(now.saturating_sub(c.time))),
                    Style::default().fg(DIM),
                ),
                Span::styled(
                    clip(&c.subject, width.saturating_sub(20) as usize),
                    Style::default().fg(DIM),
                ),
            ]));
        }
    }

    out
}

fn repo_detail<'a>(repo: &'a Repo, now: u64) -> Vec<Line<'a>> {
    let mut out = vec![
        Line::from(Span::styled(
            repo.name.clone(),
            Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            shorten_home(&repo.root),
            Style::default().fg(DIM),
        )),
        Line::raw(""),
    ];
    if let Some(remote) = &repo.remote {
        out.push(kv("origin", remote.clone(), TEXT));
    }
    if let Some(base) = &repo.default_base {
        out.push(kv("base", base.clone(), TEXT));
    }
    out.push(kv(
        "worktrees",
        format!("{} ({} linked)", repo.worktrees.len(), repo.linked_count()),
        TEXT,
    ));
    let dirty = repo.dirty_count();
    out.push(kv(
        "dirty",
        dirty.to_string(),
        if dirty > 0 { Color::Red } else { Color::Green },
    ));
    if repo.stashes > 0 {
        out.push(kv("stash", repo.stashes.to_string(), Color::Yellow));
    }

    out.extend(section("Worktrees"));
    for wt in &repo.worktrees {
        let stale = wt.staleness(now);
        out.push(Line::from(vec![
            Span::styled(
                format!("  {} ", if wt.is_main { "◆" } else { "●" }),
                Style::default().fg(stale_color(stale)),
            ),
            Span::styled(
                format!("{:<26}  ", truncate(&wt.label(), 26)),
                Style::default().fg(TEXT),
            ),
            Span::styled(
                format!("{:<34}  ", truncate(&wt.verdict(), 34)),
                Style::default().fg(salvage_color(wt.salvage())),
            ),
            Span::styled(wt.age_label(now), Style::default().fg(stale_color(stale))),
        ]));
    }
    out
}

// ------------------------------------------------------------------- footer

fn footer(f: &mut Frame, area: Rect, app: &App) {
    if app.filtering || !app.filter.is_empty() {
        let mut spans = vec![
            Span::styled(
                " filter ",
                Style::default().fg(Color::Black).bg(Color::Yellow),
            ),
            Span::raw(" "),
            Span::styled(app.filter.clone(), Style::default().fg(BRIGHT)),
        ];
        if app.filtering {
            spans.push(Span::styled("▌", Style::default().fg(ACCENT)));
            spans.push(Span::styled(
                "   enter accept · esc clear",
                Style::default().fg(DIM),
            ));
        } else {
            spans.push(Span::styled("   esc clear", Style::default().fg(DIM)));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    let stakes = app.marked_stakes();
    if stakes.worktrees > 0 {
        let mut spans = vec![
            Span::styled(
                format!(" {} marked ", stakes.worktrees),
                Style::default()
                    .fg(Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ];
        if stakes.files > 0 {
            spans.push(Span::styled(
                format!(
                    "{} uncommitted file{}",
                    stakes.files,
                    if stakes.files == 1 { "" } else { "s" }
                ),
                Style::default().fg(Color::Red),
            ));
        }
        if stakes.commits > 0 {
            if stakes.files > 0 {
                spans.push(Span::styled(" · ", Style::default().fg(DIM)));
            }
            spans.push(Span::styled(
                format!(
                    "{} commit{} only there",
                    stakes.commits,
                    if stakes.commits == 1 { "" } else { "s" }
                ),
                Style::default().fg(Color::Yellow),
            ));
        }
        if stakes.files == 0 && stakes.commits == 0 {
            spans.push(Span::styled(
                "nothing to salvage in any of them",
                Style::default().fg(Color::Green),
            ));
        }
        spans.push(Span::styled(
            "   esc clears · d still removes the row under the cursor",
            Style::default().fg(DIM),
        ));
        // A message is appended rather than allowed to replace this: what is
        // marked decides what a removal applies to, and must not vanish for six
        // seconds because a sort was cycled.
        if let Some((msg, tone, at)) = &app.status
            && at.elapsed().as_secs() < 6
        {
            spans.push(Span::styled(
                format!("   {msg}"),
                Style::default().fg(tone_color(*tone)),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    if let Some((msg, tone, at)) = &app.status
        && at.elapsed().as_secs() < 6
    {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(msg.clone(), Style::default().fg(tone_color(*tone))),
            ])),
            area,
        );
        return;
    }

    let keys = [
        ("j/k", "move"),
        ("space", "mark"),
        ("←/→", "fold"),
        ("d", "remove"),
        ("D", "+branch"),
        ("p", "prune"),
        ("c", "shell"),
        ("f", "lens"),
        ("s", "sort"),
        ("/", "find"),
        ("?", "help"),
    ];
    let mut spans = vec![Span::raw(" ")];
    for (k, label) in keys {
        spans.push(Span::styled(k, Style::default().fg(ACCENT)));
        spans.push(Span::styled(
            format!(" {label}  "),
            Style::default().fg(DIM),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ------------------------------------------------------------------ overlays

/// How many rows this line takes once wrapped, breaking on whitespace the way
/// `Paragraph` does. Counting characters instead undercounts, because a word
/// too long for the remaining space moves down whole.
fn wrapped_rows(text: &str, width: usize) -> usize {
    if width == 0 {
        return 1;
    }
    let mut rows = 1;
    let mut used = 0;
    for word in text.split_whitespace() {
        let w = word.chars().count();
        let needed = if used == 0 { w } else { w + 1 };
        if used + needed > width && used > 0 {
            rows += 1;
            used = w.min(width);
        } else {
            used += needed;
        }
        // A word longer than the line wraps within itself.
        if w > width {
            rows += (w - 1) / width;
            used = w % width;
        }
    }
    rows
}

fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(4));
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

fn help(f: &mut Frame, area: Rect) {
    let rows: &[(&str, &str)] = &[
        ("j / k · ↓ / ↑", "move"),
        ("space", "mark a worktree, and step down"),
        ("J / K", "jump to next / previous repo"),
        ("g / G", "first / last row"),
        ("← / → · enter", "fold or unfold a repo"),
        ("z / Z", "fold all / unfold all"),
        ("esc", "clear the marking, then the filter"),
        ("PgUp / PgDn", "scroll the detail pane"),
        ("", ""),
        ("d", "remove the marking, or the row under the cursor"),
        ("D", "the same, and delete the branches too"),
        ("p", "prune the repo's stale worktree records"),
        ("L", "lock or unlock the worktree"),
        ("", ""),
        ("c", "drop into a shell in the worktree"),
        ("o", "open it in the file manager"),
        ("y", "copy its path to the clipboard"),
        ("", ""),
        ("f", "cycle lens: all → dirty → salvageable → stale → safe"),
        ("s", "cycle sort: activity → name → risk"),
        ("/", "filter by repo, branch or path"),
        ("r", "re-probe the selected repo"),
        ("R", "rescan the disk from scratch"),
        ("q", "quit"),
    ];

    let area = popup(area, 78, rows.len() as u16 + 4);
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(
            " keys ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    f.render_widget(block, area);

    let lines: Vec<Line> = rows
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k:<15}"), Style::default().fg(ACCENT)),
                Span::styled(*v, Style::default().fg(TEXT)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn confirm(f: &mut Frame, area: Rect, app: &App) {
    let Some(c) = &app.confirm else { return };

    // Body lines wrap, so counting them is not counting rows — and the popup is
    // narrowed on a small terminal, so the width has to be settled before the
    // height can be. Getting this wrong clips the body.
    let width = 72.min(area.width.saturating_sub(4)).max(8);
    let text_width = width.saturating_sub(4).max(1) as usize;
    let rows: usize = c
        .body
        .iter()
        .map(|(text, _)| wrapped_rows(text, text_width))
        .sum();
    let height = (rows as u16).saturating_add(5);

    let area = popup(area, width, height);
    f.render_widget(Clear, area);

    let danger = c.body.iter().any(|(_, t)| *t == Tone::Bad);
    let border = if danger { Color::Red } else { Color::Yellow };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            format!(" {} ", c.title),
            Style::default().fg(border).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    f.render_widget(block, area);

    // The answer row is placed, not appended: however the body wraps or is
    // clipped, "how do I say no" stays on screen.
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let lines: Vec<Line> = c
        .body
        .iter()
        .map(|(text, tone)| {
            Line::from(Span::styled(
                text.clone(),
                Style::default().fg(tone_color(*tone)),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), split[0]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " y ",
                Style::default()
                    .fg(Color::Black)
                    .bg(border)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" do it    ", Style::default().fg(TEXT)),
            Span::styled(" n ", Style::default().fg(Color::Black).bg(Color::Gray)),
            Span::styled(" cancel", Style::default().fg(TEXT)),
        ])),
        split[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{now, test_worktree};

    /// The age column once fell off the right-hand edge: the row was built one
    /// column wider than the pane, and the overflow was silently truncated.
    #[test]
    fn wrapping_is_counted_by_words_not_characters() {
        assert_eq!(wrapped_rows("", 10), 1);
        assert_eq!(wrapped_rows("short", 10), 1);
        // Breaking on whitespace: "a bb ccc" fits, one more word does not.
        assert_eq!(wrapped_rows("aaaa bbbb", 9), 1);
        assert_eq!(wrapped_rows("aaaa bbbb", 8), 2);
        // A word longer than the line wraps inside itself.
        assert_eq!(wrapped_rows("aaaaaaaaaaaa", 5), 3);
    }

    #[test]
    fn worktree_rows_are_exactly_as_wide_as_the_pane() {
        for width in [20u16, 30, 42, 58, 80, 120] {
            for (ahead, behind, dirty) in [(0, 0, 0), (2, 0, 0), (0, 0, 7), (115, 124, 24)] {
                let mut wt = test_worktree("refactor/site-tailwind-shadcn-and-more");
                wt.upstream = Some("origin/main".into());
                wt.ahead = ahead;
                wt.behind = behind;
                wt.untracked = dirty;
                let line = worktree_line(&wt, now(), false, false, width);
                assert_eq!(
                    line.width(),
                    width as usize,
                    "width {width}, badges {ahead}/{behind}/{dirty}"
                );
                let age = line.spans.last().expect("age span").content.to_string();
                assert!(
                    age.ends_with(&wt.age_label(now())),
                    "age column lost: {age:?}"
                );
            }
        }
    }

    #[test]
    fn repo_rows_are_exactly_as_wide_as_the_pane() {
        let app = App::new();
        for width in [20u16, 34, 58, 120] {
            let repo = crate::git::test_repo("a-repository-with-a-long-name", 3);
            let line = repo_line(&repo, &app, false, width);
            assert_eq!(line.width(), width as usize, "width {width}");
        }
    }
}
