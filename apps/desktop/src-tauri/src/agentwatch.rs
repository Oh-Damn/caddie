use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(150);

const MAX_HOLD: Duration = Duration::from_millis(750);

pub struct AgentWatch {

    _watcher: RecommendedWatcher,
}

impl AgentWatch {

    pub fn spawn<F>(paths: &[PathBuf], on_change: F) -> Option<Self>
    where
        F: Fn() + Send + 'static,
    {
        if paths.is_empty() {
            return None;
        }
        let (tx, rx) = mpsc::channel::<()>();
        let mut watcher = match notify::recommended_watcher(move |res: notify::Result<_>| {
            if res.is_ok() {

                let _ = tx.send(());
            }
        }) {
            Ok(watcher) => watcher,
            Err(err) => {
                tracing::warn!("agent watch unavailable, {err}");
                return None;
            }
        };

        let mut watched = 0usize;
        for path in paths {

            match watcher.watch(path, RecursiveMode::Recursive) {
                Ok(()) => watched += 1,
                Err(err) => tracing::debug!("not watching {}, {err}", path.display()),
            }
        }
        if watched == 0 {
            return None;
        }

        std::thread::spawn(move || debounce(rx, on_change));
        Some(Self { _watcher: watcher })
    }
}

fn debounce<F: Fn()>(rx: mpsc::Receiver<()>, on_change: F) {
    while rx.recv().is_ok() {
        let burst_started = Instant::now();

        while burst_started.elapsed() < MAX_HOLD {
            match rx.recv_timeout(DEBOUNCE) {
                Ok(()) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
        on_change();
    }
}

pub fn paths_for(enabled: &crate::providers::Enabled) -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").filter(|h| !h.is_empty()) else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    let mut out = Vec::new();
    if enabled.has(crate::providers::CLAUDE_CODE) {
        out.push(home.join(".claude/projects"));
    }
    if enabled.has(crate::providers::CURSOR) {

        out.push(home.join("Library/Application Support/Cursor/User/globalStorage"));
    }
    if enabled.has(crate::providers::CODEX) {
        out.push(home.join(".codex/sessions"));
    }
    out.retain(|p| p.exists());
    out
}

#[cfg(test)]
#[test]
#[ignore]
fn dump_agent_watch() {
    let paths = paths_for(&crate::providers::Enabled::default());
    println!("watching {} path(s):", paths.len());
    for p in &paths {
        println!("  {}", p.display());
    }
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = hits.clone();
    let started = Instant::now();
    let _watch = AgentWatch::spawn(&paths, move || {
        let n = seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        println!("  [{:>6.2}s] change #{n}", started.elapsed().as_secs_f32());
    });
    println!("go and type at Cursor or Claude Code for 20s...");
    std::thread::sleep(Duration::from_secs(20));
    println!(
        "{} change(s) in 20s",
        hits.load(std::sync::atomic::Ordering::SeqCst)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn a_burst_of_writes_reports_once() {
        let (tx, rx) = mpsc::channel();
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        let worker = std::thread::spawn(move || {
            debounce(rx, move || {
                seen.fetch_add(1, Ordering::SeqCst);
            })
        });
        for _ in 0..20 {
            tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(DEBOUNCE * 3);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        drop(tx);
        worker.join().unwrap();
    }

    #[test]
    fn separated_writes_report_separately() {
        let (tx, rx) = mpsc::channel();
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        let worker = std::thread::spawn(move || {
            debounce(rx, move || {
                seen.fetch_add(1, Ordering::SeqCst);
            })
        });
        tx.send(()).unwrap();
        std::thread::sleep(DEBOUNCE * 3);
        tx.send(()).unwrap();
        std::thread::sleep(DEBOUNCE * 3);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        drop(tx);
        worker.join().unwrap();
    }

    #[test]
    fn an_unbroken_burst_still_reports() {
        let (tx, rx) = mpsc::channel();
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        let worker = std::thread::spawn(move || {
            debounce(rx, move || {
                seen.fetch_add(1, Ordering::SeqCst);
            })
        });

        let start = Instant::now();
        while start.elapsed() < MAX_HOLD * 2 {
            tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(hits.load(Ordering::SeqCst) >= 1);
        drop(tx);
        worker.join().unwrap();
    }

    #[test]
    fn the_thread_stops_when_the_watcher_drops() {
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || debounce(rx, || {}));
        drop(tx);
        worker.join().unwrap();
    }

    #[test]
    fn a_write_in_a_watched_directory_fires() {
        let dir = std::env::temp_dir().join(format!("caddie-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        let watch = AgentWatch::spawn(&[dir.clone()], move || {
            seen.fetch_add(1, Ordering::SeqCst);
        })
        .expect("watcher");

        std::fs::write(dir.join("session.jsonl"), b"{}").unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while hits.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
        let fired = hits.load(Ordering::SeqCst);
        drop(watch);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(fired >= 1, "a write in a watched directory did not fire");
    }

    #[test]
    fn nothing_to_watch_is_not_a_watcher() {
        assert!(AgentWatch::spawn(&[], || {}).is_none());
    }

    #[test]
    fn disabled_providers_contribute_no_paths() {
        let off = crate::providers::Enabled::none();
        assert!(paths_for(&off).is_empty());
    }
}
