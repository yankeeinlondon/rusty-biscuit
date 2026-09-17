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
        epoch_context: None,
        document_epoch: None,
        source_context: None,
    }
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

/// An adopted target that authors `initialize` is read for its frontmatter
/// and lifecycle surface only: a body include `initialize` has yet to create
/// is not dereferenced, and nothing in the lifecycle runs.
#[test]
fn bootstrap_harness_prompt_reads_an_initialize_target_without_its_body() {
    let dir = tempfile::TempDir::new().unwrap();
    let target = dir.path().join("target.md");
    let generated = dir.path().join("generated.md");
    std::fs::write(
        &target,
        "---\ntitle: staged target\ninitialize:\n  stack:\n    \
         - action: {ensure_file: 'generated.md'}\n---\nTARGET-BODY\n\n::file generated.md\n",
    )
    .unwrap();
    let mut state = compose_state(&target, CallerInputLayers::default());
    let approval = claudine::harness::ShellApprovalOptions::default();

    let bootstrap = bootstrap_harness_prompt(&mut state, dir.path(), &approval)
        .expect("the bootstrap read never touches the body")
        .expect("the target authors initialize");

    assert!(bootstrap.prompt.is_empty(), "the bootstrap read carries no prompt");
    assert!(bootstrap.inline_closure_plan.is_none() && bootstrap.launch_schema.is_none());
    assert_eq!(bootstrap.frontmatter["title"], "staged target");
    assert!(
        !bootstrap.lifecycle.as_ref().expect("lifecycle surface").is_empty(),
        "the initialize stack is on the surface the gate approves"
    );
    assert!(!generated.exists(), "reading the surface runs no lifecycle action");
    assert!(
        materialize_harness_prompt(
            &mut state,
            None,
            dir.path(),
            None,
            SchemaStage::DeferToStabilizedReread,
        )
        .is_err(),
        "the full read of the same target still fails on the missing include"
    );

    std::fs::write(&generated, "GENERATED\n").unwrap();
    let stabilized =
        materialize_harness_prompt(&mut state, None, dir.path(), None, SchemaStage::Validate)
            .expect("the stabilized reread sees the created file");
    assert!(stabilized.prompt.contains("GENERATED"), "{}", stabilized.prompt);
}

/// A target without `initialize` has nothing to wait for: it keeps the
/// eager full read.
#[test]
fn bootstrap_harness_prompt_leaves_a_target_without_initialize_to_the_eager_read() {
    let dir = tempfile::TempDir::new().unwrap();
    let target = dir.path().join("target.md");
    std::fs::write(&target, "---\ntitle: eager\n---\nBODY\n\n::file missing.md\n").unwrap();
    let mut state = compose_state(&target, CallerInputLayers::default());

    let bootstrap = bootstrap_harness_prompt(
        &mut state,
        dir.path(),
        &claudine::harness::ShellApprovalOptions::default(),
    )
    .unwrap();

    assert!(bootstrap.is_none());
}

#[test]
fn harness_reentry_epochs_keep_launch_values_and_source_ownership() {
    let temp = tempfile::tempdir().unwrap();
    let launch_repo = temp.path().join("launch");
    let launch_dir = launch_repo.join("alpha/lib");
    let source_repo = temp.path().join("source");
    let source_dir = source_repo.join("nested");
    std::fs::create_dir_all(&launch_dir).unwrap();
    std::fs::create_dir_all(&source_dir).unwrap();
    for repo in [&launch_repo, &source_repo] {
        assert!(std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(repo)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(
        launch_repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha/lib\", \"sibling\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    std::fs::write(
        launch_dir.join("Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(launch_repo.join("sibling")).unwrap();
    std::fs::write(
        launch_repo.join("sibling/Cargo.toml"),
        "[package]\nname = \"sibling\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        launch_dir.join("schema.yaml"),
        "launch_only: string(required)\n",
    )
    .unwrap();
    std::fs::write(launch_dir.join("fragment.md"), "LAUNCH-FRAGMENT\n").unwrap();
    std::fs::write(
        source_dir.join("schema.yaml"),
        "source_marker: string(required)\nspec: 'file(eager; required)'\nprepared_area: \
         string(required)\nprepared_cwd: string(required)\nprepared_agent: string(required)\nprepared_model: string(required)\n",
    )
    .unwrap();
    std::fs::write(launch_dir.join("spec.md"), "LAUNCH-SPEC\n").unwrap();
    std::fs::write(source_dir.join("spec.md"), "SOURCE-SPEC\n").unwrap();
    std::fs::write(source_dir.join("fragment.md"), "SOURCE-FRAGMENT\n").unwrap();
    let target = source_dir.join("target.md");
    std::fs::write(
        &target,
        concat!(
            "---\n",
            "$schema: ./schema.yaml\n",
            "source_marker: source-owned\n",
            "prepared_area: '{{ ctx.area }}'\n",
            "prepared_cwd: '{{ ctx.cwd }}'\n",
            "prepared_agent: '{{ ctx.agent }}'\n",
            "prepared_model: '{{ ctx.model }}'\n",
            "---\n",
            "AREA={{ ctx.area }} CWD={{ ctx.cwd }} AGENT={{ ctx.agent }} MODEL={{ ctx.model }} ",
            "ENV={{ env.AGENT }}/{{ env.MODEL }} FILE={{ file_exists(spec) }}\n",
            "SOURCE-BODY\n",
        ),
    )
    .unwrap();

    let env = BTreeMap::from([
        ("AGENT".to_string(), "codex".to_string()),
        ("MODEL".to_string(), "gpt-5".to_string()),
    ]);
    let invocation = claudine::invocation_context::InvocationContext::capture_at(&launch_dir);
    let materialized_spec = biscuit_file::to_portable_string(&launch_dir.join("spec.md"));
    let mut state = compose_state(
        &target,
        CallerInputLayers {
            set_overrides: Some(serde_json::json!({ "spec": materialized_spec.clone() })),
            env_overrides: env,
            file_ref_fallback_dir: Some(launch_dir.clone()),
            file_resolution_context: Some(invocation.launch_file_resolution_context().clone()),
            ..CallerInputLayers::default()
        },
    );
    state.source_context = Some(invocation.derive_source(&target).unwrap());
    state.invocation_context = Some(invocation.clone());
    let approval_options = claudine::harness::ShellApprovalOptions::default();

    let assert_materialized = |entry: DocumentEntryReason,
                               materialized: &MaterializedHarnessPrompt| {
        assert!(
            materialized
                .prompt
                .contains(&format!(
                    "AREA=alpha CWD={} AGENT=codex MODEL=gpt-5 ENV=codex/gpt-5 FILE=true",
                    biscuit_file::to_portable_string(&launch_dir)
                )),
            "entry {entry:?} lost launch or target identity: {}",
            materialized.prompt
        );
        assert!(
            materialized.prompt.contains("SOURCE-BODY")
                && !materialized.prompt.contains("LAUNCH-FRAGMENT"),
            "entry {entry:?} did not keep the document body source-owned: {}",
            materialized.prompt
        );
        assert_eq!(materialized.frontmatter["prepared_area"], serde_json::json!("alpha"));
        assert_eq!(
            materialized.frontmatter["prepared_cwd"],
            serde_json::json!(biscuit_file::to_portable_string(&launch_dir))
        );
        assert_eq!(materialized.frontmatter["prepared_agent"], serde_json::json!("codex"));
        assert_eq!(materialized.frontmatter["prepared_model"], serde_json::json!("gpt-5"));
        assert_eq!(materialized.frontmatter["source_marker"], serde_json::json!("source-owned"));
        assert_eq!(
            materialized.frontmatter["spec"],
            serde_json::json!(materialized_spec),
            "entry {entry:?} must preserve the caller-materialized launch file"
        );
    };

    preflight_harness_document(&mut state, &approval_options, &launch_dir).unwrap();
    let proxy = materialize_harness_prompt(
        &mut state,
        Some(&source_repo),
        &launch_dir,
        None,
        SchemaStage::Validate,
    )
    .unwrap();
    assert_materialized(DocumentEntryReason::ProxyTarget, &proxy);
    assert_eq!(
        proxy.document_epoch.as_ref().unwrap().work_snapshot(),
        claudine::invocation_context::DocumentEpochWork {
            launch_context_constructions: 1,
            launch_context_extensions: 0,
            ambient_fallbacks: 0,
            prepared_context_consumers: BTreeMap::from([
                ("body".to_string(), 1),
                ("effective-frontmatter".to_string(), 1),
                ("preflight".to_string(), 1),
            ]),
        },
        "the proxy target's first canonical read must be one complete epoch"
    );

    // Model an initialize-time rewrite that adds a group the bootstrap
    // document did not require. The next materialization is the
    // stabilized reread in the same epoch and must extend, not recapture.
    let original = std::fs::read_to_string(&target).unwrap();
    std::fs::write(
        &target,
        original.replace(
            "SOURCE-BODY\n",
            "SOURCE-BODY OS={{ ctx.os }} REPO={{ ctx.repo_root }}\n",
        ),
    )
    .unwrap();

    preflight_harness_document(&mut state, &approval_options, &launch_dir).unwrap();
    let stabilized = materialize_harness_prompt(
        &mut state,
        Some(&source_repo),
        &launch_dir,
        None,
        SchemaStage::Validate,
    )
    .unwrap();
    assert_materialized(DocumentEntryReason::ProxyTarget, &stabilized);
    assert!(
        stabilized.prompt.contains(" OS=") && !stabilized.prompt.contains("OS= REPO="),
        "the stabilized reread must populate the newly required OS group: {}",
        stabilized.prompt
    );
    assert_eq!(
        stabilized.document_epoch.as_ref().unwrap().work_snapshot(),
        claudine::invocation_context::DocumentEpochWork {
            launch_context_constructions: 1,
            launch_context_extensions: 1,
            ambient_fallbacks: 0,
            prepared_context_consumers: BTreeMap::from([
                ("body".to_string(), 2),
                ("effective-frontmatter".to_string(), 2),
                ("preflight".to_string(), 2),
            ]),
        },
        "the stabilized reread stays inside the proxy epoch and only extends it"
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

    preflight_harness_document(&mut state, &approval_options, dir.path())
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
