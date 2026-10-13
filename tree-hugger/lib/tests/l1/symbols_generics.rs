//! The generics matrix: `type_info.generic_params` and
//! `type_info.constraints` hold exactly the parameters a declaration writes
//! itself (never an enclosing type's or `impl`'s), with bounds, defaults and
//! variance as written, on the complete index, on a declarations-only
//! query and through the persistent cache (a warm hit and its projections).
//! Each language has one row: supported forms with positive and
//! negative declarations, an absent construct, and its demonstrated grammar
//! limits.

use std::io::Write;

use tree_hugger::cache::{CacheLookup, PersistentCache, SymbolCache};
use tree_hugger::{
    FileSymbolIndex, SourceFile, GenericParamKind, ProgrammingLanguage, SymbolKindV2, SymbolQuery, SymbolRecord,
    TreeFile, TreeHuggerError,
};

use GenericParamKind::{Const, Lifetime, Type};
use ProgrammingLanguage as L;
use SymbolKindV2 as K;

/// `(name, kind, constraints, default, variance, position)`.
pub type Param = (
    &'static str,
    GenericParamKind,
    &'static [&'static str],
    Option<&'static str>,
    Option<&'static str>,
    u16,
);

type OwnedParam = (String, GenericParamKind, Vec<String>, Option<String>, Option<String>, u16);

/// One declaration of a row's source and what it must report.
pub struct Declared {
    pub name: &'static str,
    pub kind: SymbolKindV2,
    pub params: &'static [Param],
    pub constraints: &'static [&'static str],
}

/// What a demonstrated grammar limit does to the reader.
pub enum LimitRead {
    /// The pinned grammar rejects the form.
    SyntaxError,
    /// The form parses, but its shape gives this reading.
    Reads(Declared),
}

/// A form the pinned grammar cannot represent as written.
pub struct Limit {
    pub grammar: &'static str,
    pub extension: &'static str,
    pub snippet: &'static str,
    pub observation: &'static str,
    pub read: LimitRead,
}

pub enum Support {
    Supported,
    /// The language has no declared generic parameters in required scope.
    Absent(&'static str),
}

pub struct GenericsRow {
    pub language: ProgrammingLanguage,
    pub support: Support,
    pub extension: &'static str,
    pub source: &'static str,
    pub declarations: &'static [Declared],
    pub limits: &'static [Limit],
}

const fn declared(name: &'static str, kind: SymbolKindV2, params: &'static [Param]) -> Declared {
    Declared {
        name,
        kind,
        params,
        constraints: &[],
    }
}

const fn plain(name: &'static str, kind: SymbolKindV2) -> Declared {
    declared(name, kind, &[])
}

const fn param(name: &'static str, position: u16) -> Param {
    (name, Type, &[], None, None, position)
}

const fn bounded(name: &'static str, constraints: &'static [&'static str], position: u16) -> Param {
    (name, Type, constraints, None, None, position)
}

/// A C++ member that writes `requires Small<T>` on its enclosing class's
/// parameter and declares no parameters of its own.
const fn owns_requirement_only(name: &'static str, kind: SymbolKindV2) -> Declared {
    Declared {
        name,
        kind,
        params: &[],
        constraints: &["Small<T>"],
    }
}

/// `template <template <class> class TT = Box, class T = int, int N = 3>`.
const DEFAULTED_TEMPLATE_TEMPLATE: &[Param] = &[
    ("TT", Type, &[], Some("Box"), None, 0),
    ("T", Type, &[], Some("int"), None, 1),
    ("N", Const, &[], Some("3"), None, 2),
];

pub const GENERICS_MATRIX: &[GenericsRow] = &[
    GenericsRow {
        language: L::Rust,
        support: Support::Supported,
        extension: "rs",
        source: "pub struct Bag<T: Clone = String> {\n    value: T,\n}\n\
                 impl<T: Clone + Send> Bag<T> {\n    pub fn get<U>(&self, value: U) -> U where U: Copy { value }\n    pub fn size(&self) -> usize { 0 }\n}\n\
                 pub trait Store<'a, K: 'a + Ord, const N: usize = 4>: Sized\nwhere\n    K: Clone,\n    Vec<K>: Send,\n    for<'b> &'b K: Into<u8>,\n{\n    fn put<V: Into<K> /* bound */ + Copy>(&self, value: V);\n}\n\
                 pub type Pair<A, B = A> = (A, B);\n\
                 pub fn uses<T>(items: T) where T: IntoIterator, T::Item: Copy {}\n\
                 pub struct Tagged<#[cfg(all())] T, U>(T, U);\n\
                 pub fn infer(items: Vec<u8>) -> usize { items.len() }\n",
        declarations: &[
            declared("Bag", K::Type, &[("T", Type, &["Clone"], Some("String"), None, 0)]),
            // The impl's `T` (and its bounds) belong to neither the type nor
            // the method.
            Declared {
                name: "get",
                kind: K::Method,
                params: &[bounded("U", &["Copy"], 0)],
                constraints: &[],
            },
            plain("size", K::Method),
            Declared {
                name: "Store",
                kind: K::Trait,
                params: &[
                    ("'a", Lifetime, &[], None, None, 0),
                    ("K", Type, &["'a", "Ord", "Clone"], None, None, 1),
                    ("N", Const, &[], Some("4"), None, 2),
                ],
                constraints: &["Vec<K>: Send", "for<'b> &'b K: Into<u8>"],
            },
            declared("put", K::Method, &[bounded("V", &["Into<K>", "Copy"], 0)]),
            declared("Pair", K::TypeAlias, &[param("A", 0), ("B", Type, &[], Some("A"), None, 1)]),
            Declared {
                name: "uses",
                kind: K::Function,
                params: &[bounded("T", &["IntoIterator"], 0)],
                constraints: &["T::Item: Copy"],
            },
            // An attribute in the list takes no position.
            declared("Tagged", K::Type, &[param("T", 0), param("U", 1)]),
            plain("infer", K::Function),
        ],
        limits: &[],
    },
    GenericsRow {
        language: L::TypeScript,
        support: Support::Supported,
        extension: "ts",
        source: "export class Box<T extends object = {}> {\n  map<U>(fn: (value: T) => U): U { return fn({} as T); }\n  size(): number { return 0; }\n}\n\
                 interface Keyed<K extends string | number, V = unknown> {\n  get<R>(key: K): R;\n}\n\
                 type Picked<T, K extends keyof T = keyof T> = { [P in K]: T[P] };\n\
                 function first<T extends Array<Map<string, number>>>(items: T): T { return items; }\n\
                 const arrow = <A,>(value: A): A => value;\n\
                 function infer(items: number[]) { return items; }\n",
        declarations: &[
            declared("Box", K::Class, &[("T", Type, &["object"], Some("{}"), None, 0)]),
            declared("map", K::Method, &[param("U", 0)]),
            plain("size", K::Method),
            declared(
                "Keyed",
                K::Interface,
                &[bounded("K", &["string | number"], 0), ("V", Type, &[], Some("unknown"), None, 1)],
            ),
            declared("get", K::Method, &[param("R", 0)]),
            declared(
                "Picked",
                K::TypeAlias,
                &[param("T", 0), ("K", Type, &["keyof T"], Some("keyof T"), None, 1)],
            ),
            declared("first", K::Function, &[bounded("T", &["Array<Map<string, number>>"], 0)]),
            declared("arrow", K::Function, &[param("A", 0)]),
            plain("infer", K::Function),
        ],
        limits: &[Limit {
            grammar: "tree-sitter-typescript 0.23.2",
            extension: "ts",
            snippet: "interface Variant<in T> {}\n",
            observation: "`in`/`out` variance annotations (TS 4.7) are a syntax error",
            read: LimitRead::SyntaxError,
        }],
    },
    GenericsRow {
        language: L::JavaScript,
        support: Support::Absent("JavaScript declares no generic parameters; a JSDoc `@template` is not read"),
        extension: "js",
        source: "/** @template T */\nexport class Box {\n  map(value) { return value; }\n}\n/** @template T\n * @param {T} value */\nfunction identity(value) { return value; }\n",
        declarations: &[plain("Box", K::Class), plain("map", K::Method), plain("identity", K::Function)],
        limits: &[],
    },
    GenericsRow {
        language: L::Go,
        support: Support::Supported,
        extension: "go",
        source: "package p\n\n\
                 type Pair[K comparable, V any] struct {\n\tkey   K\n\tvalue V\n}\n\n\
                 type Number interface {\n\t~int | ~float64\n}\n\n\
                 func Sum[T ~int | ~float64, U Number](values []T, extra U) T {\n\tvar total T\n\treturn total\n}\n\n\
                 func Keys[K, V any](m map[K]V) []K { return nil }\n\n\
                 func (p Pair[K, V]) Key() K { return p.key }\n\n\
                 type List[T any] = []T\n\n\
                 type (\n\tGrouped[A any] struct{}\n\tPlainGroup int\n)\n\n\
                 func infer(x int) int { return x }\n",
        declarations: &[
            declared("Pair", K::Type, &[bounded("K", &["comparable"], 0), bounded("V", &["any"], 1)]),
            plain("Number", K::Type),
            // A union or `~` term stays constraint text.
            declared("Sum", K::Function, &[bounded("T", &["~int | ~float64"], 0), bounded("U", &["Number"], 1)]),
            declared("Keys", K::Function, &[bounded("K", &["any"], 0), bounded("V", &["any"], 1)]),
            // The receiver's `[K, V]` are arguments of `Pair`, not
            // parameters of the method.
            plain("Key", K::Method),
            declared("List", K::TypeAlias, &[bounded("T", &["any"], 0)]),
            declared("Grouped", K::Type, &[bounded("A", &["any"], 0)]),
            plain("PlainGroup", K::Type),
            plain("infer", K::Function),
        ],
        limits: &[],
    },
    GenericsRow {
        language: L::Python,
        support: Support::Supported,
        extension: "py",
        source: "from typing import Generic, TypeVar\n\nT = TypeVar(\"T\")\n\n\
                 class Box[T: (int, str), *Ts, **P]:\n    def map[U](self, value: U) -> U:\n        return value\n\n    def size(self) -> int:\n        return 0\n\n\
                 def first[T](items: list[T]) -> T:\n    return items[0]\n\n\
                 class Legacy(Generic[T]):\n    pass\n\n\
                 def infer(value):\n    return value\n",
        declarations: &[
            declared("Box", K::Class, &[bounded("T", &["(int, str)"], 0), param("Ts", 1), param("P", 2)]),
            declared("map", K::Method, &[param("U", 0)]),
            plain("size", K::Method),
            declared("first", K::Function, &[param("T", 0)]),
            // `Generic[T]` in the bases declares nothing.
            plain("Legacy", K::Class),
            plain("infer", K::Function),
        ],
        limits: &[Limit {
            grammar: "tree-sitter-python 0.25.0",
            extension: "py",
            snippet: "class Defaulted[T = str]:\n    pass\n",
            observation: "a type-parameter default (PEP 696) is a syntax error",
            read: LimitRead::SyntaxError,
        }],
    },
    GenericsRow {
        language: L::Java,
        support: Support::Supported,
        extension: "java",
        source: "class Box<T extends Comparable<T> & Cloneable> {\n    <U extends Number> U map(U value) { return value; }\n    <V> Box(V seed) {}\n    int size() { return 0; }\n}\n\
                 interface Keyed<K, V extends java.util.List<K>> {}\n\
                 record Pair<A, B>(A first, B second) {}\n",
        declarations: &[
            declared("Box", K::Class, &[bounded("T", &["Comparable<T>", "Cloneable"], 0)]),
            declared("map", K::Method, &[bounded("U", &["Number"], 0)]),
            declared("Box", K::Constructor, &[param("V", 0)]),
            plain("size", K::Method),
            declared("Keyed", K::Interface, &[param("K", 0), bounded("V", &["java.util.List<K>"], 1)]),
            declared("Pair", K::Type, &[param("A", 0), param("B", 1)]),
        ],
        limits: &[],
    },
    GenericsRow {
        language: L::CSharp,
        support: Support::Supported,
        extension: "cs",
        source: "interface IVariant<in T, out U> where T : class {}\n\
                 class Box<T> where T : class, IComparable<T>, new() {\n    public V Map<V>(V value) where V : notnull { return value; }\n    public int Size() { return 0; }\n}\n\
                 struct Cell<T> {}\n\
                 record Pair<A, B>(A First, B Second);\n\
                 class Loose where T : class {\n    public void Tight() where U : struct {}\n}\n",
        declarations: &[
            declared(
                "IVariant",
                K::Interface,
                &[("T", Type, &["class"], None, Some("in"), 0), ("U", Type, &[], None, Some("out"), 1)],
            ),
            declared("Box", K::Class, &[bounded("T", &["class", "IComparable<T>", "new()"], 0)]),
            declared("Map", K::Method, &[bounded("V", &["notnull"], 0)]),
            plain("Size", K::Method),
            declared("Cell", K::Type, &[param("T", 0)]),
            declared("Pair", K::Type, &[param("A", 0), param("B", 1)]),
            // The grammar accepts `where` on a declaration with no list
            // (the compiler rejects it); it names no declared parameter.
            Declared {
                name: "Loose",
                kind: K::Class,
                params: &[],
                constraints: &["T : class"],
            },
            Declared {
                name: "Tight",
                kind: K::Method,
                params: &[],
                constraints: &["U : struct"],
            },
        ],
        limits: &[],
    },
    GenericsRow {
        language: L::C,
        support: Support::Absent("C declares no generic parameters"),
        extension: "c",
        source: "struct Box { int value; };\nint run(int value) { return value; }\n",
        declarations: &[plain("Box", K::Type), plain("run", K::Function)],
        limits: &[],
    },
    GenericsRow {
        language: L::Cpp,
        support: Support::Supported,
        extension: "cpp",
        source: "template <typename T = int, int N = 3, class... Ts>\nclass Box {\npublic:\n    template <class U> void map(U value);\n    void size();\n};\n\
                 template <typename T> requires Small<T> void fits(T value) {}\n\
                 template <class T> void trailing(T value) requires Small<T> {}\n\
                 template <class T> using Alias = Box<T>;\n\
                 template <template <class> class TT, typename, int... Ns> struct Holder {};\n\
                 template <template <class> class TT = Box, class T = int, int N = 3> struct Child {};\n\
                 template <template <class> class TT = Box, class T = int, int N = 3> void build() {}\n\
                 template <template <class> class TT = Box, class T = int, int N = 3> using Wrapped = TT<int>;\n\
                 struct Host {\n    template <template <class> class TT = Box, class T = int, int N = 3> void probe() {}\n    template <template <class> class TT = Box, class T = int, int N = 3> Host(TT<int> value) {}\n};\n\
                 template <class T> struct Gate {\n    void declared() requires Small<T>;\n    void defined() requires Small<T> {}\n    Gate() requires Small<T> {}\n    friend void pal(Gate) requires Small<T> {}\n};\n\
                 template <class T> void Gate<T>::declared() requires Small<T> {}\n\
                 template <class T> requires Small<T> struct Bound {\n    void unconstrained();\n};\n\
                 int infer(int value) { return value; }\n",
        declarations: &[
            declared(
                "Box",
                K::Class,
                &[
                    ("T", Type, &[], Some("int"), None, 0),
                    ("N", Const, &[], Some("3"), None, 1),
                    param("Ts", 2),
                ],
            ),
            declared("map", K::Method, &[param("U", 0)]),
            plain("size", K::Method),
            Declared {
                name: "fits",
                kind: K::Function,
                params: &[param("T", 0)],
                constraints: &["Small<T>"],
            },
            Declared {
                name: "trailing",
                kind: K::Function,
                params: &[param("T", 0)],
                constraints: &["Small<T>"],
            },
            declared("Alias", K::TypeAlias, &[param("T", 0)]),
            // The unnamed `typename` keeps its slot.
            declared("Holder", K::Type, &[param("TT", 0), ("Ns", Const, &[], None, None, 2)]),
            // A template-template parameter's default sits on its nested
            // `class TT = Box`; the ordinary and non-type defaults beside it
            // are the controls.
            declared("Child", K::Type, DEFAULTED_TEMPLATE_TEMPLATE),
            declared("build", K::Function, DEFAULTED_TEMPLATE_TEMPLATE),
            declared("Wrapped", K::TypeAlias, DEFAULTED_TEMPLATE_TEMPLATE),
            declared("probe", K::Method, DEFAULTED_TEMPLATE_TEMPLATE),
            declared("Host", K::Constructor, DEFAULTED_TEMPLATE_TEMPLATE),
            // A member of a class template constrains itself on the class's
            // parameter with no template of its own: the requirement is the
            // member's, the parameter list stays the class's.
            declared("Gate", K::Type, &[param("T", 0)]),
            owns_requirement_only("declared", K::Method),
            owns_requirement_only("defined", K::Method),
            owns_requirement_only("Gate", K::Constructor),
            owns_requirement_only("pal", K::Function),
            // Out of class, the definition repeats the class's header.
            Declared {
                name: "declared",
                kind: K::Function,
                params: &[param("T", 0)],
                constraints: &["Small<T>"],
            },
            // A template-level `requires` is the template's alone.
            Declared {
                name: "Bound",
                kind: K::Type,
                params: &[param("T", 0)],
                constraints: &["Small<T>"],
            },
            plain("unconstrained", K::Method),
            plain("infer", K::Function),
        ],
        limits: &[Limit {
            grammar: "tree-sitter-cpp 0.23.4",
            extension: "cpp",
            snippet: "template <std::integral T> void constrained(T value) {}\n",
            observation: "a concept-constrained type parameter parses as a non-type `parameter_declaration` of type `std::integral`, the shape of `template <int N>`, so it reads as `Const` without the concept",
            read: LimitRead::Reads(Declared {
                name: "constrained",
                kind: K::Function,
                params: &[("T", Const, &[], None, None, 0)],
                constraints: &[],
            }),
        }],
    },
    GenericsRow {
        language: L::Swift,
        support: Support::Supported,
        extension: "swift",
        source: "class Box<T: Equatable, U> where U: Hashable {\n    func map<V>(value: V) -> V where V: Codable, U == Int {\n        return value\n    }\n    func size() -> Int {\n        return 0\n    }\n}\n\
                 struct Pair<A, B: Comparable & Hashable> {}\n\
                 func first<T>(items: [T]) -> T {\n    return items[0]\n}\n\
                 typealias List<T> = [T]\n\
                 func zip<each T: Equatable, U>(values: repeat each T, other: U) {}\n\
                 struct Packed<each T> {}\n\
                 func older<T: Sequence where T: Hashable, T.Element == Int>(value: T) {}\n\
                 func infer(value: Int) -> Int {\n    return value\n}\n",
        declarations: &[
            declared("Box", K::Type, &[bounded("T", &["Equatable"], 0), bounded("U", &["Hashable"], 1)]),
            // `U` is the class's: its same-type requirement is the method's
            // own, not a parameter bound.
            Declared {
                name: "map",
                kind: K::Method,
                params: &[bounded("V", &["Codable"], 0)],
                constraints: &["U == Int"],
            },
            plain("size", K::Method),
            declared("Pair", K::Type, &[param("A", 0), bounded("B", &["Comparable & Hashable"], 1)]),
            declared("first", K::Function, &[param("T", 0)]),
            declared("List", K::TypeAlias, &[param("T", 0)]),
            // A pack's name is nested in `type_parameter_pack`.
            declared("zip", K::Function, &[bounded("T", &["Equatable"], 0), param("U", 1)]),
            declared("Packed", K::Type, &[param("T", 0)]),
            // A `where` clause inside the angle brackets is nested in the
            // parameter list.
            Declared {
                name: "older",
                kind: K::Function,
                params: &[bounded("T", &["Sequence", "Hashable"], 0)],
                constraints: &["T.Element == Int"],
            },
            plain("infer", K::Function),
        ],
        limits: &[],
    },
    GenericsRow {
        language: L::Scala,
        support: Support::Supported,
        extension: "scala",
        source: "class Box[+A, -B, T >: Null <: AnyRef : Ordering, F[_ <: AnyRef]] {\n  def map[U: Ordering](value: U): U = value\n  def size: Int = 0\n}\n\
                 trait Keyed[K]\n\
                 def first[T](items: List[T]): T = items.head\n\
                 type Alias[T] = List[T]\n\
                 def infer(value: Int): Int = value\n",
        declarations: &[
            declared(
                "Box",
                K::Class,
                &[
                    ("A", Type, &[], None, Some("+"), 0),
                    ("B", Type, &[], None, Some("-"), 1),
                    bounded("T", &[">: Null", "<: AnyRef", ": Ordering"], 2),
                    param("F", 3),
                ],
            ),
            declared("map", K::Method, &[bounded("U", &[": Ordering"], 0)]),
            plain("size", K::Method),
            declared("Keyed", K::Trait, &[param("K", 0)]),
            declared("first", K::Function, &[param("T", 0)]),
            declared("Alias", K::TypeAlias, &[param("T", 0)]),
            plain("infer", K::Function),
        ],
        limits: &[],
    },
    GenericsRow {
        language: L::Php,
        support: Support::Absent("PHP has no native generic parameters; a PHPDoc `@template` is not read"),
        extension: "php",
        source: "<?php\n/** @template T */\nclass Box {\n    public function map($value) { return $value; }\n}\n",
        declarations: &[plain("Box", K::Class), plain("map", K::Method)],
        limits: &[],
    },
    GenericsRow {
        language: L::Perl,
        support: Support::Absent("Perl declares no generic parameters"),
        extension: "pl",
        source: "package Box;\nsub map_value { my ($value) = @_; return $value; }\n1;\n",
        declarations: &[plain("map_value", K::Function)],
        limits: &[],
    },
    GenericsRow {
        language: L::Bash,
        support: Support::Absent("Bash declares no generic parameters"),
        extension: "sh",
        source: "run() {\n  echo \"$1\"\n}\n",
        declarations: &[plain("run", K::Function)],
        limits: &[],
    },
    GenericsRow {
        language: L::Zsh,
        support: Support::Absent("Zsh declares no generic parameters"),
        extension: "zsh",
        source: "run() {\n  echo \"$1\"\n}\n",
        declarations: &[plain("run", K::Function)],
        limits: &[],
    },
    GenericsRow {
        language: L::Lua,
        support: Support::Absent("Lua declares no generic parameters"),
        extension: "lua",
        source: "local function run(value)\n  return value\nend\n",
        declarations: &[plain("run", K::Function)],
        limits: &[],
    },
];

/// The languages with a generics row, for the coverage guard.
pub fn generics_matrix_languages() -> Vec<ProgrammingLanguage> {
    GENERICS_MATRIX.iter().map(|row| row.language).collect()
}

fn write_temp(source: &str, extension: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .expect("create temp source");
    file.write_all(source.as_bytes()).expect("write temp source");
    file
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
    let actual: Vec<OwnedParam> = record
        .type_info
        .generic_params
        .iter()
        .map(|param| {
            (
                param.name.clone(),
                param.kind,
                param.constraints.clone(),
                param.default.clone(),
                param.variance.clone(),
                param.position,
            )
        })
        .collect();
    let wanted: Vec<OwnedParam> = expected
        .params
        .iter()
        .map(|(name, kind, constraints, default, variance, position)| {
            (
                name.to_string(),
                *kind,
                constraints.iter().map(|constraint| constraint.to_string()).collect(),
                default.map(str::to_string),
                variance.map(str::to_string),
                *position,
            )
        })
        .collect();
    assert_eq!(actual, wanted, "{label}: `{}` generic parameters", expected.name);
    assert_eq!(
        record.type_info.constraints, expected.constraints,
        "{label}: `{}` declaration constraints",
        expected.name
    );
}

crate::language_cases::language_tests!(generics_matrix_holds_on_complete_and_declarations_only_paths => check_generics_matrix_holds_on_complete_and_declarations_only_paths);

/// Each row's declarations report exactly their own parameters and
/// requirements, on the complete index and on a declarations-only query of
/// the declaration's kind; an absent-construct language reports none on any
/// record.
fn check_generics_matrix_holds_on_complete_and_declarations_only_paths(language: ProgrammingLanguage) {
    for row in GENERICS_MATRIX.iter().filter(|row| row.language == language) {
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
            assert!(selective.completed_passes.iter().all(|pass| pass.as_str() == "parse"), "{label}: declarations only");
            assert_declared(&selective, &format!("{label} declarations-only"), expected);
        }
        if let Support::Absent(reason) = row.support {
            assert!(
                complete
                    .symbols
                    .iter()
                    .all(|record| record.type_info.generic_params.is_empty() && record.type_info.constraints.is_empty()),
                "{label}: {reason}"
            );
        }
    }
}

crate::language_cases::language_tests!(generics_matrix_holds_on_warm_cache_hits_and_projections => check_generics_matrix_holds_on_warm_cache_hits_and_projections);

/// The same expectations hold through the persistent cache: a complete entry
/// is served warm as a real `Hit`, and projected to each declarations-only
/// query, with every parameter's default, bounds, variance and position
/// intact (an equal-to-uncached comparison alone would pass if both lost one).
fn check_generics_matrix_holds_on_warm_cache_hits_and_projections(language: ProgrammingLanguage) {
    let sources = tempfile::TempDir::new().expect("sources");
    for (row_index, row) in GENERICS_MATRIX.iter().enumerate().filter(|(_, row)| row.language == language) {
        let label = row.language.name();
        let path = sources.path().join(format!("row_{row_index}.{}", row.extension));
        std::fs::write(&path, row.source).expect("write row source");
        let dir = tempfile::TempDir::new().expect("cache dir");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let read = || SourceFile::read(&path).expect("read row source");
        let query = |query: &SymbolQuery| {
            let outcome = cache.query(read(), query).expect("cached query");
            assert!(outcome.write_error.is_none(), "{label}: write failed: {:?}", outcome.write_error);
            outcome
        };
        let complete = SymbolQuery::complete_index();
        assert!(!matches!(query(&complete).lookup, CacheLookup::Hit), "{label}: cold");
        let warm = query(&complete);
        assert_eq!(warm.lookup, CacheLookup::Hit, "{label}: warm");
        for expected in row.declarations {
            assert_declared(&warm.result.index, &format!("{label} warm"), expected);
            let narrowed = query(&SymbolQuery::new().kinds([expected.kind]));
            assert_eq!(narrowed.lookup, CacheLookup::Hit, "{label}: `{}` projection", expected.name);
            assert_declared(&narrowed.result.index, &format!("{label} projected"), expected);
        }
    }
}

crate::language_cases::language_tests!(generics_grammar_limits_hold_for_the_pinned_grammars => check_generics_grammar_limits_hold_for_the_pinned_grammars);

/// Each demonstrated grammar limit still holds for the pinned grammar: the
/// form is rejected, or parses into the recorded reading.
fn check_generics_grammar_limits_hold_for_the_pinned_grammars(language: ProgrammingLanguage) {
    for row in GENERICS_MATRIX.iter().filter(|row| row.language == language) {
        for limit in row.limits {
            let label = format!("{} ({})", row.language.name(), limit.grammar);
            let file = write_temp(limit.snippet, limit.extension);
            let tree_file = TreeFile::new(file.path()).expect("parse limit snippet");
            match &limit.read {
                LimitRead::SyntaxError => assert!(
                    !tree_file.syntax_diagnostics().is_empty(),
                    "{label}: expected a syntax error: {}",
                    limit.observation
                ),
                LimitRead::Reads(expected) => {
                    assert!(tree_file.syntax_diagnostics().is_empty(), "{label}: {}", limit.observation);
                    assert_declared(&tree_file.symbol_index_v2().expect("index"), &label, expected);
                }
            }
        }
    }
}

/// `"fn wide<T0, T1, …>() {}"` with `count` parameters.
fn wide_function(count: usize) -> String {
    let params: Vec<String> = (0..count).map(|index| format!("T{index}")).collect();
    format!("fn wide<{}>() {{}}\n", params.join(", "))
}

/// A list whose last position is `u16::MAX` is read in full. The boundary
/// needs declarations only, avoiding unrelated passes over 65,536 names.
#[test]
fn a_generic_list_at_the_u16_boundary_is_read_in_full() {
    let fits = usize::from(u16::MAX) + 1;
    let file = write_temp(&wide_function(fits), "rs");
    let query = SymbolQuery::new().kinds([K::Function]);
    let index = TreeFile::new(file.path()).expect("parse").query(&query).expect("a full u16 list is representable").index;
    let wide = record_of(&index, "fits", "wide", K::Function);
    assert_eq!(wide.type_info.generic_params.len(), fits);
    let last = wide.type_info.generic_params.last().expect("last parameter");
    assert_eq!((last.name.as_str(), last.position), ("T65535", u16::MAX));

}

/// One parameter past the representable range names the oversized declaration
/// in an analysis error; a query that does not select it still succeeds.
#[test]
fn a_generic_list_beyond_u16_positions_is_an_analysis_error() {
    let fits = usize::from(u16::MAX) + 1;
    let query = SymbolQuery::new().kinds([K::Function]);
    let file = write_temp(&format!("{}struct Other;\n", wide_function(fits + 1)), "rs");
    let tree_file = TreeFile::new(file.path()).expect("parse");
    match tree_file.query(&query) {
        Err(TreeHuggerError::TooManyGenericParameters { symbol, count, .. }) => {
            assert_eq!((symbol.as_str(), count), ("wide", fits + 1));
        }
        other => panic!("expected TooManyGenericParameters, got {other:?}"),
    }
    let structs = tree_file
        .query(&SymbolQuery::new().kinds([K::Type]))
        .expect("the oversized function is not selected");
    assert_eq!(structs.index.symbols.len(), 1);
}
