//! The deprecation and test part of the flags matrix of
//! `2026-10-03-symbol-filters-and-reports` (`async` is `symbols_async`):
//! `visibility.is_deprecated` and `visibility.is_test` are set exactly by
//! each language's markers, attributes and file-name rules, never by an
//! ordinary comment or a look-alike name, and the attribute facet stays
//! empty.

use std::collections::BTreeSet;
use std::path::Path;

use tempfile::TempDir;
use tree_hugger::{
    Capability, ProgrammingLanguage, SourceFile, SymbolQuery, SymbolRecord, TreeFile,
};

use crate::symbols_fixtures::{
    in_file_name,
    TEST_FILE_FIXTURES, display_name, primary_fixture, test_file_fixture_dir,
};

use ProgrammingLanguage as L;

pub struct FlagsRow {
    pub language: ProgrammingLanguage,
    /// The qualified name of every deprecated declaration in the primary
    /// fixture, or why the language has no deprecation marker.
    pub deprecated: Result<&'static [&'static str], &'static str>,
    /// The qualified name of every test declaration in a copy of the primary
    /// fixture outside any test directory (attributes and `cfg(test)`; the
    /// file-name rules are `test_file_rules_mark_every_declaration`), or why
    /// the language has no test attribute.
    pub tests: Result<&'static [&'static str], &'static str>,
    /// Declarations that look deprecated or test-like (a marker word in an
    /// ordinary comment, a `test_helper` name, a marker the spec does not
    /// list) and must have neither flag.
    pub decoys: &'static [&'static str],
}

const NO_MARKER: &str = "the language has no deprecation marker in the spec's table";
const NO_TEST_ATTRIBUTE: &str = "only Rust, Java and C# have test attributes";

pub const FLAGS_MATRIX: &[FlagsRow] = &[
    FlagsRow {
        language: L::Rust,
        deprecated: Ok(&["deprecated_api"]),
        tests: Ok(&[
            "attribute_test",
            "cfg_test_module",
            "cfg_test_module::inside_cfg_test",
            "cfg_test_module::inside_cfg_test::cfg_flag",
        ]),
        decoys: &["undocumented_none", "test_helper", "any_test_helper"],
    },
    FlagsRow {
        language: L::TypeScript,
        deprecated: Ok(&["deprecatedApi", "Receiver.withReceiver"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["undocumentedNone", "testHelper"],
    },
    FlagsRow {
        language: L::JavaScript,
        deprecated: Ok(&["deprecatedApi"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["undocumentedNone", "lookAlikeTag", "testHelper"],
    },
    FlagsRow {
        language: L::Go,
        deprecated: Ok(&["symbols.DeprecatedAPI", "symbols.DeprecatedLater"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["symbols.UndocumentedNone", "symbols.TestHelper"],
    },
    FlagsRow {
        language: L::Python,
        deprecated: Ok(&["deprecated_api", "qualified_deprecated", "warnings_deprecated"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["undocumented_none", "look_alike_decorator", "test_helper"],
    },
    FlagsRow {
        language: L::Java,
        deprecated: Ok(&[
            "com.example.symbols.AnnotatedPackageWidget",
            "com.example.symbols.DeclarationBoundaries.boundaryAnnotated",
            "com.example.symbols.DeclarationBoundaries.boundaryAnnotationBetweenModifiers",
            "com.example.symbols.FlagsAndMetrics.deprecatedByTag",
            "com.example.symbols.FlagsAndMetrics.deprecatedQualified",
        ]),
        tests: Ok(&[
            "com.example.symbols.FlagsAndMetrics.annotatedTest",
            "com.example.symbols.FlagsAndMetrics.qualifiedTest",
        ]),
        decoys: &[
            "com.example.symbols.AnnotatedPackageWidget.annotatedWidgetMethod",
            "com.example.symbols.FlagsAndMetrics.undocumentedNone",
            "com.example.symbols.FlagsAndMetrics.testHelper",
        ],
    },
    FlagsRow {
        language: L::CSharp,
        deprecated: Ok(&[
            "Example.Symbols.DeclarationBoundaries.BoundaryAnnotated",
            "Example.Symbols.FlagsAndMetrics.ObsoleteQualified",
        ]),
        tests: Ok(&[
            "Example.Symbols.FlagsAndMetrics.FactTest",
            "Example.Symbols.FlagsAndMetrics.MethodTest",
            "Example.Symbols.FlagsAndMetrics.PlainTest",
        ]),
        decoys: &[
            "Example.Symbols.FlagsAndMetrics.UndocumentedNone",
            "Example.Symbols.FlagsAndMetrics.TestHelper",
        ],
    },
    FlagsRow {
        language: L::C,
        deprecated: Err(NO_MARKER),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["undocumented_none", "test_helper"],
    },
    FlagsRow {
        language: L::Cpp,
        // `[[gnu::deprecated]]` is a vendor attribute, not `[[deprecated]]`.
        deprecated: Ok(&["deprecated_api", "DeprecatedClass"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["undocumented_none", "namespaced_attribute", "test_helper"],
    },
    FlagsRow {
        language: L::Swift,
        // `@available(iOS, deprecated: 13)` deprecates on one platform only.
        deprecated: Ok(&["boundaryAnnotated"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["undocumentedNone", "platformDeprecated", "testHelper"],
    },
    FlagsRow {
        language: L::Scala,
        deprecated: Ok(&["example.symbols.boundaryAnnotated", "example.symbols.qualifiedDeprecated"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["example.symbols.undocumentedNone", "example.symbols.testHelper"],
    },
    FlagsRow {
        language: L::Php,
        // The `#[Deprecated]` attribute is not in the spec's table; only the
        // doc tag is.
        deprecated: Ok(&["Example\\Symbols\\deprecated_api"]),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &[
            "Example\\Symbols\\boundary_annotated",
            "Example\\Symbols\\undocumented_none",
            "Example\\Symbols\\test_helper",
        ],
    },
    FlagsRow {
        language: L::Perl,
        deprecated: Err(NO_MARKER),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["Fixture::Other::test_helper"],
    },
    FlagsRow {
        language: L::Lua,
        deprecated: Err(NO_MARKER),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["test_helper"],
    },
    FlagsRow {
        language: L::Bash,
        deprecated: Err(NO_MARKER),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["test_helper"],
    },
    FlagsRow {
        language: L::Zsh,
        deprecated: Err(NO_MARKER),
        tests: Err(NO_TEST_ATTRIBUTE),
        decoys: &["test_helper"],
    },
];

/// The languages with a flags row, for the coverage guard.
pub fn flags_matrix_languages() -> Vec<ProgrammingLanguage> {
    FLAGS_MATRIX.iter().map(|row| row.language).collect()
}

fn flags_query() -> SymbolQuery {
    SymbolQuery::new().capabilities([Capability::Flags])
}

fn records_of(path: &Path, query: &SymbolQuery) -> Vec<SymbolRecord> {
    SourceFile::read(path)
        .expect("read source")
        .query(query)
        .expect("query")
        .index
        .symbols
}

/// The records of `language`'s primary fixture, copied under its own name
/// into a directory no file-name rule applies to.
fn isolated_primary(language: ProgrammingLanguage, query: &SymbolQuery) -> (String, Vec<SymbolRecord>) {
    let path = primary_fixture(language).expect("primary fixture");
    let dir = TempDir::new().expect("temp dir");
    let copy = dir.path().join(path.file_name().expect("file name"));
    std::fs::copy(&path, &copy).expect("copy fixture");
    (display_name(&path), records_of(&copy, query))
}

fn names_where(records: &[SymbolRecord], flag: impl Fn(&SymbolRecord) -> bool) -> BTreeSet<String> {
    records
        .iter()
        .filter(|record| flag(record))
        .map(|record| in_file_name(record).to_string())
        .collect()
}

fn expected_names(cell: Result<&[&str], &str>) -> BTreeSet<String> {
    cell.map(|names| names.iter().map(|name| name.to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn flags_are_set_exactly_by_each_languages_markers() {
    for row in FLAGS_MATRIX {
        let (label, records) = isolated_primary(row.language, &flags_query());
        for names in [row.deprecated, row.tests].into_iter().flatten() {
            assert!(!names.is_empty(), "{label}: an `Ok` cell lists its declarations");
        }
        assert_eq!(
            names_where(&records, |record| record.visibility.is_deprecated),
            expected_names(row.deprecated),
            "{label}: is_deprecated"
        );
        assert_eq!(
            names_where(&records, |record| record.visibility.is_test),
            expected_names(row.tests),
            "{label}: is_test"
        );
        for decoy in row.decoys {
            let record = records
                .iter()
                .find(|record| in_file_name(record) == *decoy)
                .unwrap_or_else(|| panic!("{label}: no decoy `{decoy}`"));
            assert!(
                !record.visibility.is_deprecated && !record.visibility.is_test,
                "{label}: decoy `{decoy}` is flagged"
            );
        }
        assert!(
            records.iter().all(|record| record.attributes.is_empty()),
            "{label}: the attribute facet stays empty"
        );
    }
}

/// Every declaration of a test file is test-only code, helpers included;
/// the same source under a name no rule matches has no test declaration.
#[test]
fn test_file_rules_mark_every_declaration() {
    let dir = test_file_fixture_dir();
    for (rule, relative) in TEST_FILE_FIXTURES {
        let path = dir.join(relative);
        let records = records_of(&path, &flags_query());
        assert!(records.len() >= 2, "{rule}: a test and a helper");
        assert!(
            records.iter().all(|record| record.visibility.is_test),
            "{rule}: every declaration is a test"
        );

        let renamed_dir = TempDir::new().expect("temp dir");
        let extension = path.extension().and_then(|ext| ext.to_str()).expect("extension");
        let renamed = renamed_dir.path().join(format!("point.{extension}"));
        std::fs::copy(&path, &renamed).expect("copy fixture");
        // Only a declaration with a test attribute stays a test.
        let attribute_tests: BTreeSet<String> = if extension == "rs" {
            BTreeSet::from(["distance_is_zero".to_string()])
        } else {
            BTreeSet::new()
        };
        assert_eq!(
            names_where(&records_of(&renamed, &flags_query()), |record| record.visibility.is_test),
            attribute_tests,
            "{rule}: renamed `point.{extension}`"
        );
    }
}

/// The Rust rule is a path below the `tests/` directory of the nearest Cargo
/// package: a `tests/` without a manifest above it, `src/latest.rs` and a
/// nested `tests/` below `src/` are not tests.
#[test]
fn the_rust_tests_directory_rule_needs_a_package() {
    let source = "fn helper() {}\n";
    let dir = TempDir::new().expect("temp dir");
    let package = dir.path().join("pkg");
    let cases = [
        ("pkg/tests/point.rs", true),
        ("pkg/tests/nested/support.rs", true),
        ("pkg/src/latest.rs", false),
        ("pkg/src/tests/inner.rs", false),
        ("loose/tests/point.rs", false),
    ];
    std::fs::create_dir_all(&package).expect("package dir");
    std::fs::write(package.join("Cargo.toml"), "[package]\nname = \"pkg\"\n").expect("manifest");
    for (relative, is_test) in cases {
        let path = dir.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(&path, source).expect("write source");
        let records = records_of(&path, &flags_query());
        assert_eq!(records.len(), 1, "{relative}");
        assert_eq!(records[0].visibility.is_test, is_test, "{relative}");
    }

    // The primary Rust fixture sits under this crate's `tests/`, so read in
    // place every declaration is a test.
    let in_place = records_of(&primary_fixture(L::Rust).expect("rust fixture"), &flags_query());
    assert!(in_place.iter().all(|record| record.visibility.is_test));
}

/// Marker and attribute forms the fixtures do not hold: `(source, extension,
/// deprecated, tests, neither)` by name.
#[test]
fn flag_forms_outside_the_fixtures() {
    type Case = (&'static str, &'static str, &'static [&'static str], &'static [&'static str], &'static [&'static str]);
    let cases: &[Case] = &[
        (
            "#[tokio::test]\nasync fn async_test() {}\n#[cfg(all(test, unix))]\nfn all_test() {}\n#[deprecated = \"x\"]\nstruct Old;\nstruct S {\n    #[deprecated]\n    old_field: u8,\n}\n",
            "rs",
            &["Old", "old_field"],
            &["async_test"],
            &["all_test", "S"],
        ),
        ("#![cfg(test)]\nfn inner_attribute_file() {}\n", "rs", &[], &["inner_attribute_file"], &[]),
        (
            "mod m {\n    #![cfg(test)]\n    fn in_test_module() {}\n}\nfn outside() {}\n",
            "rs",
            &[],
            &["m", "in_test_module"],
            &["outside"],
        ),
        (
            "class C {\n    @org.junit.Test void qualified() {}\n    @TestFactory void factory() {}\n    @Deprecated(since = \"9\") int old;\n}\n",
            "java",
            &["old"],
            &["qualified"],
            &["factory"],
        ),
        (
            "class C {\n    [NUnit.Framework.TestAttribute] void Qualified() { }\n    [ObsoleteAttribute] int Old;\n    [Theory] void Theory() { }\n}\n",
            "cs",
            &["Old"],
            &["Qualified"],
            &["Theory"],
        ),
        (
            "class C {\n  @available(*, deprecated, message: \"x\")\n  func old() {}\n  @available(*, unavailable)\n  func gone() {}\n}\n",
            "swift",
            &["old"],
            &[],
            &["gone"],
        ),
        (
            "/** @deprecated */\nexport const oldArrow = () => 1;\n// @deprecated in an ordinary comment\nexport function plain() {}\n/** Not @deprecated mid-line. */\nexport function midLine() {}\n",
            "ts",
            &["oldArrow"],
            &[],
            &["plain", "midLine"],
        ),
        (
            "package p\n\n// Old says Deprecated: but not at a paragraph start.\nfunc Old() {}\n",
            "go",
            &[],
            &[],
            &["Old"],
        ),
        (
            "@deprecated\nclass OldClass:\n    pass\n\n@functools.deprecated\ndef other_module():\n    pass\n",
            "py",
            &["OldClass"],
            &[],
            &["other_module"],
        ),
        ("[[deprecated]] int old_variable = 1;\n", "cpp", &["old_variable"], &[], &[]),
    ];
    for (source, extension, deprecated, tests, neither) in cases {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("snippet.{extension}"));
        std::fs::write(&path, source).expect("write snippet");
        let errors = TreeFile::new(&path).expect("parse").syntax_diagnostics();
        assert!(errors.is_empty(), "{source}: {errors:?}");
        let records = records_of(&path, &flags_query());
        let flagged = |name: &str| -> (bool, bool) {
            let record = records
                .iter()
                .find(|record| record.identity.name == name)
                .unwrap_or_else(|| panic!("{source}: no `{name}`"));
            (record.visibility.is_deprecated, record.visibility.is_test)
        };
        for name in *deprecated {
            assert!(flagged(name).0, "{source}: `{name}` is deprecated");
        }
        for name in *tests {
            assert!(flagged(name).1, "{source}: `{name}` is a test");
        }
        for name in *neither {
            assert_eq!(flagged(name), (false, false), "{source}: `{name}` has no flag");
        }
    }
}
