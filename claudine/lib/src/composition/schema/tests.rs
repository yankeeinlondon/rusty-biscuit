//! Tests for composition schema validation.

use super::*;
use crate::composition::resolve::resolve_composition_source;
use std::fs;
use tempfile::TempDir;

fn make_source(dir: &TempDir, document: &str) -> ResolvedCompositionSource {
    let file = dir.path().join("test.md");
    fs::write(&file, document).unwrap();
    resolve_composition_source(file.to_str().unwrap()).unwrap()
}

#[test]
fn interactive_options_allowed_only_when_all_flags_true() {
    let permissive = InteractiveSchemaOptions {
        prompt_for_missing: true,
        stdin_is_tty: true,
        stderr_is_tty: true,
        silent: false,
    };
    assert!(permissive.allowed());

    assert!(
        !InteractiveSchemaOptions {
            silent: true,
            ..permissive
        }
        .allowed()
    );
    assert!(
        !InteractiveSchemaOptions {
            stdin_is_tty: false,
            ..permissive
        }
        .allowed()
    );
    assert!(
        !InteractiveSchemaOptions {
            stderr_is_tty: false,
            ..permissive
        }
        .allowed()
    );
    assert!(
        !InteractiveSchemaOptions {
            prompt_for_missing: false,
            ..permissive
        }
        .allowed()
    );
}

#[test]
fn interactive_options_default_is_denied() {
    let opts = InteractiveSchemaOptions::default();
    assert!(!opts.allowed());
}

#[test]
fn no_schema_passes_through_unchanged() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, "---\ntitle: Hello\n---\nbody\n");

    let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
    assert!(prepared.prompt.contains("body"));
}

#[test]
fn valid_required_property_passes() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\ntitle: Plan a feature\n---\nbody\n",
    );

    let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert_eq!(
        fm.get("title").and_then(|v| v.as_str()),
        Some("Plan a feature")
    );
}

#[test]
fn missing_required_returns_missing_properties_error() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties {
            missing,
            pointer_paths,
            ..
        } => {
            assert_eq!(missing.len(), 1);
            assert_eq!(missing[0].name, "title");
            assert_eq!(missing[0].type_label.as_deref(), Some("string"));
            assert!(!pointer_paths.is_empty());
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn invalid_required_returns_schema_validation_error() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  count: 'number(required)'\ncount: not-a-number\n---\nbody\n",
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    assert!(
        matches!(err, CompositionError::SchemaValidation { .. }),
        "got: {err:?}"
    );
}

#[test]
fn invalid_optional_is_dropped_and_retried() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\ntitle: Plan\ncount: not-a-number\n---\nbody\n",
    );

    let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Plan"));
    assert!(
        !fm.contains_key("count"),
        "invalid optional `count` should have been dropped"
    );
}

#[test]
fn invalid_optional_setter_is_dropped_and_retried() {
    // `count` is optional in the schema. When the user supplies a bad
    // value via `key=value` / `--set`, the override map must be
    // scrubbed alongside the source frontmatter so the retry succeeds.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\n---\nbody\n",
    );

    let options = PrepareOptions {
        set_overrides: Some(serde_json::json!({
            "title": "Plan",
            "count": "not-a-number",
        })),
        ..Default::default()
    };
    let prepared = prepare_direct_with_schema(&source, options).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Plan"));
    assert!(
        !fm.contains_key("count"),
        "invalid optional override `count` should have been dropped"
    );
}

/// Phase 7 (acceptance criteria 10 + the reproduction fixture): a prompt
/// with a user `$schema` *and* a lifecycle `failure.message: "{{err.msg}}"`
/// validates its ordinary schema inputs exactly as today (DM1b: deferred
/// lifecycle keys are excluded from user schema value validation) and still
/// reaches lifecycle parsing with the late-binding span deferred raw.
#[test]
fn schema_validates_while_lifecycle_err_span_is_deferred() {
    let dir = TempDir::new().unwrap();
    // Mirrors `prompts/implement-plan.md`: required numeric schema inputs
    // alongside a `failure` block whose message references the late-binding
    // `err` global. The `{{err.msg}}` span must not be validated against the
    // user schema, must not fail composition, and must survive raw.
    let source = make_source(
        &dir,
        "---\n$schema:\n  phase: 'number(required)'\n  total_phases: 'number(required)'\nphase: 1\ntotal_phases: 3\nfailure:\n  message: \"❌️ phase {{phase}} failed: {{err.msg}}\"\n---\nbody\n",
    );

    let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();

    // Ordinary schema inputs validated and present.
    assert_eq!(fm.get("phase").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(fm.get("total_phases").and_then(|v| v.as_i64()), Some(3));

    // The lifecycle key is deferred (DM1) and its span survives raw.
    assert!(
        prepared
            .deferred_lifecycle_keys
            .iter()
            .any(|k| k == "failure"),
        "failure should be reported as a deferred lifecycle key"
    );
    assert_eq!(
        prepared
            .lifecycle
            .failure
            .as_ref()
            .unwrap()
            .message
            .as_deref(),
        Some("❌️ phase {{phase}} failed: {{err.msg}}"),
        "lifecycle parsing sees the raw late-binding span after schema validation"
    );
}

#[test]
fn invalid_optional_drop_leaves_missing_required_surfaced() {
    // The optional `count` is invalid AND a different required value
    // is missing. After the drop+retry, the missing-required error
    // should surface so the user (or interactive loop) can fix it.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\ncount: not-a-number\n---\nbody\n",
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(missing.len(), 1);
            assert_eq!(missing[0].name, "title");
        }
        other => panic!("expected MissingProperties after drop+retry, got {other:?}"),
    }
}

#[test]
fn schema_parse_error_for_invalid_schema_shape() {
    let dir = TempDir::new().unwrap();
    // `$schema: 42` is a wrong-shape value (a `SchemaError::FrontmatterShape`),
    // which is a malformed-schema problem, not a reference-resolution one.
    let source = make_source(&dir, "---\n$schema: 42\n---\nbody\n");

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::SchemaParse { property: None, .. }
        ),
        "got: {err:?}"
    );
}

#[test]
fn schema_parse_error_for_grammar_failure_names_property_and_keeps_path_load_distinct() {
    let dir = TempDir::new().unwrap();
    // A bad constraint separator (`,` instead of `;`) is a grammar error in
    // the schema body — the motivating bug. It must surface as `SchemaParse`
    // attributed to the offending property, NOT the path-focused `SchemaLoad`.
    let source = make_source(
        &dir,
        "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nbody\n",
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    let CompositionError::SchemaParse {
        property, message, ..
    } = &err
    else {
        panic!("expected SchemaParse, got: {err:?}");
    };
    assert_eq!(property.as_deref(), Some("spec"));
    assert!(
        message.contains("between constraints"),
        "message must carry the typed grammar detail, got: {message}"
    );
}

#[test]
fn missing_required_surfaces_description_metadata() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required) -> The page title'\n---\nbody\n",
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(missing[0].description.as_deref(), Some("The page title"));
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_required_surfaces_frontmatter_description() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\ndescription: Plan a feature implementation\n---\nbody\n",
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties {
            frontmatter_description,
            ..
        } => {
            assert_eq!(
                frontmatter_description.as_deref(),
                Some("Plan a feature implementation")
            );
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn enum_missing_required_includes_members_in_type_label() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\n---\nbody\n",
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            let label = missing[0]
                .type_label
                .as_deref()
                .expect("expected typed enum label");
            assert!(label.starts_with("enum("), "got: {label}");
            assert!(label.contains("small"), "got: {label}");
            assert!(label.contains("large"), "got: {label}");
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn set_overrides_can_supply_missing_required() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    );

    let options = PrepareOptions {
        set_overrides: Some(serde_json::json!({ "title": "Plan" })),
        ..Default::default()
    };
    let prepared = prepare_direct_with_schema(&source, options).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Plan"));
}

#[test]
fn inline_compose_with_schema_validates_after_prompt_check() {
    // `inline-compose` already requires a frontmatter `prompt` property.
    // Schema validation runs after that check; absent `prompt` still
    // surfaces as `PromptPropertyMissing` rather than as a generic
    // schema problem.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  prompt: 'string(required)'\n---\nbody\n",
    );

    let err = prepare_inline_with_schema(&source, PrepareOptions::default()).unwrap_err();
    // The schema declares `prompt` as required, but `prepare_inline`
    // checks `PromptPropertyMissing` first against the raw source.
    // Darkmatter, however, runs schema validation during compose for
    // direct paths; for inline the temp markdown is built with
    // `fm.clone()` and re-validated. Either error is acceptable here;
    // we just want to make sure a typed error surfaces.
    assert!(
        matches!(
            err,
            CompositionError::PromptPropertyMissing
                | CompositionError::MissingProperties { .. }
        ),
        "got: {err:?}"
    );
}

#[test]
fn inline_compose_with_valid_prompt_and_schema_succeeds() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  prompt: 'string(required)'\nprompt: List three colors\n---\nbody\n",
    );

    let prepared = prepare_inline_with_schema(&source, PrepareOptions::default()).unwrap();
    assert!(prepared.prompt.contains("List three colors"));
}

// -- optional null acceptance (Phase 3) -------------------------------

#[test]
fn optional_string_resolved_to_null_passes_direct() {
    // Regression for the optional-schema-properties incident: an optional
    // `string` whose frontmatter ternary resolves to `null` must validate
    // successfully, and the resolved `null` must be retained in the
    // effective frontmatter rather than silently dropped.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  design: 'string'\n",
            "design: \"{{ file_exists('design.md') ? 'design.md' : null }}\"\n",
            "---\nbody\n",
        ),
    );

    let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert!(
        fm.contains_key("design"),
        "optional property resolved to null must be retained"
    );
    assert_eq!(fm.get("design"), Some(&serde_json::Value::Null));
}

#[test]
fn optional_string_resolved_to_null_passes_inline() {
    // Same null-retention contract on the inline-compose path, which also
    // requires a `prompt` property.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  prompt: 'string(required)'\n",
            "  design: 'string'\n",
            "prompt: List three colors\n",
            "design: \"{{ file_exists('design.md') ? 'design.md' : null }}\"\n",
            "---\nbody\n",
        ),
    );

    let prepared = prepare_inline_with_schema(&source, PrepareOptions::default()).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert!(
        fm.contains_key("design"),
        "optional property resolved to null must be retained in inline compose"
    );
    assert_eq!(fm.get("design"), Some(&serde_json::Value::Null));
}

#[test]
fn required_string_resolved_to_null_fails_schema_validation() {
    // A required `string` whose ternary resolves to `null` must still be
    // classified as an invalid required value (Type problem), producing
    // `SchemaValidation`. If categorization read requiredness from the JSON
    // Schema instead of the `PropertyAtom`, the null could be treated as
    // "absent" and surface as `MissingProperties` instead.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  design: 'string(required)'\n",
            "design: \"{{ file_exists('design.md') ? 'design.md' : null }}\"\n",
            "---\nbody\n",
        ),
    );

    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    assert!(
        matches!(err, CompositionError::SchemaValidation { .. }),
        "required property resolved to null must fail with SchemaValidation, got: {err:?}"
    );
}

// -- interactive_shape -----------------------------------------------

#[test]
fn missing_string_property_maps_to_text_plain_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(
                missing[0].interactive_shape,
                Some(InteractiveShape::Text {
                    format: TextFormat::Plain,
                    min_len: None,
                    max_len: None,
                })
            );
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_number_property_maps_to_number_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  count: 'number(required; integer)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(
                missing[0].interactive_shape,
                Some(InteractiveShape::Number {
                    integer: true,
                    min: None,
                    max: None,
                })
            );
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_boolean_property_maps_to_boolean_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  ready: 'boolean(required)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(missing[0].interactive_shape, Some(InteractiveShape::Boolean));
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_enum_property_maps_to_enum_one_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => match &missing[0]
            .interactive_shape
        {
            Some(InteractiveShape::EnumOne { members }) => {
                assert_eq!(members.len(), 3);
                assert!(members.iter().any(|m| m == "small"));
                assert!(members.iter().any(|m| m == "large"));
            }
            other => panic!("expected EnumOne shape, got {other:?}"),
        },
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_enum_array_property_maps_to_enum_many_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  tags: 'enum(a, b, c)[](required)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert!(matches!(
                missing[0].interactive_shape,
                Some(InteractiveShape::EnumMany { .. })
            ));
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_file_property_maps_to_file_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  template: 'file(required)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(
                missing[0].interactive_shape,
                Some(InteractiveShape::File {
                    is_array: false,
                    patterns: Vec::new(),
                })
            );
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_file_array_property_maps_to_file_array_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  attachments: 'file[](required)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(
                missing[0].interactive_shape,
                Some(InteractiveShape::File {
                    is_array: true,
                    patterns: Vec::new(),
                })
            );
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_file_property_preserves_match_patterns() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  cover: \"file(match('*.png', '*.jpg'); required)\"\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => match &missing[0]
            .interactive_shape
        {
            Some(InteractiveShape::File { patterns, is_array }) => {
                assert!(!is_array);
                assert_eq!(patterns, &["*.png", "*.jpg"]);
            }
            other => panic!("expected File shape, got {other:?}"),
        },
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

#[test]
fn missing_object_property_has_no_interactive_shape() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  config: 'object(required)'\n---\nbody\n",
    );
    let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::MissingProperties { missing, .. } => {
            assert_eq!(missing[0].interactive_shape, None);
        }
        other => panic!("expected MissingProperties, got {other:?}"),
    }
}

// -- build_schema_status_report ---------------------------------------

#[test]
fn status_report_is_none_when_no_schema() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, "---\ntitle: hi\n---\nbody\n");
    let report = build_schema_status_report(&source, None, None).unwrap();
    assert!(report.is_none());
}

#[test]
fn status_report_categorizes_required_and_optional() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n  description: 'string'\ntitle: Plan\n---\nbody\n",
    );
    let report = build_schema_status_report(&source, None, None).unwrap().unwrap();
    assert_eq!(report.required.len(), 1);
    assert_eq!(report.required[0].name, "title");
    assert_eq!(report.required[0].state, PropertyState::Valid);
    assert_eq!(report.optional.len(), 1);
    assert_eq!(report.optional[0].name, "description");
    assert_eq!(report.optional[0].state, PropertyState::Missing);
}

#[test]
fn status_report_marks_missing_required_correctly() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    );
    let report = build_schema_status_report(&source, None, None).unwrap().unwrap();
    assert_eq!(report.required[0].state, PropertyState::Missing);
}

#[test]
fn status_report_marks_invalid_required_correctly() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  count: 'number(required)'\ncount: not-a-number\n---\nbody\n",
    );
    let report = build_schema_status_report(&source, None, None).unwrap().unwrap();
    assert_eq!(report.required[0].state, PropertyState::Invalid);
}

#[test]
fn status_report_overrides_supply_missing_required() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    );
    let overrides = serde_json::json!({ "title": "supplied" });
    let report = build_schema_status_report(&source, Some(&overrides), None)
        .unwrap()
        .unwrap();
    assert_eq!(report.required[0].state, PropertyState::Valid);
}

#[test]
fn status_report_flags_invalid_optional() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\ntitle: Plan\ncount: nope\n---\nbody\n",
    );
    let report = build_schema_status_report(&source, None, None).unwrap().unwrap();
    assert!(report.has_invalid_optional);
}

#[test]
fn status_report_does_not_mark_templated_required_as_invalid() {
    // Regression test for review-4 medium finding. The status report
    // runs against the *raw* frontmatter (no composition), so a
    // schema-constrained value supplied as a template expression
    // (`{{ env.AGENT }}`) would otherwise be flagged Invalid. The
    // preflight + prepare pipeline that executes immediately after
    // composes the frontmatter and finds the value valid — the
    // status report must agree.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  runtime_agent: 'enum(goose; required)'\n",
            "  topic: 'string(required)'\n",
            "runtime_agent: '{{ env.AGENT }}'\n",
            "---\nbody\n",
        ),
    );
    let report = build_schema_status_report(&source, None, None).unwrap().unwrap();
    let runtime = report
        .required
        .iter()
        .find(|s| s.name == "runtime_agent")
        .expect("runtime_agent listed");
    assert_ne!(
        runtime.state,
        PropertyState::Invalid,
        "templated required value must not appear Invalid in the status report: {runtime:?}",
    );
    // The companion missing required must still appear Missing so the
    // user sees what they need to supply.
    let topic = report
        .required
        .iter()
        .find(|s| s.name == "topic")
        .expect("topic listed");
    assert_eq!(topic.state, PropertyState::Missing);
}

#[test]
fn status_report_does_not_mark_templated_optional_as_invalid() {
    // Same composition-tolerance, applied to optional properties:
    // a templated optional value must not contribute to
    // `has_invalid_optional`, because the prepare pipeline will not
    // drop it.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  variant: 'enum(small, medium, large)'\n",
            "title: Plan\n",
            "variant: '{{ env.SIZE }}'\n",
            "---\nbody\n",
        ),
    );
    let report = build_schema_status_report(&source, None, None).unwrap().unwrap();
    assert!(
        !report.has_invalid_optional,
        "templated optional must not be flagged invalid: {:?}",
        report.optional,
    );
    let variant = report
        .optional
        .iter()
        .find(|s| s.name == "variant")
        .expect("variant listed");
    assert_ne!(variant.state, PropertyState::Invalid);
}

#[test]
fn text_format_label_returns_human_strings() {
    assert_eq!(TextFormat::Plain.label(), "string");
    assert_eq!(TextFormat::Date.label(), "date (YYYY-MM-DD)");
    assert!(TextFormat::File.label().contains("file"));
}

#[test]
fn top_level_pointer_segment_handles_escaped_keys() {
    assert_eq!(
        top_level_pointer_segment("/title"),
        Some("title".to_string())
    );
    assert_eq!(
        top_level_pointer_segment("/nested/inner"),
        Some("nested".to_string())
    );
    assert_eq!(
        top_level_pointer_segment("/has~1slash"),
        Some("has/slash".to_string())
    );
    assert_eq!(top_level_pointer_segment(""), None);
    assert_eq!(top_level_pointer_segment("/"), None);
}

// -- composition-tolerant pre-validation ------------------------------

#[test]
fn pre_validate_does_not_reject_template_bearing_value() {
    // Regression test for review-3 high finding. A schema-constrained
    // value supplied as a template expression must NOT fail
    // pre-validation, because Darkmatter's compose pipeline can
    // resolve `{{ env.AGENT }}` into a valid enum member before the
    // prepare-time validator runs.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  runtime_agent: 'enum(goose; required)'\nruntime_agent: '{{ env.AGENT }}'\n---\nbody\n",
    );

    let pre = pre_validate_schema(&source, None, None)
        .expect("template-bearing required value must pass pre-validation");
    // Source/overrides are returned unchanged.
    assert!(pre.set_overrides.is_none());
    let raw = pre
        .source
        .markdown
        .frontmatter()
        .as_map()
        .get("runtime_agent")
        .and_then(|v| v.as_str())
        .unwrap();
    assert!(raw.contains("{{"), "value must not be dropped or scrubbed");
}

#[test]
fn pre_validate_defers_template_invalid_required_to_prepare_time() {
    // A required field with a template value used to fail at
    // pre-validation against the raw frontmatter. Composition may
    // resolve the template to a valid value, so the verdict is
    // deferred. Prepare-time still validates the composed result.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  runtime_agent: 'enum(goose; required)'\nruntime_agent: '{{ env.AGENT }}'\n---\nbody\n",
    );

    let pre = pre_validate_schema(&source, None, None);
    assert!(
        pre.is_ok(),
        "pre-validation must defer template-bearing invalid-required to prepare-time"
    );
}

#[test]
fn pre_validate_still_surfaces_literal_invalid_required() {
    // A required field with a literal (non-template) value that
    // doesn't satisfy the schema is definitively bad and surfaces
    // here as `SchemaValidation` so users see the error early.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  count: 'number(required)'\ncount: not-a-number\n---\nbody\n",
    );

    let err = pre_validate_schema(&source, None, None).unwrap_err();
    assert!(
        matches!(err, CompositionError::SchemaValidation { .. }),
        "expected SchemaValidation for literal invalid-required, got: {err:?}"
    );
}

#[test]
fn pre_validate_still_surfaces_genuinely_missing_required() {
    // Missing-required is composition-independent: no template can
    // conjure a key that isn't present anywhere. This case must still
    // produce `MissingProperties` so the CLI can drive interactive
    // collection.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    );

    let err = pre_validate_schema(&source, None, None).unwrap_err();
    assert!(
        matches!(err, CompositionError::MissingProperties { .. }),
        "expected MissingProperties, got: {err:?}"
    );
}

#[test]
fn drop_invalid_optionals_skips_template_bearing_values() {
    // `count: '{{ env.COUNT }}'` is optional and currently looks like a
    // string (invalid for `number`). The pre-preflight scrub must NOT
    // drop it — composition can produce a numeric value.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  count: 'number'\ncount: '{{ env.COUNT }}'\n---\nbody\n",
    );

    let (scrubbed, _, _) = drop_invalid_optionals(source, None, None);
    let value = scrubbed
        .markdown
        .frontmatter()
        .as_map()
        .get("count")
        .and_then(|v| v.as_str());
    assert_eq!(value, Some("{{ env.COUNT }}"));
}

#[test]
fn drop_invalid_optionals_still_drops_literal_invalid_values() {
    // Non-template invalid optional values are still dropped early as
    // before (preserves the existing UX for hardcoded mistakes).
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  count: 'number'\ncount: nope\n---\nbody\n",
    );

    let (scrubbed, _, _) = drop_invalid_optionals(source, None, None);
    assert!(
        !scrubbed
            .markdown
            .frontmatter()
            .as_map()
            .contains_key("count"),
        "literal invalid optional should still be dropped pre-preflight",
    );
}

#[test]
fn drop_invalid_optionals_keeps_optional_eager_file_failures() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  spec: 'file(eager)'\nspec: missing/spec.md\n---\nbody\n",
    );

    let (scrubbed, _, dropped) = drop_invalid_optionals(source, None, None);
    assert!(dropped.is_empty());
    assert_eq!(
        scrubbed.markdown.frontmatter().as_map().get("spec"),
        Some(&serde_json::json!("missing/spec.md")),
        "optional eager file failures must remain visible for the schema error",
    );
}

#[test]
fn pre_validate_schema_reports_optional_eager_file_failures() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  spec: 'file(eager)'\nspec: missing/spec.md\n---\nbody\n",
    );

    let err = pre_validate_schema(&source, None, None)
        .expect_err("optional eager file failures should not be dropped");
    match err {
        CompositionError::SchemaValidation {
            message, problems, ..
        } => {
            assert!(
                message.contains("missing/spec.md"),
                "schema validation should retain the invalid file reference: {message}",
            );
            assert!(
                message.contains("no existing file matched reference"),
                "schema validation should retain the targeted file-reference reason: {message}",
            );
            assert_eq!(problems, vec!["/spec".to_string()]);
        }
        other => panic!("expected SchemaValidation, got {other:?}"),
    }
}

#[test]
fn scratch_dump_file_array_problems() {
    for (label, override_val) in [
        ("scalar-string", serde_json::json!({ "attachments": "everywhere" })),
        ("array-of-one", serde_json::json!({ "attachments": ["everywhere"] })),
        (
            "array-of-two",
            serde_json::json!({ "attachments": ["everywhere", "here"] }),
        ),
    ] {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  attachments: 'file(required;match(**/*spec*.md);eager)[]'\n---\nbody\n",
        );
        let effective = load_effective_schema(&source, None).unwrap().unwrap();
        let instance = build_effective_instance(&source, Some(&override_val));
        let report = effective.validate(&instance);
        eprintln!("=== {label} valid={} ===", report.valid);
        if let Some(SimplifiedSchema::Single(s)) = effective.simplified.as_ref()
            && let Some(atom) = atom_for_property(s, "attachments")
        {
            eprintln!("  atom.is_array={} ty={:?}", atom.is_array, atom.ty);
        }
        for p in &report.problems {
            eprintln!(
                "  problem: kind={:?} path={:?} msg={:?}",
                p.kind, p.path, p.message
            );
        }
    }
}

#[test]
fn provided_file_match_partial_reports_unresolved_file_reference() {
    // `spec=everywhere` is a provided partial for a required `file(match)`
    // property with no literal `everywhere` file. Instead of the generic
    // SchemaValidation, the layer surfaces the typed UnresolvedFileReference
    // so the CLI can offer a glob+substring confirmation dialog.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  spec: 'file(required;match(**/*spec*.md);eager)'\n---\nbody\n",
    );
    let overrides = serde_json::json!({ "spec": "everywhere" });

    let err = pre_validate_schema(&source, Some(&overrides), None)
        .expect_err("a provided file(match) partial with no literal match should surface a typed error");
    match err {
        CompositionError::UnresolvedFileReference {
            property,
            provided,
            patterns,
            is_array,
            reason,
            ..
        } => {
            assert_eq!(property, "spec");
            assert_eq!(provided, "everywhere");
            assert_eq!(patterns, vec!["**/*spec*.md".to_string()]);
            assert!(!is_array);
            assert!(
                reason.contains("no existing file matched reference"),
                "reason should preserve the original file-reference failure text: {reason}",
            );
        }
        other => panic!("expected UnresolvedFileReference, got {other:?}"),
    }
}

#[test]
fn provided_file_array_match_partial_reports_unresolved_file_reference() {
    // `attachments=["everywhere"]` is a provided partial for a required
    // `file[](match)` property with no literal match. The classifier must
    // surface `is_array: true` so the CLI can resolve into an array value.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  attachments: 'file(required;match(**/*spec*.md);eager)[]'\n---\nbody\n",
    );
    let overrides = serde_json::json!({ "attachments": ["everywhere"] });

    let err = pre_validate_schema(&source, Some(&overrides), None)
        .expect_err("a provided file[](match) partial with no literal match should surface a typed error");
    match err {
        CompositionError::UnresolvedFileReference {
            property,
            provided,
            patterns,
            is_array,
            reason,
            ..
        } => {
            assert_eq!(property, "attachments");
            assert_eq!(provided, "everywhere");
            assert_eq!(patterns, vec!["**/*spec*.md".to_string()]);
            assert!(is_array, "file[] property must report is_array: true");
            assert!(
                reason.contains("no existing file matched reference"),
                "reason should preserve the original file-reference failure text: {reason}",
            );
        }
        other => panic!("expected UnresolvedFileReference, got {other:?}"),
    }
}

#[test]
fn provided_file_scalar_for_array_property_match_partial_reports_unresolved_file_reference() {
    // A scalar string supplied for a `file[]` property is treated as
    // single-element intent and should still classify as an unresolved
    // file reference with `is_array: true`.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  attachments: 'file(required;match(**/*spec*.md);eager)[]'\n---\nbody\n",
    );
    let overrides = serde_json::json!({ "attachments": "everywhere" });

    let err = pre_validate_schema(&source, Some(&overrides), None)
        .expect_err("a scalar partial for file[](match) should surface a typed error");
    match err {
        CompositionError::UnresolvedFileReference {
            property,
            provided,
            is_array,
            ..
        } => {
            assert_eq!(property, "attachments");
            assert_eq!(provided, "everywhere");
            assert!(is_array, "scalar provided to file[] property must still report is_array: true");
        }
        other => panic!("expected UnresolvedFileReference, got {other:?}"),
    }
}

#[test]
fn provided_partial_value_handles_scalar_and_array_for_file_array() {
    assert_eq!(
        provided_partial_value(Some(&serde_json::json!("everywhere"))),
        Some("everywhere".to_string())
    );
    assert_eq!(
        provided_partial_value(Some(&serde_json::json!(["everywhere"]))),
        Some("everywhere".to_string())
    );
    assert_eq!(
        provided_partial_value(Some(&serde_json::json!(["", "everywhere", "else"]))),
        Some("everywhere".to_string())
    );
    assert_eq!(provided_partial_value(Some(&serde_json::json!([]))), None);
    assert_eq!(
        provided_partial_value(Some(&serde_json::json!(["", "  "]))),
        None
    );
    assert_eq!(
        provided_partial_value(Some(&serde_json::json!([42, true]))),
        None
    );
    assert_eq!(provided_partial_value(Some(&serde_json::json!(42))), None);
}

#[test]
fn provided_file_array_with_non_string_elements_stays_schema_validation() {
    // Non-string array elements are not valid file[] values and must not be
    // misclassified as a partial.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  attachments: 'file(required;match(**/*spec*.md);eager)[]'\n---\nbody\n",
    );
    let overrides = serde_json::json!({ "attachments": [42, true] });

    let err = pre_validate_schema(&source, Some(&overrides), None)
        .expect_err("non-string array elements should fail validation");
    assert!(
        matches!(err, CompositionError::SchemaValidation { .. }),
        "expected SchemaValidation for non-string file[] elements, got {err:?}",
    );
}

#[test]
fn provided_file_without_match_stays_schema_validation() {
    // A bare `file` (no `match(...)` glob) has nothing to walk, so a bad
    // provided value stays the generic SchemaValidation error rather than
    // the resolvable UnresolvedFileReference.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  spec: 'file(required;eager)'\n---\nbody\n",
    );
    let overrides = serde_json::json!({ "spec": "missing/spec.md" });

    let err = pre_validate_schema(&source, Some(&overrides), None)
        .expect_err("a bare-file bad value should still fail validation");
    assert!(
        matches!(err, CompositionError::SchemaValidation { .. }),
        "expected SchemaValidation for a bare file property, got {err:?}",
    );
}

#[test]
fn value_needs_composition_detects_nested_templates() {
    assert!(value_needs_composition(Some(&serde_json::json!(
        "{{ env.X }}"
    ))));
    assert!(value_needs_composition(Some(&serde_json::json!([
        "a", "{{ x }}"
    ]))));
    assert!(value_needs_composition(Some(&serde_json::json!({
        "nested": "{{ x }}"
    }))));
    assert!(!value_needs_composition(Some(&serde_json::json!("plain"))));
    assert!(!value_needs_composition(Some(&serde_json::json!(42))));
    assert!(!value_needs_composition(None));
    // Frontmatter shell expressions (`$(...)`) must also defer.
    assert!(value_needs_composition(Some(&serde_json::json!(
        "$(echo small)"
    ))));
    assert!(value_needs_composition(Some(&serde_json::json!([
        "a",
        "$(echo b)"
    ]))));
}

// -- post-shell validation -------------------------------------------

#[cfg(unix)]
fn approve_echo() -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    set.insert("echo small".to_string());
    set.insert("echo medium".to_string());
    set.insert("echo large".to_string());
    set.insert("echo huge".to_string());
    set
}

#[cfg(unix)]
#[test]
fn post_shell_valid_value_passes() {
    // `$(echo small)` resolves to a valid enum member during shell
    // expansion. Post-shell validation must accept the composed value.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  tier: 'enum(small, medium, large; required)'\n",
            "tier: $(echo small)\n",
            "---\nbody\n",
        ),
    );

    let opts = PrepareOptions {
        pre_approved_commands: Some(approve_echo()),
        ..Default::default()
    };
    let prepared = prepare_direct_with_schema(&source, opts).unwrap();
    let tier = prepared
        .effective_frontmatter
        .as_object()
        .unwrap()
        .get("tier")
        .and_then(|v| v.as_str())
        .unwrap();
    assert_eq!(tier, "small");
    assert!(prepared.dropped_optionals.is_empty());
}

#[cfg(unix)]
#[test]
fn post_shell_invalid_required_returns_schema_validation_error() {
    // `$(echo huge)` produces a value that is NOT a member of the
    // enum. Post-shell validation must surface a SchemaValidation
    // error so the provider is never launched on bad final input.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  tier: 'enum(small, medium, large; required)'\n",
            "tier: $(echo huge)\n",
            "---\nbody\n",
        ),
    );

    let opts = PrepareOptions {
        pre_approved_commands: Some(approve_echo()),
        ..Default::default()
    };
    let err = prepare_direct_with_schema(&source, opts).unwrap_err();
    assert!(
        matches!(err, CompositionError::SchemaValidation { .. }),
        "expected post-shell SchemaValidation, got: {err:?}"
    );
}

#[cfg(unix)]
#[test]
fn post_shell_invalid_optional_is_dropped_with_diagnostic() {
    // Optional `tier` becomes invalid after shell expansion. Drop it
    // from the effective frontmatter, track the drop, and let the run
    // continue.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  tier: 'enum(small, medium, large)'\n",
            "title: Plan\n",
            "tier: $(echo huge)\n",
            "---\nbody\n",
        ),
    );

    let opts = PrepareOptions {
        pre_approved_commands: Some(approve_echo()),
        ..Default::default()
    };
    let prepared = prepare_direct_with_schema(&source, opts).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert!(
        !fm.contains_key("tier"),
        "invalid optional `tier` should have been dropped post-shell"
    );
    let drops: Vec<_> = prepared
        .dropped_optionals
        .iter()
        .filter(|d| d.property == "tier")
        .collect();
    assert_eq!(drops.len(), 1, "expected one post-shell drop diagnostic");
    assert_eq!(drops[0].stage, DroppedOptionalStage::PostShellExpansion);
}

#[test]
fn pre_validation_drop_surfaces_diagnostic() {
    // A file-authored invalid optional value should produce a
    // DroppedOptional diagnostic from pre-validation.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        "---\n$schema:\n  count: 'number'\ncount: nope\n---\nbody\n",
    );

    let pre = pre_validate_schema(&source, None, None).unwrap();
    assert_eq!(pre.dropped_optionals.len(), 1);
    assert_eq!(pre.dropped_optionals[0].property, "count");
    assert_eq!(
        pre.dropped_optionals[0].source,
        DroppedOptionalSource::Frontmatter
    );
    assert_eq!(
        pre.dropped_optionals[0].stage,
        DroppedOptionalStage::PreValidation
    );
}

// ── Phase 4 regression: $schema references stay document-relative when ──
// ── a file-reference fallback is threaded into claudine's schema path. ──
//
// Re-affirms Phase 2B (darkmatter `DarkmatterSchemas`) at the claudine
// integration level: claudine's `load_effective_schema` builds
// `DarkmatterSchemas` with `with_file_ref_fallback_dir`, and the
// `$schema` REFERENCE resolution must stay document-relative while only
// `file`-typed property VALUES use the fallback (verification goal #6).

/// `$schema: ./schema.yaml` resolves relative to the document directory
/// even when `load_effective_schema` is given a fallback dir that does
/// NOT contain the schema file. If the fallback leaked into `$schema`
/// reference resolution, this would fail with `SchemaLoad`.
#[test]
fn schema_reference_stays_document_relative_through_claudine_load() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    // schema.yaml lives ONLY under the document dir.
    fs::write(
        doc_dir.path().join("schema.yaml"),
        "title: string(required)\n",
    )
    .unwrap();
    let source = make_source(
        &doc_dir,
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nbody\n",
    );

    // Fallback points at a dir WITHOUT schema.yaml.
    let effective = load_effective_schema(&source, Some(fallback_dir.path())).unwrap();
    assert!(
        effective.is_some(),
        "$schema reference must resolve from the document dir, not the fallback",
    );
}

/// A root-union `$schema` with a string arm referencing a YAML file also
/// resolves that arm relative to the document directory, not the fallback
/// (verification goal #6, root-union variant through claudine's path).
#[test]
fn root_union_schema_string_arm_stays_document_relative_through_claudine_load() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    fs::write(
        doc_dir.path().join("arm-a.yaml"),
        "kind: string(required)\n",
    )
    .unwrap();
    let source = make_source(
        &doc_dir,
        "---\n$schema:\n  - ./arm-a.yaml\n  - fallback: string\nkind: feature\n---\nbody\n",
    );

    let effective = load_effective_schema(&source, Some(fallback_dir.path())).unwrap();
    assert!(
        effective.is_some(),
        "root-union $schema string arm must resolve from the document dir, not the fallback",
    );
}

/// A `file`-typed schema property value and `{{file_exists(spec)}}` agree
/// across prepare-time body interpolation and post-`chdir` schema
/// validation when both carry the same launch-area fallback
/// (verification goal #7, schema + body dimensions).
///
/// The event-time dimension (`{{file_exists(spec)}}` in a lifecycle
/// event) is covered by `prepare_time_and_event_time_agree_on_file_reference`
/// in `lifecycle_executor::tests`; this test asserts the schema validator
/// agrees with the body interpolation path so all three surfaces align.
#[test]
fn file_property_and_file_exists_agree_across_schema_and_body() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    // spec.md lives under the document directory (base_dir).
    fs::write(doc_dir.path().join("spec.md"), "# Spec\n").unwrap();

    // The prompt declares a `file`-typed `spec` property and a body
    // `{{file_exists(spec)}}`. Both must agree: schema validation passes
    // (spec resolves against the document dir) AND body interpolation renders
    // true. The launch-area fallback is diagnostic-only (D2).
    let source = make_source(
        &doc_dir,
        "---\n\
         $schema:\n\
         \x20 spec: 'file(eager; required)'\n\
         spec: spec.md\n\
         ---\n\
         result: {{file_exists(spec)}}\n",
    );

    let options = PrepareOptions {
        file_ref_fallback_dir: Some(fallback_dir.path().to_path_buf()),
        ..Default::default()
    };

    // Prepare threads the fallback into both Darkmatter composition
    // (body interpolation) and DarkmatterSchemas (schema validation).
    let prepared = prepare_direct_with_schema(&source, options).unwrap();

    // Schema validation passed: spec resolved against the document dir (no
    // SchemaValidation error was returned). The body interpolated
    // file_exists(spec) to `true`, agreeing with the schema's verdict.
    let prompt = &prepared.prompt;
    assert!(
        prompt.contains("result: true"),
        "body `{{{{file_exists(spec)}}}}` must agree with schema validation (both true) via \
         the shared document-dir anchor: {prompt:?}",
    );
}

/// Restores the process CWD on drop. CWD-mutating tests are serialized.
struct CwdGuard {
    prior: std::path::PathBuf,
}

impl CwdGuard {
    fn enter(dir: &std::path::Path) -> Self {
        let prior = std::env::current_dir().expect("read CWD");
        std::env::set_current_dir(dir).expect("set CWD");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

/// Returns a resolved file-backed prompt whose source directory is `dir`.
fn make_source_in(dir: &std::path::Path, document: &str) -> ResolvedCompositionSource {
    let file = dir.join("prompt.md");
    fs::write(&file, document).unwrap();
    resolve_composition_source(file.to_str().unwrap()).unwrap()
}

/// `pre_validate_schema` with a `file(required)` value resolves against the
/// document directory (`base_dir`), not the ambient CWD and not the launch-area
/// fallback (which is diagnostic-only, D2). Proves the document dir drives
/// resolution independently of the process CWD, which is switched to an
/// unrelated directory.
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn pre_validate_schema_resolves_file_against_document_dir() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    // spec.md lives under the document directory (base_dir).
    fs::write(doc_dir.path().join("spec.md"), "# Spec\n").unwrap();

    let source = make_source_in(
        doc_dir.path(),
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n---\nbody\n",
    );

    let _cwd = CwdGuard::enter(unrelated.path());
    let pre = pre_validate_schema(&source, None, Some(fallback_dir.path()))
        .expect("spec.md under the document dir must validate, CWD-independently");
    assert!(pre.dropped_optionals.is_empty());
}

/// A required eager value present only under the launch directory is not a
/// document-authored resolution candidate.
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn pre_validate_schema_rejects_launch_only_file() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    fs::write(fallback_dir.path().join("spec.md"), "# Spec\n").unwrap();

    let source = make_source_in(
        doc_dir.path(),
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n---\nbody\n",
    );

    let _cwd = CwdGuard::enter(unrelated.path());
    let err = pre_validate_schema(&source, None, None)
        .expect_err("launch-only spec.md must be unreachable from the document context");
    assert!(
        matches!(err, CompositionError::SchemaValidation { .. }),
        "expected SchemaValidation for an unresolvable file value, got: {err:?}",
    );
}

/// A launch-only eager file supplied by the caller is invocation input, not a
/// document-authored reference. Pre-validation leaves its final resolution to
/// canonical preparation, which retains the caller's launch-area provenance.
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn pre_validate_schema_defers_caller_eager_file_to_canonical_preparation() {
    let doc_dir = TempDir::new().unwrap();
    let launch_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    fs::write(launch_dir.path().join("spec.md"), "# Spec\n").unwrap();

    for schema in ["file(required;eager)", "'file(required;eager)'"] {
        let source = make_source_in(
            doc_dir.path(),
            &format!("---\n$schema:\n  spec: {schema}\n---\nbody\n"),
        );
        let overrides = serde_json::json!({ "spec": "spec.md" });

        let _cwd = CwdGuard::enter(unrelated.path());
        let pre = pre_validate_schema(&source, Some(&overrides), Some(launch_dir.path()))
            .expect("caller-originated `spec.md` must reach canonical preparation");
        assert_eq!(pre.set_overrides, Some(overrides));
    }
}

/// A captured launch directory alone does not turn a non-resolving caller
/// value into a deferred file. Partial values must still reach the typed
/// interactive-resolution path, including both accepted `file[]` forms.
#[test]
fn pre_validate_schema_keeps_unresolved_caller_file_partials_interactive() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("everywhere-spec.md"), "# Spec\n").unwrap();

    for (schema, overrides, property, is_array) in [
        (
            "spec: 'file(required;match(**/*spec*.md);eager)'",
            serde_json::json!({ "spec": "everywhere" }),
            "spec",
            false,
        ),
        (
            "attachments: 'file(required;match(**/*spec*.md);eager)[]'",
            serde_json::json!({ "attachments": "everywhere" }),
            "attachments",
            true,
        ),
        (
            "attachments: 'file(required;match(**/*spec*.md);eager)[]'",
            serde_json::json!({ "attachments": ["everywhere"] }),
            "attachments",
            true,
        ),
    ] {
        let source = make_source(
            &dir,
            &format!("---\n$schema:\n  {schema}\n---\nbody\n"),
        );

        let err = pre_validate_schema(&source, Some(&overrides), Some(dir.path()))
            .expect_err("the exact `everywhere` partial must remain interactive");
        match err {
            CompositionError::UnresolvedFileReference {
                property: actual_property,
                provided,
                is_array: actual_is_array,
                ..
            } => {
                assert_eq!(actual_property, property);
                assert_eq!(provided, "everywhere");
                assert_eq!(actual_is_array, is_array);
            }
            other => panic!("expected UnresolvedFileReference, got {other:?}"),
        }
    }
}

/// Captured launch metadata does not displace the source-local candidate when
/// no repository candidate exists.
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn pre_validate_schema_ignores_launch_copy_when_source_exists() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    fs::write(doc_dir.path().join("spec.md"), "# doc copy\n").unwrap();
    fs::write(fallback_dir.path().join("spec.md"), "# fallback copy\n").unwrap();

    let source = make_source_in(
        doc_dir.path(),
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n---\nbody\n",
    );

    let _cwd = CwdGuard::enter(unrelated.path());
    pre_validate_schema(&source, None, Some(fallback_dir.path()))
        .expect("the source-local value must validate independently of the launch copy");
}

/// Optional eager-file failures remain visible for the later schema error
/// path instead of being silently removed during pre-validation.
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn drop_invalid_optionals_keeps_unresolvable_eager_file_for_validation() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    fs::write(fallback_dir.path().join("notes.md"), "# Notes\n").unwrap();

    let source = make_source_in(
        doc_dir.path(),
        "---\n$schema:\n  notes: 'file(eager)'\nnotes: notes.md\n---\nbody\n",
    );

    let _cwd = CwdGuard::enter(unrelated.path());
    let (scrubbed, _overrides, dropped) =
        drop_invalid_optionals(source, None, Some(fallback_dir.path()));

    assert!(
        scrubbed
            .markdown
            .frontmatter()
            .as_map()
            .contains_key("notes"),
        "optional `notes` must remain visible for the schema validation error",
    );
    assert!(
        dropped.iter().all(|d| d.property != "notes"),
        "pre-validation must not emit a drop diagnostic for the eager-file failure",
    );
}

/// Companion negative: WITHOUT the fallback, the same optional eager
/// `file` value is unresolvable from the unrelated CWD. It must still be
/// kept by the pre-preflight scrubber because optional eager file failures
/// intentionally remain visible for the later schema error path instead of
/// being silently dropped.
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn drop_invalid_optionals_keeps_unresolved_eager_file_when_no_fallback() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    fs::write(fallback_dir.path().join("notes.md"), "# Notes\n").unwrap();

    let source = make_source_in(
        doc_dir.path(),
        "---\n$schema:\n  notes: 'file(eager)'\nnotes: notes.md\n---\nbody\n",
    );

    let _cwd = CwdGuard::enter(unrelated.path());
    let (scrubbed, _overrides, dropped) = drop_invalid_optionals(source, None, None);

    assert!(
        scrubbed
            .markdown
            .frontmatter()
            .as_map()
            .contains_key("notes"),
        "unresolvable optional eager file values must remain visible for schema validation",
    );
    assert!(
        dropped.iter().all(|d| d.property != "notes"),
        "no drop diagnostic should be emitted for an optional eager file value",
    );
}

/// Sequence phase 1C analog: each sequence step pre-validates via
/// `pre_validate_schema(source, Some(step_overrides), launch_area)` before
/// per-step prepare (see `wrap::sequence::phase1c`). A step whose `file`
/// value comes through the per-step overlay (`set_overrides`) resolves against
/// the document directory (`base_dir`), CWD-independently; the launch-area
/// fallback is diagnostic-only (D2).
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn sequence_step_pre_validation_resolves_file_against_document_dir() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    // The overlay-supplied spec lives under the document directory (base_dir).
    fs::write(doc_dir.path().join("step-spec.md"), "# Step Spec\n").unwrap();

    // The document declares a required `file` but supplies no value; the
    // per-step overlay (`set_overrides`) provides it, mirroring how
    // phase1c feeds `overlay.as_set_overrides(...)` into pre-validation.
    let source = make_source_in(
        doc_dir.path(),
        "---\n$schema:\n  spec: 'file(eager; required)'\n---\nbody\n",
    );
    let step_overrides = serde_json::json!({ "spec": "step-spec.md" });

    let _cwd = CwdGuard::enter(unrelated.path());
    pre_validate_schema(&source, Some(&step_overrides), Some(fallback_dir.path()))
        .expect("a per-step file value under the document dir must pass sequence pre-validation");
}

/// `build_schema_status_report` reports a `file`-typed value that resolves
/// against the document directory (`base_dir`) as `Valid`, not `Invalid` — so
/// the pre-prompt diagnostic agrees with the prepare pipeline instead of
/// flagging a value that will in fact validate. CWD-independent; the launch-area
/// fallback is diagnostic-only (D2).
#[test]
#[serial_test::serial(schema_validation_cwd)]
fn status_report_marks_document_dir_file_valid() {
    let doc_dir = TempDir::new().unwrap();
    let fallback_dir = TempDir::new().unwrap();
    let unrelated = TempDir::new().unwrap();
    fs::write(doc_dir.path().join("spec.md"), "# Spec\n").unwrap();

    let source = make_source_in(
        doc_dir.path(),
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n---\nbody\n",
    );

    let _cwd = CwdGuard::enter(unrelated.path());
    let report = build_schema_status_report(&source, None, Some(fallback_dir.path()))
        .unwrap()
        .unwrap();
    let spec = report
        .required
        .iter()
        .find(|s| s.name == "spec")
        .expect("spec listed");
    assert_eq!(
        spec.state,
        PropertyState::Valid,
        "a file value resolvable against the document dir must report Valid: {spec:?}",
    );
}
