//! Shared access to the per-language symbol fixtures in `tests/fixtures/symbols/`.
//!
//! Each supported language has one primary fixture named
//! `<query_name>.<extension>`, plus optional extras beside it (for example the
//! JavaScript classic script and CommonJS files) for cases one file cannot show.

use std::path::{Path, PathBuf};

use biscuit_test_harness::manifest_dir;
use tree_hugger::ProgrammingLanguage;

/// The directory holding every symbol fixture.
pub fn symbols_fixture_dir() -> PathBuf {
    manifest_dir!().join("tests/fixtures/symbols")
}

/// The primary fixture for `language`, if one exists under any of its extensions.
pub fn primary_fixture(language: ProgrammingLanguage) -> Option<PathBuf> {
    let dir = symbols_fixture_dir();
    language
        .extensions()
        .iter()
        .map(|extension| dir.join(format!("{}.{extension}", language.query_name())))
        .find(|path| path.is_file())
}

/// Every fixture file whose extension maps to a supported language, sorted by path.
pub fn all_fixtures() -> Vec<PathBuf> {
    let mut fixtures: Vec<PathBuf> = std::fs::read_dir(symbols_fixture_dir())
        .expect("read tests/fixtures/symbols")
        .map(|entry| entry.expect("read fixture entry").path())
        .filter(|path| path.is_file() && ProgrammingLanguage::from_path(path).is_some())
        .collect();
    fixtures.sort();
    fixtures
}

/// The fixture's file name, for failure messages.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
