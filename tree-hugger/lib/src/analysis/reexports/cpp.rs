//! C++ re-export extraction: namespace-scope `using` declarations and
//! directives.
//!
//! Type aliases, class-member `using` and `#include` are not forwarding.
//! C++20 module syntax is outside the pinned grammar (it parses as ordinary
//! declarations or as errors), so a file that contains it is `limited` as a
//! whole.

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportCaveat, ReexportEntry, ReexportEvidence, ReexportForm,
    ReexportLimitReason, ReexportReport,
};

pub(super) fn extract(tree_file: &TreeFile, _imports: &[ImportRecord]) -> ReexportReport {
    let root = tree_file.root_node();
    let source = tree_file.source().as_bytes();
    let mut entries = Vec::new();
    let mut limits = Vec::new();

    if has_module_syntax(root, source) {
        limits.push(limit(ReexportLimitReason::GrammarGap, root));
    }
    collect(root, None, source, &mut entries);
    for damaged in damaged_nodes(root) {
        if touches_using(damaged, source) {
            limits.push(limit(ReexportLimitReason::ParseError, damaged));
        }
    }
    finish(entries, limits)
}

/// Nodes through which a namespace-scope declaration stays namespace-scope.
fn is_transparent(kind: &str) -> bool {
    matches!(
        kind,
        "translation_unit"
            | "declaration_list"
            | "linkage_specification"
            | "preproc_if"
            | "preproc_ifdef"
            | "preproc_else"
            | "preproc_elif"
            | "preproc_elifdef"
    )
}

fn collect(node: Node<'_>, scope: Option<&str>, source: &[u8], entries: &mut Vec<ReexportEntry>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "using_declaration" {
            if let Some(found) = using_entry(child, scope, source) {
                entries.push(found);
            }
        } else if kind == "namespace_definition" {
            let inner = match child.child_by_field_name("name") {
                Some(name) => Some(join_scope(scope, &compact(name, source))),
                None => scope.map(str::to_owned),
            };
            if let Some(body) = child.child_by_field_name("body") {
                collect(body, inner.as_deref(), source, entries);
            }
        } else if is_transparent(kind) {
            // A module-export region is misparsed; nothing inside it is a
            // namespace-scope forward, and the file is limited anyway.
            if !starts_with_module_word(child, source) {
                collect(child, scope, source, entries);
            }
        }
    }
}

fn join_scope(outer: Option<&str>, name: &str) -> String {
    match outer {
        Some(outer) => format!("{outer}::{name}"),
        None => name.to_owned(),
    }
}

/// Source text of `node` with all whitespace removed (`a :: b` -> `a::b`).
fn compact(node: Node<'_>, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or_default().split_whitespace().collect()
}

fn has_token(node: Node<'_>, token: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| !child.is_named() && child.kind() == token)
}

fn using_entry(node: Node<'_>, scope: Option<&str>, source: &[u8]) -> Option<ReexportEntry> {
    // A damaged declaration is reported as a limit, never as an entry.
    if node.has_error() || node.is_missing() {
        return None;
    }
    let mut cursor = node.walk();
    let target = node
        .named_children(&mut cursor)
        .find(|child| matches!(child.kind(), "identifier" | "qualified_identifier" | "namespace_identifier"))?;
    let text = compact(target, source);
    let span = text_span_for_node(target);
    let statement = text_span_for_node(node);

    let (form, caveats, exported, original, from) = if has_token(node, "namespace") {
        (ReexportForm::Wildcard, vec![ReexportCaveat::FallbackLookup], None, None, text)
    } else if has_token(node, "enum") {
        // `using enum ns::Color;` introduces every enumerator of the enum.
        (ReexportForm::Wildcard, Vec::new(), None, None, text)
    } else {
        let (qualifier, name) = text.rsplit_once("::")?;
        if name.is_empty() {
            return None;
        }
        let qualifier = if qualifier.is_empty() { "::" } else { qualifier };
        (
            ReexportForm::Named,
            Vec::new(),
            Some(name.to_owned()),
            Some(name.to_owned()),
            qualifier.to_owned(),
        )
    };

    let mut found = entry(form, ReexportEvidence::Syntax, span, statement);
    found.exported_name = exported;
    found.original_name = original;
    found.source = Some(from);
    found.scope = scope.map(str::to_owned);
    found.caveats = caveats;
    Some(found)
}

fn first_leaf(node: Node<'_>) -> Node<'_> {
    let mut current = node;
    while let Some(child) = current.child(0) {
        current = child;
    }
    current
}

fn is_module_word(text: &str) -> bool {
    matches!(text, "export" | "import" | "module")
}

fn starts_with_module_word(node: Node<'_>, source: &[u8]) -> bool {
    first_leaf(node).utf8_text(source).is_ok_and(is_module_word)
}

/// Whether a translation-unit-level statement (also through preprocessor
/// conditionals) starts with `export`, `import` or `module`. The grammar parses
/// these as a declaration whose type is that word, or as an error.
fn has_module_syntax(node: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if matches!(kind, "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif" | "preproc_elifdef") {
            if has_module_syntax(child, source) {
                return true;
            }
        } else if kind == "ERROR" {
            if error_has_module_word(child, source) {
                return true;
            }
        } else if child.is_named()
            && !matches!(kind, "comment" | "namespace_definition" | "linkage_specification")
            && starts_with_module_word(child, source)
        {
            return true;
        }
    }
    false
}

fn error_has_module_word(node: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    if starts_with_module_word(node, source) {
        return true;
    }
    children.into_iter().any(|child| {
        if child.child_count() == 0 {
            child.utf8_text(source).is_ok_and(|text| text == "export")
        } else {
            error_has_module_word(child, source)
        }
    })
}

/// Whether damaged syntax contains, follows, or sits inside a `using`
/// statement.
fn touches_using(damaged: Node<'_>, source: &[u8]) -> bool {
    let mut ancestor = damaged.parent();
    while let Some(parent) = ancestor {
        if parent.kind() == "using_declaration" {
            return true;
        }
        ancestor = parent.parent();
    }
    if contains_using(damaged, source) {
        return true;
    }
    previous_leaf(damaged).is_some_and(|leaf| leaf.utf8_text(source).is_ok_and(|text| text == "using"))
}

fn contains_using(node: Node<'_>, source: &[u8]) -> bool {
    if node.child_count() == 0 {
        return node.utf8_text(source).is_ok_and(|text| text == "using");
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| contains_using(child, source))
}

/// The last leaf token that ends before `node` starts.
fn previous_leaf(node: Node<'_>) -> Option<Node<'_>> {
    let mut current = node;
    loop {
        if let Some(sibling) = current.prev_sibling() {
            let mut leaf = sibling;
            loop {
                let mut cursor = leaf.walk();
                let last = leaf.children(&mut cursor).last();
                match last {
                    Some(last) => leaf = last,
                    None => break,
                }
            }
            return Some(leaf);
        }
        current = current.parent()?;
    }
}
