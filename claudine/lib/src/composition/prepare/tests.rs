use super::*;
use super::super::types::{AgentHint, ModelHint};
use crate::provider::Provider;
use darkmatter::markdown::Frontmatter;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

/// The payload of the first `verb` communication action in `signal`'s stack,
/// as the parser kept it for event-time evaluation.
fn communication_expr(
    lifecycle: &crate::composition::lifecycle::LifecycleConfig,
    signal: crate::composition::lifecycle::LifecycleSignal,
    verb: &str,
) -> Option<darkmatter::markdown::compose::expression::Expr> {
    use crate::composition::lifecycle_actions::{LifecycleActionKind, for_each_action};
    let mut found = None;
    for_each_action(lifecycle.stack(signal), &mut |action| {
        if let LifecycleActionKind::Communication(comm) = &action.kind
            && comm.channel.verb() == verb
            && found.is_none()
        {
            found = Some(comm.text.clone());
        }
    });
    found
}

/// [`communication_expr`] for a payload kept as literal text (a template
/// with surrounding text is interpolated at event time, never at prepare).
fn communication_text(
    lifecycle: &crate::composition::lifecycle::LifecycleConfig,
    signal: crate::composition::lifecycle::LifecycleSignal,
    verb: &str,
) -> Option<String> {
    use darkmatter::markdown::compose::expression::Expr;
    communication_expr(lifecycle, signal, verb).map(|expr| match expr {
        Expr::StringLiteral(text) => text,
        other => panic!("expected the raw text of `{verb}`, got {other:?}"),
    })
}

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
fn file_links_prompts_export_both_listings_without_render_metadata() {
    let dir = TempDir::new().unwrap();
    for (folder, file) in [("modes", "mode-1004.md"), ("osc", "osc-8.md")] {
        fs::create_dir(dir.path().join(folder)).unwrap();
        fs::write(dir.path().join(folder).join(file), "# Standard\n").unwrap();
    }
    let body = "# Resources\n\n### Modes Standards\n\n::file-links ./modes/*.md\n\n### OSC Standards\n\n::file-links ./osc/*.md\n\n## After\n\nKeep **this** text.\n";
    let direct = make_source(&dir, &[], body);
    let inline = make_source(&dir, &[("prompt", json!(body))], "Edit this document.");
    let context = crate::test_support::context_at(dir.path());
    let prompts = [
        prepare_direct(&direct, PrepareOptions::new(context.clone())).unwrap().prompt,
        prepare_inline(&inline, PrepareOptions::new(context)).unwrap().prompt,
    ];
    for prompt in prompts {
        assert!(!prompt.contains("bt:render-tree"), "internal metadata leaked into prompt");
        assert!(!prompt.contains("::file-links"));
        assert!(prompt.contains("[mode-1004.md](modes/mode-1004.md)"), "{prompt}");
        assert!(prompt.contains("[osc-8.md](osc/osc-8.md)"), "{prompt}");
        assert!(prompt.contains("## After\n\nKeep **this** text."));
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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
fn canonical_preparation_records_each_compose_on_the_request_owner() {
    let dir = TempDir::new().unwrap();
    let direct = make_source(&dir, &[], "direct body");
    let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir.path()).unwrap();

    prepare_direct(
        &direct,
        PrepareOptions {
            invocation_context: Some(invocation.clone()),
            ..PrepareOptions::new(crate::test_support::context())
        },
    )
    .unwrap();

    let inline = make_source(&dir, &[("prompt", json!("inline body"))], "old body");
    prepare_inline(
        &inline,
        PrepareOptions {
            invocation_context: Some(invocation.clone()),
            ..PrepareOptions::new(crate::test_support::context())
        },
    )
    .unwrap();

    let work = invocation.work_snapshot();
    assert_eq!(work.compose_operations, 2);
    assert_eq!(work.ambient_fallbacks, 2);
    assert!(work.prepared_context_consumers.is_empty());
}

#[test]
fn canonical_preparation_observes_populated_snapshot_consumers() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("prepared", json!("{{ ctx.os }}"))],
        "{{ ctx.os }}",
    );
    let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir.path()).unwrap();
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let document_epoch = invocation.begin_document_epoch();
    let context = document_epoch.capture_launch_context(&requirements);

    let prepared = prepare_direct(
        &source,
        PrepareOptions {
            invocation_context: Some(invocation.clone()),
            document_epoch: Some(document_epoch),
            prepared_context: Some(context),
            ..PrepareOptions::new(crate::test_support::context())
        },
    )
    .unwrap();

    assert_eq!(
        prepared.document_epoch.unwrap().work_snapshot(),
        crate::invocation_context::DocumentEpochWork {
            volatile_observations: Default::default(),
            launch_context_constructions: 1,
            launch_context_extensions: 0,
            ambient_fallbacks: 0,
            prepared_context_consumers: std::collections::BTreeMap::from([
                ("body".to_string(), 1),
                ("effective-frontmatter".to_string(), 1),
            ]),
        },
        "direct canonical preparation must be one exact epoch"
    );
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

    let prepared = prepare_inline(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(prepared.mode, CompositionMode::InlineFrontmatterPrompt);
    assert!(prepared.prompt.contains("List three colors"));
    // File-aware header first, the author's prompt, then the guardrails bound
    // to the same native path.
    let header = format!("**Document:** `{}`", source.resolved_path.display());
    assert!(prepared.prompt.starts_with(&header), "{}", prepared.prompt);
    let guardrail = format!(
        "The document you are updating is `{}`.",
        source.resolved_path.display()
    );
    assert!(prepared.prompt.contains(&guardrail), "{}", prepared.prompt);
    assert!(prepared.prompt.contains("Never modify the `prompt`, `hash`, or `last_updated`"));
    assert!(prepared.prompt.contains("summary of what you did"));
    assert!(prepared.prompt.find("List three colors").unwrap() < prepared.prompt.find("IMPORTANT").unwrap());
    assert!(!prepared.prompt.contains("Return the replacement Markdown body"));
    // With no `$schema`, the launch schema is Claudine's trigger payload alone.
    let launch = prepared.launch_schema.as_ref().expect("Claudine's triggers match");
    assert!(launch.effective.simplified.is_none(), "no authored `$schema`");
    assert!(crate::composition::schema::bundle::is_claudine_owned(&launch.effective, "prompt"));

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
            assert_eq!(plan.document_path, source.resolved_path);
            assert!(!plan.original_document_text.is_empty());
            // Simple hash should produce a non-empty `<fm>-<body>` string.
            let flat = plan.original_hash.flat_string();
            assert!(flat.is_some() && !flat.unwrap().is_empty());
        }
        CompositionClosurePlan::Direct => panic!("expected Inline closure plan"),
    }
}

#[test]
fn inline_composition_warns_but_preserves_retired_custom_guardrails() {
    let dir = TempDir::new().unwrap();
    let custom_path = dir.path().join(".claudine/inline-compose.md");
    fs::create_dir_all(custom_path.parent().unwrap()).unwrap();
    let custom = "> Do not edit the source file directly.\n> Return your answer.\n";
    fs::write(&custom_path, custom).unwrap();
    let source = make_source(
        &dir,
        &[("prompt", json!("Write the report"))],
        "Old content",
    );

    let prepared = prepare_inline(
        &source,
        PrepareOptions {
            source_repo_root: Some(dir.path().to_path_buf()),
            ..PrepareOptions::new(crate::test_support::context())
        },
    )
    .expect("retired customization warns instead of blocking");

    assert!(prepared.prompt.contains("Do not edit the source file directly"));
    assert_eq!(fs::read_to_string(&custom_path).unwrap(), custom);
    let warning = prepared
        .warnings
        .iter()
        .find(|warning| warning.stage == "inline_guardrails")
        .expect("migration warning");
    assert!(warning.message.contains(".claudine/inline-compose.md"));
    assert!(warning.message.contains("Migrate"));
}

#[test]
fn inline_composition_missing_prompt() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Content");

    let err = prepare_inline(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    assert!(matches!(err, CompositionError::PromptPropertyMissing));
}

#[test]
fn inline_composition_prefers_a_caller_supplied_prompt_and_keeps_the_file_untouched() {
    // AC13: the caller's `prompt` overlay wins before the first interpolation
    // pass and is delivered verbatim; the authored file is not rewritten.
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("prompt", json!("authored {{ title }}")), ("title", json!("T"))], "Old");
    let before = fs::read_to_string(&source.resolved_path).unwrap();

    let prepared = prepare_inline(
        &source,
        PrepareOptions {
            set_overrides: Some(json!({ "prompt": "caller {{ title }}" })),
            ..PrepareOptions::new(crate::test_support::context())
        },
    )
    .unwrap();

    assert!(prepared.prompt.contains("caller T"), "{}", prepared.prompt);
    assert!(!prepared.prompt.contains("authored"), "{}", prepared.prompt);
    assert_eq!(prepared.effective_frontmatter["prompt"], json!("caller T"));
    assert_eq!(fs::read_to_string(&source.resolved_path).unwrap(), before);
}

#[test]
fn inline_prompt_table_marks_only_required_properties_required_at_completion() {
    // The table the agent actually receives, through the normal preparation
    // path: `eager` is a launch-timing constraint, so an eager-only property
    // stays optional at completion.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            (
                "$schema",
                json!({
                    "spec": "string(eager)",
                    "plan": "string(required; eager)",
                    "summary": "string(required)",
                    "notes": "string",
                }),
            ),
            ("prompt", json!("Do the work")),
            ("plan", json!("the plan")),
        ],
        "Old content",
    );

    let prepared = prepare_inline(&source, PrepareOptions::new(crate::test_support::context())).unwrap();

    assert!(
        prepared.prompt.contains("**Schema properties:**"),
        "{}",
        prepared.prompt
    );
    for row in [
        "| `spec` | `string(eager)` | absent | optional |",
        "| `plan` | `string(required;eager)` | present | required |",
        "| `summary` | `string(required)` | absent | required |",
        "| `notes` | `string` | absent | optional |",
    ] {
        assert!(prepared.prompt.contains(row), "{row}\n{}", prepared.prompt);
    }
}

#[test]
fn inline_composition_missing_prompt_is_satisfied_by_an_override() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Content");

    let prepared = prepare_inline(
        &source,
        PrepareOptions {
            set_overrides: Some(json!({ "prompt": "from the caller" })),
            ..PrepareOptions::new(crate::test_support::context())
        },
    )
    .unwrap();
    assert!(prepared.prompt.contains("from the caller"));
}

#[test]
fn inline_composition_wrong_type() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("prompt", json!(42))], "Content");

    let err = prepare_inline(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert!(!prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Start).is_empty());
    assert!(!prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Success).is_empty());
    assert!(prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Blocked).is_empty());
    assert!(prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Failure).is_empty());
}

/// An included document's lifecycle keys never reach the parent's stacks: the
/// parent's `start` keeps its own actions, in order, at its own paths.
#[test]
fn a_transcluded_childs_start_stack_never_replaces_the_parents() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("child.md"), "---\nstart:\n  - stderr: child\n---\nCHILD body\n").unwrap();
    let source = make_source(
        &dir,
        &[("start", json!([{ "stderr": "parent first" }, { "stderr": "parent second" }]))],
        "Parent.\n\n::file ./child.md\n",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context_for(&source.resolved_path))).unwrap();

    assert!(prepared.prompt.contains("CHILD body"), "{}", prepared.prompt);
    let mut actions = Vec::new();
    crate::composition::lifecycle_actions::for_each_action(
        prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Start),
        &mut |action| {
            let crate::composition::lifecycle_actions::LifecycleActionKind::Communication(comm) = &action.kind else {
                panic!("only `stderr` actions: {:?}", action.kind);
            };
            let darkmatter::markdown::compose::expression::Expr::StringLiteral(text) = &comm.text else {
                panic!("literal text: {:?}", comm.text);
            };
            actions.push((action.path.to_string(), text.clone()));
        },
    );
    assert_eq!(
        actions,
        [("start[0].stderr".to_string(), "parent first".to_string()), ("start[1].stderr".to_string(), "parent second".to_string())]
    );
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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

    let prepared = prepare_inline(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert!(!prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Failure).is_empty());
    assert!(prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Start).is_empty());
}

#[test]
fn invalid_lifecycle_config_fails_preparation() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"say": "Hello", "speak": "Also hello"})),
        ],
        "Content",
    );

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    assert!(matches!(err, CompositionError::LifecycleSayConflict(_)));
}

#[test]
fn lifecycle_malformed_mixed_span_is_rejected_at_prepare() {
    // C1 defers lifecycle communication strings from compose-time
    // resolution, but not from syntax checking: a span that does not parse is
    // an authoring error, reported at prepare with the operand's path.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"message": "leak {{ parent_dir(review)) }}"})),
        ],
        "Content",
    );

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    let CompositionError::LifecycleActionInvalidLongForm { message, .. } = &err else {
        panic!("expected LifecycleActionInvalidLongForm, got {err:?}");
    };
    assert!(message.contains("`start.message.text`"), "{message}");
}

#[test]
fn undefined_lifecycle_variable_is_deferred_not_rejected_at_prepare() {
    // A bare `{{ missing }}` in a lifecycle string is deferred to event time
    // (C2), where an absent property is `null`. Prepare keeps the raw span.
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    let message = communication_text(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Start, "message")
        .expect("message action");
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    let message = communication_text(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Start, "message")
        .expect("message action");
    assert_eq!(message, "working on {{ area }}");
}

#[test]
fn lifecycle_fallback_span_is_deferred_unresolved() {
    // Fallback (`{{ x || 'y' }}`) is deferred like any other lifecycle
    // span: a whole-value span is parsed, never resolved, at prepare and
    // resolves at event-time.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[(
            "start",
            json!({"message": "{{ missing_lifecycle_var || 'default' }}"}),
        )],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    use darkmatter::markdown::compose::expression::Expr;
    let message = communication_expr(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Start, "message")
        .expect("message action");
    assert_eq!(
        message,
        Expr::Fallback {
            primary: Box::new(Expr::Variable("missing_lifecycle_var".to_string())),
            fallback: Box::new(Expr::StringLiteral("default".to_string())),
        }
    );
}

#[test]
fn lifecycle_ctx_interpolation_is_deferred_unresolved() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"message": "{{ ctx.today }}"})),
        ],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    use darkmatter::markdown::compose::expression::Expr;
    let message = communication_expr(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Start, "message")
        .expect("message action");
    // Deferred: the whole-value span is the unresolved expression, evaluated
    // at event time (C2).
    assert_eq!(message, Expr::Variable("ctx.today".to_string()));
}

#[test]
fn multiple_lifecycle_spans_all_deferred_raw() {
    // Every lifecycle communication string is deferred; none trips a
    // prepare-time leak, even a span that can only fail when evaluated. They
    // all resolve at event-time.
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("title", json!("Test")),
            ("start", json!({"message": "leak {{ parent_dir(review) }}"})),
            ("failure", json!({"say": "leak {{ no_such_fn(1) }}"})),
        ],
        "Content",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(
        communication_text(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Start, "message").as_deref(),
        Some("leak {{ parent_dir(review) }}")
    );
    assert_eq!(
        communication_text(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Failure, "say").as_deref(),
        Some("leak {{ no_such_fn(1) }}")
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
        ..PrepareOptions::new(crate::test_support::context())
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
        ..PrepareOptions::new(crate::test_support::context())
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
        ..PrepareOptions::new(crate::test_support::context())
    };

    let prepared = prepare_direct(&source, options).unwrap();
    assert_eq!(
        communication_text(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Start, "message")
            .as_deref(),
        Some("{{ctx.agent}}/{{ctx.model}}")
    );
}

#[test]
fn direct_composition_perf_disabled_yields_none() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Simple content.");

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert!(prepared.compose_perf.is_none());
}

#[test]
fn direct_composition_perf_enabled_yields_some() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Simple content.");

    let options = PrepareOptions {
        perf_enabled: true,
        ..PrepareOptions::new(crate::test_support::context())
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
        ..PrepareOptions::new(crate::test_support::context())
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(
        prepared.selection_hints.agent,
        Some(AgentHint::List(vec![Provider::Gemini, Provider::Codex]))
    );
}

#[test]
fn direct_composition_parses_model_single() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!("gpt-4o"))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(
        prepared.selection_hints.model,
        Some(ModelHint::Single("gpt-4o".to_string()))
    );
}

#[test]
fn direct_composition_parses_model_list() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!(["gpt-4o", "o3-mini"]))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(prepared.selection_hints.agent, None);
    assert!(!prepared.selection_hints.agent_was_list);
}

#[test]
fn direct_composition_agent_wrong_type_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!(42))], "Content");

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    assert!(matches!(err, CompositionError::AgentHintWrongType(_)));
}

#[test]
fn direct_composition_model_wrong_type_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!(42))], "Content");

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    assert!(matches!(err, CompositionError::ModelHintWrongType(_)));
}

#[test]
fn direct_composition_agent_list_with_non_string_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("agent", json!(["claude", 42]))], "Content");

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    assert!(matches!(err, CompositionError::AgentHintWrongType(_)));
}

#[test]
fn direct_composition_model_list_with_non_string_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("model", json!(["gpt-4o", 42]))], "Content");

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    assert!(matches!(err, CompositionError::ModelHintWrongType(_)));
}

#[test]
fn direct_composition_no_agent_or_model_hints() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(prepared.selection_hints.agent, None);
    assert_eq!(prepared.selection_hints.model, None);
}

#[test]
fn direct_composition_empty_body_returns_composed_body_empty() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("Test"))], "   \n\n  \t\n");

    let options = PrepareOptions {
        set_overrides: Some(json!({"spec": "plan.md", "phase": 1})),
        ..PrepareOptions::new(crate::test_support::context())
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
        ..PrepareOptions::new(crate::test_support::context())
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
    let Some(probe_executable) = crate::test_support::cwd_probe() else {
        eprintln!("skipping cwd probe test because rustc is unavailable");
        return;
    };
    let source_dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();
    let executable = biscuit_file::to_portable_string(&probe_executable);
    let command = format!("\"{executable}\"");
    let source = make_source(
        &source_dir,
        &[("title", json!("T"))],
        &format!("::shell {command}\n"),
    );

    let mut approved = std::collections::HashSet::new();
    approved.insert(executable);
    let options = PrepareOptions {
        pre_approved_commands: Some(approved.into()),
        shell_working_directory: Some(work_dir.path().to_path_buf()),
        ..PrepareOptions::new(crate::test_support::context())
    };

    let prepared = prepare_direct(&source, options).unwrap();
    let work_leaf = work_dir.path().file_name().unwrap().to_string_lossy();
    let source_leaf = source_dir.path().file_name().unwrap().to_string_lossy();
    assert!(
        prepared.prompt.contains(work_leaf.as_ref()),
        "expected shell cwd to contain {work_leaf}; got: {}",
        prepared.prompt
    );
    assert!(
        !prepared.prompt.contains(source_leaf.as_ref()),
        "shell must not run next to the prompt source; got: {}",
        prepared.prompt
    );
}

#[test]
fn direct_composition_parses_interactive_hint() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("interactive", json!(true))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(prepared.selection_hints.interactive, Some(true));
}

#[test]
fn direct_composition_interactive_null_is_absent() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("interactive", json!(null))], "Content");

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(prepared.selection_hints.interactive, None);
}

#[test]
fn direct_composition_interactive_wrong_type_errors() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("interactive", json!("yes"))], "Content");

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
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

    let prepared = prepare_inline(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();

    let fm = prepared.effective_frontmatter.as_object().unwrap();
    let failure = fm.get("failure").unwrap();
    assert_eq!(
        failure.get("message").unwrap(),
        &json!("❌️ {{err.msg}}"),
        "deferred lifecycle subtree retains its raw span"
    );
    assert_eq!(
        communication_text(&prepared.lifecycle, crate::composition::lifecycle::LifecycleSignal::Failure, "message").as_deref(),
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
    let mut command = None;
    crate::composition::lifecycle_actions::for_each_action(lifecycle.stack(signal), &mut |action| {
        if let LifecycleActionKind::Shell(shell) = &action.kind {
            command.get_or_insert_with(|| shell.command.clone());
        }
    });
    command.expect("a shell action in the stack")
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
                json!([{ "shell": "git fetch {{branch}}" }]),
            ),
        ],
        "Do the work.",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
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
            json!([{ "shell": "rm {{err.msg}}" }]),
        )],
        "Do the work.",
    );

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    match err {
        CompositionError::LifecycleShellResolution {
            property,
            raw,
            message,
            ..
        } => {
            assert_eq!(property, "failure[0].shell.command");
            assert_eq!(raw, "rm {{err.msg}}");
            assert!(
                message.contains("err"),
                "message names the late-binding root: {message}"
            );
        }
        other => panic!("expected LifecycleShellResolution, got: {other:?}"),
    }
}

/// The lazy reserved root `current` is late-binding for the shell gate too.
///
/// A lifecycle `shell` command is approved at preflight against early-binding
/// surfaces only, so a command naming `current.*` would be approved as one
/// string and executed observing another. The rejection is what keeps the
/// approved bytes and the executed bytes identical (R33).
#[test]
fn the_lazy_current_root_in_a_lifecycle_shell_command_is_rejected_at_prepare() {
    let dir = TempDir::new().unwrap();
    let command = "rm {{current.branch}}".to_string();
    let source = make_source(
        &dir,
        &[("failure", json!([{ "shell": command.clone() }]))],
        "Do the work.",
    );

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context()))
        .expect_err("a lazy reserved root cannot be approved at preflight");
    match err {
        CompositionError::LifecycleShellResolution { raw, message, .. } => {
            assert_eq!(raw, command);
            assert!(message.contains("current"), "message names the rejected root: {message}");
        }
        other => panic!("expected LifecycleShellResolution, got: {other:?}"),
    }
}

/// `current_env` is an ordinary frontmatter property, so lifecycle shell
/// approval reads the document's value and stamps it into the approved bytes
/// instead of refusing it as a late root.
#[test]
fn current_env_in_a_lifecycle_shell_command_reads_the_frontmatter() {
    use darkmatter::markdown::compose::expression::Expr;

    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("current_env", json!({"HOME": "userland"})),
            ("start", json!([{ "shell": "echo {{ current_env.HOME }}" }])),
        ],
        "Do the work.",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context()))
        .expect("an ordinary property is approvable");
    assert_eq!(
        first_shell_command(&prepared.lifecycle, crate::composition::LifecycleSignal::Start),
        Expr::StringLiteral("echo userland".to_string())
    );
}

/// Lifecycle shell approval — what a sequence's referenced prompt goes through
/// at its turn — refuses `group` like every late global, even in a branch that
/// never runs, and carries Darkmatter's typed cause. `fm.group` reads the
/// document, and its resolved bytes are what was stamped for execution.
#[test]
fn lifecycle_shell_approval_refuses_group_in_any_branch_while_doc_group_reads_the_document() {
    use darkmatter::markdown::compose::expression::{BindingError, ExpressionError, Expr};

    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("group", json!({"label": "document"})),
            (
                "start",
                json!([{ "shell": "echo {{ false ? group.label : 'x' }}" }]),
            ),
        ],
        "Do the work.",
    );
    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    let CompositionError::LifecycleShellResolution { message, source: Some(cause), .. } = &err else {
        panic!("expected a typed LifecycleShellResolution, got: {err:?}");
    };
    assert!(message.contains("`group`"), "{message}");
    let darkmatter::markdown::MarkdownError::Interpolation { cause, .. } = cause.as_ref() else {
        panic!("expected an interpolation cause, got {cause:?}");
    };
    let ExpressionError::Binding(binding) = cause.as_ref() else {
        panic!("expected a binding cause, got {cause:?}");
    };
    let BindingError::Unavailable(read) = binding.as_ref() else {
        panic!("expected an unavailable read, got {binding:?}");
    };
    assert_eq!(read.reason.code(), "claudine.preflight-unavailable");

    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[
            ("group", json!({"label": "document"})),
            ("start", json!([{ "shell": "echo {{ fm.group.label }}" }])),
        ],
        "Do the work.",
    );
    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(
        first_shell_command(&prepared.lifecycle, crate::composition::LifecycleSignal::Start),
        Expr::StringLiteral("echo document".to_string())
    );
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
            json!([{ "shell": { "command": "rm {{err.msg}}" } }]),
        )],
        "Do the work.",
    );

    let err = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap_err();
    match err {
        CompositionError::LifecycleShellResolution { property, raw, .. } => {
            assert_eq!(property, "failure[0].shell.command");
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
        ctx: Box::new(ctx),
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

/// Every composition — not just a sequence step — initializes the `outputs`
/// accumulator, so a document written with `{{ last(seq.outputs) }}` composes
/// identically standalone and mid-sequence.
#[test]
fn direct_composition_initializes_the_outputs_accumulator() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("title", json!("Research"))],
        "# Research\n\nprev={{ last(seq.outputs) }}.",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    let fm = prepared.effective_frontmatter.as_object().unwrap();
    assert_eq!(fm.get("outputs"), None);
    assert!(
        prepared.prompt.contains("prev=."),
        "an empty accumulator renders as nothing, not as a raw span: {:?}",
        prepared.prompt
    );
}

/// The caller's runtime layers survive into the composed body: entries handed
/// through `set_overrides` are what `last(seq.outputs)` reads.
#[test]
fn prior_outputs_are_visible_to_the_composed_body() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[], "prev={{ last(seq.outputs) }}");

    let runtime = crate::composition::RuntimeState::new();
    runtime.append_output("step one output\n");
    let prepared = prepare_direct(
        &source,
        PrepareOptions::new(crate::test_support::context()).with_layered_overrides(crate::composition::layered_set_overrides(
            crate::composition::LayeredOverrides::new(),
            Some(&runtime.snapshot()),
            None,
        )),
    )
    .unwrap();

    assert!(
        prepared.prompt.contains("prev=step one output"),
        "{:?}",
        prepared.prompt
    );
    assert_eq!(
        prepared.effective_frontmatter.as_object().unwrap().get("outputs"),
        None
    );
}

/// A user setter cannot displace the accumulator.
#[test]
fn a_user_setter_cannot_replace_outputs() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[], "body");

    let prepared = prepare_direct(
        &source,
        PrepareOptions::new(crate::test_support::context()).with_layered_overrides(crate::composition::layered_set_overrides(
            crate::composition::LayeredOverrides::authored(Some(&json!({"outputs": ["hijacked"]}))),
            Some(&crate::composition::RuntimeState::new().snapshot()),
            None,
        )),
    )
    .unwrap();

    assert_eq!(
        prepared.effective_frontmatter.as_object().unwrap().get("outputs"),
        Some(&json!(["hijacked"]))
    );
}

/// An author who declares `outputs:` in their own frontmatter still gets the
/// executor's accumulator — the reserved key is never author-owned.
#[test]
fn authored_outputs_frontmatter_is_preserved_beside_the_accumulator() {
    let dir = TempDir::new().unwrap();
    let source = make_source(
        &dir,
        &[("outputs", json!(["authored"]))],
        "prev={{ last(seq.outputs) }}",
    );

    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert_eq!(
        prepared.effective_frontmatter.as_object().unwrap().get("outputs"),
        Some(&json!(["authored"]))
    );
}

#[test]
fn sequence_lifecycle_shell_snapshot_survives_later_preparation() {
    use crate::composition::lifecycle_actions::{LifecycleActionKind, for_each_action};
    use darkmatter::markdown::compose::shell_expansion::template::{DeferredShellRoots, DocumentShellValues, fill_template};
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[
        ("outputs", json!(["barrier value"])),
        ("start", json!([{"shell": "tool {{ last(seq.outputs) }} {{ label }}"},
            {"set": {"captured": "$(tool {{ last(seq.outputs) }} {{ label }})"}}])),
        ("label", json!("early")),
    ], "Prompt.");
    let mut options = PrepareOptions::new(crate::test_support::context_for(&source.resolved_path));
    options.deferred_shell_roots = DeferredShellRoots::new(["seq"]).with_placeholders_template_required();
    options.document_shell_values = Some(DocumentShellValues::default());
    options.sequence_binding = Some(json!({"outputs": ["barrier value"]}));
    validate_document_shell_late_values(&source, &options).unwrap();
    options.sequence_binding = Some(json!({"outputs": ["changed late"]}));
    options.set_overrides = Some(json!({"label": "current value"}));
    let prepared = prepare_sequence_probe(&source, options, CompositionMode::ChainedDocument).unwrap();
    let mut checked = 0;
    for_each_action(prepared.lifecycle.stack(crate::composition::lifecycle::LifecycleSignal::Start), &mut |action| {
        let sites = match &action.kind {
            LifecycleActionKind::Shell(shell) => vec![shell.template.as_ref().unwrap()],
            LifecycleActionKind::RuntimeSet(set) => set.shell_values().map(|(_, shell)| shell.template.as_ref().unwrap()).collect(),
            _ => Vec::new(),
        };
        for site in sites {
            let evaluate = |expression: &str| -> Result<serde_json::Value, darkmatter::markdown::compose::expression::ExpressionError> {
                Ok(if expression == "label" { json!("current value") } else { json!("changed late") })
            };
            let chain = fill_template(site.template(), &evaluate, site.late_values()).unwrap();
            assert_eq!(chain.segments[0].args, vec!["barrier value", "current value"]);
            checked += 1;
        }
    });
    assert_eq!(checked, 2, "shell and whole-value set both retain the barrier");
}

#[test]
fn sequence_lifecycle_requires_preapproved_shape_membership() {
    use darkmatter::markdown::compose::shell_expansion::template::{DeferredShellRoots, ShellPreApprovals};
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("out", json!("known")),
        ("start", json!([{"shell": "tool {{ out }}"}]))], "Prompt.");
    let mut options = PrepareOptions::new(crate::test_support::context_for(&source.resolved_path));
    options.deferred_shell_roots = DeferredShellRoots::new(["seq"]).with_placeholders_template_required();
    let mut approvals = ShellPreApprovals::new();
    approvals.insert_exact("tool known");
    options.pre_approved_commands = Some(approvals);
    let error = prepare_sequence_probe(&source, options, CompositionMode::ChainedDocument).unwrap_err();
    assert!(error.to_string().contains("not approved at sequence preflight"), "{error}");
}


fn assert_sequence_lifecycle_byte_grammar(event: &str, long: bool) {
    use crate::composition::lifecycle::{LifecycleSignal, actions::{LifecycleActionKind, for_each_action}};
    use darkmatter::markdown::compose::expression::Expr;
    use darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots;
    let dir = TempDir::new().unwrap();
    for (command, expected) in [("{{ tool }} probe", "tool probe"),
        ("tool {{ out }}; tool tail", "tool dist; tool tail")] {
        let action = if long { json!({"shell": {"command": command}}) } else { json!({"shell": command}) };
        let stack = if event == "loop" { json!({"until": "loop.count >= 1", "gate": [action]}) } else { json!([action]) };
        let source = make_source(&dir, &[("tool", json!("tool")), ("out", json!("dist")), (event, stack)], "Prompt.");
        let mut options = PrepareOptions::new(crate::test_support::context_for(&source.resolved_path));
        options.deferred_shell_roots = DeferredShellRoots::new(["seq", "loop", "group"]);
        let audited = validate_document_shell_late_values(&source, &options).unwrap();
        let prepared = prepare_sequence_probe(&source, options.clone(), CompositionMode::ChainedDocument).unwrap();
        for lifecycle in [&audited, &prepared.lifecycle] {
            let mut commands = Vec::new();
            for signal in LifecycleSignal::ALL {
                for_each_action(lifecycle.stack(signal), &mut |action| {
                    if let LifecycleActionKind::Shell(shell) = &action.kind {
                        assert!(shell.template.is_none(), "known bytes have no valid template shape");
                        let Expr::StringLiteral(bytes) = &shell.command else { panic!("resolved bytes") };
                        commands.push(bytes.clone());
                    }
                });
            }
            assert_eq!(commands, vec![expected.to_string()], "audit and JIT preserve the exact bytes");
        }
        options.deferred_shell_roots = options.deferred_shell_roots.with_placeholders_template_required();
        let error = validate_document_shell_late_values(&source, &options).unwrap_err();
        assert!(error.to_string().contains("invalid shell template grammar"), "{error}");
    }
}

macro_rules! sequence_lifecycle_byte_grammar_case {
    ($name:ident, $event:literal, $long:literal) => {
        #[test] fn $name() { assert_sequence_lifecycle_byte_grammar($event, $long); }
    };
}
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_start_short, "start", false);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_start_long, "start", true);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_success_short, "success", false);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_success_long, "success", true);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_finalize_short, "finalize", false);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_finalize_long, "finalize", true);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_failure_short, "failure", false);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_failure_long, "failure", true);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_blocked_short, "blocked", false);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_blocked_long, "blocked", true);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_loop_short, "loop", false);
sequence_lifecycle_byte_grammar_case!(sequence_lifecycle_byte_grammar_loop_long, "loop", true);

#[test]
fn sequence_lifecycle_byte_grammar_keeps_late_and_forbidden_reads() {
    use darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots;
    let dir = TempDir::new().unwrap();
    for command in ["{{ last(seq.outputs) }} probe", "tool {{ last(seq.outputs) }}; tool tail",
        "{{ derived }} probe", "{{ tool }} {{ last(seq.outputs) }} {{ err.msg }}",
        "{{ tool }} {{ current.branch }}"] {
        let source = make_source(&dir, &[("tool", json!("tool")),
            ("derived", json!("{{ last(seq.outputs) }}")),
            ("start", json!([{"shell": command}]))], "Prompt.");
        let mut options = PrepareOptions::new(crate::test_support::context_for(&source.resolved_path));
        options.deferred_shell_roots = DeferredShellRoots::new(["seq", "loop", "group"]);
        let error = validate_document_shell_late_values(&source, &options).unwrap_err();
        let text = error.to_string();
        if command.contains("err.msg") || command.contains("current.branch") {
            assert!(!text.contains("invalid shell template grammar"), "forbidden reads retain their earlier typed diagnosis: {error}");
            assert!(text.contains(if command.contains("err.msg") { "err" } else { "current" }), "{error}");
        } else { assert!(text.contains("invalid shell template grammar"), "{error}"); }
    }
}

#[test]
fn sequence_binding_composes_typed_rows_shorthand_and_authored_namespaces() {
    use crate::composition::lifecycle::bindings::SequenceBinding;
    use crate::composition::sequence::{build_step_overlay, resolve_sequence_plan};
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[
        ("sequence", json!([{"name":"Bob","age":32},{"name":"Sally","age":36},{"name":"Bob"}])),
        ("seq", json!({"name":"authored"})),
        ("row", json!("{{ seq }}")),
    ], "Who {{ seq }} age {{ seq.age }} next {{ seq.next }}; fm={{ fm.seq.name }}.");
    let plan = resolve_sequence_plan(&source, &crate::test_support::context_for(&source.resolved_path)).unwrap().unwrap();
    for (index, expected) in [(0, "Who Bob age 32 next Sally; fm=authored."),
        (1, "Who Sally age 36 next Bob; fm=authored."), (2, "Who Bob age  next ; fm=authored.")] {
        let mut options = PrepareOptions::new(crate::test_support::context_for(&source.resolved_path));
        options.sequence_binding = Some(SequenceBinding::step(&build_step_overlay(&plan, index), vec![json!("prior")]).to_value());
        let prepared = prepare_direct(&source, options).unwrap();
        assert_eq!(prepared.prompt, expected);
        let fm = &prepared.effective_frontmatter;
        assert_eq!(fm["seq"], json!({"name":"authored"}));
        assert_eq!(fm["row"]["name"], plan.steps[index].name);
        assert_eq!(fm["row"]["index"], json!(index + 1));
        assert_eq!(fm["row"]["outputs"], json!(["prior"]));
        assert!(fm["row"]["previous"].get("outputs").is_none());
        assert!(fm["row"]["next"].get("previous").is_none());
        for retired in ["state", "previous", "next", "outputs", "sequence_id", "step", "total_steps", "is_first", "is_last"] {
            assert!(fm.get(retired).is_none(), "generated property {retired} escaped into frontmatter");
        }
    }
}

#[test]
fn rename_presence_controls_apply_through_normal_preparation() {
    for (property, value) in [("previous", json!("mine")), ("previous", serde_json::Value::Null),
        ("outputs", json!(["mine"])), ("state", json!({"name":"mine"}))] {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[], &format!("value={{{{ {property} }}}}"));
        let error = prepare_direct(&source, PrepareOptions::new(crate::test_support::context_for(&source.resolved_path))).unwrap_err();
        assert!(matches!(error, CompositionError::RemovedGrammar { .. }), "{error}");
        let mut options = PrepareOptions::new(crate::test_support::context_for(&source.resolved_path));
        options.set_overrides = Some(json!({property: value}));
        let prepared = prepare_direct(&source, options).unwrap();
        assert_eq!(prepared.effective_frontmatter[property], value);
        assert!(!prepared.prompt.contains("{{"));
    }
}

#[test]
fn transcluded_rename_reads_respect_the_reading_documents_property_presence() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[], "::file ./child.md\n");
    for (child, wanted) in [
        ("---\nprevious: mine\nlabel: '{{ previous }}'\n---\nChild {{ label }} / {{ previous }}.\n", "Child mine / mine."),
        ("---\nprevious: null\n---\nChild {{ previous }}.\n", "Child ."),
    ] {
        fs::write(dir.path().join("child.md"), child).unwrap();
        let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context_for(&source.resolved_path))).unwrap();
        assert!(prepared.prompt.contains(wanted), "{}", prepared.prompt);
    }
    fs::write(dir.path().join("child.md"), "Child {{ previous }}.\n").unwrap();
    let error = prepare_direct(&source, PrepareOptions::new(crate::test_support::context_for(&source.resolved_path))).unwrap_err();
    assert!(matches!(error, CompositionError::RemovedGrammar { .. }), "{error}");
}

#[test]
fn rename_validation_uses_active_authored_overrides_and_never_scans_runtime_data() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("title", json!("{{ state.name }}"))], "Value {{ title }}.");
    let context = crate::test_support::context_for(&source.resolved_path);
    let mut options = PrepareOptions::new(context.clone());
    options.set_overrides = Some(json!({"title":"safe"}));
    assert_eq!(prepare_direct(&source, options).unwrap().prompt, "Value safe.");

    let data = crate::composition::LayeredOverrides::data(Some(&json!({"title":"{{ state.name }}"})));
    let options = PrepareOptions::new(context.clone()).with_layered_overrides(data);
    assert_eq!(prepare_direct(&source, options).unwrap().prompt, "Value {{ state.name }}.");

    let mut options = PrepareOptions::new(context);
    options.set_overrides = Some(json!({"title":"{{ false ? state.name : 'safe' }}"}));
    let error = prepare_direct(&source, options).unwrap_err();
    assert!(matches!(error, CompositionError::RemovedGrammar { ref removed, .. } if removed == "state.name"), "{error}");
}

#[test]
fn direct_composition_parses_yolo_hint() {
    let dir = TempDir::new().unwrap();
    let source = make_source(&dir, &[("yolo", json!(true))], "Content");
    let prepared = prepare_direct(&source, PrepareOptions::new(crate::test_support::context())).unwrap();
    assert!(prepared.selection_hints.yolo_enabled(false));
}

/// A `with:` overlay's file value resolves from the file that authored it —
/// not the target's directory and not the launch directory — whether or not
/// the caller supplied setters of its own, and a caller setter still outranks
/// it, value and origin alike.
#[test]
fn an_anchored_overlay_file_value_resolves_from_its_authoring_file() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    for (folder, marker) in [("router", "authoring"), ("target", "target"), ("", "launch")] {
        let folder = root.join(folder);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("notes.md"), format!("---\nmarker: {marker}\n---\n")).unwrap();
    }
    let target = root.join("target/target.md");
    fs::write(&target, "---\n$schema:\n  notes: 'file(required)'\n  choice: string\nmarker: \"{{ frontmatter(notes, 'marker') }}\"\n---\nROUTE={{ marker }}\n").unwrap();
    let launch = crate::test_support::context_at(&root);
    let router = launch.for_source(root.join("router/router.md"));
    let values: indexmap::IndexMap<String, serde_json::Value> =
        [("notes".to_string(), json!("./notes.md"))].into_iter().collect();

    let prepare = |setters: Option<serde_json::Value>| {
        let mut source = crate::composition::resolve_fixture_source(target.to_str().unwrap()).unwrap();
        for (key, value) in &values {
            source.markdown.frontmatter_mut().as_map_mut().insert(key.clone(), value.clone());
        }
        let records = crate::composition::CallerInputLayers::from_caller_overrides(setters.clone(), launch.clone()).caller_input_records;
        let options = PrepareOptions {
            proxy_overlay: crate::composition::AnchoredOverlay::new(values.clone(), &router),
            set_overrides: setters,
            caller_input_records: records,
            ..PrepareOptions::new(launch.for_source(&target))
        };
        prepare_direct(&source, options).unwrap().prompt
    };

    assert!(prepare(None).contains("ROUTE=authoring"), "{}", prepare(None));
    let unrelated = prepare(Some(json!({ "choice": "caller" })));
    assert!(unrelated.contains("ROUTE=authoring"), "an unrelated caller setter must not move the overlay's origin: {unrelated}");
    let pinned = prepare(Some(json!({ "notes": "notes.md" })));
    assert!(pinned.contains("ROUTE=launch"), "the caller's setter outranks `with`, from the launch directory: {pinned}");
}

/// Layering keeps each key's origin with its value: a retry's value replaces
/// the proxy's together with its origin, and a `null` removal carries none.
#[test]
fn anchored_overlays_layer_value_and_origin_together() {
    let dir = TempDir::new().unwrap();
    let source_origin = crate::test_support::context_at(dir.path()).for_source(dir.path().join("source/source.md"));
    let retry_origin = crate::test_support::context_at(dir.path()).for_source(dir.path().join("target/target.md"));
    let proxy = crate::composition::AnchoredOverlay::new(
        [("kept", json!("./a.md")), ("replaced", json!("./b.md")), ("removed", json!("./c.md"))]
            .into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        &source_origin,
    );
    let retry = crate::composition::AnchoredOverlay::new(
        [("replaced", json!("./d.md")), ("removed", serde_json::Value::Null)]
            .into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        &retry_origin,
    );

    let layered = proxy.layered(&retry);
    let records = layered.input_records();

    assert_eq!(layered.values()["replaced"], json!("./d.md"));
    assert_eq!(layered.values()["removed"], serde_json::Value::Null);
    assert_eq!(records["kept"].origin(), &source_origin);
    assert_eq!(records["replaced"].origin(), &retry_origin);
    assert_eq!(records["replaced"].raw(), &json!("./d.md"));
    assert!(!records.contains_key("removed"), "a removal has no value to anchor");
    assert_eq!(format!("{layered:?}"), r#"["kept", "replaced", "removed"]"#, "debug output names keys, never values");
}

// -- inline baseline preflight ----------------------------------------------

/// Write `text` verbatim (so its frontmatter layout survives) and resolve it.
fn raw_source(dir: &TempDir, text: &str) -> ResolvedCompositionSource {
    let file = dir.path().join("inline.md");
    fs::write(&file, text).unwrap();
    ResolvedCompositionSource {
        original_ref: file.to_str().unwrap().to_string(),
        resolved_path: file,
        original_text: text.to_string(),
        markdown: text.to_string().into(),
    }
}

/// A frontmatter `$(…)` that leaves a `ran` directory beside the document when
/// composition runs it; `mkdir` means the same thing to `sh` and `cmd`, and
/// the shell policy refuses output redirection.
const SENTINEL_COMMAND: &str = "$(mkdir ran)";

fn baseline_document(indent: &str) -> String {
    format!(
        "---\n{indent}prompt: Summarize the notes\n{indent}probe: \"{SENTINEL_COMMAND}\"\n---\nOld body\n"
    )
}

fn repo_options(dir: &TempDir) -> PrepareOptions {
    PrepareOptions {
        source_repo_root: Some(dir.path().to_path_buf()),
        // Approved up front so the outcome never depends on the host's shell
        // whitelist.
        pre_approved_commands: Some(
            std::collections::HashSet::from(["mkdir ran".to_string()]).into(),
        ),
        ..PrepareOptions::new(crate::test_support::context_at(dir.path()))
    }
}

/// Every inline preparation path (the launch/preview read, the loop seed, and
/// the sequence probe) refuses an uneditable baseline before it composes:
/// the frontmatter command never runs, no guardrail file is created, and the
/// document keeps its bytes.
#[test]
fn inline_preparation_refuses_an_indented_baseline_before_composing() {
    type Prepare = fn(&ResolvedCompositionSource, PrepareOptions) -> Result<PreparedComposition, CompositionError>;
    let entries: [(&str, Prepare); 3] = [
        ("prepare_inline", prepare_inline),
        ("prepare_loop_seed", |source, options| {
            prepare_loop_seed(source, options, CompositionMode::InlineFrontmatterPrompt)
        }),
        ("prepare_sequence_probe", |source, options| {
            prepare_sequence_probe(source, options, CompositionMode::InlineFrontmatterPrompt)
        }),
    ];
    for (entry, prepare) in entries {
        let dir = TempDir::new().unwrap();
        let document = baseline_document("    ");
        let source = raw_source(&dir, &document);

        let error = prepare(&source, repo_options(&dir)).unwrap_err();

        let CompositionError::InlineBaselineUnstampable { path, source: cause } = &error else {
            panic!("{entry}: expected an unstampable baseline, got {error:?}");
        };
        assert_eq!(path, &source.resolved_path, "{entry}");
        let MarkdownError::FrontmatterTextEdit { reason } = cause.as_ref() else {
            panic!("{entry}: expected a refused frontmatter edit, got {cause:?}");
        };
        assert!(reason.contains("indented by 4 spaces"), "{entry}: {reason}");
        assert_eq!(fs::read_to_string(&source.resolved_path).unwrap(), document, "{entry}");
        assert!(!dir.path().join("ran").is_dir(), "{entry}: the frontmatter command ran");
        assert!(
            !dir.path().join(".claudine/inline-compose.md").exists(),
            "{entry}: the guardrail file was created"
        );
    }
}

/// The control for the refusal above: the same document at column 0 prepares,
/// runs its frontmatter command, and creates the guardrail file, so their
/// absence there is evidence of the early refusal.
#[test]
fn inline_preparation_of_a_column_zero_baseline_composes_and_writes_guardrails() {
    let dir = TempDir::new().unwrap();
    let document = baseline_document("");
    let source = raw_source(&dir, &document);

    let prepared = prepare_inline(&source, repo_options(&dir)).unwrap();

    assert!(prepared.prompt.contains("Summarize the notes"), "{}", prepared.prompt);
    assert!(dir.path().join("ran").is_dir(), "the frontmatter command ran");
    assert!(dir.path().join(".claudine/inline-compose.md").exists());
    assert_eq!(fs::read_to_string(&source.resolved_path).unwrap(), document);
}
