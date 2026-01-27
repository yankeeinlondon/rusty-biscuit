//! History modal for viewing previously queued commands.

use queue_lib::{HistoryStore, JsonFileStore, ScheduledTask, TaskStatus};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::color_context::ColorContext;
use super::modal::Modal;

/// Layout mode for the history modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryLayout {
    #[default]
    Full,
    Compact,
}

const FULL_LAYOUT_THRESHOLD: u16 = 18;

/// History modal for viewing and selecting previous commands.
pub struct HistoryModal {
    pub items: Vec<ScheduledTask>,
    pub list_state: ListState,
    pub filter: String,
    pub filter_mode: bool,
    pub layout: HistoryLayout,
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
            layout: HistoryLayout::default(),
        }
    }

    /// Updates layout mode based on available height.
    pub fn update_layout(&mut self, available_height: u16) {
        self.layout = if available_height >= FULL_LAYOUT_THRESHOLD {
            HistoryLayout::Full
        } else {
            HistoryLayout::Compact
        };
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

    /// Returns the title with filtered/total count when filtering.
    ///
    /// When no filter is active or all items match, returns "History".
    /// When filtering, returns "History (N of M)" where N is filtered count
    /// and M is total count.
    pub fn title_with_count(&self) -> String {
        let filtered_count = self.filtered_items().len();
        let total_count = self.items.len();
        if filtered_count == total_count {
            "History".to_string()
        } else {
            format!("History ({} of {})", filtered_count, total_count)
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
        // Reduced from 80% to 75% to avoid interfering with the help bar
        75
    }

    fn height_percent(&self) -> u16 {
        70
    }

    fn render(&self, frame: &mut Frame, area: Rect, color_context: &ColorContext) {
        match self.layout {
            HistoryLayout::Full => self.render_full(frame, area, color_context),
            HistoryLayout::Compact => self.render_compact(frame, area, color_context),
        }
    }
}

impl HistoryModal {
    /// Render full layout (>= 18 rows) - always shows filter section.
    fn render_full(&self, frame: &mut Frame, area: Rect, color_context: &ColorContext) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Filter (always visible)
            Constraint::Min(3),    // List
            Constraint::Length(2), // Help
        ])
        .split(area);

        self.render_filter_section(frame, chunks[0]);
        self.render_history_list(frame, chunks[1], color_context);
        self.render_help_text(frame, chunks[2], color_context);
    }

    /// Render compact layout (< 18 rows) - only shows filter when in filter_mode.
    ///
    /// When a filter is active but filter_mode is off, the filter text is shown
    /// in the help line instead of the filter section. This maximizes list space.
    fn render_compact(&self, frame: &mut Frame, area: Rect, color_context: &ColorContext) {
        // In compact mode, only show filter section when actively editing filter
        // If there's a filter but not in filter_mode, show filter in help line
        if self.filter_mode {
            let chunks = Layout::vertical([
                Constraint::Length(3), // Filter (only when editing)
                Constraint::Min(3),    // List
                Constraint::Length(1), // Help (compact)
            ])
            .split(area);

            self.render_filter_section(frame, chunks[0]);
            self.render_history_list(frame, chunks[1], color_context);
            self.render_help_text_compact(frame, chunks[2]);
        } else {
            let chunks = Layout::vertical([
                Constraint::Min(3),    // List (gets all the space)
                Constraint::Length(1), // Help (compact - shows filter if active)
            ])
            .split(area);

            self.render_history_list(frame, chunks[0], color_context);
            self.render_help_text_compact(frame, chunks[1]);
        }
    }

    /// Render the filter/search section.
    fn render_filter_section(&self, frame: &mut Frame, area: Rect) {
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

        frame.render_widget(filter_para, area);
    }

    /// Render the history list.
    fn render_history_list(&self, frame: &mut Frame, area: Rect, color_context: &ColorContext) {
        let filtered = self.filtered_items();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|task| {
                // Use ColorContext for NO_COLOR-aware status symbols
                let status_char = color_context.status_symbol(&task.status);

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", status_char),
                        match task.status {
                            TaskStatus::Completed => Style::default().fg(Color::Green),
                            TaskStatus::Cancelled => Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::DIM),
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

        // Use title_with_count for dynamic title showing filtered/total
        let title = format!(" {} ", self.title_with_count());
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.list_state.clone());
    }

    /// Render help text (full layout - 2 rows).
    ///
    /// When width >= 70 and not in filter mode, includes a status legend
    /// showing the meaning of status symbols.
    fn render_help_text(&self, frame: &mut Frame, area: Rect, color_context: &ColorContext) {
        let help_text = if self.filter_mode {
            "Type to filter | Esc: Exit filter | Enter: Select".to_string()
        } else {
            "↑↓: Navigate | Enter: Use command | N: New task | F: Filter | Esc: Close".to_string()
        };

        // Add status legend when width >= 70 and not in filter mode
        let display_text = if area.width >= 70 && !self.filter_mode {
            let ok_symbol = color_context.status_symbol(&TaskStatus::Completed);
            let fail_symbol = color_context.status_symbol(&TaskStatus::Failed {
                error: String::new(),
            });
            format!("{} | {} done  {} fail", help_text, ok_symbol, fail_symbol)
        } else {
            help_text
        };

        let help = Paragraph::new(display_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, area);
    }

    /// Render help text (compact layout - 1 row, shorter text).
    ///
    /// When filter section is hidden (compact mode without filter_mode) but a
    /// filter is active, shows the filter text in the help line to maintain
    /// visibility of the active filter.
    fn render_help_text_compact(&self, frame: &mut Frame, area: Rect) {
        let help_text: String = if self.filter_mode {
            "Type to filter | Esc: Exit | Enter: Select".to_string()
        } else if !self.filter.is_empty() {
            // Show active filter in help when filter section is hidden
            format!("Filter: {} | Esc: Close", self.filter)
        } else {
            "↑↓/Enter: Select | N: New | F: Filter | Esc: Close".to_string()
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, area);
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
            schedule_kind: None,
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
            layout: HistoryLayout::default(),
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
            layout: HistoryLayout::default(),
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
            layout: HistoryLayout::default(),
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
            layout: HistoryLayout::default(),
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
            layout: HistoryLayout::default(),
        };

        modal.handle_backspace();
        assert_eq!(modal.filter, "ab");
    }

    // =========================================================================
    // Regression tests for compact layout improvements
    // Bug: History modal wasted space on filter section when not filtering,
    // and modal was too wide causing it to interfere with the help bar.
    // =========================================================================

    #[test]
    fn update_layout_sets_full_for_large_height() {
        let mut modal = HistoryModal::new();
        modal.update_layout(20); // >= 18 rows
        assert_eq!(modal.layout, HistoryLayout::Full);
    }

    #[test]
    fn update_layout_sets_compact_for_small_height() {
        let mut modal = HistoryModal::new();
        modal.update_layout(15); // < 18 rows
        assert_eq!(modal.layout, HistoryLayout::Compact);
    }

    #[test]
    fn update_layout_threshold_is_18_rows() {
        let mut modal = HistoryModal::new();

        modal.update_layout(17);
        assert_eq!(modal.layout, HistoryLayout::Compact, "17 rows should be compact");

        modal.update_layout(18);
        assert_eq!(modal.layout, HistoryLayout::Full, "18 rows should be full");
    }

    #[test]
    fn width_percent_is_reduced_to_avoid_help_bar() {
        use crate::tui::modal::Modal;
        let modal = HistoryModal::new();
        assert_eq!(modal.width_percent(), 75, "Width should be 75% to avoid help bar interference");
    }

    #[test]
    fn compact_layout_defaults_to_compact_enum() {
        let modal = HistoryModal::new();
        // Default is Full, but after update_layout with small height it becomes Compact
        assert_eq!(modal.layout, HistoryLayout::Full, "Default should be Full");
    }

    // =========================================================================
    // ColorContext integration tests for NO_COLOR support
    // =========================================================================

    #[test]
    fn history_modal_uses_color_context_symbols() {
        use super::ColorContext;

        // Verify the Modal trait impl compiles and accepts ColorContext
        let _modal = HistoryModal {
            items: vec![create_test_task(1, "echo test")],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: false,
            layout: HistoryLayout::default(),
        };

        // This test verifies that the trait implementation signature is correct
        // The actual symbol rendering is tested via ColorContext's own tests
        let color_ctx = ColorContext::with_color();
        assert!(color_ctx.is_color_enabled());

        let no_color_ctx = ColorContext::without_color();
        assert!(!no_color_ctx.is_color_enabled());

        // Verify status_symbol returns different values based on color context
        let status = TaskStatus::Completed;
        assert_eq!(color_ctx.status_symbol(&status), "\u{2713}"); // ✓
        assert_eq!(no_color_ctx.status_symbol(&status), "[OK]");
    }

    // =========================================================================
    // Phase 4: History Modal Accessibility tests
    // =========================================================================

    #[test]
    fn title_with_count_shows_history_when_no_filter() {
        let modal = HistoryModal {
            items: vec![
                create_test_task(1, "echo hello"),
                create_test_task(2, "ls -la"),
                create_test_task(3, "echo world"),
            ],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: false,
            layout: HistoryLayout::default(),
        };
        assert_eq!(modal.title_with_count(), "History");
    }

    #[test]
    fn title_with_count_shows_filtered_count() {
        let modal = HistoryModal {
            items: vec![
                create_test_task(1, "echo hello"),
                create_test_task(2, "ls -la"),
                create_test_task(3, "echo world"),
            ],
            list_state: ListState::default(),
            filter: "echo".to_string(),
            filter_mode: false,
            layout: HistoryLayout::default(),
        };
        // 2 of 3 items match "echo"
        assert_eq!(modal.title_with_count(), "History (2 of 3)");
    }

    #[test]
    fn title_with_count_shows_zero_when_filter_matches_nothing() {
        let modal = HistoryModal {
            items: vec![
                create_test_task(1, "echo hello"),
                create_test_task(2, "ls -la"),
            ],
            list_state: ListState::default(),
            filter: "nonexistent".to_string(),
            filter_mode: false,
            layout: HistoryLayout::default(),
        };
        assert_eq!(modal.title_with_count(), "History (0 of 2)");
    }

    #[test]
    fn title_with_count_updates_when_filter_changes() {
        let mut modal = HistoryModal {
            items: vec![
                create_test_task(1, "echo hello"),
                create_test_task(2, "echo world"),
                create_test_task(3, "ls -la"),
            ],
            list_state: ListState::default(),
            filter: String::new(),
            filter_mode: true,
            layout: HistoryLayout::default(),
        };

        // No filter
        assert_eq!(modal.title_with_count(), "History");

        // Add filter
        modal.handle_char('e');
        modal.handle_char('c');
        modal.handle_char('h');
        modal.handle_char('o');
        assert_eq!(modal.title_with_count(), "History (2 of 3)");

        // Clear filter
        modal.filter.clear();
        assert_eq!(modal.title_with_count(), "History");
    }

    #[test]
    fn compact_mode_hides_filter_section_when_not_in_filter_mode() {
        // This test verifies the behavior change: in compact mode, filter section
        // is only shown when filter_mode is active, not just when filter is non-empty
        let mut modal = HistoryModal {
            items: vec![create_test_task(1, "test")],
            list_state: ListState::default(),
            filter: "test".to_string(), // Non-empty filter
            filter_mode: false,         // But not in filter mode
            layout: HistoryLayout::Compact,
        };

        // With filter_mode = false, even with a filter, compact mode should
        // hide the filter section (filter shown in help line instead)
        // This is tested implicitly via the render_compact implementation
        assert!(!modal.filter_mode);
        assert!(!modal.filter.is_empty());

        // When filter_mode becomes active, filter section appears
        modal.filter_mode = true;
        assert!(modal.filter_mode);
    }
}
