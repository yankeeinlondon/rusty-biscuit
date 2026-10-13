//! Every spelling of one file is one analysis identity: `SourceFile` reads
//! through the spelling it is given but analyzes under the file's identity,
//! so its path, records, IDs and persistent-cache key are the same for a
//! relative, absolute, `..` or symlinked spelling. The scanner lists the
//! file once, and god-files reports it relative to its root however the
//! root is spelled.

use std::path::{Path, PathBuf};

use biscuit_file::PathIdentity;
use tempfile::TempDir;
use tree_hugger::cache::{CacheLookup, CacheMiss, PersistentCache, SymbolCache, SymbolSnapshot};
use tree_hugger::god_files::GodFiles;
use tree_hugger::scanner::collect_files;
use tree_hugger::{ProgrammingLanguage, SourceFile, SymbolQuery, TreeHuggerError, file_identity};

use crate::symbols_fixtures::primary_fixture;

/// A temporary tree holding `src/alpha.rs`, a copy of the Rust fixture.
fn tree() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("src").join("alpha.rs");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("create src");
    let fixture = primary_fixture(ProgrammingLanguage::Rust).expect("Rust fixture");
    std::fs::copy(fixture, &file).expect("copy fixture");
    (dir, file)
}

/// `path` relative to the working directory, when a route exists (on
/// Windows the temporary directory can be on another drive).
fn relative_to_cwd(path: &Path) -> Option<PathBuf> {
    let cwd = file_identity(&std::env::current_dir().expect("cwd"));
    PathIdentity::new(&file_identity(path))
        .relative_from(&PathIdentity::new(&cwd))
        .map(|route| route.to_path_buf())
}

/// Every spelling of `file` this host can produce, with a label.
fn spellings(dir: &TempDir, file: &Path) -> Vec<(&'static str, PathBuf)> {
    let mut spellings = vec![
        ("as the temporary directory spells it", file.to_path_buf()),
        ("canonical absolute", file_identity(file)),
        ("with `..`", dir.path().join("src").join("..").join("src").join("alpha.rs")),
        ("with `.`", dir.path().join(".").join("src").join("alpha.rs")),
    ];
    if let Some(relative) = relative_to_cwd(file) {
        spellings.push(("relative to the working directory", relative));
    }
    #[cfg(unix)]
    {
        let file_link = dir.path().join("alpha_link.rs");
        std::os::unix::fs::symlink(file, &file_link).expect("file symlink");
        spellings.push(("symlink to the file", file_link));
        let dir_link = dir.path().join("src_link");
        std::os::unix::fs::symlink(dir.path().join("src"), &dir_link).expect("directory symlink");
        spellings.push(("through a symlinked directory", dir_link.join("alpha.rs")));
    }
    spellings
}

#[test]
fn every_spelling_reads_under_one_identity() {
    let (dir, file) = tree();
    let identity = file_identity(&file);
    assert!(identity.is_absolute(), "{identity:?}");
    assert_eq!(file_identity(&identity), identity, "an identity is its own identity");

    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
    let control = SourceFile::read(&identity).expect("read control");
    let control_key = cache.key(&control).expect("key");
    let control_index = control.query(&SymbolQuery::new()).expect("query").index;
    assert!(!control_index.symbols.is_empty(), "the fixture has records");

    for (label, spelling) in spellings(&dir, &file) {
        let source = SourceFile::read(&spelling).unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(source.path(), identity, "{label}: path");
        assert_eq!(source.language(), ProgrammingLanguage::Rust, "{label}: language");
        assert_eq!(cache.key(&source).expect("key"), control_key, "{label}: cache key");
        let index = source.query(&SymbolQuery::new()).expect("query").index;
        assert_eq!(index.file, identity, "{label}: index file");
        assert_eq!(
            serde_json::to_value(&index.symbols).expect("records"),
            serde_json::to_value(&control_index.symbols).expect("records"),
            "{label}: records and IDs"
        );
        let snapshot = SymbolSnapshot::from(index);
        assert_eq!(
            snapshot.key.stable_key(),
            SymbolSnapshot::from(control_index.clone()).key.stable_key(),
            "{label}: in-memory snapshot key"
        );
    }
}

#[test]
fn every_spelling_shares_one_persistent_cache_entry() {
    let (dir, file) = tree();
    let cache_dir = dir.path().join("cache");
    let cache = SymbolCache::new(PersistentCache::open(cache_dir.clone(), true));
    let query = SymbolQuery::new();

    for (run, (label, spelling)) in spellings(&dir, &file).into_iter().enumerate() {
        let outcome = cache
            .query(SourceFile::read(&spelling).expect("read"), &query)
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert!(outcome.write_error.is_none(), "{label}: {:?}", outcome.write_error);
        if run == 0 {
            assert!(matches!(outcome.lookup, CacheLookup::Miss(CacheMiss::Absent)), "{label}: {:?}", outcome.lookup);
        } else {
            assert!(matches!(outcome.lookup, CacheLookup::Hit), "{label}: {:?}", outcome.lookup);
        }
    }
    let entries = walk(&cache_dir).into_iter().filter(|path| path.extension().is_some_and(|ext| ext == "json")).count();
    assert_eq!(entries, 1, "one entry for the one file");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found
}

/// A file that cannot be read keeps its I/O error, naming the path as given.
#[test]
fn a_missing_file_is_reported_under_the_spelling_given() {
    let (dir, _) = tree();
    let missing = dir.path().join("src").join("..").join("src").join("missing.rs");
    match SourceFile::read(&missing) {
        Err(TreeHuggerError::Io { path, source }) => {
            assert_eq!(path, missing);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected an I/O error, got {other:?}"),
    }
}

/// Detection reads the identity: a symlink without an extension analyzes as
/// its target's language, while an unsupported name still names the
/// spelling given.
#[cfg(unix)]
#[test]
fn detection_reads_the_identity_and_errors_name_the_spelling() {
    let (dir, file) = tree();
    let link = dir.path().join("alpha_script");
    std::os::unix::fs::symlink(&file, &link).expect("symlink");
    assert_eq!(SourceFile::read(&link).expect("read").language(), ProgrammingLanguage::Rust);

    let unsupported = dir.path().join("notes.unknown");
    std::fs::write(&unsupported, "text").expect("write");
    match SourceFile::read(&unsupported) {
        Err(TreeHuggerError::UnsupportedLanguage { path }) => assert_eq!(path, unsupported),
        other => panic!("expected UnsupportedLanguage, got {other:?}"),
    }
}

/// A file named explicitly and also matched by a glob, or named under two
/// spellings, is collected once, as its identity.
#[test]
fn the_scanner_lists_each_file_once_under_its_identity() {
    let (dir, file) = tree();
    let dotted = dir.path().join("src").join("..").join("src").join("alpha.rs");
    let inputs = [
        dotted.to_str().expect("utf-8").to_string(),
        file.to_str().expect("utf-8").to_string(),
        "src/*.rs".to_string(),
    ];
    let files = collect_files(dir.path(), &inputs, &[], Some(ProgrammingLanguage::Rust)).expect("collect");
    assert_eq!(files, [file_identity(&file)]);
}

/// Scanner output is already an identity, so `read_identified` analyzes it
/// exactly as `read` does, whether the walk reached the file by a literal
/// path or a glob and however the root is spelled. A spelling that is not
/// an identity is kept verbatim (the caller's contract), and errors name
/// the path given.
#[test]
fn scanner_output_reads_identified_as_read_does() {
    let (dir, file) = tree();
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
    let identity = file_identity(&file);
    let roots = [dir.path().to_path_buf(), file_identity(dir.path())];
    for root in &roots {
        for input in ["src/alpha.rs", "src/*.rs"] {
            let label = format!("{input} under {}", root.display());
            let files = collect_files(root, &[input.to_string()], &[], None).expect("collect");
            assert_eq!(files, std::slice::from_ref(&identity), "{label}");
            let identified = SourceFile::read_identified(&files[0], None).expect("read_identified");
            let read = SourceFile::read(&file).expect("read");
            assert_eq!(identified.path(), read.path(), "{label}: path");
            assert_eq!(identified.language(), read.language(), "{label}: language");
            assert_eq!(identified.forced_language(), None, "{label}: detected language");
            assert_eq!(identified.hash(), read.hash(), "{label}: hash");
            assert_eq!(cache.key(&identified).expect("key"), cache.key(&read).expect("key"), "{label}: cache key");
            assert_eq!(
                serde_json::to_value(identified.query(&SymbolQuery::new()).expect("query").index).expect("index"),
                serde_json::to_value(read.query(&SymbolQuery::new()).expect("query").index).expect("index"),
                "{label}: records and IDs"
            );
        }
    }

    let forced = SourceFile::read_identified(&identity, Some(ProgrammingLanguage::Rust)).expect("forced");
    assert_eq!(forced.forced_language(), Some(ProgrammingLanguage::Rust));

    let missing = identity.with_file_name("missing.rs");
    match SourceFile::read_identified(&missing, None) {
        Err(TreeHuggerError::Io { path, source }) => {
            assert_eq!(path, missing);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected an I/O error, got {other:?}"),
    }
    let unsupported = identity.with_file_name("notes.unknown");
    std::fs::write(&unsupported, "text").expect("write");
    match SourceFile::read_identified(&unsupported, None) {
        Err(TreeHuggerError::UnsupportedLanguage { path }) => assert_eq!(path, unsupported),
        other => panic!("expected UnsupportedLanguage, got {other:?}"),
    }

    #[cfg(unix)]
    {
        let link = file_identity(dir.path()).join("alpha_link.rs");
        std::os::unix::fs::symlink(&file, &link).expect("symlink");
        assert_eq!(SourceFile::read_identified(&link, None).expect("read").path(), link, "used verbatim");
        assert_eq!(SourceFile::read(&link).expect("read").path(), identity, "read resolves it");
    }
}

/// God-files reports a candidate relative to its root under any spelling of
/// the root.
#[test]
fn god_files_paths_are_relative_to_any_spelling_of_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let body = "print(1)\n".repeat(tree_hugger::god_files::constants::MODERATE_MIN_SLOC);
    std::fs::write(dir.path().join("big.py"), body).expect("write");
    std::fs::create_dir(dir.path().join("sub")).expect("sub");

    for root in [dir.path().to_path_buf(), dir.path().join("sub").join(".."), file_identity(dir.path())] {
        let god_files = GodFiles::new(&root);
        let paths: Vec<&Path> = god_files.analysis().iter().map(|analysis| analysis.relative_path.as_path()).collect();
        assert_eq!(paths, [Path::new("big.py")], "root {root:?}");
    }
}
