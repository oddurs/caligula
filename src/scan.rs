//! Finding repositories on disk.
//!
//! One thread walks the filesystem looking for anything that contains a `.git`
//! entry; a small pool of workers turns each hit into a fully probed [`Repo`].
//! Results stream to the UI so the list fills in while the walk continues.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::git::{self, Repo};

pub enum Event {
    Found(Box<Repo>),
    Progress(usize),
    Done,
}

pub struct Scan {
    pub rx: Receiver<Event>,
    pub dirs_seen: Arc<AtomicUsize>,
}

/// Directories that never contain interesting checkouts but cost a lot to walk.
const DENY: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "venv",
    "__pycache__",
    "Library",
    "Applications",
    "Pods",
    "DerivedData",
    "dist",
    "build",
    ".next",
];

pub fn start(roots: Vec<PathBuf>, max_depth: usize) -> Scan {
    let (tx, rx) = channel();
    let dirs_seen = Arc::new(AtomicUsize::new(0));

    let (cand_tx, cand_rx) = channel::<PathBuf>();
    let cand_rx = Arc::new(Mutex::new(cand_rx));
    let seen_repos: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

    let workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 12);
    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        let cand_rx = Arc::clone(&cand_rx);
        let seen_repos = Arc::clone(&seen_repos);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            loop {
                let candidate = {
                    let guard = match cand_rx.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    match guard.recv() {
                        Ok(p) => p,
                        Err(_) => break,
                    }
                };
                let Some(common) = git::common_dir(&candidate) else {
                    continue;
                };
                {
                    // First finder of a repo owns it; the other worktrees of the
                    // same repo resolve to the same common dir and are dropped.
                    let mut seen = match seen_repos.lock() {
                        Ok(s) => s,
                        Err(_) => break,
                    };
                    if !seen.insert(common.clone()) {
                        continue;
                    }
                }
                if let Some(repo) = git::probe_repo(&candidate)
                    && tx.send(Event::Found(Box::new(repo))).is_err()
                {
                    break;
                }
            }
        }));
    }
    let walk_tx = tx.clone();
    let walk_seen = Arc::clone(&dirs_seen);
    thread::spawn(move || {
        walk(roots, max_depth, &cand_tx, &walk_seen, &walk_tx);
        // Dropping the candidate sender is what tells the workers to stop.
        drop(cand_tx);
    });

    // Workers finish once the queue is drained; only then is the scan done.
    thread::spawn(move || {
        for h in handles {
            let _ = h.join();
        }
        let _ = tx.send(Event::Done);
    });

    Scan { rx, dirs_seen }
}

fn walk(
    roots: Vec<PathBuf>,
    max_depth: usize,
    out: &Sender<PathBuf>,
    seen: &AtomicUsize,
    progress: &Sender<Event>,
) {
    let mut stack: Vec<(PathBuf, usize)> = roots.into_iter().map(|r| (r, 0)).collect();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut count = 0usize;

    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth {
            continue;
        }
        let real = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
        if !visited.insert(real) {
            continue;
        }
        count += 1;
        seen.store(count, Ordering::Relaxed);
        if count.is_multiple_of(200) && progress.send(Event::Progress(count)).is_err() {
            return;
        }

        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children = Vec::new();
        let mut is_repo = false;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == ".git" {
                is_repo = true;
                continue;
            }
            if skip(&name) {
                continue;
            }
            // Follow no symlinks: they are the fast route to an infinite walk.
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                children.push(entry.path());
            }
        }

        if is_repo {
            // A checkout's contents are git's business, not ours; its linked
            // worktrees live elsewhere and are found on their own.
            if out.send(dir).is_err() {
                return;
            }
            continue;
        }
        stack.extend(children.into_iter().map(|c| (c, depth + 1)));
    }
}

fn skip(name: &str) -> bool {
    // Hidden directories are noise — except the one this tool exists for.
    if name.starts_with('.') && name != ".worktrees" {
        return true;
    }
    DENY.contains(&name)
}

/// Default scan roots: `~/Code`-style trees if present, else the home directory.
pub fn default_roots() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let Some(home) = home else {
        return vec![PathBuf::from(".")];
    };
    let preferred: Vec<PathBuf> = ["Code", "code", "src", "Projects", "dev", "work", "repos"]
        .iter()
        .map(|d| home.join(d))
        .filter(|p| p.is_dir())
        .collect();
    if preferred.is_empty() {
        vec![home]
    } else {
        preferred
    }
}

pub fn shorten_home(path: &Path) -> String {
    let s = path.display().to_string();
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && s.starts_with(&home) => format!("~{}", &s[home.len()..]),
        _ => s,
    }
}
