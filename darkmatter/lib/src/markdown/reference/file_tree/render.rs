//! Terminal rendering for the FileTree component.
//!
//! Implements a three-zone layout per node: reference groups above the file
//! head line, the file head itself, and transclusion edges below.  In follow
//! mode, followed edges merge with their child nodes — the edge arrow becomes
//! the child's header and the child's content renders indented below it.

use biscuit_terminal::terminal::Terminal;

use crate::markdown::reference::validate::ReferenceSeverity;
use super::icons;
use super::model::{
    FileTreeModel, FileTreeNode, FileTreeReferenceGroup, FileTreeTransclusionEdge,
    FileTreeTransclusionKind,
};

// ── Connector strings ───────────────────────────────────────────────
//
//  │        CONNECTOR_VERTICAL      (continuation)
//  ├── 🔗   CONNECTOR_REF_PREFIX    (reference row branch)
//  ├◂─      CONNECTOR_TRANSCLUSION_IN  (incoming transclusion)
//  ├─▸      CONNECTOR_TRANSCLUSION_OUT (outgoing toc-linking)
//  │        INDENT_CHILD            (child content indent)

const CONNECTOR_VERTICAL: &str = "    \u{2502}";
const CONNECTOR_REF_PREFIX: &str = "    \u{251C}\u{2500}\u{2500} ";
const CONNECTOR_TRANSCLUSION_IN: &str = "    \u{251C}\u{25C2}\u{2500} ";
const CONNECTOR_TRANSCLUSION_OUT: &str = "    \u{251C}\u{2500}\u{25B8} ";
const INDENT_CHILD: &str = "    \u{2502}   ";

// ── Public rendering entry points ───────────────────────────────────

/// Render the file tree model with terminal-aware capabilities.
pub fn render_model(model: &FileTreeModel, term: &Terminal, show_root: bool) -> String {
    let is_nerd_font = term.is_nerd_font == Some(true);
    let is_tty = term.is_tty;
    let width = term.width() as usize;

    let mut lines = Vec::new();
    if show_root {
        render_node(&model.root, &mut lines, "", is_nerd_font, is_tty, width);
    } else {
        render_node_children_only(&model.root, &mut lines, "", is_nerd_font, is_tty, width);
    }
    lines.join("\n")
}

/// Render the file tree model without terminal context (Unicode fallback).
pub fn render_model_optimistic(model: &FileTreeModel, width: usize, show_root: bool) -> String {
    let mut lines = Vec::new();
    if show_root {
        render_node(&model.root, &mut lines, "", false, false, width);
    } else {
        render_node_children_only(&model.root, &mut lines, "", false, false, width);
    }
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

    // Zone 3+4: Transclusion edges unified with follow-mode children
    render_transclusions_unified(node, lines, indent, is_nerd_font, is_tty, width);
}

/// Render the root node's content (reference groups, transclusions, children)
/// without the root file head line. Used when `show_root(false)` is set.
fn render_node_children_only(
    node: &FileTreeNode,
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
) {
    // Zone 1: Reference groups
    render_reference_groups(&node.reference_groups, lines, indent, is_nerd_font, is_tty, width);

    // Zone 3+4: Transclusion edges unified with follow-mode children
    render_transclusions_unified(node, lines, indent, is_nerd_font, is_tty, width);
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

/// Render transclusion edges and follow-mode children in a unified pass.
///
/// For edges without a followed child, renders the edge line only.
/// For edges with a followed child, the edge arrow line becomes the child's
/// header and the child's content (reference groups + sub-transclusions)
/// renders indented below — eliminating the double-rendering problem.
fn render_transclusions_unified(
    node: &FileTreeNode,
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
) {
    if node.transclusions.is_empty() {
        return;
    }

    // Separator between file head and transclusions
    lines.push(format!("{indent}{CONNECTOR_VERTICAL}"));

    // Children are pushed in the same order as their corresponding followed
    // edges in the model builder, so we consume them sequentially.
    let mut child_idx = 0;
    let edge_count = node.transclusions.len();

    for (ei, edge) in node.transclusions.iter().enumerate() {
        // Render the edge arrow line
        render_single_edge(edge, lines, indent, is_nerd_font, is_tty, width);

        // If this edge was followed, render the child's content below it
        let is_followed = edge.followable
            && edge.child_node_id.is_some()
            && child_idx < node.children.len();

        if is_followed {
            let child = &node.children[child_idx];
            child_idx += 1;

            let child_indent = format!("{indent}{INDENT_CHILD}");

            // Render the child's references (Zone 1 for the child)
            render_reference_groups(
                &child.reference_groups,
                lines,
                &child_indent,
                is_nerd_font,
                is_tty,
                width,
            );

            // Render the child's transclusions + grandchildren (recursive)
            render_transclusions_unified(child, lines, &child_indent, is_nerd_font, is_tty, width);
        }

        // Blank connector line between edges for visual separation
        if ei < edge_count - 1 {
            lines.push(format!("{indent}{CONNECTOR_VERTICAL}"));
        }
    }
}

/// Render a single transclusion edge line.
fn render_single_edge(
    edge: &FileTreeTransclusionEdge,
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
) {
    let icon = icons::transclusion_icon(&edge.kind, is_nerd_font);
    let connector = match edge.kind {
        FileTreeTransclusionKind::TocLinking => CONNECTOR_TRANSCLUSION_OUT,
        _ => CONNECTOR_TRANSCLUSION_IN,
    };

    let suffix = edge
        .validation
        .as_ref()
        .and_then(|v| v.suffix.as_deref())
        .map(|s| format!(" {s}"))
        .unwrap_or_default();

    let prefix = format!("{indent}{connector}");

    let line = if is_tty {
        let val_style = validation_style(&edge.validation, is_tty);
        if !val_style.is_empty() {
            // Validation error: entire line in error color
            let caption = if edge.caption.is_empty() {
                String::new()
            } else {
                format!(" {}", edge.caption)
            };
            let content = format!("{icon}{}{caption}{suffix}", edge.display_target);
            format!("{prefix}{val_style}{content}\x1b[0m")
        } else {
            // Normal: blue filename, dim+italic caption with normal section name
            let styled_caption = style_transclusion_caption(&edge.caption);
            format!(
                "{prefix}{icon}\x1b[38;5;75m{}\x1b[0m{styled_caption}{suffix}",
                edge.display_target,
            )
        }
    } else {
        let caption = if edge.caption.is_empty() {
            String::new()
        } else {
            format!(" {}", edge.caption)
        };
        format!("{prefix}{icon}{}{caption}{suffix}", edge.display_target)
    };

    lines.push(truncate_line(&line, width));
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Style a transclusion caption for TTY output.
///
/// Renders the caption text as dim+italic, with the section heading name
/// (delimited by single quotes) in normal weight.
fn style_transclusion_caption(caption: &str) -> String {
    if caption.is_empty() {
        return String::new();
    }
    // dim+italic = \x1b[2;3m, reset = \x1b[0m
    const DIM_ITALIC: &str = "\x1b[2;3m";
    const RESET: &str = "\x1b[0m";

    // Caption format: "inserted into the '## Heading' section"
    // Split on single-quote delimiters to style the section heading normally.
    if let Some(start) = caption.find('\'') {
        if let Some(end) = caption[start + 1..].find('\'') {
            let before = &caption[..start];
            let section = &caption[start..start + 1 + end + 1]; // includes quotes
            let after = &caption[start + 1 + end + 1..];
            return format!(" {DIM_ITALIC}{before}{RESET}{section}{DIM_ITALIC}{after}{RESET}");
        }
    }

    // No section delimiters — style the whole caption dim+italic
    format!(" {DIM_ITALIC}{caption}{RESET}")
}

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

/// Truncate a line to fit within the given visible display width.
///
/// Uses ANSI-aware and Unicode-aware width measurement so that escape
/// sequences do not count toward the column budget and multi-byte
/// characters are measured by display width rather than byte length.
fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 {
        return line.to_string();
    }
    let visible = biscuit_terminal::utils::block_constraint::visible_width(line) as usize;
    if visible <= width {
        return line.to_string();
    }
    // split_at_visible_width respects ANSI escapes and multi-byte chars.
    let (head, _) = biscuit_terminal::utils::block_constraint::split_at_visible_width(
        line,
        width.saturating_sub(1) as u32,
    );
    format!("{head}\u{2026}") // …
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

    fn child_node(label: &str) -> FileTreeNode {
        FileTreeNode {
            source: ComposeSource::File(std::path::PathBuf::from(format!("/docs/{label}"))),
            file_label: label.to_string(),
            file_icon_kind: FileTreeIconKind::Markdown,
            inline_summary: FileTreeInlineSummary::default(),
            reference_groups: vec![],
            transclusions: vec![],
            children: vec![],
            validation: FileTreeNodeValidation::default(),
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
    fn render_show_root_false_empty_model() {
        let model = simple_model();
        let output = render_model_optimistic(&model, 80, false);
        // A model with no refs/transclusions/children renders empty even without root
        assert!(output.is_empty());
    }

    #[test]
    fn render_show_root_false_preserves_children() {
        let mut model = simple_model();
        model.root.transclusions = vec![FileTreeTransclusionEdge {
            kind: FileTreeTransclusionKind::File,
            display_target: "child.md".to_string(),
            caption: String::new(),
            directive_line: 5,
            followable: true,
            child_node_id: Some("child_1".to_string()),
            validation: None,
        }];
        model.root.children = vec![child_node("child.md")];

        let output = render_model_optimistic(&model, 120, false);
        assert!(!output.contains("test.md"), "root label should be hidden");
        assert!(output.contains("child.md"), "child should still render via edge");
    }

    #[test]
    fn render_show_root_false_preserves_transclusions() {
        let mut model = simple_model();
        model.root.transclusions = vec![FileTreeTransclusionEdge {
            kind: FileTreeTransclusionKind::File,
            display_target: "child.md".to_string(),
            caption: String::new(),
            directive_line: 5,
            followable: true,
            child_node_id: None,
            validation: None,
        }];

        let output = render_model_optimistic(&model, 120, false);
        assert!(!output.contains("test.md"), "root label should be hidden");
        assert!(output.contains("child.md"), "transclusions should still render");
    }

    #[test]
    fn truncate_line_respects_ansi_escapes() {
        // A line with ANSI color codes: visible content is "Red text"
        let line = "\x1b[31mRed text\x1b[0m";
        // Visible width is 8 chars ("Red text"), ANSI codes are invisible
        let result = truncate_line(line, 20);
        // Should not truncate because visible width (8) < 20
        assert!(result.contains("Red text"));
        assert!(!result.contains('\u{2026}'));
    }

    #[test]
    fn truncate_line_truncates_visible_content() {
        // No ANSI escapes, pure ASCII
        let line = "This is a fairly long line of text";
        let result = truncate_line(line, 10);
        assert!(result.contains('\u{2026}'), "should have ellipsis");
        // Visible result should be at most 10 columns
        let visible = biscuit_terminal::utils::block_constraint::visible_width(&result) as usize;
        assert!(visible <= 10, "visible width {visible} should be <= 10");
    }

    #[test]
    fn truncate_line_zero_width_passthrough() {
        let line = "hello";
        assert_eq!(truncate_line(line, 0), "hello");
    }

    #[test]
    fn style_caption_with_section() {
        let caption = "inserted into the '## Details' section";
        let styled = style_transclusion_caption(caption);
        // Should contain dim+italic for text and reset around section name
        assert!(styled.contains("\x1b[2;3m"), "should have dim+italic");
        assert!(styled.contains("\x1b[0m"), "should have reset");
        assert!(styled.contains("'## Details'"), "section name preserved");
    }

    #[test]
    fn style_caption_no_section() {
        let caption = "inserted";
        let styled = style_transclusion_caption(caption);
        assert!(styled.contains("\x1b[2;3m"), "should have dim+italic");
        assert!(styled.contains("inserted"), "text preserved");
    }

    #[test]
    fn style_caption_empty() {
        assert_eq!(style_transclusion_caption(""), "");
    }

    #[test]
    fn render_tty_transclusion_has_blue_target() {
        let mut model = simple_model();
        model.root.transclusions = vec![FileTreeTransclusionEdge {
            kind: FileTreeTransclusionKind::File,
            display_target: "./child.md".to_string(),
            caption: "inserted into the '## Details' section".to_string(),
            directive_line: 10,
            followable: true,
            child_node_id: None,
            validation: None,
        }];

        let term = biscuit_terminal::terminal::Terminal {
            is_tty: true,
            is_nerd_font: Some(false),
            ..Default::default()
        };
        let output = render_model(&model, &term, true);
        // Blue color for filename: \x1b[38;5;75m
        assert!(output.contains("\x1b[38;5;75m./child.md"), "target should be blue");
        // Dim+italic for caption
        assert!(output.contains("\x1b[2;3m"), "caption should be dim+italic");
    }

    #[test]
    fn follow_mode_does_not_double_render() {
        let mut model = simple_model();
        let child = child_node("child.md");

        model.root.transclusions = vec![FileTreeTransclusionEdge {
            kind: FileTreeTransclusionKind::File,
            display_target: "./child.md".to_string(),
            caption: "inserted into the '## Details' section".to_string(),
            directive_line: 10,
            followable: true,
            child_node_id: Some("child_1".to_string()),
            validation: None,
        }];
        model.root.children = vec![child];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();

        // The edge arrow should appear exactly once (as the child header)
        let edge_count = lines.iter().filter(|l| l.contains("./child.md")).count();
        assert_eq!(edge_count, 1, "child.md should appear once, not twice");

        // The standalone file head "child.md" should NOT appear separately
        // (only the edge arrow "./child.md" should be present via ├◂─)
        let head_count = lines
            .iter()
            .filter(|l| {
                l.contains("child.md") && !l.contains('\u{25C2}') && !l.contains('\u{25B8}')
            })
            .count();
        assert_eq!(head_count, 0, "no standalone child file head in follow mode");
    }

    #[test]
    fn follow_mode_renders_child_refs_under_edge() {
        let mut model = simple_model();
        let mut child = child_node("child.md");
        child.reference_groups = vec![FileTreeReferenceGroup {
            kind: FileTreeReferenceGroupKind::RemoteHyperlinks,
            rows: vec![FileTreeReferenceRow {
                kind: crate::markdown::reference::types::ReferenceKind::Hyperlink,
                display_target: "https://example.com".to_string(),
                raw_reference_id: "r1".to_string(),
                validation: None,
            }],
        }];

        model.root.transclusions = vec![FileTreeTransclusionEdge {
            kind: FileTreeTransclusionKind::File,
            display_target: "./child.md".to_string(),
            caption: String::new(),
            directive_line: 10,
            followable: true,
            child_node_id: Some("child_1".to_string()),
            validation: None,
        }];
        model.root.children = vec![child];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();

        // Child's references should appear after the edge line
        let edge_idx = lines.iter().position(|l| l.contains("./child.md")).unwrap();
        let ref_idx = lines.iter().position(|l| l.contains("example.com")).unwrap();
        assert!(
            ref_idx > edge_idx,
            "child refs should render below the edge"
        );
    }
}
