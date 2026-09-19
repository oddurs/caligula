//! caligula — a terminal browser for git worktrees.

use caligula::{app, git, scan, ui};

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use app::{App, Tone};
use scan::Scan;

const USAGE: &str = "\
caligula — browse git worktrees: what they are, how stale they are,
and what you would lose by deleting them.

usage: caligula [options]

options:
  --root <path>   scan this directory (repeatable; default: ~/Code-style
                  trees if present, otherwise $HOME)
  --depth <n>     how deep to walk below each root (default 8)
  -h, --help      show this
  -V, --version   show the version
";

struct Args {
    roots: Vec<PathBuf>,
    depth: usize,
}

fn parse_args() -> Result<Args, String> {
    let mut roots = Vec::new();
    let mut depth = 8usize;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" => {
                let p = it.next().ok_or("--root needs a path")?;
                let p = PathBuf::from(p);
                if !p.is_dir() {
                    return Err(format!("not a directory: {}", p.display()));
                }
                roots.push(p);
            }
            "--depth" => {
                depth = it
                    .next()
                    .ok_or("--depth needs a number")?
                    .parse()
                    .map_err(|_| "--depth needs a number".to_string())?;
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("caligula {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if roots.is_empty() {
        roots = scan::default_roots();
    }
    Ok(Args { roots, depth })
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("caligula: {e}\n\n{USAGE}");
            std::process::exit(2);
        }
    };

    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("caligula: git is not on PATH");
        std::process::exit(1);
    }

    let mut app = App::new();
    let mut scan = scan::start(args.roots.clone(), args.depth);
    // ratatui::init installs the panic hook that restores the terminal, so do
    // not add another: verified against a real panic in 0.29, and the reasoning
    // is on cairn item 0014.
    let mut terminal = ratatui::init();

    loop {
        match run(&mut terminal, &mut app, &mut scan, &args) {
            Outcome::Quit => break,
            Outcome::Shell(path) => {
                // Hand the terminal back before spawning anything interactive.
                ratatui::restore();
                run_shell(&path);
                terminal = ratatui::init();
                terminal.clear().ok();
            }
        }
    }

    ratatui::restore();
}

enum Outcome {
    Quit,
    Shell(PathBuf),
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    scan: &mut Scan,
    args: &Args,
) -> Outcome {
    loop {
        drain_scan(app, scan);

        if terminal.draw(|f| ui::draw(f, app)).is_err() {
            return Outcome::Quit;
        }

        if event::poll(Duration::from_millis(100)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    handle_key(app, key, scan, args);
                }
                Ok(Event::Resize(_, _)) => {}
                Ok(_) => {}
                Err(_) => return Outcome::Quit,
            }
        }

        if let Some(path) = app.shell_request.take() {
            return Outcome::Shell(path);
        }
        if app.quit {
            return Outcome::Quit;
        }
    }
}

fn drain_scan(app: &mut App, scan: &mut Scan) {
    use std::sync::atomic::Ordering;
    app.dirs_seen = scan.dirs_seen.load(Ordering::Relaxed);
    // Bound the work per frame so a fast scan cannot starve the event loop.
    for _ in 0..64 {
        match scan.rx.try_recv() {
            Ok(scan::Event::Found(repo)) => app.add_repo(*repo),
            Ok(scan::Event::Progress(n)) => app.dirs_seen = n,
            Ok(scan::Event::Done) => {
                app.scanning = false;
                app.now = git::now();
                let key = app.selection_key();
                app.rebuild(key);
            }
            Err(_) => break,
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent, scan: &mut Scan, args: &Args) {
    if app.confirm.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.run_confirmed(),
            _ => {
                app.confirm = None;
                app.say("Cancelled", Tone::Info);
            }
        }
        return;
    }

    if app.help {
        app.help = false;
        return;
    }

    if app.filtering {
        match key.code {
            KeyCode::Esc => {
                app.filtering = false;
                app.filter.clear();
                let keep = app.selection_key();
                app.rebuild(keep);
            }
            KeyCode::Enter => app.filtering = false,
            KeyCode::Backspace => {
                app.filter.pop();
                app.refilter();
            }
            KeyCode::Char(c) => {
                app.filter.push(c);
                app.refilter();
            }
            _ => {}
        }
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => app.quit = true,
            KeyCode::Char('d') => app.move_by(10),
            KeyCode::Char('u') => app.move_by(-10),
            _ => {}
        }
        return;
    }

    match key.code {
        // Esc unwinds what is held, innermost first: the marking decides what a
        // removal applies to, so it must never survive a keystroke meant to
        // cancel something else.
        KeyCode::Esc if !app.marked.is_empty() => {
            app.clear_marks();
            app.say("Marking cleared", Tone::Info);
        }
        KeyCode::Char(' ') => app.toggle_mark(),
        KeyCode::Char('a') => app.sweep_repo(),
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.filter.is_empty() {
                app.quit = true;
            } else {
                app.filter.clear();
                let keep = app.selection_key();
                app.rebuild(keep);
            }
        }
        KeyCode::Char('j') | KeyCode::Down => app.move_by(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_by(-1),
        KeyCode::Char('J') => app.next_repo(true),
        KeyCode::Char('K') => app.next_repo(false),
        KeyCode::Char('g') | KeyCode::Home => app.go(0),
        KeyCode::Char('G') | KeyCode::End => app.go(usize::MAX),
        KeyCode::Left
        | KeyCode::Right
        | KeyCode::Enter
        | KeyCode::Char('h')
        | KeyCode::Char('l') => app.toggle_collapse(),
        KeyCode::Char('z') => app.collapse_all(true),
        KeyCode::Char('Z') => app.collapse_all(false),
        KeyCode::PageDown => app.detail_scroll = app.detail_scroll.saturating_add(10),
        KeyCode::PageUp => app.detail_scroll = app.detail_scroll.saturating_sub(10),

        KeyCode::Char('d') => app.ask_remove(false),
        KeyCode::Char('D') => app.ask_remove(true),
        KeyCode::Char('p') => app.ask_prune(),
        KeyCode::Char('L') => app.toggle_lock(),

        KeyCode::Char('c') => app.request_shell(),
        KeyCode::Char('o') => app.reveal(),
        KeyCode::Char('y') => app.copy_path(),

        KeyCode::Char('f') => {
            app.lens = app.lens.next();
            let keep = app.selection_key();
            app.rebuild(keep);
            let lens = app.lens.label();
            app.say(format!("Lens: {lens}"), Tone::Info);
        }
        KeyCode::Char('s') => {
            app.sort = app.sort.next();
            let keep = app.selection_key();
            app.resort();
            app.rebuild(keep);
            let sort = app.sort.label();
            app.say(format!("Sort: {sort}"), Tone::Info);
        }
        KeyCode::Char('/') => {
            app.filtering = true;
            app.filter.clear();
        }
        KeyCode::Char('r') => app.refresh_current(),
        KeyCode::Char('R') => {
            app.repos.clear();
            app.rows.clear();
            app.clear_marks();
            app.selected = 0;
            app.offset = 0;
            app.scanning = true;
            app.now = git::now();
            *scan = scan::start(args.roots.clone(), args.depth);
            app.say("Rescanning", Tone::Info);
        }
        KeyCode::Char('?') => app.help = true,
        _ => {}
    }
}

/// Drop the user into their shell inside a worktree, then come back.
fn run_shell(path: &std::path::Path) {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    println!("caligula: {} — exit the shell to return", path.display());
    let _ = Command::new(shell).current_dir(path).status();
}
