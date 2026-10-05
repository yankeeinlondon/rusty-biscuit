//! Per-language matrices the symbol filters and reports work fills, as
//! skeletons: every language has a row, and each row names the phase that
//! replaces it with real expectations. `symbols_coverage` fails on a missing
//! or repeated row, so filling a matrix means rewriting rows, not adding
//! tests beside it. Filled so far, each in its own module: scope
//! (`symbols_scope`), qualified names (`symbols_qualified_names`),
//! containment (`symbols_containment`), flags (`async` in `symbols_async`,
//! deprecation and test in `symbols_flags`), metrics (`symbols_metrics`) and
//! kind coverage (`symbols_kind_coverage`). A phase that fills a matrix moves it into its own test
//! module with a real row type; once every matrix is filled, [`Pending`] is
//! deleted.

use tree_hugger::ProgrammingLanguage;

use ProgrammingLanguage as L;

/// A row with no expectations yet; `phase` is the plan phase that fills it.
#[derive(Debug, Clone, Copy)]
pub struct Pending {
    pub phase: u8,
}

/// Every skeleton matrix, by name, for the coverage guard.
pub const PENDING_MATRICES: &[(&str, &[(ProgrammingLanguage, Pending)])] = &[
    ("SELECTION_FILTER_MATRIX", SELECTION_FILTER_MATRIX),
];

/// Selection filters: each `SymbolQuery` predicate returns exactly the expected set, or nothing where n/a.
pub const SELECTION_FILTER_MATRIX: &[(ProgrammingLanguage, Pending)] = &[
    (L::Rust, Pending { phase: 4 }),
    (L::TypeScript, Pending { phase: 4 }),
    (L::JavaScript, Pending { phase: 4 }),
    (L::Go, Pending { phase: 4 }),
    (L::Python, Pending { phase: 4 }),
    (L::Java, Pending { phase: 4 }),
    (L::CSharp, Pending { phase: 4 }),
    (L::C, Pending { phase: 4 }),
    (L::Cpp, Pending { phase: 4 }),
    (L::Swift, Pending { phase: 4 }),
    (L::Scala, Pending { phase: 4 }),
    (L::Php, Pending { phase: 4 }),
    (L::Perl, Pending { phase: 4 }),
    (L::Lua, Pending { phase: 4 }),
    (L::Bash, Pending { phase: 4 }),
    (L::Zsh, Pending { phase: 4 }),
];
