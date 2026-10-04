//! Test 4 of `2026-10-03-visibility-and-kind`: the static matrix (member
//! `static` on the shared flag, field copies and `Modifier::Static`) and
//! storage `static` (`Modifier::StaticStorage` / `is_static_storage`).

use tree_hugger::{Modifier, ProgrammingLanguage, SymbolKind, SymbolKindData, SymbolRecord};

use crate::symbols_fixtures::{Extracted, extract_primary};
use crate::symbols_kinds::rows_by_language;

use ProgrammingLanguage as L;

fn has_static(record: &SymbolRecord) -> bool {
    record.modifiers.iter().any(|modifier| matches!(modifier, Modifier::Static))
}

fn has_static_storage(record: &SymbolRecord) -> bool {
    record
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier, Modifier::StaticStorage))
}

/// Asserts the shared flags, the v2 modifiers, and every convenience copy.
fn assert_static(
    extracted: &Extracted,
    name: &str,
    container: Option<&str>,
    is_static: bool,
    is_static_storage: bool,
) {
    let label = &extracted.label;
    let symbol = extracted.symbol(name, container);
    assert_eq!(symbol.is_static, is_static, "{label}: `{name}` is_static");
    assert_eq!(
        symbol.is_static_storage, is_static_storage,
        "{label}: `{name}` is_static_storage"
    );
    if let Some(signature) = &symbol.signature {
        assert_eq!(signature.is_static, is_static, "{label}: `{name}` signature copy");
    }

    let record = extracted.record(name, container);
    assert_eq!(has_static(record), is_static, "{label}: `{name}` Modifier::Static");
    assert_eq!(
        has_static_storage(record),
        is_static_storage,
        "{label}: `{name}` Modifier::StaticStorage"
    );
    if let SymbolKindData::Field(field) = &record.kind_data {
        assert_eq!(field.is_static, is_static, "{label}: `{name}` FieldData copy");
    }
}

/// One committed cell of the static matrix: `(language, name, container,
/// is_static, is_static_storage)` in the language's primary fixture.
pub type StaticRow = (
    ProgrammingLanguage,
    &'static str,
    Option<&'static str>,
    bool,
    bool,
);

/// Languages with static members (spec section 4), plus C for storage
/// `static`; the coverage guard requires rows for each.
pub const STATIC_LANGUAGES: &[ProgrammingLanguage] = &[
    L::TypeScript,
    L::JavaScript,
    L::Java,
    L::CSharp,
    L::Php,
    L::Cpp,
    L::Swift,
    L::Scala,
    L::Python,
    L::Rust,
    L::C,
];

/// The static matrix: one static and one instance method, and one static and
/// one instance field where fields are extracted, per language; then storage
/// `static`, which is never member `static`.
pub const STATIC_MATRIX: &[StaticRow] = &[
    (L::TypeScript, "staticMethod", Some("ExportedWidget"), true, false),
    (L::TypeScript, "privateStaticMethod", Some("ExportedWidget"), true, false),
    (L::TypeScript, "publicMethod", Some("ExportedWidget"), false, false),
    (L::TypeScript, "staticProperty", Some("ExportedWidget"), true, false),
    (L::TypeScript, "privateStaticProperty", Some("ExportedWidget"), true, false),
    (L::TypeScript, "publicProperty", Some("ExportedWidget"), false, false),
    (L::TypeScript, "exportedFunction", None, false, false),
    (L::JavaScript, "staticMethod", Some("ExportedWidget"), true, false),
    (L::JavaScript, "unmarkedMethod", Some("ExportedWidget"), false, false),
    (L::JavaScript, "staticProperty", Some("ExportedWidget"), true, false),
    (L::JavaScript, "uninitializedStaticProperty", Some("ExportedWidget"), true, false),
    (L::JavaScript, "unmarkedProperty", Some("ExportedWidget"), false, false),
    (L::Java, "staticFactory", Some("PublicWidget"), true, false),
    (L::Java, "publicMethod", Some("PublicWidget"), false, false),
    (L::Java, "publicStaticField", Some("PublicWidget"), true, false),
    (L::Java, "publicField", Some("PublicWidget"), false, false),
    // interface constants are implicitly `public static final`
    (L::Java, "SIDES", Some("PublicShape"), true, false),
    (L::Java, "unit", Some("PublicShape"), true, false),
    (L::Java, "area", Some("PublicShape"), false, false),
    (L::CSharp, "StaticFactory", Some("PublicWidget"), true, false),
    (L::CSharp, "PublicMethod", Some("PublicWidget"), false, false),
    (L::CSharp, "PublicStaticField", Some("PublicWidget"), true, false),
    (L::CSharp, "PublicField", Some("PublicWidget"), false, false),
    (L::CSharp, "Unit", Some("IShape"), true, false),
    (L::Php, "staticFactory", Some("PublicWidget"), true, false),
    (L::Php, "unmarkedStaticMethod", Some("PublicWidget"), true, false),
    (L::Php, "area", Some("PublicWidget"), false, false),
    (L::Php, "publicStaticField", Some("PublicWidget"), true, false),
    (L::Php, "publicField", Some("PublicWidget"), false, false),
    (L::Php, "traitStaticMethod", Some("Greets"), true, false),
    (L::Cpp, "staticFactory", Some("AccessWidget"), true, false),
    (L::Cpp, "firstPublicMethod", Some("AccessWidget"), false, false),
    (L::Cpp, "staticMemberField", Some("AccessWidget"), true, false),
    (L::Cpp, "firstPublicField", Some("AccessWidget"), false, false),
    (L::Swift, "staticMethod", Some("OpenWidget"), true, false),
    (L::Swift, "classMethod", Some("OpenWidget"), true, false),
    (L::Swift, "instanceMethod", Some("OpenWidget"), false, false),
    (L::Swift, "staticProperty", Some("OpenWidget"), true, false),
    (L::Swift, "classProperty", Some("OpenWidget"), true, false),
    (L::Swift, "publicProperty", Some("OpenWidget"), false, false),
    // `object` members are static; class members are not
    (L::Scala, "objectMethod", Some("StandaloneObject"), true, false),
    (L::Scala, "companionStaticMethod", Some("PlainWidget"), true, false),
    (L::Scala, "companionStaticField", Some("PlainWidget"), true, false),
    (L::Scala, "publicMethod", Some("PlainWidget"), false, false),
    (L::Scala, "publicField", Some("PlainWidget"), false, false),
    (L::Python, "static_method", Some("PlainWidget"), true, false),
    (L::Python, "class_method", Some("PlainWidget"), true, false),
    (L::Python, "instance_method", Some("PlainWidget"), false, false),
    // the annotation decides: `x: T` and `x: T = v` are instance fields,
    // `x: ClassVar[T]` and unannotated `x = v` class attributes
    (L::Python, "annotated_instance_field", Some("PlainWidget"), false, false),
    (L::Python, "annotated_instance_field_with_value", Some("PlainWidget"), false, false),
    (L::Python, "class_var_static_field", Some("PlainWidget"), true, false),
    (L::Python, "class_var_static_field_with_value", Some("PlainWidget"), true, false),
    (L::Python, "unannotated_static_field", Some("PlainWidget"), true, false),
    (L::Python, "MODULE_CONSTANT", None, false, false),
    (L::Rust, "new_associated_function", Some("PublicStruct"), true, false),
    (L::Rust, "crate_associated_function", Some("PublicStruct"), true, false),
    (L::Rust, "instance_method", Some("PublicStruct"), false, false),
    (L::Rust, "public_function", None, false, false),
    // storage `static`
    (L::Rust, "PUBLIC_STATIC", None, false, true),
    (L::Rust, "PRIVATE_STATIC_MUT", None, false, true),
    (L::Rust, "LOCAL_STATIC", None, false, true),
    (L::Rust, "PUBLIC_CONSTANT", None, false, false),
    (L::Cpp, "memberLocalStatic", Some("AccessWidget"), false, true),
    (L::Cpp, "fileStaticVariable", None, false, true),
    (L::Cpp, "fileStaticFunction", None, false, true),
    (L::Cpp, "localStaticCounter", None, false, true),
    (L::Cpp, "publicGlobal", None, false, false),
    (L::C, "file_static_variable", None, false, true),
    (L::C, "file_static_function", None, false, true),
    (L::C, "local_static_counter", None, false, true),
    (L::C, "public_global_variable", None, false, false),
    (L::C, "public_function", None, false, false),
];

/// Test 4: every static-matrix row holds on the shared flags, the v2
/// modifiers, and every convenience copy.
#[test]
fn static_matrix_holds_through_both_paths() {
    for (language, rows) in rows_by_language(STATIC_MATRIX, |row| row.0) {
        let extracted = extract_primary(language);
        for (_, name, container, is_static, is_static_storage) in rows {
            assert_static(&extracted, name, *container, *is_static, *is_static_storage);
        }
    }
}

/// `hug symbols` (standalone fields) and `hug classes` (type metadata) list
/// the same Python class-body fields with the same static flag.
#[test]
fn python_class_fields_agree_with_the_standalone_fields() {
    let python = extract_primary(ProgrammingLanguage::Python);
    let mut from_metadata: Vec<(String, bool)> = python
        .symbol("PlainWidget", None)
        .type_metadata
        .as_ref()
        .expect("PlainWidget metadata")
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.is_static))
        .collect();
    let mut standalone: Vec<(String, bool)> = python
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.kind == SymbolKind::Field && symbol.container_name.as_deref() == Some("PlainWidget")
        })
        .map(|symbol| (symbol.name.clone(), symbol.is_static))
        .collect();
    from_metadata.sort();
    standalone.sort();
    assert_eq!(from_metadata, standalone);
    assert_eq!(standalone.len(), 5, "{standalone:?}");
}
