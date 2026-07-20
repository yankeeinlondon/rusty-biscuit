//! Unit tests for the per-attempt launch rebuild.

use super::*;
use claudine::composition::{EffectiveSelectionHints, ModelHint};

/// The argv the fixture's invocation recorded. Distinctive so a test can tell
/// the verbatim shortcut from a replay at a glance.
const RECORDED_ARGV: &str = "recorded-invocation-argv";

/// The model the OpenCode fixtures below launch with.
///
/// OpenCode is the only provider whose non-interactive launch *requires* a
/// resolved model, and with none supplied the rebuild falls through to the
/// host's `~/.config/opencode/config.json`. That made these tests pass on a
/// developer machine that happens to configure OpenCode and fail with "No model
/// specified" on one that does not (review-10 finding 5).
///
/// Delivered through `rebuild_launch_identity`'s `cli_model` seam rather than a
/// `model:` frontmatter hint: an explicit CLI model resolves ahead of both the
/// process environment and the model catalog, so this fixture depends on
/// neither — and cannot rot when a real model name leaves the catalog. The
/// value is deliberately not a real model; nothing here asserts on it.
const FIXTURE_MODEL: &str = "fixture/opencode-model";

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
        output_format: None,
        system_prompt_args: Vec::new(),
        system_prompt_opencode_config: None,
        system_prompt: None,
        system_prompt_cwd: PathBuf::new(),
        system_prompt_scoped_tmp: PathBuf::new(),
        sandbox_requested: false,
        has_model_env: false,
        mcp: None,
        opencode_config_base: None,
        codex_last_message_path: PathBuf::from("/tmp/claudine-test-last-message.txt"),
        // The provider-shaped keys the fixture invocation wrote, none of which
        // existed beforehand — so a rebuild that stops writing one clears it.
        credential_policy: launch_plan::CredentialPolicyInputs::default(),
        provider_env_baseline: HashMap::from([
            (std::ffi::OsString::from("MODEL"), None),
            (std::ffi::OsString::from("OPENCODE_CONFIG_CONTENT"), None),
            (
                std::ffi::OsString::from("HOME"),
                Some(std::ffi::OsString::from("/home/real")),
            ),
        ]),
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
        fallback_provider_reason: ProviderResolutionReason::FavoriteAgent,
        dispatch_context: invocation_dispatch_context(),
        launch_plan_inputs: plan_inputs(),
    }
}

/// The composition-shaped dispatch context the fixture invocation published:
/// the three selection entries a moved facet must recompute, plus an
/// invocation-fixed entry a recompute must preserve.
fn invocation_dispatch_context() -> HashMap<String, serde_json::Value> {
    let mut context = HashMap::new();
    context.insert(
        "composition_file_ref".to_string(),
        serde_json::Value::String("doc.md".to_string()),
    );
    context.insert(
        "provider_selection_reason".to_string(),
        serde_json::Value::String("FavoriteAgent".to_string()),
    );
    context.insert(
        "resolved_model".to_string(),
        serde_json::Value::String(String::new()),
    );
    context.insert(
        "model_selection_reason".to_string(),
        serde_json::Value::String("ProviderDefault".to_string()),
    );
    context
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

/// The environment an attempt's child actually receives: the invocation's base,
/// then the rebuilt plan's patch, then the document's `AGENT`/`MODEL`/`YOLO`
/// triple — the same order [`super::super::super::build_harness_launch`] applies
/// them in.
///
/// Asserting on this rather than on the overlay vector is the point: an overlay
/// that merely *omits* a key looks identical to one that clears it, and the
/// difference is exactly what reaches the provider process.
fn effective_child_env(
    base: &[(&str, &str)],
    rebuilt: &RebuiltLaunchIdentity,
) -> HashMap<std::ffi::OsString, std::ffi::OsString> {
    let mut env: HashMap<std::ffi::OsString, std::ffi::OsString> = base
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect();
    for change in &rebuilt.launch_env {
        match change {
            launch_plan::EnvChange::Set(key, value) => {
                env.insert(key.clone(), value.clone());
            }
            launch_plan::EnvChange::Remove(key) => {
                env.remove(key);
            }
        }
    }
    for (key, value) in &rebuilt.env_overrides {
        env.insert(key.clone().into(), value.clone().into());
    }
    env
}

/// The fixture intent, opened with a pinned model so a refresh can drop it.
fn intent_with_opening_model(model: &str) -> LaunchRebuildIntent {
    let mut intent = intent();
    intent.launch_plan_inputs.invocation.facets.model = Some(model.to_string());
    intent
}

/// Review-9 finding 2 — a refresh that drops `model:` clears `MODEL` from the
/// **child environment**, not merely from the overlay vector.
///
/// The overlay is applied over the invocation's base environment, which already
/// carries the opening document's `MODEL`. Omitting the key therefore left the
/// stale value in place; only a removal actually reaches the provider process
/// without it.
#[test]
fn rebuild_omits_model_when_target_pins_none() {
    let base = [("MODEL", "llamacpp/opening-model"), ("PATH", "/usr/bin")];
    let rebuilt = rebuild_launch_identity(
        &intent_with_opening_model("llamacpp/opening-model"),
        None,
        None,
        &target_with_model(None),
        None,
    )
    .unwrap();

    assert_eq!(value_of(&rebuilt.env_overrides, "AGENT"), Some("goose"));
    assert_eq!(
        value_of(&rebuilt.env_overrides, "MODEL"),
        None,
        "no frontmatter model means no MODEL overlay",
    );

    let env = effective_child_env(&base, &rebuilt);
    assert!(
        !env.contains_key(std::ffi::OsStr::new("MODEL")),
        "the child must not inherit the opening document's model; got {env:?}",
    );
    assert_eq!(
        env.get(std::ffi::OsStr::new("PATH")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("/usr/bin")),
        "the patch must touch only the provider-owned half of the environment",
    );
}

/// Review-9 finding 2 — the same rule for provider-specific base values. A
/// retry that leaves OpenCode must not hand the new provider OpenCode's inline
/// config, and one that leaves a shadow-HOME provider must get its real home
/// back rather than the previous provider's shadow.
#[test]
fn a_provider_switch_clears_the_opening_providers_environment() {
    let base = [
        ("OPENCODE_CONFIG_CONTENT", "{\"opening\":true}"),
        ("HOME", "/shadow/opencode"),
        ("PATH", "/usr/bin"),
    ];
    let rebuilt = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Gemini)),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();

    let env = effective_child_env(&base, &rebuilt);
    assert!(
        !env.contains_key(std::ffi::OsStr::new("OPENCODE_CONFIG_CONTENT")),
        "OpenCode's inline config must not reach a Gemini retry; got {env:?}",
    );
    assert_eq!(
        env.get(std::ffi::OsStr::new("HOME")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("/home/real")),
        "a shadow HOME the rebuild does not re-materialize must fall back to the \
         real one; got {env:?}",
    );
}

/// Non-secret stand-in for a credential Codex admits and Goose does not.
const FIXTURE_OPENAI_KEY: &str = "fixture-openai-not-a-real-key";

/// The fixture intent carrying an ambient credential the opening Goose profile
/// would have stripped.
fn intent_with_ambient_credential() -> LaunchRebuildIntent {
    let mut intent = intent();
    intent.launch_plan_inputs.credential_policy = launch_plan::CredentialPolicyInputs {
        ambient: HashMap::from([(
            std::ffi::OsString::from("OPENAI_API_KEY"),
            std::ffi::OsString::from(FIXTURE_OPENAI_KEY),
        )]),
        explicit_include: std::collections::HashSet::new(),
    };
    intent
}

/// Review-10 finding 1 — a Goose → Codex switch reaches the child with the
/// ambient credential Codex admits, which Goose's allow-list had removed from
/// the base environment.
#[test]
fn a_provider_switch_readmits_a_credential_the_target_admits() {
    let base = [("PATH", "/usr/bin")];
    let rebuilt = rebuild_launch_identity(
        &intent_with_ambient_credential(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Codex)),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();

    let env = effective_child_env(&base, &rebuilt);
    assert_eq!(
        env.get(std::ffi::OsStr::new("OPENAI_API_KEY"))
            .map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new(FIXTURE_OPENAI_KEY)),
        "Codex admits OPENAI_API_KEY, so a switch onto it must recover the \
         ambient value; got {env:?}",
    );
}

/// Review-10 finding 1, leak direction — a Codex → Goose switch removes an
/// ambient credential the base environment carries but Goose never admits.
#[test]
fn a_provider_switch_strips_a_credential_the_target_rejects() {
    let base = [("OPENAI_API_KEY", FIXTURE_OPENAI_KEY), ("PATH", "/usr/bin")];
    let mut intent = intent_with_ambient_credential();
    intent.launch_plan_inputs.invocation.facets.provider = Provider::Codex;
    let rebuilt = rebuild_launch_identity(
        &intent,
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Goose)),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();

    let env = effective_child_env(&base, &rebuilt);
    assert!(
        !env.contains_key(std::ffi::OsStr::new("OPENAI_API_KEY")),
        "Goose admits no credential keys, so Codex's ambient key must not reach \
         it; got {env:?}",
    );
    assert_eq!(
        env.get(std::ffi::OsStr::new("PATH")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("/usr/bin")),
        "sanitation is scoped to sensitive keys; got {env:?}",
    );
}

/// The composed system-prompt body the fixture invocation captured.
const COMPOSED_SYSTEM_PROMPT: &str = "stay terse";

/// The fixture intent carrying a resolved system prompt, plus the scoped temp
/// directory a provider-move re-delivery writes its file into.
///
/// The returned [`tempfile::TempDir`] must outlive the rebuild: it is the
/// directory, not the artifact, and dropping it early would remove the file for
/// a reason unrelated to what the test measures.
fn intent_with_system_prompt(
    mode: claudine::system_prompt::SystemPromptMode,
) -> (LaunchRebuildIntent, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let discovered = dir.path().join("system-prompt.md");
    std::fs::write(&discovered, COMPOSED_SYSTEM_PROMPT).unwrap();

    let mut intent = intent();
    intent.launch_plan_inputs.system_prompt = Some(
        claudine::system_prompt::ResolvedSystemPrompt::Ready(
            claudine::system_prompt::PreparedSystemPrompt {
                mode,
                source: claudine::system_prompt::SystemPromptSource::StandardDiscovered {
                    path: discovered,
                    scope: claudine::system_prompt::StandardPromptScope::Repo,
                },
                raw_text: COMPOSED_SYSTEM_PROMPT.to_string(),
                composed_markdown: COMPOSED_SYSTEM_PROMPT.to_string(),
                non_interactive_appendix: None,
            },
        ),
    );
    intent.launch_plan_inputs.system_prompt_cwd = dir.path().to_path_buf();
    intent.launch_plan_inputs.system_prompt_scoped_tmp = dir.path().to_path_buf();
    (intent, dir)
}

/// Every filesystem path the rebuilt bundle hands the child: argv tokens and
/// environment values alike, since file-backed delivery uses both.
fn referenced_existing_files(rebuilt: &RebuiltLaunchIdentity) -> Vec<PathBuf> {
    let from_args = rebuilt.args.iter().cloned();
    let from_env = rebuilt.launch_env.iter().filter_map(|change| match change {
        launch_plan::EnvChange::Set(_, value) => Some(value.to_string_lossy().into_owned()),
        launch_plan::EnvChange::Remove(_) => None,
    });
    from_args
        .chain(from_env)
        // `-c model_instructions_file=/path` names its path after an `=`.
        .map(|token| match token.split_once('=') {
            Some((_, tail)) => tail.to_string(),
            None => token,
        })
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .collect()
}

/// Review-10 finding 4 — a provider switch's system-prompt temp file survives
/// the `LaunchPlan` that wrote it.
///
/// File-backed delivery puts a *path* in the argv (Codex's replacement config
/// file) or in the child environment (Gemini's `GEMINI_SYSTEM_MD`), never the
/// content. The plan that created the `NamedTempFile` is consumed inside
/// `rebuild_launch_identity`, so unless the rebuilt bundle takes ownership the
/// file is unlinked when the builder returns — before `build_harness_launch`
/// spawns anything, and the provider then reads nothing.
///
/// The final assertion is the other half of the contract: the file is bound to
/// the bundle's lifetime, so dropping the bundle (which the attempt does only
/// after the child has exited) still cleans it up.
#[test]
fn a_provider_switch_keeps_its_system_prompt_file_alive_past_the_plan() {
    for (provider, mode) in [
        (
            Provider::Gemini,
            claudine::system_prompt::SystemPromptMode::Append,
        ),
        (
            Provider::Codex,
            claudine::system_prompt::SystemPromptMode::Replace,
        ),
    ] {
        let (intent, _scoped_tmp) = intent_with_system_prompt(mode);
        let rebuilt = rebuild_launch_identity(
            &intent,
            None,
            None,
            &with_hints(EffectiveSelectionHints {
                agent: Some(AgentHint::Single(provider)),
                ..EffectiveSelectionHints::default()
            }),
            None,
        )
        .unwrap();

        let referenced = referenced_existing_files(&rebuilt);
        let delivered: Vec<String> = referenced
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect();
        // `contains`, not equality: Gemini's append delivery merges the host's
        // own `~/.gemini/GEMINI.md` ahead of the composed body.
        assert!(
            delivered
                .iter()
                .any(|body| body.contains(COMPOSED_SYSTEM_PROMPT)),
            "the {provider} launch must still be able to read its system-prompt \
             file after the plan was consumed; argv {:?}, env {:?}",
            rebuilt.args,
            rebuilt.launch_env,
        );

        let paths = referenced;
        drop(rebuilt);
        assert!(
            paths.iter().all(|path| !path.exists()),
            "the artifacts are bound to the bundle, so dropping it must clean \
             them up; {paths:?} survived",
        );
    }
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
    assert_eq!(first.codex_output.is_some(), second.codex_output.is_some());
    assert_eq!(first.mcp_tags, second.mcp_tags);
    assert_eq!(first.env_overrides, second.env_overrides);
    assert_eq!(first.dispatch_context, second.dispatch_context);
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
    assert_eq!(
        env_only.codex_output.is_some(),
        with_source.codex_output.is_some()
    );
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
    let base = rebuild_launch_identity(&streaming, Some(FIXTURE_MODEL), None, &with_hints(
        EffectiveSelectionHints::default(),
    ), None).unwrap();
    let interactive = rebuild_launch_identity(&streaming, Some(FIXTURE_MODEL), None, &with_hints(
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

/// Review-10 finding 2 — the two interactivity markers follow the refreshed
/// mode into the child, in both directions.
///
/// The invocation stamped them from the *opening* mode, and nothing else in the
/// patch covers them, so an additive overlay left a retry that moved
/// `interactive:` shipping refreshed argv beside the opening mode's
/// `INTERACTIVE` (which wrapped providers read) and `CLAUDINE_INTERACTIVE`
/// (which gates hook behavior). Asserted on the effective child environment,
/// where that difference is observable.
#[test]
fn a_mode_refresh_moves_both_interactivity_markers_in_the_child_env() {
    // The base the invocation stamped for its own non-interactive opening.
    let opened_non_interactive = [
        ("INTERACTIVE", "false"),
        ("CLAUDINE_INTERACTIVE", "0"),
        ("PATH", "/usr/bin"),
    ];
    let to_interactive = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            interactive: Some(true),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();
    let env = effective_child_env(&opened_non_interactive, &to_interactive);
    assert_eq!(
        env.get(std::ffi::OsStr::new("INTERACTIVE")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("true")),
        "an interactive refresh must not ship the opening mode's INTERACTIVE; got {env:?}",
    );
    assert_eq!(
        env.get(std::ffi::OsStr::new("CLAUDINE_INTERACTIVE")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("1")),
        "the hook gate must move with the mode; got {env:?}",
    );

    // The reverse: an invocation that opened interactive, refreshed to
    // non-interactive by frontmatter.
    let opened_interactive = [
        ("INTERACTIVE", "true"),
        ("CLAUDINE_INTERACTIVE", "1"),
        ("PATH", "/usr/bin"),
    ];
    let interactive_intent = LaunchRebuildIntent {
        default_non_interactive: false,
        launch_plan_inputs: launch_plan::LaunchPlanInputs {
            invocation: launch_plan::RecordedLaunch {
                facets: launch_plan::DocumentLaunchFacets {
                    non_interactive: false,
                    ..plan_inputs().invocation.facets
                },
                ..plan_inputs().invocation
            },
            ..plan_inputs()
        },
        ..intent()
    };
    let to_non_interactive = rebuild_launch_identity(
        &interactive_intent,
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            interactive: Some(false),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();
    let env = effective_child_env(&opened_interactive, &to_non_interactive);
    assert_eq!(
        env.get(std::ffi::OsStr::new("INTERACTIVE")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("false")),
        "a non-interactive refresh must not ship the opening mode's INTERACTIVE; got {env:?}",
    );
    assert_eq!(
        env.get(std::ffi::OsStr::new("CLAUDINE_INTERACTIVE")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("0")),
        "the hook gate must move with the mode; got {env:?}",
    );
}

/// An unchanged document takes the verbatim shortcut, whose recorded overlay
/// carries no interactivity keys at all. The markers are restated absolutely
/// rather than only when the mode moved, so the bundle still agrees with the
/// mode it launches under.
#[test]
fn an_unchanged_document_still_states_the_interactivity_markers() {
    let rebuilt =
        rebuild_launch_identity(&intent(), None, None, &with_hints(
            EffectiveSelectionHints::default(),
        ), None)
        .unwrap();

    // A base that disagrees with the resolved mode: only an absolute restatement
    // corrects it.
    let env = effective_child_env(&[("INTERACTIVE", "true"), ("CLAUDINE_INTERACTIVE", "1")], &rebuilt);
    assert!(rebuilt.non_interactive);
    assert_eq!(
        env.get(std::ffi::OsStr::new("INTERACTIVE")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("false")),
        "the markers must agree with the mode this bundle launches under; got {env:?}",
    );
    assert_eq!(
        env.get(std::ffi::OsStr::new("CLAUDINE_INTERACTIVE")).map(|v| v.as_os_str()),
        Some(std::ffi::OsStr::new("0")),
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
    let non_interactive = rebuild_launch_identity(&requested, Some(FIXTURE_MODEL), None, &with_hints(
        EffectiveSelectionHints::default(),
    ), None).unwrap();
    let interactive = rebuild_launch_identity(&requested, Some(FIXTURE_MODEL), None, &with_hints(
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

/// Review-9 finding 1: the stdout/stderr noise policy and the
/// structured-stderr suppression policy come from the REBUILT profile, so a
/// provider-switch retry filters the new provider's output with the new
/// provider's prefixes — not the opening provider's.
#[test]
fn noise_and_suppression_policy_come_from_the_rebuilt_profile() {
    let base = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints::default()),
        None,
    )
    .unwrap();
    let switched = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Gemini)),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();

    let gemini = profile_for_provider(Provider::Gemini).unwrap();
    assert_eq!(base.stderr_noise, profile_for_provider(Provider::Goose).unwrap().stderr_noise_prefixes());
    assert!(!base.suppress_stderr_on_success);
    assert_eq!(
        switched.stderr_noise,
        gemini.stderr_noise_prefixes(),
        "the rebuilt Gemini attempt must filter with Gemini's prefixes",
    );
    assert_eq!(switched.stdout_noise, gemini.stdout_noise_prefixes());
    assert!(
        switched.suppress_stderr_on_success,
        "Gemini's suppression policy must ride the rebuilt bundle",
    );
}

/// An interactive rebuild clears both noise filters regardless of the
/// profile's prefixes, mirroring the invocation's own gate (a non-empty
/// stderr filter would pipe an interactive TUI's stderr and hang it on
/// SIGTTIN).
#[test]
fn an_interactive_rebuild_clears_the_noise_filters() {
    let interactive = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Gemini)),
            interactive: Some(true),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();

    assert!(interactive.stdout_noise.is_empty());
    assert!(interactive.stderr_noise.is_empty());
}

/// Review-9 finding 1: the Codex `--output-last-message` artifact rides the
/// rebuilt bundle as one coherent value — present iff the rebuilt plan
/// captures through it, wrapping the same sink path the rebuilt argv names.
#[test]
fn codex_artifact_present_iff_the_rebuilt_plan_captures_through_it() {
    let base = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints::default()),
        None,
    )
    .unwrap();
    assert!(
        base.codex_output.is_none(),
        "a non-Codex rebuild carries no artifact",
    );

    let into_codex = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Codex)),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();
    let artifact = into_codex
        .codex_output
        .as_ref()
        .expect("a non-interactive Codex rebuild captures through the artifact");
    let recorded = intent().launch_plan_inputs.codex_last_message_path;
    assert_eq!(
        artifact.last_message_path, recorded,
        "the artifact must wrap the recorded sink path",
    );
    let flag_position = into_codex
        .args
        .iter()
        .position(|arg| arg == "--output-last-message")
        .expect("the rebuilt argv names the sink");
    assert_eq!(
        into_codex.args.get(flag_position + 1).map(String::as_str),
        recorded.to_str(),
        "artifact and argv must name the same path",
    );

    let interactive_codex = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Codex)),
            interactive: Some(true),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();
    assert!(
        interactive_codex.codex_output.is_none(),
        "an interactive non-inline Codex rebuild does not capture through the file",
    );
}

/// Review-9 finding 1: a moved facet recomputes the dispatch context's
/// provider/model selection entries from the refreshed facets, while
/// invocation-fixed entries survive untouched.
#[test]
fn a_moved_facet_recomputes_the_dispatch_selection_metadata() {
    let switched = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            agent: Some(AgentHint::Single(Provider::Claude)),
            model: Some(ModelHint::Single("llamacpp/probe-model-x".to_string())),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();

    let context = &switched.dispatch_context;
    assert_eq!(
        context.get("provider_selection_reason"),
        Some(&serde_json::Value::String("FrontmatterSingle".to_string())),
    );
    assert_eq!(
        context.get("resolved_model"),
        Some(&serde_json::Value::String("llamacpp/probe-model-x".to_string())),
    );
    assert_eq!(
        context.get("model_selection_reason"),
        Some(&serde_json::Value::String("FrontmatterSingle".to_string())),
    );
    assert_eq!(
        context.get("composition_file_ref"),
        Some(&serde_json::Value::String("doc.md".to_string())),
        "invocation-fixed entries must survive the recompute",
    );
}

/// A model-only move keeps the invocation's provider-selection provenance via
/// the fallback reason (the rebuild cannot re-derive picker/favorite
/// provenance from the document), while the model entries recompute.
#[test]
fn a_model_only_move_keeps_the_fallback_provider_reason() {
    let moved = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints {
            model: Some(ModelHint::Single("llamacpp/probe-model-x".to_string())),
            ..EffectiveSelectionHints::default()
        }),
        None,
    )
    .unwrap();

    assert_eq!(
        moved.dispatch_context.get("provider_selection_reason"),
        Some(&serde_json::Value::String("FavoriteAgent".to_string())),
    );
    assert_eq!(
        moved.dispatch_context.get("resolved_model"),
        Some(&serde_json::Value::String("llamacpp/probe-model-x".to_string())),
    );
}

/// An unchanged document keeps the invocation's dispatch context verbatim —
/// including selection reasons a rebuild could not reproduce (picker,
/// favorite, sequence review).
#[test]
fn an_unchanged_document_keeps_the_invocation_dispatch_context() {
    let unchanged = rebuild_launch_identity(
        &intent(),
        None,
        None,
        &with_hints(EffectiveSelectionHints::default()),
        None,
    )
    .unwrap();

    assert_eq!(unchanged.dispatch_context, invocation_dispatch_context());
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
