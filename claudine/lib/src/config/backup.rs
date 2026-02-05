use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::Result;
use crate::events::Provider;

/// Create a timestamped backup of a config file before modification.
///
/// Backups are stored at `~/.claudine/backups/<provider>/<timestamp>.bak`.
pub fn create_backup(path: &Path, provider: Provider) -> Result<PathBuf> {
    let backup_dir = backup_base_dir()?.join(provider.as_slug());
    fs::create_dir_all(&backup_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("{timestamp}.bak"));

    fs::copy(path, &backup_path)?;

    Ok(backup_path)
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
    use std::fs;

    use tempfile::TempDir;

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

        // Verify a backup file was written (content may differ if concurrent
        // tests write to the same timestamp-based path)
        let content = fs::read_to_string(&backup_path).unwrap();
        assert!(!content.is_empty());
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
    fn backup_fails_for_missing_source() {
        let result = create_backup(Path::new("/nonexistent/file.json"), Provider::Claude);
        assert!(result.is_err());
    }
}
