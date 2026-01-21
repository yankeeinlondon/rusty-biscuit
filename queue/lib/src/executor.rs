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

use std::process::Stdio;

use chrono::Utc;
use tokio::process::Command;
use tokio::sync::mpsc;
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
pub struct TaskExecutor {
    event_tx: mpsc::Sender<TaskEvent>,
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
        Self { event_tx }
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
        tokio::spawn(async move {
            Self::execute_task(task, tx).await;
        });
    }

    /// Internal implementation of task execution.
    async fn execute_task(task: ScheduledTask, tx: mpsc::Sender<TaskEvent>) {
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

        // Execute based on target
        let result = match task.target {
            ExecutionTarget::NewPane => Self::execute_in_pane(&task.command).await,
            ExecutionTarget::NewWindow => Self::execute_in_window(&task.command).await,
            ExecutionTarget::Background => Self::execute_background(&task.command).await,
        };

        // Report completion status
        let status = match result {
            Ok(()) => TaskStatus::Completed,
            Err(e) => TaskStatus::Failed { error: e },
        };

        let _ = tx.send(TaskEvent::StatusChanged { id: task.id, status }).await;
    }

    /// Executes a command in a new Wezterm pane.
    ///
    /// If not running in Wezterm, falls back to opening a new window.
    async fn execute_in_pane(command: &str) -> Result<(), String> {
        // Check if we're in Wezterm
        if !TerminalDetector::is_wezterm() {
            return Self::execute_in_window(command).await;
        }

        let status = Command::new("wezterm")
            .args(["cli", "split-pane", "--top", "--", "/bin/sh", "-c", command])
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
                    command.replace('"', r#"\""#)
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
                    command.replace('"', r#"\""#)
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
            TerminalKind::Xterm | TerminalKind::Unknown => {
                // Fallback: try xterm
                Command::new("xterm")
                    .args(["-e", "/bin/sh", "-c", command])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
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
}
