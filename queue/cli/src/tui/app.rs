//! Application state for the TUI.

use queue_lib::{ScheduledTask, TaskStatus};
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
    /// Edit mode - modifying a selected task (Phase 7b).
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

/// Events from the task executor to update TUI state.
///
/// Used in Phase 7b for TUI-Executor integration.
#[allow(dead_code)]
pub enum TaskEvent {
    /// A task's status has changed.
    StatusChanged {
        /// The task ID.
        id: u64,
        /// The new status.
        status: TaskStatus,
    },
    /// A new task was added.
    TaskAdded(ScheduledTask),
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
    /// State for the task table widget (tracks selection for rendering).
    pub table_state: TableState,
    /// Input modal state (present when InputModal mode is active).
    pub input_modal: Option<InputModal>,
    /// History modal state (present when HistoryModal mode is active).
    pub history_modal: Option<HistoryModal>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new application state with default values.
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            tasks: Vec::new(),
            mode: AppMode::Normal,
            selected_index: 0,
            should_quit: false,
            event_rx: None,
            table_state,
            input_modal: None,
            history_modal: None,
        }
    }

    /// Sets the event receiver for task updates (Phase 7b).
    #[allow(dead_code)]
    pub fn with_event_receiver(mut self, rx: mpsc::Receiver<TaskEvent>) -> Self {
        self.event_rx = Some(rx);
        self
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
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.status = status;
                }
            }
            TaskEvent::TaskAdded(task) => {
                self.tasks.push(task);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use queue_lib::ExecutionTarget;

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
    fn handle_task_added_appends_task() {
        let mut app = App::new();
        assert!(app.tasks.is_empty());

        let task = make_task(1, "new task");
        app.handle_task_event(TaskEvent::TaskAdded(task));

        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].command, "new task");
    }
}
