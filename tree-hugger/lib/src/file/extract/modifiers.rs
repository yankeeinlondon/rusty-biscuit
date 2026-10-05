//! Declaration modifiers read at extraction (`async`; `static` lives in
//! `static_flags`).

use super::*;

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
            let function = match name.parent() {
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
