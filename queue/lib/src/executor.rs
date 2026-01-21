//! Task execution engine for scheduled commands.
//!
//! This module provides the core execution infrastructure for running scheduled
//! tasks at their designated times. It supports multiple execution targets:
//! - Wezterm panes for integrated terminal workflows
//! - Native terminal windows for various terminal emulators
//! - Background processes for detached execution
//!
//! ## Examples
//!
//! ```no_run
//! use queue_lib::{ExecutionTarget, ScheduledTask, TaskEvent, TaskExecutor};
//! use tokio::sync::mpsc;
//! use chrono::Utc;
//!
//! # async fn example() {
//! let (tx, mut rx) = mpsc::channel::<TaskEvent>(100);
//! let executor = TaskExecutor::new(tx);
//!
//! let task = ScheduledTask::new(
//!     1,
//!     "echo hello".to_string(),
//!     Utc::now(),
//!     ExecutionTarget::Background,
//! );
//!
//! executor.schedule(task);
//!
//! // Listen for status updates
//! while let Some(event) = rx.recv().await {
//!     match event {
//!         TaskEvent::StatusChanged { id, status } => {
//!             println!("Task {} is now {:?}", id, status);
//!         }
//!     }
//! }
//! # }
//! ```

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::process::Command;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep_until, Instant};

use crate::{ExecutionTarget, ScheduledTask, TaskStatus, TerminalDetector, TerminalKind};

/// Event emitted when a task's status changes.
///
/// These events are sent through the channel provided to [`TaskExecutor::new`]
/// to notify listeners of task lifecycle transitions.
#[derive(Debug, Clone)]
pub enum TaskEvent {
    /// A task's status has changed.
    StatusChanged {
        /// The unique identifier of the task.
        id: u64,
        /// The new status of the task.
        status: TaskStatus,
    },
}

/// Executes scheduled tasks at their designated times.
///
/// The executor spawns a tokio task for each scheduled task that waits until
/// the scheduled time, executes the command in the appropriate environment,
/// and reports status changes through the event channel.
///
/// ## Event Flow
///
/// 1. Task is scheduled via [`schedule`](TaskExecutor::schedule)
/// 2. Executor waits until `scheduled_at` time
/// 3. [`TaskEvent::StatusChanged`] with [`TaskStatus::Running`] is emitted
/// 4. Command executes in the specified [`ExecutionTarget`]
/// 5. [`TaskEvent::StatusChanged`] with [`TaskStatus::Completed`] or [`TaskStatus::Failed`] is emitted
///
/// ## Pane Management
///
/// When running in Wezterm, the executor can be configured with a target pane ID
/// for task execution. Tasks with `NewPane` target will create new panes within
/// that target area, keeping the TUI pane separate.
pub struct TaskExecutor {
    event_tx: mpsc::Sender<TaskEvent>,
    /// The pane ID where tasks should be executed (for Wezterm pane support).
    /// This is shared across all spawned tasks.
    task_pane_id: Arc<RwLock<Option<String>>>,
    /// Handles to scheduled task futures for cancellation.
    task_handles: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
}

impl TaskExecutor {
    /// Creates a new task executor.
    ///
    /// ## Arguments
    ///
    /// * `event_tx` - Channel sender for task status events
    ///
    /// ## Examples
    ///
    /// ```
    /// use queue_lib::TaskExecutor;
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, _rx) = mpsc::channel(100);
    /// let executor = TaskExecutor::new(tx);
    /// ```
    #[must_use]
    pub fn new(event_tx: mpsc::Sender<TaskEvent>) -> Self {
        Self {
            event_tx,
            task_pane_id: Arc::new(RwLock::new(None)),
            task_handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Sets the target pane ID for task execution.
    ///
    /// When set, tasks with `NewPane` target will create new panes within
    /// this target pane area, keeping them separate from the TUI.
    pub async fn set_task_pane_id(&self, pane_id: Option<String>) {
        let mut guard = self.task_pane_id.write().await;
        *guard = pane_id;
    }

    /// Sets the target pane ID synchronously (for non-async contexts).
    ///
    /// This is useful during initialization before the async runtime is fully active.
    pub fn set_task_pane_id_sync(&self, pane_id: Option<String>) {
        // Use try_write to avoid blocking - this should always succeed during init
        if let Ok(mut guard) = self.task_pane_id.try_write() {
            *guard = pane_id;
        }
    }

    /// Schedules a task for execution.
    ///
    /// This method spawns a background tokio task that will:
    /// 1. Wait until the scheduled time (if in the future)
    /// 2. Execute the command in the appropriate environment
    /// 3. Send status updates through the event channel
    ///
    /// ## Arguments
    ///
    /// * `task` - The task to schedule
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use queue_lib::{ExecutionTarget, ScheduledTask, TaskExecutor};
    /// use tokio::sync::mpsc;
    /// use chrono::{Duration, Utc};
    ///
    /// # async fn example() {
    /// let (tx, _rx) = mpsc::channel(100);
    /// let executor = TaskExecutor::new(tx);
    ///
    /// let task = ScheduledTask::new(
    ///     1,
    ///     "cargo build".to_string(),
    ///     Utc::now() + Duration::minutes(5),
    ///     ExecutionTarget::NewPane,
    /// );
    ///
    /// executor.schedule(task);
    /// # }
    /// ```
    pub fn schedule(&self, task: ScheduledTask) {
        let tx = self.event_tx.clone();
        let task_pane_id = self.task_pane_id.clone();
        let task_handles = self.task_handles.clone();
        let task_id = task.id;
        let handle = tokio::spawn(async move {
            Self::execute_task(task, tx, task_pane_id, task_handles.clone()).await;
        });
        if let Ok(mut handles) = self.task_handles.lock() {
            if !handle.is_finished() {
                handles.insert(task_id, handle);
            }
        }
    }

    /// Cancels a scheduled task if it hasn't started executing.
    #[must_use]
    pub fn cancel_task(&self, task_id: u64) -> bool {
        if let Ok(mut handles) = self.task_handles.lock() {
            if let Some(handle) = handles.remove(&task_id) {
                handle.abort();
                return true;
            }
        }
        false
    }

    /// Internal implementation of task execution.
    async fn execute_task(
        task: ScheduledTask,
        tx: mpsc::Sender<TaskEvent>,
        task_pane_id: Arc<RwLock<Option<String>>>,
        task_handles: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
    ) {
        // Wait until scheduled time
        let now = Utc::now();
        if task.scheduled_at > now {
            let duration = (task.scheduled_at - now).to_std().unwrap_or_default();
            let deadline = Instant::now() + duration;
            sleep_until(deadline).await;
        }

        // Mark as running
        let _ = tx
            .send(TaskEvent::StatusChanged {
                id: task.id,
                status: TaskStatus::Running,
            })
            .await;

        // Get the target pane ID for task execution
        let pane_id = task_pane_id.read().await.clone();

        // Execute based on target
        let result = match task.target {
            ExecutionTarget::NewPane => Self::execute_in_pane(&task.command, pane_id.as_deref()).await,
            ExecutionTarget::NewWindow => Self::execute_in_window(&task.command).await,
            ExecutionTarget::Background => Self::execute_background(&task.command).await,
        };

        // Report completion status
        let status = match result {
            Ok(()) => TaskStatus::Completed,
            Err(e) => TaskStatus::Failed { error: e },
        };

        let _ = tx.send(TaskEvent::StatusChanged { id: task.id, status }).await;

        if let Ok(mut handles) = task_handles.lock() {
            handles.remove(&task.id);
        }
    }

    /// Executes a command in a new Wezterm pane.
    ///
    /// Creates a new pane in the task execution area (separate from the TUI).
    /// If a `task_pane_id` is provided, the pane is created by splitting that
    /// specific pane. This ensures tasks don't interfere with the TUI.
    ///
    /// The command runs directly in the new pane with full terminal access,
    /// supporting interactive programs. The pane closes when the command exits.
    ///
    /// If not running in Wezterm, falls back to opening a new window.
    ///
    /// ## Arguments
    ///
    /// * `command` - The command to execute
    /// * `task_pane_id` - Optional pane ID to split for the new task pane
    async fn execute_in_pane(command: &str, task_pane_id: Option<&str>) -> Result<(), String> {
        // Check if we're in Wezterm
        if !TerminalDetector::is_wezterm() {
            return Self::execute_in_window(command).await;
        }

        // Build command arguments - run the command directly without wrapping
        let mut args = vec!["cli", "split-pane", "--top"];

        // If we have a specific task pane ID, target that pane
        // This creates the task pane in the designated area, not in the TUI pane
        if let Some(pane_id) = task_pane_id {
            args.extend(["--pane-id", pane_id]);
        }

        args.extend(["--", "/bin/sh", "-c", command]);

        let status = Command::new("wezterm")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("wezterm exited with status {status}"))
        }
    }

    /// Executes a command in a new terminal window.
    ///
    /// Detects the current terminal emulator and uses the appropriate method
    /// to open a new window. Supports:
    /// - Wezterm (spawn new tab)
    /// - iTerm2 (AppleScript)
    /// - Terminal.app (AppleScript)
    /// - GNOME Terminal
    /// - Konsole
    /// - Xfce4 Terminal
    /// - XTerm (fallback)
    async fn execute_in_window(command: &str) -> Result<(), String> {
        let caps = TerminalDetector::detect();

        let result = match caps.kind {
            TerminalKind::Wezterm => {
                // Wezterm: open new tab
                Command::new("wezterm")
                    .args(["cli", "spawn", "--", "/bin/sh", "-c", command])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
            }
            TerminalKind::ITerm2 => {
                // iTerm2: use osascript
                let script = format!(
                    r#"tell application "iTerm2"
                        create window with default profile
                        tell current session of current window
                            write text "{}"
                        end tell
                    end tell"#,
                    command.replace('\\', "\\\\").replace('"', "\\\"")
                );
                Command::new("osascript")
                    .args(["-e", &script])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
            }
            TerminalKind::TerminalApp => {
                // macOS Terminal.app
                let script = format!(
                    r#"tell application "Terminal"
                        do script "{}"
                        activate
                    end tell"#,
                    command.replace('\\', "\\\\").replace('"', "\\\"")
                );
                Command::new("osascript")
                    .args(["-e", &script])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
            }
            TerminalKind::GnomeTerminal => {
                Command::new("gnome-terminal")
                    .args(["--", "/bin/sh", "-c", command])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
            }
            TerminalKind::Konsole => {
                Command::new("konsole")
                    .args(["-e", "/bin/sh", "-c", command])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
            }
            TerminalKind::Xfce4Terminal => {
                Command::new("xfce4-terminal")
                    .args(["-e", command])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
            }
            TerminalKind::Xterm => {
                // XTerm
                Command::new("xterm")
                    .args(["-e", "/bin/sh", "-c", command])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
            }
            TerminalKind::Unknown => {
                // Unknown terminal - use platform-appropriate fallback
                #[cfg(target_os = "macos")]
                {
                    // macOS: use Terminal.app via osascript
                    let script = format!(
                        r#"tell application "Terminal"
                            do script "{}"
                            activate
                        end tell"#,
                        command.replace('\\', "\\\\").replace('"', "\\\"")
                    );
                    Command::new("osascript")
                        .args(["-e", &script])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .await
                }
                #[cfg(not(target_os = "macos"))]
                {
                    // Linux/other: try common terminal emulators in order
                    // Try xterm first, then x-terminal-emulator (Debian/Ubuntu default)
                    let xterm_result = Command::new("xterm")
                        .args(["-e", "/bin/sh", "-c", command])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .await;

                    if xterm_result.is_ok() {
                        xterm_result
                    } else {
                        // Fallback to x-terminal-emulator on Debian/Ubuntu
                        Command::new("x-terminal-emulator")
                            .args(["-e", "/bin/sh", "-c", command])
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .await
                    }
                }
            }
        };

        match result {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(format!("terminal exited with status {status}")),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Executes a command as a detached background process.
    ///
    /// The process runs independently and continues even after the executor
    /// is dropped. Output is discarded.
    async fn execute_background(command: &str) -> Result<(), String> {
        Command::new("/bin/sh")
            .args(["-c", command])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn task_event_status_changed_creation() {
        let event = TaskEvent::StatusChanged {
            id: 42,
            status: TaskStatus::Running,
        };

        match event {
            TaskEvent::StatusChanged { id, status } => {
                assert_eq!(id, 42);
                assert_eq!(status, TaskStatus::Running);
            }
        }
    }

    #[tokio::test]
    async fn task_executor_creation() {
        let (tx, _rx) = mpsc::channel::<TaskEvent>(100);
        let _executor = TaskExecutor::new(tx);
        // Executor created successfully
    }

    #[tokio::test]
    async fn task_scheduled_immediately_sends_running_status() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        // Create a task scheduled for the past (runs immediately)
        let task = ScheduledTask::new(
            1,
            "true".to_string(), // Simple command that succeeds
            Utc::now() - Duration::seconds(1),
            ExecutionTarget::Background,
        );

        executor.schedule(task);

        // Should receive Running status
        let event = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("channel closed");

        match event {
            TaskEvent::StatusChanged { id, status } => {
                assert_eq!(id, 1);
                assert_eq!(status, TaskStatus::Running);
            }
        }
    }

    #[tokio::test]
    async fn task_background_success_sends_completed_status() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        let task = ScheduledTask::new(
            2,
            "true".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );

        executor.schedule(task);

        // Skip Running status
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout")
            .expect("closed");

        // Should receive Completed status
        let event = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout waiting for completion")
            .expect("channel closed");

        match event {
            TaskEvent::StatusChanged { id, status } => {
                assert_eq!(id, 2);
                assert_eq!(status, TaskStatus::Completed);
            }
        }
    }

    #[tokio::test]
    async fn task_waits_for_scheduled_time() {
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        // Schedule task 100ms in the future
        let scheduled_at = Utc::now() + Duration::milliseconds(100);
        let task = ScheduledTask::new(
            3,
            "true".to_string(),
            scheduled_at,
            ExecutionTarget::Background,
        );

        let start = std::time::Instant::now();
        executor.schedule(task);

        // Wait for Running status
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout")
            .expect("closed");

        let elapsed = start.elapsed();
        // Should have waited at least ~100ms (with some tolerance)
        assert!(elapsed >= std::time::Duration::from_millis(50));
    }

    #[test]
    fn task_event_clone() {
        let event = TaskEvent::StatusChanged {
            id: 1,
            status: TaskStatus::Completed,
        };
        let cloned = event.clone();

        match (event, cloned) {
            (
                TaskEvent::StatusChanged { id: id1, status: status1 },
                TaskEvent::StatusChanged { id: id2, status: status2 },
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(status1, status2);
            }
        }
    }

    #[test]
    fn task_event_debug() {
        let event = TaskEvent::StatusChanged {
            id: 42,
            status: TaskStatus::Failed {
                error: "test error".to_string(),
            },
        };
        let debug = format!("{event:?}");
        assert!(debug.contains("StatusChanged"));
        assert!(debug.contains("42"));
        assert!(debug.contains("test error"));
    }

    // =========================================================================
    // Regression tests for bug: Tasks execute in TUI pane instead of task pane
    // =========================================================================

    #[tokio::test]
    async fn executor_can_set_task_pane_id() {
        // Regression test: The executor must support setting a task pane ID
        // so tasks execute in the correct area, not in the TUI pane
        let (tx, _rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        // Set a mock task pane ID
        executor.set_task_pane_id(Some("task-pane-123".to_string())).await;

        // Verify via try_read (not awaiting, just checking it was set)
        let pane_id = executor.task_pane_id.read().await;
        assert_eq!(pane_id.as_deref(), Some("task-pane-123"));
    }

    #[tokio::test]
    async fn executor_task_pane_id_defaults_to_none() {
        // Regression test: The task pane ID should default to None
        // This is important for non-Wezterm environments where no pane split occurs
        let (tx, _rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        let pane_id = executor.task_pane_id.read().await;
        assert!(pane_id.is_none());
    }

    #[test]
    fn executor_can_set_task_pane_id_sync() {
        // Regression test: set_task_pane_id_sync should work during initialization
        // before the async runtime is fully active
        let (tx, _rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        executor.set_task_pane_id_sync(Some("sync-pane-456".to_string()));

        // Use try_read to verify (this is sync)
        let guard = executor.task_pane_id.try_read().unwrap();
        assert_eq!(guard.as_deref(), Some("sync-pane-456"));
    }

    #[tokio::test]
    async fn executor_can_clear_task_pane_id() {
        // Regression test: The task pane ID should be clearable
        let (tx, _rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        // Set and then clear
        executor.set_task_pane_id(Some("pane-to-clear".to_string())).await;
        executor.set_task_pane_id(None).await;

        let pane_id = executor.task_pane_id.read().await;
        assert!(pane_id.is_none());
    }

    #[tokio::test]
    async fn multiple_tasks_share_task_pane_id() {
        // Regression test: Multiple scheduled tasks should all use the same
        // task pane ID, ensuring they execute in the correct area
        let (tx, mut rx) = mpsc::channel::<TaskEvent>(100);
        let executor = TaskExecutor::new(tx);

        // Set the task pane ID
        executor.set_task_pane_id(Some("shared-pane".to_string())).await;

        // Schedule two background tasks (won't actually use pane ID but tests the sharing)
        let task1 = ScheduledTask::new(
            1,
            "true".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );
        let task2 = ScheduledTask::new(
            2,
            "true".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );

        executor.schedule(task1);
        executor.schedule(task2);

        // Both tasks should complete (receive 4 events: 2 Running + 2 Completed)
        let mut completed_count = 0;
        let timeout = std::time::Duration::from_secs(5);
        let start = std::time::Instant::now();

        while completed_count < 2 && start.elapsed() < timeout {
            if let Ok(Some(event)) = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                rx.recv(),
            ).await {
                if let TaskEvent::StatusChanged { status: TaskStatus::Completed, .. } = event {
                    completed_count += 1;
                }
            }
        }

        assert_eq!(completed_count, 2, "Both tasks should complete");

        // Verify the pane ID is still set correctly
        let pane_id = executor.task_pane_id.read().await;
        assert_eq!(pane_id.as_deref(), Some("shared-pane"));
    }

    // =========================================================================
    // Regression tests for Bug 2: Tasks should run independently
    // =========================================================================

    // Note: The execute_in_pane function no longer wraps commands with
    // "Press Enter to close" prompts. This allows interactive commands to
    // work properly in new panes. The fix removed the command wrapping that
    // would have blocked interactive programs.
    //
    // This is verified by code review - the wrapped_command variable that
    // previously contained the "Press Enter" wrapper has been removed.

    // =========================================================================
    // Regression tests for Bug 4: Task execution in non-Wezterm terminals
    // =========================================================================

    // Note: execute_in_window() now has proper fallback behavior for
    // TerminalKind::Unknown on macOS. Previously it would try to use `xterm`
    // which doesn't exist on macOS, causing all task executions to fail.
    //
    // The fix:
    // - On macOS with Unknown terminal: uses Terminal.app via osascript
    // - On Linux with Unknown terminal: tries xterm, then x-terminal-emulator
    //
    // This is a platform-specific code path that cannot be unit tested without
    // mocking the system terminal infrastructure. Integration testing confirms
    // the fix works correctly.
}
