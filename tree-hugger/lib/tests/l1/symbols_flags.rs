//! The deprecation and test part of the flags matrix of
//! `2026-10-03-symbol-filters-and-reports` (`async` is `symbols_async`):
//! `visibility.is_deprecated` and `visibility.is_test` are set exactly by
//! each language's markers, attributes and file-name rules, never by an
//! ordinary comment or a look-alike name, and the attribute facet stays
//! empty.

use std::collections::BTreeSet;
use std::path::Path;

use tempfile::TempDir;
use tree_hugger::cache::{CacheLookup, PersistentCache, SymbolCache};
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

/// `(name, is_test)` per record, sorted.
fn test_flags(records: &[SymbolRecord]) -> Vec<(String, bool)> {
    let mut flags: Vec<(String, bool)> = records
        .iter()
        .map(|record| (in_file_name(record).to_string(), record.visibility.is_test))
        .collect();
    flags.sort();
    flags
}

/// The file-wide test rules are applied after cache lookup: a cache hit over
/// an unchanged file follows a manifest added above it, a nearer one, and
/// their removal exactly as fresh analysis does, while the stored entry
/// keeps only the source-derived flags (`#[test]`, `cfg(test)`).
#[test]
fn cached_test_flags_follow_the_package_layout() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("tests").join("sample.rs");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("tests dir");
    std::fs::write(&file, "pub fn helper() {}\n#[test]\nfn checks() {}\n").expect("write source");
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
    let outer = dir.path().join("Cargo.toml");
    let nearer = dir.path().join("tests").join("Cargo.toml");
    let manifest = "[package]\nname = \"sample\"\n";
    let query = flags_query();

    let cold = cache.query(SourceFile::read(&file).expect("read"), &query).expect("cold query");
    assert!(matches!(cold.lookup, CacheLookup::Miss(_)), "{:?}", cold.lookup);
    let step = |label: &str, helper_is_test: bool| {
        let warm = cache.query(SourceFile::read(&file).expect("read"), &query).expect("warm query");
        assert!(matches!(warm.lookup, CacheLookup::Hit), "{label}: {:?}", warm.lookup);
        let fresh = records_of(&file, &query);
        assert_eq!(test_flags(&warm.result.index.symbols), test_flags(&fresh), "{label}: warm vs fresh");
        assert_eq!(
            test_flags(&fresh),
            [("checks".to_string(), true), ("helper".to_string(), helper_is_test)],
            "{label}"
        );
        // A narrower query answered from the result reapplies nothing new.
        let narrowed = warm.result.narrowed(&query).expect("same query narrows");
        assert_eq!(test_flags(&narrowed.index.symbols), test_flags(&fresh), "{label}: narrowed");
    };

    step("no manifest", false);
    std::fs::write(&outer, manifest).expect("outer manifest");
    step("outer manifest", true);
    std::fs::write(&nearer, manifest).expect("nearer manifest");
    step("nearer manifest", false);
    std::fs::remove_file(&nearer).expect("remove nearer");
    step("nearer removed", true);
    std::fs::remove_file(&outer).expect("remove outer");
    step("outer removed", false);
}

/// `(id, is_test)` of every record, sorted: what a reused result must agree
/// with a fresh one on.
fn ids_and_test_flags(records: &[SymbolRecord]) -> Vec<(String, bool)> {
    let mut flags: Vec<(String, bool)> =
        records.iter().map(|record| (record.id.0.clone(), record.visibility.is_test)).collect();
    flags.sort();
    flags
}

/// One package-layout change around `tests/probe.rs`: the manifests present
/// when the reused result is made, then the manifests present when it is
/// narrowed.
struct LayoutChange {
    label: &'static str,
    before: &'static [&'static str],
    after: &'static [&'static str],
}

const LAYOUT_CHANGES: &[LayoutChange] = &[
    LayoutChange { label: "no manifest -> add outer", before: &[], after: &["Cargo.toml"] },
    LayoutChange { label: "outer -> remove it", before: &["Cargo.toml"], after: &[] },
    LayoutChange {
        label: "outer -> add nearer tests/Cargo.toml",
        before: &["Cargo.toml"],
        after: &["Cargo.toml", "tests/Cargo.toml"],
    },
    LayoutChange {
        label: "nearer -> remove it",
        before: &["Cargo.toml", "tests/Cargo.toml"],
        after: &["Cargo.toml"],
    },
    LayoutChange { label: "unchanged outer", before: &["Cargo.toml"], after: &["Cargo.toml"] },
];

/// Writes exactly `manifests` (relative to `root`) and removes the others
/// of [`LAYOUT_CHANGES`].
fn lay_out(root: &Path, manifests: &[&str]) {
    for manifest in ["Cargo.toml", "tests/Cargo.toml"] {
        let path = root.join(manifest);
        if manifests.contains(&manifest) {
            std::fs::write(&path, "[package]\nname = \"probe\"\n").expect("write manifest");
        } else if path.exists() {
            std::fs::remove_file(&path).expect("remove manifest");
        }
    }
}

/// A test selector by name, applied to a query.
type Selector = (&'static str, fn(SymbolQuery) -> SymbolQuery);

/// A result reused through the public `narrowed` API equals a fresh query
/// after the package layout changes in either direction: a record that only
/// the file-wide Rust package-`tests/` rule made a test follows the layout,
/// while a `#[test]` function stays a test. A result already filtered by a
/// test selector is reused only while the file-wide decision it was filtered
/// under still holds, since a record it dropped may match now.
#[test]
fn narrowed_test_flags_follow_the_package_layout() {
    let selectors: [Selector; 3] = [
        ("no selector", |query| query),
        ("tests_only", |query| query.tests_only(true)),
        ("exclude_tests", |query| query.exclude_tests(true)),
    ];
    for change in LAYOUT_CHANGES {
        let dir = TempDir::new().expect("temp dir");
        let file = dir.path().join("tests").join("probe.rs");
        std::fs::create_dir_all(file.parent().expect("parent")).expect("tests dir");
        std::fs::write(&file, "pub fn helper() {}\n#[test]\nfn marked() {}\n").expect("write source");
        // `tests/` is an integration-test directory only under an outer
        // package that does not make `tests/` its own root.
        let in_package_tests =
            |manifests: &[&str]| manifests.contains(&"Cargo.toml") && !manifests.contains(&"tests/Cargo.toml");
        let helper_is_test = in_package_tests(change.after);
        let classification_changes = in_package_tests(change.before) != helper_is_test;

        lay_out(dir.path(), change.before);
        let wide = SourceFile::read(&file).expect("read").query(&flags_query()).expect("wide");
        let prefiltered: Vec<_> = selectors
            .iter()
            .map(|(name, select)| {
                let query = select(flags_query());
                (*name, query.clone(), SourceFile::read(&file).expect("read").query(&query).expect("filtered"))
            })
            .collect();
        lay_out(dir.path(), change.after);

        let fresh_all = records_of(&file, &flags_query());
        assert_eq!(
            test_flags(&fresh_all),
            [("helper".to_string(), helper_is_test), ("marked".to_string(), true)],
            "{}: fresh",
            change.label
        );
        for (name, select) in &selectors {
            let query = select(flags_query());
            let fresh = records_of(&file, &query);
            let narrowed = wide.clone().narrowed(&query).unwrap_or_else(|| panic!("{}: {name}", change.label));
            assert_eq!(
                ids_and_test_flags(&narrowed.index.symbols),
                ids_and_test_flags(&fresh),
                "{}: wide narrowed to {name}",
                change.label
            );
        }
        for (name, query, filtered) in prefiltered {
            let fresh = records_of(&file, &query);
            match filtered.narrowed(&query) {
                Some(reused) => {
                    assert!(
                        name == "no selector" || !classification_changes,
                        "{}: {name} reused across a changed file-wide decision",
                        change.label
                    );
                    assert_eq!(
                        ids_and_test_flags(&reused.index.symbols),
                        ids_and_test_flags(&fresh),
                        "{}: {name} reused",
                        change.label
                    );
                }
                None => assert!(classification_changes, "{}: {name} declined", change.label),
            }
        }
    }
}

/// Every language's primary fixture, copied under a scratch `tests/`
/// directory, narrows to a fresh result across every layout change: only
/// Rust's file-wide rule reads the manifests, and every language's
/// source-derived flags survive reuse.
#[test]
fn narrowed_fixture_flags_follow_the_package_layout() {
    for row in FLAGS_MATRIX {
        let path = primary_fixture(row.language).expect("primary fixture");
        for change in LAYOUT_CHANGES {
            let dir = TempDir::new().expect("temp dir");
            let copy = dir.path().join("tests").join(path.file_name().expect("file name"));
            std::fs::create_dir_all(copy.parent().expect("parent")).expect("tests dir");
            std::fs::copy(&path, &copy).expect("copy fixture");
            lay_out(dir.path(), change.before);
            let wide = SourceFile::read(&copy).expect("read").query(&flags_query()).expect("wide");
            lay_out(dir.path(), change.after);
            let narrowed = wide.narrowed(&flags_query()).expect("same query narrows");
            assert_eq!(
                ids_and_test_flags(&narrowed.index.symbols),
                ids_and_test_flags(&records_of(&copy, &flags_query())),
                "{}: {}",
                display_name(&path),
                change.label
            );
        }
    }
}

/// Repeated reuse across successive layout changes never accumulates a
/// file-wide decision, and a name-rule file beside it (Go `_test.go`) is a
/// test file throughout.
#[test]
fn chained_narrowing_tracks_every_layout_change() {
    let dir = TempDir::new().expect("temp dir");
    let rust = dir.path().join("tests").join("probe.rs");
    let go = dir.path().join("tests").join("probe_test.go");
    std::fs::create_dir_all(rust.parent().expect("parent")).expect("tests dir");
    std::fs::write(&rust, "pub fn helper() {}\n#[test]\nfn marked() {}\n").expect("write rust");
    std::fs::write(&go, "package probe\n\nfunc helper() {}\n").expect("write go");
    let query = flags_query();
    let mut reused: Vec<_> = [&rust, &go]
        .into_iter()
        .map(|file| SourceFile::read(file).expect("read").query(&query).expect("query"))
        .collect();
    for manifests in [&["Cargo.toml"][..], &["Cargo.toml", "tests/Cargo.toml"], &["Cargo.toml"], &[], &[]] {
        lay_out(dir.path(), manifests);
        reused = reused
            .into_iter()
            .zip([&rust, &go])
            .map(|(result, file)| {
                let narrowed = result.narrowed(&query).expect("same query narrows");
                assert_eq!(
                    ids_and_test_flags(&narrowed.index.symbols),
                    ids_and_test_flags(&records_of(file, &query)),
                    "{manifests:?}: {}",
                    file.display()
                );
                narrowed
            })
            .collect();
    }
    assert!(reused[1].index.symbols.iter().all(|record| record.visibility.is_test));
}

/// The name rules are applied after lookup too: on a cache hit every
/// declaration of a test-named Go, Python or TypeScript file is a test.
#[test]
fn cached_test_file_name_rules_hold_on_a_hit() {
    let dir = TempDir::new().expect("temp dir");
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
    for (name, source) in [
        ("point_test.go", "package point\n\nfunc helper() {}\n"),
        ("test_point.py", "def helper():\n    pass\n"),
        ("point.spec.ts", "export function helper() {}\n"),
    ] {
        let file = dir.path().join(name);
        std::fs::write(&file, source).expect("write source");
        cache.query(SourceFile::read(&file).expect("read"), &flags_query()).expect("cold");
        let warm = cache.query(SourceFile::read(&file).expect("read"), &flags_query()).expect("warm");
        assert!(matches!(warm.lookup, CacheLookup::Hit), "{name}: {:?}", warm.lookup);
        let flags = test_flags(&warm.result.index.symbols);
        assert!(flags.iter().any(|(record, _)| record.ends_with("helper")), "{name}: {flags:?}");
        assert!(flags.iter().all(|(_, is_test)| *is_test), "{name}: {flags:?}");
    }
}

/// `(source, extension, deprecated, tests, neither)` by name.
type FlagCase = (&'static str, &'static str, &'static [&'static str], &'static [&'static str], &'static [&'static str]);

/// Marker and attribute forms the fixtures do not hold.
#[test]
fn flag_forms_outside_the_fixtures() {
    let cases: &[FlagCase] = &[
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
    assert_flag_cases(cases);
}

/// Markers are read from syntax: a marker word inside a string or another
/// attribute's argument never counts, and a genuine marker separated from
/// its neighbors by comments, quoted commas or a doc paragraph break still
/// does.
#[test]
fn marker_boundaries_follow_syntax_not_text() {
    let cases: &[FlagCase] = &[
        (
            "class C {\n  @available(*, message: \"hello, deprecated, world\")\n  func decoy() {}\n  @available(*, deprecated, message: \"old, deprecated, gone\")\n  func real() {}\n  @available(*, /* why */ deprecated)\n  func commented() {}\n}\n",
            "swift",
            &["real", "commented"],
            &[],
            &["decoy"],
        ),
        (
            "#[cfg(/* rationale */ test)]\nmod checks {\n    pub fn helper() {}\n}\n#[cfg(test /* trailing */)]\nfn trailing() {}\n#[cfg(any(test, /* c */ unix))]\nfn any_test() {}\n#[/* x */ test]\nfn commented_test() {}\n#[tokio:: /* c */ test]\nasync fn qualified_test() {}\n#[deprecated /* c */]\nfn commented_deprecated() {}\n#[doc = \"deprecated, test\"]\nfn doc_decoy() {}\n",
            "rs",
            &["commented_deprecated"],
            &["checks", "helper", "trailing", "commented_test", "qualified_test"],
            &["any_test", "doc_decoy"],
        ),
        (
            "package p\n\n/* OldBlock documents OldBlock.\n\nDeprecated: use New.\n*/\nfunc OldBlock() {}\n\n/* Deprecated: use New. */\nfunc FirstBlock() {}\n\n// OldLines documents OldLines.\n//\n// Deprecated: use New.\nfunc OldLines() {}\n\n/* MidBlock says\nDeprecated: mid-paragraph.\n*/\nfunc MidBlock() {}\n\n// MidLines says\n// Deprecated: mid-paragraph.\nfunc MidLines() {}\n",
            "go",
            &["OldBlock", "FirstBlock", "OldLines"],
            &[],
            &["MidBlock", "MidLines"],
        ),
        (
            "[[deprecated(\"old, deprecated\")]] int quoted = 1;\n[[nodiscard(\"deprecated\")]] int decoy();\n[[nodiscard, deprecated]] int listed();\n[[/* c */ deprecated]] int commented = 2;\n",
            "cpp",
            &["quoted", "listed", "commented"],
            &[],
            &["decoy"],
        ),
        (
            "class C {\n    [Obsolete /* c */] int Old;\n    [Test, Fact] void Both() { }\n    [NUnit. /* c */ Framework.Test] void Qualified() { }\n    [Description(\"Obsolete, Test\")] int Decoy;\n}\n",
            "cs",
            &["Old"],
            &["Both", "Qualified"],
            &["Decoy"],
        ),
        (
            "class C {\n    @java.lang. /* c */ Deprecated int old;\n    @org. /* c */ junit.Test void qualified() {}\n    @SuppressWarnings(\"Deprecated, Test\") int decoy;\n}\n",
            "java",
            &["old"],
            &["qualified"],
            &["decoy"],
        ),
        (
            "object O {\n  @deprecated /* c */ (\"x\", \"1\") def old(): Int = 1\n  @nowarn(\"deprecated\") def decoy(): Int = 2\n}\n",
            "scala",
            &["old"],
            &[],
            &["decoy"],
        ),
        (
            "@(deprecated)\ndef wrapped():\n    pass\n\n@(\n    warnings.deprecated  # why\n)\ndef commented():\n    pass\n\n@decorate(\"deprecated, test\")\ndef decoy():\n    pass\n",
            "py",
            &["wrapped", "commented"],
            &[],
            &["decoy"],
        ),
        (
            "/**\n * Example:\n * ```\n * @deprecated\n * ```\n */\nexport function fenced() {}\n/** Mark it with `@deprecated` sparingly. */\nexport function codeSpan() {}\n/**\n * \"@deprecated\" in quotes.\n */\nexport function quoted() {}\n/// @deprecated\nexport function lineComment() {}\n/**\n * ```\n * code\n * ```\n * @deprecated after the fence\n */\nexport function afterFence() {}\n",
            "ts",
            &["afterFence"],
            &[],
            &["fenced", "codeSpan", "quoted", "lineComment"],
        ),
        (
            "<?php\n/**\n * ```\n * @deprecated\n * ```\n */\nfunction fenced() {}\n/**\n * @deprecated use other\n */\nfunction old() {}\n",
            "php",
            &["old"],
            &[],
            &["fenced"],
        ),
    ];
    assert_flag_cases(cases);
}

/// One documentation shape for the doc-tag readers: the doc block's lines
/// (without delimiters) and whether its declaration is deprecated.
struct FenceRow {
    name: &'static str,
    lines: &'static [&'static str],
    deprecated: bool,
}

/// Every fence boundary the `@deprecated` doc-tag readers must honor: a
/// fence closes only on a run of its own character at least as long as the
/// opening run with nothing after it, so a shorter run, the other character
/// or a run followed by text is example content (no false positive), and a
/// real tag after the actual close still counts (no false negative).
const FENCE_MATRIX: &[FenceRow] = &[
    FenceRow { name: "plain_tag", lines: &["@deprecated use other"], deprecated: true },
    FenceRow { name: "ordinary_text", lines: &["Ordinary documentation text."], deprecated: false },
    FenceRow { name: "backtick_fence", lines: &["```", "@deprecated example", "```"], deprecated: false },
    FenceRow { name: "tilde_fence", lines: &["~~~", "@deprecated example", "~~~"], deprecated: false },
    FenceRow {
        name: "shorter_backtick_run_inside",
        lines: &["````text", "```", "@deprecated example", "````"],
        deprecated: false,
    },
    FenceRow {
        name: "shorter_tilde_run_inside",
        lines: &["~~~~", "~~~", "@deprecated example", "~~~~"],
        deprecated: false,
    },
    FenceRow {
        name: "other_marker_inside",
        lines: &["```", "~~~", "@deprecated example", "```"],
        deprecated: false,
    },
    FenceRow {
        name: "run_with_text_inside",
        lines: &["```", "```text", "@deprecated example", "```"],
        deprecated: false,
    },
    FenceRow {
        name: "longer_close_then_tag",
        lines: &["```", "code", "`````", "@deprecated use other"],
        deprecated: true,
    },
    FenceRow {
        name: "matching_close_then_tag",
        lines: &["```", "code", "```", "@deprecated use other"],
        deprecated: true,
    },
    FenceRow {
        name: "shorter_run_then_close_then_tag",
        lines: &["````", "```", "code", "````", "@deprecated use other"],
        deprecated: true,
    },
    FenceRow {
        name: "other_marker_then_close_then_tag",
        lines: &["```", "~~~", "code", "```", "@deprecated use other"],
        deprecated: true,
    },
    FenceRow {
        name: "run_with_text_then_close_then_tag",
        lines: &["```", "``` trailing", "code", "```", "@deprecated use other"],
        deprecated: true,
    },
    // Beyond the review's matrix: a backtick info string holding a backtick
    // opens no fence, and an unclosed fence runs to the end of the doc.
    FenceRow {
        name: "backtick_in_info_string",
        lines: &["```not`a fence", "@deprecated use other"],
        deprecated: true,
    },
    FenceRow { name: "unclosed_fence", lines: &["```", "@deprecated example"], deprecated: false },
];

/// `source` for `extension` declaring one function (Java: one method of a
/// class) per `FENCE_MATRIX` row, each documented by its row's lines.
fn fence_matrix_source(extension: &str) -> String {
    let doc = |row: &FenceRow| {
        let body: String = row.lines.iter().map(|line| format!(" * {line}\n")).collect();
        format!("/**\n{body} */\n")
    };
    let declarations: String = FENCE_MATRIX
        .iter()
        .map(|row| match extension {
            "java" => format!("{}void {}() {{}}\n", doc(row), row.name),
            "php" => format!("{}function {}() {{}}\n", doc(row), row.name),
            _ => format!("{}export function {}() {{}}\n", doc(row), row.name),
        })
        .collect();
    match extension {
        "java" => format!("class Fences {{\n{declarations}}}\n"),
        "php" => format!("<?php\n{declarations}"),
        _ => declarations,
    }
}

#[test]
fn doc_tag_readers_follow_fence_boundaries() {
    for extension in ["java", "js", "ts", "php"] {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join(format!("fences.{extension}"));
        let source = fence_matrix_source(extension);
        std::fs::write(&path, &source).expect("write snippet");
        let errors = TreeFile::new(&path).expect("parse").syntax_diagnostics();
        assert!(errors.is_empty(), "{extension}: {errors:?}\n{source}");

        let records = records_of(&path, &flags_query());
        let mismatches: Vec<String> = FENCE_MATRIX
            .iter()
            .filter_map(|row| {
                let record = records
                    .iter()
                    .find(|record| record.identity.name == row.name)
                    .unwrap_or_else(|| panic!("{extension}: no `{}`", row.name));
                (record.visibility.is_deprecated != row.deprecated)
                    .then(|| format!("{} (expected {})", row.name, row.deprecated))
            })
            .collect();
        assert!(mismatches.is_empty(), "{extension}: wrong is_deprecated for {mismatches:?}");
    }
}

/// A Go `Deprecated:` line indented past the doc's common indentation is a
/// code block (`go/doc/comment`), in `//` and `/* */` docs alike; a doc
/// indented as a whole is unindented first, and a paragraph after a code
/// block still counts.
#[test]
fn go_deprecated_paragraphs_skip_code_blocks() {
    let cases: &[FlagCase] = &[(
        concat!(
            "package p\n\n",
            "// CodeLine shows an example.\n//\n//\tDeprecated: example text\nfunc CodeLine() {}\n\n",
            "/* CodeBlock shows an example.\n\n    Deprecated: example text\n*/\nfunc CodeBlock() {}\n\n",
            "// AfterCode shows an example.\n//\n//\tcode()\n//\n// Deprecated: use New.\nfunc AfterCode() {}\n\n",
            "/*\n\tAllIndented documents itself.\n\n\tDeprecated: use New.\n*/\nfunc AllIndented() {}\n\n",
            "//  Deprecated: use New.\nfunc IndentedLine() {}\n",
        ),
        "go",
        &["AfterCode", "AllIndented", "IndentedLine"],
        &[],
        &["CodeLine", "CodeBlock"],
    )];
    assert_flag_cases(cases);
}

fn assert_flag_cases(cases: &[FlagCase]) {
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

/// One row of the Go comment-text matrix: a function documented by `doc`
/// and whether that makes it deprecated.
struct GoDocRow {
    name: &'static str,
    doc: &'static str,
    deprecated: bool,
}

/// Go defines no comment decoration: a block loses exactly `/*` and `*/`, a
/// line comment `//`, and a `*` anywhere else is text (`CommentGroup.Text`),
/// so a starred `Deprecated:` is not a marker. Only whitespace common to the
/// whole doc is removed (`go/doc/comment`).
const GO_DOC_MATRIX: &[GoDocRow] = &[
    GoDocRow { name: "PlainBlock", doc: "/* Deprecated: use New. */", deprecated: true },
    GoDocRow {
        name: "BlockIntroMarker",
        doc: "/*\nBlockIntroMarker documents itself.\n\nDeprecated: use New.\n*/",
        deprecated: true,
    },
    GoDocRow { name: "PlainLine", doc: "// Deprecated: use New.", deprecated: true },
    GoDocRow {
        name: "LineIntroMarker",
        doc: "// LineIntroMarker documents itself.\n//\n// Deprecated: use New.",
        deprecated: true,
    },
    GoDocRow {
        name: "BlockIndentedExample",
        doc: "/* BlockIndentedExample shows an example.\n\n    Deprecated: example text\n*/",
        deprecated: false,
    },
    GoDocRow {
        name: "LineIndentedExample",
        doc: "// LineIndentedExample shows an example.\n//\n//\tDeprecated: example text",
        deprecated: false,
    },
    GoDocRow {
        name: "BlockStarBody",
        doc: "/*\n*Deprecated: this is literal starred text.\n*/",
        deprecated: false,
    },
    GoDocRow {
        name: "BlockStarSingle",
        doc: "/* *Deprecated: this is literal starred text. */",
        deprecated: false,
    },
    GoDocRow { name: "BlockStarSpaceBody", doc: "/*\n * Deprecated: use New.\n */", deprecated: false },
    GoDocRow {
        name: "BlockIntroStarIndented",
        doc: "/*\nBlockIntroStarIndented documents itself.\n\n *    Deprecated: use New.\n*/",
        deprecated: false,
    },
    GoDocRow { name: "BlockDoubleStarOpening", doc: "/**Deprecated: use New. */", deprecated: false },
    GoDocRow { name: "LineStar", doc: "// *Deprecated: use New.", deprecated: false },
    GoDocRow { name: "LineStarSpace", doc: "// * Deprecated: use New.", deprecated: false },
    GoDocRow { name: "BlockDoubleStarBody", doc: "/* **Deprecated: use New. */", deprecated: false },
    GoDocRow {
        name: "MidParagraph",
        doc: "// MidParagraph documents itself and\n// Deprecated: appears mid-paragraph.",
        deprecated: false,
    },
    GoDocRow {
        name: "StarredThenMarker",
        doc: "// *A literal starred paragraph.*\n//\n// Deprecated: use New.",
        deprecated: true,
    },
];

/// Every `GO_DOC_MATRIX` row, a star-only presence pair and the function,
/// method and type readers, appended to a copy of the primary Go fixture.
#[test]
fn go_doc_text_keeps_literal_stars() {
    let fixture = std::fs::read_to_string(primary_fixture(L::Go).expect("primary fixture")).expect("read fixture");
    let rows: String = GO_DOC_MATRIX
        .iter()
        .map(|row| format!("\n{}\nfunc {}() {{}}\n", row.doc, row.name))
        .collect();
    let source = format!(
        "{fixture}{rows}\n/* * */\nfunc PresenceBlockStar() {{}}\n\n// *\nfunc PresenceLineStar() {{}}\n\n\
         func PresenceNone() {{}}\n\n\
         /* *Deprecated: literal. */\ntype StarredType struct{{}}\n\n\
         /* *Deprecated: literal. */\nfunc (StarredType) StarredMethod() {{}}\n\n\
         /* Deprecated: use New. */\ntype MarkedType struct{{}}\n\n\
         /* Deprecated: use New. */\nfunc (MarkedType) MarkedMethod() {{}}\n"
    );
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("go.go");
    std::fs::write(&path, &source).expect("write snippet");
    let errors = TreeFile::new(&path).expect("parse").syntax_diagnostics();
    assert!(errors.is_empty(), "{errors:?}");

    let records = records_of(&path, &SymbolQuery::new().capabilities([Capability::Flags, Capability::Metrics]));
    let record = |name: &str| {
        records
            .iter()
            .find(|record| record.identity.name == name)
            .unwrap_or_else(|| panic!("no `{name}`"))
    };
    let mismatches: Vec<String> = GO_DOC_MATRIX
        .iter()
        .filter(|row| record(row.name).visibility.is_deprecated != row.deprecated)
        .map(|row| format!("{} (expected {})", row.name, row.deprecated))
        .collect();
    assert!(mismatches.is_empty(), "wrong is_deprecated for {mismatches:?}");

    let has_docs = |name: &str| record(name).metrics.as_ref().and_then(|metrics| metrics.has_docs);
    assert_eq!(has_docs("PresenceBlockStar"), Some(true), "`/* * */` is documentation");
    assert_eq!(has_docs("PresenceLineStar"), Some(true), "`// *` is documentation");
    assert_eq!(has_docs("PresenceNone"), Some(false), "control: no comment");

    let deprecated = |name: &str| record(name).visibility.is_deprecated;
    for name in ["PresenceBlockStar", "PresenceLineStar", "StarredType", "StarredMethod"] {
        assert!(!deprecated(name), "`{name}` is not deprecated");
    }
    for name in ["MarkedType", "MarkedMethod"] {
        assert!(deprecated(name), "control: `{name}` is deprecated");
    }
}
