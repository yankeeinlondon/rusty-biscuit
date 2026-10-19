//! The walk finds every speech phrase a parsed lifecycle can play.

use std::path::Path;

use darkmatter::markdown::compose::expression::Expr;
use serde_json::json;

use super::*;
use crate::composition::lifecycle::{parse_lifecycle_config, parse_task_action_stack};

/// `(authored path, literal phrase)` for each collected phrase, in order.
fn spoken(phrases: &[Phrase<'_>]) -> Vec<(String, String)> {
    phrases
        .iter()
        .map(|phrase| {
            let Expr::StringLiteral(text) = phrase.text else {
                panic!("expected a literal phrase at {}", phrase.path);
            };
            (phrase.path.to_string(), text.clone())
        })
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .collect()
}

#[test]
fn every_stack_but_initialize_is_walked_in_event_order() {
    let config = parse_lifecycle_config(
        &json!({
            "initialize": [{ "say": "never warmed" }],
            "start": [{ "say": "starting" }],
            "success": { "effect": "confirmation", "say": "bundled success" },
            "blocked": [{ "speak": "blocked alias" }],
            "failure": [{ "say": { "text": "named failure", "no_error": true } }],
            "finalize": [{ "say": ["positional finalize"] }],
            "loop": { "while": "false", "gate": [{ "say": "gate phrase" }] },
        }),
        Path::new("test.md"),
    )
    .expect("the fixture parses");

    let phrases = lifecycle_phrases(&config);

    assert_eq!(
        spoken(&phrases),
        pairs(&[
            ("start[0].say", "starting"),
            ("success.say", "bundled success"),
            ("blocked[0].speak", "blocked alias"),
            ("failure[0].say", "named failure"),
            ("finalize[0].say", "positional finalize"),
            ("loop.gate[0].say", "gate phrase"),
        ])
    );
    let scopes: Vec<LifecycleScope> = phrases.iter().map(|phrase| phrase.scope).collect();
    assert_eq!(
        scopes,
        [
            LifecycleSignal::Start,
            LifecycleSignal::Success,
            LifecycleSignal::Blocked,
            LifecycleSignal::Failure,
            LifecycleSignal::Finalize,
            LifecycleSignal::Loop,
        ]
        .map(LifecycleScope::Event)
    );
}

#[test]
fn loose_loop_verbs_are_walked() {
    let config = parse_lifecycle_config(
        &json!({ "loop": { "while": "false", "max": 2, "say": "loose loop phrase" } }),
        Path::new("test.md"),
    )
    .expect("the fixture parses");

    assert_eq!(
        spoken(&lifecycle_phrases(&config)),
        pairs(&[("loop.say", "loose loop phrase")])
    );
}

#[test]
fn both_branches_are_walked_at_every_depth_without_evaluating_guards() {
    // `fm.never` is absent, so a guard evaluation would select only `else`;
    // the walk must report the `then` bodies anyway.
    let config = parse_lifecycle_config(
        &json!({
            "success": [{
                "when": "fm.never",
                "then": [
                    { "say": "then phrase" },
                    { "when": "fm.a", "then": [
                        { "when": "fm.b", "then": [{ "speak": "depth three" }] },
                    ]},
                ],
                "else": [{ "say": "else phrase" }],
            }],
            "failure": [{ "when": "err.category == 'cap'", "say": "guarded bundle" }],
        }),
        Path::new("test.md"),
    )
    .expect("the fixture parses");

    assert_eq!(
        spoken(&lifecycle_phrases(&config)),
        pairs(&[
            ("success[0].then[0].say", "then phrase"),
            ("success[0].then[1].then[0].then[0].speak", "depth three"),
            ("success[0].else[0].say", "else phrase"),
            ("failure[0].say", "guarded bundle"),
        ])
    );
}

#[test]
fn only_speech_is_collected() {
    let config = parse_lifecycle_config(
        &json!({
            "success": [
                { "effect": "confirmation" },
                { "message": "not speech" },
                { "stderr": "not speech" },
                { "info": "not speech" },
                { "notify": "not speech" },
                { "say": "speech" },
            ],
        }),
        Path::new("test.md"),
    )
    .expect("the fixture parses");

    assert_eq!(
        spoken(&lifecycle_phrases(&config)),
        pairs(&[("success[5].say", "speech")])
    );
}

#[test]
fn frontmatter_keys_named_say_are_not_phrases() {
    let config = parse_lifecycle_config(
        &json!({
            "say": "a document property",
            "metadata": { "say": "nested property" },
            "start": [{ "info": "no speech here" }],
        }),
        Path::new("test.md"),
    )
    .expect("the fixture parses");

    assert!(lifecycle_phrases(&config).is_empty());
}

#[test]
fn task_setup_and_teardown_stacks_are_walked_in_their_own_scope() {
    let setup = parse_task_action_stack(
        LifecycleSignal::Start,
        &json!([{ "say": "setting up" }, { "when": "fm.x", "then": [{ "speak": "nested setup" }] }]),
        Path::new("test.md"),
        "tasks[0].setup",
    )
    .expect("setup parses");
    let teardown = parse_task_action_stack(
        LifecycleSignal::Finalize,
        &json!({ "say": "tearing down", "info": "bye" }),
        Path::new("test.md"),
        "tasks[0].teardown",
    )
    .expect("teardown parses");

    let setup_phrases = stack_phrases(LifecycleScope::TaskSetup, &setup);
    let teardown_phrases = stack_phrases(LifecycleScope::TaskTeardown, &teardown);

    assert_eq!(
        spoken(&setup_phrases),
        pairs(&[
            ("tasks[0].setup[0].say", "setting up"),
            ("tasks[0].setup[1].then[0].speak", "nested setup"),
        ])
    );
    assert!(setup_phrases.iter().all(|phrase| phrase.scope == LifecycleScope::TaskSetup));
    assert_eq!(
        spoken(&teardown_phrases),
        pairs(&[("tasks[0].teardown.say", "tearing down")])
    );
    assert!(teardown_phrases.iter().all(|phrase| phrase.scope == LifecycleScope::TaskTeardown));
}
