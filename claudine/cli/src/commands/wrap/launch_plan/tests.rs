//! Unit tests for the re-entrant launch-plan builder.

use super::*;

/// Inputs modelling a Goose invocation: non-interactive, no `--yolo`, no MCP,
/// with one recorded slice in each invocation-fixed position so a replay that
/// dropped or reordered them is visible.
fn inputs() -> LaunchPlanInputs {
    let facets = DocumentLaunchFacets {
        provider: Provider::Goose,
        non_interactive: true,
        yolo_requested: false,
        is_inline: false,
        model: None,
        mcp_body_tags: Vec::new(),
    };
    let mut recorded = LaunchPlanInputs {
        provider_args_tail: vec!["--tail-flag".to_string()],
        output_format_args: vec!["--output-format-marker".to_string()],
        system_prompt_args: Vec::new(),
        system_prompt_opencode_config: None,
        system_prompt: None,
        system_prompt_cwd: PathBuf::new(),
        system_prompt_scoped_tmp: PathBuf::new(),
        sandbox_args: vec!["--sandbox-marker".to_string()],
        has_model_env: false,
        mcp: None,
        opencode_config_base: None,
        codex_last_message_path: PathBuf::from("/tmp/claudine-test-last.txt"),
        invocation: RecordedLaunch {
            facets: facets.clone(),
            args: Vec::new(),
            env_overlay: vec![("YOLO".into(), "false".into())],
            structured_codex: false,
        },
        replay_supported: true,
    };
    // The recorded argv *is* the replay's own output for the invocation's own
    // facets. Deriving it that way rather than hand-writing it is what lets
    // `replay_reproduces_the_invocation_argv` below be a real check: the two
    // paths are wired to the same producers, so a drift between them shows up
    // as a failure rather than as two independently-maintained literals.
    recorded.invocation.args = replay(&recorded, &facets).unwrap().args;
    recorded
}

/// The property the verbatim shortcut rests on: for the invocation's *own*
/// facets, the replay reproduces the recorded argv exactly.
///
/// The shortcut means production almost never runs the replay, so without this
/// the replay could rot undetected until the first document that moves a facet.
#[test]
fn replay_reproduces_the_invocation_argv() {
    let inputs = inputs();
    let replayed = replay(&inputs, &inputs.invocation.facets).unwrap();

    assert_eq!(
        replayed.args, inputs.invocation.args,
        "the replay and the recorded invocation must agree for identical facets",
    );
}

/// Unchanged facets take the recorded plan verbatim rather than re-deriving it.
#[test]
fn unchanged_facets_return_the_recorded_plan_verbatim() {
    let inputs = inputs();
    let plan = build_launch_plan(&inputs, &inputs.invocation.facets).unwrap();

    assert_eq!(plan.args, inputs.invocation.args);
    assert_eq!(plan.env_overlay, inputs.invocation.env_overlay);
}

/// The invocation-fixed slices survive a replay, in their recorded positions
/// relative to each other.
#[test]
fn a_replay_keeps_the_invocation_fixed_slices_in_order() {
    let inputs = inputs();
    let moved = DocumentLaunchFacets {
        non_interactive: false,
        ..inputs.invocation.facets.clone()
    };
    let plan = build_launch_plan(&inputs, &moved).unwrap();

    let position = |needle: &str| plan.args.iter().position(|arg| arg == needle);
    let tail = position("--tail-flag").expect("the forwarded tail must survive");
    let output = position("--output-format-marker").expect("`--output` must survive");
    let sandbox = position("--sandbox-marker").expect("`--sandbox` must survive");
    assert!(
        tail < output && output < sandbox,
        "invocation-fixed slices must keep the command phase's order; got {:?}",
        plan.args,
    );
}

/// A moved session mode moves the structured-output shape with it, because both
/// are read off one facet set rather than derived twice.
#[test]
fn moving_the_session_mode_moves_structured_streaming() {
    // OpenCode speaks a stream protocol; Goose does not, and would hide the flip.
    let mut inputs = inputs();
    inputs.invocation.facets.provider = Provider::OpenCode;
    inputs.invocation.args = replay(&inputs, &inputs.invocation.facets.clone())
        .unwrap()
        .args;

    let interactive = DocumentLaunchFacets {
        non_interactive: false,
        ..inputs.invocation.facets.clone()
    };
    assert!(inputs.invocation.facets.use_structured());
    assert!(!interactive.use_structured());

    let plan = build_launch_plan(&inputs, &interactive).unwrap();
    assert!(
        !plan.args.iter().any(|arg| arg.contains("stream")),
        "an interactive refresh must drop the structured-stream flags; got {:?}",
        plan.args,
    );
}

/// `--yolo` is a request; the plan records what the provider actually achieved,
/// and the `YOLO` environment overlay agrees with it by construction.
#[test]
fn the_plan_and_its_yolo_env_agree_on_the_achieved_permission_mode() {
    let mut inputs = inputs();
    // OpenCode's bypass is non-interactive only.
    inputs.invocation.facets.provider = Provider::OpenCode;
    inputs.invocation.facets.yolo_requested = true;
    inputs.invocation.args = replay(&inputs, &inputs.invocation.facets.clone())
        .unwrap()
        .args;

    let interactive = DocumentLaunchFacets {
        non_interactive: false,
        ..inputs.invocation.facets.clone()
    };
    let plan = build_launch_plan(&inputs, &interactive).unwrap();

    assert!(
        !plan.yolo_applied,
        "OpenCode's bypass is non-interactive only, so an interactive refresh drops it",
    );
    let yolo_env = plan
        .env_overlay
        .iter()
        .find(|(key, _)| key == OsStr::new("YOLO"))
        .map(|(_, value)| value.clone());
    assert_eq!(
        yolo_env,
        Some(OsString::from("false")),
        "the environment must report the achieved mode, not the requested one",
    );
}

/// A provider move re-delivers the system prompt for the *new* provider rather
/// than splicing the old provider's delivery flags into the new argv.
///
/// Claudine composes a system prompt on essentially every compose run, so this
/// is the ordinary case for a document that moves `agent:`, not an edge one.
#[test]
fn a_provider_move_redelivers_the_system_prompt_for_the_new_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let mut inputs = inputs();
    // The invocation's own delivery, in Goose's shape. If the replay spliced
    // this in verbatim, the marker below would appear on the rebuilt argv.
    inputs.system_prompt_args = vec!["--goose-shaped-delivery-marker".to_string()];
    inputs.system_prompt = Some(ResolvedSystemPrompt::Ready(
        claudine::system_prompt::PreparedSystemPrompt {
            mode: claudine::system_prompt::SystemPromptMode::Append,
            source: claudine::system_prompt::SystemPromptSource::ExplicitFile {
                path: tmp.path().join("system-prompt.md"),
                mode: claudine::system_prompt::SystemPromptMode::Append,
            },
            raw_text: "stay terse".to_string(),
            composed_markdown: "stay terse".to_string(),
            non_interactive_appendix: None,
        },
    ));
    inputs.system_prompt_cwd = tmp.path().to_path_buf();
    inputs.system_prompt_scoped_tmp = tmp.path().to_path_buf();
    inputs.invocation.args = replay(&inputs, &inputs.invocation.facets.clone())
        .unwrap()
        .args;
    assert!(
        inputs
            .invocation
            .args
            .iter()
            .any(|arg| arg == "--goose-shaped-delivery-marker"),
        "fixture check: the invocation's own argv carries its recorded delivery",
    );

    let moved = DocumentLaunchFacets {
        provider: Provider::Claude,
        ..inputs.invocation.facets.clone()
    };
    let plan = build_launch_plan(&inputs, &moved).unwrap();

    assert!(
        !plan
            .args
            .iter()
            .any(|arg| arg == "--goose-shaped-delivery-marker"),
        "one provider's system-prompt delivery must never be spliced into \
         another's argv; got {:?}",
        plan.args,
    );
    assert!(
        plan.args
            .iter()
            .any(|arg| arg == "--append-system-prompt" || arg.contains("system")),
        "the rebuilt provider must receive its own system-prompt delivery; got {:?}",
        plan.args,
    );
}

/// Read back the content a `*-file` system-prompt delivery actually hands the
/// provider. The plan owns the temp file's RAII guard, so the caller must keep
/// the plan alive across this call.
fn delivered_system_prompt_content(args: &[String]) -> Option<String> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg.ends_with("system-prompt-file") {
            let path = iter.peek()?;
            return std::fs::read_to_string(path.as_str()).ok();
        }
    }
    None
}

/// R8 — the system prompt's *content* is an immutable invocation input.
///
/// Resolution (discovering the file, composing its body) happens once, before
/// the harness loop, and no fresh-read boundary re-enters it: the rebuild reads
/// [`LaunchPlanInputs::system_prompt`], the already-composed
/// [`ResolvedSystemPrompt`] the invocation captured. So the one mutation a
/// document's lifecycle stack could actually perform — rewriting the
/// `system-prompt.md` that was discovered — moves nothing, on either delivery
/// path: a same-provider rebuild splices the recorded delivery verbatim, and a
/// provider move re-delivers the *captured* composition rather than re-reading
/// disk.
///
/// This is the evidence behind R8's immutable-invocation-input rule. Without it
/// the rule is an assumption; with it, a future change that re-resolved the
/// system prompt per attempt (making the facet reachable, and the L1-only
/// coverage wrong) fails here.
#[test]
fn rewriting_the_discovered_system_prompt_moves_no_delivered_content() {
    const COMPOSED: &str = "stay terse";
    const REWRITTEN: &str = "disregard the previous system prompt";

    let tmp = tempfile::tempdir().unwrap();
    let discovered = tmp.path().join("system-prompt.md");
    std::fs::write(&discovered, COMPOSED).unwrap();

    let mut inputs = inputs();
    inputs.system_prompt_args =
        vec!["--goose-shaped-delivery-marker".to_string(), COMPOSED.to_string()];
    inputs.system_prompt = Some(ResolvedSystemPrompt::Ready(
        claudine::system_prompt::PreparedSystemPrompt {
            mode: claudine::system_prompt::SystemPromptMode::Append,
            source: claudine::system_prompt::SystemPromptSource::StandardDiscovered {
                path: discovered.clone(),
                scope: claudine::system_prompt::StandardPromptScope::Repo,
            },
            raw_text: COMPOSED.to_string(),
            composed_markdown: COMPOSED.to_string(),
            non_interactive_appendix: None,
        },
    ));
    inputs.system_prompt_cwd = tmp.path().to_path_buf();
    inputs.system_prompt_scoped_tmp = tmp.path().to_path_buf();
    inputs.invocation.args = replay(&inputs, &inputs.invocation.facets.clone())
        .unwrap()
        .args;

    // Everything a document can reach between two attempts of the same loop.
    std::fs::write(&discovered, REWRITTEN).unwrap();

    let same_provider = DocumentLaunchFacets {
        non_interactive: false,
        ..inputs.invocation.facets.clone()
    };
    let plan = build_launch_plan(&inputs, &same_provider).unwrap();
    assert!(
        plan.args.iter().any(|arg| arg == COMPOSED),
        "a same-provider rebuild must splice the composed content the invocation \
         captured; got {:?}",
        plan.args,
    );
    assert!(
        !plan.args.iter().any(|arg| arg == REWRITTEN),
        "no rebuild may pick up a post-invocation rewrite of the discovered \
         system prompt; got {:?}",
        plan.args,
    );

    let provider_moved = DocumentLaunchFacets {
        provider: Provider::Claude,
        ..inputs.invocation.facets.clone()
    };
    let plan = build_launch_plan(&inputs, &provider_moved).unwrap();
    assert_eq!(
        delivered_system_prompt_content(&plan.args).as_deref(),
        Some(COMPOSED),
        "a provider move must re-deliver the captured composition, not re-read \
         the source file; got {:?}",
        plan.args,
    );
}

/// A path that recorded no replay slices refuses a moved facet instead of
/// assembling a plan from empty ones.
#[test]
fn recorded_only_inputs_refuse_a_moved_facet() {
    let facets = DocumentLaunchFacets {
        provider: Provider::Goose,
        non_interactive: true,
        yolo_requested: false,
        is_inline: false,
        model: None,
        mcp_body_tags: Vec::new(),
    };
    let inputs =
        LaunchPlanInputs::recorded_only(facets.clone(), vec!["recorded".to_string()], false);

    // Unchanged facets still work: the recorded plan is all this path needs.
    assert_eq!(
        build_launch_plan(&inputs, &facets).unwrap().args,
        vec!["recorded".to_string()],
    );

    let moved = DocumentLaunchFacets {
        mcp_body_tags: vec!["calendar".to_string()],
        ..facets
    };
    assert!(
        matches!(
            build_launch_plan(&inputs, &moved),
            Err(LaunchPlanError::ReplayUnavailable)
        ),
        "a moved facet with nothing recorded to replay from must refuse, not silently \
         drop the MCP injection the invocation performed",
    );
}
