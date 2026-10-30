//! A raw JSON Schema root `anyOf` is judged with every assertion beside it.
//!
//! Root unions are validated arm by arm, so each arm is projected into a
//! standalone schema. That projection keeps the root's sibling assertions
//! (`type`, `required`, ...) and boolean arms, and a malformed root `anyOf`
//! is a preparation failure instead of an unconstrained arm. A root
//! `unevaluatedProperties` or `unevaluatedItems` sees what every accepting
//! arm evaluated, as it does for the complete schema. Every factory and the
//! public `Prepared` reach the same compiled validators, so each is checked
//! here in both modes.

use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use simplified_schema::{
    Prepared, SchemaError, SimplifiedSchema, ValidationFailure, ValidationProblemCode, ValidationSuccess,
};

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

/// The root requires `name`; the arms only discriminate on `kind`.
fn required_beside_any_of() -> Value {
    json!({
        "$schema": DRAFT,
        "type": "object",
        "required": ["name"],
        "anyOf": [
            {"properties": {"kind": {"const": "a"}}},
            {"properties": {"kind": {"const": "b"}}}
        ]
    })
}

fn raw_schema(schema: &Value) -> SimplifiedSchema {
    SimplifiedSchema::try_from(schema.to_string().as_str()).expect("raw JSON Schema content")
}

#[track_caller]
fn assert_missing_name(result: Result<ValidationSuccess<'_>, ValidationFailure>) {
    match result {
        Err(ValidationFailure::InvalidData { problems }) => assert!(
            problems
                .iter()
                .any(|problem| problem.code == ValidationProblemCode::MissingRequired && problem.message.contains("name")),
            "expected a missing-required `name` problem, got {problems:?}"
        ),
        other => panic!("expected a missing-required failure, got {other:?}"),
    }
}

#[track_caller]
fn assert_unprepared(result: Result<ValidationSuccess<'_>, ValidationFailure>) {
    match result {
        Err(ValidationFailure::Unprepared { error }) => {
            assert!(matches!(*error, SchemaError::BuildValidator { .. }), "expected BuildValidator, got {error:?}");
        }
        other => panic!("expected a preparation failure, got {other:?}"),
    }
}

#[test]
fn root_required_beside_any_of_is_enforced_by_the_content_factory_strictly() {
    let schema = raw_schema(&required_beside_any_of());

    assert_missing_name(schema.validate_strict(&json!({"kind": "a"})));
    assert!(schema.validate_strict(&json!({"kind": "a", "name": "Ada"})).is_ok());
}

#[test]
fn root_required_beside_any_of_is_enforced_by_the_content_factory_softly() {
    let schema = raw_schema(&required_beside_any_of());

    assert_missing_name(schema.validate_soft(&json!({"kind": "a"})));
    assert!(schema.validate_soft(&json!({"kind": "b", "name": "Ada"})).is_ok());
}

#[test]
fn root_required_beside_any_of_is_enforced_by_prepared() {
    let prepared = Prepared::new(
        Arc::new(required_beside_any_of()),
        None,
        FileResolutionContext::new(std::env::temp_dir()),
    )
    .expect("schema compiles");
    let data = json!({"kind": "a"});

    assert_missing_name(prepared.validate_strict(&data));
    assert_missing_name(prepared.validate_soft(&data));
    assert!(prepared.validate_strict(&json!({"kind": "a", "name": "Ada"})).is_ok());
}

#[test]
fn root_type_beside_any_of_is_enforced() {
    let schema = raw_schema(&required_beside_any_of());

    assert!(schema.validate_strict(&json!(["kind", "a"])).is_err());
    assert!(schema.validate_soft(&json!(7)).is_err());
}

#[test]
fn root_union_of_false_arms_rejects_every_value() {
    let schema = raw_schema(&json!({"$schema": DRAFT, "anyOf": [false, false]}));

    for value in [json!({}), json!({"name": "Ada"}), json!(null), json!(7)] {
        assert!(
            matches!(schema.validate_strict(&value), Err(ValidationFailure::InvalidData { .. })),
            "strict accepted {value}"
        );
        assert!(
            matches!(schema.validate_soft(&value), Err(ValidationFailure::InvalidData { .. })),
            "soft accepted {value}"
        );
    }
}

#[test]
fn root_union_with_a_true_arm_accepts_any_value() {
    let schema = raw_schema(&json!({"$schema": DRAFT, "anyOf": [false, true]}));

    assert!(schema.validate_strict(&json!({"anything": 1})).is_ok());
}

#[test]
fn root_union_with_a_non_schema_arm_is_a_preparation_failure() {
    for any_of in [json!([7, {"required": ["name"]}]), json!([7])] {
        let schema = raw_schema(&json!({"$schema": DRAFT, "anyOf": any_of}));

        assert_unprepared(schema.validate_strict(&json!({"name": "Ada"})));
        assert_unprepared(schema.validate_soft(&json!({"name": "Ada"})));
    }
}

#[test]
fn empty_root_union_is_a_preparation_failure() {
    let schema = raw_schema(&json!({"$schema": DRAFT, "anyOf": []}));

    assert_unprepared(schema.validate_strict(&json!({})));
    assert_unprepared(schema.validate_soft(&json!({})));
}

#[test]
fn root_required_without_any_of_reports_the_required_keyword() {
    let schema = raw_schema(&json!({"$schema": DRAFT, "type": "object", "required": ["name"]}));

    let Err(ValidationFailure::InvalidData { problems }) = schema.validate_strict(&json!({"kind": "a"})) else {
        panic!("expected invalid data");
    };
    assert!(problems.iter().any(|problem| {
        problem.code == ValidationProblemCode::MissingRequired
            && problem.schema_path.as_ref().is_some_and(|path| path.segments().last().map(String::as_str) == Some("required"))
    }));
}

#[test]
fn arm_level_required_is_enforced() {
    let schema = raw_schema(&json!({
        "$schema": DRAFT,
        "anyOf": [{"type": "object", "required": ["name"]}]
    }));

    assert_missing_name(schema.validate_strict(&json!({"kind": "a"})));
    assert_missing_name(schema.validate_soft(&json!({"kind": "a"})));
}

#[test]
fn generated_root_union_keeps_its_arm_diagnostics() {
    let schema = SimplifiedSchema::try_from(
        "$schema:\n  - kind: literal(a)\n    name: string(required)\n  - kind: literal(b)\n    size: number(required)\n",
    )
    .expect("generated root union");

    assert!(schema.validate_strict(&json!({"kind": "b", "size": 3})).is_ok());
    let Err(ValidationFailure::InvalidData { problems }) = schema.validate_strict(&json!({"kind": "a"})) else {
        panic!("expected invalid data");
    };
    assert!(problems.iter().all(|problem| problem.arm_index == Some(0)), "{problems:?}");
    assert!(problems.iter().any(|problem| problem.path == "/name" || problem.property.as_deref() == Some("name")));
}

/// A problem on a property the root declares beside `anyOf` carries that
/// property's description, as it does without the union.
#[test]
fn root_property_beside_any_of_keeps_its_description() {
    let name = json!({"type": "string", "description": "Who is speaking"});
    let with_union = json!({
        "$schema": DRAFT,
        "type": "object",
        "properties": {"name": name},
        "anyOf": [
            {"properties": {"kind": {"const": "a"}}, "required": ["kind"]},
            {"properties": {"kind": {"const": "b"}}, "required": ["kind"]}
        ]
    });
    let without_union = json!({"$schema": DRAFT, "type": "object", "properties": {"name": name}});

    for (label, schema) in [("with `anyOf`", &with_union), ("without `anyOf`", &without_union)] {
        let Err(ValidationFailure::InvalidData { problems }) =
            raw_schema(schema).validate_strict(&json!({"kind": "a", "name": 7}))
        else {
            panic!("{label}: expected invalid data");
        };
        let problem = problems.iter().find(|problem| problem.path == "/name").unwrap_or_else(|| {
            panic!("{label}: expected a `/name` problem, got {problems:?}")
        });
        assert_eq!(problem.description.as_deref(), Some("Who is speaking"), "{label}: {problem:?}");
    }
}

#[test]
fn nested_property_beside_any_of_keeps_its_description() {
    let name = json!({"type": "string", "description": "Who is speaking"});
    let with_union = json!({
        "$schema": DRAFT,
        "type": "object",
        "properties": {"speaker": {
            "type": "object",
            "properties": {"name": name},
            "anyOf": [
                {"properties": {"kind": {"const": "a"}}, "required": ["kind"]},
                {"properties": {"kind": {"const": "b"}}, "required": ["kind"]}
            ]
        }}
    });
    let without_union = json!({
        "$schema": DRAFT,
        "type": "object",
        "properties": {"speaker": {"type": "object", "properties": {"name": name}}}
    });

    for (label, schema) in [("with `anyOf`", &with_union), ("without `anyOf`", &without_union)] {
        let Err(ValidationFailure::InvalidData { problems }) =
            raw_schema(schema).validate_strict(&json!({"speaker": {"kind": "a", "name": 7}}))
        else {
            panic!("{label}: expected invalid data");
        };
        let problem = problems.iter().find(|problem| problem.path == "/speaker/name").unwrap_or_else(|| {
            panic!("{label}: expected a `/speaker/name` problem, got {problems:?}")
        });
        assert_eq!(problem.description.as_deref(), Some("Who is speaking"), "{label}: {problem:?}");
    }
}

/// Either alternative names the record, both may, and nothing else is
/// allowed: the restriction reads what every accepting arm evaluated.
fn alternatives_with_unevaluated_properties(restriction: Value) -> Value {
    json!({
        "$schema": DRAFT,
        "type": "object",
        "anyOf": [
            {"properties": {"name": {"type": "string"}}, "required": ["name"]},
            {"properties": {"age": {"type": "number"}}, "required": ["age"]}
        ],
        "unevaluatedProperties": restriction
    })
}

fn contains_alternatives_with_unevaluated_items() -> Value {
    json!({
        "$schema": DRAFT,
        "type": "array",
        "anyOf": [{"contains": {"type": "string"}}, {"contains": {"type": "number"}}],
        "unevaluatedItems": false
    })
}

/// The complete schema's verdict, which the facade must agree with.
#[track_caller]
fn complete_validator_accepts(schema: &Value, data: &Value) -> bool {
    jsonschema::validator_for(schema).expect("complete schema compiles").is_valid(data)
}

#[track_caller]
fn assert_accepted_as_written(result: Result<ValidationSuccess<'_>, ValidationFailure>, data: &Value) {
    match result {
        Ok(success) => {
            assert!(!success.is_converted(), "expected the input borrowed, got {success:?}");
            assert_eq!(success.value.as_ref(), data);
        }
        Err(failure) => panic!("expected {data} accepted, got {failure:?}"),
    }
}

#[test]
fn fields_of_two_accepting_arms_pass_unevaluated_properties_in_the_content_factory_strictly() {
    let raw = alternatives_with_unevaluated_properties(json!(false));
    let data = json!({"name": "Ada", "age": 36});
    assert!(complete_validator_accepts(&raw, &data));

    assert_accepted_as_written(raw_schema(&raw).validate_strict(&data), &data);
}

#[test]
fn fields_of_two_accepting_arms_pass_unevaluated_properties_in_the_content_factory_softly() {
    let raw = alternatives_with_unevaluated_properties(json!(false));
    let data = json!({"name": "Ada", "age": 36});

    assert_accepted_as_written(raw_schema(&raw).validate_soft(&data), &data);
}

#[test]
fn fields_of_two_accepting_arms_pass_unevaluated_properties_in_prepared() {
    let prepared = Prepared::new(
        Arc::new(alternatives_with_unevaluated_properties(json!(false))),
        None,
        FileResolutionContext::new(std::env::temp_dir()),
    )
    .expect("schema compiles");
    let data = json!({"name": "Ada", "age": 36});

    assert_accepted_as_written(prepared.validate_strict(&data), &data);
    assert_accepted_as_written(prepared.validate_soft(&data), &data);
}

#[test]
fn items_of_two_accepting_arms_pass_unevaluated_items() {
    let raw = contains_alternatives_with_unevaluated_items();
    let data = json!(["Ada", 36]);
    assert!(complete_validator_accepts(&raw, &data));
    let schema = raw_schema(&raw);

    assert_accepted_as_written(schema.validate_strict(&data), &data);
    assert_accepted_as_written(schema.validate_soft(&data), &data);
}

#[test]
fn schema_valued_unevaluated_properties_sees_fields_of_two_accepting_arms() {
    let raw = alternatives_with_unevaluated_properties(json!({"type": "boolean"}));
    let data = json!({"name": "Ada", "age": 36});
    assert!(complete_validator_accepts(&raw, &data));
    let schema = raw_schema(&raw);

    assert_accepted_as_written(schema.validate_strict(&data), &data);
    assert_accepted_as_written(schema.validate_soft(&data), &data);
    assert!(schema.validate_strict(&json!({"name": "Ada", "flag": true})).is_ok());
    assert!(schema.validate_strict(&json!({"name": "Ada", "flag": "yes"})).is_err());
}

#[test]
fn a_single_accepting_arm_passes_unevaluated_restrictions() {
    let object = raw_schema(&alternatives_with_unevaluated_properties(json!(false)));
    let array = raw_schema(&contains_alternatives_with_unevaluated_items());

    for data in [json!({"name": "Ada"}), json!({"age": 36})] {
        assert_accepted_as_written(object.validate_strict(&data), &data);
        assert_accepted_as_written(object.validate_soft(&data), &data);
    }
    let data = json!(["Ada"]);
    assert_accepted_as_written(array.validate_strict(&data), &data);
    assert_accepted_as_written(array.validate_soft(&data), &data);
}

#[test]
fn a_field_no_accepting_arm_evaluates_is_still_rejected() {
    let raw = alternatives_with_unevaluated_properties(json!(false));
    let data = json!({"name": "Ada", "extra": true});
    assert!(!complete_validator_accepts(&raw, &data));
    let schema = raw_schema(&raw);

    for result in [schema.validate_strict(&data), schema.validate_soft(&data)] {
        let Err(ValidationFailure::InvalidData { problems }) = result else {
            panic!("expected `extra` rejected, got {result:?}");
        };
        assert!(
            problems.iter().all(|problem| problem.arm_index == Some(0)),
            "the `name` arm is the closest: {problems:?}"
        );
        assert!(problems.iter().any(|problem| problem.message.contains("extra")), "{problems:?}");
    }
    assert!(schema.validate_strict(&json!(["Ada", true])).is_err());
}

#[test]
fn a_union_with_unevaluated_properties_under_a_property_or_root_all_of_accepts_both_fields() {
    let original = alternatives_with_unevaluated_properties(json!(false));
    let mut nested = original.clone();
    nested.as_object_mut().unwrap().remove("$schema");
    let under_property = json!({
        "$schema": DRAFT,
        "type": "object",
        "properties": {"person": nested}
    });
    let under_all_of = json!({"$schema": DRAFT, "allOf": [nested]});

    for (raw, data) in [
        (under_property, json!({"person": {"name": "Ada", "age": 36}})),
        (under_all_of, json!({"name": "Ada", "age": 36})),
    ] {
        let schema = raw_schema(&raw);
        assert_accepted_as_written(schema.validate_strict(&data), &data);
        assert_accepted_as_written(schema.validate_soft(&data), &data);
    }
}

/// A root property beside the union converts, and the converted value is
/// still judged with every accepting arm's evaluated fields.
#[test]
fn a_root_property_beside_a_restricted_union_converts_softly() {
    let mut raw = alternatives_with_unevaluated_properties(json!(false));
    raw["properties"] = json!({"count": {"type": "number"}});
    let schema = raw_schema(&raw);

    let data = json!({"name": "Ada", "age": 36, "count": "4"});

    let accepted = schema.validate_soft(&data).expect("converted copy accepted");
    assert!(accepted.is_converted());
    assert_eq!(accepted.value.as_ref(), &json!({"name": "Ada", "age": 36, "count": 4}));
}

#[cfg(feature = "file")]
mod files {
    use std::path::Path;

    use biscuit_file::FileResolutionContext;
    use serde_json::json;
    use simplified_schema::{SchemaSource, SimplifiedSchema};

    use super::{
        alternatives_with_unevaluated_properties, assert_accepted_as_written, assert_missing_name,
        required_beside_any_of,
    };

    fn bound(path: &Path, dir: &Path) -> SchemaSource {
        SchemaSource::from(path).bound(FileResolutionContext::new(dir))
    }

    #[track_caller]
    fn assert_both_modes_reject(schema: &SimplifiedSchema) {
        let data = json!({"kind": "a"});
        assert_missing_name(schema.validate_strict(&data));
        assert_missing_name(schema.validate_soft(&data));
    }

    /// Polls a future that never suspends, as `SimplifiedSchema::resolve` does.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("schema resolution does not suspend"),
        }
    }

    #[test]
    fn root_required_beside_any_of_is_enforced_by_the_file_factories() {
        let dir = tempfile::tempdir().unwrap();
        let text = serde_json::to_string_pretty(&required_beside_any_of()).unwrap();
        let json_path = dir.path().join("schema.json");
        let yaml_path = dir.path().join("schema.yaml");
        std::fs::write(&json_path, &text).unwrap();
        std::fs::write(&yaml_path, &text).unwrap();

        assert_both_modes_reject(&SimplifiedSchema::try_from_json_schema(bound(&json_path, dir.path())).unwrap());
        assert_both_modes_reject(&SimplifiedSchema::try_from_yaml(bound(&yaml_path, dir.path())).unwrap());
        assert_both_modes_reject(&block_on(SimplifiedSchema::resolve(bound(&json_path, dir.path()))).unwrap());
        assert_both_modes_reject(&block_on(SimplifiedSchema::resolve(bound(&yaml_path, dir.path()))).unwrap());
    }

    #[test]
    fn fields_of_two_accepting_arms_pass_unevaluated_properties_in_the_file_factories() {
        let dir = tempfile::tempdir().unwrap();
        let text = serde_json::to_string_pretty(&alternatives_with_unevaluated_properties(json!(false))).unwrap();
        let json_path = dir.path().join("schema.json");
        let yaml_path = dir.path().join("schema.yaml");
        std::fs::write(&json_path, &text).unwrap();
        std::fs::write(&yaml_path, &text).unwrap();
        let data = json!({"name": "Ada", "age": 36});

        for schema in [
            SimplifiedSchema::try_from_json_schema(bound(&json_path, dir.path())).unwrap(),
            SimplifiedSchema::try_from_yaml(bound(&yaml_path, dir.path())).unwrap(),
            block_on(SimplifiedSchema::resolve(bound(&json_path, dir.path()))).unwrap(),
            block_on(SimplifiedSchema::resolve(bound(&yaml_path, dir.path()))).unwrap(),
        ] {
            assert_accepted_as_written(schema.validate_strict(&data), &data);
            assert_accepted_as_written(schema.validate_soft(&data), &data);
        }
    }
}
