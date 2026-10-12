//! The bundled file adapter against real directories: every accepted and
//! rejected path form of the spec's path table, inside a repository and
//! outside one, plus the `FileChanged` lifecycle on disk.
//!
//! Every adapter is given its tree root explicitly, so no test depends on
//! the process's current directory and the tests stay parallel-safe.

#![cfg(feature = "file-adapter")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::NaiveDate;
use content_policy::{
    DiagnosticCode, EntryOutcome, EvaluationContext, FileAdapter, FileObservation, FileProvider,
    FileRequest, FingerprintScheme, Location, RenewalContext, RenewalError, Status,
    UnknownReason, apply_renewal, evaluate_document, plan_renewal,
};
use tempfile::TempDir;

fn observe(adapter: &FileAdapter, path: &str, base_dir: &Path) -> FileObservation {
    adapter.observe(&FileRequest { path, base_dir })
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn assert_present(observation: FileObservation, content: &str, what: &str) {
    assert_eq!(observation, FileObservation::Present(content.as_bytes().to_vec()), "{what}");
}

fn assert_outside(observation: FileObservation, what: &str) {
    assert!(matches!(observation, FileObservation::OutsideBoundary(_)), "{what}: {observation:?}");
}

/// A repository with a package area `area`, a package `area/pkg`, and a
/// document directory `area/pkg/docs`; a sibling directory `outside` holds a
/// file beyond the repository.
struct Repository {
    _temp: TempDir,
    root: PathBuf,
    docs: PathBuf,
    outside: PathBuf,
}

fn repository() -> Repository {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("repo");
    fs::create_dir_all(&root).unwrap();
    gix::init(&root).expect("git init");
    write(&root.join("README.md"), "repository readme");
    write(&root.join("config.rs"), "root config");
    write(&root.join("area/README.md"), "area readme");
    write(&root.join("area/pkg/Cargo.toml"), "[package]\n");
    write(&root.join("area/pkg/README.md"), "package readme");
    write(&root.join("area/pkg/src/lib.rs"), "pub fn lib() {}\n");
    write(&root.join("area/pkg/docs/note.md"), "note");
    fs::create_dir_all(root.join("area/pkg/docs/sub")).unwrap();
    let outside = temp.path().join("outside");
    write(&outside.join("secret.md"), "secret");
    let docs = root.join("area/pkg/docs");
    Repository {
        _temp: temp,
        root,
        docs,
        outside,
    }
}

impl Repository {
    /// An adapter whose tree root lies elsewhere, proving the boundary inside
    /// a repository never depends on where the command started.
    fn adapter(&self) -> FileAdapter {
        FileAdapter::new().with_tree_root(self.outside.clone())
    }
}

// --- Inside a repository ------------------------------------------------------

#[test]
fn relative_paths_resolve_from_the_base_directory_inside_the_repository() {
    let repo = repository();
    let adapter = repo.adapter();
    assert_present(observe(&adapter, "note.md", &repo.docs), "note", "implicit relative");
    assert_present(observe(&adapter, "./note.md", &repo.docs), "note", "explicit relative");
    assert_present(observe(&adapter, "../src/lib.rs", &repo.docs), "pub fn lib() {}\n", "`../` inside");
    assert_present(observe(&adapter, "../../../config.rs", &repo.docs), "root config", "`../` up to the root");
}

#[test]
fn a_parent_path_escaping_the_repository_is_outside_the_boundary() {
    let repo = repository();
    let adapter = repo.adapter();
    assert_outside(observe(&adapter, "../../../../outside/secret.md", &repo.docs), "existing target");
    assert_outside(observe(&adapter, "../../../../../elsewhere.rs", &repo.docs), "missing target");
}

/// A bare path is looked up at the base directory only; Biscuit File's
/// repository-root retry would silently watch a different file.
#[test]
fn a_bare_path_that_exists_only_at_the_repository_root_is_not_found() {
    let repo = repository();
    assert!(repo.root.join("config.rs").is_file());
    assert_eq!(observe(&repo.adapter(), "config.rs", &repo.docs), FileObservation::Missing);
}

#[test]
fn repository_sigils_resolve_inside_the_repository() {
    let repo = repository();
    let adapter = repo.adapter();
    assert_present(observe(&adapter, "&README.md", &repo.docs), "repository readme", "`&`");
    assert_present(observe(&adapter, "&/README.md", &repo.docs), "repository readme", "`&/`");
    assert_present(observe(&adapter, "&area/pkg/docs/note.md", &repo.docs), "note", "`&` nested");
    assert_outside(observe(&adapter, "&../outside/secret.md", &repo.docs), "`&` escape");
    assert_outside(observe(&adapter, "^../outside/secret.md", &repo.docs), "`^` escape");

    // `^` searches the package, then the package area, then the repository root.
    assert_present(observe(&adapter, "^README.md", &repo.docs), "package readme", "`^` package");
    fs::remove_file(repo.root.join("area/pkg/README.md")).unwrap();
    assert_present(observe(&adapter, "^README.md", &repo.docs), "area readme", "`^` package area");
    fs::remove_file(repo.root.join("area/README.md")).unwrap();
    assert_present(observe(&adapter, "^README.md", &repo.docs), "repository readme", "`^` root");
    assert_eq!(observe(&adapter, "^missing.md", &repo.docs), FileObservation::Missing);
}

#[test]
fn a_directory_is_not_a_file_and_a_missing_file_is_missing() {
    let repo = repository();
    let adapter = repo.adapter();
    assert_eq!(observe(&adapter, "sub", &repo.docs), FileObservation::NotAFile);
    assert_eq!(observe(&adapter, "./sub/", &repo.docs), FileObservation::NotAFile);
    assert_eq!(observe(&adapter, "&area", &repo.docs), FileObservation::NotAFile);
    assert_eq!(observe(&adapter, "gone.md", &repo.docs), FileObservation::Missing);
    assert_eq!(observe(&adapter, "gone/deeper.md", &repo.docs), FileObservation::Missing);
}

#[cfg(unix)]
#[test]
fn symlinks_are_followed_and_checked_against_the_boundary() {
    use std::os::unix::fs::symlink;

    let repo = repository();
    let adapter = repo.adapter();
    symlink(repo.outside.join("secret.md"), repo.docs.join("escape.md")).unwrap();
    assert_outside(observe(&adapter, "escape.md", &repo.docs), "symlink out of the repository");
    symlink(repo.root.join("config.rs"), repo.docs.join("inner.rs")).unwrap();
    assert_present(observe(&adapter, "inner.rs", &repo.docs), "root config", "symlink inside");
    symlink(repo.docs.join("gone.md"), repo.docs.join("dangling.md")).unwrap();
    assert_eq!(observe(&adapter, "dangling.md", &repo.docs), FileObservation::Missing, "broken symlink");
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_is_unreadable() {
    use std::os::unix::fs::PermissionsExt;

    let repo = repository();
    let path = repo.docs.join("locked.md");
    write(&path, "locked");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&path).is_ok() {
        // Running as root: permission bits do not stop the read.
        eprintln!("skipped: the process can read a mode-000 file");
        return;
    }
    let observation = observe(&repo.adapter(), "locked.md", &repo.docs);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    match observation {
        FileObservation::Unreadable(reason) => assert!(!reason.is_empty()),
        other => panic!("expected unreadable, got {other:?}"),
    }
}

// --- Outside a repository -------------------------------------------------------

/// `<temp>/writing/notes/doc.md` beside `<temp>/writing/drafts/x.md`, with no
/// repository anywhere above.
struct Tree {
    _temp: TempDir,
    writing: PathBuf,
    notes: PathBuf,
}

fn tree() -> Tree {
    let temp = TempDir::new().unwrap();
    assert!(
        biscuit_file::find_git_root(temp.path()).unwrap().is_none(),
        "the temporary directory must not be inside a repository"
    );
    let writing = temp.path().join("writing");
    write(&writing.join("README.md"), "tree readme");
    write(&writing.join("drafts/x.md"), "draft");
    write(&writing.join("notes/doc.md"), "---\n---\n");
    let notes = writing.join("notes");
    Tree {
        _temp: temp,
        writing,
        notes,
    }
}

/// The spec's two-directory example: the same rule is valid when checked
/// from `~/writing` and invalid when checked from `~/writing/notes`.
#[test]
fn outside_a_repository_the_boundary_is_the_starting_directory() {
    let tree = tree();
    // `policy check notes/doc.md` run from `writing`: the base is `notes`.
    let from_writing = FileAdapter::new().with_tree_root(tree.writing.clone());
    assert_present(observe(&from_writing, "../drafts/x.md", Path::new("notes")), "draft", "from writing");
    // `policy check doc.md` run from `writing/notes`: the base is `.`.
    let from_notes = FileAdapter::new().with_tree_root(tree.notes.clone());
    assert_outside(observe(&from_notes, "../drafts/x.md", Path::new(".")), "from notes");
    // An absolute base directory resolves the same way.
    assert_present(observe(&from_writing, "../drafts/x.md", &tree.notes), "draft", "absolute base");
    assert_outside(observe(&from_writing, "../../elsewhere.md", &tree.notes), "escape");
}

#[test]
fn outside_a_repository_both_sigils_resolve_from_the_tree_root() {
    let tree = tree();
    let adapter = FileAdapter::new().with_tree_root(tree.writing.clone());
    assert_present(observe(&adapter, "&README.md", &tree.notes), "tree readme", "`&`");
    assert_present(observe(&adapter, "^README.md", &tree.notes), "tree readme", "`^` falls back");
    assert_present(observe(&adapter, "&drafts/x.md", &tree.notes), "draft", "`&` nested");
    assert_outside(observe(&adapter, "&../escape.md", &tree.notes), "`&` escape");
    // From a deeper tree root, the sigils name a different file.
    let deeper = FileAdapter::new().with_tree_root(tree.notes.clone());
    assert_eq!(observe(&deeper, "&README.md", &tree.notes), FileObservation::Missing);
}

#[test]
fn a_base_directory_outside_the_tree_root_admits_nothing() {
    let tree = tree();
    let adapter = FileAdapter::new().with_tree_root(tree.notes.clone());
    assert_outside(observe(&adapter, "README.md", &tree.writing), "base above the tree root");
}

#[test]
fn a_missing_base_directory_is_unreadable() {
    let tree = tree();
    let adapter = FileAdapter::new().with_tree_root(tree.writing.clone());
    assert!(matches!(observe(&adapter, "x.md", Path::new("no-such-dir")), FileObservation::Unreadable(_)));
}

// --- Through evaluation and renewal (AC 8) ---------------------------------------

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()
}

fn check(document: &Path, adapter: &FileAdapter) -> content_policy::Report {
    let bytes = fs::read(document).unwrap();
    let context = EvaluationContext::new(today().and_hms_opt(0, 0, 0).unwrap().and_utc())
        .with_files(Arc::new(adapter.clone()), document.parent().unwrap());
    evaluate_document(&bytes, &context).unwrap_or_else(|error| panic!("{error}"))
}

fn renew(document: &Path, adapter: &FileAdapter) -> Result<(), RenewalError> {
    let bytes = fs::read(document).unwrap();
    let context = RenewalContext::new(today()).with_files(Arc::new(adapter.clone()), document.parent().unwrap());
    let plan = plan_renewal(&bytes, &context)?;
    apply_renewal(document, &plan)
}

/// Capture, match, edit the watched file, triggered, renew, fresh; then
/// delete it: triggered ("source removed"), and renewal writes nothing.
#[test]
fn the_file_changed_lifecycle_on_disk() {
    let repo = repository();
    let adapter = repo.adapter();
    let document = repo.docs.join("guide.md");
    let body = "---\ncontent_policy:\n  - FileChanged(../src/lib.rs, @lib_fingerprint)\n---\n\n# Guide\n";
    write(&document, body);
    let watched = repo.root.join("area/pkg/src/lib.rs");

    let report = check(&document, &adapter);
    assert_eq!(report.results[0].outcome, EntryOutcome::Unknown(UnknownReason::MissingBaseline));

    renew(&document, &adapter).unwrap();
    let captured = FingerprintScheme::Blake3Lf.fingerprint(&fs::read(&watched).unwrap());
    let renewed = fs::read_to_string(&document).unwrap();
    assert_eq!(
        renewed,
        body.replace("---\n\n# Guide", &format!("lib_fingerprint: {captured}\n---\n\n# Guide"))
    );
    assert_eq!(check(&document, &adapter).status, Status::Fresh);

    fs::write(&watched, "pub fn lib() { changed() }\n").unwrap();
    let report = check(&document, &adapter);
    assert_eq!((report.status, report.results[0].outcome), (Status::Stale, EntryOutcome::Triggered));
    assert_eq!(report.results[0].reason, "Watched file changed");

    renew(&document, &adapter).unwrap();
    assert_eq!(check(&document, &adapter).status, Status::Fresh);
    let recaptured = FingerprintScheme::Blake3Lf.fingerprint(b"pub fn lib() { changed() }\n");
    assert!(fs::read_to_string(&document).unwrap().contains(&format!("lib_fingerprint: {recaptured}\n")));

    fs::remove_file(&watched).unwrap();
    let report = check(&document, &adapter);
    assert_eq!((report.status, report.results[0].reason.as_str()), (Status::Stale, "Source removed"));
    let before = fs::read(&document).unwrap();
    assert!(matches!(renew(&document, &adapter), Err(RenewalError::MissingEvidence { .. })));
    assert_eq!(fs::read(&document).unwrap(), before, "nothing is written");
}

#[test]
fn a_document_watching_outside_its_repository_gets_no_verdict() {
    let repo = repository();
    let document = repo.docs.join("leak.md");
    write(&document, "---\ncontent_policy:\n  - FileChanged(../../../../outside/secret.md, @fp)\n---\n");
    let bytes = fs::read(&document).unwrap();
    let context = EvaluationContext::new(today().and_hms_opt(0, 0, 0).unwrap().and_utc())
        .with_files(Arc::new(repo.adapter()), &repo.docs);
    let Err(content_policy::DocumentError::Invalid(invalid)) = evaluate_document(&bytes, &context) else {
        panic!("expected no verdict");
    };
    assert_eq!(invalid.diagnostics[0].code, DiagnosticCode::OutsideBoundary);
    assert_eq!(invalid.diagnostics[0].location, Location::Entry { index: 0 });
    // Renewal validates the same way and writes nothing.
    assert!(matches!(renew(&document, &repo.adapter()), Err(RenewalError::Invalid(_))));
}
