//! A [`BlockingPolicy`] through composition's public results: a call warning
//! it blocks fails the document before any of that document's expressions
//! run (a transcluded child included), without changing the warning's code
//! or severity; and each recovery reports its warning under the code the
//! `md` registry maps to it.

use std::path::Path;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::{Severity, UNCERTAIN_CALL_CODE};
use darkmatter::markdown::compose::policy::{
    BlockingPolicy, INVALID_FRONTMATTER_ASSIGNMENT_CODE, MISSING_TRANSCLUSION_CODE,
    REASSIGNED_FRONTMATTER_PROPERTY_CODE,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::json;
use tempfile::TempDir;

fn compose(dir: &Path, name: &str, options: ComposeOptions) -> Result<(Markdown, ComposeReport), MarkdownError> {
    let path = dir.join(name);
    let options = options.with_source_file(path.clone());
    Markdown::try_from(path.as_path()).unwrap().compose_with(&crate::request_support::request_at(dir, options))
}

fn options_in(dir: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
}

/// `probe(n)` counts its handler entries; `min(label, 5)` is category 3.
const DOCUMENT: &str = "---\n$schema:\n  label: string\nlabel: \"7\"\n---\n{{ probe(1) }} {{ min(label, 5) }}\n";

fn register_probe() -> hooks::TestFunctionGuard {
    let registration = FunctionRegistration::builder("probe", "function(parameters(v: number); returns(number))")
        .build()
        .expect("the test signature compiles");
    hooks::register(registration, Arc::new(|_| Ok(json!(1))))
}

#[test]
fn a_blocking_call_warning_stops_the_document_before_any_expression_runs() {
    let _probe = register_probe();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("doc.md"), DOCUMENT).unwrap();

    hooks::reset_handler_entries();
    let error = compose(dir.path(), "doc.md", options_in(dir.path()).with_blocking_policy(BlockingPolicy::default().deny_all()))
        .expect_err("the category 3 call blocks");
    let MarkdownError::BlockedDiagnostics { blocking, .. } = &error else {
        panic!("expected blocked diagnostics: {error}");
    };
    assert_eq!(blocking.len(), 1, "{blocking:#?}");
    assert_eq!(blocking[0].code.as_deref(), Some(UNCERTAIN_CALL_CODE));
    let record = blocking[0].diagnostic.as_deref().expect("the shared record");
    assert_eq!(record.severity, Severity::Warning, "blocking changes the outcome, never the severity");
    assert_eq!(hooks::handler_entries("probe"), 0, "no expression ran");

    // A denial of another code, and the default policy, leave it a warning.
    for policy in [BlockingPolicy::default(), BlockingPolicy::default().deny_code("dm.expression.always_false_call")] {
        hooks::reset_handler_entries();
        let (composed, report) = compose(dir.path(), "doc.md", options_in(dir.path()).with_blocking_policy(policy))
            .unwrap_or_else(|error| panic!("composes: {error}"));
        assert_eq!(composed.content().trim(), "1 5");
        assert!(report.warnings.iter().any(|warning| warning.code.as_deref() == Some(UNCERTAIN_CALL_CODE)));
        assert_eq!(hooks::handler_entries("probe"), 1);
    }
}

#[test]
fn a_transcluded_child_blocks_before_its_own_expressions_run() {
    let _probe = register_probe();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("child.md"), DOCUMENT).unwrap();
    std::fs::write(dir.path().join("parent.md"), "Before\n\n::file ./child.md\n").unwrap();

    hooks::reset_handler_entries();
    let error = compose(
        dir.path(),
        "parent.md",
        options_in(dir.path()).with_blocking_policy(BlockingPolicy::default().deny_code(UNCERTAIN_CALL_CODE)),
    )
    .expect_err("the child's category 3 call blocks");
    assert!(error.to_string().contains("blocking diagnostic"), "{error}");
    assert_eq!(hooks::handler_entries("probe"), 0, "the child's expressions did not run");
}

#[test]
fn each_recovery_reports_under_its_registered_code() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("child.md"), "---\nname: Alice\n---\n\n{{ name }}\n").unwrap();
    std::fs::write(dir.path().join("invalid.md"), "::file child.md set=42 set.name=\"Bob\"\n").unwrap();
    std::fs::write(dir.path().join("twice.md"), "::file child.md set.name=\"Bob\" set.name=\"Mary\"\n").unwrap();
    std::fs::write(dir.path().join("missing.md"), "Before\n\n::file ./gone.md\n\nAfter\n").unwrap();

    let codes = |report: &ComposeReport| -> Vec<String> { report.warnings.iter().filter_map(|w| w.code.clone()).collect() };

    let (_, report) = compose(dir.path(), "invalid.md", options_in(dir.path()).with_allow_invalid_frontmatter_assignment(true))
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(codes(&report).contains(&INVALID_FRONTMATTER_ASSIGNMENT_CODE.to_string()), "{:#?}", report.warnings);

    let (_, report) = compose(dir.path(), "twice.md", options_in(dir.path()).with_allow_reassigned_frontmatter_property(true))
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(codes(&report).contains(&REASSIGNED_FRONTMATTER_PROPERTY_CODE.to_string()), "{:#?}", report.warnings);

    // Without the recovery a missing target fails; with it the directive is
    // removed and reported.
    compose(dir.path(), "missing.md", options_in(dir.path())).expect_err("a missing transclusion fails");
    let (composed, report) = compose(dir.path(), "missing.md", options_in(dir.path()).with_allow_missing_transclusions(true))
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(!composed.content().contains("::file"), "{}", composed.content());
    assert!(composed.content().contains("After"));
    assert!(codes(&report).contains(&MISSING_TRANSCLUSION_CODE.to_string()), "{:#?}", report.warnings);
}
