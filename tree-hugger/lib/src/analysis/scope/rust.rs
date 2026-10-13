//! Rust binding rules (tree-sitter-rust 0.24).
//!
//! Items (`fn`, `struct`, `const`, `static`, `mod`, `use`) are visible
//! throughout their block; a `let` only after its statement, so the
//! initializer of `let f = f();` reads the outer `f`, and the latest visible
//! `let` of a block wins. Associated items (`impl` and `trait` members) are
//! never lexical names. Types, values and macros are separate categories; a
//! tuple or unit struct is both a type and a value.
//!
//! An item cannot capture: code in a nested `fn`, `const`, `static`, type,
//! trait or `impl` sees the items of the blocks around it, never their
//! `let`s, parameters, pattern bindings or generic parameters (closures
//! do capture). An inline `mod` sees nothing of its parent but
//! `macro_rules!` macros. A pattern binding of an `if let` (or a `let` in a
//! chain) is visible from its `let` to the end of the consequence, never in
//! the `else` branch.
//!
//! Whether a pattern name binds is never read from its spelling (naming
//! conventions are lints): `mut x`, `ref x` and `x @ …` always bind, and a
//! bare name is a [`BindingSite::bare_pattern`] that reference resolution
//! decides by lookup (`analysis::occurrences::bare_pattern_meaning`).

use tree_sitter::Node;

use super::{ANY_CATEGORY, BindingRule, BindingSite, Boundary, MACRO, Placement, Refutability, TYPE, VALUE};
use crate::file::parents::IndexedParent as _;

/// Scopes the vendored locals query lacks: an inline `mod` body, a trait,
/// the declarations whose generic parameters would otherwise leak into the
/// enclosing scope, and `const`/`static` items, whose initializers cannot
/// capture the locals around them.
pub(super) fn extra_scope(node: Node<'_>) -> bool {
    match node.kind() {
        "declaration_list" => node.indexed_parent().is_some_and(|parent| parent.kind() == "mod_item"),
        "trait_item" | "type_item" | "union_item" | "function_signature_item" | "const_item" | "static_item" => true,
        _ => false,
    }
}

/// What leaving `node` leaves behind: an inline `mod` its parent's names,
/// any other item that is not an associated item its enclosing locals (an
/// associated item still sees its `impl`'s or trait's generic parameters).
pub(super) fn boundary(node: Node<'_>) -> Boundary {
    match node.kind() {
        "mod_item" => Boundary::Module,
        "function_item" | "function_signature_item" | "const_item" | "static_item" | "type_item"
            if is_associated(node) =>
        {
            Boundary::None
        }
        "function_item" | "function_signature_item" | "const_item" | "static_item" | "type_item" | "struct_item"
        | "enum_item" | "union_item" | "trait_item" | "impl_item" => Boundary::Item,
        _ => Boundary::None,
    }
}

const fn item(categories: u8, callable: bool) -> BindingRule {
    BindingRule {
        categories,
        placement: Placement::Innermost,
        visible_from: 0,
        visible_until: None,
        local: false,
        ordered: false,
        callable,
        pattern_constant: false,
    }
}

/// A binding of an item's body or header, which nested items cannot see.
const fn local(categories: u8) -> BindingRule {
    BindingRule { local: true, ..item(categories, false) }
}

/// Where an `if let` condition's bindings stop being visible: the end of
/// the consequence (through a `let` chain). `None` elsewhere (`while let`,
/// a match guard), whose scope ends with the body anyway.
fn conditional_end(condition: Node<'_>) -> Option<usize> {
    let mut node = condition;
    while let Some(parent) = node.indexed_parent() {
        match parent.kind() {
            "let_chain" => node = parent,
            "if_expression" => {
                let in_condition = parent
                    .child_by_field_name("condition")
                    .is_some_and(|written| written.id() == node.id());
                return in_condition
                    .then(|| parent.child_by_field_name("consequence"))
                    .flatten()
                    .map(|consequence| consequence.end_byte());
            }
            _ => return None,
        }
    }
    None
}

/// An `impl` or `trait` member: reached only through a path or a receiver.
fn is_associated(node: Node<'_>) -> bool {
    node.indexed_parent().is_some_and(|list| {
        list.kind() == "declaration_list"
            && list
                .indexed_parent()
                .is_some_and(|owner| matches!(owner.kind(), "impl_item" | "trait_item"))
    })
}

pub(super) fn binding_sites<'tree>(node: Node<'tree>, sites: &mut Vec<BindingSite<'tree>>) {
    let mut push = |name: Option<Node<'tree>>, rule: BindingRule| {
        if let Some(name) = name {
            sites.push(BindingSite { name, rule, is_import: false, bare_pattern: None });
        }
    };
    let name = node.child_by_field_name("name");
    match node.kind() {
        "function_item" | "function_signature_item" if !is_associated(node) => push(name, item(VALUE, true)),
        "const_item" if !is_associated(node) => push(name, BindingRule { pattern_constant: true, ..item(VALUE, false) }),
        "static_item" if !is_associated(node) => push(name, item(VALUE, false)),
        "struct_item" => {
            // A tuple or unit struct also names its constructor; a bare
            // pattern naming a unit struct matches it.
            let body = node.child_by_field_name("body").map(|body| body.kind());
            let constructs = body != Some("field_declaration_list");
            let categories = if constructs { TYPE | VALUE } else { TYPE };
            push(name, BindingRule { pattern_constant: body.is_none(), ..item(categories, constructs) });
        }
        "union_item" | "enum_item" | "trait_item" | "mod_item" => push(name, item(TYPE, false)),
        "type_item" if !is_associated(node) => push(name, item(TYPE, false)),
        "type_parameter" => push(name, local(TYPE)),
        "const_parameter" => push(name, BindingRule { pattern_constant: true, ..local(VALUE) }),
        "macro_definition" => push(
            name,
            BindingRule {
                visible_from: node.end_byte(),
                ordered: true,
                ..item(MACRO, false)
            },
        ),
        _ => {}
    }
    match node.kind() {
        "let_declaration" => {
            let rule = BindingRule {
                visible_from: node.end_byte(),
                ordered: true,
                ..local(VALUE)
            };
            // `let mut x`: the grammar puts the `mut` beside the pattern.
            let written_mut = has_child(node, "mutable_specifier");
            let refutability = if node.child_by_field_name("alternative").is_some() {
                Refutability::Refutable
            } else {
                Refutability::Irrefutable
            };
            pattern_sites(node.child_by_field_name("pattern"), rule, written_mut, refutability, sites);
        }
        "parameter" => {
            let written_mut = has_child(node, "mutable_specifier");
            let pattern = node.child_by_field_name("pattern");
            pattern_sites(pattern, local(VALUE), written_mut, Refutability::Irrefutable, sites);
        }
        "closure_parameters" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() != "parameter" {
                    pattern_sites(Some(child), local(VALUE), false, Refutability::Irrefutable, sites);
                }
            }
        }
        "for_expression" => {
            let visible_from = node
                .child_by_field_name("body")
                .map_or(node.end_byte(), |body| body.start_byte());
            let rule = BindingRule { visible_from, ..local(VALUE) };
            pattern_sites(node.child_by_field_name("pattern"), rule, false, Refutability::Irrefutable, sites);
        }
        "match_pattern" => {
            // The guard (`condition`) reads the bindings; it binds none.
            let pattern = node.named_child(0).filter(|pattern| {
                node.child_by_field_name("condition")
                    .is_none_or(|condition| condition.id() != pattern.id())
            });
            pattern_sites(pattern, local(VALUE), false, Refutability::Refutable, sites);
        }
        "let_condition" => {
            let rule = BindingRule {
                visible_from: node.end_byte(),
                visible_until: conditional_end(node),
                ..local(VALUE)
            };
            pattern_sites(node.child_by_field_name("pattern"), rule, false, Refutability::Refutable, sites);
        }
        "use_declaration" => {
            use_names(node.child_by_field_name("argument"), &mut |name| {
                sites.push(BindingSite { name, rule: item(ANY_CATEGORY, true), is_import: true, bare_pattern: None });
            });
        }
        "extern_crate_declaration" => {
            let bound = node.child_by_field_name("alias").or(name);
            if let Some(name) = bound {
                sites.push(BindingSite { name, rule: item(ANY_CATEGORY, true), is_import: true, bare_pattern: None });
            }
        }
        _ => {}
    }
}

/// Whether `node` has an unnamed or named child of `kind`.
fn has_child(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}

/// The binding sites of `pattern`. `written_mut`: a `mut` written before
/// the whole pattern (`let mut x`, a `mut x` parameter), which the grammar
/// keeps beside it, so an identifier pattern is then an explicit binding.
fn pattern_sites<'tree>(
    pattern: Option<Node<'tree>>,
    rule: BindingRule,
    written_mut: bool,
    refutability: Refutability,
    sites: &mut Vec<BindingSite<'tree>>,
) {
    pattern_names(pattern, &mut |name, bare| {
        let bare = bare && !(written_mut && pattern.is_some_and(|pattern| pattern.id() == name.id()));
        sites.push(BindingSite { name, rule, is_import: false, bare_pattern: bare.then_some(refutability) });
    });
}

/// The names a `use` tree binds: a path's last segment, an alias, every
/// entry of a list, and for a `self` entry the last segment of its list's
/// path (`a` in `use a::{self};`). Globs bind no name here.
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
        "scoped_use_list" => {
            let list = node.child_by_field_name("list");
            let has_self = list.is_some_and(|list| {
                let mut cursor = list.walk();
                list.named_children(&mut cursor).any(|entry| entry.kind() == "self")
            });
            if has_self
                && let Some(path) = node.child_by_field_name("path")
            {
                use_names(Some(path), bind);
            }
            use_names(list, bind);
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                use_names(Some(child), bind);
            }
        }
        _ => {}
    }
}

/// The identifiers a pattern binds or may bind, each with whether it is
/// bare. An identifier written with `mut`, `ref` or `x @ …`, and a struct
/// field shorthand written with them, always binds; a bare one binds only
/// when it names no constant in view ([`BindingSite::bare_pattern`]),
/// in every pattern position alike (`let`, parameters, `for`, `match`,
/// `if let`/`while let`), since rustc resolves them all the same way.
fn pattern_names<'tree>(pattern: Option<Node<'tree>>, bind: &mut impl FnMut(Node<'tree>, bool)) {
    let Some(pattern) = pattern else { return };
    // `(node, explicit)`: whether a binding mode or `@` was written for it.
    let mut stack = vec![(pattern, false)];
    while let Some((node, explicit)) = stack.pop() {
        match node.kind() {
            "identifier" => bind(node, !explicit),
            "field_pattern" => {
                let mut cursor = node.walk();
                let moded = node
                    .children(&mut cursor)
                    .any(|child| matches!(child.kind(), "ref" | "mutable_specifier"));
                match node.child_by_field_name("pattern") {
                    Some(pattern) => stack.push((pattern, false)),
                    None => {
                        if let Some(name) = node
                            .child_by_field_name("name")
                            .filter(|name| name.kind() == "shorthand_field_identifier")
                        {
                            bind(name, !moded);
                        }
                    }
                }
            }
            "mut_pattern" | "ref_pattern" => {
                let mut cursor = node.walk();
                let children: Vec<Node<'tree>> = node.named_children(&mut cursor).collect();
                for child in children {
                    if child.kind() != "mutable_specifier" {
                        stack.push((child, true));
                    }
                }
            }
            "captured_pattern" => {
                let mut cursor = node.walk();
                let children: Vec<Node<'tree>> = node.named_children(&mut cursor).collect();
                for (index, child) in children.into_iter().enumerate() {
                    // The name before `@` binds; the pattern after it is
                    // read like any other.
                    stack.push((child, index == 0));
                }
            }
            // Paths, types, literals and expressions name no binding; a
            // generic path (`None::<u8>`) is a path.
            "scoped_identifier" | "scoped_type_identifier" | "type_identifier" | "generic_type" | "generic_pattern"
            | "range_pattern" | "macro_invocation" | "field_identifier" | "const_block" | "closure_expression" => {}
            _ => {
                let path = match node.kind() {
                    "tuple_struct_pattern" | "struct_pattern" => node.child_by_field_name("type"),
                    _ => None,
                };
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if path.is_none_or(|path| path.id() != child.id()) {
                        stack.push((child, false));
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
    let Some(parent) = node.indexed_parent() else { return VALUE };
    let is_field = |field: &str| parent.child_by_field_name(field).is_some_and(|child| child.id() == node.id());
    match parent.kind() {
        "macro_invocation" if is_field("macro") => MACRO,
        "scoped_identifier" | "scoped_type_identifier" | "scoped_use_list" if is_field("path") => TYPE,
        _ => VALUE,
    }
}
