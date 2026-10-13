//! The two meanings of `static`, kept apart:
//!
//! - member `static` (`SymbolInfo::is_static`, `Modifier::Static`): a member
//!   reachable without an instance;
//! - storage `static` (`SymbolInfo::is_static_storage`,
//!   `Modifier::StaticStorage`): the keyword that sets storage duration or
//!   linkage, such as a Rust `static` item or a C file-scope `static`.
//!
//! A declaration never carries both.

use super::*;

mod cpp;
mod ecma;
mod java;
mod php;
mod python;
mod rust;
mod scala;
mod swift;

pub(crate) use cpp::extract_cpp_is_static;
pub(crate) use ecma::extract_ts_is_static;
pub(crate) use java::extract_java_csharp_is_static;
pub(crate) use php::extract_php_is_static;
pub(crate) use python::is_python_class_attribute;
pub(crate) use swift::extract_swift_is_static;

/// Whether `declaration` is a member reachable without an instance.
///
/// Only member kinds (functions, methods, constructors, fields, properties,
/// constants and variables) can be static; parameters and locals never are.
pub(crate) fn extract_is_static(declaration: Node<'_>, ctx: &DeclarationContext<'_, '_>) -> bool {
    if ctx.is_local()
        || !matches!(
            ctx.kind,
            SymbolKind::Function
                | SymbolKind::Method
                | SymbolKind::Constructor
                | SymbolKind::Field
                | SymbolKind::Property
                | SymbolKind::Constant
                | SymbolKind::Variable
        )
    {
        return false;
    }

    let source = ctx.source;
    match ctx.language {
        ProgrammingLanguage::Java | ProgrammingLanguage::CSharp => {
            java::extract_java_csharp_is_static(declaration, source)
        }
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            ecma::extract_ts_is_static(declaration)
        }
        ProgrammingLanguage::Php => php::extract_php_is_static(declaration),
        ProgrammingLanguage::Python => python::extract_python_is_static(declaration, source),
        ProgrammingLanguage::Swift => swift::extract_swift_is_static(declaration, source),
        ProgrammingLanguage::Scala => scala::extract_scala_is_static(declaration),
        // `static` on a namespace-scope declaration is storage, not membership.
        ProgrammingLanguage::Cpp => {
            cpp::is_class_member(declaration) && cpp::extract_cpp_is_static(declaration, source)
        }
        ProgrammingLanguage::Rust => {
            (ctx.kind.is_function() || ctx.kind == SymbolKind::Constant)
                && rust::extract_rust_is_static(declaration)
        }
        ProgrammingLanguage::C
        | ProgrammingLanguage::Go
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => false,
    }
}

/// Whether `declaration` uses `static` for storage duration or linkage.
pub(crate) fn extract_is_static_storage(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> bool {
    if ctx.kind == SymbolKind::Parameter {
        return false;
    }

    match ctx.language {
        ProgrammingLanguage::Rust => declaration.kind() == "static_item",
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => {
            !cpp::is_class_member(declaration)
                && cpp::extract_cpp_is_static(declaration, ctx.source)
        }
        _ => false,
    }
}
