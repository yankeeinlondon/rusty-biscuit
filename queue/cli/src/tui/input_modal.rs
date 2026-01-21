//! Input modal for creating and editing scheduled tasks.

use queue_lib::{parse_at_time, parse_delay, ExecutionTarget, ScheduledTask};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::modal::Modal;

/// The active field in the input form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputField {
    #[default]
    Command,
    ScheduleType,
    ScheduleValue,
    Target,
}

/// Schedule type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScheduleType {
    #[default]
    AtTime,
    AfterDelay,
}

/// Input modal for creating/editing tasks.
pub struct InputModal {
    pub command: String,
    pub cursor_pos: usize,
    pub schedule_type: ScheduleType,
    pub schedule_value: String,
    pub target: ExecutionTarget,
    pub active_field: InputField,
    pub error_message: Option<String>,
    pub editing_task_id: Option<u64>,
}

impl InputModal {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            cursor_pos: 0,
            schedule_type: ScheduleType::default(),
            schedule_value: String::new(),
            target: ExecutionTarget::default(),
            active_field: InputField::default(),
            error_message: None,
            editing_task_id: None,
        }
    }

    pub fn for_edit(task: &ScheduledTask) -> Self {
        Self {
            command: task.command.clone(),
            cursor_pos: task.command.len(),
            schedule_type: ScheduleType::AtTime,
            schedule_value: task.scheduled_at.format("%H:%M").to_string(),
            target: task.target,
            active_field: InputField::Command,
            error_message: None,
            editing_task_id: Some(task.id),
        }
    }

    /// Move to next field (Tab).
    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            InputField::Command => InputField::ScheduleType,
            InputField::ScheduleType => InputField::ScheduleValue,
            InputField::ScheduleValue => InputField::Target,
            InputField::Target => InputField::Command,
        };
        self.error_message = None;
    }

    /// Move to previous field (Shift+Tab).
    pub fn prev_field(&mut self) {
        self.active_field = match self.active_field {
            InputField::Command => InputField::Target,
            InputField::ScheduleType => InputField::Command,
            InputField::ScheduleValue => InputField::ScheduleType,
            InputField::Target => InputField::ScheduleValue,
        };
        self.error_message = None;
    }

    /// Toggle schedule type.
    pub fn toggle_schedule_type(&mut self) {
        self.schedule_type = match self.schedule_type {
            ScheduleType::AtTime => ScheduleType::AfterDelay,
            ScheduleType::AfterDelay => ScheduleType::AtTime,
        };
    }

    /// Cycle through execution targets.
    pub fn cycle_target(&mut self) {
        self.target = match self.target {
            ExecutionTarget::NewPane => ExecutionTarget::NewWindow,
            ExecutionTarget::NewWindow => ExecutionTarget::Background,
            ExecutionTarget::Background => ExecutionTarget::NewPane,
        };
    }

    /// Handle character input for the active text field.
    pub fn handle_char(&mut self, c: char) {
        match self.active_field {
            InputField::Command => {
                self.command.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            InputField::ScheduleValue => {
                self.schedule_value.push(c);
            }
            InputField::ScheduleType => {
                self.toggle_schedule_type();
            }
            InputField::Target => {
                self.cycle_target();
            }
        }
        self.error_message = None;
    }

    /// Handle backspace.
    pub fn handle_backspace(&mut self) {
        match self.active_field {
            InputField::Command => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.command.remove(self.cursor_pos);
                }
            }
            InputField::ScheduleValue => {
                self.schedule_value.pop();
            }
            _ => {}
        }
    }

    /// Validate the input and return error if invalid.
    pub fn validate(&mut self) -> Result<(), String> {
        if self.command.trim().is_empty() {
            self.error_message = Some("Command cannot be empty".to_string());
            return Err(self.error_message.clone().unwrap());
        }

        if self.schedule_value.trim().is_empty() {
            self.error_message = Some("Schedule value cannot be empty".to_string());
            return Err(self.error_message.clone().unwrap());
        }

        match self.schedule_type {
            ScheduleType::AtTime => {
                if parse_at_time(&self.schedule_value).is_err() {
                    self.error_message =
                        Some("Invalid time format (try 7:00am or 19:30)".to_string());
                    return Err(self.error_message.clone().unwrap());
                }
            }
            ScheduleType::AfterDelay => {
                if parse_delay(&self.schedule_value).is_err() {
                    self.error_message =
                        Some("Invalid delay format (try 15m, 2h, or 30s)".to_string());
                    return Err(self.error_message.clone().unwrap());
                }
            }
        }

        self.error_message = None;
        Ok(())
    }
}

impl Default for InputModal {
    fn default() -> Self {
        Self::new()
    }
}

impl Modal for InputModal {
    fn title(&self) -> &str {
        if self.editing_task_id.is_some() {
            "Edit Task"
        } else {
            "New Task"
        }
    }

    fn width_percent(&self) -> u16 {
        70
    }
    fn height_percent(&self) -> u16 {
        60
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Command
            Constraint::Length(3), // Schedule type
            Constraint::Length(3), // Schedule value
            Constraint::Length(3), // Target
            Constraint::Length(2), // Error message
            Constraint::Min(0),    // Spacer
            Constraint::Length(2), // Help
        ])
        .split(area);

        // Command field
        render_text_field(
            frame,
            chunks[0],
            "Command",
            &self.command,
            self.active_field == InputField::Command,
            Some(self.cursor_pos),
        );

        // Schedule type
        let schedule_type_text = match self.schedule_type {
            ScheduleType::AtTime => "At time",
            ScheduleType::AfterDelay => "After delay",
        };
        render_selector_field(
            frame,
            chunks[1],
            "Schedule",
            schedule_type_text,
            self.active_field == InputField::ScheduleType,
        );

        // Schedule value
        let placeholder = match self.schedule_type {
            ScheduleType::AtTime => "e.g., 7:00am or 19:30",
            ScheduleType::AfterDelay => "e.g., 15m, 2h, or 30s",
        };
        render_text_field_with_placeholder(
            frame,
            chunks[2],
            "When",
            &self.schedule_value,
            placeholder,
            self.active_field == InputField::ScheduleValue,
        );

        // Target
        let target_text = match self.target {
            ExecutionTarget::NewPane => "New pane",
            ExecutionTarget::NewWindow => "New window",
            ExecutionTarget::Background => "Background",
        };
        render_selector_field(
            frame,
            chunks[3],
            "Execute in",
            target_text,
            self.active_field == InputField::Target,
        );

        // Error message
        if let Some(ref error) = self.error_message {
            let error_para = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_para, chunks[4]);
        }

        // Help text
        let help = Paragraph::new("Tab: Next field | Enter: Submit | Esc: Cancel")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[6]);
    }
}

fn render_text_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_active: bool,
    cursor_pos: Option<usize>,
) {
    let style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", label))
        .border_style(style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let display_value = if is_active {
        if let Some(pos) = cursor_pos {
            let (before, after) = value.split_at(pos.min(value.len()));
            format!("{}|{}", before, after)
        } else {
            format!("{}|", value)
        }
    } else {
        value.to_string()
    };

    let para = Paragraph::new(display_value);
    frame.render_widget(para, inner);
}

fn render_text_field_with_placeholder(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    placeholder: &str,
    is_active: bool,
) {
    let style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", label))
        .border_style(style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (text, text_style) = if value.is_empty() {
        (placeholder, Style::default().fg(Color::DarkGray))
    } else {
        (value, Style::default())
    };

    let display = if is_active && !value.is_empty() {
        format!("{}|", value)
    } else {
        text.to_string()
    };

    let para = Paragraph::new(display).style(text_style);
    frame.render_widget(para, inner);
}

fn render_selector_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_active: bool,
) {
    let style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", label))
        .border_style(style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let arrows = if is_active { "< " } else { "  " };
    let arrows_end = if is_active { " >" } else { "  " };

    let para = Paragraph::new(format!("{}{}{}", arrows, value, arrows_end));
    frame.render_widget(para, inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_modal_has_defaults() {
        let modal = InputModal::new();
        assert!(modal.command.is_empty());
        assert_eq!(modal.active_field, InputField::Command);
        assert_eq!(modal.schedule_type, ScheduleType::AtTime);
        assert_eq!(modal.target, ExecutionTarget::NewPane);
        assert!(modal.editing_task_id.is_none());
    }

    #[test]
    fn next_field_cycles_through_all_fields() {
        let mut modal = InputModal::new();
        assert_eq!(modal.active_field, InputField::Command);
        modal.next_field();
        assert_eq!(modal.active_field, InputField::ScheduleType);
        modal.next_field();
        assert_eq!(modal.active_field, InputField::ScheduleValue);
        modal.next_field();
        assert_eq!(modal.active_field, InputField::Target);
        modal.next_field();
        assert_eq!(modal.active_field, InputField::Command);
    }

    #[test]
    fn handle_char_appends_to_command() {
        let mut modal = InputModal::new();
        modal.handle_char('h');
        modal.handle_char('i');
        assert_eq!(modal.command, "hi");
    }

    #[test]
    fn handle_backspace_removes_char() {
        let mut modal = InputModal::new();
        modal.command = "hello".to_string();
        modal.cursor_pos = 5;
        modal.handle_backspace();
        assert_eq!(modal.command, "hell");
    }

    #[test]
    fn validate_rejects_empty_command() {
        let mut modal = InputModal::new();
        modal.schedule_value = "15m".to_string();
        assert!(modal.validate().is_err());
        assert!(modal.error_message.is_some());
    }

    #[test]
    fn validate_accepts_valid_input() {
        let mut modal = InputModal::new();
        modal.command = "echo hi".to_string();
        modal.schedule_value = "15m".to_string();
        modal.schedule_type = ScheduleType::AfterDelay;
        assert!(modal.validate().is_ok());
    }

    #[test]
    fn toggle_schedule_type_cycles() {
        let mut modal = InputModal::new();
        assert_eq!(modal.schedule_type, ScheduleType::AtTime);
        modal.toggle_schedule_type();
        assert_eq!(modal.schedule_type, ScheduleType::AfterDelay);
        modal.toggle_schedule_type();
        assert_eq!(modal.schedule_type, ScheduleType::AtTime);
    }

    #[test]
    fn cycle_target_cycles_all_targets() {
        let mut modal = InputModal::new();
        assert_eq!(modal.target, ExecutionTarget::NewPane);
        modal.cycle_target();
        assert_eq!(modal.target, ExecutionTarget::NewWindow);
        modal.cycle_target();
        assert_eq!(modal.target, ExecutionTarget::Background);
        modal.cycle_target();
        assert_eq!(modal.target, ExecutionTarget::NewPane);
    }
}
