//! The modifiers and attributes matrix: `modifiers` holds exactly the
//! modifier tokens a declaration writes itself (`Const`, `Unsafe`,
//! `Readonly`, `Mut` and the lowercase `Custom` keywords, beside the
//! existing `Static`/`StaticStorage`/`Async`), in their one serialized order,
//! and `attributes` its own attributes, annotations and decorators in source
//! order, with repeats and empty argument lists kept. A keyword in a body, a
//! string, a type or a nested parameter never marks the enclosing
//! declaration, and a wrapper's or file's attributes never reach every
//! declaration. Each language has one row, checked on the complete index and
//! on a declarations-only query.

use std::io::Write;

use tree_hugger::{
    FileSymbolIndex, Modifier, ProgrammingLanguage, SymbolKindV2, SymbolQuery, SymbolRecord, TreeFile,
};

use ProgrammingLanguage as L;
use SymbolKindV2 as K;

/// One declaration of a row's source and what it must report. Modifiers are
/// spelled as their variant name (`Static`, `Mut`) or, for `Custom`, the
/// keyword itself (`abstract`); attributes as `(name, value)`.
pub struct Declared {
    pub name: &'static str,
    pub kind: SymbolKindV2,
    pub modifiers: &'static [&'static str],
    pub attributes: &'static [(&'static str, Option<&'static str>)],
}

/// A form the pinned grammar rejects.
pub struct Limit {
    pub grammar: &'static str,
    pub extension: &'static str,
    pub snippet: &'static str,
    pub observation: &'static str,
}

pub enum Support {
    Supported,
    /// The language writes none of the new modifiers and no attribute this
    /// reader reads.
    Absent(&'static str),
}

pub struct ModifiersRow {
    pub language: ProgrammingLanguage,
    pub support: Support,
    pub extension: &'static str,
    pub source: &'static str,
    pub declarations: &'static [Declared],
    pub limits: &'static [Limit],
}

const fn declared(
    name: &'static str,
    kind: SymbolKindV2,
    modifiers: &'static [&'static str],
    attributes: &'static [(&'static str, Option<&'static str>)],
) -> Declared {
    Declared {
        name,
        kind,
        modifiers,
        attributes,
    }
}

const fn bare(name: &'static str, kind: SymbolKindV2) -> Declared {
    declared(name, kind, &[], &[])
}

pub const MODIFIERS_MATRIX: &[ModifiersRow] = &[
    ModifiersRow {
        language: L::Rust,
        support: Support::Supported,
        extension: "rs",
        source: "#![allow(dead_code)]\n\
                 #[inline]\n#[cfg_attr(test, derive(Debug))]\n#[doc = \"a  b\"]\n#[allow()]\n#[allow()]\n\
                 pub const unsafe fn both() -> u8 { 0 }\n\
                 #[tokio::test]\npub async unsafe fn later() { let text = \"const unsafe mut\"; unsafe {} }\n\
                 pub fn takes(mut count: u32, fallback: u32) -> u32 { let mut total = count; let fixed = fallback; total + fixed }\n\
                 pub static mut COUNTER: u32 = 0;\n\
                 pub unsafe trait Marker {}\n\
                 pub struct Cell { value: u8 }\n\
                 impl Cell {\n    pub fn set(&mut self, value: u8) { self.value = value; }\n    pub fn take(mut self) -> u8 { self.value }\n    pub fn typed(self: &mut Self) {}\n    pub fn read(&self, other: &mut u8) -> u8 { *other }\n    pub const fn new() -> Self { Cell { value: 0 } }\n}\n",
        declarations: &[
            // The file's inner `#![allow]` is not the first item's.
            declared(
                "both",
                K::Function,
                &["Const", "Unsafe"],
                &[
                    ("inline", None),
                    ("cfg_attr", Some("test, derive(Debug)")),
                    ("doc", Some("\"a  b\"")),
                    ("allow", Some("")),
                    ("allow", Some("")),
                ],
            ),
            // A keyword in the body or a string marks nothing.
            declared("later", K::Function, &["Async", "Unsafe"], &[("tokio::test", None)]),
            // A `mut` parameter marks the parameter, never its function.
            bare("takes", K::Function),
            declared("count", K::Parameter, &["Mut"], &[]),
            bare("fallback", K::Parameter),
            declared("total", K::Variable, &["Mut"], &[]),
            bare("fixed", K::Variable),
            declared("COUNTER", K::Variable, &["StaticStorage", "Mut"], &[]),
            declared("Marker", K::Trait, &["Unsafe"], &[]),
            bare("Cell", K::Type),
            declared("set", K::Method, &["Mut"], &[]),
            declared("take", K::Method, &["Mut"], &[]),
            declared("typed", K::Method, &["Mut"], &[]),
            // A `&mut` in a later parameter's type is not the receiver's.
            bare("read", K::Method),
            declared("new", K::Method, &["Static", "Const"], &[]),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::TypeScript,
        support: Support::Supported,
        extension: "ts",
        source: "@sealed\nexport class Exported {\n  run(): void {}\n}\n\
                 export abstract class Shape {\n  abstract area(): number;\n  @log() @trace.calls(\"a  b\") override describe(): string { const text = \"abstract readonly\"; return text; }\n  readonly sides: number = 0;\n  @observable abstract label: string;\n  static async load(): Promise<void> {}\n  constructor(readonly id: number, @Inject() service: string) {}\n}\n\
                 interface Named {\n  readonly title: string;\n  size: readonly string[];\n}\n",
        declarations: &[
            // An exported class's decorator sits on the `export` statement;
            // it is the class's, not its members'.
            declared("Exported", K::Class, &[], &[("sealed", None)]),
            bare("run", K::Method),
            declared("Shape", K::Class, &["abstract"], &[]),
            declared("area", K::Method, &["abstract"], &[]),
            declared(
                "describe",
                K::Method,
                &["override"],
                &[("log", Some("")), ("trace.calls", Some("\"a  b\""))],
            ),
            declared("sides", K::Property, &["Readonly"], &[]),
            declared("label", K::Property, &["abstract"], &[("observable", None)]),
            declared("load", K::Method, &["Static", "Async"], &[]),
            // A parameter's decorator is the parameter's.
            bare("constructor", K::Constructor),
            declared("id", K::Parameter, &["Readonly"], &[]),
            declared("service", K::Parameter, &[], &[("Inject", Some(""))]),
            declared("title", K::Property, &["Readonly"], &[]),
            // `readonly` in a type is not the declaration's.
            bare("size", K::Property),
        ],
        limits: &[Limit {
            grammar: "tree-sitter-typescript 0.23.2",
            extension: "ts",
            snippet: "class A {\n  @(flag ? one : two) m() {}\n}\n",
            observation: "a decorator is only a path, a call of a path or a parenthesized one of those; any other expression is a syntax error",
        }],
    },
    ModifiersRow {
        language: L::JavaScript,
        support: Support::Supported,
        extension: "js",
        source: "@register\nclass Widget {\n  @bound() @a.b render() { return \"abstract override readonly\"; }\n  @state count = 0;\n  static async load() {}\n}\n\
                 export @tagged class Tagged {}\n",
        declarations: &[
            declared("Widget", K::Class, &[], &[("register", None)]),
            declared("render", K::Method, &[], &[("bound", Some("")), ("a.b", None)]),
            declared("count", K::Property, &[], &[("state", None)]),
            declared("load", K::Method, &["Static", "Async"], &[]),
            declared("Tagged", K::Class, &[], &[("tagged", None)]),
        ],
        limits: &[Limit {
            grammar: "tree-sitter-javascript 0.25.0",
            extension: "js",
            snippet: "class A {\n  @(flag ? one : two) m() {}\n}\n",
            observation: "a decorator is only a path, a call of a path or a parenthesized one of those; any other expression is a syntax error",
        }],
    },
    ModifiersRow {
        language: L::Go,
        support: Support::Absent("Go has no declaration modifiers or attributes"),
        extension: "go",
        source: "package shapes\n\n// Final is documented.\nfunc Final() string {\n\treturn \"final abstract override\"\n}\n\ntype Shape struct {\n\tSides int `json:\"sides\"`\n}\n",
        declarations: &[bare("Final", K::Function), bare("Shape", K::Type)],
        limits: &[],
    },
    ModifiersRow {
        language: L::Python,
        support: Support::Supported,
        extension: "py",
        source: "@app.route( \"/x\" ,  methods=[\"GET\"] )\n@app.route(\"/y\")\n@staticmethod\ndef handler():\n    pass\n\n\
                 @dataclass\nclass Shape:\n    @abstractmethod\n    def area(self):\n        pass\n\n    @functools.cache\n    def name(self):\n        return \"final override abstract\"\n\n    @(lambda function: function)\n    def odd(self):\n        pass\n\n    @registry[\"  k \"]\n    def keyed(self):\n        pass\n\n    @staticmethod\n    def make():\n        pass\n\n    async def fetch(self):\n        pass\n",
        declarations: &[
            // A module-level function is no member, so `@staticmethod` makes
            // it no `Static`; no decorator implies a modifier.
            declared(
                "handler",
                K::Function,
                &[],
                &[
                    ("app.route", Some("\"/x\" ,  methods=[\"GET\"]")),
                    ("app.route", Some("\"/y\"")),
                    ("staticmethod", None),
                ],
            ),
            declared("Shape", K::Class, &[], &[("dataclass", None)]),
            declared("area", K::Method, &[], &[("abstractmethod", None)]),
            declared("name", K::Method, &[], &[("functools.cache", None)]),
            // Neither a path nor a call of one: the expression is the name,
            // as written, literal whitespace kept.
            declared("odd", K::Method, &[], &[("lambda function: function", None)]),
            declared("keyed", K::Method, &[], &[("registry[\"  k \"]", None)]),
            declared("make", K::Method, &["Static"], &[("staticmethod", None)]),
            declared("fetch", K::Method, &["Async"], &[]),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::Java,
        support: Support::Supported,
        extension: "java",
        source: "@Deprecated(since = \"9\")\n@SuppressWarnings({\"a\", \"b\"})\npublic abstract class Shape {\n    @Override @Override public final String toString() { String text = \"abstract final\"; return text; }\n    public abstract double area();\n    private final int sides = 0, corners = 4;\n    public static final int MAX = 1;\n    @Test void check(final int input) { final int local = input; }\n}\n",
        declarations: &[
            declared(
                "Shape",
                K::Class,
                &["abstract"],
                &[("Deprecated", Some("since = \"9\"")), ("SuppressWarnings", Some("{\"a\", \"b\"}"))],
            ),
            declared("toString", K::Method, &["final"], &[("Override", None), ("Override", None)]),
            declared("area", K::Method, &["abstract"], &[]),
            // One modifier list, every declarator of it.
            declared("sides", K::Field, &["final"], &[]),
            declared("corners", K::Field, &["final"], &[]),
            declared("MAX", K::Field, &["Static", "final"], &[]),
            declared("check", K::Method, &[], &[("Test", None)]),
            declared("input", K::Parameter, &["final"], &[]),
            declared("local", K::Variable, &["final"], &[]),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::CSharp,
        support: Support::Supported,
        extension: "cs",
        source: "[assembly: System.Reflection.AssemblyVersion(\"1.0\")]\n\
                 [Serializable]\npublic abstract class Shape {\n    [Obsolete( \"a  b\" )] [Fact] [return: NotNull] public abstract string Name();\n    public virtual void Draw() { var text = \"virtual sealed readonly\"; }\n    public sealed override string ToString() => \"\";\n    public readonly int Sides, Corners;\n    public static async Task LoadAsync() {}\n}\n\
                 public readonly struct Point {}\npublic sealed class Leaf {}\n",
        declarations: &[
            // The `[assembly: …]` attribute is the file's.
            declared("Shape", K::Class, &["abstract"], &[("Serializable", None)]),
            declared(
                "Name",
                K::Method,
                &["abstract"],
                &[("Obsolete", Some("\"a  b\"")), ("Fact", None), ("NotNull", None)],
            ),
            declared("Draw", K::Method, &["virtual"], &[]),
            declared("ToString", K::Method, &["override", "sealed"], &[]),
            declared("Sides", K::Field, &["Readonly"], &[]),
            declared("Corners", K::Field, &["Readonly"], &[]),
            declared("LoadAsync", K::Method, &["Static", "Async"], &[]),
            declared("Point", K::Type, &["Readonly"], &[]),
            declared("Leaf", K::Class, &["sealed"], &[]),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::C,
        support: Support::Absent("C writes none of the modifiers and no attribute form this reader reads"),
        extension: "c",
        source: "static int counter;\n\nint final_value(void) {\n    const char *text = \"virtual final override\";\n    return text[0];\n}\n",
        declarations: &[
            declared("counter", K::Variable, &["StaticStorage"], &[]),
            bare("final_value", K::Function),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::Cpp,
        support: Support::Supported,
        extension: "cpp",
        source: "class Base {\npublic:\n    virtual void draw() const;\n    virtual int area() = 0;\n    void plain() { const char *text = \"virtual override final\"; }\n};\n\
                 class Shape final : public Base {\npublic:\n    void paint() const override;\n    int measure() final { return 0; }\n    [[nodiscard]] [[deprecated(\"old  way\")]] int legacy();\n    static int count;\n    template <class T> [[nodiscard]] T convert(T value);\n};\n\
                 int with_callback(int (*handler)(int));\n",
        declarations: &[
            bare("Base", K::Class),
            declared("draw", K::Method, &["virtual"], &[]),
            declared("area", K::Method, &["virtual"], &[]),
            bare("plain", K::Method),
            declared("Shape", K::Class, &["final"], &[]),
            declared("paint", K::Method, &["override"], &[]),
            declared("measure", K::Method, &["final"], &[]),
            declared(
                "legacy",
                K::Method,
                &[],
                &[("nodiscard", None), ("deprecated", Some("\"old  way\""))],
            ),
            declared("count", K::Field, &["Static"], &[]),
            // Read through the `template` wrapper.
            declared("convert", K::Method, &[], &[("nodiscard", None)]),
            bare("with_callback", K::Function),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::Swift,
        support: Support::Supported,
        extension: "swift",
        source: "@available(*, deprecated)\n@objc\nfinal class Shape {\n    override func draw() {\n        let text = \"final override mutating\"\n    }\n    final var sides: Int = 0\n    static func make() -> Int { 0 }\n    func load() async {}\n}\n\
                 struct Point {\n    mutating func move() {}\n    nonmutating func peek() {}\n}\n\
                 protocol Movable {\n    mutating func step()\n}\n",
        declarations: &[
            declared("Shape", K::Type, &["final"], &[("available", Some("*, deprecated")), ("objc", None)]),
            declared("draw", K::Method, &["override"], &[]),
            declared("sides", K::Field, &["final"], &[]),
            declared("make", K::Method, &["Static"], &[]),
            declared("load", K::Method, &["Async"], &[]),
            // `mutating` is Swift's own keyword, never Rust's `Mut`.
            declared("move", K::Method, &["mutating"], &[]),
            bare("peek", K::Method),
            declared("step", K::Method, &["mutating"], &[]),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::Scala,
        support: Support::Supported,
        extension: "scala",
        source: "@deprecated(\"m\", \"1.0\")\nabstract class Shape {\n  def area: Double\n  @inline final override def toString = \"abstract final override\"\n  final val sides = 0\n}\n\
                 object Factory {\n  def make = 1\n}\nsealed trait Marker\n",
        declarations: &[
            declared("Shape", K::Class, &["abstract"], &[("deprecated", Some("\"m\", \"1.0\""))]),
            bare("area", K::Method),
            declared("toString", K::Method, &["final", "override"], &[("inline", None)]),
            declared("sides", K::Field, &["final"], &[]),
            declared("make", K::Method, &["Static"], &[]),
            // `sealed` is outside the Scala set.
            bare("Marker", K::Trait),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::Php,
        support: Support::Supported,
        extension: "php",
        source: "<?php\n#[Attribute]\nabstract class Shape {\n    abstract public function area(): float;\n    final public function name(): string { return \"abstract final\"; }\n    public static function make() {}\n}\nfinal class Leaf {}\n",
        declarations: &[
            // PHP attributes are outside the attribute set this reader reads.
            declared("Shape", K::Class, &["abstract"], &[]),
            declared("area", K::Method, &["abstract"], &[]),
            declared("name", K::Method, &["final"], &[]),
            declared("make", K::Method, &["Static"], &[]),
            declared("Leaf", K::Class, &["final"], &[]),
        ],
        limits: &[],
    },
    ModifiersRow {
        language: L::Perl,
        support: Support::Absent("Perl writes none of the modifiers; sub attributes are outside the attribute set"),
        extension: "pl",
        source: "sub final_value { my $text = \"abstract override\"; return $text; }\nsub value :lvalue { my $x; }\n",
        declarations: &[bare("final_value", K::Function), bare("value", K::Function)],
        limits: &[],
    },
    ModifiersRow {
        language: L::Bash,
        support: Support::Absent("Bash has no declaration modifiers or attributes"),
        extension: "sh",
        source: "final_value() {\n  local text=\"abstract final\"\n  echo \"$text\"\n}\n",
        declarations: &[bare("final_value", K::Function)],
        limits: &[],
    },
    ModifiersRow {
        language: L::Zsh,
        support: Support::Absent("Zsh has no declaration modifiers or attributes"),
        extension: "zsh",
        source: "function final_value {\n  local text=\"override\"\n  print $text\n}\n",
        declarations: &[bare("final_value", K::Function)],
        limits: &[],
    },
    ModifiersRow {
        language: L::Lua,
        support: Support::Absent("Lua has no declaration modifiers or attributes"),
        extension: "lua",
        source: "local function final_value()\n  local text = \"abstract final\"\n  return text\nend\n",
        declarations: &[bare("final_value", K::Function)],
        limits: &[],
    },
];

pub fn modifiers_matrix_languages() -> Vec<ProgrammingLanguage> {
    MODIFIERS_MATRIX.iter().map(|row| row.language).collect()
}

fn write_temp(source: &str, extension: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .expect("create temp source");
    file.write_all(source.as_bytes()).expect("write temp source");
    file
}

fn spelling(modifier: &Modifier) -> String {
    match modifier {
        Modifier::Custom(keyword) => keyword.clone(),
        other => format!("{other:?}"),
    }
}

fn record_of<'a>(index: &'a FileSymbolIndex, label: &str, name: &str, kind: SymbolKindV2) -> &'a SymbolRecord {
    let matches: Vec<&SymbolRecord> = index
        .symbols
        .iter()
        .filter(|record| record.identity.name == name && record.kind == kind)
        .collect();
    assert_eq!(matches.len(), 1, "{label}: expected one {kind:?} `{name}`, found {}", matches.len());
    matches[0]
}

fn assert_declared(index: &FileSymbolIndex, label: &str, expected: &Declared) {
    let record = record_of(index, label, expected.name, expected.kind);
    let modifiers: Vec<String> = record.modifiers.iter().map(spelling).collect();
    assert_eq!(modifiers, expected.modifiers, "{label}: `{}` modifiers", expected.name);
    let attributes: Vec<(String, Option<String>)> = record
        .attributes
        .iter()
        .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
        .collect();
    let wanted: Vec<(String, Option<String>)> = expected
        .attributes
        .iter()
        .map(|(name, value)| (name.to_string(), value.map(str::to_string)))
        .collect();
    assert_eq!(attributes, wanted, "{label}: `{}` attributes", expected.name);
}

crate::language_cases::language_tests!(modifiers_matrix_holds_on_complete_and_declarations_only_paths => check_modifiers_matrix_holds_on_complete_and_declarations_only_paths);

/// Each row's declarations report exactly their own modifiers and
/// attributes, on the complete index and on a declarations-only query of the
/// declaration's kind (which runs only the parse pass); an absent-construct
/// language reports no attribute and no modifier beyond `static`/`async` on
/// any record.
fn check_modifiers_matrix_holds_on_complete_and_declarations_only_paths(language: ProgrammingLanguage) {
    for row in MODIFIERS_MATRIX.iter().filter(|row| row.language == language) {
        let label = row.language.name();
        let file = write_temp(row.source, row.extension);
        let tree_file = TreeFile::new(file.path()).expect("parse row source");
        assert!(
            tree_file.syntax_diagnostics().is_empty(),
            "{label}: row source must parse cleanly: {:?}",
            tree_file.syntax_diagnostics()
        );
        let complete = tree_file.symbol_index_v2().expect("complete index");
        for expected in row.declarations {
            assert_declared(&complete, &format!("{label} complete"), expected);
            let selective = tree_file
                .query(&SymbolQuery::new().kinds([expected.kind]))
                .expect("declarations-only query")
                .index;
            assert!(
                selective.completed_passes.iter().all(|pass| pass.as_str() == "parse"),
                "{label}: declarations only"
            );
            assert_declared(&selective, &format!("{label} declarations-only"), expected);
        }
        if let Support::Absent(reason) = row.support {
            for record in &complete.symbols {
                assert!(
                    record.attributes.is_empty()
                        && record
                            .modifiers
                            .iter()
                            .all(|modifier| matches!(modifier, Modifier::Static | Modifier::StaticStorage | Modifier::Async)),
                    "{label}: {reason}: `{}` has {:?} {:?}",
                    record.identity.name,
                    record.modifiers,
                    record.attributes
                );
            }
        }
    }
}

/// Each demonstrated grammar limit still holds for the pinned grammar.
#[test]
fn modifiers_grammar_limits_hold_for_the_pinned_grammars() {
    for row in MODIFIERS_MATRIX {
        for limit in row.limits {
            let label = format!("{} ({})", row.language.name(), limit.grammar);
            let file = write_temp(limit.snippet, limit.extension);
            let tree_file = TreeFile::new(file.path()).expect("parse limit snippet");
            assert!(
                !tree_file.syntax_diagnostics().is_empty(),
                "{label}: expected a syntax error: {}",
                limit.observation
            );
        }
    }
}

/// Modifiers serialize in one order with no repeats, whatever order the
/// source writes them in: `override sealed` and `sealed override` give the
/// same list, and the JSON holds `Static` before `Async` before `Custom`.
#[test]
fn modifiers_serialize_in_one_order_whatever_the_source_order() {
    let file = write_temp(
        "class A {\n    public override sealed string ToString() => \"\";\n    public sealed override int GetHashCode() => 1;\n    static public async Task Run() {}\n    async static public Task Walk() {}\n}\n",
        "cs",
    );
    let index = TreeFile::new(file.path()).expect("parse").symbol_index_v2().expect("index");
    let order = |name: &str| -> serde_json::Value {
        serde_json::to_value(&record_of(&index, "order", name, K::Method).modifiers).expect("serialize")
    };
    assert_eq!(order("ToString"), order("GetHashCode"));
    assert_eq!(
        order("ToString"),
        serde_json::json!([{ "type": "Custom", "value": "override" }, { "type": "Custom", "value": "sealed" }])
    );
    assert_eq!(order("Run"), order("Walk"));
    assert_eq!(order("Run"), serde_json::json!([{ "type": "Static" }, { "type": "Async" }]));
}
