//! `DarkmatterSchemas::validate_summarized`: the report and the schema
//! presence come from one effective-schema preparation, the report equals
//! what `validate` returns, and the older entry points delegate to the same
//! operation.
//!
//! `md schema validate`'s preparation counts are asserted here, at the
//! library: the CLI binary is never built with `work-counters`. The
//! assignment cases mirror its order: one preparation to coerce assignment
//! values, then the final summarized validation after the assignments.
//! Preparation counts are asserted only with the `work-counters` feature;
//! the report and summary assertions run either way.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, PreparationSummary, SchemaError, SchemaPhase, ValidationReport,
    ValidationTimings,
};
use serde_json::json;

/// A canonical temp repository root; the guard keeps it alive.
fn repository() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    (dir, root)
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

/// The entry point with no baseline.
fn schemas(root: &Path) -> DarkmatterSchemas {
    let context = biscuit_file::FileResolutionContext::from_snapshot(root, None, HashMap::new())
        .with_repository_root(root);
    DarkmatterSchemas::new(context)
}

fn document(root: &Path, name: &str, text: &str) -> Markdown {
    let path = root.join(name);
    write(&path, text);
    Markdown::try_from(path.as_path()).expect("document")
}

/// Two triggers selected by `mode`, each requiring its own field.
fn mode_triggers(root: &Path) -> DarkmatterSchemas {
    for mode in ["alpha", "beta"] {
        write(
            &root.join(format!("schemas/{mode}.trigger.yaml")),
            &format!(
                "kind: schema-trigger\nmatch:\n  mode: enum({mode}; required)\n$schema:\n  {mode}_field: string(required)\n"
            ),
        );
    }
    schemas(root)
        .with_trigger_discovery()
        .expect("trigger discovery")
}

/// The counters a preparation count is measured from; nothing without the
/// `work-counters` feature, where [`assert_preparations`] checks nothing.
#[cfg(feature = "work-counters")]
type Preparations = darkmatter::work_counters::WorkSnapshot;
#[cfg(not(feature = "work-counters"))]
struct Preparations;

fn preparations() -> Preparations {
    #[cfg(feature = "work-counters")]
    return darkmatter::work_counters::snapshot();
    #[cfg(not(feature = "work-counters"))]
    Preparations
}

#[cfg_attr(not(feature = "work-counters"), allow(unused_variables))]
fn assert_preparations(before: &Preparations, expected: u64, case: &str) {
    #[cfg(feature = "work-counters")]
    {
        use darkmatter::work_counters::{Work, snapshot};
        assert_eq!(
            snapshot().since(before).get(Work::EffectivePreparation),
            expected,
            "{case}"
        );
    }
}

/// Summarizes `doc`, asserting one preparation and a report identical to
/// `validate`'s.
fn summarize(
    api: &DarkmatterSchemas,
    doc: &Markdown,
    case: &str,
) -> (ValidationReport, PreparationSummary) {
    let before = preparations();
    let summarized = api.validate_summarized(doc).expect(case);
    assert_preparations(&before, 1, case);
    let plain = api.validate(doc).expect(case);
    assert_eq!(
        format!("{:?}", summarized.0),
        format!("{plain:?}"),
        "{case}: same report as validate"
    );
    summarized
}

#[test]
fn no_schema_with_the_baseline_disabled_is_vacuously_valid_without_a_schema() {
    let (_dir, root) = repository();
    let doc = document(&root, "doc.md", "---\ntitle: plain\n---\nbody\n");

    let (report, summary) = summarize(&schemas(&root), &doc, "no schema");

    assert!(report.valid);
    assert!(report.problems.is_empty());
    assert!(!summary.has_schema);
}

#[test]
fn baseline_only_has_a_schema() {
    let (_dir, root) = repository();
    let doc = document(&root, "doc.md", "---\ntitle: plain\n---\nbody\n");
    let api = schemas(&root)
        .with_darkmatter_baseline_json_schema()
        .expect("embedded baseline");

    let (report, summary) = summarize(&api, &doc, "baseline only");

    assert!(report.valid, "{:?}", report.problems);
    assert!(summary.has_schema);
}

#[test]
fn explicit_schema_problems_come_from_the_same_preparation() {
    let (_dir, root) = repository();
    let doc = document(
        &root,
        "doc.md",
        "---\n$schema:\n  summary_count: number(required)\nsummary_count: many\n---\n",
    );

    let (report, summary) = summarize(&schemas(&root), &doc, "explicit schema");

    assert!(!report.valid);
    assert_eq!(report.problems.len(), 1);
    assert_eq!(report.problems[0].path, "/summary_count");
    assert!(
        report.problems[0].line.is_some(),
        "problems keep source positions"
    );
    assert!(summary.has_schema);
}

#[test]
fn a_matched_trigger_has_a_schema() {
    let (_dir, root) = repository();
    let api = mode_triggers(&root);
    let doc = document(&root, "doc.md", "---\nmode: alpha\nalpha_field: x\n---\n");

    let (report, summary) = summarize(&api, &doc, "trigger schema");

    assert!(report.valid, "{:?}", report.problems);
    assert!(summary.has_schema);
}

#[test]
fn a_preparation_error_is_the_same_error_validate_returns() {
    let (_dir, root) = repository();
    let doc = document(
        &root,
        "doc.md",
        "---\n$schema: ./missing-summary-schema.yaml\n---\n",
    );
    let api = schemas(&root);

    let before = preparations();
    let summarized: SchemaError = api
        .validate_summarized(&doc)
        .map(|_| ())
        .expect_err("missing schema fails");
    assert_preparations(&before, 1, "a failed preparation still prepares once");
    let plain = api
        .validate(&doc)
        .map(|_| ())
        .expect_err("missing schema fails");

    assert_eq!(summarized.to_string(), plain.to_string());
}

#[test]
fn a_pending_shell_value_reports_as_validate_does() {
    let (_dir, root) = repository();
    let doc = document(
        &root,
        "doc.md",
        "---\n$schema:\n  summary_when: string(required)\nsummary_when: $(date)\n---\n",
    );

    let (report, summary) = summarize(&schemas(&root), &doc, "pending shell value");

    assert!(report.valid, "{:?}", report.problems);
    assert!(summary.has_schema);
}

#[test]
fn phase_validation_delegates_to_one_preparation() {
    let (_dir, root) = repository();
    let doc = document(
        &root,
        "doc.md",
        "---\n$schema:\n  summary_phase: string(required)\n---\n",
    );
    let api = schemas(&root);

    let before = preparations();
    let launch = api
        .validate_for_phase(&doc, SchemaPhase::Launch)
        .expect("launch phase");
    assert_preparations(&before, 1, "validate_for_phase prepares once");

    assert!(
        launch.valid,
        "a non-eager required value is not required at launch"
    );
    assert!(!api.validate(&doc).expect("unphased").valid);
}

#[test]
fn assignments_prepare_twice_and_the_final_report_follows_the_assigned_trigger() {
    let (_dir, root) = repository();
    let api = mode_triggers(&root);
    let mut doc = document(&root, "doc.md", "---\nmode: alpha\nalpha_field: x\n---\n");

    let before = preparations();
    // The pre-assignment preparation `md schema validate` coerces with.
    let coercion = api
        .effective_for(&doc)
        .expect("pre-assignment schema")
        .expect("alpha applies");
    assert!(
        coercion
            .validate(&json!({ "mode": "alpha" }))
            .problems
            .iter()
            .any(|p| p.property.as_deref() == Some("alpha_field"))
    );
    doc.frontmatter_mut()
        .as_map_mut()
        .insert("mode".to_string(), json!("beta"));
    let (validated, timings) = ValidationTimings::record(|| api.validate_summarized(&doc));
    assert_preparations(
        &before,
        2,
        "one coercion preparation plus one final preparation",
    );

    let (report, summary) = validated.expect("final validation");
    assert!(summary.has_schema);
    assert_eq!(
        timings.matched_triggers(),
        ["beta.trigger.yaml".to_string()]
    );
    assert!(!report.valid);
    let properties: Vec<_> = report
        .problems
        .iter()
        .filter_map(|p| p.property.as_deref())
        .collect();
    assert_eq!(
        properties,
        ["beta_field"],
        "judged by the post-assignment trigger"
    );
}

#[test]
fn an_assignment_that_selects_the_first_schema_turns_no_schema_into_a_schema() {
    let (_dir, root) = repository();
    let api = mode_triggers(&root);
    let mut doc = document(&root, "doc.md", "---\ntitle: plain\n---\n");

    let before = preparations();
    assert!(
        api.effective_for(&doc).expect("pre-assignment").is_none(),
        "no trigger matches yet"
    );
    doc.frontmatter_mut()
        .as_map_mut()
        .insert("mode".to_string(), json!("alpha"));
    let (report, summary) = api.validate_summarized(&doc).expect("final validation");
    assert_preparations(
        &before,
        2,
        "one coercion preparation plus one final preparation",
    );

    assert!(
        summary.has_schema,
        "presence reflects the post-assignment preparation"
    );
    assert!(!report.valid);
}
