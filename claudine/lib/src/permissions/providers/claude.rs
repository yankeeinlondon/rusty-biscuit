use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::fs;

use crate::error::{ClaudineError, Result};
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalApprovalMode, CanonicalPolicy, CanonicalRuleProvenance, CanonicalSandboxMode,
    CommandAccessRule, DomainAccessRule, MappingFidelity, PathAccessRule, PathProtectionRule,
    PolicyEffect, PolicyMode, PolicyWarning, SubagentRule, TernaryState,
};
use crate::permissions::change::{
    CommandPattern, PolicyChange, PolicyChangeOp, PolicyChangeTarget, PolicyPersistence,
};
use crate::permissions::context::{CliPolicyInput, PolicyContext};
use crate::permissions::json_utils::{ensure_json_array, ensure_json_value};
use crate::permissions::mutation::{
    ConfigEditPlan, OneShotMutationPlan, PersistentMutationPlan, PolicyMutationPlan,
};
use crate::permissions::native::{
    NativeEffectivePolicy, NativePolicyLayer, PolicySource, PolicySourceKind, ProviderCliOverrides,
};
use crate::provider::Provider;

#[derive(Debug, Clone, Default)]
struct ClaudeConfig {
    allow: Vec<String>,
    ask: Vec<String>,
    deny: Vec<String>,
    additional_directories: Vec<String>,
    default_mode: Option<String>,
    sandbox_enabled: Option<bool>,
    allow_read: Vec<String>,
    deny_read: Vec<String>,
    allow_write: Vec<String>,
    deny_write: Vec<String>,
    allowed_domains: Vec<String>,
    allow_local_binding: Option<bool>,
}

#[derive(Debug, Clone, Default)]
struct ClaudeCliOverrides {
    allow: Vec<String>,
    deny: Vec<String>,
    permission_mode: Option<String>,
    dangerous_skip: bool,
    settings_override: Option<ClaudeConfig>,
}

#[derive(Debug, Clone)]
struct ClaudeState {
    layers: Vec<(PolicySource, ClaudeConfig)>,
    cli: ClaudeCliOverrides,
}

#[derive(Debug, Default)]
pub(crate) struct ClaudePolicyBackend;

#[async_trait]
impl ProviderPolicyBackend for ClaudePolicyBackend {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::full(BackendFidelity::High)
    }

    async fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
        let mut sources = Vec::new();

        if let Some(system_root) = &ctx.system_root {
            let path = system_root.join("etc/claude-code/managed-settings.json");
            if path.exists() {
                sources.push(PolicySource {
                    id: "claude-managed".to_owned(),
                    kind: PolicySourceKind::ManagedConfig,
                    path: Some(path),
                    precedence: 10,
                    writable: false,
                });
            }
        }

        if let Some(repo_root) = &ctx.repo_root {
            let local = repo_root.join(".claude/settings.local.json");
            if local.exists() {
                sources.push(PolicySource {
                    id: "claude-repo-local".to_owned(),
                    kind: PolicySourceKind::LocalOverride,
                    path: Some(local),
                    precedence: 20,
                    writable: true,
                });
            }

            let shared = repo_root.join(".claude/settings.json");
            if shared.exists() {
                sources.push(PolicySource {
                    id: "claude-repo".to_owned(),
                    kind: PolicySourceKind::RepoConfig,
                    path: Some(shared),
                    precedence: 30,
                    writable: true,
                });
            }
        }

        if let Some(home) = &ctx.home_dir {
            let settings = home.join(".claude/settings.json");
            if settings.exists() {
                sources.push(PolicySource {
                    id: "claude-user".to_owned(),
                    kind: PolicySourceKind::UserConfig,
                    path: Some(settings),
                    precedence: 40,
                    writable: true,
                });
            }
        }

        sources.sort_by_key(|source| source.precedence);
        Ok(sources)
    }

    async fn load_native_layers(
        &self,
        _ctx: &PolicyContext,
        sources: &[PolicySource],
    ) -> Result<Vec<NativePolicyLayer>> {
        let mut layers = Vec::new();
        for source in sources {
            let path = source.path.as_ref().ok_or_else(|| {
                ClaudineError::PolicySourceDiscovery(format!(
                    "Claude source `{}` is missing a path",
                    source.id
                ))
            })?;
            let content = fs::read_to_string(path).await?;
            let value: Value = serde_json::from_str(&content).map_err(|error| {
                ClaudineError::PolicyNativeParse {
                    source_id: source.id.clone(),
                    message: error.to_string(),
                }
            })?;
            layers.push(NativePolicyLayer::new(
                source.clone(),
                parse_claude_config(&value),
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
                Provider::Claude,
                ClaudeCliOverrides::default(),
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::Claude {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::Claude,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                    });
                }
                let typed = parsed
                    .payload::<ClaudeCliOverrides>()
                    .cloned()
                    .ok_or_else(|| ClaudineError::PolicyCliParse {
                        provider: Provider::Claude,
                        message: "parsed CLI overrides had an unexpected payload type".to_owned(),
                    })?;
                Ok(ProviderCliOverrides::new(Provider::Claude, typed))
            }
            CliPolicyInput::Argv(argv) => {
                let mut overrides = ClaudeCliOverrides::default();
                let mut i = 0;
                while i < argv.len() {
                    match argv[i].as_str() {
                        "--allowedTools" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.allow.push(value.clone());
                                i += 1;
                            }
                        }
                        "--disallowedTools" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.deny.push(value.clone());
                                i += 1;
                            }
                        }
                        "--permission-mode" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.permission_mode = Some(value.clone());
                                i += 1;
                            }
                        }
                        "--dangerously-skip-permissions" => {
                            overrides.dangerous_skip = true;
                        }
                        "--settings" => {
                            if let Some(value) = argv.get(i + 1) {
                                if value.trim_start().starts_with('{') {
                                    let parsed: Value =
                                        serde_json::from_str(value).map_err(|error| {
                                            ClaudineError::PolicyCliParse {
                                                provider: Provider::Claude,
                                                message: error.to_string(),
                                            }
                                        })?;
                                    overrides.settings_override =
                                        Some(parse_claude_config(&parsed));
                                }
                                i += 1;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                Ok(ProviderCliOverrides::new(Provider::Claude, overrides))
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
                let config = layer.payload::<ClaudeConfig>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "Claude layer payload type mismatch".to_owned(),
                    }
                })?;
                Ok((layer.source.clone(), config))
            })
            .collect::<Result<Vec<_>>>()?;

        let cli = cli
            .and_then(|overrides| overrides.payload::<ClaudeCliOverrides>().cloned())
            .unwrap_or_default();

        let mut sources = typed_layers
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        if !cli.allow.is_empty()
            || !cli.deny.is_empty()
            || cli.permission_mode.is_some()
            || cli.dangerous_skip
            || cli.settings_override.is_some()
        {
            sources.push(PolicySource {
                id: "claude-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            });
        }

        Ok(NativeEffectivePolicy::new(
            Provider::Claude,
            sources,
            ClaudeState {
                layers: typed_layers,
                cli,
            },
        ))
    }

    async fn canonicalize(
        &self,
        ctx: &PolicyContext,
        native: &NativeEffectivePolicy,
    ) -> Result<CanonicalPolicy> {
        let state =
            native
                .payload::<ClaudeState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "claude-effective".to_owned(),
                    message: "Claude effective policy payload type mismatch".to_owned(),
                })?;

        let mut policy = CanonicalPolicy::empty(
            Provider::Claude,
            if has_cli_overrides(&state.cli) {
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
                        description: "Claude configuration file".to_owned(),
                        provenance: CanonicalRuleProvenance::approximate(
                            source.id.clone(),
                            "settings.json",
                            "Agent configuration should be treated as protected.",
                        ),
                    });
            }
        }

        let workspace_root = ctx.repo_root.as_deref().unwrap_or(&ctx.cwd);

        for (source, config) in &state.layers {
            push_claude_rules(&mut policy, source, config, workspace_root);
        }

        if let Some(settings) = &state.cli.settings_override {
            let source = PolicySource {
                id: "claude-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            };
            push_claude_rules(&mut policy, &source, settings, workspace_root);
        }

        for rule in &state.cli.deny {
            push_permission_rule(
                &mut policy,
                "claude-cli",
                "cli.disallowedTools",
                rule,
                PolicyEffect::Deny,
            );
        }
        for rule in &state.cli.allow {
            push_permission_rule(
                &mut policy,
                "claude-cli",
                "cli.allowedTools",
                rule,
                PolicyEffect::Allow,
            );
        }

        let resolved_mode = state
            .cli
            .permission_mode
            .as_deref()
            .map(str::to_owned)
            .or_else(|| {
                state
                    .layers
                    .iter()
                    .find_map(|(_, cfg)| cfg.default_mode.clone())
            });

        policy.axes.runtime.approval_mode =
            resolved_mode.as_deref().and_then(map_claude_approval_mode);
        policy.axes.runtime.sandbox_mode = if state.cli.dangerous_skip {
            Some(CanonicalSandboxMode::None)
        } else {
            state
                .layers
                .iter()
                .find_map(|(_, cfg)| cfg.sandbox_enabled)
                .map(|enabled| {
                    if enabled {
                        CanonicalSandboxMode::Partial
                    } else {
                        CanonicalSandboxMode::None
                    }
                })
        };
        policy.axes.runtime.can_bypass_permissions =
            if state.cli.dangerous_skip || resolved_mode.as_deref() == Some("bypassPermissions") {
                TernaryState::Yes
            } else {
                TernaryState::No
            };

        policy.axes.network.enabled = if state
            .layers
            .iter()
            .any(|(_, cfg)| !cfg.allowed_domains.is_empty())
        {
            TernaryState::Yes
        } else {
            TernaryState::Unknown
        };

        if ctx.repo_root.is_none() {
            policy.warnings.push(PolicyWarning {
                code: "claude.no_repo_root".to_owned(),
                message: "Repo-root scoped Claude settings could not be discovered.".to_owned(),
                source_id: None,
            });
        }

        Ok(policy)
    }

    async fn plan_change(
        &self,
        ctx: &PolicyContext,
        current: &NativeEffectivePolicy,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan> {
        let (source_id, path) = choose_target_path(ctx, current, change.target)?;
        let before_text = fs::read_to_string(&path).await.ok();
        let mut root = before_text
            .as_deref()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .unwrap_or_else(|| json!({}));

        for operation in &change.operations {
            apply_persistent_operation(&mut root, operation)?;
        }

        let after_preview = serde_json::to_string_pretty(&root)? + "\n";

        let persistent_plan = if change.persistence == PolicyPersistence::Persistent
            || change.persistence == PolicyPersistence::OneShot
        {
            Some(PersistentMutationPlan {
                edits: vec![ConfigEditPlan {
                    source_id: source_id.clone(),
                    path: path.clone(),
                    description: "Update Claude settings".to_owned(),
                    before_preview: before_text.clone(),
                    after_preview: after_preview.clone(),
                }],
                fidelity: MappingFidelity::Exact,
            })
        } else {
            None
        };

        let one_shot_plan = build_one_shot_plan(change)?;

        Ok(PolicyMutationPlan {
            provider: Provider::Claude,
            persistent_plan: if change.persistence == PolicyPersistence::OneShot {
                None
            } else {
                persistent_plan
            },
            one_shot_plan,
            warnings: Vec::new(),
            supported: true,
        })
    }
}

fn parse_claude_config(value: &Value) -> ClaudeConfig {
    let permissions = value
        .get("permissions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let sandbox = value
        .get("sandbox")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let sandbox_fs = sandbox
        .get("filesystem")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let sandbox_network = sandbox
        .get("network")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    ClaudeConfig {
        allow: string_array(permissions.get("allow")),
        ask: string_array(permissions.get("ask")),
        deny: string_array(permissions.get("deny")),
        additional_directories: string_array(permissions.get("additionalDirectories")),
        default_mode: permissions
            .get("defaultMode")
            .and_then(Value::as_str)
            .map(str::to_owned),
        sandbox_enabled: sandbox.get("enabled").and_then(Value::as_bool),
        allow_read: string_array(sandbox_fs.get("allowRead")),
        deny_read: string_array(sandbox_fs.get("denyRead")),
        allow_write: string_array(sandbox_fs.get("allowWrite")),
        deny_write: string_array(sandbox_fs.get("denyWrite")),
        allowed_domains: string_array(sandbox_network.get("allowedDomains")),
        allow_local_binding: sandbox_network
            .get("allowLocalBinding")
            .and_then(Value::as_bool),
    }
}

fn push_claude_rules(
    policy: &mut CanonicalPolicy,
    source: &PolicySource,
    config: &ClaudeConfig,
    workspace_root: &std::path::Path,
) {
    for pattern in &config.deny_read {
        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: normalize_relative_pattern(pattern, workspace_root),
            effect: PolicyEffect::Deny,
            provenance: CanonicalRuleProvenance::exact(
                source.id.clone(),
                "sandbox.filesystem.denyRead",
            ),
        });
    }
    for pattern in &config.allow_read {
        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: normalize_relative_pattern(pattern, workspace_root),
            effect: PolicyEffect::Allow,
            provenance: CanonicalRuleProvenance::exact(
                source.id.clone(),
                "sandbox.filesystem.allowRead",
            ),
        });
    }
    for pattern in &config.deny_write {
        policy.axes.filesystem.write_rules.push(PathAccessRule {
            pattern: normalize_relative_pattern(pattern, workspace_root),
            effect: PolicyEffect::Deny,
            provenance: CanonicalRuleProvenance::exact(
                source.id.clone(),
                "sandbox.filesystem.denyWrite",
            ),
        });
    }
    for pattern in &config.allow_write {
        policy.axes.filesystem.write_rules.push(PathAccessRule {
            pattern: normalize_relative_pattern(pattern, workspace_root),
            effect: PolicyEffect::Allow,
            provenance: CanonicalRuleProvenance::exact(
                source.id.clone(),
                "sandbox.filesystem.allowWrite",
            ),
        });
    }
    for pattern in &config.additional_directories {
        let normalized = normalize_relative_pattern(pattern, workspace_root);
        let provenance = CanonicalRuleProvenance::approximate(
            source.id.clone(),
            "permissions.additionalDirectories",
            "Mapped additional directories to read, write, and traverse access.",
        );
        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: normalized.clone(),
            effect: PolicyEffect::Allow,
            provenance: provenance.clone(),
        });
        policy.axes.filesystem.write_rules.push(PathAccessRule {
            pattern: normalized.clone(),
            effect: PolicyEffect::Allow,
            provenance: provenance.clone(),
        });
        policy.axes.filesystem.traversal_rules.push(PathAccessRule {
            pattern: normalized,
            effect: PolicyEffect::Allow,
            provenance,
        });
    }

    for domain in &config.allowed_domains {
        policy.axes.network.domain_rules.push(DomainAccessRule {
            pattern: domain.clone(),
            effect: PolicyEffect::Allow,
            provenance: CanonicalRuleProvenance::exact(
                source.id.clone(),
                "sandbox.network.allowedDomains",
            ),
        });
    }
    if let Some(binding) = config.allow_local_binding {
        policy.axes.network.local_binding = if binding {
            TernaryState::Yes
        } else {
            TernaryState::No
        };
    }

    for rule in &config.deny {
        push_permission_rule(
            policy,
            &source.id,
            "permissions.deny",
            rule,
            PolicyEffect::Deny,
        );
    }
    for rule in &config.ask {
        push_permission_rule(
            policy,
            &source.id,
            "permissions.ask",
            rule,
            PolicyEffect::Ask,
        );
    }
    for rule in &config.allow {
        push_permission_rule(
            policy,
            &source.id,
            "permissions.allow",
            rule,
            PolicyEffect::Allow,
        );
    }
}

fn push_permission_rule(
    policy: &mut CanonicalPolicy,
    source_id: &str,
    native_reference: &str,
    raw: &str,
    effect: PolicyEffect,
) {
    let provenance =
        CanonicalRuleProvenance::exact(source_id.to_owned(), native_reference.to_owned());

    if let Some(specifier) = raw
        .strip_prefix("Bash(")
        .and_then(|value| value.strip_suffix(')'))
    {
        policy.axes.commands.shell_rules.push(CommandAccessRule {
            pattern: if specifier.is_empty() {
                "*".to_owned()
            } else {
                specifier.to_owned()
            },
            effect,
            provenance,
        });
        return;
    }

    if raw == "Bash" {
        policy.axes.commands.shell_rules.push(CommandAccessRule {
            pattern: "*".to_owned(),
            effect,
            provenance,
        });
        return;
    }

    if let Some(specifier) = raw
        .strip_prefix("Agent(")
        .and_then(|value| value.strip_suffix(')'))
    {
        policy.axes.agents.subagent_rules.push(SubagentRule {
            name: Some(specifier.to_owned()),
            effect,
            provenance,
        });
        return;
    }

    if raw == "Read" {
        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect,
            provenance: CanonicalRuleProvenance::approximate(
                source_id.to_owned(),
                native_reference.to_owned(),
                "Mapped Claude Read permission to all path reads.",
            ),
        });
        return;
    }

    if raw == "Edit" || raw == "Write" {
        policy.axes.filesystem.write_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect,
            provenance: CanonicalRuleProvenance::approximate(
                source_id.to_owned(),
                native_reference.to_owned(),
                "Mapped Claude edit permission to all path writes.",
            ),
        });
        return;
    }

    if raw.starts_with("mcp__") {
        let parts = raw.split("__").collect::<Vec<_>>();
        if parts.len() == 3 {
            policy
                .axes
                .mcp
                .tool_rules
                .push(crate::permissions::canonical::McpToolRule {
                    server_id: parts[1].to_owned(),
                    tool_name: parts[2].to_owned(),
                    effect,
                    provenance,
                });
        }
    }
}

fn map_claude_approval_mode(mode: &str) -> Option<CanonicalApprovalMode> {
    match mode {
        "default" | "plan" | "dontAsk" => Some(CanonicalApprovalMode::AlwaysAsk),
        "acceptEdits" | "auto" | "bypassPermissions" => Some(CanonicalApprovalMode::AutoApprove),
        _ => None,
    }
}

fn choose_target_path(
    ctx: &PolicyContext,
    current: &NativeEffectivePolicy,
    target: PolicyChangeTarget,
) -> Result<(String, PathBuf)> {
    let repo_path = ctx
        .repo_root
        .as_ref()
        .map(|root| root.join(".claude/settings.json"));
    let repo_local_path = ctx
        .repo_root
        .as_ref()
        .map(|root| root.join(".claude/settings.local.json"));
    let user_path = ctx
        .home_dir
        .as_ref()
        .map(|home| home.join(".claude/settings.json"));
    match target {
        PolicyChangeTarget::UserConfig => user_path
            .map(|path| ("claude-user".to_owned(), path))
            .ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Claude user config path is unavailable".to_owned(),
                )
            }),
        PolicyChangeTarget::RepoConfig => repo_path
            .map(|path| ("claude-repo".to_owned(), path))
            .ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Claude repo config path is unavailable".to_owned(),
                )
            }),
        PolicyChangeTarget::LocalOverride => repo_local_path
            .map(|path| ("claude-repo-local".to_owned(), path))
            .ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Claude local override path is unavailable".to_owned(),
                )
            }),
        PolicyChangeTarget::Auto => {
            let has_repo_source = current.sources.iter().any(|source| {
                matches!(
                    source.kind,
                    PolicySourceKind::RepoConfig | PolicySourceKind::LocalOverride
                )
            });
            if has_repo_source
                && ctx.trust.is_trusted == Some(true)
                && let Some(path) = repo_path
            {
                return Ok(("claude-repo".to_owned(), path));
            }
            user_path
                .map(|path| ("claude-user".to_owned(), path))
                .or_else(|| repo_path.map(|path| ("claude-repo".to_owned(), path)))
                .ok_or_else(|| {
                    ClaudineError::PolicyAmbiguousContext(
                        "Could not determine Claude mutation target".to_owned(),
                    )
                })
        }
    }
}

fn apply_persistent_operation(root: &mut Value, operation: &PolicyChangeOp) -> Result<()> {
    match operation {
        PolicyChangeOp::GrantRead(path) => push_json_array_value(
            root,
            &["sandbox", "filesystem", "allowRead"],
            path.to_string_lossy().into_owned(),
        ),
        PolicyChangeOp::GrantWrite(path) => push_json_array_value(
            root,
            &["sandbox", "filesystem", "allowWrite"],
            path.to_string_lossy().into_owned(),
        ),
        PolicyChangeOp::DenyRead(path) => push_json_array_value(
            root,
            &["sandbox", "filesystem", "denyRead"],
            path.to_string_lossy().into_owned(),
        ),
        PolicyChangeOp::DenyWrite(path) => push_json_array_value(
            root,
            &["sandbox", "filesystem", "denyWrite"],
            path.to_string_lossy().into_owned(),
        ),
        PolicyChangeOp::AllowCommand(CommandPattern { raw }) => {
            push_json_array_value(root, &["permissions", "allow"], format!("Bash({raw})"))
        }
        PolicyChangeOp::RequireApprovalForCommand(CommandPattern { raw }) => {
            push_json_array_value(root, &["permissions", "ask"], format!("Bash({raw})"))
        }
        PolicyChangeOp::DenyCommand(CommandPattern { raw }) => {
            push_json_array_value(root, &["permissions", "deny"], format!("Bash({raw})"))
        }
        PolicyChangeOp::AllowSubagent(Some(name)) => {
            push_json_array_value(root, &["permissions", "allow"], format!("Agent({name})"))
        }
        PolicyChangeOp::DenySubagent(Some(name)) => {
            push_json_array_value(root, &["permissions", "deny"], format!("Agent({name})"))
        }
        PolicyChangeOp::SetApprovalMode(mode) => set_json_string(
            root,
            &["permissions", "defaultMode"],
            map_approval_mode_to_claude(*mode),
        ),
        PolicyChangeOp::SetSandboxMode(mode) => {
            let enabled = !matches!(mode, CanonicalSandboxMode::None);
            set_json_bool(root, &["sandbox", "enabled"], enabled);
        }
        other => {
            return Err(ClaudineError::PolicyUnsupportedMutation {
                provider: Provider::Claude,
                op: format!("{other:?}"),
            });
        }
    }
    Ok(())
}

fn build_one_shot_plan(change: &PolicyChange) -> Result<Option<OneShotMutationPlan>> {
    let mut argv = Vec::new();
    let mut overlay = json!({});
    let mut needs_overlay = false;

    for operation in &change.operations {
        match operation {
            PolicyChangeOp::SetApprovalMode(mode) => {
                argv.push("--permission-mode".to_owned());
                argv.push(map_approval_mode_to_claude(*mode).to_owned());
            }
            PolicyChangeOp::AllowCommand(CommandPattern { raw }) => {
                argv.push("--allowedTools".to_owned());
                argv.push(format!("Bash({raw})"));
            }
            PolicyChangeOp::DenyCommand(CommandPattern { raw }) => {
                argv.push("--disallowedTools".to_owned());
                argv.push(format!("Bash({raw})"));
            }
            PolicyChangeOp::GrantRead(_)
            | PolicyChangeOp::GrantWrite(_)
            | PolicyChangeOp::DenyRead(_)
            | PolicyChangeOp::DenyWrite(_)
            | PolicyChangeOp::SetSandboxMode(_)
            | PolicyChangeOp::RequireApprovalForCommand(_)
            | PolicyChangeOp::AllowSubagent(_)
            | PolicyChangeOp::DenySubagent(_) => {
                apply_persistent_operation(&mut overlay, operation)?;
                needs_overlay = true;
            }
            other => {
                return Err(ClaudineError::PolicyUnsupportedMutation {
                    provider: Provider::Claude,
                    op: format!("{other:?}"),
                });
            }
        }
    }

    if needs_overlay {
        argv.push("--settings".to_owned());
        argv.push(serde_json::to_string(&overlay)?);
    }

    Ok(Some(super::common::one_shot_plan(
        argv,
        MappingFidelity::Exact,
    )))
}

fn push_json_array_value(root: &mut Value, path: &[&str], value: String) {
    let array = ensure_json_array(root, path);
    if !array
        .iter()
        .any(|entry| entry.as_str() == Some(value.as_str()))
    {
        array.push(Value::String(value));
    }
}

fn set_json_string(root: &mut Value, path: &[&str], value: &str) {
    *ensure_json_value(root, path) = Value::String(value.to_owned());
}

fn set_json_bool(root: &mut Value, path: &[&str], value: bool) {
    *ensure_json_value(root, path) = Value::Bool(value);
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_relative_pattern(pattern: &str, workspace_root: &std::path::Path) -> String {
    let pattern = pattern.trim();
    if pattern == "*" || pattern == "**" {
        return pattern.to_owned();
    }
    if std::path::Path::new(pattern).is_absolute() {
        return normalize_path_like_pattern(pattern);
    }

    normalize_path_like_pattern(&workspace_root.join(pattern).to_string_lossy())
}

fn normalize_path_like_pattern(pattern: &str) -> String {
    let mut normalized = PathBuf::new();
    for component in std::path::Path::new(pattern).components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                normalized.push(component.as_os_str())
            }
            std::path::Component::Normal(segment) => normalized.push(segment),
        }
    }
    normalized.to_string_lossy().into_owned()
}

fn map_approval_mode_to_claude(mode: CanonicalApprovalMode) -> &'static str {
    match mode {
        CanonicalApprovalMode::AlwaysAsk => "default",
        CanonicalApprovalMode::AutoApprove => "auto",
        CanonicalApprovalMode::SuggestOnly => "plan",
    }
}

fn has_cli_overrides(cli: &ClaudeCliOverrides) -> bool {
    !cli.allow.is_empty()
        || !cli.deny.is_empty()
        || cli.permission_mode.is_some()
        || cli.dangerous_skip
        || cli.settings_override.is_some()
}

#[cfg(test)]
mod tests;
