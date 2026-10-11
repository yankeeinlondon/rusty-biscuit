//! C# re-exports: `[assembly: TypeForwardedTo(typeof(N.T))]` only.
//!
//! Every `using` form is a per-file or per-compilation name lookup, not a
//! forward, and is not reported. The target assembly is not in the file, so
//! each entry has no `source` and carries `source_unresolved`.

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportCaveat, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimitReason,
    ReexportReport,
};

const TRIGGER: &str = "TypeForwardedTo";

pub(super) fn extract(tree_file: &TreeFile, _imports: &[ImportRecord]) -> ReexportReport {
    let root = tree_file.root_node();
    let source = tree_file.source();
    let mut entries = Vec::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if node.kind() == "global_attribute" {
            if !node.has_error() && targets_assembly(node) {
                collect_attributes(node, source, &mut entries);
            }
            continue;
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }

    let limits = damaged_nodes(root)
        .into_iter()
        .filter(|damaged| touches_trigger(*damaged, source))
        .map(|damaged| limit(ReexportLimitReason::ParseError, damaged))
        .collect();
    finish(entries, limits)
}

/// Whether the attribute list's target is `assembly` (not `module`).
fn targets_assembly(list: Node<'_>) -> bool {
    let mut cursor = list.walk();
    list.children(&mut cursor).any(|child| !child.is_named() && child.kind() == "assembly")
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or_default()
}

fn collect_attributes(list: Node<'_>, source: &str, entries: &mut Vec<ReexportEntry>) {
    let mut cursor = list.walk();
    for attribute in list.children(&mut cursor).filter(|child| child.kind() == "attribute") {
        if let Some(forwarded) = forwarded_type(attribute, list, source) {
            entries.push(forwarded);
        }
    }
}

/// The entry for `TypeForwardedTo(typeof(T))`, or `None` for any other
/// attribute or argument shape.
fn forwarded_type(attribute: Node<'_>, list: Node<'_>, source: &str) -> Option<ReexportEntry> {
    let name = attribute.child(0)?;
    let written = text(name, source);
    let last = written.rsplit('.').next().unwrap_or(written).trim();
    if last != TRIGGER && last != "TypeForwardedToAttribute" {
        return None;
    }
    let arguments = attribute.child_by_field_name("arguments").or_else(|| {
        let mut cursor = attribute.walk();
        attribute.children(&mut cursor).find(|child| child.kind() == "attribute_argument_list")
    })?;
    let mut cursor = arguments.walk();
    let mut real = arguments.children(&mut cursor).filter(|child| child.kind() == "attribute_argument");
    let argument = real.next()?;
    if real.next().is_some() {
        return None;
    }
    let typeof_expression = argument.named_child(0).filter(|node| node.kind() == "typeof_expression")?;
    let mut inner = typeof_expression.walk();
    let target = typeof_expression.named_children(&mut inner).next()?;
    let target_text: String = text(target, source).chars().filter(|c| !c.is_whitespace()).collect();

    let mut forwarded = entry(
        ReexportForm::Named,
        ReexportEvidence::Syntax,
        text_span_for_node(attribute),
        text_span_for_node(list),
    );
    forwarded.exported_name = Some(target_text.clone());
    forwarded.original_name = Some(target_text);
    forwarded.caveats = vec![ReexportCaveat::SourceUnresolved];
    Some(forwarded)
}

/// Whether a damaged region contains or directly follows `TypeForwardedTo`,
/// or sits inside an attribute list that names it.
fn touches_trigger(damaged: Node<'_>, source: &str) -> bool {
    if text(damaged, source).contains(TRIGGER) || source[..damaged.start_byte()].trim_end().ends_with(TRIGGER) {
        return true;
    }
    let mut ancestor = damaged.parent();
    while let Some(node) = ancestor {
        if node.parent().is_none() {
            break;
        }
        if matches!(node.kind(), "global_attribute" | "attribute" | "attribute_list")
            && text(node, source).contains(TRIGGER)
        {
            return true;
        }
        ancestor = node.parent();
    }
    false
}
