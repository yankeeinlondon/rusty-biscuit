use super::*;
use crate::permissions::{CommandQuery, ConfiguredPolicySnapshot, PolicyContext};

fn setup_ctx() -> (tempfile::TempDir, PolicyContext) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(home.join(".gemini/policies")).unwrap();
    std::fs::create_dir_all(repo.join(".gemini/policies")).unwrap();
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
async fn gemini_backend_queries_policy_rules() {
    let (_dir, ctx) = setup_ctx();
    tokio::fs::write(
        ctx.repo_root
            .as_ref()
            .unwrap()
            .join(".gemini/settings.json"),
        serde_json::to_string_pretty(&json!({
            "general": { "defaultApprovalMode": "default" },
            "tools": { "sandbox": { "enabled": true, "type": "docker" } },
            "mcp": { "allowed": ["github"] }
        }))
        .unwrap(),
    )
    .await
    .unwrap();
    tokio::fs::write(
        ctx.repo_root
            .as_ref()
            .unwrap()
            .join(".gemini/policies/default.toml"),
        r#"
[[rule]]
toolName = "shell"
commandPrefix = "git status"
decision = "allow"
priority = 100

[[rule]]
subagent = "reviewer"
decision = "deny"
priority = 100
"#,
    )
    .await
    .unwrap();

    let backend = GeminiPolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Gemini, native, canonical, &ctx);

    assert!(
        snapshot
            .can_execute(&CommandQuery::from_raw("git status"))
            .is_allowed()
    );
    assert!(snapshot.can_write("src/main.rs").is_ask());
    assert!(snapshot.can_use_mcp_server("github").is_allowed());
    assert!(snapshot.can_spawn_subagent(Some("reviewer")).is_denied());
}

#[tokio::test]
async fn gemini_mutation_plan_builds_settings_and_policy_files() {
    let (_dir, ctx) = setup_ctx();
    let backend = GeminiPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Gemini,
        Vec::new(),
        GeminiState {
            layers: Vec::new(),
            cli: GeminiCliOverrides::default(),
        },
    );
    let change = PolicyChange::persistent(vec![
        PolicyChangeOp::SetApprovalMode(CanonicalApprovalMode::AutoApprove),
        PolicyChangeOp::AllowCommand(CommandPattern::new("npm test")),
    ]);

    let plan = backend.plan_change(&ctx, &current, &change).await.unwrap();
    assert_eq!(plan.persistent_plan.as_ref().unwrap().edits.len(), 2);
    assert!(
        plan.one_shot_plan
            .as_ref()
            .unwrap()
            .argv
            .contains(&"--approval-mode".to_owned())
    );
}

#[tokio::test]
async fn gemini_unknown_trust_skips_repo_sources_and_queries_are_unknown() {
    let (_dir, mut ctx) = setup_ctx();
    ctx.trust.is_trusted = None;
    ctx.trust.source = crate::permissions::TrustSource::Unknown;

    tokio::fs::write(
        ctx.repo_root
            .as_ref()
            .unwrap()
            .join(".gemini/settings.json"),
        serde_json::to_string_pretty(&json!({
            "general": { "defaultApprovalMode": "yolo" }
        }))
        .unwrap(),
    )
    .await
    .unwrap();
    tokio::fs::write(
        ctx.home_dir.as_ref().unwrap().join(".gemini/settings.json"),
        serde_json::to_string_pretty(&json!({
            "general": { "defaultApprovalMode": "default" }
        }))
        .unwrap(),
    )
    .await
    .unwrap();

    let backend = GeminiPolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].id, "gemini-user-settings");

    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Gemini, native, canonical, &ctx);
    let result = snapshot.can_write("src/main.rs");

    assert!(result.is_unknown());
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.code == "gemini.trust_unknown")
    );
}

#[tokio::test]
async fn gemini_round_trip_mutation_changes_query_result() {
    let (_dir, ctx) = setup_ctx();
    let backend = GeminiPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Gemini,
        Vec::new(),
        GeminiState {
            layers: Vec::new(),
            cli: GeminiCliOverrides::default(),
        },
    );
    let change = PolicyChange::persistent(vec![
        PolicyChangeOp::SetApprovalMode(CanonicalApprovalMode::AutoApprove),
        PolicyChangeOp::AllowCommand(CommandPattern::new("npm test")),
        PolicyChangeOp::AllowMcpServer("filesystem".to_owned()),
    ]);

    let plan = backend.plan_change(&ctx, &current, &change).await.unwrap();
    for edit in &plan.persistent_plan.as_ref().unwrap().edits {
        tokio::fs::create_dir_all(edit.path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&edit.path, edit.after_preview.as_bytes())
            .await
            .unwrap();
    }

    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();
    let snapshot =
        ConfiguredPolicySnapshot::from_parts(Provider::Gemini, native, canonical, &ctx);

    assert!(
        snapshot
            .can_execute(&CommandQuery::from_raw("npm test"))
            .is_allowed()
    );
    assert!(snapshot.can_use_mcp_server("filesystem").is_allowed());
    // Tool queries on an allowed server inherit the server-level allow,
    // even without an explicit tool-level rule.
    assert!(
        snapshot
            .can_use_mcp_tool("filesystem", "read_file")
            .is_allowed()
    );
}

#[tokio::test]
async fn gemini_local_override_target_returns_error() {
    let (_dir, ctx) = setup_ctx();
    let backend = GeminiPolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Gemini,
        Vec::new(),
        GeminiState {
            layers: Vec::new(),
            cli: GeminiCliOverrides::default(),
        },
    );
    let change = PolicyChange {
        operations: vec![PolicyChangeOp::AllowMcpServer("fs".to_owned())],
        target: crate::permissions::PolicyChangeTarget::LocalOverride,
        persistence: crate::permissions::PolicyPersistence::Persistent,
    };

    let result = backend.plan_change(&ctx, &current, &change).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("LocalOverride"));
}
