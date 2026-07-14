use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::fs;

use crate::error::{ClaudineError, Result};
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalApprovalMode, CanonicalPolicy, CanonicalRuleProvenance, CanonicalSandboxMode,
    CommandAccessRule, McpServerRule, McpToolRule, PathAccessRule, PathProtectionRule,
    PolicyEffect, PolicyMode, PolicyWarning, TernaryState,
};
use crate::permissions::change::{
    CommandPattern, PolicyChange, PolicyChangeOp, PolicyChangeTarget, PolicyPersistence,
};
use crate::permissions::context::{CliPolicyInput, PolicyContext};
use crate::permissions::json_utils::ensure_json_value;
use crate::permissions::mutation::{
    ConfigEditPlan, OneShotMutationPlan, PersistentMutationPlan, PolicyMutationPlan,
};
use crate::permissions::native::{
    NativeEffectivePolicy, NativePolicyLayer, PolicySource, PolicySourceKind, ProviderCliOverrides,
};
use crate::provider::Provider;

#[derive(Debug, Clone, Default)]
struct QwenMcpServer {
    trust: Option<bool>,
    include_tools: Vec<String>,
    exclude_tools: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct QwenConfig {
    default_mode: Option<String>,
    allow: Vec<String>,
    ask: Vec<String>,
    deny: Vec<String>,
    additional_directories: Vec<String>,
    sandbox: Option<String>,
    folder_trust_enabled: Option<bool>,
    mcp_servers: BTreeMap<String, QwenMcpServer>,
}

#[derive(Debug, Clone, Default)]
struct QwenCliOverrides {
    approval_mode: Option<String>,
    allowed_tools: Vec<String>,
    excluded_tools: Vec<String>,
    allowed_mcp_server_names: Vec<String>,
    sandbox: Option<String>,
}

#[derive(Debug, Clone)]
struct QwenState {
    layers: Vec<(PolicySource, QwenConfig)>,
    cli: QwenCliOverrides,
}

#[derive(Debug, Default)]
pub(crate) struct QwenPolicyBackend;

#[async_trait]
impl ProviderPolicyBackend for QwenPolicyBackend {
    fn provider(&self) -> Provider {
        Provider::QwenCode
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            fidelity: BackendFidelity::Medium,
            filesystem_queries: true,
            command_queries: true,
            network_queries: false,
            mcp_queries: true,
            agent_queries: false,
            persistent_mutations: true,
            one_shot_mutations: true,
        }
    }

    async fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
        let mut sources = Vec::new();

        if let Some(system_root) = &ctx.system_root {
            let settings = system_root.join("etc/qwen-code/settings.json");
            if fs::try_exists(&settings).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "qwen-system".to_owned(),
                    kind: PolicySourceKind::SystemConfig,
                    path: Some(settings),
                    precedence: 10,
                    writable: false,
                });
            }
        }

        if ctx.trust.is_trusted != Some(false)
            && let Some(repo_root) = &ctx.repo_root
        {
            let settings = repo_root.join(".qwen/settings.json");
            if fs::try_exists(&settings).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "qwen-project".to_owned(),
                    kind: PolicySourceKind::RepoConfig,
                    path: Some(settings),
                    precedence: 20,
                    writable: true,
                });
            }
        }

        if let Some(home) = &ctx.home_dir {
            let settings = home.join(".qwen/settings.json");
            if fs::try_exists(&settings).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "qwen-user".to_owned(),
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
                    "Qwen source `{}` is missing a path",
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
                parse_qwen_config(&value),
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
                Provider::QwenCode,
                QwenCliOverrides::default(),
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::QwenCode {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::QwenCode,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                    });
                }
                let typed = parsed
                    .payload::<QwenCliOverrides>()
                    .cloned()
                    .ok_or_else(|| ClaudineError::PolicyCliParse {
                        provider: Provider::QwenCode,
                        message: "parsed CLI overrides had an unexpected payload type".to_owned(),
                    })?;
                Ok(ProviderCliOverrides::new(Provider::QwenCode, typed))
            }
            CliPolicyInput::Argv(argv) => {
                let mut overrides = QwenCliOverrides::default();
                let mut i = 0;
                while i < argv.len() {
                    match argv[i].as_str() {
                        "--approval-mode" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.approval_mode = Some(value.clone());
                                i += 1;
                            }
                        }
                        "-y" | "--yolo" => overrides.approval_mode = Some("yolo".to_owned()),
                        "--allowed-tools" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.allowed_tools.extend(split_csv_list(value));
                                i += 1;
                            }
                        }
                        "--exclude-tools" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.excluded_tools.extend(split_csv_list(value));
                                i += 1;
                            }
                        }
                        "--allowed-mcp-server-names" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides
                                    .allowed_mcp_server_names
                                    .extend(split_csv_list(value));
                                i += 1;
                            }
                        }
                        "-s" | "--sandbox" => {
                            overrides.sandbox = Some("sandbox".to_owned());
                        }
                        _ => {}
                    }
                    i += 1;
                }
                Ok(ProviderCliOverrides::new(Provider::QwenCode, overrides))
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
                let config = layer.payload::<QwenConfig>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "Qwen layer payload type mismatch".to_owned(),
                    }
                })?;
                Ok((layer.source.clone(), config))
            })
            .collect::<Result<Vec<_>>>()?;

        let cli = cli
            .and_then(|value| value.payload::<QwenCliOverrides>().cloned())
            .unwrap_or_default();

        let mut sources = typed_layers
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        if has_cli(&cli) {
            sources.push(PolicySource {
                id: "qwen-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            });
        }

        Ok(NativeEffectivePolicy::new(
            Provider::QwenCode,
            sources,
            QwenState {
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
                .payload::<QwenState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "qwen-effective".to_owned(),
                    message: "Qwen effective policy payload type mismatch".to_owned(),
                })?;

        let mut policy = CanonicalPolicy::empty(
            Provider::QwenCode,
            if has_cli(&state.cli) {
                PolicyMode::Effective
            } else {
                PolicyMode::Configured
            },
        );

        policy.warnings.push(PolicyWarning {
            code: "qwen_evolving_policy".to_owned(),
            message:
                "Qwen permission modeling is best-effort; sandbox and trust behavior can vary by release."
                    .to_owned(),
            source_id: None,
        });

        if ctx.trust.is_trusted == Some(false) {
            policy.warnings.push(PolicyWarning {
                code: "qwen_untrusted_folder".to_owned(),
                message:
                    "Project settings were not loaded because the workspace is marked untrusted."
                        .to_owned(),
                source_id: Some("qwen-project".to_owned()),
            });
        }

        for source in &native.sources {
            if let Some(path) = &source.path {
                let path_string = path.to_string_lossy().into_owned();
                let provenance = CanonicalRuleProvenance::approximate(
                    source.id.clone(),
                    "settings.json",
                    "Qwen settings files should be treated as protected configuration.",
                );
                policy
                    .axes
                    .filesystem
                    .protected_config_paths
                    .push(PathProtectionRule {
                        pattern: path_string.clone(),
                        description: "Qwen configuration file".to_owned(),
                        provenance: provenance.clone(),
                    });
                policy.axes.filesystem.write_rules.push(PathAccessRule {
                    pattern: path_string,
                    effect: PolicyEffect::Ask,
                    provenance,
                });
            }
        }

        let approval_mode = state
            .cli
            .approval_mode
            .clone()
            .or_else(|| {
                state
                    .layers
                    .iter()
                    .find_map(|(_, config)| config.default_mode.clone())
            })
            .unwrap_or_else(|| "default".to_owned());

        policy.axes.runtime.approval_mode = Some(map_qwen_approval_mode(&approval_mode));
        policy.axes.runtime.sandbox_mode = Some(
            if state.cli.sandbox.is_some()
                || state
                    .layers
                    .iter()
                    .any(|(_, config)| config.sandbox.is_some())
            {
                CanonicalSandboxMode::Partial
            } else {
                CanonicalSandboxMode::None
            },
        );
        policy.axes.runtime.can_bypass_permissions = if approval_mode == "yolo" {
            TernaryState::Yes
        } else {
            TernaryState::No
        };
        policy.axes.runtime.project_trust_required = if state
            .layers
            .iter()
            .any(|(_, config)| config.folder_trust_enabled == Some(true))
        {
            TernaryState::Yes
        } else {
            TernaryState::No
        };

        push_workspace_rules(&mut policy, ctx, &approval_mode);

        for (source, config) in &state.layers {
            for pattern in &config.deny {
                push_qwen_pattern(&mut policy, source, pattern, PolicyEffect::Deny);
            }
        }
        for pattern in &state.cli.excluded_tools {
            let source = cli_source();
            push_qwen_pattern(&mut policy, &source, pattern, PolicyEffect::Deny);
        }
        for (source, config) in &state.layers {
            for pattern in &config.ask {
                push_qwen_pattern(&mut policy, source, pattern, PolicyEffect::Ask);
            }
        }
        for (source, config) in &state.layers {
            for pattern in &config.allow {
                push_qwen_pattern(&mut policy, source, pattern, PolicyEffect::Allow);
            }
        }
        for pattern in &state.cli.allowed_tools {
            let source = cli_source();
            push_qwen_pattern(&mut policy, &source, pattern, PolicyEffect::Allow);
        }

        for (source, config) in &state.layers {
            for directory in &config.additional_directories {
                let provenance = CanonicalRuleProvenance::exact(
                    source.id.clone(),
                    "permissions.additionalDirectories",
                );
                policy.axes.filesystem.read_rules.push(PathAccessRule {
                    pattern: directory.clone(),
                    effect: PolicyEffect::Allow,
                    provenance: provenance.clone(),
                });
                policy.axes.filesystem.write_rules.push(PathAccessRule {
                    pattern: directory.clone(),
                    effect: qwen_write_effect(&approval_mode),
                    provenance: provenance.clone(),
                });
                policy.axes.filesystem.traversal_rules.push(PathAccessRule {
                    pattern: directory.clone(),
                    effect: PolicyEffect::Allow,
                    provenance,
                });
            }

            for (server_name, server) in &config.mcp_servers {
                if server.trust == Some(true) {
                    policy.axes.mcp.server_rules.push(McpServerRule {
                        server_id: server_name.clone(),
                        effect: PolicyEffect::Allow,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            format!("mcpServers.{server_name}.trust"),
                        ),
                    });
                }

                for tool in &server.exclude_tools {
                    policy.axes.mcp.tool_rules.push(McpToolRule {
                        server_id: server_name.clone(),
                        tool_name: tool.clone(),
                        effect: PolicyEffect::Deny,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            format!("mcpServers.{server_name}.excludeTools"),
                        ),
                    });
                }

                for tool in &server.include_tools {
                    policy.axes.mcp.tool_rules.push(McpToolRule {
                        server_id: server_name.clone(),
                        tool_name: tool.clone(),
                        effect: PolicyEffect::Allow,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            format!("mcpServers.{server_name}.includeTools"),
                        ),
                    });
                }
            }
        }

        if !state.cli.allowed_mcp_server_names.is_empty() {
            let source = cli_source();
            for server in &state.cli.allowed_mcp_server_names {
                policy.axes.mcp.server_rules.push(McpServerRule {
                    server_id: server.clone(),
                    effect: PolicyEffect::Allow,
                    provenance: CanonicalRuleProvenance::approximate(
                        source.id.clone(),
                        "argv.--allowed-mcp-server-names",
                        "CLI MCP server filtering is modeled as an allowlist.",
                    ),
                });
            }
            policy.axes.mcp.server_rules.push(McpServerRule {
                server_id: "*".to_owned(),
                effect: PolicyEffect::Deny,
                provenance: CanonicalRuleProvenance::approximate(
                    source.id,
                    "argv.--allowed-mcp-server-names",
                    "Unlisted MCP servers are denied by the CLI filter.",
                ),
            });
        }

        let mcp_default = qwen_mcp_effect(&approval_mode);
        policy.axes.mcp.server_rules.push(McpServerRule {
            server_id: "*".to_owned(),
            effect: mcp_default,
            provenance: CanonicalRuleProvenance::approximate(
                "qwen-default-mode",
                "permissions.defaultMode",
                "Approval mode supplies the fallback MCP decision when no explicit rule matches.",
            ),
        });
        policy.axes.mcp.tool_rules.push(McpToolRule {
            server_id: "*".to_owned(),
            tool_name: "*".to_owned(),
            effect: mcp_default,
            provenance: CanonicalRuleProvenance::approximate(
                "qwen-default-mode",
                "permissions.defaultMode",
                "Approval mode supplies the fallback MCP tool decision when no explicit rule matches.",
            ),
        });

        Ok(policy)
    }

    async fn plan_change(
        &self,
        ctx: &PolicyContext,
        current: &NativeEffectivePolicy,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan> {
        let (source_id, path) = choose_target(ctx, current, change.target)?;
        let before_text = fs::read_to_string(&path).await.ok();
        let mut root = before_text
            .as_deref()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .unwrap_or_else(|| json!({}));

        let mut unsupported = Vec::new();
        for operation in &change.operations {
            if !apply_qwen_operation(&mut root, operation) {
                unsupported.push(operation_name(operation));
            }
        }

        let after_preview = serde_json::to_string_pretty(&root)? + "\n";
        let one_shot_plan = build_one_shot_plan(change)?;
        let supported = unsupported.is_empty();
        let mut warnings = Vec::new();
        if !supported {
            warnings.push(PolicyWarning {
                code: "unsupported_qwen_mutation".to_owned(),
                message: format!(
                    "Qwen mutation planning skipped unsupported operations: {}",
                    unsupported.join(", "),
                ),
                source_id: None,
            });
        }

        Ok(PolicyMutationPlan {
            provider: Provider::QwenCode,
            persistent_plan: if change.persistence == PolicyPersistence::OneShot {
                None
            } else {
                Some(PersistentMutationPlan {
                    edits: vec![ConfigEditPlan {
                        source_id,
                        path,
                        description: "Update Qwen permissions".to_owned(),
                        before_preview: before_text,
                        after_preview,
                    }],
                    fidelity: crate::permissions::MappingFidelity::Approximate,
                })
            },
            one_shot_plan,
            warnings,
            supported,
        })
    }
}

fn parse_qwen_config(value: &Value) -> QwenConfig {
    let permissions = value
        .get("permissions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let tools = value
        .get("tools")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let security = value
        .get("security")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let folder_trust = security
        .get("folderTrust")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mcp_servers = value
        .get("mcpServers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    QwenConfig {
        default_mode: permissions
            .get("defaultMode")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                tools
                    .get("approvalMode")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            }),
        allow: array_of_strings(permissions.get("allow")),
        ask: array_of_strings(permissions.get("ask")),
        deny: array_of_strings(permissions.get("deny")),
        additional_directories: array_of_strings(permissions.get("additionalDirectories")),
        sandbox: parse_sandbox_value(tools.get("sandbox")),
        folder_trust_enabled: folder_trust.get("enabled").and_then(Value::as_bool),
        mcp_servers: mcp_servers
            .into_iter()
            .map(|(name, entry)| {
                let map = entry.as_object().cloned().unwrap_or_default();
                (
                    name,
                    QwenMcpServer {
                        trust: map.get("trust").and_then(Value::as_bool),
                        include_tools: array_of_strings(map.get("includeTools")),
                        exclude_tools: array_of_strings(map.get("excludeTools")),
                    },
                )
            })
            .collect(),
    }
}

fn array_of_strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_sandbox_value(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::Bool(true)) => Some("sandbox".to_owned()),
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn split_csv_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn push_workspace_rules(policy: &mut CanonicalPolicy, ctx: &PolicyContext, approval_mode: &str) {
    let workspace = ctx
        .repo_root
        .as_ref()
        .unwrap_or(&ctx.cwd)
        .to_string_lossy()
        .into_owned();
    let provenance = CanonicalRuleProvenance::approximate(
        "qwen-workspace",
        "workspace-root",
        "Workspace access is modeled from Qwen approval modes and additionalDirectories.",
    );

    policy.axes.filesystem.read_rules.push(PathAccessRule {
        pattern: workspace.clone(),
        effect: PolicyEffect::Allow,
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.traversal_rules.push(PathAccessRule {
        pattern: workspace.clone(),
        effect: PolicyEffect::Allow,
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.write_rules.push(PathAccessRule {
        pattern: workspace,
        effect: qwen_write_effect(approval_mode),
        provenance: provenance.clone(),
    });

    policy.axes.filesystem.read_rules.push(PathAccessRule {
        pattern: "*".to_owned(),
        effect: PolicyEffect::Ask,
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.traversal_rules.push(PathAccessRule {
        pattern: "*".to_owned(),
        effect: PolicyEffect::Ask,
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.write_rules.push(PathAccessRule {
        pattern: "*".to_owned(),
        effect: match approval_mode {
            "yolo" | "auto-edit" => PolicyEffect::Ask,
            "default" => PolicyEffect::Ask,
            _ => PolicyEffect::Deny,
        },
        provenance,
    });
}

fn push_qwen_pattern(
    policy: &mut CanonicalPolicy,
    source: &PolicySource,
    pattern: &str,
    effect: PolicyEffect,
) {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return;
    }

    if let Some(command) = parse_bash_pattern(pattern) {
        policy.axes.commands.shell_rules.push(CommandAccessRule {
            pattern: command,
            effect,
            provenance: CanonicalRuleProvenance::exact(source.id.clone(), "permissions.*"),
        });
        return;
    }

    if pattern == "Read" {
        let provenance = CanonicalRuleProvenance::approximate(
            source.id.clone(),
            "permissions.read",
            "Read aliases expand to broad filesystem read and traversal access.",
        );
        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect,
            provenance: provenance.clone(),
        });
        policy.axes.filesystem.traversal_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect,
            provenance,
        });
        return;
    }

    if let Some(path) = parse_read_file_pattern(pattern) {
        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: path,
            effect,
            provenance: CanonicalRuleProvenance::approximate(
                source.id.clone(),
                "permissions.read",
                "Qwen ReadFile rules are mapped to canonical read-path rules.",
            ),
        });
        return;
    }

    if pattern == "Edit" {
        policy.axes.filesystem.write_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect,
            provenance: CanonicalRuleProvenance::approximate(
                source.id.clone(),
                "permissions.edit",
                "Qwen Edit aliases are mapped to broad write access.",
            ),
        });
        return;
    }

    if let Some((server, tool)) = parse_mcp_pattern(pattern) {
        if tool == "*" {
            policy.axes.mcp.server_rules.push(McpServerRule {
                server_id: server.clone(),
                effect,
                provenance: CanonicalRuleProvenance::exact(source.id.clone(), "permissions.mcp"),
            });
        }
        policy.axes.mcp.tool_rules.push(McpToolRule {
            server_id: server,
            tool_name: tool,
            effect,
            provenance: CanonicalRuleProvenance::exact(source.id.clone(), "permissions.mcp"),
        });
        return;
    }

    policy.warnings.push(PolicyWarning {
        code: "qwen_pattern_unmapped".to_owned(),
        message: format!("Qwen tool pattern `{pattern}` could not be mapped to a canonical axis."),
        source_id: Some(source.id.clone()),
    });
}

fn parse_bash_pattern(pattern: &str) -> Option<String> {
    if pattern == "Bash" {
        return Some("*".to_owned());
    }

    pattern
        .strip_prefix("Bash(")
        .and_then(|value| value.strip_suffix(')'))
        .map(str::to_owned)
}

fn parse_read_file_pattern(pattern: &str) -> Option<String> {
    pattern
        .strip_prefix("ReadFile(")
        .and_then(|value| value.strip_suffix(')'))
        .map(str::to_owned)
}

fn parse_mcp_pattern(pattern: &str) -> Option<(String, String)> {
    let pattern = pattern.strip_prefix("mcp__")?;
    let (server, tool) = pattern.split_once("__")?;
    Some((server.to_owned(), tool.to_owned()))
}

fn qwen_write_effect(mode: &str) -> PolicyEffect {
    match mode {
        "yolo" | "auto-edit" => PolicyEffect::Allow,
        "default" => PolicyEffect::Ask,
        _ => PolicyEffect::Deny,
    }
}

fn qwen_mcp_effect(mode: &str) -> PolicyEffect {
    match mode {
        "yolo" => PolicyEffect::Allow,
        "default" | "auto-edit" => PolicyEffect::Ask,
        _ => PolicyEffect::Deny,
    }
}

fn map_qwen_approval_mode(mode: &str) -> CanonicalApprovalMode {
    match mode {
        "yolo" | "auto-edit" => CanonicalApprovalMode::AutoApprove,
        "plan" => CanonicalApprovalMode::SuggestOnly,
        _ => CanonicalApprovalMode::AlwaysAsk,
    }
}

fn cli_source() -> PolicySource {
    PolicySource {
        id: "qwen-cli".to_owned(),
        kind: PolicySourceKind::CliOverride,
        path: None,
        precedence: 5,
        writable: false,
    }
}

fn has_cli(cli: &QwenCliOverrides) -> bool {
    cli.approval_mode.is_some()
        || !cli.allowed_tools.is_empty()
        || !cli.excluded_tools.is_empty()
        || !cli.allowed_mcp_server_names.is_empty()
        || cli.sandbox.is_some()
}

fn choose_target(
    ctx: &PolicyContext,
    current: &NativeEffectivePolicy,
    target: PolicyChangeTarget,
) -> Result<(String, PathBuf)> {
    let user_path = ctx
        .home_dir
        .as_ref()
        .map(|home| home.join(".qwen/settings.json"));
    let repo_path = ctx
        .repo_root
        .as_ref()
        .map(|repo_root| repo_root.join(".qwen/settings.json"));

    match target {
        PolicyChangeTarget::UserConfig => Ok((
            "qwen-user".to_owned(),
            user_path.ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Qwen user settings path is unavailable".to_owned(),
                )
            })?,
        )),
        PolicyChangeTarget::RepoConfig => Ok((
            "qwen-project".to_owned(),
            repo_path.ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Qwen project settings path is unavailable".to_owned(),
                )
            })?,
        )),
        PolicyChangeTarget::LocalOverride => Err(ClaudineError::PolicyUnsupportedMutation {
            provider: Provider::QwenCode,
            op: "LocalOverride target is not supported by Qwen (no local override concept)"
                .to_owned(),
        }),
        PolicyChangeTarget::Auto => {
            let has_repo = current
                .sources
                .iter()
                .any(|source| source.kind == PolicySourceKind::RepoConfig);
            if has_repo
                && ctx.trust.is_trusted != Some(false)
                && let Some(path) = repo_path
            {
                return Ok(("qwen-project".to_owned(), path));
            }
            Ok((
                "qwen-user".to_owned(),
                user_path.or(repo_path).ok_or_else(|| {
                    ClaudineError::PolicyAmbiguousContext(
                        "Could not determine Qwen settings target".to_owned(),
                    )
                })?,
            ))
        }
    }
}

fn apply_qwen_operation(root: &mut Value, operation: &PolicyChangeOp) -> bool {
    match operation {
        PolicyChangeOp::SetApprovalMode(mode) => {
            ensure_json_value(root, &["permissions", "defaultMode"])
                .clone_from(&Value::String(map_qwen_mode_setting(*mode).to_owned()));
            true
        }
        PolicyChangeOp::SetSandboxMode(mode) => {
            ensure_json_value(root, &["tools", "sandbox"]).clone_from(&Value::Bool(!matches!(
                mode,
                crate::permissions::CanonicalSandboxMode::None
            )));
            true
        }
        PolicyChangeOp::AllowCommand(CommandPattern { raw }) => {
            push_string_array_value(root, &["permissions", "allow"], &format!("Bash({raw})"));
            true
        }
        PolicyChangeOp::DenyCommand(CommandPattern { raw }) => {
            push_string_array_value(root, &["permissions", "deny"], &format!("Bash({raw})"));
            true
        }
        PolicyChangeOp::AllowMcpServer(server) => {
            push_string_array_value(
                root,
                &["permissions", "allow"],
                &format!("mcp__{server}__*"),
            );
            true
        }
        PolicyChangeOp::DenyMcpServer(server) => {
            push_string_array_value(root, &["permissions", "deny"], &format!("mcp__{server}__*"));
            true
        }
        PolicyChangeOp::AllowMcpTool { server, tool } => {
            push_string_array_value(
                root,
                &["permissions", "allow"],
                &format!("mcp__{server}__{tool}"),
            );
            true
        }
        PolicyChangeOp::DenyMcpTool { server, tool } => {
            push_string_array_value(
                root,
                &["permissions", "deny"],
                &format!("mcp__{server}__{tool}"),
            );
            true
        }
        PolicyChangeOp::GrantRead(path) | PolicyChangeOp::GrantWrite(path) => {
            let target = if path.is_dir() {
                path.clone()
            } else {
                path.parent().unwrap_or(path.as_path()).to_path_buf()
            };
            push_string_array_value(
                root,
                &["permissions", "additionalDirectories"],
                &target.to_string_lossy(),
            );
            true
        }
        _ => false,
    }
}

fn build_one_shot_plan(change: &PolicyChange) -> Result<Option<OneShotMutationPlan>> {
    let mut argv = Vec::new();
    let mut allowed_tools = Vec::new();
    let mut excluded_tools = Vec::new();
    let mut allowed_servers = BTreeSet::new();

    for operation in &change.operations {
        match operation {
            PolicyChangeOp::SetApprovalMode(mode) => match mode {
                CanonicalApprovalMode::AutoApprove => argv.push("--yolo".to_owned()),
                _ => {
                    argv.push("--approval-mode".to_owned());
                    argv.push(map_qwen_mode_setting(*mode).to_owned());
                }
            },
            PolicyChangeOp::SetSandboxMode(mode)
                if !matches!(mode, crate::permissions::CanonicalSandboxMode::None) =>
            {
                argv.push("--sandbox".to_owned());
            }
            PolicyChangeOp::SetSandboxMode(_) => return Ok(None),
            PolicyChangeOp::AllowCommand(CommandPattern { raw }) => {
                allowed_tools.push(format!("Bash({raw})"));
            }
            PolicyChangeOp::DenyCommand(CommandPattern { raw }) => {
                excluded_tools.push(format!("Bash({raw})"));
            }
            PolicyChangeOp::AllowMcpServer(server) => {
                allowed_servers.insert(server.clone());
            }
            PolicyChangeOp::AllowMcpTool { server, tool } => {
                allowed_tools.push(format!("mcp__{server}__{tool}"));
            }
            PolicyChangeOp::DenyMcpServer(server) => {
                excluded_tools.push(format!("mcp__{server}__*"));
            }
            PolicyChangeOp::DenyMcpTool { server, tool } => {
                excluded_tools.push(format!("mcp__{server}__{tool}"));
            }
            _ => return Ok(None),
        }
    }

    if !allowed_tools.is_empty() {
        argv.push("--allowed-tools".to_owned());
        argv.push(allowed_tools.join(","));
    }
    if !excluded_tools.is_empty() {
        argv.push("--exclude-tools".to_owned());
        argv.push(excluded_tools.join(","));
    }
    if !allowed_servers.is_empty() {
        argv.push("--allowed-mcp-server-names".to_owned());
        argv.push(allowed_servers.into_iter().collect::<Vec<_>>().join(","));
    }

    Ok(Some(super::common::one_shot_plan(
        argv,
        crate::permissions::MappingFidelity::Approximate,
    )))
}

fn push_string_array_value(root: &mut Value, path: &[&str], value: &str) {
    let target = ensure_json_value(root, path);
    if !target.is_array() {
        *target = Value::Array(Vec::new());
    }
    let array = target
        .as_array_mut()
        .expect("push_string_array_value: target was just set to Array above");
    let already_present = array
        .iter()
        .any(|existing| existing.as_str() == Some(value));
    if !already_present {
        array.push(Value::String(value.to_owned()));
    }
}

fn map_qwen_mode_setting(mode: CanonicalApprovalMode) -> &'static str {
    match mode {
        CanonicalApprovalMode::AlwaysAsk => "default",
        CanonicalApprovalMode::AutoApprove => "yolo",
        CanonicalApprovalMode::SuggestOnly => "plan",
    }
}

fn operation_name(operation: &PolicyChangeOp) -> &'static str {
    match operation {
        PolicyChangeOp::GrantRead(_) => "grant-read",
        PolicyChangeOp::GrantWrite(_) => "grant-write",
        PolicyChangeOp::DenyRead(_) => "deny-read",
        PolicyChangeOp::DenyWrite(_) => "deny-write",
        PolicyChangeOp::RequireApprovalForCommand(_) => "require-command-approval",
        PolicyChangeOp::AllowCommand(_) => "allow-command",
        PolicyChangeOp::DenyCommand(_) => "deny-command",
        PolicyChangeOp::AllowDomain(_) => "allow-domain",
        PolicyChangeOp::DenyDomain(_) => "deny-domain",
        PolicyChangeOp::AllowMcpServer(_) => "allow-mcp-server",
        PolicyChangeOp::DenyMcpServer(_) => "deny-mcp-server",
        PolicyChangeOp::AllowMcpTool { .. } => "allow-mcp-tool",
        PolicyChangeOp::DenyMcpTool { .. } => "deny-mcp-tool",
        PolicyChangeOp::AllowSubagent(_) => "allow-subagent",
        PolicyChangeOp::DenySubagent(_) => "deny-subagent",
        PolicyChangeOp::SetApprovalMode(_) => "set-approval-mode",
        PolicyChangeOp::SetSandboxMode(_) => "set-sandbox-mode",
    }
}

#[cfg(test)]
mod tests {
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
}
