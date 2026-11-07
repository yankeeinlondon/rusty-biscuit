//! The schema status report judges a file a caller supplied inside an object
//! (`author={"avatar":"assets/avatar.png"}`) from the caller's launch
//! directory, not the document's, and still reports a nested file that
//! resolves from neither or that the document itself authored.
//!
//! Each fixture launches from `<tmp>/launch` with the document at
//! `<tmp>/launch/sub/doc.md`, so a relative `assets/avatar.png` resolves from
//! the launch directory and never from the document's. The context is built
//! from a `RequestSnapshot` over the fixture, never the process CWD.

use std::path::{Path, PathBuf};

use claudine::composition::{
    PropertyState, SchemaStatusReport, TriggerFindings, build_schema_status_report,
    resolve_composition_source_in_context,
};
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};

const SCHEMA: &str = "$schema:\n  author:\n    avatar: file(eager)";

struct Fixture {
    _dir: tempfile::TempDir,
    launch: PathBuf,
    document: PathBuf,
}

impl Fixture {
    /// A document under `launch/sub/` with `frontmatter`; the avatar exists
    /// at `launch/assets/avatar.png` only when `with_launch_avatar`.
    fn new(frontmatter: &str, with_launch_avatar: bool) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let launch = root.join("launch");
        let document = launch.join("sub/doc.md");
        std::fs::create_dir_all(document.parent().expect("parent")).expect("document dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        std::fs::write(&document, format!("---\n{frontmatter}\n---\nAVATAR={{{{ author.avatar }}}}\n"))
            .expect("write document");
        if with_launch_avatar {
            std::fs::create_dir_all(launch.join("assets")).expect("assets dir");
            std::fs::write(launch.join("assets/avatar.png"), "launch").expect("write avatar");
        }
        Self { _dir: dir, launch, document }
    }

    fn report(&self, overrides: Option<&serde_json::Value>) -> SchemaStatusReport {
        let snapshot = RequestSnapshot::new(self.launch.clone()).with_home(Some(self.launch.with_file_name("home")));
        let context = build_resolution_context(&snapshot).expect("fixture context");
        let source = resolve_composition_source_in_context(path_str(&self.document), &context)
            .expect("the document resolves");
        build_schema_status_report(&source, overrides, Some(&self.launch), &context, &TriggerFindings::default())
            .expect("the status report loads")
            .expect("the document has a schema")
    }
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("UTF-8 path")
}

fn author_state(report: &SchemaStatusReport) -> PropertyState {
    report
        .required
        .iter()
        .chain(&report.optional)
        .find(|status| status.name == "author")
        .unwrap_or_else(|| panic!("no `author` row: {report:?}"))
        .state
}

fn supplied_author() -> serde_json::Value {
    serde_json::json!({ "author": { "avatar": "assets/avatar.png" } })
}

/// The nested caller file resolves from the launch directory (and not from
/// `sub/`), so the document-base verdict is dropped and `author` is valid.
#[test]
fn nested_caller_file_resolving_from_the_launch_directory_is_not_a_problem() {
    let fixture = Fixture::new(SCHEMA, true);

    let report = fixture.report(Some(&supplied_author()));

    assert_eq!(author_state(&report), PropertyState::Valid, "{report:?}");
    assert!(!report.has_invalid_optional, "{report:?}");
}

/// Control: the same nested caller file, missing from the launch directory
/// and the document's, is still reported; the drop only covers a value that
/// resolves from the caller's origin.
#[test]
fn nested_caller_file_missing_everywhere_is_still_reported() {
    let fixture = Fixture::new(SCHEMA, false);

    let report = fixture.report(Some(&supplied_author()));

    assert_eq!(author_state(&report), PropertyState::Invalid, "{report:?}");
    assert!(report.has_invalid_optional, "{report:?}");
}

/// Control: an authored nested file resolves from the document's directory
/// only, so the same relative path, present beside the launch but not under
/// `sub/`, is still reported.
#[test]
fn authored_nested_file_not_resolving_from_the_document_is_still_reported() {
    let fixture = Fixture::new(&format!("{SCHEMA}\nauthor:\n  avatar: assets/avatar.png"), true);

    let report = fixture.report(None);

    assert_eq!(author_state(&report), PropertyState::Invalid, "{report:?}");
    assert!(report.has_invalid_optional, "{report:?}");
}
