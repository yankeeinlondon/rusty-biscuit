use super::*;
use crate::permissions::{CommandQuery, ConfiguredPolicySnapshot, PolicyContext};

fn setup_ctx() -> (tempfile::TempDir, PolicyContext) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(home.join(".codex")).unwrap();
    std::fs::create_dir_all(repo.join(".codex")).unwrap();
    (
        dir,
        PolicyContext::new(repo.clone())
            .with_home_dir(home)
            .with_repo_root(repo)
            .with_trust(crate::permissions::ProjectTrustContext {
                is_trusted: Some(true),
                source: crate::permissions::TrustSource::ExplicitInput,
            }),
    )
}

#[tokio::test]
async fn codex_backend_models_workspace_write() {
    let (_dir, ctx) = setup_ctx();
    let path = ctx.repo_root.as_ref().unwrap().join(".codex/config.toml");
    tokio::fs::write(
        &path,
        r#"
sandbox_mode = "workspace-write"
approval_policy = "on-request"

[sandbox_workspace_write]
writable_roots = ["/tmp/build-output"]
network_access = false
"#,
    )
    .await
    .unwrap();

    let backend = CodexPolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Codex, native, canonical, &ctx);

    assert!(
        snapshot
            .can_write(ctx.repo_root.as_ref().unwrap().join("src/main.rs"))
            .is_allowed()
    );
    assert!(
        snapshot
            .can_write("/tmp/build-output/file.txt")
            .is_allowed()
    );
    assert!(snapshot.can_write("/etc/hosts").is_ask());
    assert!(
        snapshot
            .can_execute(&CommandQuery::from_raw("git status"))
            .is_ask()
    );
}

#[tokio::test]
async fn codex_mutation_plan_generates_add_dir_and_rule_file() {
    let (_dir, ctx) = setup_ctx();
    let backend = CodexPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Codex,
        Vec::new(),
        CodexState {
            layers: Vec::new(),
            cli: CodexCliOverrides::default(),
        },
    );
    let change = PolicyChange::persistent(vec![
        PolicyChangeOp::GrantWrite(PathBuf::from("/tmp/cache")),
        PolicyChangeOp::DenyCommand(CommandPattern::new("rm -rf")),
    ]);

    let plan = backend.plan_change(&ctx, &current, &change).await.unwrap();
    assert_eq!(plan.persistent_plan.as_ref().unwrap().edits.len(), 2);
    assert!(
        plan.one_shot_plan
            .as_ref()
            .unwrap()
            .argv
            .contains(&"--add-dir".to_owned())
    );
}

#[tokio::test]
async fn codex_full_auto_cli_override_is_effective() {
    let (_dir, ctx) = setup_ctx();
    let backend = CodexPolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let cli = backend
        .parse_cli_overrides(&ctx, CliPolicyInput::Argv(&["--full-auto".to_owned()]))
        .unwrap();
    let native = backend
        .compose_native_policy(&ctx, &layers, Some(&cli))
        .unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Codex, native, canonical, &ctx);

    assert_eq!(
        snapshot.canonical.axes.runtime.sandbox_mode,
        Some(CanonicalSandboxMode::Partial),
    );
    assert_eq!(
        snapshot.canonical.axes.runtime.approval_mode,
        Some(CanonicalApprovalMode::AlwaysAsk),
    );
    assert!(
        snapshot
            .can_write(ctx.repo_root.as_ref().unwrap().join("src/main.rs"))
            .is_allowed()
    );
    assert!(snapshot.can_write("/etc/hosts").is_ask());
}

#[tokio::test]
async fn codex_unknown_trust_skips_repo_config_and_degrades_query_answers() {
    let (_dir, mut ctx) = setup_ctx();
    ctx.trust.is_trusted = None;
    ctx.trust.source = crate::permissions::TrustSource::Unknown;

    tokio::fs::write(
        ctx.repo_root.as_ref().unwrap().join(".codex/config.toml"),
        r#"
sandbox_mode = "danger-full-access"
approval_policy = "never"
"#,
    )
    .await
    .unwrap();
    tokio::fs::write(
        ctx.home_dir.as_ref().unwrap().join(".codex/config.toml"),
        r#"
sandbox_mode = "read-only"
approval_policy = "on-request"
"#,
    )
    .await
    .unwrap();

    let backend = CodexPolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].id, "codex-user");

    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Codex, native, canonical, &ctx);
    let result = snapshot.can_write("src/main.rs");

    assert!(result.is_unknown());
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.code == "codex.trust_unknown")
    );
}

#[tokio::test]
async fn codex_backend_queries_mcp_controls() {
    let (_dir, ctx) = setup_ctx();
    tokio::fs::write(
        ctx.repo_root.as_ref().unwrap().join(".codex/config.toml"),
        r#"
[mcp_servers.filesystem]
enabled = true
disabled_tools = ["delete_file"]

[mcp_servers.github]
enabled = false

[mcp_servers.browser]
enabled = true
enabled_tools = ["navigate"]
"#,
    )
    .await
    .unwrap();

    let backend = CodexPolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Codex, native, canonical, &ctx);

    assert!(snapshot.can_use_mcp_server("filesystem").is_allowed());
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "read_file")
            .is_allowed()
    );
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "delete_file")
            .is_denied()
    );
    assert!(snapshot.can_use_mcp_server("github").is_denied());
    assert!(
        snapshot
            .can_use_mcp_tool("browser", "navigate")
            .is_allowed()
    );
    assert!(snapshot.can_use_mcp_tool("browser", "click").is_denied());
}

#[tokio::test]
async fn codex_mcp_mutation_plan_updates_config_and_one_shot_args() {
    let (_dir, ctx) = setup_ctx();
    let backend = CodexPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Codex,
        Vec::new(),
        CodexState {
            layers: Vec::new(),
            cli: CodexCliOverrides::default(),
        },
    );
    let change = PolicyChange::persistent(vec![
        PolicyChangeOp::DenyMcpServer("github".to_owned()),
        PolicyChangeOp::AllowMcpTool {
            server: "filesystem".to_owned(),
            tool: "read_file".to_owned(),
        },
    ]);

    let plan = backend.plan_change(&ctx, &current, &change).await.unwrap();
    let edit = &plan.persistent_plan.as_ref().unwrap().edits[0];

    assert!(edit.after_preview.contains("github"));
    assert!(edit.after_preview.contains("enabled = false"));
    assert!(edit.after_preview.contains("read_file"));
    assert!(
        plan.one_shot_plan
            .as_ref()
            .unwrap()
            .argv
            .contains(&"mcp_servers.github.enabled=false".to_owned())
    );
}

#[tokio::test]
async fn codex_mcp_round_trip_mutation_changes_query_result() {
    let (_dir, ctx) = setup_ctx();
    let backend = CodexPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Codex,
        Vec::new(),
        CodexState {
            layers: Vec::new(),
            cli: CodexCliOverrides::default(),
        },
    );
    let change = PolicyChange::persistent(vec![
        PolicyChangeOp::DenyMcpServer("github".to_owned()),
        PolicyChangeOp::AllowMcpTool {
            server: "filesystem".to_owned(),
            tool: "read_file".to_owned(),
        },
    ]);

    let plan = backend.plan_change(&ctx, &current, &change).await.unwrap();
    let edit = &plan.persistent_plan.as_ref().unwrap().edits[0];
    tokio::fs::create_dir_all(edit.path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&edit.path, edit.after_preview.as_bytes())
        .await
        .unwrap();

    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Codex, native, canonical, &ctx);

    assert!(snapshot.can_use_mcp_server("github").is_denied());
    assert!(
        snapshot
            .can_use_mcp_tool("github", "create_issue")
            .is_denied()
    );
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "read_file")
            .is_allowed()
    );
}

#[tokio::test]
async fn codex_local_override_target_returns_error() {
    let (_dir, ctx) = setup_ctx();
    let backend = CodexPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Codex,
        Vec::new(),
        CodexState {
            layers: Vec::new(),
            cli: CodexCliOverrides::default(),
        },
    );
    let change = PolicyChange {
        operations: vec![PolicyChangeOp::DenyMcpServer("github".to_owned())],
        target: crate::permissions::PolicyChangeTarget::LocalOverride,
        persistence: crate::permissions::PolicyPersistence::Persistent,
    };

    let result = backend.plan_change(&ctx, &current, &change).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("LocalOverride"));
}
