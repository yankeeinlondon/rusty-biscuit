//! The path-spelling rows: spaces and non-ASCII names, non-UTF-8 names
//! where the platform allows them, and case-insensitive filesystem
//! spelling differences.

use crate::Repo;

#[test]
fn spaced_and_non_ascii_paths_are_reported_once() {
    let repo = Repo::committed(&[("日本語/追跡.rs", "fn tracked() {}\n")]);
    repo.write("dir with space/file name.rs", "fn spaced() {}\n");
    repo.write("héllo-wörld.rs", "fn accented() {}\n");
    repo.write("日本語/新規.rs", "fn fresh() {}\n");
    repo.write("日本語/追跡.rs", "fn tracked() {}\nfn tracked_edit() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        crate::files_and_names(&report),
        vec![
            (
                "dir with space/file name.rs".to_string(),
                vec!["spaced".to_string()],
            ),
            ("héllo-wörld.rs".to_string(), vec!["accented".to_string()]),
            ("日本語/新規.rs".to_string(), vec!["fresh".to_string()]),
            (
                "日本語/追跡.rs".to_string(),
                vec!["tracked".to_string(), "tracked_edit".to_string()],
            ),
        ],
        "native identity matching, deterministic deduplication and ordering"
    );
    assert_eq!(report["errors"].as_array().map(Vec::len), Some(0));
}

/// A non-UTF-8 file name can exist only where the filesystem accepts it
/// (macOS rejects the bytes at the syscall level); the run is asserted
/// through `--count`, which needs no path spelling.
#[cfg(unix)]
#[test]
fn non_utf8_paths_are_selected_where_the_filesystem_allows_them() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    let name = OsStr::from_bytes(b"bad-\xff-file.rs");
    match std::fs::write(repo.dir.path().join(name), "fn raw_name() {}\n") {
        Ok(()) => {}
        #[cfg(target_os = "macos")]
        Err(error) if error.raw_os_error() == Some(92) => {
            eprintln!("non-UTF-8 worktree fixture unavailable: macOS EILSEQ; Linux exercises this case");
            return;
        }
        Err(error) => panic!("could not create non-UTF-8 fixture: {error}"),
    }

    let stdout = repo.stdout(&["symbols", "--changed", "--count"]);
    assert_eq!(stdout, "1\n", "the non-UTF-8 named changed file is selected");
}

/// Whether the host filesystem resolves names case-insensitively: a probe
/// file written one way must be visible spelled another.
fn filesystem_is_case_insensitive(scratch: &std::path::Path) -> bool {
    let written = scratch.join("CaseSpellingProbe.tmp");
    std::fs::write(&written, "probe").unwrap();
    let lower = scratch.join("casespellingprobe.tmp");
    lower.exists()
}

#[test]
fn case_insensitive_spelling_differences_resolve_to_one_identity() {
    if !filesystem_is_case_insensitive(tempfile::TempDir::new().unwrap().path()) {
        eprintln!("case-sensitive filesystem: case-only alias scenario is inapplicable");
        return;
    }

    // The committed spelling is SRC/lib.rs; the working tree renames the
    // directory to src/ (case-only) and edits the file.
    let repo = Repo::committed(&[("SRC/lib.rs", "fn one() {}\n")]);
    std::fs::rename(repo.dir.path().join("SRC"), repo.dir.path().join("src")).unwrap();
    repo.write("src/lib.rs", "fn one() {}\nfn edited() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    let files = crate::files_and_names(&report);
    assert_eq!(
        files.len(),
        1,
        "one file entry regardless of which spelling Git reported: {files:?}"
    );
    assert!(files[0].1.contains(&"edited".to_string()), "{files:?}");
}
