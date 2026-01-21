//! Application state for the TUI.

use queue_lib::{
    HistoryStore,
    JsonFileStore,
    ScheduledTask,
    TaskEvent,
    TaskExecutor,
    TerminalCapabilities,
    TerminalDetector,
};
use ratatui::widgets::TableState;
use tokio::sync::mpsc;

use super::history_modal::HistoryModal;
use super::input_modal::InputModal;

/// Application mode determining keyboard behavior and UI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    /// Normal mode - standard navigation and commands.
    #[default]
    Normal,
    /// Edit mode - modifying a selected task.
    #[allow(dead_code)]
    EditMode,
    /// Remove mode - confirming task removal.
    RemoveMode,
    /// Input modal - entering a new task.
    InputModal,
    /// History modal - viewing completed tasks.
    HistoryModal,
    /// Confirm quit - awaiting Y/N to exit.
    ConfirmQuit,
}

/// TUI application state.
pub struct App {
    /// List of scheduled tasks.
    pub tasks: Vec<ScheduledTask>,
    /// Current application mode.
    pub mode: AppMode,
    /// Index of the currently selected task in the list.
    pub selected_index: usize,
    /// Flag indicating the application should exit.
    pub should_quit: bool,
    /// Receiver for task events from the executor.
    pub event_rx: Option<mpsc::Receiver<TaskEvent>>,
    /// Task executor for scheduling tasks.
    pub executor: Option<TaskExecutor>,
    /// State for the task table widget (tracks selection for rendering).
    pub table_state: TableState,
    /// Input modal state (present when InputModal mode is active).
    pub input_modal: Option<InputModal>,
    /// History modal state (present when HistoryModal mode is active).
    pub history_modal: Option<HistoryModal>,
    /// Terminal capabilities - determines available execution targets.
    pub capabilities: TerminalCapabilities,
    /// History store for persisting tasks (optional for test isolation).
    pub history_store: Option<JsonFileStore>,
    /// Next task ID to allocate for new tasks.
    pub next_task_id: u64,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new application state with default values.
    ///
    /// Detects terminal capabilities at startup to determine which
    /// execution targets are available (panes, windows, background).
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            tasks: Vec::new(),
            mode: AppMode::Normal,
            selected_index: 0,
            should_quit: false,
            event_rx: None,
            executor: None,
            table_state,
            input_modal: None,
            history_modal: None,
            capabilities: TerminalDetector::detect(),
            history_store: None,
            next_task_id: 1,
        }
    }

    /// Initializes the app with an executor and event channel.
    ///
    /// This sets up the task execution infrastructure for scheduling
    /// and receiving status updates.
    pub fn with_executor(mut self) -> Self {
        let (tx, rx) = mpsc::channel(100);
        self.event_rx = Some(rx);
        self.executor = Some(TaskExecutor::new(tx));
        self
    }

    /// Adds a history store for persisting tasks.
    pub fn with_history_store(mut self, store: JsonFileStore) -> Self {
        let next_task_id = match store.load_all() {
            Ok(tasks) => tasks.iter().map(|task| task.id).max().unwrap_or(0).saturating_add(1),
            Err(err) => {
                tracing::warn!(error = %err, "Failed to load history for task IDs");
                self.next_task_id
            }
        };
        self.history_store = Some(store);
        self.next_task_id = next_task_id.max(self.next_task_id);
        self
    }

    /// Schedules a task for execution with the executor.
    ///
    /// If no executor is configured, the task is added but not scheduled.
    pub fn schedule_task(&mut self, task: ScheduledTask) {
        if let Some(ref executor) = self.executor {
            executor.schedule(task.clone());
        }
        self.save_history(&task);
        self.tasks.push(task);
        if let Some(next_id) = self.tasks.iter().map(|t| t.id).max().and_then(|id| id.checked_add(1)) {
            self.next_task_id = self.next_task_id.max(next_id);
        }
    }

    /// Allocates the next task ID.
    pub fn alloc_task_id(&mut self) -> u64 {
        let id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        id
    }

    /// Updates an existing task and reschedules if pending.
    pub fn update_task(
        &mut self,
        task_id: u64,
        command: String,
        scheduled_at: chrono::DateTime<chrono::Utc>,
        target: queue_lib::ExecutionTarget,
    ) -> bool {
        let (updated_task, should_reschedule) = if let Some(task) =
            self.tasks.iter_mut().find(|t| t.id == task_id)
        {
            let was_pending = task.is_pending();
            task.command = command;
            task.scheduled_at = scheduled_at;
            task.target = target;
            (task.clone(), was_pending)
        } else {
            return false;
        };

        if should_reschedule {
            if let Some(ref executor) = self.executor {
                let _ = executor.cancel_task(task_id);
                executor.schedule(updated_task.clone());
            }
        }

        self.update_history(&updated_task);
        true
    }

    /// Cancels a pending task by removing it from the list.
    ///
    /// Only pending tasks can be cancelled. Returns true if the task was
    /// cancelled, false if the task was not found or not pending.
    pub fn cancel_task(&mut self, id: u64) -> bool {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == id && t.is_pending()) {
            let cancelled = match self.executor.as_ref() {
                Some(executor) => executor.cancel_task(id),
                None => true,
            };

            if !cancelled {
                return false;
            }

            let mut task = self.tasks.remove(pos);
            task.mark_cancelled();
            self.update_history(&task);
            // Adjust selection if needed
            if self.selected_index >= self.tasks.len() && !self.tasks.is_empty() {
                self.selected_index = self.tasks.len() - 1;
                self.table_state.select(Some(self.selected_index));
            }
            true
        } else {
            false
        }
    }

    /// Selects the next task in the list (wraps around).
    pub fn select_next(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.tasks.len();
            self.table_state.select(Some(self.selected_index));
        }
    }

    /// Selects the previous task in the list (stops at 0).
    pub fn select_previous(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_index = self.selected_index.saturating_sub(1);
            self.table_state.select(Some(self.selected_index));
        }
    }

    /// Returns the currently selected task, if any.
    pub fn selected_task(&self) -> Option<&ScheduledTask> {
        self.tasks.get(self.selected_index)
    }

    /// Handles a task event from the executor.
    pub fn handle_task_event(&mut self, event: TaskEvent) {
        match event {
            TaskEvent::StatusChanged { id, status } => {
                let updated_task = if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id)
                {
                    task.status = status;
                    Some(task.clone())
                } else {
                    None
                };

                if let Some(task) = updated_task {
                    self.update_history(&task);
                }
            }
        }
    }

    fn save_history(&self, task: &ScheduledTask) {
        if let Some(ref store) = self.history_store {
            if let Err(err) = store.save(task) {
                tracing::warn!(error = %err, "Failed to persist task history");
            }
        }
    }

    fn update_history(&self, task: &ScheduledTask) {
        if let Some(ref store) = self.history_store {
            if let Err(err) = store.update(task) {
                tracing::warn!(error = %err, "Failed to update task history");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use queue_lib::{ExecutionTarget, TaskStatus};

    fn make_task(id: u64, command: &str) -> ScheduledTask {
        ScheduledTask::new(
            id,
            command.to_string(),
            Utc::now(),
            ExecutionTarget::default(),
        )
    }

    #[test]
    fn new_app_has_default_state() {
        let app = App::new();
        assert!(app.tasks.is_empty());
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(app.event_rx.is_none());
        assert_eq!(app.table_state.selected(), Some(0));
    }

    #[test]
    fn select_next_wraps_around() {
        let mut app = App::new();
        app.tasks = vec![make_task(1, "a"), make_task(2, "b"), make_task(3, "c")];

        assert_eq!(app.selected_index, 0);
        assert_eq!(app.table_state.selected(), Some(0));

        app.select_next();
        assert_eq!(app.selected_index, 1);
        assert_eq!(app.table_state.selected(), Some(1));

        app.select_next();
        assert_eq!(app.selected_index, 2);
        assert_eq!(app.table_state.selected(), Some(2));

        app.select_next();
        assert_eq!(app.selected_index, 0); // Wraps around
        assert_eq!(app.table_state.selected(), Some(0));
    }

    #[test]
    fn select_previous_stops_at_zero() {
        let mut app = App::new();
        app.tasks = vec![make_task(1, "a"), make_task(2, "b")];
        app.selected_index = 1;
        app.table_state.select(Some(1));

        app.select_previous();
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.table_state.selected(), Some(0));

        app.select_previous();
        assert_eq!(app.selected_index, 0); // Stays at 0
        assert_eq!(app.table_state.selected(), Some(0));
    }

    #[test]
    fn select_on_empty_list_is_noop() {
        let mut app = App::new();
        app.select_next();
        assert_eq!(app.selected_index, 0);
        app.select_previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn selected_task_returns_correct_task() {
        let mut app = App::new();
        app.tasks = vec![make_task(1, "first"), make_task(2, "second")];

        assert_eq!(app.selected_task().unwrap().command, "first");
        app.select_next();
        assert_eq!(app.selected_task().unwrap().command, "second");
    }

    #[test]
    fn selected_task_returns_none_when_empty() {
        let app = App::new();
        assert!(app.selected_task().is_none());
    }

    #[test]
    fn handle_status_changed_updates_task() {
        let mut app = App::new();
        let task = make_task(42, "test");
        app.tasks.push(task);

        app.handle_task_event(TaskEvent::StatusChanged {
            id: 42,
            status: TaskStatus::Running,
        });

        assert!(app.tasks[0].is_running());
    }

    #[test]
    fn handle_status_changed_ignores_unknown_id() {
        let mut app = App::new();
        let task = make_task(42, "test");
        app.tasks.push(task);

        // Should not panic or affect existing task
        app.handle_task_event(TaskEvent::StatusChanged {
            id: 999,
            status: TaskStatus::Completed,
        });

        assert!(app.tasks[0].is_pending());
    }

    #[test]
    fn schedule_task_adds_to_list() {
        let mut app = App::new();
        assert!(app.tasks.is_empty());

        let task = make_task(1, "new task");
        app.schedule_task(task);

        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].command, "new task");
    }

    #[test]
    fn cancel_task_removes_pending_task() {
        let mut app = App::new();
        app.tasks = vec![make_task(1, "a"), make_task(2, "b"), make_task(3, "c")];

        assert!(app.cancel_task(2));
        assert_eq!(app.tasks.len(), 2);
        assert!(app.tasks.iter().all(|t| t.id != 2));
    }

    #[test]
    fn cancel_task_returns_false_for_unknown_id() {
        let mut app = App::new();
        app.tasks = vec![make_task(1, "a")];

        assert!(!app.cancel_task(999));
        assert_eq!(app.tasks.len(), 1);
    }

    #[test]
    fn cancel_task_adjusts_selection() {
        let mut app = App::new();
        app.tasks = vec![make_task(1, "a"), make_task(2, "b")];
        app.selected_index = 1;
        app.table_state.select(Some(1));

        assert!(app.cancel_task(2));
        // Selection should adjust to the last valid index
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.table_state.selected(), Some(0));
    }

    #[test]
    fn cancel_task_fails_for_non_pending() {
        let mut app = App::new();
        let mut task = make_task(1, "a");
        task.mark_running();
        app.tasks.push(task);

        // Cannot cancel a running task
        assert!(!app.cancel_task(1));
        assert_eq!(app.tasks.len(), 1);
    }

    #[test]
    fn with_executor_sets_up_channel() {
        let app = App::new().with_executor();
        assert!(app.event_rx.is_some());
        assert!(app.executor.is_some());
    }
}
