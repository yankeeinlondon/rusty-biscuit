//! The sequence error family's blocks and projections, one row per variant
//! shape: the plain block an author reads, and the JSON a handler reads
//! (`err.code`, facets, `err.detail`, the hint the passive check carries).
//!
//! Each located case has an unlocated control, so a site row that appears
//! without a site, or vanishes with one, fails.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::{CompositionError, ExecutableField, SequenceLoadCause, SequenceSite, SourceParseStage};
use claudine::diagnostics::{Diagnostic, DiagnosticSnapshot, DiagnosticSnapshotExt};
use serde_json::json;

use crate::common::strip_ansi;

fn site() -> Box<SequenceSite> {
    Box::new(SequenceSite::at(Path::new("prompts/seq.md"), "sequence[1].prompt").in_task("review"))
}

fn unlocated() -> Box<SequenceSite> {
    Box::new(SequenceSite::unlocated())
}

fn yaml_error() -> SequenceLoadCause {
    let source = biscuit_file::Yaml::from_str("a: [1, 2").expect_err("unterminated flow sequence");
    SequenceLoadCause::Yaml(source)
}

/// The cases: a name, then the error.
fn cases() -> Vec<(&'static str, CompositionError)> {
    vec![
        (
            "reference_not_found",
            CompositionError::SequenceReferenceResolution {
                field: ExecutableField::Prompt,
                reference: "./missing.md".into(),
                resolution: None,
                site: site(),
                source: SequenceLoadCause::NotFound { glob_hint: None },
            },
        ),
        (
            "reference_not_found_unlocated",
            CompositionError::SequenceReferenceResolution {
                field: ExecutableField::Sequence,
                reference: "./child.md".into(),
                resolution: None,
                site: unlocated(),
                source: SequenceLoadCause::NotFound { glob_hint: None },
            },
        ),
        (
            "source_read",
            CompositionError::SequenceSourceRead {
                field: Some(ExecutableField::Task),
                path: PathBuf::from("tasks/build.yaml"),
                site: site(),
                source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied"),
            },
        ),
        (
            "source_parse_yaml",
            CompositionError::SequenceSourceParse {
                field: Some(ExecutableField::Task),
                path: PathBuf::from("tasks/bad.yaml"),
                stage: SourceParseStage::Yaml,
                site: site(),
                source: yaml_error(),
            },
        ),
        (
            "source_parse_unknown_field_noun",
            CompositionError::SequenceSourceParse {
                field: None,
                path: PathBuf::from("data/rows.yaml"),
                stage: SourceParseStage::Yaml,
                site: unlocated(),
                source: yaml_error(),
            },
        ),
        (
            "exclusive_executable",
            CompositionError::SequenceExclusiveExecutable {
                index: 1,
                fields: vec!["prompt".into(), "shell".into()],
                site: site(),
            },
        ),
        (
            "unknown_task_field_near_miss",
            CompositionError::SequenceUnknownTaskField {
                key: "whem".into(),
                executable: ExecutableField::Shell,
                allowed: vec!["name".into(), "shell".into(), "when".into()],
                suggestion: Some("when".into()),
                site: site(),
            },
        ),
        (
            "unknown_task_field_no_near_miss",
            CompositionError::SequenceUnknownTaskField {
                key: "zzz".into(),
                executable: ExecutableField::Shell,
                allowed: vec!["name".into(), "shell".into(), "when".into()],
                suggestion: None,
                site: site(),
            },
        ),
        (
            "invalid",
            CompositionError::sequence_invalid("step 3 must be a string or object, got number", *site()),
        ),
        ("empty", CompositionError::SequenceEmpty { site: site() }),
        ("step_name_missing", CompositionError::SequenceStepNameMissing { index: 0, site: site() }),
        (
            "external_wrong_type",
            CompositionError::SequenceExternalWrongType {
                path: PathBuf::from("data/rows.yaml"),
                problem: "expected a list, found a string".into(),
                site: unlocated(),
            },
        ),
        ("null_item", CompositionError::SequenceNullItem { index: 3, site: site() }),
    ]
}

fn plain(err: &CompositionError) -> String {
    let term = Terminal::new_optimistic(100);
    strip_ansi(&err.status_block(&term).render(&term))
}

fn projection(err: &CompositionError) -> serde_json::Value {
    let snapshot = DiagnosticSnapshot::from_diagnostic(err);
    json!({
        "code": err.code(),
        "category": format!("{:?}", err.category()),
        "disposition": format!("{:?}", err.disposition()),
        "origin": format!("{:?}", err.origin()),
        "detail": err.detail(),
        "snapshot_cause": snapshot.cause.as_ref().map(|cause| cause.code.clone()),
        "message": err.to_string(),
        "hint": err.sequence_hint(),
    })
}

#[test]
fn every_sequence_case_renders_its_block_and_projection() {
    for (name, err) in cases() {
        insta::assert_snapshot!(format!("sequence_block_{name}"), plain(&err));
        insta::assert_json_snapshot!(format!("sequence_projection_{name}"), projection(&err));
    }
}

/// A located error shows its site rows; the same error unlocated shows none.
#[test]
fn site_rows_follow_the_site() {
    let located = plain(&CompositionError::SequenceEmpty { site: site() });
    let bare = plain(&CompositionError::SequenceEmpty { site: unlocated() });
    for row in ["at:", "property:", "task:"] {
        assert!(located.contains(row), "{row} missing:\n{located}");
        assert!(!bare.contains(row), "{row} without a site:\n{bare}");
    }
}

/// Each new and reshaped variant keeps the identity of the one it replaced:
/// the `composition.failed` code, its facets, and a null detail base.
#[test]
fn reshaped_sequence_variants_keep_their_identity() {
    let before = CompositionError::sequence_invalid("anything", SequenceSite::unlocated());
    for (name, err) in cases() {
        assert_eq!(err.code(), "composition.failed", "{name}");
        assert_eq!(err.category(), before.category(), "{name}");
        assert_eq!(err.disposition(), before.disposition(), "{name}");
        assert_eq!(err.origin(), before.origin(), "{name}");
        assert_eq!(err.detail(), before.detail(), "{name}");
        assert!(DiagnosticSnapshot::from_diagnostic(&err).cause.is_none(), "{name}: causes stay unregistered");
    }
}

/// Each case's link to the guarded report inventory: the code the passive
/// check (`--dry-run`) reports for the authored shape behind it, whose
/// fixture pair and both rendered snapshots `dry_run_check::codes` (a
/// Claudine `dry_run_report` code) or Darkmatter's `findings` guard (a
/// `REPORTED_FINDING_CODES` entry) require; `None` where the check reports
/// the block's own `composition.failed` row, which these snapshots pin.
const REPORT_LINKS: &[(&str, Option<&str>)] = &[
    ("reference_not_found", Some(darkmatter::findings::MISSING_DOCUMENT_CODE)),
    ("reference_not_found_unlocated", Some(darkmatter::findings::MISSING_DOCUMENT_CODE)),
    ("source_read", None),
    ("source_parse_yaml", None),
    ("source_parse_unknown_field_noun", None),
    ("exclusive_executable", Some("composition.multiple_executables")),
    ("unknown_task_field_near_miss", None),
    ("unknown_task_field_no_near_miss", None),
    ("invalid", None),
    ("empty", None),
    ("step_name_missing", None),
    ("external_wrong_type", None),
    ("null_item", None),
];

/// Every case is linked (a new case must say which report code, if any,
/// covers its shape), and every linked code is in a guarded inventory.
#[test]
fn every_case_is_linked_to_the_guarded_report_inventory() {
    let names: Vec<&str> = cases().into_iter().map(|(name, _)| name).collect();
    let linked: Vec<&str> = REPORT_LINKS.iter().map(|(name, _)| *name).collect();
    assert_eq!(linked, names);
    for (name, code) in REPORT_LINKS {
        let Some(code) = code else { continue };
        let guarded = darkmatter::findings::REPORTED_FINDING_CODES.contains(code)
            || claudine::diagnostics::code_spec(code).is_some_and(|spec| spec.dry_run_report);
        assert!(guarded, "{name}: `{code}` is in no guarded report inventory");
    }
}
