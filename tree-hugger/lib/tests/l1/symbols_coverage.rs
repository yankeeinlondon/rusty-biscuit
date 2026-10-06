//! Every supported language is covered by a primary symbol fixture and by
//! rows in the kind, visibility, keyword and signature-text matrices (and the
//! static matrix, where it has static members), and in each report matrix
//! of the symbol filters and reports (scope, qualified names, containment,
//! async, flags, metrics, kind coverage, selection filters, generics), and every
//! symbol fixture is valid input.

use tree_hugger::{ProgrammingLanguage, SymbolKind, TreeFile};

use crate::symbols_fixtures::{
    TEST_FILE_FIXTURES, all_fixtures, display_name, multi_fixture_dir, multi_fixtures,
    primary_fixture, test_file_fixture_dir,
};
use crate::symbols_header::{KEYWORD_MATRIX, SHAPES, SIGNATURE_MATRIX};
use crate::symbols_kinds::KIND_MATRIX;
use crate::symbols_async::async_matrix_languages;
use crate::symbols_containment::containment_matrix_languages;
use crate::symbols_flags::flags_matrix_languages;
use crate::symbols_generics::generics_matrix_languages;
use crate::symbols_modifiers::modifiers_matrix_languages;
use crate::symbols_relations::relations_matrix_languages;
use crate::symbols_kind_coverage::kind_coverage_matrix_languages;
use crate::symbols_metrics::{body_lines_matrix_languages, metrics_matrix_languages};
use crate::symbols_qualified_names::qualified_name_matrix_languages;
use crate::symbols_scope::scope_matrix_languages;
use crate::symbols_selection_filters::selection_filter_matrix_languages;
use crate::symbols_static::{STATIC_LANGUAGES, STATIC_MATRIX};
use crate::symbols_visibility::VISIBILITY_MATRIX;

#[test]
fn every_language_has_a_primary_symbols_fixture() {
    let missing: Vec<&str> = ProgrammingLanguage::all()
        .into_iter()
        .filter(|language| primary_fixture(*language).is_none())
        .map(|language| language.name())
        .collect();

    assert!(
        missing.is_empty(),
        "languages without a primary tests/fixtures/symbols/<language>.<ext> fixture: {missing:?}"
    );
}

#[test]
fn every_symbols_fixture_parses_without_syntax_errors() {
    let fixtures = all_fixtures();
    // Non-vacuity: 16 primary fixtures plus the JS/TS script, CommonJS and
    // module-by-extension extras.
    assert!(fixtures.len() >= 21, "found only {} symbol fixture(s): {fixtures:?}", fixtures.len());

    for path in fixtures {
        let tree_file = TreeFile::new(&path)
            .unwrap_or_else(|error| panic!("{} failed to load: {error}", display_name(&path)));
        let errors: Vec<String> = tree_file
            .syntax_diagnostics()
            .into_iter()
            .map(|diagnostic| format!("{}:{} {}", diagnostic.range.start_line, diagnostic.range.start_column, diagnostic.message))
            .collect();
        assert!(errors.is_empty(), "{} has syntax errors: {errors:?}", display_name(&path));
    }
}

/// The `multi/` tree's contract with the multi-file, ordering and golden
/// tests: exactly one cleanly parsing file per language, at least one file in
/// a nested directory, and one type name declared in two files (the
/// `--sort name` tie-break case).
#[test]
fn multi_tree_has_one_clean_file_per_language() {
    let root = multi_fixture_dir();
    let files = multi_fixtures();
    let mut languages: Vec<ProgrammingLanguage> = Vec::new();
    for path in &files {
        let language = ProgrammingLanguage::from_path(path)
            .unwrap_or_else(|| panic!("{} maps to no supported language", path.display()));
        assert!(
            !languages.contains(&language),
            "multi/ has a second {} file: {}",
            language.name(),
            path.display()
        );
        languages.push(language);

        let tree_file = TreeFile::new(path)
            .unwrap_or_else(|error| panic!("{} failed to load: {error}", path.display()));
        let errors = tree_file.syntax_diagnostics();
        assert!(errors.is_empty(), "{} has syntax errors: {errors:?}", path.display());
    }
    let missing: Vec<&str> = ProgrammingLanguage::all()
        .into_iter()
        .filter(|language| !languages.contains(language))
        .map(|language| language.name())
        .collect();
    assert!(missing.is_empty(), "multi/ has no file for: {missing:?}");

    assert!(
        files.iter().any(|path| path.parent() != Some(root.as_path())),
        "multi/ has no file in a nested directory"
    );

    let files_declaring_widget = files
        .iter()
        .filter(|path| {
            TreeFile::new(path)
                .expect("load multi fixture")
                .symbols()
                .expect("extract symbols")
                .iter()
                .any(|symbol| symbol.name == "Widget" && symbol.kind == SymbolKind::Type)
        })
        .count();
    assert_eq!(files_declaring_widget, 2, "`Widget` must be a type in exactly two multi/ files");
}

/// A 17th language fails here until the matrices cover it.
#[test]
fn every_language_has_matrix_rows() {
    for language in ProgrammingLanguage::all() {
        assert!(
            KIND_MATRIX.iter().any(|row| row.0 == language),
            "{} has no kind-matrix row",
            language.name()
        );
        assert!(
            VISIBILITY_MATRIX.iter().any(|row| row.0 == language),
            "{} has no visibility-matrix row",
            language.name()
        );
        assert!(
            KEYWORD_MATRIX.iter().any(|row| row.0 == language),
            "{} has no keyword-matrix row",
            language.name()
        );
        for shape in SHAPES {
            assert!(
                SIGNATURE_MATRIX.iter().any(|row| row.0 == language && row.1 == *shape),
                "{} has no signature-matrix row for {shape:?} (add a case, NotApplicable or Unsupported)",
                language.name()
            );
        }
    }
    for language in STATIC_LANGUAGES {
        assert!(
            STATIC_MATRIX.iter().any(|row| row.0 == *language),
            "{} has no static-matrix row",
            language.name()
        );
    }
}

/// Exactly one row per language in every report matrix: a missing row fails
/// closed, and a repeated one would let a later phase fill the wrong row.
#[test]
fn every_language_has_one_row_in_each_report_matrix() {
    let matrices: [(&str, Vec<ProgrammingLanguage>); 12] = [
        ("SCOPE_MATRIX", scope_matrix_languages()),
        ("QUALIFIED_NAME_MATRIX", qualified_name_matrix_languages()),
        ("CONTAINMENT_MATRIX", containment_matrix_languages()),
        ("ASYNC_MATRIX", async_matrix_languages()),
        ("FLAGS_MATRIX", flags_matrix_languages()),
        ("METRICS_MATRIX", metrics_matrix_languages()),
        ("BODY_LINES_MATRIX", body_lines_matrix_languages()),
        ("KIND_COVERAGE_MATRIX", kind_coverage_matrix_languages()),
        ("SELECTION_FILTER_MATRIX", selection_filter_matrix_languages()),
        ("GENERICS_MATRIX", generics_matrix_languages()),
        ("MODIFIERS_MATRIX", modifiers_matrix_languages()),
        ("RELATIONS_MATRIX", relations_matrix_languages()),
    ];
    for (matrix, rows) in &matrices {
        for language in ProgrammingLanguage::all() {
            let count = rows.iter().filter(|row| **row == language).count();
            assert_eq!(count, 1, "{matrix} has {count} row(s) for {}", language.name());
        }
    }
}

/// Every test-file name rule has a fixture that parses cleanly and declares
/// at least a test and a helper.
#[test]
fn every_test_file_rule_has_a_clean_fixture() {
    let dir = test_file_fixture_dir();
    for (rule, relative) in TEST_FILE_FIXTURES {
        let path = dir.join(relative);
        let tree_file = TreeFile::new(&path)
            .unwrap_or_else(|error| panic!("{rule}: {} failed to load: {error}", path.display()));
        let errors = tree_file.syntax_diagnostics();
        assert!(errors.is_empty(), "{rule}: {} has syntax errors: {errors:?}", path.display());
        let functions = tree_file
            .symbols()
            .expect("extract symbols")
            .into_iter()
            .filter(|symbol| symbol.kind == SymbolKind::Function)
            .count();
        assert!(functions >= 2, "{rule}: {} declares {functions} function(s), want a test and a helper", path.display());
    }
}
