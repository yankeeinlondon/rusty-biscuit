//! The generated lifecycle verb types judge operands through the public
//! schema API the way a host references them (`expression[]@darkmatter:lifecycle`,
//! `bundle[]@darkmatter:lifecycle`, `<verb>-short@darkmatter:lifecycle`), and
//! their short forms reach the same verdict as the passive call binder
//! (`validate_operands`) for every overloaded verb.

use std::collections::BTreeSet;
use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::effects::effect_registration;
use darkmatter::lifecycle::schema_projection::{
    LifecycleSchemaProjection, ProjectedVerb, TEMPLATE_OPERAND, template_operand_type,
};
use darkmatter::lifecycle::{OperandVerdict, validate_operands};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::functions::function_registration;
use darkmatter::markdown::compose::expression::{CallErrorKind, FunctionRegistration};
use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
use darkmatter::markdown::schemas::simplified::{
    FunctionType, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr, serialize_property_atom, serialize_property_def,
};
use darkmatter::markdown::schemas::validate::build_position_map;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, ValidationReport};
use serde_json::{Value, json};
use serde_yaml_ng::{Mapping, Value as YamlValue};

/// The effective schema of a document whose `$schema` mapping is `wiring`
/// (`property: type` pairs), resolved from `dir`.
fn effective(dir: &Path, wiring: &[(&str, String)]) -> EffectiveSchema {
    let mut schema = String::from("---\n$schema:\n");
    for (property, ty) in wiring {
        schema.push_str(&format!("    {property}: \"{ty}\"\n"));
    }
    let source: Markdown = format!("{schema}---\nbody\n").into();
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .effective_for(&source)
        .expect("the schema resolves")
        .expect("an effective schema")
}

fn validate(schema: &EffectiveSchema, yaml: &str) -> ValidationReport {
    let value: Value = serde_yaml_ng::from_str(yaml).expect("fixture YAML parses");
    schema.validate_with_positions(&value, &build_position_map(yaml))
}

// ── Host-shaped validation ──────────────────────────────────────────────────

/// A list of `expression` actions and a list of `bundle`s, as a host's event
/// stack references them.
fn host_schema() -> EffectiveSchema {
    effective(
        &std::env::current_dir().expect("process directory"),
        &[
            ("actions", "expression[]@darkmatter:lifecycle".to_string()),
            ("bundles", "bundle[]@darkmatter:lifecycle".to_string()),
        ],
    )
}

#[track_caller]
fn assert_action_passes(schema: &EffectiveSchema, action: &str) {
    let yaml = format!("actions:\n  - {action}\n");
    let report = validate(schema, &yaml);
    assert!(report.valid, "rejected `{action}`: {:#?}", report.problems);
}

/// The action fails with a positioned problem at `path` (below `/actions/0`)
/// or below it: a union reports the arm closest to the value.
#[track_caller]
fn assert_action_fails_at(schema: &EffectiveSchema, action: &str, path: &str) {
    let yaml = format!("actions:\n  - {action}\n");
    let report = validate(schema, &yaml);
    let full = format!("/actions/0{path}");
    let problem = report
        .problems
        .iter()
        .find(|problem| problem.path == full || problem.path.starts_with(&format!("{full}/")))
        .unwrap_or_else(|| panic!("`{action}`: no problem at {full}: {:#?}", report.problems));
    assert!(!report.valid);
    assert!(problem.line == Some(2) && problem.column.is_some(), "`{action}` is unpositioned: {problem:#?}");
}

#[test]
fn numeric_operands_follow_the_shared_conversion_rules() {
    let schema = host_schema();

    assert_action_passes(&schema, "{ pr_list: 3 }");
    assert_action_passes(&schema, "{ pr_list: \"3\" }");
    assert_action_passes(&schema, "{ pr_list: [3] }");
    assert_action_passes(&schema, "{ pr_list: { count: \"3\" } }");
    assert_action_fails_at(&schema, "{ pr_list: three }", "/pr_list");
    assert_action_fails_at(&schema, "{ pr_list: 0 }", "/pr_list");
    assert_action_fails_at(&schema, "{ pr_list: 101 }", "/pr_list");
    assert_action_fails_at(&schema, "{ pr_list: { count: 101 } }", "/pr_list/count");
}

#[test]
fn a_template_operand_is_deferred_and_its_literal_siblings_are_judged() {
    let schema = host_schema();

    assert_action_passes(&schema, "{ pr_list: \"{{ fm.count }}\" }");
    assert_action_passes(&schema, "{ pr_list: { count: \"{{ fm.count }}\" } }");
    assert_action_passes(&schema, "{ min: [1, \"{{ fm.x }}\"] }");
    assert_action_fails_at(&schema, "{ min: [three, \"{{ fm.x }}\"] }", "/min/0");
    assert_action_fails_at(&schema, "{ min: [1, \"{{ fm.x }}\", 3] }", "/min/2");
}

#[test]
fn a_template_never_defers_the_long_form_keys() {
    let schema = host_schema();

    assert_action_fails_at(&schema, "{ pr_list: { count: \"{{ fm.count }}\", bogus: 1 } }", "/pr_list");
    assert_action_fails_at(&schema, "{ min: { a: \"{{ fm.a }}\" } }", "/min");
    assert_action_fails_at(&schema, "{ ping: { timeout: \"{{ fm.t }}\" } }", "/ping");
    assert_action_fails_at(&schema, "{ min: [\"{{ fm.a }}\"] }", "/min");
}

#[test]
fn a_malformed_template_is_judged_as_text() {
    let schema = host_schema();

    assert_action_fails_at(&schema, "{ pr_list: \"{{ 1 + }}\" }", "/pr_list");
    assert_action_fails_at(&schema, "{ abs: [\"{{ 1 + }}\"] }", "/abs/0");
}

#[test]
fn ensure_file_keeps_both_overloads_and_nothing_beyond_them() {
    let schema = host_schema();

    for action in [
        "{ ensure_file: notes.txt }",
        "{ ensure_file: [notes.txt] }",
        "{ ensure_file: [notes.txt, hello] }",
        "{ ensure_file: [notes.txt, { a: 1 }] }",
        "{ ensure_file: { file: notes.txt } }",
        "{ ensure_file: { file: notes.txt, content: hello } }",
        "{ ensure_file: { file: notes.txt, content: null, no_error: true } }",
    ] {
        assert_action_passes(&schema, action);
    }
    assert_action_fails_at(&schema, "{ ensure_file: [notes.txt, hello, extra] }", "/ensure_file");
    assert_action_fails_at(&schema, "{ ensure_file: { file: notes.txt, content: hi, mode: x } }", "/ensure_file");
    assert_action_fails_at(&schema, "{ ensure_file: { content: hi } }", "/ensure_file");
}

/// A `null` action value is a call with no arguments, so a short form admits
/// it only for a verb with a zero-argument overload. Judged on the short type
/// under a required key: an optional key admits `null` whatever its type.
#[test]
fn a_null_short_form_is_a_call_with_no_arguments() {
    let schema = effective(
        &std::env::current_dir().expect("process directory"),
        &[
            ("ensure_file", "{ v: ensure_file-short(required)@darkmatter:lifecycle }".to_string()),
            ("branch", "{ v: branch_exists_on_remote-short(required)@darkmatter:lifecycle }".to_string()),
        ],
    );

    let report = validate(&schema, "ensure_file:\n  v: null\nbranch:\n  v: null\n");

    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/ensure_file/v"]);
}

#[test]
fn variadic_verbs_take_any_members_positionally_and_have_no_long_form() {
    let schema = host_schema();

    for verb in ["and", "or"] {
        assert_action_passes(&schema, &format!("{{ {verb}: [true, 1, text, null, [1], {{ a: 1 }}, \"{{{{ fm.x }}}}\"] }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: [] }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: null }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: true }}"));
        assert_action_fails_at(&schema, &format!("{{ {verb}: {{ values: [true] }} }}"), &format!("/{verb}"));
    }
}

#[test]
fn an_optional_parameter_that_rejects_null_may_be_omitted_but_not_null() {
    let schema = host_schema();

    assert_action_passes(&schema, "{ ping: 127.0.0.1 }");
    assert_action_passes(&schema, "{ ping: [127.0.0.1] }");
    assert_action_passes(&schema, "{ ping: [127.0.0.1, 2] }");
    assert_action_passes(&schema, "{ ping: { address: 127.0.0.1 } }");
    assert_action_passes(&schema, "{ ping: { address: 127.0.0.1, timeout: 2 } }");
    assert_action_fails_at(&schema, "{ ping: [127.0.0.1, null] }", "/ping/1");
    assert_action_fails_at(&schema, "{ ping: { address: 127.0.0.1, timeout: null } }", "/ping/timeout");
    assert_action_fails_at(&schema, "{ ping: not-an-address }", "/ping");
}

#[test]
fn a_required_parameter_that_admits_null_cannot_be_omitted() {
    let schema = host_schema();

    assert_action_passes(&schema, "{ has_agentic_cli: claude }");
    assert_action_passes(&schema, "{ has_agentic_cli: [null] }");
    assert_action_passes(&schema, "{ has_agentic_cli: { agent: null } }");
    assert_action_passes(&schema, "{ has_agentic_cli: { agent: \"{{ fm.agent }}\", no_error: true } }");
    assert_action_fails_at(&schema, "{ has_agentic_cli: nope }", "/has_agentic_cli");
    assert_action_fails_at(&schema, "{ has_agentic_cli: {} }", "/has_agentic_cli");
    assert_action_fails_at(&schema, "{ has_agentic_cli: { no_error: true } }", "/has_agentic_cli");
}

#[test]
fn a_missing_required_parameter_fails() {
    let schema = host_schema();

    assert_action_fails_at(&schema, "{ min: [1] }", "/min");
    assert_action_fails_at(&schema, "{ min: { a: 1 } }", "/min");
    assert_action_fails_at(&schema, "{ set_frontmatter: { file: d.md, value: 1 } }", "/set_frontmatter");
}

#[test]
fn an_unknown_parameter_accepts_any_value() {
    let schema = host_schema();

    for value in ["null", "1", "text", "[1, null]", "{ a: { b: [true] } }", "\"{{ fm.x }}\""] {
        assert_action_passes(&schema, &format!("{{ set_frontmatter: [d.md, title, {value}] }}"));
        assert_action_passes(&schema, &format!("{{ set_frontmatter: {{ file: d.md, prop: title, value: {value} }} }}"));
    }
}

#[test]
fn long_forms_bind_per_overload_like_the_parser() {
    let schema = host_schema();

    assert_action_passes(&schema, "{ code_link: { file: src/lib.rs } }");
    assert_action_passes(&schema, "{ code_link: { target: src/lib.rs, desc: the library } }");
    assert_action_fails_at(&schema, "{ code_link: { target: src/lib.rs } }", "/code_link");
    assert_action_fails_at(&schema, "{ code_link: { file: src/lib.rs, desc: x } }", "/code_link");
    assert_action_fails_at(&schema, "{ branch_exists_on_remote: { remote: origin } }", "/branch_exists_on_remote");
    assert_action_fails_at(&schema, "{ pr_list: { query: { state: open }, count: 3 } }", "/pr_list");
}

#[test]
fn bundles_take_short_forms_only() {
    let schema = host_schema();
    let bundle = |yaml: &str| validate(&schema, &format!("bundles:\n  - {yaml}\n"));

    assert!(bundle("{ pr_list: 3, ensure_dir: out, info: hi }").valid);
    assert!(bundle("{ min: [1, \"{{ fm.x }}\"], when: \"{{ true }}\" }").valid);
    assert!(!bundle("{ pr_list: { count: 3 } }").valid);
    assert!(!bundle("{ pr_list: 101 }").valid);
    assert!(!bundle("{ ensure_file: [a, b, c] }").valid);
}

// ── Binder parity ───────────────────────────────────────────────────────────

/// A temporary schema file holding the projection's helper types, `short`
/// (the verb's short form) and `arm-<i>` (overload `i`'s short-form arms).
struct ArmFile {
    dir: tempfile::TempDir,
    overloads: usize,
}

fn atom_yaml(atom: &PropertyAtom) -> YamlValue {
    if *atom == PropertyAtom::bare(SimplifiedType::Null) {
        YamlValue::Null
    } else {
        YamlValue::String(serialize_property_atom(atom))
    }
}

fn arms_yaml(arms: &[PropertyAtom]) -> YamlValue {
    match arms {
        [atom] => atom_yaml(atom),
        _ => YamlValue::Sequence(arms.iter().map(atom_yaml).collect()),
    }
}

impl ArmFile {
    fn new(verb: &ProjectedVerb, helpers: &[(String, PropertyDef)]) -> Self {
        let mut types = Mapping::new();
        types.insert(TEMPLATE_OPERAND.into(), serialize_property_def(&template_operand_type()).into());
        for (name, def) in helpers {
            types.insert(name.clone().into(), serialize_property_def(def).into());
        }
        types.insert("short".into(), arms_yaml(&verb.short));
        for overload in &verb.overloads {
            types.insert(format!("arm-{}", overload.index).into(), arms_yaml(&overload.short_arms()));
        }
        let mut document = Mapping::new();
        document.insert("kind".into(), "schema".into());
        document.insert("types".into(), YamlValue::Mapping(types));
        let dir = tempfile::tempdir().expect("temp dir");
        let text = serde_yaml_ng::to_string(&document).expect("the arm file serializes");
        std::fs::write(dir.path().join("arms.yaml"), text).expect("write the arm file");
        Self {
            dir,
            overloads: verb.overloads.len(),
        }
    }

    /// `shipped` is the verb's short type in the shipped partial, which is
    /// judged beside the projection's copy.
    fn schema(&self, shipped: Option<&str>) -> EffectiveSchema {
        // Each value sits one level down, where templates defer, under a
        // required key, so the optional-property `null` allowance cannot
        // stand in for a zero-argument form.
        let mut wiring = vec![("short".to_string(), "{ v: short(required)@./arms.yaml }".to_string())];
        if let Some(verb) = shipped {
            wiring.push(("shipped".to_string(), format!("{{ v: {verb}-short(required)@darkmatter:lifecycle }}")));
        }
        for index in 0..self.overloads {
            wiring.push((format!("arm{index}"), format!("{{ v: arm-{index}(required)@./arms.yaml }}")));
        }
        let wiring: Vec<(&str, String)> = wiring.iter().map(|(property, ty)| (property.as_str(), ty.clone())).collect();
        effective(self.dir.path(), &wiring)
    }
}

/// The schema's verdict on one encoded short-form value: whether the short
/// form accepts it (`None` for the shipped copy when absent), and which
/// overloads' arms do.
#[derive(Debug, PartialEq)]
struct SchemaVerdict {
    short: bool,
    shipped: Option<bool>,
    overloads: BTreeSet<usize>,
}

fn schema_verdict(schema: &EffectiveSchema, overloads: usize, shipped: bool, encoded: &Value) -> SchemaVerdict {
    let mut document = serde_json::Map::new();
    document.insert("short".into(), json!({ "v": encoded }));
    if shipped {
        document.insert("shipped".into(), json!({ "v": encoded }));
    }
    for index in 0..overloads {
        document.insert(format!("arm{index}"), json!({ "v": encoded }));
    }
    let yaml = serde_yaml_ng::to_string(&Value::Object(document)).expect("the instance serializes");
    let report = validate(schema, &yaml);
    let passes = |property: &str| !report.problems.iter().any(|problem| problem.path.starts_with(&format!("/{property}/")));
    SchemaVerdict {
        short: passes("short"),
        shipped: shipped.then(|| passes("shipped")),
        overloads: (0..overloads).filter(|index| passes(&format!("arm{index}"))).collect(),
    }
}

/// Every short-form spelling of `args`: the list, plus `null` for no
/// arguments and the bare value for one scalar argument.
fn encodings(args: &[Value]) -> Vec<Value> {
    let mut encoded = vec![Value::Array(args.to_vec())];
    match args {
        [] => encoded.push(Value::Null),
        [single] if !(single.is_null() || single.is_array() || single.is_object()) => encoded.push(single.clone()),
        _ => {}
    }
    encoded
}

/// Compares the binder and the generated schema on every call, returning a
/// line per disagreement.
fn parity(
    verb: &ProjectedVerb,
    helpers: &[(String, PropertyDef)],
    registration: &FunctionRegistration,
    shipped: bool,
    calls: &[Vec<Value>],
) -> Vec<String> {
    let file = ArmFile::new(verb, helpers);
    let schema = file.schema(shipped.then_some(verb.name.as_str()));
    let mut disagreements = Vec::new();
    for args in calls {
        let binder = validate_operands(registration, args);
        let (accepted, chosen): (bool, BTreeSet<usize>) = match &binder {
            OperandVerdict::Bound { overload } => (true, BTreeSet::from([*overload])),
            OperandVerdict::Deferred { overloads } => (true, overloads.iter().copied().collect()),
            OperandVerdict::Rejected(_) => (false, BTreeSet::new()),
        };
        for encoded in encodings(args) {
            let verdict = schema_verdict(&schema, file.overloads, shipped, &encoded);
            let mut problems = Vec::new();
            if verdict.short != accepted {
                problems.push(format!("short form {}", if verdict.short { "accepts" } else { "rejects" }));
            }
            if verdict.shipped.is_some_and(|shipped| shipped != verdict.short) {
                problems.push("the shipped short form differs from the projection".to_string());
            }
            if accepted && verdict.overloads != chosen {
                problems.push(format!("overload arms {:?} accept", verdict.overloads));
            }
            if !problems.is_empty() {
                disagreements.push(format!(
                    "{}({}) as {encoded}: binder {binder_text}; {}",
                    verb.name,
                    args.iter().map(Value::to_string).collect::<Vec<_>>().join(", "),
                    problems.join("; "),
                    binder_text = match &binder {
                        OperandVerdict::Rejected(error) => format!("rejects ({:?})", error.kind()),
                        other => format!("{other:?}"),
                    },
                ));
            }
        }
    }
    disagreements
}

fn shipped_registration(verb: &str) -> &'static FunctionRegistration {
    function_registration(verb).or_else(|| effect_registration(verb)).unwrap_or_else(|| panic!("`{verb}` is registered"))
}

/// Operand vectors per verb: every overloaded verb, both variadic verbs,
/// a two-number verb, and an optional nonnullable parameter.
fn parity_calls() -> Vec<(&'static str, Vec<Vec<Value>>)> {
    let template = json!("{{ fm.x }}");
    vec![
        ("code_link", vec![vec![], vec![json!("src/lib.rs")], vec![json!("a.rs"), json!("desc")], vec![json!(null)], vec![json!("a.rs"), json!(null)], vec![template.clone()], vec![json!("a"), json!("b"), json!("c")]]),
        ("link", vec![vec![json!("a.md")], vec![json!("a.md"), json!("A")], vec![json!(1), json!(2)], vec![json!("a"), json!("b"), json!("c")]]),
        ("frontmatter", vec![vec![json!("d.md")], vec![json!("d.md"), json!("title")], vec![json!("d.md"), json!(null)], vec![json!("d.md"), template.clone()], vec![]]),
        ("validate_schema", vec![vec![json!("d.md")], vec![json!("d.md"), json!({"type": "object"})], vec![json!("d.md"), json!("x")], vec![json!("d.md"), template.clone()]]),
        ("branch_exists_on_remote", vec![vec![], vec![json!("main")], vec![json!("main"), json!("origin")], vec![json!("main"), json!("origin"), json!("x")], vec![json!({"a": 1})]]),
        ("pr_list", vec![vec![json!(3)], vec![json!("3")], vec![json!("three")], vec![json!(0)], vec![json!(101)], vec![json!({"state": "open"})], vec![template.clone()], vec![], vec![json!(1), json!(2)]]),
        ("cicd_list", vec![vec![json!(50)], vec![json!({"status": "failure"})], vec![json!("x")], vec![template.clone()]]),
        ("ensure_file", vec![vec![json!("a.txt")], vec![json!("a.txt"), json!("hi")], vec![json!("a.txt"), json!(null)], vec![json!("a.txt"), json!({"k": [1]})], vec![json!("a"), json!("b"), json!("c")], vec![], vec![json!(null)]]),
        ("and", vec![vec![], vec![json!(true)], vec![json!(true), json!(1), json!(null), json!([1]), json!({"a": 1})], vec![template.clone(), json!(false)]]),
        ("or", vec![vec![], vec![json!("x")], vec![json!(null), json!(null)]]),
        ("min", vec![vec![json!(1), json!(2)], vec![json!("1"), json!(2)], vec![json!(1), template.clone()], vec![json!("three"), template.clone()], vec![json!(1)], vec![json!(1), json!(2), json!(3)]]),
        ("ping", vec![vec![json!("127.0.0.1")], vec![json!("127.0.0.1"), json!(2)], vec![json!("127.0.0.1"), json!(null)], vec![json!("localhost")], vec![template.clone(), json!("2")], vec![]]),
    ]
}

#[test]
fn every_overloaded_verb_reaches_the_binders_verdict_and_overload() {
    let projection = LifecycleSchemaProjection::shipped().expect("the shipped catalogs project");
    let mut disagreements = Vec::new();
    for (name, calls) in parity_calls() {
        let verb = projection.verb(name).unwrap_or_else(|| panic!("`{name}` is generated"));
        disagreements.extend(parity(verb, &projection.helper_types, shipped_registration(name), true, &calls));
    }
    assert!(disagreements.is_empty(), "binder and schema disagree:\n{}", disagreements.join("\n"));
}

#[test]
fn the_parity_table_covers_every_overloaded_and_variadic_verb() {
    let projection = LifecycleSchemaProjection::shipped().expect("the shipped catalogs project");
    let covered: BTreeSet<&str> = parity_calls().iter().map(|(name, _)| *name).collect();
    for (_, verb) in &projection.verbs {
        if verb.overloads.len() > 1 || verb.long.is_none() {
            assert!(covered.contains(verb.name.as_str()), "`{}` needs parity rows", verb.name);
        }
    }
}

fn function_type(text: &str) -> FunctionType {
    match parse_type_expr("fixture", text).expect("the fixture signature parses").ty {
        TypeExpr::Function(function) => *function,
        other => panic!("not a function: {other:?}"),
    }
}

/// No shipped overload set has two same-arity overloads that both convert
/// one value, so the case the two selection rules decide differently comes
/// from a fixture: `pick(x: number) | pick(x: boolean)` with `"1"`, which
/// each overload accepts only after conversion.
#[test]
fn two_overloads_that_both_convert_are_ambiguous_to_the_binder_but_accepted_by_the_schema() {
    let overloads = [
        function_type("function(parameters(x: number); returns(boolean))"),
        function_type("function(parameters(x: boolean); returns(boolean))"),
    ];
    let registration = FunctionRegistration::builder_from_types("pick", overloads.to_vec())
        .build()
        .expect("the fixture registers");
    let references: Vec<&FunctionType> = overloads.iter().collect();
    let verb = ProjectedVerb::from_function_types("pick", &references, &|_| None).expect("the fixture projects");

    let OperandVerdict::Rejected(error) = validate_operands(&registration, &[json!("1")]) else {
        panic!("the binder must not choose between two converting overloads");
    };
    assert_eq!(error.kind(), CallErrorKind::AmbiguousOverload);

    let file = ArmFile::new(&verb, &[]);
    let schema = file.schema(None);
    for encoded in encodings(&[json!("1")]) {
        let verdict = schema_verdict(&schema, file.overloads, false, &encoded);
        assert!(verdict.short, "the union accepts {encoded} through either arm");
        assert_eq!(verdict.overloads, BTreeSet::from([0, 1]), "{encoded}");
    }

    // Exact values bind and agree.
    let disagreements = parity(&verb, &[], &registration, false, &[vec![json!(2)], vec![json!(true)], vec![json!("x")]]);
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}
