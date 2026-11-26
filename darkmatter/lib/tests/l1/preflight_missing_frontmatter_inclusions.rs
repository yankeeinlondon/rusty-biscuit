//! Pre-flight makes composition's absence decision for a `prologue` or
//! `epilogue` entry. Under `with_allow_missing_transclusions(true)` a
//! well-formed entry that names no file (failure class `NoMatch`) is skipped,
//! as composition removes it, and the walk goes on to discover the valid
//! sibling after it. Without the allowance the miss is the located error
//! composition raises. A malformed entry (`@`) and a present file the
//! directive cannot include (`notes.txt`) are no absence: they refuse under
//! the allowance too.

use biscuit_file::{FileResolutionContext, ResolutionFailure};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::{MarkdownError, MarkdownResult};
use std::path::Path;
use tempfile::TempDir;

/// A temporary repository holding `side.md`, whose one shell command marks
/// it as discovered, and the plain-text `notes.txt`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("side.md"), "Side\n\n::shell echo side\n").unwrap();
    std::fs::write(repo.path().join("notes.txt"), "plain text\n").unwrap();
    repo
}

/// A request in the repository's frozen context, so nothing scans the real
/// working tree.
fn request(root: &Path, allow_missing: bool) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
            .with_allow_missing_transclusions(allow_missing),
        context,
    )
}

/// `doc.md` with `key` set to `value` (a YAML scalar or flow list).
fn document(repo: &TempDir, key: &str, value: &str) -> Markdown {
    let path = repo.path().join("doc.md");
    std::fs::write(&path, format!("---\ntitle: doc\n{key}: {value}\n---\nBody\n")).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

fn preflight(repo: &TempDir, key: &str, value: &str, allow_missing: bool) -> MarkdownResult<Vec<String>> {
    let md = document(repo, key, value);
    md.compose_preflight(&request(repo.path(), allow_missing)).map(|report| report.approval_set())
}

/// Composes `md` the way the CLI does after a successful walk: under the
/// walk's approvals and with its graph.
fn compose_after_preflight(repo: &TempDir, md: &Markdown) -> MarkdownResult<String> {
    let report = md.compose_preflight(&request(repo.path(), true))?;
    let approvals: ShellPreApprovals = report.approval_set().into_iter().collect();
    let request = request(repo.path(), true).map_options(|options| {
        options.with_pre_approved_commands(approvals).with_preflight_graph(report.preflight_graph)
    });
    md.compose_with(&request).map(|(composed, _)| composed.content().to_string())
}

/// The one error record of `error`: its code, line, and failure class.
fn located(error: &MarkdownError) -> (String, Option<usize>, Option<String>) {
    let [record] = error_diagnostics(error).try_into().unwrap_or_else(|records| panic!("one record: {records:#?}"));
    let line = record.source.as_ref().and_then(|source| source.line);
    let failure = record.reference_details.as_ref().and_then(|details| details.failure.clone());
    (record.code, line, failure)
}

/// The failure class composition's recovery reads.
fn failure_class(error: &MarkdownError) -> Option<ResolutionFailure> {
    match error {
        MarkdownError::Transclusion(inner) => inner.resolution_failure(),
        _ => None,
    }
}

/// The written entry, the line it is on (the key's line for a scalar, the
/// entry's for a block list), and the shape's frontmatter value.
fn shapes(target: &str) -> [(String, usize); 2] {
    [(format!("\"{target}\""), 3), (format!("\n  - \"{target}\"\n  - \"./side.md\""), 4)]
}

macro_rules! frontmatter_cases {
    ($($key:ident),+) => {$(
        mod $key {
            use super::*;

            /// With the allowance the absent entry is skipped and the valid
            /// sibling after it is still walked; composition agrees.
            #[test]
            fn an_absent_entry_is_skipped_under_the_allowance() {
                let repo = repository();
                for (value, _) in shapes("./gone.md") {
                    let approvals = preflight(&repo, stringify!($key), &value, true)
                        .unwrap_or_else(|error| panic!("{value}: {error}"));
                    let listed = value.contains("side.md");
                    assert_eq!(approvals.contains(&"echo side".to_string()), listed, "{value}: {approvals:?}");
                    // Composition under the walk's approvals runs the
                    // sibling's command and removes the absent entry.
                    let md = document(&repo, stringify!($key), &value);
                    let composed = compose_after_preflight(&repo, &md).unwrap_or_else(|error| panic!("compose {value}: {error}"));
                    assert_eq!(composed.contains("side"), listed, "{value}: {composed}");
                    assert!(composed.contains("Body"), "{value}: {composed}");
                }
            }

            /// Without it the same miss is composition's located error, of
            /// class `NoMatch`.
            #[test]
            fn an_absent_entry_is_a_located_error_without_the_allowance() {
                let repo = repository();
                for (value, line) in shapes("./gone.md") {
                    let error = preflight(&repo, stringify!($key), &value, false).expect_err("a missing entry refuses");
                    assert_eq!(failure_class(&error), Some(ResolutionFailure::NoMatch), "{value}: {error}");
                    let (_, at, failure) = located(&error);
                    assert_eq!(at, Some(line), "{value}: {error}");
                    assert_eq!(failure.as_deref(), Some("no-match"), "{value}: {error}");
                }
            }

            /// A malformed entry is an invalid reference, not an absence: it
            /// refuses at its site under the allowance too.
            #[test]
            fn a_malformed_entry_refuses_under_the_allowance() {
                let repo = repository();
                for (value, line) in shapes("@") {
                    for allow_missing in [false, true] {
                        let error = preflight(&repo, stringify!($key), &value, allow_missing)
                            .expect_err("a malformed entry refuses");
                        assert_ne!(failure_class(&error), Some(ResolutionFailure::NoMatch), "{value}: {error}");
                        let (_, at, failure) = located(&error);
                        assert_eq!(at, Some(line), "{value} {allow_missing}: {error}");
                        assert_eq!(failure.as_deref(), Some("invalid-reference"), "{value}: {error}");
                    }
                }
            }

            /// A present file the directive cannot include is no absence.
            #[test]
            fn a_present_unsupported_entry_refuses_under_the_allowance() {
                let repo = repository();
                for (value, _) in shapes("./notes.txt") {
                    let error = preflight(&repo, stringify!($key), &value, true).expect_err("a text file refuses");
                    assert_ne!(failure_class(&error), Some(ResolutionFailure::NoMatch), "{value}: {error}");
                    let md = document(&repo, stringify!($key), &value);
                    md.compose_with(&request(repo.path(), true)).expect_err("composition refuses it too");
                }
            }
        }
    )+};
}

frontmatter_cases!(prologue, epilogue);

/// Sibling walker: composition checks each copy a collection body repeats
/// against the site pre-flight discovered for it. An absent copy pre-flight
/// skipped is removed under the allowance, never refused as unapproved, and
/// the valid copy beside it composes.
#[test]
fn an_absent_repeated_inclusion_is_skipped_under_the_allowance() {
    let repo = repository();
    let path = repo.path().join("doc.md");
    std::fs::write(
        &path,
        "---\n$schema:\n  names: string[](required)\nnames: [./gone.md, ./side.md]\n---\n::map names\n::file {{ i }}\n::end-map\n",
    )
    .unwrap();
    let md = Markdown::try_from(path.as_path()).unwrap();
    let approvals = md.compose_preflight(&request(repo.path(), true)).unwrap_or_else(|error| panic!("{error}")).approval_set();
    assert_eq!(approvals, ["echo side"]);
    let composed = compose_after_preflight(&repo, &md).unwrap_or_else(|error| panic!("compose: {error}"));
    assert!(composed.contains("side"), "{composed}");
    assert!(!composed.contains("::file"), "{composed}");

    let error = md.compose_preflight(&request(repo.path(), false)).expect_err("without the allowance it refuses");
    assert_eq!(failure_class(&error), Some(ResolutionFailure::NoMatch), "{error}");
}
