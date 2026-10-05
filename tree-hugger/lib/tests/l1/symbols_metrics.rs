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
        assert_eq!(
            record.metrics,
            Some(MetricsFacet { parameters: *parameters, has_docs: *has_docs }),
            "{source}: `{name}`"
        );
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
