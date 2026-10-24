//! The generated lifecycle verb types judge operands through the public
//! schema API the way a host references them (`expression[]@darkmatter:lifecycle`,
//! `bundle[]@darkmatter:lifecycle`, `<verb>-short@darkmatter:lifecycle`), and
//! reach the passive call binder's (`validate_operands`) verdict: their
//! projected structure agrees with it for every overloaded verb, and schema
//! validation binds each value through the verb's runtime registration, so
//! the registration's own rules (numeric recovery, source exclusions,
//! overload preference) decide where the structure alone cannot.

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
    Constraint, FunctionType, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr, serialize_property_atom,
    serialize_property_def,
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
fn a_malformed_template_fails_a_numeric_operand() {
    let schema = host_schema();

    assert_action_fails_at(&schema, "{ pr_list: \"{{ 1 + }}\" }", "/pr_list");
    assert_action_fails_at(&schema, "{ abs: [\"{{ 1 + }}\"] }", "/abs/0");
}

/// The action fails with exactly one problem at or below `/actions/0{path}`:
/// the malformed template's syntax problem, at the string's own position.
#[track_caller]
fn assert_malformed_at(schema: &EffectiveSchema, action: &str, path: &str) {
    assert_malformed_under(schema, "actions", action, path);
}

/// [`assert_malformed_at`] for an action listed under `key`.
#[track_caller]
fn assert_malformed_under(schema: &EffectiveSchema, key: &str, action: &str, path: &str) {
    let yaml = format!("{key}:\n  - {action}\n");
    let report = validate(schema, &yaml);
    let full = format!("/{key}/0{path}");
    assert!(!report.valid, "`{action}` passed");
    let within: Vec<_> = report
        .problems
        .iter()
        .filter(|problem| problem.path == full || problem.path.starts_with(&format!("{full}/")))
        .collect();
    assert_eq!(within.len(), 1, "`{action}`: {:#?}", report.problems);
    let problem = within[0];
    assert_eq!(problem.path, full, "`{action}`");
    assert!(
        problem.message.starts_with("malformed template `") && problem.message.contains("1 +"),
        "`{action}`: {}",
        problem.message
    );
    let column = yaml.lines().nth(1).and_then(|line| {
        let quoted = line.find('"').or_else(|| line.find('\''))?;
        u32::try_from(quoted + 1).ok()
    });
    assert_eq!((problem.line, problem.column), (Some(2), column), "`{action}`: {problem:#?}");
}

#[test]
fn a_malformed_template_in_a_string_operand_is_a_syntax_problem() {
    let schema = host_schema();

    assert_malformed_at(&schema, "{ trim: \"{{ 1 + }}\" }", "/trim");
    assert_malformed_at(&schema, "{ trim: [\"{{ 1 + }}\"] }", "/trim/0");
    assert_malformed_at(&schema, "{ trim: { text: \"{{ 1 + }}\" } }", "/trim/text");
    assert_malformed_at(&schema, "{ ensure_file: \"{{ 1 + }}\" }", "/ensure_file");
    assert_malformed_at(&schema, "{ ensure_file: [\"{{ 1 + }}\"] }", "/ensure_file/0");
    assert_malformed_at(&schema, "{ ensure_file: { file: \"{{ 1 + }}\" } }", "/ensure_file/file");
    assert_malformed_at(&schema, "{ ensure_file: [notes.txt, \"{{ 1 + }}\"] }", "/ensure_file/1");
}

#[test]
fn a_malformed_span_in_mixed_text_is_a_syntax_problem() {
    let schema = host_schema();

    assert_malformed_at(&schema, "{ trim: 'hello {{ 1 + }}' }", "/trim");
    assert_malformed_at(&schema, "{ ensure_file: 'out/{{ 1 + }}.md' }", "/ensure_file");
    assert_malformed_at(&schema, "{ trim: '{{ fm.name }} and {{ 1 + }}' }", "/trim");
}

#[test]
fn a_malformed_template_in_a_number_or_mapping_operand_reports_its_syntax() {
    let schema = host_schema();

    assert_malformed_at(&schema, "{ min: [\"{{ 1 + }}\", 3] }", "/min/0");
    assert_malformed_at(&schema, "{ min: { a: 1, b: \"{{ 1 + }}\" } }", "/min/b");
    assert_malformed_at(&schema, "{ merge_frontmatter: [d.md, '{{ 1 + }}'] }", "/merge_frontmatter/1");
}

#[test]
fn a_malformed_template_leaves_its_literal_siblings_judged() {
    let schema = host_schema();

    let report = validate(&schema, "actions:\n  - { min: [\"{{ 1 + }}\", three] }\n");
    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert!(paths.contains(&"/actions/0/min/0"), "{:#?}", report.problems);
    assert!(paths.contains(&"/actions/0/min/1"), "{:#?}", report.problems);
}

#[test]
fn deferred_templates_text_and_unclosed_braces_stay_valid_string_operands() {
    let schema = host_schema();

    for action in [
        "{ trim: \"{{ fm.name }}\" }",
        "{ trim: 'hello {{ fm.name }}' }",
        "{ trim: hello }",
        "{ trim: \"{{ unclosed\" }",
        "{ ensure_file: \"{{ fm.name }}\" }",
        "{ ensure_file: 'out/{{ fm.name }}.md' }",
        "{ ensure_file: { file: \"{{ unclosed\" } }",
        "{ min: [\"{{ fm.a }}\", 3] }",
    ] {
        assert_action_passes(&schema, action);
    }
}

#[test]
fn a_literal_token_holding_malformed_template_text_is_data() {
    let schema = host_schema();
    let token = darkmatter::markdown::literal_token::encode("{{ 1 + }}");
    let yaml = format!("actions:\n  - {{ trim: \"{token}\" }}\n");
    let value: Value = serde_yaml_ng::from_str(&yaml).expect("fixture YAML parses");
    let options = darkmatter::markdown::schemas::ValidationOptions::default();

    let report = schema.validate_with_options(&value, &build_position_map(&yaml), &options);

    assert!(report.valid, "{:#?}", report.problems);
}

// ── Stored literal tokens are data ──────────────────────────────────────────

/// `actions: [<action>]` with `TOKEN` replaced by the stored literal token of
/// `text`, judged by the public options API, which decodes the token.
fn validate_stored_action(schema: &EffectiveSchema, action: &str, text: &str) -> ValidationReport {
    let token = darkmatter::markdown::literal_token::encode(text);
    let yaml = format!("actions:\n  - {}\n", action.replace("TOKEN", &format!("\"{token}\"")));
    let value: Value = serde_yaml_ng::from_str(&yaml).expect("fixture YAML parses");
    let options = darkmatter::markdown::schemas::ValidationOptions::default();
    schema.validate_with_options(&value, &build_position_map(&yaml), &options)
}

/// The report rejects with a positioned problem at `path` (below
/// `/actions/0`) or below it, and defers nothing.
#[track_caller]
fn assert_stored_rejected_at(report: &ValidationReport, path: &str) {
    let full = format!("/actions/0{path}");
    let problem = report
        .problems
        .iter()
        .find(|problem| problem.path == full || problem.path.starts_with(&format!("{full}/")))
        .unwrap_or_else(|| panic!("no problem at {full}: {:#?}", report.problems));
    assert!(!report.valid);
    assert!(problem.line == Some(2) && problem.column.is_some(), "unpositioned: {problem:#?}");
    assert!(report.deferred.is_empty(), "stored data was deferred: {:?}", report.deferred);
}

#[test]
fn a_literal_token_holding_template_text_is_bound_as_that_text() {
    let schema = host_schema();

    let scalar = validate_stored_action(&schema, "{ abs: TOKEN }", "{{ fm.count }}");
    assert_stored_rejected_at(&scalar, "/abs");
    let positional = validate_stored_action(&schema, "{ min: [TOKEN, 1] }", "{{ fm.count }}");
    assert_stored_rejected_at(&positional, "/min/0");
    let named = validate_stored_action(&schema, "{ min: { a: TOKEN, b: 1 } }", "{{ fm.count }}");
    assert_stored_rejected_at(&named, "/min/a");
    let structured = validate_stored_action(&schema, "{ merge_frontmatter: [doc.md, TOKEN] }", "{{ fm.count }}");
    assert_stored_rejected_at(&structured, "/merge_frontmatter/1");
}

#[test]
fn a_literal_token_keeps_the_conversion_and_template_controls() {
    let schema = host_schema();

    let text = validate_stored_action(&schema, "{ trim: TOKEN }", "{{ fm.count }}");
    assert!(text.valid && text.deferred.is_empty(), "{text:#?}");
    let word = validate_stored_action(&schema, "{ abs: TOKEN }", "three");
    assert_stored_rejected_at(&word, "/abs");
    let numeral = validate_stored_action(&schema, "{ abs: TOKEN }", "3");
    assert!(numeral.valid, "{:#?}", numeral.problems);

    let yaml = "actions:\n  - { abs: \"{{ fm.count }}\" }\n";
    let value: Value = serde_yaml_ng::from_str(yaml).expect("fixture YAML parses");
    let options = darkmatter::markdown::schemas::ValidationOptions::default();
    let authored = schema.validate_with_options(&value, &build_position_map(yaml), &options);
    assert!(authored.valid, "{:#?}", authored.problems);
    assert_eq!(authored.deferred, ["/actions/0/abs"]);
}

#[test]
fn the_phase_apis_keep_a_stored_token_apart_from_an_authored_template() {
    use darkmatter::markdown::schemas::{PositionMap, SchemaPhase};

    let schema = host_schema();
    let token = darkmatter::markdown::literal_token::encode("{{ fm.count }}");
    let stored = json!({ "actions": [{ "abs": token }] });
    let authored = json!({ "actions": [{ "abs": "{{ fm.count }}" }] });

    assert!(!schema.validate_raw(&stored).valid, "raw validation judges the token as written");
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        let encoded = schema.validate_for_phase(&stored, phase).expect("projects");
        assert!(!encoded.valid, "{phase:?}: the encoded token is not a template");
        let decoded = schema.validate_for_phase(&authored, phase).expect("projects");
        assert!(decoded.valid, "{phase:?}: an authored template is deferred: {:#?}", decoded.problems);

        let report = schema.validate_stored(&stored, &PositionMap::new(), Some(phase)).expect("projects");
        assert!(!report.valid && report.deferred.is_empty(), "{phase:?}: {report:#?}");
        let report = schema.validate_stored(&authored, &PositionMap::new(), Some(phase)).expect("projects");
        assert!(report.valid, "{phase:?}: {:#?}", report.problems);
        assert_eq!(report.deferred, ["/actions/0/abs"], "{phase:?}");
    }
    let unphased = schema.validate_stored(&stored, &PositionMap::new(), None).expect("never fails unphased");
    assert!(!unphased.valid);
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

// ── Registration rules ──────────────────────────────────────────────────────

/// `number` and `round` recover a subject that does not convert through
/// their fallback, so the call binds although the projected subject type
/// rejects the text; the long form selects the same overload.
#[test]
fn a_numeric_recovery_fallback_binds_a_subject_the_types_reject() {
    let schema = host_schema();

    for verb in ["number", "round"] {
        assert_action_passes(&schema, &format!("{{ {verb}: [pear, 7] }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: [pear, 2.7] }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: [[1, 2], 7] }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: [null, \"7\"] }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: {{ x: pear, fallback: 7 }} }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: {{ x: pear, fallback: 2.7, no_error: true }} }}"));
        assert_action_passes(&schema, &format!("{{ {verb}: [\"{{{{ fm.x }}}}\", 7] }}"));
        // Without a usable fallback the subject must convert, as at runtime.
        assert_action_fails_at(&schema, &format!("{{ {verb}: pear }}"), &format!("/{verb}"));
        assert_action_fails_at(&schema, &format!("{{ {verb}: [pear] }}"), &format!("/{verb}"));
        assert_action_fails_at(&schema, &format!("{{ {verb}: [pear, null] }}"), &format!("/{verb}"));
        assert_action_fails_at(&schema, &format!("{{ {verb}: [pear, plum] }}"), &format!("/{verb}"));
        assert_action_fails_at(&schema, &format!("{{ {verb}: {{ x: pear }} }}"), &format!("/{verb}"));
    }
}

/// `length` refuses a boolean before any conversion, in every spelling,
/// although its projected `string` operand converts one.
#[test]
fn a_source_exclusion_rejects_a_value_the_types_accept() {
    let schema = host_schema();

    assert_action_fails_at(&schema, "{ length: true }", "/length");
    assert_action_fails_at(&schema, "{ length: [true] }", "/length/0");
    assert_action_fails_at(&schema, "{ length: { val: false } }", "/length/val");
    let report = validate(&schema, "actions:\n  - { length: [true] }\n");
    let problem = report.problems.iter().find(|problem| problem.path == "/actions/0/length/0").expect("a problem");
    assert!(problem.message.contains("does not accept a boolean"), "{problem:#?}");
    assert_eq!(problem.line, Some(2));
    assert!(problem.column.is_some_and(|column| column > 14), "{problem:#?}");
}

#[test]
fn ordinary_calls_of_the_registered_verbs_still_pass() {
    let schema = host_schema();

    for action in [
        "{ number: [3, 7] }",
        "{ number: \"3\" }",
        "{ number: { x: 3 } }",
        "{ round: [3.7] }",
        "{ round: 3.7 }",
        "{ length: [hi] }",
        "{ length: hi }",
        "{ length: { val: hi } }",
        "{ length: [\"{{ fm.v }}\"] }",
    ] {
        assert_action_passes(&schema, action);
    }
}

/// A deferred or settled operand never hides a mistake beside it: an
/// extra operand, an unknown or missing name, or a bad `no_error`.
#[test]
fn a_bound_call_keeps_the_checks_beside_its_operands() {
    let schema = host_schema();

    assert_action_fails_at(&schema, "{ number: [pear, 7, 8] }", "/number");
    assert_action_fails_at(&schema, "{ number: { x: pear, fallback: 7, bogus: 1 } }", "/number");
    assert_action_fails_at(&schema, "{ number: { fallback: 7 } }", "/number");
    assert_action_fails_at(&schema, "{ number: { x: pear, fallback: 7, no_error: [1] } }", "/number");
    assert_action_fails_at(&schema, "{ length: [\"{{ fm.v }}\", true] }", "/length");
    assert_action_fails_at(&schema, "{ number: { x: \"{{ fm.x }}\", fallback: 7, bogus: 1 } }", "/number");
}

#[test]
fn bundles_bind_their_short_forms_and_still_refuse_long_forms() {
    let schema = host_schema();
    let bundle = |yaml: &str| validate(&schema, &format!("bundles:\n  - {yaml}\n"));

    assert!(bundle("{ number: [pear, 7], info: hi }").valid);
    assert!(bundle("{ round: [pear, 2.7], when: \"{{ true }}\" }").valid);
    let report = bundle("{ length: [true], info: hi }");
    assert!(report.problems.iter().any(|problem| problem.path == "/bundles/0/length/0"), "{:#?}", report.problems);
    assert!(!bundle("{ number: { x: pear, fallback: 7 }, info: hi }").valid);
}

/// The review's operand matrix: with a valid fallback, `number` and `round`
/// bind every subject shape, and `length` refuses only a boolean; the schema
/// now reaches the binder's verdict on each, in every spelling.
#[test]
fn the_registered_verbs_reach_the_binders_verdict() {
    let projection = LifecycleSchemaProjection::shipped().expect("the shipped catalogs project");
    let subjects = [
        json!(null),
        json!(true),
        json!(3),
        json!("3"),
        json!("pear"),
        json!(""),
        json!([]),
        json!([1, "a"]),
        json!([1, 2]),
        json!({}),
        json!({ "a": 1 }),
        json!("{{ fm.x }}"),
    ];
    let mut disagreements = Vec::new();
    for (name, fallbacks) in [("number", vec![json!(7), json!(null), json!("x")]), ("round", vec![json!(2.7)])] {
        let mut calls: Vec<Vec<Value>> = vec![vec![]];
        for subject in &subjects {
            calls.push(vec![subject.clone()]);
            for fallback in &fallbacks {
                calls.push(vec![subject.clone(), fallback.clone()]);
            }
        }
        let verb = projection.verb(name).expect("generated");
        disagreements.extend(bound_parity(verb, &projection.helper_types, &calls));
    }
    let mut calls: Vec<Vec<Value>> = subjects.iter().map(|subject| vec![subject.clone()]).collect();
    calls.push(vec![]);
    calls.push(vec![json!("a"), json!("b")]);
    let verb = projection.verb("length").expect("generated");
    disagreements.extend(bound_parity(verb, &projection.helper_types, &calls));
    assert!(disagreements.is_empty(), "binder and schema disagree:\n{}", disagreements.join("\n"));
}

/// The shipped short form's verdict on every spelling of each call, against
/// the binder's.
fn bound_parity(verb: &ProjectedVerb, helpers: &[(String, PropertyDef)], calls: &[Vec<Value>]) -> Vec<String> {
    let file = ArmFile::new(verb, helpers);
    let schema = file.schema(Some(verb.name.as_str()));
    let registration = shipped_registration(&verb.name);
    let mut disagreements = Vec::new();
    for args in calls {
        let accepted = !matches!(validate_operands(registration, args), OperandVerdict::Rejected(_));
        for encoded in encodings(args) {
            let verdict = schema_verdict(&schema, file.overloads, true, &encoded);
            if verdict.shipped != Some(accepted) {
                disagreements.push(format!("{}({encoded}): binder accepts: {accepted}", verb.name));
            }
        }
    }
    disagreements
}

// ── Binder parity ───────────────────────────────────────────────────────────

/// `atom` without its `operands(...)` marker: the projected structure alone,
/// which the schema judges without binding through a registration.
fn structural(atom: &PropertyAtom) -> PropertyAtom {
    let mut atom = atom.clone();
    atom.constraints.retain(|constraint| !matches!(constraint, Constraint::Operands(_)));
    atom.array_constraints.retain(|constraint| !matches!(constraint, Constraint::Operands(_)));
    atom
}

/// A temporary schema file holding the projection's helper types, `short`
/// (the verb's short form, structure only), `bound` (the short form as
/// generated, bound through its registration), and `arm-<i>` (overload
/// `i`'s short-form arms, structure only).
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
        let structure: Vec<PropertyAtom> = verb.short.iter().map(structural).collect();
        types.insert("short".into(), arms_yaml(&structure));
        types.insert("bound".into(), arms_yaml(&verb.short));
        for overload in &verb.overloads {
            let arms: Vec<PropertyAtom> = overload.short_arms().iter().map(structural).collect();
            types.insert(format!("arm-{}", overload.index).into(), arms_yaml(&arms));
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
/// each overload accepts only after conversion. The projected union alone
/// accepts it through either arm; the generated type, bound through the
/// registration a call dispatches to, rejects it as the binder does.
#[test]
fn two_overloads_that_both_convert_are_ambiguous_to_the_binder_and_the_bound_schema() {
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
    bound_pick_follows_the_binder(&file, registration.clone());

    // Exact values bind and agree.
    let disagreements = parity(&verb, &[], &registration, false, &[vec![json!(2)], vec![json!(true)], vec![json!("x")]]);
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}

/// The generated `pick` type, bound through `pick`'s registration (a test
/// function, which dispatch, and so validation, consults first), reaches the
/// binder's verdict: `"1"` is ambiguous, exact values bind.
#[cfg(feature = "effects-instrumentation")]
fn bound_pick_follows_the_binder(file: &ArmFile, registration: FunctionRegistration) {
    use darkmatter::markdown::compose::expression::function_test_hooks;

    let _pick = function_test_hooks::register(registration, std::sync::Arc::new(|_| Ok(json!(true))));
    let schema = effective(file.dir.path(), &[("bound", "{ v: bound(required)@./arms.yaml }".to_string())]);
    for encoded in encodings(&[json!("1")]) {
        let yaml = serde_yaml_ng::to_string(&json!({ "bound": { "v": encoded } })).expect("serializes");
        let report = validate(&schema, &yaml);
        let problem = report
            .problems
            .iter()
            .find(|problem| problem.path.starts_with("/bound/v"))
            .unwrap_or_else(|| panic!("{encoded} is accepted: {:#?}", report.problems));
        assert!(problem.message.contains("ambiguous call"), "{problem:#?}");
        assert!(problem.line.is_some(), "{problem:#?}");
    }
    for encoded in [json!(2), json!(true), json!([2]), json!([true])] {
        let yaml = serde_yaml_ng::to_string(&json!({ "bound": { "v": encoded } })).expect("serializes");
        let report = validate(&schema, &yaml);
        assert!(report.valid, "{encoded}: {:#?}", report.problems);
    }
}

/// Without the instrumentation feature no test function can be registered,
/// so no registration exists for `pick` and the bound type keeps the
/// projected union's verdict; the bound half runs under `just test`.
#[cfg(not(feature = "effects-instrumentation"))]
fn bound_pick_follows_the_binder(_: &ArmFile, _: FunctionRegistration) {}

// ── Hand-written verbs: malformed templates ─────────────────────────────────

/// A list of each hand-written category and of `bundle`s, as a host's event
/// stack references them.
fn hand_written_schema() -> EffectiveSchema {
    effective(
        &std::env::current_dir().expect("process directory"),
        &[
            ("communicate", "communicate[]@darkmatter:lifecycle".to_string()),
            ("flow", "flow[]@darkmatter:lifecycle".to_string()),
            ("shell", "shell[]@darkmatter:lifecycle".to_string()),
            ("expression", "expression[]@darkmatter:lifecycle".to_string()),
            ("bundles", "bundle[]@darkmatter:lifecycle".to_string()),
        ],
    )
}

/// The category list `action`'s verb belongs under in [`hand_written_schema`].
fn category_of(action: &str) -> &'static str {
    let value: Value = serde_yaml_ng::from_str(action).expect("the action parses as YAML");
    let verb = value
        .as_object()
        .and_then(|map| map.keys().find(|key| key.as_str() != "when" && key.as_str() != "no_error"))
        .expect("one verb")
        .clone();
    match verb.as_str() {
        "shell" => "shell",
        "set" => "expression",
        "error" | "proxy" | "retry" | "resume" | "defer" | "break" | "prep" | "stop" | "skip" => "flow",
        _ => "communicate",
    }
}

/// Whether the lifecycle parser accepts `action` as the one action of md's
/// `composed` stack.
fn parser_accepts(action: &str) -> bool {
    use darkmatter::lifecycle::registry::EventRegistry;
    use darkmatter::lifecycle::{LifecycleParseOptions, UnknownVerbPolicy, parse_lifecycle_config_with};

    let value: Value = serde_yaml_ng::from_str(action).expect("the action parses as YAML");
    let registry = EventRegistry::md();
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: &registry,
    };
    parse_lifecycle_config_with(&json!({ "composed": [value] }), Path::new("doc.md"), &options).is_ok()
}

/// Every string the lifecycle parser reads as a template in a hand-written
/// verb's value (communication text, directive strings, `shell`, `set`
/// values, and a `with:` overlay) reports a malformed span where the parser
/// rejects it, at the string's own path and position.
#[test]
fn a_malformed_template_in_a_hand_written_verb_is_a_syntax_problem() {
    let schema = hand_written_schema();

    for (action, path) in [
        ("{ say: \"{{ 1 + }}\" }", "/say"),
        ("{ say: { text: 'hello {{ 1 + }}' } }", "/say/text"),
        ("{ info: \"{{ 1 + }}\" }", "/info"),
        ("{ effect: \"{{ 1 + }}\" }", "/effect"),
        ("{ error: \"{{ 1 + }}\" }", "/error"),
        ("{ error: { reason: \"{{ 1 + }}\" } }", "/error/reason"),
        ("{ proxy: \"{{ 1 + }}\" }", "/proxy"),
        ("{ proxy: { target: \"{{ 1 + }}\" } }", "/proxy/target"),
        ("{ proxy: { with: { round: \"{{ 1 + }}\" }, target: next.md } }", "/proxy/with/round"),
        ("{ proxy: { with: { list: [\"{{ 1 + }}\"] }, target: next.md } }", "/proxy/with/list/0"),
        ("{ retry: \"{{ 1 + }}\" }", "/retry"),
        ("{ retry: { with: { round: \"{{ 1 + }}\" } } }", "/retry/with/round"),
        ("{ resume: \"{{ 1 + }}\" }", "/resume"),
        ("{ resume: { message: \"{{ 1 + }}\" } }", "/resume/message"),
        ("{ defer: { delay: \"{{ 1 + }}\" } }", "/defer/delay"),
        ("{ break: \"{{ 1 + }}\" }", "/break"),
        ("{ prep: { target: \"{{ 1 + }}\" } }", "/prep/target"),
        ("{ prep: { with: { round: \"{{ 1 + }}\" }, target: setup.md } }", "/prep/with/round"),
        ("{ shell: \"echo {{ 1 + }}\" }", "/shell"),
        ("{ shell: { command: \"echo {{ 1 + }}\" } }", "/shell/command"),
        ("{ set: { round: \"{{ 1 + }}\" } }", "/set/round"),
        ("{ set: { nested: { deep: \"{{ 1 + }}\" } } }", "/set/nested/deep"),
    ] {
        assert!(!parser_accepts(action), "the parser accepts `{action}`");
        assert_malformed_under(&schema, category_of(action), action, path);
    }
}

#[test]
fn a_malformed_template_in_a_bundle_verb_is_a_syntax_problem() {
    let schema = hand_written_schema();

    let yaml = "bundles:\n  - { info: \"{{ 1 + }}\", stop: null }\n";
    let report = validate(&schema, yaml);

    let problems: Vec<_> = report.problems.iter().map(|problem| (problem.path.as_str(), problem.line, problem.column)).collect();
    assert_eq!(problems, [("/bundles/0/info", Some(2), Some(13))], "{:#?}", report.problems);
    assert!(report.problems[0].message.starts_with("malformed template `"), "{:#?}", report.problems);
}

/// Valid templates, plain text, an unclosed `{{`, a `set` value the parser
/// reads as a shell command, and a `when` condition stay as they were.
#[test]
fn hand_written_verbs_keep_valid_templates_text_and_shell_values() {
    let schema = hand_written_schema();

    for action in [
        "{ say: \"{{ fm.name }}\" }",
        "{ say: { text: 'hello {{ fm.name }}' } }",
        "{ info: plain text }",
        "{ info: \"{{ unclosed\" }",
        "{ error: done }",
        "{ proxy: { target: next.md, with: { round: \"{{ fm.round + 1 }}\", note: \"{{ unclosed\" } } }",
        "{ shell: \"echo {{ fm.name }}\" }",
        "{ set: { round: \"{{ fm.round + 1 }}\", text: plain } }",
        "{ set: { listing: \"$(echo {{ 1 + }})\" } }",
        "{ when: \"fm.ready\", info: ready }",
        "{ break: { reason: done, code: 3 } }",
    ] {
        assert!(parser_accepts(action), "the parser rejects `{action}`");
        let key = category_of(action);
        let report = validate(&schema, &format!("{key}:\n  - {action}\n"));
        assert!(report.valid, "rejected `{action}`: {:#?}", report.problems);
    }
}

#[test]
fn a_literal_token_in_a_hand_written_verb_is_data() {
    let schema = hand_written_schema();
    let token = darkmatter::markdown::literal_token::encode("{{ 1 + }}");
    let yaml = format!("communicate:\n  - {{ say: \"{token}\" }}\nexpression:\n  - {{ set: {{ v: \"{token}\" }} }}\n");
    let value: Value = serde_yaml_ng::from_str(&yaml).expect("fixture YAML parses");
    let options = darkmatter::markdown::schemas::ValidationOptions::default();

    let report = schema.validate_with_options(&value, &build_position_map(&yaml), &options);

    assert!(report.valid, "{:#?}", report.problems);
}
