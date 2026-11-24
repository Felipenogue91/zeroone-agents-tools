//! Atomic file writes for config files.
//!
//! Config files such as `~/.claude.json` are rewritten constantly by the CLIs
//! themselves. A plain `std::fs::write` truncates the file first, so a crash
//! or a concurrent reader can observe an empty/partial file. `write_atomic`
//! writes to a temporary sibling file and renames it over the target, which
//! is atomic on the same filesystem (POSIX `rename`, Windows `MoveFileExW`
//! with replace).
//!
//! Symlinks are followed: when the target path is a symlink (dotfile repos,
//! `~/.agents` shared layouts), the *link target* is replaced and the link
//! itself is preserved. Existing file permissions are kept (important for
//! files holding secrets, e.g. 0600).

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Resolve the path that must actually be replaced: follow symlinks so the
/// link is never clobbered by a regular file.
fn resolve_target(path: &Path) -> io::Result<PathBuf> {
    let mut current = path.to_path_buf();
    // Bounded loop protects against symlink cycles.
    for _ in 0..32 {
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let link = std::fs::read_link(&current)?;
                current = if link.is_absolute() {
                    link
                } else {
                    current.parent().map(|p| p.join(&link)).unwrap_or(link)
                };
            }
            _ => return Ok(current),
        }
    }
    Err(io::Error::other(format!(
        "too many levels of symbolic links: {}",
        path.display()
    )))
}

/// Drop-in replacement for `std::fs::write` that writes atomically.
pub fn write_atomic<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let target = resolve_target(path.as_ref())?;
    let dir = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let file_name = target
        .file_name()
        .ok_or_else(|| io::Error::other(format!("invalid file path: {}", target.display())))?
        .to_string_lossy()
        .to_string();
    let tmp = dir.join(format!(
        ".{}.zeroone-tmp-{}-{}",
        file_name,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(contents.as_ref())?;
        f.sync_all()?;
        drop(f);
        if let Ok(meta) = std::fs::metadata(&target) {
            std::fs::set_permissions(&tmp, meta.permissions())?;
        }
        std::fs::rename(&tmp, &target)
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn writes_new_file() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("a.json");
        write_atomic(&p, "{}").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "{}");
    }

    #[test]
    fn replaces_existing_file_and_leaves_no_temp() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("a.json");
        std::fs::write(&p, "old").unwrap();
        write_atomic(&p, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("zeroone-tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn fails_when_parent_missing_like_fs_write() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("missing").join("a.json");
        assert!(write_atomic(&p, "x").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("secret.json");
        std::fs::write(&p, "old").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
        write_atomic(&p, "new").unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn follows_symlink_and_keeps_link() {
        let dir = TempDir::new().unwrap();
        let real = dir.path().join("real.json");
        let link = dir.path().join("link.json");
        std::fs::write(&real, "old").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, "new").unwrap();
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn follows_relative_symlink() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let real = dir.path().join("sub").join("real.json");
        std::fs::write(&real, "old").unwrap();
        let link = dir.path().join("link.json");
        std::os::unix::fs::symlink("sub/real.json", &link).unwrap();
        write_atomic(&link, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "new");
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
