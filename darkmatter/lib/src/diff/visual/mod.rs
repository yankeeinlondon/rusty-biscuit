//! Visual diff rendering for terminal output.
//!
//! Provides line-by-line diff visualization with two display modes:
//! - Side-by-side (terminals > 110 columns)
//! - Unified (terminals <= 110 columns)
//!
//! ## Examples
//!
//! ```rust
//! use darkmatter_lib::diff::visual::{render_visual_diff_str, VisualDiffOptions};
//!
//! let original = "Hello\nWorld";
//! let updated = "Hello\nUniverse";
//!
//! let options = VisualDiffOptions::default();
//! let output = render_visual_diff_str(original, updated, &options);
//! println!("{}", output);
//! ```

mod diff;
mod side_by_side;
mod unified;

pub use diff::{compute_visual_diff, DiffLine, InlineSpan};

use std::path::Path;
use terminal_size::{terminal_size, Width};

/// Threshold for switching between side-by-side and unified views.
const SIDE_BY_SIDE_THRESHOLD: u16 = 110;

/// The visual diff layout mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualDiffMode {
    /// Automatically choose based on terminal width.
    Auto,
    /// Force side-by-side rendering.
    SideBySide,
    /// Force unified rendering.
    Unified,
}

/// Options for rendering visual diffs.
#[derive(Debug, Clone)]
pub struct VisualDiffOptions {
    /// Terminal width in columns.
    pub terminal_width: u16,
    /// Whether to show line numbers.
    pub show_line_numbers: bool,
    /// Number of context lines around changes.
    pub context_lines: usize,
    /// Layout mode for rendering.
    pub mode: VisualDiffMode,
}

impl Default for VisualDiffOptions {
    fn default() -> Self {
        let width = terminal_size().map(|(Width(w), _)| w).unwrap_or(80);
        Self {
            terminal_width: width,
            show_line_numbers: true,
            context_lines: 3,
            mode: VisualDiffMode::Auto,
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

    /// Resolve the effective render mode.
    pub fn resolved_mode(&self) -> VisualDiffMode {
        match self.mode {
            VisualDiffMode::Auto => {
                if self.terminal_width > SIDE_BY_SIDE_THRESHOLD {
                    VisualDiffMode::SideBySide
                } else {
                    VisualDiffMode::Unified
                }
            }
            other => other,
        }
    }
}

/// Input for visual diff rendering.
pub struct VisualDiffInput<'a> {
    /// The original text content.
    pub original: &'a str,
    /// The updated text content.
    pub updated: &'a str,
    /// Label for the original side (e.g., filename).
    pub label_original: &'a str,
    /// Label for the updated side (e.g., filename).
    pub label_updated: &'a str,
}

/// Summary statistics for a visual diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualDiffStats {
    /// Number of added lines.
    pub added: usize,
    /// Number of removed lines.
    pub removed: usize,
    /// Number of context (unchanged) lines.
    pub unchanged: usize,
    /// Best-effort count of modified lines.
    pub modified: usize,
}

/// Output from visual diff rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualDiffOutput {
    /// Rendered diff with ANSI escape codes.
    pub rendered: String,
    /// Mode used to render the diff.
    pub mode: VisualDiffMode,
    /// Summary statistics for the diff.
    pub stats: VisualDiffStats,
}

/// Render a visual diff between two strings.
///
/// Automatically selects between side-by-side and unified views based on
/// terminal width (unless mode is forced). The output includes ANSI color
/// codes for terminal display.
pub fn render_visual_diff(
    input: VisualDiffInput<'_>,
    options: &VisualDiffOptions,
) -> VisualDiffOutput {
    let diff = compute_visual_diff(input.original, input.updated);
    let mode = options.resolved_mode();
    let rendered = match mode {
        VisualDiffMode::SideBySide => {
            side_by_side::render(&diff, input.label_original, input.label_updated, options)
        }
        VisualDiffMode::Unified => {
            unified::render(&diff, input.label_original, input.label_updated, options)
        }
        VisualDiffMode::Auto => {
            // Auto is always resolved, but keep a safe fallback.
            unified::render(&diff, input.label_original, input.label_updated, options)
        }
    };

    let stats = compute_stats(&diff);

    VisualDiffOutput {
        rendered,
        mode,
        stats,
    }
}

/// Render a visual diff between two strings, returning only the rendered output.
pub fn render_visual_diff_str(
    original: &str,
    updated: &str,
    options: &VisualDiffOptions,
) -> String {
    render_visual_diff(
        VisualDiffInput {
            original,
            updated,
            label_original: "original",
            label_updated: "updated",
        },
        options,
    )
    .rendered
}

/// Render a visual diff between two files.
pub fn render_visual_diff_files(
    path_original: &Path,
    path_updated: &Path,
    options: &VisualDiffOptions,
) -> std::io::Result<String> {
    let original = std::fs::read_to_string(path_original)?;
    let updated = std::fs::read_to_string(path_updated)?;
    let (label_original, label_updated) = default_labels(path_original, path_updated);

    Ok(render_visual_diff(
        VisualDiffInput {
            original: &original,
            updated: &updated,
            label_original: &label_original,
            label_updated: &label_updated,
        },
        options,
    )
    .rendered)
}

/// Default labels for file-based diffs.
pub fn default_labels(path_original: &Path, path_updated: &Path) -> (String, String) {
    (
        path_original.to_string_lossy().to_string(),
        path_updated.to_string_lossy().to_string(),
    )
}

fn compute_stats(diff: &[DiffLine]) -> VisualDiffStats {
    let mut stats = VisualDiffStats::default();

    for line in diff {
        if line.is_context() {
            stats.unchanged += 1;
        } else if line.is_removed() {
            stats.removed += 1;
        } else if line.is_added() {
            stats.added += 1;
        }
    }

    stats.modified = stats.added.min(stats.removed);
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = VisualDiffOptions::default();
        assert!(options.show_line_numbers);
        assert_eq!(options.context_lines, 3);
        assert_eq!(options.mode, VisualDiffMode::Auto);
    }

    #[test]
    fn test_with_width() {
        let options = VisualDiffOptions::with_width(120);
        assert_eq!(options.terminal_width, 120);
        assert_eq!(options.resolved_mode(), VisualDiffMode::SideBySide);

        let options = VisualDiffOptions::with_width(80);
        assert_eq!(options.terminal_width, 80);
        assert_eq!(options.resolved_mode(), VisualDiffMode::Unified);
    }

    #[test]
    fn test_render_identical_files() {
        let content = "Hello\nWorld";
        let options = VisualDiffOptions::with_width(80);
        let output = render_visual_diff(
            VisualDiffInput {
                original: content,
                updated: content,
                label_original: "a.md",
                label_updated: "b.md",
            },
            &options,
        )
        .rendered;

        // Should contain context lines but no add/remove markers
        assert!(!output.contains("\x1b[48;5;22m")); // No green background
        assert!(!output.contains("\x1b[48;5;52m")); // No red background
    }
}
