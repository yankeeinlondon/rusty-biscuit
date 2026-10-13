//! The metrics matrix: `parameters`, `has_docs`, `lines` and `cyclomatic`
//! for each fixture function, `None` where a metric is unsupported, and the
//! language-wide rules behind them; `BODY_LINES_MATRIX` holds the body forms
//! `lines` is counted from, and `RUST_CYCLOMATIC`, `JAVASCRIPT_CYCLOMATIC`
//! and `TYPESCRIPT_CYCLOMATIC` the reviewed counting rules.

use std::path::Path;

use tempfile::TempDir;
use tree_hugger::{
    Capability, MetricsFacet, ProgrammingLanguage, SourceFile, SymbolKindV2, SymbolQuery,
    SymbolRecord, SymbolScope, TextSpan, TreeFile,
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

/// `(qualified name of a function, parameters, has_docs, lines, cyclomatic)`.
pub type MetricsCase = (&'static str, Option<u32>, Option<bool>, Option<u32>, Option<u32>);

pub struct MetricsRow {
    pub language: ProgrammingLanguage,
    pub parameters: Support,
    pub docs: Support,
    /// Whether the language has reviewed cyclomatic counting rules; the
    /// rules themselves are pinned by the `*_CYCLOMATIC` tables.
    pub cyclomatic: Support,
    pub cases: &'static [MetricsCase],
}

const NO_PARAMETERS: Support = Support::Unsupported("parameters are not extracted");
const NO_DOCS: Support = Support::Unsupported("doc comments are not detected");

/// No reviewed counting rules: every record reports `None`, never an
/// approximate count.
const NO_CYCLOMATIC: Support = Support::Unsupported("no reviewed cyclomatic counting rules");

pub const METRICS_MATRIX: &[MetricsRow] = &[
    MetricsRow {
        language: L::Rust,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true), Some(1), Some(1)),
            ("undocumented_none", Some(0), Some(false), Some(1), Some(1)),
            ("deprecated_api", Some(0), Some(false), Some(1), Some(1)),
            // The `&self` receiver is not counted.
            ("Receiver::with_receiver", Some(1), Some(false), Some(1), Some(1)),
            ("cfg_test_module::inside_cfg_test", Some(1), Some(false), Some(1), Some(1)),
            ("function_with_nested_items::nested_local_module::nested_module_function", Some(0), Some(false), Some(1), Some(1)),
        ],
    },
    MetricsRow {
        language: L::TypeScript,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true), Some(1), Some(1)),
            ("undocumentedNone", Some(0), Some(false), Some(1), Some(1)),
            ("variadic", Some(2), Some(false), Some(1), Some(1)),
            ("Receiver.withReceiver", Some(1), Some(true), Some(1), Some(1)),
            ("boundaryArrow", Some(1), Some(false), Some(1), Some(1)),
        ],
    },
    MetricsRow {
        language: L::JavaScript,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true), Some(1), Some(1)),
            ("undocumentedNone", Some(0), Some(false), Some(1), Some(1)),
            ("variadic", Some(2), Some(false), Some(1), Some(1)),
            ("documentedArrow", Some(1), Some(true), Some(1), Some(1)),
            ("lookAlikeTag", Some(0), Some(true), Some(1), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Go,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("symbols.DocumentedPair", Some(2), Some(true), Some(1), None),
            ("symbols.UndocumentedNone", Some(0), Some(false), Some(3), None),
            ("symbols.Variadic", Some(2), Some(false), Some(1), None),
            // The receiver has its own list and is not counted.
            ("symbols.ExportedStruct.WithReceiver", Some(1), Some(false), Some(1), None),
            ("symbols.DeprecatedLater", Some(0), Some(true), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::Python,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("documented_pair", Some(2), Some(true), Some(1), None),
            ("undocumented_none", Some(0), Some(false), Some(1), None),
            // `*rest` and `**options` count once each.
            ("variadic", Some(3), Some(false), Some(1), None),
            // `self` is an ordinary parameter in Python's syntax.
            ("Receiver.with_receiver", Some(2), Some(false), Some(1), None),
            ("outer_function.nested_function", Some(0), Some(false), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::Java,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("com.example.symbols.FlagsAndMetrics.documentedPair", Some(2), Some(true), Some(1), None),
            ("com.example.symbols.FlagsAndMetrics.undocumentedNone", Some(0), Some(false), Some(1), None),
            ("com.example.symbols.FlagsAndMetrics.deprecatedByTag", Some(0), Some(true), Some(1), None),
            ("com.example.symbols.FlagsAndMetrics.variadic", Some(2), Some(false), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::CSharp,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("Example.Symbols.FlagsAndMetrics.DocumentedPair", Some(2), Some(true), Some(1), None),
            ("Example.Symbols.FlagsAndMetrics.UndocumentedNone", Some(0), Some(false), Some(1), None),
            ("Example.Symbols.FlagsAndMetrics.Variadic", Some(2), Some(false), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::C,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("documented_pair", Some(2), Some(true), Some(1), None),
            // `(void)` is a known empty list.
            ("undocumented_none", Some(0), Some(false), Some(1), None),
            ("variadic", Some(2), Some(false), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::Cpp,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("documented_pair", Some(2), Some(true), Some(1), None),
            ("undocumented_none", Some(0), Some(false), Some(1), None),
            ("variadic", Some(2), Some(false), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::Swift,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("documentedPair", Some(2), Some(true), Some(1), None),
            ("undocumentedNone", Some(0), Some(false), Some(1), None),
            ("variadic", Some(2), Some(false), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::Scala,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("example.symbols.documentedPair", Some(2), Some(true), Some(1), None),
            ("example.symbols.undocumentedNone", Some(0), Some(false), Some(1), None),
            ("example.symbols.variadic", Some(2), Some(false), Some(1), None),
            // `def area: Int` has no parameter list at all.
            ("example.symbols.FirstShape.area", None, Some(false), Some(1), None),
        ],
    },
    MetricsRow {
        language: L::Php,
        parameters: Support::Measured,
        docs: Support::Measured,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("Example\\Symbols\\documented_pair", Some(2), Some(true), Some(2), None),
            ("Example\\Symbols\\undocumented_none", Some(0), Some(false), Some(2), None),
            ("Example\\Symbols\\variadic", Some(2), Some(false), Some(2), None),
        ],
    },
    MetricsRow {
        language: L::Perl,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[("Fixture::Other::test_helper", None, None, Some(1), None)],
    },
    MetricsRow {
        language: L::Lua,
        parameters: Support::Measured,
        docs: NO_DOCS,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[
            ("documented_pair", Some(2), None, Some(0), None),
            // `...` counts once.
            ("variadic", Some(2), None, Some(0), None),
            ("test_helper", Some(0), None, Some(0), None),
        ],
    },
    MetricsRow {
        language: L::Bash,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[("test_helper", None, None, Some(3), None)],
    },
    MetricsRow {
        language: L::Zsh,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cyclomatic: NO_CYCLOMATIC,
        cases: &[("test_helper", None, None, Some(3), None)],
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
        for (name, parameters, has_docs, lines, cyclomatic) in row.cases {
            let matching: Vec<&SymbolRecord> = records
                .iter()
                .filter(|record| in_file_name(record) == *name && is_function(record.kind))
                .collect();
            assert_eq!(matching.len(), 1, "{label}: one function `{name}`");
            let metrics = matching[0].metrics.unwrap_or_else(|| panic!("{label}: `{name}` has no metrics"));
            assert_eq!(
                (metrics.parameters, metrics.has_docs, metrics.lines, metrics.cyclomatic),
                (*parameters, *has_docs, *lines, *cyclomatic),
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
            // `lines` is read from the stored body span, so the two never
            // disagree; other kinds have none even with a body.
            let expected_lines = is_function(record.kind)
                .then(|| record.source.body_span.as_ref().map(span_lines))
                .flatten();
            assert_eq!(metrics.lines, expected_lines, "{label}: `{name}` ({:?}) lines", record.kind);
            // Only reviewed rules measure, and only a callable's body; the
            // reviewed rules measure every callable body of a clean fixture.
            let measured = row.cyclomatic == Support::Measured
                && is_function(record.kind)
                && record.source.body_span.is_some();
            assert_eq!(metrics.cyclomatic.is_some(), measured, "{label}: `{name}` ({:?}) cyclomatic", record.kind);
        }
    }
}

/// The spec's count of physical lines a nonempty span touches, with
/// one-based locations; `0` for an empty span.
fn span_lines(span: &TextSpan) -> u32 {
    if span.start_byte == span.end_byte {
        return 0;
    }
    span.end.line - span.start.line + u32::from(span.end.column > 1)
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
        // A parameter bound by a pattern counts like one bound by a name.
        ("pub fn patterned((a, b): (u32, u32), _: String) {}\n", "rs", "patterned", Some(2), Some(false)),
        ("struct T;\nimpl T { fn boxed(self: Box<Self>, n: u8) {} }\n", "rs", "boxed", Some(1), Some(false)),
        ("extern \"C\" { fn vlog(format: *const u8, ...); }\n", "rs", "vlog", Some(2), Some(false)),
        ("export function patterned({x}: {x:number}, [a,b]: number[]) {}\n", "ts", "patterned", Some(2), Some(false)),
        ("function restPattern(d = 1, ...[r, s]: number[]) {}\n", "ts", "restPattern", Some(2), Some(false)),
        // A TypeScript `this` parameter is not counted.
        ("function withThis(this: Window, a: number) {}\n", "ts", "withThis", Some(1), Some(false)),
        ("function patterned({x}, [a,b]) {}\n", "js", "patterned", Some(2), Some(false)),
        ("function defaultsAndRest({y} = {}, ...[r, s]) {}\n", "js", "defaultsAndRest", Some(2), Some(false)),
        // Every list of a curried `def` counts, `using`/`implicit` included.
        ("object O {\n  def patterned(a: Int)(b: String): Int = a\n}\n", "scala", "patterned", Some(2), Some(false)),
        ("object O {\n  def contextual(a: Int)(b: Int)(implicit c: Ctx): Int = a\n}\n", "scala", "contextual", Some(3), Some(false)),
        // Sibling shapes already measured: Python separators and splats,
        // PHP promoted and variadic parameters, C# `params`, a Java receiver.
        ("def py_shapes(a, /, b, *, c: str, d=1, *args, **kw):\n    pass\n", "py", "py_shapes", Some(6), Some(false)),
        ("<?php\nclass A { function __construct(private int $p, string ...$rest) {} }\n", "php", "__construct", Some(2), Some(false)),
        ("class A { void Cs(int a, params string[] b, ref int c) {} }\n", "cs", "Cs", Some(3), Some(false)),
        ("class A { void j(A this, int a, String... b) {} }\n", "java", "j", Some(2), Some(false)),
        ("func swiftShapes(_ a: Int, b c: String, d: Int...) {}\n", "swift", "swiftShapes", Some(3), Some(false)),
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
        let metrics = record.metrics.unwrap_or_else(|| panic!("{source}: `{name}` has no metrics"));
        // `lines` is `BODY_LINES_MATRIX`'s.
        assert_eq!((metrics.parameters, metrics.has_docs), (*parameters, *has_docs), "{source}: `{name}`");
    }
}

/// A Python docstring is the first statement of a `def` or `class` body
/// when it is a nonempty `str` constant: ordinary literals (any quoting,
/// `r`/`u` prefixes) and their adjacent concatenation. Bytes and formatted
/// literals are expressions; emptiness is read from the source, never by
/// evaluating escapes. Each form is checked on a function and a class.
#[test]
fn python_docstrings_are_nonempty_string_constants() {
    let forms: &[(&str, bool)] = &[
        (r#""docs""#, true),
        (r#"'docs'"#, true),
        (r#""""Triple.""""#, true),
        (r#"r"raw \d""#, true),
        (r#"R"raw""#, true),
        (r#"u"unicode""#, true),
        (r#""A""#, true),
        (r#""doc" "umented""#, true),
        (r#""" "umented""#, true),
        (r#"("parenthesized")"#, true),
        (r#"f"docs""#, false),
        (r#"F"docs""#, false),
        (r#"rf"docs""#, false),
        (r#"fR"docs""#, false),
        (r#"t"docs""#, false),
        (r#"b"docs""#, false),
        (r#"rb"docs""#, false),
        (r#"Br"docs""#, false),
        (r#""doc" f"umented""#, false),
        (r#""doc" b"umented""#, false),
        (r#""""#, false),
        (r#""" """#, false),
        (r#""   ""#, false),
        (r#"""" """"#, false),
        ("1", false),
    ];
    let dir = TempDir::new().expect("temp dir");
    for (index, (literal, documented)) in forms.iter().enumerate() {
        for (keyword, name, kind) in [
            ("def", "function", SymbolKindV2::Function),
            ("class", "Class", SymbolKindV2::Class),
        ] {
            let header = if keyword == "def" { format!("def {name}():") } else { format!("class {name}:") };
            let source = format!("{header}\n    {literal}\n    pass\n");
            let path = dir.path().join(format!("docs_{index}_{keyword}.py"));
            std::fs::write(&path, &source).expect("write snippet");
            let records = metrics_records(&path);
            let record = records
                .iter()
                .find(|record| record.identity.name == name && record.kind == kind)
                .unwrap_or_else(|| panic!("{source}: no `{name}`"));
            assert_eq!(
                record.metrics.and_then(|metrics| metrics.has_docs),
                Some(*documented),
                "{source}"
            );
        }
    }
}

/// Documentation comments are decided by their opening delimiter, and an
/// empty one documents nothing.
#[test]
fn doc_comment_delimiters_at_their_edges() {
    let cases: &[(&str, &str, &str, bool)] = &[
        ("/***/\nfn stars() {}\n", "rs", "stars", false),
        ("/**/\nfn empty_block() {}\n", "rs", "empty_block", false),
        ("/** */\nfn blank_block() {}\n", "rs", "blank_block", false),
        ("//// Four slashes.\nfn four() {}\n", "rs", "four", false),
        ("/***/\nclass Stars {}\n", "java", "Stars", false),
        ("/**/\nfunction emptyBlock() {}\n", "ts", "emptyBlock", false),
        ("/** Doc. */\nfunction documented() {}\n", "js", "documented", true),
        ("///\nclass A { void Empty() {} }\n", "cs", "A", false),
        ("/**/\nint empty_c(void) { return 0; }\n", "c", "empty_c", false),
        ("//!\nint empty_bang(void) { return 0; }\n", "cpp", "empty_bang", false),
        ("package p\n\n//\nfunc EmptyGo() {}\n", "go", "EmptyGo", false),
    ];
    for (source, extension, name, documented) in cases {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("snippet.{extension}"));
        std::fs::write(&path, source).expect("write snippet");
        let records = metrics_records(&path);
        let record = records
            .iter()
            .find(|record| record.identity.name == *name)
            .unwrap_or_else(|| panic!("{source}: no `{name}`"));
        assert_eq!(
            record.metrics.and_then(|metrics| metrics.has_docs),
            Some(*documented),
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

/// One documentation shape of the block-decoration matrix: the comment
/// (written at the declaration's indentation), then the expected
/// `(has_docs, raw doc)` under a doc tool that removes a block comment's
/// leading stars only when they are consistent (Rust, C#, Swift) and under
/// one that removes a leading star from every line (Java, JavaScript,
/// TypeScript, Scala, PHP, C, C++). A `None` raw doc is not asserted.
struct DecorationShape {
    name: &'static str,
    doc: &'static str,
    consistent: (bool, Option<&'static str>),
    per_line: (bool, Option<&'static str>),
}

const DECORATION_SHAPES: &[DecorationShape] = &[
    DecorationShape { name: "Blank", doc: "/** */", consistent: (false, None), per_line: (false, None) },
    DecorationShape {
        name: "Aligned",
        doc: "/**\n *\n *\n */",
        consistent: (false, None),
        per_line: (false, None),
    },
    DecorationShape {
        name: "Misaligned",
        doc: "/**\n *\n  *\n*/",
        consistent: (true, None),
        per_line: (false, None),
    },
    DecorationShape {
        name: "TabSpace",
        doc: "/**\n *\n\t*\n*/",
        consistent: (true, None),
        per_line: (false, None),
    },
    DecorationShape {
        name: "SingleStar",
        doc: "/** * */",
        consistent: (true, Some("*")),
        per_line: (false, None),
    },
    DecorationShape {
        name: "MixedProse",
        doc: "/**\nIntro\n *b* is next\n*/",
        consistent: (true, Some("Intro\n*b* is next")),
        per_line: (true, Some("Intro\nb* is next")),
    },
    DecorationShape {
        name: "Plain",
        doc: "/** Plain doc. */",
        consistent: (true, Some("Plain doc.")),
        per_line: (true, Some("Plain doc.")),
    },
    DecorationShape {
        name: "Decorated",
        doc: "/**\n * Starred text.\n */",
        consistent: (true, Some("Starred text.")),
        per_line: (true, Some("Starred text.")),
    },
    // `per_line` is for a language with `///` docs (C, C++); the other
    // per-line languages have no `///` form and report it undocumented.
    DecorationShape { name: "LineStar", doc: "/// *", consistent: (true, Some("*")), per_line: (true, Some("*")) },
];

/// One language of the decoration matrix: its file extension, the source
/// around the declarations (`{decls}`), each declaration family as a
/// template (`{doc}` at column `indent`, `{S}` the shape name), and which
/// decoration rule its doc tool follows.
struct DecorationLanguage {
    extension: &'static str,
    wrap: fn(&str) -> String,
    families: &'static [(&'static str, usize, &'static str)],
    consistent: bool,
    triple_slash: bool,
}

const DECORATION_LANGUAGES: &[DecorationLanguage] = &[
    DecorationLanguage {
        extension: "rs",
        wrap: |decls| decls.to_string(),
        families: &[
            ("fn_{S}", 0, "{doc}\npub fn fn_{S}() {}\n"),
            ("Type{S}", 0, "{doc}\npub struct Type{S} {\n{field}}\n"),
            ("field_{S}", 4, "{doc}\n    pub field_{S}: i32,\n"),
            ("method_{S}", 4, "impl Type{S} {\n{doc}\n    pub fn method_{S}(&self) {}\n}\n"),
            ("CONST_{S}", 0, "{doc}\npub const CONST_{S}: i32 = 1;\n"),
            ("STATIC_{S}", 0, "{doc}\npub static STATIC_{S}: i32 = 1;\n"),
        ],
        consistent: true,
        triple_slash: true,
    },
    DecorationLanguage {
        extension: "cs",
        wrap: |decls| decls.to_string(),
        families: &[
            ("Class{S}", 0, "{doc}\npublic class Class{S} {\n{field}}\n"),
            ("Method{S}", 4, "{doc}\n    public void Method{S}() {}\n"),
            ("field{S}", 4, "{doc}\n    public int field{S};\n"),
            ("Property{S}", 4, "{doc}\n    public int Property{S} { get; set; }\n"),
        ],
        consistent: true,
        triple_slash: true,
    },
    DecorationLanguage {
        extension: "swift",
        wrap: |decls| decls.to_string(),
        families: &[
            ("func{S}", 0, "{doc}\nfunc func{S}() {}\n"),
            ("Type{S}", 0, "{doc}\nstruct Type{S} {\n{field}}\n"),
            ("method{S}", 4, "{doc}\n    func method{S}() {}\n"),
            ("stored{S}", 4, "{doc}\n    var stored{S}: Int = 1\n"),
            ("let{S}", 0, "{doc}\nlet let{S} = 1\n"),
            ("var{S}", 0, "{doc}\nvar var{S} = 1\n"),
        ],
        consistent: true,
        triple_slash: true,
    },
    DecorationLanguage {
        extension: "java",
        wrap: |decls| format!("class JavaHolder {{\n{decls}}}\n"),
        families: &[("method{S}", 4, "{doc}\n    void method{S}() {}\n")],
        consistent: false,
        triple_slash: false,
    },
    DecorationLanguage {
        extension: "js",
        wrap: |decls| decls.to_string(),
        families: &[("fn{S}", 0, "{doc}\nfunction fn{S}() {}\n")],
        consistent: false,
        triple_slash: false,
    },
    DecorationLanguage {
        extension: "ts",
        wrap: |decls| decls.to_string(),
        families: &[("fn{S}", 0, "{doc}\nfunction fn{S}(): void {}\n")],
        consistent: false,
        triple_slash: false,
    },
    DecorationLanguage {
        extension: "scala",
        wrap: |decls| format!("object ScalaHolder {{\n{decls}}}\n"),
        families: &[("method{S}", 2, "{doc}\n  def method{S}: Int = 1\n")],
        consistent: false,
        triple_slash: false,
    },
    DecorationLanguage {
        extension: "php",
        wrap: |decls| format!("<?php\n{decls}"),
        families: &[("fn{S}", 0, "{doc}\nfunction fn{S}() {}\n")],
        consistent: false,
        triple_slash: false,
    },
    DecorationLanguage {
        extension: "c",
        wrap: |decls| decls.to_string(),
        families: &[("fn{S}", 0, "{doc}\nint fn{S}(void) { return 0; }\n")],
        consistent: false,
        triple_slash: true,
    },
    DecorationLanguage {
        extension: "cpp",
        wrap: |decls| decls.to_string(),
        families: &[("fn{S}", 0, "{doc}\nint fn{S}(void) { return 0; }\n")],
        consistent: false,
        triple_slash: true,
    },
];

/// `doc` with every line indented by `indent` spaces, so the comment sits
/// at the declaration's column and keeps its own relative layout.
fn indented(doc: &str, indent: usize) -> String {
    let pad = " ".repeat(indent);
    doc.lines().map(|line| format!("{pad}{line}")).collect::<Vec<_>>().join("\n")
}

/// The source of one decoration-matrix file: every family of `language`
/// documented by every shape. A family whose template holds `{field}` is a
/// container and receives the shape's member families (indent above 0).
fn decoration_source(language: &DecorationLanguage) -> String {
    let containers: Vec<_> = language.families.iter().filter(|(_, _, template)| template.contains("{field}")).collect();
    let mut decls = String::new();
    for shape in DECORATION_SHAPES {
        let render = |(_, indent, template): &(&str, usize, &str)| {
            template.replace("{doc}", &indented(shape.doc, *indent)).replace("{S}", shape.name)
        };
        let members: String = language
            .families
            .iter()
            .filter(|(_, indent, template)| *indent > 0 && !template.starts_with("impl"))
            .map(render)
            .collect();
        for family in language.families {
            if containers.iter().any(|container| container.0 == family.0) {
                decls.push_str(&render(family).replace("{field}", &members));
            } else if family.1 == 0 || family.2.starts_with("impl") || containers.is_empty() {
                decls.push_str(&render(family));
            }
        }
    }
    (language.wrap)(&decls)
}

/// Rust (rustdoc), C# (the compiler's common pattern) and Swift (its
/// comment-column decoration) remove a block comment's leading stars only
/// when they are consistent, so a star-only block indented differently, a
/// tab against spaces, a first-line star and a star in unstarred prose are
/// documentation text, at every declaration family. Java, JavaScript,
/// TypeScript, Scala, PHP, C and C++ keep their per-line convention: one
/// leading star is decoration on every line. `/// *` documents in every
/// language with `///` docs.
#[test]
fn block_decoration_is_removed_only_where_the_doc_tool_removes_it() {
    let mut mismatches = Vec::new();
    for language in DECORATION_LANGUAGES {
        let source = decoration_source(language);
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("decoration.{}", language.extension));
        std::fs::write(&path, &source).expect("write source");
        let errors = tree_hugger::TreeFile::new(&path).expect("parse").syntax_diagnostics();
        assert!(errors.is_empty(), "{}: {errors:?}\n{source}", language.extension);
        let records = metrics_records(&path);
        for shape in DECORATION_SHAPES {
            let (has_docs, raw) = match (language.consistent, shape.name) {
                (false, "LineStar") if !language.triple_slash => (false, None),
                (true, _) => shape.consistent,
                (false, _) => shape.per_line,
            };
            for (template, _, _) in language.families {
                let name = template.replace("{S}", shape.name);
                let Some(record) = records.iter().find(|record| record.identity.name == name) else {
                    mismatches.push(format!("{}: no `{name}`", language.extension));
                    continue;
                };
                let actual = record.metrics.and_then(|metrics| metrics.has_docs);
                if actual != Some(has_docs) {
                    mismatches.push(format!("{}: `{name}` has_docs {actual:?}, expected {has_docs}", language.extension));
                }
                if let Some(raw) = raw
                    && record.docs.raw_doc.as_deref() != Some(raw)
                {
                    mismatches.push(format!(
                        "{}: `{name}` raw doc {:?}, expected {raw:?}",
                        language.extension, record.docs.raw_doc
                    ));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// The docs pass reads the extractor's cleaned doc: a star or `//` left at
/// the start of a line is text, so the summary keeps it (Markdown emphasis
/// and bold, a Go list item) in every language family.
#[test]
fn structured_doc_summaries_keep_literal_leading_stars() {
    let cases: &[(&str, &str, &str, &str)] = &[
        ("/**\nIntro\n *b* is next\n*/\npub fn mixed() {}\n", "rs", "mixed", "Intro *b* is next"),
        ("/// **Bold** text\npub fn bold() {}\n", "rs", "bold", "**Bold** text"),
        ("/// * listed\nclass A { void Listed() {} }\n", "cs", "A", "* listed"),
        ("/// *only*\nfunc only() {}\n", "swift", "only", "*only*"),
        ("/**\n * **Bold** text\n */\nfunction bold() {}\n", "js", "bold", "**Bold** text"),
        ("/// // not a marker\nint slashes(void) { return 0; }\n", "c", "slashes", "// not a marker"),
        ("package p\n\n// * item\nfunc Item() {}\n", "go", "Item", "* item"),
    ];
    let query = SymbolQuery::new().capabilities([Capability::StructuredDocs]);
    for (source, extension, name, summary) in cases {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("snippet.{extension}"));
        std::fs::write(&path, source).expect("write snippet");
        let records = SourceFile::read(&path).expect("read").query(&query).expect("query").index.symbols;
        let record = records
            .iter()
            .find(|record| record.identity.name == *name)
            .unwrap_or_else(|| panic!("{source}: no `{name}`"));
        let actual = record.docs.parsed.as_ref().and_then(|parsed| parsed.summary.as_deref());
        assert_eq!(actual, Some(*summary), "{extension}: `{name}`");
    }
}

/// What a [`BodyCase`] expects of a record's `source.body_span` and
/// `metrics.lines`.
#[derive(Debug, Clone, Copy)]
pub enum Body {
    /// A function, method or constructor whose body span holds exactly this
    /// text (LF line endings), counting `lines`.
    Text(SymbolKindV2, &'static str, u32),
    /// A function, method or constructor without an implementation body:
    /// no span, `lines` is `None`.
    Absent(SymbolKindV2),
    /// A declaration that is not callable but has a body span (a type, a
    /// module): `lines` is still `None`.
    NotCallable,
}

/// `(in-file qualified name, expectation)`.
pub type BodyCase = (&'static str, Body);

pub struct BodyLinesRow {
    pub language: ProgrammingLanguage,
    pub extension: &'static str,
    pub source: &'static str,
    pub cases: &'static [BodyCase],
}

use Body::{Absent, NotCallable, Text};
use SymbolKindV2 as K;

/// The body each language's callables are measured from: block and
/// expression bodies, nested and returned callables that are not the body,
/// defaults and types that are not the body, declarations without one, and
/// non-callable declarations that have a body span. Every source is also
/// checked with CRLF line endings.
pub const BODY_LINES_MATRIX: &[BodyLinesRow] = &[
    BodyLinesRow {
        language: L::Rust,
        extension: "rs",
        source: "fn one() {}\n\nfn multi() {\n    // a comment\n\n    fn nested() {}\n    let x = 1;\n}\n\nfn returns() -> Box<dyn Fn() -> u8> { Box::new(|| 1) }\n\ntrait T {\n    fn required(&self);\n    fn provided(&self) {}\n}\n\nmod m {\n}\n",
        cases: &[
            ("one", Text(K::Function, "{}", 1)),
            ("multi", Text(K::Function, "{\n    // a comment\n\n    fn nested() {}\n    let x = 1;\n}", 6)),
            ("multi::nested", Text(K::Function, "{}", 1)),
            ("returns", Text(K::Function, "{ Box::new(|| 1) }", 1)),
            ("T::required", Absent(K::Method)),
            ("T::provided", Text(K::Method, "{}", 1)),
            ("T", NotCallable),
            ("m", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::TypeScript,
        extension: "ts",
        source: "export const arrow = (x: number): number => x * 2;\nconst curried = () => () => 1;\nfunction withDefault(cb = () => 3): number {\n  return cb();\n}\nabstract class A {\n  abstract m(): void;\n  n(): { a: number } { return { a: 1 }; }\n}\ninterface I {\n  i(): void;\n}\n",
        cases: &[
            ("arrow", Text(K::Function, "x * 2", 1)),
            // The returned arrow is the expression body itself.
            ("curried", Text(K::Function, "() => 1", 1)),
            ("withDefault", Text(K::Function, "{\n  return cb();\n}", 3)),
            ("A.m", Absent(K::Method)),
            // An object return type is not the body.
            ("A.n", Text(K::Method, "{ return { a: 1 }; }", 1)),
            ("I.i", Absent(K::Method)),
            ("A", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::JavaScript,
        extension: "js",
        source: "const arrow = async x => ({ x });\nfunction multi() {\n  return 1;\n}\nclass K {\n  m() {}\n}\n",
        cases: &[
            ("arrow", Text(K::Function, "({ x })", 1)),
            ("multi", Text(K::Function, "{\n  return 1;\n}", 3)),
            ("K.m", Text(K::Method, "{}", 1)),
            ("K", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::Go,
        extension: "go",
        source: "package p\n\nfunc F() {}\n\nfunc G() func() int {\n\treturn func() int { return 1 }\n}\n\ntype S struct {\n\tA int\n}\n\nfunc (s S) M() {\n}\n",
        cases: &[
            ("p.F", Text(K::Function, "{}", 1)),
            // The returned literal's body is inside, not instead of, `G`'s.
            ("p.G", Text(K::Function, "{\n\treturn func() int { return 1 }\n}", 3)),
            ("p.S.M", Text(K::Method, "{\n}", 2)),
        ],
    },
    BodyLinesRow {
        language: L::Python,
        extension: "py",
        source: "def with_default(x=lambda: 1):\n    # leading\n    return x\n\ndef one(): pass\n\n@decorator\ndef decorated():\n    pass\n\nclass C:\n    def m(self): ...\n",
        cases: &[
            // The default `lambda` is not the body; the comment the grammar
            // leaves before the body's block is part of it.
            ("with_default", Text(K::Function, "# leading\n    return x", 2)),
            ("one", Text(K::Function, "pass", 1)),
            ("decorated", Text(K::Function, "pass", 1)),
            ("C.m", Text(K::Method, "...", 1)),
            ("C", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::Java,
        extension: "java",
        source: "interface I { void m(); default void n() {} }\nabstract class A {\n  abstract void q();\n  A() {\n  }\n}\n",
        cases: &[
            ("I.m", Absent(K::Method)),
            ("I.n", Text(K::Method, "{}", 1)),
            ("A.q", Absent(K::Method)),
            ("A.A", Text(K::Constructor, "{\n  }", 2)),
            ("A", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::CSharp,
        extension: "cs",
        source: "abstract class A {\n  int x;\n  int M() => 1;\n  A() => x = 1;\n  abstract void Q();\n  void N()\n  {\n  }\n  static int L() { int Local() => 3; return Local(); }\n}\n",
        cases: &[
            // An expression body is its expression, without `=>`.
            ("A.M", Text(K::Method, "1", 1)),
            ("A.A", Text(K::Constructor, "x = 1", 1)),
            ("A.Q", Absent(K::Method)),
            ("A.N", Text(K::Method, "{\n  }", 2)),
            ("A.L", Text(K::Method, "{ int Local() => 3; return Local(); }", 1)),
            ("A.L.Local", Text(K::Function, "3", 1)),
            ("A", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::C,
        extension: "c",
        source: "int add(int a, int b)\n{\n  return a + b;\n}\nint proto(int a);\nint (*get(void))(int) { return 0; }\nstruct S {\n  int a;\n};\n",
        cases: &[
            ("add", Text(K::Function, "{\n  return a + b;\n}", 3)),
            ("proto", Absent(K::Function)),
            // A function returning a function pointer has its own body.
            ("get", Text(K::Function, "{ return 0; }", 1)),
            ("S", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::Cpp,
        extension: "cpp",
        source: "struct C {\n  C() = default;\n  virtual void v() = 0;\n  void m();\n  int f() { return 1; }\n};\nstruct D {\n  D(int) = delete;\n};\nvoid C::m() {\n}\ntemplate <typename T> T id(T x) { return x; }\nint tryf() try { return 1; } catch (...) { return 0; }\n",
        cases: &[
            ("C::C", Absent(K::Constructor)),
            ("D::D", Absent(K::Constructor)),
            ("C::v", Absent(K::Method)),
            ("C::m", Absent(K::Method)),
            ("C::f", Text(K::Method, "{ return 1; }", 1)),
            // The out-of-class definition (a container-less function).
            ("m", Text(K::Function, "{\n}", 2)),
            ("id", Text(K::Function, "{ return x; }", 1)),
            ("tryf", Text(K::Function, "try { return 1; } catch (...) { return 0; }", 1)),
            ("C", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::Swift,
        extension: "swift",
        source: "protocol P { func m() }\nstruct S {\n  init() {}\n  func g(cb: () -> Int = { 1 }) -> Int {\n    cb()\n  }\n}\n",
        cases: &[
            ("P.m", Absent(K::Method)),
            ("S.init", Text(K::Constructor, "{}", 1)),
            // The default closure is not the body.
            ("S.g", Text(K::Method, "{\n    cb()\n  }", 3)),
            ("S", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::Scala,
        extension: "scala",
        source: "object O {\n  def f = 1\n  def g(x: Int = 2): Int = {\n    x\n  }\n  def h: Int\n}\ntrait T { def a(): Int }\n\ndef braceless(x: Int): Int =\n  val y = x\n  y\n\ndef after = 1\n",
        cases: &[
            // An expression body is its expression, without `=`.
            ("O.f", Text(K::Method, "1", 1)),
            ("O.g", Text(K::Method, "{\n    x\n  }", 3)),
            ("O.h", Absent(K::Method)),
            ("T.a", Absent(K::Method)),
            // The grammar's indented block runs over the blank line after
            // it, which is not counted.
            ("braceless", Text(K::Function, "val y = x\n  y", 2)),
            ("O", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::Php,
        extension: "php",
        source: "<?php\ninterface I { function m(); }\nfunction f()\n{\n}\nabstract class A { abstract function q(); }\n",
        cases: &[
            ("I::m", Absent(K::Method)),
            ("f", Text(K::Function, "{\n}", 2)),
            ("A::q", Absent(K::Method)),
            ("A", NotCallable),
        ],
    },
    BodyLinesRow {
        language: L::Perl,
        extension: "pl",
        source: "sub f { 1 }\nsub g {\n  my $x = 1;\n\n  return $x;\n}\n",
        cases: &[
            ("f", Text(K::Function, "{ 1 }", 1)),
            ("g", Text(K::Function, "{\n  my $x = 1;\n\n  return $x;\n}", 5)),
        ],
    },
    BodyLinesRow {
        language: L::Lua,
        extension: "lua",
        source: "local function f(a) return a end\nlocal function empty() end\nfunction c()\n  -- only a comment\nend\nfunction d()\n  -- leading\n  return 1\n  -- trailing\nend\n",
        cases: &[
            ("f", Text(K::Function, "return a", 1)),
            // An empty body is an empty span, not a missing one.
            ("empty", Text(K::Function, "", 0)),
            // Comments the grammar leaves beside the body's block are in it.
            ("c", Text(K::Function, "-- only a comment", 1)),
            ("d", Text(K::Function, "-- leading\n  return 1\n  -- trailing", 3)),
        ],
    },
    BodyLinesRow {
        language: L::Bash,
        extension: "sh",
        source: "f() ( echo sub )\ng() {\n  :\n}\n",
        cases: &[
            ("f", Text(K::Function, "( echo sub )", 1)),
            ("g", Text(K::Function, "{\n  :\n}", 3)),
        ],
    },
    BodyLinesRow {
        language: L::Zsh,
        extension: "zsh",
        source: "outer() {\n  inner() { :; }\n}\n",
        cases: &[
            ("outer", Text(K::Function, "{\n  inner() { :; }\n}", 3)),
            ("outer.inner", Text(K::Function, "{ :; }", 1)),
        ],
    },
];

/// The languages with a body-lines row, for the coverage guard.
pub fn body_lines_matrix_languages() -> Vec<ProgrammingLanguage> {
    BODY_LINES_MATRIX.iter().map(|row| row.language).collect()
}

crate::language_cases::language_tests!(body_lines_are_counted_from_the_callables_own_body => check_body_lines_are_counted_from_the_callables_own_body);

/// Each case's `source.body_span` and `metrics.lines` agree, on a
/// metrics-only query and on the complete index, with LF and with CRLF
/// line endings.
fn check_body_lines_are_counted_from_the_callables_own_body(language: ProgrammingLanguage) {
    let dir = TempDir::new().expect("temp dir");
    for row in BODY_LINES_MATRIX.iter().filter(|row| row.language == language) {
        for (ending, source) in [("LF", row.source.to_string()), ("CRLF", row.source.replace('\n', "\r\n"))] {
            let path = dir.path().join(format!("body_{:?}_{ending}.{}", row.language, row.extension));
            std::fs::write(&path, &source).expect("write snippet");
            let selective = metrics_records(&path);
            let complete = SourceFile::read(&path)
                .expect("read source")
                .query(&SymbolQuery::complete_index())
                .expect("query")
                .index
                .symbols;
            let errors = TreeFile::new(&path).expect("parse").syntax_diagnostics();
            assert!(errors.is_empty(), "{:?} {ending}: the snippet parses cleanly: {errors:?}", row.language);
            for records in [&selective, &complete] {
                for (name, body) in row.cases {
                    let label = format!("{:?} {ending} `{name}`", row.language);
                    let callable = !matches!(body, NotCallable);
                    let matching: Vec<&SymbolRecord> = records
                        .iter()
                        .filter(|record| in_file_name(record) == *name && is_function(record.kind) == callable)
                        .collect();
                    assert_eq!(matching.len(), 1, "{label}: one record");
                    let record = matching[0];
                    let lines = record.metrics.unwrap_or_else(|| panic!("{label}: no metrics")).lines;
                    let text = record.source.body_span.as_ref().map(|span| {
                        source[span.start_byte as usize..span.end_byte as usize].replace("\r\n", "\n")
                    });
                    match *body {
                        Text(kind, expected, count) => {
                            assert_eq!(record.kind, kind, "{label}: kind");
                            assert_eq!(text.as_deref(), Some(expected), "{label}: body");
                            assert_eq!(lines, Some(count), "{label}: lines");
                        }
                        Absent(kind) => {
                            assert_eq!(record.kind, kind, "{label}: kind");
                            assert_eq!(text, None, "{label}: body");
                            assert_eq!(lines, None, "{label}: lines");
                        }
                        NotCallable => {
                            assert!(text.is_some(), "{label}: has a body span");
                            assert_eq!(lines, None, "{label}: lines");
                        }
                    }
                }
            }
        }
    }
}

/// Inside a present `metrics` facet `lines` is always a key, `null` when
/// unsupported; without `Capability::Metrics` the whole facet is absent.
#[test]
fn lines_key_is_present_null_or_absent_with_its_facet() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("Shape.java");
    std::fs::write(&path, "interface Shape {\n  double area();\n  default int sides() {\n    return 0;\n  }\n}\n")
        .expect("write snippet");
    let json = |query: &SymbolQuery, name: &str| -> serde_json::Value {
        let records = SourceFile::read(&path).expect("read source").query(query).expect("query").index.symbols;
        let record = records.iter().find(|record| record.identity.name == name).expect("record");
        serde_json::to_value(record).expect("serialize")
    };
    let metrics = SymbolQuery::new().capabilities([Capability::Metrics]);
    assert_eq!(
        json(&metrics, "area")["metrics"],
        serde_json::json!({ "parameters": 0, "has_docs": false, "lines": null, "cyclomatic": null })
    );
    assert_eq!(
        json(&metrics, "sides")["metrics"],
        serde_json::json!({ "parameters": 0, "has_docs": false, "lines": 3, "cyclomatic": null })
    );
    assert_eq!(
        json(&metrics, "Shape")["metrics"],
        serde_json::json!({ "parameters": null, "has_docs": false, "lines": null, "cyclomatic": null })
    );
    for name in ["area", "sides", "Shape"] {
        let record = json(&SymbolQuery::new(), name);
        assert!(record.get("metrics").is_none(), "`{name}`: no facet without Metrics: {record}");
    }
}

/// `(name, cyclomatic)` of one function in a [`CyclomaticRow`].
type CyclomaticCase = (&'static str, Option<u32>);

/// A clean source and the cyclomatic value of each named function in it.
struct CyclomaticRow {
    source: &'static str,
    cases: &'static [CyclomaticCase],
}

/// Every Rust counting rule with its exact total, plus the forms that
/// contribute 0: plain `else`, `loop`, `?`, non-short-circuit operators,
/// or-patterns, macro token trees, comments, strings, decisions outside the
/// body (return type, attributes) and nested callables, closures and items.
const RUST_CYCLOMATIC: &[CyclomaticRow] = &[
    CyclomaticRow { source: "fn empty() {}\n", cases: &[("empty", Some(1))] },
    CyclomaticRow {
        source: "fn else_if(a: bool) { if a {} else if !a {} else {} }\n",
        cases: &[("else_if", Some(3))],
    },
    CyclomaticRow {
        source: "fn short_circuit(a: bool, b: bool) -> bool { a && b || !a }\n",
        cases: &[("short_circuit", Some(3))],
    },
    CyclomaticRow {
        source: "fn bitwise(mut a: bool, b: bool) -> bool { a &= b; a |= b; a & b | b }\n",
        cases: &[("bitwise", Some(1))],
    },
    // Two non-last arms (the or-pattern is one) and one guard.
    CyclomaticRow {
        source: "fn arms(x: Option<i32>) -> i32 { match x { Some(1) | Some(2) => 1, Some(n) if n > 3 => 2, _ => 3 } }\n",
        cases: &[("arms", Some(4))],
    },
    // The last arm is the default whatever its pattern: a two-arm `match`
    // scores like `if`/`else`.
    CyclomaticRow {
        source: "fn two_arms(x: bool) -> i32 { match x { true => 1, false => 2 } }\n",
        cases: &[("two_arms", Some(2))],
    },
    // An or-pattern is one arm; a match's arms are never grouped across
    // `,`, so the last-arm convention holds whatever the patterns.
    CyclomaticRow {
        source: "fn grouped_pattern(x: u8) -> u8 { match x { 1 | 2 => 1, _ => 2 } }\n\
                 fn wildcard_first(x: u8) -> u8 { match x { _ if x > 1 => 1, 1 | 2 => 2, _ => 3 } }\n",
        cases: &[("grouped_pattern", Some(2)), ("wildcard_first", Some(4))],
    },
    CyclomaticRow {
        source: "fn guard_and(x: Option<i32>) -> i32 { match x { Some(n) if n > 3 && n < 9 => 1, _ => 0 } }\n",
        cases: &[("guard_and", Some(4))],
    },
    CyclomaticRow {
        source: "fn let_else(x: Option<i32>) { let Some(_y) = x else { return }; let _z = 1; }\n",
        cases: &[("let_else", Some(2))],
    },
    CyclomaticRow {
        source: "fn chain(x: Option<i32>, a: bool) { if let Some(_v) = x && a {} }\n",
        cases: &[("chain", Some(3))],
    },
    CyclomaticRow {
        source: "fn looped() { loop { break; } }\n",
        cases: &[("looped", Some(1))],
    },
    CyclomaticRow {
        source: "fn loops(a: bool) { while a {} for _ in 0..3 {} while let Some(_) = None::<u8> {} }\n",
        cases: &[("loops", Some(4))],
    },
    CyclomaticRow {
        source: "fn tried() -> Result<(), ()> { g()?; Ok(()) }\nfn g() -> Result<(), ()> { Ok(()) }\n",
        cases: &[("tried", Some(1))],
    },
    // A closure and a nested function are boundaries; the nested function
    // has its own record and total.
    CyclomaticRow {
        source: "fn outer(a: bool) {\n    let _c = |k: bool| if k { 1 } else { 2 };\n    fn inner(a: bool) { if a {} if a {} }\n}\n",
        cases: &[("outer", Some(1)), ("inner", Some(3))],
    },
    CyclomaticRow {
        source: "#[cfg(any(unix, windows))]\nfn look_alikes(a: bool) {\n    m!(if a {} && a);\n    /* if a && b */\n    let _s = \"if a && b\";\n}\n",
        cases: &[("look_alikes", Some(1))],
    },
    CyclomaticRow {
        source: "fn return_type() -> [u8; { if true { 1 } else { 2 } }] { [0; 1] }\n",
        cases: &[("return_type", Some(1))],
    },
    // Items declared in a body are not part of the call.
    CyclomaticRow {
        source: "fn items() {\n    struct S([u8; if true { 1 } else { 2 }]);\n    const C: u8 = if true { 1 } else { 2 };\n    static T: bool = true && false;\n}\n",
        cases: &[("items", Some(1))],
    },
    // `async`, `unsafe` and labeled blocks run as part of the function.
    CyclomaticRow {
        source: "fn blocks(a: bool) {\n    let _f = async { if a {} };\n    unsafe { if a {} }\n    'l: { if a { break 'l; } }\n}\n",
        cases: &[("blocks", Some(4))],
    },
    // Type syntax in the body adds nothing, wherever the type is written;
    // the value beside it (a cast operand, a call's arguments, an array
    // repeat length, an inline `const` block) still counts.
    CyclomaticRow {
        source: "fn array_annotation() { let _x: [u8; if true { 1 } else { 2 }]; }\n\
                 fn reference_annotation() { let _x: &[u8; if true { 1 } else { 2 }]; }\n\
                 fn tuple_annotation() { let _x: ([u8; if true { 1 } else { 2 }],); }\n\
                 fn pointer_annotation() { let _x: *const [u8; if true { 1 } else { 2 }]; }\n\
                 fn fn_pointer_annotation() { let _x: fn([u8; if true { 1 } else { 2 }]); }\n",
        cases: &[
            ("array_annotation", Some(1)),
            ("reference_annotation", Some(1)),
            ("tuple_annotation", Some(1)),
            ("pointer_annotation", Some(1)),
            ("fn_pointer_annotation", Some(1)),
        ],
    },
    CyclomaticRow {
        source: "struct Buf<const N: usize>;\n\
                 fn make<const N: usize>() {}\n\
                 fn generic_argument() { let _x: Buf<{ if true { 1 } else { 2 } }> = Buf; }\n\
                 fn call_turbofish() { make::<{ if true { 1 } else { 2 } }>(); }\n\
                 fn value_turbofish() { let _x = Buf::<{ if true { 1 } else { 2 } }>; }\n\
                 fn turbofish_arguments(a: bool) { let _x = Some::<u8>(if a { 1 } else { 2 }); }\n",
        cases: &[
            ("make", Some(1)),
            ("generic_argument", Some(1)),
            ("call_turbofish", Some(1)),
            ("value_turbofish", Some(1)),
            ("turbofish_arguments", Some(2)),
        ],
    },
    CyclomaticRow {
        source: "fn cast_target() { let _x = std::ptr::null::<u8>() as *const [u8; if true { 1 } else { 2 }]; }\n\
                 fn cast_operand(a: bool) { let _x = (if a { 1 } else { 2 }) as u8; }\n\
                 fn qualified_path() { let _x = <[u8; if true { 1 } else { 2 }] as Default>::default(); }\n",
        cases: &[("cast_target", Some(1)), ("cast_operand", Some(2)), ("qualified_path", Some(1))],
    },
    CyclomaticRow {
        source: "fn named_types() {\n    type Buffer = [u8; if true { 1 } else { 2 }];\n    struct S([u8; if true { 1 } else { 2 }]);\n    let _x: Buffer;\n}\n\
                 fn header(_p: [u8; if true { 1 } else { 2 }]) -> [u8; if true { 1 } else { 2 }] { [0; 1] }\n\
                 fn nested_const() { const X: usize = if true { 1 } else { 2 }; }\n",
        cases: &[("named_types", Some(1)), ("header", Some(1)), ("nested_const", Some(1))],
    },
    CyclomaticRow {
        source: "fn inline_const() { let _x = const { if true { 1 } else { 2 } }; }\n\
                 fn repeat_length() { let _x = [0; if true { 1 } else { 2 }]; }\n\
                 fn initializer() { let _x = if true { 1 } else { 2 }; }\n",
        cases: &[("inline_const", Some(2)), ("repeat_length", Some(2)), ("initializer", Some(2))],
    },
    // Methods and constructors measure; bodyless declarations do not.
    CyclomaticRow {
        source: "struct W;\nimpl W {\n    fn new(a: bool) -> Self { if a { W } else { W } }\n    fn get(&self, a: bool) -> bool { a || false }\n}\ntrait T {\n    fn required(&self);\n    fn provided(&self, a: bool) { while a {} }\n}\n",
        cases: &[("new", Some(2)), ("get", Some(2)), ("required", None), ("provided", Some(2))],
    },
];

/// Every JavaScript counting rule with its exact total, plus the forms that
/// contribute 0: plain `else`, `default`, `finally`, `for (;;)`, `?.`,
/// non-logical compound assignments and bitwise operators, comments,
/// strings, parameter defaults, and nested callables and classes.
const JAVASCRIPT_CYCLOMATIC: &[CyclomaticRow] = &[
    CyclomaticRow { source: "function empty() {}\n", cases: &[("empty", Some(1))] },
    CyclomaticRow {
        source: "function elseIf(a, b) { if (a) {} else if (b) {} else {} }\n",
        cases: &[("elseIf", Some(3))],
    },
    // `??` mixed with `||` must be parenthesized in JavaScript.
    CyclomaticRow {
        source: "function shortCircuit(a, b, c, d) { return (a && b || c) ?? d; }\n",
        cases: &[("shortCircuit", Some(4))],
    },
    CyclomaticRow {
        source: "function assigned(a, b) { a &&= b; a ||= b; a ??= b; a += 1; a |= b; a &= b; return a & b | b; }\n",
        cases: &[("assigned", Some(4))],
    },
    CyclomaticRow { source: "function chained(o) { return o?.x?.y; }\n", cases: &[("chained", Some(1))] },
    // `case 1: case 2:` share one body and are one arm; `default` is 0.
    CyclomaticRow {
        source: "function cases(a) { switch (a) { case 1: case 2: break; case 3: return; default: break; } }\n",
        cases: &[("cases", Some(3))],
    },
    // A label grouped with `default` adds nothing; a bodyless last case is
    // an arm of its own; a comment between grouped labels changes nothing.
    CyclomaticRow {
        source: "function withDefault(b) { switch (b) { case 1: default: break; } }\n\
                 function lastEmpty(b) { switch (b) { case 1: break; case 2: } }\n\
                 function commented(b) {\n    switch (b) {\n        case 1: // one\n        case 2:\n            break;\n    }\n}\n",
        cases: &[("withDefault", Some(1)), ("lastEmpty", Some(3)), ("commented", Some(2))],
    },
    CyclomaticRow {
        source: "function tried() { try { g(); } catch (e) {} finally {} try { g(); } catch {} }\nfunction g() {}\n",
        cases: &[("tried", Some(3))],
    },
    CyclomaticRow {
        source: "function forever() { for (;;) { break; } for (let i = 0;; i++) { break; } }\n\
                 function counted() { for (let i = 0; i < 3; i++) {} }\n",
        cases: &[("forever", Some(1)), ("counted", Some(2))],
    },
    CyclomaticRow {
        source: "function loops(o) { for (const k in o) {} for (const v of o) {} while (o) {} do {} while (o); }\n",
        cases: &[("loops", Some(5))],
    },
    CyclomaticRow {
        source: "function labeled(a) { outer: for (const x of a) { if (x) continue outer; } }\n",
        cases: &[("labeled", Some(3))],
    },
    // Expression and block arrow bodies; an arrow returning an arrow is the
    // outer one's boundary.
    CyclomaticRow {
        source: "const ternary = (k) => k ? 1 : 2;\nconst constant = () => 1;\nconst block = async (a) => { if (a) {} };\nconst curried = (a) => (b) => a && b;\n",
        cases: &[("ternary", Some(2)), ("constant", Some(1)), ("block", Some(2)), ("curried", Some(1))],
    },
    CyclomaticRow {
        source: "function lookAlikes(a, b = a ? 1 : 2) {\n    /* if (a) && b */\n    const s = \"a ? b : c\";\n    return `${s} || if`;\n}\n\
                 function templated(a) { return `${a ? 1 : 2}`; }\n",
        cases: &[("lookAlikes", Some(1)), ("templated", Some(2))],
    },
    // Nested functions, arrows, classes (field initializers included) are
    // boundaries; each nested callable has its own total.
    CyclomaticRow {
        source: "function outer(a) {\n    function inner() { if (a) {} }\n    const arrow = () => a && a;\n    class C {\n        x = a ? 1 : 2;\n        m() { if (a) {} }\n    }\n    return [inner, arrow, C];\n}\n",
        cases: &[("outer", Some(1)), ("inner", Some(2)), ("arrow", Some(2)), ("m", Some(2))],
    },
    // Object literal values run inline; its methods are boundaries.
    CyclomaticRow {
        source: "function literal(a) { return { p: a ? 1 : 2, q() { if (a) {} } }; }\n",
        cases: &[("literal", Some(2))],
    },
    CyclomaticRow {
        source: "function anonymousClass(a) { return class { x = a ? 1 : 2; }; }\n",
        cases: &[("anonymousClass", Some(1))],
    },
    CyclomaticRow {
        source: "class K {\n    constructor(a) { if (a) {} }\n    get g() { return this.a || 0; }\n    static s(a) { while (a) {} }\n}\n",
        cases: &[("constructor", Some(2)), ("g", Some(2)), ("s", Some(2))],
    },
    CyclomaticRow {
        source: "function view(a) { return <div>{a && <b />}</div>; }\n",
        cases: &[("view", Some(2))],
    },
];

/// The TypeScript rules, for `.ts` and `.tsx`: JavaScript's decisions, plus
/// forms that contribute 0 (type syntax wherever it is written, `as`,
/// `satisfies`, `!`), type declarations and namespaces as boundaries, and
/// bodyless declarations as `None`.
const TYPESCRIPT_CYCLOMATIC: &[CyclomaticRow] = &[
    CyclomaticRow {
        source: "function typed<T>(a: boolean): T extends string ? 1 : 2 { let x: T extends string ? 1 : 2; return x as any; }\n",
        cases: &[("typed", Some(1))],
    },
    CyclomaticRow {
        source: "function asserted(a: any, b: number) { return (a as number) ?? (b! satisfies number); }\n",
        cases: &[("asserted", Some(2))],
    },
    CyclomaticRow {
        source: "function elseIf(a: boolean, b: boolean) { if (a) {} else if (b && a) {} else {} }\n",
        cases: &[("elseIf", Some(4))],
    },
    CyclomaticRow {
        source: "const asyncArrow = async (k: number): Promise<number> => k > 1 ? 1 : 2;\n",
        cases: &[("asyncArrow", Some(2))],
    },
    CyclomaticRow {
        source: "function cases(b: number) { switch (b) { case 1: default: break; } switch (b) { case 1: case 2: return; case 3: } }\n",
        cases: &[("cases", Some(3))],
    },
    CyclomaticRow {
        source: "function loops(o: number[]) { for (;;) { break; } for (const v of o) { try { v?.toString(); } catch { o ||= []; } } }\n",
        cases: &[("loops", Some(4))],
    },
    // Interfaces, aliases, enums, classes and namespaces in a body are
    // declarations of their own.
    CyclomaticRow {
        source: "function declares(a: boolean) {\n    interface I { m(): void }\n    type U = typeof a extends true ? 1 : 2;\n    enum E { A = a ? 1 : 2 }\n    abstract class K { n() { if (a) {} } }\n    namespace N { export const x = a ? 1 : 2; }\n}\n",
        cases: &[("declares", Some(1)), ("n", Some(2))],
    },
    CyclomaticRow {
        source: "function outer(a: boolean) {\n    function inner(): void { if (a) {} }\n    const arrow = (): boolean => a || !a;\n    class C { x = a ? 1 : 2; private m(): void { while (a) {} } }\n}\n",
        cases: &[("outer", Some(1)), ("inner", Some(2)), ("arrow", Some(2)), ("m", Some(2))],
    },
    CyclomaticRow {
        source: "class C {\n    constructor(private a: boolean) { if (a) {} }\n}\n",
        cases: &[("constructor", Some(2))],
    },
    // Type syntax in the body adds nothing, wherever the type is written;
    // the value beside it (an assertion operand, a call's arguments, an
    // object literal) still counts.
    CyclomaticRow {
        source: "function objectAnnotation(a: boolean) { let x: { [a ? 1 : 2]: string }; }\n\
                 function arrayAnnotation(a: boolean) { let x: { [a ? 1 : 2]: string }[]; let y: [{ [a ? 1 : 2]: string }]; }\n\
                 function typeQuery(a: boolean) { let x: typeof import(a ? 'a' : 'b'); }\n\
                 function conditionalType<T>() { let x: T extends string ? number : boolean; }\n",
        cases: &[
            ("objectAnnotation", Some(1)),
            ("arrayAnnotation", Some(1)),
            ("typeQuery", Some(1)),
            ("conditionalType", Some(1)),
        ],
    },
    CyclomaticRow {
        source: "function asTarget(a: boolean) { return {} as { [a ? 1 : 2]: string }; }\n\
                 function importTarget(a: boolean) { return {} as import(a ? 'a' : 'b'); }\n\
                 function satisfiesTarget(a: boolean) { return {} satisfies { [a ? 1 : 2]: string }; }\n\
                 function asOperand(a: boolean) { return (a ? 1 : 2) as number; }\n",
        cases: &[("asTarget", Some(1)), ("importTarget", Some(1)), ("satisfiesTarget", Some(1)), ("asOperand", Some(2))],
    },
    CyclomaticRow {
        source: "function f<T>(x?: number) {}\n\
                 function typeArgument(a: boolean) { f<{ [a ? 1 : 2]: string }>(); }\n\
                 function callArguments(a: boolean) { f<number>(a ? 1 : 2); }\n",
        cases: &[("f", Some(1)), ("typeArgument", Some(1)), ("callArguments", Some(2))],
    },
    CyclomaticRow {
        source: "function header(p: { [a ? 1 : 2]: string }): { [a ? 1 : 2]: string } { return p; }\n\
                 function namedTypes(a: boolean) { type O = { [a ? 1 : 2]: string }; interface I { [a ? 1 : 2]: string } }\n\
                 function nestedClass(a: boolean) { class C { x = a ? 1 : 2; } }\n\
                 function objectValue(a: boolean) { return { p: a ? 1 : 2 }; }\n",
        cases: &[("header", Some(1)), ("namedTypes", Some(1)), ("nestedClass", Some(1)), ("objectValue", Some(2))],
    },
    // A declaration without a body has no value.
    CyclomaticRow {
        source: "abstract class A {\n    abstract m(): void;\n    n(a: boolean): number { return a ? 1 : 0; }\n}\n",
        cases: &[("m", None), ("n", Some(2))],
    },
];

/// Switch label groups, run as JavaScript, TypeScript and TSX: labels
/// sharing one body are one arm, decided by the whole group, so a `default`
/// anywhere in it makes the group add nothing whatever the label order; a
/// `default` with a body of its own, a bodyless final case and comments
/// between labels keep their meaning.
const SWITCH_GROUP_CYCLOMATIC: &[CyclomaticRow] = &[
    CyclomaticRow {
        source: "function defaultFirst(x) { switch (x) { default: case 1: break; } }\n\
                 function defaultMiddle(x) { switch (x) { case 1: default: case 2: break; } }\n\
                 function defaultLast(x) { switch (x) { case 1: default: break; } }\n",
        cases: &[("defaultFirst", Some(1)), ("defaultMiddle", Some(1)), ("defaultLast", Some(1))],
    },
    CyclomaticRow {
        source: "function commentedDefault(x) { switch (x) { default: /* c */ case 1: /* d */ case 2: break; } }\n\
                 function lineCommentedDefault(x) {\n    switch (x) {\n        default: // c\n        case 1:\n            break;\n    }\n}\n",
        cases: &[("commentedDefault", Some(1)), ("lineCommentedDefault", Some(1))],
    },
    CyclomaticRow {
        source: "function emptyDefaultFirst(x) { switch (x) { default: case 1: } }\n\
                 function emptyDefaultLast(x) { switch (x) { case 1: default: } }\n\
                 function defaultOnly(x) { switch (x) { default: break; } }\n",
        cases: &[("emptyDefaultFirst", Some(1)), ("emptyDefaultLast", Some(1)), ("defaultOnly", Some(1))],
    },
    CyclomaticRow {
        source: "function defaultBody(x) { switch (x) { default: x++; case 1: break; } }\n\
                 function ordinaryGroup(x) { switch (x) { case 1: case 2: break; } }\n\
                 function finalEmptyCase(x) { switch (x) { case 1: break; case 2: } }\n\
                 function separateBodies(x) { switch (x) { case 1: x++; case 2: x--; break; default: break; } }\n",
        cases: &[
            ("defaultBody", Some(2)),
            ("ordinaryGroup", Some(2)),
            ("finalEmptyCase", Some(3)),
            ("separateBodies", Some(3)),
        ],
    },
    // Each group is decided on its own: the `default` of the second group
    // does not reach back across the first group's body. `groups` is the
    // example in `docs/cyclomatic-complexity.md`.
    CyclomaticRow {
        source: "function twoGroups(x) { switch (x) { case 1: case 2: break; case 3: default: case 4: break; case 5: } }\n\
                 function groups(x) {\n  switch (x) {\n    default:\n    case 1: break;\n    case 2:\n    case 3: break;\n    case 4:\n  }\n}\n",
        cases: &[("twoGroups", Some(3)), ("groups", Some(3))],
    },
];

/// TypeScript rows that only `.ts` parses: the angle-bracket assertion is
/// JSX in `.tsx`.
const TS_ONLY_CYCLOMATIC: &[CyclomaticRow] = &[CyclomaticRow {
    source: "function angleTarget(a: boolean) { return <{ [a ? 1 : 2]: string }>{}; }\n\
             function angleOperand(a: boolean) { return <number>(a ? 1 : 2); }\n",
    cases: &[("angleTarget", Some(1)), ("angleOperand", Some(2))],
}];

fn write_snippet(dir: &TempDir, file: &str, source: &str) -> std::path::PathBuf {
    let path = dir.path().join(file);
    std::fs::write(&path, source).expect("write snippet");
    path
}

fn complete_records(path: &Path) -> Vec<SymbolRecord> {
    SourceFile::read(path)
        .expect("read source")
        .query(&SymbolQuery::complete_index())
        .expect("query")
        .index
        .symbols
}

fn function_named<'a>(records: &'a [SymbolRecord], name: &str, label: &str) -> &'a SymbolRecord {
    let matching: Vec<&SymbolRecord> = records
        .iter()
        .filter(|record| record.identity.name == name && is_function(record.kind))
        .collect();
    assert_eq!(matching.len(), 1, "{label}: one function `{name}`");
    matching[0]
}

/// Each row parses cleanly and every listed function has its exact total,
/// on a metrics-only query and on the complete index, with LF and CRLF line
/// endings; every non-callable record in the row reports `None`.
fn assert_cyclomatic_rows(rows: &[CyclomaticRow], extension: &str) {
    let dir = TempDir::new().expect("temp dir");
    for (index, row) in rows.iter().enumerate() {
        for (ending, source) in [("LF", row.source.to_string()), ("CRLF", row.source.replace('\n', "\r\n"))] {
            let path = write_snippet(&dir, &format!("cyclomatic_{index}_{ending}.{extension}"), &source);
            let errors = TreeFile::new(&path).expect("parse").syntax_diagnostics();
            assert!(errors.is_empty(), "{}: the snippet parses cleanly: {errors:?}", row.source);
            for records in [metrics_records(&path), complete_records(&path)] {
                for (name, expected) in row.cases {
                    let label = format!("{ending} {}", row.source);
                    let metrics = function_named(&records, name, &label).metrics.expect("metrics facet");
                    assert_eq!(metrics.cyclomatic, *expected, "{label}: `{name}`");
                }
                for record in records.iter().filter(|record| !is_function(record.kind)) {
                    let metrics = record.metrics.expect("metrics facet");
                    assert_eq!(metrics.cyclomatic, None, "{}: `{}` ({:?})", row.source, record.identity.name, record.kind);
                }
            }
        }
    }
}

#[test]
fn rust_cyclomatic_follows_the_counting_rules() {
    assert_cyclomatic_rows(RUST_CYCLOMATIC, "rs");
}

#[test]
fn javascript_cyclomatic_follows_the_counting_rules() {
    assert_cyclomatic_rows(JAVASCRIPT_CYCLOMATIC, "js");
}

#[test]
fn switch_groups_are_decided_by_every_label_in_the_group() {
    for extension in ["js", "ts", "tsx"] {
        assert_cyclomatic_rows(SWITCH_GROUP_CYCLOMATIC, extension);
    }
}

#[test]
fn typescript_cyclomatic_follows_the_counting_rules() {
    assert_cyclomatic_rows(TYPESCRIPT_CYCLOMATIC, "ts");
    assert_cyclomatic_rows(TYPESCRIPT_CYCLOMATIC, "tsx");
    assert_cyclomatic_rows(TS_ONLY_CYCLOMATIC, "ts");
}

/// `(extension, source)`: functions `broken` (an error in its own body),
/// `outer` (one `if`, around a nested `innerBroken` whose body has an error)
/// and `sibling` (one `&&`), each with one parameter and a one-line body.
const MALFORMED_BODIES: &[(&str, &str)] = &[
    (
        "rs",
        "fn broken(a: bool) { if a { let x = ; } }\n\
         fn outer(a: bool) { if a {} fn innerBroken() { let y = ; } }\n\
         fn sibling(a: bool) -> bool { a && a }\n",
    ),
    (
        "js",
        "function broken(a) { if (a) { let x = ; } }\n\
         function outer(a) { if (a) {} function innerBroken() { let y = ; } }\n\
         function sibling(a) { return a && a; }\n",
    ),
    (
        "ts",
        "function broken(a: boolean) { if (a) { let x = ; } }\n\
         function outer(a: boolean) { if (a) {} function innerBroken(): void { let y = ; } }\n\
         function sibling(a: boolean) { return a && a; }\n",
    ),
];

/// A syntax error inside the counted body makes that function's total
/// unavailable, never a partial count. An error inside a nested function is
/// that function's own: the outer total and a valid sibling keep their
/// values, and the other metrics are unaffected.
#[test]
fn cyclomatic_is_unavailable_only_for_a_malformed_counted_body() {
    let dir = TempDir::new().expect("temp dir");
    for (extension, source) in MALFORMED_BODIES {
        let path = write_snippet(&dir, &format!("malformed.{extension}"), source);
        assert!(!TreeFile::new(&path).expect("parse").syntax_diagnostics().is_empty(), "{source}: the snippet is malformed");
        for records in [metrics_records(&path), complete_records(&path)] {
            let metrics = |name: &str| function_named(&records, name, source).metrics.expect("metrics facet");
            assert_eq!(
                metrics("broken"),
                MetricsFacet { parameters: Some(1), has_docs: Some(false), lines: Some(1), cyclomatic: None },
                "{source}"
            );
            assert_eq!(metrics("innerBroken").cyclomatic, None, "{source}");
            assert_eq!(metrics("outer").cyclomatic, Some(2), "{source}");
            assert_eq!(metrics("sibling").cyclomatic, Some(2), "{source}");
        }
    }
}

/// A query that leaves a nested callable out (scope `Item` only) reports
/// the same pruned outer total as one that includes it.
#[test]
fn an_outer_total_does_not_depend_on_selecting_its_nested_callables() {
    let dir = TempDir::new().expect("temp dir");
    let sources = [
        ("rs", "fn outer(a: bool) { if a {} fn inner(a: bool) { if a {} } }\n"),
        ("js", "function outer(a) { if (a) {} const inner = () => a && a; function nested(b) { while (b) {} } }\n"),
        ("ts", "function outer(a: boolean) { if (a) {} const inner = (): boolean => a && a; }\n"),
    ];
    for (extension, source) in sources {
        let path = write_snippet(&dir, &format!("nested.{extension}"), source);
        let outer_only = SourceFile::read(&path)
            .expect("read source")
            .query(&SymbolQuery::new().scopes([SymbolScope::Item]).capabilities([Capability::Metrics]))
            .expect("query")
            .index
            .symbols;
        assert_eq!(outer_only.len(), 1, "{source}: only `outer` is selected: {outer_only:?}");
        let selected = function_named(&outer_only, "outer", source).metrics.expect("metrics facet");
        let complete = complete_records(&path);
        let all = function_named(&complete, "outer", source).metrics.expect("metrics facet");
        assert_eq!(selected.cyclomatic, Some(2), "{source}");
        assert_eq!(selected, all, "{source}");
    }
}

/// Inside a present `metrics` facet `cyclomatic` is always a key: a number
/// for a measured Rust callable, `null` for a struct or a bodyless method;
/// without `Capability::Metrics` the whole facet is absent.
#[test]
fn cyclomatic_key_is_present_null_or_absent_with_its_facet() {
    let dir = TempDir::new().expect("temp dir");
    let path = write_snippet(&dir, "shape.rs", "struct S;\ntrait T {\n    fn area(&self);\n}\nfn sides(a: bool) -> u8 {\n    if a { 4 } else { 3 }\n}\n");
    let json = |query: &SymbolQuery, name: &str| -> serde_json::Value {
        let records = SourceFile::read(&path).expect("read source").query(query).expect("query").index.symbols;
        let record = records.iter().find(|record| record.identity.name == name).expect("record");
        serde_json::to_value(record).expect("serialize")
    };
    let metrics = SymbolQuery::new().capabilities([Capability::Metrics]);
    assert_eq!(json(&metrics, "sides")["metrics"]["cyclomatic"], serde_json::json!(2));
    for name in ["S", "area"] {
        let facet = &json(&metrics, name)["metrics"];
        assert_eq!(facet.get("cyclomatic"), Some(&serde_json::Value::Null), "`{name}`: {facet}");
    }
    for name in ["S", "area", "sides"] {
        let record = json(&SymbolQuery::new(), name);
        assert!(record.get("metrics").is_none(), "`{name}`: no facet without Metrics: {record}");
    }
}
