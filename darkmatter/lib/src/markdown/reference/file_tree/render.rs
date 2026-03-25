//! Terminal rendering for the FileTree component.
//!
//! Implements the three-zone layout: reference groups above the file head line,
//! the file head itself, and transclusion edges below.

use biscuit_terminal::terminal::Terminal;

use crate::markdown::reference::validate::ReferenceSeverity;
use super::icons;
use super::model::{
    FileTreeModel, FileTreeNode, FileTreeReferenceGroup, FileTreeTransclusionKind,
};

// ── Connector strings ───────────────────────────────────────────────

const CONNECTOR_VERTICAL: &str = "    \u{2502}";
const CONNECTOR_REF_PREFIX: &str = "    \u{2502} \u{0305} \u{0305} \u{0305} \u{0305} ";
const CONNECTOR_TRANSCLUSION_IN: &str = "    \u{2502}<--- ";
const CONNECTOR_TRANSCLUSION_OUT: &str = "    \u{2502}---> ";
const INDENT_CHILD: &str = "    \u{2502}   ";

// ── Public rendering entry points ───────────────────────────────────

/// Render the file tree model with terminal-aware capabilities.
pub fn render_model(model: &FileTreeModel, term: &Terminal, show_root: bool) -> String {
    let is_nerd_font = term.is_nerd_font == Some(true);
    let is_tty = term.is_tty;
    let width = term.width() as usize;

    if !show_root {
        return String::new();
    }

    let mut lines = Vec::new();
    render_node(&model.root, &mut lines, "", is_nerd_font, is_tty, width);
    lines.join("\n")
}

/// Render the file tree model without terminal context (Unicode fallback).
pub fn render_model_optimistic(model: &FileTreeModel, width: usize, show_root: bool) -> String {
    if !show_root {
        return String::new();
    }

    let mut lines = Vec::new();
    render_node(&model.root, &mut lines, "", false, false, width);
    lines.join("\n")
}

// ── Node rendering ──────────────────────────────────────────────────

fn render_node(
    node: &FileTreeNode,
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
) {
    // Zone 1: Reference groups above the file line
    render_reference_groups(&node.reference_groups, lines, indent, is_nerd_font, is_tty, width);

    // Zone 2: File head line
    render_file_head(node, lines, indent, is_nerd_font, is_tty);

    // Zone 3: Transclusion edges below
    render_transclusion_edges(
        &node.transclusions,
        lines,
        indent,
        is_nerd_font,
        is_tty,
        width,
    );

    // Follow-mode children
    for (i, child) in node.children.iter().enumerate() {
        // Add a blank connector line before each child
        lines.push(format!("{indent}{CONNECTOR_VERTICAL}"));

        let child_indent = format!("{indent}{INDENT_CHILD}");
        render_node(child, lines, &child_indent, is_nerd_font, is_tty, width);

        // Trailing connector after non-last children
        if i < node.children.len() - 1 {
            lines.push(format!("{indent}{CONNECTOR_VERTICAL}"));
        }
    }
}

fn render_reference_groups(
    groups: &[FileTreeReferenceGroup],
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
) {
    let non_empty: Vec<&FileTreeReferenceGroup> =
        groups.iter().filter(|g| !g.rows.is_empty()).collect();

    for (gi, group) in non_empty.iter().enumerate() {
        let icon = icons::reference_icon(&group.kind, is_nerd_font);

        for row in &group.rows {
            let suffix = row
                .validation
                .as_ref()
                .and_then(|v| v.suffix.as_deref())
                .map(|s| format!(" {s}"))
                .unwrap_or_default();

            let content = format!("{icon}{}{suffix}", row.display_target);
            let prefix = format!("{indent}{CONNECTOR_REF_PREFIX}");

            let line = if is_tty {
                let style = validation_style(&row.validation, is_tty);
                let reset = if style.is_empty() { "" } else { "\x1b[0m" };
                format!("{prefix}{style}{content}{reset}")
            } else {
                format!("{prefix}{content}")
            };

            lines.push(truncate_line(&line, width));
        }

        // Blank line separator between non-empty groups (not after last)
        if gi < non_empty.len() - 1 {
            lines.push(format!("{indent}{CONNECTOR_VERTICAL}"));
        }
    }

    // Separator between reference groups and file head if there were any groups
    if !non_empty.is_empty() {
        lines.push(format!("{indent}{CONNECTOR_VERTICAL}"));
    }
}

fn render_file_head(
    node: &FileTreeNode,
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
) {
    let icon = icons::file_icon(&node.file_icon_kind, is_nerd_font);
    let summary = node
        .inline_summary
        .to_display_string()
        .map(|s| format!(" {s}"))
        .unwrap_or_default();

    let label = &node.file_label;

    if is_tty {
        let dim = if summary.is_empty() { "" } else { "\x1b[2m" };
        let dim_reset = if summary.is_empty() { "" } else { "\x1b[0m" };
        lines.push(format!(
            "{indent}{icon}\x1b[1m{label}\x1b[0m{dim}{summary}{dim_reset}"
        ));
    } else {
        lines.push(format!("{indent}{icon}{label}{summary}"));
    }
}

fn render_transclusion_edges(
    transclusions: &[super::model::FileTreeTransclusionEdge],
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
) {
    if transclusions.is_empty() {
        return;
    }

    // Separator between file head and transclusions
    lines.push(format!("{indent}{CONNECTOR_VERTICAL}"));

    for edge in transclusions {
        let icon = icons::transclusion_icon(&edge.kind, is_nerd_font);
        let connector = match edge.kind {
            FileTreeTransclusionKind::TocLinking => CONNECTOR_TRANSCLUSION_OUT,
            _ => CONNECTOR_TRANSCLUSION_IN,
        };

        let caption = if edge.caption.is_empty() {
            String::new()
        } else {
            format!(" {}", edge.caption)
        };

        let suffix = edge
            .validation
            .as_ref()
            .and_then(|v| v.suffix.as_deref())
            .map(|s| format!(" {s}"))
            .unwrap_or_default();

        let content = format!("{icon}{}{caption}{suffix}", edge.display_target);
        let prefix = format!("{indent}{connector}");

        let line = if is_tty {
            let style = validation_style(&edge.validation, is_tty);
            let reset = if style.is_empty() { "" } else { "\x1b[0m" };
            format!("{prefix}{style}{content}{reset}")
        } else {
            format!("{prefix}{content}")
        };

        lines.push(truncate_line(&line, width));
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Get ANSI style prefix for validation status.
fn validation_style(
    validation: &Option<super::model::FileTreeReferenceValidation>,
    is_tty: bool,
) -> &'static str {
    if !is_tty {
        return "";
    }
    match validation {
        Some(v) if !v.is_valid => match v.severity {
            ReferenceSeverity::Error => "\x1b[31m",   // red
            ReferenceSeverity::Warning => "\x1b[33m", // yellow
            ReferenceSeverity::Info => "\x1b[36m",    // cyan
        },
        _ => "",
    }
}

/// Truncate a line to fit within the given width.
fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 || line.len() <= width {
        return line.to_string();
    }
    // Simple truncation — does not account for ANSI escape sequences or
    // multi-byte characters perfectly, but avoids mangling connectors by
    // only truncating content at the end.
    let mut truncated = line[..width.saturating_sub(1)].to_string();
    truncated.push('\u{2026}'); // …
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeSource;
    use crate::markdown::reference::file_tree::model::*;

    fn simple_model() -> FileTreeModel {
        FileTreeModel {
            root: FileTreeNode {
                source: ComposeSource::File(std::path::PathBuf::from("/docs/test.md")),
                file_label: "test.md".to_string(),
                file_icon_kind: FileTreeIconKind::Markdown,
                inline_summary: FileTreeInlineSummary::default(),
                reference_groups: vec![],
                transclusions: vec![],
                children: vec![],
                validation: FileTreeNodeValidation::default(),
            },
        }
    }

    #[test]
    fn render_single_file_no_references() {
        let model = simple_model();
        let output = render_model_optimistic(&model, 80, true);
        assert!(output.contains("test.md"));
        assert!(output.contains('\u{1F4C4}')); // 📄
    }

    #[test]
    fn render_with_references_above() {
        let mut model = simple_model();
        model.root.reference_groups = vec![
            FileTreeReferenceGroup {
                kind: FileTreeReferenceGroupKind::RemoteHyperlinks,
                rows: vec![FileTreeReferenceRow {
                    kind: crate::markdown::reference::types::ReferenceKind::Hyperlink,
                    display_target: "https://example.com".to_string(),
                    raw_reference_id: "ref1".to_string(),
                    validation: None,
                }],
            },
            FileTreeReferenceGroup {
                kind: FileTreeReferenceGroupKind::Images,
                rows: vec![FileTreeReferenceRow {
                    kind: crate::markdown::reference::types::ReferenceKind::Image,
                    display_target: "./logo.png".to_string(),
                    raw_reference_id: "ref2".to_string(),
                    validation: None,
                }],
            },
        ];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();

        // Should have reference rows, separator, and file head
        assert!(output.contains("https://example.com"));
        assert!(output.contains("./logo.png"));
        assert!(output.contains("test.md"));

        // References should come before the file head
        let link_line = lines.iter().position(|l| l.contains("example.com")).unwrap();
        let file_line = lines.iter().position(|l| l.contains("test.md")).unwrap();
        assert!(link_line < file_line);
    }

    #[test]
    fn render_with_transclusions_below() {
        let mut model = simple_model();
        model.root.transclusions = vec![FileTreeTransclusionEdge {
            kind: FileTreeTransclusionKind::File,
            display_target: "@docs/child.md".to_string(),
            caption: "inserted into the '## Details' section".to_string(),
            directive_line: 10,
            followable: true,
            child_node_id: Some("child_1".to_string()),
            validation: None,
        }];

        let output = render_model_optimistic(&model, 120, true);
        assert!(output.contains("@docs/child.md"));
        assert!(output.contains("inserted into the '## Details' section"));

        let lines: Vec<&str> = output.lines().collect();
        let file_line = lines.iter().position(|l| l.contains("test.md")).unwrap();
        let trans_line = lines
            .iter()
            .position(|l| l.contains("@docs/child.md"))
            .unwrap();
        assert!(trans_line > file_line);
    }

    #[test]
    fn render_blank_line_between_groups() {
        let mut model = simple_model();
        model.root.reference_groups = vec![
            FileTreeReferenceGroup {
                kind: FileTreeReferenceGroupKind::RemoteHyperlinks,
                rows: vec![FileTreeReferenceRow {
                    kind: crate::markdown::reference::types::ReferenceKind::Hyperlink,
                    display_target: "https://a.com".to_string(),
                    raw_reference_id: "r1".to_string(),
                    validation: None,
                }],
            },
            FileTreeReferenceGroup {
                kind: FileTreeReferenceGroupKind::Images,
                rows: vec![FileTreeReferenceRow {
                    kind: crate::markdown::reference::types::ReferenceKind::Image,
                    display_target: "./img.png".to_string(),
                    raw_reference_id: "r2".to_string(),
                    validation: None,
                }],
            },
        ];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();

        // Find the blank connector line between the two groups
        let link_idx = lines.iter().position(|l| l.contains("a.com")).unwrap();
        let img_idx = lines.iter().position(|l| l.contains("img.png")).unwrap();
        // There should be a connector-only line between them
        assert!(img_idx - link_idx >= 2, "expected blank line between groups");
    }

    #[test]
    fn render_validation_suffix() {
        let mut model = simple_model();
        model.root.reference_groups = vec![FileTreeReferenceGroup {
            kind: FileTreeReferenceGroupKind::LocalHyperlinks,
            rows: vec![FileTreeReferenceRow {
                kind: crate::markdown::reference::types::ReferenceKind::Hyperlink,
                display_target: "./missing.md".to_string(),
                raw_reference_id: "r1".to_string(),
                validation: Some(FileTreeReferenceValidation {
                    is_valid: false,
                    suffix: Some("[missing]".to_string()),
                    severity: crate::markdown::reference::validate::ReferenceSeverity::Error,
                }),
            }],
        }];

        let output = render_model_optimistic(&model, 120, true);
        assert!(output.contains("[missing]"));
    }

    #[test]
    fn render_show_root_false() {
        let model = simple_model();
        let output = render_model_optimistic(&model, 80, false);
        assert!(output.is_empty());
    }
}
