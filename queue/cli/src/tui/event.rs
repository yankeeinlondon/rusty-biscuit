//! Event handling and main application loop.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use super::app::{App, AppMode};
use super::history_modal::HistoryModal;
use super::input_modal::{InputField, InputModal, ScheduleType};
use super::render;

/// Runs the TUI application main loop.
///
/// This function handles:
/// - Rendering the UI on each frame
/// - Processing async task events
/// - Handling keyboard input
/// - Graceful shutdown
///
/// ## Errors
///
/// Returns an I/O error if terminal operations fail.
pub fn run_app(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render::render(app, frame))?;

        // Check for task events (non-blocking)
        // Collect events first to avoid borrow conflicts
        let events: Vec<_> = app
            .event_rx
            .as_mut()
            .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
            .unwrap_or_default();

        for event in events {
            app.handle_task_event(event);
        }

        // Poll for keyboard input with timeout (allows task event processing)
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            // Only handle Press events (Windows compatibility)
            if key.kind != KeyEventKind::Press {
                continue;
            }

            handle_input(app, key.code, key.modifiers);

            if app.should_quit {
                return Ok(());
            }
        }
    }
}

/// Routes keyboard input to the appropriate mode handler.
fn handle_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    match app.mode {
        AppMode::Normal => handle_normal_mode(app, key),
        AppMode::ConfirmQuit => handle_confirm_quit(app, key),
        AppMode::EditMode => handle_edit_mode(app, key),
        AppMode::RemoveMode => handle_remove_mode(app, key),
        AppMode::InputModal => handle_input_modal(app, key, modifiers),
        AppMode::HistoryModal => handle_history_modal(app, key),
    }
}

/// Handles keyboard input in normal mode.
fn handle_normal_mode(app: &mut App, key: KeyCode) {
    match key {
        // Quit commands
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            let has_active_tasks = app
                .tasks
                .iter()
                .any(|task| task.is_pending() || task.is_running());
            if has_active_tasks {
                app.mode = AppMode::ConfirmQuit;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Esc => {
            // Immediate exit without confirmation
            app.should_quit = true;
        }

        // Navigation
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_previous();
        }

        // Mode switching
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.input_modal = Some(InputModal::new(app.capabilities.clone()));
            app.mode = AppMode::InputModal;
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if let Some(task) = app.selected_task().cloned() {
                app.input_modal = Some(InputModal::for_edit(&task, app.capabilities.clone()));
                app.mode = AppMode::InputModal;
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.mode = AppMode::RemoveMode;
        }
        KeyCode::Char('h') | KeyCode::Char('H') => {
            app.history_modal = Some(HistoryModal::new());
            app.mode = AppMode::HistoryModal;
        }

        // Cancel selected pending task
        KeyCode::Char('x') | KeyCode::Char('X') => {
            if let Some(task) = app.selected_task() {
                let task_id = task.id;
                if task.is_pending() {
                    app.cancel_task(task_id);
                }
            }
        }

        _ => {}
    }
}

/// Handles keyboard input in the quit confirmation dialog.
fn handle_confirm_quit(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.should_quit = true;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

/// Handles keyboard input in edit mode.
/// Will be fully implemented in Phase 6.
fn handle_edit_mode(app: &mut App, key: KeyCode) {
    if key == KeyCode::Esc {
        app.mode = AppMode::Normal;
    }
}

/// Handles keyboard input in remove mode.
/// Will be fully implemented in Phase 6.
fn handle_remove_mode(app: &mut App, key: KeyCode) {
    if key == KeyCode::Esc {
        app.mode = AppMode::Normal;
    }
}

/// Handles keyboard input in the input modal.
fn handle_input_modal(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    let Some(modal) = app.input_modal.as_mut() else {
        app.mode = AppMode::Normal;
        return;
    };

    match key {
        KeyCode::Esc => {
            app.input_modal = None;
            app.mode = AppMode::Normal;
        }
        KeyCode::Tab => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                modal.prev_field();
            } else {
                modal.next_field();
            }
        }
        KeyCode::BackTab => {
            modal.prev_field();
        }
        KeyCode::Enter => {
            let action = if modal.validate().is_ok() {
                let scheduled_at = match modal.schedule_type {
                    ScheduleType::AtTime => {
                        use chrono::{Local, TimeZone, Utc};
                        let time = queue_lib::parse_at_time(&modal.schedule_value).unwrap();
                        // Combine with today's date, in local time, then convert to UTC
                        let today = Local::now().date_naive();
                        let local_dt = today.and_time(time);
                        // If the time is in the past, schedule for tomorrow
                        let local_dt = Local.from_local_datetime(&local_dt).single().unwrap();
                        let scheduled = if local_dt < Local::now() {
                            local_dt + chrono::Duration::days(1)
                        } else {
                            local_dt
                        };
                        scheduled.with_timezone(&Utc)
                    }
                    ScheduleType::AfterDelay => {
                        use chrono::Utc;
                        let delay = queue_lib::parse_delay(&modal.schedule_value).unwrap();
                        Utc::now() + delay
                    }
                };

                Some((
                    modal.editing_task_id,
                    modal.command.clone(),
                    scheduled_at,
                    modal.target,
                ))
            } else {
                None
            };

            if let Some((editing_task_id, command, scheduled_at, target)) = action {
                if let Some(task_id) = editing_task_id {
                    app.update_task(task_id, command, scheduled_at, target);
                } else {
                    let id = app.alloc_task_id();
                    let task = queue_lib::ScheduledTask::new(id, command, scheduled_at, target);
                    app.schedule_task(task);
                }

                app.input_modal = None;
                app.mode = AppMode::Normal;
            }
        }
        KeyCode::Backspace => {
            modal.handle_backspace();
        }
        KeyCode::Left | KeyCode::Right => {
            // Handle cursor movement for command field
            if modal.active_field == InputField::Command {
                match key {
                    KeyCode::Left if modal.cursor_pos > 0 => {
                        modal.cursor_pos -= 1;
                    }
                    KeyCode::Right if modal.cursor_pos < modal.command.len() => {
                        modal.cursor_pos += 1;
                    }
                    _ => {}
                }
            } else if matches!(
                modal.active_field,
                InputField::ScheduleType | InputField::Target
            ) {
                // Left/Right toggle selector fields
                if modal.active_field == InputField::ScheduleType {
                    modal.toggle_schedule_type();
                } else {
                    modal.cycle_target();
                }
            }
        }
        KeyCode::Char(' ') => {
            // Space toggles selector fields
            match modal.active_field {
                InputField::ScheduleType => modal.toggle_schedule_type(),
                InputField::Target => modal.cycle_target(),
                _ => modal.handle_char(' '),
            }
        }
        KeyCode::Char(c) => {
            modal.handle_char(c);
        }
        _ => {}
    }
}

/// Handles keyboard input in the history modal.
fn handle_history_modal(app: &mut App, key: KeyCode) {
    let Some(modal) = app.history_modal.as_mut() else {
        app.mode = AppMode::Normal;
        return;
    };

    // In filter mode, handle text input specially
    if modal.filter_mode {
        match key {
            KeyCode::Esc => {
                modal.filter_mode = false;
                modal.filter.clear();
                // Reset selection to first item
                if !modal.items.is_empty() {
                    modal.list_state.select(Some(0));
                }
            }
            KeyCode::Enter => {
                modal.filter_mode = false;
            }
            KeyCode::Backspace => {
                modal.handle_backspace();
            }
            KeyCode::Char(c) => {
                modal.handle_char(c);
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Esc => {
            app.history_modal = None;
            app.mode = AppMode::Normal;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            modal.select_next();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            modal.select_previous();
        }
        KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Char('/') => {
            modal.filter_mode = true;
        }
        KeyCode::Enter => {
            // Open input modal with the selected command pre-filled
            if let Some(task) = modal.selected_task().cloned() {
                let mut input = InputModal::new(app.capabilities.clone());
                input.command = task.command;
                input.cursor_pos = input.command.len();
                app.input_modal = Some(input);
                app.history_modal = None;
                app.mode = AppMode::InputModal;
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            // Open input modal with the selected command as template for a new task
            if let Some(task) = modal.selected_task().cloned() {
                let mut input = InputModal::new(app.capabilities.clone());
                input.command = task.command;
                input.cursor_pos = input.command.len();
                // Use same target as the historical task
                input.target = task.target;
                app.input_modal = Some(input);
                app.history_modal = None;
                app.mode = AppMode::InputModal;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use queue_lib::{TerminalCapabilities, TerminalKind};

    fn input(app: &mut App, key: KeyCode) {
        handle_input(app, key, KeyModifiers::NONE);
    }

    /// Creates capabilities for a terminal with pane support (like Wezterm).
    fn wezterm_caps() -> TerminalCapabilities {
        TerminalCapabilities {
            kind: TerminalKind::Wezterm,
            supports_panes: true,
            supports_new_window: true,
        }
    }

    #[test]
    fn normal_mode_q_triggers_confirm_quit() {
        let mut app = App::new();
        use chrono::Utc;
        use queue_lib::{ExecutionTarget, ScheduledTask};
        app.tasks.push(ScheduledTask::new(
            1,
            "a".into(),
            Utc::now(),
            ExecutionTarget::default(),
        ));
        input(&mut app, KeyCode::Char('q'));
        assert_eq!(app.mode, AppMode::ConfirmQuit);
        assert!(!app.should_quit);
    }

    #[test]
    fn normal_mode_q_exits_without_active_tasks() {
        let mut app = App::new();
        input(&mut app, KeyCode::Char('q'));
        assert!(app.should_quit);
    }

    #[test]
    fn normal_mode_esc_quits_immediately() {
        let mut app = App::new();
        input(&mut app, KeyCode::Esc);
        assert!(app.should_quit);
    }

    #[test]
    fn confirm_quit_y_exits() {
        let mut app = App::new();
        app.mode = AppMode::ConfirmQuit;
        input(&mut app, KeyCode::Char('y'));
        assert!(app.should_quit);
    }

    #[test]
    fn confirm_quit_enter_exits() {
        let mut app = App::new();
        app.mode = AppMode::ConfirmQuit;
        input(&mut app, KeyCode::Enter);
        assert!(app.should_quit);
    }

    #[test]
    fn confirm_quit_n_returns_to_normal() {
        let mut app = App::new();
        app.mode = AppMode::ConfirmQuit;
        input(&mut app, KeyCode::Char('n'));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn confirm_quit_esc_returns_to_normal() {
        let mut app = App::new();
        app.mode = AppMode::ConfirmQuit;
        input(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn normal_mode_navigation_keys() {
        let mut app = App::new();
        // Add tasks so navigation has effect
        use chrono::Utc;
        use queue_lib::{ExecutionTarget, ScheduledTask};

        app.tasks = vec![
            ScheduledTask::new(1, "a".into(), Utc::now(), ExecutionTarget::default()),
            ScheduledTask::new(2, "b".into(), Utc::now(), ExecutionTarget::default()),
        ];

        assert_eq!(app.selected_index, 0);

        input(&mut app, KeyCode::Down);
        assert_eq!(app.selected_index, 1);

        input(&mut app, KeyCode::Up);
        assert_eq!(app.selected_index, 0);

        input(&mut app, KeyCode::Char('j'));
        assert_eq!(app.selected_index, 1);

        input(&mut app, KeyCode::Char('k'));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn normal_mode_switches_to_input_modal() {
        let mut app = App::new();
        input(&mut app, KeyCode::Char('n'));
        assert_eq!(app.mode, AppMode::InputModal);
        assert!(app.input_modal.is_some());
    }

    #[test]
    fn normal_mode_e_only_opens_modal_with_selected_task() {
        let mut app = App::new();
        // E with no tasks should not open modal
        input(&mut app, KeyCode::Char('e'));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.input_modal.is_none());

        // Add a task and try again
        use chrono::Utc;
        use queue_lib::{ExecutionTarget, ScheduledTask};
        app.tasks
            .push(ScheduledTask::new(1, "test".into(), Utc::now(), ExecutionTarget::default()));

        input(&mut app, KeyCode::Char('e'));
        assert_eq!(app.mode, AppMode::InputModal);
        assert!(app.input_modal.is_some());
        assert!(app.input_modal.as_ref().unwrap().editing_task_id.is_some());
    }

    #[test]
    fn normal_mode_switches_to_remove_mode() {
        let mut app = App::new();
        input(&mut app, KeyCode::Char('r'));
        assert_eq!(app.mode, AppMode::RemoveMode);
    }

    #[test]
    fn normal_mode_switches_to_history_modal() {
        let mut app = App::new();
        input(&mut app, KeyCode::Char('h'));
        assert_eq!(app.mode, AppMode::HistoryModal);
        assert!(app.history_modal.is_some());
    }

    #[test]
    fn input_modal_escape_clears_modal() {
        let mut app = App::new();
        app.input_modal = Some(InputModal::new(wezterm_caps()));
        app.mode = AppMode::InputModal;

        input(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.input_modal.is_none());
    }

    #[test]
    fn input_modal_tab_cycles_fields() {
        let mut app = App::new();
        app.input_modal = Some(InputModal::new(wezterm_caps()));
        app.mode = AppMode::InputModal;

        assert_eq!(
            app.input_modal.as_ref().unwrap().active_field,
            InputField::Command
        );
        input(&mut app, KeyCode::Tab);
        assert_eq!(
            app.input_modal.as_ref().unwrap().active_field,
            InputField::ScheduleType
        );
    }

    #[test]
    fn input_modal_enter_creates_task_on_valid_input() {
        let mut app = App::new();
        let mut modal = InputModal::new(wezterm_caps());
        modal.command = "echo hello".to_string();
        modal.schedule_value = "15m".to_string();
        modal.schedule_type = ScheduleType::AfterDelay;
        app.input_modal = Some(modal);
        app.mode = AppMode::InputModal;

        input(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.input_modal.is_none());
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].command, "echo hello");
    }

    #[test]
    fn input_modal_enter_shows_error_on_invalid_input() {
        let mut app = App::new();
        app.input_modal = Some(InputModal::new(wezterm_caps())); // Empty command
        app.mode = AppMode::InputModal;

        input(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, AppMode::InputModal); // Still in modal
        assert!(app.input_modal.as_ref().unwrap().error_message.is_some());
    }

    #[test]
    fn all_modes_escape_to_normal() {
        for mode in [AppMode::EditMode, AppMode::RemoveMode, AppMode::HistoryModal] {
            let mut app = App::new();
            app.mode = mode;
            input(&mut app, KeyCode::Esc);
            assert_eq!(app.mode, AppMode::Normal);
        }
    }

    #[test]
    fn input_modal_escape_returns_to_normal() {
        let mut app = App::new();
        app.input_modal = Some(InputModal::new(wezterm_caps()));
        app.mode = AppMode::InputModal;
        input(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.input_modal.is_none());
    }

    #[test]
    fn history_modal_escape_clears_modal() {
        use ratatui::widgets::ListState;

        let mut app = App::new();
        app.history_modal = Some(HistoryModal {
            items: vec![],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: false,
        });
        app.mode = AppMode::HistoryModal;

        input(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.history_modal.is_none());
    }

    #[test]
    fn history_modal_navigation() {
        use chrono::Utc;
        use queue_lib::{ExecutionTarget, ScheduledTask, TaskStatus};
        use ratatui::widgets::ListState;

        let mut app = App::new();
        let mut state = ListState::default();
        state.select(Some(0));
        app.history_modal = Some(HistoryModal {
            items: vec![
                ScheduledTask {
                    id: 1,
                    command: "echo 1".to_string(),
                    scheduled_at: Utc::now(),
                    target: ExecutionTarget::Background,
                    status: TaskStatus::Completed,
                    created_at: Utc::now(),
                },
                ScheduledTask {
                    id: 2,
                    command: "echo 2".to_string(),
                    scheduled_at: Utc::now(),
                    target: ExecutionTarget::Background,
                    status: TaskStatus::Completed,
                    created_at: Utc::now(),
                },
            ],
            list_state: state,
            filter: String::new(),
            filter_mode: false,
        });
        app.mode = AppMode::HistoryModal;

        assert_eq!(app.history_modal.as_ref().unwrap().list_state.selected(), Some(0));

        input(&mut app, KeyCode::Down);
        assert_eq!(app.history_modal.as_ref().unwrap().list_state.selected(), Some(1));

        input(&mut app, KeyCode::Up);
        assert_eq!(app.history_modal.as_ref().unwrap().list_state.selected(), Some(0));
    }

    #[test]
    fn history_modal_enter_opens_input_modal() {
        use chrono::Utc;
        use queue_lib::{ExecutionTarget, ScheduledTask, TaskStatus};
        use ratatui::widgets::ListState;

        let mut app = App::new();
        let mut state = ListState::default();
        state.select(Some(0));
        app.history_modal = Some(HistoryModal {
            items: vec![ScheduledTask {
                id: 1,
                command: "echo hello".to_string(),
                scheduled_at: Utc::now(),
                target: ExecutionTarget::Background,
                status: TaskStatus::Completed,
                created_at: Utc::now(),
            }],
            list_state: state,
            filter: String::new(),
            filter_mode: false,
        });
        app.mode = AppMode::HistoryModal;

        input(&mut app, KeyCode::Enter);

        assert_eq!(app.mode, AppMode::InputModal);
        assert!(app.history_modal.is_none());
        assert!(app.input_modal.is_some());
        assert_eq!(app.input_modal.as_ref().unwrap().command, "echo hello");
    }

    #[test]
    fn history_modal_f_enters_filter_mode() {
        use ratatui::widgets::ListState;

        let mut app = App::new();
        app.history_modal = Some(HistoryModal {
            items: vec![],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: false,
        });
        app.mode = AppMode::HistoryModal;

        assert!(!app.history_modal.as_ref().unwrap().filter_mode);
        input(&mut app, KeyCode::Char('f'));
        assert!(app.history_modal.as_ref().unwrap().filter_mode);
    }

    #[test]
    fn normal_mode_x_cancels_pending_task() {
        use chrono::Utc;
        use queue_lib::{ExecutionTarget, ScheduledTask};

        let mut app = App::new();
        app.tasks = vec![
            ScheduledTask::new(1, "a".into(), Utc::now(), ExecutionTarget::default()),
            ScheduledTask::new(2, "b".into(), Utc::now(), ExecutionTarget::default()),
        ];
        app.selected_index = 1;
        app.table_state.select(Some(1));

        assert_eq!(app.tasks.len(), 2);
        input(&mut app, KeyCode::Char('x'));
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].id, 1); // Task 2 was removed
    }

    #[test]
    fn normal_mode_x_does_not_cancel_running_task() {
        use chrono::Utc;
        use queue_lib::{ExecutionTarget, ScheduledTask};

        let mut app = App::new();
        let mut task = ScheduledTask::new(1, "a".into(), Utc::now(), ExecutionTarget::default());
        task.mark_running();
        app.tasks.push(task);

        input(&mut app, KeyCode::Char('x'));
        assert_eq!(app.tasks.len(), 1); // Task not cancelled
    }
}
