use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::conditions::evaluate_condition_against;
use darkmatter::markdown::compose::directive_targets::{
    TargetNullability, analyze_directive_targets, classify_target_nullability,
    narrowed_expression_paths,
};
use darkmatter::markdown::compose::directives_api::DirectiveKind;
use darkmatter::markdown::compose::expression::{parse, parse_condition};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, SchemaOrigin, SimplifiedSchema, parse_yaml_schema,
};
use serde_json::{Value, json};

fn document_with_schema(properties: &str, values: &str) -> (Markdown, EffectiveSchema, Value) {
    let source = format!("---\n$schema:\n{properties}{values}---\nbody\n");
    let markdown = Markdown::from(source.as_str());
    let effective = DarkmatterSchemas::new()
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("effective schema");
    let frontmatter = Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .filter(|(key, _)| key.as_str() != "$schema")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    (markdown, effective, frontmatter)
}

fn assert_nullable(actual: TargetNullability, expected_root: &str) {
    match actual {
        TargetNullability::Nullable { root } => assert_eq!(root.as_str(), expected_root),
        other => panic!("expected nullable `{expected_root}`, got {other:?}"),
    }
}

#[test]
fn classifies_schema_frontmatter_and_context_nullability_without_guessing() {
    let (_, effective, missing) = document_with_schema(
        "  optional: file\n  defaulted: \"string(default('a'))\"\n  required: 'file(required)'\n  native: string\n  quoted: string\n",
        "",
    );

    assert_nullable(
        classify_target_nullability(&parse("optional").unwrap(), Some(&effective), &missing),
        "optional",
    );
    assert_nullable(
        classify_target_nullability(&parse("doc.defaulted").unwrap(), Some(&effective), &missing),
        "defaulted",
    );
    assert_eq!(
        classify_target_nullability(&parse("required").unwrap(), Some(&effective), &missing),
        TargetNullability::NonNullable,
    );

    let (_, _, explicit_null) = document_with_schema("  optional: file\n", "optional: null\n");
    assert_nullable(
        classify_target_nullability(
            &parse("optional").unwrap(),
            Some(&effective),
            &explicit_null,
        ),
        "optional",
    );

    let (_, _, concrete) = document_with_schema(
        "  optional: file\n  native: string\n  quoted: string\n",
        "optional: ./log.md\nnative: 7\nquoted: \"7\"\n",
    );
    for path in ["optional", "native", "quoted"] {
        assert_eq!(
            classify_target_nullability(&parse(path).unwrap(), Some(&effective), &concrete),
            TargetNullability::NonNullable,
            "concrete scalar `{path}` must suppress nullability",
        );
    }

    assert_eq!(
        classify_target_nullability(&parse("ctx.today").unwrap(), None, &json!({})),
        TargetNullability::NonNullable,
    );
    assert_nullable(
        classify_target_nullability(&parse("ctx.cwd").unwrap(), None, &json!({})),
        "ctx.cwd",
    );
    assert_eq!(
        classify_target_nullability(&parse("ctx.not_cataloged").unwrap(), None, &json!({})),
        TargetNullability::Unknown,
    );

    let mut referenced = effective.clone();
    referenced.origins.insert(
        "optional".into(),
        SchemaOrigin::referenced_file("schema.yaml"),
    );
    assert_nullable(
        classify_target_nullability(&parse("optional").unwrap(), Some(&referenced), &missing),
        "optional",
    );

    for origin in [SchemaOrigin::baseline(), SchemaOrigin::trigger("trigger.yaml")] {
        let mut unsupported = effective.clone();
        unsupported.origins.insert("optional".into(), origin);
        assert_eq!(
            classify_target_nullability(
                &parse("optional").unwrap(),
                Some(&unsupported),
                &missing,
            ),
            TargetNullability::Unknown,
        );
    }

    let mut raw_json = effective.clone();
    raw_json.simplified = None;
    assert_eq!(
        classify_target_nullability(&parse("optional").unwrap(), Some(&raw_json), &missing),
        TargetNullability::Unknown,
    );

    for expression in ["nested.value", "doc.nested.value", "optional || 'fallback'"] {
        assert_eq!(
            classify_target_nullability(&parse(expression).unwrap(), Some(&effective), &missing),
            TargetNullability::Unknown,
            "unsupported expression `{expression}` must remain unknown",
        );
    }
    assert_eq!(
        classify_target_nullability(&parse("optional").unwrap(), None, &missing),
        TargetNullability::Unknown,
    );

    let union_yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("- optional: file\n- optional: file(required)\n").unwrap();
    let union = parse_yaml_schema(&union_yaml).unwrap();
    assert!(matches!(union, SimplifiedSchema::Union(_)));
    let mut root_union = effective.clone();
    root_union.simplified = Some(union);
    root_union.origins.clear();
    assert_eq!(
        classify_target_nullability(&parse("optional").unwrap(), Some(&root_union), &missing),
        TargetNullability::Unknown,
    );
}

#[test]
fn guard_narrowing_matches_evaluator_truth_tables() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("present.md"), "present").unwrap();

    let cases = [
        ("file_exists(x)", json!({"x": "present.md"}), true, &["x"][..]),
        ("x", json!({"x": "value"}), true, &["x"][..]),
        ("!!x", json!({"x": "value"}), true, &["x"][..]),
        ("(x)", json!({"x": "value"}), true, &["x"][..]),
        ("x != null", json!({"x": "value"}), true, &["x"][..]),
        ("x != null", json!({"x": null}), false, &["x"][..]),
        ("x != ''", json!({"x": "value"}), true, &["x"][..]),
        ("x != ''", json!({"x": ""}), false, &["x"][..]),
        ("a && b", json!({"a": true, "b": true}), true, &["a", "b"][..]),
        ("a || b", json!({"a": true, "b": false}), true, &[][..]),
        ("!x", json!({"x": false}), true, &[][..]),
        ("x == 'value'", json!({"x": "value"}), true, &[][..]),
        ("length(x) > 0", json!({"x": [1]}), true, &[][..]),
    ];

    for (condition, values, expected_result, expected_paths) in cases {
        assert_eq!(
            evaluate_condition_against(condition, &values, temp.path()).unwrap(),
            expected_result,
            "evaluator result for `{condition}`",
        );
        let parsed = parse_condition(condition).unwrap();
        let actual: Vec<_> = narrowed_expression_paths(&parsed)
            .into_iter()
            .map(|path| path.as_str().to_string())
            .collect();
        assert_eq!(actual, expected_paths, "narrowed paths for `{condition}`");
    }
}

#[test]
fn integrates_whole_targets_with_direct_and_nested_guard_narrowing() {
    let (_, effective, frontmatter) = document_with_schema(
        "  log: file\n  outer: file\n  required: 'file(required)'\n  bound: file\n",
        "bound: real.md\n",
    );
    let source = "::file {{log}}\n\n::block when=\"file_exists(log)\"\n::code {{log}}\n::end-block\n\n::block when=\"file_exists(outer)\"\n::block when=\"true\"\n::url {{outer}}\n::end-block\n::end-block\n\n::file {{required}}\n::file {{bound}}\n::file \"{{log}}/mixed.md\"\n::file {{log || 'fallback.md'}}\n";

    let analyses = analyze_directive_targets(source, Some(&effective), &frontmatter).unwrap();
    assert_eq!(analyses.len(), 6);

    let unguarded = &analyses[0];
    assert_eq!(unguarded.kind, DirectiveKind::File);
    assert_eq!(&source[unguarded.expression_span.clone()], "{{log}}");
    assert_eq!(unguarded.root.as_ref().unwrap().as_str(), "log");
    assert_nullable(unguarded.nullability.clone(), "log");
    assert!(!unguarded.is_narrowed());

    let direct = &analyses[1];
    assert_eq!(direct.kind, DirectiveKind::Code);
    assert!(direct.is_narrowed());
    assert_eq!(direct.narrowed_paths[0].as_str(), "log");

    let nested = &analyses[2];
    assert_eq!(nested.kind, DirectiveKind::Url);
    assert!(nested.is_narrowed());
    assert_eq!(nested.narrowed_paths[0].as_str(), "outer");

    assert_eq!(analyses[3].nullability, TargetNullability::NonNullable);
    assert_eq!(analyses[4].nullability, TargetNullability::NonNullable);
    assert_eq!(analyses[5].nullability, TargetNullability::Unknown);
    assert!(analyses[5].root.is_none());
}

#[test]
fn passive_analysis_does_not_execute_or_mutate_inputs() {
    let (_, effective, frontmatter) = document_with_schema(
        "  remote: file\n  command: string\n",
        "remote: https://example.invalid/never-fetch.md\ncommand: 'touch should-not-exist'\n",
    );
    let source = "::file {{remote}}\n::code {{command}}\n";
    let original_source = source.to_string();
    let original_frontmatter = frontmatter.clone();
    #[cfg(feature = "effects-instrumentation")]
    let before_effects = (
        darkmatter::effects::engine_build_count(),
        darkmatter::effects::network_attempt_count(),
    );

    let analyses = analyze_directive_targets(source, Some(&effective), &frontmatter).unwrap();

    assert_eq!(analyses.len(), 2);
    assert_eq!(source, original_source);
    assert_eq!(frontmatter, original_frontmatter);
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(
        (
            darkmatter::effects::engine_build_count(),
            darkmatter::effects::network_attempt_count(),
        ),
        before_effects,
    );
    assert!(!std::path::Path::new("should-not-exist").exists());
}
