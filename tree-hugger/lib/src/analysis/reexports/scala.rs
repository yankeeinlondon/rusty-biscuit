//! Scala 3 re-exports: `export` clauses.
//!
//! Scala 2 has no `export` construct, so a file without one reports nothing
//! and `complete`. An `export` the grammar cannot read faithfully (modified,
//! annotated, damaged, or several clauses flattened into one) is never read
//! as a plain public entry; it limits coverage instead.

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportCaveat, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimit,
    ReexportLimitReason, ReexportReport,
};

pub(super) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    let root = tree_file.root_node();
    let source = tree_file.source();
    let mut entries = Vec::new();
    let mut limits = Vec::new();

    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "export_declaration" {
            export_entries(node, source, imports, &mut entries, &mut limits);
            continue;
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }

    for damaged in damaged_nodes(root) {
        if touches_export(damaged, source) {
            limits.push(limit(ReexportLimitReason::ParseError, damaged));
        }
    }
    finish(entries, limits)
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or_default()
}

/// Backticks quote a name but are not part of it.
fn bare(name: &str) -> String {
    name.trim().trim_matches('`').to_owned()
}

/// Whether `word` occurs in `haystack` as a whole identifier.
fn contains_word(haystack: &str, word: &str) -> bool {
    haystack.match_indices(word).any(|(at, _)| {
        let before = haystack[..at].chars().next_back();
        let after = haystack[at + word.len()..].chars().next();
        let part_of_name = |c: char| c.is_alphanumeric() || c == '_';
        !before.is_some_and(part_of_name) && !after.is_some_and(part_of_name)
    })
}

/// Whether the region holds or abuts an `export`: it contains the word,
/// follows it, is inside an `export` clause, or directly precedes one (the
/// recovered form of a modified or annotated `export`).
fn touches_export(damaged: Node<'_>, source: &str) -> bool {
    if contains_word(text(damaged, source), "export") || follows_export(damaged, source) {
        return true;
    }
    if precedes_export(damaged) {
        return true;
    }
    let mut ancestor = damaged.parent();
    while let Some(node) = ancestor {
        if node.kind() == "export_declaration" {
            return true;
        }
        ancestor = node.parent();
    }
    false
}

fn follows_export(damaged: Node<'_>, source: &str) -> bool {
    let before = source[..damaged.start_byte()].trim_end();
    before.rsplit(|c: char| !(c.is_alphanumeric() || c == '_')).next() == Some("export")
}

/// An `export_declaration` that starts after only whitespace past `damaged`.
fn precedes_export(damaged: Node<'_>) -> bool {
    damaged.next_sibling().is_some_and(|next| next.kind() == "export_declaration")
}

fn export_entries(
    declaration: Node<'_>,
    source: &str,
    imports: &[ImportRecord],
    entries: &mut Vec<ReexportEntry>,
    limits: &mut Vec<ReexportLimit>,
) {
    // A damaged clause, or one right after damage, may have lost a modifier
    // (`private export` parses as an ERROR plus a plain-looking export).
    let after_damage = declaration.prev_sibling().is_some_and(|previous| previous.is_error());
    if declaration.has_error() || after_damage {
        return;
    }
    let mut cursor = declaration.walk();
    let children: Vec<Node<'_>> = declaration.children(&mut cursor).filter(|c| c.kind() != "export").collect();
    // The grammar concatenates `export a.b, c.d` into one node; the clause
    // boundary is lost, so no entry can be trusted.
    if children.iter().any(|child| child.kind() == ",") {
        limits.push(limit(ReexportLimitReason::GrammarGap, declaration));
        return;
    }
    let Some(last) = children.last().copied() else { return };
    let qualifier: Vec<Node<'_>> = children[..children.len() - 1]
        .iter()
        .copied()
        .filter(|child| child.is_named())
        .collect();
    let path = qualifier.iter().map(|part| text(*part, source).trim()).collect::<Vec<_>>().join(".");
    let scope = enclosing_scope(declaration, source);
    let statement = text_span_for_node(declaration);

    let base = |form, span_node: Node<'_>| {
        let mut forwarded = entry(form, ReexportEvidence::Syntax, text_span_for_node(span_node), statement.clone());
        forwarded.source = (!path.is_empty()).then(|| path.clone());
        forwarded.scope = scope.clone();
        link_import(&mut forwarded, &qualifier, source, imports);
        forwarded
    };

    match last.kind() {
        "namespace_wildcard" => entries.push(wildcard(&base, last, source, &[])),
        "namespace_selectors" => selector_entries(last, source, &base, entries),
        "identifier" => {
            let name = bare(text(last, source));
            if qualifier.is_empty() {
                return;
            }
            let mut named = base(ReexportForm::Named, last);
            named.exported_name = Some(name.clone());
            named.original_name = Some(name);
            entries.push(named);
        }
        _ => {}
    }
}

/// A wildcard entry for a `*`, `_` or `given` selector.
fn wildcard(
    base: &impl Fn(ReexportForm, Node<'_>) -> ReexportEntry,
    selector: Node<'_>,
    source: &str,
    excluded: &[String],
) -> ReexportEntry {
    let mut forwarded = base(ReexportForm::Wildcard, selector);
    if text(selector, source).trim() == "given" {
        forwarded.caveats = vec![ReexportCaveat::GivensOnly];
    } else {
        forwarded.excluded_names = excluded.to_vec();
    }
    forwarded
}

fn selector_entries(
    selectors: Node<'_>,
    source: &str,
    base: &impl Fn(ReexportForm, Node<'_>) -> ReexportEntry,
    entries: &mut Vec<ReexportEntry>,
) {
    let mut cursor = selectors.walk();
    let items: Vec<Node<'_>> = selectors.named_children(&mut cursor).collect();

    // `x as _` removes `x` from the `*` wildcard of the same clause.
    let excluded: Vec<String> = items
        .iter()
        .filter_map(|item| renamed_parts(*item).filter(|(_, alias)| alias.kind() == "wildcard"))
        .map(|(name, _)| bare(text(name, source)))
        .collect();

    for item in items {
        match item.kind() {
            "identifier" => {
                let name = bare(text(item, source));
                let mut named = base(ReexportForm::Named, item);
                named.exported_name = Some(name.clone());
                named.original_name = Some(name);
                entries.push(named);
            }
            // `given T`: the grammar reads the type as a `type_identifier`.
            "type_identifier" => {
                let mut givens = base(ReexportForm::Wildcard, item);
                givens.caveats = vec![ReexportCaveat::GivensOnly];
                entries.push(givens);
            }
            "namespace_wildcard" => entries.push(wildcard(base, item, source, &excluded)),
            "as_renamed_identifier" | "arrow_renamed_identifier" => {
                let Some((name, alias)) = renamed_parts(item) else { continue };
                if alias.kind() == "wildcard" {
                    continue;
                }
                let mut named = base(ReexportForm::Named, item);
                named.exported_name = Some(bare(text(alias, source)));
                named.original_name = Some(bare(text(name, source)));
                entries.push(named);
            }
            _ => {}
        }
    }
}

/// The `(name, alias)` nodes of a rename selector.
fn renamed_parts(item: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    if !matches!(item.kind(), "as_renamed_identifier" | "arrow_renamed_identifier") {
        return None;
    }
    let mut cursor = item.walk();
    let mut named = item.named_children(&mut cursor);
    let name = item.child_by_field_name("name").or_else(|| named.next())?;
    let last_index = u32::try_from(item.named_child_count().checked_sub(1)?).ok()?;
    let alias = item.child_by_field_name("alias").or_else(|| item.named_child(last_index))?;
    (name.id() != alias.id()).then_some((name, alias))
}

/// Records the same-file `import` the export's qualifier goes through, when
/// the qualifier is one name bound by an import declaration that is in scope
/// at the export: it precedes the export in the same or an enclosing
/// template body or block, the nearest such import wins, and no definition or
/// class parameter of that name sits between them (a definition takes
/// precedence over an import).
fn link_import(forwarded: &mut ReexportEntry, qualifier: &[Node<'_>], source: &str, imports: &[ImportRecord]) {
    let [only] = qualifier else { return };
    let used = text(*only, source).trim();
    let Some(declaration) = only.parent() else { return };
    let root = root_of(declaration);
    // `(depth of the import's scope, start)` of the best candidate so far.
    let mut nearest: Option<((usize, usize), &ImportRecord, Node<'_>)> = None;
    for import in imports {
        if import.alias.as_deref().unwrap_or(&import.name) != used {
            continue;
        }
        let Some(import_declaration) = root
            .descendant_for_byte_range(import.span.start_byte as usize, import.span.end_byte as usize)
            .and_then(|node| ancestor_of_kind(node, "import_declaration"))
        else {
            continue;
        };
        let Some(scope) = import_declaration.parent() else { continue };
        if import_declaration.end_byte() > declaration.start_byte() {
            continue;
        }
        let Some(depth) = ancestor_depth(declaration, scope) else { continue };
        // A smaller depth is a nearer scope; within one scope the later
        // import wins.
        let key = (usize::MAX - depth, import_declaration.start_byte());
        if nearest.as_ref().is_none_or(|(best, _, _)| key > *best) {
            nearest = Some((key, import, scope));
        }
    }
    let Some((_, import, scope)) = nearest else { return };
    if defined_between(declaration, scope, used, source) {
        return;
    }
    forwarded.local_name = Some(bare(used));
    forwarded.import_span = Some(import.span.clone());
}

/// How many parents up from `node` `ancestor` is, if it is an ancestor.
fn ancestor_depth(node: Node<'_>, ancestor: Node<'_>) -> Option<usize> {
    let mut current = node;
    let mut depth = 0;
    while let Some(parent) = current.parent() {
        depth += 1;
        if parent.id() == ancestor.id() {
            return Some(depth);
        }
        current = parent;
    }
    None
}

/// Whether a scope from `node` up to and including `scope` directly defines
/// `name`: a member or local definition, or a class parameter.
fn defined_between(node: Node<'_>, scope: Node<'_>, name: &str, source: &str) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        let mut cursor = parent.walk();
        let found = parent.named_children(&mut cursor).any(|child| match child.kind() {
            "class_parameters" => {
                let mut inner = child.walk();
                child
                    .named_children(&mut inner)
                    .any(|parameter| definition_name(parameter, source).as_deref() == Some(bare(name).as_str()))
            }
            _ => defines(child, name, source),
        });
        if found {
            return true;
        }
        if parent.id() == scope.id() {
            return false;
        }
        current = parent;
    }
    false
}

/// Whether the statement `child` is a definition binding `name`.
fn defines(child: Node<'_>, name: &str, source: &str) -> bool {
    match child.kind() {
        "val_definition" | "var_definition" | "val_declaration" | "var_declaration" => child
            .child_by_field_name("pattern")
            .or_else(|| child.child_by_field_name("name"))
            .is_some_and(|pattern| pattern_binds(pattern, name, source)),
        // A class or trait name is a type, not the term an export path
        // starts with; an enum also defines its companion term.
        "function_definition" | "function_declaration" | "object_definition" | "enum_definition"
        | "given_definition" | "package_object" => {
            definition_name(child, source).as_deref() == Some(bare(name).as_str())
        }
        _ => false,
    }
}

/// Whether a `val`/`var` pattern binds `name` as a plain identifier.
fn pattern_binds(pattern: Node<'_>, name: &str, source: &str) -> bool {
    match pattern.kind() {
        "identifier" => bare(text(pattern, source)) == bare(name),
        "identifiers" | "tuple_pattern" => {
            let mut cursor = pattern.walk();
            pattern.named_children(&mut cursor).any(|part| pattern_binds(part, name, source))
        }
        _ => false,
    }
}

fn root_of(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node.parent() {
        node = parent;
    }
    node
}

fn ancestor_of_kind<'t>(mut node: Node<'t>, kind: &str) -> Option<Node<'t>> {
    loop {
        if node.kind() == kind {
            return Some(node);
        }
        node = node.parent()?;
    }
}

/// Qualified name of the packages and definitions around `node`; `None` at
/// file level with no `package`.
fn enclosing_scope(node: Node<'_>, source: &str) -> Option<String> {
    let mut names: Vec<String> = Vec::new();
    let mut current = node;
    while let Some(parent) = current.parent() {
        if matches!(
            parent.kind(),
            "object_definition" | "class_definition" | "trait_definition" | "enum_definition" | "package_object"
        ) && let Some(name) = definition_name(parent, source)
        {
            names.push(name);
        } else if parent.kind() == "package_clause" && let Some(name) = package_name(parent, source) {
            names.push(name);
        }
        current = parent;
    }
    // Brace-less `package p` lines scope everything after them in the file.
    let mut packages = Vec::new();
    if let Some(root) = Some(root_of(node)) {
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "package_clause"
                && child.end_byte() <= node.start_byte()
                && child.child_by_field_name("body").is_none()
                && !inside_range(node, child)
                && let Some(name) = package_name(child, source)
            {
                packages.push(name);
            }
        }
    }
    names.reverse();
    packages.extend(names);
    (!packages.is_empty()).then(|| packages.join("."))
}

fn inside_range(inner: Node<'_>, outer: Node<'_>) -> bool {
    inner.start_byte() >= outer.start_byte() && inner.end_byte() <= outer.end_byte()
}

fn definition_name(definition: Node<'_>, source: &str) -> Option<String> {
    let name = definition.child_by_field_name("name").or_else(|| {
        let mut cursor = definition.walk();
        definition.children(&mut cursor).find(|child| child.kind() == "identifier")
    })?;
    Some(bare(text(name, source)))
}

fn package_name(clause: Node<'_>, source: &str) -> Option<String> {
    let name = clause.child_by_field_name("name").or_else(|| {
        let mut cursor = clause.walk();
        clause.children(&mut cursor).find(|child| child.kind() == "package_identifier")
    })?;
    Some(text(name, source).split_whitespace().collect())
}
