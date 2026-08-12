//! Transclusion-ordering regression for the portable-string boundary.
//!
//! `link_resolve` erroring on a declined destination is only worth anything if
//! the failure happens *before* the child's content reaches the root. Children
//! run their own pipeline inside the transclusion engine, so a unit test on
//! `link_resolve` alone cannot show where in the order the check lands, nor
//! what the root document ends up containing.
//!
//! The engine applies its general policy to the child's failure: it records a
//! `transclusion` warning and replaces the directive with a failure notice, so
//! the root compose returns `Ok`. Two invariants are pinned here — no
//! absolutized child link is ever spliced into the root, and the gap is visible
//! in the artifact — plus the `fail_fast` escape hatch that still returns the
//! error, which is what separates a deliberate downgrade from a lost one.

#![cfg(windows)]

use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions, ComposeReport};
use darkmatter::markdown::{Markdown, MarkdownError};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// A directory name ending in `.` is reachable only through the verbatim
/// namespace, which is exactly why `dunce` refuses to spell it legacy-style.
/// It gives a local fixture the same decline a UNC share would, without waiting
/// on SMB name resolution.
///
/// It must be created and addressed through a `\\?\` path throughout: Win32
/// strips a trailing dot from every component, so a legacy spelling silently
/// creates `trailing` instead and the fixture stops testing anything.
const DECLINED_DIR: &str = "trailing.";

const OPERATIONS: [ComposeOperation; 3] = [
    ComposeOperation::LinkResolve,
    ComposeOperation::BlockTransclusion,
    ComposeOperation::Cleanup,
];

/// Writes `root.md` (transcluding `child.md`), `child.md` (linking
/// `sibling.md`), and `sibling.md` into a fresh `name` directory under
/// `verbatim_parent`, and returns the root document's path.
fn write_documents(verbatim_parent: &Path, name: &str) -> PathBuf {
    let dir = verbatim_parent.join(name);
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("sibling.md"), "# Sibling\n").unwrap();
    fs::write(dir.join("child.md"), "[sib](sibling.md)\n").unwrap();

    let root = dir.join("root.md");
    fs::write(&root, "# Root\n\n::file child.md\n").unwrap();
    root
}

fn compose(root: &Path) -> Result<(Markdown, ComposeReport), MarkdownError> {
    let md = Markdown::try_from(root).unwrap();
    md.compose_with(ComposeOptions::new().with_source_file(root).only(&OPERATIONS))
}

fn compose_fail_fast(root: &Path) -> Result<(Markdown, ComposeReport), MarkdownError> {
    let md = Markdown::try_from(root).unwrap();
    md.compose_with(
        ComposeOptions::new()
            .with_source_file(root)
            .with_fail_fast(true)
            .only(&OPERATIONS),
    )
}

#[test]
fn transcluded_child_under_a_declined_path_becomes_a_visible_notice() {
    let dir = tempdir().unwrap();
    let verbatim_parent = fs::canonicalize(dir.path()).unwrap();
    let root = write_documents(&verbatim_parent, DECLINED_DIR);
    assert!(
        root.to_string_lossy().contains(DECLINED_DIR),
        "Win32 stripped the trailing dot; the fixture is not a declined path: {}",
        root.display()
    );

    let (composed, report) = compose(&root).unwrap();

    assert_eq!(
        report.transclusions_applied, 0,
        "the child must not be spliced in: {}",
        composed.content()
    );
    assert_eq!(report.transclusions_skipped, 1);
    assert!(
        composed.content().contains("_Could not transclude `child.md`_"),
        "the failure should be visible in the document, not only on stderr: {}",
        composed.content()
    );
    assert!(
        !composed.content().contains("::file"),
        "directive syntax must not leak into the output: {}",
        composed.content()
    );
    assert!(
        !composed.content().contains("sibling"),
        "no child link, absolutized or authored, may reach the root: {}",
        composed.content()
    );
    assert!(
        report.warnings.iter().any(|warning| warning.stage == "transclusion"
            && warning.message.contains("no faithful portable Markdown destination")
            && warning.message.contains(DECLINED_DIR)),
        "expected the child's link-resolution failure to be reported, got: {:?}",
        report.warnings
    );

    // `TempDir` deletes through the legacy API, which cannot name a
    // trailing-dot directory; the verbatim spelling has to remove it here.
    let _ = fs::remove_dir_all(verbatim_parent.join(DECLINED_DIR));
}

/// The downgrade above is a policy, not a loss: `fail_fast` still surfaces the
/// child's `MarkdownError::Transform` at the root boundary, naming the path.
///
/// Without this, nothing distinguishes "the engine chose to keep composing" from
/// "the error was discarded on the way out", and the two have very different
/// consequences for a caller that needs to know the document is incomplete.
#[test]
fn fail_fast_surfaces_the_declined_child_link_as_an_error() {
    let dir = tempdir().unwrap();
    let verbatim_parent = fs::canonicalize(dir.path()).unwrap();
    let root = write_documents(&verbatim_parent, DECLINED_DIR);

    let err = compose_fail_fast(&root)
        .expect_err("fail_fast must not downgrade the child's failure to a warning");

    assert!(
        matches!(&err, MarkdownError::Transform(message)
            if message.contains("sibling.md") && message.contains(DECLINED_DIR)),
        "expected the child's link-resolution failure, got: {err:?}"
    );

    let _ = fs::remove_dir_all(verbatim_parent.join(DECLINED_DIR));
}

/// The control. Without it, the test above would pass just as happily if
/// `::file` transclusion were broken outright, or if the fixture's directory
/// name defeated reference resolution rather than the portability check.
#[test]
fn transcluded_child_under_an_ordinary_path_still_composes() {
    let dir = tempdir().unwrap();
    let verbatim_parent = fs::canonicalize(dir.path()).unwrap();
    let root = write_documents(&verbatim_parent, "ordinary");

    let (composed, report) = compose(&root).unwrap();

    assert_eq!(report.transclusions_applied, 1);
    assert!(
        composed.content().contains("/ordinary/sibling.md"),
        "the child's link should have been transcluded and absolutized: {}",
        composed.content()
    );
}
