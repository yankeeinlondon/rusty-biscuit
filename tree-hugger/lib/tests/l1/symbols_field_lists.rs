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

use crate::symbols_fixtures::{Extracted, all_fixtures, display_name, extract_inline, extract_primary};

const PUBLIC: Option<Visibility> = Some(Visibility::Public);
const PRIVATE: Option<Visibility> = Some(Visibility::Private);
const PROTECTED: Option<Visibility> = Some(Visibility::Protected);
const INTERNAL: Option<Visibility> = Some(Visibility::Internal);
const PACKAGE: Option<Visibility> = Some(Visibility::Package);

/// Whether a source-declared member also has a standalone record. Record
/// components and parameters (Java/C# records, Scala class parameters), Rust
/// tuple-struct positions and Go struct fields are listed in metadata only.
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
            record.identity.module_path.as_deref() == Some(type_name)
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

    // Go struct fields and Scala class parameters are listed in metadata only.
    let go = extract_inline("package p\ntype Pair struct { A, b int }\n", "go");
    assert_field_lists(&go, &[("Pair", &[("A", false, PUBLIC, No), ("b", false, PACKAGE, No)])]);
    let scala = extract_inline("class Holder(val a: Int) { var b = 2 }\n", "scala");
    assert_field_lists(&scala, &[("Holder", &[("a", false, None, No)])]);
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

/// The class-level guard over every fixture: a standalone Field, Property or
/// Constant declared in the body of a type in the same file is an entry of
/// that type's metadata field list with the same flags, on the records path
/// `hug classes` reads. Enum owners list variants, not fields, and are
/// skipped; Rust `impl` members are declared outside the type's body.
#[test]
fn every_fixture_member_is_in_its_type_field_list() {
    let mut checked = 0usize;
    for path in all_fixtures() {
        let name = display_name(&path);
        let records = TreeFile::new(&path)
            .expect("load fixture")
            .symbol_index_v2()
            .expect("index")
            .symbols;
        for member in records.iter().filter(|record| is_member_kind(SymbolKind::from(record.kind))) {
            let Some(container) = member.identity.module_path.as_deref() else {
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
    assert!(checked > 40, "only {checked} fixture members checked");
}
