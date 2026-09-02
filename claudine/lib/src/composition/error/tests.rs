//! Tests for composition errors.

use super::*;

fn source_from(text: &str) -> ResolvedCompositionSource {
    ResolvedCompositionSource {
        original_ref: "review.md".to_string(),
        resolved_path: PathBuf::from("review.md"),
        original_text: text.to_string(),
        markdown: text.to_string().into(),
    }
}

#[test]
fn visible_error_paths_use_portable_spelling() {
    let err = CompositionError::SchemaLoad {
        source_path: PathBuf::from(r"C:\repo\prompts\review.md"),
        message: "did not resolve".to_string(),
    };

    assert!(err.to_string().contains("C:/repo/prompts/review.md"));
    assert_eq!(
        err.detail()["source_path"],
        serde_json::json!("C:/repo/prompts/review.md")
    );
}

#[test]
fn file_link_uses_encoded_url_and_portable_label() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("a b#%.md");
    let link = super::render::render_file_link(&path);

    assert!(link.contains("a%20b%23%25.md"), "expected encoded URL: {link}");
    assert!(
        link.contains(&biscuit_file::to_portable_string(&path)),
        "expected portable label: {link}"
    );
}

#[test]
fn file_link_absolutizes_a_missing_relative_path() {
    let link = super::render::render_file_link(Path::new("missing-relative-prompt.md"));

    assert!(link.contains("<a href=\"file://"), "expected file URL: {link}");
    assert!(link.contains("missing-relative-prompt.md"), "expected label: {link}");
}

#[test]
fn enrich_wraps_lifecycle_leak_with_excerpt() {
    let source = source_from(
        "---\nreview_file: x\nsuccess:\n    message: \"at {{review-file}}\"\n---\nbody\n",
    );
    let err = CompositionError::LifecycleInterpolationLeak {
        source_path: PathBuf::from("review.md"),
        property: "success.message".to_string(),
        expression: "review-file".to_string(),
        reason: String::new(),
    }
    .enrich_frontmatter(&source, true);

    assert!(matches!(err, CompositionError::WithFrontmatter { .. }));
    assert!(err.frontmatter_excerpt().is_some());
    // Display still delegates to the inner leak diagnostic.
    assert!(err.to_string().contains("interpolation leaked"), "got: {err}");
}

#[test]
fn enrich_is_noop_for_unrelated_error() {
    let source = source_from("---\ntitle: x\n---\nbody\n");
    let err = CompositionError::NoRunnableProviders.enrich_frontmatter(&source, true);
    assert!(matches!(err, CompositionError::NoRunnableProviders));
}

#[test]
fn already_emitted_wraps_once_and_delegates_display() {
    let err = CompositionError::LifecycleEvaluationError {
        source_path: PathBuf::from("review.md"),
        event: "success".to_string(),
        surface: "when".to_string(),
        message: "boom".to_string(),
    };
    let display = err.to_string();
    let marked = err.already_emitted();
    assert!(marked.is_already_emitted());
    // Display still delegates to the inner evaluation error.
    assert_eq!(marked.to_string(), display);
    // Idempotent: re-marking does not double-wrap.
    let again = marked.already_emitted();
    match &again {
        CompositionError::LifecycleEvaluationAlreadyEmitted { inner } => {
            assert!(
                !inner.is_already_emitted(),
                "must not nest the already-emitted wrapper"
            );
        }
        other => panic!("expected LifecycleEvaluationAlreadyEmitted, got {other:?}"),
    }
}

#[test]
fn enrich_is_idempotent() {
    let source = source_from("---\ntitle: x\n---\nbody\n");
    let err = CompositionError::PromptPropertyMissing
        .enrich_frontmatter(&source, true)
        .enrich_frontmatter(&source, true);
    // Wrapped exactly once — the inner is the bare missing-prompt error.
    match err {
        CompositionError::WithFrontmatter { inner, .. } => {
            assert!(matches!(*inner, CompositionError::PromptPropertyMissing));
        }
        other => panic!("expected WithFrontmatter, got: {other:?}"),
    }
}

#[test]
fn enrich_frontmatter_fence_mismatch_attaches_excerpt() {
    let source = source_from(
        "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n",
    );
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("review.md"),
        PathBuf::from("review.md"),
        source.original_text.clone(),
    );
    let md_err = MarkdownError::FrontmatterFenceMismatch {
        ctx: Box::new(ctx),
        found: "----".to_string(),
        line: 1,
    };
    let err = CompositionError::FrontmatterParse(md_err).enrich_frontmatter(&source, true);

    assert!(
        matches!(err, CompositionError::WithFrontmatter { .. }),
        "expected WithFrontmatter wrapper, got: {err:?}"
    );
    assert!(err.frontmatter_excerpt().is_some(), "expected excerpt attached");
}

#[test]
fn enrich_frontmatter_fence_mismatch_highlights_line_one() {
    let source = source_from(
        "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n",
    );
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("review.md"),
        PathBuf::from("review.md"),
        source.original_text.clone(),
    );
    let md_err = MarkdownError::FrontmatterFenceMismatch {
        ctx: Box::new(ctx),
        found: "----".to_string(),
        line: 1,
    };
    let err = CompositionError::FrontmatterParse(md_err).enrich_frontmatter(&source, true);
    let excerpt = err.frontmatter_excerpt().expect("excerpt attached");
    assert_eq!(
        excerpt.highlight_line(),
        Some(1),
        "line 1 should be highlighted"
    );
}

#[test]
fn enrich_frontmatter_parse_regular_error_gets_block_only_excerpt() {
    let source = source_from("---\nprompt: |-\n    four spaces\n   three spaces\n---\nbody\n");
    let yaml_err: biscuit_file::YamlParseError =
        biscuit_file::serde_yaml_ng::from_str::<biscuit_file::serde_yaml_ng::Value>(
            "prompt: |-\n    four spaces\n   three spaces\n",
        )
        .expect_err("malformed YAML should fail to parse");
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("review.md"),
        PathBuf::from("review.md"),
        source.original_text.clone(),
    );
    let md_err = MarkdownError::FrontmatterParse {
        ctx,
        source: yaml_err,
    };
    let err = CompositionError::FrontmatterParse(md_err).enrich_frontmatter(&source, true);

    assert!(
        matches!(err, CompositionError::WithFrontmatter { .. }),
        "expected WithFrontmatter wrapper, got: {err:?}"
    );
    assert!(err.frontmatter_excerpt().is_some(), "expected excerpt attached");
}

#[test]
fn enrich_frontmatter_interpolation_focuses_on_receiving_key() {
    use darkmatter::markdown::SourceRef;
    use darkmatter::markdown::compose::expression::ExpressionError;

    // A whole-value interpolation failure naming a receiving key must focus
    // the excerpt on that key's line, not dump the whole frontmatter block.
    let source = source_from(
        "---\n$schema:\n    spec: file(match(**/*spec*.md))\niteration: \"{{ frontmatter(spec, 'x') }}\"\n---\nbody\n",
    );
    let md_err = MarkdownError::Interpolation {
        key: Some("iteration".to_string()),
        expression: "frontmatter(spec, 'x')".to_string(),
        source: Box::new(SourceRef::Effective {
            rendered: "frontmatter(spec, 'x')".to_string(),
            origin_key: Some("iteration".to_string()),
        }),
        cause: Box::new(ExpressionError::Parse("boom".to_string())),
    };
    let err = CompositionError::ComposeFailed(md_err);
    assert!(
        matches!(
            err.frontmatter_block_spec(),
            Some(FrontmatterHighlight::Property(ref p)) if p == "iteration"
        ),
        "interpolation error must focus the excerpt on its receiving key"
    );

    let enriched = err.enrich_frontmatter(&source, true);
    assert!(
        enriched.frontmatter_excerpt().is_some(),
        "a focused excerpt must be attached"
    );
}

#[test]
fn enrich_schema_parse_highlights_offending_property_line() {
    // A grammar failure attributed to `spec` (bad `,` separator) must focus
    // the excerpt on the `$schema.spec` type-string line (line 3), not the
    // top-level `spec` value on line 4 and not the whole block.
    let source = source_from(
        "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nbody\n",
    );
    let err = CompositionError::SchemaParse {
        source_path: PathBuf::from("review.md"),
        property: Some("spec".to_string()),
        message: "expected `;` between constraints".to_string(),
        span: Some(14..15),
    }
    .enrich_frontmatter(&source, true);

    let excerpt = err.frontmatter_excerpt().expect("excerpt must attach");
    assert_eq!(
        excerpt.highlight_line(),
        Some(3),
        "must highlight the `$schema.spec` type-string line"
    );
    assert_ne!(
        excerpt.highlight_line(),
        Some(4),
        "must not highlight the unrelated top-level `spec` value line"
    );
}

#[test]
fn enrich_schema_parse_shape_falls_back_to_schema_parent_line() {
    // A whole-shape failure (no property, no span) highlights the `$schema`
    // parent line (line 2).
    let source = source_from("---\n$schema: 42\n---\nbody\n");
    let err = CompositionError::SchemaParse {
        source_path: PathBuf::from("review.md"),
        property: None,
        message: "expected mapping, got integer".to_string(),
        span: None,
    }
    .enrich_frontmatter(&source, true);

    let excerpt = err.frontmatter_excerpt().expect("excerpt must attach");
    assert_eq!(excerpt.highlight_line(), Some(2));
}

#[test]
fn schema_parse_block_links_prompt_file_and_strips_when_no_color() {
    // The rendered body OSC8-links the prompt file when color is available,
    // and strips all escapes (no raw OSC8) at `ColorDepth::None`.
    let source_path = std::env::temp_dir().join("review.md");
    let err = CompositionError::SchemaParse {
        source_path,
        property: Some("spec".to_string()),
        message: "expected `;` between constraints".to_string(),
        span: Some(14..15),
    };

    let color_term = Terminal::new_optimistic(80);
    let linked = err.report_block_error(&color_term);
    assert!(
        linked.contains("\x1b]8;;"),
        "color render must carry an OSC8 link; got: {linked:?}"
    );

    let plain_term = Terminal::builder()
        .width(80)
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
        .build();
    let plain = err.report_block_error(&plain_term);
    assert!(
        !plain.contains('\x1b'),
        "no-color render must strip escapes; got: {plain:?}"
    );
    assert!(
        plain.contains("review.md"),
        "plain render must still name the prompt file; got: {plain}"
    );
}

#[test]
fn loop_iteration_failed_display_surfaces_reason_and_iteration() {
    let err = CompositionError::LoopIterationFailed {
        iteration: 2,
        prompt_path: PathBuf::from("fixes/plan.md"),
        exit_code: 1,
        reason: "step_timeout after 30m of stream silence".to_string(),
        exit_reason: Some("step_timeout".to_string()),
        snapshot: None,
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("loop iteration 2 of fixes/plan.md"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains("step_timeout after 30m of stream silence"),
        "got: {rendered}"
    );
    assert!(
        !rendered.contains("invalid loop definition"),
        "got: {rendered}"
    );
}

#[test]
fn loop_rate_limited_display_includes_reset_time_when_present() {
    let reset = DateTime::parse_from_rfc3339("2026-05-12T18:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let err = CompositionError::LoopRateLimited {
        iteration: 1,
        prompt_path: PathBuf::from("plan.md"),
        provider: Some("k2p6".to_string()),
        model: Some("kimi-for-coding".to_string()),
        reset_at: Some(reset),
        message: Some("Usage limit reached for k2p6".to_string()),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("k2p6"), "got: {rendered}");
    assert!(rendered.contains("resets at"), "got: {rendered}");
    assert!(rendered.contains("Usage limit reached"), "got: {rendered}");
}

#[test]
fn loop_rate_limited_display_omits_optional_fields_when_absent() {
    let err = CompositionError::LoopRateLimited {
        iteration: 3,
        prompt_path: PathBuf::from("plan.md"),
        provider: None,
        model: None,
        reset_at: None,
        message: None,
    };
    let rendered = err.to_string();
    assert!(
        rendered.starts_with("loop halted at iteration 3 of plan.md"),
        "got: {rendered}"
    );
    assert!(
        rendered.contains("provider rate limited"),
        "got: {rendered}"
    );
    // No reset clause when reset_at is absent
    assert!(!rendered.contains("resets at"), "got: {rendered}");
}

#[test]
fn loop_iteration_failed_falls_back_when_no_exit_reason() {
    let err = CompositionError::LoopIterationFailed {
        iteration: 4,
        prompt_path: PathBuf::from("plan.md"),
        exit_code: 1,
        reason: "provider exited non-zero".to_string(),
        exit_reason: None,
        snapshot: None,
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("provider exited non-zero"),
        "got: {rendered}"
    );
}

#[test]
fn loop_invalid_still_reserved_for_frontmatter_problems() {
    // Sanity: LoopInvalid is still the right variant for malformed
    // frontmatter and renders distinctly from the runtime-fault
    // variants above.
    let err = CompositionError::LoopInvalid("`loop.max` must be greater than zero".to_string());
    let rendered = err.to_string();
    assert!(
        rendered.starts_with("invalid loop definition:"),
        "got: {rendered}"
    );
}

// -------------------------------------------------------------------------
// Schema errors
// -------------------------------------------------------------------------

#[test]
fn schema_load_display_includes_path_and_message() {
    let err = CompositionError::SchemaLoad {
        source_path: PathBuf::from("prompts/plan.md"),
        message: "unsupported `http://` schema reference".to_string(),
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("prompts/plan.md"),
        "expected path in display: {rendered}"
    );
    assert!(
        rendered.contains("unsupported `http://`"),
        "expected message in display: {rendered}"
    );
}

#[test]
fn schema_validation_display_includes_message() {
    let err = CompositionError::SchemaValidation {
        source_path: PathBuf::from("prompts/plan.md"),
        message: "expected number, got string".to_string(),
        problems: vec!["/properties/count".to_string()],
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("expected number, got string"),
        "got: {rendered}"
    );
}

#[test]
fn missing_properties_display_lists_names_in_order() {
    let err = CompositionError::MissingProperties {
        source_path: PathBuf::from("prompts/plan.md"),
        missing: vec![
            MissingProperty {
                name: "target".to_string(),
                type_label: Some("string".to_string()),
                description: None,
                interactive_shape: None,
            },
            MissingProperty {
                name: "count".to_string(),
                type_label: Some("number".to_string()),
                description: None,
                interactive_shape: None,
            },
        ],
        frontmatter_description: None,
        pointer_paths: Vec::new(),
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("target, count"),
        "expected declaration-order names: {rendered}"
    );
    assert!(
        rendered.contains("properties"),
        "expected plural form for >1 missing: {rendered}"
    );
}

#[test]
fn missing_properties_display_uses_singular_for_one() {
    let err = CompositionError::MissingProperties {
        source_path: PathBuf::from("prompts/plan.md"),
        missing: vec![MissingProperty {
            name: "target".to_string(),
            type_label: Some("string".to_string()),
            description: None,
            interactive_shape: None,
        }],
        frontmatter_description: None,
        pointer_paths: Vec::new(),
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("property") && !rendered.contains("properties"),
        "expected singular form: {rendered}"
    );
}

#[test]
fn unsupported_interactive_schema_display_mentions_shape() {
    let err = CompositionError::UnsupportedInteractiveSchema {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "config".to_string(),
        shape: "object".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("`config`"), "got: {rendered}");
    assert!(rendered.contains("object"), "got: {rendered}");
}

#[test]
fn missing_properties_status_block_includes_remediation_hint() {
    use biscuit_terminal::prelude::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    let err = CompositionError::MissingProperties {
        source_path: PathBuf::from("prompts/plan.md"),
        missing: vec![MissingProperty {
            name: "target".to_string(),
            type_label: Some("string".to_string()),
            description: Some("the target to act on".to_string()),
            interactive_shape: None,
        }],
        frontmatter_description: Some("Plan a feature".to_string()),
        pointer_paths: Vec::new(),
    };
    let block = err.status_block(&Terminal::default());
    let rendered = block.render(&Terminal::default());
    assert!(
        rendered.contains("Pass key=value")
            || rendered.contains("prompt_for_missing"),
        "expected remediation hint in rendered output: {rendered}"
    );
}

#[test]
fn sequence_missing_properties_status_block_lists_each_step() {
    use biscuit_terminal::prelude::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    let err = CompositionError::SequenceMissingProperties {
        failure_count: 2,
        failures: vec![
            SequenceMissingPropertiesStep {
                step: 1,
                step_name: "research".to_string(),
                source_path: PathBuf::from("prompts/seq.md"),
                missing: vec![MissingProperty {
                    name: "topic".to_string(),
                    type_label: Some("string".to_string()),
                    description: None,
                    interactive_shape: None,
                }],
                frontmatter_description: None,
                pointer_paths: Vec::new(),
            },
            SequenceMissingPropertiesStep {
                step: 2,
                step_name: "summarize".to_string(),
                source_path: PathBuf::from("prompts/seq.md"),
                missing: vec![MissingProperty {
                    name: "tone".to_string(),
                    type_label: Some("enum(formal|casual)".to_string()),
                    description: Some("the desired tone".to_string()),
                    interactive_shape: None,
                }],
                frontmatter_description: None,
                pointer_paths: Vec::new(),
            },
        ],
    };
    let block = err.status_block(&Terminal::default());
    let rendered = block.render(&Terminal::default());
    assert!(rendered.contains("Step 1"), "got: {rendered}");
    assert!(rendered.contains("Step 2"), "got: {rendered}");
    assert!(rendered.contains("topic"), "got: {rendered}");
    assert!(rendered.contains("tone"), "got: {rendered}");
    assert!(
        rendered.contains("research") && rendered.contains("summarize"),
        "got: {rendered}"
    );
}

#[test]
fn sequence_missing_properties_display_includes_failure_count() {
    let err = CompositionError::SequenceMissingProperties {
        failure_count: 3,
        failures: Vec::new(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("3 step(s)"), "got: {rendered}");
}

// -------------------------------------------------------------------------
// Agent-resolution no-TTY abort body parity with the dry-run / TTY message
// -------------------------------------------------------------------------

use super::super::agent_message::{agent_state_breakdown, invalid_agent_message};
use super::super::types::AgentResolutionState;

const FILE_LINK: &str = "<a href=\"file:///doc.md\">doc.md</a>";

#[test]
fn no_tty_no_agent_body_matches_canonical_breakdown() {
    let state = AgentResolutionState::NoAgent;
    let body = render_agent_resolution_failed_body(&state, &[], FILE_LINK);
    assert_eq!(body, agent_state_breakdown(&state));
}

#[test]
fn no_tty_not_installed_body_matches_canonical_breakdown() {
    let state = AgentResolutionState::SingleNotInstalled {
        provider: Provider::Gemini,
    };
    let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
    assert_eq!(body, agent_state_breakdown(&state));
}

#[test]
fn no_tty_list_multiple_body_matches_canonical_breakdown() {
    // Regression: the old body used "the interactive picker would ask …",
    // which drifted from the dry-run cell wording.
    let state = AgentResolutionState::ListMultipleInstalled {
        installed: vec![Provider::Claude, Provider::Codex],
        not_installed: vec![Provider::Gemini],
        invalid: vec!["bad".into()],
    };
    let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
    assert_eq!(body, agent_state_breakdown(&state));
    assert!(
        body.contains("choose interactively between suggested Agents"),
        "got: {body}"
    );
    assert!(!body.contains("the interactive picker would ask"), "got: {body}");
}

#[test]
fn no_tty_zero_installed_body_matches_canonical_breakdown() {
    // Regression: the old body appended "the current session is not
    // interactive", which the TTY/dry-run message never showed.
    let state = AgentResolutionState::ZeroInstalledList {
        not_installed: vec![Provider::Gemini],
        invalid: vec!["bad".into()],
    };
    let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
    assert_eq!(body, agent_state_breakdown(&state));
    assert!(!body.contains("not interactive"), "got: {body}");
}

#[test]
fn no_tty_single_invalid_body_is_imperative_message_plus_installed_list() {
    let state = AgentResolutionState::SingleInvalid {
        hint: "nope".into(),
    };
    let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
    assert!(body.starts_with(&invalid_agent_message("nope", FILE_LINK)), "got: {body}");
    assert!(body.contains(&format!("- {}", Provider::Claude)), "got: {body}");
}

#[test]
fn no_tty_single_invalid_body_notes_no_agents_when_none_installed() {
    let state = AgentResolutionState::SingleInvalid {
        hint: "nope".into(),
    };
    let body = render_agent_resolution_failed_body(&state, &[], FILE_LINK);
    assert!(body.contains("no agents are installed"), "got: {body}");
}

#[test]
fn missing_properties_status_block_lists_pointer_paths_when_no_typed_metadata() {
    use biscuit_terminal::prelude::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    let err = CompositionError::MissingProperties {
        source_path: PathBuf::from("prompts/plan.md"),
        missing: Vec::new(),
        frontmatter_description: None,
        pointer_paths: vec!["/properties/target".to_string()],
    };
    let block = err.status_block(&Terminal::default());
    let rendered = block.render(&Terminal::default());
    assert!(
        rendered.contains("/properties/target"),
        "expected JSON pointer in rendered output: {rendered}"
    );
}

// -------------------------------------------------------------------------
// Inline-compose / sequence mismatch diagnostic (spec criteria 11-16)
// -------------------------------------------------------------------------

use biscuit_terminal::utils::escape_codes::strip_escape_codes;

fn mismatch_err() -> CompositionError {
    CompositionError::InlineComposeSequenceMismatch {
        source_path: PathBuf::from("prompts/greeting.md"),
    }
}

#[test]
fn mismatch_render_includes_diagnostic() {
    // The diagnostic names the document, both properties, the `claudine
    // sequence` directive, and the `sections` note. The authored YAML
    // block is appended separately by the CLI walker (tested there).
    let err = mismatch_err();
    let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
    assert!(rendered.contains("greeting.md"), "document name: {rendered}");
    assert!(rendered.contains("prompt"), "names prompt: {rendered}");
    assert!(rendered.contains("sequence"), "names sequence: {rendered}");
    assert!(
        rendered.contains("claudine sequence"),
        "sequence directive: {rendered}"
    );
    assert!(rendered.contains("sections"), "sections note: {rendered}");
}

#[test]
fn mismatch_plain_terminal_render_has_no_escape_bytes() {
    // A terminal with no color depth cannot display SGR styling or OSC 8
    // hyperlinks, so the rendered diagnostic must contain no escape byte at
    // all — otherwise redirected / `NO_COLOR` output is polluted.
    let mut term = Terminal::builder()
        .width(80)
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
        .build();
    term.is_nerd_font = Some(false);
    let err = mismatch_err();
    let rendered = err.report_block_error(&term);
    assert!(
        !rendered.contains('\x1b'),
        "plain render must contain no escape byte; got: {rendered:?}"
    );
    assert!(rendered.contains("greeting.md"), "got: {rendered}");
    assert!(rendered.contains("claudine sequence"), "got: {rendered}");
}

#[test]
fn mismatch_display_message_is_plain() {
    // The `#[error(...)]` summary is plain text with no rendering markup.
    let err = mismatch_err();
    let rendered = err.to_string();
    assert!(
        rendered.contains("cannot run a document configured as a sequence"),
        "got: {rendered}"
    );
    assert!(!rendered.contains('<'), "no markup in Display: {rendered}");
}

// -------------------------------------------------------------------------
// Phase 4: regression — hint inside block quote for composition errors
// -------------------------------------------------------------------------

#[test]
fn unsupported_interactive_schema_hint_appears_inside_block_quote_border() {
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let err = CompositionError::UnsupportedInteractiveSchema {
        source_path: PathBuf::from("prompts/review.md"),
        property: "spec".to_string(),
        shape: "(unknown)".to_string(),
    };
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(err.report_block_error(&term));

    let hint_token = "Pass the value with key=value";
    let hint_lines: Vec<&str> = rendered
        .lines()
        .filter(|l| l.contains(hint_token))
        .collect();
    assert!(
        !hint_lines.is_empty(),
        "hint text must appear in rendered output: {rendered}"
    );
    for hint_line in &hint_lines {
        assert!(
            hint_line.contains('┃'),
            "regression: hint must appear inside block quote border, got: {hint_line:?}\nfull output:\n{rendered}"
        );
    }

    let body_token = "cannot be collected interactively";
    assert!(
        rendered.contains(body_token),
        "body text must appear: {rendered}"
    );
}

#[test]
fn missing_properties_hint_appears_inside_block_quote_border() {
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let err = CompositionError::MissingProperties {
        source_path: PathBuf::from("prompts/plan.md"),
        missing: vec![MissingProperty {
            name: "target".to_string(),
            type_label: Some("string".to_string()),
            description: None,
            interactive_shape: None,
        }],
        frontmatter_description: None,
        pointer_paths: Vec::new(),
    };
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(err.report_block_error(&term));

    let hint_token = "Pass key=value";
    let hint_lines: Vec<&str> = rendered
        .lines()
        .filter(|l| l.contains(hint_token))
        .collect();
    assert!(
        !hint_lines.is_empty(),
        "hint text must appear in rendered output: {rendered}"
    );
    for hint_line in &hint_lines {
        assert!(
            hint_line.contains('┃'),
            "regression: hint must appear inside block quote border, got: {hint_line:?}\nfull output:\n{rendered}"
        );
    }
}

#[test]
fn resume_incompatible_status_block_names_each_changed_facet() {
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let err = CompositionError::LifecycleResumeIncompatible {
        source_path: PathBuf::from("prompts/deploy.md"),
        facets: vec!["model".to_string(), "workspace CWD".to_string()],
    };
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(err.report_block_error(&term));

    assert!(
        rendered.contains("model") && rendered.contains("workspace CWD"),
        "the diagnostic must name every changed facet: {rendered}"
    );
    assert!(
        rendered.to_lowercase().contains("retry"),
        "the diagnostic must recommend retry: {rendered}"
    );
    // `Display` (the log/JSON surface) must also carry the facets.
    assert!(err.to_string().contains("model"));
}

#[test]
fn schema_load_hint_appears_inside_block_quote_border() {
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let err = CompositionError::SchemaLoad {
        source_path: PathBuf::from("prompts/deploy.md"),
        message: "unsupported protocol".to_string(),
    };
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(err.report_block_error(&term));

    let hint_token = "Verify the `$schema` path";
    let hint_lines: Vec<&str> = rendered
        .lines()
        .filter(|l| l.contains(hint_token))
        .collect();
    assert!(
        !hint_lines.is_empty(),
        "hint text must appear in rendered output: {rendered}"
    );
    for hint_line in &hint_lines {
        assert!(
            hint_line.contains('┃'),
            "regression: hint must appear inside block quote border, got: {hint_line:?}\nfull output:\n{rendered}"
        );
    }
}

// -------------------------------------------------------------------------
// Phase 4: shell-expansion failure boundary fidelity
// -------------------------------------------------------------------------

fn shell_expansion_failed_err() -> CompositionError {
    use darkmatter::markdown::compose::ShellCommandOrigin;

    let content = "---\ntitle: Test\n---\n# Body\n\n::shell \"cmd-that-fails\"\n";
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("/repo/prompts/test.md"),
        PathBuf::from("prompts/test.md"),
        content,
    );
    let shell = ShellExpansionError::ExecutionFailed {
        ctx: Box::new(ctx),
        command: "cmd-that-fails".to_string(),
        code: 2,
        stdout: "".to_string(),
        stderr: "this command failed\nunknown flag --whatever".to_string(),
        origin: ShellCommandOrigin::Body { line: 6 },
    };
    CompositionError::ShellExpansionFailed {
        source_path: PathBuf::from("prompts/test.md"),
        error: Box::new(shell),
    }
}

#[test]
fn shell_expansion_failed_status_block_delegates_to_shell_error() {
    use biscuit_terminal::prelude::TerminalRenderable;

    let err = shell_expansion_failed_err();
    let term = Terminal::new_optimistic(80);
    let rendered = err.status_block(&term).render(&term);
    assert!(
        rendered.contains("ShellExpansionError"),
        "expected delegated shell-expansion header: {rendered}"
    );
    assert!(
        !rendered.contains("CompositionError"),
        "must not use the generic composition header: {rendered}"
    );
}

#[test]
fn shell_expansion_failed_preserves_rich_diagnostic() {
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let err = shell_expansion_failed_err();
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(err.report_block_error(&term));

    assert!(
        rendered.contains("line 6"),
        "expected file-relative line in diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("this command failed"),
        "expected stderr text in diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("::shell"),
        "expected source excerpt in diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("cmd-that-fails"),
        "expected command name in diagnostic: {rendered}"
    );
}

#[test]
fn shell_expansion_failed_plain_terminal_has_no_escape_bytes() {
    let mut term = Terminal::builder()
        .width(80)
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
        .build();
    term.is_nerd_font = Some(false);

    let err = shell_expansion_failed_err();
    let rendered = err.report_block_error(&term);

    assert!(
        !rendered.contains('\x1b'),
        "plain render must contain no escape byte; got: {rendered:?}"
    );
    assert!(rendered.contains("line 6"), "got: {rendered}");
    assert!(rendered.contains("this command failed"), "got: {rendered}");
    assert!(rendered.contains("::shell"), "got: {rendered}");
}

/// Exercise the full Markdown → `map_compose_error` → `report_block_error`
/// path with a real failing `::shell` directive.
///
/// This complements the hand-built `shell_expansion_failed_err` tests by
/// proving that a captured `ExecutionFailed` from an actual subprocess
/// survives through `prepare_direct` and renders with file-relative line
/// numbers, the command's stderr, a source excerpt, and the composed
/// frontmatter block.
#[test]
fn shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic() {
    use std::collections::{BTreeMap, HashSet};

    use biscuit_terminal::terminal::Terminal;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    use super::super::prepare::{PrepareOptions, prepare_direct};
    use super::super::resolve::resolve_composition_source;

    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.md");
    let executable = std::env::current_exe().expect("test executable path should be available");
    let executable = biscuit_file::to_portable_string(&executable);
    let command = format!("\"{executable}\" --definitely-invalid-libtest-option");
    let approved_command = format!("{executable} --definitely-invalid-libtest-option");
    let content = format!(
        "---\ntitle: Shell demo\n---\n\nPre.\n\n::shell {command}\n\nPost.\n"
    );
    std::fs::write(&file_path, content).unwrap();

    let source = resolve_composition_source(file_path.to_str().unwrap()).unwrap();

    let mut approved = HashSet::new();
    approved.insert(approved_command);
    let options = PrepareOptions {
        defer_schema_verdict: false,
        set_overrides: None,
        caller_input_records: Default::default(),
        pre_approved_commands: Some(approved),
        env_overrides: BTreeMap::new(),
        perf_enabled: false,
        source_repo_root: None,
        shell_working_directory: None,
        prepared_context: Some(
            darkmatter::markdown::compose::ComposeContext::capture_for_content(
                temp_dir.path(),
                "",
            ),
        ),
        file_ref_fallback_dir: None,
        file_resolution_context: None,
        name_coercion_keys: Vec::new(),
        allow_empty_body: false,
        invocation_context: None,
        document_epoch: None,
    };

    let err = prepare_direct(&source, options).unwrap_err();
    let term = Terminal::new_optimistic(80);
    let rendered = strip_escape_codes(err.report_block_error(&term));

    assert!(
        rendered.contains("line 7"),
        "expected file-relative line in diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("error:"),
        "expected captured rustc stderr text in diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("::shell"),
        "expected source excerpt in diagnostic: {rendered}"
    );
    assert!(
        rendered.contains("title:") || rendered.contains("---"),
        "expected frontmatter block in diagnostic: {rendered}"
    );
}

#[test]
fn shell_expansion_detail_command_is_the_failed_command_not_source_path() {
    // `composition.shell_expansion` declares `command`. A handler reading
    // `detail["command"]` must get the authored shell command, never the
    // Markdown source path. `shell_expansion_failed_err` carries
    // `command: "cmd-that-fails"` and `source_path: "prompts/test.md"`.
    let detail = shell_expansion_failed_err().detail();
    assert_eq!(
        detail["command"],
        json!("cmd-that-fails"),
        "command must project the failed command, got: {detail}"
    );
}

#[test]
fn shell_expansion_detail_command_is_null_for_command_less_variant() {
    // `ParseDirective` carries no command, so `command` stays the seeded
    // JSON null rather than falling back to the source path.
    use darkmatter::markdown::compose::ShellCommandOrigin;

    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("/repo/prompts/test.md"),
        PathBuf::from("prompts/test.md"),
        "---\ntitle: Test\n---\n",
    );
    let err = CompositionError::ShellExpansionFailed {
        source_path: PathBuf::from("prompts/test.md"),
        error: Box::new(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx),
            origin: ShellCommandOrigin::Body { line: 1 },
            message: "bad directive".to_string(),
        }),
    };
    let detail = err.detail();
    assert_eq!(
        detail["command"],
        Value::Null,
        "command-less variant must leave `command` as JSON null, got: {detail}"
    );
}

// -------------------------------------------------------------------------
// New lifecycle action form errors (Phase 2)
// -------------------------------------------------------------------------

#[test]
fn lifecycle_short_form_removed_display_includes_rewrite() {
    let err = CompositionError::LifecycleShortFormRemoved {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        raw: "success(\"x\")".to_string(),
        rewrite: "success: \"x\"".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("short-form lifecycle action"), "got: {rendered}");
    assert!(rendered.contains("success(\"x\")"), "got: {rendered}");
    assert!(rendered.contains("success: \"x\""), "got: {rendered}");
    assert!(rendered.contains("prompts/plan.md"), "got: {rendered}");
}

#[test]
fn lifecycle_short_form_removed_status_block_is_escape_free_at_none() {
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::terminal::Terminal;
    let err = CompositionError::LifecycleShortFormRemoved {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        raw: "success(\"x\")".to_string(),
        rewrite: "success: \"x\"".to_string(),
    };
    let term = Terminal {
        color_depth: ColorDepth::None,
        ..Terminal::new_optimistic(80)
    };
    let rendered = err.report_block_error(&term);
    assert!(
        !rendered.contains('\x1b'),
        "expected no escape codes at ColorDepth::None: {rendered}"
    );
    assert!(rendered.contains("short-form action removed"), "got: {rendered}");
    assert!(
        rendered.contains("Rewrite to positional form:"),
        "got: {rendered}"
    );
    assert!(rendered.contains("success:"), "got: {rendered}");
    assert!(rendered.contains("\\\"x\\\""), "got: {rendered}");
}

#[test]
fn phase_1_plain_composition_error_block_snapshots() {
    use biscuit_terminal::discovery::detection::ColorDepth;

    let term = Terminal {
        color_depth: ColorDepth::None,
        ..Terminal::new_optimistic(80)
    };
    let short_form = CompositionError::LifecycleShortFormRemoved {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        raw: "success(\"x\")".to_string(),
        rewrite: "success: \"x\"".to_string(),
    }
    .report_block_error(&term);
    let selection = CompositionError::NoRunnableProviders.report_block_error(&term);

    assert_eq!(
        short_form,
        "⤫ CompositionError: short-form action removed\n\
┃ \n\
┃ Short-form lifecycle action `success(\\\"x\\\")` in `success` in\n\
┃ prompts/plan.md has been removed.\n\
┃ \n\
┃ Rewrite to positional form: `success: \\\"x\\\"`\n\
┃ \n\
┃ Use positional form (`verb: value`) or key/value form (`{ action: verb,\n\
┃ ... }`). `verb(args)` is no longer accepted."
    );
    assert_eq!(
        selection,
        "⤫ CompositionError: composition failed\n\
┃ \n\
┃ no runnable providers available (all excluded or uninstalled)"
    );
}

#[test]
fn lifecycle_unknown_verb_display_includes_rewrite() {
    let err = CompositionError::LifecycleUnknownVerb {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        verb: "sucess".to_string(),
        rewrite: "did you mean `success`?".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("unknown lifecycle action"), "got: {rendered}");
    assert!(rendered.contains("sucess"), "got: {rendered}");
    assert!(rendered.contains("did you mean `success`?"), "got: {rendered}");
}

#[test]
fn lifecycle_wrong_arity_display_includes_message() {
    let err = CompositionError::LifecycleWrongArity {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        verb: "set_frontmatter".to_string(),
        message: "expected 3 arguments [file, prop, value], got 1".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("set_frontmatter"), "got: {rendered}");
    assert!(rendered.contains("expected 3 arguments"), "got: {rendered}");
}

#[test]
fn lifecycle_multiple_actions_code_is_lifecycle_invalid() {
    // Regression: this cardinality variant used to fall through code()'s
    // catch-all to `composition.failed`, diverging from its sibling
    // `LifecycleActionOrder` which already mapped to the lifecycle family.
    let err = CompositionError::LifecycleMultipleLifecycleActions {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "start".to_string(),
    };
    assert_eq!(err.code(), "composition.lifecycle_invalid");
}

#[test]
fn lifecycle_multiple_actions_detail_projects_property_and_message() {
    let err = CompositionError::LifecycleMultipleLifecycleActions {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "start".to_string(),
    };
    let detail = err.detail();
    assert_eq!(
        detail["property"],
        json!("start"),
        "property must project the offending event name, got: {detail}"
    );
    // No dedicated `message` field on this variant, so the synthesized
    // value must be a present, non-null string.
    assert!(
        detail["message"].is_string(),
        "message must be a present non-null string, got: {detail}"
    );
}

#[test]
fn lifecycle_action_order_detail_projects_property_and_message() {
    // The named counterpart in the review: it too lacked an explicit
    // detail() arm and only got `property: null`. Guard it per-variant.
    let err = CompositionError::LifecycleActionOrder {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "start".to_string(),
    };
    assert_eq!(err.code(), "composition.lifecycle_invalid");
    let detail = err.detail();
    assert_eq!(detail["property"], json!("start"), "got: {detail}");
    assert!(detail["message"].is_string(), "got: {detail}");
}

#[test]
fn lifecycle_object_data_positional_display_mentions_interpolation() {
    let err = CompositionError::LifecycleObjectDataThroughInterpolationPositional {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        verb: "merge_frontmatter".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("merge_frontmatter"), "got: {rendered}");
    assert!(rendered.contains("whole-value"), "got: {rendered}");
    assert!(rendered.contains("{{ ... }}"), "got: {rendered}");
}

#[test]
fn lifecycle_object_data_parameter_display_mentions_param() {
    let err = CompositionError::LifecycleObjectDataThroughInterpolationParameter {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        verb: "set_frontmatter".to_string(),
        param: "value".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("set_frontmatter"), "got: {rendered}");
    assert!(rendered.contains("parameter `value`"), "got: {rendered}");
}

#[test]
fn lifecycle_stack_ambiguous_display_includes_message() {
    let err = CompositionError::LifecycleStackAmbiguous {
        source_path: PathBuf::from("prompts/plan.md"),
        property: "success".to_string(),
        message: "did you mean `success: ...` or `{ action: success, ... }`?".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("ambiguous lifecycle stack item"), "got: {rendered}");
    assert!(rendered.contains("did you mean"), "got: {rendered}");
}

#[test]
fn new_lifecycle_errors_get_frontmatter_excerpt() {
    let source = source_from(
        "---\nsuccess:\n    sucess: \"x\"\n---\nbody\n",
    );
    let err = CompositionError::LifecycleUnknownVerb {
        source_path: PathBuf::from("review.md"),
        property: "success".to_string(),
        verb: "sucess".to_string(),
        rewrite: "did you mean `success`?".to_string(),
    }
    .enrich_frontmatter(&source, true);

    assert!(matches!(err, CompositionError::WithFrontmatter { .. }));
    assert!(err.frontmatter_excerpt().is_some());
}

// -------------------------------------------------------------------------
// Phase 1 autocomplete error variants
// -------------------------------------------------------------------------

#[test]
fn autocomplete_no_matches_display_includes_query() {
    let err = CompositionError::AutocompleteNoMatches {
        query: "foo".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("foo"), "got: {rendered}");
    assert!(rendered.contains("no files matched"), "got: {rendered}");
}

#[test]
fn autocomplete_over_cap_display_includes_query_and_cap() {
    let err = CompositionError::AutocompleteOverCap {
        query: "bar".to_string(),
        cap: 500,
    };
    let rendered = err.to_string();
    assert!(rendered.contains("bar"), "got: {rendered}");
    assert!(rendered.contains("500"), "got: {rendered}");
    assert!(rendered.contains("narrow your query"), "got: {rendered}");
}

#[test]
fn autocomplete_not_interactive_display_is_actionable() {
    let err = CompositionError::AutocompleteNotInteractive;
    let rendered = err.to_string();
    assert!(
        rendered.contains("interactive terminal"),
        "got: {rendered}"
    );
}

#[test]
fn autocomplete_errors_do_not_get_frontmatter_excerpt() {
    let source = source_from("---\ntitle: x\n---\nbody\n");
    let err = CompositionError::AutocompleteNoMatches {
        query: "q".to_string(),
    }
    .enrich_frontmatter(&source, true);
    assert!(
        matches!(err, CompositionError::AutocompleteNoMatches { .. }),
        "expected no wrapping, got: {err:?}"
    );
}

#[test]
fn autocomplete_over_cap_status_block_names_query() {
    use biscuit_terminal::prelude::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;

    let err = CompositionError::AutocompleteOverCap {
        query: "plan".to_string(),
        cap: 500,
    };
    let rendered = err.status_block(&Terminal::default()).render(&Terminal::default());
    assert!(rendered.contains("plan"), "got: {rendered}");
    assert!(rendered.contains("500"), "got: {rendered}");
    assert!(rendered.contains("narrow"), "got: {rendered}");
}

/// A hand-off refused for want of a coordinator must tell the user *both*
/// halves: which command cannot host it, and which command can.
///
/// Naming only the failure would leave the operator with a correct diagnostic
/// and no next step — the direct provider wrappers give no hint that `compose`
/// is the command that owns an active-document coordinator (R10 / AC30).
#[test]
fn proxy_without_owning_coordinator_names_the_command_that_can_host_it() {
    use biscuit_terminal::prelude::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;

    let err = CompositionError::LifecycleProxyWithoutOwningCoordinator {
        source_path: PathBuf::from("/repo/CLAUDE.md"),
        property: "failure.stack[0].action[1]".to_string(),
        target: "prompts/next.md".to_string(),
        command: "claudine claude".to_string(),
    };
    let rendered = err
        .status_block(&Terminal::default())
        .render(&Terminal::default());

    assert!(
        rendered.contains("prompts/next.md"),
        "the refused target is named, got: {rendered}"
    );
    assert!(
        rendered.contains("claudine claude"),
        "the command that cannot host it is named, got: {rendered}"
    );
    assert!(
        rendered.contains("claudine compose"),
        "a command that can host it is named, got: {rendered}"
    );
    assert!(
        rendered.contains("failure.stack[0].action[1]"),
        "the authored `proxy` action is located, got: {rendered}"
    );
}

/// Wrap a [`FileReferenceDiagnostic`] in the same `ComposeFailed` /
/// `Interpolation` shape the live compose path produces, so `detail()`
/// exercises the real projection.
fn file_ref_compose_error(diagnostic: FileReferenceDiagnostic) -> CompositionError {
    use darkmatter::markdown::SourceRef;
    CompositionError::ComposeFailed(MarkdownError::Interpolation {
        key: Some("spec".to_string()),
        expression: "frontmatter('features/spec.md')".to_string(),
        source: Box::new(SourceRef::Effective {
            rendered: "frontmatter('features/spec.md')".to_string(),
            origin_key: Some("spec".to_string()),
        }),
        cause: Box::new(ExpressionError::FileReference(diagnostic)),
    })
}

#[test]
fn caller_file_classification_drift_keeps_schema_diagnostic_identity() {
    let err = CompositionError::ComposeFailed(
        MarkdownError::CallerFileClassificationChanged {
            property: "spec".to_string(),
        },
    );

    assert_eq!(err.code(), "composition.schema_validation");
    assert_eq!(err.detail()["problems"], json!(["/spec"]));
    assert_eq!(err.detail()["pointer_paths"], json!(["/spec"]));
    let rendered = err
        .status_block(&biscuit_terminal::terminal::Terminal::default())
        .render(&biscuit_terminal::terminal::Terminal::default());
    assert!(rendered.contains("spec"), "got: {rendered}");
    assert!(rendered.contains("eager"), "got: {rendered}");
}

#[test]
fn file_reference_detail_serializes_kind_as_snake_case() {
    // `kind` must be the catalog snake_case slug, never the Debug form.
    for (kind, expected) in [
        (FileRefFailure::NotFound, "not_found"),
        (FileRefFailure::Malformed, "malformed"),
        (FileRefFailure::FoundElsewhere, "found_elsewhere"),
        (FileRefFailure::RemoteNotEnabled, "remote_not_enabled"),
    ] {
        let err = file_ref_compose_error(FileReferenceDiagnostic {
            function: "frontmatter",
            reference: "features/spec.md".to_string(),
            kind,
            base_dir: PathBuf::from("/repo"),
            fallback_dir: None,
            source: None,
        });
        let detail = err.detail();
        assert_eq!(
            detail["kind"],
            json!(expected),
            "kind must serialize snake_case, not Debug: {detail}"
        );
        assert_ne!(detail["kind"], json!(format!("{kind:?}")));
    }
}

#[test]
fn file_reference_detail_emits_full_registry_field_set() {
    let err = file_ref_compose_error(FileReferenceDiagnostic {
        function: "frontmatter",
        reference: "features/spec.md".to_string(),
        kind: FileRefFailure::Malformed,
        base_dir: PathBuf::from("/repo/area"),
        fallback_dir: None,
        source: None,
    });
    let detail = err.detail();
    // Read the field set from the registry rather than restating it: the
    // catalog is additive, so a hard-coded list would keep passing while the
    // projection silently omitted every field added after it was written.
    let spec = crate::diagnostics::code_spec("composition.invalid_file_reference").unwrap();
    for &field in spec.detail {
        assert!(
            detail.get(field).is_some(),
            "detail missing registry field `{field}`: {detail}"
        );
    }
    assert_eq!(
        detail.as_object().unwrap().len(),
        spec.detail.len(),
        "detail carries keys the catalog does not declare: {detail}"
    );
    assert_eq!(detail["reference"], json!("features/spec.md"));
    assert_eq!(detail["base_dir"], json!("/repo/area"));
    // No fallback_dir set → projects to null (the optional sentinel).
    assert_eq!(detail["fallback_dir"], Value::Null);
    // Malformed reference offers no sibling suggestions.
    assert_eq!(detail["suggestions"], json!([]));
}

#[test]
fn file_reference_detail_reserves_the_unavailable_resolver_fields_as_null() {
    // The `FileReferenceDiagnostic` this projects from carries no authoring
    // context or candidate record, so these keys are present-and-null: the
    // file-resolution feature fills them. `failure` in particular must not be
    // back-derived from `kind` — Darkmatter folds permission and
    // missing-context errors into `NotFound`, so `no_match` here would assert a
    // classification nothing made.
    let err = file_ref_compose_error(FileReferenceDiagnostic {
        function: "frontmatter",
        reference: "features/spec.md".to_string(),
        kind: FileRefFailure::NotFound,
        base_dir: PathBuf::from("/repo/area"),
        fallback_dir: None,
        source: None,
    });
    let detail = err.detail();

    for field in [
        "source_path",
        "property",
        "event",
        "repository_root",
        "candidates",
        "failure",
    ] {
        // `.get`, not `detail[field]` — indexing yields `Null` for an *absent*
        // key too, which would pass this assertion on a payload that omits the
        // field entirely. Present-and-null is the contract.
        assert_eq!(
            detail.get(field),
            Some(&Value::Null),
            "`{field}` must be present and null until a resolver supplies it: {detail}"
        );
    }
    // ...while the fields the resolver *does* supply are unaffected.
    assert_eq!(detail["kind"], json!("not_found"));
    assert_eq!(detail["reference"], json!("features/spec.md"));
}

#[test]
fn file_reference_detail_carries_fallback_dir_when_set() {
    let err = file_ref_compose_error(FileReferenceDiagnostic {
        function: "frontmatter",
        reference: "features/spec.md".to_string(),
        kind: FileRefFailure::NotFound,
        base_dir: PathBuf::from("/repo/area"),
        fallback_dir: Some(PathBuf::from("/launch/area")),
        source: None,
    });
    let detail = err.detail();
    assert_eq!(detail["fallback_dir"], json!("/launch/area"));
}

#[test]
fn file_reference_detail_suggestions_match_rendered_did_you_mean() {
    // A missing `specs.md` next to a real `spec.md`: the detail
    // `suggestions` must equal the exact render-time computation.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("spec.md"), b"x").unwrap();

    let diagnostic = FileReferenceDiagnostic {
        function: "frontmatter",
        reference: "specs.md".to_string(),
        kind: FileRefFailure::NotFound,
        base_dir: dir.path().to_path_buf(),
        fallback_dir: None,
        source: None,
    };
    let err = file_ref_compose_error(diagnostic.clone());
    let detail = err.detail();

    // The same computation the renderer runs (errors/blocks.rs).
    let expected_path = diagnostic.base_dir.join(&diagnostic.reference);
    let rendered = suggest_sibling_files(&expected_path, DEFAULT_MAX_SUGGESTIONS);

    assert_eq!(rendered, vec!["spec.md".to_string()], "fixture sanity");
    assert_eq!(
        detail["suggestions"],
        json!(rendered),
        "err.detail.suggestions must equal the rendered did-you-mean set"
    );
}

#[test]
fn file_reference_detail_suggestions_match_rendered_for_stale_directory() {
    // Stale-directory case (the motivating real-errors failure): the
    // reference's parent directory does not exist, so the suggestion logic
    // walks up to the nearest existing ancestor and ranks sibling
    // directories that contain the leaf file. The detail `suggestions`
    // must be NON-empty and carry the suggested relative path
    // (sibling_dir/leaf), matching the render-time computation byte-for-byte.
    let dir = tempfile::tempdir().expect("tempdir");
    let features = dir.path().join("features");
    std::fs::create_dir_all(features.join("2026-06-28-real-errors")).unwrap();
    std::fs::write(
        features.join("2026-06-28-real-errors").join("spec.md"),
        b"x",
    )
    .unwrap();

    let diagnostic = FileReferenceDiagnostic {
        function: "frontmatter",
        reference: "features/2026-06-21-opencode-log-fix/spec.md".to_string(),
        kind: FileRefFailure::NotFound,
        base_dir: dir.path().to_path_buf(),
        fallback_dir: None,
        source: None,
    };
    let err = file_ref_compose_error(diagnostic.clone());
    let detail = err.detail();

    // The same computation the renderer runs (errors/blocks.rs).
    let expected_path = diagnostic.base_dir.join(&diagnostic.reference);
    let rendered = suggest_sibling_files(&expected_path, DEFAULT_MAX_SUGGESTIONS);

    assert_eq!(
        rendered,
        vec!["2026-06-28-real-errors/spec.md".to_string()],
        "fixture sanity: stale-directory arm must surface the real sibling path"
    );
    assert_eq!(
        detail["suggestions"],
        json!(rendered),
        "err.detail.suggestions must equal the rendered did-you-mean set for the stale-directory case"
    );
    // Non-empty + carries the suggested relative path explicitly.
    let suggestions = detail["suggestions"]
        .as_array()
        .expect("suggestions is an array");
    assert!(
        !suggestions.is_empty(),
        "stale-directory case must surface at least one suggestion"
    );
    assert!(
        suggestions.iter().any(|v| v
            .as_str()
            .is_some_and(|s| s.contains("2026-06-28-real-errors/spec.md"))),
        "stale-directory suggestion must carry the sibling/leaf relative path: {suggestions:?}"
    );
}

/// Each error family must reach its own family renderer, not collapse into the
/// generic `composition failed` catch-all. A mis-routed dispatcher arm would
/// swap the family-specific header for the generic one, so the header line is a
/// precise routing witness; the no-escape assertion confirms every family keeps
/// the `ColorDepth::None` plain-text contract after the split.
#[test]
fn phase_11_family_dispatch_routes_to_family_renderers() {
    use biscuit_terminal::discovery::detection::ColorDepth;

    let term = Terminal {
        color_depth: ColorDepth::None,
        ..Terminal::new_optimistic(80)
    };

    // (error, expected family-specific header line).
    let cases: Vec<(CompositionError, &str)> = vec![
        (
            // lifecycle family
            CompositionError::LifecycleErrNotAvailable {
                source_path: PathBuf::from("prompts/plan.md"),
                property: "start".to_string(),
                event: "start".to_string(),
            },
            "⤫ CompositionError: `err` not available in this event",
        ),
        (
            // schema / frontmatter family
            CompositionError::SchemaLoad {
                source_path: PathBuf::from("prompts/plan.md"),
                message: "no such file".to_string(),
            },
            "⤫ CompositionError: schema load failed",
        ),
        (
            // selection / target family
            CompositionError::AutocompleteNoMatches {
                query: "foo".to_string(),
            },
            "⤫ CompositionError: no autocomplete matches",
        ),
        (
            // sequence / loop family
            CompositionError::SequenceInteractiveRejected(PathBuf::from("prompts/seq.md")),
            "⤫ CompositionError: interactive rejected for sequence",
        ),
    ];

    for (err, expected_header) in cases {
        let rendered = err.report_block_error(&term);
        let header = rendered.lines().next().unwrap_or_default();
        assert_eq!(
            header, expected_header,
            "family renderer routing regressed for {err:?}"
        );
        assert!(
            !rendered.contains('\u{1b}'),
            "ColorDepth::None output must be escape-free for {err:?}: {rendered:?}"
        );
    }
}

// -- typed transport (Phase 4) --------------------------------------------
//
// The wrapper's whole purpose is that the concrete cause stays reachable. A
// test that only asserts `Display` text would pass against the `eyre!("{e}")`
// flattening these replaced.

fn proxy_reference_error() -> CompositionError {
    CompositionError::InvalidFileReference {
        context: Box::new(FileReferenceContext {
            source_path: PathBuf::from("/repo/run.md"),
            event: Some("initialize".to_string()),
            property: "initialize".to_string(),
            reference: "nope.md".to_string(),
            hint: "A `proxy` target must name an existing Markdown document.".to_string(),
        }),
        source: crate::harness::HarnessError::PathResolutionFailed {
            raw: "nope.md".to_string(),
            failure: crate::harness::PathResolutionFailure::TargetMissing,
            source_path: Some(PathBuf::from("/repo/run.md")),
            resolved: Some(PathBuf::from("/repo/nope.md")),
            resolution: None,
        },
    }
}

#[test]
fn invalid_file_reference_exposes_its_concrete_source() {
    let err = proxy_reference_error();
    let source = std::error::Error::source(&err).expect("wrapper must expose a source");
    let concrete = source
        .downcast_ref::<crate::harness::HarnessError>()
        .expect("the concrete typed error must survive the wrapper");
    assert!(matches!(
        concrete,
        crate::harness::HarnessError::PathResolutionFailed { .. }
    ));
}

#[test]
fn invalid_file_reference_owns_the_shared_code_for_every_surface() {
    // The wrapper must not coin a proxy-specific code: one identity, with the
    // surface told apart by `event` / `property` in detail.
    use crate::diagnostics::Diagnostic;
    let err = proxy_reference_error();
    assert_eq!(err.code(), "composition.invalid_file_reference");
    assert_eq!(err.role(), crate::diagnostics::DiagnosticRole::Semantic);
}

#[test]
fn invalid_file_reference_detail_unions_authoring_context_over_the_source() {
    use crate::diagnostics::Diagnostic;
    let detail = proxy_reference_error().detail();

    // Authoring context only this layer has.
    assert_eq!(detail["property"], serde_json::json!("initialize"));
    assert_eq!(detail["event"], serde_json::json!("initialize"));
    assert_eq!(detail["reference"], serde_json::json!("nope.md"));
    assert_eq!(detail["source_path"], serde_json::json!("/repo/run.md"));
    // Carried up from the typed source's own projection, not re-derived.
    assert_eq!(detail["failure"], serde_json::json!("no_match"));
    // Still unavailable: not invented to fill the shape.
    assert_eq!(detail["kind"], serde_json::Value::Null);
    assert_eq!(detail["candidates"], serde_json::Value::Null);
}

#[test]
fn invalid_file_reference_detail_declares_every_catalog_field() {
    use crate::diagnostics::Diagnostic;
    let detail = proxy_reference_error().detail();
    let spec = crate::diagnostics::code_spec("composition.invalid_file_reference").unwrap();
    for field in spec.detail {
        assert!(
            detail.get(*field).is_some(),
            "declared field `{field}` absent from the wrapper's projection"
        );
    }
    assert!(!detail.is_null(), "a registered code must not project top-level null");
}

#[test]
fn invalid_file_reference_without_an_event_still_renders_and_classifies() {
    // A non-lifecycle surface (schema, transclusion) has no event; the shared
    // code and the `null` event are both part of the contract.
    use crate::diagnostics::Diagnostic;
    let err = CompositionError::InvalidFileReference {
        context: Box::new(FileReferenceContext {
            source_path: PathBuf::from("/repo/run.md"),
            event: None,
            property: "$schema.plan".to_string(),
            reference: "@missing/plan.md".to_string(),
            hint: "hint".to_string(),
        }),
        source: crate::harness::HarnessError::RepoRootRequired {
            path: "@missing/plan.md".to_string(),
        },
    };
    assert_eq!(err.code(), "composition.invalid_file_reference");
    assert_eq!(err.detail()["event"], serde_json::Value::Null);
    assert!(err.to_string().contains("$schema.plan"), "got: {err}");
    let term = biscuit_terminal::terminal::Terminal::default();
    assert!(!err.status_block(&term).render(&term).is_empty());
}

#[test]
fn invalid_file_reference_anchors_its_frontmatter_excerpt_on_the_property() {
    let source = source_from("---\ninitialize:\n    proxy: nope.md\n---\nbody\n");
    let enriched = proxy_reference_error().enrich_frontmatter(&source, true);
    assert!(
        matches!(enriched, CompositionError::WithFrontmatter { .. }),
        "the wrapper is frontmatter-rooted and must capture an excerpt"
    );
    // The transparent wrapper still reaches the concrete cause underneath.
    assert!(enriched.to_string().contains("nope.md"), "got: {enriched}");
}

#[test]
fn invalid_file_reference_body_does_not_leak_escape_backslashes() {
    // Regression: the source's `Display` quotes the reference (`"nope.md"`),
    // and the href-oriented `escape_prose_path` escapes `"` — which Prose does
    // not treat as special, so the backslash rendered literally as `\"`.
    let term = biscuit_terminal::terminal::Terminal::default();
    let rendered = proxy_reference_error().status_block(&term).render(&term);
    assert!(
        !rendered.contains(r#"\""#),
        "escape backslash leaked into rendered body:\n{rendered}"
    );
    assert!(rendered.contains("nope.md"), "{rendered}");
}

/// Stage a bare-implicit `reference` under `<repo>/prompts/run.md` and wrap the
/// resulting harness resolution failure in the authoring surface, so the block
/// renders against a real (repository-first) candidate plan.
fn implicit_reference_error(reference: &str, repo: &std::path::Path) -> CompositionError {
    let source = repo.join("prompts/run.md");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, "x").unwrap();

    let ctx = crate::harness::HarnessResolutionContext {
        source_path: &source,
        repo_root: Some(repo),
        package_area: None,
    };
    let source_err = crate::harness::resolve_harness_path(reference, &ctx).unwrap_err();

    CompositionError::InvalidFileReference {
        context: Box::new(FileReferenceContext {
            source_path: source,
            event: Some("initialize".to_string()),
            property: "initialize.stack[0].proxy".to_string(),
            reference: reference.to_string(),
            hint: "A `proxy` target must name an existing Markdown document.".to_string(),
        }),
        source: source_err,
    }
}

#[test]
fn invalid_file_reference_block_enumerates_the_plan_only_for_a_multi_candidate_miss() {
    // The ordered "Tried:" list is enumerated only when more than one candidate
    // was probed: an implicit miss (repository + source) shows it; an explicit
    // `./` miss (one candidate) does not, since its single path is already named
    // (spec §D8).
    let term = biscuit_terminal::terminal::Terminal::default();

    let implicit_repo = tempfile::tempdir().unwrap();
    let implicit = implicit_reference_error("absent.md", implicit_repo.path());
    let implicit_rendered = implicit.status_block(&term).render(&term);
    assert!(
        implicit_rendered.contains("Tried:"),
        "an implicit two-candidate miss must enumerate its plan:\n{implicit_rendered}"
    );

    let explicit_repo = tempfile::tempdir().unwrap();
    let explicit = implicit_reference_error("./absent.md", explicit_repo.path());
    let explicit_rendered = explicit.status_block(&term).render(&term);
    assert!(
        !explicit_rendered.contains("Tried:"),
        "an explicit single-candidate miss must not enumerate a one-item plan:\n{explicit_rendered}"
    );
}

// -- Burn-down batch 3: typed sources on the composition edges ---------------
//
// Each site below traded a flattened string for a retained `#[source]`. Two
// contracts are locked per site: the concrete cause is recoverable through
// `Error::source` (spec §L1), and every observable projection stayed exactly
// where it was (spec §D10 — richer detail, never renamed or re-valued detail).

fn markdown_error() -> MarkdownError {
    MarkdownError::AstParse("unbalanced `{{`".to_string())
}

/// The shell-audit catch-all publishes its `HarnessError` unboxed, so
/// `as_diagnostic` — a downcast list over concrete types — can resolve it.
/// Boxing it would publish the box instead and skip the diagnostic entirely.
#[test]
fn pre_flight_shell_audit_failed_publishes_its_harness_error() {
    let err = CompositionError::PreFlightShellAuditFailed {
        source: crate::harness::HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        },
    };

    let published = (&err as &(dyn std::error::Error + 'static))
        .source()
        .expect("the audit failure must publish a cause");
    assert!(
        published
            .downcast_ref::<crate::harness::HarnessError>()
            .is_some(),
        "the chain must publish the concrete HarnessError, not a wrapper"
    );
    assert!(
        crate::diagnostics::as_diagnostic(published).is_some(),
        "the published cause must resolve through the central registry"
    );
}

/// `PreFlightShellAuditFailed` replaced `PreFlightFailed(e.to_string())`, and
/// `PreFlightStateBuildFailed` replaced a `PreFlightFailed(format!(…))`. Both
/// duplicate the prose their prose twin rendered in order to hold a `#[source]`,
/// so lock the twins against drift: every observable projection must agree for
/// the same underlying failure.
#[test]
fn pre_flight_twins_agree() {
    let audit_cause = crate::harness::HarnessError::ShellCommandDenied {
        command: "rm -rf /".to_string(),
    };
    let plain = CompositionError::PreFlightFailed(audit_cause.to_string());
    let sourced = CompositionError::PreFlightShellAuditFailed {
        source: crate::harness::HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        },
    };
    assert_eq!(plain.to_string(), sourced.to_string());
    assert_eq!(plain.code(), sourced.code());
    assert_eq!(plain.category(), sourced.category());
    assert_eq!(plain.disposition(), sourced.disposition());
    assert_eq!(plain.origin(), sourced.origin());
    assert_eq!(plain.detail(), sourced.detail());

    let merge_cause = CtxMergeError::InvalidUserCtx {
        kind: "array".to_string(),
    };
    let plain = CompositionError::PreFlightFailed(format!(
        "lifecycle shell pre-flight: building early-binding state failed: {merge_cause}"
    ));
    let sourced = CompositionError::PreFlightStateBuildFailed {
        source: CtxMergeError::InvalidUserCtx {
            kind: "array".to_string(),
        },
    };
    assert_eq!(plain.to_string(), sourced.to_string());
    assert_eq!(plain.code(), sourced.code());
    assert_eq!(plain.detail(), sourced.detail());
}

/// The state builder's `CtxMergeError` survives as a chain member.
#[test]
fn pre_flight_state_build_failed_publishes_its_merge_error() {
    let err = CompositionError::PreFlightStateBuildFailed {
        source: CtxMergeError::InvalidUserCtx {
            kind: "array".to_string(),
        },
    };
    assert!(
        (&err as &(dyn std::error::Error + 'static))
            .source()
            .and_then(|c| c.downcast_ref::<CtxMergeError>())
            .is_some()
    );
}

/// `LifecycleShellResolution`'s source is `Option` because the same variant is
/// raised by this layer's own late-binding guard, which never calls Darkmatter
/// and so has no typed error to retain. Both shapes must render identically —
/// only the recoverable cause differs.
#[test]
fn lifecycle_shell_resolution_source_is_optional_and_leaves_display_unmoved() {
    let untyped = CompositionError::LifecycleShellResolution {
        source_path: PathBuf::from("run.md"),
        property: "start.stack[0].action.command".to_string(),
        raw: "echo {{ err.msg }}".to_string(),
        message: "late-binding reference `err`".to_string(),
        source: None,
    };
    let typed = CompositionError::LifecycleShellResolution {
        source_path: PathBuf::from("run.md"),
        property: "start.stack[0].action.command".to_string(),
        raw: "echo {{ err.msg }}".to_string(),
        message: "late-binding reference `err`".to_string(),
        source: Some(Box::new(markdown_error())),
    };

    assert_eq!(untyped.to_string(), typed.to_string());
    assert_eq!(untyped.code(), typed.code());
    assert_eq!(untyped.detail(), typed.detail());
    assert!(
        (&untyped as &(dyn std::error::Error + 'static))
            .source()
            .is_none(),
        "the late-binding guard has no typed cause to publish"
    );
    assert!(
        (&typed as &(dyn std::error::Error + 'static))
            .source()
            .is_some(),
        "the DM2 failure must publish its cause"
    );
}

/// The permissions probe now keeps the `io::Error`, so a handler can read the
/// OS reason instead of substring-matching the message. `Display` must not move.
#[test]
fn insufficient_file_permissions_keeps_its_io_error_and_display() {
    let err = CompositionError::InsufficientFilePermissions {
        path: PathBuf::from("/repo/run.md"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    };

    assert_eq!(
        err.to_string(),
        "insufficient file permissions (need read+write): /repo/run.md: denied"
    );
    let io = (&err as &(dyn std::error::Error + 'static))
        .source()
        .and_then(|c| c.downcast_ref::<std::io::Error>())
        .expect("the probe's io::Error must be recoverable");
    assert_eq!(io.kind(), std::io::ErrorKind::PermissionDenied);
}

/// `InlineRewriteFailed` carries the prose its `InvalidInlineResponse(format!(…))`
/// predecessor rendered, so the twins must agree on every projection.
#[test]
fn inline_rewrite_failed_twins_agree_and_publish_the_markdown_error() {
    let plain = CompositionError::InvalidInlineResponse(format!(
        "failed to update last_updated: {}",
        markdown_error()
    ));
    let sourced = CompositionError::InlineRewriteFailed(markdown_error());

    assert_eq!(plain.to_string(), sourced.to_string());
    assert_eq!(plain.code(), sourced.code());
    assert_eq!(plain.detail(), sourced.detail());
    assert!(
        (&sourced as &(dyn std::error::Error + 'static))
            .source()
            .and_then(|c| c.downcast_ref::<MarkdownError>())
            .is_some()
    );
}

// -- Staged for batches 4 and 5 ---------------------------------------------
//
// These variants have no call site yet; their consuming batches add them. The
// contract they must hold on arrival is that they are drop-in replacements for
// the prose variants they supersede, so it is locked here rather than left for
// the batch that discovers it is not true.

fn parse_error() -> ParseError {
    darkmatter::markdown::compose::expression::parse("1 +").unwrap_err()
}

/// `LifecycleWhenExpressionInvalid` shares `LifecycleStackInvalidShape`'s
/// `message`, so batch 4 can retain a `when:` parse error without moving
/// `Display`, the code, the detail payload, or the rendered block.
#[test]
fn lifecycle_when_expression_invalid_twins_with_the_shape_variant() {
    let message = format!("`when` is not a valid expression: {}", parse_error());
    let plain = CompositionError::LifecycleStackInvalidShape {
        source_path: PathBuf::from("run.md"),
        property: "start".to_string(),
        message: message.clone(),
    };
    let sourced = CompositionError::LifecycleWhenExpressionInvalid {
        source_path: PathBuf::from("run.md"),
        property: "start".to_string(),
        message,
        source: parse_error(),
    };

    assert_eq!(plain.to_string(), sourced.to_string());
    assert_eq!(plain.code(), sourced.code());
    assert_eq!(plain.category(), sourced.category());
    assert_eq!(plain.disposition(), sourced.disposition());
    assert_eq!(plain.origin(), sourced.origin());
    assert_eq!(plain.detail(), sourced.detail());

    let term = biscuit_terminal::terminal::Terminal::default();
    assert_eq!(
        plain.status_block(&term).render(&term),
        sourced.status_block(&term).render(&term),
        "the staged variant must render as its shape twin"
    );
    assert!(
        (&sourced as &(dyn std::error::Error + 'static))
            .source()
            .and_then(|c| c.downcast_ref::<ParseError>())
            .is_some()
    );
}

/// `LifecycleActionInvalidLongForm`'s optional `#[source]` retains the typed
/// `ActionExprError` (and its `ParseError` cause) without moving `Display`, the
/// code, the detail payload, or the rendered block. A `None`-sourced twin (the
/// shape failures that never had a lower cause) must be indistinguishable on
/// every observable surface.
#[test]
fn lifecycle_action_invalid_long_form_source_twins_with_the_sourceless_shape() {
    let cause = ActionExprError::Parse(Box::new(parse_error()));
    let message = format!("`x` is not a valid value: {cause}");
    let plain = CompositionError::LifecycleActionInvalidLongForm {
        source_path: PathBuf::from("run.md"),
        property: "start".to_string(),
        action: "set".to_string(),
        message: message.clone(),
        source: None,
    };
    let sourced = CompositionError::LifecycleActionInvalidLongForm {
        source_path: PathBuf::from("run.md"),
        property: "start".to_string(),
        action: "set".to_string(),
        message,
        source: Some(cause),
    };

    assert_eq!(plain.to_string(), sourced.to_string());
    assert_eq!(plain.code(), sourced.code());
    assert_eq!(plain.category(), sourced.category());
    assert_eq!(plain.disposition(), sourced.disposition());
    assert_eq!(plain.origin(), sourced.origin());
    assert_eq!(plain.detail(), sourced.detail());

    let term = biscuit_terminal::terminal::Terminal::default();
    assert_eq!(
        plain.status_block(&term).render(&term),
        sourced.status_block(&term).render(&term),
        "the sourced variant must render as its source-less twin"
    );

    // The concrete cause enum is recoverable directly (it is carried unboxed),
    // and its `Parse` arm carries the typed Darkmatter parse error.
    let action_cause = (&sourced as &(dyn std::error::Error + 'static))
        .source()
        .and_then(|c| c.downcast_ref::<ActionExprError>())
        .expect("the source downcasts to ActionExprError");
    assert!(
        matches!(action_cause, ActionExprError::Parse(_)),
        "the retained cause is the typed parse failure, got: {action_cause:?}"
    );
}

/// `LoopExpressionInvalid` derives its prose from `kind`, `condition`, and the
/// cause's stage. Both stages must reproduce `LoopInvalid`'s text byte for byte.
#[test]
fn loop_expression_invalid_twins_with_loop_invalid_at_both_stages() {
    let parse_twin = CompositionError::LoopInvalid(format!(
        "failed to parse loop.while `1 +`: {}",
        parse_error()
    ));
    let parse_sourced = CompositionError::LoopExpressionInvalid {
        kind: "while".to_string(),
        condition: "1 +".to_string(),
        source: LoopExpressionCause::Parse(parse_error()),
    };
    assert_eq!(parse_twin.to_string(), parse_sourced.to_string());
    assert_eq!(parse_twin.code(), parse_sourced.code());
    assert_eq!(parse_twin.detail(), parse_sourced.detail());

    let eval_cause = ExpressionError::UnknownFunction {
        name: "nope".to_string(),
    };
    let eval_twin = CompositionError::LoopInvalid(format!(
        "failed to evaluate loop.until `nope()`: {eval_cause}"
    ));
    let eval_sourced = CompositionError::LoopExpressionInvalid {
        kind: "until".to_string(),
        condition: "nope()".to_string(),
        source: LoopExpressionCause::Evaluate(Box::new(eval_cause)),
    };
    assert_eq!(eval_twin.to_string(), eval_sourced.to_string());
    assert_eq!(eval_twin.code(), eval_sourced.code());
}

/// `LoopActionExpressionInvalid` derives `InvalidAction`'s two template
/// messages from the cause's stage, so batch 5 can drop it in per stage.
#[test]
fn loop_action_expression_invalid_twins_with_invalid_action_at_both_stages() {
    let parse_twin = CompositionError::InvalidAction {
        iteration: 2,
        action_index: 1,
        total_actions: 3,
        message: format!(
            "invalid template `{{{{x +}}}}` in loop action: {}",
            parse_error()
        ),
    };
    let parse_sourced = CompositionError::LoopActionExpressionInvalid {
        iteration: 2,
        action_index: 1,
        total_actions: 3,
        expression: "x +".to_string(),
        source: LoopExpressionCause::Parse(parse_error()),
    };
    assert_eq!(parse_twin.to_string(), parse_sourced.to_string());
    assert_eq!(parse_twin.code(), parse_sourced.code());
    assert_eq!(parse_twin.detail(), parse_sourced.detail());

    let eval_cause = ExpressionError::UnknownFunction {
        name: "nope".to_string(),
    };
    let eval_twin = CompositionError::InvalidAction {
        iteration: 2,
        action_index: 1,
        total_actions: 3,
        message: format!("failed to evaluate template `{{{{nope()}}}}`: {eval_cause}"),
    };
    let eval_sourced = CompositionError::LoopActionExpressionInvalid {
        iteration: 2,
        action_index: 1,
        total_actions: 3,
        expression: "nope()".to_string(),
        source: LoopExpressionCause::Evaluate(Box::new(eval_cause)),
    };
    assert_eq!(eval_twin.to_string(), eval_sourced.to_string());
}

/// A registered code never projects a top-level `null` detail (spec §D7), and
/// every new variant must satisfy it — including the staged ones, whose
/// consuming batch would otherwise be the one to discover it does not.
#[test]
fn every_batch_3_variant_projects_a_catalog_shaped_detail() {
    let errors: Vec<CompositionError> = vec![
        CompositionError::PreFlightShellAuditFailed {
            source: crate::harness::HarnessError::ShellCommandDenied {
                command: "rm -rf /".to_string(),
            },
        },
        CompositionError::PreFlightStateBuildFailed {
            source: CtxMergeError::InvalidUserCtx {
                kind: "array".to_string(),
            },
        },
        CompositionError::InsufficientFilePermissions {
            path: PathBuf::from("/repo/run.md"),
            source: std::io::Error::other("denied"),
        },
        CompositionError::InlineRewriteFailed(markdown_error()),
        CompositionError::LifecycleShellResolution {
            source_path: PathBuf::from("run.md"),
            property: "start.stack[0].action.command".to_string(),
            raw: "echo hi".to_string(),
            message: "boom".to_string(),
            source: Some(Box::new(markdown_error())),
        },
        CompositionError::LifecycleWhenExpressionInvalid {
            source_path: PathBuf::from("run.md"),
            property: "start".to_string(),
            message: "bad".to_string(),
            source: parse_error(),
        },
        CompositionError::LoopExpressionInvalid {
            kind: "while".to_string(),
            condition: "1 +".to_string(),
            source: LoopExpressionCause::Parse(parse_error()),
        },
        CompositionError::LoopActionExpressionInvalid {
            iteration: 1,
            action_index: 1,
            total_actions: 1,
            expression: "x".to_string(),
            source: LoopExpressionCause::Parse(parse_error()),
        },
    ];

    for err in &errors {
        assert!(
            !err.detail().is_null(),
            "`{}` projects a top-level null detail for code `{}`",
            err,
            err.code()
        );
    }
}

// -- carried diagnostic snapshots (spec §D9) -------------------------------
//
// `LoopIterationFailed` and `SequenceTaskPromptLaunch` are both raised at a
// boundary whose upstream returns an erased `color_eyre::eyre::Report`. The
// prose field records what failed; the snapshot records *which diagnostic*
// failed, so the facets survive a boundary no error value crosses.

/// The projection the CLI sites build from the erased wiring error.
fn carried_snapshot() -> DiagnosticSnapshot {
    DiagnosticSnapshot::from_diagnostic(&CompositionError::FileNotFound("missing.md".to_string()))
}

#[test]
fn loop_iteration_failed_carries_the_wiring_diagnostic_facets() {
    let err = CompositionError::LoopIterationFailed {
        iteration: 1,
        prompt_path: PathBuf::from("plan.md"),
        exit_code: 1,
        reason: "could not resolve provider binary".to_string(),
        exit_reason: None,
        snapshot: Some(Box::new(carried_snapshot())),
    };

    let CompositionError::LoopIterationFailed { snapshot, .. } = &err else {
        panic!("constructed variant");
    };
    let snapshot = snapshot.as_ref().expect("the wiring chain carried one");
    assert_eq!(snapshot.code, "composition.invalid_file_reference");
    assert_eq!(snapshot.category, "composition");
    assert_eq!(snapshot.detail["reference"], "missing.md");
}

#[test]
fn sequence_task_prompt_launch_carries_the_wrapper_diagnostic_facets() {
    let err = CompositionError::SequenceTaskPromptLaunch {
        task: "review".to_string(),
        path: PathBuf::from("tasks/review.md"),
        message: "could not resolve provider binary".to_string(),
        snapshot: Some(Box::new(carried_snapshot())),
    };

    let CompositionError::SequenceTaskPromptLaunch { snapshot, .. } = &err else {
        panic!("constructed variant");
    };
    let snapshot = snapshot.as_ref().expect("the wrapper chain carried one");
    assert_eq!(snapshot.code, "composition.invalid_file_reference");
    assert_eq!(snapshot.detail["reference"], "missing.md");
}

#[test]
fn carrying_a_snapshot_does_not_change_the_rendered_prose() {
    // Spec §D10: the snapshot is additive. Every existing surface — `Display`,
    // and therefore the summary text and status-block body built from it —
    // must read byte-for-byte the same with and without it.
    let without = CompositionError::LoopIterationFailed {
        iteration: 2,
        prompt_path: PathBuf::from("fixes/plan.md"),
        exit_code: 1,
        reason: "step_timeout".to_string(),
        exit_reason: Some("step_timeout".to_string()),
        snapshot: None,
    };
    let with = CompositionError::LoopIterationFailed {
        iteration: 2,
        prompt_path: PathBuf::from("fixes/plan.md"),
        exit_code: 1,
        reason: "step_timeout".to_string(),
        exit_reason: Some("step_timeout".to_string()),
        snapshot: Some(Box::new(carried_snapshot())),
    };
    assert_eq!(with.to_string(), without.to_string());

    let launch_without = CompositionError::SequenceTaskPromptLaunch {
        task: "review".to_string(),
        path: PathBuf::from("tasks/review.md"),
        message: "boom".to_string(),
        snapshot: None,
    };
    let launch_with = CompositionError::SequenceTaskPromptLaunch {
        task: "review".to_string(),
        path: PathBuf::from("tasks/review.md"),
        message: "boom".to_string(),
        snapshot: Some(Box::new(carried_snapshot())),
    };
    assert_eq!(launch_with.to_string(), launch_without.to_string());
}

#[test]
fn carrying_a_snapshot_does_not_move_the_variants_own_identity() {
    // The carried snapshot is data on the record, not a `#[source]`. Selection
    // must still stop at the variant itself, so `err.code` — an authored
    // matching surface — cannot shift under a `when:` clause (spec §D10).
    let with = CompositionError::LoopIterationFailed {
        iteration: 1,
        prompt_path: PathBuf::from("plan.md"),
        exit_code: 1,
        reason: "boom".to_string(),
        exit_reason: None,
        snapshot: Some(Box::new(carried_snapshot())),
    };

    assert_eq!(with.code(), "composition.failed");
    assert_eq!(
        crate::diagnostics::DiagnosticSnapshot::select(&with)
            .map(|selected| selected.code)
            .as_deref(),
        Some("composition.failed"),
    );
}
// ---------------------------------------------------------------------------
// `proxy.with` diagnostics
// ---------------------------------------------------------------------------

/// Render an error's status block to plain, single-spaced text.
///
/// Assertions here are about wording, not layout. The raw block word-wraps to
/// the terminal width, threads its left rule through every wrap point, and
/// carries SGR styling between words — so a multi-word assertion against it
/// would fail on presentation rather than content.
fn render(err: &CompositionError) -> String {
    use biscuit_terminal::prelude::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    let term = Terminal::default();
    let raw = err.status_block(&term).render(&term);

    let mut plain = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        match ch {
            // CSI: `ESC [` params, ending at the first final byte. `[` is
            // itself inside the final-byte range, so it must be consumed
            // before the scan starts.
            '\u{1b}' => match chars.next() {
                Some('[') => {
                    for next in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&next) {
                            break;
                        }
                    }
                }
                // OSC: `ESC ]` … terminated by BEL or ST (`ESC \`).
                Some(']') => {
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\u{2503}' => plain.push(' '),
            _ => plain.push(ch),
        }
    }
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn proxy_with_not_mapping_renders_and_locates_the_with_line() {
    let err = CompositionError::LifecycleProxyWithNotMapping {
        source_path: PathBuf::from("router.md"),
        property: "initialize.stack[0]".to_string(),
        path: "action[0].with".to_string(),
        actual: "array".to_string(),
    };
    assert!(
        matches!(
            err.frontmatter_block_spec(),
            Some(FrontmatterHighlight::Property(ref p))
                if p == "initialize.stack[0].action[0].with"
        ),
        "the excerpt must focus the `with` property"
    );

    let rendered = render(&err);
    assert!(rendered.contains("array"), "names the authored type: {rendered}");
    assert!(
        rendered.contains("initialize.stack[0].action[0].with"),
        "names the full property path: {rendered}"
    );
    assert!(
        rendered.contains("with: {}"),
        "the hint must offer the empty-mapping equivalence: {rendered}"
    );
}

#[test]
fn proxy_with_whole_mapping_renders_named_follow_up_and_explicit_key_hint() {
    let err = CompositionError::LifecycleProxyWithWholeMapping {
        source_path: PathBuf::from("router.md"),
        property: "failure.stack[1]".to_string(),
        path: "action[0].with".to_string(),
        raw: "{{ payload }}".to_string(),
    };
    assert!(
        matches!(
            err.frontmatter_block_spec(),
            Some(FrontmatterHighlight::Property(ref p))
                if p == "failure.stack[1].action[0].with"
        ),
        "the excerpt must focus the diagnostic's property path"
    );

    let rendered = render(&err);
    assert!(rendered.contains("payload"), "echoes the authored span: {rendered}");
    assert!(
        rendered.contains("not supported in this version"),
        "must say this is a named follow-up, not a permanent rule: {rendered}"
    );
    assert!(
        rendered.contains("explicit keys"),
        "the hint must point at explicit-key authoring: {rendered}"
    );
}

#[test]
fn proxy_with_dynamic_key_renders_without_inventing_a_dotted_path() {
    let err = CompositionError::LifecycleProxyWithDynamicKey {
        source_path: PathBuf::from("router.md"),
        property: "initialize.stack[0]".to_string(),
        path: "action[0].with".to_string(),
        key: "{{ dynamic }}".to_string(),
    };
    // The path stops at `with`: appending `.{{ dynamic }}` would name a
    // property that does not exist.
    assert!(
        matches!(
            err.frontmatter_block_spec(),
            Some(FrontmatterHighlight::Property(ref p))
                if p == "initialize.stack[0].action[0].with"
        ),
        "the excerpt must focus the diagnostic's property path"
    );

    let rendered = render(&err);
    assert!(rendered.contains("dynamic"), "names the offending key: {rendered}");
    assert!(
        rendered.contains("never interpolated"),
        "must explain that only values resolve: {rendered}"
    );
    assert!(
        rendered.contains("literal key"),
        "the hint must offer the fix: {rendered}"
    );
}

#[test]
fn proxy_only_parameter_renders_verb_and_key_value_rewrite() {
    let err = CompositionError::LifecycleProxyOnlyParameter {
        source_path: PathBuf::from("router.md"),
        property: "failure.stack[0]".to_string(),
        verb: "retry".to_string(),
        param: "with".to_string(),
    };
    assert!(
        matches!(
            err.frontmatter_block_spec(),
            Some(FrontmatterHighlight::Property(ref p)) if p == "failure.stack[0]"
        ),
        "the excerpt must focus the diagnostic's property path"
    );

    let rendered = render(&err);
    assert!(rendered.contains("retry"), "names the receiving verb: {rendered}");
    assert!(rendered.contains("proxy"), "names the owning verb: {rendered}");
    assert!(
        rendered.contains("action: proxy"),
        "the hint must show the key/value rewrite: {rendered}"
    );
}

#[test]
fn proxy_with_diagnostics_share_the_lifecycle_authoring_code_and_project_facets() {
    // The lifecycle family shares one code; the `property`/`message` facets
    // are what distinguish these for finer handlers, so both must project.
    let cases = [
        CompositionError::LifecycleProxyWithNotMapping {
            source_path: PathBuf::from("router.md"),
            property: "initialize.stack[0]".to_string(),
            path: "action[0].with".to_string(),
            actual: "array".to_string(),
        },
        CompositionError::LifecycleProxyWithWholeMapping {
            source_path: PathBuf::from("router.md"),
            property: "initialize.stack[0]".to_string(),
            path: "action[0].with".to_string(),
            raw: "{{ payload }}".to_string(),
        },
        CompositionError::LifecycleProxyWithDynamicKey {
            source_path: PathBuf::from("router.md"),
            property: "initialize.stack[0]".to_string(),
            path: "action[0].with".to_string(),
            key: "{{ k }}".to_string(),
        },
        CompositionError::LifecycleProxyOnlyParameter {
            source_path: PathBuf::from("router.md"),
            property: "initialize.stack[0]".to_string(),
            verb: "retry".to_string(),
            param: "with".to_string(),
        },
    ];
    for err in &cases {
        assert_eq!(err.code(), "composition.lifecycle_invalid", "for: {err:?}");
        let detail = err.detail();
        assert!(
            detail.get("property").and_then(|v| v.as_str()).is_some(),
            "`property` facet must project for: {err:?}"
        );
        assert!(
            detail.get("message").and_then(|v| v.as_str()).is_some(),
            "`message` facet must project for: {err:?}"
        );
    }
}
