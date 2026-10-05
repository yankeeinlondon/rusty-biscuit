//! The metrics matrix of `2026-10-03-symbol-filters-and-reports`:
//! `parameters` and `has_docs` for each fixture function, `None` where a
//! metric is unsupported, and the language-wide rules behind them.

use std::path::Path;

use tempfile::TempDir;
use tree_hugger::{
    Capability, MetricsFacet, ProgrammingLanguage, SourceFile, SymbolKindV2, SymbolQuery,
    SymbolRecord,
};

use crate::symbols_fixtures::{display_name, in_file_name, primary_fixture};

use ProgrammingLanguage as L;

/// Whether a metric is measured in a language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    Measured,
    /// Unsupported extraction, with the reason; every record reports `None`.
    Unsupported(&'static str),
}

/// `(qualified name of a function, parameters, has_docs)`.
pub type MetricsCase = (&'static str, Option<u32>, Option<bool>);

pub struct MetricsRow {
    pub language: ProgrammingLanguage,
    pub parameters: Support,
    pub docs: Support,
    pub cases: &'static [MetricsCase],
}

const NO_PARAMETERS: Support = Support::Unsupported("parameters are not extracted");
const NO_DOCS: Support = Support::Unsupported("doc comments are not detected");

pub const METRICS_MATRIX: &[MetricsRow] = &[
    MetricsRow {
        language: L::Rust,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true)),
            ("undocumented_none", Some(0), Some(false)),
            ("deprecated_api", Some(0), Some(false)),
            // The `&self` receiver is not counted.
            ("Receiver::with_receiver", Some(1), Some(false)),
            ("cfg_test_module::inside_cfg_test", Some(1), Some(false)),
            ("function_with_nested_items::nested_local_module::nested_module_function", Some(0), Some(false)),
        ],
    },
    MetricsRow {
        language: L::TypeScript,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true)),
            ("undocumentedNone", Some(0), Some(false)),
            ("variadic", Some(2), Some(false)),
            ("Receiver.withReceiver", Some(1), Some(true)),
            ("boundaryArrow", Some(1), Some(false)),
        ],
    },
    MetricsRow {
        language: L::JavaScript,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true)),
            ("undocumentedNone", Some(0), Some(false)),
            ("variadic", Some(2), Some(false)),
            ("documentedArrow", Some(1), Some(true)),
            ("lookAlikeTag", Some(0), Some(true)),
        ],
    },
    MetricsRow {
        language: L::Go,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("symbols.DocumentedPair", Some(2), Some(true)),
            ("symbols.UndocumentedNone", Some(0), Some(false)),
            ("symbols.Variadic", Some(2), Some(false)),
            // The receiver has its own list and is not counted.
            ("symbols.ExportedStruct.WithReceiver", Some(1), Some(false)),
            ("symbols.DeprecatedLater", Some(0), Some(true)),
        ],
    },
    MetricsRow {
        language: L::Python,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true)),
            ("undocumented_none", Some(0), Some(false)),
            // `*rest` and `**options` count once each.
            ("variadic", Some(3), Some(false)),
            // `self` is an ordinary parameter in Python's syntax.
            ("Receiver.with_receiver", Some(2), Some(false)),
            ("outer_function.nested_function", Some(0), Some(false)),
        ],
    },
    MetricsRow {
        language: L::Java,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("com.example.symbols.FlagsAndMetrics.documentedPair", Some(2), Some(true)),
            ("com.example.symbols.FlagsAndMetrics.undocumentedNone", Some(0), Some(false)),
            ("com.example.symbols.FlagsAndMetrics.deprecatedByTag", Some(0), Some(true)),
            ("com.example.symbols.FlagsAndMetrics.variadic", Some(2), Some(false)),
        ],
    },
    MetricsRow {
        language: L::CSharp,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("Example.Symbols.FlagsAndMetrics.DocumentedPair", Some(2), Some(true)),
            ("Example.Symbols.FlagsAndMetrics.UndocumentedNone", Some(0), Some(false)),
            ("Example.Symbols.FlagsAndMetrics.Variadic", Some(2), Some(false)),
        ],
    },
    MetricsRow {
        language: L::C,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true)),
            // `(void)` is a known empty list.
            ("undocumented_none", Some(0), Some(false)),
            ("variadic", Some(2), Some(false)),
        ],
    },
    MetricsRow {
        language: L::Cpp,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true)),
            ("undocumented_none", Some(0), Some(false)),
            ("variadic", Some(2), Some(false)),
        ],
    },
    MetricsRow {
        language: L::Swift,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true)),
            ("undocumentedNone", Some(0), Some(false)),
            ("variadic", Some(2), Some(false)),
        ],
    },
    MetricsRow {
        language: L::Scala,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("example.symbols.documentedPair", Some(2), Some(true)),
            ("example.symbols.undocumentedNone", Some(0), Some(false)),
            ("example.symbols.variadic", Some(2), Some(false)),
            // `def area: Int` has no parameter list at all.
            ("example.symbols.FirstShape.area", None, Some(false)),
        ],
    },
    MetricsRow {
        language: L::Php,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("Example\\Symbols\\documented_pair", Some(2), Some(true)),
            ("Example\\Symbols\\undocumented_none", Some(0), Some(false)),
            ("Example\\Symbols\\variadic", Some(2), Some(false)),
        ],
    },
    MetricsRow {
        language: L::Perl,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cases: &[("Fixture::Other::test_helper", None, None)],
    },
    MetricsRow {
        language: L::Lua,
        parameters: Support::Measured,
        docs: NO_DOCS,
        cases: &[
            ("documented_pair", Some(2), None),
            // `...` counts once.
            ("variadic", Some(2), None),
            ("test_helper", Some(0), None),
        ],
    },
    MetricsRow {
        language: L::Bash,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cases: &[("test_helper", None, None)],
    },
    MetricsRow {
        language: L::Zsh,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cases: &[("test_helper", None, None)],
    },
];

/// The languages with a metrics row, for the coverage guard.
pub fn metrics_matrix_languages() -> Vec<ProgrammingLanguage> {
    METRICS_MATRIX.iter().map(|row| row.language).collect()
}

fn metrics_records(path: &Path) -> Vec<SymbolRecord> {
    SourceFile::read(path)
        .expect("read source")
        .query(&SymbolQuery::new().capabilities([Capability::Metrics]))
        .expect("query")
        .index
        .symbols
}

fn is_function(kind: SymbolKindV2) -> bool {
    matches!(kind, SymbolKindV2::Function | SymbolKindV2::Method | SymbolKindV2::Constructor)
}

#[test]
fn metrics_match_each_fixture_function() {
    for row in METRICS_MATRIX {
        let path = primary_fixture(row.language).expect("primary fixture");
        let label = display_name(&path);
        let records = metrics_records(&path);
        assert!(!row.cases.is_empty(), "{label}: a row lists its functions");
        for (name, parameters, has_docs) in row.cases {
            let matching: Vec<&SymbolRecord> = records
                .iter()
                .filter(|record| in_file_name(record) == *name && is_function(record.kind))
                .collect();
            assert_eq!(matching.len(), 1, "{label}: one function `{name}`");
            assert_eq!(
                matching[0].metrics,
                Some(MetricsFacet { parameters: *parameters, has_docs: *has_docs }),
                "{label}: `{name}`"
            );
        }

        for record in &records {
            let name = &record.identity.qualified_name;
            let metrics = record.metrics.unwrap_or_else(|| panic!("{label}: `{name}` has no metrics"));
            if !is_function(record.kind) || row.parameters != Support::Measured {
                assert_eq!(metrics.parameters, None, "{label}: `{name}` ({:?})", record.kind);
            }
            assert_eq!(
                metrics.has_docs.is_some(),
                row.docs == Support::Measured,
                "{label}: `{name}` has_docs"
            );
        }
    }
}

/// `(source, extension, name, parameters, has_docs)`.
type FormCase = (&'static str, &'static str, &'static str, Option<u32>, Option<bool>);

/// Parameter lists and documentation forms the fixtures do not hold.
#[test]
fn metric_forms_outside_the_fixtures() {
    let cases: &[FormCase] = &[
        // A lone unparenthesized arrow parameter is a list of one.
        ("const single = x => x;\n", "js", "single", Some(1), Some(false)),
        // Python's docstring belongs to its own `def`, never to the next.
        ("class C:\n    \"\"\"Class doc.\"\"\"\n    def m(self):\n        pass\n", "py", "m", Some(1), Some(false)),
        ("class C:\n    \"\"\"Class doc.\"\"\"\n    def m(self):\n        pass\n", "py", "C", None, Some(true)),
        // An empty doc comment is no documentation.
        ("///\nfn empty_doc() {}\n", "rs", "empty_doc", Some(0), Some(false)),
        // `//!` documents the enclosing module, not the next item.
        ("//! Module docs.\nfn after_inner_doc() {}\n", "rs", "after_inner_doc", Some(0), Some(false)),
        ("/** Block doc. */\nfn block_doc() {}\n", "rs", "block_doc", Some(0), Some(true)),
        ("/* Not a doc. */\nint plain(int a) { return a; }\n", "c", "plain", Some(1), Some(false)),
        ("/*! Doxygen. */\nint bang(int a) { return a; }\n", "c", "bang", Some(1), Some(true)),
        ("/// Doxygen.\nint slashes(void) { return 0; }\n", "cpp", "slashes", Some(0), Some(true)),
        ("// Ordinary.\nfunction ordinary() {}\n", "js", "ordinary", Some(0), Some(false)),
        ("/* Not javadoc. */\nclass C { void m(int a) {} }\n", "java", "C", None, Some(false)),
        // Go documents only with a comment group that touches the declaration.
        ("package p\n\n// Detached.\n\nfunc Detached() {}\n", "go", "Detached", Some(0), Some(false)),
        ("package p\n\n// Attached.\nfunc Attached(a, b int) {}\n", "go", "Attached", Some(2), Some(true)),
        ("/// Swift docs.\nfunc swiftDoc(_ a: Int) {}\n", "swift", "swiftDoc", Some(1), Some(true)),
        ("<?php\n// Ordinary.\nfunction php_plain($a) {}\n", "php", "php_plain", Some(1), Some(false)),
        // Unnamed parameters count, though a signature cannot name them.
        ("int unnamed(int, char *);\n", "c", "unnamed", Some(2), Some(false)),
        ("int void_list(void);\n", "c", "void_list", Some(0), Some(false)),
        ("void unnamed_cpp(int, double) {}\n", "cpp", "unnamed_cpp", Some(2), Some(false)),
        ("package p\n\nfunc Unnamed(int, string) {}\n", "go", "Unnamed", Some(2), Some(false)),
        ("package p\n\nfunc Grouped(a, b int, rest ...string) {}\n", "go", "Grouped", Some(3), Some(false)),
    ];
    for (source, extension, name, parameters, has_docs) in cases {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("snippet.{extension}"));
        std::fs::write(&path, source).expect("write snippet");
        let records = metrics_records(&path);
        let record = records
            .iter()
            .find(|record| record.identity.name == *name)
            .unwrap_or_else(|| panic!("{source}: no `{name}`"));
        assert_eq!(
            record.metrics,
            Some(MetricsFacet { parameters: *parameters, has_docs: *has_docs }),
            "{source}: `{name}`"
        );
    }
}

/// An `--undocumented`-style selection (`has_docs == Some(false)`) over a
/// mixed tree lists only languages that detect docs: a Bash function's
/// unknown `has_docs` is neither documented nor undocumented.
#[test]
fn undocumented_selection_skips_languages_without_doc_detection() {
    let dir = TempDir::new().expect("temp dir");
    let files = [
        ("lib.rs", "/// Documented.\npub fn documented() {}\npub fn undocumented() {}\n"),
        ("tool.sh", "# A comment, not documentation.\nshell_function() {\n  :\n}\n"),
    ];
    let mut undocumented = Vec::new();
    for (name, source) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write source");
        for record in metrics_records(&path) {
            if record.metrics.and_then(|metrics| metrics.has_docs) == Some(false) {
                undocumented.push((name, record.identity.name));
            }
        }
    }
    assert_eq!(undocumented, [("lib.rs", "undocumented".to_string())]);
}
