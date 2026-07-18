//! Unit tests for the per-attempt launch rebuild.

use super::*;
use claudine::composition::{EffectiveSelectionHints, ModelHint};

/// The argv the fixture's invocation recorded. Distinctive so a test can tell
/// the verbatim shortcut from a replay at a glance.
const RECORDED_ARGV: &str = "recorded-invocation-argv";

/// The launch-plan inputs the fixture invocation recorded: Goose,
/// non-interactive, no `--yolo`, no model, no body tags.
///
/// `mcp: None` deliberately — these are unit tests, and a replay that reached
/// [`launch_plan::McpRebuildInputs`] would load the host's real MCP catalog and
/// make the result depend on the developer's machine. The MCP *facet* still
/// participates (it is part of `DocumentLaunchFacets`), so tag movement is still
/// observable; only the injector call is out of scope here.
fn plan_inputs() -> launch_plan::LaunchPlanInputs {
    launch_plan::LaunchPlanInputs {
        provider_args_tail: Vec::new(),
        output_format_args: Vec::new(),
        system_prompt_args: Vec::new(),
        system_prompt_opencode_config: None,
        system_prompt: None,
        system_prompt_cwd: PathBuf::new(),
        system_prompt_scoped_tmp: PathBuf::new(),
        sandbox_args: Vec::new(),
        has_model_env: false,
        mcp: None,
        opencode_config_base: None,
        codex_last_message_path: PathBuf::from("/tmp/claudine-test-last-message.txt"),
        invocation: launch_plan::RecordedLaunch {
            facets: launch_plan::DocumentLaunchFacets {
                provider: Provider::Goose,
                non_interactive: true,
                yolo_requested: false,
                is_inline: false,
                model: None,
                mcp_body_tags: Vec::new(),
            },
            args: vec![RECORDED_ARGV.to_string()],
            env_overlay: vec![("YOLO".into(), "false".into())],
            structured_codex: false,
        },
        replay_supported: true,
    }
}

/// The invocation intent an unchanged run resolves against: Goose,
/// non-interactive, nothing explicit, MCP in play so a body tag counts.
fn intent() -> LaunchRebuildIntent {
    LaunchRebuildIntent {
        explicit_provider: None,
        fallback_provider: Provider::Goose,
        fallback_binary: PathBuf::from("/usr/bin/goose"),
        installed_snapshot: None,
        default_non_interactive: true,
        cli_yolo: false,
        is_inline: false,
        mcp_enabled: true,
        launch_plan_inputs: plan_inputs(),
    }
}

/// Build a materialized prompt carrying a target's `model:` hint plus a
/// matching effective-frontmatter object, as canonical preparation would.
fn target_with_model(model: Option<&str>) -> MaterializedHarnessPrompt {
    let (selection_hints, frontmatter) = match model {
        Some(model) => (
            EffectiveSelectionHints {
                model: Some(ModelHint::Single(model.to_string())),
                ..EffectiveSelectionHints::default()
            },
            serde_json::json!({ "model": model }),
        ),
        None => (
            EffectiveSelectionHints::default(),
            serde_json::json!({ "title": "no model" }),
        ),
    };
    document(selection_hints, frontmatter, "body")
}

fn document(
    selection_hints: EffectiveSelectionHints,
    frontmatter: serde_json::Value,
    prompt: &str,
) -> MaterializedHarnessPrompt {
    MaterializedHarnessPrompt {
        live_frontmatter: MaterializedHarnessPrompt::live_cell_from(&frontmatter),
        frontmatter,
        prompt: prompt.to_string(),
        env_overrides: Vec::new(),
        selection_hints,
        inline_closure_plan: None,
        lifecycle: None,
    }
}

fn with_hints(hints: EffectiveSelectionHints) -> MaterializedHarnessPrompt {
    document(hints, serde_json::json!({ "title": "t" }), "body")
}

fn value_of<'a>(overrides: &'a [(String, String)], key: &str) -> Option<&'a str> {
    overrides
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// A target that pins its own `model:` resolves it into the rebuilt launch
/// identity — the `AGENT`/`MODEL`/`YOLO` env AND the early-binding context
/// that lifecycle `env.*` reads. This is the launch facet the L2 pinned-model
/// equivalence test observes through the target's `success` stack.
#[test]
fn rebuild_projects_target_model_into_env_and_context() {
    // A namespaced local-runner id is catalog-valid by construction, so this
    // is independent of the host's live model listings.
    let target = target_with_model(Some("llamacpp/probe-model-x"));
    let rebuild = rebuild_target_launch(&intent(), None, None, Path::new("."), &target).unwrap();

    assert_eq!(value_of(&rebuild.env_overrides, "AGENT"), Some("goose"));
    assert_eq!(
        value_of(&rebuild.env_overrides, "MODEL"),
        Some("llamacpp/probe-model-x"),
        "the target's own pinned model must reach the launch env",
    );
    assert_eq!(value_of(&rebuild.env_overrides, "YOLO"), Some("false"));
    assert_eq!(
        rebuild.prepared_context.env().get("MODEL").map(String::as_str),
        Some("llamacpp/probe-model-x"),
        "the lifecycle early-binding context must carry the target's model",
    );
}

/// Explicit `--model` is immutable invocation intent: it stays authoritative
/// over the target's frontmatter `model:` exactly as it would for a direct
/// invocation under the same CLI arguments.
#[test]
fn rebuild_keeps_explicit_cli_model_authoritative() {
    let target = target_with_model(Some("llamacpp/probe-model-x"));
    let rebuild = rebuild_target_launch(
        &intent(),
        Some("llamacpp/cli-pinned"),
        None,
        Path::new("."),
        &target,
    )
    .unwrap();

    assert_eq!(
        value_of(&rebuild.env_overrides, "MODEL"),
        Some("llamacpp/cli-pinned"),
        "explicit --model must win over the target frontmatter model",
    );
}

/// A target with no `model:` produces no `MODEL` overlay (matching
/// `install_agent_env_for_composition`), so the child env falls through to
/// the provider default rather than a stale router model.
#[test]
fn rebuild_omits_model_when_target_pins_none() {
    let target = target_with_model(None);
    let rebuild = rebuild_target_launch(&intent(), None, None, Path::new("."), &target).unwrap();

    assert_eq!(value_of(&rebuild.env_overrides, "AGENT"), Some("goose"));
    assert_eq!(
        value_of(&rebuild.env_overrides, "MODEL"),
        None,
        "no frontmatter model means no MODEL overlay",
    );
}

/// The property the whole R8 comparison rests on: the rebuild is a pure
/// function of (document, intent), so an unchanged document cannot produce a
/// false refusal no matter how many attempts run.
#[test]
fn an_unchanged_document_rebuilds_to_an_identical_identity() {
    let doc = target_with_model(Some("llamacpp/probe-model-x"));
    let first = rebuild_launch_identity(&intent(), None, None, &doc, None).unwrap();
    let second = rebuild_launch_identity(&intent(), None, None, &doc, None).unwrap();

    assert_eq!(first.provider, second.provider);
    assert_eq!(first.binary_path, second.binary_path);
    assert_eq!(first.non_interactive, second.non_interactive);
    assert_eq!(first.yolo, second.yolo);
    assert_eq!(first.use_structured, second.use_structured);
    assert_eq!(first.structured_codex, second.structured_codex);
    assert_eq!(first.mcp_tags, second.mcp_tags);
    assert_eq!(first.env_overrides, second.env_overrides);
}

/// The single-bundle seam's safety property.
///
/// The attempt used to rebuild twice: once without a source path, whose
/// `env_overrides` overlaid the child environment, and once *with* one, whose
/// result fed the compatibility key. Collapsing them into the one call that
/// passes the source path is only sound if the source path moves nothing but
/// [`RebuiltLaunchIdentity::mcp_tags`] — otherwise the surviving call would
/// hand the child a different environment than the deleted one did.
#[test]
fn the_source_path_moves_only_the_mcp_tags_of_a_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("doc.md");
    std::fs::write(&source, "---\ntitle: t\n---\nwork on #calendar today\n").unwrap();
    let doc = target_with_model(Some("llamacpp/probe-model-x"));

    let env_only = rebuild_launch_identity(&intent(), None, None, &doc, None).unwrap();
    let with_source =
        rebuild_launch_identity(&intent(), None, None, &doc, Some(source.as_path())).unwrap();

    assert_eq!(
        env_only.env_overrides, with_source.env_overrides,
        "the one surviving rebuild must hand the child exactly the environment \
         the deleted env-only rebuild did",
    );
    assert_eq!(env_only.provider, with_source.provider);
    assert_eq!(env_only.binary_path, with_source.binary_path);
    assert_eq!(env_only.non_interactive, with_source.non_interactive);
    assert_eq!(env_only.yolo, with_source.yolo);
    assert_eq!(env_only.use_structured, with_source.use_structured);
    assert_eq!(env_only.structured_codex, with_source.structured_codex);
    assert!(
        env_only.mcp_tags.is_empty() && with_source.mcp_tags == vec!["calendar".to_string()],
        "the source path is what lexes the body tags, and is the only facet it moves",
    );
}

/// Frontmatter `agent:` moves the provider — and with it the binary and the
/// resume protocol the compatibility key reads.
#[test]
fn frontmatter_agent_moves_the_provider_and_its_binary() {
    let base = rebuild_launch_identity(&intent(), None, None, &with_hints(
        EffectiveSelectionHints::default(),
    ), None).unwrap();
    let switched = rebuild_launch_identity(&intent(), None, None, &with_hints(
        EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Claude)),
            ..EffectiveSelectionHints::default()
        },
    ), None).unwrap();

    assert_eq!(base.provider, Provider::Goose);
    assert_eq!(switched.provider, Provider::Claude);
    assert_ne!(
        base.binary_path, switched.binary_path,
        "a provider switch must move the binary the key records",
    );
}

/// Explicit CLI provider selection is immutable invocation intent: a
/// frontmatter `agent:` cannot move it, so it can never refuse a resume.
#[test]
fn an_explicit_cli_provider_pins_the_rebuilt_provider() {
    let pinned = LaunchRebuildIntent {
        explicit_provider: Some(Provider::Goose),
        ..intent()
    };
    let rebuilt = rebuild_launch_identity(&pinned, None, None, &with_hints(
        EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Claude)),
            ..EffectiveSelectionHints::default()
        },
    ), None).unwrap();

    assert_eq!(rebuilt.provider, Provider::Goose);
}

/// Frontmatter `interactive:` moves the session mode, and the
/// structured-output mode that mode implies moves with it.
#[test]
fn frontmatter_interactive_moves_the_mode_and_structured_output() {
    // OpenCode has a stream protocol, so structured streaming is on for it in
    // non-interactive mode; Goose has none and would hide the flip.
    let streaming = LaunchRebuildIntent {
        fallback_provider: Provider::OpenCode,
        ..intent()
    };
    let base = rebuild_launch_identity(&streaming, None, None, &with_hints(
        EffectiveSelectionHints::default(),
    ), None).unwrap();
    let interactive = rebuild_launch_identity(&streaming, None, None, &with_hints(
        EffectiveSelectionHints {
            interactive: Some(true),
            ..EffectiveSelectionHints::default()
        },
    ), None).unwrap();

    assert!(base.non_interactive);
    assert!(!interactive.non_interactive);
    assert!(
        base.use_structured && !interactive.use_structured,
        "structured streaming is non-interactive only, so the mode flip must move it",
    );
}

/// `--yolo` records the permission mode the provider actually achieves, not
/// the one that was asked for: a provider whose bypass is non-interactive
/// only declines in an interactive session, which is a real mode change.
#[test]
fn the_permission_mode_records_what_yolo_achieved_not_what_was_asked() {
    // OpenCode's bypass is non-interactive only; Goose sets an env var that
    // applies in either mode and would hide the difference.
    let requested = LaunchRebuildIntent {
        cli_yolo: true,
        fallback_provider: Provider::OpenCode,
        ..intent()
    };
    let non_interactive = rebuild_launch_identity(&requested, None, None, &with_hints(
        EffectiveSelectionHints::default(),
    ), None).unwrap();
    let interactive = rebuild_launch_identity(&requested, None, None, &with_hints(
        EffectiveSelectionHints {
            interactive: Some(true),
            ..EffectiveSelectionHints::default()
        },
    ), None).unwrap();

    assert!(non_interactive.yolo);
    assert!(
        !interactive.yolo,
        "OpenCode bypass is non-interactive only, so an interactive refresh drops it",
    );
}

/// MCP `#tag`s lexed from the document **on disk** participate in the
/// identity — but only when MCP is in play for the invocation at all.
///
/// Reading the source rather than `document.prompt` is what makes the signal
/// survive a resume, whose prompt field carries the follow-up message.
#[test]
fn body_mcp_tags_are_lexed_from_disk_and_only_when_mcp_is_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("doc.md");
    std::fs::write(&source, "---\ntitle: t\n---\nwork on #calendar today\n").unwrap();
    // The prompt field deliberately carries a resume follow-up, proving the
    // tags do not come from it.
    let doc = document(
        EffectiveSelectionHints::default(),
        serde_json::json!({ "title": "t" }),
        "please finish the resumed work",
    );

    let enabled =
        rebuild_launch_identity(&intent(), None, None, &doc, Some(source.as_path())).unwrap();
    assert_eq!(enabled.mcp_tags, vec!["calendar".to_string()]);

    let off = LaunchRebuildIntent {
        mcp_enabled: false,
        ..intent()
    };
    assert!(
        rebuild_launch_identity(&off, None, None, &doc, Some(source.as_path()))
            .unwrap()
            .mcp_tags
            .is_empty(),
        "with MCP off no body tag participates in the launch, so none may refuse a resume",
    );
}
