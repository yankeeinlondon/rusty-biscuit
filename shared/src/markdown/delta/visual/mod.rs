//! Visual diff rendering for terminal output.
//!
//! Provides line-by-line diff visualization with two display modes:
//! - Side-by-side (terminals > 110 columns)
//! - Unified (terminals <= 110 columns)
//!
//! ## Examples
//!
//! ```rust
//! use shared::markdown::delta::visual::{render_visual_diff, VisualDiffOptions};
//!
//! let original = "Hello\nWorld";
//! let updated = "Hello\nUniverse";
//!
//! let options = VisualDiffOptions::default();
//! let output = render_visual_diff(original, updated, "file1.md", "file2.md", &options);
//! println!("{}", output);
//! ```

mod diff;
mod side_by_side;
mod unified;

pub use diff::{compute_visual_diff, DiffLine, InlineSpan};

use terminal_size::{terminal_size, Width};

/// Threshold for switching between side-by-side and unified views.
const SIDE_BY_SIDE_THRESHOLD: u16 = 110;

/// Options for rendering visual diffs.
#[derive(Debug, Clone)]
pub struct VisualDiffOptions {
    /// Terminal width in columns.
    pub terminal_width: u16,
    /// Whether to show line numbers.
    pub show_line_numbers: bool,
    /// Number of context lines around changes.
    pub context_lines: usize,
}

impl Default for VisualDiffOptions {
    fn default() -> Self {
        let width = terminal_size().map(|(Width(w), _)| w).unwrap_or(80);
        Self {
            terminal_width: width,
            show_line_numbers: true,
            context_lines: 3,
        }
    }
}

impl VisualDiffOptions {
    /// Create options with a specific terminal width.
    pub fn with_width(width: u16) -> Self {
        Self {
            terminal_width: width,
            ..Default::default()
        }
    }

    /// Check if side-by-side mode should be used.
    pub fn use_side_by_side(&self) -> bool {
        self.terminal_width > SIDE_BY_SIDE_THRESHOLD
    }
}

/// Render a visual diff between two strings.
///
/// Automatically selects between side-by-side and unified views based on
/// terminal width. The output includes ANSI color codes for terminal display.
///
/// ## Arguments
///
/// * `original` - The original text content
/// * `updated` - The updated text content
/// * `label_original` - Label for the original side (e.g., filename)
/// * `label_updated` - Label for the updated side (e.g., filename)
/// * `options` - Rendering options
///
/// ## Returns
///
/// A string containing the formatted diff with ANSI escape codes.
pub fn render_visual_diff(
    original: &str,
    updated: &str,
    label_original: &str,
    label_updated: &str,
    options: &VisualDiffOptions,
) -> String {
    let diff = compute_visual_diff(original, updated);

    if options.use_side_by_side() {
        side_by_side::render(&diff, label_original, label_updated, options)
    } else {
        unified::render(&diff, label_original, label_updated, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = VisualDiffOptions::default();
        assert!(options.show_line_numbers);
        assert_eq!(options.context_lines, 3);
    }

    #[test]
    fn test_with_width() {
        let options = VisualDiffOptions::with_width(120);
        assert_eq!(options.terminal_width, 120);
        assert!(options.use_side_by_side());

        let options = VisualDiffOptions::with_width(80);
        assert_eq!(options.terminal_width, 80);
        assert!(!options.use_side_by_side());
    }

    #[test]
    fn test_render_identical_files() {
        let content = "Hello\nWorld";
        let options = VisualDiffOptions::with_width(80);
        let output = render_visual_diff(content, content, "a.md", "b.md", &options);
        // Should contain context lines but no add/remove markers
        assert!(!output.contains("\x1b[48;5;22m")); // No green background
        assert!(!output.contains("\x1b[48;5;52m")); // No red background
    }
}
