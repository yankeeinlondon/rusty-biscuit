//! The `edit` lifecycle action's grammar through the public parser
//! (`parse_lifecycle_config_with` under md's registry): the short and long
//! forms, every invalid operand shape with its located error, bundle
//! ordering and `no_error`, its registration as a lifecycle-only verb (never
//! an expression function or side effect), and the passive surfaces that
//! walk its `file` operand without evaluating it.

use std::path::Path;

use darkmatter::effects::EFFECT_DESCRIPTORS;
use darkmatter::lifecycle::actions::{
    EditAction, LIFECYCLE_ONLY_VERBS, LifecycleAction, LifecycleActionKind, all_lifecycle_verbs, for_each_action,
    is_known_lifecycle_verb, is_lifecycle_only_verb,
};
use darkmatter::lifecycle::error::LifecycleError;
use darkmatter::lifecycle::registry::{ActionType, EventRegistry};
use darkmatter::lifecycle::{
    LifecycleConfig, LifecycleParseOptions, LifecycleSignal, UnknownVerbPolicy, action_type,
    lifecycle_call_checks, parse_lifecycle_config_with, reject_incompatible_calls, runs_a_shell, template_operands,
};
use darkmatter::markdown::compose::expression::{Expr, expression_function_descriptors};
use serde_json::{Value, json};
use simplified_schema::simplified::OperandForm;

const SOURCE: &str = "prompts/new-spec.md";

/// The spec's motivating `initialize` workflow, unedited.
const MOTIVATING: &str = r#"
fqn: "{{ ctx.cwd }}/features/2026-11-14-example/spec.md"
initialize:
    - when: file_exists(fqn)
      then:
        - warn: "The spec already exists: {{ fqn }}"
        - skip
      else:
        - edit:
            file: "{{ fqn }}"
            create_new: true
        - info: "The spec is ready at {{ fqn }}"
        - skip
"#;

fn yaml(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap_or_else(|error| panic!("fixture is YAML: {error}\n{text}"))
}

fn parse(frontmatter: &Value) -> Result<LifecycleConfig, Box<LifecycleError>> {
    let registry = EventRegistry::md();
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        authored_values: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: &registry,
    };
    parse_lifecycle_config_with(frontmatter, Path::new(SOURCE), &options).map_err(Box::new)
}

/// Every action of `signal`'s stack, through both branches.
fn leaves(config: &LifecycleConfig, signal: LifecycleSignal) -> Vec<LifecycleAction> {
    let mut found = Vec::new();
    for_each_action(config.stack(signal), &mut |action| found.push(action.clone()));
    found
}

/// The `edit` actions of md's `composed` stack holding `action` alone.
fn edit_of(action: Value) -> (EditAction, LifecycleAction) {
    let config = parse(&json!({ "composed": [action.clone()] })).unwrap_or_else(|error| panic!("{action}: {error}"));
    let actions = leaves(&config, LifecycleSignal::Start);
    assert_eq!(actions.len(), 1, "{action}");
    match &actions[0].kind {
        LifecycleActionKind::Edit(edit) => (edit.clone(), actions[0].clone()),
        other => panic!("{action}: expected an edit action, got {other:?}"),
    }
}

/// What a refused fixture's error reports: its variant, source file, located
/// property, and rendered text.
#[derive(Debug)]
struct Refusal {
    variant: &'static str,
    source_path: String,
    property: String,
    text: String,
}

fn refusal(frontmatter: &Value) -> Refusal {
    let error = parse(frontmatter).expect_err(&format!("{frontmatter} is refused"));
    let text = error.to_string();
    let (variant, source_path, property) = match &*error {
        LifecycleError::LifecycleInvalidArgs { source_path, property, .. } => ("InvalidArgs", source_path, property),
        LifecycleError::LifecycleWrongArity { source_path, property, .. } => ("WrongArity", source_path, property),
        LifecycleError::LifecycleActionInvalidLongForm { source_path, property, .. } => {
            ("InvalidLongForm", source_path, property)
        }
        LifecycleError::LifecycleObjectDataThroughInterpolationParameter { source_path, property, .. } => {
            ("ObjectParameter", source_path, property)
        }
        LifecycleError::LifecycleObjectDataThroughInterpolationPositional { source_path, property, .. } => {
            ("ObjectPositional", source_path, property)
        }
        LifecycleError::LifecycleWithUnsupported { source_path, property, .. } => {
            ("WithUnsupported", source_path, property)
        }
        LifecycleError::LifecycleStackInvalidShape { source_path, property, .. } => {
            ("StackInvalidShape", source_path, property)
        }
        other => panic!("{frontmatter}: unexpected error {other:?}"),
    };
    Refusal {
        variant,
        source_path: biscuit_file::to_forward_slash_string(source_path),
        property: property.clone(),
        text,
    }
}

/// Asserts `action`, placed at `initialize[0].else[0]`, is refused as
/// `variant` at `property` with a message containing `message`.
fn assert_refused(action: Value, variant: &str, property: &str, message: &str) {
    let frontmatter = json!({
        "initialize": [{ "when": "false", "then": [{ "info": "no" }], "else": [action.clone()] }],
    });
    let refused = refusal(&frontmatter);
    assert_eq!(refused.variant, variant, "{action}: {refused:?}");
    assert_eq!(refused.source_path, SOURCE, "{action}");
    assert_eq!(refused.property, format!("initialize[0].else[0].{property}"), "{action}: {}", refused.text);
    assert!(refused.text.contains(message), "{action}: `{}` lacks `{message}`", refused.text);
}

// ── control rows ──────────────────────────────────────────────────────────

#[test]
fn the_motivating_workflow_parses_to_an_exclusive_edit_in_the_else_branch() {
    let config = parse(&yaml(MOTIVATING)).expect("the motivating workflow parses");
    let actions = leaves(&config, LifecycleSignal::Initialize);
    let edits: Vec<(&EditAction, String)> = actions
        .iter()
        .filter_map(|action| match &action.kind {
            LifecycleActionKind::Edit(edit) => Some((edit, action.path.to_string())),
            _ => None,
        })
        .collect();
    assert_eq!(edits.len(), 1);
    let (edit, path) = &edits[0];
    assert_eq!(path, "initialize[0].else[0].edit");
    assert!(edit.create_new);
    assert_eq!(edit.gui, None);
    // `{{ fqn }}` is a whole-value interpolation: an expression for the
    // event to evaluate, never a literal path or a value read at parse time.
    assert_eq!(edit.file, Expr::Variable("fqn".to_string()));
}

#[test]
fn the_short_forms_open_the_operand_with_default_flags() {
    let (edit, action) = edit_of(json!({ "edit": "notes.md" }));
    assert_eq!(edit, EditAction { file: Expr::StringLiteral("notes.md".into()), create_new: false, gui: None });
    assert!(!action.no_error);
    assert_eq!(action.path.to_string(), "composed[0].edit");

    // A one-element list is the scalar form.
    assert_eq!(edit_of(json!({ "edit": ["{{ fqn }}"] })).0, edit_of(json!({ "edit": "{{ fqn }}" })).0);
}

#[test]
fn a_bare_word_operand_is_a_literal_file_name_not_a_variable() {
    assert_eq!(edit_of(json!({ "edit": "fqn" })).0.file, Expr::StringLiteral("fqn".into()));
}

#[test]
fn a_nonblank_literal_keeps_its_whitespace() {
    assert_eq!(edit_of(json!({ "edit": "  my notes.md " })).0.file, Expr::StringLiteral("  my notes.md ".into()));
    assert_eq!(
        edit_of(json!({ "edit": { "file": " a.md" } })).0.file,
        Expr::StringLiteral(" a.md".into())
    );
}

#[test]
fn a_mixed_template_operand_stays_a_template() {
    let (edit, _) = edit_of(json!({ "edit": "{{ ctx.cwd }}/notes.md" }));
    assert_eq!(edit.file, Expr::StringLiteral("{{ ctx.cwd }}/notes.md".into()));
}

#[test]
fn the_long_form_reads_each_literal_boolean() {
    for (create_new, gui) in [(false, false), (true, true), (true, false), (false, true)] {
        let (edit, action) = edit_of(json!({
            "edit": { "file": "a.md", "create_new": create_new, "gui": gui, "no_error": true }
        }));
        assert_eq!(edit.create_new, create_new);
        assert_eq!(edit.gui, Some(gui));
        assert!(action.no_error);
    }
    // Absent flags take their defaults: `false`, and either surface.
    let (edit, action) = edit_of(json!({ "edit": { "file": "a.md" } }));
    assert_eq!((edit.create_new, edit.gui, action.no_error), (false, None, false));
}

// ── `file`: the non-blank string validator ────────────────────────────────

#[test]
fn file_absent_is_refused() {
    assert_refused(json!({ "edit": { "create_new": true } }), "InvalidArgs", "edit", "`edit` requires a `file` parameter");
    assert_refused(json!({ "edit": {} }), "InvalidArgs", "edit", "`edit` requires a `file` parameter");
}

#[test]
fn file_null_is_refused() {
    assert_refused(json!({ "edit": { "file": null } }), "InvalidLongForm", "edit", "null values are not supported");
    // The short form's `null` supplies no operand.
    assert_refused(json!({ "edit": null }), "WrongArity", "edit", "expects exactly 1 argument, got 0");
}

#[test]
fn a_number_or_boolean_file_names_the_file_by_its_text() {
    // The shared conversion engine reads a scalar for a `string` parameter
    // as its text, so these name the files `5`, `true`, and `1.5`.
    for (operand, name) in [(json!(5), "5"), (json!(true), "true"), (json!(1.5), "1.5"), (json!(false), "false")] {
        let named = Expr::StringLiteral(name.to_string());
        assert_eq!(edit_of(json!({ "edit": operand.clone() })).0.file, named, "{operand}");
        assert_eq!(edit_of(json!({ "edit": [operand.clone()] })).0.file, named, "[{operand}]");
        assert_eq!(edit_of(json!({ "edit": { "file": operand.clone() } })).0.file, named, "file: {operand}");
    }
}

#[test]
fn file_of_the_wrong_type_is_refused() {
    assert_refused(json!({ "edit": { "file": ["a.md"] } }), "InvalidLongForm", "edit", "array values are not supported");
    assert_refused(json!({ "edit": { "file": { "a": 1 } } }), "ObjectParameter", "edit", "parameter `file`");
}

#[test]
fn a_list_element_of_the_wrong_type_is_refused() {
    // Every element wrong (a number element names a file; see
    // `a_number_or_boolean_file_names_the_file_by_its_text`).
    assert_refused(json!({ "edit": [null] }), "InvalidLongForm", "edit", "`initialize[0].else[0].edit.file` is not a valid value");
    assert_refused(json!({ "edit": [{ "a": 1 }] }), "InvalidLongForm", "edit", "object values are not supported");
    // One element wrong: a second operand is an arity error before its type matters.
    assert_refused(json!({ "edit": ["a.md", 1] }), "WrongArity", "edit", "expects exactly 1 argument, got 2");
    assert_refused(json!({ "edit": ["a.md", "b.md"] }), "WrongArity", "edit", "expects exactly 1 argument, got 2");
}

#[test]
fn an_empty_operand_is_refused() {
    assert_refused(json!({ "edit": "" }), "InvalidArgs", "edit", "`edit.file` must not be blank");
    assert_refused(json!({ "edit": "   " }), "InvalidArgs", "edit", "`edit.file` must not be blank");
    assert_refused(json!({ "edit": [""] }), "InvalidArgs", "edit", "`edit.file` must not be blank");
    assert_refused(json!({ "edit": { "file": " \t" } }), "InvalidArgs", "edit.file", "`edit.file` must not be blank");
    assert_refused(json!({ "edit": [] }), "WrongArity", "edit", "expects exactly 1 argument, got 0");
}

#[test]
fn a_bare_edit_is_an_arity_error_not_an_unknown_verb() {
    assert_refused(json!("edit"), "WrongArity", "edit", "expects exactly 1 argument, got 0");
}

#[test]
fn a_direct_object_short_operand_is_refused() {
    // `edit: { x: 1 }` is the long form without `file`; a nested mapping
    // in the list form is not an operand.
    assert_refused(json!({ "edit": { "x": 1 } }), "InvalidArgs", "edit", "`edit` requires a `file` parameter");
}

// ── `create_new` / `gui`: the literal-boolean validator ───────────────────

#[test]
fn a_flag_the_conversion_does_not_read_as_a_boolean_is_refused_at_its_key() {
    for flag in ["create_new", "gui"] {
        for (value, kind) in [
            // Flags are literals: a template is never evaluated.
            (json!("{{ true }}"), "string"),
            (json!("maybe"), "string"),
            // A native number never becomes a boolean.
            (json!(1), "number"),
            (json!(0), "number"),
            (json!([true]), "array"),
            (json!({ "a": 1 }), "object"),
        ] {
            let action = json!({ "edit": { "file": "a.md", flag: value } });
            assert_refused(
                action,
                "InvalidArgs",
                &format!("edit.{flag}"),
                &format!("`edit.{flag}` must be a boolean, got {kind}"),
            );
        }
    }
}

#[test]
fn a_boolean_like_flag_converts_and_a_null_flag_takes_its_default() {
    for (value, expected) in [(json!("yes"), true), (json!("true"), true), (json!("no"), false), (json!("False"), false)] {
        let (edit, _) = edit_of(json!({ "edit": { "file": "a.md", "create_new": value.clone(), "gui": value.clone() } }));
        assert_eq!((edit.create_new, edit.gui), (expected, Some(expected)), "{value}");
    }
    // `null` is no value: `create_new` is false and `gui` states no preference.
    let (edit, _) = edit_of(json!({ "edit": { "file": "a.md", "create_new": null, "gui": null } }));
    assert_eq!((edit.create_new, edit.gui), (false, None));
}

#[test]
fn unknown_and_with_keys_are_refused() {
    assert_refused(json!({ "edit": { "file": "a.md", "editor": "vim" } }), "InvalidLongForm", "edit", "unknown parameter(s): editor");
    assert_refused(json!({ "edit": { "file": "a.md", "with": { "a": 1 } } }), "WithUnsupported", "edit", "does not accept a `with` overlay");
}

// ── bundles, `no_error`, and placement ────────────────────────────────────

#[test]
fn a_compact_dictionary_runs_edit_after_communication_and_shell_before_flow_control() {
    let config = parse(&json!({
        "composed": { "stop": null, "ensure_file": "b.md", "edit": "a.md", "shell": "true", "info": "hi" },
    }))
    .expect("the bundle parses");
    let order: Vec<String> =
        leaves(&config, LifecycleSignal::Start).iter().map(|action| action.path.to_string()).collect();
    assert_eq!(
        order,
        ["composed.info", "composed.shell", "composed.edit", "composed.ensure_file", "composed.stop"]
    );
}

#[test]
fn a_sibling_no_error_applies_and_a_duplicate_is_refused() {
    let config = parse(&json!({ "composed": { "edit": "a.md", "no_error": true } })).expect("sibling no_error");
    let actions = leaves(&config, LifecycleSignal::Start);
    assert!(matches!(actions[0].kind, LifecycleActionKind::Edit(_)));
    assert!(actions[0].no_error);

    let refused = refusal(&json!({ "composed": { "edit": { "file": "a.md", "no_error": true }, "no_error": true } }));
    assert_eq!(refused.variant, "StackInvalidShape");
    assert_eq!(refused.property, "composed.edit.no_error");
    assert!(refused.text.contains("set both beside `edit` and inside it"), "{}", refused.text);

    // Equal placements are still two placements.
    let refused = refusal(&json!({ "composed": { "edit": { "file": "a.md", "no_error": false }, "no_error": false } }));
    assert!(refused.text.contains("set both beside `edit` and inside it"), "{}", refused.text);

    // A `null` placement is omitted, so it never conflicts.
    for frontmatter in [
        json!({ "composed": { "edit": { "file": "a.md", "no_error": null }, "no_error": true } }),
        json!({ "composed": { "edit": { "file": "a.md", "no_error": true }, "no_error": null } }),
    ] {
        let config = parse(&frontmatter).unwrap_or_else(|error| panic!("{frontmatter}: {error}"));
        assert!(leaves(&config, LifecycleSignal::Start)[0].no_error, "{frontmatter}");
    }

    // Boolean-like text converts; a native number does not.
    let (_, action) = edit_of(json!({ "edit": { "file": "a.md", "no_error": "yes" } }));
    assert!(action.no_error);
    assert_refused(
        json!({ "edit": { "file": "a.md", "no_error": 1 } }),
        "InvalidLongForm",
        "edit",
        "`no_error` must be a boolean, got number",
    );
}

#[test]
fn edit_is_an_expression_action_admitted_in_initialize_and_every_md_event() {
    let config = parse(&yaml(MOTIVATING)).expect("parses");
    let edit = leaves(&config, LifecycleSignal::Initialize)
        .into_iter()
        .find(|action| matches!(action.kind, LifecycleActionKind::Edit(_)))
        .expect("an edit");
    assert_eq!(action_type(&edit), Some(ActionType::Expression));
    assert!(!runs_a_shell(&edit), "`edit` is not a shell, so `initialize` admits it");

    for event in ["initialize", "composed", "failed", "cancelled", "finalize"] {
        let frontmatter = json!({ event: [{ "edit": "a.md" }, { "when": "true", "then": [{ "edit": "b.md" }] }] });
        parse(&frontmatter).unwrap_or_else(|error| panic!("`{event}` admits edit: {error}"));
    }
}

// ── registration ──────────────────────────────────────────────────────────

#[test]
fn edit_is_the_one_lifecycle_only_verb_and_a_known_verb() {
    assert_eq!(LIFECYCLE_ONLY_VERBS, ["edit"]);
    assert!(is_lifecycle_only_verb("edit"));
    assert!(!is_lifecycle_only_verb("ensure_file"));
    assert!(is_known_lifecycle_verb("edit"));
    assert!(all_lifecycle_verbs().contains(&"edit"));
}

#[test]
fn no_expression_function_or_side_effect_is_named_like_a_lifecycle_only_verb() {
    for verb in LIFECYCLE_ONLY_VERBS {
        assert!(
            !expression_function_descriptors().iter().any(|descriptor| &*descriptor.name == *verb),
            "`{verb}` must stay a lifecycle-only verb, never an expression function"
        );
        assert!(
            !EFFECT_DESCRIPTORS.iter().any(|descriptor| descriptor.signature.split('(').next() == Some(*verb)),
            "`{verb}` must stay a lifecycle-only verb, never a side effect"
        );
    }
}

#[test]
fn a_misspelled_edit_suggests_edit() {
    let refused = parse(&json!({ "composed": [{ "edti": "a.md" }] })).expect_err("unknown verb");
    let LifecycleError::LifecycleUnknownVerb { verb, property, rewrite, .. } = &*refused else {
        panic!("expected an unknown verb, got {refused:?}");
    };
    assert_eq!((verb.as_str(), property.as_str()), ("edti", "composed[0].edti"));
    assert_eq!(rewrite, "; did you mean `edit`?");
}

// ── passive surfaces ──────────────────────────────────────────────────────

#[test]
fn the_call_check_walks_the_file_operand_in_both_branches_without_evaluating_it() {
    let frontmatter = yaml(
        r#"
initialize:
  - when: file_exists(fqn)
    then:
      - edit: "{{ ctx.cwd }}/{{ fqn }}"
    else:
      - edit:
          file: "{{ missing_function_never_run(fqn) }}"
          gui: true
"#,
    );
    let config = parse(&frontmatter).expect("parses");
    let registry = EventRegistry::md();
    let surfaces: Vec<String> = lifecycle_call_checks(&frontmatter, &config, &registry, None)
        .into_iter()
        .map(|surface| format!("{} = {}", surface.property, surface.text))
        .collect();
    assert_eq!(
        surfaces,
        [
            "initialize[0].when = file_exists(fqn)",
            "initialize[0].then[0].edit.file = {{ ctx.cwd }}/{{ fqn }}",
            "initialize[0].else[0].edit.file = {{ missing_function_never_run(fqn) }}",
        ]
    );
}

#[test]
fn a_proven_bad_call_in_an_untaken_edit_branch_is_rejected_at_its_operand() {
    let frontmatter = yaml(
        r#"
composed:
  - when: "false"
    then:
      - info: never
    else:
      - edit: { file: "{{ min(true, 5) }}", create_new: true }
"#,
    );
    let config = parse(&frontmatter).expect("parses");
    let error = reject_incompatible_calls(&frontmatter, &config, Path::new(SOURCE), None, &EventRegistry::md(), None)
        .expect_err("a literal no overload accepts is rejected");
    let LifecycleError::LifecycleCallRejected { property, .. } = &error else {
        panic!("expected a call rejection, got {error:?}");
    };
    assert_eq!(property, "composed[0].else[0].edit.file");
}

#[test]
fn schema_template_operands_cover_file_but_not_the_boolean_flags() {
    let long = json!({ "file": "{{ fqn }}", "create_new": true, "gui": false, "no_error": true });
    let operands = template_operands("edit", OperandForm::Long, &long).expect("edit is parser-owned");
    let keys: Vec<String> = operands.iter().map(|operand| format!("{:?}", operand.location)).collect();
    assert_eq!(keys, [r#"Key("file")"#]);
    assert!(operands.iter().all(|operand| !operand.nested));

    let short = template_operands("edit", OperandForm::Short, &json!("{{ fqn }}")).expect("edit is parser-owned");
    assert_eq!(short.len(), 1);
    let listed = template_operands("edit", OperandForm::Short, &json!(["{{ fqn }}"])).expect("edit is parser-owned");
    assert_eq!(listed.len(), 1);
}
