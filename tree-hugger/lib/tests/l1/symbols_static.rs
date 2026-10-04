//! Test 4 of `2026-10-03-visibility-and-kind`: the static matrix (member
//! `static` on the shared flag, field copies and `Modifier::Static`) and
//! storage `static` (`Modifier::StaticStorage` / `is_static_storage`).

use tree_hugger::{Modifier, ProgrammingLanguage, SymbolKindData, SymbolRecord};

use crate::symbols_fixtures::{Extracted, extract_primary};

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

#[test]
fn member_static_reaches_methods_and_fields() {
    let typescript = extract_primary(ProgrammingLanguage::TypeScript);
    assert_static(&typescript, "staticMethod", Some("ExportedWidget"), true, false);
    assert_static(&typescript, "publicMethod", Some("ExportedWidget"), false, false);
    assert_static(&typescript, "staticProperty", Some("ExportedWidget"), true, false);
    assert_static(&typescript, "publicProperty", Some("ExportedWidget"), false, false);

    let java = extract_primary(ProgrammingLanguage::Java);
    assert_static(&java, "staticFactory", Some("PublicWidget"), true, false);
    assert_static(&java, "publicStaticField", Some("PublicWidget"), true, false);
    assert_static(&java, "publicField", Some("PublicWidget"), false, false);

    let rust = extract_primary(ProgrammingLanguage::Rust);
    assert_static(&rust, "new_associated_function", Some("PublicStruct"), true, false);
    assert_static(&rust, "instance_method", Some("PublicStruct"), false, false);
    assert_static(&rust, "public_function", None, false, false);
}

/// `static` that sets storage or linkage is never member `static`, and a C++
/// class member's `static` is never storage.
#[test]
fn storage_static_is_kept_apart_from_member_static() {
    let cpp = extract_primary(ProgrammingLanguage::Cpp);
    assert_static(&cpp, "staticMemberField", Some("AccessWidget"), true, false);
    assert_static(&cpp, "staticFactory", Some("AccessWidget"), true, false);
    assert_static(&cpp, "memberLocalStatic", Some("AccessWidget"), false, true);
    assert_static(&cpp, "fileStaticVariable", None, false, true);
    assert_static(&cpp, "fileStaticFunction", None, false, true);
    assert_static(&cpp, "localStaticCounter", None, false, true);
    assert_static(&cpp, "publicGlobal", None, false, false);

    let c = extract_primary(ProgrammingLanguage::C);
    assert_static(&c, "file_static_variable", None, false, true);
    assert_static(&c, "file_static_function", None, false, true);
    assert_static(&c, "local_static_counter", None, false, true);
    assert_static(&c, "public_global_variable", None, false, false);
    assert_static(&c, "public_function", None, false, false);
}
