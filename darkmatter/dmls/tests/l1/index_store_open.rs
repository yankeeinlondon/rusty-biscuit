//! The Input Robustness Matrix for the store: every shape of store file at
//! open, every `DMLS_CACHE_DIR` value, and one corrupted cell per table
//! through the shared row decode. Plus store keys, file names, and Unix
//! modes.
//!
//! Default outcome: an unusable store is a typed failure that ends in memory
//! facts, never an empty result. Exceptions: a zero-byte file is a new store,
//! and a root mismatch refills rather than warning about corruption.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dmls::clock::SystemClock;
use dmls::graph::index_document;
use dmls::index_store::{
    FACTS_VERSION, FileFacts, IndexStore, OpenReport, RootFacts, RootOpened, StoreFacts, StoreFailure,
    StoreLocation, root_key, store_path,
};

struct Fixture {
    _dir: tempfile::TempDir,
    base: PathBuf,
    root: PathBuf,
    path: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let base = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let root = base.join("ws");
    std::fs::create_dir(&root).unwrap();
    let path = store_path(&base, &root);
    Fixture { _dir: dir, base, root, path }
}

fn open(fixture: &Fixture) -> Result<(IndexStore, OpenReport), StoreFailure> {
    IndexStore::open(&fixture.path, &fixture.root, Arc::new(SystemClock))
}

fn facts(path: &Path, source: &str) -> FileFacts {
    FileFacts::from_index(&index_document(path, source))
}

/// A store holding one file with a heading, a link, a wiki link (with a
/// heading), and a transclusion, so every table has a row.
fn filled(fixture: &Fixture) -> PathBuf {
    let file = fixture.root.join("a.md");
    let (mut store, _) = open(fixture).unwrap();
    store
        .upsert_batch(&[facts(&file, "# A\n\n[b](b.md)\n[[b#Top]]\n\n::file c.md\n")])
        .unwrap();
    file
}

/// Runs `sql` on the store file through a separate connection.
fn tamper(fixture: &Fixture, sql: &str) {
    let conn = rusqlite::Connection::open(&fixture.path).unwrap();
    conn.execute_batch(sql).unwrap();
}

fn assert_corrupt<T: std::fmt::Debug>(result: Result<T, StoreFailure>, what: &str) {
    assert!(matches!(result, Err(StoreFailure::Corrupt(_))), "{what}: {result:?}");
}

#[test]
fn control_store_opens_created_then_reopens_with_its_rows() {
    let fixture = fixture();
    let (_, report) = open(&fixture).unwrap();
    assert_eq!(report, OpenReport { created: true, rows: 0 });
    filled(&fixture);
    let (store, report) = open(&fixture).unwrap();
    assert_eq!(report, OpenReport { created: false, rows: 1 });
    assert_eq!(store.load(&[fixture.root.join("a.md")]).unwrap().len(), 1);
}

#[test]
fn zero_byte_file_is_a_new_store() {
    let fixture = fixture();
    std::fs::write(&fixture.path, b"").unwrap();
    assert_eq!(open(&fixture).unwrap().1, OpenReport { created: true, rows: 0 });
}

#[test]
fn garbage_and_truncated_files_are_corrupt_and_recreated_once() {
    let fixture = fixture();
    std::fs::write(&fixture.path, b"not a database at all, but long enough to have a header").unwrap();
    assert_corrupt(open(&fixture), "garbage");
    let (store, opened) = StoreFacts::open(&fixture.path, &fixture.root, Arc::new(SystemClock)).unwrap();
    assert!(matches!(opened.recreated, Some(StoreFailure::Corrupt(_))), "{opened:?}");
    assert!(opened.report.created);
    drop(store);

    // Fill enough rows that the tables' pages lie past the first page, then
    // cut the file to that page.
    let (mut store, _) = open(&fixture).unwrap();
    let many: Vec<FileFacts> = (0..300)
        .map(|n| facts(&fixture.root.join(format!("{n}.md")), &format!("# H{n}\n[x](y{n}.md)\n")))
        .collect();
    store.upsert_batch(&many).unwrap();
    drop(store);
    std::fs::OpenOptions::new().write(true).open(&fixture.path).unwrap().set_len(4096).unwrap();
    assert_corrupt(open(&fixture), "truncated");
}

#[test]
fn a_missing_table_is_corrupt() {
    let fixture = fixture();
    filled(&fixture);
    tamper(&fixture, "DROP TABLE wiki;");
    let result = open(&fixture);
    assert!(matches!(&result, Err(StoreFailure::Corrupt(message)) if message.contains("wiki")), "{result:?}");
}

#[test]
fn a_format_or_facts_stamp_other_than_this_build_is_corrupt() {
    for key in ["format", "facts_version"] {
        let fixture = fixture();
        filled(&fixture);
        tamper(&fixture, &format!("UPDATE meta SET value = '99' WHERE key = '{key}';"));
        assert_corrupt(open(&fixture), key);
        tamper(&fixture, &format!("DELETE FROM meta WHERE key = '{key}';"));
        assert_corrupt(open(&fixture), &format!("missing {key}"));
    }
}

#[test]
fn a_store_for_another_root_is_a_root_mismatch_and_refills() {
    let fixture = fixture();
    filled(&fixture);
    let other = fixture.base.join("other");
    let result = IndexStore::open(&fixture.path, &other, Arc::new(SystemClock));
    assert!(
        matches!(&result, Err(StoreFailure::RootMismatch { stored }) if stored == fixture.root.to_str().unwrap()),
        "{result:?}"
    );
    let (store, opened) = StoreFacts::open(&fixture.path, &other, Arc::new(SystemClock)).unwrap();
    assert!(matches!(opened.recreated, Some(StoreFailure::RootMismatch { .. })));
    assert_eq!(opened.report.rows, 0);
    assert!(store.store().hashes().unwrap().is_empty());
}

#[test]
fn strict_tables_refuse_a_wrongly_typed_value() {
    let fixture = fixture();
    filled(&fixture);
    let conn = rusqlite::Connection::open(&fixture.path).unwrap();
    let error = conn.execute("UPDATE headings SET level = 'one'", []).unwrap_err();
    assert!(error.to_string().contains("cannot store TEXT value in INTEGER column"), "{error}");
}

/// One corrupted cell per table, each read through the call that decodes it.
#[test]
fn a_corrupted_cell_in_any_table_is_corruption_never_an_empty_result() {
    let cases: [(&str, &str); 7] = [
        ("files.path", "UPDATE files SET path = CAST(x'ff' AS TEXT);"),
        ("headings.level", "UPDATE headings SET level = 9;"),
        ("links.span", "UPDATE links SET span_start = span_end + 1;"),
        ("wiki.unsupported", "UPDATE wiki SET unsupported = 2;"),
        ("wiki.heading_span", "UPDATE wiki SET heading_end = NULL;"),
        ("transclusions.line", "UPDATE transclusions SET line = -1;"),
        ("cand.key", "UPDATE cand SET key = CAST(x'ff' AS TEXT) WHERE rowid = (SELECT min(rowid) FROM cand);"),
    ];
    for (cell, sql) in cases {
        let fixture = fixture();
        let file = filled(&fixture);
        let (control, _) = open(&fixture).unwrap();
        assert_eq!(control.load(std::slice::from_ref(&file)).unwrap().len(), 1, "{cell} control");
        drop(control);
        tamper(&fixture, sql);
        let (store, _) = open(&fixture).unwrap();
        if cell == "files.path" {
            assert_corrupt(store.hashes(), cell);
            assert_corrupt(store.candidates_for(&fixture.root.join("b.md")), cell);
            continue;
        }
        assert_corrupt(store.load(std::slice::from_ref(&file)), cell);
        if cell == "headings.level" {
            assert_corrupt(store.scan_headings(), cell);
        }
    }
}

#[test]
fn a_corrupted_cell_read_through_root_facts_falls_back_to_memory() {
    let fixture = fixture();
    let file = filled(&fixture);
    tamper(&fixture, "UPDATE headings SET level = 9;");
    let location = StoreLocation::Directory(fixture.base.clone());
    let (mut facts_source, opened) = RootFacts::open(&fixture.root, &location, Arc::new(SystemClock));
    assert!(matches!(opened, RootOpened::Store(_)));
    assert!(facts_source.load(std::slice::from_ref(&file)).is_empty());
    assert!(facts_source.in_memory());
    assert!(facts_source.take_switched(), "the caller learns it must revalidate");
}

#[test]
fn cache_dir_override_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let base = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let cache = base.join("cache");
    let default = cache.join("darkmatter").join("dmls");

    for unset in [None, Some(OsStr::new(""))] {
        assert_eq!(StoreLocation::prepare(unset, Some(cache.clone())), StoreLocation::Directory(default.clone()));
        assert!(default.is_dir());
        assert!(matches!(StoreLocation::prepare(unset, None), StoreLocation::Unavailable(_)));
    }
    for off in ["off", "OFF", "Off"] {
        assert_eq!(StoreLocation::prepare(Some(OsStr::new(off)), Some(cache.clone())), StoreLocation::Disabled);
    }
    let missing = base.join("override/deep");
    assert_eq!(StoreLocation::prepare(Some(missing.as_os_str()), None), StoreLocation::Directory(missing.clone()));
    assert!(missing.is_dir());
    assert_eq!(StoreLocation::prepare(Some(missing.as_os_str()), None), StoreLocation::Directory(missing.clone()));

    let relative = StoreLocation::prepare(Some(OsStr::new("relative/cache")), Some(cache.clone()));
    assert!(matches!(&relative, StoreLocation::Unavailable(reason) if reason.contains("not an absolute path")), "{relative:?}");
    let file = base.join("a-file");
    std::fs::write(&file, "").unwrap();
    let named_file = StoreLocation::prepare(Some(file.as_os_str()), None);
    assert!(matches!(&named_file, StoreLocation::Unavailable(reason) if reason.contains("not a directory")), "{named_file:?}");
    let under_file = StoreLocation::prepare(Some(file.join("sub").as_os_str()), None);
    assert!(matches!(&under_file, StoreLocation::Unavailable(reason) if reason.contains("cannot create")), "{under_file:?}");
}

#[test]
fn store_key_is_stable_sixteen_hex_digits_and_names_the_facts_version() {
    let fixture = fixture();
    let key = root_key(&fixture.root);
    assert_eq!(key.len(), 16);
    assert!(key.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(root_key(&fixture.root), key);
    assert_eq!(root_key(&fixture.root.join(".").join("..").join("ws")), key, "keyed on the canonical path");
    assert_ne!(root_key(&fixture.base), key);
    assert_eq!(fixture.path, fixture.base.join(format!("{key}-f{FACTS_VERSION}.sqlite")));
}

/// macOS's default volume is case-insensitive and canonicalization returns
/// the on-disk case, so two spellings of one root share a store.
#[cfg(target_os = "macos")]
#[test]
fn macos_canonicalization_folds_a_differently_cased_root_to_one_key() {
    let fixture = fixture();
    let mixed = fixture.base.join("MixedCase");
    std::fs::create_dir(&mixed).unwrap();
    assert_eq!(root_key(&fixture.base.join("mixedcase")), root_key(&mixed));
    assert_eq!(root_key(&fixture.base.join("MIXEDCASE")), root_key(&mixed));
}

#[cfg(unix)]
#[test]
fn store_files_are_private_and_a_created_directory_is_0700() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    let dir = tempfile::tempdir().unwrap();
    let base = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let StoreLocation::Directory(store_dir) = StoreLocation::prepare(Some(base.join("cache").as_os_str()), None) else {
        panic!("override directory not created");
    };
    assert_eq!(mode(&store_dir), 0o700);
    let root = base.join("ws");
    std::fs::create_dir(&root).unwrap();
    let path = store_path(&store_dir, &root);
    let (mut store, _) = IndexStore::open(&path, &root, Arc::new(SystemClock)).unwrap();
    store.upsert_batch(&[facts(&root.join("a.md"), "# A\n")]).unwrap();
    for suffix in ["", "-wal", "-shm", ".lock"] {
        let file = PathBuf::from(format!("{}{suffix}", path.display()));
        assert_eq!(mode(&file), 0o600, "{suffix}");
    }
}
