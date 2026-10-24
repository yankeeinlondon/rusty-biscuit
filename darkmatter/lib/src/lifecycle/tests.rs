//! The unknown-verb profile: [`UnknownVerbPolicy::Reject`] keeps the closed
//! grammar, [`UnknownVerbPolicy::Ignore`] keeps an unknown verb as an opaque
//! action, and every known verb and the `when`/`then`/`else` structure
//! validate identically under both.

use serde_json::{Value, json};

use super::*;

fn options(unknown_verbs: UnknownVerbPolicy) -> LifecycleParseOptions<'static> {
    LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        unknown_verbs,
        registry: super::registry::tests::agent_host(),
    }
}

fn parse_with(frontmatter: &Value, policy: UnknownVerbPolicy) -> Result<LifecycleConfig, LifecycleError> {
    parse_lifecycle_config_with(frontmatter, Path::new("doc.md"), &options(policy))
}

fn ignored(frontmatter: Value) -> LifecycleConfig {
    parse_with(&frontmatter, UnknownVerbPolicy::Ignore)
        .unwrap_or_else(|error| panic!("{frontmatter} parses under Ignore: {error}"))
}

/// Every action of `signal`'s stack, through both branches.
fn leaves(config: &LifecycleConfig, signal: LifecycleSignal) -> Vec<&LifecycleAction> {
    let mut leaves = Vec::new();
    actions::for_each_action(config.stack(signal), &mut |action| leaves.push(action));
    leaves
}

fn ignored_parts(action: &LifecycleAction) -> (String, String) {
    match &action.kind {
        LifecycleActionKind::Ignored(IgnoredAction { verb, path }) => {
            assert_eq!(path, &action.path, "the opaque action keeps the action's own path");
            (verb.clone(), path.to_string())
        }
        other => panic!("expected an ignored action, got {other:?}"),
    }
}

#[test]
fn reject_refuses_an_unknown_verb_with_the_closed_grammar_error() {
    for (frontmatter, property) in [
        (json!({ "start": [{ "md": "render" }] }), "start[0].md"),
        (json!({ "start": ["md"] }), "start[0].md"),
        (json!({ "start": { "md": "render" } }), "start.md"),
    ] {
        let error = parse_with(&frontmatter, UnknownVerbPolicy::Reject).expect_err("unknown verb");
        assert!(
            matches!(
                &error,
                LifecycleError::LifecycleUnknownVerb { verb, property: at, .. } if verb == "md" && at == property
            ),
            "{frontmatter}: {error:?}"
        );
    }
}

#[test]
fn reject_is_the_default_policy() {
    assert_eq!(UnknownVerbPolicy::default(), UnknownVerbPolicy::Reject);
}

#[test]
fn ignore_keeps_an_unknown_verb_as_an_opaque_action_with_its_name_and_path() {
    let config = ignored(json!({
        "start": [{ "md": "render" }, "md", { "info": "next" }],
        "success": { "md": "render", "say": "done" },
    }));

    let start = leaves(&config, LifecycleSignal::Start);
    assert_eq!(ignored_parts(start[0]), ("md".to_string(), "start[0].md".to_string()));
    assert_eq!(ignored_parts(start[1]), ("md".to_string(), "start[1].md".to_string()));
    assert!(matches!(start[2].kind, LifecycleActionKind::Communication(_)));

    let success = leaves(&config, LifecycleSignal::Success);
    let verbs: Vec<String> = success.iter().map(|action| action.path.to_string()).collect();
    assert_eq!(verbs, ["success.say", "success.md"], "a bundle still expands in canonical order");
    assert_eq!(ignored_parts(success[1]), ("md".to_string(), "success.md".to_string()));
}

#[test]
fn ignore_never_interprets_an_unknown_verbs_operand() {
    let frontmatter = json!({
        "initialize": [
            { "md": "{{ err.code }}" },
            { "md": "$(echo initialize)" },
            { "md": { "nested": { "deep": "$(rm -rf /)", "span": "{{ 'a {{ b }}' }}" } } },
            { "md": ["{{ fm. }}", { "x": 1 }] },
            { "md": "{{ 'a {{ b }}' }}" },
        ],
        "start": { "say": "hi", "md": { "long": "form", "beside": "another verb" } },
    });
    let config = ignored(frontmatter.clone());

    let initialize = leaves(&config, LifecycleSignal::Initialize);
    assert_eq!(initialize.len(), 5);
    for action in &initialize {
        assert_eq!(ignored_parts(action).0, "md");
    }
    let mut surfaces = Vec::new();
    for signal in LifecycleSignal::ALL {
        walk_stack(config.stack(signal), &mut |path, _| surfaces.push(path.to_string()));
    }
    assert_eq!(surfaces, ["start.say.text"], "an ignored operand has no expression surface");
    assert!(collect_lifecycle_shell_commands(&config).is_empty(), "an ignored operand is never a shell");
    validate_no_nested_spans_in_literals(&frontmatter, &config, Path::new("doc.md"))
        .expect("an ignored operand is never scanned for nested spans");
}

#[test]
fn ignore_still_rejects_a_malformed_known_verb() {
    for frontmatter in [
        json!({ "start": [{ "stop": "now" }] }),
        json!({ "start": [{ "say": { "bogus": "x" } }] }),
        json!({ "start": [{ "shell": "{{ fm. }}" }] }),
        json!({ "initialize": [{ "shell": "echo hi" }] }),
        json!({ "start": [{ "skip": [] }] }),
        json!({ "start": [{ "stop": [], "error": "x" }] }),
        json!({ "start": [{ "set": ["k", "v"] }] }),
        json!({ "start": [{ "stack": [] }] }),
        json!({ "start": ["say(\"hi\")"] }),
    ] {
        let under_reject = parse_with(&frontmatter, UnknownVerbPolicy::Reject).expect_err("malformed");
        let under_ignore = parse_with(&frontmatter, UnknownVerbPolicy::Ignore).expect_err("malformed");
        assert_eq!(under_ignore.to_string(), under_reject.to_string(), "{frontmatter}");
    }
}

#[test]
fn ignore_still_validates_when_then_else_structure() {
    for (frontmatter, expected) in [
        (json!({ "start": [{ "then": [{ "md": "x" }] }] }), "`then` requires `when` on the same item"),
        (json!({ "start": [{ "when": "true", "else": [{ "md": "x" }] }] }), "`else` requires `then`"),
        (json!({ "start": [{ "when": "true", "then": [] }] }), "an explicit `then` must contain at least one action"),
        (json!({ "start": [{ "when": "true", "then": [{ "md": "x" }], "md": "y" }] }), "cannot also carry the sibling `md`"),
        (json!({ "start": { "when": "true", "md": "x" } }), "a dictionary of verbs never carries `when`"),
        (json!({ "start": [{ "when": "(", "then": [{ "md": "x" }] }] }), "`when` is not a valid expression"),
    ] {
        let error = parse_with(&frontmatter, UnknownVerbPolicy::Ignore).expect_err("structure error");
        assert!(
            matches!(
                error,
                LifecycleError::LifecycleStackInvalidShape { .. } | LifecycleError::LifecycleWhenExpressionInvalid { .. }
            ),
            "{frontmatter}: {error:?}"
        );
        assert!(error.to_string().contains(expected), "{frontmatter}: {error}");
    }

    let config = ignored(json!({
        "start": [{ "when": "true", "then": [{ "md": "x" }], "else": { "md": "y" } }],
    }));
    let paths: Vec<(String, String)> = leaves(&config, LifecycleSignal::Start).into_iter().map(ignored_parts).collect();
    assert_eq!(
        paths,
        [("md".to_string(), "start[0].then[0].md".to_string()), ("md".to_string(), "start[0].else.md".to_string())]
    );
}

fn parse_under(frontmatter: &Value, registry: &super::registry::EventRegistry) -> Result<LifecycleConfig, LifecycleError> {
    let options = LifecycleParseOptions { registry, ..options(UnknownVerbPolicy::Reject) };
    parse_lifecycle_config_with(frontmatter, Path::new("doc.md"), &options)
}

#[test]
fn a_renaming_host_refuses_the_darkmatter_spelling_before_any_stack_parses() {
    for (authored, replacement) in [("composed", "start"), ("failed", "blocked")] {
        let frontmatter = json!({ authored: [], "start": [{ "unknown_verb": 1 }] });
        let error = parse_with(&frontmatter, UnknownVerbPolicy::Reject).unwrap_err();
        let LifecycleError::LifecycleRenamedEvent { authored: found, replacement: named, .. } = &error else {
            panic!("expected LifecycleRenamedEvent, got {error:?}");
        };
        assert_eq!((found.as_str(), named.as_str()), (authored, replacement));
        assert!(error.to_string().contains(&format!("write `{replacement}:`")), "{error}");
    }
}

#[test]
fn md_parses_its_own_spellings_and_leaves_host_only_keys_as_data() {
    let md = super::registry::EventRegistry::md();
    let frontmatter = json!({
        "composed": [{ "info": "composed" }],
        "failed": [{ "warn": "failed" }],
        "start": [{ "say": "hi" }],
        "success": "not a stack",
        "loop": 3,
    });
    let config = parse_under(&frontmatter, &md).unwrap();
    assert_eq!(config.stack(LifecycleSignal::Start).len(), 1);
    assert_eq!(config.stack(LifecycleSignal::Blocked).len(), 1);
    assert!(config.stack(LifecycleSignal::Success).is_empty());
    assert!(config.stack(LifecycleSignal::Loop).is_empty());
}

mod overloads;
