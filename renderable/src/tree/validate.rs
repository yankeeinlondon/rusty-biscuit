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

use crate::tree::ComponentHints;
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
            | NodeKind::Disclosure { .. }
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
            | NodeKind::Extended { .. }
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
        NodeKind::Disclosure { .. } => "Disclosure",
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
        NodeKind::Extended { .. } => "Extended",
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
    if node.attrs.list_marker_policy != crate::tree::ListMarkerPolicy::default()
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

    // Per-component hints are matched to a specific node kind. Validation reads
    // the typed `component` field directly — no `data`-bag round-trip — and a
    // hint carried on the wrong kind is malformed typed IR. The table-title
    // rule below adds its own message for the title slice of `Table` hints.
    if let Some(component) = node.attrs.component.as_deref() {
        let mismatch: Option<(&str, &str)> = match component {
            ComponentHints::List(_) if !matches!(node.kind, NodeKind::List { .. }) => {
                Some(("list render hints", "List"))
            }
            ComponentHints::Code(_) if !matches!(node.kind, NodeKind::Code { .. }) => {
                Some(("code render hints", "Code"))
            }
            ComponentHints::Progress(_) if !matches!(node.kind, NodeKind::Paragraph { .. }) => {
                Some(("progress hints", "Paragraph"))
            }
            ComponentHints::Columns(_) if !matches!(node.kind, NodeKind::BlockQuote { .. }) => {
                Some(("column hints", "BlockQuote"))
            }
            ComponentHints::Task(_) if !matches!(node.kind, NodeKind::ListItem { .. }) => {
                Some(("task hints", "ListItem"))
            }
            ComponentHints::Table(_) if !matches!(node.kind, NodeKind::Table { .. }) => {
                Some(("table hints", "Table"))
            }
            ComponentHints::TableCell(_) if !matches!(node.kind, NodeKind::TableCell { .. }) => {
                Some(("table cell hints", "TableCell"))
            }
            _ => None,
        };
        if let Some((what, required_kind)) = mismatch {
            report.findings.push(error(
                format!(
                    "{what} are permitted only on a {required_kind} node, found on {}",
                    kind_name(&node.kind),
                ),
                span.clone(),
            ));
        }
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

    // First-class presentation now lives in typed `NodeAttrs` fields, so a
    // `renderable.`-namespaced key in `data` is a stale hint that no accessor
    // reads. Reject it rather than let it silently drop intent. Other
    // namespaces (`darkmatter.*`, etc.) are package-local extensions and pass.
    for key in node.attrs.data.keys() {
        if key.starts_with("renderable.") {
            report.findings.push(error(
                format!(
                    "stale renderable-owned hint key in data: {key}; first-class attrs are typed fields"
                ),
                span.clone(),
            ));
        }
    }

    // Width-dependent text intent is supported only on link, image, and
    // list-item nodes; the renderers resolve it against those kinds.
    if node.attrs.text_layout_ref().is_some()
        && !matches!(
            node.kind,
            NodeKind::Link { .. } | NodeKind::Image { .. } | NodeKind::ListItem { .. }
        )
    {
        report.findings.push(error(
            format!(
                "text-layout hints are permitted only on Link, Image, or ListItem nodes, found on {}",
                kind_name(&node.kind),
            ),
            span.clone(),
        ));
    }

    // Thematic-break styling is supported only on a ThematicBreak node; the
    // terminal and browser renderers read it only when folding that kind.
    if node.attrs.thematic_break_ref().is_some()
        && !matches!(node.kind, NodeKind::ThematicBreak)
    {
        report.findings.push(error(
            format!(
                "thematic-break attributes are permitted only on a ThematicBreak node, found on {}",
                kind_name(&node.kind),
            ),
            span.clone(),
        ));
    }

    // Kind-specific browser sub-groups: a link group belongs only on a Link
    // node, an image group only on an Image node. The remaining browser fields
    // (inline_style, data/aria attrs) apply to any node, and the validated name
    // newtypes already guarantee safe attribute names at construction.
    if let Some(browser) = node.attrs.browser_ref() {
        if browser.link.is_some() && !matches!(node.kind, NodeKind::Link { .. }) {
            report.findings.push(error(
                format!(
                    "link browser attributes are permitted only on a Link node, found on {}",
                    kind_name(&node.kind),
                ),
                span.clone(),
            ));
        }
        if browser.image.is_some() && !matches!(node.kind, NodeKind::Image { .. }) {
            report.findings.push(error(
                format!(
                    "image browser attributes are permitted only on an Image node, found on {}",
                    kind_name(&node.kind),
                ),
                span.clone(),
            ));
        }
    }

    // Layout attributes are permitted only on block-level nodes.
    if let Some(layout) = node.attrs.layout() {
        if is_inline_kind(&node.kind) {
            report.findings.push(error(
                "layout attributes are permitted only on block-level nodes",
                span.clone(),
            ));
        }

        if let Err(err) = layout.validate() {
            report
                .findings
                .push(error(format!("invalid layout: {err}"), span));
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
    fn layout_on_inline_node_is_an_error() {
        use crate::layout::Layout;

        let mut text = RenderNode::text("hello");
        text.attrs.set_layout(&Layout::default());
        let root = RenderNode::root(vec![RenderNode::paragraph(vec![text])]);

        let report = validate(&root, ValidationMode::Full);
        assert!(
            report.has_errors(),
            "layout on an inline Text node must be an error: Layout is block-only"
        );
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("block-level")),
            "should contain an error about block-level"
        );
        assert!(ensure_valid(&root).is_err());
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

    // ── Typed ComponentHints kind-placement ───────────────────────────────

    #[test]
    fn list_hints_on_non_list_is_an_error() {
        use crate::tree::ListRenderHints;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_list_hints(&ListRenderHints {
            bullet: Some("* ".into()),
            ..Default::default()
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("list render hints are permitted only"))
        );
    }

    #[test]
    fn code_hints_on_non_code_is_an_error() {
        use crate::tree::CodeRenderHints;
        let mut item = RenderNode::list_item(None, vec![]);
        item.attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            ..Default::default()
        });
        let root = RenderNode::root(vec![RenderNode::list(false, None, vec![item])]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("code render hints are permitted only"))
        );
    }

    #[test]
    fn code_hints_on_code_is_valid() {
        use crate::tree::CodeRenderHints;
        let mut code = RenderNode::code(Some("rust".into()), None, "let x = 1;");
        code.attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            ..Default::default()
        });
        let root = RenderNode::root(vec![code]);
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn progress_hints_on_non_paragraph_is_an_error() {
        use crate::tree::ProgressHints;
        let mut bq = RenderNode::block_quote(vec![RenderNode::text("x")]);
        bq.attrs.set_progress_hints(&ProgressHints::default());
        let root = RenderNode::root(vec![bq]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("progress hints are permitted only"))
        );
    }

    #[test]
    fn progress_hints_on_paragraph_is_valid() {
        use crate::tree::ProgressHints;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("50%")]);
        para.attrs.set_progress_hints(&ProgressHints::default());
        let root = RenderNode::root(vec![para]);
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn columns_hints_on_non_block_quote_is_an_error() {
        use crate::tree::ColumnsHints;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_columns_hints(&ColumnsHints::default());
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("column hints are permitted only"))
        );
    }

    #[test]
    fn columns_hints_on_block_quote_is_valid() {
        use crate::tree::ColumnsHints;
        let mut bq = RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(
            "left",
        )])]);
        bq.attrs.set_columns_hints(&ColumnsHints::default());
        let root = RenderNode::root(vec![bq]);
        assert!(ensure_valid(&root).is_ok());
    }

    #[test]
    fn table_column_hints_on_non_table_is_an_error() {
        use crate::tree::TableColumnHints;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_table_column_hints(
            0,
            &TableColumnHints {
                min_width: Some(4),
                ..Default::default()
            },
        );
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("table hints are permitted only")),
            "table column hints on a non-Table node must be an error",
        );
    }

    #[test]
    fn table_terminal_hints_on_non_table_is_an_error() {
        use crate::tree::TableTerminalHints;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_table_terminal_hints(&TableTerminalHints {
            alternate_background: true,
            ..Default::default()
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("table hints are permitted only")),
            "table terminal hints on a non-Table node must be an error",
        );
    }

    #[test]
    fn table_cell_hints_on_non_cell_is_an_error() {
        use crate::tree::TableCellHints;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_table_cell_hints(&TableCellHints {
            kind: "integer".into(),
            raw_value: serde_json::json!(1),
            alignment: "right".into(),
            vertical_alignment: "top".into(),
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("table cell hints are permitted only"))
        );
    }

    #[test]
    fn table_cell_hints_on_cell_is_valid() {
        use crate::tree::TableCellHints;
        let mut cell = RenderNode::table_cell(vec![RenderNode::text("42")]);
        cell.attrs.set_table_cell_hints(&TableCellHints {
            kind: "integer".into(),
            raw_value: serde_json::json!(42),
            alignment: "right".into(),
            vertical_alignment: "top".into(),
        });
        let root = RenderNode::root(vec![RenderNode::table(
            vec![ColumnAlign::Right],
            vec![RenderNode::table_row(vec![cell])],
        )]);
        assert!(ensure_valid(&root).is_ok());
    }

    // ── Stale renderable-owned data keys ───────────────────────────────────

    #[test]
    fn stale_renderable_hint_key_in_data_is_an_error() {
        // First-class hints are typed fields now; a `renderable.`-namespaced
        // key in `data` is stale and no accessor reads it.
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs
            .data
            .insert("renderable.layout.layout".into(), serde_json::json!({}));
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("renderable.")),
            "a stale renderable.* data key must be an error"
        );
    }

    #[test]
    fn extension_namespace_key_in_data_is_allowed() {
        // Non-`renderable.` namespaces are opaque package-local extensions and
        // pass; only `renderable.*` keys are rejected as stale.
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs
            .data
            .insert("myapp.custom.kind".into(), serde_json::json!("solid"));
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(report.is_empty(), "extension-namespace keys must pass");
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

    // ── Typed text-layout placement ───────────────────────────────────────

    #[test]
    fn text_layout_on_paragraph_is_an_error() {
        use crate::tree::TextLayoutHints;
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_text_layout(&TextLayoutHints::default());
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("text-layout hints are permitted only")),
            "text-layout hints on a Paragraph must be an error",
        );
    }

    #[test]
    fn text_layout_on_link_image_and_list_item_is_valid() {
        use crate::tree::TextLayoutHints;

        let mut link = RenderNode::link("u", None, vec![RenderNode::text("l")]);
        link.attrs.set_text_layout(&TextLayoutHints::default());
        let mut image = RenderNode::image("u", None, "alt");
        image.attrs.set_text_layout(&TextLayoutHints::default());
        let mut item = RenderNode::list_item(None, vec![]);
        item.attrs.set_text_layout(&TextLayoutHints::default());

        let root = RenderNode::root(vec![
            RenderNode::paragraph(vec![link, image]),
            RenderNode::list(false, None, vec![item]),
        ]);
        assert!(ensure_valid(&root).is_ok());
    }

    // ── Typed thematic-break placement ────────────────────────────────────

    #[test]
    fn thematic_break_attrs_on_paragraph_is_an_error() {
        use crate::tree::{HrKind, ThematicBreakAttrs};
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_thematic_break(&ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            ..Default::default()
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("thematic-break attributes are permitted only")),
            "thematic-break attributes on a Paragraph must be an error",
        );
    }

    #[test]
    fn thematic_break_attrs_on_thematic_break_is_valid() {
        use crate::tree::{HrKind, HrWeight, ThematicBreakAttrs};
        let mut hr = RenderNode::thematic_break();
        hr.attrs.set_thematic_break(&ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            weight: Some(HrWeight::Thick),
            ..Default::default()
        });
        let root = RenderNode::root(vec![hr]);
        assert!(ensure_valid(&root).is_ok());
    }

    // ── Typed browser-attribute placement ─────────────────────────────────

    #[test]
    fn link_browser_attrs_on_non_link_is_an_error() {
        use crate::tree::{BrowserAttrs, LinkBrowserAttrs, LinkTarget};
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.set_browser(&BrowserAttrs {
            link: Some(LinkBrowserAttrs {
                target: Some(LinkTarget::Blank),
                ..Default::default()
            }),
            ..Default::default()
        });
        let root = RenderNode::root(vec![para]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("link browser attributes are permitted only")),
        );
    }

    #[test]
    fn image_browser_attrs_on_non_image_is_an_error() {
        use crate::tree::{BrowserAttrs, ImageBrowserAttrs, ImageLoading};
        let mut link = RenderNode::link("u", None, vec![RenderNode::text("l")]);
        link.attrs.set_browser(&BrowserAttrs {
            image: Some(ImageBrowserAttrs {
                loading: Some(ImageLoading::Lazy),
                ..Default::default()
            }),
            ..Default::default()
        });
        let root = RenderNode::root(vec![RenderNode::paragraph(vec![link])]);
        let report = validate(&root, ValidationMode::Full);
        assert!(
            report
                .errors()
                .any(|f| f.message.contains("image browser attributes are permitted only")),
        );
    }

    #[test]
    fn valid_browser_attrs_match_their_node_kind() {
        use crate::tree::{
            BrowserAttrs, DataAttrName, ImageBrowserAttrs, ImageLoading, LinkBrowserAttrs,
            LinkTarget,
        };

        let mut link = RenderNode::link("u", None, vec![RenderNode::text("l")]);
        link.attrs.set_browser(&BrowserAttrs {
            link: Some(LinkBrowserAttrs {
                target: Some(LinkTarget::Blank),
                ..Default::default()
            }),
            ..Default::default()
        });
        let mut image = RenderNode::image("u", None, "alt");
        image.attrs.set_browser(&BrowserAttrs {
            image: Some(ImageBrowserAttrs {
                loading: Some(ImageLoading::Lazy),
                ..Default::default()
            }),
            ..Default::default()
        });
        // The generic fields (inline_style, data/aria attrs) are valid anywhere.
        let mut para = RenderNode::paragraph(vec![link, image]);
        para.attrs.set_browser(&BrowserAttrs {
            data_attrs: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(DataAttrName::new("prompt").unwrap(), "x".into());
                m
            },
            ..Default::default()
        });
        let root = RenderNode::root(vec![para]);
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
