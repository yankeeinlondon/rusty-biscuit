//! The dasherized-identifiers motivating incident
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`, Success
//! Criterion 8), against the shipped `prompts/_reviews/feature-review.md`.
//!
//! The typo landed in `7658203f8` as `{{spec-name}}`. The grammar then lexed it
//! as `spec - name`, which warned and shipped the verbatim span. The prompt is
//! composed here as Claudine's prepare path composes it: the lifecycle event
//! keys are deferred to event time, so only the document itself is judged.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport, ComposeWarning};
use darkmatter::markdown::{Markdown, MarkdownError, SourceRef};
use tempfile::TempDir;

/// Claudine's `LIFECYCLE_EVENT_KEYS`, deferred from compose-time resolution.
const LIFECYCLE_EVENT_KEYS: &[&str] =
    &["initialize", "start", "success", "blocked", "failure", "finalize", "loop"];

const SHIPPED_SPEC_NAME: &str = "`{{spec_name}}` spec";

fn checkout() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .parent()
        .and_then(Path::parent)
        .expect("the darkmatter library lives two levels below the repository root")
        .to_path_buf()
}

/// A repository holding the shipped prompt (with `prompt` as its text), the
/// shipped files it transcludes, and one spec.
fn repository(prompt: &str) -> (TempDir, PathBuf, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::create_dir_all(root.join("prompts/_reviews")).unwrap();
    for transcluded in ["prompts/_senior-reviewer.md", "prompts/_ready.md", "prompts/_test-tiers.md"] {
        std::fs::copy(checkout().join(transcluded), root.join(transcluded)).unwrap();
    }
    let prompt_path = root.join("prompts/_reviews/feature-review.md");
    std::fs::write(&prompt_path, prompt).unwrap();
    let spec = root.join("feature/2026-09-01-alpha/spec.md");
    std::fs::create_dir_all(spec.parent().unwrap()).unwrap();
    std::fs::write(&spec, "---\nreview_iterations: 0\n---\n# Spec\n").unwrap();
    (dir, prompt_path, spec)
}

fn compose(prompt_path: &Path, spec: &Path, fail_fast: bool) -> Result<String, MarkdownError> {
    compose_with_report(prompt_path, spec, fail_fast).map(|(content, _)| content)
}

fn compose_with_report(
    prompt_path: &Path,
    spec: &Path,
    fail_fast: bool,
) -> Result<(String, ComposeReport), MarkdownError> {
    let document = Markdown::try_from(prompt_path).unwrap();
    // Capture what the prompt references (it reads `ctx.area`), as a real
    // invocation's request does.
    let context = ComposeContext::capture_for_document(prompt_path.parent().unwrap(), &document);
    let options = ComposeOptions::new_with_context(context)
        .with_source_file(prompt_path.to_path_buf())
        .with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied())
        .with_set_overrides(serde_json::json!({ "spec": spec.to_string_lossy() }))
        .with_fail_fast(fail_fast);
    document
        .compose_with(options)
        .map(|(composed, report)| (composed.content().to_string(), report))
}

fn unknown_identifiers(report: &ComposeReport) -> Vec<&ComposeWarning> {
    report
        .warnings
        .iter()
        .filter(|w| w.code.as_deref() == Some(ComposeWarning::UNKNOWN_IDENTIFIER_CODE))
        .collect()
}

fn shipped() -> String {
    let text = std::fs::read_to_string(checkout().join("prompts/_reviews/feature-review.md")).unwrap();
    assert_eq!(
        text.matches(SHIPPED_SPEC_NAME).count(),
        1,
        "the shipped prompt no longer names its spec as {SHIPPED_SPEC_NAME}; update this regression"
    );
    text
}

#[test]
fn the_shipped_feature_review_composes_and_names_its_spec() {
    let (_dir, prompt, spec) = repository(&shipped());

    let (composed, report) =
        compose_with_report(&prompt, &spec, false).expect("the shipped prompt composes");

    assert!(composed.contains("defined by the `2026-09-01-alpha` spec"), "{composed}");
    assert!(!composed.contains("{{"), "{composed}");
    assert!(unknown_identifiers(&report).is_empty(), "{:?}", report.warnings);
}

/// The incident: the typo as the grammar read it when it was introduced now
/// stops the build, with the prompt's path and line, instead of shipping text.
#[test]
fn the_original_typo_would_have_failed_to_compose() {
    let historical = shipped().replace(SHIPPED_SPEC_NAME, "`{{spec - name}}` spec");
    let line = historical.lines().position(|line| line.contains("{{spec - name}}")).unwrap() + 1;
    let (_dir, prompt, spec) = repository(&historical);

    for fail_fast in [false, true] {
        let error = compose(&prompt, &spec, fail_fast).expect_err("the incident typo stops the build");

        let MarkdownError::Interpolation { key: None, expression, source, cause } = &error else {
            panic!("expected a body interpolation error, got {error:?}");
        };
        assert_eq!(expression, "spec - name");
        assert!(cause.to_string().contains("Subtraction requires numeric operands"), "{cause}");
        let SourceRef::OnDiskSpan { context, span } = source.as_ref() else {
            panic!("expected the authored span, got {source:?}");
        };
        assert!(context.display.ends_with("prompts/_reviews/feature-review.md"), "{:?}", context.display);
        assert_eq!(span.line(), line);
        assert_eq!(&historical[span.range()], "{{spec - name}}");
    }
}

/// Today's grammar reads the typo as one identifier, so it no longer fails as
/// subtraction or ships the verbatim span.
#[test]
fn the_typo_is_one_identifier_under_the_current_grammar() {
    let typo = shipped().replace(SHIPPED_SPEC_NAME, "`{{spec-name}}` spec");
    let typo_line = typo.lines().position(|line| line.contains("{{spec-name}}")).unwrap() + 1;
    let (_dir, prompt, spec) = repository(&typo);

    let (composed, report) =
        compose_with_report(&prompt, &spec, false).expect("an unknown identifier renders empty");

    let line = composed.lines().find(|line| line.contains("functionality defined by the")).unwrap();
    assert!(!line.contains("spec-name") && !line.contains("alpha"), "{line}");
    assert!(!composed.contains("{{"), "{composed}");
    // Requirement 4: the typo is no longer silent.
    let warnings = unknown_identifiers(&report);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert!(warnings[0].message.starts_with("unknown identifier 'spec-name' at "), "{}", warnings[0].message);
    assert_eq!(warnings[0].line_number, Some(typo_line));
}
