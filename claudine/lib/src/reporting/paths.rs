use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDate};

use crate::error::{ClaudineError, Result};

/// Expand a leading `~` to the current user's home directory.
pub fn expand_path(path: &Path) -> Result<PathBuf> {
    if !path.starts_with("~") {
        return Ok(path.to_path_buf());
    }

    let home = dirs::home_dir().ok_or_else(|| {
        ClaudineError::ReportingPathUnavailable("home directory is not available".to_string())
    })?;

    Ok(home.join(path.strip_prefix("~").unwrap_or(path)))
}

/// Return the base `~/.claudine` directory.
pub fn claudine_home_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|path| path.join(".claudine"))
        .ok_or_else(|| {
            ClaudineError::ReportingPathUnavailable("home directory is not available".to_string())
        })
}

/// Return the shared log directory used by both event writing and reporting.
pub fn default_logs_dir() -> Result<PathBuf> {
    Ok(claudine_home_dir()?.join("logs"))
}

/// Return the default SQLite metrics database path.
pub fn default_metrics_db_path() -> Result<PathBuf> {
    Ok(default_logs_dir()?.join("metrics.db"))
}

/// Resolve a file log path, optionally rotating by local calendar day.
pub fn resolve_file_log_path(path: Option<&Path>, rotate_daily: bool) -> Result<PathBuf> {
    resolve_file_log_path_at(path, rotate_daily, Local::now())
}

/// Resolve a file log path using a caller-supplied timestamp.
pub fn resolve_file_log_path_at(
    path: Option<&Path>,
    rotate_daily: bool,
    now: DateTime<Local>,
) -> Result<PathBuf> {
    if let Some(path) = path {
        return expand_path(path);
    }

    let base = default_logs_dir()?;
    if rotate_daily {
        return Ok(base.join(format!("{}.jsonl", now.format("%Y-%m-%d"))));
    }

    Ok(base.join("events.jsonl"))
}

/// Resolve the shared daily log file path for a specific date.
pub fn daily_log_path(date: NaiveDate) -> Result<PathBuf> {
    Ok(default_logs_dir()?.join(format!("{}.jsonl", date.format("%Y-%m-%d"))))
}

/// Parse a local-date daily log filename such as `2026-03-06.jsonl`.
pub fn daily_log_date_from_path(path: &Path) -> Option<NaiveDate> {
    let stem = path.file_stem()?.to_str()?;
    NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    #[test]
    fn resolve_file_log_path_at_uses_daily_rotation_name() {
        let now = Local.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();
        let path = resolve_file_log_path_at(None, true, now).unwrap();

        assert!(path.ends_with("2026-04-03.jsonl"));
    }

    #[test]
    fn daily_log_date_from_path_parses_valid_date_stems() {
        let parsed = daily_log_date_from_path(Path::new("/tmp/2026-04-03.jsonl"));

        assert_eq!(parsed, NaiveDate::from_ymd_opt(2026, 4, 3));
        assert!(daily_log_date_from_path(Path::new("/tmp/events.jsonl")).is_none());
    }
}
