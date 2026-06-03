//! Stateless helpers for extracting symbol data from tree-sitter nodes.
//!
//! These functions were carved out of `tree_file.rs` to separate the
//! `TreeFile` type and its query orchestration from the large body of
//! language-specific AST extraction logic. Submodules group helpers by
//! responsibility; all are re-exported here so each submodule can reach the
//! others via `use super::*`.

pub(crate) mod classify;
pub(crate) mod doc_comments;
pub(crate) mod navigation;
pub(crate) mod signatures;
pub(crate) mod type_metadata;

pub(crate) use classify::*;
pub(crate) use doc_comments::*;
pub(crate) use navigation::*;
pub(crate) use signatures::*;
pub(crate) use type_metadata::*;

// Shared imports re-exported so submodules can simply `use super::*`.
pub(crate) use crate::shared::{
    CodeRange, FieldInfo, FunctionSignature, ParameterInfo, ProgrammingLanguage, SymbolInfo,
    SymbolKind, SymbolKindV2, SymbolRecord, TextPoint, TextSpan, TypeAliasData, TypeMetadata,
    VariantInfo, Visibility,
};
pub(crate) use tree_sitter::Node;
