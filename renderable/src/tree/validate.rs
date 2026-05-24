//! Structural validation of the canonical render tree.
//!
//! [`validate`] walks a [`RenderNode`] tree and reports structural problems
//! as a [`ValidationReport`]. [`ensure_valid`] is a convenience wrapper that
//! turns any error-severity finding into a [`ValidationError`].
//!
//! ## Examples
//!
//! ```
//! use renderable::tree::{ensure_valid, RenderNode};
//!
//! let tree = RenderNode::root(vec![
//!     RenderNode::paragraph(vec![RenderNode::text("ok")]),
//! ]);
//! assert!(ensure_valid(&tree).is_ok());
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::tree::Severity;
use crate::tree::node::{NodeKind, RenderNode};
use crate::tree::source::SourceSpan;

/// How thoroughly [`validate`] walks the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Collect every finding in the tree.
    Full,
    /// Stop walking at the first [`Severity::Error`] finding.
    FailFast,
}

/// A single structural problem found by [`validate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationFinding {
    /// How serious the finding is.
    pub severity: Severity,
    /// A human-readable description.
    pub message: String,
    /// The source location the finding refers to, if known.
    pub span: Option<SourceSpan>,
}

/// The result of validating a render tree.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    /// Every finding raised during the walk.
    pub findings: Vec<ValidationFinding>,
}

impl ValidationReport {
    /// Returns `true` if the report contains no findings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    /// Returns `true` if any finding has [`Severity::Error`].
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }

    /// Returns an iterator over the [`Severity::Error`] findings.
    pub fn errors(&self) -> impl Iterator<Item = &ValidationFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
    }
}

/// An error raised when a tree fails structural validation.
///
/// Carries every error-severity finding from the underlying
/// [`ValidationReport`].
#[derive(Debug, Clone, PartialEq, Error)]
#[error(
    "render tree failed validation with {} error(s): {}",
    .findings.len(),
    format_findings(.findings)
)]
pub struct ValidationError {
    /// The error-severity findings that caused the failure.
    pub findings: Vec<ValidationFinding>,
}

fn format_findings(findings: &[ValidationFinding]) -> String {
    findings
        .iter()
        .map(|f| f.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Returns `true` if `kind` is a block-level node.
fn is_block(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Root { .. }
            | NodeKind::Heading { .. }
            | NodeKind::Section { .. }
            | NodeKind::Paragraph { .. }
            | NodeKind::BlockQuote { .. }
            | NodeKind::List { .. }
            | NodeKind::ListItem { .. }
            | NodeKind::Code { .. }
            | NodeKind::ThematicBreak
            | NodeKind::Table { .. }
            | NodeKind::TableRow { .. }
            | NodeKind::TableCell { .. }
            | NodeKind::FootnoteDefinition { .. }
    )
}

/// Returns `true` if `kind` is an inline (phrasing-level) node.
fn is_inline_kind(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Text { .. }
            | NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Span { .. }
            | NodeKind::InlineCode { .. }
            | NodeKind::Link { .. }
            | NodeKind::Image { .. }
            | NodeKind::FootnoteReference { .. }
            | NodeKind::SoftBreak
            | NodeKind::HardBreak
    )
}

/// Returns `true` if `kind` is a container that may hold only phrasing content.
fn is_phrasing_only(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Paragraph { .. }
            | NodeKind::Heading { .. }
            | NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Span { .. }
            | NodeKind::Link { .. }
    )
}

/// A short name for a node kind, used in finding messages.
fn kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Root { .. } => "Root",
        NodeKind::Heading { .. } => "Heading",
        NodeKind::Section { .. } => "Section",
        NodeKind::Paragraph { .. } => "Paragraph",
        NodeKind::BlockQuote { .. } => "BlockQuote",
        NodeKind::List { .. } => "List",
        NodeKind::ListItem { .. } => "ListItem",
        NodeKind::Code { .. } => "Code",
        NodeKind::ThematicBreak => "ThematicBreak",
        NodeKind::Table { .. } => "Table",
        NodeKind::TableRow { .. } => "TableRow",
        NodeKind::TableCell { .. } => "TableCell",
        NodeKind::FootnoteDefinition { .. } => "FootnoteDefinition",
        NodeKind::Text { .. } => "Text",
        NodeKind::Emphasis { .. } => "Emphasis",
        NodeKind::Strong { .. } => "Strong",
        NodeKind::Delete { .. } => "Delete",
        NodeKind::Span { .. } => "Span",
        NodeKind::InlineCode { .. } => "InlineCode",
        NodeKind::Link { .. } => "Link",
        NodeKind::Image { .. } => "Image",
        NodeKind::FootnoteReference { .. } => "FootnoteReference",
        NodeKind::SoftBreak => "SoftBreak",
        NodeKind::HardBreak => "HardBreak",
        NodeKind::Html { .. } => "Html",
        NodeKind::Unsupported { .. } => "Unsupported",
    }
}

/// Validates the structure of a render tree.
///
/// The tree is walked recursively. Each violation of the Milestone 1
/// structural rules becomes a [`ValidationFinding`]. In [`ValidationMode::FailFast`]
/// the walk stops as soon as the first [`Severity::Error`] finding is recorded.
///
/// ## Returns
///
/// A [`ValidationReport`] listing every finding (or, in fail-fast mode, the
/// findings collected up to and including the first error).
#[must_use]
pub fn validate(node: &RenderNode, mode: ValidationMode) -> ValidationReport {
    let mut report = ValidationReport::default();
    walk(node, true, None, mode, &mut report);
    report
}

/// Recursively validates `node`.
///
/// `is_root_position` is `true` only for the top-level node. `parent` is the
/// kind of the enclosing node, if any.
fn walk(
    node: &RenderNode,
    is_root_position: bool,
    parent: Option<&NodeKind>,
    mode: ValidationMode,
    report: &mut ValidationReport,
) {
    if mode == ValidationMode::FailFast && report.has_errors() {
        return;
    }

    check_node(node, is_root_position, parent, report);

    if mode == ValidationMode::FailFast && report.has_errors() {
        return;
    }

    // Section.heading children must be validated as phrasing content. We use
    // a synthetic Heading parent kind to trigger the phrasing-only check.
    if let NodeKind::Section { heading, .. } = &node.kind {
        // Use a synthetic Heading as the parent to enforce phrasing-only rules.
        let synthetic_heading = NodeKind::Heading {
            depth: crate::tree::HeadingDepth::new(1).unwrap(),
            children: vec![],
        };
        for child in heading {
            walk(child, false, Some(&synthetic_heading), mode, report);
            if mode == ValidationMode::FailFast && report.has_errors() {
                return;
            }
        }
    }

    for child in node.children() {
        walk(child, false, Some(&node.kind), mode, report);
        if mode == ValidationMode::FailFast && report.has_errors() {
            return;
        }
    }
}

/// Records findings for a single node given its position and parent.
fn check_node(
    node: &RenderNode,
    is_root_position: bool,
    parent: Option<&NodeKind>,
    report: &mut ValidationReport,
) {
    let span = Some(node.span.clone());

    // `Root` may appear only as the top-level node.
    if matches!(node.kind, NodeKind::Root { .. }) && !is_root_position {
        report.findings.push(error(
            "Root node may appear only as the top-level node",
            span.clone(),
        ));
    }

    // `Unsupported` is a warning, not a structural error.
    if let NodeKind::Unsupported { label } = &node.kind {
        report.findings.push(ValidationFinding {
            severity: Severity::Warning,
            message: format!("Unsupported node: {label}"),
            span: span.clone(),
        });
    }

    // Containment rules based on the parent kind.
    match parent {
        Some(NodeKind::Table { .. }) => {}
        Some(_) | None => {
            if matches!(node.kind, NodeKind::TableRow { .. }) {
                report.findings.push(error(
                    "TableRow may appear only directly inside a Table",
                    span.clone(),
                ));
            }
        }
    }

    if matches!(node.kind, NodeKind::TableCell { .. })
        && !matches!(parent, Some(NodeKind::TableRow { .. }))
    {
        report.findings.push(error(
            "TableCell may appear only directly inside a TableRow",
            span.clone(),
        ));
    }

    if matches!(node.kind, NodeKind::ListItem { .. })
        && !matches!(parent, Some(NodeKind::List { .. }))
    {
        report.findings.push(error(
            "ListItem may appear only directly inside a List",
            span.clone(),
        ));
    }

    // Block-level nodes must not appear inside phrasing-only containers.
    if let Some(parent_kind) = parent
        && is_phrasing_only(parent_kind)
        && is_block(&node.kind)
    {
        report.findings.push(error(
            format!(
                "block-level {} node inside phrasing-only {} container",
                kind_name(&node.kind),
                kind_name(parent_kind),
            ),
            span.clone(),
        ));
    }

    // A sequence-join policy is a Root-only render contract.
    if node.attrs.sequence_join().is_some() && !matches!(node.kind, NodeKind::Root { .. }) {
        report.findings.push(error(
            format!(
                "sequence-join policy is permitted only on a Root node, found on {}",
                kind_name(&node.kind),
            ),
            span.clone(),
        ));
    }

    // A list marker policy is a List-only hint.
    if node
        .attrs
        .get_hint(crate::tree::HintNamespace::LIST, "marker_policy")
        .is_some()
        && !matches!(node.kind, NodeKind::List { .. })
    {
        report.findings.push(error(
            format!(
                "list marker policy is permitted only on a List node, found on {}",
                kind_name(&node.kind),
            ),
            span.clone(),
        ));
    }

    // Task hints are a ListItem-only hint.
    if node.attrs.task_hints().is_some() && !matches!(node.kind, NodeKind::ListItem { .. }) {
        report.findings.push(error(
            format!(
                "task hints are permitted only on a ListItem node, found on {}",
                kind_name(&node.kind),
            ),
            span.clone(),
        ));
    }

    // A table title/caption is a Table-only hint.
    if node.attrs.table_title().is_some() && !matches!(node.kind, NodeKind::Table { .. }) {
        report.findings.push(error(
            format!(
                "table title is permitted only on a Table node, found on {}",
                kind_name(&node.kind),
            ),
            span.clone(),
        ));
    }

    // Typed render hints with a known key but a malformed token or value are
    // rejected, so a serialized-and-reloaded tree cannot silently lose hint
    // intent by falling back to a default.
    check_hints(node, &span, report);

    // Layout attributes are permitted only on block-level nodes.
    if let Some(layout) = node.attrs.layout() {
        if is_inline_kind(&node.kind) {
            report.findings.push(ValidationFinding {
                severity: Severity::Warning,
                message: "layout attributes are permitted only on block-level nodes".into(),
                span: span.clone(),
            });
        }

        if let Err(err) = layout.validate() {
            report
                .findings
                .push(error(format!("invalid layout: {err}"), span));
        }
    }
}

/// Validates typed render hints whose key is present but whose serialized
/// token or value is malformed.
///
/// The hint accessors deliberately fall back to a default for unrecognized
/// tokens so a renderer never panics. That makes a malformed hint *invisible*
/// once a tree is serialized and reloaded, which weakens the render tree as a
/// typed IR. This check restores a hard signal: a known hint key carrying an
/// invalid token or wrong value type becomes an error-severity finding.
fn check_hints(node: &RenderNode, span: &Option<SourceSpan>, report: &mut ValidationReport) {
    use crate::tree::{HintNamespace, ListMarkerPolicy, SequenceJoin, TaskState};

    // Sequence-join token must be a recognized join policy.
    if let Some(value) = node.attrs.get_hint(HintNamespace::LAYOUT, "sequence_join") {
        let valid = value
            .as_str()
            .is_some_and(|token| SequenceJoin::from_token(token).is_some());
        if !valid {
            report.findings.push(error(
                format!("invalid sequence-join hint value: {value}"),
                span.clone(),
            ));
        }
    }

    // List marker policy token must be a recognized policy.
    if let Some(value) = node.attrs.get_hint(HintNamespace::LIST, "marker_policy") {
        let valid = value
            .as_str()
            .is_some_and(|token| ListMarkerPolicy::from_token(token).is_some());
        if !valid {
            report.findings.push(error(
                format!("invalid list marker policy hint value: {value}"),
                span.clone(),
            ));
        }
    }

    // Task-state token must be a recognized state.
    if let Some(value) = node.attrs.get_hint(HintNamespace::WIDGET_TASK, "state") {
        let valid = value
            .as_str()
            .is_some_and(|token| TaskState::from_token(token).is_some());
        if !valid {
            report.findings.push(error(
                format!("invalid task state hint value: {value}"),
                span.clone(),
            ));
        }
    }

    // The two-column `left_width_kind` must be `fixed` or `percent`, and the
    // paired `left_width` value must match the kind's expected JSON type.
    if let Some(kind) = node
        .attrs
        .get_hint(HintNamespace::WIDGET_COLUMNS, "left_width_kind")
    {
        let width = node
            .attrs
            .get_hint(HintNamespace::WIDGET_COLUMNS, "left_width");
        match kind.as_str() {
            Some("fixed") => {
                if let Some(width) = width
                    && u32::try_from(width.as_u64().unwrap_or(u64::MAX)).is_err()
                {
                    report.findings.push(error(
                        format!("invalid columns left_width for 'fixed' kind: {width}"),
                        span.clone(),
                    ));
                }
            }
            Some("percent") => {
                if let Some(width) = width
                    && width.as_f64().is_none()
                {
                    report.findings.push(error(
                        format!("invalid columns left_width for 'percent' kind: {width}"),
                        span.clone(),
                    ));
                }
            }
            _ => report.findings.push(error(
                format!("invalid columns left_width_kind hint value: {kind}"),
                span.clone(),
            )),
        }
    }
}

/// Builds an [`Severity::Error`] finding.
fn error(message: impl Into<String>, span: Option<SourceSpan>) -> ValidationFinding {
    ValidationFinding {
        severity: Severity::Error,
        message: message.into(),
        span,
    }
}

/// Validates a render tree and fails if any error-severity finding is found.
///
/// [`Severity::Warning`] findings do not cause a failure.
///
/// ## Errors
///
/// Returns [`ValidationError`] carrying every error-severity finding if the
/// tree violates a structural rule.
pub fn ensure_valid(node: &RenderNode) -> Result<(), ValidationError> {
    let report = validate(node, ValidationMode::Full);
    let errors: Vec<ValidationFinding> = report.errors().cloned().collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError { findings: errors })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{ColumnAlign, HeadingDepth};

    #[test]
    fn valid_tree_has_no_findings() {
        let tree = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("hello")])]);
        let report = validate(&tree, ValidationMode::Full);
        assert!(report.is_empty());
        assert!(!report.has_errors());
    }

    #[test]
    fn orphaned_table_cell_is_an_error() {
        // A TableCell directly inside a Paragraph: orphaned and block-in-phrasing.
        let tree = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::table_cell(
            vec![RenderNode::text("x")],
        )])]);
        let report = validate(&tree, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("TableCell may appear only"))
        );
    }

    #[test]
    fn block_inside_paragraph_is_an_error() {
        let tree = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::block_quote(
            vec![RenderNode::text("x")],
        )])]);
        let report = validate(&tree, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("block-level BlockQuote"))
        );
    }

    #[test]
    fn unsupported_node_is_a_warning_not_error() {
        let tree = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::unsupported(
            "custom directive",
        )])]);
        let report = validate(&tree, ValidationMode::Full);
        assert!(!report.has_errors());
        assert!(!report.is_empty());
        assert_eq!(report.findings[0].severity, Severity::Warning);
        assert!(ensure_valid(&tree).is_ok());
    }

    #[test]
    fn ensure_valid_fails_on_error_tree_and_passes_otherwise() {
        let bad = RenderNode::root(vec![RenderNode::table_cell(vec![])]);
        assert!(ensure_valid(&bad).is_err());

        let clean = RenderNode::root(vec![RenderNode::heading(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Title")],
        )]);
        assert!(ensure_valid(&clean).is_ok());

        let warning_only = RenderNode::root(vec![RenderNode::unsupported("thing")]);
        assert!(ensure_valid(&warning_only).is_ok());
    }

    #[test]
    fn fail_fast_stops_at_first_error() {
        // Two independent errors: an orphaned ListItem and an orphaned TableRow.
        let tree = RenderNode::root(vec![
            RenderNode::list_item(None, vec![]),
            RenderNode::table_row(vec![]),
        ]);
        let full = validate(&tree, ValidationMode::Full);
        let fast = validate(&tree, ValidationMode::FailFast);
        assert!(full.errors().count() >= 2);
        assert_eq!(fast.errors().count(), 1);
        assert!(fast.findings.len() < full.findings.len());
    }

    #[test]
    fn nested_root_is_an_error() {
        let tree = RenderNode::root(vec![RenderNode::root(vec![])]);
        let report = validate(&tree, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("Root node may appear only"))
        );
    }

    #[test]
    fn valid_table_passes() {
        let tree = RenderNode::root(vec![RenderNode::table(
            vec![ColumnAlign::Left],
            vec![RenderNode::table_row(vec![RenderNode::table_cell(vec![
                RenderNode::text("cell"),
            ])])],
        )]);
        assert!(ensure_valid(&tree).is_ok());
    }

    #[test]
    fn section_builder_roundtrip() {
        let section = RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Title")],
            vec![RenderNode::paragraph(vec![RenderNode::text("Body")])],
        );
        assert!(matches!(section.kind, NodeKind::Section { .. }));
        // children() returns the body children, not the heading
        assert_eq!(section.children().len(), 1);
    }

    #[test]
    fn section_children_access() {
        let mut section = RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Heading")],
            vec![
                RenderNode::paragraph(vec![RenderNode::text("Para 1")]),
                RenderNode::paragraph(vec![RenderNode::text("Para 2")]),
            ],
        );
        assert_eq!(section.children().len(), 2);
        // children_mut returns mutable access to body children
        let children = section.children_mut().unwrap();
        children.push(RenderNode::paragraph(vec![RenderNode::text("Para 3")]));
        assert_eq!(section.children().len(), 3);
    }

    #[test]
    fn valid_section_passes() {
        let tree = RenderNode::root(vec![RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Section Title")],
            vec![RenderNode::paragraph(vec![RenderNode::text("Body text")])],
        )]);
        assert!(ensure_valid(&tree).is_ok());
    }

    #[test]
    fn block_in_section_heading_is_an_error() {
        // Section.heading must contain only phrasing content; a Paragraph is
        // block-level and violates this rule.
        let tree = RenderNode::root(vec![RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::paragraph(vec![RenderNode::text(
                "Block in heading",
            )])],
            vec![],
        )]);
        let report = validate(&tree, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("block-level Paragraph"))
        );
    }

    #[test]
    fn nested_section_passes() {
        // Sections may contain other sections in their body.
        let tree = RenderNode::root(vec![RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Parent")],
            vec![RenderNode::section(
                HeadingDepth::new(2).unwrap(),
                vec![RenderNode::text("Child")],
                vec![RenderNode::paragraph(vec![RenderNode::text("Content")])],
            )],
        )]);
        assert!(ensure_valid(&tree).is_ok());
    }

    #[test]
    fn layout_on_inline_node_is_a_warning() {
        use crate::layout::Layout;

        let mut text = RenderNode::text("hello");
        text.attrs.set_layout(&Layout::default());
        let root = RenderNode::root(vec![RenderNode::paragraph(vec![text])]);

        let report = validate(&root, ValidationMode::Full);
        assert!(
            !report.has_errors(),
            "layout on an inline Text node must be a warning, not an error"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.severity == Severity::Warning && f.message.contains("block-level")),
            "should contain a warning about block-level"
        );
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn layout_on_block_node_is_valid() {
        use crate::layout::Layout;

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout::default());
        let root = RenderNode::root(vec![para]);

        let report = validate(&root, ValidationMode::Full);
        assert!(!report.has_errors());
    }

    #[test]
    fn section_is_block_level() {
        // A Section inside a phrasing-only container should be an error.
        let tree = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Bad")],
            vec![],
        )])]);
        let report = validate(&tree, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("block-level Section"))
        );
    }

    #[test]
    fn invalid_layout_rejected_by_tree_validation() {
        use crate::layout::{Layout, Length, TargetValue};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            max_width: Some(TargetValue::universal(Length::Percent(200.0))),
            ..Layout::default()
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid layout")),
            "invalid layout values should produce an error finding"
        );
    }

    #[test]
    fn invalid_universal_css_unit_rejected_by_tree_validation() {
        use crate::layout::{Layout, Length, TargetValue};
        use crate::stylesheet::CssSizing;

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            max_width: Some(TargetValue::universal(Length::css(CssSizing::rem(1.0)))),
            ..Layout::default()
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid layout")),
            "CSS units in universal branch should produce an error finding"
        );
    }

    #[test]
    fn sequence_join_on_non_root_is_an_error() {
        use crate::tree::SequenceJoin;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_sequence_join(SequenceJoin::None);
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("sequence-join policy is permitted only"))
        );
    }

    #[test]
    fn sequence_join_on_root_is_valid() {
        use crate::tree::SequenceJoin;
        let mut root = RenderNode::root(vec![RenderNode::text("foo")]);
        root.attrs.set_sequence_join(SequenceJoin::None);
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn list_marker_policy_on_non_list_is_an_error() {
        use crate::tree::ListMarkerPolicy;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs
            .set_list_marker_policy(ListMarkerPolicy::TreeConnectors);
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("list marker policy is permitted only"))
        );
    }

    #[test]
    fn list_marker_policy_on_list_is_valid() {
        use crate::tree::ListMarkerPolicy;
        let mut list = RenderNode::list(false, None, vec![RenderNode::list_item(None, vec![])]);
        list.attrs.set_list_marker_policy(ListMarkerPolicy::None);
        let root = RenderNode::root(vec![list]);
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn task_hints_on_non_list_item_is_an_error() {
        use crate::tree::{TaskHints, TaskState};
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_task_hints(&TaskHints {
            state: TaskState::Open,
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("task hints are permitted only"))
        );
    }

    #[test]
    fn task_hints_on_list_item_is_valid() {
        use crate::tree::{TaskHints, TaskState};
        let mut item = RenderNode::list_item(Some(false), vec![]);
        item.attrs.set_task_hints(&TaskHints {
            state: TaskState::InProgress,
        });
        let root = RenderNode::root(vec![RenderNode::list(false, None, vec![item])]);
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn table_title_on_non_table_is_an_error() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_table_title("Bad");
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("table title is permitted only"))
        );
    }

    #[test]
    fn table_title_on_table_is_valid() {
        let mut table = RenderNode::table(
            vec![ColumnAlign::Left],
            vec![RenderNode::table_row(vec![RenderNode::table_cell(vec![
                RenderNode::text("H"),
            ])])],
        );
        table.attrs.set_table_title("Caption");
        let root = RenderNode::root(vec![table]);
        assert!(ensure_valid(&root).is_ok());
    }

    // ── Malformed typed-hint validation ────────────────────────────────────

    #[test]
    fn invalid_sequence_join_token_is_an_error() {
        use crate::tree::HintNamespace;
        let mut root = RenderNode::root(vec![RenderNode::text("x")]);
        root.attrs.set_hint(
            HintNamespace::LAYOUT,
            "sequence_join",
            serde_json::json!("bogus"),
        );
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid sequence-join hint value"))
        );
    }

    #[test]
    fn invalid_list_marker_policy_token_is_an_error() {
        use crate::tree::HintNamespace;
        let mut list = RenderNode::list(false, None, vec![RenderNode::list_item(None, vec![])]);
        list.attrs.set_hint(
            HintNamespace::LIST,
            "marker_policy",
            serde_json::json!("bogus"),
        );
        let root = RenderNode::root(vec![list]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid list marker policy hint value"))
        );
    }

    #[test]
    fn invalid_task_state_token_is_an_error() {
        use crate::tree::HintNamespace;
        let mut item = RenderNode::list_item(Some(false), vec![]);
        item.attrs.set_hint(
            HintNamespace::WIDGET_TASK,
            "state",
            serde_json::json!("bogus"),
        );
        let root = RenderNode::root(vec![RenderNode::list(false, None, vec![item])]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid task state hint value"))
        );
    }

    #[test]
    fn wrong_type_task_state_hint_is_an_error() {
        use crate::tree::HintNamespace;
        let mut item = RenderNode::list_item(Some(false), vec![]);
        item.attrs
            .set_hint(HintNamespace::WIDGET_TASK, "state", serde_json::json!(42));
        let root = RenderNode::root(vec![RenderNode::list(false, None, vec![item])]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid task state hint value"))
        );
    }

    #[test]
    fn invalid_columns_left_width_kind_is_an_error() {
        use crate::tree::HintNamespace;
        let mut bq = RenderNode::block_quote(vec![]);
        bq.attrs.set_hint(
            HintNamespace::WIDGET_COLUMNS,
            "left_width_kind",
            serde_json::json!("bogus"),
        );
        let root = RenderNode::root(vec![bq]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid columns left_width_kind"))
        );
    }

    #[test]
    fn invalid_columns_left_width_value_is_an_error() {
        use crate::tree::HintNamespace;
        let mut bq = RenderNode::block_quote(vec![]);
        bq.attrs.set_hint(
            HintNamespace::WIDGET_COLUMNS,
            "left_width_kind",
            serde_json::json!("fixed"),
        );
        bq.attrs.set_hint(
            HintNamespace::WIDGET_COLUMNS,
            "left_width",
            serde_json::json!("not-a-number"),
        );
        let root = RenderNode::root(vec![bq]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid columns left_width for 'fixed'"))
        );
    }

    #[test]
    fn well_formed_typed_hints_pass_validation() {
        use crate::tree::{ListMarkerPolicy, SequenceJoin, TaskHints, TaskState};
        let mut item = RenderNode::list_item(Some(false), vec![]);
        item.attrs.set_task_hints(&TaskHints {
            state: TaskState::Blocked,
        });
        let mut list = RenderNode::list(false, None, vec![item]);
        list.attrs
            .set_list_marker_policy(ListMarkerPolicy::TreeConnectors);
        let mut root = RenderNode::root(vec![list]);
        root.attrs.set_sequence_join(SequenceJoin::None);
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn empty_per_target_map_rejected_by_tree_validation() {
        use crate::layout::{Layout, TargetValue};
        use std::collections::BTreeMap;

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            max_width: Some(TargetValue::PerTarget(BTreeMap::new())),
            ..Layout::default()
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.has_errors());
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("invalid layout")),
            "empty per-target map should produce an error finding"
        );
    }
}
