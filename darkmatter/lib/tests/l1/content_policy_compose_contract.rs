//! Composition under the Darkmatter baseline (`md compose`'s default) checks
//! `content_policy` without changing it, inserting it, or acting on it.
//!
//! Validation through `DarkmatterSchemas` is covered by
//! `content_policy_embedded_baseline`; these tests hold the compose pipeline
//! to the same declaration while pinning what composition adds: pending
//! values are judged after resolution, the baseline never materializes the
//! property, and nothing the policy names is read, fetched, evaluated, or
//! renewed.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::content_policy_fixtures::MIXED_POLICY_FRONTMATTER;

fn temp_root() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// Composes `frontmatter` as `root/doc.md` under the Darkmatter baseline,
/// returning the composed document and the file's bytes afterwards.
fn compose(root: &Path, frontmatter: &str) -> (Result<Markdown, MarkdownError>, Vec<u8>) {
    let path = root.join("doc.md");
    std::fs::write(&path, format!("---\n{frontmatter}---\nBody\n")).expect("write document");
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
        .with_source_file(path.clone())
        .with_darkmatter_baseline_schema();
    let composed = Markdown::try_from(path.as_path())
        .expect("the document loads")
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed);
    (composed, std::fs::read(&path).expect("reread document"))
}

fn problem_paths(error: &MarkdownError) -> Vec<String> {
    match error {
        MarkdownError::SchemaValidationFailed { problems, .. } => {
            problems.iter().map(|problem| problem.path.clone()).collect()
        }
        other => panic!("expected a schema validation failure, got {other}"),
    }
}

/// The authoring example composes, and its `content_policy` (values and
/// order) and `last_updated` come out exactly as authored.
#[test]
fn the_authoring_example_composes_with_its_policy_unchanged() {
    let (_dir, root) = temp_root();

    let (composed, _) = compose(&root, MIXED_POLICY_FRONTMATTER);

    let composed = composed.expect("the authoring example composes");
    let frontmatter = composed.frontmatter().as_map();
    assert_eq!(
        frontmatter.get("content_policy"),
        Some(&json!(["ValidFor(3mo)", {"rule": "ValidUntil(2027-01-01)", "action": "archive"}]))
    );
    let authored: Markdown = format!("---\n{MIXED_POLICY_FRONTMATTER}---\nBody\n").as_str().into();
    assert_eq!(frontmatter.get("last_updated"), authored.frontmatter().as_map().get("last_updated"));
}

/// The baseline declares `content_policy` without a default: a document that
/// omits it composes without gaining the key, as a list or as `null`.
#[test]
fn a_document_without_a_policy_gains_no_policy_key() {
    let (_dir, root) = temp_root();

    let (composed, _) = compose(&root, "title: no policy\n");

    let composed = composed.expect("a policy-free document composes");
    assert!(!composed.frontmatter().as_map().contains_key("content_policy"));
}

/// An already-resolved invalid rule fails composition at the offending item.
#[test]
fn a_resolved_invalid_policy_fails_composition_at_its_item() {
    let (_dir, root) = temp_root();

    let (composed, _) = compose(&root, "content_policy:\n  - \"ValidFor(3mo)\"\n  - Duration(3mo)\n");

    let error = composed.expect_err("an unknown rule fails composition");
    assert_eq!(problem_paths(&error), ["/content_policy/1"]);
}

/// Templates are resolved before the baseline judges them, as for every other
/// frontmatter value: a template that resolves to a valid rule or list
/// passes, and one that resolves to an invalid rule or list fails at the
/// resolved node.
#[test]
fn pending_templates_are_checked_after_resolution() {
    let (_dir, root) = temp_root();

    for (frontmatter, expected) in [
        ("rule: ValidFor(3mo)\ncontent_policy: [\"{{ rule }}\"]\n", json!(["ValidFor(3mo)"])),
        (
            "rules: [TimeSensitive, \"ValidFor(2wk)\"]\ncontent_policy: \"{{ rules }}\"\n",
            json!(["TimeSensitive", "ValidFor(2wk)"]),
        ),
    ] {
        let (composed, _) = compose(&root, frontmatter);
        let composed = composed.unwrap_or_else(|error| panic!("{frontmatter:?}: {error}"));
        assert_eq!(composed.frontmatter().as_map().get("content_policy"), Some(&expected), "{frontmatter:?}");
    }

    for (frontmatter, expected) in [
        ("rule: Duration(3mo)\ncontent_policy: [\"{{ rule }}\"]\n", "/content_policy/0"),
        ("rules: []\ncontent_policy: \"{{ rules }}\"\n", "/content_policy"),
    ] {
        let (composed, _) = compose(&root, frontmatter);
        let error = composed.expect_err(frontmatter);
        assert_eq!(problem_paths(&error), [expected], "{frontmatter:?}");
    }
}

/// A document that declares `content_policy` itself keeps composition's rule
/// for an absent optional document-declared property: it is materialized as
/// `null`. The baseline's own declaration never does that (see
/// `a_document_without_a_policy_gains_no_policy_key`).
#[test]
fn a_document_declared_policy_keeps_optional_null_materialization() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    std::fs::write(root.join("content-policy.yaml"), shipped).expect("write schema");
    let declared = "$schema:\n  content_policy: \"policy[](min(1))@./content-policy.yaml\"\n";

    let (composed, _) = compose(&root, &format!("{declared}title: no policy\n"));
    let composed = composed.expect("an absent optional policy composes");
    assert_eq!(composed.frontmatter().as_map().get("content_policy"), Some(&Value::Null));

    let (composed, _) = compose(&root, &format!("{declared}content_policy: [Duration(3mo)]\n"));
    assert_eq!(problem_paths(&composed.expect_err("the document's type still applies")), ["/content_policy/0"]);
}

/// Validating and composing a policy acts on nothing it names: the document's
/// bytes are unchanged, no `last_updated` is stamped, the watched file and
/// URL are never needed, and no effect engine or network access runs.
#[test]
fn policy_checks_are_passive() {
    #[cfg(feature = "effects-instrumentation")]
    let before = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());
    let (_dir, root) = temp_root();
    let frontmatter = concat!(
        "content_policy:\n",
        "  - \"FileChanged(does/not/exist.md, @missing_fingerprint)\"\n",
        // Schema-valid, parser-invalid (a documented limitation): still never fetched.
        "  - \"FileChanged(https://example.com/a.md, @url_fingerprint)\"\n",
        "  - \"ValidFor(3mo, @reviewed_on)\"\n",
        "  - rule: \"ValidUntil(@retire_on)\"\n",
        "    action: archive\n",
    );
    let document = format!("---\n{frontmatter}---\nBody\n");

    let api = darkmatter::markdown::schemas::DarkmatterSchemas::new(crate::request_support::context_at(&root))
        .with_darkmatter_baseline_json_schema()
        .expect("the embedded baseline attaches");
    let path = root.join("doc.md");
    std::fs::write(&path, &document).expect("write document");
    let markdown = Markdown::try_from(path.as_path()).expect("loads");
    let report = api.validate(&markdown).expect("prepares");
    assert!(report.valid, "{:?}", report.problems);
    assert_eq!(std::fs::read(&path).expect("reread"), document.as_bytes(), "validation leaves the file alone");

    let (composed, bytes) = compose(&root, frontmatter);
    let composed = composed.expect("the policy composes without its evidence");
    assert_eq!(bytes, document.as_bytes(), "composition leaves the file alone");
    let composed_frontmatter = composed.frontmatter().as_map();
    for absent in ["last_updated", "missing_fingerprint", "url_fingerprint", "reviewed_on", "retire_on"] {
        assert!(!composed_frontmatter.contains_key(absent), "`{absent}` was stamped");
    }
    assert_eq!(composed_frontmatter.get("content_policy"), markdown.frontmatter().as_map().get("content_policy"));
    assert!(!root.join("does").exists(), "the watched path was not created");

    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(
        (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
        before,
        "policy checks build no effect engine and attempt no network access",
    );
}
