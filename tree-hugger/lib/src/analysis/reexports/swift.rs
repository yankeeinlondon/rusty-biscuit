//! Swift re-exports: `@_exported import`, whole-module or selective.
//!
//! `public import` and plain imports are access-level or dependency facts,
//! not forwarding, and are not reported. `@_exported` is an underscored
//! attribute, so every entry carries the `unstable_feature` caveat.

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportCaveat, ReexportEntry, ReexportEvidence, ReexportForm,
    ReexportLimitReason, ReexportReport,
};

/// The attribute that makes an import forward its module.
const FORWARDING_ATTRIBUTE: &str = "_exported";

/// Import kinds (`import struct Foo.S`) that select one declaration.
const SELECTIVE_KINDS: [&str; 8] = ["typealias", "struct", "class", "enum", "protocol", "let", "var", "func"];

pub(super) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    let root = tree_file.root_node();
    let source = tree_file.source();
    let mut entries = Vec::new();
    let mut limits = Vec::new();

    // `#if` has no wrapper node: it is a sibling `directive` before the
    // declarations it guards and another `#endif` after them.
    let mut open_conditions = 0usize;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "directive" => match first_token_kind(child) {
                Some("#if") => open_conditions += 1,
                Some("#endif") => open_conditions = open_conditions.saturating_sub(1),
                _ => {}
            },
            "import_declaration" => {
                if !has_forwarding_attribute(child, source) {
                    continue;
                }
                if child.has_error() {
                    limits.push(limit(ReexportLimitReason::ParseError, child));
                    continue;
                }
                match import_entry(child, source, imports, open_conditions > 0) {
                    Some(found) => entries.push(found),
                    // The grammar accepts `import` with a zero-width module
                    // name without marking it damaged.
                    None => limits.push(limit(ReexportLimitReason::ParseError, child)),
                }
            }
            _ => {}
        }
    }

    for damaged in damaged_nodes(root) {
        if touches_forwarding(damaged, source) {
            limits.push(limit(ReexportLimitReason::ParseError, damaged));
        }
    }
    finish(entries, limits)
}

fn first_token_kind(node: Node<'_>) -> Option<&str> {
    node.child(0).map(|token| token.kind())
}

/// Whether `modifiers` holds an `@_exported` attribute.
fn has_forwarding_attribute(declaration: Node<'_>, source: &str) -> bool {
    let mut cursor = declaration.walk();
    declaration
        .children(&mut cursor)
        .filter(|child| child.kind() == "modifiers")
        .any(|modifiers| {
            let mut inner = modifiers.walk();
            modifiers.children(&mut inner).any(|attribute| {
                attribute.kind() == "attribute"
                    && attribute
                        .named_child(0)
                        .is_some_and(|name| text(name, source) == FORWARDING_ATTRIBUTE)
            })
        })
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or_default()
}

fn import_entry(
    declaration: Node<'_>,
    source: &str,
    imports: &[ImportRecord],
    conditional: bool,
) -> Option<ReexportEntry> {
    let mut cursor = declaration.walk();
    let children: Vec<Node<'_>> = declaration.children(&mut cursor).collect();
    let path = children.iter().find(|child| child.kind() == "identifier")?;
    let selective = children.iter().any(|child| !child.is_named() && SELECTIVE_KINDS.contains(&child.kind()));
    let written: String = text(*path, source).chars().filter(|c| !c.is_whitespace()).collect();
    if written.is_empty() {
        return None;
    }

    let mut forwarded = if selective {
        let (module, name) = match written.rsplit_once('.') {
            Some((module, name)) => (Some(module.to_owned()), name.to_owned()),
            None => (None, written.clone()),
        };
        let mut named = entry(
            ReexportForm::Named,
            ReexportEvidence::Syntax,
            text_span_for_node(*path),
            text_span_for_node(declaration),
        );
        named.exported_name = Some(name.clone());
        named.original_name = Some(name);
        named.source = module;
        named
    } else {
        let mut wildcard = entry(
            ReexportForm::Wildcard,
            ReexportEvidence::Syntax,
            text_span_for_node(*path),
            text_span_for_node(declaration),
        );
        wildcard.source = Some(written);
        wildcard
    };
    forwarded.conditional = conditional;
    forwarded.caveats = vec![ReexportCaveat::UnstableFeature];
    forwarded.import_span = imports
        .iter()
        .find(|import| {
            import.span.start_byte >= declaration.start_byte() as u32
                && import.span.end_byte <= declaration.end_byte() as u32
        })
        .map(|import| import.span.clone());
    Some(forwarded)
}

/// Whether a damaged region contains or directly follows `_exported`, or
/// sits inside an import declaration that names it.
fn touches_forwarding(damaged: Node<'_>, source: &str) -> bool {
    if text(damaged, source).contains(FORWARDING_ATTRIBUTE) {
        return true;
    }
    if source[..damaged.start_byte()].trim_end().ends_with(FORWARDING_ATTRIBUTE) {
        return true;
    }
    let mut ancestor = damaged.parent();
    while let Some(node) = ancestor {
        if node.parent().is_none() {
            break;
        }
        if matches!(node.kind(), "import_declaration" | "modifiers" | "attribute")
            && text(node, source).contains(FORWARDING_ATTRIBUTE)
        {
            return true;
        }
        ancestor = node.parent();
    }
    false
}


