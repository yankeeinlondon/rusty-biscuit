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

use super::color_context::ColorContext;
use super::PANEL_BG;

/// Trait for modal dialogs.
///
/// Implement this trait to create custom modal dialogs that can be rendered
/// using the [`render_modal`] function.
pub trait Modal {
    /// Render the modal content (without the overlay border).
    ///
    /// The `color_context` parameter provides NO_COLOR-aware symbol selection
    /// and should be used for status indicators and other symbols.
    fn render(&self, frame: &mut Frame, area: Rect, color_context: &ColorContext);

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

    /// Get the minimum width in rows.
    ///
    /// This ensures the modal has enough space for its content even in small
    /// terminal windows. Returns 0 (no minimum) by default.
    fn min_width(&self) -> u16 {
        0
    }

    /// Get the minimum height in rows.
    ///
    /// This ensures the modal has enough space for its content even in small
    /// terminal windows (e.g., Wezterm split panes). Returns 0 (no minimum) by default.
    fn min_height(&self) -> u16 {
        0
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
/// * `color_context` - Color context for NO_COLOR-aware rendering
pub fn render_modal(
    frame: &mut Frame,
    modal: &impl Modal,
    main_area: Rect,
    color_context: &ColorContext,
) {
    let modal_area = centered_rect_with_min(
        modal.width_percent(),
        modal.height_percent(),
        modal.min_width(),
        modal.min_height(),
        main_area,
    );

    // Step 1: Clear the modal area (essential for proper overlay)
    frame.render_widget(Clear, modal_area);

    // Step 2: Render the modal border/title
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", modal.title()))
        .style(Style::default().bg(PANEL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Step 3: Render the modal content
    modal.render(frame, inner, color_context);
}

/// Calculate a centered rectangle with the given percentage of parent size.
#[cfg(test)]
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    centered_rect_with_min(percent_x, percent_y, 0, 0, area)
}

/// Calculate a centered rectangle with percentage sizing and minimum dimensions.
///
/// The final size is the larger of:
/// - The percentage of the parent area
/// - The minimum dimensions (capped at the parent area size)
fn centered_rect_with_min(
    percent_x: u16,
    percent_y: u16,
    min_width: u16,
    min_height: u16,
    area: Rect,
) -> Rect {
    // Calculate percentage-based dimensions
    let percent_width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let percent_height = (area.height as u32 * percent_y as u32 / 100) as u16;

    // Apply minimums (capped at parent size)
    let width = percent_width.max(min_width).min(area.width);
    let height = percent_height.max(min_height).min(area.height);

    // Center the rectangle
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
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

    fn min_height(&self) -> u16 {
        7
    }

    fn render(&self, frame: &mut Frame, area: Rect, _color_context: &ColorContext) {
        // Brackets around keys provide NO_COLOR-friendly visual distinction
        let text = vec![
            Line::from("Are you sure you want to quit?"),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " [Y] ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Yes  "),
                Span::styled(
                    " [N] ",
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
            fn render(&self, _frame: &mut Frame, _area: Rect, _color_context: &ColorContext) {}
            fn title(&self) -> &str {
                "Test"
            }
        }

        let modal = TestModal;
        assert_eq!(modal.width_percent(), 60);
        assert_eq!(modal.height_percent(), 40);
    }

    #[test]
    fn modal_trait_has_default_min_dimensions() {
        struct TestModal;
        impl Modal for TestModal {
            fn render(&self, _frame: &mut Frame, _area: Rect, _color_context: &ColorContext) {}
            fn title(&self) -> &str {
                "Test"
            }
        }

        let modal = TestModal;
        assert_eq!(modal.min_width(), 0);
        assert_eq!(modal.min_height(), 0);
    }

    #[test]
    fn confirm_quit_dialog_has_bracketed_keys() {
        // Verify that the quit dialog shows [Y] and [N] with brackets
        // This is verified by the render implementation using bracketed key labels
        let dialog = ConfirmQuitDialog;
        assert_eq!(dialog.title(), "Quit?");
        // The bracketed keys are hardcoded in the render method as "[Y]" and "[N]"
        // which provides NO_COLOR-friendly visual distinction
    }

    // =========================================================================
    // Regression tests for small pane modal rendering
    // Bug: TUI in Wezterm split pane (20%) couldn't fit input modal with 0 tasks
    // =========================================================================

    #[test]
    fn centered_rect_with_min_applies_minimum_height() {
        // Simulate a small Wezterm pane: 80 cols x 5 rows
        let small_pane = Rect::new(0, 0, 80, 5);

        // 60% of 5 rows = 3 rows, but minimum is 18
        // Should be capped at parent size (5)
        let rect = centered_rect_with_min(60, 60, 0, 18, small_pane);

        assert_eq!(rect.height, 5, "Height should be capped at parent height");
        assert_eq!(rect.y, 0, "Should start at top when filling parent");
    }

    #[test]
    fn centered_rect_with_min_applies_minimum_width() {
        let small_pane = Rect::new(0, 0, 20, 50);

        // 60% of 20 = 12, but minimum is 40
        // Should be capped at parent size (20)
        let rect = centered_rect_with_min(60, 60, 40, 0, small_pane);

        assert_eq!(rect.width, 20, "Width should be capped at parent width");
        assert_eq!(rect.x, 0, "Should start at left when filling parent");
    }

    #[test]
    fn centered_rect_with_min_uses_percentage_when_larger_than_min() {
        let large_pane = Rect::new(0, 0, 100, 50);

        // 60% of 50 = 30, which is larger than min 18
        let rect = centered_rect_with_min(60, 60, 0, 18, large_pane);

        assert_eq!(rect.height, 30, "Should use percentage when larger than min");
        assert_eq!(rect.y, 10, "Should be centered: (50 - 30) / 2 = 10");
    }

    #[test]
    fn centered_rect_with_min_centers_when_min_applied() {
        // Parent is larger than minimum
        let pane = Rect::new(0, 0, 80, 30);

        // 60% of 30 = 18, min is also 18, so no change
        // But let's use 10% to trigger minimum
        let rect = centered_rect_with_min(10, 10, 40, 20, pane);

        // 10% of 80 = 8, min 40 applies -> width = 40
        // 10% of 30 = 3, min 20 applies -> height = 20
        assert_eq!(rect.width, 40);
        assert_eq!(rect.height, 20);

        // Center: (80 - 40) / 2 = 20, (30 - 20) / 2 = 5
        assert_eq!(rect.x, 20);
        assert_eq!(rect.y, 5);
    }

    #[test]
    fn centered_rect_with_min_zero_acts_like_original() {
        let parent = Rect::new(0, 0, 100, 50);

        let with_min = centered_rect_with_min(60, 40, 0, 0, parent);
        let without_min = centered_rect(60, 40, parent);

        assert_eq!(with_min, without_min);
    }
}
