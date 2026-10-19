//! R1 and R9: stored facts round-trip exactly, and a golden snapshot of the
//! corpus facts fails on any extraction change made without a
//! `FACTS_VERSION` bump.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dmls::clock::SystemClock;
use dmls::graph::{DocumentIndex, index_document};
use dmls::index_store::{FACTS_VERSION, FileFacts, IndexStore, UpsertReport};
use dmls::wiki::nfc;

/// The static corpus's Markdown files as `(virtual path, text)`, under the
/// fixed root `/corpus` so paths and keys are the same on every host.
fn corpus_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out.sort();
    out
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(root, &path, out);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            let relative: Vec<String> = path
                .strip_prefix(root)
                .unwrap()
                .components()
                .map(|component| nfc(&component.as_os_str().to_string_lossy()))
                .collect();
            out.push((PathBuf::from(format!("/corpus/{}", relative.join("/"))), std::fs::read_to_string(&path).unwrap()));
        }
    }
}

fn open_store(dir: &Path) -> IndexStore {
    IndexStore::open(&dir.join("store.sqlite"), Path::new("/corpus"), Arc::new(SystemClock)).unwrap().0
}

/// `index` with the fields the store does not keep cleared.
fn stored_part(mut index: DocumentIndex) -> DocumentIndex {
    index.schema_uses.clear();
    index.file_uses.clear();
    index.variable_uses.clear();
    index.frontmatter_keys.clear();
    index
}

#[test]
fn corpus_facts_round_trip_through_the_store_and_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let files = corpus_files(&biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus"));
    let indices: Vec<DocumentIndex> = files.iter().map(|(path, text)| index_document(path, text)).collect();
    let facts: Vec<FileFacts> = indices.iter().map(FileFacts::from_index).collect();
    let paths: Vec<PathBuf> = files.iter().map(|(path, _)| path.clone()).collect();

    let mut store = open_store(dir.path());
    assert_eq!(store.upsert_batch(&facts).unwrap(), UpsertReport { written: facts.len(), skipped: 0 });
    let loaded = store.load(&paths).unwrap();
    assert_eq!(loaded, facts);
    for (loaded, index) in loaded.into_iter().zip(&indices) {
        assert_eq!(loaded.into_index(), stored_part(index.clone()));
    }

    // Write what was read: every row is skipped and nothing changes.
    let reread = store.load(&paths).unwrap();
    assert_eq!(store.upsert_batch(&reread).unwrap(), UpsertReport { written: 0, skipped: facts.len() });
    assert_eq!(store.load(&paths).unwrap(), facts);

    // And again after the process lets go of the store.
    drop(store);
    let store = open_store(dir.path());
    assert_eq!(store.load(&paths).unwrap(), facts);
    let hashes = store.hashes().unwrap();
    for file in &facts {
        assert_eq!(hashes[&file.path], file.hash, "{}", file.path.display());
    }
}

#[test]
fn a_changed_file_replaces_its_row_and_drops_its_old_facts() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open_store(dir.path());
    let path = PathBuf::from("/corpus/ws/a.md");
    let before = FileFacts::from_index(&index_document(&path, "# Old\n\n[x](old.md)\n[[old]]\n\n::file old.md\n"));
    let after = FileFacts::from_index(&index_document(&path, "# New\n"));
    store.upsert_batch(&[before]).unwrap();
    assert_eq!(store.upsert_batch(std::slice::from_ref(&after)).unwrap(), UpsertReport { written: 1, skipped: 0 });
    assert_eq!(store.load(&[path]).unwrap(), vec![after]);
    assert!(store.candidates_for(Path::new("/corpus/ws/old.md")).unwrap().is_empty());
}

#[test]
fn an_empty_file_and_an_empty_heading_title_are_stored_facts() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open_store(dir.path());
    let empty = FileFacts::from_index(&index_document(Path::new("/corpus/empty.md"), ""));
    let untitled = FileFacts::from_index(&index_document(Path::new("/corpus/untitled.md"), "#\n\ntext\n"));
    assert_eq!(untitled.headings.len(), 1);
    assert_eq!(untitled.headings[0].title, "");
    store.upsert_batch(&[empty.clone(), untitled.clone()]).unwrap();
    let paths = [empty.path.clone(), untitled.path.clone()];
    assert_eq!(store.load(&paths).unwrap(), vec![empty, untitled]);
    let scanned = store.scan_headings().unwrap();
    assert_eq!(scanned.len(), 2, "a file without headings is still scanned");
    assert!(scanned[0].1.is_empty());
}

const GOLDEN: &str = include_str!("../fixtures/index_store_golden.txt");

/// The golden text: a `facts_version` header, then every corpus file's facts.
fn render(facts: &[FileFacts]) -> String {
    let mut out = format!("facts_version: {FACTS_VERSION}\n");
    for file in facts {
        out.push_str(&format!("{file:#?}\n"));
    }
    out
}

#[test]
fn corpus_facts_match_the_golden_snapshot_for_this_facts_version() {
    let facts: Vec<FileFacts> =
        corpus_files(&biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_corpus")).iter().map(|(path, text)| FileFacts::from_index(&index_document(path, text))).collect();
    let actual = render(&facts);
    let golden = GOLDEN.replace("\r\n", "\n");
    let golden_version = golden.lines().next().and_then(|line| line.strip_prefix("facts_version: "));
    let same_version = golden_version == Some(FACTS_VERSION.to_string().as_str());

    if std::env::var_os("DMLS_UPDATE_GOLDEN").is_some() && actual != golden {
        assert!(
            !same_version,
            "stored facts changed but FACTS_VERSION is still {FACTS_VERSION}: bump it in \
             dmls/src/index_store/mod.rs, then regenerate"
        );
        let path = biscuit_test_harness::manifest_dir!().join("tests/fixtures/index_store_golden.txt");
        std::fs::write(path, &actual).unwrap();
        return;
    }
    assert!(
        same_version,
        "the golden snapshot is for facts_version {golden_version:?}, this build is {FACTS_VERSION}: \
         regenerate it with DMLS_UPDATE_GOLDEN=1"
    );
    assert!(
        actual == golden,
        "stored facts changed without a FACTS_VERSION bump. Bump FACTS_VERSION in \
         dmls/src/index_store/mod.rs, then regenerate with DMLS_UPDATE_GOLDEN=1.\n--- actual ---\n{actual}"
    );
}

#[test]
fn two_spellings_path_equality_treats_as_one_name_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open_store(dir.path());
    let canonical = PathBuf::from("/corpus/ws/a.md");
    let mut spellings = vec![PathBuf::from("/corpus//ws/./a.md")];
    if cfg!(windows) {
        // A file list joins with `\`; a caller formatting a path may not.
        spellings.push(PathBuf::from(r"\corpus\ws/a.md"));
    }
    for spelling in spellings {
        assert_eq!(spelling, canonical, "PathBuf equality is the premise");
        store.upsert_batch(&[FileFacts::from_index(&index_document(&spelling, "# A\n"))]).unwrap();
        assert_eq!(store.hashes().unwrap().len(), 1, "{} made a second row", spelling.display());
        assert_eq!(store.load(std::slice::from_ref(&canonical)).unwrap().len(), 1);
        assert_eq!(store.load(&[spelling]).unwrap().len(), 1);
    }
}
