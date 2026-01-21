//! Rendering functions for the TUI.
//!
//! This module implements the main window layout with a task table and status footer.

use chrono::Utc;
use queue_lib::{ExecutionTarget, ScheduledTask, TaskStatus};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use super::app::{App, AppMode};
use super::modal::{render_modal, ConfirmQuitDialog};

/// Renders the entire application UI.
///
/// The layout consists of:
/// - A task table filling most of the screen
/// - A footer bar showing keyboard shortcuts for the current mode
/// - Modal overlays rendered on top based on current mode
pub fn render(app: &mut App, frame: &mut Frame) {
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
            render_modal(frame, &ConfirmQuitDialog, frame.area());
        }
        AppMode::InputModal => {
            if let Some(ref input_modal) = app.input_modal {
                render_modal(frame, input_modal, frame.area());
            }
        }
        AppMode::HistoryModal => {
            if let Some(ref history_modal) = app.history_modal {
                render_modal(frame, history_modal, frame.area());
            }
        }
        _ => {}
    }
}

/// Renders the task table with columns for ID, WHEN, Command, Where, and Status.
fn render_task_table(app: &mut App, frame: &mut Frame, area: Rect) {
    let header = Row::new([
        Cell::from("ID"),
        Cell::from("WHEN"),
        Cell::from("Command"),
        Cell::from("Where"),
        Cell::from("Status"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let rows = app.tasks.iter().map(|task| {
        let style = task_style(task);
        Row::new([
            Cell::from(task.id.to_string()),
            Cell::from(format_schedule(task)),
            Cell::from(task.command.as_str()),
            Cell::from(format_target(&task.target)),
            Cell::from(format_status(&task.status)),
        ])
        .style(style)
    });

    let widths = [
        Constraint::Length(4),  // ID
        Constraint::Length(12), // WHEN
        Constraint::Min(20),    // Command
        Constraint::Length(10), // Where
        Constraint::Length(10), // Status
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Tasks "))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

/// Renders the footer with keyboard shortcuts appropriate for the current mode.
fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let (bg_color, shortcuts) = match app.mode {
        AppMode::Normal => (
            Color::DarkGray,
            vec![
                ("Q", "Quit"),
                ("N", "New"),
                ("E", "Edit"),
                ("R", "Remove"),
                ("X", "Cancel"),
                ("H", "History"),
                ("\u{2191}\u{2193}", "Navigate"),
            ],
        ),
        AppMode::EditMode => (
            Color::Yellow,
            vec![("Enter", "Edit Selected"), ("Esc", "Cancel")],
        ),
        AppMode::RemoveMode => (
            Color::Red,
            vec![("Enter", "Remove Selected"), ("Esc", "Cancel")],
        ),
        AppMode::ConfirmQuit => (
            Color::DarkGray,
            vec![("Y", "Yes, Quit"), ("N", "No, Stay")],
        ),
        AppMode::InputModal | AppMode::HistoryModal => {
            (Color::DarkGray, vec![("Esc", "Back")])
        }
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
        TaskStatus::Completed => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC | Modifier::DIM),
        TaskStatus::Running => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        TaskStatus::Failed { .. } => Style::default().fg(Color::Red),
        TaskStatus::Pending => Style::default(),
    }
}

/// Formats a scheduled time as a human-readable relative or absolute string.
///
/// For pending tasks, shows relative time (e.g., "in 5m", "in 1h 30m").
/// For completed/running tasks, shows the absolute time when scheduled (HH:MM).
fn format_schedule(task: &ScheduledTask) -> String {
    let now = Utc::now();
    let duration = task.scheduled_at.signed_duration_since(now);

    // For non-pending tasks, show the scheduled time as HH:MM
    if !task.is_pending() {
        return task.scheduled_at.with_timezone(&chrono::Local).format("%H:%M").to_string();
    }

    // For pending tasks, show relative time
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
fn format_target(target: &ExecutionTarget) -> &'static str {
    match target {
        ExecutionTarget::NewPane => "new pane",
        ExecutionTarget::NewWindow => "window",
        ExecutionTarget::Background => "bg",
    }
}

/// Formats a task status as a short label.
fn format_status(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Completed => "done",
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

    #[test]
    fn format_target_values() {
        assert_eq!(format_target(&ExecutionTarget::NewPane), "new pane");
        assert_eq!(format_target(&ExecutionTarget::NewWindow), "window");
        assert_eq!(format_target(&ExecutionTarget::Background), "bg");
    }

    #[test]
    fn format_status_values() {
        assert_eq!(format_status(&TaskStatus::Pending), "pending");
        assert_eq!(format_status(&TaskStatus::Running), "running");
        assert_eq!(format_status(&TaskStatus::Completed), "done");
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
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC | Modifier::DIM)
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
        assert!(content.contains("bg"));
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
}
