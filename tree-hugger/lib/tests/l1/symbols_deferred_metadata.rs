//! Metadata of the declarations whose precise kind was once deferred
//! (constructors, enum variants, type aliases, fields and namespaces): the
//! qualified name, access, `static`, declaring keyword, display header and
//! kind-specific detail each one reports, asserted through `symbols()` and
//! `symbol_index_v2()` alike. Kinds and containers are in
//! `symbols_kinds::DEFERRED_KIND_ROWS`.

use std::collections::HashSet;

use tree_hugger::{ProgrammingLanguage, SymbolKind, SymbolKindData, SymbolKindV2, Visibility};

use crate::symbols_fixtures::{extract_inline, in_file_name};
use crate::symbols_kinds::CPP_WRAPPED_CONSTRUCTORS;
use crate::symbols_static::assert_static;
use crate::symbols_visibility::assert_level;

use ProgrammingLanguage as L;

/// What a row asserts beyond the shared metadata.
#[derive(Debug, Clone, Copy)]
pub enum Detail {
    None,
    /// A function-like declaration's parameters, each `name` or
    /// `name=default`, and no return type.
    Parameters(&'static [&'static str]),
    /// A type alias's right-hand side as written.
    AliasTarget(&'static str),
}

/// One declaration: its name and container in `source`, and what it reports.
pub struct MetadataRow {
    pub language: ProgrammingLanguage,
    pub source: &'static str,
    pub name: &'static str,
    pub container: Option<&'static str>,
    /// The in-file qualified name.
    pub qualified: &'static str,
    /// `None` is Unknown.
    pub access: Option<Visibility>,
    pub is_static: bool,
    pub keyword: Option<&'static str>,
    /// The display header (`signature_text`).
    pub header: Option<&'static str>,
    pub detail: Detail,
}

const PUBLIC: Option<Visibility> = Some(Visibility::Public);
const MODULE_PRIVATE: Option<Visibility> = Some(Visibility::ModulePrivate);
const PACKAGE: Option<Visibility> = Some(Visibility::Package);
const PROTECTED: Option<Visibility> = Some(Visibility::Protected);
const PRIVATE: Option<Visibility> = Some(Visibility::Private);
const INTERNAL: Option<Visibility> = Some(Visibility::Internal);
const UNKNOWN: Option<Visibility> = None;

/// One section per language; a language's rows are added to its own section
/// only.
pub const DEFERRED_METADATA_ROWS: &[MetadataRow] = &[
    // TypeScript: enum members inherit the enum's access; a namespace is a
    // top-level declaration.
    MetadataRow {
        language: L::TypeScript,
        source: "export enum Ex { Green, Blue = 2 }\n",
        name: "Green",
        container: Some("Ex"),
        qualified: "Ex.Green",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("Green"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::TypeScript,
        source: "enum E { Red, Blue = 2 }\nexport const x = 1;\n",
        name: "Blue",
        container: Some("E"),
        qualified: "E.Blue",
        access: MODULE_PRIVATE,
        is_static: false,
        keyword: None,
        header: Some("Blue"),
        detail: Detail::None,
    },
    // A classic script has no module access.
    MetadataRow {
        language: L::TypeScript,
        source: "enum S { A }\n",
        name: "A",
        container: Some("S"),
        qualified: "S.A",
        access: UNKNOWN,
        is_static: false,
        keyword: None,
        header: Some("A"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::TypeScript,
        source: "enum Q { \"x y\" = 1 }\nexport {};\n",
        name: "x y",
        container: Some("Q"),
        qualified: "Q.x y",
        access: MODULE_PRIVATE,
        is_static: false,
        keyword: None,
        header: Some("\"x y\""),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::TypeScript,
        source: "namespace Space {\n  export function inside() {}\n}\nexport {};\n",
        name: "Space",
        container: None,
        qualified: "Space",
        access: MODULE_PRIVATE,
        is_static: false,
        keyword: Some("namespace"),
        header: Some("namespace Space"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::TypeScript,
        source: "namespace Space {\n  export function inside() {}\n}\nexport {};\n",
        name: "inside",
        container: Some("Space"),
        qualified: "Space.inside",
        access: UNKNOWN,
        is_static: false,
        keyword: Some("function"),
        header: Some("function inside()"),
        detail: Detail::Parameters(&[]),
    },
    MetadataRow {
        language: L::TypeScript,
        source: "export module Mod {\n  namespace Inner {\n    const deep = 1;\n  }\n}\n",
        name: "Mod",
        container: None,
        qualified: "Mod",
        access: PUBLIC,
        is_static: false,
        keyword: Some("module"),
        header: Some("module Mod"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::TypeScript,
        source: "export module Mod {\n  namespace Inner {\n    const deep = 1;\n  }\n}\n",
        name: "deep",
        container: Some("Inner"),
        qualified: "Mod.Inner.deep",
        access: UNKNOWN,
        is_static: false,
        keyword: Some("const"),
        header: Some("const deep"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::TypeScript,
        source: "export namespace A.B {\n  export const v = 1;\n}\n",
        name: "A.B",
        container: None,
        qualified: "A.B",
        access: PUBLIC,
        is_static: false,
        keyword: Some("namespace"),
        header: Some("namespace A.B"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::TypeScript,
        source: "export namespace A.B {\n  export const v = 1;\n}\n",
        name: "v",
        container: Some("A.B"),
        qualified: "A.B.v",
        access: UNKNOWN,
        is_static: false,
        keyword: Some("const"),
        header: Some("const v"),
        detail: Detail::None,
    },
    // Go: access follows the name's first letter (Unicode aware); fields
    // are instance members.
    MetadataRow {
        language: L::Go,
        source: "package p\n\ntype Alias = int\n",
        name: "Alias",
        container: None,
        qualified: "p.Alias",
        access: PUBLIC,
        is_static: false,
        keyword: Some("type"),
        header: Some("type Alias"),
        detail: Detail::AliasTarget("int"),
    },
    MetadataRow {
        language: L::Go,
        source: "package p\n\ntype (\n\tgrouped = map[string]int\n\tDefined int\n)\n",
        name: "grouped",
        container: None,
        qualified: "p.grouped",
        access: PACKAGE,
        is_static: false,
        keyword: Some("type"),
        header: Some("grouped"),
        detail: Detail::AliasTarget("map[string]int"),
    },
    MetadataRow {
        language: L::Go,
        source: "package p\n\ntype GenAlias[T any] = []T\n",
        name: "GenAlias",
        container: None,
        qualified: "p.GenAlias",
        access: PUBLIC,
        is_static: false,
        keyword: Some("type"),
        header: Some("type GenAlias[T any]"),
        detail: Detail::AliasTarget("[]T"),
    },
    MetadataRow {
        language: L::Go,
        source: "package p\n\ntype S struct {\n\tX, y int\n\tÜber bool\n}\n",
        name: "X",
        container: Some("S"),
        qualified: "p.S.X",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("X, y int"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Go,
        source: "package p\n\ntype S struct {\n\tX, y int\n\tÜber bool\n}\n",
        name: "y",
        container: Some("S"),
        qualified: "p.S.y",
        access: PACKAGE,
        is_static: false,
        keyword: None,
        header: Some("X, y int"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Go,
        source: "package p\n\ntype S struct {\n\tX, y int\n\tÜber bool\n}\n",
        name: "Über",
        container: Some("S"),
        qualified: "p.S.Über",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("Über bool"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Go,
        source: "package p\n\ntype (\n\tA struct{ P int }\n\tB struct{ q int }\n)\n",
        name: "q",
        container: Some("B"),
        qualified: "p.B.q",
        access: PACKAGE,
        is_static: false,
        keyword: None,
        header: Some("q int"),
        detail: Detail::None,
    },
    // Python: a constructor keeps its parameters, defaults included, and is
    // not static for want of a decorator.
    MetadataRow {
        language: L::Python,
        source: "class C:\n    def __init__(self, value=1, *args):\n        \"\"\"Init doc.\"\"\"\n",
        name: "__init__",
        container: Some("C"),
        qualified: "C.__init__",
        access: UNKNOWN,
        is_static: false,
        keyword: Some("def"),
        header: Some("def __init__(self, value=1, *args)"),
        detail: Detail::Parameters(&["self", "value=1", "args"]),
    },
    MetadataRow {
        language: L::Python,
        source: "class D:\n    @decorate\n    def __init__(self):\n        pass\n",
        name: "__init__",
        container: Some("D"),
        qualified: "D.__init__",
        access: UNKNOWN,
        is_static: false,
        keyword: Some("def"),
        header: Some("def __init__(self)"),
        detail: Detail::Parameters(&["self"]),
    },
    // Java
    // Java: an enum's constants are Public and its unmodified constructor
    // Private.
    MetadataRow {
        language: L::Java,
        source: "package app;\nclass C {\n    /** Ctor doc. */\n    protected C(String s, int... rest) {}\n}\n",
        name: "C",
        container: Some("C"),
        qualified: "app.C.C",
        access: PROTECTED,
        is_static: false,
        keyword: None,
        header: Some("protected C(String s, int... rest)"),
        detail: Detail::Parameters(&["s", "rest"]),
    },
    MetadataRow {
        language: L::Java,
        source: "enum E {\n    RED,\n    BLUE(2);\n    E() {}\n}\n",
        name: "BLUE",
        container: Some("E"),
        qualified: "E.BLUE",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("BLUE(2)"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Java,
        source: "enum E {\n    RED;\n    E() {}\n}\n",
        name: "E",
        container: Some("E"),
        qualified: "E.E",
        access: PRIVATE,
        is_static: false,
        keyword: None,
        header: Some("E()"),
        detail: Detail::Parameters(&[]),
    },
    // C#: a constructor without an access keyword is Private, like any
    // class member; enum members are Public.
    MetadataRow {
        language: L::CSharp,
        source: "namespace App;\nclass C\n{\n    internal C(string s) : this(1) { }\n}\n",
        name: "C",
        container: Some("C"),
        qualified: "App.C.C",
        access: INTERNAL,
        is_static: false,
        keyword: None,
        header: Some("internal C(string s) : this(1)"),
        detail: Detail::Parameters(&["s"]),
    },
    MetadataRow {
        language: L::CSharp,
        source: "struct S\n{\n    public int X;\n    S(int x) => X = x;\n}\n",
        name: "S",
        container: Some("S"),
        qualified: "S.S",
        access: PRIVATE,
        is_static: false,
        keyword: None,
        header: Some("S(int x)"),
        detail: Detail::Parameters(&["x"]),
    },
    MetadataRow {
        language: L::CSharp,
        source: "enum E { Red, Blue = 2 }\n",
        name: "Blue",
        container: Some("E"),
        qualified: "E.Blue",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("Blue"),
        detail: Detail::None,
    },
    // C: enumerators keep Unknown access under the linkage policy, and the
    // initializer stays in the declaration text, not the header.
    MetadataRow {
        language: L::C,
        source: "enum E { RED, BLUE = 2 };\n",
        name: "BLUE",
        container: Some("E"),
        qualified: "E.BLUE",
        access: UNKNOWN,
        is_static: false,
        keyword: None,
        header: Some("BLUE"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::C,
        source: "enum { ANON_A };\n",
        name: "ANON_A",
        container: None,
        qualified: "ANON_A",
        access: UNKNOWN,
        is_static: false,
        keyword: None,
        header: Some("ANON_A"),
        detail: Detail::None,
    },
    // C++: a constructor's access is its class's access section;
    // enumerators keep Unknown access.
    MetadataRow {
        language: L::Cpp,
        source: "namespace N {\nclass C {\npublic:\n    explicit C(double d, int n = 2);\n};\n}\n",
        name: "C",
        container: Some("C"),
        qualified: "N::C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("explicit C(double d, int n = 2)"),
        detail: Detail::Parameters(&["d", "n=2"]),
    },
    MetadataRow {
        language: L::Cpp,
        source: "class C {\n    template <typename T> C(T t) {}\n};\n",
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PRIVATE,
        is_static: false,
        keyword: None,
        header: Some("C(T t)"),
        detail: Detail::Parameters(&["t"]),
    },
    // An attribute or parentheses around a constructor's name keep its
    // container, access and parameters.
    MetadataRow {
        language: L::Cpp,
        source: CPP_WRAPPED_CONSTRUCTORS[0],
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("C [[deprecated]] ()"),
        detail: Detail::Parameters(&[]),
    },
    MetadataRow {
        language: L::Cpp,
        source: CPP_WRAPPED_CONSTRUCTORS[1],
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("C [[deprecated]] (int x)"),
        detail: Detail::Parameters(&["x"]),
    },
    MetadataRow {
        language: L::Cpp,
        source: CPP_WRAPPED_CONSTRUCTORS[2],
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("C [[deprecated]] () = default;"),
        detail: Detail::Parameters(&[]),
    },
    MetadataRow {
        language: L::Cpp,
        source: CPP_WRAPPED_CONSTRUCTORS[3],
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("C [[deprecated]] (int x) = delete;"),
        detail: Detail::Parameters(&["x"]),
    },
    MetadataRow {
        language: L::Cpp,
        source: CPP_WRAPPED_CONSTRUCTORS[4],
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("C [[deprecated]] (T x)"),
        detail: Detail::Parameters(&["x"]),
    },
    MetadataRow {
        language: L::Cpp,
        source: CPP_WRAPPED_CONSTRUCTORS[5],
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("explicit C [[deprecated]] (int x)"),
        detail: Detail::Parameters(&["x"]),
    },
    MetadataRow {
        language: L::Cpp,
        source: CPP_WRAPPED_CONSTRUCTORS[6],
        name: "C",
        container: Some("C"),
        qualified: "C::C",
        access: PUBLIC,
        is_static: false,
        keyword: None,
        header: Some("(C)(int x)"),
        detail: Detail::Parameters(&["x"]),
    },
    MetadataRow {
        language: L::Cpp,
        source: "enum class Scoped : int { S_A, S_B = 4 };\n",
        name: "S_B",
        container: Some("Scoped"),
        qualified: "Scoped::S_B",
        access: UNKNOWN,
        is_static: false,
        keyword: None,
        header: Some("S_B"),
        detail: Detail::None,
    },
    // Swift: an initializer keeps the source name `init`, its keyword, and
    // its parameter defaults in the header.
    MetadataRow {
        language: L::Swift,
        source: "public class C {\n    /// Init doc.\n    public init(x: Int, y: Int = 2) {}\n}\n",
        name: "init",
        container: Some("C"),
        qualified: "C.init",
        access: PUBLIC,
        is_static: false,
        keyword: Some("init"),
        header: Some("public init(x: Int, y: Int = 2)"),
        detail: Detail::Parameters(&["x", "y=2"]),
    },
    MetadataRow {
        language: L::Swift,
        source: "struct S {\n    init?(s: String) { return nil }\n}\n",
        name: "init",
        container: Some("S"),
        qualified: "S.init",
        access: INTERNAL,
        is_static: false,
        keyword: Some("init"),
        header: Some("init?(s: String)"),
        detail: Detail::Parameters(&["s"]),
    },
    MetadataRow {
        language: L::Swift,
        source: "class C {\n    private typealias Alias = [String: Int]\n}\n",
        name: "Alias",
        container: Some("C"),
        qualified: "C.Alias",
        access: PRIVATE,
        is_static: false,
        keyword: Some("typealias"),
        header: Some("private typealias Alias"),
        detail: Detail::AliasTarget("[String: Int]"),
    },
    MetadataRow {
        language: L::Swift,
        source: "public typealias G<T> = Array<T>\n",
        name: "G",
        container: None,
        qualified: "G",
        access: PUBLIC,
        is_static: false,
        keyword: Some("typealias"),
        header: Some("public typealias G<T>"),
        detail: Detail::AliasTarget("Array<T>"),
    },
    // A function's parameter default stays in its header too.
    MetadataRow {
        language: L::Swift,
        source: "func f(x: Int, y: Int = 2) {}\n",
        name: "f",
        container: None,
        qualified: "f",
        access: INTERNAL,
        is_static: false,
        keyword: Some("func"),
        header: Some("func f(x: Int, y: Int = 2)"),
        detail: Detail::Parameters(&["x", "y=2"]),
    },
    // Scala: a case inherits the enum's recorded access; an object's
    // fields are static, a class's or trait's are not.
    MetadataRow {
        language: L::Scala,
        source: "package app\n\nenum Color {\n  case Red, Blue\n  case Value(x: Int)\n}\n",
        name: "Value",
        container: Some("Color"),
        qualified: "app.Color.Value",
        access: PUBLIC,
        is_static: false,
        keyword: Some("case"),
        header: Some("Value(x: Int)"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Scala,
        source: "package app\n\nenum Color {\n  case Red, Blue\n  case Value(x: Int)\n}\n",
        name: "x",
        container: Some("Color"),
        qualified: "app.Color.Value.x",
        access: UNKNOWN,
        is_static: false,
        keyword: None,
        header: Some("x: Int"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Scala,
        source: "object O {\n  type Gen[A] = List[A]\n}\n",
        name: "Gen",
        container: Some("O"),
        qualified: "O.Gen",
        access: PUBLIC,
        is_static: false,
        keyword: Some("type"),
        header: Some("type Gen[A]"),
        detail: Detail::AliasTarget("List[A]"),
    },
    MetadataRow {
        language: L::Scala,
        source: "object O {\n  private val hidden = 3\n}\n",
        name: "hidden",
        container: Some("O"),
        qualified: "O.hidden",
        access: PRIVATE,
        is_static: true,
        keyword: Some("val"),
        header: Some("private val hidden"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Scala,
        source: "trait T {\n  var d: Int\n}\n",
        name: "d",
        container: Some("T"),
        qualified: "T.d",
        access: PUBLIC,
        is_static: false,
        keyword: Some("var"),
        header: Some("var d: Int"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Scala,
        source: "class K {\n  protected var f = 2\n}\n",
        name: "f",
        container: Some("K"),
        qualified: "K.f",
        access: PROTECTED,
        is_static: false,
        keyword: Some("var"),
        header: Some("protected var f"),
        detail: Detail::None,
    },
    // PHP: a namespace keeps its full name and qualifies members once; enum
    // cases are Public.
    MetadataRow {
        language: L::Php,
        source: "<?php\nnamespace App;\n\nclass C {\n    /** Ctor doc. */\n    public function __construct(int $x = 1, string ...$rest) {}\n}\n",
        name: "__construct",
        container: Some("C"),
        qualified: "App\\C::__construct",
        access: PUBLIC,
        is_static: false,
        keyword: Some("function"),
        header: Some("public function __construct(int $x = 1, string ...$rest)"),
        detail: Detail::Parameters(&["x=1", "rest"]),
    },
    MetadataRow {
        language: L::Php,
        source: "<?php\nnamespace App;\n\nfunction inside() {}\n",
        name: "App",
        container: None,
        qualified: "App",
        access: UNKNOWN,
        is_static: false,
        keyword: Some("namespace"),
        header: Some("namespace App"),
        detail: Detail::None,
    },
    MetadataRow {
        language: L::Php,
        source: "<?php\nnamespace App;\n\nfunction inside() {}\n",
        name: "inside",
        container: None,
        qualified: "App\\inside",
        access: PUBLIC,
        is_static: false,
        keyword: Some("function"),
        header: Some("function inside()"),
        detail: Detail::Parameters(&[]),
    },
    MetadataRow {
        language: L::Php,
        source: "<?php\nnamespace App\\Braced {\n    function inside() {}\n}\n",
        name: "inside",
        container: Some("App\\Braced"),
        qualified: "App\\Braced\\inside",
        access: PUBLIC,
        is_static: false,
        keyword: Some("function"),
        header: Some("function inside()"),
        detail: Detail::Parameters(&[]),
    },
    MetadataRow {
        language: L::Php,
        source: "<?php\nenum Backed: string {\n    case H = 'h';\n}\n",
        name: "H",
        container: Some("Backed"),
        qualified: "Backed::H",
        access: PUBLIC,
        is_static: false,
        keyword: Some("case"),
        header: Some("case H"),
        detail: Detail::None,
    },
];

/// Asserts every row on both public paths.
pub fn assert_metadata_rows(rows: &[MetadataRow]) {
    for row in rows {
        let extracted = extract_inline(row.source, row.language.extensions()[0]);
        let label = format!("{:?} {:?}: `{}`", row.language, row.source, row.name);
        let symbol = extracted.symbol(row.name, row.container);
        let record = extracted.record(row.name, row.container);

        assert_eq!(symbol.qualified_name, row.qualified, "{label}: symbols() qualified name");
        assert_eq!(in_file_name(record), row.qualified, "{label}: record qualified name");
        assert_level(&extracted, row.name, row.container, row.access);
        assert_static(&extracted, row.name, row.container, row.is_static, false);
        assert_eq!(record.identity.keyword.as_deref(), row.keyword, "{label}: keyword");
        assert_eq!(record.source.signature_text.as_deref(), row.header, "{label}: header");

        match row.detail {
            Detail::None => {}
            Detail::Parameters(expected) => {
                let signature = symbol
                    .signature
                    .as_ref()
                    .unwrap_or_else(|| panic!("{label}: no signature"));
                let parameters: Vec<String> = signature
                    .parameters
                    .iter()
                    .map(|parameter| match &parameter.default_value {
                        Some(default) => format!("{}={default}", parameter.name),
                        None => parameter.name.clone(),
                    })
                    .collect();
                assert_eq!(parameters, expected, "{label}: parameters");
                assert_eq!(signature.return_type, None, "{label}: no invented return type");
                let SymbolKindData::Function(function) = &record.kind_data else {
                    panic!("{label}: record has {:?}, not function data", record.kind_data);
                };
                assert_eq!(function.arity as usize, expected.len(), "{label}: record arity");
            }
            Detail::AliasTarget(expected) => {
                let SymbolKindData::TypeAlias(alias) = &record.kind_data else {
                    panic!("{label}: record has {:?}, not alias data", record.kind_data);
                };
                assert_eq!(alias.aliased_type_text.as_deref(), Some(expected), "{label}: alias target");
            }
        }
    }
}

#[test]
fn deferred_declarations_report_their_metadata() {
    assert_metadata_rows(DEFERRED_METADATA_ROWS);
}

/// Overloaded constructors: `(language, source, constructor name, each
/// overload's parameter names in source order)`.
pub const CONSTRUCTOR_OVERLOADS: &[(ProgrammingLanguage, &str, &str, &[&[&str]])] = &[
    (L::Java, "class C {\n    C() {}\n    C(int x) {}\n    C(int x, String y) {}\n}\n", "C", &[&[], &["x"], &["x", "y"]]),
    (L::CSharp, "class C\n{\n    public C() { }\n    public C(int x) : this() { }\n    static C() { }\n}\n", "C", &[&[], &["x"]]),
    (L::Cpp, "class C {\npublic:\n    C();\n    C(int x) {}\n    C(int x, int y) = delete;\n    ~C();\n};\n", "C", &[&[], &["x"], &["x", "y"]]),
    (L::Swift, "class C {\n    init() {}\n    init?(s: String) { return nil }\n    convenience init(x: Int, y: Int = 2) { self.init() }\n    deinit {}\n}\n", "init", &[&[], &["s"], &["x", "y"]]),
];

/// Each overload is its own Constructor record, with its own parameters and
/// a distinct ID, in source order.
#[test]
fn overloaded_constructors_are_separate_records() {
    for &(language, source, name, overloads) in CONSTRUCTOR_OVERLOADS {
        let extracted = extract_inline(source, language.extensions()[0]);
        let label = format!("{language:?} {source:?}");
        let parameters: Vec<Vec<String>> = extracted
            .symbols
            .iter()
            .filter(|symbol| symbol.name == name && symbol.kind == SymbolKind::Constructor)
            .map(|symbol| {
                symbol
                    .signature
                    .as_ref()
                    .map(|signature| signature.parameters.iter().map(|parameter| parameter.name.clone()).collect())
                    .unwrap_or_default()
            })
            .collect();
        let expected: Vec<Vec<String>> = overloads
            .iter()
            .map(|names| names.iter().map(|name| name.to_string()).collect())
            .collect();
        assert_eq!(parameters, expected, "{label}: overloads in symbols()");

        let ids: HashSet<&str> = extracted
            .records
            .iter()
            .filter(|record| record.identity.name == name && record.kind == SymbolKindV2::Constructor)
            .map(|record| record.id.0.as_str())
            .collect();
        assert_eq!(ids.len(), overloads.len(), "{label}: one record and ID per overload");
    }
}
