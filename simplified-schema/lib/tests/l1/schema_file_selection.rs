//! `select_schema_file`: the one rule that picks the file a `$schema`,
//! named-type import, or `example(...)` reference names, with the ordered
//! locations it probed and the path a missing reference would be created at.
//!
//! Each fixture is a repository `{tmp}/repo` (with `.git`) whose document
//! lives in `repo/docs/`, and two schema roots outside it, `{tmp}/first` and
//! `{tmp}/second`, created only when a case needs them.

#![cfg(feature = "file")]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReferenceError, FileResolutionContext, canonicalize_simplified};
use simplified_schema::SchemaError;
use simplified_schema::resolve::{SchemaFileSelection, SelectionFailure, SelectionOutcome, select_schema_file};

struct Fixture {
    _tmp: tempfile::TempDir,
    base: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let base = canonicalize_simplified(tmp.path()).unwrap();
        fs::create_dir_all(base.join("repo/.git")).unwrap();
        fs::create_dir_all(base.join("repo/docs")).unwrap();
        Self { _tmp: tmp, base }
    }

    fn repo(&self) -> PathBuf {
        self.base.join("repo")
    }

    fn docs(&self) -> PathBuf {
        self.repo().join("docs")
    }

    /// A schema root folder, created.
    fn root(&self, name: &str) -> PathBuf {
        let path = self.base.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write(&self, path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "$schema:\n  name: string\n").unwrap();
    }

    /// The document's context: no home and no environment, so nothing is
    /// read from the test process.
    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.docs(), None, HashMap::new()).with_repository_root(self.repo())
    }

    fn select(&self, reference: &str, roots: &[PathBuf]) -> SchemaFileSelection {
        select_schema_file(reference, &self.docs(), roots, &self.context())
    }
}

fn found(selection: &SchemaFileSelection) -> &Path {
    match &selection.outcome {
        SelectionOutcome::Found(path) => path,
        other => panic!("expected Found, got {other:?}"),
    }
}

fn creation(selection: &SchemaFileSelection) -> Option<&Path> {
    match &selection.outcome {
        SelectionOutcome::Missing { creation, .. } => creation.as_deref(),
        SelectionOutcome::SiblingOnly { creation, .. } => Some(creation),
        other => panic!("expected a miss, got {other:?}"),
    }
}

#[test]
fn a_bare_name_is_selected_from_the_first_root_that_holds_it() {
    let fixture = Fixture::new();
    let (first, second) = (fixture.root("first"), fixture.root("second"));
    let roots = [first.clone(), second.clone()];

    // Only in the first root.
    fixture.write(&first.join("only-first.yaml"));
    let selection = fixture.select("only-first.yaml", &roots);
    assert_eq!(found(&selection), first.join("only-first.yaml"));
    assert_eq!(selection.attempted, [first.join("only-first.yaml")]);

    // Only in the second root: the first root's miss is recorded before it.
    fixture.write(&second.join("only-second.yaml"));
    let selection = fixture.select("only-second.yaml", &roots);
    assert_eq!(found(&selection), second.join("only-second.yaml"));
    assert_eq!(selection.attempted, [first.join("only-second.yaml"), second.join("only-second.yaml")]);

    // In both: the first wins, and the second is never probed.
    fixture.write(&first.join("both.yaml"));
    fixture.write(&second.join("both.yaml"));
    let selection = fixture.select("  both.yaml  ", &roots);
    assert_eq!(selection.reference, "both.yaml");
    assert_eq!(found(&selection), first.join("both.yaml"));
    assert_eq!(selection.attempted, [first.join("both.yaml")]);
    assert_eq!(selection.into_result().expect("found"), first.join("both.yaml"));
}

#[test]
fn a_bare_name_missing_from_every_root_links_into_the_first_root() {
    let fixture = Fixture::new();
    let (first, second) = (fixture.root("first"), fixture.root("second"));

    for roots in [vec![first.clone()], vec![first.clone(), second.clone()]] {
        let selection = fixture.select("absent.yaml", &roots);
        assert!(matches!(selection.outcome, SelectionOutcome::Missing { bare_name: true, .. }), "{selection:?}");
        assert_eq!(creation(&selection), Some(first.join("absent.yaml").as_path()));
        let expected: Vec<PathBuf> = roots.iter().map(|root| root.join("absent.yaml")).collect();
        assert_eq!(selection.attempted, expected);
        assert!(matches!(
            selection.into_result(),
            Err(SchemaError::NoMatch { reference, bare_name: true }) if reference == "absent.yaml"
        ));
    }
    // Nothing was created by planning the link.
    assert!(!first.join("absent.yaml").exists());
}

#[test]
fn a_bare_name_with_no_roots_resolves_like_any_file_reference() {
    let fixture = Fixture::new();

    // A clean miss plans the document folder first.
    let selection = fixture.select("absent.yaml", &[]);
    assert!(matches!(selection.outcome, SelectionOutcome::Missing { bare_name: false, .. }), "{selection:?}");
    assert_eq!(creation(&selection), Some(fixture.docs().join("absent.yaml").as_path()));
    assert_eq!(selection.attempted, [fixture.docs().join("absent.yaml"), fixture.repo().join("absent.yaml")]);
    assert!(matches!(
        selection.into_result(),
        Err(SchemaError::NoMatch { reference, bare_name: false }) if reference == "absent.yaml"
    ));
    assert!(!fixture.docs().join("absent.yaml").exists());

    // A repository-root file is found through the fallback, never replaced
    // by a creation link.
    fixture.write(&fixture.repo().join("fallback.yaml"));
    let selection = fixture.select("fallback.yaml", &[]);
    assert_eq!(found(&selection), fixture.repo().join("fallback.yaml"));
    assert_eq!(
        selection.attempted,
        [fixture.docs().join("fallback.yaml"), fixture.repo().join("fallback.yaml")]
    );
}

#[test]
fn a_bare_name_only_beside_the_document_suggests_the_relative_spelling() {
    let fixture = Fixture::new();
    let first = fixture.root("first");
    let sibling = fixture.docs().join("near.yaml");
    fixture.write(&sibling);

    let selection = fixture.select("near.yaml", std::slice::from_ref(&first));
    match &selection.outcome {
        SelectionOutcome::SiblingOnly { sibling: found_sibling, suggestion, creation } => {
            assert_eq!(found_sibling, &sibling);
            assert_eq!(suggestion, "./near.yaml");
            assert_eq!(creation, &first.join("near.yaml"));
        }
        other => panic!("expected SiblingOnly, got {other:?}"),
    }
    // The sibling is not a schema-root probe.
    assert_eq!(selection.attempted, [first.join("near.yaml")]);
    assert!(matches!(
        selection.into_result(),
        Err(SchemaError::BareNameSiblingExists { reference, suggestion })
            if reference == "near.yaml" && suggestion == "./near.yaml"
    ));
}

#[test]
fn an_explicit_relative_miss_plans_beside_the_document() {
    let fixture = Fixture::new();
    let first = fixture.root("first");

    // Roots do not apply to an explicit relative reference.
    let selection = fixture.select("./x.yaml", std::slice::from_ref(&first));
    assert!(matches!(selection.outcome, SelectionOutcome::Missing { bare_name: false, .. }), "{selection:?}");
    assert_eq!(creation(&selection), Some(fixture.docs().join("x.yaml").as_path()));
    assert_eq!(selection.attempted, [fixture.docs().join("x.yaml")]);

    // A nested implicit path is not a bare name either.
    let selection = fixture.select("sub/x.yaml", std::slice::from_ref(&first));
    assert_eq!(creation(&selection), Some(fixture.docs().join("sub/x.yaml").as_path()));
}

#[test]
fn unresolvable_references_fail_without_a_path() {
    let fixture = Fixture::new();
    let first = fixture.root("first");
    let roots = [first];

    // Past the repository boundary.
    let selection = fixture.select("../../../outside.yaml", &roots);
    assert!(
        matches!(selection.outcome, SelectionOutcome::Failed(SelectionFailure::Resolution(_))),
        "{selection:?}"
    );
    assert!(matches!(selection.into_result(), Err(SchemaError::Unresolved { .. })));

    // Malformed.
    let selection = fixture.select("   ", &roots);
    assert!(
        matches!(selection.outcome, SelectionOutcome::Failed(SelectionFailure::Resolution(_))),
        "{selection:?}"
    );
    assert!(selection.attempted.is_empty());
    assert!(matches!(selection.into_result(), Err(SchemaError::Unresolved { .. })));

    // A missing anchor: the context has no home.
    let selection = fixture.select("~/schemas/x.yaml", &roots);
    assert!(
        matches!(selection.outcome, SelectionOutcome::Failed(SelectionFailure::Resolution(_))),
        "{selection:?}"
    );
    assert!(matches!(
        selection.into_result(),
        Err(SchemaError::Unresolved { reference, .. }) if reference == "~/schemas/x.yaml"
    ));

    // Remote.
    let selection = fixture.select("https://example.com/schema.yaml", &roots);
    assert!(matches!(selection.outcome, SelectionOutcome::Failed(SelectionFailure::Remote)), "{selection:?}");
    assert!(selection.attempted.is_empty());
    assert!(matches!(
        selection.into_result(),
        Err(SchemaError::RemoteUnsupported { reference }) if reference == "https://example.com/schema.yaml"
    ));
}

/// A first root behind an ancestor this user cannot search is an error
/// naming it, and the second root's same-named file never wins.
#[cfg(unix)]
#[test]
fn an_unreadable_first_root_fails_rather_than_falling_through() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let locked = fixture.root("locked");
    let first = locked.join("first");
    fs::create_dir_all(&first).unwrap();
    fixture.write(&first.join("policy.yaml"));
    let second = fixture.root("second");
    fixture.write(&second.join("policy.yaml"));
    let roots = [first.clone(), second.clone()];

    // Readable control.
    assert_eq!(found(&fixture.select("policy.yaml", &roots)), first.join("policy.yaml"));

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let denied = fs::symlink_metadata(&first)
        .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied);
    let selection = fixture.select("policy.yaml", &roots);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if !denied {
        // A privileged user can still search the folder.
        return;
    }
    assert!(
        matches!(
            &selection.outcome,
            SelectionOutcome::Failed(SelectionFailure::Resolution(FileReferenceError::Io { .. }))
        ),
        "{selection:?}"
    );
    assert!(!selection.attempted.contains(&second.join("policy.yaml")), "{:?}", selection.attempted);
    assert!(matches!(selection.into_result(), Err(SchemaError::Unresolved { reference, .. }) if reference == "policy.yaml"));
}
