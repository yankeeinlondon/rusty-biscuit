use darkmatter::markdown::{
    Markdown,
    schemas::{
        DarkmatterSchemas, SchemaPhase, ValidationProblemCode, schema_constraint_descriptors,
        schema_type_descriptors,
    },
};
use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;

fn effective(schema: &str) -> darkmatter::markdown::schemas::EffectiveSchema {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new()
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema")
}

fn phase_valid(schema: &str, instance: Value, phase: SchemaPhase) -> bool {
    effective(schema)
        .validate_for_phase(&instance, phase)
        .expect("phase schema builds")
        .valid
}

#[test]
fn eager_controls_validation_timing_and_required_controls_presence() {
    let schema = "  input: string(eager)\n  output: string(required)\n  optional: number\n";

    assert!(phase_valid(schema, json!({}), SchemaPhase::Launch));
    assert!(phase_valid(schema, json!({ "input": "ready" }), SchemaPhase::Launch));
    assert!(!phase_valid(schema, json!({ "input": [] }), SchemaPhase::Launch));
    assert!(!phase_valid(
        schema,
        json!({ "input": "ready" }),
        SchemaPhase::Completion,
    ));
    assert!(phase_valid(
        schema,
        json!({ "output": "done" }),
        SchemaPhase::Completion,
    ));

    assert!(phase_valid(schema, json!({ "input": null }), SchemaPhase::Launch));
    assert!(!phase_valid(
        schema,
        json!({ "output": null }),
        SchemaPhase::Completion,
    ));
    assert!(!phase_valid(
        "  input: string(required; eager)\n",
        json!({}),
        SchemaPhase::Launch,
    ));
}

#[test]
fn original_voip_schema_launches_with_outputs_absent_and_enforces_them_at_completion() {
    let schema = concat!(
        "  prompt: string(required;eager)\n",
        "  last_updated: date(required)\n",
        "  researched_by: string(required)\n",
        "  products: object[](required)\n",
    );
    let launch = json!({ "prompt": "Research UniFi Talk" });
    assert!(phase_valid(schema, launch.clone(), SchemaPhase::Launch));
    assert!(!phase_valid(schema, launch, SchemaPhase::Completion));
    assert!(phase_valid(
        schema,
        json!({
            "prompt": "Research UniFi Talk",
            "last_updated": "2026-09-05",
            "researched_by": "opencode/glm-5.3",
            "products": [{ "name": "Phone Touch" }],
        }),
        SchemaPhase::Completion,
    ));
}

#[test]
fn completion_observes_types_without_coercing_the_final_instance() {
    let schema = effective("  researched_by: string(required)\n");
    let report = schema.validate_for_phase(
        &json!({ "researched_by": 42 }),
        SchemaPhase::Completion,
    ).expect("phase schema builds");
    assert!(!report.valid);
    assert!(report.problems.iter().any(|problem| {
        problem.instance_path.segments() == ["researched_by"]
            && problem.code == ValidationProblemCode::TypeMismatch
    }));
}

#[test]
fn all_eager_scalar_representations_are_coerced_and_checked() {
    let schema = concat!(
        "  title: string(eager)\n",
        "  count: number(eager)\n",
        "  metadata: object(eager)\n",
        "  published: datetime(eager)\n",
    );
    assert!(phase_valid(
        schema,
        json!({
            "title": "Report",
            "count": 2,
            "metadata": { "source": "native" },
            "published": "2026-09-05T12:30:00Z",
        }),
        SchemaPhase::Launch,
    ));
    assert!(phase_valid(
        schema,
        json!({
            "title": "Report",
            "count": "2",
            "metadata": { "source": "quoted-number" },
            "published": "2026-09-05T12:30:00",
        }),
        SchemaPhase::Launch,
    ));
    assert!(!phase_valid(
        schema,
        json!({
            "title": 7,
            "count": "not-a-number",
            "metadata": [],
            "published": "September 5",
        }),
        SchemaPhase::Launch,
    ));
}

#[test]
fn nested_and_union_eager_presence_is_recursive_and_hoisted() {
    let nested = "  job: \"{ source: string(eager), result: number(required) }(eager)\"\n";
    assert!(phase_valid(nested, json!({}), SchemaPhase::Launch));
    assert!(phase_valid(nested, json!({ "job": {} }), SchemaPhase::Launch));
    assert!(!phase_valid(
        nested,
        json!({ "job": { "source": [] } }),
        SchemaPhase::Launch,
    ));
    assert!(phase_valid(
        nested,
        json!({ "job": { "source": "input" } }),
        SchemaPhase::Launch,
    ));
    assert!(!phase_valid(
        nested,
        json!({ "job": { "source": "input" } }),
        SchemaPhase::Completion,
    ));

    let union = "  value:\n    - string\n    - number(eager)\n";
    assert!(phase_valid(union, json!({}), SchemaPhase::Launch));
    assert!(phase_valid(union, json!({ "value": "quoted" }), SchemaPhase::Launch));
    assert!(phase_valid(union, json!({ "value": 3 }), SchemaPhase::Launch));
}

#[test]
fn eager_array_placement_owns_items_or_property() {
    let item_eager = "  inputs: file(eager)[]\n";
    assert!(phase_valid(item_eager, json!({}), SchemaPhase::Launch));
    assert!(phase_valid(item_eager, json!({ "inputs": null }), SchemaPhase::Launch));

    let property_eager = "  inputs: file[](eager)\n";
    assert!(phase_valid(property_eager, json!({}), SchemaPhase::Launch));
    assert!(phase_valid(property_eager, json!({ "inputs": null }), SchemaPhase::Launch));
    assert!(phase_valid(property_eager, json!({ "inputs": [] }), SchemaPhase::Launch));
}

#[test]
fn array_level_required_owns_presence_independently_of_array_level_eager() {
    // Presence lives in the postfix `[](...)` list for an array property, so
    // phase projection must read `required` from the array constraints rather
    // than only from the item constraints.
    let property_required = "  inputs: file[](required)\n";
    assert!(phase_valid(property_required, json!({}), SchemaPhase::Launch));
    assert!(!phase_valid(property_required, json!({}), SchemaPhase::Completion));
    assert!(!phase_valid(property_required, json!({ "inputs": null }), SchemaPhase::Completion));
    assert!(phase_valid(property_required, json!({ "inputs": [] }), SchemaPhase::Completion));

    let property_required_eager = "  inputs: file[](required; eager)\n";
    assert!(!phase_valid(property_required_eager, json!({}), SchemaPhase::Launch));
    assert!(!phase_valid(property_required_eager, json!({}), SchemaPhase::Completion));
    assert!(phase_valid(property_required_eager, json!({ "inputs": [] }), SchemaPhase::Launch));

    // Either placement of `required` governs property presence, so a reader
    // that inspects only one constraint list is wrong for one of these forms.
    let item_required = "  inputs: file(required)[]\n";
    assert!(!phase_valid(item_required, json!({}), SchemaPhase::Completion));
}

#[test]
fn required_eager_is_mandatory_at_both_phases_including_explicit_null() {
    // The fourth cell of the semantic matrix, asserted end to end rather than
    // only at launch-absence: presence *and* validity at both seams.
    let schema = "  plan: string(required; eager)\n";
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        assert!(!phase_valid(schema, json!({}), phase), "{phase:?}: absence");
        assert!(!phase_valid(schema, json!({ "plan": null }), phase), "{phase:?}: null");
        assert!(!phase_valid(schema, json!({ "plan": [] }), phase), "{phase:?}: wrong type");
        assert!(phase_valid(schema, json!({ "plan": "ready" }), phase), "{phase:?}: supplied");
    }
}

#[test]
fn phase_validation_is_passive_while_unphased_eager_file_semantics_remain() {
    let schema = effective("  source: file(eager)\n");
    let instance = json!({ "source": "definitely-missing-phase-validation-file.md" });
    assert!(!schema.validate(&instance).valid, "ordinary eager validation probes existence");
    assert!(
        schema
            .validate_for_phase(&instance, SchemaPhase::Launch)
            .expect("phase schema builds")
            .valid,
        "phase validation must not resolve or read the file",
    );
}

#[test]
fn generated_required_keeps_authoring_compatibility_but_fails_completion() {
    let schema = effective("  artifact: string(generated; required)\n");
    assert!(schema.validate(&json!({})).valid);
    assert!(schema
        .validate_for_phase(&json!({}), SchemaPhase::Launch)
        .expect("phase schema builds")
        .valid);
    let completion = schema
        .validate_for_phase(&json!({}), SchemaPhase::Completion)
        .expect("phase schema builds");
    assert!(!completion.valid);
    assert!(completion.problems.iter().any(|problem| {
        problem.property.as_deref() == Some("artifact")
            && problem.code == ValidationProblemCode::MissingRequired
    }));
}

#[test]
fn raw_json_schema_keeps_required_at_launch() {
    let dir = tempdir().expect("tempdir");
    fs::write(
        dir.path().join("raw.json"),
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","required":["legacy"],"properties":{"legacy":{"type":"string"}}}"#,
    )
    .expect("write raw schema");
    let document = dir.path().join("document.md");
    fs::write(&document, "---\n$schema: ./raw.json\n---\nbody\n").expect("write document");
    let source = Markdown::try_from(document.as_path()).expect("read document");
    let schema = DarkmatterSchemas::new()
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema");
    assert!(schema.simplified.is_none());
    assert!(!schema
        .validate_for_phase(&json!({}), SchemaPhase::Launch)
        .expect("phase schema builds")
        .valid);
}

#[test]
fn phase_projection_preserves_merged_baseline_properties() {
    use darkmatter::markdown::schemas::parse_yaml_schema;

    let baseline_yaml = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(
        "baseline: string(required)\noutput: string(required)\n",
    )
    .expect("baseline YAML");
    let baseline = parse_yaml_schema(&baseline_yaml).expect("baseline schema");
    let api = DarkmatterSchemas::new()
        .with_baseline(baseline)
        .expect("baseline accepted");
    let source: Markdown = "---\n$schema:\n  output: string(required)\n---\nbody\n".into();
    let schema = api
        .effective_for(&source)
        .expect("effective schema")
        .expect("schema present");

    let launch = schema
        .validate_for_phase(&json!({}), SchemaPhase::Launch)
        .expect("phase schema builds");
    assert!(!launch.valid, "baseline required remains a launch gate");
    assert!(launch.problems.iter().any(|problem| {
        problem.property.as_deref() == Some("baseline")
            && problem.code == ValidationProblemCode::MissingRequired
    }));
    assert!(!launch.problems.iter().any(|problem| {
        problem.property.as_deref() == Some("output")
            && problem.code == ValidationProblemCode::MissingRequired
    }));
}

#[test]
fn independent_conversion_failures_are_aggregated_by_property() {
    use darkmatter::markdown::schemas::{SchemaError, parse_yaml_schema, to_json_schema};

    let yaml = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(
        "first: string(integer)\nsecond: boolean(pattern(x))\nvalid: number\n",
    )
    .expect("schema YAML");
    let schema = parse_yaml_schema(&yaml).expect("schema grammar");
    let error = to_json_schema(&schema).expect_err("two conversion errors");
    let SchemaError::Aggregate { errors } = error else {
        panic!("expected aggregate error");
    };
    assert_eq!(errors.len(), 2);
    let properties = errors
        .iter()
        .map(|error| match error {
            SchemaError::Convert { property, .. } => property.as_str(),
            other => panic!("unexpected aggregate child: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(properties, ["first", "second"]);
}

#[test]
fn nested_conversion_failure_uses_a_dotted_property_path() {
    use darkmatter::markdown::schemas::{SchemaError, parse_yaml_schema, to_json_schema};

    let yaml = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(
        "meta:\n  prompt: string(integer)\n",
    )
    .expect("schema YAML");
    let schema = parse_yaml_schema(&yaml).expect("schema grammar");
    let error = to_json_schema(&schema).expect_err("nested conversion error");
    let SchemaError::Convert { property, .. } = error else {
        panic!("expected one conversion error, got {error:?}");
    };
    assert_eq!(property, "meta.prompt");
}

#[test]
fn eager_is_universal_in_catalog_and_round_trips() {
    let eager = schema_constraint_descriptors()
        .iter()
        .find(|descriptor| descriptor.keyword == "eager")
        .expect("eager descriptor");
    assert_eq!(eager.target_types, "all types");

    // The catalog is the single wording authority for `md schema about`, DMLS
    // completion detail, and both DMLS hover surfaces. Its prose must state the
    // independent-axis contract, not the retired eager-implies-required rule.
    assert!(
        eager
            .description
            .contains("absence is allowed unless `required` is also declared"),
        "eager must disclose that it does not control presence: {}",
        eager.description
    );
    assert!(
        !eager.description.contains("Requires the containing property"),
        "eager must not claim it requires the containing property: {}",
        eager.description
    );
    assert!(
        eager
            .json_schema_effect
            .contains("does not add the property to the parent's `required` list"),
        "eager's JSON Schema effect must disclaim the `required` list: {}",
        eager.json_schema_effect
    );

    for descriptor in schema_type_descriptors() {
        assert!(
            descriptor.accepted_constraints.contains("eager"),
            "{} omits eager",
            descriptor.keyword,
        );
    }

    for definition in [
        "string(eager)",
        "number(eager)",
        "object(eager)",
        "datetime(eager)",
        "file(eager)[]",
        "file[](eager)",
    ] {
        let schema = format!("  value: {definition}\n");
        effective(&schema);
    }
}

#[test]
fn passive_trigger_schemas_reject_eager_on_every_shape_and_placement() {
    use darkmatter::markdown::schemas::{SchemaError, parse_trigger_envelope_from_str};

    for definition in [
        "string(eager)",
        "number(eager)",
        "{ nested: string }(eager)",
        "file(eager)[]",
        "file[](eager)",
    ] {
        let source = format!(
            "kind: trigger-schema\nmatch:\n  candidate: '{definition}'\n$schema:\n  candidate: string\n",
        );
        let error = parse_trigger_envelope_from_str(&source)
            .expect_err("trigger match eager must be rejected");
        assert!(
            matches!(error, SchemaError::TriggerForbiddenConstraint { ref property, ref constraint }
                if property == "candidate" && constraint == "eager"),
            "{definition}: {error:?}",
        );
    }
}

/// Exact claims from the retired eager-implies-`required` model. Each string
/// appeared verbatim in one of the swept documents before 2026-09-07, so the
/// sweep below is a discriminator rather than a spelling check.
const RETIRED_EAGER_CLAIMS: &[&str] = &[
    "the property must be present at stabilized launch and completion",
    "| `eager` | Required | Required |",
    "exactly equivalent to `required; eager`",
    "There is no way to say \"optional, but resolve and check it eagerly when supplied\"",
    "while `file[](eager)` requires the array property at launch",
    "An eager `null` fails at launch",
    "Launch requires recursively declared `eager` properties",
    "completion requires `required` or `eager`",
    "`file[](eager)` owns property presence",
    "Requires the containing property at stabilized launch",
];

/// Markdown wraps prose across lines, so a claim is matched on its
/// whitespace-collapsed form rather than its authored line breaks.
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every explanatory surface that a reader or an agent may consult must state
/// the same independent-axis contract as the descriptor catalog and the phase
/// projection. The prose is checked against the retired claims *and* against
/// the real validator, so neither a doc rewrite nor a runtime change can drift
/// alone.
#[test]
fn public_docs_and_skill_describe_required_and_eager_as_independent_axes() {
    let root = repo_root();
    let surfaces = [
        (
            "darkmatter/docs/topics/schema-definition.md",
            vec![
                "`required` and `eager` are independent axes",
                "| `required; eager` | Required, and the value is validated eagerly | Required |",
                "| `eager` | Optional; a present value is validated eagerly | Optional; present values remain type-checked |",
                "An eager-only `null` is therefore allowed at both phases",
                "Neither placement makes the property required",
            ],
        ),
        (
            "darkmatter/docs/inline/schema-validation.md",
            vec![
                "| `eager` | valid when present; absence is allowed | valid when present; absence is allowed |",
                "| `required; eager` | must be present and valid | must be present and valid |",
                "An eager-only `null` is therefore allowed at both phases",
            ],
        ),
        (
            ".claude/skills/darkmatter/schema.md",
            vec![
                "`required` and `eager` are independent axes",
                "| `eager` | absence allowed; a present value is validated eagerly | absence allowed; a present value is type-checked |",
                "`eager` is universal timing metadata and never controls presence",
            ],
        ),
    ];

    for (relative, expected) in surfaces {
        let path = root.join(relative);
        let text = collapse_whitespace(
            &fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} is unreadable: {error}", path.display())),
        );
        for retired in RETIRED_EAGER_CLAIMS {
            assert!(
                !text.contains(retired),
                "{relative} still asserts the retired claim: {retired}",
            );
        }
        for claim in expected {
            assert!(text.contains(claim), "{relative} no longer states: {claim}");
        }
    }

    // The descriptor catalog is the wording authority DMLS and `md schema
    // about` read; the documents above must agree with it, not with each other.
    let eager = schema_constraint_descriptors()
        .iter()
        .find(|descriptor| descriptor.keyword == "eager")
        .expect("eager descriptor");
    assert!(
        eager
            .description
            .contains("absence is allowed unless `required` is also declared"),
        "descriptor drifted from the documented contract: {}",
        eager.description
    );

    // Re-derive the documented four-cell matrix from the real validator so the
    // prose cannot outlive the projection it describes.
    for (declaration, launch_absence_ok, completion_absence_ok) in [
        ("string", true, true),
        ("string(eager)", true, true),
        ("string(required)", true, false),
        ("string(required; eager)", false, false),
    ] {
        let schema = format!("  value: {declaration}\n");
        assert_eq!(
            phase_valid(&schema, json!({}), SchemaPhase::Launch),
            launch_absence_ok,
            "{declaration}: documented launch absence verdict",
        );
        assert_eq!(
            phase_valid(&schema, json!({ "value": null }), SchemaPhase::Completion),
            completion_absence_ok,
            "{declaration}: documented completion absence verdict",
        );
    }
}

fn repo_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/darkmatter/lib`.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root is two levels above darkmatter/lib")
        .to_path_buf()
}

fn yaml_files_in(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{} is unreadable: {error}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    files.sort();
    files
}

/// Every shipped schema artifact must still resolve through the ordinary
/// `$schema: <file>` path and project both phases, and every shipped trigger
/// must still parse under the `eager`-rejecting trigger grammar. The
/// directories are walked rather than listed so a newly shipped artifact is
/// covered the day it lands.
#[test]
fn shipped_schema_and_trigger_corpus_parses_passively() {
    use darkmatter::markdown::schemas::parse_trigger_envelope_from_str;

    use darkmatter::markdown::schemas::parse_property_definition;

    let root = repo_root();
    let schema_dirs = [
        root.join("darkmatter/docs/schemas"),
        root.join("claudine/docs/schemas"),
        root.join("schemas"),
    ];
    let mut definitions = 0usize;
    for dir in &schema_dirs {
        for path in yaml_files_in(dir) {
            let source = fs::read_to_string(&path).expect("shipped schema is readable");
            // Several shipped names are placeholders reserved for schemas that
            // have not been authored yet.
            if source.trim().is_empty() {
                continue;
            }
            let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source)
                .unwrap_or_else(|error| panic!("{} is not YAML: {error}", path.display()));
            let Some(map) = document.as_mapping() else { continue };
            // An artifact carrying keys Darkmatter's schema loader does not
            // recognize is design documentation, not a loadable schema.
            let loadable = map.keys().all(|key| {
                key.as_str().is_some_and(|key| {
                    matches!(key, "kind" | "name" | "description" | "$schema" | "types")
                })
            });
            if !loadable {
                continue;
            }
            let body = ["$schema", "types"]
                .iter()
                .find_map(|key| map.get(serde_yaml_ng::Value::String((*key).into())))
                .and_then(serde_yaml_ng::Value::as_mapping);
            let Some(body) = body else { continue };

            for (name, value) in body {
                let Some(name) = name.as_str() else { continue };
                // Trigger envelopes and object-arity metadata are not property
                // definitions.
                if name == "$constraints" {
                    continue;
                }
                parse_property_definition(name, value).unwrap_or_else(|error| {
                    panic!("{}: `{name}` does not parse passively: {error}", path.display())
                });
                definitions += 1;
            }
        }
    }
    assert!(
        definitions >= 80,
        "the shipped schema corpus shrank to {definitions} property definitions"
    );

    let triggers: Vec<_> = yaml_files_in(&root.join("darkmatter/tests/fixtures/schema-triggers/schemas"))
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".trigger.yaml"))
        })
        .collect();
    assert!(!triggers.is_empty(), "the shipped trigger corpus must not be empty");
    for path in triggers {
        let source = fs::read_to_string(&path).expect("shipped trigger is readable");
        assert!(
            parse_trigger_envelope_from_str(&source)
                .unwrap_or_else(|error| panic!(
                    "{} does not parse passively: {error}",
                    path.display()
                ))
                .is_some(),
            "{} is not a trigger envelope",
            path.display()
        );
    }
}

/// The real shipped `implement-plan` schema, resolved through the ordinary
/// `effective_for` path, must split its obligations across the two phases:
/// `plan` is `required; eager` and gates launch, while `phase` and
/// `total_phases` are `required` only and are deferred to completion. The
/// optional `spec` is absent from both.
#[test]
fn real_shipped_inline_schema_uses_normal_resolution_and_phase_path() {
    let text = include_str!("../../../claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md");
    let source: Markdown = text.into();
    let schema = DarkmatterSchemas::new()
        .effective_for(&source)
        .expect("shipped schema resolves")
        .expect("the shipped document declares a schema");

    let missing = |instance: &Value, phase| -> Vec<String> {
        let mut names: Vec<String> = schema
            .validate_for_phase(instance, phase)
            .expect("phase schema builds")
            .problems
            .iter()
            .filter_map(|problem| problem.property.clone())
            .collect();
        names.sort();
        names
    };

    let empty = json!({});
    assert_eq!(missing(&empty, SchemaPhase::Launch), vec!["plan".to_string()]);
    assert_eq!(
        missing(&empty, SchemaPhase::Completion),
        vec!["phase".to_string(), "plan".to_string(), "total_phases".to_string()],
    );

    // The shipped document's own null-yielding `spec` expression is the input
    // that used to fail; it must satisfy both phases now that `spec` is
    // optional.
    let satisfied = json!({
        "plan": "features/x/plan.md",
        "phase": 1,
        "total_phases": 3,
        "spec": Value::Null,
    });
    assert!(schema
        .validate_for_phase(&satisfied, SchemaPhase::Launch)
        .expect("phase schema builds")
        .valid);
    assert!(schema
        .validate_for_phase(&satisfied, SchemaPhase::Completion)
        .expect("phase schema builds")
        .valid);
}
