//! Test 9 of `2026-10-03-visibility-and-kind`: every fixture symbol carries a
//! non-empty `source.declaration_text`, with exact-text cases for the
//! declarations whose syntax a visibility level cannot carry.

use tree_hugger::{ProgrammingLanguage, TreeFile};

use crate::symbols_fixtures::{display_name, extract_primary};

crate::language_cases::language_tests!(every_fixture_record_has_declaration_text => check_every_fixture_record_has_declaration_text);

/// Every record of every fixture carries non-empty declaration text, in the
/// parse-pass records and in the full index that later passes rebuild.
fn check_every_fixture_record_has_declaration_text(language: ProgrammingLanguage) {
    let mut checked = 0;
    for path in crate::symbols_fixtures::fixtures_for(language, false) {
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        for (pass, records) in [
            ("symbol_records()", tree_file.symbol_records().expect("records")),
            ("symbol_index_v2()", tree_file.symbol_index_v2().expect("index").symbols),
        ] {
            for record in records {
                let text = record.source.declaration_text.as_deref().unwrap_or_default();
                assert!(
                    !text.trim().is_empty(),
                    "{} {pass}: `{}` ({:?}) has no declaration text",
                    display_name(&path),
                    record.identity.name,
                    record.kind
                );
                assert_eq!(text, text.trim_end(), "trailing whitespace is trimmed");
                checked += 1;
            }
        }
    }

    assert!(checked >= 2 * crate::language_cases::corpus_minimums(language).symbols, "only {checked} records checked");
}

/// The text keeps syntax a visibility level cannot carry and stops at the body.
#[test]
fn declaration_text_keeps_unmappable_syntax_and_stops_at_the_body() {
    let cases: &[(ProgrammingLanguage, &str, Option<&str>, &str)] = &[
        (
            ProgrammingLanguage::Rust,
            "in_path_function",
            Some("crate_inner_module"),
            "pub(in crate::outer_module) fn in_path_function()",
        ),
        (
            ProgrammingLanguage::Rust,
            "crate_field",
            Some("PublicStruct"),
            "pub(crate) crate_field: u32",
        ),
        (
            ProgrammingLanguage::Rust,
            "required_trait_method",
            Some("PublicTrait"),
            "fn required_trait_method(&self) -> u32;",
        ),
        (
            ProgrammingLanguage::Swift,
            "count",
            Some("PublicApi"),
            "public private(set) var count = 0",
        ),
        (
            ProgrammingLanguage::CSharp,
            "Count",
            Some("PublicWidget"),
            "public int Count { get; private set; }",
        ),
        (
            ProgrammingLanguage::Scala,
            "secret",
            Some("Shape"),
            "private[Shape] def secret() =",
        ),
        // each name of a list declaration shares the declaration's text
        (
            ProgrammingLanguage::Perl,
            "$first_name",
            Some("greet"),
            "my ($first_name, $last_name)",
        ),
        (
            ProgrammingLanguage::Perl,
            "$last_name",
            Some("greet"),
            "my ($first_name, $last_name)",
        ),
        (
            ProgrammingLanguage::Java,
            "AnnotatedPackageWidget",
            None,
            "@Deprecated\nclass AnnotatedPackageWidget",
        ),
        (
            ProgrammingLanguage::TypeScript,
            "ExportedWidget",
            None,
            "class ExportedWidget extends ExportedAbstractBase",
        ),
    ];
    for (language, name, container, expected) in cases {
        let extracted = extract_primary(*language);
        assert_eq!(
            extracted
                .record(name, *container)
                .source
                .declaration_text
                .as_deref(),
            Some(*expected),
            "{}: `{name}`",
            extracted.label
        );
    }
}
