//! Every supported language is covered by a primary symbol fixture and by
//! rows in the kind, visibility, keyword and signature-text matrices (and the
//! static matrix, where it has static members), and every symbol fixture is
//! valid input.

use tree_hugger::{ProgrammingLanguage, SymbolKind, TreeFile};

use crate::symbols_fixtures::{
    all_fixtures, display_name, multi_fixture_dir, multi_fixtures, primary_fixture,
};
use crate::symbols_header::{KEYWORD_MATRIX, SHAPES, SIGNATURE_MATRIX};
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
