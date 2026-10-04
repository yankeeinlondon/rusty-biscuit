//! Declaration visibility: one reader per language behind a single dispatcher
//! that accepts any declaration node, not only functions.
//!
//! A level describes the access declared on, or implicitly assigned to, this
//! declaration, not its effective access through enclosing scopes. `None`
//! means Unknown: no access applies, or it cannot be established from the
//! file.

use super::*;

mod c;
mod cpp;
mod csharp;
mod ecma;
mod go;
mod java;
mod lua;
mod php;
mod rust;
mod scala;
mod swift;

pub(crate) use csharp::extract_csharp_visibility;
pub(crate) use go::go_identifier_visibility;
pub(crate) use ecma::{extract_ts_visibility, is_ecma_module_file};
pub(crate) use java::extract_java_visibility;
pub(crate) use php::extract_php_visibility;
pub(crate) use swift::extract_swift_visibility;

/// Reads the visibility level of `declaration`.
///
/// Parameters and local declarations always report `None`: member and module
/// access do not apply inside a function body.
pub(crate) fn extract_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    if ctx.is_local || ctx.kind == SymbolKind::Parameter {
        return None;
    }

    match ctx.language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            ecma::extract_ecma_visibility(declaration, ctx)
        }
        ProgrammingLanguage::Java => java::extract_java_declaration_visibility(declaration, ctx),
        ProgrammingLanguage::CSharp => csharp::extract_csharp_declaration_visibility(declaration, ctx),
        ProgrammingLanguage::Php => php::extract_php_declaration_visibility(declaration, ctx),
        ProgrammingLanguage::Rust => rust::extract_rust_visibility(declaration, ctx),
        ProgrammingLanguage::Cpp => cpp::extract_cpp_visibility(declaration, ctx),
        ProgrammingLanguage::Swift => swift::extract_swift_declaration_visibility(declaration, ctx),
        ProgrammingLanguage::C => c::extract_c_visibility(declaration, ctx),
        ProgrammingLanguage::Go => go::extract_go_visibility(declaration, ctx),
        ProgrammingLanguage::Scala => scala::extract_scala_visibility(declaration, ctx),
        ProgrammingLanguage::Lua => lua::extract_lua_visibility(declaration, ctx),
        // Privacy is only a naming convention (Python `_name`, Perl `_sub`) or
        // does not exist (shell functions); a guess is not reported as fact.
        ProgrammingLanguage::Python
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => None,
    }
}
