//! Common test utilities for markdown integration tests.
//!
//! Provides helper functions for loading fixtures and setting up test environments.

use std::fs;
use std::path::PathBuf;

/// Loads a markdown fixture from the `tests/fixtures/` directory.
///
/// ## Arguments
///
/// * `path` - Relative path to fixture (e.g., "valid/simple.md")
///
/// ## Returns
///
/// The fixture content as a string
///
/// ## Panics
///
/// Panics if the fixture file cannot be read or does not exist.
///
/// ## Examples
///
/// ```no_run
/// let content = load_fixture("valid/simple.md");
/// assert!(!content.is_empty());
/// ```
pub fn load_fixture(path: &str) -> String {
    let mut fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_path.push("tests");
    fixture_path.push("fixtures");
    fixture_path.push(path);

    fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read fixture at {:?}: {}",
            fixture_path.display(),
            e
        )
    })
}
