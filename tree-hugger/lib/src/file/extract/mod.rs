//! Stateless helpers for extracting symbol data from tree-sitter nodes.
//!
//! These functions were carved out of `tree_file.rs` to separate the
//! `TreeFile` type and its query orchestration from the large body of
//! language-specific AST extraction logic. Submodules group helpers by
//! responsibility; all but `attributes` are re-exported here (`complexity`
//! only through `cyclomatic`) so each submodule can reach the others via
//! `use super::*`.

pub(crate) mod attributes;
pub(crate) mod c_declarators;
pub(crate) mod classify;
pub(crate) mod complexity;
pub(crate) mod declaration;
pub(crate) mod doc_comments;
pub(crate) mod flags;
pub(crate) mod generics;
pub(crate) mod header;
pub(crate) mod metrics;
pub(crate) mod modifiers;
pub(crate) mod navigation;
pub(crate) mod qualified_name;
pub(crate) mod relations;
pub(crate) mod rust_impl;
pub(crate) mod scope;
pub(crate) mod signatures;
pub(crate) mod static_flags;
pub(crate) mod type_metadata;
pub(crate) mod visibility;

pub(crate) use c_declarators::*;
pub(crate) use classify::*;
pub(crate) use complexity::cyclomatic;
pub(crate) use declaration::*;
pub(crate) use doc_comments::*;
pub(crate) use flags::*;
pub(crate) use generics::*;
pub(crate) use header::*;
pub(crate) use metrics::*;
pub(crate) use modifiers::*;
pub(crate) use navigation::*;
pub(crate) use qualified_name::*;
pub(crate) use relations::*;
pub(crate) use rust_impl::*;
pub(crate) use scope::*;
pub(crate) use signatures::*;
pub(crate) use static_flags::*;
pub(crate) use type_metadata::*;
pub(crate) use visibility::*;

// Shared imports re-exported so submodules can simply `use super::*`.
pub(crate) use crate::shared::{
    CodeRange, FieldInfo, FunctionSignature, ParameterInfo, ProgrammingLanguage, SymbolInfo,
    SymbolKind, TextPoint, TextSpan, TypeAliasData, TypeMetadata,
    VariantInfo, Visibility,
};
pub(crate) use tree_sitter::Node;
