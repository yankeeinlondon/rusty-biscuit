use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::{Map, Value, json};

use crate::error::{ClaudineError, Result};
use crate::events::Provider;
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalApprovalMode, CanonicalPolicy, CanonicalRuleProvenance, CommandAccessRule,
    MappingFidelity, PathAccessRule, PathProtectionRule, PolicyEffect, PolicyMode, SubagentRule,
    TernaryState,
};
use crate::permissions::change::{
    CommandPattern, PolicyChange, PolicyChangeOp, PolicyChangeTarget, PolicyPersistence,
};
use crate::permissions::context::{CliPolicyInput, PolicyContext};
use crate::permissions::mutation::{
    ConfigEditPlan, OneShotMutationPlan, PersistentMutationPlan, PolicyMutationPlan,
};
use crate::permissions::native::{
    NativeEffectivePolicy, NativePolicyLayer, PolicySource, PolicySourceKind, ProviderCliOverrides,
};

#[derive(Debug, Clone, Default)]
struct PermissionSpec {
    default: Option<PolicyEffect>,
    patterns: Vec<(String, PolicyEffect)>,
}

#[derive(Debug, Clone, Default)]
struct OpenCodeConfig {
    yolo: bool,
    global: Option<PolicyEffect>,
    read: PermissionSpec,
    edit: PermissionSpec,
    bash: PermissionSpec,
    external_directory: PermissionSpec,
    task: PermissionSpec,
}

#[derive(Debug, Clone, Default)]
struct OpenCodeCliOverrides {
    yolo: bool,
    config_content: Option<OpenCodeConfig>,
}

#[derive(Debug, Clone)]
struct OpenCodeState {
    layers: Vec<(PolicySource, OpenCodeConfig)>,
    cli: OpenCodeCliOverrides,
}

#[derive(Debug, Default)]
pub(crate) struct OpenCodePolicyBackend;

impl ProviderPolicyBackend for OpenCodePolicyBackend {
    fn provider(&self) -> Provider {
        Provider::OpenCode
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            fidelity: BackendFidelity::High,
            filesystem_queries: true,
            command_queries: true,
            network_queries: false,
            mcp_queries: false,
            agent_queries: true,
            persistent_mutations: true,
            one_shot_mutations: true,
        }
    }

    fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
        let mut sources = Vec::new();
        if let Some(repo_root) = &ctx.repo_root {
            let path = repo_root.join("opencode.json");
            if path.exists() {
                sources.push(PolicySource {
                    id: "opencode-repo".to_owned(),
                    kind: PolicySourceKind::RepoConfig,
                    path: Some(path),
                    precedence: 20,
                    writable: true,
                });
            }
        }
        if let Some(home) = &ctx.home_dir {
            let path = home.join(".config/opencode/opencode.json");
            if path.exists() {
                sources.push(PolicySource {
                    id: "opencode-user".to_owned(),
                    kind: PolicySourceKind::UserConfig,
                    path: Some(path),
                    precedence: 40,
                    writable: true,
                });
            }
        }
        sources.sort_by_key(|source| source.precedence);
        Ok(sources)
    }

    fn load_native_layers(
        &self,
        _ctx: &PolicyContext,
        sources: &[PolicySource],
    ) -> Result<Vec<NativePolicyLayer>> {
        let mut layers = Vec::new();
        for source in sources {
            let path = source.path.as_ref().ok_or_else(|| {
                ClaudineError::PolicySourceDiscovery(format!(
                    "OpenCode source `{}` is missing a path",
                    source.id
                ))
            })?;
            let content = fs::read_to_string(path)?;
            let value: Value = serde_json::from_str(&content).map_err(|error| {
                ClaudineError::PolicyNativeParse {
                    source_id: source.id.clone(),
                    message: error.to_string(),
                }
            })?;
            layers.push(NativePolicyLayer::new(
                source.clone(),
                parse_opencode_config(&value),
            ));
        }
        Ok(layers)
    }

    fn parse_cli_overrides(
        &self,
        _ctx: &PolicyContext,
        input: CliPolicyInput<'_>,
    ) -> Result<ProviderCliOverrides> {
        match input {
            CliPolicyInput::None => Ok(ProviderCliOverrides::new(
                Provider::OpenCode,
                OpenCodeCliOverrides::default(),
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::OpenCode {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::OpenCode,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                    });
                }
                let typed = parsed
                    .payload::<OpenCodeCliOverrides>()
                    .cloned()
                    .ok_or_else(|| ClaudineError::PolicyCliParse {
                        provider: Provider::OpenCode,
                        message: "parsed CLI overrides had an unexpected payload type".to_owned(),
                    })?;
                Ok(ProviderCliOverrides::new(Provider::OpenCode, typed))
            }
            CliPolicyInput::Argv(argv) => {
                let mut overrides = OpenCodeCliOverrides::default();
                let mut i = 0;
                while i < argv.len() {
                    match argv[i].as_str() {
                        "--yolo" => overrides.yolo = true,
                        _ => {}
                    }
                    i += 1;
                }
                Ok(ProviderCliOverrides::new(Provider::OpenCode, overrides))
            }
        }
    }

    fn compose_native_policy(
        &self,
        _ctx: &PolicyContext,
        layers: &[NativePolicyLayer],
        cli: Option<&ProviderCliOverrides>,
    ) -> Result<NativeEffectivePolicy> {
        let typed_layers = layers
            .iter()
            .map(|layer| {
                let config = layer.payload::<OpenCodeConfig>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "OpenCode layer payload type mismatch".to_owned(),
                    }
                })?;
                Ok((layer.source.clone(), config))
            })
            .collect::<Result<Vec<_>>>()?;
        let cli = cli
            .and_then(|value| value.payload::<OpenCodeCliOverrides>().cloned())
            .unwrap_or_default();
        let mut sources = typed_layers
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        if cli.yolo || cli.config_content.is_some() {
            sources.push(PolicySource {
                id: "opencode-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            });
        }
        Ok(NativeEffectivePolicy::new(
            Provider::OpenCode,
            sources,
            OpenCodeState {
                layers: typed_layers,
                cli,
            },
        ))
    }

    fn canonicalize(
        &self,
        _ctx: &PolicyContext,
        native: &NativeEffectivePolicy,
    ) -> Result<CanonicalPolicy> {
        let state =
            native
                .payload::<OpenCodeState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "opencode-effective".to_owned(),
                    message: "OpenCode effective policy payload type mismatch".to_owned(),
                })?;

        let mut policy = CanonicalPolicy::empty(
            Provider::OpenCode,
            if state.cli.yolo || state.cli.config_content.is_some() {
                PolicyMode::Effective
            } else {
                PolicyMode::Configured
            },
        );

        for source in &native.sources {
            if let Some(path) = &source.path {
                policy
                    .axes
                    .filesystem
                    .protected_config_paths
                    .push(PathProtectionRule {
                        pattern: path.to_string_lossy().into_owned(),
                        description: "OpenCode configuration file".to_owned(),
                        provenance: CanonicalRuleProvenance::approximate(
                            source.id.clone(),
                            "opencode.json",
                            "Configuration files should be treated as protected.",
                        ),
                    });
            }
        }

        for (source, config) in &state.layers {
            push_opencode_rules(&mut policy, source, config);
        }
        if let Some(config) = &state.cli.config_content {
            let source = PolicySource {
                id: "opencode-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            };
            push_opencode_rules(&mut policy, &source, config);
        }

        let effective_yolo = state.cli.yolo || state.layers.iter().any(|(_, cfg)| cfg.yolo);
        policy.axes.runtime.approval_mode = Some(if effective_yolo {
            CanonicalApprovalMode::AutoApprove
        } else {
            CanonicalApprovalMode::AlwaysAsk
        });
        policy.axes.runtime.can_bypass_permissions = if effective_yolo {
            TernaryState::Yes
        } else {
            TernaryState::No
        };

        Ok(policy)
    }

    fn plan_change(
        &self,
        ctx: &PolicyContext,
        current: &NativeEffectivePolicy,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan> {
        let (source_id, path) = choose_target(ctx, current, change.target)?;
        let before_text = fs::read_to_string(&path).ok();
        let mut root = before_text
            .as_deref()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .unwrap_or_else(|| json!({}));

        for operation in &change.operations {
            apply_opencode_operation(&mut root, operation)?;
        }

        let after_preview = serde_json::to_string_pretty(&root)? + "\n";
        let one_shot_plan = build_one_shot_plan(change)?;

        Ok(PolicyMutationPlan {
            provider: Provider::OpenCode,
            persistent_plan: if change.persistence == PolicyPersistence::OneShot {
                None
            } else {
                Some(PersistentMutationPlan {
                    edits: vec![ConfigEditPlan {
                        source_id,
                        path,
                        description: "Update OpenCode permissions".to_owned(),
                        before_preview: before_text,
                        after_preview,
                    }],
                    fidelity: MappingFidelity::Exact,
                })
            },
            one_shot_plan,
            warnings: Vec::new(),
            supported: true,
        })
    }
}

fn parse_opencode_config(value: &Value) -> OpenCodeConfig {
    let permission = value
        .get("permission")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    OpenCodeConfig {
        yolo: value.get("yolo").and_then(Value::as_bool).unwrap_or(false),
        global: permission.get("*").and_then(parse_effect_value),
        read: parse_permission_spec(permission.get("read")),
        edit: parse_permission_spec(permission.get("edit")),
        bash: parse_permission_spec(permission.get("bash")),
        external_directory: parse_permission_spec(permission.get("external_directory")),
        task: parse_permission_spec(permission.get("task")),
    }
}

fn parse_permission_spec(value: Option<&Value>) -> PermissionSpec {
    match value {
        Some(Value::String(_)) => PermissionSpec {
            default: value.and_then(parse_effect_value),
            patterns: Vec::new(),
        },
        Some(Value::Object(map)) => PermissionSpec {
            default: map.get("*").and_then(parse_effect_value),
            patterns: map
                .iter()
                .filter(|(key, _)| key.as_str() != "*")
                .filter_map(|(key, value)| {
                    parse_effect_value(value).map(|effect| (key.clone(), effect))
                })
                .collect(),
        },
        _ => PermissionSpec::default(),
    }
}

fn parse_effect_value(value: &Value) -> Option<PolicyEffect> {
    match value.as_str() {
        Some("allow") => Some(PolicyEffect::Allow),
        Some("ask") => Some(PolicyEffect::Ask),
        Some("deny") => Some(PolicyEffect::Deny),
        _ => None,
    }
}

fn push_opencode_rules(
    policy: &mut CanonicalPolicy,
    source: &PolicySource,
    config: &OpenCodeConfig,
) {
    push_permission_spec(
        &mut policy.axes.filesystem.read_rules,
        source,
        &config.read,
        config.global,
        "permission.read",
        "*",
        None,
    );
    push_permission_spec(
        &mut policy.axes.filesystem.write_rules,
        source,
        &config.edit,
        config.global,
        "permission.edit",
        "*",
        None,
    );
    push_permission_spec(
        &mut policy.axes.filesystem.traversal_rules,
        source,
        &config.external_directory,
        config.global,
        "permission.external_directory",
        "*",
        None,
    );

    for (pattern, effect) in &config.bash.patterns {
        policy.axes.commands.shell_rules.push(CommandAccessRule {
            pattern: pattern.clone(),
            effect: *effect,
            provenance: CanonicalRuleProvenance::exact(source.id.clone(), "permission.bash"),
        });
    }
    if let Some(effect) = config.bash.default.or(config.global) {
        policy.axes.commands.shell_rules.push(CommandAccessRule {
            pattern: "*".to_owned(),
            effect,
            provenance: CanonicalRuleProvenance::exact(source.id.clone(), "permission.bash"),
        });
    }

    for (pattern, effect) in &config.task.patterns {
        policy.axes.agents.subagent_rules.push(SubagentRule {
            name: Some(pattern.clone()),
            effect: *effect,
            provenance: CanonicalRuleProvenance::exact(source.id.clone(), "permission.task"),
        });
    }
    if let Some(effect) = config.task.default.or(config.global) {
        policy.axes.agents.subagent_rules.push(SubagentRule {
            name: None,
            effect,
            provenance: CanonicalRuleProvenance::exact(source.id.clone(), "permission.task"),
        });
    }
}

fn push_permission_spec(
    target: &mut Vec<PathAccessRule>,
    source: &PolicySource,
    spec: &PermissionSpec,
    global: Option<PolicyEffect>,
    native_reference: &str,
    default_pattern: &str,
    note: Option<&str>,
) {
    for (pattern, effect) in &spec.patterns {
        target.push(PathAccessRule {
            pattern: pattern.clone(),
            effect: *effect,
            provenance: CanonicalRuleProvenance::exact(source.id.clone(), native_reference),
        });
    }
    if let Some(effect) = spec.default.or(global) {
        target.push(PathAccessRule {
            pattern: default_pattern.to_owned(),
            effect,
            provenance: if let Some(note) = note {
                CanonicalRuleProvenance::approximate(source.id.clone(), native_reference, note)
            } else {
                CanonicalRuleProvenance::exact(source.id.clone(), native_reference)
            },
        });
    }
}

fn choose_target(
    ctx: &PolicyContext,
    current: &NativeEffectivePolicy,
    target: PolicyChangeTarget,
) -> Result<(String, PathBuf)> {
    let repo_path = ctx
        .repo_root
        .as_ref()
        .map(|root| root.join("opencode.json"));
    let user_path = ctx
        .home_dir
        .as_ref()
        .map(|home| home.join(".config/opencode/opencode.json"));
    match target {
        PolicyChangeTarget::UserConfig => user_path
            .map(|path| ("opencode-user".to_owned(), path))
            .ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "OpenCode user config path is unavailable".to_owned(),
                )
            }),
        PolicyChangeTarget::RepoConfig | PolicyChangeTarget::LocalOverride => repo_path
            .map(|path| ("opencode-repo".to_owned(), path))
            .ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "OpenCode repo config path is unavailable".to_owned(),
                )
            }),
        PolicyChangeTarget::Auto => {
            let has_repo = current
                .sources
                .iter()
                .any(|source| source.kind == PolicySourceKind::RepoConfig);
            if has_repo
                && ctx.trust.is_trusted == Some(true)
                && let Some(path) = repo_path.clone()
            {
                return Ok(("opencode-repo".to_owned(), path));
            }
            user_path
                .map(|path| ("opencode-user".to_owned(), path))
                .or_else(|| repo_path.map(|path| ("opencode-repo".to_owned(), path)))
                .ok_or_else(|| {
                    ClaudineError::PolicyAmbiguousContext(
                        "Could not determine OpenCode mutation target".to_owned(),
                    )
                })
        }
    }
}

fn apply_opencode_operation(root: &mut Value, operation: &PolicyChangeOp) -> Result<()> {
    match operation {
        PolicyChangeOp::GrantRead(path) => set_permission_pattern(
            root,
            &["permission", "read"],
            &path.to_string_lossy(),
            "allow",
        ),
        PolicyChangeOp::GrantWrite(path) => set_permission_pattern(
            root,
            &["permission", "edit"],
            &path.to_string_lossy(),
            "allow",
        ),
        PolicyChangeOp::DenyWrite(path) => set_permission_pattern(
            root,
            &["permission", "edit"],
            &path.to_string_lossy(),
            "deny",
        ),
        PolicyChangeOp::AllowCommand(CommandPattern { raw }) => {
            set_permission_pattern(root, &["permission", "bash"], raw, "allow")
        }
        PolicyChangeOp::RequireApprovalForCommand(CommandPattern { raw }) => {
            set_permission_pattern(root, &["permission", "bash"], raw, "ask")
        }
        PolicyChangeOp::DenyCommand(CommandPattern { raw }) => {
            set_permission_pattern(root, &["permission", "bash"], raw, "deny")
        }
        PolicyChangeOp::AllowSubagent(Some(name)) => {
            set_permission_pattern(root, &["permission", "task"], name, "allow")
        }
        PolicyChangeOp::DenySubagent(Some(name)) => {
            set_permission_pattern(root, &["permission", "task"], name, "deny")
        }
        PolicyChangeOp::SetApprovalMode(mode) => {
            let yolo = matches!(mode, CanonicalApprovalMode::AutoApprove);
            *ensure_json_value(root, &["yolo"]) = Value::Bool(yolo);
        }
        other => {
            return Err(ClaudineError::PolicyUnsupportedMutation {
                provider: Provider::OpenCode,
                op: format!("{other:?}"),
            });
        }
    }
    Ok(())
}

fn build_one_shot_plan(change: &PolicyChange) -> Result<Option<OneShotMutationPlan>> {
    let mut argv = Vec::new();
    let mut env = BTreeMap::new();
    let mut overlay = json!({});
    let mut needs_overlay = false;

    for operation in &change.operations {
        match operation {
            PolicyChangeOp::SetApprovalMode(mode)
                if matches!(mode, CanonicalApprovalMode::AutoApprove) =>
            {
                argv.push("--yolo".to_owned());
            }
            PolicyChangeOp::SetApprovalMode(_) => {
                apply_opencode_operation(&mut overlay, operation)?;
                needs_overlay = true;
            }
            _ => {
                apply_opencode_operation(&mut overlay, operation)?;
                needs_overlay = true;
            }
        }
    }

    if needs_overlay {
        env.insert(
            "OPENCODE_CONFIG_CONTENT".to_owned(),
            serde_json::to_string(&overlay)?,
        );
    }

    Ok(Some(OneShotMutationPlan {
        argv,
        env,
        fidelity: MappingFidelity::Exact,
    }))
}

fn set_permission_pattern(root: &mut Value, path: &[&str], pattern: &str, effect: &str) {
    let target = ensure_json_value(root, path);
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    target
        .as_object_mut()
        .expect("object")
        .insert(pattern.to_owned(), Value::String(effect.to_owned()));
}

fn ensure_json_value<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Value {
    let mut current = root;
    for key in path {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        current = current
            .as_object_mut()
            .expect("object")
            .entry((*key).to_owned())
            .or_insert(Value::Null);
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{CommandQuery, ConfiguredPolicySnapshot, PolicyContext};

    fn setup_ctx() -> (tempfile::TempDir, PolicyContext) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let repo = dir.path().join("repo");
        fs::create_dir_all(home.join(".config/opencode")).unwrap();
        fs::create_dir_all(&repo).unwrap();
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

    #[test]
    fn opencode_backend_queries_bash_and_tasks() {
        let (_dir, ctx) = setup_ctx();
        let path = ctx.repo_root.as_ref().unwrap().join("opencode.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "permission": {
                    "*": "ask",
                    "read": "allow",
                    "edit": "ask",
                    "bash": {
                        "*": "ask",
                        "git *": "allow",
                        "rm *": "deny"
                    },
                    "task": {
                        "*": "deny",
                        "code-reviewer": "ask"
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let backend = OpenCodePolicyBackend;
        let sources = backend.discover_sources(&ctx).unwrap();
        let layers = backend.load_native_layers(&ctx, &sources).unwrap();
        let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
        let canonical = backend.canonicalize(&ctx, &native).unwrap();
        let snapshot =
            ConfiguredPolicySnapshot::from_parts(Provider::OpenCode, native, canonical, &ctx);

        assert!(snapshot.can_read("/tmp/file.txt").is_allowed());
        assert!(
            snapshot
                .can_execute(&CommandQuery::from_raw("git status"))
                .is_allowed()
        );
        assert!(
            snapshot
                .can_execute(&CommandQuery::from_raw("rm -rf build"))
                .is_denied()
        );
        assert!(snapshot.can_spawn_subagent(Some("code-reviewer")).is_ask());
    }

    #[test]
    fn opencode_one_shot_plan_uses_env_override() {
        let (_dir, ctx) = setup_ctx();
        let backend = OpenCodePolicyBackend;
        let current = NativeEffectivePolicy::new(
            Provider::OpenCode,
            Vec::new(),
            OpenCodeState {
                layers: Vec::new(),
                cli: OpenCodeCliOverrides::default(),
            },
        );
        let change = PolicyChange::one_shot(vec![
            PolicyChangeOp::AllowCommand(CommandPattern::new("git status")),
            PolicyChangeOp::SetApprovalMode(CanonicalApprovalMode::AutoApprove),
        ]);

        let plan = backend.plan_change(&ctx, &current, &change).unwrap();
        let one_shot = plan.one_shot_plan.unwrap();

        assert!(one_shot.argv.contains(&"--yolo".to_owned()));
        assert!(one_shot.env.contains_key("OPENCODE_CONFIG_CONTENT"));
    }
}
