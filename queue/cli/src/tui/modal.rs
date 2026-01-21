//! Modal overlay infrastructure for the TUI.
//!
//! This module provides reusable modal dialog rendering with proper overlay handling.
//! Modals are rendered centered over the main content using the three-step pattern:
//! 1. Calculate centered area
//! 2. Clear the area (important for proper overlay)
//! 3. Render modal border and content

use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Trait for modal dialogs.
///
/// Implement this trait to create custom modal dialogs that can be rendered
/// using the [`render_modal`] function.
pub trait Modal {
    /// Render the modal content (without the overlay border).
    fn render(&self, frame: &mut Frame, area: Rect);

    /// Get the modal title displayed in the border.
    fn title(&self) -> &str;

    /// Get the desired width percentage (0-100).
    fn width_percent(&self) -> u16 {
        60
    }

    /// Get the desired height percentage (0-100).
    fn height_percent(&self) -> u16 {
        40
    }
}

/// Renders a modal overlay centered on the screen.
///
/// This function handles the three-step modal rendering pattern:
/// 1. Calculate centered area based on modal dimensions
/// 2. Clear the modal area with the `Clear` widget
/// 3. Render the modal border/title and delegate content rendering
///
/// ## Arguments
///
/// * `frame` - The frame to render to
/// * `modal` - The modal implementation to render
/// * `main_area` - The parent area to center within (typically `frame.area()`)
pub fn render_modal(frame: &mut Frame, modal: &impl Modal, main_area: Rect) {
    let modal_area = centered_rect(modal.width_percent(), modal.height_percent(), main_area);

    // Step 1: Clear the modal area (essential for proper overlay)
    frame.render_widget(Clear, modal_area);

    // Step 2: Render the modal border/title
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", modal.title()))
        .style(Style::default().bg(Color::DarkGray));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Step 3: Render the modal content
    modal.render(frame, inner);
}

/// Calculate a centered rectangle with the given percentage of parent size.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .split(area);

    Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}

/// Confirm quit dialog asking the user to confirm before exiting.
pub struct ConfirmQuitDialog;

impl Modal for ConfirmQuitDialog {
    fn title(&self) -> &str {
        "Quit?"
    }

    fn width_percent(&self) -> u16 {
        40
    }

    fn height_percent(&self) -> u16 {
        20
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let text = vec![
            Line::from("Are you sure you want to quit?"),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " Y ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Yes  "),
                Span::styled(
                    " N ",
                    Style::default()
                        .bg(Color::Red)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" No"),
            ]),
        ];

        let paragraph = Paragraph::new(text).alignment(Alignment::Center);

        // Center vertically within the modal
        let centered = Layout::vertical([Constraint::Length(3)])
            .flex(Flex::Center)
            .split(area);

        frame.render_widget(paragraph, centered[0]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_calculates_correct_size() {
        let parent = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(60, 40, parent);

        // Width should be 60% of 100 = 60
        assert_eq!(centered.width, 60);
        // Height should be 40% of 50 = 20
        assert_eq!(centered.height, 20);
    }

    #[test]
    fn centered_rect_is_centered() {
        let parent = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 50, parent);

        // Should be centered horizontally: (100 - 50) / 2 = 25
        assert_eq!(centered.x, 25);
        // Should be centered vertically: (50 - 25) / 2 = 12 (rounded)
        assert!(centered.y >= 12 && centered.y <= 13);
    }

    #[test]
    fn confirm_quit_dialog_has_correct_dimensions() {
        let dialog = ConfirmQuitDialog;
        assert_eq!(dialog.width_percent(), 40);
        assert_eq!(dialog.height_percent(), 20);
        assert_eq!(dialog.title(), "Quit?");
    }

    #[test]
    fn modal_trait_has_default_dimensions() {
        struct TestModal;
        impl Modal for TestModal {
            fn render(&self, _frame: &mut Frame, _area: Rect) {}
            fn title(&self) -> &str {
                "Test"
            }
        }

        let modal = TestModal;
        assert_eq!(modal.width_percent(), 60);
        assert_eq!(modal.height_percent(), 40);
    }
}
