//! Generated verbs check their arguments per catalog overload: a positional
//! call must fit one overload's count, and a long form must name exactly one
//! overload's parameters, which then order the arguments.

use serde_json::{Value, json};

use super::*;

fn parse(action: Value) -> Result<LifecycleConfig, LifecycleError> {
    parse_with(&json!({ "success": [action] }), UnknownVerbPolicy::Reject)
}

/// The verb and arguments of the one parsed action.
fn call(action: Value) -> (String, Vec<Expr>) {
    let config = parse(action.clone()).unwrap_or_else(|error| panic!("{action} parses: {error}"));
    let actions = leaves(&config, LifecycleSignal::Success);
    assert_eq!(actions.len(), 1, "{action}");
    match &actions[0].kind {
        LifecycleActionKind::ExpressionFunction(function) => (function.function.clone(), function.args.clone()),
        LifecycleActionKind::SideEffect(effect) => (effect.verb.clone(), effect.args.clone()),
        other => panic!("{action}: expected a generated verb, got {other:?}"),
    }
}

fn text(value: &str) -> Expr {
    Expr::StringLiteral(value.to_string())
}

fn long_form_message(action: Value) -> String {
    match parse(action.clone()) {
        Err(LifecycleError::LifecycleActionInvalidLongForm { message, .. }) => message,
        other => panic!("{action}: expected LifecycleActionInvalidLongForm, got {other:?}"),
    }
}

fn arity_message(action: Value) -> String {
    match parse(action.clone()) {
        Err(LifecycleError::LifecycleWrongArity { message, .. }) => message,
        other => panic!("{action}: expected LifecycleWrongArity, got {other:?}"),
    }
}

#[test]
fn link_long_form_with_file_alone_selects_the_one_parameter_overload() {
    // Was rejected while overloads were merged into `[target, desc?]`.
    for verb in ["link", "code_link"] {
        assert_eq!(call(json!({ verb: { "file": "a.md" } })), (verb.to_string(), vec![text("a.md")]));
    }
}

#[test]
fn link_long_form_with_target_requires_desc() {
    // Was accepted while overloads were merged; no overload takes `target` alone.
    for verb in ["link", "code_link"] {
        let message = long_form_message(json!({ verb: { "target": "a.md" } }));
        assert!(message.contains("missing required parameter(s): desc"), "{message}");
        assert!(message.contains(&format!("{verb}(target, desc)")), "{message}");

        let ordered = call(json!({ verb: { "desc": "A", "target": "a.md" } }));
        assert_eq!(ordered.1, [text("a.md"), text("A")], "args follow the selected overload's order");
    }
}

#[test]
fn list_long_forms_accept_either_overloads_parameter_name() {
    // Was rejected for `query`, the earlier overload's name, while the merge
    // kept only the last one-parameter overload's name.
    for verb in ["pr_list", "cicd_list"] {
        let (_, args) = call(json!({ verb: { "query": "{{ fm.query }}" } }));
        assert_eq!(args.len(), 1);
        assert_eq!(call(json!({ verb: { "count": 5 } })).1, [Expr::NumberLiteral(5.0)]);

        let message = long_form_message(json!({ verb: { "query": "{{ fm.query }}", "count": 5 } }));
        assert!(message.contains("no overload of"), "{message}");
    }
}

#[test]
fn branch_exists_on_remote_long_form_needs_branch_with_remote() {
    // Was accepted while the merge made both parameters optional.
    let message = long_form_message(json!({ "branch_exists_on_remote": { "remote": "origin" } }));
    assert!(message.contains("missing required parameter(s): branch"), "{message}");

    assert_eq!(call(json!({ "branch_exists_on_remote": { "branch": "main" } })).1, [text("main")]);
    assert_eq!(
        call(json!({ "branch_exists_on_remote": { "remote": "origin", "branch": "main" } })).1,
        [text("main"), text("origin")]
    );
}

#[test]
fn ensure_file_positional_fits_one_overload() {
    assert_eq!(call(json!({ "ensure_file": "a.md" })).1, [text("a.md")]);
    assert_eq!(call(json!({ "ensure_file": ["a.md", "body"] })).1, [text("a.md"), text("body")]);

    let message = arity_message(json!({ "ensure_file": ["a.md", "body", "extra"] }));
    assert!(message.contains("no overload that takes 3 argument(s)"), "{message}");
    assert!(message.contains("ensure_file(file) | ensure_file(file, content)"), "{message}");
}

#[test]
fn ensure_file_long_form_selects_by_keys() {
    assert_eq!(call(json!({ "ensure_file": { "file": "a.md" } })).1, [text("a.md")]);
    assert_eq!(
        call(json!({ "ensure_file": { "content": "body", "file": "a.md" } })).1,
        [text("a.md"), text("body")]
    );
}

#[test]
fn variadic_functions_stay_positional_only() {
    for verb in ["and", "or"] {
        let error = parse(json!({ verb: { "values": true } })).expect_err("long form of a variadic function");
        assert!(
            matches!(&error, LifecycleError::LifecycleExpressionFunctionKeyValueUnsupported { verb: named, .. } if named == verb),
            "{error:?}"
        );
        assert_eq!(call(json!({ verb: [true, false, true] })).1.len(), 3);
    }
}

#[test]
fn unknown_and_missing_parameters_are_still_rejected() {
    let unknown = long_form_message(json!({ "set_frontmatter": { "file": "a.md", "prop": "p", "value": 1, "bogus": 2 } }));
    assert!(unknown.starts_with("unknown parameter(s): bogus"), "{unknown}");

    let missing = long_form_message(json!({ "set_frontmatter": { "file": "a.md", "prop": "p" } }));
    assert_eq!(missing, "`set_frontmatter` is missing required parameter(s): value");

    let function = long_form_message(json!({ "length": { "value": "x" } }));
    assert!(function.starts_with("unknown parameter(s): value"), "{function}");
}
