//! Presentation scope (`Item`, `Member`, `Local`) of a declaration, decided
//! once at extraction from its node. The rules were fixed by a reviewed dump
//! of every fixture declaration's scope (`2026-10-03-symbol-filters-and-reports`).

use super::*;
use crate::file::parents::IndexedParent as _;
use crate::shared::SymbolScope;

/// The scope of the declaration `name` belongs to.
///
/// Walking out from the declaration's member slot, the first function body
/// (or non-function local body, [`is_local_scope_kind`]) makes it `Local`,
/// the first type, trait, interface, object or `impl` makes it `Member`, and
/// reaching the file root makes it `Item`. Bodies are walked past, so a body
/// shared by modules and types (Rust and C# `declaration_list`) is decided by
/// the declaration that owns it: a module or namespace body keeps walking, an
/// `impl`, trait or class body ends at its owner.
pub(crate) fn symbol_scope(
    declaration: Node<'_>,
    name: Node<'_>,
    kind: SymbolKind,
    language: ProgrammingLanguage,
) -> SymbolScope {
    // A C++ template type parameter is captured as a Type; like a function
    // parameter it names nothing outside its declaration.
    if kind == SymbolKind::Parameter || is_type_parameter_kind(declaration.kind()) {
        return SymbolScope::Local;
    }
    // A Go method is declared at file level but names its receiver type; a
    // Lua `function M:m()` / `function M.f()` / `M.x = …` names its table.
    // Both are members of what they name, wherever they are written.
    if language == ProgrammingLanguage::Go && member_slot(declaration).kind() == "method_declaration" {
        return SymbolScope::Member;
    }
    if language == ProgrammingLanguage::Lua && lua_table_of_name(name).is_some() {
        return SymbolScope::Member;
    }

    let mut current = member_slot(declaration).indexed_parent();
    while let Some(node) = current {
        let node_kind = node.kind();
        if is_function_node_kind(node_kind) || is_local_scope_kind(node_kind, language) {
            return SymbolScope::Local;
        }
        // An unnamed owner (an anonymous class, object literal or class
        // expression) owns its members as a named type does.
        if is_scope_owner_kind(node_kind) || is_unnamed_type_owner(node) {
            return SymbolScope::Member;
        }
        current = node.indexed_parent();
    }
    SymbolScope::Item
}

/// Declarations whose bodies hold members: the member owners the extractor
/// knows minus the bodies (decided by their parent), plus owners with no
/// member-kind captures: C/C++ `enum_specifier`, Go struct and interface
/// types and the Java annotation type.
fn is_scope_owner_kind(kind: &str) -> bool {
    let is_body = kind.ends_with("_body")
        || matches!(kind, "block" | "declaration_list" | "field_declaration_list" | "template_body");
    (is_member_owner_kind(kind) && !is_body)
        || matches!(
            kind,
            "enum_specifier" | "struct_type" | "interface_type" | "annotation_type_declaration"
        )
}

fn is_type_parameter_kind(kind: &str) -> bool {
    matches!(
        kind,
        "type_parameter_declaration"
            | "optional_type_parameter_declaration"
            | "variadic_type_parameter_declaration"
            | "template_template_parameter_declaration"
    )
}

/// For a Lua name written on its table (`M:m`, `M.f`, `M.x`), the table
/// expression and whether the name is a method (`:`).
pub(crate) fn lua_table_of_name(name: Node<'_>) -> Option<(Node<'_>, bool)> {
    let parent = name.indexed_parent()?;
    let is_method = match parent.kind() {
        "method_index_expression" => true,
        "dot_index_expression" => false,
        _ => return None,
    };
    let field = if is_method { "method" } else { "field" };
    if parent.child_by_field_name(field)? != name {
        return None;
    }
    Some((parent.child_by_field_name("table")?, is_method))
}
