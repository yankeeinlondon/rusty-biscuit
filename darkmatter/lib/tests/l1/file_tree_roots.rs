//! The tree root (`base_dir`) a transcluded document resolves its relative
//! references within, through the library compose entry point.
//!
//! A document opened through a `~` or `{{VAR}}` reference takes that anchor as
//! its tree root, so a relative link inside it may move around the anchor but
//! not leave it. A document inside a repository is bounded by the repository.
//! Every fixture is a fresh temporary directory with an explicit request
//! snapshot (injected home and environment), so nothing reads the host.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReferenceError, FileResolutionContext};
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions, ComposeReport, TransclusionError};
use darkmatter::markdown::{Markdown, MarkdownError};

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// `temp/` holds a launch directory, a home, a `NOTES` directory, and one file
/// outside both anchors that an escaping link would reach.
struct Layout {
    _temp: tempfile::TempDir,
    root: PathBuf,
    work: PathBuf,
    home: PathBuf,
    notes: PathBuf,
}

fn layout() -> Layout {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let layout = Layout {
        work: root.join("work"),
        home: root.join("home"),
        notes: root.join("notes"),
        root,
        _temp: temp,
    };
    std::fs::create_dir_all(&layout.work).unwrap();
    write(&layout.root.join("outside.md"), "outside-content\n");
    for anchor in [&layout.home, &layout.notes] {
        write(&anchor.join("b.md"), "in-tree-content\n");
    }
    for child in [layout.home.join("Downloads"), layout.notes.join("inbox")] {
        write(&child.join("a.md"), "::file ../b.md\n");
        write(&child.join("escape.md"), "::file ../../outside.md\n");
    }
    layout
}

/// The request snapshot a host captures in `work`: no repository, the
/// injected home, and `NOTES` in the environment.
fn snapshot(layout: &Layout) -> FileResolutionContext {
    let env = HashMap::from([(
        "NOTES".to_string(),
        layout.notes.to_string_lossy().into_owned(),
    )]);
    FileResolutionContext::from_snapshot(&layout.work, Some(layout.home.clone()), env)
}

/// Composes `work/parent.md` with transclusion only, so Darkmatter's own
/// `{{ … }}` interpolation never evaluates a `{{VAR}}` reference. `fail_fast`
/// surfaces a nested child's failure as the error instead of a placeholder.
fn compose_parent(
    work: &Path,
    snapshot: FileResolutionContext,
    body: &str,
    fail_fast: bool,
) -> Result<(Markdown, ComposeReport), MarkdownError> {
    let parent = work.join("parent.md");
    write(&parent, body);
    let options = ComposeOptions::new()
        .with_file_resolution_context(snapshot)
        .with_source_file(&parent)
        .with_fail_fast(fail_fast)
        .only(&[ComposeOperation::BlockTransclusion]);
    Markdown::try_from(parent.as_path()).unwrap().compose_with(options)
}

fn composed(work: &Path, snapshot: FileResolutionContext, body: &str) -> String {
    match compose_parent(work, snapshot, body, true) {
        Ok((composed, _)) => composed.content().to_string(),
        Err(error) => panic!("compose must succeed: {error:?}"),
    }
}

/// The typed boundary error, or a panic naming what came back instead.
///
/// Also checks the tolerant default: the escape becomes a visible
/// placeholder and a warning, and the target outside the tree is never read.
fn tree_escape(work: &Path, snapshot: FileResolutionContext, body: &str) -> (PathBuf, String) {
    let (tolerated, report) = compose_parent(work, snapshot.clone(), body, false)
        .unwrap_or_else(|error| panic!("a nested failure is tolerated by default: {error:?}"));
    assert!(tolerated.content().contains("_Could not transclude"), "{}", tolerated.content());
    assert!(!tolerated.content().contains("outside-content"), "{}", tolerated.content());
    assert!(
        report.warnings.iter().any(|warning| warning.message.contains("leaves")),
        "{:?}",
        report.warnings
    );
    match compose_parent(work, snapshot, body, true) {
        Err(MarkdownError::Transclusion(error)) => match *error {
            TransclusionError::FileReference(FileReferenceError::RelativeTreeEscape {
                base_dir,
                reference,
                ..
            }) => (base_dir, reference),
            other => panic!("expected RelativeTreeEscape, got {other:?}"),
        },
        Err(other) => panic!("expected a transclusion error, got {other:?}"),
        Ok((composed, _)) => panic!("expected RelativeTreeEscape, composed {:?}", composed.content()),
    }
}

#[test]
fn a_home_anchored_document_resolves_within_home_and_cannot_leave_it() {
    let layout = layout();

    let content = composed(&layout.work, snapshot(&layout), "::file ~/Downloads/a.md\n");
    assert!(content.contains("in-tree-content"), "{content}");

    let (base_dir, reference) =
        tree_escape(&layout.work, snapshot(&layout), "::file ~/Downloads/escape.md\n");
    assert_eq!(base_dir, layout.home);
    assert_eq!(reference, "../../outside.md");
}

#[test]
fn an_environment_anchored_document_resolves_within_the_variable_and_cannot_leave_it() {
    let layout = layout();

    let content = composed(&layout.work, snapshot(&layout), "::file \"{{NOTES}}/inbox/a.md\"\n");
    assert!(content.contains("in-tree-content"), "{content}");

    let (base_dir, reference) =
        tree_escape(&layout.work, snapshot(&layout), "::file \"{{NOTES}}/inbox/escape.md\"\n");
    assert_eq!(base_dir, layout.notes);
    assert_eq!(reference, "../../outside.md");
}

/// Control: the same document opened by its absolute path carries no anchor,
/// so its tree is only a fallback to its own directory and does not bound
/// `../../outside.md`. This is what makes the anchored cases above
/// load-bearing.
#[test]
fn the_same_document_opened_without_an_anchor_has_no_boundary() {
    let layout = layout();
    for child in [
        layout.home.join("Downloads/escape.md"),
        layout.notes.join("inbox/escape.md"),
    ] {
        let body = format!("::file \"{}\"\n", biscuit_file::to_portable_string(&child));
        let content = composed(&layout.work, snapshot(&layout), &body);
        assert!(content.contains("outside-content"), "{}: {content}", child.display());
    }
}

/// An anchor never replaces a tree that already contains the document: a
/// `~`-opened document inside the request repository stays bounded by the
/// repository, not by home.
#[test]
fn an_anchor_inside_the_repository_keeps_the_repository_as_the_tree() {
    let layout = layout();
    let repo = layout.home.join("repo");
    write(&repo.join("docs/escape.md"), "::file ../../b.md\n");
    let snapshot = FileResolutionContext::from_snapshot(&repo, Some(layout.home.clone()), HashMap::new())
        .with_repository_root(&repo);

    let (base_dir, reference) = tree_escape(&repo, snapshot, "::file ~/repo/docs/escape.md\n");
    assert_eq!(base_dir, repo);
    assert_eq!(reference, "../../b.md");
}

/// A transcluded document inside a repository may not reach outside it with a
/// relative link, whichever spelling the link uses.
#[test]
fn a_relative_link_that_leaves_the_repository_is_an_error() {
    let layout = layout();
    let repo = layout.work.clone();
    write(&repo.join("docs/inside.md"), "::file ../shared.md\n");
    write(&repo.join("shared.md"), "shared-content\n");
    let snapshot = || {
        FileResolutionContext::from_snapshot(&repo, Some(layout.home.clone()), HashMap::new())
            .with_repository_root(&repo)
    };

    let content = composed(&repo, snapshot(), "::file ./docs/inside.md\n");
    assert!(content.contains("shared-content"), "{content}");

    for escaping in ["./../../outside.md", "../../outside.md", "sub/../../../outside.md"] {
        write(&repo.join("docs/escape.md"), &format!("::file {escaping}\n"));
        let (base_dir, reference) = tree_escape(&repo, snapshot(), "::file ./docs/escape.md\n");
        assert_eq!(base_dir, repo, "{escaping}");
        assert_eq!(reference, escaping);
    }
}
