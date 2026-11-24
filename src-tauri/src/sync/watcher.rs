//! File watcher for every config file the sync engine reads or writes.
//!
//! Folders are watched non-recursively (CLIs and editors replace files by
//! rename, which a per-file watch would lose) and events are filtered by
//! file name. Bursts are debounced; the callback receives the changed paths.

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// (folder, file names of interest; empty = any entry in the folder)
pub type WatchSpec = Vec<(PathBuf, Vec<String>)>;

pub fn watch_spec(home: &Path, projects: &[PathBuf]) -> WatchSpec {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let mut spec: WatchSpec = vec![
        (home.to_path_buf(), s(&[".claude.json"])),
        (
            home.join(".agents").join("mcp"),
            s(&["global.json", "sync.json", "secrets.env"]),
        ),
        (home.join(".agents").join("skills"), vec![]),
        (home.join(".claude").join("skills"), vec![]),
        (home.join(".codex"), s(&["config.toml"])),
        (
            home.join(".gemini").join("config"),
            s(&["mcp_config.json", "skills.json"]),
        ),
    ];
    for p in projects {
        spec.push((p.clone(), s(&[".mcp.json"])));
        spec.push((p.join(".claude"), s(&["settings.json"])));
        spec.push((p.join(".codex"), s(&["config.toml"])));
        spec.push((p.join(".agents"), s(&["mcp_config.json"])));
    }
    spec
}

/// Does an event path match the spec?
pub fn interesting(spec: &WatchSpec, path: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    // our own temp files are noise
    if name.contains(".zeroone-tmp-") || name.ends_with(".mcp-sync.tmp") {
        return false;
    }
    spec.iter()
        .any(|(dir, names)| dir == parent && (names.is_empty() || names.iter().any(|n| *n == name)))
}

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
}

/// Start watching. `on_change` runs on a background thread after each
/// debounced burst (at most once per `debounce`).
pub fn start<F>(spec: WatchSpec, debounce: Duration, on_change: F) -> notify::Result<ConfigWatcher>
where
    F: Fn(Vec<PathBuf>) + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<PathBuf>();
    let filter = spec.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            if matches!(ev.kind, notify::EventKind::Access(_)) {
                return;
            }
            for p in ev.paths {
                if interesting(&filter, &p) {
                    let _ = tx.send(p);
                }
            }
        }
    })?;
    for (dir, _) in &spec {
        if dir.is_dir() {
            if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
                log::debug!("[watcher] cannot watch {}: {}", dir.display(), e);
            }
        }
    }
    std::thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut batch: BTreeSet<PathBuf> = BTreeSet::new();
            batch.insert(first);
            let deadline = Instant::now() + debounce;
            while let Some(left) = deadline.checked_duration_since(Instant::now()) {
                match rx.recv_timeout(left) {
                    Ok(p) => {
                        batch.insert(p);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            on_change(batch.into_iter().collect());
        }
    });
    Ok(ConfigWatcher { _watcher: watcher })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[test]
    fn filter_matches_only_config_files() {
        let home = Path::new("/h");
        let spec = watch_spec(home, &[PathBuf::from("/w/p")]);
        assert!(interesting(&spec, Path::new("/h/.claude.json")));
        assert!(!interesting(&spec, Path::new("/h/.bashrc")));
        assert!(interesting(&spec, Path::new("/h/.agents/skills/new-skill")));
        assert!(interesting(&spec, Path::new("/w/p/.mcp.json")));
        assert!(interesting(&spec, Path::new("/w/p/.claude/settings.json")));
        assert!(!interesting(
            &spec,
            Path::new("/w/p/.claude/settings.local.json")
        ));
        assert!(!interesting(
            &spec,
            Path::new("/h/.codex/.config.toml.zeroone-tmp-1-2")
        ));
    }

    #[test]
    fn debounced_callback_receives_changes() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        let got: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let g = got.clone();
        let _w = start(
            watch_spec(&home, &[]),
            Duration::from_millis(150),
            move |paths| {
                g.lock().unwrap().extend(paths);
            },
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(home.join(".codex/config.toml"), "a = 1").unwrap();
        std::fs::write(home.join(".codex/other.txt"), "x").unwrap();
        crate::utils::atomic::write_atomic(home.join(".codex/config.toml"), "a = 2").unwrap();
        for _ in 0..40 {
            if !got.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(300));
        let got = got.lock().unwrap();
        assert!(
            got.iter().any(|p| p.ends_with(".codex/config.toml")),
            "{:?}",
            got
        );
        assert!(got.iter().all(|p| !p.ends_with("other.txt")));
    }
}
