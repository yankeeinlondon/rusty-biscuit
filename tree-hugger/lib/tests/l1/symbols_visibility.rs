//! Test 3 of `2026-10-03-visibility-and-kind`: the visibility matrix for all
//! sixteen languages, asserted on `SymbolInfo::visibility` and
//! `SymbolRecord.visibility.level`. Rows land with each language's visibility
//! rules.

use tree_hugger::{ProgrammingLanguage, SymbolKind, Visibility, VisibilityLevel};

use crate::symbols_fixtures::{Extracted, extract_inline, extract_primary};

fn assert_level(extracted: &Extracted, name: &str, container: Option<&str>, expected: Option<Visibility>) {
    let symbol = extracted.symbol(name, container);
    assert_eq!(
        symbol.visibility, expected,
        "{}: `{name}` SymbolInfo::visibility",
        extracted.label
    );
    assert_eq!(
        extracted.record(name, container).visibility.level,
        VisibilityLevel::from(expected),
        "{}: `{name}` SymbolRecord.visibility.level",
        extracted.label
    );
}

/// Visibility is read from any declaration, not only functions.
#[test]
fn declaration_keywords_apply_to_every_kind() {
    let java = extract_primary(ProgrammingLanguage::Java);
    assert_level(&java, "PublicWidget", None, Some(Visibility::Public));
    assert_level(&java, "publicField", Some("PublicWidget"), Some(Visibility::Public));
    assert_level(&java, "privateField", Some("PublicWidget"), Some(Visibility::Private));
    assert_level(&java, "PublicColor", None, Some(Visibility::Public));

    let csharp = extract_primary(ProgrammingLanguage::CSharp);
    assert_level(&csharp, "InternalClass", Some("Example.Symbols"), Some(Visibility::Internal));
    assert_level(&csharp, "ProtectedField", Some("PublicWidget"), Some(Visibility::Protected));

    let typescript = extract_primary(ProgrammingLanguage::TypeScript);
    assert_level(&typescript, "privateProperty", Some("ExportedWidget"), Some(Visibility::Private));

    let rust = extract_primary(ProgrammingLanguage::Rust);
    assert_level(&rust, "PublicStruct", None, Some(Visibility::Public));
    assert_level(&rust, "outer_module", None, Some(Visibility::Public));
    assert_level(&rust, "PUBLIC_CONSTANT", None, Some(Visibility::Public));
    assert_level(&rust, "public_field", Some("PublicStruct"), Some(Visibility::Public));

    let php = extract_primary(ProgrammingLanguage::Php);
    assert_level(&php, "protectedField", Some("PublicWidget"), Some(Visibility::Protected));

    // An access section reaches a C++ declaration the class body declares.
    let cpp = extract_primary(ProgrammingLanguage::Cpp);
    assert_level(&cpp, "AccessWidget", Some("AccessWidget"), Some(Visibility::Public));
    assert_level(&cpp, "protectedField", Some("AccessWidget"), Some(Visibility::Protected));
}

/// Parameters and locals have no member or module access, even inside a
/// type whose members default to a level.
#[test]
fn parameters_and_locals_stay_unknown() {
    let java = extract_inline(
        "interface Shape {\n    default int area(int side) {\n        int squared = side * side;\n        return squared;\n    }\n}\n",
        "java",
    );
    assert_level(&java, "area", Some("Shape"), Some(Visibility::Public));
    for name in ["side", "squared"] {
        let symbol = java
            .symbols
            .iter()
            .find(|symbol| symbol.name == name)
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(symbol.visibility, None, "`{name}` inside an interface method");
        assert!(matches!(symbol.kind, SymbolKind::Parameter | SymbolKind::Variable));
    }

    let rust = extract_inline("pub fn outer() {\n    pub fn nested() {}\n}\n", "rs");
    assert_level(&rust, "outer", None, Some(Visibility::Public));
    assert_level(&rust, "nested", None, None);

    let rust_fixture = extract_primary(ProgrammingLanguage::Rust);
    assert_level(&rust_fixture, "local_value", None, None);
    assert_level(&rust_fixture, "parameter_value", None, None);
}
