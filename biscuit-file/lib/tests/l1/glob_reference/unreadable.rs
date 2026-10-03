//! A directory the search must enter but cannot read fails the search with
//! `GlobReferenceError::Io` naming it; it is never left out of a listing that
//! looks complete. A search directory that does not exist still holds no
//! matches.
//!
//! Permissions are removed with `chmod 000`, so these tests are Unix-only. A
//! privileged user reads through any mode; each test first proves the
//! directory is unreadable and returns early when it is not.

use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use biscuit_file::{
    FileReference, FileReferenceError, FileResolutionContext, GlobReferenceError, to_portable_string,
};

use super::directory_case::{absolute, assert_admits, assert_rejects};
use super::{Fixture, glob, listed};

/// Removes every permission from a directory and restores them on drop, so a
/// failing assertion still leaves a fixture the temporary directory can
/// delete.
struct Locked(PathBuf);

impl Locked {
    /// Lock `dir`, or `None` when this user can still read it (root).
    fn new(dir: &Path) -> Option<Self> {
        Self::with_mode(dir, 0o000)
    }

    /// Set `dir` to `mode`, which must deny listing, or `None` when this user
    /// can still list it (root). Mode `0o111` permits traversal only.
    fn with_mode(dir: &Path, mode: u32) -> Option<Self> {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).unwrap();
        let locked = Self(dir.to_path_buf());
        match std::fs::read_dir(dir) {
            Err(error) if error.kind() == ErrorKind::PermissionDenied => Some(locked),
            other => {
                eprintln!(
                    "skipping: {} is still readable after chmod {mode:o} ({other:?}); running as a privileged user?",
                    dir.display()
                );
                None
            }
        }
    }
}

impl Drop for Locked {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

/// `repo/docs/a.md` beside `repo/docs/locked/secret.md`.
fn locked_docs() -> (Fixture, PathBuf) {
    let fx = Fixture::new();
    fx.file("repo/docs/a.md");
    fx.file("repo/docs/locked/secret.md");
    let locked = fx.repo.join("docs/locked");
    (fx, locked)
}

/// Assert `result` is an I/O error naming `dir`.
fn assert_io<T: std::fmt::Debug>(result: Result<T, GlobReferenceError>, dir: &Path, form: &str) {
    match result {
        Err(GlobReferenceError::Io { path, source }) => {
            assert_eq!(path, dir, "`{form}` names the unreadable directory");
            assert_eq!(source.kind(), ErrorKind::PermissionDenied, "`{form}`");
        }
        other => panic!("`{form}` must fail with Io naming {}: {other:?}", dir.display()),
    }
}

fn ctx(fx: &Fixture) -> FileResolutionContext {
    fx.ctx(&fx.repo)
}

#[test]
fn an_unreadable_descendant_fails_a_recursive_listing() {
    let (fx, locked) = locked_docs();
    let Some(_guard) = Locked::new(&locked) else { return };

    assert_io(glob(&["docs/**/*.md"]).list_files(&ctx(&fx)), &locked, "docs/**/*.md");
}

#[test]
fn an_unreadable_search_directory_fails_list_files_and_take_first() {
    let (fx, locked) = locked_docs();
    let Some(_guard) = Locked::new(&locked) else { return };
    let ctx = ctx(&fx);

    assert_io(glob(&["docs/locked/*.md"]).list_files(&ctx), &locked, "list_files");
    assert_io(glob(&["docs/locked/*.md"]).take_first(&ctx), &locked, "take_first");
}

#[test]
fn every_listing_prefix_reports_the_unreadable_directory() {
    let (fx, locked) = locked_docs();
    let absolute = format!("{}/docs/locked/*.md", to_portable_string(&fx.repo));
    let Some(_guard) = Locked::new(&locked) else { return };
    let ctx = ctx(&fx);

    for form in ["docs/locked/*.md", "./docs/locked/*.md", "&docs/locked/*.md", "^docs/locked/*.md", absolute.as_str()] {
        assert_io(glob(&[form]).list_files(&ctx), &locked, form);
    }
}

#[test]
fn a_home_pattern_reports_the_unreadable_directory() {
    let fx = Fixture::new();
    fx.file("home/notes/locked/secret.md");
    let locked = fx.home.join("notes/locked");
    let Some(_guard) = Locked::new(&locked) else { return };

    assert_io(glob(&["~/notes/**/*.md"]).list_files(&ctx(&fx)), &locked, "~/notes/**/*.md");
}

/// `take_first` fails only when the unreadable directory could hold a file
/// that precedes its match: children of `docs/locked` are deeper than
/// `docs/a.md`, so the shallowest match stands, while a pattern whose only
/// candidates lie in the locked directory fails.
#[test]
fn take_first_fails_only_when_the_unreadable_directory_could_precede_its_match() {
    let (fx, locked) = locked_docs();
    let Some(_guard) = Locked::new(&locked) else { return };
    let ctx = ctx(&fx);

    assert_eq!(glob(&["docs/**/*.md"]).take_first(&ctx).unwrap(), Some(fx.repo.join("docs/a.md")));
    assert_io(glob(&["docs/**/secret.md"]).take_first(&ctx), &locked, "docs/**/secret.md");
}

#[test]
fn recursive_resolution_reports_the_unreadable_directory() {
    let (fx, locked) = locked_docs();
    let Some(_guard) = Locked::new(&locked) else { return };

    let result = FileReference::new("%secret.md").unwrap().resolve_in_context(&ctx(&fx));
    match result {
        Err(FileReferenceError::Io { path, source }) => {
            assert_eq!(path, locked);
            assert_eq!(source.kind(), ErrorKind::PermissionDenied);
        }
        other => panic!("`%secret.md` must fail with Io: {other:?}"),
    }
}

#[test]
fn a_file_symlink_into_an_unreadable_directory_fails_the_listing() {
    let (fx, locked) = locked_docs();
    let link = fx.repo.join("docs/leak.md");
    std::os::unix::fs::symlink(locked.join("secret.md"), &link).unwrap();
    let Some(_guard) = Locked::new(&locked) else { return };

    assert_io(glob(&["docs/*.md"]).list_files(&ctx(&fx)), &link, "docs/*.md");
}

/// `docs/*.md` cannot match a file inside `docs/locked`, so the unreadable
/// directory hides nothing and the listing is complete.
#[test]
fn an_unreadable_directory_deeper_than_the_pattern_reaches_is_not_an_error() {
    let (fx, locked) = locked_docs();
    let Some(_guard) = Locked::new(&locked) else { return };
    let ctx = ctx(&fx);

    assert_eq!(listed(&["docs/*.md"], &ctx), [fx.repo.join("docs/a.md")]);
    assert_eq!(listed(&["*/*.md"], &ctx), [fx.repo.join("docs/a.md")]);
}

#[test]
fn a_dangling_file_symlink_is_not_an_error() {
    let fx = Fixture::new();
    let real = fx.file("repo/docs/a.md");
    std::os::unix::fs::symlink(fx.repo.join("docs/gone.md"), fx.repo.join("docs/dangling.md")).unwrap();

    assert_eq!(listed(&["docs/*.md"], &ctx(&fx)), [real]);
}

#[test]
fn a_missing_search_directory_still_lists_nothing() {
    let (fx, locked) = locked_docs();
    let Some(_guard) = Locked::new(&locked) else { return };
    let ctx = ctx(&fx);

    assert!(listed(&["missing/*.md"], &ctx).is_empty());
    assert!(listed(&["docs/missing/**/*.md"], &ctx).is_empty());
    assert_eq!(glob(&["missing/*.md"]).take_first(&ctx).unwrap(), None);
}

#[test]
fn a_readable_tree_lists_every_file() {
    let (fx, locked) = locked_docs();
    drop(Locked::new(&locked));

    assert_eq!(
        listed(&["docs/**/*.md"], &ctx(&fx)),
        [fx.repo.join("docs/a.md"), locked.join("secret.md")]
    );
}

/// Criterion 26: a traversal-only ancestor (mode `0111`) cannot be listed,
/// and that must not approve an absolute pattern's mismatched directory
/// spelling. The correct spelling is admitted whether or not the ancestor can
/// be listed.
#[test]
fn a_traversal_only_ancestor_does_not_approve_a_mismatched_absolute_directory() {
    let fx = Fixture::new();
    let file = fx.file("repo/locked/anchor/docs/a.md");
    let locked = fx.repo.join("locked");
    let ctx = ctx(&fx);
    let wrong = absolute(&fx.repo, "locked/anchor/DOCS/*.md");
    let right = absolute(&fx.repo, "locked/anchor/docs/*.md");

    let check = |state: &str| {
        assert_rejects(&wrong, &file, &ctx);
        assert!(glob(&[&wrong]).roots(&ctx).is_empty(), "{state}: `{wrong}` exposes no root");
        assert!(!glob(&[&wrong]).matches_without_context(&file), "{state}: `{wrong}` detached");
        assert_admits(&right, &file, &ctx);
        assert!(glob(&[&right]).matches_without_context(&file), "{state}: `{right}` detached");
    };

    check("0755");
    let Some(_guard) = Locked::with_mode(&locked, 0o111) else { return };
    assert!(
        std::fs::read_dir(locked.join("anchor/docs")).is_ok(),
        "the directory below the traversal-only ancestor stays readable"
    );
    check("0111");
}
