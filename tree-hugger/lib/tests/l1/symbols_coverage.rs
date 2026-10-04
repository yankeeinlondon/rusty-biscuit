//! Test 1 of `2026-10-03-visibility-and-kind`: every supported language is
//! covered by a primary symbol fixture and by rows in the kind and visibility
//! matrices (and the static matrix, where it has static members), and every
//! symbol fixture is valid input.

use tree_hugger::{ProgrammingLanguage, TreeFile};

use crate::symbols_fixtures::{all_fixtures, display_name, primary_fixture};
use crate::symbols_kinds::KIND_MATRIX;
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
    }
    for language in STATIC_LANGUAGES {
        assert!(
            STATIC_MATRIX.iter().any(|row| row.0 == *language),
            "{} has no static-matrix row",
            language.name()
        );
    }
}
