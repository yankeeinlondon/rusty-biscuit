use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::Result;
use crate::provider::Provider;
/// Maximum number of backup files to retain per provider.
const MAX_BACKUPS: usize = 10;

/// Create a timestamped backup of a config file before modification.
///
/// Backups are stored at `~/.claudine/backups/<provider>/<timestamp>-<unique>.bak`.
/// The unique suffix prevents concurrent registrations from targeting the same
/// backup file. After creation, old backups beyond [`MAX_BACKUPS`] are pruned.
pub fn create_backup(path: &Path, provider: Provider) -> Result<PathBuf> {
    let backup_dir = backup_base_dir()?.join(provider.as_slug());
    create_backup_in(path, provider, &backup_dir)
}

fn create_backup_in(path: &Path, provider: Provider, backup_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(backup_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%9f");
    let mut backup = tempfile::Builder::new()
        .prefix(&format!("{timestamp}-"))
        .suffix(".bak")
        .tempfile_in(backup_dir)?;

    let mut source = fs::File::open(path)?;
    io::copy(&mut source, &mut backup)?;
    backup.as_file_mut().sync_all()?;
    let (_, backup_path) = backup.keep().map_err(|error| error.error)?;

    let deleted = cleanup_old_backups(backup_dir)?;
    if deleted > 0 {
        tracing::debug!(provider = %provider.as_slug(), deleted, retained = MAX_BACKUPS, "pruned old backups");
    }

    Ok(backup_path)
}

/// Remove old backups beyond [`MAX_BACKUPS`], keeping the most recent.
///
/// Files are sorted lexicographically by name (format
/// `YYYYMMDD_HHMMSS_nnnnnnnnn-<unique>.bak`), so alphabetical order follows
/// creation time.
///
/// ## Returns
///
/// The number of deleted backup files.
fn cleanup_old_backups(backup_dir: &Path) -> Result<usize> {
    let mut bak_files: Vec<PathBuf> = fs::read_dir(backup_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("bak") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if bak_files.len() <= MAX_BACKUPS {
        return Ok(0);
    }

    // Sort ascending by filename — oldest first
    bak_files.sort();

    let to_delete = bak_files.len() - MAX_BACKUPS;
    let mut deleted = 0;

    for path in bak_files.iter().take(to_delete) {
        if fs::remove_file(path).is_ok() {
            deleted += 1;
        } else {
            tracing::warn!(path = %path.display(), "failed to remove old backup");
        }
    }

    Ok(deleted)
}

/// Returns the base backup directory (`~/.claudine/backups`).
fn backup_base_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(home.join(".claudine").join("backups"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::sync::{Arc, Barrier};
    use std::thread;

    use tempfile::TempDir;
    use tracing_test::traced_test;

    use super::*;

    #[test]
    fn creates_backup_with_correct_structure() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("settings.json");
        let unique_content = format!(r#"{{"backup_test_id": "{}"}}"#, std::process::id());
        fs::write(&source, &unique_content).unwrap();

        let result = create_backup(&source, Provider::Claude);
        assert!(result.is_ok());

        let backup_path = result.unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.to_string_lossy().contains("claude"));
        assert!(backup_path.extension().unwrap() == "bak");
    }

    #[test]
    fn backup_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("config.toml");
        fs::write(&source, "key = \"value\"").unwrap();

        let result = create_backup(&source, Provider::Codex);
        assert!(result.is_ok());
        assert!(result.unwrap().exists());
    }

    #[test]
    fn concurrent_backups_use_distinct_files() {
        let tmp = TempDir::new().unwrap();
        let source = Arc::new(tmp.path().join("settings.json"));
        let backup_dir = Arc::new(tmp.path().join("backups").join("antigravity"));
        fs::write(&*source, "shared config").unwrap();

        let barrier = Arc::new(Barrier::new(8));
        let workers = (0..8)
            .map(|_| {
                let source = Arc::clone(&source);
                let backup_dir = Arc::clone(&backup_dir);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    create_backup_in(&source, Provider::Antigravity, &backup_dir).unwrap()
                })
            })
            .collect::<Vec<_>>();

        let paths = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(paths.iter().collect::<HashSet<_>>().len(), paths.len());
        for path in paths {
            assert_eq!(fs::read_to_string(path).unwrap(), "shared config");
        }
    }

    #[test]
    fn backup_fails_for_missing_source() {
        let result = create_backup(Path::new("/nonexistent/file.json"), Provider::Claude);
        assert!(result.is_err());
    }

    #[test]
    fn cleanup_retains_only_max_backups() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Create 15 fake backup files
        for i in 0..15 {
            let name = format!("20260201_{:06}.bak", i);
            fs::write(dir.join(&name), format!("backup {i}")).unwrap();
        }

        let deleted = cleanup_old_backups(dir).unwrap();
        assert_eq!(deleted, 5);

        let remaining: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("bak"))
            .collect();
        assert_eq!(remaining.len(), MAX_BACKUPS);
    }

    #[test]
    fn cleanup_leaves_fewer_than_max_untouched() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Create 5 fake backup files (fewer than MAX_BACKUPS)
        for i in 0..5 {
            let name = format!("20260201_{:06}.bak", i);
            fs::write(dir.join(&name), format!("backup {i}")).unwrap();
        }

        let deleted = cleanup_old_backups(dir).unwrap();
        assert_eq!(deleted, 0);

        let remaining: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("bak"))
            .collect();
        assert_eq!(remaining.len(), 5);
    }

    #[test]
    fn cleanup_keeps_most_recent_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Create 12 backups — oldest 2 should be deleted
        for i in 0..12 {
            let name = format!("20260201_{:06}.bak", i);
            fs::write(dir.join(&name), format!("backup {i}")).unwrap();
        }

        cleanup_old_backups(dir).unwrap();

        // The two oldest (000000, 000001) should be gone
        assert!(!dir.join("20260201_000000.bak").exists());
        assert!(!dir.join("20260201_000001.bak").exists());
        // The rest should remain
        assert!(dir.join("20260201_000002.bak").exists());
        assert!(dir.join("20260201_000011.bak").exists());
    }

    #[traced_test]
    #[test]
    fn cleanup_warns_on_unremovable_backup() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Add extra backups that are directories and sort oldest; remove_file
        // will fail on them so they produce warnings while being retained.
        for i in 0..2 {
            let name = format!("20260201_{:06}.bak", i);
            fs::create_dir(dir.join(&name)).unwrap();
        }
        for i in 2..(MAX_BACKUPS + 2) {
            let name = format!("20260201_{:06}.bak", i);
            fs::write(dir.join(&name), format!("backup {i}")).unwrap();
        }

        let deleted = cleanup_old_backups(dir).unwrap();
        assert_eq!(deleted, 0);

        logs_assert(|logs| {
            let warnings: Vec<_> = logs
                .iter()
                .filter(|l| l.contains("failed to remove old backup"))
                .collect();
            assert_eq!(warnings.len(), 2, "expected two warnings, got: {:?}", logs);
            Ok(())
        });
    }
}
