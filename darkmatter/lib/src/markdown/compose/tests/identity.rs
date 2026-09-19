//! Execution identity through the compose boundary (AC4). Every test drives
//! the real demand-driven capture (`ComposeOptions::new()`), so the Document
//! group is captured only when a document names one of its keys.

use super::*;
use crate::markdown::compose::context::capture::with_forced_nonce_failure;
use crate::markdown::compose::expression::ExpressionError;

const FAILURE: &str = "entropy exhausted (test seam)";

fn compose(document: &str) -> MarkdownResult<(Markdown, ComposeReport)> {
    Markdown::from(document).compose_with(ComposeOptions::new())
}

/// The `name=[value]` probe on the composed text.
fn probe(content: &str, name: &str) -> String {
    let start = content
        .find(&format!("{name}=["))
        .unwrap_or_else(|| panic!("no `{name}` probe in {content}"))
        + name.len()
        + 2;
    let end = start + content[start..].find(']').expect("probe closes");
    content[start..end].to_string()
}

/// AC4: an entropy failure reaches the caller as the typed
/// `ExecutionNonceUnavailable` compose error, naming the key that was read.
#[test]
fn an_entropy_failure_is_the_typed_compose_error_for_id_and_sid() {
    for (document, expected_key) in [("run=[{{ ctx.id }}]\n", "id"), ("run=[{{ ctx.sid }}]\n", "sid")] {
        let error = with_forced_nonce_failure(FAILURE, || compose(document))
            .expect_err("an execution without a nonce must not compose");

        match error.missing_runtime_context() {
            Some(ExpressionError::ExecutionNonceUnavailable { key, detail }) => {
                assert_eq!(key, expected_key, "{error:?}");
                assert_eq!(detail, FAILURE, "{error:?}");
            }
            other => panic!("{document}: expected ExecutionNonceUnavailable, got {other:?} from {error:?}"),
        }
        assert!(error.to_string().contains(FAILURE), "{error}");
    }
}

/// Capture is demand-driven: with the entropy source broken, a document that
/// never reads `ctx.id` / `ctx.sid` still composes. A Document-group read
/// other than the identity projects normally and reports the partial capture
/// as a warning rather than failing.
#[test]
fn documents_that_never_read_the_identity_still_compose_without_entropy() {
    with_forced_nonce_failure(FAILURE, || {
        let (composed, report) = compose("plain=[{{ 1 + 1 }}]\n").expect("no Document group is demanded");
        assert_eq!(probe(composed.content(), "plain"), "2");
        assert!(
            !report.warnings.iter().any(|warning| warning.message.contains("document.nonce")),
            "the nonce is never drawn for a document that names no Document key: {:?}",
            report.warnings,
        );

        let (_, report) = compose("hash=[{{ ctx.hash }}]\n").expect("a non-identity Document key composes");
        assert!(
            report.warnings.iter().any(|warning| {
                warning.message.contains("document.nonce") && warning.message.contains(FAILURE)
            }),
            "the failed nonce is reported, not fatal, when the identity is unread: {:?}",
            report.warnings,
        );
    });
}

/// AC4 through composition: two executions of one document differ in both
/// digests, and `ctx.sid` is never a re-spelling of `ctx.id`.
#[test]
fn id_and_sid_differ_across_executions_and_from_each_other() {
    let run = || {
        let (composed, _) = compose("id=[{{ ctx.id }}] sid=[{{ ctx.sid }}]\n").expect("composes");
        let content = composed.content();
        (probe(content, "id"), probe(content, "sid"))
    };
    let (first_id, first_sid) = run();
    let (second_id, second_sid) = run();

    assert_eq!(first_id.len(), 16, "{first_id}");
    assert_eq!(first_sid.len(), 64, "{first_sid}");
    assert_ne!(first_id, second_id, "ctx.id must differ across executions");
    assert_ne!(first_sid, second_sid, "ctx.sid must differ across executions");
    assert_ne!(first_id, first_sid[..16], "ctx.sid is not a re-spelling of ctx.id");
}
