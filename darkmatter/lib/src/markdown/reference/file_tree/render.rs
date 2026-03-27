//! Terminal rendering for the FileTree component.
//!
//! Implements a three-zone layout per node: reference groups above the file
//! head line, the file head itself, and transclusion edges below.  In follow
//! mode, followed edges merge with their child nodes — the edge arrow becomes
//! the child's header and the child's content renders indented below it.
//!
//! ## Line Art
//!
//! Curved corners (`╭`/`╰`) cap the first and last items of each section,
//! preventing unterminated vertical lines.  Between same-kind edges (without
//! child content) no separator is emitted; a `│` blank line appears only on
//! kind changes or after followed children.

use biscuit_terminal::terminal::Terminal;

use crate::markdown::reference::validate::ReferenceSeverity;
use super::icons;
use super::model::{
    FileTreeModel, FileTreeNode, FileTreeReferenceGroup, FileTreeTransclusionEdge,
    FileTreeTransclusionKind,
};

// ── Connector strings ───────────────────────────────────────────────
//
// ╭ only where nothing continues above (first ref row).
// ╰ only where nothing continues below (last transclusion edge).
// ├ everywhere else (vertical continues in both directions).
//
//  ╭── 🔗   first reference row   (nothing above)
//  ├── 🔗   subsequent ref rows   (vertical continues to file head)
//  │        vertical continuation
//  ├◂─      transclusion edge     (vertical from file head above)
//  ╰◂─      last transclusion     (terminates branch)
//  ╰─▸      last toc-linking      (terminates branch)

// ── Vertical ────────────────────────────────────────────────────────
const VERT: &str       = "    \u{2502}";

// ── Reference row branches ──────────────────────────────────────────
const REF_FIRST: &str  = "    \u{256D}\u{2500}\u{2500} ";  // ╭── (nothing above)
const REF_MID: &str    = "    \u{251C}\u{2500}\u{2500} ";  // ├── (continues)

// ── Transclusion edge branches ──────────────────────────────────────
const TRANS_IN: &str       = "    \u{2502}\u{25C0}\u{2500} ";  // │◀─
const TRANS_IN_LAST: &str  = "    \u{2570}\u{25C0}\u{2500} ";  // ╰◀─
const TRANS_OUT: &str      = "    \u{251C}\u{2500}\u{25B6} ";  // ├─▶
const TRANS_OUT_LAST: &str = "    \u{2570}\u{2500}\u{25B6} ";  // ╰─▶

// ── Child indents ───────────────────────────────────────────────────
const INDENT_CHILD: &str      = "    \u{2502}   ";
const INDENT_CHILD_LAST: &str = "        ";

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
    render_reference_groups_inner(groups, lines, indent, is_nerd_font, is_tty, width, true);
}

fn render_reference_groups_inner(
    groups: &[FileTreeReferenceGroup],
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
    trailing_separator: bool,
) {
    // Flatten all rows with their group context for position tracking.
    let non_empty: Vec<&FileTreeReferenceGroup> =
        groups.iter().filter(|g| !g.rows.is_empty()).collect();

    let total_rows: usize = non_empty.iter().map(|g| g.rows.len()).sum();
    if total_rows == 0 {
        return;
    }

    let mut row_idx = 0;
    for (gi, group) in non_empty.iter().enumerate() {
        let icon = icons::reference_icon(&group.kind, is_nerd_font);

        for row in &group.rows {
            let is_first = row_idx == 0;
            let connector = if is_first { REF_FIRST } else { REF_MID };

            let suffix = row
                .validation
                .as_ref()
                .and_then(|v| v.suffix.as_deref())
                .map(|s| format!(" {s}"))
                .unwrap_or_default();

            let content = format!("{icon}{}{suffix}", row.display_target);
            let prefix = format!("{indent}{connector}");

            let line = if is_tty {
                let style = validation_style(&row.validation, is_tty);
                let reset = if style.is_empty() { "" } else { "\x1b[0m" };
                format!("{prefix}{style}{content}{reset}")
            } else {
                format!("{prefix}{content}")
            };

            lines.push(truncate_line(&line, width));
            row_idx += 1;
        }

        // Blank line separator between non-empty groups (not after last)
        if gi < non_empty.len() - 1 {
            lines.push(format!("{indent}{VERT}"));
        }
    }

    // Separator between reference groups and file head (only when a file head follows)
    if trailing_separator {
        lines.push(format!("{indent}{VERT}"));
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
/// Uses curved connectors (`╭`/`╰`) for first/last edges.  Between same-kind
/// edges without child content, no separator is emitted.  A `│` blank line
/// appears only when the transclusion kind changes or after followed children.
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

    // │ connects the file head to the first edge below
    lines.push(format!("{indent}{VERT}"));

    let edge_count = node.transclusions.len();
    let mut child_idx = 0;

    for (ei, edge) in node.transclusions.iter().enumerate() {
        let is_last = ei == edge_count - 1;

        // Separator: always insert │ on kind change.
        // After followed children, skip separator for same-kind edges
        // (child content already provides visual separation).
        if ei > 0 {
            let kind_changed = edge.kind != node.transclusions[ei - 1].kind;
            if kind_changed {
                lines.push(format!("{indent}{VERT}"));
            }
        }

        // Pick position-aware connector
        let connector = pick_edge_connector(edge.kind, is_last);

        // Render the edge arrow line
        render_single_edge(edge, connector, lines, indent, is_nerd_font, is_tty, width);

        // If this edge was followed, render the child's content below it
        let is_followed = edge.followable
            && edge.child_node_id.is_some()
            && child_idx < node.children.len();

        if is_followed {
            let child = &node.children[child_idx];
            child_idx += 1;

            // After ╰ the vertical is terminated; use blank indent
            let child_indent_str = if is_last { INDENT_CHILD_LAST } else { INDENT_CHILD };
            let child_indent = format!("{indent}{child_indent_str}");

            render_reference_groups(
                &child.reference_groups,
                lines,
                &child_indent,
                is_nerd_font,
                is_tty,
                width,
            );

            render_transclusions_unified(child, lines, &child_indent, is_nerd_font, is_tty, width);
        }
    }
}

/// Pick the correct connector string based on edge kind and position.
///
/// Only the last edge uses `╰` (terminates branch). All others use `├`
/// because the vertical connects up to the file head.
fn pick_edge_connector(kind: FileTreeTransclusionKind, is_last: bool) -> &'static str {
    let is_out = kind == FileTreeTransclusionKind::TocLinking;
    match (is_out, is_last) {
        (true, true)   => TRANS_OUT_LAST,
        (true, false)  => TRANS_OUT,
        (false, true)  => TRANS_IN_LAST,
        (false, false) => TRANS_IN,
    }
}

/// Render a single transclusion edge line.
fn render_single_edge(
    edge: &FileTreeTransclusionEdge,
    connector: &str,
    lines: &mut Vec<String>,
    indent: &str,
    is_nerd_font: bool,
    is_tty: bool,
    width: usize,
) {
    let icon = icons::transclusion_icon(&edge.kind, is_nerd_font);
    let is_fm = matches!(
        edge.kind,
        FileTreeTransclusionKind::Prologue | FileTreeTransclusionKind::Epilogue
    );
    let is_literal_fm = is_fm && edge.display_target.is_empty();

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
            let caption = if edge.caption.is_empty() {
                String::new()
            } else {
                format!(" {}", edge.caption)
            };
            let content = format!("{icon}{}{caption}{suffix}", edge.display_target);
            format!("{prefix}{val_style}{content}\x1b[0m")
        } else if is_fm {
            // Prologue/epilogue: inverse label + dim/italic caption
            let label = match edge.kind {
                FileTreeTransclusionKind::Prologue => "prologue",
                _ => "epilogue",
            };
            let caption = if is_literal_fm {
                format!(" \x1b[2;3m{}\x1b[0m", edge.caption)
            } else {
                format!(
                    " \x1b[2;3mreferences \x1b[0m\x1b[38;5;75m{}\x1b[0m",
                    edge.display_target,
                )
            };
            format!("{prefix}\x1b[7m {label} \x1b[0m{caption}{suffix}")
        } else {
            let styled_caption = style_transclusion_caption(&edge.caption);
            format!(
                "{prefix}{icon}\x1b[38;5;75m{}\x1b[0m{styled_caption}{suffix}",
                edge.display_target,
            )
        }
    } else if is_fm {
        let label = match edge.kind {
            FileTreeTransclusionKind::Prologue => "prologue",
            _ => "epilogue",
        };
        if is_literal_fm {
            format!("{prefix}[{label}] {}{suffix}", edge.caption)
        } else {
            format!("{prefix}[{label}] references {}{suffix}", edge.display_target)
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
    const DIM_ITALIC: &str = "\x1b[2;3m";
    const RESET: &str = "\x1b[0m";

    if let Some(start) = caption.find('\'')
        && let Some(end) = caption[start + 1..].find('\'')
    {
        let before = &caption[..start];
        let section = &caption[start..start + 1 + end + 1];
        let after = &caption[start + 1 + end + 1..];
        return format!(" {DIM_ITALIC}{before}{RESET}{section}{DIM_ITALIC}{after}{RESET}");
    }

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
            ReferenceSeverity::Error => "\x1b[31m",
            ReferenceSeverity::Warning => "\x1b[33m",
            ReferenceSeverity::Info => "\x1b[36m",
        },
        _ => "",
    }
}

/// Truncate a line to fit within the given visible display width.
fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 {
        return line.to_string();
    }
    let visible = biscuit_terminal::utils::block_constraint::visible_width(line) as usize;
    if visible <= width {
        return line.to_string();
    }
    let (head, _) = biscuit_terminal::utils::block_constraint::split_at_visible_width(
        line,
        width.saturating_sub(1) as u32,
    );
    format!("{head}\u{2026}")
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

        assert!(output.contains("https://example.com"));
        assert!(output.contains("./logo.png"));
        assert!(output.contains("test.md"));

        let link_line = lines.iter().position(|l| l.contains("example.com")).unwrap();
        let file_line = lines.iter().position(|l| l.contains("test.md")).unwrap();
        assert!(link_line < file_line);

        // First ref row should use ╭
        assert!(lines[link_line].contains('\u{256D}'), "first ref should use ╭");
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

        // Single transclusion should use ╰ (last/only)
        assert!(lines[trans_line].contains('\u{25C0}'), "incoming edge should have ◀");
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

        let link_idx = lines.iter().position(|l| l.contains("a.com")).unwrap();
        let img_idx = lines.iter().position(|l| l.contains("img.png")).unwrap();
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
    fn same_kind_edges_no_separator() {
        let mut model = simple_model();
        model.root.transclusions = vec![
            FileTreeTransclusionEdge {
                kind: FileTreeTransclusionKind::File,
                display_target: "./a.md".to_string(),
                caption: String::new(),
                directive_line: 1,
                followable: true,
                child_node_id: None,
                validation: None,
            },
            FileTreeTransclusionEdge {
                kind: FileTreeTransclusionKind::File,
                display_target: "./b.md".to_string(),
                caption: String::new(),
                directive_line: 2,
                followable: true,
                child_node_id: None,
                validation: None,
            },
        ];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();
        let a_idx = lines.iter().position(|l| l.contains("./a.md")).unwrap();
        let b_idx = lines.iter().position(|l| l.contains("./b.md")).unwrap();
        // Same-kind edges should be adjacent (no blank │ between them)
        assert_eq!(b_idx - a_idx, 1, "same-kind edges should be adjacent");
    }

    #[test]
    fn kind_change_inserts_separator() {
        let mut model = simple_model();
        model.root.transclusions = vec![
            FileTreeTransclusionEdge {
                kind: FileTreeTransclusionKind::File,
                display_target: "./a.md".to_string(),
                caption: String::new(),
                directive_line: 1,
                followable: true,
                child_node_id: None,
                validation: None,
            },
            FileTreeTransclusionEdge {
                kind: FileTreeTransclusionKind::TocLinking,
                display_target: "./b.md".to_string(),
                caption: String::new(),
                directive_line: 2,
                followable: true,
                child_node_id: None,
                validation: None,
            },
        ];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();
        let a_idx = lines.iter().position(|l| l.contains("./a.md")).unwrap();
        let b_idx = lines.iter().position(|l| l.contains("./b.md")).unwrap();
        // Kind change should insert a separator
        assert!(b_idx - a_idx >= 2, "kind change should have separator");
    }

    #[test]
    fn first_last_curved_connectors() {
        let mut model = simple_model();
        model.root.transclusions = vec![
            FileTreeTransclusionEdge {
                kind: FileTreeTransclusionKind::File,
                display_target: "./a.md".to_string(),
                caption: String::new(),
                directive_line: 1,
                followable: true,
                child_node_id: None,
                validation: None,
            },
            FileTreeTransclusionEdge {
                kind: FileTreeTransclusionKind::File,
                display_target: "./b.md".to_string(),
                caption: String::new(),
                directive_line: 2,
                followable: true,
                child_node_id: None,
                validation: None,
            },
            FileTreeTransclusionEdge {
                kind: FileTreeTransclusionKind::File,
                display_target: "./c.md".to_string(),
                caption: String::new(),
                directive_line: 3,
                followable: true,
                child_node_id: None,
                validation: None,
            },
        ];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();
        let a_line = lines.iter().find(|l| l.contains("./a.md")).unwrap();
        let b_line = lines.iter().find(|l| l.contains("./b.md")).unwrap();
        let c_line = lines.iter().find(|l| l.contains("./c.md")).unwrap();

        // All incoming File edges use │◀─
        assert!(a_line.contains('\u{25C0}'), "first edge should have ◀");
        assert!(b_line.contains('\u{25C0}'), "middle edge should have ◀");
        assert!(c_line.contains('\u{25C0}'), "last edge should have ◀");
    }

    #[test]
    fn truncate_line_respects_ansi_escapes() {
        let line = "\x1b[31mRed text\x1b[0m";
        let result = truncate_line(line, 20);
        assert!(result.contains("Red text"));
        assert!(!result.contains('\u{2026}'));
    }

    #[test]
    fn truncate_line_truncates_visible_content() {
        let line = "This is a fairly long line of text";
        let result = truncate_line(line, 10);
        assert!(result.contains('\u{2026}'), "should have ellipsis");
        let visible = biscuit_terminal::utils::block_constraint::visible_width(&result) as usize;
        assert!(visible <= 10, "visible width {visible} should be <= 10");
    }

    #[test]
    fn truncate_line_zero_width_passthrough() {
        assert_eq!(truncate_line("hello", 0), "hello");
    }

    #[test]
    fn style_caption_with_section() {
        let caption = "inserted into the '## Details' section";
        let styled = style_transclusion_caption(caption);
        assert!(styled.contains("\x1b[2;3m"));
        assert!(styled.contains("'## Details'"));
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
        assert!(output.contains("\x1b[38;5;75m./child.md"), "target should be blue");
        assert!(output.contains("\x1b[2;3m"), "caption should be dim+italic");
    }

    #[test]
    fn follow_mode_does_not_double_render() {
        let mut model = simple_model();
        model.root.transclusions = vec![FileTreeTransclusionEdge {
            kind: FileTreeTransclusionKind::File,
            display_target: "./child.md".to_string(),
            caption: "inserted".to_string(),
            directive_line: 10,
            followable: true,
            child_node_id: Some("child_1".to_string()),
            validation: None,
        }];
        model.root.children = vec![child_node("child.md")];

        let output = render_model_optimistic(&model, 120, true);
        let lines: Vec<&str> = output.lines().collect();

        let edge_count = lines.iter().filter(|l| l.contains("./child.md")).count();
        assert_eq!(edge_count, 1, "child.md should appear once, not twice");
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

        let edge_idx = lines.iter().position(|l| l.contains("./child.md")).unwrap();
        let ref_idx = lines.iter().position(|l| l.contains("example.com")).unwrap();
        assert!(ref_idx > edge_idx, "child refs should render below the edge");
    }
}
