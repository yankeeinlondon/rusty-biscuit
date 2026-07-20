//! Snapshot round-trip and catalog-parity tests.
//!
//! The round-trip cases deliberately mutate serialized JSON rather than build
//! Rust values: version skew arrives as *bytes* from a newer producer, and a
//! test that can only express what today's enums can name would never see the
//! unknown code or unknown detail key it is meant to prove survives.

use serde_json::{Value, json};

use super::*;
use crate::composition::CompositionError;
use crate::diagnostics::{CODES, code_spec, null_detail_for};
use crate::error::ClaudineError;
use crate::harness::error::HarnessError;

fn file_not_found() -> CompositionError {
    CompositionError::FileNotFound("missing.md".to_string())
}

// -- projection ------------------------------------------------------------

#[test]
fn snapshot_projects_every_facet_of_the_diagnostic() {
    let err = file_not_found();
    let snapshot = DiagnosticSnapshot::from_diagnostic(&err);

    assert_eq!(snapshot.schema_version, DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(snapshot.code, "composition.invalid_file_reference");
    assert_eq!(snapshot.category, "composition");
    assert_eq!(snapshot.disposition, "correctable");
    assert_eq!(snapshot.origin, "author");
    assert_eq!(snapshot.severity, "error");
    assert_eq!(snapshot.detail["reference"], json!("missing.md"));
    assert_eq!(snapshot.message, err.to_string());
}

#[test]
fn snapshot_category_is_always_the_codes_prefix() {
    // The facets are independent strings on the wire, so nothing but this
    // guarantees the pair a consumer matches on stays coherent.
    for err in typed_errors() {
        let snapshot = DiagnosticSnapshot::from_diagnostic(err.as_ref());
        assert!(
            snapshot.code.starts_with(&format!("{}.", snapshot.category)),
            "code `{}` is not prefixed by category `{}`",
            snapshot.code,
            snapshot.category
        );
    }
}

#[test]
fn snapshot_message_is_notification_safe() {
    // A multi-line `Display` must not reach a TTS route or webhook intact.
    let err = CompositionError::SchemaLoad {
        source_path: std::path::PathBuf::from("p.md"),
        message: "line one\nline two\ttabbed".to_string(),
    };
    let snapshot = DiagnosticSnapshot::from_diagnostic(&err);

    assert!(!snapshot.message.is_empty());
    assert!(
        !snapshot.message.contains(['\n', '\t', '\x1b']),
        "message carries control characters: {:?}",
        snapshot.message
    );
    assert!(snapshot.message.contains("line one line two tabbed"));
    assert!(snapshot.message.chars().count() <= 240);
}

#[test]
fn snapshot_message_is_clamped_for_an_overlong_display() {
    let err = CompositionError::SchemaLoad {
        source_path: std::path::PathBuf::from("p.md"),
        message: "x".repeat(500),
    };
    let snapshot = DiagnosticSnapshot::from_diagnostic(&err);

    assert_eq!(snapshot.message.chars().count(), 240);
    assert!(snapshot.message.ends_with('…'));
}

// -- one-level cause -------------------------------------------------------

#[test]
fn snapshot_carries_the_next_registered_cause() {
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(file_not_found()),
    };
    let snapshot = DiagnosticSnapshot::from_diagnostic(&wrapped);

    let cause = snapshot.cause.expect("the inner diagnostic is registered");
    assert_eq!(cause.code, "composition.invalid_file_reference");
    assert_eq!(cause.origin, "author");
    assert_eq!(cause.detail["reference"], json!("missing.md"));
    assert_eq!(cause.message, "file not found: missing.md");
}

#[test]
fn snapshot_has_no_cause_when_the_chain_ends() {
    let snapshot = DiagnosticSnapshot::from_diagnostic(&file_not_found());
    assert!(snapshot.cause.is_none());
}

#[test]
fn a_cause_is_projected_only_one_level_deep() {
    // Three registered diagnostics stacked; only the second may appear.
    let deep = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(CompositionError::LifecycleEvaluationAlreadyEmitted {
            inner: Box::new(file_not_found()),
        }),
    };
    let value = serde_json::to_value(DiagnosticSnapshot::from_diagnostic(&deep)).unwrap();

    assert!(value["cause"].is_object());
    assert!(
        value["cause"].get("cause").is_none(),
        "v1 must not expose `err.cause.cause`: {value}"
    );
}

#[test]
fn an_absent_cause_is_omitted_rather_than_null() {
    let value = serde_json::to_value(DiagnosticSnapshot::from_diagnostic(&file_not_found())).unwrap();
    assert!(
        value.get("cause").is_none(),
        "an absent cause must not serialize as a key: {value}"
    );
}

// -- round trip ------------------------------------------------------------

#[test]
fn snapshot_round_trips_every_facet_detail_message_and_cause() {
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(file_not_found()),
    };
    let original = DiagnosticSnapshot::from_diagnostic(&wrapped);

    let json = serde_json::to_string(&original).unwrap();
    let decoded: DiagnosticSnapshot = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, original);
    // ...and a second cycle is byte-identical, so nothing is lost on the way in.
    assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
}

#[test]
fn every_typed_error_round_trips() {
    for err in typed_errors() {
        let original = DiagnosticSnapshot::from_diagnostic(err.as_ref());
        let json = serde_json::to_string(&original).unwrap();
        let decoded: DiagnosticSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, original, "round trip lost data for `{err}`");
    }
}

#[test]
fn an_unknown_code_survives_a_read_write_cycle() {
    // A newer producer's code has no `CodeSpec` here. Deserialization must not
    // reject it, and re-serialization must not normalize it away — otherwise an
    // older reporting path silently rewrites the newer producer's identity.
    let wire = json!({
        "schema_version": DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        "category": "composition",
        "code": "composition.not_a_code_we_know",
        "disposition": "correctable",
        "origin": "author",
        "severity": "error",
        "detail": { "reference": "x.md" },
        "message": "something newer went wrong",
    });

    let decoded: DiagnosticSnapshot = serde_json::from_value(wire.clone()).unwrap();

    assert_eq!(decoded.code, "composition.not_a_code_we_know");
    assert!(code_spec(&decoded.code).is_none(), "test premise: the code is unknown");
    assert_eq!(serde_json::to_value(&decoded).unwrap(), wire);
}

#[test]
fn an_unknown_facet_value_survives_a_read_write_cycle() {
    // Facets are owned strings precisely so a value outside today's closed
    // enums round-trips instead of failing to deserialize.
    let wire = json!({
        "schema_version": DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        "category": "quantum",
        "code": "quantum.decohered",
        "disposition": "reschedulable",
        "origin": "cosmic_ray",
        "severity": "catastrophic",
        "detail": Value::Null,
        "message": "a facet vocabulary from the future",
    });

    let decoded: DiagnosticSnapshot = serde_json::from_value(wire.clone()).unwrap();

    assert_eq!(decoded.disposition, "reschedulable");
    assert_eq!(serde_json::to_value(&decoded).unwrap(), wire);
}

#[test]
fn unknown_detail_fields_survive_a_read_write_cycle() {
    // The catalog evolves additively; a detail key added after this consumer
    // shipped must pass through untouched, including inside the cause.
    let wire = json!({
        "schema_version": DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        "category": "composition",
        "code": "composition.invalid_file_reference",
        "disposition": "correctable",
        "origin": "author",
        "severity": "error",
        "detail": {
            "reference": "x.md",
            "candidates": [{ "path": "/repo/x.md", "root": "repository", "probe": "missing" }],
            "a_field_from_the_future": { "nested": [1, 2, 3] },
        },
        "message": "no match",
        "cause": {
            "category": "io",
            "code": "io.read_failed",
            "disposition": "correctable",
            "origin": "environment",
            "severity": "error",
            "detail": { "path": "/repo/x.md", "another_future_field": true },
            "message": "no such file",
        },
    });

    let decoded: DiagnosticSnapshot = serde_json::from_value(wire.clone()).unwrap();

    assert_eq!(decoded.detail["a_field_from_the_future"]["nested"], json!([1, 2, 3]));
    assert_eq!(
        decoded.cause.as_ref().unwrap().detail["another_future_field"],
        json!(true)
    );
    assert_eq!(serde_json::to_value(&decoded).unwrap(), wire);
}

// -- forward compatibility through the real `err.*` consumer ----------------

/// A snapshot projected from a locked catalog code stays catalog-shaped: the
/// facets derive from the [`CodeSpec`](crate::diagnostics::CodeSpec), every
/// declared detail key is present-and-null, and there is no cause. This is the
/// label-only path `LifecycleErrorInfo::from_action_failure` reaches.
#[test]
fn from_code_projects_a_catalog_shaped_snapshot() {
    let snapshot = DiagnosticSnapshot::from_code("cap.rate_limit", "slow down".to_string())
        .expect("cap.rate_limit is a registered code");

    assert_eq!(snapshot.schema_version, DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(snapshot.category, "cap");
    assert_eq!(snapshot.disposition, "throttled");
    assert_eq!(snapshot.message, "slow down");
    assert!(snapshot.detail.is_object(), "detail stays catalog-shaped, not null");
    assert!(snapshot.cause.is_none(), "a label carries no typed cause");

    assert!(DiagnosticSnapshot::from_code("no.such_code", String::new()).is_none());
}

/// An unknown code and unknown detail keys from a newer producer survive the
/// read/write path of a **real** production consumer: deserialized into a
/// snapshot, embedded in a [`LifecycleErrorInfo`], and projected to `err.*` and
/// `err.cause.*`. A standalone snapshot round-trip cannot prove that the
/// projection an older consumer actually runs preserves them (spec §D9).
#[test]
fn an_unknown_code_and_detail_survive_the_err_star_projection() {
    use crate::composition::LifecycleErrorInfo;

    let wire = json!({
        "schema_version": DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        "category": "composition",
        "code": "composition.not_a_code_we_know",
        "disposition": "correctable",
        "origin": "author",
        "severity": "error",
        "detail": {
            "reference": "x.md",
            "a_field_from_the_future": { "nested": [1, 2, 3] },
        },
        "message": "something newer went wrong",
        "cause": {
            "category": "io",
            "code": "io.read_failed",
            "disposition": "correctable",
            "origin": "environment",
            "severity": "error",
            "detail": { "path": "/repo/x.md", "another_future_field": true },
            "message": "no such file",
        },
    });
    let snapshot: DiagnosticSnapshot = serde_json::from_value(wire).unwrap();
    assert!(
        code_spec(&snapshot.code).is_none(),
        "test premise: the code is unknown to this consumer"
    );

    let info = LifecycleErrorInfo {
        kind: "LifecycleAction",
        variant: "when".to_string(),
        msg: snapshot.message.clone(),
        snapshot: Some(Box::new(snapshot)),
    };
    let value = info.to_value();

    // The unknown code reaches `err.code` (and its deprecated `variant` alias)
    // untouched — an older `when:` clause keying on it still matches.
    assert_eq!(value["code"], json!("composition.not_a_code_we_know"));
    assert_eq!(value["variant"], json!("composition.not_a_code_we_know"));
    // The unknown detail keys survive into `err.detail.*` and `err.cause.detail.*`.
    assert_eq!(value["detail"]["a_field_from_the_future"]["nested"], json!([1, 2, 3]));
    assert_eq!(value["cause"]["detail"]["another_future_field"], json!(true));
}

// -- catalog parity --------------------------------------------------------

#[test]
fn null_detail_for_agrees_with_the_catalog_for_every_code() {
    for spec in CODES {
        let base = null_detail_for(spec.code);
        let object = base
            .as_object()
            .unwrap_or_else(|| panic!("`{}` seeded a non-object detail", spec.code));

        assert_eq!(
            object.len(),
            spec.detail.len(),
            "`{}` seeds {} keys for {} declared fields: {base}",
            spec.code,
            object.len(),
            spec.detail.len()
        );
        for &field in spec.detail {
            assert_eq!(
                object.get(field),
                Some(&Value::Null),
                "`{}` must seed declared field `{field}` to null: {base}",
                spec.code
            );
        }
    }
}

#[test]
fn no_registered_code_projects_a_top_level_null_detail() {
    // A code that claims a catalog entry must project that entry's object
    // shape. `Value::Null` would make `err.detail.<field>` unresolvable for
    // every field the code advertises.
    for err in typed_errors() {
        let snapshot = DiagnosticSnapshot::from_diagnostic(err.as_ref());
        if code_spec(&snapshot.code).is_none() {
            continue;
        }
        assert!(
            snapshot.detail.is_object(),
            "`{}` projects a non-object detail for registered code `{}`: {}",
            err,
            snapshot.code,
            snapshot.detail
        );
    }
}

#[test]
fn a_registered_snapshot_carries_every_declared_detail_key() {
    for err in typed_errors() {
        let snapshot = DiagnosticSnapshot::from_diagnostic(err.as_ref());
        let Some(spec) = code_spec(&snapshot.code) else {
            continue;
        };
        for &field in spec.detail {
            assert!(
                snapshot.detail.get(field).is_some(),
                "`{}` omits declared field `{field}` of `{}`: {}",
                err,
                snapshot.code,
                snapshot.detail
            );
        }
    }
}

/// One diagnostic per production `impl Diagnostic`, plus the file-reference
/// variants this phase extends. Not exhaustive over variants — the source
/// -parity guard (Phase 6) is what makes the *type* list complete.
fn typed_errors() -> Vec<Box<dyn Diagnostic>> {
    vec![
        Box::new(file_not_found()),
        Box::new(CompositionError::InvalidReference {
            reference: "@@bad".to_string(),
            source: biscuit_file::FileReferenceError::InvalidSyntax("@@bad".to_string()),
        }),
        Box::new(CompositionError::LifecycleEvaluationAlreadyEmitted {
            inner: Box::new(file_not_found()),
        }),
        Box::new(ClaudineError::ProviderNotAvailable("codex".to_string())),
        Box::new(HarnessError::PathResolutionFailed {
            raw: "@/missing".to_string(),
            failure: crate::harness::PathResolutionFailure::TargetMissing,
            source_path: None,
            resolved: None,
            resolution: None,
        }),
    ]
}

// -- selection through an erased boundary ----------------------------------
//
// `DiagnosticSnapshot::select` exists for the exec-wiring boundary, which
// returns `color_eyre::eyre::Report`. The library cannot depend on color_eyre,
// but the property that makes the seam work is `Report`'s, not its own:
// boxing an error publishes it to the chain rather than discarding it, and the
// selection walk reaches it by downcast. `Box<dyn Error>` exercises exactly
// that shape.

/// The erasure the exec-wiring boundary performs, in the form the library can
/// construct: a concrete diagnostic behind a trait object.
fn erased(error: CompositionError) -> Box<dyn std::error::Error + 'static> {
    Box::new(error)
}

#[test]
fn select_recovers_every_facet_through_an_erased_boundary() {
    let erased = erased(file_not_found());

    let snapshot = DiagnosticSnapshot::select(erased.as_ref())
        .expect("a registered diagnostic in the chain projects a snapshot");

    assert_eq!(snapshot.code, "composition.invalid_file_reference");
    assert_eq!(snapshot.category, "composition");
    assert_eq!(snapshot.origin, "author");
    assert_eq!(snapshot.detail["reference"], "missing.md");
}

#[test]
fn select_agrees_with_direct_projection() {
    // The seam must not become a second, divergent derivation: recovering a
    // diagnostic through erasure and projecting it directly are the same walk.
    let direct = DiagnosticSnapshot::from_diagnostic(&file_not_found());
    let through_erasure = DiagnosticSnapshot::select(erased(file_not_found()).as_ref())
        .expect("the erased chain still carries the diagnostic");

    assert_eq!(through_erasure, direct);
}

#[test]
fn select_projects_the_one_level_cause_through_an_erased_boundary() {
    // A transparent wrapper over a semantic cause: selection walks through the
    // wrapper, so the *cause* is what must survive erasure with facets intact.
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(file_not_found()),
    };

    let snapshot = DiagnosticSnapshot::select(erased(wrapped).as_ref())
        .expect("the wrapper's cause is registered");

    assert_eq!(snapshot.code, "composition.invalid_file_reference");
    assert_eq!(snapshot.detail["reference"], "missing.md");
}

#[test]
fn select_returns_none_when_the_chain_holds_only_prose() {
    // The honest-`None` case the three record sites rely on: a boundary whose
    // failure never was a Claudine diagnostic records prose and no snapshot,
    // rather than a fabricated identity.
    #[derive(Debug)]
    struct Prose;
    impl std::fmt::Display for Prose {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("something went wrong")
        }
    }
    impl std::error::Error for Prose {}

    let erased: Box<dyn std::error::Error + 'static> = Box::new(Prose);
    assert!(DiagnosticSnapshot::select(erased.as_ref()).is_none());
}

#[test]
fn a_selected_snapshot_round_trips_through_json() {
    // The record types that now carry a snapshot are the reason it must
    // survive serialization: the facets and the one-level cause are the payload.
    let snapshot = DiagnosticSnapshot::select(erased(file_not_found()).as_ref()).unwrap();

    let json = serde_json::to_string(&snapshot).expect("snapshot serializes");
    let restored: DiagnosticSnapshot = serde_json::from_str(&json).expect("snapshot deserializes");

    assert_eq!(restored, snapshot);
}

#[test]
fn a_record_serialized_without_a_cause_still_deserializes() {
    // Backward compatibility for a record written before its producer had a
    // cause to project: `cause` is `skip_serializing_if`, so an older payload
    // simply lacks the key and must not fail to parse.
    let snapshot = DiagnosticSnapshot::select(erased(file_not_found()).as_ref()).unwrap();
    let mut json: Value = serde_json::to_value(&snapshot).unwrap();
    json.as_object_mut().unwrap().remove("cause");

    let restored: DiagnosticSnapshot =
        serde_json::from_value(json).expect("a payload without `cause` deserializes");
    assert!(restored.cause.is_none());
    assert_eq!(restored.code, snapshot.code);
}
