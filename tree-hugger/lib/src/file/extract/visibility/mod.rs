//! Declaration visibility: one reader per language behind a single dispatcher
//! that accepts any declaration node, not only functions.
//!
//! A level describes the access declared on, or implicitly assigned to, this
//! declaration, not its effective access through enclosing scopes. `None`
//! means Unknown: no access applies, or it cannot be established from the
//! file.

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};

use super::*;
use crate::analysis::counters::count_shared_context_steps;

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
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    if ctx.is_local() || ctx.kind == SymbolKind::Parameter {
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

/// Facts about one file that several declarations' visibility depends on,
/// each derived at most once per file (or per class body) on first use
/// rather than re-read from the tree for every declaration. Create one per
/// file and hand it to every [`DeclarationContext`] of that file.
#[derive(Default)]
pub(crate) struct VisibilityMemo<'tree> {
    /// C/C++: the file's namespaces and its earlier `static` function
    /// declarations ([`c::LinkageIndex`]).
    c_linkage: OnceCell<c::LinkageIndex<'tree>>,
    /// C++: class body (or nested level) node id → its children's access
    /// sections.
    cpp_sections: RefCell<HashMap<usize, SectionsInForce>>,
    /// Scala: every identifier segment of the file's `package` clauses.
    scala_package_segments: OnceCell<HashSet<&'tree str>>,
    /// Rust: trait name → the level its same-named traits agree on (`None`
    /// when they disagree).
    rust_trait_levels: OnceCell<HashMap<&'tree str, Option<Visibility>>>,
}

/// Child node id → the C++ access section in force before that child
/// (`None`: no section precedes it; `Some(None)`: the nearest section's text
/// names no level).
type SectionsInForce = HashMap<usize, Option<Option<Visibility>>>;

impl std::fmt::Debug for VisibilityMemo<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("VisibilityMemo").finish_non_exhaustive()
    }
}
