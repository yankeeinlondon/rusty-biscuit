//! Test 10 of `2026-10-03-visibility-and-kind`: `is_exported` is decided by
//! export syntax, never by access level.

use tree_hugger::{ProgrammingLanguage, SymbolRecord, TreeFile, Visibility, VisibilityLevel};

use crate::symbols_fixtures::{all_fixtures, display_name, primary_fixture};

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
                record.identity.name == name && record.identity.container_name.as_deref() == container
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
    // Exported `const` declarations, a function bound to one included.
    for language in [ProgrammingLanguage::TypeScript, ProgrammingLanguage::JavaScript] {
        assert_exported(language, "EXPORTED_CONSTANT", None, true);
        assert_exported(language, "MODULE_CONSTANT", None, false);
        assert_exported(language, "asyncArrow", None, true);
    }
    assert_exported(ProgrammingLanguage::JavaScript, "boundaryArrow", None, true);
    assert_exported(ProgrammingLanguage::JavaScript, "documentedArrow", None, false);
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

/// `From<SymbolInfo> for SymbolRecord` has no export syntax to read, so a
/// Public symbol converts to `is_exported: false`, including the declarations
/// the index does report as exported (TS `export function`, Rust `pub fn`).
#[test]
fn converting_a_public_symbol_never_marks_it_exported() {
    let mut public = 0;
    for path in all_fixtures() {
        for symbol in TreeFile::new(&path).expect("parse").symbols().expect("symbols") {
            if symbol.visibility != Some(Visibility::Public) {
                continue;
            }
            let label = format!("{}: `{}` ({:?})", display_name(&path), symbol.name, symbol.kind);
            let record = SymbolRecord::from(symbol);
            assert_eq!(record.visibility.level, VisibilityLevel::Public, "{label}: level");
            assert!(!record.visibility.is_exported, "{label}: is_exported after conversion");
            public += 1;
        }
    }
    assert!(public > 100, "only {public} Public symbols converted");

    for (language, name) in [
        (ProgrammingLanguage::TypeScript, "exportedFunction"),
        (ProgrammingLanguage::Rust, "public_function"),
    ] {
        let symbols = TreeFile::new(primary_fixture(language).expect("fixture"))
            .expect("parse")
            .symbols()
            .expect("symbols");
        let symbol = symbols
            .into_iter()
            .find(|symbol| symbol.name == name && symbol.container_name.is_none())
            .unwrap_or_else(|| panic!("{}: no `{name}`", language.name()));
        assert!(
            !SymbolRecord::from(symbol).visibility.is_exported,
            "{}: `{name}` converted from SymbolInfo",
            language.name()
        );
    }
}
