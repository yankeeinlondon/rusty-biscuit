//! Go re-exports: alias declarations (`type X = pkg.Y`) and package-level
//! value copies (`var X = pkg.Y`, `const X = pkg.Y`).
//!
//! The qualifier must name an import of the same file: by explicit alias, or
//! else by the last segment of the import path, which is a guess because the
//! package's own `package` clause is not visible here. Defined types,
//! embedding, wrappers and every kind of plain import are not forwarding.

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportCaveat, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimit,
    ReexportLimitReason, ReexportReport, TextSpan,
};

/// An import usable as a qualifier in this file.
struct FileImport {
    binding: String,
    path: String,
    /// The binding is the path's last segment, not a written alias.
    inferred: bool,
    import_span: Option<TextSpan>,
}

pub(super) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    let root = tree_file.root_node();
    let source = tree_file.source();
    let table = import_table(root, source, imports);
    let conditional = has_build_constraint(root, source);

    let mut entries = Vec::new();
    let mut cursor = root.walk();
    for declaration in root.children(&mut cursor) {
        match declaration.kind() {
            "type_declaration" => {
                for spec in spec_nodes(declaration, "type_alias") {
                    if let Some(found) = alias_entry(spec, declaration, source, &table) {
                        entries.push(found);
                    }
                }
            }
            "var_declaration" => {
                for spec in spec_nodes(declaration, "var_spec") {
                    value_copy_entries(spec, declaration, true, source, &table, &mut entries);
                }
            }
            "const_declaration" => {
                for spec in spec_nodes(declaration, "const_spec") {
                    value_copy_entries(spec, declaration, false, source, &table, &mut entries);
                }
            }
            _ => {}
        }
    }
    for found in &mut entries {
        found.conditional = conditional;
    }

    let limits = forwarding_damage(root);
    finish(entries, limits)
}

/// The `kind` specs of a declaration, looking through grouped lists.
fn spec_nodes<'tree>(declaration: Node<'tree>, kind: &str) -> Vec<Node<'tree>> {
    let mut found = Vec::new();
    let mut cursor = declaration.walk();
    for child in declaration.children(&mut cursor) {
        if child.kind() == kind {
            found.push(child);
        } else if child.kind().ends_with("_spec_list") {
            let mut inner = child.walk();
            found.extend(child.children(&mut inner).filter(|node| node.kind() == kind));
        }
    }
    found
}

fn text<'a>(node: Node<'_>, source: &'a str) -> Option<&'a str> {
    node.utf8_text(source.as_bytes()).ok()
}

/// Whether `name` starts with an uppercase letter, which is what makes a
/// package-level Go identifier exported.
fn is_exported(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// `type X = pkg.Y` or `type X[T any] = pkg.Y[T]`; any other aliased type
/// (pointer, map, local name) is not a forward of a named entity.
fn alias_entry(
    alias: Node<'_>,
    declaration: Node<'_>,
    source: &str,
    table: &[FileImport],
) -> Option<ReexportEntry> {
    let exported = text(alias.child_by_field_name("name")?, source)?;
    if !is_exported(exported) {
        return None;
    }
    let mut target = alias.child_by_field_name("type")?;
    if target.kind() == "generic_type" {
        target = target.child_by_field_name("type")?;
    }
    if target.kind() != "qualified_type" {
        return None;
    }
    let qualifier = text(target.child_by_field_name("package")?, source)?;
    let original = text(target.child_by_field_name("name")?, source)?;
    let import = table.iter().find(|import| import.binding == qualifier)?;
    let mut found = entry(
        ReexportForm::Named,
        ReexportEvidence::Syntax,
        text_span_for_node(alias),
        text_span_for_node(declaration),
    );
    found.exported_name = Some(exported.to_owned());
    found.original_name = Some(original.to_owned());
    found.local_name = Some(import.binding.clone());
    found.source = Some(import.path.clone());
    found.import_span = import.import_span.clone();
    if import.inferred {
        found.caveats.push(ReexportCaveat::InferredPackageName);
    }
    Some(found)
}

/// Names of a `var`/`const` spec paired by position with `pkg.Member`
/// initializers.
fn value_copy_entries(
    spec: Node<'_>,
    declaration: Node<'_>,
    mutable: bool,
    source: &str,
    table: &[FileImport],
    entries: &mut Vec<ReexportEntry>,
) {
    let mut cursor = spec.walk();
    let names: Vec<Node<'_>> = spec.children_by_field_name("name", &mut cursor).filter(Node::is_named).collect();
    let Some(values) = spec.child_by_field_name("value") else { return };
    let mut cursor = values.walk();
    let values: Vec<Node<'_>> = values.named_children(&mut cursor).filter(|node| node.kind() != "comment").collect();
    if names.len() != values.len() {
        return;
    }
    for (name, value) in names.into_iter().zip(values) {
        let Some(exported) = text(name, source).filter(|name| is_exported(name)) else { continue };
        if value.kind() != "selector_expression" {
            continue;
        }
        let (Some(operand), Some(field)) = (value.child_by_field_name("operand"), value.child_by_field_name("field"))
        else {
            continue;
        };
        if operand.kind() != "identifier" {
            continue;
        }
        let (Some(qualifier), Some(original)) = (text(operand, source), text(field, source)) else { continue };
        let Some(import) = table.iter().find(|import| import.binding == qualifier) else { continue };
        let mut found = entry(
            ReexportForm::ValueCopy,
            ReexportEvidence::Declared,
            text_span_for_node(name_clause(spec, name)),
            text_span_for_node(declaration),
        );
        found.exported_name = Some(exported.to_owned());
        found.original_name = Some(original.to_owned());
        found.local_name = Some(import.binding.clone());
        found.source = Some(import.path.clone());
        found.import_span = import.import_span.clone();
        if mutable {
            found.caveats.push(ReexportCaveat::MutableBinding);
        }
        if import.inferred {
            found.caveats.push(ReexportCaveat::InferredPackageName);
        }
        entries.push(found);
    }
}

/// The node an entry's `span` covers: the whole spec when it declares one
/// name, else the declared name (specs with several names would otherwise
/// share one span and collapse).
fn name_clause<'tree>(spec: Node<'tree>, name: Node<'tree>) -> Node<'tree> {
    let mut cursor = spec.walk();
    if spec.children_by_field_name("name", &mut cursor).filter(Node::is_named).count() == 1 { spec } else { name }
}

/// Every named, aliased or path-named import of the file; blank and dot
/// imports bind no usable qualifier.
fn import_table(root: Node<'_>, source: &str, imports: &[ImportRecord]) -> Vec<FileImport> {
    let mut table = Vec::new();
    let mut cursor = root.walk();
    for declaration in root.children(&mut cursor).filter(|node| node.kind() == "import_declaration") {
        let mut specs = Vec::new();
        let mut inner = declaration.walk();
        for child in declaration.children(&mut inner) {
            match child.kind() {
                "import_spec" => specs.push(child),
                "import_spec_list" => {
                    let mut listed = child.walk();
                    specs.extend(child.children(&mut listed).filter(|node| node.kind() == "import_spec"));
                }
                _ => {}
            }
        }
        for spec in specs {
            let Some(path_node) = spec.child_by_field_name("path") else { continue };
            let Some(path) = text(path_node, source).map(|raw| raw.trim_matches(|c| c == '"' || c == '`')) else {
                continue;
            };
            let (binding, inferred, key) = match spec.child_by_field_name("name") {
                Some(name) if matches!(name.kind(), "blank_identifier" | "dot") => continue,
                Some(name) => (text(name, source).unwrap_or_default().to_owned(), false, name),
                None => (path.rsplit('/').next().unwrap_or(path).to_owned(), true, path_node),
            };
            if binding.is_empty() {
                continue;
            }
            let key = text_span_for_node(key);
            let import_span = imports.iter().find(|record| record.span == key).map(|record| record.span.clone());
            table.push(FileImport { binding, path: path.to_owned(), inferred, import_span });
        }
    }
    table
}

/// Whether a `//go:build` or `// +build` line comment precedes `package`.
fn has_build_constraint(root: Node<'_>, source: &str) -> bool {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "comment" => {
                let comment = text(child, source).unwrap_or_default();
                if comment.starts_with("//go:build ") || comment.starts_with("// +build ") {
                    return true;
                }
            }
            "package_clause" => return false,
            _ => {}
        }
    }
    false
}

/// Declaration keywords whose damage may hide a forward.
fn is_trigger(kind: &str) -> bool {
    matches!(kind, "type" | "var" | "const")
}

/// `parse_error` limits for damage that contains or directly follows a
/// package-level declaration keyword.
fn forwarding_damage(root: Node<'_>) -> Vec<ReexportLimit> {
    damaged_nodes(root)
        .into_iter()
        .filter(|node| !inside_function_body(*node))
        .filter(|node| {
            contains_trigger(*node)
                || in_declaration(*node)
                || previous_leaf(*node).is_some_and(|leaf| is_trigger(leaf.kind()))
        })
        .map(|node| limit(ReexportLimitReason::ParseError, node))
        .collect()
}

/// Damage nested in a package-level `type`, `var` or `const` declaration.
fn in_declaration(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if matches!(ancestor.kind(), "type_declaration" | "var_declaration" | "const_declaration") {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}

fn inside_function_body(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if matches!(ancestor.kind(), "block" | "function_declaration" | "method_declaration" | "func_literal") {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}

fn contains_trigger(node: Node<'_>) -> bool {
    if node.child_count() == 0 {
        return is_trigger(node.kind());
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).any(contains_trigger)
}

/// The last leaf token that ends before `node` starts.
fn previous_leaf(node: Node<'_>) -> Option<Node<'_>> {
    let mut current = node;
    loop {
        if let Some(mut sibling) = current.prev_sibling() {
            while sibling.child_count() > 0 {
                sibling = sibling.child(u32::try_from(sibling.child_count() - 1).ok()?)?;
            }
            return Some(sibling);
        }
        current = current.parent()?;
    }
}
