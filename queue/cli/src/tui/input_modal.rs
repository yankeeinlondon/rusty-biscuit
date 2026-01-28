//! Input modal for creating and editing scheduled tasks.

use queue_lib::{parse_at_time, parse_delay, ExecutionTarget, ScheduledTask, TerminalCapabilities};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::color_context::ColorContext;
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

/// Layout mode for the input modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputLayout {
    #[default]
    Full,
    Compact,
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
    pub schedule_value_cursor_pos: usize,
    pub target: ExecutionTarget,
    pub active_field: InputField,
    pub layout: InputLayout,
    pub error_message: Option<String>,
    pub editing_task_id: Option<u64>,
    /// Terminal capabilities - determines which execution targets are available.
    pub capabilities: TerminalCapabilities,
}

const FULL_LAYOUT_THRESHOLD: u16 = 18;
const FULL_MODAL_MIN_HEIGHT: u16 = 16;
const COMPACT_MODAL_MIN_HEIGHT: u16 = 12;
const COMPACT_LABEL_WIDTH: usize = 10;

impl InputModal {
    /// Creates a new input modal with default target based on terminal capabilities.
    ///
    /// - In Wezterm: default to NewPane (pane support available)
    /// - In other terminals with window support: default to NewWindow
    /// - Otherwise: default to Background
    pub fn new(capabilities: TerminalCapabilities) -> Self {
        let default_target = if capabilities.supports_panes {
            ExecutionTarget::NewPane
        } else if capabilities.supports_new_window {
            ExecutionTarget::NewWindow
        } else {
            ExecutionTarget::Background
        };

        Self {
            command: String::new(),
            cursor_pos: 0,
            schedule_type: ScheduleType::default(),
            schedule_value: String::new(),
            schedule_value_cursor_pos: 0,
            target: default_target,
            active_field: InputField::default(),
            layout: InputLayout::default(),
            error_message: None,
            editing_task_id: None,
            capabilities,
        }
    }

    pub fn for_edit(task: &ScheduledTask, capabilities: TerminalCapabilities) -> Self {
        // Convert UTC to local time for display - the reverse of the conversion
        // done in event.rs when submitting the form (local → UTC).
        let local_time = task.scheduled_at.with_timezone(&chrono::Local);
        let schedule_value = local_time.format("%H:%M").to_string();
        let schedule_value_cursor_pos = schedule_value.len();
        Self {
            command: task.command.clone(),
            cursor_pos: task.command.len(),
            schedule_type: ScheduleType::AtTime,
            schedule_value,
            schedule_value_cursor_pos,
            target: task.target,
            active_field: InputField::Command,
            layout: InputLayout::default(),
            error_message: None,
            editing_task_id: Some(task.id),
            capabilities,
        }
    }

    /// Updates layout mode based on available height.
    pub fn update_layout(&mut self, available_height: u16) {
        self.layout = if available_height >= FULL_LAYOUT_THRESHOLD {
            InputLayout::Full
        } else {
            InputLayout::Compact
        };
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

    /// Cycle through execution targets based on terminal capabilities.
    ///
    /// Only cycles through targets that are actually supported:
    /// - NewPane: only in terminals with pane support (Wezterm, iTerm2)
    /// - NewWindow: only in terminals with window support
    /// - Background: always available
    pub fn cycle_target(&mut self) {
        self.target = match self.target {
            ExecutionTarget::NewPane => {
                if self.capabilities.supports_new_window {
                    ExecutionTarget::NewWindow
                } else {
                    ExecutionTarget::Background
                }
            }
            ExecutionTarget::NewWindow => ExecutionTarget::Background,
            ExecutionTarget::Background => {
                if self.capabilities.supports_panes {
                    ExecutionTarget::NewPane
                } else if self.capabilities.supports_new_window {
                    ExecutionTarget::NewWindow
                } else {
                    ExecutionTarget::Background
                }
            }
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
                self.schedule_value
                    .insert(self.schedule_value_cursor_pos, c);
                self.schedule_value_cursor_pos += 1;
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
                if self.schedule_value_cursor_pos > 0 {
                    self.schedule_value_cursor_pos -= 1;
                    self.schedule_value.remove(self.schedule_value_cursor_pos);
                }
            }
            _ => {}
        }
    }

    /// Move cursor to the beginning of the active text field.
    pub fn move_cursor_start(&mut self) {
        match self.active_field {
            InputField::Command => {
                self.cursor_pos = 0;
            }
            InputField::ScheduleValue => {
                self.schedule_value_cursor_pos = 0;
            }
            _ => {}
        }
    }

    /// Move cursor to the end of the active text field.
    pub fn move_cursor_end(&mut self) {
        match self.active_field {
            InputField::Command => {
                self.cursor_pos = self.command.len();
            }
            InputField::ScheduleValue => {
                self.schedule_value_cursor_pos = self.schedule_value.len();
            }
            _ => {}
        }
    }

    /// Move cursor left in the active text field.
    pub fn move_cursor_left(&mut self) {
        match self.active_field {
            InputField::Command => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            InputField::ScheduleValue => {
                if self.schedule_value_cursor_pos > 0 {
                    self.schedule_value_cursor_pos -= 1;
                }
            }
            _ => {}
        }
    }

    /// Move cursor right in the active text field.
    pub fn move_cursor_right(&mut self) {
        match self.active_field {
            InputField::Command => {
                if self.cursor_pos < self.command.len() {
                    self.cursor_pos += 1;
                }
            }
            InputField::ScheduleValue => {
                if self.schedule_value_cursor_pos < self.schedule_value.len() {
                    self.schedule_value_cursor_pos += 1;
                }
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

impl Modal for InputModal {
    fn title(&self) -> &str {
        if self.editing_task_id.is_some() {
            "Edit Task"
        } else {
            "New Task"
        }
    }

    fn width_percent(&self) -> u16 {
        75
    }

    fn height_percent(&self) -> u16 {
        60
    }

    /// Minimum height to ensure the modal renders properly in small panes.
    ///
    /// The compact layout supports 12 rows so it fits in small panes.
    fn min_height(&self) -> u16 {
        match self.layout {
            InputLayout::Full => FULL_MODAL_MIN_HEIGHT,
            InputLayout::Compact => COMPACT_MODAL_MIN_HEIGHT,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, _color_context: &ColorContext) {
        match self.layout {
            InputLayout::Full => self.render_full(frame, area),
            InputLayout::Compact => self.render_compact(frame, area),
        }
    }
}

impl InputModal {
    /// Calculate the number of rows needed for the command field in compact mode.
    ///
    /// Returns 2 rows minimum, expanding to 3-4 for longer commands based on
    /// how many lines the text wraps to at the available width.
    fn calculate_command_rows(&self, area_width: u16) -> u16 {
        const MIN_ROWS: u16 = 2;
        const MAX_ROWS: u16 = 4;

        // Account for label width (" Command   : " = 1 space + 10 label + ": " = 13 chars)
        let content_width = area_width.saturating_sub(COMPACT_LABEL_WIDTH as u16 + 3);
        if content_width == 0 {
            return MIN_ROWS;
        }

        // Calculate lines needed for the command text.
        // Use actual text length - cursor is positioned by terminal, not a character.
        let text_len = self.command.len();
        // Ensure at least 1 line when empty
        let lines_needed = if text_len == 0 {
            1
        } else {
            ((text_len as f32) / (content_width as f32)).ceil() as u16
        };

        lines_needed.clamp(MIN_ROWS, MAX_ROWS)
    }

    fn render_full(&self, frame: &mut Frame, area: Rect) {
        let error_height = if self.error_message.is_some() { 1 } else { 0 };
        let chunks = Layout::vertical([
            Constraint::Length(3), // Command
            Constraint::Length(3), // Schedule type
            Constraint::Length(3), // Schedule value
            Constraint::Length(3), // Target
            Constraint::Length(error_height),
            Constraint::Min(1), // Help
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
            self.schedule_value_cursor_pos,
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
        frame.render_widget(help, chunks[5]);
    }

    fn render_compact(&self, frame: &mut Frame, area: Rect) {
        let error_height = if self.error_message.is_some() { 1 } else { 0 };

        // Calculate command field height based on content length.
        // Minimum 2 rows, expand to 3-4 for longer commands.
        let command_rows = self.calculate_command_rows(area.width);

        let chunks = Layout::vertical([
            Constraint::Length(1),            // Spacer (blank line before Command)
            Constraint::Length(command_rows), // Command (2-4 rows)
            Constraint::Length(1),            // Schedule type
            Constraint::Length(1),            // Schedule value
            Constraint::Length(1),            // Target
            Constraint::Length(error_height),
            Constraint::Min(1), // Help
        ])
        .split(area);

        render_compact_multiline_text_field(
            frame,
            chunks[1],
            "Command",
            &self.command,
            None,
            self.active_field == InputField::Command,
            Some(self.cursor_pos),
        );

        let schedule_type_text = match self.schedule_type {
            ScheduleType::AtTime => "At time",
            ScheduleType::AfterDelay => "After delay",
        };
        render_compact_selector_field(
            frame,
            chunks[2],
            "Schedule",
            schedule_type_text,
            self.active_field == InputField::ScheduleType,
        );

        let placeholder = match self.schedule_type {
            ScheduleType::AtTime => "e.g., 7:00am or 19:30",
            ScheduleType::AfterDelay => "e.g., 15m, 2h, or 30s",
        };
        render_compact_text_field(
            frame,
            chunks[3],
            "When",
            &self.schedule_value,
            Some(placeholder),
            self.active_field == InputField::ScheduleValue,
            Some(self.schedule_value_cursor_pos),
        );

        let target_text = match self.target {
            ExecutionTarget::NewPane => "New pane",
            ExecutionTarget::NewWindow => "New window",
            ExecutionTarget::Background => "Background",
        };
        render_compact_selector_field(
            frame,
            chunks[4],
            "Execute in",
            target_text,
            self.active_field == InputField::Target,
        );

        if let Some(ref error) = self.error_message {
            let error_para = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_para, chunks[5]);
        }

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

    // Display the value without cursor character
    let para = Paragraph::new(value);
    frame.render_widget(para, inner);

    // Position the real terminal cursor when active
    if is_active {
        let cursor_offset = cursor_pos.unwrap_or(value.len());
        let cursor_x = inner.x + cursor_offset as u16;
        let cursor_y = inner.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_text_field_with_placeholder(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    placeholder: &str,
    is_active: bool,
    cursor_pos: usize,
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

    // Show placeholder only when empty and not active
    let is_placeholder = value.is_empty() && !is_active;
    let display_text = if is_placeholder { placeholder } else { value };
    let text_style = if is_placeholder {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };

    let para = Paragraph::new(display_text).style(text_style);
    frame.render_widget(para, inner);

    // Position the real terminal cursor when active
    if is_active {
        let cursor_x = inner.x + cursor_pos.min(value.len()) as u16;
        let cursor_y = inner.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
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

fn render_compact_text_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    placeholder: Option<&str>,
    is_active: bool,
    cursor_pos: Option<usize>,
) {
    let label_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let value_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let label_text = format!("{label:<width$}: ", width = COMPACT_LABEL_WIDTH);

    // Calculate label prefix length for cursor positioning
    // Format: " {label_text}" = 1 space + label + ": "
    let label_prefix_len = 1 + COMPACT_LABEL_WIDTH + 2; // " " + label + ": "

    let line = if value.is_empty() {
        if is_active {
            // Empty field when active: just show label (cursor will be positioned)
            Line::from(vec![Span::styled(format!(" {label_text}"), label_style)])
        } else if let Some(placeholder) = placeholder {
            // Empty field when not active: show placeholder in dim color
            Line::from(vec![
                Span::styled(format!(" {label_text}"), label_style),
                Span::styled(placeholder, Style::default().fg(Color::DarkGray)),
            ])
        } else {
            Line::from(vec![Span::styled(format!(" {label_text}"), label_style)])
        }
    } else {
        // Field has value: display without cursor character
        Line::from(vec![
            Span::styled(format!(" {label_text}"), label_style),
            Span::styled(value, value_style),
        ])
    };

    frame.render_widget(Paragraph::new(line), area);

    // Position the real terminal cursor when active
    if is_active {
        let cursor_offset = cursor_pos.unwrap_or(value.len());
        let cursor_x = area.x + label_prefix_len as u16 + cursor_offset as u16;
        let cursor_y = area.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Renders a compact text field that can span multiple rows for longer content.
///
/// The first row shows the label and beginning of the value. Subsequent rows
/// continue the value text (indented to align with the first row's value).
fn render_compact_multiline_text_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    placeholder: Option<&str>,
    is_active: bool,
    cursor_pos: Option<usize>,
) {
    let label_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let value_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    // Label prefix: " Command   : " (1 space + label + ": ")
    let label_text = format!("{label:<width$}: ", width = COMPACT_LABEL_WIDTH);
    let label_prefix = format!(" {label_text}");
    let label_prefix_len = label_prefix.len();

    // Continuation indent (spaces to align with value start)
    let indent = " ".repeat(label_prefix_len);

    // Calculate content width (area width minus the label prefix)
    let content_width = (area.width as usize).saturating_sub(label_prefix_len);
    if content_width == 0 {
        return;
    }

    // Determine display value (without cursor character)
    let display_value = if value.is_empty() {
        if is_active {
            String::new() // Empty when active, cursor will be positioned
        } else if let Some(ph) = placeholder {
            ph.to_string()
        } else {
            String::new()
        }
    } else {
        value.to_string()
    };

    let is_placeholder = value.is_empty() && !is_active && placeholder.is_some();
    let text_style = if is_placeholder {
        Style::default().fg(Color::DarkGray)
    } else {
        value_style
    };

    // Split the display value into chunks that fit within content_width
    let mut lines: Vec<Line> = Vec::new();
    let mut remaining = display_value.as_str();

    // First line includes the label
    let first_chunk_len = remaining.len().min(content_width);
    let (first_chunk, rest) = remaining.split_at(first_chunk_len);
    lines.push(Line::from(vec![
        Span::styled(label_prefix.clone(), label_style),
        Span::styled(first_chunk.to_string(), text_style),
    ]));
    remaining = rest;

    // Subsequent lines are indented
    while !remaining.is_empty() {
        let chunk_len = remaining.len().min(content_width);
        let (chunk, rest) = remaining.split_at(chunk_len);
        lines.push(Line::from(vec![
            Span::raw(indent.clone()),
            Span::styled(chunk.to_string(), text_style),
        ]));
        remaining = rest;
    }

    // Render as a paragraph (which handles multiple lines)
    let para = Paragraph::new(lines);
    frame.render_widget(para, area);

    // Position the real terminal cursor when active
    if is_active {
        let cursor_offset = cursor_pos.unwrap_or(value.len());

        // Calculate which line and column the cursor is on
        // First line has content_width chars, subsequent lines also have content_width
        let (cursor_line, cursor_col) = if cursor_offset < content_width {
            // Cursor is on the first line
            (0, cursor_offset)
        } else {
            // Cursor is on a subsequent line
            let chars_after_first = cursor_offset - content_width;
            let line_num = 1 + chars_after_first / content_width;
            let col = chars_after_first % content_width;
            (line_num, col)
        };

        // X position: label prefix + column offset
        let cursor_x = area.x + label_prefix_len as u16 + cursor_col as u16;
        // Y position: area.y + which line we're on
        let cursor_y = area.y + cursor_line as u16;

        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_compact_selector_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_active: bool,
) {
    let label_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let value_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let label_text = format!("{label:<width$}: ", width = COMPACT_LABEL_WIDTH);
    let display_value = if is_active {
        format!("< {} >", value)
    } else {
        value.to_string()
    };

    let line = Line::from(vec![
        Span::styled(format!(" {label_text}"), label_style),
        Span::styled(display_value, value_style),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use queue_lib::TerminalKind;

    /// Creates capabilities for a terminal with pane support (like Wezterm).
    fn wezterm_caps() -> TerminalCapabilities {
        TerminalCapabilities {
            kind: TerminalKind::Wezterm,
            supports_panes: true,
            supports_new_window: true,
        }
    }

    /// Creates capabilities for a terminal with window support only (like Terminal.app).
    fn terminal_app_caps() -> TerminalCapabilities {
        TerminalCapabilities {
            kind: TerminalKind::TerminalApp,
            supports_panes: false,
            supports_new_window: true,
        }
    }

    /// Creates capabilities for an unknown terminal (background only).
    fn unknown_caps() -> TerminalCapabilities {
        TerminalCapabilities {
            kind: TerminalKind::Unknown,
            supports_panes: false,
            supports_new_window: false,
        }
    }

    #[test]
    fn new_modal_defaults_to_new_pane_in_wezterm() {
        let modal = InputModal::new(wezterm_caps());
        assert!(modal.command.is_empty());
        assert_eq!(modal.active_field, InputField::Command);
        assert_eq!(modal.schedule_type, ScheduleType::AtTime);
        assert_eq!(modal.target, ExecutionTarget::NewPane);
        assert!(modal.editing_task_id.is_none());
    }

    #[test]
    fn new_modal_defaults_to_new_window_in_terminal_app() {
        let modal = InputModal::new(terminal_app_caps());
        assert_eq!(modal.target, ExecutionTarget::NewWindow);
    }

    #[test]
    fn new_modal_defaults_to_background_in_unknown_terminal() {
        let modal = InputModal::new(unknown_caps());
        assert_eq!(modal.target, ExecutionTarget::Background);
    }

    #[test]
    fn next_field_cycles_through_all_fields() {
        let mut modal = InputModal::new(wezterm_caps());
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
        let mut modal = InputModal::new(wezterm_caps());
        modal.handle_char('h');
        modal.handle_char('i');
        assert_eq!(modal.command, "hi");
    }

    #[test]
    fn handle_backspace_removes_char() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.command = "hello".to_string();
        modal.cursor_pos = 5;
        modal.handle_backspace();
        assert_eq!(modal.command, "hell");
    }

    #[test]
    fn validate_rejects_empty_command() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.schedule_value = "15m".to_string();
        assert!(modal.validate().is_err());
        assert!(modal.error_message.is_some());
    }

    #[test]
    fn validate_accepts_valid_input() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.command = "echo hi".to_string();
        modal.schedule_value = "15m".to_string();
        modal.schedule_type = ScheduleType::AfterDelay;
        assert!(modal.validate().is_ok());
    }

    #[test]
    fn toggle_schedule_type_cycles() {
        let mut modal = InputModal::new(wezterm_caps());
        assert_eq!(modal.schedule_type, ScheduleType::AtTime);
        modal.toggle_schedule_type();
        assert_eq!(modal.schedule_type, ScheduleType::AfterDelay);
        modal.toggle_schedule_type();
        assert_eq!(modal.schedule_type, ScheduleType::AtTime);
    }

    #[test]
    fn cycle_target_cycles_all_targets_in_wezterm() {
        let mut modal = InputModal::new(wezterm_caps());
        assert_eq!(modal.target, ExecutionTarget::NewPane);
        modal.cycle_target();
        assert_eq!(modal.target, ExecutionTarget::NewWindow);
        modal.cycle_target();
        assert_eq!(modal.target, ExecutionTarget::Background);
        modal.cycle_target();
        assert_eq!(modal.target, ExecutionTarget::NewPane);
    }

    #[test]
    fn cycle_target_skips_new_pane_in_terminal_app() {
        let mut modal = InputModal::new(terminal_app_caps());
        assert_eq!(modal.target, ExecutionTarget::NewWindow);
        modal.cycle_target();
        assert_eq!(modal.target, ExecutionTarget::Background);
        modal.cycle_target();
        // Should cycle back to NewWindow, skipping NewPane
        assert_eq!(modal.target, ExecutionTarget::NewWindow);
    }

    #[test]
    fn cycle_target_stays_on_background_in_unknown_terminal() {
        let mut modal = InputModal::new(unknown_caps());
        assert_eq!(modal.target, ExecutionTarget::Background);
        modal.cycle_target();
        // Should stay on Background since nothing else is supported
        assert_eq!(modal.target, ExecutionTarget::Background);
    }

    // =========================================================================
    // Regression test for small pane modal rendering
    // Bug: TUI in Wezterm split pane (20%) couldn't fit input modal with 0 tasks
    // =========================================================================

    #[test]
    fn input_modal_has_minimum_height_for_small_panes() {
        use crate::tui::modal::Modal;

        let mut modal = InputModal::new(wezterm_caps());
        modal.update_layout(12);

        // The compact modal fits within 12 rows.
        assert_eq!(
            modal.min_height(),
            12,
            "InputModal should have min_height of 12 for small pane support"
        );
    }

    // =========================================================================
    // Regression tests for compact layout vertical balance
    // Bug: Compact modal had no spacer before Command field and Command was
    // only 1 row regardless of content length. Fixed to add spacer and allow
    // Command field to expand from 2-4 rows based on content.
    // =========================================================================

    #[test]
    fn compact_command_rows_returns_minimum_for_empty_command() {
        let modal = InputModal::new(wezterm_caps());
        // Simulate a 80-column width
        let rows = modal.calculate_command_rows(80);
        assert_eq!(rows, 2, "Empty command should get minimum 2 rows");
    }

    #[test]
    fn compact_command_rows_returns_minimum_for_short_command() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.command = "echo hello".to_string();
        let rows = modal.calculate_command_rows(80);
        assert_eq!(rows, 2, "Short command should get minimum 2 rows");
    }

    #[test]
    fn compact_command_rows_expands_for_long_command() {
        let mut modal = InputModal::new(wezterm_caps());
        // Create a command that needs ~3 rows at 80 cols
        // Label takes ~13 chars, so content width is ~67
        // 67 * 2 = 134 chars for 2 rows, so 135+ needs 3 rows
        modal.command = "x".repeat(140);
        let rows = modal.calculate_command_rows(80);
        assert_eq!(rows, 3, "Long command should expand to 3 rows");
    }

    #[test]
    fn compact_command_rows_caps_at_maximum() {
        let mut modal = InputModal::new(wezterm_caps());
        // Create a very long command that would need 5+ rows
        modal.command = "x".repeat(500);
        let rows = modal.calculate_command_rows(80);
        assert_eq!(rows, 4, "Very long command should cap at 4 rows");
    }

    #[test]
    fn compact_command_rows_handles_narrow_terminal() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.command = "echo test".to_string();
        // Very narrow terminal (15 - 13 label = ~2 content width)
        // "echo test" (9 chars) / 2 = 4.5 rows, rounded up = 5, capped to 4
        let rows = modal.calculate_command_rows(15);
        assert_eq!(rows, 4, "Narrow terminal should expand rows to fit content (capped at max)");
    }

    #[test]
    fn compact_command_rows_handles_zero_width() {
        let modal = InputModal::new(wezterm_caps());
        let rows = modal.calculate_command_rows(0);
        assert_eq!(rows, 2, "Zero width should return minimum 2 rows");
    }

    // =========================================================================
    // Regression test for UTC-to-local time conversion in edit modal
    // Bug: Task scheduled for 15:00 local time (stored as 23:00 UTC for UTC-8)
    // displayed as 23:00 in edit modal instead of 15:00.
    // Fix: Convert UTC back to local time before displaying in the form.
    // =========================================================================

    #[test]
    fn for_edit_displays_local_time_not_utc() {
        use chrono::Local;

        // Create a task with a specific UTC time
        let task = queue_lib::ScheduledTask::new(
            1,
            "echo hello".to_string(),
            chrono::Utc::now(),
            queue_lib::ExecutionTarget::Background,
        );

        let modal = InputModal::for_edit(&task, wezterm_caps());

        // The schedule_value should match the local time, not UTC
        // We verify this by converting the task's UTC time to local and formatting
        let expected_local_time = task.scheduled_at.with_timezone(&Local);
        let expected_value = expected_local_time.format("%H:%M").to_string();

        assert_eq!(
            modal.schedule_value, expected_value,
            "Edit modal should display local time, not UTC. \
             Task UTC: {}, Expected local: {}, Got: {}",
            task.scheduled_at.format("%H:%M"),
            expected_value,
            modal.schedule_value
        );
    }

    #[test]
    fn for_edit_preserves_task_properties() {
        use chrono::Utc;

        let task = queue_lib::ScheduledTask::new(
            42,
            "cargo build --release".to_string(),
            Utc::now(),
            queue_lib::ExecutionTarget::NewWindow,
        );

        let modal = InputModal::for_edit(&task, wezterm_caps());

        assert_eq!(modal.command, "cargo build --release");
        assert_eq!(modal.cursor_pos, task.command.len());
        assert_eq!(modal.target, queue_lib::ExecutionTarget::NewWindow);
        assert_eq!(modal.editing_task_id, Some(42));
        assert_eq!(modal.schedule_type, ScheduleType::AtTime);
        assert_eq!(modal.active_field, InputField::Command);
    }

    // =========================================================================
    // Tests for real cursor positioning (no pipe character in display)
    // Bug: Previous implementation inserted "|" at cursor position, which
    // appeared in copied text. Now using frame.set_cursor_position() instead.
    // =========================================================================

    #[test]
    fn render_text_field_does_not_include_pipe() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 40, 3);
                render_text_field(
                    frame,
                    area,
                    "Command",
                    "echo hello",
                    true, // active
                    Some(5),
                );
            })
            .unwrap();

        // Get the rendered buffer content
        let buffer = terminal.backend().buffer();

        // Check that no pipe character appears in the rendered text
        // The field should contain "echo hello" without a "|" character
        let mut found_pipe_in_value = false;
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                // Check the content area (inside the border)
                if y == 1 && x > 0 && x < 39 {
                    if cell.symbol() == "|" {
                        // Pipe found - but it could be part of the border, check if it's in the value area
                        // The value starts after the left border (x >= 1)
                        if x >= 1 && x <= 20 {
                            found_pipe_in_value = true;
                        }
                    }
                }
            }
        }

        assert!(
            !found_pipe_in_value,
            "Pipe character should not appear in field text"
        );
    }

    #[test]
    fn cursor_positioned_at_correct_offset() {
        use ratatui::backend::TestBackend;
        use ratatui::prelude::Position;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(5, 2, 40, 3);
                render_text_field(
                    frame,
                    area,
                    "Command",
                    "hello",
                    true, // active
                    Some(3), // cursor at position 3 (after "hel")
                );
            })
            .unwrap();

        // The cursor position should be set
        // inner area is area with borders removed: x+1, y+1
        // So cursor_x = (5+1) + 3 = 9, cursor_y = 2+1 = 3
        let expected_x = 5 + 1 + 3; // area.x + border + cursor_offset
        let expected_y = 2 + 1; // area.y + border

        terminal
            .backend_mut()
            .assert_cursor_position(Position::new(expected_x, expected_y));
    }

    #[test]
    fn cursor_at_end_of_value() {
        use ratatui::backend::TestBackend;
        use ratatui::prelude::Position;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 40, 3);
                render_text_field(
                    frame,
                    area,
                    "Label",
                    "test",
                    true,
                    None, // cursor_pos None means end of value
                );
            })
            .unwrap();

        // Cursor should be at end of "test" (length 4)
        // inner area starts at x=1 (after border)
        let expected_x = 1 + 4; // border + value.len()
        let expected_y = 1; // border

        terminal
            .backend_mut()
            .assert_cursor_position(Position::new(expected_x, expected_y));
    }

    #[test]
    fn cursor_not_moved_when_field_inactive() {
        use ratatui::backend::{Backend, TestBackend};
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        // Get the initial cursor position
        let initial_pos = terminal.backend_mut().get_cursor_position().unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 40, 3);
                render_text_field(
                    frame,
                    area,
                    "Label",
                    "test",
                    false, // NOT active
                    Some(2),
                );
            })
            .unwrap();

        // Cursor position should not have changed (inactive field doesn't set cursor)
        let final_pos = terminal.backend_mut().get_cursor_position().unwrap();
        assert_eq!(
            initial_pos, final_pos,
            "Cursor should not be moved when field is inactive"
        );
    }

    #[test]
    fn placeholder_shown_when_empty_and_inactive() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 50, 3);
                render_text_field_with_placeholder(
                    frame,
                    area,
                    "When",
                    "",                      // empty value
                    "e.g., 7:00am or 19:30", // placeholder
                    false,                   // NOT active
                    0,                       // cursor_pos
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Check that placeholder text appears in the buffer
        // The placeholder should be visible somewhere in row 1 (inside borders)
        let row_content: String = (1..49)
            .map(|x| buffer[(x, 1)].symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(
            row_content.contains("e.g."),
            "Placeholder should be visible when field is empty and inactive. Got: '{}'",
            row_content
        );
    }

    #[test]
    fn placeholder_hidden_when_active_and_empty() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 50, 3);
                render_text_field_with_placeholder(
                    frame,
                    area,
                    "When",
                    "",                      // empty value
                    "e.g., 7:00am or 19:30", // placeholder
                    true,                    // active
                    0,                       // cursor_pos
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Check that placeholder text does NOT appear when active
        let row_content: String = (1..49)
            .map(|x| buffer[(x, 1)].symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(
            !row_content.contains("e.g."),
            "Placeholder should NOT be visible when field is active. Got: '{}'",
            row_content
        );
    }

    #[test]
    fn compact_multiline_cursor_on_first_line() {
        use ratatui::backend::TestBackend;
        use ratatui::prelude::Position;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 4);
                render_compact_multiline_text_field(
                    frame,
                    area,
                    "Command",
                    "short",
                    None,
                    true,
                    Some(3), // cursor at position 3
                );
            })
            .unwrap();

        // Label prefix: " Command   : " = 13 chars
        // Cursor at position 3 within value
        let expected_x = 0 + 13 + 3; // area.x + label_prefix + cursor_offset
        let expected_y = 0;

        terminal
            .backend_mut()
            .assert_cursor_position(Position::new(expected_x, expected_y));
    }

    #[test]
    fn compact_multiline_cursor_on_wrapped_line() {
        use ratatui::backend::TestBackend;
        use ratatui::prelude::Position;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        // With 80 width, content_width = 80 - 13 = 67
        // Create a command that wraps to second line
        let long_command = "x".repeat(70); // 70 chars, wraps at 67

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 4);
                render_compact_multiline_text_field(
                    frame,
                    area,
                    "Command",
                    &long_command,
                    None,
                    true,
                    Some(68), // cursor at position 68 (1 char into second line)
                );
            })
            .unwrap();

        // First line has 67 chars, cursor at 68 is position 1 on second line
        // 68 - 67 = 1, so column 1 on line 1 (0-indexed)
        let expected_x = 0 + 13 + 1; // area.x + label_prefix + column
        let expected_y = 1; // second line

        terminal
            .backend_mut()
            .assert_cursor_position(Position::new(expected_x, expected_y));
    }

    // =========================================================================
    // Tests for Ctrl+A/E cursor movement and schedule_value cursor tracking
    // =========================================================================

    #[test]
    fn move_cursor_start_moves_command_cursor_to_beginning() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.command = "echo hello".to_string();
        modal.cursor_pos = 5;
        modal.active_field = InputField::Command;

        modal.move_cursor_start();

        assert_eq!(modal.cursor_pos, 0);
    }

    #[test]
    fn move_cursor_end_moves_command_cursor_to_end() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.command = "echo hello".to_string();
        modal.cursor_pos = 0;
        modal.active_field = InputField::Command;

        modal.move_cursor_end();

        assert_eq!(modal.cursor_pos, 10); // "echo hello".len()
    }

    #[test]
    fn move_cursor_start_moves_schedule_value_cursor_to_beginning() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.schedule_value = "15:30".to_string();
        modal.schedule_value_cursor_pos = 3;
        modal.active_field = InputField::ScheduleValue;

        modal.move_cursor_start();

        assert_eq!(modal.schedule_value_cursor_pos, 0);
    }

    #[test]
    fn move_cursor_end_moves_schedule_value_cursor_to_end() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.schedule_value = "15:30".to_string();
        modal.schedule_value_cursor_pos = 0;
        modal.active_field = InputField::ScheduleValue;

        modal.move_cursor_end();

        assert_eq!(modal.schedule_value_cursor_pos, 5); // "15:30".len()
    }

    #[test]
    fn schedule_value_cursor_moves_left_and_right() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.schedule_value = "15:30".to_string();
        modal.schedule_value_cursor_pos = 2;
        modal.active_field = InputField::ScheduleValue;

        modal.move_cursor_left();
        assert_eq!(modal.schedule_value_cursor_pos, 1);

        modal.move_cursor_right();
        assert_eq!(modal.schedule_value_cursor_pos, 2);

        modal.move_cursor_right();
        assert_eq!(modal.schedule_value_cursor_pos, 3);
    }

    #[test]
    fn schedule_value_cursor_stays_at_boundaries() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.schedule_value = "15m".to_string();
        modal.active_field = InputField::ScheduleValue;

        // Start at beginning
        modal.schedule_value_cursor_pos = 0;
        modal.move_cursor_left();
        assert_eq!(modal.schedule_value_cursor_pos, 0, "Should stay at 0");

        // Move to end
        modal.schedule_value_cursor_pos = 3;
        modal.move_cursor_right();
        assert_eq!(modal.schedule_value_cursor_pos, 3, "Should stay at end");
    }

    #[test]
    fn schedule_value_insert_at_cursor() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.schedule_value = "1:30".to_string();
        modal.schedule_value_cursor_pos = 1;
        modal.active_field = InputField::ScheduleValue;

        modal.handle_char('5');

        assert_eq!(modal.schedule_value, "15:30");
        assert_eq!(modal.schedule_value_cursor_pos, 2);
    }

    #[test]
    fn schedule_value_backspace_at_cursor() {
        let mut modal = InputModal::new(wezterm_caps());
        modal.schedule_value = "15:30".to_string();
        modal.schedule_value_cursor_pos = 2;
        modal.active_field = InputField::ScheduleValue;

        modal.handle_backspace();

        assert_eq!(modal.schedule_value, "1:30");
        assert_eq!(modal.schedule_value_cursor_pos, 1);
    }
}
