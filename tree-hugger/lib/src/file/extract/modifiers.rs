//! Declaration modifiers read at extraction: `async`, and the written
//! modifiers of [`declaration_modifiers`]; `static` lives in `static_flags`.

use super::attributes::modifier_host;
use super::*;
use crate::file::parents::IndexedParent as _;
use crate::shared::Modifier;

/// Whether a declaration is declared `async`: its own `async` keyword token,
/// or, for a JavaScript/TypeScript name bound to a function value
/// (`const f = async () => …`, a class field holding one), that function's.
///
/// Only Rust, JavaScript, TypeScript, Python, C# and Swift have async
/// declarations; every other language reports `false`.
pub(crate) fn is_async_declaration(declaration: Node<'_>, name: Node<'_>, language: ProgrammingLanguage) -> bool {
    match language {
        ProgrammingLanguage::Rust => {
            matches!(declaration.kind(), "function_item" | "function_signature_item")
                && find_child_by_kind(declaration, "function_modifiers")
                    .is_some_and(|modifiers| has_async_token(modifiers))
        }
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            let function = match name.indexed_parent() {
                Some(binding)
                    if matches!(
                        binding.kind(),
                        "variable_declarator" | "public_field_definition" | "field_definition" | "pair"
                    ) =>
                {
                    binding.child_by_field_name("value")
                }
                _ => Some(declaration),
            };
            function.is_some_and(|function| is_function_node_kind(function.kind()) && has_async_token(function))
        }
        ProgrammingLanguage::Python => declaration.kind() == "function_definition" && has_async_token(declaration),
        ProgrammingLanguage::CSharp => {
            let mut cursor = declaration.walk();
            is_function_node_kind(declaration.kind())
                && declaration
                    .children(&mut cursor)
                    .any(|child| child.kind() == "modifier" && has_async_token(child))
        }
        ProgrammingLanguage::Swift => is_function_node_kind(declaration.kind()) && has_async_token(declaration),
        ProgrammingLanguage::Go
        | ProgrammingLanguage::Java
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::Scala
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => false,
    }
}

/// Whether `node` has the anonymous `async` keyword token as a direct child.
fn has_async_token(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| !child.is_named() && child.kind() == "async")
}

/// The written declaration modifiers beyond `static` and `async`: `Const`
/// (Rust `const fn` only), `Unsafe`, `Readonly`, `Mut` (Rust bindings and
/// receivers) and the lowercase `Custom` keywords `abstract`, `final`,
/// `override`, `virtual`, C# `sealed` and Swift `mutating`.
///
/// Only tokens the declaration owns count: its own keyword tokens and
/// modifier lists, never a keyword in its body, its type, a nested
/// parameter or a string. Unordered and possibly repeated; the record
/// normalizes them.
pub(crate) fn declaration_modifiers(
    declaration: Node<'_>,
    name: Node<'_>,
    language: ProgrammingLanguage,
) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    match language {
        ProgrammingLanguage::Rust => rust_modifiers(declaration, name, &mut modifiers),
        ProgrammingLanguage::Cpp => cpp_modifiers(declaration, name, &mut modifiers),
        ProgrammingLanguage::TypeScript => {
            let mut cursor = declaration.walk();
            for child in declaration.children(&mut cursor) {
                match (child.is_named(), child.kind()) {
                    (false, "abstract") => modifiers.push(custom("abstract")),
                    (false, "readonly") => modifiers.push(Modifier::Readonly),
                    (true, "override_modifier") => modifiers.push(custom("override")),
                    _ => {}
                }
            }
        }
        ProgrammingLanguage::Java => {
            for list in children_of_kind(modifier_host(declaration, language), "modifiers") {
                for token in anonymous_tokens(list) {
                    if matches!(token, "abstract" | "final") {
                        modifiers.push(custom(token));
                    }
                }
            }
        }
        ProgrammingLanguage::CSharp => {
            for modifier in children_of_kind(modifier_host(declaration, language), "modifier") {
                for token in anonymous_tokens(modifier) {
                    match token {
                        "readonly" => modifiers.push(Modifier::Readonly),
                        "abstract" | "override" | "virtual" | "sealed" => modifiers.push(custom(token)),
                        _ => {}
                    }
                }
            }
        }
        ProgrammingLanguage::Swift => {
            for list in children_of_kind(declaration, "modifiers") {
                let mut cursor = list.walk();
                for modifier in list.named_children(&mut cursor) {
                    if !matches!(
                        modifier.kind(),
                        "inheritance_modifier" | "member_modifier" | "mutation_modifier"
                    ) {
                        continue;
                    }
                    for token in anonymous_tokens(modifier) {
                        if matches!(token, "final" | "override" | "mutating") {
                            modifiers.push(custom(token));
                        }
                    }
                }
            }
        }
        ProgrammingLanguage::Scala => {
            for list in children_of_kind(declaration, "modifiers") {
                for token in anonymous_tokens(list) {
                    if matches!(token, "abstract" | "final" | "override") {
                        modifiers.push(custom(token));
                    }
                }
            }
        }
        ProgrammingLanguage::Php => {
            let mut cursor = declaration.walk();
            for child in declaration.named_children(&mut cursor) {
                match child.kind() {
                    "abstract_modifier" => modifiers.push(custom("abstract")),
                    "final_modifier" => modifiers.push(custom("final")),
                    _ => {}
                }
            }
        }
        // JavaScript has only `static` and `async`; Python decorators imply
        // no modifier; the rest have none of the set.
        ProgrammingLanguage::JavaScript
        | ProgrammingLanguage::Python
        | ProgrammingLanguage::Go
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => {}
    }
    modifiers
}

/// Rust: `const`/`unsafe` from a function's `function_modifiers`, `unsafe
/// trait`, a method's `mut` receiver, and a `mut` binding (`let mut x`,
/// `static mut S`, `mut a: T`, `mut self`, a name under a `mut` pattern).
fn rust_modifiers(declaration: Node<'_>, name: Node<'_>, modifiers: &mut Vec<Modifier>) {
    match declaration.kind() {
        "function_item" | "function_signature_item" => {
            if let Some(list) = find_child_by_kind(declaration, "function_modifiers") {
                for token in anonymous_tokens(list) {
                    match token {
                        "const" => modifiers.push(Modifier::Const),
                        "unsafe" => modifiers.push(Modifier::Unsafe),
                        _ => {}
                    }
                }
            }
            if has_mut_receiver(declaration) {
                modifiers.push(Modifier::Mut);
            }
        }
        "trait_item" if anonymous_tokens(declaration).any(|token| token == "unsafe") => {
            modifiers.push(Modifier::Unsafe);
        }
        // `mut self` binds mutably; in `&mut self` the `mut` is the borrow's.
        "let_declaration" | "static_item" | "parameter" | "self_parameter"
            if find_child_by_kind(declaration, "mutable_specifier").is_some()
                && !anonymous_tokens(declaration).any(|token| token == "&") =>
        {
            modifiers.push(Modifier::Mut);
        }
        _ => {}
    }
    if name.indexed_parent().is_some_and(|parent| parent.kind() == "mut_pattern") {
        modifiers.push(Modifier::Mut);
    }
}

/// Whether a Rust method's receiver is written with `mut`: `&mut self`,
/// `mut self`, `self: &mut Self` or `mut self: Box<Self>`.
fn has_mut_receiver(function: Node<'_>) -> bool {
    let Some(parameters) = function.child_by_field_name("parameters") else {
        return false;
    };
    let mut cursor = parameters.walk();
    let Some(first) = parameters.named_children(&mut cursor).find(|child| {
        !matches!(child.kind(), "attribute_item" | "line_comment" | "block_comment")
    }) else {
        return false;
    };
    match first.kind() {
        "self_parameter" => find_child_by_kind(first, "mutable_specifier").is_some(),
        "parameter" => {
            first.child_by_field_name("pattern").is_some_and(|pattern| pattern.kind() == "self")
                && (find_child_by_kind(first, "mutable_specifier").is_some()
                    || first.child_by_field_name("type").is_some_and(|ty| {
                        ty.kind() == "reference_type" && find_child_by_kind(ty, "mutable_specifier").is_some()
                    }))
        }
        _ => false,
    }
}

/// C++: a class's `final`, a member's `virtual` keyword and the `override`/
/// `final` specifiers of the function declarator that declares `name`.
fn cpp_modifiers(declaration: Node<'_>, name: Node<'_>, modifiers: &mut Vec<Modifier>) {
    let mut specifier_hosts = Vec::new();
    match declaration.kind() {
        "class_specifier" | "struct_specifier" | "union_specifier" => specifier_hosts.push(declaration),
        "field_declaration" | "declaration" | "function_definition" => {
            if anonymous_tokens(declaration).any(|token| token == "virtual") {
                modifiers.push(custom("virtual"));
            }
            if let Some(function) = declared_function_named(declaration, name) {
                specifier_hosts.push(function.function);
            }
        }
        _ => {}
    }
    for host in specifier_hosts {
        for specifier in children_of_kind(host, "virtual_specifier") {
            for token in anonymous_tokens(specifier) {
                if matches!(token, "final" | "override") {
                    modifiers.push(custom(token));
                }
            }
        }
    }
}

fn custom(keyword: &str) -> Modifier {
    Modifier::Custom(keyword.to_string())
}

/// The kinds of `node`'s anonymous (keyword and punctuation) children.
fn anonymous_tokens<'tree>(node: Node<'tree>) -> impl Iterator<Item = &'static str> + 'tree {
    let mut cursor = node.walk();
    let tokens: Vec<&'static str> = node
        .children(&mut cursor)
        .filter(|child| !child.is_named())
        .map(|child| child.kind())
        .collect();
    tokens.into_iter()
}

fn children_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == kind)
        .collect()
}
