//! Core data types for the queue system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// How the task was scheduled - affects display in the WHEN column.
///
/// - `AtTime`: User specified a clock time (e.g., "7:00am"). Shows the time
///   until within 1 minute of execution, then switches to countdown.
/// - `AfterDelay`: User specified a duration (e.g., "15m"). Always shows
///   countdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    /// Scheduled at a specific time (e.g., "7:00am").
    AtTime,
    /// Scheduled after a delay (e.g., "15m").
    AfterDelay,
}

/// Where to execute a scheduled task.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTarget {
    /// Execute in a new Wezterm pane.
    #[default]
    NewPane,
    /// Execute in a new native terminal window.
    NewWindow,
    /// Execute as a detached background process.
    Background,
}

/// The execution status of a scheduled task.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum TaskStatus {
    /// Task is waiting to be executed.
    #[default]
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task was cancelled before execution.
    Cancelled,
    /// Task failed with an error.
    Failed {
        /// The error message describing why the task failed.
        error: String,
    },
}

/// A scheduled task in the queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Unique identifier for the task.
    pub id: u64,
    /// The command to execute.
    pub command: String,
    /// When the task is scheduled to run.
    pub scheduled_at: DateTime<Utc>,
    /// Where to execute the task.
    pub target: ExecutionTarget,
    /// Current status of the task.
    pub status: TaskStatus,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
    /// How the task was scheduled (time vs duration).
    ///
    /// This affects display: time-based schedules show the clock time until
    /// within 1 minute, then switch to countdown. Duration-based schedules
    /// always show countdown.
    ///
    /// `None` for backwards compatibility with tasks created before this field
    /// existed - treated as `AfterDelay` (countdown display).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_kind: Option<ScheduleKind>,
}

impl ScheduledTask {
    /// Creates a new pending task.
    ///
    /// ## Examples
    ///
    /// ```
    /// use queue_lib::{ScheduledTask, ExecutionTarget};
    /// use chrono::Utc;
    ///
    /// let task = ScheduledTask::new(
    ///     1,
    ///     "echo hello".to_string(),
    ///     Utc::now(),
    ///     ExecutionTarget::NewPane,
    /// );
    /// assert_eq!(task.command, "echo hello");
    /// ```
    pub fn new(
        id: u64,
        command: String,
        scheduled_at: DateTime<Utc>,
        target: ExecutionTarget,
    ) -> Self {
        Self {
            id,
            command,
            scheduled_at,
            target,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            schedule_kind: None,
        }
    }

    /// Creates a new pending task with a specific schedule kind.
    ///
    /// Use this when you know whether the task was scheduled at a specific
    /// time or after a delay, so the WHEN column can display appropriately.
    ///
    /// ## Examples
    ///
    /// ```
    /// use queue_lib::{ScheduledTask, ExecutionTarget, ScheduleKind};
    /// use chrono::Utc;
    ///
    /// // Task scheduled at a specific time - will show "07:00" in WHEN column
    /// let task = ScheduledTask::with_schedule_kind(
    ///     1,
    ///     "echo hello".to_string(),
    ///     Utc::now(),
    ///     ExecutionTarget::NewPane,
    ///     ScheduleKind::AtTime,
    /// );
    /// assert_eq!(task.schedule_kind, Some(ScheduleKind::AtTime));
    /// ```
    pub fn with_schedule_kind(
        id: u64,
        command: String,
        scheduled_at: DateTime<Utc>,
        target: ExecutionTarget,
        schedule_kind: ScheduleKind,
    ) -> Self {
        Self {
            id,
            command,
            scheduled_at,
            target,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            schedule_kind: Some(schedule_kind),
        }
    }

    /// Marks the task as running.
    pub fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
    }

    /// Marks the task as completed.
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
    }

    /// Marks the task as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = TaskStatus::Cancelled;
    }

    /// Marks the task as failed with the given error.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed {
            error: error.into(),
        };
    }

    /// Returns true if the task is pending.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, TaskStatus::Pending)
    }

    /// Returns true if the task is running.
    pub fn is_running(&self) -> bool {
        matches!(self.status, TaskStatus::Running)
    }

    /// Returns true if the task is completed.
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    /// Returns true if the task has failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.status, TaskStatus::Failed { .. })
    }

    /// Returns true if the task was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self.status, TaskStatus::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn execution_target_serializes_correctly() {
        let json = serde_json::to_string(&ExecutionTarget::NewPane).unwrap();
        assert_eq!(json, r#""new_pane""#);

        let json = serde_json::to_string(&ExecutionTarget::NewWindow).unwrap();
        assert_eq!(json, r#""new_window""#);

        let json = serde_json::to_string(&ExecutionTarget::Background).unwrap();
        assert_eq!(json, r#""background""#);
    }

    #[test]
    fn execution_target_deserializes_correctly() {
        let target: ExecutionTarget = serde_json::from_str(r#""new_pane""#).unwrap();
        assert_eq!(target, ExecutionTarget::NewPane);

        let target: ExecutionTarget = serde_json::from_str(r#""new_window""#).unwrap();
        assert_eq!(target, ExecutionTarget::NewWindow);

        let target: ExecutionTarget = serde_json::from_str(r#""background""#).unwrap();
        assert_eq!(target, ExecutionTarget::Background);
    }

    #[test]
    fn task_status_serializes_correctly() {
        let json = serde_json::to_string(&TaskStatus::Pending).unwrap();
        assert_eq!(json, r#"{"status":"pending"}"#);

        let json = serde_json::to_string(&TaskStatus::Running).unwrap();
        assert_eq!(json, r#"{"status":"running"}"#);

        let json = serde_json::to_string(&TaskStatus::Completed).unwrap();
        assert_eq!(json, r#"{"status":"completed"}"#);

        let json = serde_json::to_string(&TaskStatus::Cancelled).unwrap();
        assert_eq!(json, r#"{"status":"cancelled"}"#);

        let json = serde_json::to_string(&TaskStatus::Failed {
            error: "oops".to_string(),
        })
        .unwrap();
        assert_eq!(json, r#"{"status":"failed","error":"oops"}"#);
    }

    #[test]
    fn task_status_deserializes_correctly() {
        let status: TaskStatus = serde_json::from_str(r#"{"status":"pending"}"#).unwrap();
        assert_eq!(status, TaskStatus::Pending);

        let status: TaskStatus = serde_json::from_str(r#"{"status":"running"}"#).unwrap();
        assert_eq!(status, TaskStatus::Running);

        let status: TaskStatus = serde_json::from_str(r#"{"status":"completed"}"#).unwrap();
        assert_eq!(status, TaskStatus::Completed);

        let status: TaskStatus = serde_json::from_str(r#"{"status":"cancelled"}"#).unwrap();
        assert_eq!(status, TaskStatus::Cancelled);

        let status: TaskStatus =
            serde_json::from_str(r#"{"status":"failed","error":"oops"}"#).unwrap();
        assert_eq!(
            status,
            TaskStatus::Failed {
                error: "oops".to_string()
            }
        );
    }

    #[test]
    fn scheduled_task_round_trips_through_json() {
        let scheduled_at = Utc::now() + Duration::hours(1);
        let task = ScheduledTask::new(
            42,
            "cargo build".to_string(),
            scheduled_at,
            ExecutionTarget::NewPane,
        );

        let json = serde_json::to_string(&task).unwrap();
        let restored: ScheduledTask = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, task.id);
        assert_eq!(restored.command, task.command);
        assert_eq!(restored.scheduled_at, task.scheduled_at);
        assert_eq!(restored.target, task.target);
        assert_eq!(restored.status, task.status);
        assert_eq!(restored.created_at, task.created_at);
    }

    #[test]
    fn scheduled_task_status_transitions() {
        let mut task = ScheduledTask::new(
            1,
            "echo test".to_string(),
            Utc::now(),
            ExecutionTarget::Background,
        );

        assert!(task.is_pending());
        assert!(!task.is_running());
        assert!(!task.is_completed());
        assert!(!task.is_failed());

        task.mark_running();
        assert!(!task.is_pending());
        assert!(task.is_running());

        task.mark_completed();
        assert!(task.is_completed());

        task.mark_cancelled();
        assert!(task.is_cancelled());

        task.mark_failed("something went wrong");
        assert!(task.is_failed());
        assert_eq!(
            task.status,
            TaskStatus::Failed {
                error: "something went wrong".to_string()
            }
        );
    }

    #[test]
    fn execution_target_default_is_new_pane() {
        assert_eq!(ExecutionTarget::default(), ExecutionTarget::NewPane);
    }

    #[test]
    fn task_status_default_is_pending() {
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }
}
