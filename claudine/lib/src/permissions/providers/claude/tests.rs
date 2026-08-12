use super::*;
use crate::permissions::{CommandQuery, PolicyContext};

fn setup_ctx() -> (tempfile::TempDir, PolicyContext) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(home.join(".claude")).unwrap();
    std::fs::create_dir_all(repo.join(".claude")).unwrap();
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
async fn claude_backend_queries_paths_and_commands() {
    let (_dir, ctx) = setup_ctx();
    let repo_settings = ctx
        .repo_root
        .as_ref()
        .unwrap()
        .join(".claude/settings.json");
    tokio::fs::write(
        &repo_settings,
        serde_json::to_string_pretty(&json!({
            "permissions": {
                "allow": ["Bash(git status)", "Agent(Explore)"]
            },
            "sandbox": {
                "enabled": true,
                "filesystem": {
                    "allowRead": ["."],
                    "denyWrite": ["/etc"],
                    "allowWrite": ["./src"]
                },
                "network": {
                    "allowedDomains": ["github.com"]
                }
            }
        }))
        .unwrap(),
    )
    .await
    .unwrap();

    let backend = ClaudePolicyBackend;
    let sources = backend.discover_sources(&ctx).await.unwrap();
    let layers = backend.load_native_layers(&ctx, &sources).await.unwrap();
    let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
    let canonical = backend.canonicalize(&ctx, &native).await.unwrap();

    let snapshot = crate::permissions::ConfiguredPolicySnapshot::from_parts(
        Provider::Claude,
        native,
        canonical,
        &ctx,
    );

    assert!(
        snapshot
            .can_read(ctx.repo_root.as_ref().unwrap().join("README.md"))
            .is_allowed()
    );
    assert!(snapshot.can_write("src/main.rs").is_allowed());
    assert!(snapshot.can_write("./src/main.rs").is_allowed());
    let normalized = snapshot.can_write("src/../src/main.rs");
    assert!(normalized.is_allowed());
    assert!(normalized.explanation.summary.contains("workspace"));
    assert!(snapshot.can_write("/etc/hosts").is_denied());
    assert!(
        snapshot
            .can_execute(&CommandQuery::from_raw("git status"))
            .is_allowed()
    );
    assert!(snapshot.can_spawn_subagent(Some("Explore")).is_allowed());
    assert!(snapshot.can_access_domain("github.com").is_allowed());
}

#[tokio::test]
async fn claude_mutation_plan_generates_settings_overlay() {
    let (_dir, ctx) = setup_ctx();
    let backend = ClaudePolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Claude,
        Vec::new(),
        ClaudeState {
            layers: Vec::new(),
            cli: ClaudeCliOverrides::default(),
        },
    );
    let change = PolicyChange::persistent(vec![
        PolicyChangeOp::GrantWrite(PathBuf::from("/tmp/build")),
        PolicyChangeOp::AllowCommand(CommandPattern::new("npm test")),
    ]);

    let plan = backend.plan_change(&ctx, &current, &change).await.unwrap();
    let edit = &plan.persistent_plan.as_ref().unwrap().edits[0];

    assert!(edit.after_preview.contains("allowWrite"));
    assert!(edit.after_preview.contains("Bash(npm test)"));
    assert!(
        plan.one_shot_plan
            .as_ref()
            .unwrap()
            .argv
            .contains(&"--settings".to_owned())
    );
}

#[tokio::test]
async fn claude_local_override_target_uses_settings_local() {
    let (_dir, ctx) = setup_ctx();
    let backend = ClaudePolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Claude,
        Vec::new(),
        ClaudeState {
            layers: Vec::new(),
            cli: ClaudeCliOverrides::default(),
        },
    );
    let change = PolicyChange {
        operations: vec![PolicyChangeOp::GrantWrite(PathBuf::from("/tmp/build"))],
        target: crate::permissions::PolicyChangeTarget::LocalOverride,
        persistence: crate::permissions::PolicyPersistence::Persistent,
    };

    let plan = backend.plan_change(&ctx, &current, &change).await.unwrap();
    let edit = &plan.persistent_plan.as_ref().unwrap().edits[0];

    assert_eq!(edit.source_id, "claude-repo-local");
    assert!(edit.path.ends_with(".claude/settings.local.json"));
}

#[tokio::test]
async fn claude_local_override_round_trip_changes_query_result() {
    let (_dir, ctx) = setup_ctx();
    let backend = ClaudePolicyBackend;
    let current = NativeEffectivePolicy::new(
        Provider::Claude,
        Vec::new(),
        ClaudeState {
            layers: Vec::new(),
            cli: ClaudeCliOverrides::default(),
        },
    );
    let change = PolicyChange {
        operations: vec![PolicyChangeOp::GrantWrite(
            ctx.repo_root.as_ref().unwrap().join("src"),
        )],
        target: crate::permissions::PolicyChangeTarget::LocalOverride,
        persistence: crate::permissions::PolicyPersistence::Persistent,
    };

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
    let snapshot = crate::permissions::ConfiguredPolicySnapshot::from_parts(
        Provider::Claude,
        native,
        canonical,
        &ctx,
    );

    assert!(
        snapshot
            .can_write(ctx.repo_root.as_ref().unwrap().join("src/main.rs"))
            .is_allowed()
    );
    assert!(edit.path.ends_with(".claude/settings.local.json"));
}
