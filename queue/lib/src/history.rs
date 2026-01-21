//! History storage for scheduled tasks.
//!
//! This module provides persistence abstractions for storing and retrieving
//! scheduled tasks. The primary implementation uses JSONL (newline-delimited JSON)
//! files with file locking for concurrent access safety.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use fs2::FileExt;

use crate::error::HistoryError;
use crate::types::ScheduledTask;

/// Default history file name.
const DEFAULT_HISTORY_FILE: &str = ".queue-history.jsonl";

/// Trait for history storage backends.
///
/// Implementations must handle concurrent access safely.
pub trait HistoryStore {
    /// Loads all tasks from the history store.
    ///
    /// ## Errors
    ///
    /// Returns an error if reading or parsing fails.
    fn load_all(&self) -> Result<Vec<ScheduledTask>, HistoryError>;

    /// Saves a task to the history store.
    ///
    /// ## Errors
    ///
    /// Returns an error if writing fails.
    fn save(&self, task: &ScheduledTask) -> Result<(), HistoryError>;

    /// Updates an existing task in the history store.
    ///
    /// This rewrites the entire history file with the updated task.
    ///
    /// ## Errors
    ///
    /// Returns an error if reading, writing, or parsing fails.
    fn update(&self, task: &ScheduledTask) -> Result<(), HistoryError>;
}

/// JSONL file-based history storage.
///
/// Stores tasks as newline-delimited JSON with file locking for safe
/// concurrent access. Uses `fs2` for cross-platform file locking.
///
/// ## Examples
///
/// ```no_run
/// use queue_lib::{JsonFileStore, HistoryStore, ScheduledTask, ExecutionTarget};
/// use chrono::Utc;
///
/// let store = JsonFileStore::new("/tmp/test-history.jsonl".into());
/// let task = ScheduledTask::new(1, "echo hello".to_string(), Utc::now(), ExecutionTarget::NewPane);
/// store.save(&task).unwrap();
///
/// let tasks = store.load_all().unwrap();
/// assert_eq!(tasks.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    /// Creates a new JSON file store at the given path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Creates a new JSON file store at the default path (`~/.queue-history.jsonl`).
    ///
    /// ## Panics
    ///
    /// Panics if the home directory cannot be determined.
    pub fn default_path() -> Self {
        let home = dirs::home_dir().expect("could not determine home directory");
        Self::new(home.join(DEFAULT_HISTORY_FILE))
    }

    /// Returns the path to the history file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Ensures the history file exists, creating it if necessary.
    fn ensure_file_exists(&self) -> Result<(), HistoryError> {
        if !self.path.exists() {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            File::create(&self.path)?;
        }
        Ok(())
    }
}

impl HistoryStore for JsonFileStore {
    fn load_all(&self) -> Result<Vec<ScheduledTask>, HistoryError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        file.lock_shared().map_err(|_| HistoryError::Lock)?;

        let reader = BufReader::new(&file);
        let mut tasks = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let task: ScheduledTask = serde_json::from_str(&line)?;
            tasks.push(task);
        }

        file.unlock().map_err(|_| HistoryError::Lock)?;
        Ok(tasks)
    }

    fn save(&self, task: &ScheduledTask) -> Result<(), HistoryError> {
        self.ensure_file_exists()?;

        let mut file = OpenOptions::new().append(true).open(&self.path)?;

        file.lock_exclusive().map_err(|_| HistoryError::Lock)?;

        let json = serde_json::to_string(task)?;
        writeln!(file, "{json}")?;

        file.unlock().map_err(|_| HistoryError::Lock)?;
        Ok(())
    }

    fn update(&self, task: &ScheduledTask) -> Result<(), HistoryError> {
        self.ensure_file_exists()?;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)?;

        file.lock_exclusive().map_err(|_| HistoryError::Lock)?;

        // Read all tasks
        let reader = BufReader::new(&file);
        let mut tasks = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let existing: ScheduledTask = serde_json::from_str(&line)?;
            if existing.id == task.id {
                tasks.push(task.clone());
            } else {
                tasks.push(existing);
            }
        }

        // Rewrite the file
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        for t in &tasks {
            let json = serde_json::to_string(t)?;
            writeln!(file, "{json}")?;
        }

        file.unlock().map_err(|_| HistoryError::Lock)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExecutionTarget;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;

    fn create_test_store() -> (JsonFileStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test-history.jsonl");
        let store = JsonFileStore::new(path);
        (store, temp_dir)
    }

    #[test]
    fn load_all_returns_empty_vec_for_nonexistent_file() {
        let (store, _temp_dir) = create_test_store();
        let tasks = store.load_all().unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn save_and_load_single_task() {
        let (store, _temp_dir) = create_test_store();

        let scheduled_at = Utc::now() + Duration::hours(1);
        let task = ScheduledTask::new(
            1,
            "cargo build".to_string(),
            scheduled_at,
            ExecutionTarget::NewPane,
        );

        store.save(&task).unwrap();

        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, 1);
        assert_eq!(tasks[0].command, "cargo build");
    }

    #[test]
    fn save_multiple_tasks_appends() {
        let (store, _temp_dir) = create_test_store();

        for i in 1..=3 {
            let task = ScheduledTask::new(
                i,
                format!("task {i}"),
                Utc::now() + Duration::minutes(i as i64),
                ExecutionTarget::Background,
            );
            store.save(&task).unwrap();
        }

        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].id, 1);
        assert_eq!(tasks[1].id, 2);
        assert_eq!(tasks[2].id, 3);
    }

    #[test]
    fn update_modifies_existing_task() {
        let (store, _temp_dir) = create_test_store();

        let mut task = ScheduledTask::new(
            1,
            "original command".to_string(),
            Utc::now(),
            ExecutionTarget::NewPane,
        );
        store.save(&task).unwrap();

        task.mark_completed();
        store.update(&task).unwrap();

        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].is_completed());
    }

    #[test]
    fn update_preserves_other_tasks() {
        let (store, _temp_dir) = create_test_store();

        let task1 = ScheduledTask::new(
            1,
            "task 1".to_string(),
            Utc::now(),
            ExecutionTarget::NewPane,
        );
        let mut task2 = ScheduledTask::new(
            2,
            "task 2".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );
        let task3 = ScheduledTask::new(
            3,
            "task 3".to_string(),
            Utc::now(),
            ExecutionTarget::NewWindow,
        );

        store.save(&task1).unwrap();
        store.save(&task2).unwrap();
        store.save(&task3).unwrap();

        task2.mark_failed("test error");
        store.update(&task2).unwrap();

        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].is_pending());
        assert!(tasks[1].is_failed());
        assert!(tasks[2].is_pending());
    }

    #[test]
    fn jsonl_format_is_correct() {
        let (store, _temp_dir) = create_test_store();

        let task = ScheduledTask::new(
            1,
            "echo hello".to_string(),
            Utc::now(),
            ExecutionTarget::NewPane,
        );
        store.save(&task).unwrap();

        let contents = std::fs::read_to_string(store.path()).unwrap();
        let lines: Vec<&str> = contents.lines().collect();

        // Should be exactly one line (JSONL format)
        assert_eq!(lines.len(), 1);

        // Line should be valid JSON
        let parsed: ScheduledTask = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed.id, 1);
    }

    #[test]
    fn empty_lines_are_skipped_during_load() {
        let (store, _temp_dir) = create_test_store();

        let task = ScheduledTask::new(
            1,
            "test".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );

        // Manually write with empty lines
        let json = serde_json::to_string(&task).unwrap();
        std::fs::write(store.path(), format!("\n{json}\n\n")).unwrap();

        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn default_path_points_to_home_directory() {
        let store = JsonFileStore::default_path();
        let home = dirs::home_dir().unwrap();
        assert_eq!(store.path(), &home.join(".queue-history.jsonl"));
    }

    #[test]
    fn file_locking_allows_sequential_access() {
        let (store, _temp_dir) = create_test_store();

        // Simulate sequential access patterns
        for i in 1..=5 {
            let task = ScheduledTask::new(
                i,
                format!("task {i}"),
                Utc::now(),
                ExecutionTarget::NewPane,
            );
            store.save(&task).unwrap();
        }

        // Multiple sequential reads should work
        let _ = store.load_all().unwrap();
        let _ = store.load_all().unwrap();
        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 5);
    }
}
