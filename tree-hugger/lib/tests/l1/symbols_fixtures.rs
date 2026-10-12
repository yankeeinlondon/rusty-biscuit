//! Shared access to the per-language symbol fixtures in `tests/fixtures/symbols/`.
//!
//! Each supported language has one primary fixture named
//! `<query_name>.<extension>`, plus optional extras beside it (for example the
//! JavaScript classic script and CommonJS files) for cases one file cannot show.

use std::io::Write;
use std::path::{Path, PathBuf};

use biscuit_test_harness::manifest_dir;
use tree_hugger::{ProgrammingLanguage, SymbolInfo, SymbolKind, SymbolKindV2, SymbolRecord, TreeFile};

/// `record`'s in-file qualified name: its displayed `qualified_name` without
/// the default prefix (the file name plus `#`) that a query with no scan
/// root or naming context gives every name not rooted at a namespace.
pub fn in_file_name(record: &SymbolRecord) -> &str {
    let qualified = &record.identity.qualified_name;
    if record.identity.is_namespace_rooted {
        return qualified;
    }
    let file_name = record.source.file_path.file_name().expect("file name").to_string_lossy();
    qualified
        .strip_prefix(&format!("{file_name}#"))
        .unwrap_or_else(|| panic!("`{qualified}` lacks the `{file_name}#` prefix"))
}

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

/// One fixture per test-file name rule, as `(rule, path under
/// tests/fixtures/symbols/test_files/)`. Each file holds a test and a
/// non-test helper; both are test-only code because of the file name.
///
/// The Rust file reads as a test in place only because it sits under this
/// crate's `tests/`; a non-test reading needs a copy in a scratch package.
pub const TEST_FILE_FIXTURES: &[(&str, &str)] = &[
    ("Go *_test.go", "go/point_test.go"),
    ("Python test_*.py", "python/test_point.py"),
    ("Python *_test.py", "python/point_test.py"),
    ("Python conftest.py", "python/conftest.py"),
    ("JavaScript *.test.*", "javascript/point.test.js"),
    ("JavaScript *.spec.*", "javascript/point.spec.js"),
    ("TypeScript *.test.*", "typescript/point.test.ts"),
    ("TypeScript *.spec.*", "typescript/point.spec.ts"),
    ("Rust package tests/", "rust/tests/point.rs"),
];

/// The directory holding [`TEST_FILE_FIXTURES`].
pub fn test_file_fixture_dir() -> PathBuf {
    manifest_dir!().join("tests/fixtures/symbols/test_files")
}

/// The 16-language tree for multi-file, ordering, parallel and CLI golden
/// tests; one file per language, some under `nested/`.
pub fn multi_fixture_dir() -> PathBuf {
    manifest_dir!().join("tests/fixtures/symbols/multi")
}

/// Every file under [`multi_fixture_dir`], recursively, sorted by path.
pub fn multi_fixtures() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read multi fixture dir") {
            let path = entry.expect("read multi fixture entry").path();
            if path.is_dir() {
                walk(&path, found);
            } else {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&multi_fixture_dir(), &mut found);
    found.sort();
    found
}

/// The fixture's file name, for failure messages.
pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// One file's symbols through both public paths: `TreeFile::symbols()` and
/// the records of `TreeFile::symbol_index_v2()` (parse, bind and later passes).
pub struct Extracted {
    pub label: String,
    pub symbols: Vec<SymbolInfo>,
    pub records: Vec<SymbolRecord>,
}

impl Extracted {
    pub fn from_tree_file(label: String, tree_file: &TreeFile) -> Self {
        Self {
            label,
            symbols: tree_file.symbols().expect("extract symbols"),
            records: tree_file
                .symbol_index_v2()
                .expect("build symbol index")
                .symbols,
        }
    }

    /// The single v1 symbol named `name` whose container is `container`.
    pub fn symbol(&self, name: &str, container: Option<&str>) -> &SymbolInfo {
        let matches: Vec<&SymbolInfo> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.name == name && symbol.container_name.as_deref() == container)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "{}: expected one symbol `{name}` in {container:?}, found {}",
            self.label,
            matches.len()
        );
        matches[0]
    }

    /// The single v2 record named `name` whose container is `container`.
    pub fn record(&self, name: &str, container: Option<&str>) -> &SymbolRecord {
        let matches: Vec<&SymbolRecord> = self
            .records
            .iter()
            .filter(|record| {
                record.identity.name == name && record.identity.container_name.as_deref() == container
            })
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "{}: expected one record `{name}` in {container:?}, found {}",
            self.label,
            matches.len()
        );
        matches[0]
    }

    /// Asserts kind, container and container kind on both paths.
    pub fn assert_kind(
        &self,
        name: &str,
        container: Option<&str>,
        kind: SymbolKind,
        container_kind: Option<SymbolKind>,
    ) {
        let symbol = self.symbol(name, container);
        assert_eq!(symbol.kind, kind, "{}: `{name}` kind in symbols()", self.label);
        assert_eq!(
            symbol.container_kind, container_kind,
            "{}: `{name}` container kind in symbols()",
            self.label
        );
        let record = self.record(name, container);
        assert_eq!(
            record.kind,
            SymbolKindV2::from(kind),
            "{}: `{name}` kind in symbol_index_v2()",
            self.label
        );
        assert_eq!(
            record.identity.container_kind,
            container_kind.map(SymbolKindV2::from),
            "{}: `{name}` container kind in symbol_index_v2()",
            self.label
        );
    }
}

/// Extracts the primary fixture of `language`.
pub fn extract_primary(language: ProgrammingLanguage) -> Extracted {
    let path = primary_fixture(language)
        .unwrap_or_else(|| panic!("no primary symbols fixture for {}", language.name()));
    extract_path(&path)
}

/// Extracts the fixture at `path`.
pub fn extract_path(path: &Path) -> Extracted {
    let tree_file = TreeFile::new(path).expect("parse fixture");
    Extracted::from_tree_file(display_name(path), &tree_file)
}

/// Extracts `source` written to a temporary file with `extension`, for a
/// construct the shared fixtures cannot hold without changing other rows.
pub fn extract_inline(source: &str, extension: &str) -> Extracted {
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .expect("create temp source");
    file.write_all(source.as_bytes()).expect("write temp source");
    let tree_file = TreeFile::new(file.path()).expect("parse inline source");
    Extracted::from_tree_file(format!("inline .{extension}"), &tree_file)
}

/// Every primary/extra fixture for a language, optionally including its multi-tree file.
pub fn fixtures_for(language: ProgrammingLanguage, include_multi: bool) -> Vec<PathBuf> {
    let fixtures = all_fixtures().into_iter().chain(if include_multi { multi_fixtures() } else { Vec::new() });
    let selected: Vec<_> = fixtures.filter(|path| ProgrammingLanguage::from_path(path) == Some(language)).collect();
    assert!(!selected.is_empty(), "{language}: no fixtures");
    selected
}
