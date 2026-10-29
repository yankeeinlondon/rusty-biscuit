//! Helpers shared by the corpus submodules.

#![allow(dead_code)]

use darkmatter::markdown::compose::ComposeOperation;
use darkmatter::markdown::compose::collection_blocks::CollectionKind;

use crate::collection_execution::compose_configured;

/// Both directive kinds, in the order the corpus runs them.
pub(crate) const KINDS: [CollectionKind; 2] = [CollectionKind::Map, CollectionKind::Filter];

/// `::map` / `::filter`.
pub(crate) fn opener(kind: CollectionKind) -> String {
    format!("::{}", kind.opener())
}

/// `::end-map` / `::end-filter`.
pub(crate) fn closer(kind: CollectionKind) -> String {
    format!("::{}", kind.closer())
}

/// The other kind, for crossed-closer rows.
pub(crate) fn other(kind: CollectionKind) -> CollectionKind {
    match kind {
        CollectionKind::Map => CollectionKind::Filter,
        CollectionKind::Filter => CollectionKind::Map,
    }
}

/// A complete region of `kind` whose opener line is `::kind {header}`.
pub(crate) fn region(kind: CollectionKind, header: &str, body: &str) -> String {
    format!("{} {header}\n{body}{}\n", opener(kind), closer(kind))
}

/// Generates one `#[test]` per name that calls `$run(name)`, plus a const
/// listing the names so a guard can compare them with the source table.
macro_rules! row_tests {
    ($list:ident, $run:path; $($name:ident),+ $(,)?) => {
        const $list: &[&str] = &[$(stringify!($name)),+];
        $( #[test] fn $name() { $run(stringify!($name)); } )+
    };
}
pub(crate) use row_tests;

/// Composes with the cleanup stage disabled, so the output is the exact bytes
/// the collection, interpolation, and block stages produced.
#[track_caller]
pub(crate) fn composed_raw(content: &str) -> String {
    compose_configured(content, |options| options.disable(ComposeOperation::Cleanup))
        .unwrap_or_else(|error| panic!("{content:?} failed: {error}"))
}

/// Like [`composed_raw`] for a failing document.
#[track_caller]
pub(crate) fn failed_raw(content: &str) -> darkmatter::markdown::MarkdownError {
    match compose_configured(content, |options| options.disable(ComposeOperation::Cleanup)) {
        Ok(output) => panic!("{content:?} composed:\n{output}"),
        Err(error) => error,
    }
}
