//! Declaration order for the symbol commands (`--sort`).
//!
//! Every order ends with the same tie-breakers: portable file path, start
//! byte, end byte, kind (in [`kind_rank`] order) and record ID, so two runs
//! over the same tree always agree. `--sort file` is exactly those
//! tie-breakers. See `docs/symbol-reports.md` for the user-facing contract.

use std::cmp::Ordering;

use clap::ValueEnum;
use tree_hugger::{SymbolKindV2, SymbolRecord};

/// The order of declarations within a listing (`--sort`).
#[derive(ValueEnum, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SortOrder {
    /// By file, then source position.
    #[default]
    File,
    /// By the written, case-sensitive name.
    Name,
    /// By kind: modules and types first, then callables, then values.
    Kind,
}

/// Position of `kind` in the `--sort kind` order.
pub(crate) fn kind_rank(kind: SymbolKindV2) -> u8 {
    match kind {
        SymbolKindV2::Module => 0,
        SymbolKindV2::Namespace => 1,
        SymbolKindV2::Type => 2,
        SymbolKindV2::Class => 3,
        SymbolKindV2::Interface => 4,
        SymbolKindV2::Trait => 5,
        SymbolKindV2::Enum => 6,
        SymbolKindV2::TypeAlias => 7,
        SymbolKindV2::Function => 8,
        SymbolKindV2::Method => 9,
        SymbolKindV2::Constructor => 10,
        SymbolKindV2::Macro => 11,
        SymbolKindV2::Constant => 12,
        SymbolKindV2::Field => 13,
        SymbolKindV2::Property => 14,
        SymbolKindV2::EnumVariant => 15,
        SymbolKindV2::Variable => 16,
        SymbolKindV2::Parameter => 17,
        SymbolKindV2::Unknown => 18,
    }
}

/// Orders two portable (`/`-separated) paths directory by directory, so a
/// directory's files stay together: `a/b.rs` sorts before `a-c.rs`.
pub(crate) fn portable_path_cmp(left: &str, right: &str) -> Ordering {
    left.split('/').cmp(right.split('/'))
}

/// Compares two records under `order`, each paired with its portable path.
pub(crate) fn compare(
    order: SortOrder,
    (left_path, left): (&str, &SymbolRecord),
    (right_path, right): (&str, &SymbolRecord),
) -> Ordering {
    let primary = match order {
        SortOrder::File => Ordering::Equal,
        SortOrder::Name => left.identity.name.cmp(&right.identity.name),
        SortOrder::Kind => kind_rank(left.kind).cmp(&kind_rank(right.kind)),
    };
    let left_span = &left.source.declaration_span;
    let right_span = &right.source.declaration_span;
    primary
        .then_with(|| portable_path_cmp(left_path, right_path))
        .then_with(|| left_span.start_byte.cmp(&right_span.start_byte))
        .then_with(|| left_span.end_byte.cmp(&right_span.end_byte))
        .then_with(|| kind_rank(left.kind).cmp(&kind_rank(right.kind)))
        .then_with(|| left.id.0.cmp(&right.id.0))
}

/// Sorts one file's records under `order`.
pub(crate) fn sort_file_records(records: &mut [SymbolRecord], path: &str, order: SortOrder) {
    records.sort_by(|left, right| compare(order, (path, left), (path, right)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_paths_order_directory_by_directory() {
        let mut paths = vec!["a-c.rs", "a/b.rs", "B.rs", "a.rs"];
        paths.sort_by(|left, right| portable_path_cmp(left, right));
        assert_eq!(paths, vec!["B.rs", "a/b.rs", "a-c.rs", "a.rs"]);
    }
}
