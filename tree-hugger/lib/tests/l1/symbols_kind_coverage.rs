//! The kind-coverage matrix of `2026-10-03-symbol-filters-and-reports`:
//! each construct whose precise kind is deferred (a `FEAT` cell of the
//! visibility-and-kind fix's kind matrix) is asserted as unsupported
//! extraction, separately from the kinds a language has no construct for
//! (its `—` cells). Landing a deferred construct moves its entry out of
//! `unsupported`; a form the pinned grammar cannot parse is a
//! [`GRAMMAR_LIMITS`] entry, demonstrated by its own failing parse.

use tempfile::TempDir;
use tree_hugger::{ProgrammingLanguage, SymbolKindV2, TreeFile, VisibilityLevel};

use crate::symbols_fixtures::extract_primary;

use ProgrammingLanguage as L;
use SymbolKindV2 as K;

/// A construct of a deferred (`FEAT`) cell, as a snippet declaring `name`.
pub struct Construct {
    pub construct: &'static str,
    pub extension: &'static str,
    pub source: &'static str,
    /// The declared name.
    pub name: &'static str,
    /// The precise kind the cell asks for.
    pub deferred: SymbolKindV2,
}

pub struct KindCoverageRow {
    pub language: ProgrammingLanguage,
    /// Not (yet) extracted with the precise kind: absent from the records,
    /// or reported with another kind.
    pub unsupported: &'static [Construct],
    /// Cells extracted with the precise kind already, as a side effect of
    /// other work.
    pub landed: &'static [Construct],
    /// Kinds of constructs the language does not have: no record of the
    /// primary fixture has them.
    pub no_construct: &'static [SymbolKindV2],
}

/// Every kind but `Function` (and the catch-alls): shells declare only
/// functions and variables.
const SHELL_NO_CONSTRUCT: &[SymbolKindV2] = &[
    K::Method,
    K::Constructor,
    K::Class,
    K::Interface,
    K::Trait,
    K::Enum,
    K::EnumVariant,
    K::Type,
    K::TypeAlias,
    K::Module,
    K::Namespace,
    K::Field,
    K::Property,
];

pub const KIND_COVERAGE_MATRIX: &[KindCoverageRow] = &[
    KindCoverageRow {
        language: L::Rust,
        unsupported: &[],
        landed: &[],
        no_construct: &[K::Constructor, K::Class],
    },
    KindCoverageRow {
        language: L::TypeScript,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "enum member",
                extension: "ts",
                source: "enum Color { Red }\n",
                name: "Red",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "namespace",
                extension: "ts",
                source: "namespace Space { export const inside = 1; }\n",
                name: "Space",
                deferred: K::Namespace,
            },
            Construct {
                construct: "namespace (`module`)",
                extension: "ts",
                source: "module Space { export const inside = 1; }\n",
                name: "Space",
                deferred: K::Namespace,
            },
        ],
        no_construct: &[K::Type],
    },
    KindCoverageRow {
        language: L::JavaScript,
        unsupported: &[],
        landed: &[],
        no_construct: &[
            K::Interface,
            K::Trait,
            K::Enum,
            K::EnumVariant,
            K::Type,
            K::TypeAlias,
            K::Module,
            K::Namespace,
        ],
    },
    KindCoverageRow {
        language: L::Go,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "type alias",
                extension: "go",
                source: "package p\n\ntype Alias = int\n",
                name: "Alias",
                deferred: K::TypeAlias,
            },
            Construct {
                construct: "struct field",
                extension: "go",
                source: "package p\n\ntype S struct {\n\tInside int\n}\n",
                name: "Inside",
                deferred: K::Field,
            },
            Construct {
                construct: "grouped type alias",
                extension: "go",
                source: "package p\n\ntype (\n\tAlias = int\n)\n",
                name: "Alias",
                deferred: K::TypeAlias,
            },
        ],
        no_construct: &[K::Constructor, K::Class, K::Enum, K::EnumVariant],
    },
    KindCoverageRow {
        language: L::Python,
        unsupported: &[],
        landed: &[Construct {
            construct: "constructor",
            extension: "py",
            source: "class C:\n    def __init__(self):\n        pass\n",
            name: "__init__",
            deferred: K::Constructor,
        }],
        no_construct: &[
            K::Interface,
            K::Trait,
            K::Enum,
            K::EnumVariant,
            K::Type,
            K::TypeAlias,
            K::Module,
            K::Namespace,
        ],
    },
    KindCoverageRow {
        language: L::Java,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "constructor",
                extension: "java",
                source: "class C {\n    C() {}\n}\n",
                name: "C",
                deferred: K::Constructor,
            },
            Construct {
                construct: "enum constant",
                extension: "java",
                source: "enum E { RED }\n",
                name: "RED",
                deferred: K::EnumVariant,
            },
        ],
        no_construct: &[K::Function, K::TypeAlias],
    },
    KindCoverageRow {
        language: L::CSharp,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "constructor",
                extension: "cs",
                source: "class C\n{\n    public C() { }\n}\n",
                name: "C",
                deferred: K::Constructor,
            },
            Construct {
                construct: "enum member",
                extension: "cs",
                source: "enum E { Red }\n",
                name: "Red",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "expression-bodied struct constructor",
                extension: "cs",
                source: "struct S\n{\n    public int X;\n    public S(int x) => X = x;\n}\n",
                name: "S",
                deferred: K::Constructor,
            },
        ],
        no_construct: &[K::TypeAlias],
    },
    KindCoverageRow {
        language: L::C,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "enumerator",
                extension: "c",
                source: "enum color { RED };\n",
                name: "RED",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "enumerator of an anonymous enum",
                extension: "c",
                source: "enum { ANON };\n",
                name: "ANON",
                deferred: K::EnumVariant,
            },
        ],
        no_construct: &[
            K::Method,
            K::Constructor,
            K::Class,
            K::Interface,
            K::Trait,
            K::Module,
            K::Namespace,
        ],
    },
    KindCoverageRow {
        language: L::Cpp,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "constructor",
                extension: "cpp",
                source: "class C {\npublic:\n    C();\n};\n",
                name: "C",
                deferred: K::Constructor,
            },
            Construct {
                construct: "template-wrapped constructor",
                extension: "cpp",
                source: "class C {\npublic:\n    template <typename T> C(T t) {}\n};\n",
                name: "C",
                deferred: K::Constructor,
            },
            Construct {
                construct: "enumerator",
                extension: "cpp",
                source: "enum E { RED };\n",
                name: "RED",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "scoped enumerator",
                extension: "cpp",
                source: "enum class E { RED };\n",
                name: "RED",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "type alias (`using`)",
                extension: "cpp",
                source: "using Alias = int;\n",
                name: "Alias",
                deferred: K::TypeAlias,
            },
            Construct {
                construct: "namespace",
                extension: "cpp",
                source: "namespace space {\nint inside;\n}\n",
                name: "space",
                deferred: K::Namespace,
            },
        ],
        no_construct: &[K::Interface, K::Trait],
    },
    KindCoverageRow {
        language: L::Swift,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "initializer",
                extension: "swift",
                source: "class C {\n    init() {}\n}\n",
                name: "init",
                deferred: K::Constructor,
            },
            Construct {
                construct: "type alias",
                extension: "swift",
                source: "typealias Alias = Int\n",
                name: "Alias",
                deferred: K::TypeAlias,
            },
            Construct {
                construct: "failable initializer",
                extension: "swift",
                source: "struct S {\n    init?(s: String) { return nil }\n}\n",
                name: "init",
                deferred: K::Constructor,
            },
        ],
        no_construct: &[K::Module, K::Namespace],
    },
    KindCoverageRow {
        language: L::Scala,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "enum case",
                extension: "scala",
                source: "enum Color:\n  case Red\n",
                name: "Red",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "type alias",
                extension: "scala",
                source: "object O {\n  type Alias = Int\n}\n",
                name: "Alias",
                deferred: K::TypeAlias,
            },
            Construct {
                construct: "class field",
                extension: "scala",
                source: "class C {\n  val inside: Int = 1\n}\n",
                name: "inside",
                deferred: K::Field,
            },
            Construct {
                construct: "enum case with parameters",
                extension: "scala",
                source: "enum Color {\n  case Value(x: Int)\n}\n",
                name: "Value",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "trait field declaration",
                extension: "scala",
                source: "trait T {\n  var d: Int\n}\n",
                name: "d",
                deferred: K::Field,
            },
        ],
        no_construct: &[K::Constructor],
    },
    KindCoverageRow {
        language: L::Php,
        unsupported: &[],
        landed: &[
            Construct {
                construct: "constructor",
                extension: "php",
                source: "<?php\nclass C\n{\n    public function __construct() {}\n}\n",
                name: "__construct",
                deferred: K::Constructor,
            },
            Construct {
                construct: "enum case",
                extension: "php",
                source: "<?php\nenum Suit\n{\n    case Hearts;\n}\n",
                name: "Hearts",
                deferred: K::EnumVariant,
            },
            Construct {
                construct: "namespace",
                extension: "php",
                source: "<?php\nnamespace App;\n\nfunction inside() {}\n",
                name: "App",
                deferred: K::Namespace,
            },
            Construct {
                construct: "braced namespace",
                extension: "php",
                source: "<?php\nnamespace App\\Braced {\n    function inside() {}\n}\n",
                name: "App\\Braced",
                deferred: K::Namespace,
            },
            Construct {
                construct: "backed enum case",
                extension: "php",
                source: "<?php\nenum Suit: string\n{\n    case Hearts = 'h';\n}\n",
                name: "Hearts",
                deferred: K::EnumVariant,
            },
        ],
        no_construct: &[K::Type, K::TypeAlias],
    },
    KindCoverageRow {
        language: L::Perl,
        unsupported: &[],
        landed: &[],
        no_construct: &[
            K::Method,
            K::Constructor,
            K::Class,
            K::Interface,
            K::Trait,
            K::Enum,
            K::EnumVariant,
            K::Type,
            K::TypeAlias,
            K::Field,
            K::Property,
        ],
    },
    KindCoverageRow {
        language: L::Lua,
        unsupported: &[],
        landed: &[Construct {
            construct: "method (`:` function)",
            extension: "lua",
            source: "local M = {}\nfunction M:greet() end\nreturn M\n",
            name: "greet",
            deferred: K::Method,
        }],
        no_construct: &[
            K::Constructor,
            K::Class,
            K::Interface,
            K::Trait,
            K::Enum,
            K::EnumVariant,
            K::Type,
            K::TypeAlias,
            K::Module,
            K::Namespace,
            K::Field,
            K::Property,
        ],
    },
    KindCoverageRow {
        language: L::Bash,
        unsupported: &[],
        landed: &[],
        no_construct: SHELL_NO_CONSTRUCT,
    },
    KindCoverageRow {
        language: L::Zsh,
        unsupported: &[],
        landed: &[],
        no_construct: SHELL_NO_CONSTRUCT,
    },
];

/// A form of a deferred cell the pinned grammar cannot parse: the snippet
/// has syntax errors, and `remaining` is what the extractor still reports.
pub struct GrammarLimit {
    pub language: ProgrammingLanguage,
    pub construct: &'static str,
    pub extension: &'static str,
    pub source: &'static str,
    pub grammar: &'static str,
    pub reason: &'static str,
    /// `(name, kind, access)` of each record the extractor still reports.
    pub remaining: &'static [(&'static str, SymbolKindV2, VisibilityLevel)],
}

pub const GRAMMAR_LIMITS: &[GrammarLimit] = &[GrammarLimit {
    language: L::Scala,
    construct: "enum case of an access-modified enum",
    extension: "scala",
    source: "object W {\n  private enum Hidden {\n    case H\n  }\n}\n",
    grammar: "tree-sitter-scala 0.24.0",
    reason: "an access modifier before `enum` parses as an ERROR node followed by the enum, so the modifier is lost and the case inherits Public",
    remaining: &[
        ("Hidden", K::Enum, VisibilityLevel::Public),
        ("H", K::EnumVariant, VisibilityLevel::Public),
    ],
}];

/// Each recorded grammar limit is demonstrated: its snippet does not parse
/// cleanly, and the extractor reports what the limit says it does.
#[test]
fn grammar_limits_are_demonstrated() {
    for limit in GRAMMAR_LIMITS {
        let label = format!("{} {} ({}): {}", limit.language.name(), limit.construct, limit.grammar, limit.reason);
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("snippet.{}", limit.extension));
        std::fs::write(&path, limit.source).expect("write snippet");
        let tree_file = TreeFile::new(&path).expect("parse snippet");
        assert_eq!(tree_file.language, limit.language, "{label}");
        assert!(!tree_file.syntax_diagnostics().is_empty(), "{label}: the snippet now parses; land the form");
        let records = tree_file.symbol_index_v2().expect("index").symbols;
        for &(name, kind, access) in limit.remaining {
            assert!(
                records
                    .iter()
                    .any(|record| record.identity.name == name && record.kind == kind && record.visibility.level == access),
                "{label}: `{name}` is not {kind:?} with {access:?}"
            );
        }
    }
}

/// The languages with a kind-coverage row, for the coverage guard.
pub fn kind_coverage_matrix_languages() -> Vec<ProgrammingLanguage> {
    KIND_COVERAGE_MATRIX.iter().map(|row| row.language).collect()
}

/// Whether `case`'s snippet yields a record of its name with its precise
/// kind.
fn extracts_precisely(language: ProgrammingLanguage, case: &Construct) -> bool {
    let label = format!("{} {}", language.name(), case.construct);
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(format!("snippet.{}", case.extension));
    std::fs::write(&path, case.source).expect("write snippet");
    let tree_file = TreeFile::new(&path).expect("parse snippet");
    assert_eq!(tree_file.language, language, "{label}");
    assert!(tree_file.syntax_diagnostics().is_empty(), "{label}: the snippet parses");
    tree_file
        .symbol_index_v2()
        .expect("index")
        .symbols
        .iter()
        .any(|record| record.identity.name == case.name && record.kind == case.deferred)
}

#[test]
fn deferred_constructs_are_unsupported_extraction() {
    for row in KIND_COVERAGE_MATRIX {
        for case in row.unsupported {
            assert!(
                !extracts_precisely(row.language, case),
                "{} {}: `{}` is now extracted as {:?}; move it to `landed`",
                row.language.name(),
                case.construct,
                case.name,
                case.deferred
            );
        }
        for case in row.landed {
            assert!(
                extracts_precisely(row.language, case),
                "{} {}: `{}` is not extracted as {:?}",
                row.language.name(),
                case.construct,
                case.name,
                case.deferred
            );
        }
    }
}

crate::language_cases::language_tests!(kinds_a_language_lacks_never_appear_in_its_fixture => check_kinds_a_language_lacks_never_appear_in_its_fixture);

fn check_kinds_a_language_lacks_never_appear_in_its_fixture(language: ProgrammingLanguage) {
    for row in KIND_COVERAGE_MATRIX.iter().filter(|row| row.language == language) {
        let extracted = extract_primary(row.language);
        for record in &extracted.records {
            assert!(
                !row.no_construct.contains(&record.kind),
                "{}: `{}` has {:?}, a kind the language has no construct for",
                extracted.label,
                record.identity.qualified_name,
                record.kind
            );
        }
    }
}
