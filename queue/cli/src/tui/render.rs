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
            Cell::from(format_schedule(&task.scheduled_at)),
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
fn format_schedule(scheduled_at: &chrono::DateTime<chrono::Utc>) -> String {
    let now = Utc::now();
    let duration = scheduled_at.signed_duration_since(now);

    if duration.num_seconds() < 0 {
        "past".to_string()
    } else if duration.num_hours() < 1 {
        format!("in {}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!(
            "in {}h {}m",
            duration.num_hours(),
            duration.num_minutes() % 60
        )
    } else {
        scheduled_at.format("%b %d %H:%M").to_string()
    }
}

/// Formats an execution target as a short label.
fn format_target(target: &ExecutionTarget) -> &'static str {
    match target {
        ExecutionTarget::NewPane => "pane",
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

    #[test]
    fn format_schedule_past_time() {
        let past = Utc::now() - Duration::hours(1);
        assert_eq!(format_schedule(&past), "past");
    }

    #[test]
    fn format_schedule_under_one_hour() {
        let future = Utc::now() + Duration::minutes(30);
        let result = format_schedule(&future);
        assert!(result.starts_with("in "));
        assert!(result.ends_with("m"));
        assert!(!result.contains("h"));
    }

    #[test]
    fn format_schedule_under_24_hours() {
        let future = Utc::now() + Duration::hours(5) + Duration::minutes(15);
        let result = format_schedule(&future);
        assert!(result.contains("h"));
        assert!(result.contains("m"));
    }

    #[test]
    fn format_schedule_over_24_hours() {
        let future = Utc::now() + Duration::days(2);
        let result = format_schedule(&future);
        // Should be absolute format like "Jan 23 14:30"
        assert!(!result.starts_with("in "));
    }

    #[test]
    fn format_target_values() {
        assert_eq!(format_target(&ExecutionTarget::NewPane), "pane");
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
}
