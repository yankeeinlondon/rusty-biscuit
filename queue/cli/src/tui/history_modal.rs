//! History modal for viewing previously queued commands.

use queue_lib::{HistoryStore, JsonFileStore, ScheduledTask, TaskStatus};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::modal::Modal;

/// History modal for viewing and selecting previous commands.
pub struct HistoryModal {
    pub items: Vec<ScheduledTask>,
    pub list_state: ListState,
    pub filter: String,
    pub filter_mode: bool,
}

impl HistoryModal {
    pub fn new() -> Self {
        let store = JsonFileStore::default_path();
        let items = store.load_all().unwrap_or_default();

        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }

        Self {
            items,
            list_state: state,
            filter: String::new(),
            filter_mode: false,
        }
    }

    /// Get the filtered items.
    pub fn filtered_items(&self) -> Vec<&ScheduledTask> {
        if self.filter.is_empty() {
            self.items.iter().collect()
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.items
                .iter()
                .filter(|task| task.command.to_lowercase().contains(&filter_lower))
                .collect()
        }
    }

    /// Select next item.
    pub fn select_next(&mut self) {
        let filtered = self.filtered_items();
        if filtered.is_empty() {
            return;
        }

        let current = self.list_state.selected().unwrap_or(0);
        let next = (current + 1) % filtered.len();
        self.list_state.select(Some(next));
    }

    /// Select previous item.
    pub fn select_previous(&mut self) {
        let filtered = self.filtered_items();
        if filtered.is_empty() {
            return;
        }

        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            filtered.len() - 1
        } else {
            current - 1
        };
        self.list_state.select(Some(prev));
    }

    /// Get selected task.
    pub fn selected_task(&self) -> Option<&ScheduledTask> {
        let filtered = self.filtered_items();
        self.list_state
            .selected()
            .and_then(|i| filtered.get(i).copied())
    }

    /// Toggle filter mode.
    #[cfg(test)]
    pub fn toggle_filter_mode(&mut self) {
        self.filter_mode = !self.filter_mode;
        if !self.filter_mode && !self.filter.is_empty() {
            let filtered = self.filtered_items();
            if !filtered.is_empty() {
                self.list_state.select(Some(0));
            }
        }
    }

    /// Handle character input for filter.
    pub fn handle_char(&mut self, c: char) {
        if self.filter_mode {
            self.filter.push(c);
            let filtered = self.filtered_items();
            if !filtered.is_empty() {
                self.list_state.select(Some(0));
            } else {
                self.list_state.select(None);
            }
        }
    }

    /// Handle backspace for filter.
    pub fn handle_backspace(&mut self) {
        if self.filter_mode {
            self.filter.pop();
            let filtered = self.filtered_items();
            if !filtered.is_empty() {
                self.list_state.select(Some(0));
            }
        }
    }

    /// Clear filter.
    #[allow(dead_code)]
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.filter_mode = false;
        if !self.items.is_empty() {
            self.list_state.select(Some(0));
        }
    }
}

impl Default for HistoryModal {
    fn default() -> Self {
        Self::new()
    }
}

impl Modal for HistoryModal {
    fn title(&self) -> &str {
        "Command History"
    }

    fn width_percent(&self) -> u16 {
        80
    }
    fn height_percent(&self) -> u16 {
        70
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Filter
            Constraint::Min(3),    // List
            Constraint::Length(2), // Help
        ])
        .split(area);

        // Filter field
        let filter_style = if self.filter_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let filter_text = if self.filter.is_empty() && !self.filter_mode {
            "Press F to filter".to_string()
        } else if self.filter_mode {
            format!("Filter: {}|", self.filter)
        } else {
            format!("Filter: {}", self.filter)
        };

        let filter_block = Block::default()
            .borders(Borders::ALL)
            .title(" Search ")
            .border_style(filter_style);

        let filter_para = Paragraph::new(filter_text)
            .style(if self.filter.is_empty() && !self.filter_mode {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            })
            .block(filter_block);

        frame.render_widget(filter_para, chunks[0]);

        // History list
        let filtered = self.filtered_items();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|task| {
                let status_char = match task.status {
                    TaskStatus::Completed => "✓",
                    TaskStatus::Failed { .. } => "✗",
                    TaskStatus::Running => "▶",
                    TaskStatus::Pending => "○",
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", status_char),
                        match task.status {
                            TaskStatus::Completed => Style::default().fg(Color::Green),
                            TaskStatus::Failed { .. } => Style::default().fg(Color::Red),
                            TaskStatus::Running => Style::default().fg(Color::Yellow),
                            TaskStatus::Pending => Style::default().fg(Color::DarkGray),
                        },
                    ),
                    Span::raw(&task.command),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" History "))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, chunks[1], &mut self.list_state.clone());

        // Help text
        let help_text = if self.filter_mode {
            "Type to filter | Esc: Exit filter | Enter: Select"
        } else {
            "↑↓: Navigate | Enter: Use command | N: New task | F: Filter | Esc: Close"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use queue_lib::ExecutionTarget;

    fn create_test_task(id: u64, command: &str) -> ScheduledTask {
        ScheduledTask {
            id,
            command: command.to_string(),
            scheduled_at: Utc::now(),
            target: ExecutionTarget::Background,
            status: TaskStatus::Completed,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn select_next_cycles_through_items() {
        let mut modal = HistoryModal {
            items: vec![
                create_test_task(1, "echo 1"),
                create_test_task(2, "echo 2"),
                create_test_task(3, "echo 3"),
            ],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: false,
        };
        modal.list_state.select(Some(0));

        modal.select_next();
        assert_eq!(modal.list_state.selected(), Some(1));

        modal.select_next();
        assert_eq!(modal.list_state.selected(), Some(2));

        modal.select_next(); // Wraps
        assert_eq!(modal.list_state.selected(), Some(0));
    }

    #[test]
    fn filter_narrows_results() {
        let modal = HistoryModal {
            items: vec![
                create_test_task(1, "echo hello"),
                create_test_task(2, "ls -la"),
                create_test_task(3, "echo world"),
            ],
            list_state: ListState::default(),
            filter: "echo".to_string(),
            filter_mode: false,
        };

        let filtered = modal.filtered_items();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn toggle_filter_mode() {
        let mut modal = HistoryModal {
            items: vec![],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: false,
        };

        assert!(!modal.filter_mode);
        modal.toggle_filter_mode();
        assert!(modal.filter_mode);
        modal.toggle_filter_mode();
        assert!(!modal.filter_mode);
    }

    #[test]
    fn handle_char_adds_to_filter() {
        let mut modal = HistoryModal {
            items: vec![create_test_task(1, "test")],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: true,
        };

        modal.handle_char('a');
        modal.handle_char('b');
        assert_eq!(modal.filter, "ab");
    }

    #[test]
    fn handle_backspace_removes_from_filter() {
        let mut modal = HistoryModal {
            items: vec![],
            list_state: ListState::default(),
            filter: "abc".to_string(),
            filter_mode: true,
        };

        modal.handle_backspace();
        assert_eq!(modal.filter, "ab");
    }
}
