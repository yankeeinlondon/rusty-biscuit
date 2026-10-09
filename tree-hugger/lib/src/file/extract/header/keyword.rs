//! The declaring keyword of a declaration: the grammar token that introduces
//! the construct (`fn`, `struct`, `def`, `const`, `my`, ...).
//!
//! Each language maps a declaration node kind to where that token sits: a
//! direct child of the declaration, the token inside a named wrapper child
//! (Perl `scope`, Swift `value_binding_pattern`), a marker node that stands
//! for the keyword (PHP `var_modifier`), or a
//! direct child of the statement that wraps the declaration (Go `const`/`var`
//! specs, Scala enum cases, Lua and shell `local`). Only anonymous tokens are read, so a comment
//! that names a keyword can never supply one, and the token's kind is the
//! keyword, whatever its spelling in the source (PHP keywords are
//! case-insensitive).

use super::super::*;
use crate::file::parents::IndexedParent as _;

/// Where a declaration kind's keyword token sits.
enum KeywordSite {
    /// A direct anonymous child of the declaration, one of these tokens.
    Child(&'static [&'static str]),
    /// The anonymous token inside the declaration's first child of this kind.
    Wrapped(&'static str),
    /// The anonymous token of the declaration's child with this field name.
    Field(&'static str),
    /// The anonymous token inside the child of `kind` of the declaration's
    /// `field` child.
    WrappedInField { field: &'static str, kind: &'static str },
    /// A named leaf child of this kind that stands for the keyword (PHP
    /// `var_modifier` has no token inside it).
    Marker { kind: &'static str, keyword: &'static str },
    /// A direct anonymous child of the enclosing statement, which must be of
    /// one of `parents`, looking through one list node of `list` kind.
    Parent {
        parents: &'static [&'static str],
        list: Option<&'static str>,
        tokens: &'static [&'static str],
    },
    /// The construct has no declaring keyword.
    None,
}

/// The keyword that introduces `declaration`, read from its syntax tree.
pub(crate) fn declaration_keyword(declaration: Node<'_>, language: ProgrammingLanguage) -> Option<String> {
    let token = match keyword_site(declaration, language) {
        KeywordSite::Child(tokens) => child_token(declaration, tokens),
        KeywordSite::Wrapped(kind) => find_child_by_kind(declaration, kind).and_then(first_token),
        KeywordSite::Field(field) => declaration.child_by_field_name(field).and_then(first_token),
        KeywordSite::WrappedInField { field, kind } => declaration
            .child_by_field_name(field)
            .and_then(|child| find_child_by_kind(child, kind))
            .and_then(first_token),
        KeywordSite::Marker { kind, keyword } => {
            return find_child_by_kind(declaration, kind).map(|_| keyword.to_string());
        }
        KeywordSite::Parent { parents, list, tokens } => declaration
            .indexed_parent()
            .and_then(|parent| match list {
                Some(list) if parent.kind() == list => parent.indexed_parent(),
                _ => Some(parent),
            })
            .filter(|statement| parents.contains(&statement.kind()))
            .and_then(|statement| child_token(statement, tokens)),
        KeywordSite::None => None,
    };
    token.map(|token| token.kind().to_string())
}

fn keyword_site(declaration: Node<'_>, language: ProgrammingLanguage) -> KeywordSite {
    use KeywordSite::{Child, Field, Parent, Wrapped, WrappedInField};
    use ProgrammingLanguage as L;

    let kind = declaration.kind();
    match (language, kind) {
        (L::Rust, "function_item" | "function_signature_item") => Child(&["fn"]),
        (L::Rust, "struct_item") => Child(&["struct"]),
        (L::Rust, "union_item") => Child(&["union"]),
        (L::Rust, "enum_item") => Child(&["enum"]),
        (L::Rust, "trait_item") => Child(&["trait"]),
        (L::Rust, "type_item" | "associated_type") => Child(&["type"]),
        (L::Rust, "mod_item") => Child(&["mod"]),
        (L::Rust, "const_item") => Child(&["const"]),
        (L::Rust, "static_item") => Child(&["static"]),
        (L::Rust, "let_declaration") => Child(&["let"]),
        (L::Rust, "macro_definition") => Child(&["macro_rules!"]),

        (L::Go, "function_declaration" | "method_declaration") => Child(&["func"]),
        (L::Go, "type_declaration") => Child(&["type"]),
        (L::Go, "package_clause") => Child(&["package"]),
        (L::Go, "type_spec" | "type_alias") => Parent {
            parents: &["type_declaration"],
            list: None,
            tokens: &["type"],
        },
        (L::Go, "const_spec") => Parent {
            parents: &["const_declaration"],
            list: None,
            tokens: &["const"],
        },
        (L::Go, "var_spec") => Parent {
            parents: &["var_declaration"],
            list: Some("var_spec_list"),
            tokens: &["var"],
        },

        // `declaration_kind` holds `class`, `struct`, `enum`, `actor` or
        // `protocol`.
        (L::Swift, "class_declaration" | "protocol_declaration") => Field("declaration_kind"),
        (L::Swift, "function_declaration" | "protocol_function_declaration") => Child(&["func"]),
        (L::Swift, "enum_entry") => Child(&["case"]),
        (L::Swift, "init_declaration") => Child(&["init"]),
        (L::Swift, "typealias_declaration") => Child(&["typealias"]),
        (L::Swift, "property_declaration") => Wrapped("value_binding_pattern"),
        // The binding sits inside the name pattern (`var name: T { get }`).
        (L::Swift, "protocol_property_declaration") => WrappedInField {
            field: "name",
            kind: "value_binding_pattern",
        },

        (L::Scala, "function_definition" | "function_declaration") => Child(&["def"]),
        (L::Scala, "val_definition" | "val_declaration") => Child(&["val"]),
        (L::Scala, "var_definition" | "var_declaration") => Child(&["var"]),
        (L::Scala, "class_definition") => Child(&["class"]),
        (L::Scala, "object_definition") => Child(&["object"]),
        (L::Scala, "trait_definition") => Child(&["trait"]),
        (L::Scala, "enum_definition") => Child(&["enum"]),
        (L::Scala, "class_parameter") => Child(&["val", "var"]),
        (L::Scala, "type_definition") => Child(&["type"]),
        // `case Red, Blue` declares each name in its own node.
        (L::Scala, "simple_enum_case" | "full_enum_case") => Parent {
            parents: &["enum_case_definitions"],
            list: None,
            tokens: &["case"],
        },

        (L::TypeScript | L::JavaScript, "function_declaration" | "function_signature") => Child(&["function"]),
        (L::TypeScript | L::JavaScript, "class_declaration" | "abstract_class_declaration") => Child(&["class"]),
        (L::TypeScript, "interface_declaration") => Child(&["interface"]),
        (L::TypeScript, "enum_declaration") => Child(&["enum"]),
        (L::TypeScript, "type_alias_declaration") => Child(&["type"]),
        (L::TypeScript, "internal_module") => Child(&["namespace"]),
        (L::TypeScript, "module") => Child(&["module"]),
        (L::TypeScript | L::JavaScript, "lexical_declaration") => Field("kind"),
        (L::TypeScript | L::JavaScript, "variable_declaration") => Child(&["var"]),

        (L::Java, "class_declaration") => Child(&["class"]),
        (L::Java, "interface_declaration") => Child(&["interface"]),
        (L::Java, "enum_declaration") => Child(&["enum"]),
        (L::Java, "record_declaration") => Child(&["record"]),
        (L::Java, "package_declaration") => Child(&["package"]),

        (L::CSharp, "class_declaration") => Child(&["class"]),
        (L::CSharp, "struct_declaration") => Child(&["struct"]),
        (L::CSharp, "interface_declaration") => Child(&["interface"]),
        (L::CSharp, "enum_declaration") => Child(&["enum"]),
        (L::CSharp, "record_declaration") => Child(&["record"]),
        (L::CSharp, "namespace_declaration" | "file_scoped_namespace_declaration") => Child(&["namespace"]),
        (L::CSharp, "event_field_declaration") => Child(&["event"]),

        (L::C | L::Cpp, "struct_specifier") => Child(&["struct"]),
        (L::C | L::Cpp, "union_specifier") => Child(&["union"]),
        (L::C | L::Cpp, "enum_specifier") => Child(&["enum"]),
        (L::C | L::Cpp, "type_definition") => Child(&["typedef"]),
        (L::C | L::Cpp, "preproc_def" | "preproc_function_def") => Child(&["#define"]),
        (L::Cpp, "class_specifier") => Child(&["class"]),
        (L::Cpp, "namespace_definition") => Child(&["namespace"]),
        (L::Cpp, "alias_declaration") => Child(&["using"]),
        (L::Cpp, "type_parameter_declaration") => Child(&["class", "typename"]),

        (L::Php, "function_definition" | "method_declaration") => Child(&["function"]),
        (L::Php, "class_declaration") => Child(&["class"]),
        (L::Php, "interface_declaration") => Child(&["interface"]),
        (L::Php, "trait_declaration") => Child(&["trait"]),
        (L::Php, "enum_declaration") => Child(&["enum"]),
        (L::Php, "const_declaration") => Child(&["const"]),
        (L::Php, "enum_case") => Child(&["case"]),
        (L::Php, "namespace_definition") => Child(&["namespace"]),
        // Only the legacy `var $name;` form has a keyword.
        (L::Php, "property_declaration") => KeywordSite::Marker {
            kind: "var_modifier",
            keyword: "var",
        },

        (L::Python, "function_definition") => Child(&["def"]),
        (L::Python, "class_definition") => Child(&["class"]),

        (L::Perl, "function_definition") => Child(&["sub"]),
        (L::Perl, "package_statement") => Child(&["package"]),
        (L::Perl, "variable_declaration") => Wrapped("scope"),

        (L::Lua, "function_declaration") => Child(&["function"]),
        (L::Lua, "assignment_statement") => Parent {
            parents: &["variable_declaration"],
            list: None,
            tokens: &["local"],
        },

        (L::Bash | L::Zsh, "function_definition") => Child(&["function"]),
        (L::Bash | L::Zsh, "variable_assignment") => Parent {
            parents: &["declaration_command"],
            list: None,
            tokens: &["local", "declare", "typeset", "export", "readonly", "integer", "float"],
        },

        _ => KeywordSite::None,
    }
}

/// The first direct anonymous child of `node` that is one of `tokens`.
fn child_token<'tree>(node: Node<'tree>, tokens: &[&str]) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| !child.is_named() && tokens.contains(&child.kind()))
}

/// `node` itself when it is a token, else its first anonymous descendant in
/// document order.
fn first_token(node: Node<'_>) -> Option<Node<'_>> {
    if !node.is_named() {
        return Some(node);
    }
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children.into_iter().find_map(first_token)
}
