//! Rust binding rules (tree-sitter-rust 0.24).
//!
//! Items (`fn`, `struct`, `const`, `static`, `mod`, `use`) are visible
//! throughout their block; a `let` only after its statement, so the
//! initializer of `let f = f();` reads the outer `f`, and the latest visible
//! `let` of a block wins. Associated items (`impl` and `trait` members) are
//! never lexical names. Types, values and macros are separate categories; a
//! tuple or unit struct is both a type and a value.

use tree_sitter::Node;

use super::{ANY_CATEGORY, BindingRule, BindingSite, MACRO, Placement, TYPE, VALUE};

/// Scopes the vendored locals query lacks: an inline `mod` body, a trait,
/// and the declarations whose generic parameters would otherwise leak into
/// the enclosing scope.
pub(super) fn extra_scope(node: Node<'_>) -> bool {
    match node.kind() {
        "declaration_list" => node.parent().is_some_and(|parent| parent.kind() == "mod_item"),
        "trait_item" | "type_item" | "union_item" | "function_signature_item" => true,
        _ => false,
    }
}

const fn item(categories: u8, callable: bool) -> BindingRule {
    BindingRule {
        categories,
        placement: Placement::Innermost,
        visible_from: 0,
        ordered: false,
        callable,
    }
}

/// An `impl` or `trait` member: reached only through a path or a receiver.
fn is_associated(node: Node<'_>) -> bool {
    node.parent().is_some_and(|list| {
        list.kind() == "declaration_list"
            && list
                .parent()
                .is_some_and(|owner| matches!(owner.kind(), "impl_item" | "trait_item"))
    })
}

pub(super) fn binding_sites<'tree>(node: Node<'tree>, source: &str, sites: &mut Vec<BindingSite<'tree>>) {
    let mut push = |name: Option<Node<'tree>>, rule: BindingRule| {
        if let Some(name) = name {
            sites.push(BindingSite { name, rule, is_import: false });
        }
    };
    let name = node.child_by_field_name("name");
    match node.kind() {
        "function_item" | "function_signature_item" if !is_associated(node) => push(name, item(VALUE, true)),
        "const_item" | "static_item" if !is_associated(node) => push(name, item(VALUE, false)),
        "struct_item" => {
            // A tuple or unit struct also names its constructor.
            let constructs = node
                .child_by_field_name("body")
                .is_none_or(|body| body.kind() != "field_declaration_list");
            let categories = if constructs { TYPE | VALUE } else { TYPE };
            push(name, item(categories, constructs));
        }
        "union_item" | "enum_item" | "trait_item" | "mod_item" => push(name, item(TYPE, false)),
        "type_item" if !is_associated(node) => push(name, item(TYPE, false)),
        "type_parameter" => push(name, item(TYPE, false)),
        "const_parameter" => push(name, item(VALUE, false)),
        "macro_definition" => push(
            name,
            BindingRule {
                visible_from: node.end_byte(),
                ordered: true,
                ..item(MACRO, false)
            },
        ),
        "let_declaration" => {
            let rule = BindingRule {
                visible_from: node.end_byte(),
                ordered: true,
                ..item(VALUE, false)
            };
            pattern_names(node.child_by_field_name("pattern"), false, source, &mut |name| push(Some(name), rule));
        }
        "parameter" => {
            pattern_names(node.child_by_field_name("pattern"), false, source, &mut |name| {
                push(Some(name), item(VALUE, false));
            });
        }
        "closure_parameters" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() != "parameter" {
                    pattern_names(Some(child), false, source, &mut |name| push(Some(name), item(VALUE, false)));
                }
            }
        }
        "for_expression" => {
            let visible_from = node
                .child_by_field_name("body")
                .map_or(node.end_byte(), |body| body.start_byte());
            let rule = BindingRule { visible_from, ..item(VALUE, false) };
            pattern_names(node.child_by_field_name("pattern"), false, source, &mut |name| push(Some(name), rule));
        }
        "match_pattern" => {
            // The guard (`condition`) reads the bindings; it binds none.
            let pattern = node.named_child(0).filter(|pattern| {
                node.child_by_field_name("condition")
                    .is_none_or(|condition| condition.id() != pattern.id())
            });
            pattern_names(pattern, true, source, &mut |name| push(Some(name), item(VALUE, false)));
        }
        "let_condition" => {
            let rule = BindingRule {
                visible_from: node.end_byte(),
                ..item(VALUE, false)
            };
            pattern_names(node.child_by_field_name("pattern"), true, source, &mut |name| push(Some(name), rule));
        }
        "use_declaration" => {
            use_names(node.child_by_field_name("argument"), &mut |name| {
                sites.push(BindingSite { name, rule: item(ANY_CATEGORY, true), is_import: true });
            });
        }
        "extern_crate_declaration" => {
            let bound = node.child_by_field_name("alias").or(name);
            if let Some(name) = bound {
                sites.push(BindingSite { name, rule: item(ANY_CATEGORY, true), is_import: true });
            }
        }
        _ => {}
    }
}

/// The names a `use` tree binds: a path's last segment, an alias, every
/// entry of a list. `self` and globs bind no name here.
fn use_names<'tree>(node: Option<Node<'tree>>, bind: &mut impl FnMut(Node<'tree>)) {
    let Some(node) = node else { return };
    match node.kind() {
        "identifier" => bind(node),
        "scoped_identifier" => {
            if let Some(name) = node.child_by_field_name("name")
                && name.kind() == "identifier"
            {
                bind(name);
            }
        }
        "use_as_clause" => {
            if let Some(alias) = node.child_by_field_name("alias") {
                bind(alias);
            }
        }
        "scoped_use_list" => use_names(node.child_by_field_name("list"), bind),
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                use_names(Some(child), bind);
            }
        }
        _ => {}
    }
}

/// The identifiers a pattern binds. In a refutable pattern (`match` arm,
/// `if let`) a bare capitalized identifier is read as a constant or unit
/// variant, as rustc does for names in scope, and binds nothing.
fn pattern_names<'tree>(
    pattern: Option<Node<'tree>>,
    refutable: bool,
    source: &str,
    bind: &mut impl FnMut(Node<'tree>),
) {
    let Some(pattern) = pattern else { return };
    let mut stack = vec![pattern];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "identifier" => {
                let capitalized = source
                    .get(node.start_byte()..node.end_byte())
                    .and_then(|text| text.chars().next())
                    .is_some_and(char::is_uppercase);
                if !(refutable && capitalized) {
                    bind(node);
                }
            }
            "shorthand_field_identifier" => bind(node),
            // Paths, types and literals name no binding.
            "scoped_identifier" | "scoped_type_identifier" | "type_identifier" | "generic_type"
            | "range_pattern" | "macro_invocation" | "field_identifier" => {}
            _ => {
                let path = match node.kind() {
                    "tuple_struct_pattern" | "struct_pattern" => node.child_by_field_name("type"),
                    _ => None,
                };
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if path.is_none_or(|path| path.id() != child.id()) {
                        stack.push(child);
                    }
                }
            }
        }
    }
}

/// A reference's categories: a macro name, a path's qualifier (a module or
/// type), a type, else a value.
pub(super) fn reference_category(node: Node<'_>) -> u8 {
    if node.kind() == "type_identifier" {
        return TYPE;
    }
    let Some(parent) = node.parent() else { return VALUE };
    let is_field = |field: &str| parent.child_by_field_name(field).is_some_and(|child| child.id() == node.id());
    match parent.kind() {
        "macro_invocation" if is_field("macro") => MACRO,
        "scoped_identifier" | "scoped_type_identifier" | "scoped_use_list" if is_field("path") => TYPE,
        _ => VALUE,
    }
}
