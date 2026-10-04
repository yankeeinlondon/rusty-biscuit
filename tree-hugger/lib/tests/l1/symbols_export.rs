//! Test 10 of `2026-10-03-visibility-and-kind`: `is_exported` is decided by
//! export syntax, never by access level.

use tree_hugger::{ProgrammingLanguage, SymbolRecord, TreeFile};

use crate::symbols_fixtures::primary_fixture;

/// The parse-pass records (`symbol_records()`) and the bound index, which
/// decide export separately.
fn both_paths(language: ProgrammingLanguage) -> [(&'static str, Vec<SymbolRecord>); 2] {
    let tree_file = TreeFile::new(primary_fixture(language).expect("fixture")).expect("parse");
    [
        ("symbol_records()", tree_file.symbol_records().expect("records")),
        (
            "symbol_index_v2()",
            tree_file.symbol_index_v2().expect("index").symbols,
        ),
    ]
}

fn assert_exported(language: ProgrammingLanguage, name: &str, container: Option<&str>, expected: bool) {
    for (path, records) in both_paths(language) {
        let record = records
            .iter()
            .find(|record| {
                record.identity.name == name && record.identity.module_path.as_deref() == container
            })
            .unwrap_or_else(|| panic!("{}: no `{name}` in {path}", language.name()));
        assert_eq!(
            record.visibility.is_exported,
            expected,
            "{} {path}: `{name}` is_exported",
            language.name()
        );
    }
}

#[test]
fn export_follows_export_syntax_not_access() {
    assert_exported(ProgrammingLanguage::TypeScript, "exportedFunction", None, true);
    assert_exported(ProgrammingLanguage::TypeScript, "moduleFunction", None, false);
    assert_exported(ProgrammingLanguage::Rust, "public_function", None, true);
    assert_exported(ProgrammingLanguage::Rust, "private_function", None, false);
    // Public members and types are not exports.
    assert_exported(
        ProgrammingLanguage::TypeScript,
        "publicMethod",
        Some("ExportedWidget"),
        false,
    );
    assert_exported(ProgrammingLanguage::Java, "PublicWidget", None, false);
}

/// Constructors, properties and enum variants are members, excluded as
/// methods and fields are, even inside an exported type.
#[test]
fn members_of_exported_types_are_not_exports() {
    for (name, container) in [
        ("constructor", Some("ExportedWidget")),
        ("publicProperty", Some("ExportedWidget")),
        ("staticProperty", Some("ExportedWidget")),
    ] {
        assert_exported(ProgrammingLanguage::TypeScript, name, container, false);
    }
    for variant in ["UnitVariant", "TupleVariant", "StructVariant"] {
        assert_exported(ProgrammingLanguage::Rust, variant, Some("PublicEnum"), false);
    }
}
