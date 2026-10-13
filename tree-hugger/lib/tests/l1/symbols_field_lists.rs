//! Review-2 finding 3 of `2026-10-03-visibility-and-kind`: every member a
//! type declares in its source is a standalone Field, Property or Constant
//! record **and** an entry of that type's metadata field list, with the same
//! access and member `static`, in source order. The rows below list each
//! type's complete expected field list, so a member missing from either path
//! fails, not only a disagreement between entries that both paths report.
//!
//! The metadata list is checked on `TreeFile::symbols()`, after the v1 -> v2
//! -> v1 adapter round trip, and on the records of `symbol_index_v2()`
//! converted back to `SymbolInfo` (the path `hug classes` reads).

use tree_hugger::{SymbolInfo, SymbolKind, SymbolRecord, TreeFile, TypeMetadata, Visibility};

use tree_hugger::ProgrammingLanguage;

use crate::symbols_fixtures::{Extracted, display_name, extract_inline, extract_primary};

const PUBLIC: Option<Visibility> = Some(Visibility::Public);
const PRIVATE: Option<Visibility> = Some(Visibility::Private);
const PROTECTED: Option<Visibility> = Some(Visibility::Protected);
const INTERNAL: Option<Visibility> = Some(Visibility::Internal);
const PACKAGE: Option<Visibility> = Some(Visibility::Package);

/// Whether a source-declared member also has a standalone record. Record
/// components and parameters (Java/C# records, Scala class parameters) and
/// Rust tuple-struct positions are listed in metadata only.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Standalone {
    Yes,
    No,
}
use Standalone::{No, Yes};

/// `(member name, is_static, visibility, standalone)`.
pub type Member = (&'static str, bool, Option<Visibility>, Standalone);

/// One type and its complete field list in source order.
pub type TypeRow = (&'static str, &'static [Member]);

fn is_member_kind(kind: SymbolKind) -> bool {
    matches!(kind, SymbolKind::Field | SymbolKind::Property | SymbolKind::Constant)
}

fn field_list(label: &str, metadata: Option<&TypeMetadata>) -> Vec<(String, bool, Option<Visibility>)> {
    metadata
        .unwrap_or_else(|| panic!("{label}: no type metadata"))
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.is_static, field.visibility))
        .collect()
}

fn type_symbol<'a>(extracted: &'a Extracted, type_name: &str) -> &'a SymbolInfo {
    let matches: Vec<&SymbolInfo> = extracted
        .symbols
        .iter()
        .filter(|symbol| symbol.name == type_name && (symbol.kind.is_type() || symbol.kind == SymbolKind::Class))
        .collect();
    assert_eq!(matches.len(), 1, "{}: expected one type `{type_name}`", extracted.label);
    matches[0]
}

pub fn assert_field_lists(extracted: &Extracted, rows: &[TypeRow]) {
    let label = &extracted.label;
    for &(type_name, members) in rows {
        let expected: Vec<(String, bool, Option<Visibility>)> = members
            .iter()
            .map(|&(name, is_static, visibility, _)| (name.to_string(), is_static, visibility))
            .collect();

        // Metadata, three ways.
        let symbol = type_symbol(extracted, type_name);
        assert_eq!(
            field_list(&format!("{label}: `{type_name}` symbols()"), symbol.type_metadata.as_ref()),
            expected,
            "{label}: `{type_name}` metadata field list in symbols()"
        );
        let round_trip = SymbolInfo::try_from(SymbolRecord::from(symbol.clone())).expect("round trip");
        assert_eq!(
            field_list(&format!("{label}: `{type_name}` round trip"), round_trip.type_metadata.as_ref()),
            expected,
            "{label}: `{type_name}` metadata field list after the adapter round trip"
        );
        let from_record = SymbolInfo::try_from(extracted.record(type_name, symbol.container_name.as_deref()).clone())
            .expect("record");
        assert_eq!(
            field_list(&format!("{label}: `{type_name}` symbol_index_v2()"), from_record.type_metadata.as_ref()),
            expected,
            "{label}: `{type_name}` metadata field list from symbol_index_v2()"
        );

        // Standalone records: exactly the members marked `Yes`, same flags.
        let standalone: Vec<(String, bool, Option<Visibility>)> = extracted
            .symbols
            .iter()
            .filter(|member| member.container_name.as_deref() == Some(type_name) && is_member_kind(member.kind))
            .map(|member| (member.name.clone(), member.is_static, member.visibility))
            .collect();
        let expected_standalone: Vec<(String, bool, Option<Visibility>)> = members
            .iter()
            .filter(|member| member.3 == Yes)
            .map(|&(name, is_static, visibility, _)| (name.to_string(), is_static, visibility))
            .collect();
        assert_eq!(
            standalone, expected_standalone,
            "{label}: `{type_name}` standalone Field/Property/Constant members"
        );
        for record in extracted.records.iter().filter(|record| {
            record.identity.container_name.as_deref() == Some(type_name)
                && is_member_kind(SymbolKind::from(record.kind))
        }) {
            assert!(
                expected_standalone.iter().any(|(name, ..)| *name == record.identity.name),
                "{label}: unexpected record `{}` in `{type_name}`",
                record.identity.name
            );
        }
    }
}

#[test]
fn javascript_class_body_properties_are_listed() {
    let js = extract_inline(
        "class Fields {\n  \
             plain = 1;\n  \
             #secret = 2;\n  \
             static count = 0;\n  \
             bare;\n  \
             static #hidden = 3;\n  \
             \"quoted\" = 4;\n  \
             m() {}\n\
         }\n",
        "js",
    );
    assert_field_lists(
        &js,
        &[(
            "Fields",
            &[
                ("plain", false, PUBLIC, Yes),
                ("#secret", false, PRIVATE, Yes),
                ("count", true, PUBLIC, Yes),
                ("bare", false, PUBLIC, Yes),
                ("#hidden", true, PRIVATE, Yes),
                ("quoted", false, PUBLIC, Yes),
            ],
        )],
    );
}

#[test]
fn typescript_abstract_private_and_quoted_members_are_listed() {
    let ts = extract_inline(
        "abstract class Abs { static count = 0; name: string = \"\"; }\n\
         class PrivateFields { #inst = 1; static #count = 0; }\n\
         class Quoted { \"label\": string = \"\"; 'other' = 1; plain = 2; }\n\
         interface Named { \"label\": string; plain: number; }\n\
         class Plain { a = 1; static b = 2; private c = 3; protected d: number = 4; }\n\
         interface Ctrl { x: number; }\n",
        "ts",
    );
    assert_field_lists(
        &ts,
        &[
            ("Abs", &[("count", true, PUBLIC, Yes), ("name", false, PUBLIC, Yes)]),
            ("PrivateFields", &[("#inst", false, PRIVATE, Yes), ("#count", true, PRIVATE, Yes)]),
            (
                "Quoted",
                &[("label", false, PUBLIC, Yes), ("other", false, PUBLIC, Yes), ("plain", false, PUBLIC, Yes)],
            ),
            ("Named", &[("label", false, PUBLIC, Yes), ("plain", false, PUBLIC, Yes)]),
            // Controls.
            (
                "Plain",
                &[
                    ("a", false, PUBLIC, Yes),
                    ("b", true, PUBLIC, Yes),
                    ("c", false, PRIVATE, Yes),
                    ("d", false, PROTECTED, Yes),
                ],
            ),
            ("Ctrl", &[("x", false, PUBLIC, Yes)]),
        ],
    );
}

#[test]
fn rust_union_fields_are_listed() {
    let rust = extract_inline(
        "union Fields { number: i32, pub bytes: [u8; 4] }\n\
         struct Plain { a: i32, b: [u8; 2] }\n\
         struct Tuple(i32, u8);\n",
        "rs",
    );
    assert_field_lists(
        &rust,
        &[
            ("Fields", &[("number", false, PRIVATE, Yes), ("bytes", false, PUBLIC, Yes)]),
            // Controls: struct fields and tuple-struct positions.
            ("Plain", &[("a", false, PRIVATE, Yes), ("b", false, PRIVATE, Yes)]),
            ("Tuple", &[("0", false, None, No), ("1", false, None, No)]),
        ],
    );
}

#[test]
fn c_union_anonymous_and_declarator_fields_are_listed() {
    let c = extract_inline(
        "union Number { int i; float f; };\n\
         struct Outer {\n    \
             int direct;\n    \
             union { int ai; float af; };\n    \
             struct { int si; };\n    \
             int values[3];\n    \
             int *ptrs[2];\n    \
             int (*callback)(int);\n    \
             int **pointer;\n    \
             int *single;\n    \
             unsigned bits : 3;\n    \
             int grid[2][3];\n    \
             struct { int hidden; } named;\n\
         };\n",
        "c",
    );
    assert_field_lists(
        &c,
        &[
            ("Number", &[("i", false, None, Yes), ("f", false, None, Yes)]),
            (
                "Outer",
                &[
                    ("direct", false, None, Yes),
                    ("ai", false, None, Yes),
                    ("af", false, None, Yes),
                    ("si", false, None, Yes),
                    ("values", false, None, Yes),
                    ("ptrs", false, None, Yes),
                    ("callback", false, None, Yes),
                    ("pointer", false, None, Yes),
                    ("single", false, None, Yes),
                    ("bits", false, None, Yes),
                    ("grid", false, None, Yes),
                    ("named", false, None, Yes),
                ],
            ),
        ],
    );
}

#[test]
fn cpp_union_anonymous_declarator_and_template_fields_are_listed() {
    let cpp = extract_inline(
        "union Number { int i; float f; };\n\
         struct Outer {\n    \
             int direct;\n    \
             union { int ai; float af; };\n    \
             int values[3];\n    \
             int *ptrs[2];\n    \
             int (*callback)(int);\n    \
             int **pointer;\n    \
             int &ref;\n    \
             static int table[4];\n    \
             template<class E> static E cache[3];\n    \
             template<class E> static E scalar;\n    \
             unsigned bits : 3;\n    \
             int *method();\n    \
             void plainMethod();\n\
         };\n\
         class Access { int hidden; union { int secretU; }; public: union { int openU; }; static const int limit[2]; };\n",
        "cpp",
    );
    assert_field_lists(
        &cpp,
        &[
            ("Number", &[("i", false, PUBLIC, Yes), ("f", false, PUBLIC, Yes)]),
            (
                "Outer",
                &[
                    ("direct", false, PUBLIC, Yes),
                    ("ai", false, PUBLIC, Yes),
                    ("af", false, PUBLIC, Yes),
                    ("values", false, PUBLIC, Yes),
                    ("ptrs", false, PUBLIC, Yes),
                    ("callback", false, PUBLIC, Yes),
                    ("pointer", false, PUBLIC, Yes),
                    ("ref", false, PUBLIC, Yes),
                    ("table", true, PUBLIC, Yes),
                    ("cache", true, PUBLIC, Yes),
                    ("scalar", true, PUBLIC, Yes),
                    ("bits", false, PUBLIC, Yes),
                ],
            ),
            (
                "Access",
                &[
                    ("hidden", false, PRIVATE, Yes),
                    ("secretU", false, PRIVATE, Yes),
                    ("openU", false, PUBLIC, Yes),
                    ("limit", true, PUBLIC, Yes),
                ],
            ),
        ],
    );
}

#[test]
fn csharp_properties_and_record_body_members_are_listed() {
    let csharp = extract_inline(
        "class Fields { public static int Count { get; private set; } public int Total { get; set; } int f; static int sf; public event System.Action Changed; }\n\
         struct StructFields { public int P { get; set; } public static int Q { get; } }\n\
         record RecordFields(int A) { public int value; static int s; public int Prop { get; set; } }\n\
         interface IProps { int X { get; } static int Y { get; } }\n",
        "cs",
    );
    assert_field_lists(
        &csharp,
        &[
            (
                "Fields",
                &[
                    ("Count", true, PUBLIC, Yes),
                    ("Total", false, PUBLIC, Yes),
                    ("f", false, PRIVATE, Yes),
                    ("sf", true, PRIVATE, Yes),
                    ("Changed", false, PUBLIC, Yes),
                ],
            ),
            ("StructFields", &[("P", false, PUBLIC, Yes), ("Q", true, PUBLIC, Yes)]),
            (
                "RecordFields",
                &[
                    ("A", false, PUBLIC, No),
                    ("value", false, PUBLIC, Yes),
                    ("s", true, PRIVATE, Yes),
                    ("Prop", false, PUBLIC, Yes),
                ],
            ),
            // Control.
            ("IProps", &[("X", false, PUBLIC, Yes), ("Y", true, PUBLIC, Yes)]),
        ],
    );
    // A setter-only `private` does not change the property's level; the
    // restriction stays in the declaration text.
    let count = csharp.record("Count", Some("Fields"));
    assert!(
        count
            .source
            .declaration_text
            .as_deref()
            .is_some_and(|text| text.contains("private set")),
        "{:?}",
        count.source.declaration_text
    );
}

#[test]
fn java_record_body_fields_are_listed() {
    let java = extract_inline(
        "record Point(int a) { static int count = 0; }\n\
         class Multi { int x, y; static final int Z = 1, W = 2; }\n\
         interface Constants { int A = 1, B = 2; }\n",
        "java",
    );
    assert_field_lists(
        &java,
        &[
            ("Point", &[("a", false, PUBLIC, No), ("count", true, PACKAGE, Yes)]),
            // Controls.
            (
                "Multi",
                &[
                    ("x", false, PACKAGE, Yes),
                    ("y", false, PACKAGE, Yes),
                    ("Z", true, PACKAGE, Yes),
                    ("W", true, PACKAGE, Yes),
                ],
            ),
            ("Constants", &[("A", true, PUBLIC, Yes), ("B", true, PUBLIC, Yes)]),
        ],
    );
}

#[test]
fn python_chained_and_pattern_assignments_are_listed() {
    let python = extract_inline(
        "class Fields:\n    \
             first = second = 0\n    \
             a, b = (1, 2)\n    \
             (c, d) = 1, 2\n    \
             [e, f] = 1, 2\n    \
             x: int\n    \
             y: int = 0\n    \
             z: ClassVar[int] = 0\n    \
             w: typing.ClassVar[int] = 0\n    \
             decoy: NotClassVar[int] = 0\n\
         \n\
         one = two = 0\n\
         def run():\n    \
             p = q = 1\n",
        "py",
    );
    assert_field_lists(
        &python,
        &[(
            "Fields",
            &[
                ("first", true, None, Yes),
                ("second", true, None, Yes),
                ("a", true, None, Yes),
                ("b", true, None, Yes),
                ("c", true, None, Yes),
                ("d", true, None, Yes),
                ("e", true, None, Yes),
                ("f", true, None, Yes),
                ("x", false, None, Yes),
                ("y", false, None, Yes),
                ("z", true, None, Yes),
                ("w", true, None, Yes),
                ("decoy", false, None, Yes),
            ],
        )],
    );
    // Chained targets outside a class stay variables.
    for name in ["one", "two", "p", "q"] {
        let found: Vec<&SymbolInfo> = python.symbols.iter().filter(|symbol| symbol.name == name).collect();
        assert_eq!(found.len(), 1, "{name}");
        assert_eq!(found[0].kind, SymbolKind::Variable, "{name}");
    }
}

#[test]
fn swift_multi_binding_properties_are_listed() {
    let swift = extract_inline(
        "struct Fields {\n    \
             var first = 1, second = 2\n    \
             static var sa = 1, sb: Int = 2\n    \
             var single: Int = 0\n\
         }\n\
         protocol Requirement { var req: Int { get } }\n",
        "swift",
    );
    assert_field_lists(
        &swift,
        &[
            (
                "Fields",
                &[
                    ("first", false, INTERNAL, Yes),
                    ("second", false, INTERNAL, Yes),
                    ("sa", true, INTERNAL, Yes),
                    ("sb", true, INTERNAL, Yes),
                    ("single", false, INTERNAL, Yes),
                ],
            ),
            // Control.
            ("Requirement", &[("req", false, INTERNAL, Yes)]),
        ],
    );
}

#[test]
fn php_go_and_scala_controls_keep_their_lists() {
    let php = extract_inline(
        "<?php\nclass Props { public $a, $b; public static $s = []; private array $arr = []; }\n\
         trait Shared { protected $t; }\n",
        "php",
    );
    assert_field_lists(
        &php,
        &[
            (
                "Props",
                &[
                    ("a", false, PUBLIC, Yes),
                    ("b", false, PUBLIC, Yes),
                    ("s", true, PUBLIC, Yes),
                    ("arr", false, PRIVATE, Yes),
                ],
            ),
            ("Shared", &[("t", false, PROTECTED, Yes)]),
        ],
    );

    // Go struct fields are standalone fields too. A Scala class parameter is
    // listed in metadata only; a body `val`/`var` is a standalone field that
    // completes the list.
    let go = extract_inline("package p\ntype Pair struct { A, b int }\n", "go");
    assert_field_lists(&go, &[("Pair", &[("A", false, PUBLIC, Yes), ("b", false, PACKAGE, Yes)])]);
    let scala = extract_inline("class Holder(val a: Int) { var b = 2 }\n", "scala");
    assert_field_lists(&scala, &[("Holder", &[("a", false, None, No), ("b", false, PUBLIC, Yes)])]);
}

/// Each type of a grouped Go `type ( … )` lists only its own fields: the
/// types of one block no longer share one declaration (`B` used to list
/// `A`'s fields).
#[test]
fn grouped_go_types_list_only_their_own_fields() {
    let go = extract_inline(
        "package p\n\ntype (\n\tA struct{ X, Y int }\n\tB struct{ z string }\n\tC int\n)\n",
        "go",
    );
    assert_field_lists(
        &go,
        &[
            ("A", &[("X", false, PUBLIC, Yes), ("Y", false, PUBLIC, Yes)]),
            ("B", &[("z", false, PACKAGE, Yes)]),
        ],
    );
    let defined = type_symbol(&go, "C");
    assert!(
        defined.type_metadata.as_ref().is_none_or(|metadata| metadata.fields.is_empty()),
        "`C int` lists no fields: {:?}",
        defined.type_metadata
    );
}

/// A Go struct's named fields are standalone fields paired with its field
/// list, once each and with their written types; embedded fields stay out
/// of both, and an anonymous struct type's fields are not the struct's. An
/// interface's method elements are never Field records, nor (since the
/// grammar's `method_elem` is not read) field-list entries.
#[test]
fn go_struct_fields_pair_with_the_field_list_and_keep_their_types() {
    let source = "package p\n\ntype S struct {\n\tX, Y int\n\tz    string `json:\"z\"`\n\tÜber bool\n\tanon struct{ Inner int }\n\tEmbedded\n\t*Ptr\n}\n\ntype G[T any] struct {\n\tVal T\n}\n\ntype I interface{ M() }\n\ntype Embedded struct{}\ntype Ptr struct{}\n";
    let go = extract_inline(source, "go");
    assert_field_lists(
        &go,
        &[
            (
                "S",
                &[
                    ("X", false, PUBLIC, Yes),
                    ("Y", false, PUBLIC, Yes),
                    ("z", false, PACKAGE, Yes),
                    ("Über", false, PUBLIC, Yes),
                    ("anon", false, PACKAGE, Yes),
                ],
            ),
            ("G", &[("Val", false, PUBLIC, Yes)]),
        ],
    );

    let types: Vec<(String, Option<String>)> = type_symbol(&go, "S")
        .type_metadata
        .as_ref()
        .expect("S metadata")
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.type_annotation.clone()))
        .collect();
    let expected = [("X", "int"), ("Y", "int"), ("z", "string"), ("Über", "bool"), ("anon", "struct{ Inner int }")]
        .map(|(name, annotation)| (name.to_string(), Some(annotation.to_string())));
    assert_eq!(types, expected, "S field types");

    for absent in ["Inner", "M"] {
        assert!(
            go.symbols.iter().all(|symbol| symbol.name != absent || symbol.kind != SymbolKind::Field),
            "`{absent}` is not a field of a named type"
        );
    }
    assert!(
        type_symbol(&go, "I").type_metadata.as_ref().is_none_or(|metadata| metadata.fields.is_empty()),
        "{:?}",
        type_symbol(&go, "I").type_metadata
    );
}

/// A Scala type body's `val`/`var` definitions and declarations are
/// standalone fields paired with the owner's field list, once each, with
/// their written types: static in an object, instance members of a class
/// or trait. Class parameters stay metadata only, and method locals stay
/// out.
#[test]
fn scala_body_fields_pair_with_the_field_list() {
    let scala = extract_inline(
        "trait T {\n  /** C doc. */\n  val c: Int\n  var d: String\n}\n\nclass K(p: Int) {\n  val e = 1\n  protected var f: Long = 2\n  def g(): Int = { val local = 1; local }\n}\n",
        "scala",
    );
    assert_field_lists(
        &scala,
        &[
            ("T", &[("c", false, PUBLIC, Yes), ("d", false, PUBLIC, Yes)]),
            ("K", &[("p", false, None, No), ("e", false, PUBLIC, Yes), ("f", false, PROTECTED, Yes)]),
        ],
    );
    let types: Vec<(String, Option<String>, Option<String>)> = type_symbol(&scala, "T")
        .type_metadata
        .as_ref()
        .expect("T metadata")
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.type_annotation.clone(), field.doc_comment.clone()))
        .collect();
    assert_eq!(
        types,
        [
            ("c".to_string(), Some("Int".to_string()), Some("C doc.".to_string())),
            ("d".to_string(), Some("String".to_string()), None),
        ],
        "T field types and docs"
    );
}

/// Grouped Scala names (`val a, b: Int = 1`, `val e, f: Long`) are one
/// standalone Field and one list entry each, every entry keeping the
/// declaration's shared written type, for `val` and `var`, defined or only
/// declared, in a class and a trait. A tuple pattern lists nothing, and a
/// grouped method local stays out.
#[test]
fn scala_grouped_body_fields_are_listed_with_their_shared_type() {
    let scala = extract_inline(
        "class K(p: Int) {\n  val (t1, t2) = (1, 2)\n  val a, b: Int = 1\n  val u, w = 2\n  protected var c, d: String = \"\"\n  def m(): Int = {\n    val x, y = 1\n    x + y\n  }\n  val e, f: Long\n}\n\n\
         trait T {\n  val a, b: Int = 1\n  var c, d: Int = 2\n  val e, f: Int\n  var g, h: Int\n}\n",
        "scala",
    );
    assert_field_lists(
        &scala,
        &[
            (
                "K",
                &[
                    ("p", false, None, No),
                    ("a", false, PUBLIC, Yes),
                    ("b", false, PUBLIC, Yes),
                    ("u", false, PUBLIC, Yes),
                    ("w", false, PUBLIC, Yes),
                    ("c", false, PROTECTED, Yes),
                    ("d", false, PROTECTED, Yes),
                    ("e", false, PUBLIC, Yes),
                    ("f", false, PUBLIC, Yes),
                ],
            ),
            (
                "T",
                &[
                    ("a", false, PUBLIC, Yes),
                    ("b", false, PUBLIC, Yes),
                    ("c", false, PUBLIC, Yes),
                    ("d", false, PUBLIC, Yes),
                    ("e", false, PUBLIC, Yes),
                    ("f", false, PUBLIC, Yes),
                    ("g", false, PUBLIC, Yes),
                    ("h", false, PUBLIC, Yes),
                ],
            ),
        ],
    );

    let field_types = |type_name: &str| -> Vec<(String, Option<String>)> {
        type_symbol(&scala, type_name)
            .type_metadata
            .as_ref()
            .unwrap_or_else(|| panic!("{type_name} metadata"))
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.type_annotation.clone()))
            .collect()
    };
    let typed = |pairs: &[(&str, Option<&str>)]| -> Vec<(String, Option<String>)> {
        pairs
            .iter()
            .map(|&(name, annotation)| (name.to_string(), annotation.map(str::to_string)))
            .collect()
    };
    assert_eq!(
        field_types("K"),
        typed(&[
            ("p", Some("Int")),
            ("a", Some("Int")),
            ("b", Some("Int")),
            ("u", None),
            ("w", None),
            ("c", Some("String")),
            ("d", Some("String")),
            ("e", Some("Long")),
            ("f", Some("Long")),
        ]),
        "K field types"
    );
    assert_eq!(
        field_types("T"),
        typed(&[
            ("a", Some("Int")),
            ("b", Some("Int")),
            ("c", Some("Int")),
            ("d", Some("Int")),
            ("e", Some("Int")),
            ("f", Some("Int")),
            ("g", Some("Int")),
            ("h", Some("Int")),
        ]),
        "T field types"
    );
}

/// An enum's variant list keeps each variant's explicit value as written,
/// on `symbols()`, on the record and after the round trip back; a member
/// without one, and a Java constant's constructor arguments, have none.
/// Variants never enter a field list.
#[test]
fn enum_variant_lists_keep_discriminant_text() {
    type Variants = &'static [(&'static str, Option<&'static str>)];
    let cases: [(&str, &str, &str, Variants); 8] = [
        ("rs", "pub enum R { A, B = 2, C }\n", "R", &[("A", None), ("B", Some("2")), ("C", None)]),
        ("c", "enum E { RED, BLUE = 2 };\n", "E", &[("RED", None), ("BLUE", Some("2"))]),
        ("cpp", "enum class Scoped { S_A, S_B = 1 << 2 };\n", "Scoped", &[("S_A", None), ("S_B", Some("1 << 2"))]),
        ("cs", "enum E { Red, Blue = 2 }\n", "E", &[("Red", None), ("Blue", Some("2"))]),
        ("ts", "enum E { Red, Blue = 2, Green = \"g\" }\n", "E", &[("Red", None), ("Blue", Some("2")), ("Green", Some("\"g\""))]),
        ("php", "<?php\nenum Suit: string { case Hearts = 'h'; case Spades = 's'; }\n", "Suit", &[("Hearts", Some("'h'")), ("Spades", Some("'s'"))]),
        ("java", "enum Planet { MERCURY(1.0), VENUS(2.0); Planet(double m) {} }\n", "Planet", &[("MERCURY", None), ("VENUS", None)]),
        ("scala", "enum S {\n  case A, B\n  case V(x: Int = 1)\n}\n", "S", &[("A", None), ("B", None), ("V", None)]),
    ];
    for (extension, source, enum_name, expected) in cases {
        let extracted = extract_inline(source, extension);
        let label = format!("{extension}: `{enum_name}`");
        let expected: Vec<(String, Option<String>)> = expected
            .iter()
            .map(|(name, value)| (name.to_string(), value.map(str::to_string)))
            .collect();
        let of_metadata = |metadata: Option<&TypeMetadata>| -> Vec<(String, Option<String>)> {
            let metadata = metadata.unwrap_or_else(|| panic!("{label}: no metadata"));
            assert!(metadata.fields.is_empty(), "{label}: variants never enter the field list");
            metadata
                .variants
                .iter()
                .map(|variant| (variant.name.clone(), variant.discriminant.clone()))
                .collect()
        };

        let symbol = type_symbol(&extracted, enum_name);
        assert_eq!(symbol.kind, SymbolKind::Enum, "{label}");
        assert_eq!(of_metadata(symbol.type_metadata.as_ref()), expected, "{label}: symbols()");

        let record = extracted.record(enum_name, symbol.container_name.as_deref());
        let tree_hugger::SymbolKindData::Enum(data) = &record.kind_data else {
            panic!("{label}: enum payload, got {:?}", record.kind_data);
        };
        let from_record: Vec<(String, Option<String>)> = data
            .variants
            .iter()
            .map(|variant| (variant.name.clone(), variant.discriminant.clone()))
            .collect();
        assert_eq!(from_record, expected, "{label}: symbol_index_v2()");

        let round_trip = SymbolInfo::try_from(record.clone()).expect("round trip");
        assert_eq!(of_metadata(round_trip.type_metadata.as_ref()), expected, "{label}: round trip");
    }
}

/// One variant as `(name, discriminant, tuple payload types, named payload
/// fields with their types)`.
type VariantPayload = (String, Option<String>, Vec<String>, Vec<(String, Option<String>)>);

fn payload_of(variant: &tree_hugger::VariantInfo) -> VariantPayload {
    (
        variant.name.clone(),
        variant.discriminant.clone(),
        variant.tuple_fields.clone(),
        variant
            .struct_fields
            .iter()
            .map(|field| (field.name.clone(), field.type_annotation.clone()))
            .collect(),
    )
}

fn payload_of_record(variant: &tree_hugger::EnumVariantData) -> VariantPayload {
    (
        variant.name.clone(),
        variant.discriminant.clone(),
        variant.tuple_fields.clone(),
        variant
            .struct_fields
            .iter()
            .map(|field| (field.name.clone(), field.type_text.clone()))
            .collect(),
    )
}

fn unit(name: &str) -> VariantPayload {
    (name.to_string(), None, Vec::new(), Vec::new())
}

fn named(name: &str, fields: &[(&str, &str)]) -> VariantPayload {
    let fields = fields
        .iter()
        .map(|&(field, type_text)| (field.to_string(), Some(type_text.to_string())))
        .collect();
    (name.to_string(), None, Vec::new(), fields)
}

/// `(extension, source, enum name, expected variants in source order)`.
type PayloadCase = (&'static str, &'static str, &'static str, Vec<VariantPayload>);

pub const SCALA_BRACE_ENUM: &str = "enum Shape[T] {\n  case Empty\n  case Red, Green\n  case Point(x: Int, y: String)\n  case Sized(x: Int = 1, y: List[String])\n  case Held(item: T)\n  case Curried(x: Int)(using order: Ordering[T])\n}\n";
pub const SCALA_INDENTED_ENUM: &str = "enum Mode:\n  case Off, On\n  case Level(value: Int, render: Int => String)\n";

fn payload_cases() -> Vec<PayloadCase> {
    vec![
        (
            "scala",
            SCALA_BRACE_ENUM,
            "Shape",
            vec![
                unit("Empty"),
                unit("Red"),
                unit("Green"),
                named("Point", &[("x", "Int"), ("y", "String")]),
                named("Sized", &[("x", "Int"), ("y", "List[String]")]),
                named("Held", &[("item", "T")]),
                named("Curried", &[("x", "Int"), ("order", "Ordering[T]")]),
            ],
        ),
        (
            "scala",
            SCALA_INDENTED_ENUM,
            "Mode",
            vec![unit("Off"), unit("On"), named("Level", &[("value", "Int"), ("render", "Int => String")])],
        ),
        (
            "rs",
            "pub enum R { Unit, Pair(i32, String), Named { x: i32, y: String } }\n",
            "R",
            vec![
                unit("Unit"),
                ("Pair".to_string(), None, vec!["i32".to_string(), "String".to_string()], Vec::new()),
                named("Named", &[("x", "i32"), ("y", "String")]),
            ],
        ),
    ]
}

/// An enum's variant list keeps each declared payload with its written
/// types: a Scala full case lists its class parameters (every clause, in
/// order) as named fields, a default value is no discriminant, a generic
/// case keeps `T`, and unit and grouped cases stay unit, once each, in brace
/// and indentation syntax; Rust tuple and struct variants are the controls.
/// The payload is the same on `symbols()`, the complete record, the v1 round
/// trip and an enum-only query, while each standalone `EnumVariant` record
/// keeps an `Unknown` payload and each case parameter its own `Parameter`.
#[test]
fn enum_variant_lists_keep_declared_payload_types() {
    use tree_hugger::{SymbolKindData, SymbolKindV2, SymbolQuery};
    for (extension, source, enum_name, expected) in payload_cases() {
        let label = format!("{extension}: `{enum_name}`");
        let mut file = tempfile::Builder::new()
            .suffix(&format!(".{extension}"))
            .tempfile()
            .expect("create temp source");
        std::io::Write::write_all(&mut file, source.as_bytes()).expect("write temp source");
        let tree_file = TreeFile::new(file.path()).expect("parse inline source");
        let extracted = crate::symbols_fixtures::Extracted::from_tree_file(label.clone(), &tree_file);

        let symbol = type_symbol(&extracted, enum_name);
        assert_eq!(symbol.kind, SymbolKind::Enum, "{label}");
        let metadata = symbol.type_metadata.as_ref().unwrap_or_else(|| panic!("{label}: no metadata"));
        assert!(metadata.fields.is_empty(), "{label}: variants never enter the field list");
        assert_eq!(metadata.variants.iter().map(payload_of).collect::<Vec<_>>(), expected, "{label}: symbols()");

        let record = extracted.record(enum_name, None);
        let SymbolKindData::Enum(data) = &record.kind_data else {
            panic!("{label}: enum payload, got {:?}", record.kind_data);
        };
        assert_eq!(data.variants.iter().map(payload_of_record).collect::<Vec<_>>(), expected, "{label}: symbol_index_v2()");

        let round_trip = SymbolInfo::try_from(record.clone()).expect("round trip");
        let round_trip = round_trip.type_metadata.as_ref().unwrap_or_else(|| panic!("{label}: round-trip metadata"));
        assert_eq!(round_trip.variants.iter().map(payload_of).collect::<Vec<_>>(), expected, "{label}: round trip");

        let narrowed = tree_file.query(&SymbolQuery::new().kinds([SymbolKindV2::Enum])).expect("enum-only query");
        let narrowed: Vec<&SymbolRecord> = narrowed
            .index
            .symbols
            .iter()
            .filter(|candidate| candidate.kind == SymbolKindV2::Enum && candidate.identity.name == enum_name)
            .collect();
        assert_eq!(narrowed.len(), 1, "{label}: one enum record in the enum-only query");
        assert_eq!(
            serde_json::to_value(&narrowed[0].kind_data).expect("serialize narrowed"),
            serde_json::to_value(&record.kind_data).expect("serialize complete"),
            "{label}: an enum-only query reports the complete payload"
        );

        for (variant, _, _, fields) in &expected {
            let variant_record = extracted.record(variant, Some(enum_name));
            assert_eq!(variant_record.kind, SymbolKindV2::EnumVariant, "{label}: `{variant}`");
            assert!(
                matches!(variant_record.kind_data, SymbolKindData::Unknown),
                "{label}: `{variant}` keeps an Unknown payload, got {:?}",
                variant_record.kind_data
            );
            if extension == "scala" {
                let owner = format!("{enum_name}.{variant}.");
                let mut parameters: Vec<String> = extracted
                    .symbols
                    .iter()
                    .filter(|candidate| {
                        candidate.kind == SymbolKind::Parameter
                            && candidate.qualified_name == format!("{owner}{}", candidate.name)
                    })
                    .map(|candidate| candidate.name.clone())
                    .collect();
                parameters.sort();
                let mut declared: Vec<String> = fields.iter().map(|(name, _)| name.clone()).collect();
                declared.sort();
                assert_eq!(parameters, declared, "{label}: `{variant}` keeps its Parameter records");
            }
        }
    }
}

/// Members the language makes static without a `static` keyword are Static
/// Fields on both paths; explicit `static` and instance members are controls.
#[test]
fn implicitly_static_constants_are_static_members() {
    let csharp = extract_inline(
        "class Limits { public const int Max = 1; int count; }\n\
         struct Point { const int Origin = 0; public static int Scale; }\n\
         interface IHasLimit { const int Limit = 3; int Size { get; } }\n",
        "cs",
    );
    assert_field_lists(
        &csharp,
        &[
            ("Limits", &[("Max", true, PUBLIC, Yes), ("count", false, PRIVATE, Yes)]),
            ("Point", &[("Origin", true, PRIVATE, Yes), ("Scale", true, PUBLIC, Yes)]),
            ("IHasLimit", &[("Limit", true, PUBLIC, Yes), ("Size", false, PUBLIC, Yes)]),
        ],
    );

    let rust = extract_inline(
        "trait Shape { const SIDES: u32; fn area(&self) -> u32; }\n\
         struct Square;\n\
         impl Square { pub const EDGE: u32 = 4; }\n\
         const TOP: u32 = 1;\n",
        "rs",
    );
    assert_field_lists(&rust, &[("Shape", &[("SIDES", true, PRIVATE, Yes)])]);
    let edge = rust.symbol("EDGE", Some("Square"));
    assert!(edge.is_static, "{edge:?}");
    let top = rust.symbol("TOP", None);
    assert!(!top.is_static, "{top:?}");

    let php = extract_inline(
        "<?php\nclass Limits { const MAX = 1, MIN = 0; public static $scale = 1; public $count; }\n\
         interface HasLimit { const LIMIT = 2; }\n\
         trait Limited { protected const CAP = 3; }\n\
         const TOP = 4;\n",
        "php",
    );
    assert_field_lists(
        &php,
        &[
            (
                "Limits",
                &[
                    ("MAX", true, PUBLIC, Yes),
                    ("MIN", true, PUBLIC, Yes),
                    ("scale", true, PUBLIC, Yes),
                    ("count", false, PUBLIC, Yes),
                ],
            ),
            ("HasLimit", &[("LIMIT", true, PUBLIC, Yes)]),
            ("Limited", &[("CAP", true, PROTECTED, Yes)]),
        ],
    );
    let top = php.symbol("TOP", None);
    assert_eq!(top.kind, SymbolKind::Constant);
    assert!(!top.is_static, "{top:?}");

    // Controls: an explicit C++ `static constexpr` member, a Java interface
    // constant, and Swift, TypeScript and Python members that need a keyword
    // or a class-level assignment.
    let cpp = extract_inline("struct Limits { static constexpr int max = 1; const int floor = 0; };\n", "cpp");
    assert_field_lists(&cpp, &[("Limits", &[("max", true, PUBLIC, Yes), ("floor", false, PUBLIC, Yes)])]);
    let java = extract_inline("interface Limits { int MAX = 1; }\n", "java");
    assert_field_lists(&java, &[("Limits", &[("MAX", true, PUBLIC, Yes)])]);
    let swift = extract_inline("struct Limits { let max = 1; static let min = 0 }\n", "swift");
    assert_field_lists(&swift, &[("Limits", &[("max", false, INTERNAL, Yes), ("min", true, INTERNAL, Yes)])]);
    let ts = extract_inline("class Limits { readonly max = 1; static readonly min = 0; }\n", "ts");
    assert_field_lists(&ts, &[("Limits", &[("max", false, PUBLIC, Yes), ("min", true, PUBLIC, Yes)])]);
}

/// The review's reproductions, appended to the primary fixtures.
#[test]
fn primary_fixture_reproductions_are_listed() {
    use ProgrammingLanguage as L;
    let cases: &[(L, &[TypeRow])] = &[
        (
            L::TypeScript,
            &[
                ("AbstractFieldForms", &[("count", true, PUBLIC, Yes)]),
                ("PrivateFieldForms", &[("#count", true, PRIVATE, Yes)]),
                ("QuotedFieldForms", &[("label", false, PUBLIC, Yes)]),
            ],
        ),
        (
            L::Rust,
            &[("UnionFieldForms", &[("number", false, PRIVATE, Yes), ("bytes", false, PRIVATE, Yes)])],
        ),
        (
            L::Cpp,
            &[(
                "DeclaratorFieldForms",
                &[
                    ("values", false, PUBLIC, Yes),
                    ("pointer", false, PUBLIC, Yes),
                    ("callback", false, PUBLIC, Yes),
                    ("cache", true, PUBLIC, Yes),
                ],
            )],
        ),
        (
            L::CSharp,
            &[
                ("PropertyFieldForms", &[("Count", true, PUBLIC, Yes)]),
                ("RecordFieldForms", &[("value", false, PUBLIC, Yes)]),
            ],
        ),
        (
            L::Python,
            &[(
                "AssignmentFieldForms",
                &[
                    ("first", true, None, Yes),
                    ("second", true, None, Yes),
                    ("a", true, None, Yes),
                    ("b", true, None, Yes),
                ],
            )],
        ),
        (
            L::Swift,
            &[("BindingFieldForms", &[("first", true, INTERNAL, Yes), ("second", true, INTERNAL, Yes)])],
        ),
    ];
    for &(language, rows) in cases {
        assert_field_lists(&extract_primary(language), rows);
    }
}

crate::language_cases::language_tests!(every_fixture_member_is_in_its_type_field_list => check_every_fixture_member_is_in_its_type_field_list);

/// The class-level guard over every fixture: a standalone Field, Property or
/// Constant declared in the body of a type in the same file is an entry of
/// that type's metadata field list with the same flags, on the records path
/// `hug classes` reads. Enum owners list variants, not fields, and are
/// skipped; Rust `impl` members are declared outside the type's body.
fn check_every_fixture_member_is_in_its_type_field_list(language: ProgrammingLanguage) {
    let mut checked = 0usize;
    for path in crate::symbols_fixtures::fixtures_for(language, false) {
        let name = display_name(&path);
        let records = TreeFile::new(&path)
            .expect("load fixture")
            .symbol_index_v2()
            .expect("index")
            .symbols;
        for member in records.iter().filter(|record| is_member_kind(SymbolKind::from(record.kind))) {
            let Some(container) = member.identity.container_name.as_deref() else {
                continue;
            };
            let span = &member.source.declaration_span;
            let owners: Vec<&SymbolRecord> = records
                .iter()
                .filter(|record| {
                    let kind = SymbolKind::from(record.kind);
                    record.identity.name == container
                        && (kind.is_type() || kind == SymbolKind::Class)
                        && !matches!(kind, SymbolKind::Enum | SymbolKind::TypeAlias)
                        && record.source.declaration_span.start_byte <= span.start_byte
                        && record.source.declaration_span.end_byte >= span.end_byte
                })
                .collect();
            let [owner] = owners.as_slice() else {
                continue;
            };
            let owner = SymbolInfo::try_from((*owner).clone()).expect("owner");
            let member = SymbolInfo::try_from(member.clone()).expect("member");
            let listed = owner.type_metadata.as_ref().is_some_and(|metadata| {
                metadata.fields.iter().any(|field| {
                    field.name == member.name
                        && field.is_static == member.is_static
                        && field.visibility == member.visibility
                })
            });
            assert!(
                listed,
                "{name}: `{}` ({:?}, static {}, {:?}) is missing from `{container}`'s field list: {:?}",
                member.name,
                member.kind,
                member.is_static,
                member.visibility,
                owner.type_metadata
            );
            checked += 1;
        }
    }
    assert!(checked >= crate::language_cases::corpus_minimums(language).members, "{language}: corpus coverage shrank ({checked})");
}
