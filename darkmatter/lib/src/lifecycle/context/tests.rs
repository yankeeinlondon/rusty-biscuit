//! The `err`/`timing` envelope: what `to_value` projects for a coded and an
//! uncoded failure, and how the catalog-taking constructors use the catalog.

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::*;
use crate::lifecycle::envelope::{DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, DiagnosticCause};

/// A catalog that knows one failure label and one code, and selects nothing.
struct OneCode;

impl DiagnosticCatalog for OneCode {
    fn select(&self, _error: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot> {
        None
    }

    fn snapshot_for_code(&self, code: &str, message: String) -> Option<DiagnosticSnapshot> {
        (code == "provider.timeout").then(|| snapshot(code, message))
    }

    fn snapshot_for_error_kind(&self, label: &str, message: String) -> Option<DiagnosticSnapshot> {
        (label == "timeout").then(|| snapshot("provider.timeout", message))
    }
}

fn snapshot(code: &str, message: String) -> DiagnosticSnapshot {
    DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: "provider".to_string(),
        code: code.to_string(),
        disposition: "transient".to_string(),
        origin: "provider".to_string(),
        severity: "error".to_string(),
        detail: json!({ "retry_after_ms": 500, "reset_at": null }),
        message,
        cause: None,
        frontmatter_excerpt: None,
    }
}

#[test]
fn an_uncoded_failure_projects_only_the_legacy_aliases() {
    let info = LifecycleErrorInfo::from_action_failure_with(&OneCode, "shell", "command `x` exited with code 2");

    assert!(info.snapshot.is_none());
    assert_eq!(
        info.to_value(),
        json!({ "kind": "LifecycleAction", "variant": "shell", "msg": "command `x` exited with code 2" })
    );
}

#[test]
fn a_coded_failure_projects_its_facets_conveniences_and_a_null_cause() {
    let info = LifecycleErrorInfo::from_action_failure_with(&OneCode, "timeout", "provider timed out");

    assert_eq!(
        info.to_value(),
        json!({
            "kind": "provider",
            "variant": "provider.timeout",
            "msg": "provider timed out",
            "code": "provider.timeout",
            "category": "provider",
            "disposition": "transient",
            "origin": "provider",
            "severity": "error",
            "detail": { "retry_after_ms": 500, "reset_at": null },
            "is_transient": true,
            "is_throttled": false,
            "is_correctable": false,
            "reset_at": null,
            "retry_after_ms": 500,
            "cause": null,
        })
    );
}

#[test]
fn a_registered_cause_projects_one_level_under_cause() {
    let mut coded = snapshot("provider.timeout", "outer".to_string());
    coded.cause = Some(DiagnosticCause {
        category: "io".to_string(),
        code: "io.read".to_string(),
        disposition: "transient".to_string(),
        origin: "environment".to_string(),
        severity: "error".to_string(),
        detail: Value::Null,
        message: "inner".to_string(),
    });
    let info = LifecycleErrorInfo::from_snapshot("HostError", "Timeout".to_string(), coded);

    assert_eq!(info.msg, "outer");
    assert_eq!(info.to_value()["cause"]["code"], "io.read");
    assert_eq!(info.to_value()["cause"]["message"], "inner");
}

#[test]
fn an_action_failure_message_is_made_concise() {
    let info = LifecycleErrorInfo::from_action_failure_with(&OneCode, "shell", "first\n\tsecond");

    assert_eq!(info.msg, "first second");
}

#[test]
fn an_unselected_error_falls_back_to_its_verb_and_prose_and_keeps_its_cause() {
    let info = LifecycleErrorInfo::from_expr_error_with(&OneCode, "when", LifecycleExprError::prose("bad operand"));

    assert_eq!(info.kind, "LifecycleAction");
    assert_eq!(info.variant, "when");
    assert_eq!(info.msg, "bad operand");
    assert_eq!(info.cause.as_ref().map(ToString::to_string).as_deref(), Some("bad operand"));
}

#[test]
fn at_property_keeps_the_first_property() {
    let info = LifecycleErrorInfo::from_action_failure_with(&OneCode, "shell", "x")
        .at_property("start[0].shell")
        .at_property("start");

    assert_eq!(info.property.as_deref(), Some("start[0].shell"));
}

#[test]
fn timing_projects_only_measured_fields() {
    let start = Instant::now();
    let timing = LifecycleTiming::from_instants(start, None, start + Duration::from_millis(1500));

    assert_eq!(timing.to_value(), json!({ "document_ms": 1500 }));
}

#[test]
fn timing_projects_total_from_the_run_start() {
    let run_start = Instant::now();
    let document_start = run_start + Duration::from_millis(200);
    let timing =
        LifecycleTiming::from_instants(document_start, Some(run_start), run_start + Duration::from_millis(700));

    assert_eq!(timing.to_value(), json!({ "document_ms": 500, "total_ms": 700 }));
}
