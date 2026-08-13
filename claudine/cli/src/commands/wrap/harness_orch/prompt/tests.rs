use super::*;
use claudine::composition::{CallerInputLayers, DocumentEntryReason, SchemaStage};
use std::collections::BTreeMap;

fn compose_state(source_path: &Path, input_layers: CallerInputLayers) -> HarnessPromptState {
    HarnessPromptState {
        mode: HarnessPromptMode::Compose,
        source_path: source_path.to_path_buf(),
        original_ref: source_path.display().to_string(),
        base_prompt: None,
        overlay: indexmap::IndexMap::new(),
        prompt_tail: Vec::new(),
        input_layers,
        runtime_state: std::sync::Arc::new(claudine::composition::RuntimeState::new()),
        suppress_output_commit: false,
        last_final_output: None,
        entry: DocumentEntryReason::ProxyTarget,
        invocation_context: None,
        source_context: None,
        epoch_context: None,
        epoch_context_requirements: None,
    }
}

fn init_repo(root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    let output = std::process::Command::new("git")
        .args(["init", "--quiet", "--initial-branch=main"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
}

/// Real directory a launch repository is created beneath.
///
/// The leading `.` is the point of the fixture: it puts a path separator
/// immediately before a `.` in every composed `ctx.repo_root`, which on
/// Windows is the `\.` sequence CommonMark consumes as an escape unless
/// interpolation is literal. The backtick pair earns the same coverage on
/// Unix, where the separator is `/` and nothing else in a temporary path is
/// Markdown syntax.
const HIDDEN_LAUNCH_PARENT: &str = ".tmp`ZZZ`";

/// Launch repository for a fixture rooted at `fixture_root`.
///
/// Fails loudly rather than degrading quietly: a fixture that stopped
/// crossing the separator-then-`.` boundary would still pass every
/// assertion below while no longer reproducing the regression.
fn launch_repository(fixture_root: &Path) -> PathBuf {
    let repo = fixture_root.join(HIDDEN_LAUNCH_PARENT).join("launch");
    let boundary = format!("{}{HIDDEN_LAUNCH_PARENT}", std::path::MAIN_SEPARATOR);
    assert!(
        repo.to_string_lossy().contains(boundary.as_str()),
        "the launch repository must sit behind a `{boundary}` boundary: {}",
        repo.display()
    );
    repo
}

/// Literal text of a composed Markdown prompt.
///
/// An interpolated scalar is serialized with whatever escaping its literal
/// text requires, so the provider's bytes and the value they denote are two
/// different strings. Inline code contributes its content without its
/// delimiters, so a value that escaped into a code span is reported as the
/// text it is *not*, rather than being reassembled into a passing
/// comparison.
fn parsed_text(markdown: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser, TagEnd};

    let mut text = String::new();
    for event in Parser::new_ext(markdown, Options::all() - Options::ENABLE_SMART_PUNCTUATION)
    {
        match event {
            Event::Text(chunk) => text.push_str(&chunk),
            Event::Code(code) => text.push_str(&code),
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => {
                text.push('\n');
            }
            _ => {}
        }
    }
    text
}

#[test]
fn stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity() {
    let fixture = tempfile::tempdir().unwrap();
    let launch_root = fixture.path().join("launch");
    let launch_area = launch_root.join("alpha/lib");
    init_repo(&launch_root);
    let beta_area = launch_root.join("beta/lib");
    std::fs::create_dir_all(launch_area.join("src")).unwrap();
    std::fs::create_dir_all(beta_area.join("src")).unwrap();
    std::fs::write(
        launch_root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"alpha/lib\", \"beta/lib\"]\n",
    )
    .unwrap();
    std::fs::write(
        launch_area.join("Cargo.toml"),
        "[package]\nname = \"alpha-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(launch_area.join("src/lib.rs"), "").unwrap();
    std::fs::write(
        beta_area.join("Cargo.toml"),
        "[package]\nname = \"beta-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(beta_area.join("src/lib.rs"), "").unwrap();

    let source_root = fixture.path().join("source");
    init_repo(&source_root);
    let target = source_root.join("target.md");
    std::fs::write(&target, "---\n---\n{{ ctx.area }}/{{ ctx.agent }}\n").unwrap();

    let invocation =
        claudine::invocation_context::InvocationContext::capture_at(&launch_area);
    let source_context = invocation.derive_source(&target).unwrap();
    let source = load_overlaid_source(&compose_state(
        &target,
        CallerInputLayers::default(),
    ))
    .unwrap();
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let mut context = invocation.capture_launch_context(&requirements);
    context
        .env_mut()
        .insert("AGENT".to_string(), "codex".to_string());
    context
        .env_mut()
        .insert("MODEL".to_string(), "gpt-test".to_string());

    let mut env = BTreeMap::new();
    env.insert("AGENT".to_string(), "codex".to_string());
    env.insert("MODEL".to_string(), "gpt-test".to_string());
    let mut state = compose_state(
        &target,
        CallerInputLayers {
            env_overrides: env,
            ..CallerInputLayers::default()
        },
    );
    state.invocation_context = Some(invocation.clone());
    state.source_context = Some(source_context);
    state.epoch_context = Some(context);
    state.epoch_context_requirements = Some(requirements);

    let first = harness_prepare_options(&mut state, &source, &launch_root);
    let first_context = first.prepared_context.expect("prepared epoch context");
    let first_effective = first_context.as_object();
    assert_eq!(first_context.get("area").and_then(|v| v.as_str()), Some("alpha-lib"));
    assert_eq!(first_effective.get("agent").and_then(|v| v.as_str()), Some("codex"));
    assert_eq!(first_effective.get("model").and_then(|v| v.as_str()), Some("gpt-test"));
    assert_eq!(invocation.work_snapshot().launch_context_constructions, 1);
    assert_eq!(invocation.work_snapshot().launch_context_extensions, 0);

    std::fs::write(
        &target,
        "---\n---\n{{ ctx.area }}/{{ ctx.agent }}/{{ ctx.os }}\n",
    )
    .unwrap();
    let reread = load_overlaid_source(&state).unwrap();
    let second = harness_prepare_options(&mut state, &reread, &launch_root);
    let second_context = second.prepared_context.expect("extended epoch context");
    let effective = second_context.as_object();
    assert_eq!(second_context.get("area").and_then(|v| v.as_str()), Some("alpha-lib"));
    assert_eq!(effective.get("agent").and_then(|v| v.as_str()), Some("codex"));
    assert_eq!(effective.get("model").and_then(|v| v.as_str()), Some("gpt-test"));
    assert!(second_context.get("os").is_some());

    let work = invocation.work_snapshot();
    assert_eq!(work.launch_context_constructions, 1);
    assert_eq!(work.launch_context_extensions, 1);
    assert_eq!(work.ambient_fallbacks, 0);
}

#[test]
fn proxy_retry_and_resume_start_fresh_target_adjusted_launch_epochs() {
    let fixture = tempfile::tempdir().unwrap();
    let launch_root = launch_repository(fixture.path());
    init_repo(&launch_root);
    let source_root = fixture.path().join("source");
    init_repo(&source_root);
    let target = source_root.join("target.md");
    std::fs::write(
        &target,
        "---\n---\n{{ ctx.agent }}/{{ ctx.model }}/{{ env.AGENT }}/{{ env.MODEL }}/{{ ctx.repo_root }}\n",
    )
    .unwrap();

    let invocation =
        claudine::invocation_context::InvocationContext::capture_at(&launch_root);
    for entry in [
        DocumentEntryReason::ProxyTarget,
        DocumentEntryReason::Retry,
        DocumentEntryReason::Resume,
    ] {
        let mut state = compose_state(
            &target,
            CallerInputLayers {
                env_overrides: BTreeMap::from([
                    ("AGENT".to_string(), "codex".to_string()),
                    ("MODEL".to_string(), "gpt-reentry".to_string()),
                ]),
                ..CallerInputLayers::default()
            },
        );
        state.entry = entry;
        state.invocation_context = Some(invocation.clone());
        state.source_context = Some(invocation.derive_source(&target).unwrap());
        let expected_source_root = state
            .source_context
            .as_ref()
            .and_then(claudine::invocation_context::SourceContext::repository_root)
            .unwrap()
            .to_path_buf();

        let materialized = materialize_harness_prompt(
            &mut state,
            None,
            &launch_root,
            None,
            SchemaStage::Validate,
        )
        .unwrap();
        assert_eq!(
            parsed_text(&materialized.prompt).trim(),
            format!(
                "codex/gpt-reentry/codex/gpt-reentry/{}",
                launch_root.display()
            ),
            "wrong target-adjusted launch snapshot for {entry:?}; provider received:\n{}",
            materialized.prompt
        );
        assert_eq!(
            materialized
                .file_resolution_context
                .as_ref()
                .and_then(biscuit_file::FileResolutionContext::repository_root),
            Some(expected_source_root.as_path()),
            "file resolution must remain source-relative for {entry:?}"
        );
    }

    let work = invocation.work_snapshot();
    assert_eq!(work.launch_context_constructions, 3);
    assert_eq!(work.launch_context_extensions, 0);
    assert_eq!(work.ambient_fallbacks, 0);
}

/// Issue #2 regression: a proxy/retry re-materialization must resolve
/// `ctx.agent`/`ctx.model` from the carried env overrides. Before the fix
/// the re-composition captured a fresh env-less context and both collapsed
/// to the `unknown`/`default` fallbacks.
#[test]
fn compose_rematerialize_resolves_ctx_agent_from_env() {
    let dir = tempfile::TempDir::new().unwrap();
    let target = dir.path().join("target.md");
    std::fs::write(&target, "---\ndescription: t\n---\n{{ ctx.agent }}/{{ ctx.model }}\n")
        .unwrap();

    let mut env = BTreeMap::new();
    env.insert("AGENT".to_string(), "codex".to_string());
    env.insert("MODEL".to_string(), "gpt-5".to_string());
    let mut state = compose_state(
        &target,
        CallerInputLayers {
            env_overrides: env,
            ..CallerInputLayers::default()
        },
    );

    let materialized = materialize_harness_prompt(&mut state, None, dir.path(), None, SchemaStage::Validate).unwrap();
    assert_eq!(
        materialized.prompt.trim(),
        "codex/gpt-5",
        "ctx.agent/ctx.model must resolve from the carried env, not the fallbacks",
    );
}

/// Issue #1 regression: a proxy target's own frontmatter `$(...)` shell
/// command must be discovered and approved at hand-off and folded into the
/// carried pre-approved set, so the subsequent re-materialize compose does
/// not reject a whitelisted command with `NotPreApproved`.
#[test]
fn proxy_target_preflight_approves_frontmatter_shell_and_rematerializes() {
    let dir = tempfile::TempDir::new().unwrap();
    // Whitelist `basename` so the audit auto-approves without a handler,
    // mirroring the real review prompt's reliance on the repo whitelist.
    std::fs::write(dir.path().join(".darkmatter-shell-whitelist"), "prefix basename\n")
        .unwrap();
    let target = dir.path().join("target.md");
    std::fs::write(
        &target,
        "---\nbase: \"$(basename '{{ spec }}')\"\n---\nreviewing {{ base }}\n",
    )
    .unwrap();

    let mut state = compose_state(
        &target,
        CallerInputLayers {
            set_overrides: Some(serde_json::json!({ "spec": "features/x/spec.md" })),
            ..CallerInputLayers::default()
        },
    );

    let approval_options = claudine::harness::ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };

    // Before hand-off pre-flight, the carried set has no approval for the
    // target's own frontmatter shell command.
    assert!(state.input_layers.pre_approved_commands.is_none());

    preflight_proxy_target(&mut state, &approval_options, dir.path())
        .expect("whitelisted proxy-target command must pre-flight cleanly");

    let approved = state
        .input_layers
        .pre_approved_commands
        .as_ref()
        .expect("pre-approved set must be populated after target pre-flight");
    assert!(
        approved.contains("basename features/x/spec.md"),
        "expected the resolved frontmatter shell command; got: {approved:?}",
    );

    // The re-materialize compose now expands the frontmatter command against
    // the augmented pre-approved set instead of failing NotPreApproved.
    let materialized = materialize_harness_prompt(&mut state, None, dir.path(), None, SchemaStage::Validate).unwrap();
    assert_eq!(materialized.prompt.trim(), "reviewing spec.md");
}
