//! Filesystem capabilities a fixture needs, probed on the fixture's own
//! filesystem rather than inferred from the OS: a macOS `TMPDIR` can live on
//! a case-sensitive volume, and a Linux directory can be case-insensitive.
//!
//! Also compiled into the binary's unit tests, through `#[path]`, by
//! `src/completion/schema_completion/parity_tests.rs`.

use std::path::Path;

/// Probe whether `authored` opens the directory stored as `stored` below
/// `dir` (both relative; only their last components differ). When it does,
/// the alias must reach that very directory, which is what makes a caller's
/// mismatch checks a regression test; when it does not, those checks still
/// run and hold trivially. The outcome is printed so a run's output shows
/// whether alias coverage ran.
pub(crate) fn probe_directory_alias(dir: &Path, stored: &str, authored: &str) {
    let stored_path = dir.join(stored);
    let authored_path = dir.join(authored);
    let parent = stored_path.parent().expect("a stored directory has a parent");
    let authored_name = authored_path.file_name().expect("an authored directory name");
    let spelled = std::fs::read_dir(parent)
        .unwrap()
        .any(|entry| entry.unwrap().file_name() == authored_name);
    assert!(!spelled, "no entry is spelled `{authored}`");
    if authored_path.is_dir() {
        let reached = std::fs::canonicalize(&authored_path).unwrap();
        assert_eq!(reached, std::fs::canonicalize(&stored_path).unwrap(), "`{authored}` reaches `{stored}`");
        eprintln!("alias coverage ran: `{authored}` opens `{stored}` on this filesystem");
    } else {
        eprintln!("alias coverage not applicable: `{authored}` does not open `{stored}` on this filesystem");
    }
}
