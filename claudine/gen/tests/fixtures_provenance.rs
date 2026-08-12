//! Provenance-manifest enforcement for the signals fixture corpus.
//!
//! `docs/research/signals/fixtures/provenance.yaml` is the machine-readable
//! provenance record (ruling 2026-07-06): every fixture file must carry
//! exactly one manifest entry with a `class` from the four allowed provenance
//! classes and a non-empty `source`. This test closes the bijection in both
//! directions — an unmanifested fixture and a dangling manifest entry both
//! fail loudly.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use serde::Deserialize;

/// Filenames under fixtures/ that are corpus metadata, not fixtures.
const NON_FIXTURE_FILES: [&str; 3] = ["README.md", ".gitkeep", "provenance.yaml"];

const ALLOWED_CLASSES: [&str; 4] = ["capture", "test_vector", "source_shape", "docs_example"];

/// The claudine package-area root (parent of this crate's manifest dir).
fn area() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("gen crate lives under the claudine package area")
}

#[derive(Deserialize)]
struct Manifest {
    fixtures: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    file: String,
    class: String,
    source: String,
}

/// All fixture files under `root`, as `/`-separated paths relative to `root`,
/// excluding [`NON_FIXTURE_FILES`].
fn fixture_files(root: &Path) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for dir_entry in std::fs::read_dir(&dir)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
        {
            let path = dir_entry
                .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", dir.display()))
                .path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if NON_FIXTURE_FILES.contains(&name) {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("walked path is under the fixtures root")
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            files.insert(relative);
        }
    }
    files
}

#[test]
fn provenance_manifest_matches_fixture_corpus() {
    let fixtures_root = area().join("docs/research/signals/fixtures");
    let manifest_path = fixtures_root.join("provenance.yaml");
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
    let manifest: Manifest = serde_yaml_ng::from_str(&manifest_text)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", manifest_path.display()));

    // Exactly one entry per manifest file: reject duplicates.
    let mut entry_counts: HashMap<&str, usize> = HashMap::new();
    for entry in &manifest.fixtures {
        *entry_counts.entry(entry.file.as_str()).or_default() += 1;
    }
    let duplicated: Vec<&str> = entry_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(file, _)| *file)
        .collect();
    assert!(
        duplicated.is_empty(),
        "provenance.yaml has more than one entry for these fixtures: {duplicated:?}"
    );

    // Every manifest entry's file exists on disk.
    let missing_files: Vec<&str> = manifest
        .fixtures
        .iter()
        .filter(|entry| !fixtures_root.join(&entry.file).is_file())
        .map(|entry| entry.file.as_str())
        .collect();
    assert!(
        missing_files.is_empty(),
        "provenance.yaml entries point at fixture files that do not exist under {}: {missing_files:?}",
        fixtures_root.display()
    );

    // Every fixture file on disk has a manifest entry.
    let manifested: BTreeSet<&str> = manifest
        .fixtures
        .iter()
        .map(|entry| entry.file.as_str())
        .collect();
    let unmanifested: Vec<String> = fixture_files(&fixtures_root)
        .into_iter()
        .filter(|file| !manifested.contains(file.as_str()))
        .collect();
    assert!(
        unmanifested.is_empty(),
        "fixture files with no provenance.yaml entry (every fixture requires a \
         class + source entry): {unmanifested:?}"
    );

    // Every class is one of the four allowed provenance classes.
    let bad_classes: Vec<String> = manifest
        .fixtures
        .iter()
        .filter(|entry| !ALLOWED_CLASSES.contains(&entry.class.as_str()))
        .map(|entry| format!("{} (class: {})", entry.file, entry.class))
        .collect();
    assert!(
        bad_classes.is_empty(),
        "provenance.yaml entries with a class outside {ALLOWED_CLASSES:?}: {bad_classes:?}"
    );

    // Every entry carries a non-empty source.
    let empty_sources: Vec<&str> = manifest
        .fixtures
        .iter()
        .filter(|entry| entry.source.trim().is_empty())
        .map(|entry| entry.file.as_str())
        .collect();
    assert!(
        empty_sources.is_empty(),
        "provenance.yaml entries with an empty `source`: {empty_sources:?}"
    );
}
