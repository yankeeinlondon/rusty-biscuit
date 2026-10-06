//! The metrics matrix: `parameters`, `has_docs` and `lines` for each
//! fixture function, `None` where a metric is unsupported, and the
//! language-wide rules behind them; `BODY_LINES_MATRIX` holds the body forms
//! `lines` is counted from.

use std::path::Path;

use tempfile::TempDir;
use tree_hugger::{
    Capability, MetricsFacet, ProgrammingLanguage, SourceFile, SymbolKindV2, SymbolQuery,
    SymbolRecord, TextSpan, TreeFile,
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

/// `(qualified name of a function, parameters, has_docs, lines)`.
pub type MetricsCase = (&'static str, Option<u32>, Option<bool>, Option<u32>);

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
            ("documented_pair", Some(2), Some(true), Some(1)),
            ("undocumented_none", Some(0), Some(false), Some(1)),
            ("deprecated_api", Some(0), Some(false), Some(1)),
            // The `&self` receiver is not counted.
            ("Receiver::with_receiver", Some(1), Some(false), Some(1)),
            ("cfg_test_module::inside_cfg_test", Some(1), Some(false), Some(1)),
            ("function_with_nested_items::nested_local_module::nested_module_function", Some(0), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::TypeScript,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true), Some(1)),
            ("undocumentedNone", Some(0), Some(false), Some(1)),
            ("variadic", Some(2), Some(false), Some(1)),
            ("Receiver.withReceiver", Some(1), Some(true), Some(1)),
            ("boundaryArrow", Some(1), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::JavaScript,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true), Some(1)),
            ("undocumentedNone", Some(0), Some(false), Some(1)),
            ("variadic", Some(2), Some(false), Some(1)),
            ("documentedArrow", Some(1), Some(true), Some(1)),
            ("lookAlikeTag", Some(0), Some(true), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Go,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("symbols.DocumentedPair", Some(2), Some(true), Some(1)),
            ("symbols.UndocumentedNone", Some(0), Some(false), Some(3)),
            ("symbols.Variadic", Some(2), Some(false), Some(1)),
            // The receiver has its own list and is not counted.
            ("symbols.ExportedStruct.WithReceiver", Some(1), Some(false), Some(1)),
            ("symbols.DeprecatedLater", Some(0), Some(true), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Python,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true), Some(1)),
            ("undocumented_none", Some(0), Some(false), Some(1)),
            // `*rest` and `**options` count once each.
            ("variadic", Some(3), Some(false), Some(1)),
            // `self` is an ordinary parameter in Python's syntax.
            ("Receiver.with_receiver", Some(2), Some(false), Some(1)),
            ("outer_function.nested_function", Some(0), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Java,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("com.example.symbols.FlagsAndMetrics.documentedPair", Some(2), Some(true), Some(1)),
            ("com.example.symbols.FlagsAndMetrics.undocumentedNone", Some(0), Some(false), Some(1)),
            ("com.example.symbols.FlagsAndMetrics.deprecatedByTag", Some(0), Some(true), Some(1)),
            ("com.example.symbols.FlagsAndMetrics.variadic", Some(2), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::CSharp,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("Example.Symbols.FlagsAndMetrics.DocumentedPair", Some(2), Some(true), Some(1)),
            ("Example.Symbols.FlagsAndMetrics.UndocumentedNone", Some(0), Some(false), Some(1)),
            ("Example.Symbols.FlagsAndMetrics.Variadic", Some(2), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::C,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true), Some(1)),
            // `(void)` is a known empty list.
            ("undocumented_none", Some(0), Some(false), Some(1)),
            ("variadic", Some(2), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Cpp,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documented_pair", Some(2), Some(true), Some(1)),
            ("undocumented_none", Some(0), Some(false), Some(1)),
            ("variadic", Some(2), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Swift,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("documentedPair", Some(2), Some(true), Some(1)),
            ("undocumentedNone", Some(0), Some(false), Some(1)),
            ("variadic", Some(2), Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Scala,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("example.symbols.documentedPair", Some(2), Some(true), Some(1)),
            ("example.symbols.undocumentedNone", Some(0), Some(false), Some(1)),
            ("example.symbols.variadic", Some(2), Some(false), Some(1)),
            // `def area: Int` has no parameter list at all.
            ("example.symbols.FirstShape.area", None, Some(false), Some(1)),
        ],
    },
    MetricsRow {
        language: L::Php,
        parameters: Support::Measured,
        docs: Support::Measured,
        cases: &[
            ("Example\\Symbols\\documented_pair", Some(2), Some(true), Some(2)),
            ("Example\\Symbols\\undocumented_none", Some(0), Some(false), Some(2)),
            ("Example\\Symbols\\variadic", Some(2), Some(false), Some(2)),
        ],
    },
    MetricsRow {
        language: L::Perl,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cases: &[("Fixture::Other::test_helper", None, None, Some(1))],
    },
    MetricsRow {
        language: L::Lua,
        parameters: Support::Measured,
        docs: NO_DOCS,
        cases: &[
            ("documented_pair", Some(2), None, Some(0)),
            // `...` counts once.
            ("variadic", Some(2), None, Some(0)),
            ("test_helper", Some(0), None, Some(0)),
        ],
    },
    MetricsRow {
        language: L::Bash,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cases: &[("test_helper", None, None, Some(3))],
    },
    MetricsRow {
        language: L::Zsh,
        parameters: NO_PARAMETERS,
        docs: NO_DOCS,
        cases: &[("test_helper", None, None, Some(3))],
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
        for (name, parameters, has_docs, lines) in row.cases {
            let matching: Vec<&SymbolRecord> = records
                .iter()
                .filter(|record| in_file_name(record) == *name && is_function(record.kind))
                .collect();
            assert_eq!(matching.len(), 1, "{label}: one function `{name}`");
            assert_eq!(
                matching[0].metrics,
                Some(MetricsFacet { parameters: *parameters, has_docs: *has_docs, lines: *lines }),
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

/// Each case's `source.body_span` and `metrics.lines` agree, on a
/// metrics-only query and on the complete index, with LF and with CRLF
/// line endings.
#[test]
fn body_lines_are_counted_from_the_callables_own_body() {
    let dir = TempDir::new().expect("temp dir");
    for row in BODY_LINES_MATRIX {
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
        serde_json::json!({ "parameters": 0, "has_docs": false, "lines": null })
    );
    assert_eq!(
        json(&metrics, "sides")["metrics"],
        serde_json::json!({ "parameters": 0, "has_docs": false, "lines": 3 })
    );
    assert_eq!(
        json(&metrics, "Shape")["metrics"],
        serde_json::json!({ "parameters": null, "has_docs": false, "lines": null })
    );
    for name in ["area", "sides", "Shape"] {
        let record = json(&SymbolQuery::new(), name);
        assert!(record.get("metrics").is_none(), "`{name}`: no facet without Metrics: {record}");
    }
}
