use super::*;
use super::super::types::{AgentHint, ModelHint};
use crate::provider::Provider;
use darkmatter::markdown::Frontmatter;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn make_source(
    dir: &TempDir,
    frontmatter: &[(&str, serde_json::Value)],
    content: &str,
) -> ResolvedCompositionSource {
    let file = dir.path().join("test.md");
    let mut fm = Frontmatter::new();
    for (key, value) in frontmatter {
        fm.insert(key, value.clone()).unwrap();
    }
    let md = Markdown::with_frontmatter(fm, content);
    fs::write(&file, md.as_string()).unwrap();

    // Read once and construct Markdown from the string to avoid double I/O
    let original_text = fs::read_to_string(&file).unwrap();
    let markdown: Markdown = original_text.clone().into();
    ResolvedCompositionSource {
        original_ref: file.to_str().unwrap().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    }
}

#[test]
fn direct_composition_uses_effective_frontmatter() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("title", json!("Research")), ("agent", json!("codex"))],
        "# Research\n\nDo the research.",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.mode, CompositionMode::ChainedDocument);
    assert!(prepared.prompt.contains("Research"));
    // Effective frontmatter should be a JSON object with the keys
    assert!(prepared.effective_frontmatter.is_object());
    let fm_obj = prepared.effective_frontmatter.as_object().unwrap();
    assert_eq!(fm_obj.get("title"), Some(&json!("Research")));
    assert_eq!(
        prepared.selection_hints.agent,
        Some(AgentHint::Single(Provider::Codex))
    );
    assert!(matches!(prepared.closure, CompositionClosurePlan::Direct));
}

#[test]
fn inline_composition_uses_effective_frontmatter() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("prompt", json!("List three colors")),
            ("agent", json!("claude")),
        ],
        "Old content",
    );

    let prepared = prepare_inline(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.mode, CompositionMode::InlineFrontmatterPrompt);
    assert!(prepared.prompt.contains("List three colors"));
    assert!(
        prepared
            .prompt
            .contains("Return the replacement Markdown body content only")
    );

    // Effective frontmatter should contain composed keys
    assert!(prepared.effective_frontmatter.is_object());
    let fm_obj = prepared.effective_frontmatter.as_object().unwrap();
    assert!(fm_obj.contains_key("prompt"));
    assert_eq!(
        prepared.selection_hints.agent,
        Some(AgentHint::Single(Provider::Claude))
    );

    // Closure should be Inline with captured hash
    match &prepared.closure {
        CompositionClosurePlan::Inline(plan) => {
            assert!(!plan.original_document_text.is_empty());
            // Simple hash should produce a non-empty `<fm>-<body>` string.
            let flat = plan.original_hash.flat_string();
            assert!(flat.is_some() && !flat.unwrap().is_empty());
        }
        CompositionClosurePlan::Direct => panic!("expected Inline closure plan"),
    }
}

#[test]
fn inline_composition_missing_prompt() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Content");

    let err = prepare_inline(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::PromptPropertyMissing));
}

#[test]
fn inline_composition_wrong_type() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("prompt", json!(42))], "Content");

    let err = prepare_inline(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::PromptPropertyWrongType(_)));
}

#[test]
fn direct_composition_parses_lifecycle_config() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"stderr": "Starting", "effect": "doorbell"})),
            ("success", json!({"say": "All done"})),
        ],
        "Do the work.",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert!(prepared.lifecycle.start.is_some());
    assert!(prepared.lifecycle.success.is_some());
    assert!(prepared.lifecycle.blocked.is_none());
    assert!(prepared.lifecycle.failure.is_none());
}

/// Regression: Darkmatter's `normalize_list_spacing` used to insert a
/// blank line between a tight parent list item and the first child of a
/// nested unordered list. That blank line was mis-rendered as an indented
/// code block by downstream renderers, corrupting prompts like the
/// `## Closure` section of `prompts/review-feature.md`.
#[test]
fn direct_composition_preserves_tight_nested_list() {
    let dir = TempDir::new().unwrap();
    let body = r#"## Closure

- Save your review suggestions to a file
- Save the following frontmatter properties on "review.md":
    - based on your review suggestions indicate whether you think this feature is **ready for production**
    - set the `agent` frontmatter property to claude
    - set the `model` frontmatter property to some-model
    - set the `created` frontmatter property to today

**bold:**
"#;
    let source = make_source(&dir, &[("title", json!("Test"))], body);

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert!(
        prepared.prompt.contains("properties on \"review.md\":\n    - based on"),
        "parent item must be immediately followed by its first child; got:\n{}",
        prepared.prompt
    );
    assert!(
        !prepared.prompt.contains("properties on \"review.md\":\n\n    - "),
        "tight nested list must not gain a blank line between parent and child; got:\n{}",
        prepared.prompt
    );
}

#[test]
fn inline_composition_parses_lifecycle_config() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("prompt", json!("Write something")),
            ("failure", json!({"stderr": "Failed"})),
        ],
        "Old content",
    );

    let prepared = prepare_inline(&source, PrepareOptions::default()).unwrap();
    assert!(prepared.lifecycle.failure.is_some());
    assert!(prepared.lifecycle.start.is_none());
}

#[test]
fn invalid_lifecycle_config_fails_preparation() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"say": "Hello", "say_first": "Also hello"})),
        ],
        "Content",
    );

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::LifecycleSayConflict(_)));
}

#[test]
fn lifecycle_malformed_span_is_deferred_raw_through_prepare() {
    // C1: lifecycle communication strings are deferred from compose-time
    // resolution, so their authored `{{ }}` spans survive prepare intact.
    // A malformed span is no longer a prepare-time leak — it is resolved
    // (and fails closed) at event-time via DM2 (C2). Prepare keeps the raw
    // span verbatim.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"message": "leak {{ parent_dir(review)) }}"})),
        ],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    let message = prepared
        .lifecycle
        .start
        .as_ref()
        .unwrap()
        .message
        .as_ref()
        .unwrap();
    assert_eq!(message, "leak {{ parent_dir(review)) }}");
}

#[test]
fn undefined_lifecycle_variable_is_deferred_not_rejected_at_prepare() {
    // Previously a bare `{{ missing }}` in a lifecycle string was rejected
    // at prepare. With event-time interpolation (C2) the span is deferred;
    // an unknown root fails closed at event-time via DM2 strict mode rather
    // than at prepare. Prepare keeps the raw span.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            (
                "start",
                json!({"message": "before {{ missing_lifecycle_var }} after"}),
            ),
        ],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    let message = prepared
        .lifecycle
        .start
        .as_ref()
        .unwrap()
        .message
        .as_ref()
        .unwrap();
    assert_eq!(message, "before {{ missing_lifecycle_var }} after");
}

#[test]
fn lifecycle_message_referencing_frontmatter_is_deferred_raw() {
    // A bare frontmatter reference in a lifecycle string is deferred to
    // event-time, not resolved at prepare.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("area", json!("claudine")),
            ("start", json!({"message": "working on {{ area }}"})),
        ],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    let message = prepared
        .lifecycle
        .start
        .as_ref()
        .unwrap()
        .message
        .as_ref()
        .unwrap();
    assert_eq!(message, "working on {{ area }}");
}

#[test]
fn lifecycle_fallback_span_is_deferred_raw() {
    // Fallback (`{{ x || 'y' }}`) is deferred like any other lifecycle
    // span; it resolves at event-time.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "start",
            json!({"message": "{{ missing_lifecycle_var || 'default' }}"}),
        )],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    let message = prepared
        .lifecycle
        .start
        .as_ref()
        .unwrap()
        .message
        .as_ref()
        .unwrap();
    assert_eq!(message, "{{ missing_lifecycle_var || 'default' }}");
}

#[test]
fn lifecycle_ctx_interpolation_is_deferred_raw() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"message": "{{ ctx.today }}"})),
        ],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    let start = prepared.lifecycle.start.as_ref().unwrap();
    let message = start.message.as_ref().unwrap();
    // Deferred: the span survives raw for event-time interpolation (C2).
    assert_eq!(message, "{{ ctx.today }}");
}

#[test]
fn multiple_lifecycle_spans_all_deferred_raw() {
    // Every lifecycle communication string is deferred; none trips a
    // prepare-time leak. They all resolve at event-time.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"message": "leak {{ parent_dir(review)) }}"})),
            ("failure", json!({"say": "leak {{ broken( }}"})),
        ],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(
        prepared.lifecycle.start.as_ref().unwrap().message.as_deref(),
        Some("leak {{ parent_dir(review)) }}")
    );
    assert_eq!(
        prepared.lifecycle.failure.as_ref().unwrap().say.as_deref(),
        Some("leak {{ broken( }}")
    );
}

#[test]
fn malformed_whole_value_spec_path_is_rejected() {
    // Regression for the original `implement-suggestions.md` reproduction:
    // a `spec_path` frontmatter value that is a malformed whole-value
    // interpolation — `{{ dirname(review) + '/spec.md') }}` carries an
    // unbalanced paren. A whole-value `{{ … }}` is executable state, so
    // composition must abort with a frontmatter interpolation parse error
    // that names `spec_path`, instead of leaking the raw template
    // downstream as a successful effective-frontmatter value. The fixture
    // is self-contained — it must not read the shipped prompt, whose shape
    // is free to change without breaking this guard.
    let dir = TempDir::new().unwrap();
    let review_file = dir.path().join("review.md");
    fs::write(&review_file, "# Review\n").unwrap();

    let source = make_source(
        &dir,
        &[
            ("title", json!("Implement Suggestions")),
            ("spec_path", json!("{{ dirname(review) + '/spec.md') }}")),
        ],
        "Implement the suggestions from {{ spec_path }}.",
    );

    let options = PrepareOptions {
        set_overrides: Some(json!({ "review": review_file.to_str().unwrap() })),
        ..Default::default()
    };

    let err = prepare_direct(&source, options).unwrap_err();
    // The offending key is captured as structured scope (not Display prose),
    // and the typed cause is an interpolation parse failure.
    let CompositionError::ComposeFailed(MarkdownError::Interpolation { key, cause, .. }) = &err
    else {
        panic!("expected ComposeFailed(Interpolation), got: {err:?}");
    };
    assert_eq!(
        key.as_deref(),
        Some("spec_path"),
        "error must capture the offending key"
    );
    assert!(
        matches!(
            cause.as_ref(),
            darkmatter::markdown::compose::expression::ExpressionError::Parse(_)
        ),
        "cause must be an interpolation parse failure, got: {cause:?}"
    );
}

#[test]
fn direct_composition_with_env_overrides() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("title", json!("Test"))],
        "FAIL_FAST is {{env.FAIL_FAST}}",
    );

    let options = PrepareOptions {
        env_overrides: std::collections::BTreeMap::from([(
            "FAIL_FAST".to_string(),
            "false".to_string(),
        )]),
        ..Default::default()
    };

    let prepared = prepare_direct(&source, options).unwrap();
    assert!(prepared.prompt.contains("FAIL_FAST is false"));
}

#[test]
fn direct_lifecycle_ctx_message_is_deferred_raw() {
    // A lifecycle message referencing `ctx.*` is deferred to event-time;
    // env-override-driven ctx resolution happens then (C2), not at prepare.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "start",
            json!({
                "message": "{{ctx.agent}}/{{ctx.model}}"
            }),
        )],
        "Prompt",
    );

    let options = PrepareOptions {
        env_overrides: std::collections::BTreeMap::from([
            ("AGENT".to_string(), "codex".to_string()),
            ("MODEL".to_string(), "gpt-5".to_string()),
        ]),
        ..Default::default()
    };

    let prepared = prepare_direct(&source, options).unwrap();
    let start = prepared.lifecycle.start.as_ref().unwrap();
    assert_eq!(start.message.as_deref(), Some("{{ctx.agent}}/{{ctx.model}}"));
}

#[test]
fn direct_composition_perf_disabled_yields_none() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Simple content.");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert!(prepared.compose_perf.is_none());
}

#[test]
fn direct_composition_perf_enabled_yields_some() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Simple content.");

    let options = PrepareOptions {
        perf_enabled: true,
        ..Default::default()
    };
    let prepared = prepare_direct(&source, options).unwrap();
    assert!(prepared.compose_perf.is_some());
}

#[test]
fn inline_composition_preserves_closure_with_perf_enabled() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("prompt", json!("List three colors")),
            ("agent", json!("claude")),
        ],
        "Old content",
    );

    let options = PrepareOptions {
        perf_enabled: true,
        ..Default::default()
    };
    let prepared = prepare_inline(&source, options).unwrap();
    assert_eq!(prepared.mode, CompositionMode::InlineFrontmatterPrompt);
    assert!(prepared.compose_perf.is_some());

    // Closure should still be Inline with captured hash
    match &prepared.closure {
        CompositionClosurePlan::Inline(plan) => {
            assert!(!plan.original_document_text.is_empty());
            let flat = plan.original_hash.flat_string();
            assert!(flat.is_some() && !flat.unwrap().is_empty());
        }
        CompositionClosurePlan::Direct => panic!("expected Inline closure plan"),
    }
}

#[test]
fn direct_composition_parses_agent_list() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!(["gemini", "codex"]))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(
        prepared.selection_hints.agent,
        Some(AgentHint::List(vec![Provider::Gemini, Provider::Codex]))
    );
}

#[test]
fn direct_composition_parses_model_single() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!("gpt-4o"))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(
        prepared.selection_hints.model,
        Some(ModelHint::Single("gpt-4o".to_string()))
    );
}

#[test]
fn direct_composition_parses_model_list() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!(["gpt-4o", "o3-mini"]))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(
        prepared.selection_hints.model,
        Some(ModelHint::List(vec![
            "gpt-4o".to_string(),
            "o3-mini".to_string()
        ]))
    );
}

#[test]
fn direct_composition_agent_unknown_provider_is_non_fatal() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!("unknown-provider"))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.agent, None);
    assert_eq!(
        prepared.selection_hints.agent_invalid,
        vec!["unknown-provider".to_string()]
    );
}

#[test]
fn direct_composition_agent_list_skips_invalid_entries() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("agent", json!(["claude", "not-real", "codex"]))],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(
        prepared.selection_hints.agent,
        Some(AgentHint::List(vec![Provider::Claude, Provider::Codex]))
    );
    assert_eq!(
        prepared.selection_hints.agent_invalid,
        vec!["not-real".to_string()]
    );
}

#[test]
fn direct_composition_agent_list_all_invalid_is_empty_hint() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!(["bad", "worse"]))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.agent, None);
    assert_eq!(
        prepared.selection_hints.agent_invalid,
        vec!["bad".to_string(), "worse".to_string()]
    );
    // Even with no valid providers, the list-ness must survive so a
    // zero-valid list is not mistaken for a scalar value downstream.
    assert!(prepared.selection_hints.agent_was_list);
}

#[test]
fn direct_composition_single_entry_all_invalid_list_preserves_list_flag() {
    // A one-element invalid list (`agent: ["not-real"]`) must record
    // `agent_was_list = true` so classification routes it to the
    // zero-installed-list state, not the single-invalid scalar state.
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!(["not-real"]))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.agent, None);
    assert_eq!(
        prepared.selection_hints.agent_invalid,
        vec!["not-real".to_string()]
    );
    assert!(prepared.selection_hints.agent_was_list);
}

#[test]
fn direct_composition_single_scalar_invalid_is_not_list() {
    // A scalar invalid value (`agent: not-real`) must record
    // `agent_was_list = false`.
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!("not-real"))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.agent, None);
    assert!(!prepared.selection_hints.agent_was_list);
}

#[test]
fn direct_composition_agent_wrong_type_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!(42))], "Content");

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::AgentHintWrongType(_)));
}

#[test]
fn direct_composition_model_wrong_type_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!(42))], "Content");

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::ModelHintWrongType(_)));
}

#[test]
fn direct_composition_agent_list_with_non_string_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!(["claude", 42]))], "Content");

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::AgentHintWrongType(_)));
}

#[test]
fn direct_composition_model_list_with_non_string_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!(["gpt-4o", 42]))], "Content");

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::ModelHintWrongType(_)));
}

#[test]
fn direct_composition_no_agent_or_model_hints() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.agent, None);
    assert_eq!(prepared.selection_hints.model, None);
}

#[test]
fn direct_composition_empty_body_returns_composed_body_empty() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "   \n\n  \t\n");

    let options = PrepareOptions {
        set_overrides: Some(json!({"spec": "plan.md", "phase": 1})),
        ..Default::default()
    };

    let err = prepare_direct(&source, options).unwrap_err();
    match err {
        CompositionError::ComposedBodyEmpty {
            source_path,
            mode,
            provided_overrides,
        } => {
            assert_eq!(source_path, source.resolved_path);
            assert_eq!(mode, CompositionMode::ChainedDocument);
            let keys: std::collections::HashSet<String> =
                provided_overrides.into_iter().collect();
            assert_eq!(
                keys,
                ["spec", "phase"].iter().map(|s| s.to_string()).collect()
            );
        }
        other => panic!("expected ComposedBodyEmpty, got: {other:?}"),
    }
}

#[test]
fn direct_composition_block_strips_everything_returns_composed_body_empty() {
    let dir = TempDir::new().unwrap();
    // Body consists entirely of a `::block when="review"` whose guard is
    // not satisfied (no `review` override provided). After composition
    // the body should be empty and detection must fire — this mirrors
    // the real-world prompt that triggered the user-facing bug report.
    let body = "::block when=\"review\"\n## Context\n\nThis is review-only content.\n::end-block\n";
    let source = make_source(&dir, &[("title", json!("Test"))], body);

    let options = PrepareOptions {
        set_overrides: Some(json!({"spec": "plan.md"})),
        ..Default::default()
    };

    let err = prepare_direct(&source, options).unwrap_err();
    let CompositionError::ComposedBodyEmpty {
        mode,
        provided_overrides,
        ..
    } = err
    else {
        panic!("expected ComposedBodyEmpty, got a different error");
    };
    assert_eq!(mode, CompositionMode::ChainedDocument);
    assert_eq!(provided_overrides, vec!["spec".to_string()]);
}

/// Regression: a `@`-resolved prompt physically lives outside the repo
/// it reasons about (e.g. `~/.claudine/prompts/commit.md`). With
/// `shell_working_directory` set, `::shell` directives must run there,
/// not next to the template file — otherwise `sniff repo packages` and
/// friends execute in the wrong repo.
#[test]
fn direct_composition_runs_shell_in_configured_working_directory() {
    let source_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let source = make_source(&source_dir, &[("title", json!("T"))], "::shell pwd\n");

    let mut approved = std::collections::HashSet::new();
    approved.insert("pwd".to_string());
    let options = PrepareOptions {
        pre_approved_commands: Some(approved),
        shell_working_directory: Some(work_dir.path().to_path_buf()),
        ..Default::default()
    };

    let prepared = prepare_direct(&source, options).unwrap();
    // `pwd` reports the physical path, so compare against canonicalized
    // temp dirs (macOS routes `/var` through a `/private` symlink).
    let work_canon = std::fs::canonicalize(work_dir.path()).unwrap();
    let source_canon = std::fs::canonicalize(source_dir.path()).unwrap();
    assert!(
        prepared.prompt.contains(work_canon.to_str().unwrap()),
        "expected shell to run in work_dir; got: {}",
        prepared.prompt
    );
    assert!(
        !prepared.prompt.contains(source_canon.to_str().unwrap()),
        "shell must not run in the prompt's parent dir; got: {}",
        prepared.prompt
    );
}

#[test]
fn direct_composition_parses_interactive_hint() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("interactive", json!(true))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.interactive, Some(true));
}

#[test]
fn direct_composition_interactive_null_is_absent() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("interactive", json!(null))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.interactive, None);
}

#[test]
fn direct_composition_interactive_wrong_type_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("interactive", json!("yes"))], "Content");

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    assert!(matches!(err, CompositionError::InteractiveHintWrongType(_)));
}

#[test]
fn inline_composition_parses_interactive_hint() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("prompt", json!("Write something")), ("interactive", json!(false))],
        "Old content",
    );

    let prepared = prepare_inline(&source, PrepareOptions::default()).unwrap();
    assert_eq!(prepared.selection_hints.interactive, Some(false));
}

// ── C1: deferred lifecycle subtree (raw spans survive prepare) ───────

#[test]
fn lifecycle_err_span_survives_raw_in_effective_frontmatter() {
    // C1: deferring the lifecycle keys leaves `{{err.msg}}` raw in
    // `effective_frontmatter`, and `parse_lifecycle_config` reads it raw —
    // the span the original bug collapsed to empty now survives for
    // event-time interpolation.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("failure", json!({"message": "❌️ {{err.msg}}"}))],
        "Do the work.",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();

    let fm = prepared.effective_frontmatter.as_object().unwrap();
    let failure = fm.get("failure").unwrap();
    assert_eq!(
        failure.get("message").unwrap(),
        &json!("❌️ {{err.msg}}"),
        "deferred lifecycle subtree retains its raw span"
    );
    assert_eq!(
        prepared
            .lifecycle
            .failure
            .as_ref()
            .unwrap()
            .message
            .as_deref(),
        Some("❌️ {{err.msg}}"),
        "parse_lifecycle_config sees the raw span"
    );
}

// ── C3: pre-flight shell resolution (early-binding only) ─────────────

fn first_shell_command(
    lifecycle: &crate::composition::lifecycle::LifecycleConfig,
    signal: crate::composition::lifecycle::LifecycleSignal,
) -> darkmatter::markdown::compose::expression::Expr {
    use crate::composition::lifecycle_actions::LifecycleActionKind;
    let stack = lifecycle.stack(signal).expect("stack present");
    for item in stack {
        for action in &item.actions {
            if let LifecycleActionKind::Shell(shell) = &action.kind {
                return shell.command.clone();
            }
        }
    }
    panic!("no shell action in stack");
}

#[test]
fn shell_command_early_binding_resolves_at_preflight() {
    use crate::composition::lifecycle::LifecycleSignal;
    use darkmatter::markdown::compose::expression::Expr;
    // `{"shell": "git fetch {{branch}}"}` resolves `branch` (a frontmatter
    // key) at pre-flight; the stamped command equals what will execute.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("branch", json!("main")),
            (
                "start",
                json!({"stack": [{"action": {"shell": "git fetch {{branch}}"}}]}),
            ),
        ],
        "Do the work.",
    );

    let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
    let command = first_shell_command(&prepared.lifecycle, LifecycleSignal::Start);
    assert_eq!(command, Expr::StringLiteral("git fetch main".to_string()));
}

#[test]
fn shell_command_late_binding_reference_rejected_at_prepare() {
    // `{"shell": "rm {{err.msg}}"}` references a late-binding global;
    // shell is resolved at pre-flight (before any event fires), so it is
    // rejected with the property path.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "failure",
            json!({"stack": [{"action": {"shell": "rm {{err.msg}}"}}]}),
        )],
        "Do the work.",
    );

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::LifecycleShellResolution {
            property,
            raw,
            message,
            ..
        } => {
            assert_eq!(property, "failure.stack[0].action[0].command");
            assert_eq!(raw, "rm {{err.msg}}");
            assert!(
                message.contains("err"),
                "message names the late-binding root: {message}"
            );
        }
        other => panic!("expected LifecycleShellResolution, got: {other:?}"),
    }
}

#[test]
fn shell_long_form_command_late_binding_rejected_at_prepare() {
    // Long-form `command: "rm {{err.msg}}"` is rejected the same way as
    // the positional `shell: ...` form.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "failure",
            json!({"stack": [{"action": {"action": "shell", "command": "rm {{err.msg}}"}}]}),
        )],
        "Do the work.",
    );

    let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
    match err {
        CompositionError::LifecycleShellResolution { property, raw, .. } => {
            assert_eq!(property, "failure.stack[0].action[0].command");
            assert_eq!(raw, "rm {{err.msg}}");
        }
        other => panic!("expected LifecycleShellResolution, got: {other:?}"),
    }
}

#[test]
fn compose_error_preserves_frontmatter_fence_mismatch_source() {
    // `map_compose_error` is a catch-all for non-shell MarkdownErrors.
    // A compose-time (transclusion/reload) surface that produces a
    // `FrontmatterFenceMismatch` must keep the typed error in the chain so
    // the CLI walker can render the rich Darkmatter block instead of a flat
    // string.
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("nested.md"),
        PathBuf::from("nested.md"),
        "----\nname: x\n----\n".to_string(),
    );
    let md_err = MarkdownError::FrontmatterFenceMismatch {
        ctx,
        found: "----".to_string(),
        line: 1,
    };
    let path = PathBuf::from("nested.md");
    let composed = map_compose_error(&path, md_err);

    match composed {
        CompositionError::ComposeFailed(inner) => {
            let msg = inner.to_string();
            assert!(
                msg.contains("----"),
                "typed error must name the offending fence: {msg}"
            );
            assert!(
                matches!(
                    inner,
                    MarkdownError::FrontmatterFenceMismatch { ref found, .. } if found == "----"
                ),
                "inner MarkdownError must be FrontmatterFenceMismatch: {inner:?}"
            );
        }
        other => panic!("expected ComposeFailed, got: {other:?}"),
    }
}
