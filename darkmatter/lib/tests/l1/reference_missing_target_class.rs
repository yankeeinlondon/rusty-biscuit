//! `ReferenceIssue::is_missing_target` is decided by the failure class: only
//! a well-formed target that names no file is a missing target, whatever its
//! reference kind. An invalid file reference, an unreadable candidate, a
//! malformed URL, and an empty target keep their own classes, so no
//! missing-target recovery can cover them.

use biscuit_file::{FileResolutionContext, ResolutionFailure};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::reference::types::{ReferenceGraphOptions, ReferenceKind};
use darkmatter::markdown::reference::validate::{
    ReferenceIssue, ReferenceIssueCode, ReferenceSeverity, ReferenceValidationOptions,
};
use std::path::Path;
use tempfile::TempDir;

fn request(root: &Path) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")),
        context,
    )
}

/// The one issue validation reports for `body`, written as `doc.md` in a
/// temporary repository.
fn issue_for(repo: &TempDir, body: &str) -> ReferenceIssue {
    let path = repo.path().join("doc.md");
    std::fs::write(&path, body).unwrap();
    let md = Markdown::try_from(path.as_path()).unwrap();
    let options = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request(repo.path())));
    let report = md.validate_references(options).expect("validation runs");
    let [issue] = report.issues.as_slice() else { panic!("{body}: one issue, got {:#?}", report.issues) };
    issue.clone()
}

fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    repo
}

macro_rules! class_cases {
    ($($name:ident: $body:expr => $kind:ident, $code:ident, $failure:expr, $missing:expr;)+) => {$(
        #[test]
        fn $name() {
            let repo = repository();
            let issue = issue_for(&repo, $body);
            assert_eq!(issue.kind, ReferenceKind::$kind, "{issue:#?}");
            assert_eq!(issue.code, ReferenceIssueCode::$code, "{issue:#?}");
            assert_eq!(issue.resolution_failure(), $failure, "{issue:#?}");
            assert_eq!(issue.is_missing_target(), $missing, "{issue:#?}");
        }
    )+};
}

class_cases! {
    an_absent_link_target_is_missing: "[Guide](./gone.md)\n" => Hyperlink, MissingLocalTarget, Some(ResolutionFailure::NoMatch), true;
    an_absent_image_target_is_missing: "![Guide](./gone.png)\n" => Image, MissingLocalTarget, Some(ResolutionFailure::NoMatch), true;
    an_absent_transclusion_target_is_missing: "::file ./gone.md\n" => Transclusion, MissingLocalTarget, Some(ResolutionFailure::NoMatch), true;
    an_invalid_link_target_is_not_missing: "[Guide](@)\n" => Hyperlink, InvalidReference, Some(ResolutionFailure::InvalidReference), false;
    an_angle_bracketed_invalid_link_target_is_not_missing: "[Guide](<@>)\n" => Hyperlink, InvalidReference, Some(ResolutionFailure::InvalidReference), false;
    an_invalid_image_target_is_not_missing: "![Guide](@)\n" => Image, InvalidReference, Some(ResolutionFailure::InvalidReference), false;
    an_invalid_prologue_link_is_not_missing: "---\nprologue: \"[Guide](@)\"\n---\nBody\n" => Hyperlink, InvalidReference, Some(ResolutionFailure::InvalidReference), false;
    a_malformed_link_url_is_not_missing: "[Guide](http://[bad)\n" => Hyperlink, InvalidUrl, None, false;
    an_empty_link_target_is_not_missing: "[Guide]()\n" => Hyperlink, EmptyTarget, None, false;
}

/// A candidate under a directory the process cannot traverse is an I/O
/// failure, never a missing file.
#[cfg(unix)]
#[test]
fn an_unreadable_link_target_is_not_missing() {
    use std::os::unix::fs::PermissionsExt;
    let repo = repository();
    let locked = repo.path().join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::write(locked.join("guide.md"), "# Guide\n").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    // A process that can still read through it (root) cannot produce the case.
    let enforced = std::fs::metadata(locked.join("guide.md")).is_err();
    let issue = enforced.then(|| issue_for(&repo, "[Guide](./locked/guide.md)\n"));
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    let Some(issue) = issue else {
        eprintln!("skipped: directory permissions are not enforced for this process");
        return;
    };
    assert_eq!(issue.code, ReferenceIssueCode::UnreadableTarget, "{issue:#?}");
    assert_eq!(issue.severity, ReferenceSeverity::Error, "{issue:#?}");
    assert_eq!(issue.resolution_failure(), Some(ResolutionFailure::Io), "{issue:#?}");
    assert!(!issue.is_missing_target(), "{issue:#?}");
}
