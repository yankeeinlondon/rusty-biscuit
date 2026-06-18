//! Terminal view for Markdown document deltas.

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use crate::diff::visual::{VisualDiffInput, VisualDiffOptions, render_visual_diff};
use crate::markdown::{
    ChangeAction, DocumentChange, Markdown, MarkdownDelta,
};

const INVERSE: &str = "\x1b[7m";
const BOLD: &str = "\x1b[1m";
const ITALIC: &str = "\x1b[3m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// A [`MarkdownDelta`] prepared for terminal output.
///
/// This is a terminal-only view (ADR-2). It preserves the visual shape of the
/// previous CLI delta report while moving the domain rendering out of
/// `darkmatter-cli`.
#[derive(Debug, Clone)]
pub struct DeltaReport {
    delta: MarkdownDelta,
    verbose: bool,
    original: Option<Markdown>,
    updated: Option<Markdown>,
    layout: Layout,
}

impl DeltaReport {
    /// Creates a terminal view of the given delta.
    pub fn new(delta: MarkdownDelta) -> Self {
        Self {
            delta,
            verbose: false,
            original: None,
            updated: None,
            layout: Layout::default(),
        }
    }

    /// Enables summary statistics and visual diff output.
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Adds the compared documents used for verbose visual diff output.
    pub fn with_documents(mut self, original: Markdown, updated: Markdown) -> Self {
        self.original = Some(original);
        self.updated = Some(updated);
        self
    }
}

impl TerminalRenderable for DeltaReport {
    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn render(&self, _term: &Terminal) -> String {
        render_delta_report(
            &self.delta,
            self.verbose,
            self.original.as_ref(),
            self.updated.as_ref(),
        )
    }
}

/// Formats a code block change with ANSI styling.
pub fn format_code_block_change(lang: &str, section_path: &str, description: &str) -> String {
    if let Some(rest) = description.strip_prefix("Language: ") {
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
             changed its {BOLD}language{RESET} setting: {rest}"
        )
    } else if let Some(rest) = description.strip_prefix("'") {
        if let Some((prop_name, value_part)) = rest.split_once("':") {
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
                 changed its {BOLD}{prop_name}{RESET} property:{value_part}"
            )
        } else {
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
                 changed: {description}"
            )
        }
    } else if description.starts_with("Modified") {
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
             was {BOLD}modified{RESET}"
        )
    } else if description.starts_with("Added") {
        format!("{INVERSE}{lang}{RESET} code block added in {BOLD}{section_path}{RESET}")
    } else if description.starts_with("Removed") {
        format!("{INVERSE}{lang}{RESET} code block removed from {BOLD}{section_path}{RESET}")
    } else {
        format!("{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET}: {description}")
    }
}

fn render_delta_report(
    delta: &MarkdownDelta,
    verbose: bool,
    original: Option<&Markdown>,
    updated: Option<&Markdown>,
) -> String {
    let mut out = String::new();
    out.push('\n');

    let (classification_symbol, classification_name) = match delta.classification {
        DocumentChange::NoChange => ("✓", "No changes"),
        DocumentChange::WhitespaceOnly => ("~", "Whitespace changes only"),
        DocumentChange::FrontmatterOnly => ("◈", "Frontmatter only"),
        DocumentChange::FrontmatterAndWhitespace => ("◈", "Frontmatter and whitespace"),
        DocumentChange::StructuralOnly => ("⊕", "Structural only"),
        DocumentChange::ContentMinor => ("△", "Minor changes"),
        DocumentChange::ContentModerate => ("◐", "Moderate changes"),
        DocumentChange::ContentMajor => ("◉", "Major changes"),
        DocumentChange::Rewritten => ("★", "Rewritten"),
    };

    out.push_str(&format!(
        "{} {} ({:.1}% changed)\n\n",
        classification_symbol,
        classification_name,
        delta.statistics.content_change_ratio * 100.0
    ));

    if delta.frontmatter_changed {
        out.push_str("Frontmatter:\n");
        if delta.frontmatter_formatting_only {
            out.push_str("  (formatting changes only)\n");
        } else {
            for change in &delta.frontmatter_changes {
                let symbol = match change.action {
                    ChangeAction::PropertyAdded => "+",
                    ChangeAction::PropertyRemoved => "-",
                    ChangeAction::PropertyUpdated => "~",
                    _ => "?",
                };
                out.push_str(&format!("  {} {}: {}\n", symbol, change.key, change.description));
            }
        }
        out.push('\n');
    }

    if delta.preamble_changed {
        if delta.preamble_whitespace_only {
            out.push_str("Preamble: whitespace changes only\n");
        } else {
            out.push_str("Preamble: modified\n");
        }
        out.push('\n');
    }

    if !delta.added.is_empty() {
        out.push_str(&format!("Added ({}):\n", delta.added.len()));
        for change in &delta.added {
            let path_str = change.new_path.as_ref().map(|p| p.join(" > ")).unwrap_or_default();
            if verbose {
                out.push_str(&format!("  + {} (line {})\n", path_str, change.new_line.unwrap_or(0)));
            } else {
                out.push_str(&format!("  + {}\n", path_str));
            }
        }
        out.push('\n');
    }

    if !delta.removed.is_empty() {
        out.push_str(&format!("Removed ({}):\n", delta.removed.len()));
        for change in &delta.removed {
            let path_str = change
                .original_path
                .as_ref()
                .map(|p| p.join(" > "))
                .unwrap_or_default();
            if verbose {
                out.push_str(&format!(
                    "  - {} (was line {})\n",
                    path_str,
                    change.original_line.unwrap_or(0)
                ));
            } else {
                out.push_str(&format!("  - {}\n", path_str));
            }
        }
        out.push('\n');
    }

    let content_changes: Vec<_> = delta
        .modified
        .iter()
        .filter(|c| !matches!(c.action, ChangeAction::WhitespaceOnly))
        .collect();
    let whitespace_changes: Vec<_> = delta
        .modified
        .iter()
        .filter(|c| matches!(c.action, ChangeAction::WhitespaceOnly))
        .collect();

    if !content_changes.is_empty() {
        out.push_str(&format!("Modified ({}):\n", content_changes.len()));
        for change in &content_changes {
            out.push_str(&format!("  - {}\n", change.description));
        }
        out.push('\n');
    }

    if !delta.moved.is_empty() {
        out.push_str(&format!("Moved ({}):\n", delta.moved.len()));
        for moved in &delta.moved {
            let from = moved.original_path.join(" > ");
            let to = moved.new_path.join(" > ");
            let level_change = if moved.level_delta < 0 {
                format!(" (promoted by {})", -moved.level_delta)
            } else if moved.level_delta > 0 {
                format!(" (demoted by {})", moved.level_delta)
            } else {
                String::new()
            };
            out.push_str(&format!("  ↷ {} → {}{}\n", from, to, level_change));
        }
        out.push('\n');
    }

    if !delta.code_block_changes.is_empty() {
        out.push_str("Code blocks:\n");
        for change in &delta.code_block_changes {
            let lang = change.language.as_deref().unwrap_or("plain");
            let section_path = if change.section_path.len() > 1 {
                change.section_path[1..].join(" > ")
            } else if !change.section_path.is_empty() {
                change.section_path[0].clone()
            } else {
                String::from("(preamble)")
            };
            let formatted = format_code_block_change(lang, &section_path, &change.description);
            out.push_str(&format!("  - {}\n", formatted));
        }
        out.push('\n');
    }

    if !delta.broken_links.is_empty() {
        out.push_str(&format!("⚠ Broken links ({}):\n", delta.broken_links.len()));
        for link in &delta.broken_links {
            out.push_str(&format!("  ✗ #{} at line {}", link.target_slug, link.line_number));
            if let Some(ref suggestion) = link.suggested_replacement {
                out.push_str(&format!(" → did you mean #{}?\n", suggestion));
            } else {
                out.push('\n');
            }
        }
        out.push('\n');
    }

    if !whitespace_changes.is_empty() {
        out.push_str(&format!("Whitespace only ({}):\n", whitespace_changes.len()));
        for change in &whitespace_changes {
            let path_str = change
                .original_path
                .as_ref()
                .map(|p| {
                    if p.len() > 1 {
                        p[1..].join(" > ")
                    } else if !p.is_empty() {
                        p[0].clone()
                    } else {
                        String::from("(preamble)")
                    }
                })
                .unwrap_or_default();
            out.push_str(&format!(
                "  - {}: {ITALIC}{}{RESET}\n",
                path_str, change.description
            ));
        }
        out.push('\n');
        out.push_str(&format!(
            "  {DIM}{ITALIC}whitespace only changes have no visual effect when rendered{RESET}\n\n"
        ));
    }

    if verbose {
        let stats = &delta.statistics;
        out.push_str("Statistics:\n");
        out.push_str(&format!(
            "  Bytes: {} → {} ({} changed)\n",
            stats.original_bytes, stats.new_bytes, stats.bytes_changed
        ));
        out.push_str(&format!(
            "  Sections: {} → {} ({} unchanged)\n\n",
            stats.original_section_count, stats.new_section_count, stats.sections_unchanged
        ));

        if let (Some(original), Some(updated)) = (original, updated) {
            let options = VisualDiffOptions::default();

            if delta.frontmatter_changed && !delta.frontmatter_formatting_only {
                let fm_orig =
                    serde_yaml_ng::to_string(original.frontmatter().as_map()).unwrap_or_default();
                let fm_upd =
                    serde_yaml_ng::to_string(updated.frontmatter().as_map()).unwrap_or_default();

                if !fm_orig.is_empty() || !fm_upd.is_empty() {
                    out.push_str(&format!("{BOLD}Frontmatter Visual Diff:{RESET}\n"));
                    out.push_str(
                        &render_visual_diff(
                            VisualDiffInput {
                                original: &fm_orig,
                                updated: &fm_upd,
                                label_original: "original",
                                label_updated: "updated",
                            },
                            &options,
                        )
                        .rendered,
                    );
                    out.push('\n');
                }
            }

            let has_content_changes = !delta.added.is_empty()
                || !delta.removed.is_empty()
                || !delta.modified.is_empty()
                || delta.preamble_changed;

            if has_content_changes {
                out.push_str(
                    &render_visual_diff(
                        VisualDiffInput {
                            original: original.content(),
                            updated: updated.content(),
                            label_original: "original",
                            label_updated: "updated",
                        },
                        &options,
                    )
                    .rendered,
                );
                out.push('\n');
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;

    #[test]
    fn renders_empty_delta() {
        let rendered = DeltaReport::new(MarkdownDelta::new()).render(&Terminal::new_optimistic(80));
        assert_eq!(rendered, "\n✓ No changes (0.0% changed)\n\n");
    }

    #[test]
    fn renders_additions_and_deletions() {
        let original: Markdown = "# Doc\n\n## Old\n\nGone\n".into();
        let updated: Markdown = "# Doc\n\n## New\n\nAdded\n".into();
        let delta = original.delta(&updated);
        let rendered = DeltaReport::new(delta).render(&Terminal::new_optimistic(80));

        assert!(rendered.contains("Added (1):"));
        assert!(rendered.contains("  + Doc > New"));
        assert!(rendered.contains("Removed (1):"));
        assert!(rendered.contains("  - Doc > Old"));
    }

    #[test]
    fn renders_code_block_changes() {
        let original: Markdown = "# Doc\n\n## Code\n\n```rust\nlet a = 1;\n```\n".into();
        let updated: Markdown = "# Doc\n\n## Code\n\n```rust\nlet a = 2;\n```\n".into();
        let delta = original.delta(&updated);
        let rendered = DeltaReport::new(delta).render(&Terminal::new_optimistic(80));

        assert!(rendered.contains("Code blocks:"));
        assert!(rendered.contains("code block in"));
        assert!(rendered.contains("was \u{1b}[1mmodified\u{1b}[0m"));
    }
}
