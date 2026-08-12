use super::*;
use crate::permissions::{CommandQuery, ConfiguredPolicySnapshot, PolicyContext};

fn setup_ctx() -> (tempfile::TempDir, PolicyContext) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(home.join(".qwen")).unwrap();
    std::fs::create_dir_all(repo.join(".qwen")).unwrap();
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
async fn qwen_backend_answers_workspace_command_and_mcp_queries() {
    let (_dir, ctx) = setup_ctx();
    let path = ctx.repo_root.as_ref().unwrap().join(".qwen/settings.json");
    tokio::fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "permissions": {
                "defaultMode": "auto-edit",
                "deny": ["Bash(rm *)"],
                "additionalDirectories": ["/shared"]
            },
            "tools": {
                "sandbox": true
            },
            "mcpServers": {
                "filesystem": {
                    "trust": true,
                    "includeTools": ["read_file"],
                    "excludeTools": ["delete_file"]
                }
            }
        }))
        .unwrap(),
    )
    .await
    .unwrap();

    let backend = QwenPolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::QwenCode, native, canonical, &ctx);

    assert!(
        snapshot
            .can_write(ctx.repo_root.as_ref().unwrap())
            .is_allowed()
    );
    assert!(
        snapshot
            .can_execute(&CommandQuery::from_raw("rm -rf target"))
            .is_denied()
    );
    assert!(snapshot.can_use_mcp_server("filesystem").is_allowed());
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "delete_file")
            .is_denied()
    );
    assert_eq!(
        snapshot.canonical.axes.runtime.sandbox_mode,
        Some(CanonicalSandboxMode::Partial)
    );
}

#[tokio::test]
async fn qwen_cli_allowed_mcp_servers_deny_unlisted_servers() {
    let (_dir, ctx) = setup_ctx();
    let backend = QwenPolicyBackend;
    let cli = vec![
        "--allowed-mcp-server-names".to_owned(),
        "filesystem".to_owned(),
    ];
    let native = backend
        .compose_native_policy(
            &ctx,
            &[],
            Some(
                &backend
                    .parse_cli_overrides(&ctx, CliPolicyInput::Argv(&cli))
                    .unwrap(),
            ),
        )
        .unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::QwenCode, native, canonical, &ctx);

    assert!(snapshot.can_use_mcp_server("filesystem").is_allowed());
    assert!(snapshot.can_use_mcp_server("github").is_denied());
    // Tool queries on unlisted servers must also be denied.
    assert!(
        snapshot
            .can_use_mcp_tool("github", "create_issue")
            .is_denied()
    );
}

#[tokio::test]
async fn qwen_mcp_round_trip_mutation_changes_query_result() {
    let (_dir, ctx) = setup_ctx();
    let backend = QwenPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::QwenCode,
        Vec::new(),
        QwenState {
            layers: Vec::new(),
            cli: QwenCliOverrides::default(),
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
        ConfiguredPolicySnapshot::from_parts(Provider::QwenCode, native, canonical, &ctx);

    assert!(snapshot.can_use_mcp_server("github").is_denied());
    // Server-level deny must propagate to tool queries on that server.
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
async fn qwen_local_override_target_returns_error() {
    let (_dir, ctx) = setup_ctx();
    let backend = QwenPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::QwenCode,
        Vec::new(),
        QwenState {
            layers: Vec::new(),
            cli: QwenCliOverrides::default(),
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
