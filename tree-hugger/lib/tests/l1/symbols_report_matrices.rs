//! Per-language matrices the symbol filters and reports work fills, as
//! skeletons: every language has a row, and each row names the phase that
//! replaces it with real expectations. `symbols_coverage` fails on a missing
//! or repeated row, so filling a matrix means rewriting rows, not adding
//! tests beside it. A phase that fills a matrix moves it into its own test
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
    ("SCOPE_MATRIX", SCOPE_MATRIX),
    ("QUALIFIED_NAME_MATRIX", QUALIFIED_NAME_MATRIX),
    ("CONTAINMENT_MATRIX", CONTAINMENT_MATRIX),
    ("FLAGS_MATRIX", FLAGS_MATRIX),
    ("METRICS_MATRIX", METRICS_MATRIX),
    ("KIND_COVERAGE_MATRIX", KIND_COVERAGE_MATRIX),
    ("SELECTION_FILTER_MATRIX", SELECTION_FILTER_MATRIX),
];

/// Scope: one `Item`, `Member` (or n/a) and `Local` per language; parameters `Local` where extracted.
pub const SCOPE_MATRIX: &[(ProgrammingLanguage, Pending)] = &[
    (L::Rust, Pending { phase: 2 }),
    (L::TypeScript, Pending { phase: 2 }),
    (L::JavaScript, Pending { phase: 2 }),
    (L::Go, Pending { phase: 2 }),
    (L::Python, Pending { phase: 2 }),
    (L::Java, Pending { phase: 2 }),
    (L::CSharp, Pending { phase: 2 }),
    (L::C, Pending { phase: 2 }),
    (L::Cpp, Pending { phase: 2 }),
    (L::Swift, Pending { phase: 2 }),
    (L::Scala, Pending { phase: 2 }),
    (L::Php, Pending { phase: 2 }),
    (L::Perl, Pending { phase: 2 }),
    (L::Lua, Pending { phase: 2 }),
    (L::Bash, Pending { phase: 2 }),
    (L::Zsh, Pending { phase: 2 }),
];

/// Qualified names: cached in-file names, explicit namespaces, file-qualified fallbacks, library-only context names.
pub const QUALIFIED_NAME_MATRIX: &[(ProgrammingLanguage, Pending)] = &[
    (L::Rust, Pending { phase: 2 }),
    (L::TypeScript, Pending { phase: 2 }),
    (L::JavaScript, Pending { phase: 2 }),
    (L::Go, Pending { phase: 2 }),
    (L::Python, Pending { phase: 2 }),
    (L::Java, Pending { phase: 2 }),
    (L::CSharp, Pending { phase: 2 }),
    (L::C, Pending { phase: 2 }),
    (L::Cpp, Pending { phase: 2 }),
    (L::Swift, Pending { phase: 2 }),
    (L::Scala, Pending { phase: 2 }),
    (L::Php, Pending { phase: 2 }),
    (L::Perl, Pending { phase: 2 }),
    (L::Lua, Pending { phase: 2 }),
    (L::Bash, Pending { phase: 2 }),
    (L::Zsh, Pending { phase: 2 }),
];

/// Containment: members attach to the right container (Rust `impl`, TS classes, Go receivers, Python classes).
pub const CONTAINMENT_MATRIX: &[(ProgrammingLanguage, Pending)] = &[
    (L::Rust, Pending { phase: 2 }),
    (L::TypeScript, Pending { phase: 2 }),
    (L::JavaScript, Pending { phase: 2 }),
    (L::Go, Pending { phase: 2 }),
    (L::Python, Pending { phase: 2 }),
    (L::Java, Pending { phase: 2 }),
    (L::CSharp, Pending { phase: 2 }),
    (L::C, Pending { phase: 2 }),
    (L::Cpp, Pending { phase: 2 }),
    (L::Swift, Pending { phase: 2 }),
    (L::Scala, Pending { phase: 2 }),
    (L::Php, Pending { phase: 2 }),
    (L::Perl, Pending { phase: 2 }),
    (L::Lua, Pending { phase: 2 }),
    (L::Bash, Pending { phase: 2 }),
    (L::Zsh, Pending { phase: 2 }),
];

/// Async, deprecation and test flags: set exactly by each language's markers and file-name rules.
pub const FLAGS_MATRIX: &[(ProgrammingLanguage, Pending)] = &[
    (L::Rust, Pending { phase: 3 }),
    (L::TypeScript, Pending { phase: 3 }),
    (L::JavaScript, Pending { phase: 3 }),
    (L::Go, Pending { phase: 3 }),
    (L::Python, Pending { phase: 3 }),
    (L::Java, Pending { phase: 3 }),
    (L::CSharp, Pending { phase: 3 }),
    (L::C, Pending { phase: 3 }),
    (L::Cpp, Pending { phase: 3 }),
    (L::Swift, Pending { phase: 3 }),
    (L::Scala, Pending { phase: 3 }),
    (L::Php, Pending { phase: 3 }),
    (L::Perl, Pending { phase: 3 }),
    (L::Lua, Pending { phase: 3 }),
    (L::Bash, Pending { phase: 3 }),
    (L::Zsh, Pending { phase: 3 }),
];

/// Metrics: `parameters` and `has_docs` per fixture function, `None` where unsupported.
pub const METRICS_MATRIX: &[(ProgrammingLanguage, Pending)] = &[
    (L::Rust, Pending { phase: 3 }),
    (L::TypeScript, Pending { phase: 3 }),
    (L::JavaScript, Pending { phase: 3 }),
    (L::Go, Pending { phase: 3 }),
    (L::Python, Pending { phase: 3 }),
    (L::Java, Pending { phase: 3 }),
    (L::CSharp, Pending { phase: 3 }),
    (L::C, Pending { phase: 3 }),
    (L::Cpp, Pending { phase: 3 }),
    (L::Swift, Pending { phase: 3 }),
    (L::Scala, Pending { phase: 3 }),
    (L::Php, Pending { phase: 3 }),
    (L::Perl, Pending { phase: 3 }),
    (L::Lua, Pending { phase: 3 }),
    (L::Bash, Pending { phase: 3 }),
    (L::Zsh, Pending { phase: 3 }),
];

/// Kind coverage: each deferred construct asserted as unsupported extraction, apart from constructs the language lacks.
pub const KIND_COVERAGE_MATRIX: &[(ProgrammingLanguage, Pending)] = &[
    (L::Rust, Pending { phase: 3 }),
    (L::TypeScript, Pending { phase: 3 }),
    (L::JavaScript, Pending { phase: 3 }),
    (L::Go, Pending { phase: 3 }),
    (L::Python, Pending { phase: 3 }),
    (L::Java, Pending { phase: 3 }),
    (L::CSharp, Pending { phase: 3 }),
    (L::C, Pending { phase: 3 }),
    (L::Cpp, Pending { phase: 3 }),
    (L::Swift, Pending { phase: 3 }),
    (L::Scala, Pending { phase: 3 }),
    (L::Php, Pending { phase: 3 }),
    (L::Perl, Pending { phase: 3 }),
    (L::Lua, Pending { phase: 3 }),
    (L::Bash, Pending { phase: 3 }),
    (L::Zsh, Pending { phase: 3 }),
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
