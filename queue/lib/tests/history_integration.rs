//! Integration tests for history persistence.
//!
//! These tests verify that history stores persist data correctly
//! across process restarts (simulated by creating new store instances).

use chrono::Utc;
use queue_lib::{ExecutionTarget, HistoryStore, JsonFileStore, ScheduledTask};
use tempfile::tempdir;

#[test]
fn history_persists_across_store_instances() {
    let dir = tempdir().unwrap();
    let history_path = dir.path().join("history.jsonl");

    // First store instance: save a task
    {
        let store = JsonFileStore::new(history_path.clone());
        let task = ScheduledTask::new(
            1,
            "echo persisted".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );
        store.save(&task).unwrap();
    }

    // Second store instance: load and verify
    {
        let store = JsonFileStore::new(history_path.clone());
        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].command, "echo persisted");
    }
}

#[test]
fn history_accumulates_tasks() {
    let dir = tempdir().unwrap();
    let history_path = dir.path().join("history.jsonl");

    // Save first task
    {
        let store = JsonFileStore::new(history_path.clone());
        let task = ScheduledTask::new(
            1,
            "first".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );
        store.save(&task).unwrap();
    }

    // Save second task in new instance
    {
        let store = JsonFileStore::new(history_path.clone());
        let task = ScheduledTask::new(
            2,
            "second".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );
        store.save(&task).unwrap();
    }

    // Verify both tasks exist
    {
        let store = JsonFileStore::new(history_path);
        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].command, "first");
        assert_eq!(tasks[1].command, "second");
    }
}

#[test]
fn history_update_persists_changes() {
    let dir = tempdir().unwrap();
    let history_path = dir.path().join("history.jsonl");

    // Save initial task
    {
        let store = JsonFileStore::new(history_path.clone());
        let task = ScheduledTask::new(
            1,
            "original".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );
        store.save(&task).unwrap();
    }

    // Update the task
    {
        let store = JsonFileStore::new(history_path.clone());
        let mut task = ScheduledTask::new(
            1,
            "original".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );
        task.mark_completed();
        store.update(&task).unwrap();
    }

    // Verify update persisted
    {
        let store = JsonFileStore::new(history_path);
        let tasks = store.load_all().unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].is_completed());
    }
}

#[test]
fn history_handles_sequential_saves_with_different_ids() {
    let dir = tempdir().unwrap();
    let history_path = dir.path().join("history.jsonl");

    // Save multiple tasks sequentially (simulating what happens
    // in real usage when tasks are saved one at a time)
    let store = JsonFileStore::new(history_path.clone());

    for i in 0..5 {
        let task = ScheduledTask::new(
            i,
            format!("task {i}"),
            Utc::now(),
            ExecutionTarget::Background,
        );
        store.save(&task).unwrap();
    }

    // Verify all tasks were saved
    let loaded = store.load_all().unwrap();
    assert_eq!(loaded.len(), 5);

    // Verify they're in order
    for (i, task) in loaded.iter().enumerate() {
        assert_eq!(task.id, i as u64);
        assert_eq!(task.command, format!("task {i}"));
    }
}
