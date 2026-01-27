//! Rendering functions for the TUI.
//!
//! This module implements the main window layout with a task table and status footer.

use chrono::Utc;
use queue_lib::{ExecutionTarget, ScheduledTask, TaskStatus};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use super::app::{App, AppMode};
use super::color_context::ColorContext;
use super::modal::{render_modal, ConfirmQuitDialog};
use super::PANEL_BG;

/// Renders the entire application UI.
///
/// The layout consists of:
/// - A task table filling most of the screen
/// - A footer bar showing keyboard shortcuts for the current mode
/// - Modal overlays rendered on top based on current mode
pub fn render(app: &mut App, frame: &mut Frame) {
    // Create color context once for NO_COLOR-aware rendering
    let color_context = ColorContext::new();

    let chunks = Layout::vertical([
        Constraint::Min(3),    // Task table
        Constraint::Length(3), // Footer
    ])
    .split(frame.area());

    render_task_table(app, frame, chunks[0]);
    render_footer(app, frame, chunks[1]);

    // Render modal overlays based on mode
    match app.mode {
        AppMode::ConfirmQuit => {
            render_modal(frame, &ConfirmQuitDialog, frame.area(), &color_context);
        }
        AppMode::InputModal => {
            if let Some(input_modal) = app.input_modal.as_mut() {
                input_modal.update_layout(frame.area().height);
                render_modal(frame, input_modal, frame.area(), &color_context);
            }
        }
        AppMode::HistoryModal => {
            if let Some(history_modal) = app.history_modal.as_mut() {
                history_modal.update_layout(frame.area().height);
                render_modal(frame, history_modal, frame.area(), &color_context);
            }
        }
        _ => {}
    }
}

/// Configuration for responsive column display based on terminal width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColumnConfig {
    /// Whether to show the Status column.
    show_status: bool,
    /// Whether to abbreviate the Where column values.
    abbreviated_target: bool,
}

/// Calculates the column configuration based on available width.
///
/// - Width >= 80: Full display (all columns, full text)
/// - Width 60-79: All columns, abbreviated "Where" values
/// - Width < 60: Hide "Status" column, abbreviated "Where" values
fn calculate_column_config(width: u16) -> ColumnConfig {
    ColumnConfig {
        show_status: width >= 60,
        abbreviated_target: width < 80,
    }
}

/// Renders the task table with columns for ID, WHEN, Command, Where, and Status.
///
/// Displays an empty state message when no tasks are scheduled.
/// Responsive to terminal width:
/// - Width >= 80: All columns with full text
/// - Width 60-79: Abbreviated "Where" column values
/// - Width < 60: Status column hidden
fn render_task_table(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Tasks ");

    // Handle empty state
    if app.tasks.is_empty() {
        let message = if area.width < 40 {
            "Press N to add task"
        } else {
            "No tasks scheduled. Press N to add one."
        };
        let paragraph = Paragraph::new(message)
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
        return;
    }

    let config = calculate_column_config(area.width);

    // Build header based on config
    let header_cells: Vec<Cell> = if config.show_status {
        vec![
            Cell::from("ID"),
            Cell::from("WHEN"),
            Cell::from("Command"),
            Cell::from("Where"),
            Cell::from("Status"),
        ]
    } else {
        vec![
            Cell::from("ID"),
            Cell::from("WHEN"),
            Cell::from("Command"),
            Cell::from("Where"),
        ]
    };
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows = app.tasks.iter().map(|task| {
        let style = task_style(task);
        let cells: Vec<Cell> = if config.show_status {
            vec![
                Cell::from(task.id.to_string()),
                Cell::from(format_schedule(task)),
                Cell::from(task.command.as_str()),
                Cell::from(format_target(&task.target, config.abbreviated_target)),
                Cell::from(format_status(&task.status)),
            ]
        } else {
            vec![
                Cell::from(task.id.to_string()),
                Cell::from(format_schedule(task)),
                Cell::from(task.command.as_str()),
                Cell::from(format_target(&task.target, config.abbreviated_target)),
            ]
        };
        Row::new(cells).style(style)
    });

    let widths: Vec<Constraint> = if config.show_status {
        vec![
            Constraint::Length(4),  // ID
            Constraint::Length(12), // WHEN
            Constraint::Min(20),    // Command
            Constraint::Length(10), // Where
            Constraint::Length(10), // Status
        ]
    } else {
        vec![
            Constraint::Length(4),  // ID
            Constraint::Length(12), // WHEN
            Constraint::Min(20),    // Command
            Constraint::Length(10), // Where
        ]
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

/// Returns the appropriate keyboard shortcuts for Normal mode based on terminal width.
///
/// - Width >= 80: Full labels (e.g., "Remove", "History", "Navigate")
/// - Width 60-79: Abbreviated labels (e.g., "Rm", "Hist", "Nav")
/// - Width < 60: Minimal shortcuts (only most essential actions)
fn get_normal_mode_shortcuts(width: u16) -> Vec<(&'static str, &'static str)> {
    if width >= 80 {
        // Full labels
        vec![
            ("Q", "Quit"),
            ("N", "New"),
            ("E", "Edit"),
            ("R", "Remove"),
            ("X", "Cancel"),
            ("H", "History"),
            ("\u{2191}\u{2193}", "Navigate"),
        ]
    } else if width >= 60 {
        // Medium - shorter labels
        vec![
            ("Q", "Quit"),
            ("N", "New"),
            ("E", "Edit"),
            ("R", "Rm"),
            ("X", "Cancel"),
            ("H", "Hist"),
            ("\u{2191}\u{2193}", "Nav"),
        ]
    } else {
        // Compact - minimal shortcuts
        vec![("Q", "Quit"), ("N", "New"), ("E", "Edit"), ("H", "Hist")]
    }
}

/// Renders the footer with keyboard shortcuts appropriate for the current mode.
///
/// The footer is responsive to terminal width in Normal mode:
/// - Width >= 80: Full shortcut labels
/// - Width 60-79: Abbreviated labels
/// - Width < 60: Minimal set of shortcuts
fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let width = area.width;

    let (bg_color, shortcuts) = match app.mode {
        AppMode::Normal => (PANEL_BG, get_normal_mode_shortcuts(width)),
        AppMode::EditMode => (
            Color::Yellow,
            vec![("Enter", "Edit Selected"), ("Esc", "Cancel")],
        ),
        AppMode::RemoveMode => (
            Color::Red,
            vec![("Enter", "Remove Selected"), ("Esc", "Cancel")],
        ),
        AppMode::ConfirmQuit => (PANEL_BG, vec![("Y", "Yes, Quit"), ("N", "No, Stay")]),
        AppMode::InputModal | AppMode::HistoryModal => (PANEL_BG, vec![("Esc", "Back")]),
    };

    let spans: Vec<Span> = shortcuts
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(
                    format!(" {key} "),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" {desc} ")),
            ]
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().bg(bg_color));

    frame.render_widget(footer, area);
}

/// Returns the appropriate style for a task based on its status.
fn task_style(task: &ScheduledTask) -> Style {
    match task.status {
        TaskStatus::Completed => Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
        TaskStatus::Cancelled => Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::DIM),
        TaskStatus::Running => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        TaskStatus::Failed { .. } => Style::default().fg(Color::Red),
        TaskStatus::Pending => Style::default(),
    }
}

/// Formats a scheduled time as a human-readable relative or absolute string.
///
/// For pending tasks:
/// - `AtTime` schedule: Shows clock time (e.g., "07:00") until within 1 minute,
///   then switches to countdown (e.g., "in 30s")
/// - `AfterDelay` schedule (or legacy tasks): Always shows countdown
///
/// For completed/running tasks, shows the absolute time when scheduled (HH:MM).
fn format_schedule(task: &ScheduledTask) -> String {
    use queue_lib::ScheduleKind;

    let now = Utc::now();
    let duration = task.scheduled_at.signed_duration_since(now);

    // For non-pending tasks, show the scheduled time as HH:MM
    if !task.is_pending() {
        return task
            .scheduled_at
            .with_timezone(&chrono::Local)
            .format("%H:%M")
            .to_string();
    }

    // For pending tasks scheduled at a specific time, show the time
    // until we're within 1 minute, then switch to countdown for precision
    if task.schedule_kind == Some(ScheduleKind::AtTime) && duration.num_seconds() >= 60 {
        return task
            .scheduled_at
            .with_timezone(&chrono::Local)
            .format("%H:%M")
            .to_string();
    }

    // For delay-based schedules (or legacy tasks without schedule_kind),
    // show relative countdown
    if duration.num_seconds() < 0 {
        // Task is overdue but still pending - show as "now"
        "now".to_string()
    } else if duration.num_seconds() < 60 {
        // Less than a minute - show seconds
        format!("in {}s", duration.num_seconds())
    } else if duration.num_hours() < 1 {
        format!("in {}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!(
            "in {}h {}m",
            duration.num_hours(),
            duration.num_minutes() % 60
        )
    } else {
        task.scheduled_at.format("%b %d %H:%M").to_string()
    }
}

/// Formats an execution target as a short label.
///
/// When `abbreviated` is true, returns shorter versions suitable for narrow terminals.
fn format_target(target: &ExecutionTarget, abbreviated: bool) -> &'static str {
    match (target, abbreviated) {
        (ExecutionTarget::NewPane, false) => "new pane",
        (ExecutionTarget::NewPane, true) => "pane",
        (ExecutionTarget::NewWindow, false) => "window",
        (ExecutionTarget::NewWindow, true) => "win",
        (ExecutionTarget::Background, false) => "background",
        (ExecutionTarget::Background, true) => "bg",
    }
}

/// Formats a task status as a short label.
fn format_status(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Completed => "done",
        TaskStatus::Cancelled => "cancelled",
        TaskStatus::Failed { .. } => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_pending_task(scheduled_at: chrono::DateTime<Utc>) -> ScheduledTask {
        ScheduledTask::new(1, "test".to_string(), scheduled_at, ExecutionTarget::default())
    }

    #[test]
    fn format_schedule_pending_past_shows_now() {
        // Regression test: pending task scheduled in the past should show "now", not "in 0m"
        let task = make_pending_task(Utc::now() - Duration::hours(1));
        assert_eq!(format_schedule(&task), "now");
    }

    #[test]
    fn format_schedule_pending_under_one_minute_shows_seconds() {
        // Regression test: pending task < 1 minute away should show seconds, not "in 0m"
        let task = make_pending_task(Utc::now() + Duration::seconds(30));
        let result = format_schedule(&task);
        assert!(result.starts_with("in "));
        assert!(result.ends_with("s"));
    }

    #[test]
    fn format_schedule_pending_under_one_hour() {
        let task = make_pending_task(Utc::now() + Duration::minutes(30));
        let result = format_schedule(&task);
        assert!(result.starts_with("in "));
        assert!(result.ends_with("m"));
        assert!(!result.contains("h"));
    }

    #[test]
    fn format_schedule_pending_under_24_hours() {
        let task = make_pending_task(Utc::now() + Duration::hours(5) + Duration::minutes(15));
        let result = format_schedule(&task);
        assert!(result.contains("h"));
        assert!(result.contains("m"));
    }

    #[test]
    fn format_schedule_pending_over_24_hours() {
        let task = make_pending_task(Utc::now() + Duration::days(2));
        let result = format_schedule(&task);
        // Should be absolute format like "Jan 23 14:30"
        assert!(!result.starts_with("in "));
    }

    #[test]
    fn format_schedule_completed_shows_hh_mm() {
        // Regression test: completed tasks should show HH:MM, not "past"
        let mut task = make_pending_task(Utc::now() - Duration::hours(1));
        task.mark_completed();
        let result = format_schedule(&task);
        // Should be HH:MM format (e.g., "14:30")
        assert!(result.contains(":"), "Expected HH:MM format, got: {}", result);
        assert!(!result.starts_with("in "));
        assert_ne!(result, "past");
        assert_ne!(result, "now");
    }

    #[test]
    fn format_schedule_running_shows_hh_mm() {
        // Running tasks should also show the scheduled time
        let mut task = make_pending_task(Utc::now() - Duration::minutes(5));
        task.mark_running();
        let result = format_schedule(&task);
        assert!(result.contains(":"), "Expected HH:MM format, got: {}", result);
    }

    // =========================================================================
    // Regression tests for schedule_kind display behavior
    // Bug: Tasks scheduled at a specific time (e.g., "7:00am") were showing
    // countdown ("in 3h 15m") instead of the time ("07:00") until the last
    // minute. Duration-based schedules should always show countdown.
    // =========================================================================

    fn make_task_with_schedule_kind(
        scheduled_at: chrono::DateTime<Utc>,
        schedule_kind: Option<queue_lib::ScheduleKind>,
    ) -> ScheduledTask {
        let mut task = ScheduledTask::new(1, "test".to_string(), scheduled_at, ExecutionTarget::default());
        task.schedule_kind = schedule_kind;
        task
    }

    #[test]
    fn format_schedule_at_time_shows_clock_time_when_far() {
        // Task scheduled "at time" should show the clock time when > 1 minute away
        use queue_lib::ScheduleKind;

        let task = make_task_with_schedule_kind(
            Utc::now() + Duration::hours(2),
            Some(ScheduleKind::AtTime),
        );
        let result = format_schedule(&task);

        // Should show time format like "14:30", not "in 2h 0m"
        assert!(result.contains(":"), "AtTime schedule should show clock time, got: {}", result);
        assert!(!result.starts_with("in "), "AtTime schedule should not show countdown, got: {}", result);
    }

    #[test]
    fn format_schedule_at_time_shows_countdown_when_close() {
        // Task scheduled "at time" should switch to countdown when < 1 minute away
        use queue_lib::ScheduleKind;

        let task = make_task_with_schedule_kind(
            Utc::now() + Duration::seconds(30),
            Some(ScheduleKind::AtTime),
        );
        let result = format_schedule(&task);

        // Should show countdown "in 30s" for precision in the final minute
        assert!(result.starts_with("in "), "AtTime schedule should show countdown when close, got: {}", result);
        assert!(result.ends_with("s"), "Should show seconds when < 1 minute, got: {}", result);
    }

    #[test]
    fn format_schedule_after_delay_always_shows_countdown() {
        // Task scheduled with delay should always show countdown
        use queue_lib::ScheduleKind;

        let task = make_task_with_schedule_kind(
            Utc::now() + Duration::hours(2),
            Some(ScheduleKind::AfterDelay),
        );
        let result = format_schedule(&task);

        // Should show countdown "in 2h 0m"
        assert!(result.starts_with("in "), "AfterDelay schedule should show countdown, got: {}", result);
    }

    #[test]
    fn format_schedule_legacy_task_shows_countdown() {
        // Legacy tasks (schedule_kind = None) should show countdown for backwards compatibility
        let task = make_task_with_schedule_kind(
            Utc::now() + Duration::hours(2),
            None,
        );
        let result = format_schedule(&task);

        // Should show countdown "in 2h 0m" (legacy behavior)
        assert!(result.starts_with("in "), "Legacy task should show countdown, got: {}", result);
    }

    #[test]
    fn format_target_full_values() {
        assert_eq!(format_target(&ExecutionTarget::NewPane, false), "new pane");
        assert_eq!(format_target(&ExecutionTarget::NewWindow, false), "window");
        assert_eq!(format_target(&ExecutionTarget::Background, false), "background");
    }

    #[test]
    fn format_target_abbreviated_values() {
        assert_eq!(format_target(&ExecutionTarget::NewPane, true), "pane");
        assert_eq!(format_target(&ExecutionTarget::NewWindow, true), "win");
        assert_eq!(format_target(&ExecutionTarget::Background, true), "bg");
    }

    #[test]
    fn format_status_values() {
        assert_eq!(format_status(&TaskStatus::Pending), "pending");
        assert_eq!(format_status(&TaskStatus::Running), "running");
        assert_eq!(format_status(&TaskStatus::Completed), "done");
        assert_eq!(format_status(&TaskStatus::Cancelled), "cancelled");
        assert_eq!(
            format_status(&TaskStatus::Failed {
                error: "test".to_string()
            }),
            "failed"
        );
    }

    #[test]
    fn task_style_pending_is_default() {
        let task = ScheduledTask::new(
            1,
            "test".to_string(),
            Utc::now(),
            ExecutionTarget::default(),
        );
        let style = task_style(&task);
        assert_eq!(style, Style::default());
    }

    #[test]
    fn task_style_running_is_yellow_bold() {
        let mut task = ScheduledTask::new(
            1,
            "test".to_string(),
            Utc::now(),
            ExecutionTarget::default(),
        );
        task.mark_running();
        let style = task_style(&task);
        assert_eq!(
            style,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn task_style_completed_is_dimmed() {
        let mut task = ScheduledTask::new(
            1,
            "test".to_string(),
            Utc::now(),
            ExecutionTarget::default(),
        );
        task.mark_completed();
        let style = task_style(&task);
        assert_eq!(
            style,
            Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)
        );
    }

    #[test]
    fn task_style_failed_is_red() {
        let mut task = ScheduledTask::new(
            1,
            "test".to_string(),
            Utc::now(),
            ExecutionTarget::default(),
        );
        task.mark_failed("error");
        let style = task_style(&task);
        assert_eq!(style, Style::default().fg(Color::Red));
    }

    #[test]
    fn task_style_cancelled_is_dimmed() {
        let mut task = ScheduledTask::new(
            1,
            "test".to_string(),
            Utc::now(),
            ExecutionTarget::default(),
        );
        task.mark_cancelled();
        let style = task_style(&task);
        assert_eq!(
            style,
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
        );
    }

    #[test]
    fn render_produces_output_with_empty_task_list() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        // Verify the header is rendered
        let first_row = buffer
            .content
            .iter()
            .take(80)
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(first_row.contains("Tasks"));
    }

    #[test]
    fn render_shows_tasks_in_table() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.tasks.push(ScheduledTask::new(
            1,
            "echo hello".to_string(),
            Utc::now() + Duration::minutes(15),
            ExecutionTarget::Background,
        ));

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        // Convert buffer to string for inspection
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content.contains("echo hello"));
        // At 80 width, shows full "background" text (not abbreviated "bg")
        assert!(content.contains("background"));
    }

    #[test]
    fn render_shows_footer_shortcuts() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // Footer should show keyboard shortcuts
        assert!(content.contains("Quit"));
        assert!(content.contains("New"));
        assert!(content.contains("Navigate"));
    }

    #[test]
    fn render_shows_confirm_quit_modal() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.mode = AppMode::ConfirmQuit;

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Quit"));
    }

    // =========================================================================
    // Column configuration tests
    // =========================================================================

    #[test]
    fn calculate_column_config_full_at_80_plus() {
        let config = calculate_column_config(80);
        assert!(config.show_status);
        assert!(!config.abbreviated_target);

        let config = calculate_column_config(120);
        assert!(config.show_status);
        assert!(!config.abbreviated_target);
    }

    #[test]
    fn calculate_column_config_abbreviated_at_70() {
        let config = calculate_column_config(70);
        assert!(config.show_status);
        assert!(config.abbreviated_target);

        let config = calculate_column_config(79);
        assert!(config.show_status);
        assert!(config.abbreviated_target);
    }

    #[test]
    fn calculate_column_config_no_status_at_50() {
        let config = calculate_column_config(50);
        assert!(!config.show_status);
        assert!(config.abbreviated_target);

        let config = calculate_column_config(59);
        assert!(!config.show_status);
        assert!(config.abbreviated_target);
    }

    #[test]
    fn calculate_column_config_boundary_60() {
        // At exactly 60, show_status should be true
        let config = calculate_column_config(60);
        assert!(config.show_status);
        assert!(config.abbreviated_target);
    }

    // =========================================================================
    // Empty state tests
    // =========================================================================

    #[test]
    fn render_shows_empty_state_when_no_tasks() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("No tasks scheduled"),
            "Should show full empty state message at 80 cols, got: {}",
            content
        );
        assert!(
            content.contains("Press N to add one"),
            "Should include instruction, got: {}",
            content
        );
    }

    #[test]
    fn render_shows_short_empty_state_in_narrow_terminal() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(39, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(
            content.contains("Press N to add task"),
            "Should show short message at narrow width, got: {}",
            content
        );
        assert!(
            !content.contains("No tasks scheduled"),
            "Should NOT show long message at narrow width, got: {}",
            content
        );
    }

    // =========================================================================
    // Responsive column tests
    // =========================================================================

    #[test]
    fn render_shows_all_columns_at_80_width() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.tasks.push(ScheduledTask::new(
            1,
            "test cmd".to_string(),
            Utc::now() + Duration::minutes(15),
            ExecutionTarget::Background,
        ));

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // At 80 width, should show full "background" and "Status" column
        assert!(content.contains("Status"), "Should show Status column header");
        assert!(content.contains("background"), "Should show full target text");
    }

    #[test]
    fn render_abbreviates_target_at_70_width() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(70, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.tasks.push(ScheduledTask::new(
            1,
            "test cmd".to_string(),
            Utc::now() + Duration::minutes(15),
            ExecutionTarget::Background,
        ));

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // At 70 width, should show abbreviated "bg" and still have "Status" column
        assert!(content.contains("Status"), "Should still show Status column");
        assert!(content.contains("bg"), "Should show abbreviated target 'bg'");
        assert!(
            !content.contains("background"),
            "Should NOT show full 'background' at 70 width"
        );
    }

    #[test]
    fn render_hides_status_at_50_width() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(50, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.tasks.push(ScheduledTask::new(
            1,
            "test cmd".to_string(),
            Utc::now() + Duration::minutes(15),
            ExecutionTarget::Background,
        ));

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // At 50 width, should hide Status column entirely
        assert!(
            !content.contains("Status"),
            "Should NOT show Status column at narrow width, got: {}",
            content
        );
        assert!(
            !content.contains("pending"),
            "Should NOT show status value at narrow width"
        );
    }

    // =========================================================================
    // Responsive footer tests
    // =========================================================================

    #[test]
    fn get_normal_mode_shortcuts_full_at_80_width() {
        let shortcuts = get_normal_mode_shortcuts(80);
        // Full labels should include "Remove", "History", and "Navigate"
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "Remove"));
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "History"));
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "Navigate"));
        // Should have all 7 shortcuts
        assert_eq!(shortcuts.len(), 7);
    }

    #[test]
    fn get_normal_mode_shortcuts_abbreviated_at_70_width() {
        let shortcuts = get_normal_mode_shortcuts(70);
        // Abbreviated labels should use "Rm", "Hist", and "Nav"
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "Rm"));
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "Hist"));
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "Nav"));
        // Should NOT have full labels
        assert!(!shortcuts.iter().any(|(_, desc)| *desc == "Remove"));
        assert!(!shortcuts.iter().any(|(_, desc)| *desc == "History"));
        assert!(!shortcuts.iter().any(|(_, desc)| *desc == "Navigate"));
        // Should still have all 7 shortcuts
        assert_eq!(shortcuts.len(), 7);
    }

    #[test]
    fn get_normal_mode_shortcuts_minimal_at_50_width() {
        let shortcuts = get_normal_mode_shortcuts(50);
        // Minimal set should only have Q, N, E, H
        assert!(shortcuts.iter().any(|(key, _)| *key == "Q"));
        assert!(shortcuts.iter().any(|(key, _)| *key == "N"));
        assert!(shortcuts.iter().any(|(key, _)| *key == "E"));
        assert!(shortcuts.iter().any(|(key, _)| *key == "H"));
        // Should NOT have R, X, or arrows
        assert!(!shortcuts.iter().any(|(key, _)| *key == "R"));
        assert!(!shortcuts.iter().any(|(key, _)| *key == "X"));
        assert!(!shortcuts.iter().any(|(key, _)| *key == "\u{2191}\u{2193}"));
        // Should have only 4 shortcuts
        assert_eq!(shortcuts.len(), 4);
    }

    #[test]
    fn get_normal_mode_shortcuts_boundary_at_60() {
        let shortcuts = get_normal_mode_shortcuts(60);
        // At exactly 60, should use abbreviated labels (medium tier)
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "Rm"));
        assert!(shortcuts.iter().any(|(_, desc)| *desc == "Hist"));
        assert_eq!(shortcuts.len(), 7);
    }

    #[test]
    fn get_normal_mode_shortcuts_boundary_at_59() {
        let shortcuts = get_normal_mode_shortcuts(59);
        // Below 60, should use minimal set
        assert_eq!(shortcuts.len(), 4);
    }

    #[test]
    fn footer_shows_full_shortcuts_at_80_width() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // At 80 width, footer should show full labels
        assert!(
            content.contains("Remove"),
            "Should show 'Remove' at 80 width, got: {}",
            content
        );
        assert!(
            content.contains("History"),
            "Should show 'History' at 80 width, got: {}",
            content
        );
        assert!(
            content.contains("Navigate"),
            "Should show 'Navigate' at 80 width, got: {}",
            content
        );
    }

    #[test]
    fn footer_shows_abbreviated_shortcuts_at_70_width() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(70, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // At 70 width, footer should show abbreviated labels
        assert!(
            content.contains("Rm"),
            "Should show 'Rm' at 70 width, got: {}",
            content
        );
        assert!(
            content.contains("Hist"),
            "Should show 'Hist' at 70 width, got: {}",
            content
        );
        // Should NOT show full labels
        assert!(
            !content.contains("Remove"),
            "Should NOT show 'Remove' at 70 width, got: {}",
            content
        );
        assert!(
            !content.contains("History"),
            "Should NOT show 'History' at 70 width, got: {}",
            content
        );
    }

    #[test]
    fn footer_shows_minimal_shortcuts_at_50_width() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(50, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not fail");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // At 50 width, footer should show minimal set
        assert!(
            content.contains("Quit"),
            "Should show 'Quit' at 50 width, got: {}",
            content
        );
        assert!(
            content.contains("New"),
            "Should show 'New' at 50 width, got: {}",
            content
        );
        assert!(
            content.contains("Edit"),
            "Should show 'Edit' at 50 width, got: {}",
            content
        );
        // Should NOT show removed shortcuts
        assert!(
            !content.contains("Cancel"),
            "Should NOT show 'Cancel' at 50 width, got: {}",
            content
        );
    }

    // =========================================================================
    // Extreme width tests - verify no panics at edge cases
    // =========================================================================

    #[test]
    fn render_handles_very_narrow_terminal() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // 20x10 is extremely narrow - should not panic
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Test with empty task list
        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not panic at 20x10");

        // Test with a task
        app.tasks.push(ScheduledTask::new(
            1,
            "echo hello world with a very long command".to_string(),
            Utc::now() + Duration::minutes(15),
            ExecutionTarget::Background,
        ));

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render with task should not panic at 20x10");
    }

    #[test]
    fn render_handles_very_wide_terminal() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // 200x50 is very wide - should not panic
        let backend = TestBackend::new(200, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Test with empty task list
        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not panic at 200x50");

        // Test with a task
        app.tasks.push(ScheduledTask::new(
            1,
            "echo hello".to_string(),
            Utc::now() + Duration::minutes(15),
            ExecutionTarget::Background,
        ));

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render with task should not panic at 200x50");

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        // At very wide terminal, should show all columns with full text
        assert!(content.contains("Status"), "Should show Status column at wide width");
        assert!(content.contains("background"), "Should show full target text at wide width");
    }

    #[test]
    fn render_handles_minimal_terminal_1x1() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // Absolute minimum - 1x1 should not panic
        let backend = TestBackend::new(1, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Should not panic even at 1x1
        let result = terminal.draw(|frame| render(&mut app, frame));
        assert!(result.is_ok(), "render should not panic at 1x1");
    }

    #[test]
    fn render_handles_very_short_terminal() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // 80 cols but only 5 rows - tests vertical constraints
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.tasks.push(ScheduledTask::new(
            1,
            "echo test".to_string(),
            Utc::now() + Duration::minutes(15),
            ExecutionTarget::Background,
        ));

        terminal
            .draw(|frame| render(&mut app, frame))
            .expect("render should not panic at 80x5");
    }
}
