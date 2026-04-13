use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use biscuit_file::serde_yaml_ng;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::{ClaudineError, Result};
use crate::events::Provider;
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalApprovalMode, CanonicalPolicy, CanonicalRuleProvenance, CommandAccessRule,
    McpServerRule, McpToolRule, ModeSwitchRule, PathAccessRule, PathProtectionRule, PolicyEffect,
    PolicyMode, PolicyWarning, TernaryState,
};
use crate::permissions::change::{PolicyChange, PolicyChangeOp, PolicyPersistence};
use crate::permissions::context::{CliPolicyInput, PolicyContext};
use crate::permissions::json_utils::ensure_json_value;
use crate::permissions::mutation::{
    ConfigEditPlan, OneShotMutationPlan, PersistentMutationPlan, PolicyMutationPlan,
};
use crate::permissions::native::{
    NativeEffectivePolicy, NativePolicyLayer, PolicySource, PolicySourceKind, ProviderCliOverrides,
};

const ROO_PROTECTED_PATTERNS: &[&str] = &[
    ".rooignore",
    ".roomodes",
    ".roorules*",
    ".clinerules*",
    ".roo/**",
    ".vscode/**",
    "*.code-workspace",
    ".rooprotected",
    "AGENTS.md",
    "AGENT.md",
];

#[derive(Debug, Clone, Default)]
struct RooCliSettings {
    mode: Option<String>,
    require_approval: Option<bool>,
    dangerously_skip_permissions: Option<bool>,
}

#[derive(Debug, Clone, Default)]
struct RooMcpServer {
    disabled: bool,
    always_allow: Vec<String>,
    disabled_tools: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct RooMcpConfig {
    servers: BTreeMap<String, RooMcpServer>,
}

#[derive(Debug, Clone, Default)]
struct RooMode {
    slug: String,
    groups: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct RooModesConfig {
    modes: Vec<RooMode>,
}

#[derive(Debug, Clone, Default)]
struct RooIgnoreConfig {
    patterns: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct RooCliOverrides {
    require_approval: Option<bool>,
    mode: Option<String>,
}

#[derive(Debug, Clone)]
enum RooLayerData {
    CliSettings(RooCliSettings),
    Modes(RooModesConfig),
    Mcp(RooMcpConfig),
    Ignore(RooIgnoreConfig),
}

#[derive(Debug, Clone)]
struct RooState {
    layers: Vec<(PolicySource, RooLayerData)>,
    cli: RooCliOverrides,
}

#[derive(Debug, Default)]
pub(crate) struct RooPolicyBackend;

impl ProviderPolicyBackend for RooPolicyBackend {
    fn provider(&self) -> Provider {
        Provider::RooCode
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            fidelity: BackendFidelity::Medium,
            filesystem_queries: true,
            command_queries: true,
            network_queries: false,
            mcp_queries: true,
            agent_queries: true,
            persistent_mutations: true,
            one_shot_mutations: true,
        }
    }

    fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
        let mut sources = Vec::new();

        if let Some(repo_root) = &ctx.repo_root {
            for (id, rel, precedence, kind) in [
                (
                    "roo-project-modes",
                    ".roomodes",
                    20,
                    PolicySourceKind::RepoConfig,
                ),
                (
                    "roo-project-custom-modes-yaml",
                    ".roo/custom_modes.yaml",
                    22,
                    PolicySourceKind::RepoConfig,
                ),
                (
                    "roo-project-custom-modes-json",
                    ".roo/custom_modes.json",
                    23,
                    PolicySourceKind::RepoConfig,
                ),
                (
                    "roo-project-mcp",
                    ".roo/mcp.json",
                    24,
                    PolicySourceKind::RepoConfig,
                ),
                (
                    "roo-project-ignore",
                    ".rooignore",
                    25,
                    PolicySourceKind::RuleFile,
                ),
            ] {
                let path = repo_root.join(rel);
                if path.exists() {
                    sources.push(PolicySource {
                        id: id.to_owned(),
                        kind,
                        path: Some(path),
                        precedence,
                        writable: true,
                    });
                }
            }
        }

        if let Some(home) = &ctx.home_dir {
            for (id, rel, precedence, kind) in [
                (
                    "roo-user-cli-settings",
                    ".roo/cli-settings.json",
                    40,
                    PolicySourceKind::UserConfig,
                ),
                (
                    "roo-user-custom-modes-yaml",
                    ".roo/custom_modes.yaml",
                    50,
                    PolicySourceKind::UserConfig,
                ),
                (
                    "roo-user-custom-modes-json",
                    ".roo/custom_modes.json",
                    51,
                    PolicySourceKind::UserConfig,
                ),
            ] {
                let path = home.join(rel);
                if path.exists() {
                    sources.push(PolicySource {
                        id: id.to_owned(),
                        kind,
                        path: Some(path),
                        precedence,
                        writable: true,
                    });
                }
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
                    "Roo source `{}` is missing a path",
                    source.id
                ))
            })?;
            let content = fs::read_to_string(path)?;
            let payload = parse_roo_layer(source, path, &content)?;
            layers.push(NativePolicyLayer::new(source.clone(), payload));
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
                Provider::RooCode,
                RooCliOverrides::default(),
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::RooCode {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::RooCode,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                    });
                }
                let typed = parsed
                    .payload::<RooCliOverrides>()
                    .cloned()
                    .ok_or_else(|| ClaudineError::PolicyCliParse {
                        provider: Provider::RooCode,
                        message: "parsed CLI overrides had an unexpected payload type".to_owned(),
                    })?;
                Ok(ProviderCliOverrides::new(Provider::RooCode, typed))
            }
            CliPolicyInput::Argv(argv) => {
                let mut overrides = RooCliOverrides::default();
                let mut i = 0;
                while i < argv.len() {
                    match argv[i].as_str() {
                        "-a" | "--require-approval" => overrides.require_approval = Some(true),
                        "--mode" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.mode = Some(value.clone());
                                i += 1;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                Ok(ProviderCliOverrides::new(Provider::RooCode, overrides))
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
                let payload = layer.payload::<RooLayerData>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "Roo layer payload type mismatch".to_owned(),
                    }
                })?;
                Ok((layer.source.clone(), payload))
            })
            .collect::<Result<Vec<_>>>()?;
        let cli = cli
            .and_then(|value| value.payload::<RooCliOverrides>().cloned())
            .unwrap_or_default();
        let mut sources = typed_layers
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        if has_cli(&cli) {
            sources.push(PolicySource {
                id: "roo-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            });
        }
        Ok(NativeEffectivePolicy::new(
            Provider::RooCode,
            sources,
            RooState {
                layers: typed_layers,
                cli,
            },
        ))
    }

    fn canonicalize(
        &self,
        ctx: &PolicyContext,
        native: &NativeEffectivePolicy,
    ) -> Result<CanonicalPolicy> {
        let state =
            native
                .payload::<RooState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "roo-effective".to_owned(),
                    message: "Roo effective policy payload type mismatch".to_owned(),
                })?;

        let mut policy = CanonicalPolicy::empty(
            Provider::RooCode,
            if has_cli(&state.cli) {
                PolicyMode::Effective
            } else {
                PolicyMode::Configured
            },
        );

        policy.warnings.push(PolicyWarning {
            code: "roo_partial_model".to_owned(),
            message:
                "Roo modeling is approximate because extension-global auto-approval settings are not discovered in v1."
                    .to_owned(),
            source_id: None,
        });

        for source in &native.sources {
            if let Some(path) = &source.path {
                let path_string = path.to_string_lossy().into_owned();
                let provenance = CanonicalRuleProvenance::approximate(
                    source.id.clone(),
                    "roo-config",
                    "Roo config and rule files should be treated as protected configuration.",
                );
                policy
                    .axes
                    .filesystem
                    .protected_config_paths
                    .push(PathProtectionRule {
                        pattern: path_string.clone(),
                        description: "Roo configuration file".to_owned(),
                        provenance: provenance.clone(),
                    });
                policy.axes.filesystem.write_rules.push(PathAccessRule {
                    pattern: path_string,
                    effect: PolicyEffect::Ask,
                    provenance,
                });
            }
        }

        let cli_settings = find_cli_settings(&state.layers);
        let selected_mode = state
            .cli
            .mode
            .clone()
            .or_else(|| cli_settings.and_then(|settings| settings.mode.clone()))
            .unwrap_or_else(|| "code".to_owned());
        let require_approval = state
            .cli
            .require_approval
            .or_else(|| cli_settings.and_then(|settings| settings.require_approval))
            .or_else(|| {
                cli_settings
                    .and_then(|settings| settings.dangerously_skip_permissions)
                    .map(|value| !value)
            })
            .unwrap_or(false);

        policy.axes.runtime.approval_mode = Some(if require_approval {
            CanonicalApprovalMode::AlwaysAsk
        } else {
            CanonicalApprovalMode::AutoApprove
        });
        policy.axes.runtime.can_bypass_permissions = TernaryState::No;

        let (mode_groups, from_custom_mode) = resolve_mode_groups(&state.layers, &selected_mode);
        if !from_custom_mode {
            policy.warnings.push(PolicyWarning {
                code: "roo_builtin_mode_assumption".to_owned(),
                message: format!(
                    "Roo built-in mode `{selected_mode}` was mapped using a conservative heuristic."
                ),
                source_id: None,
            });
        }

        push_roo_ignore_rules(&mut policy, ctx, &state.layers);
        push_roo_protected_rules(&mut policy, ctx);
        push_workspace_rules(&mut policy, ctx, &mode_groups, require_approval);
        push_command_rules(&mut policy, &mode_groups, require_approval);
        push_agent_rules(&mut policy, &mode_groups, require_approval);
        push_mcp_rules(&mut policy, &state.layers, &mode_groups, require_approval);

        Ok(policy)
    }

    fn plan_change(
        &self,
        ctx: &PolicyContext,
        _current: &NativeEffectivePolicy,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan> {
        let path = ctx
            .home_dir
            .as_ref()
            .map(|home| home.join(".roo/cli-settings.json"));
        let path = path.ok_or_else(|| {
            ClaudineError::PolicyAmbiguousContext("Roo CLI settings path is unavailable".to_owned())
        })?;
        let before_text = fs::read_to_string(&path).ok();
        let mut root = before_text
            .as_deref()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .unwrap_or_else(|| json!({}));

        let mut supported = true;
        for operation in &change.operations {
            match operation {
                PolicyChangeOp::SetApprovalMode(mode) => {
                    ensure_json_value(&mut root, &["requireApproval"]).clone_from(&Value::Bool(
                        matches!(mode, CanonicalApprovalMode::AlwaysAsk),
                    ));
                }
                _ => {
                    supported = false;
                    break;
                }
            }
        }

        if !supported {
            return Ok(PolicyMutationPlan::unsupported(
                Provider::RooCode,
                "Roo mutation planning only supports `SetApprovalMode` until more stable edit targets are modeled.",
            ));
        }

        let one_shot_plan = build_one_shot_plan(change);
        let after_preview = serde_json::to_string_pretty(&root)? + "\n";

        Ok(PolicyMutationPlan {
            provider: Provider::RooCode,
            persistent_plan: if change.persistence == PolicyPersistence::OneShot {
                None
            } else {
                Some(PersistentMutationPlan {
                    edits: vec![ConfigEditPlan {
                        source_id: "roo-user-cli-settings".to_owned(),
                        path,
                        description: "Update Roo CLI approval defaults".to_owned(),
                        before_preview: before_text,
                        after_preview,
                    }],
                    fidelity: crate::permissions::MappingFidelity::Approximate,
                })
            },
            one_shot_plan,
            warnings: Vec::new(),
            supported: true,
        })
    }
}

fn parse_roo_layer(source: &PolicySource, path: &Path, content: &str) -> Result<RooLayerData> {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    match filename {
        "cli-settings.json" => {
            let parsed: RooCliSettingsFile = serde_json::from_str(content).map_err(|error| {
                ClaudineError::PolicyNativeParse {
                    source_id: source.id.clone(),
                    message: error.to_string(),
                }
            })?;
            Ok(RooLayerData::CliSettings(RooCliSettings {
                mode: parsed.mode,
                require_approval: parsed.require_approval,
                dangerously_skip_permissions: parsed.dangerously_skip_permissions,
            }))
        }
        "mcp.json" => {
            let parsed: RooMcpFile = serde_json::from_str(content).map_err(|error| {
                ClaudineError::PolicyNativeParse {
                    source_id: source.id.clone(),
                    message: error.to_string(),
                }
            })?;
            Ok(RooLayerData::Mcp(RooMcpConfig {
                servers: parsed
                    .mcp_servers
                    .into_iter()
                    .map(|(name, config)| {
                        (
                            name,
                            RooMcpServer {
                                disabled: config.disabled,
                                always_allow: config.always_allow,
                                disabled_tools: config.disabled_tools,
                            },
                        )
                    })
                    .collect(),
            }))
        }
        ".rooignore" => Ok(RooLayerData::Ignore(RooIgnoreConfig {
            patterns: content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_owned)
                .collect(),
        })),
        _ => {
            if content.trim_start().starts_with('{') {
                let parsed: RooModesFile = serde_json::from_str(content).map_err(|error| {
                    ClaudineError::PolicyNativeParse {
                        source_id: source.id.clone(),
                        message: error.to_string(),
                    }
                })?;
                Ok(RooLayerData::Modes(convert_modes(parsed.custom_modes)))
            } else {
                let parsed: RooModesFile = serde_yaml_ng::from_str(content).map_err(|error| {
                    ClaudineError::PolicyNativeParse {
                        source_id: source.id.clone(),
                        message: error.to_string(),
                    }
                })?;
                Ok(RooLayerData::Modes(convert_modes(parsed.custom_modes)))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct RooCliSettingsFile {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default, rename = "requireApproval")]
    require_approval: Option<bool>,
    #[serde(default, rename = "dangerouslySkipPermissions")]
    dangerously_skip_permissions: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RooMcpFile {
    #[serde(default, rename = "mcpServers")]
    mcp_servers: BTreeMap<String, RooMcpServerFile>,
}

#[derive(Debug, Deserialize)]
struct RooMcpServerFile {
    #[serde(default)]
    disabled: bool,
    #[serde(default, rename = "alwaysAllow")]
    always_allow: Vec<String>,
    #[serde(default, rename = "disabledTools")]
    disabled_tools: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RooModesFile {
    #[serde(default, rename = "customModes")]
    custom_modes: Vec<RooModeFile>,
}

#[derive(Debug, Deserialize)]
struct RooModeFile {
    slug: String,
    #[serde(default)]
    groups: Vec<RooGroupEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RooGroupEntry {
    Plain(String),
    Detailed((String, serde_yaml_ng::Value)),
}

fn convert_modes(modes: Vec<RooModeFile>) -> RooModesConfig {
    RooModesConfig {
        modes: modes
            .into_iter()
            .map(|mode| RooMode {
                slug: mode.slug,
                groups: mode
                    .groups
                    .into_iter()
                    .map(|entry| match entry {
                        RooGroupEntry::Plain(value) => value,
                        RooGroupEntry::Detailed((value, _)) => value,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn find_cli_settings(layers: &[(PolicySource, RooLayerData)]) -> Option<&RooCliSettings> {
    layers.iter().find_map(|(_, layer)| match layer {
        RooLayerData::CliSettings(settings) => Some(settings),
        _ => None,
    })
}

fn resolve_mode_groups(
    layers: &[(PolicySource, RooLayerData)],
    selected_mode: &str,
) -> (BTreeSet<String>, bool) {
    for (_, layer) in layers {
        if let RooLayerData::Modes(config) = layer
            && let Some(mode) = config.modes.iter().find(|mode| mode.slug == selected_mode)
        {
            return (mode.groups.iter().cloned().collect(), true);
        }
    }

    let groups: &[&str] = match selected_mode {
        "ask" => &["read", "modes"],
        _ => &["read", "edit", "command", "mcp", "modes"],
    };
    (
        groups.iter().map(|value| (*value).to_owned()).collect(),
        false,
    )
}

fn push_roo_ignore_rules(
    policy: &mut CanonicalPolicy,
    ctx: &PolicyContext,
    layers: &[(PolicySource, RooLayerData)],
) {
    let workspace_root = ctx.repo_root.as_ref().unwrap_or(&ctx.cwd);
    for (source, layer) in layers {
        let RooLayerData::Ignore(ignore) = layer else {
            continue;
        };
        for pattern in &ignore.patterns {
            let normalized = if pattern.starts_with('/') || pattern.starts_with('*') {
                pattern.clone()
            } else {
                workspace_root.join(pattern).to_string_lossy().into_owned()
            };
            let provenance = CanonicalRuleProvenance::approximate(
                source.id.clone(),
                ".rooignore",
                "Roo ignore rules are modeled as deny rules for filesystem access.",
            );
            policy.axes.filesystem.read_rules.push(PathAccessRule {
                pattern: normalized.clone(),
                effect: PolicyEffect::Deny,
                provenance: provenance.clone(),
            });
            policy.axes.filesystem.write_rules.push(PathAccessRule {
                pattern: normalized.clone(),
                effect: PolicyEffect::Deny,
                provenance: provenance.clone(),
            });
            policy.axes.filesystem.traversal_rules.push(PathAccessRule {
                pattern: normalized,
                effect: PolicyEffect::Deny,
                provenance,
            });
        }
    }
}

fn push_roo_protected_rules(policy: &mut CanonicalPolicy, ctx: &PolicyContext) {
    let workspace_root = ctx.repo_root.as_ref().unwrap_or(&ctx.cwd);
    for pattern in ROO_PROTECTED_PATTERNS {
        let normalized = if pattern.starts_with('/') || pattern.starts_with('*') {
            (*pattern).to_owned()
        } else {
            workspace_root.join(pattern).to_string_lossy().into_owned()
        };
        let provenance = CanonicalRuleProvenance::approximate(
            "roo-protected-files",
            "protected-files",
            "Roo protected-file patterns require approval for modification.",
        );
        policy
            .axes
            .filesystem
            .protected_config_paths
            .push(PathProtectionRule {
                pattern: (*pattern).to_owned(),
                description: "Roo protected file".to_owned(),
                provenance: provenance.clone(),
            });
        policy.axes.filesystem.write_rules.push(PathAccessRule {
            pattern: normalized,
            effect: PolicyEffect::Ask,
            provenance,
        });
    }
}

fn push_workspace_rules(
    policy: &mut CanonicalPolicy,
    ctx: &PolicyContext,
    groups: &BTreeSet<String>,
    require_approval: bool,
) {
    let workspace = ctx
        .repo_root
        .as_ref()
        .unwrap_or(&ctx.cwd)
        .to_string_lossy()
        .into_owned();
    let read_effect = effect_for_group(groups.contains("read"), require_approval);
    let write_effect = effect_for_group(groups.contains("edit"), require_approval);
    let provenance = CanonicalRuleProvenance::approximate(
        "roo-mode",
        "mode-groups",
        "Workspace access is inferred from Roo mode groups and CLI approval settings.",
    );

    policy.axes.filesystem.read_rules.push(PathAccessRule {
        pattern: workspace.clone(),
        effect: read_effect,
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.traversal_rules.push(PathAccessRule {
        pattern: workspace.clone(),
        effect: read_effect,
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.write_rules.push(PathAccessRule {
        pattern: workspace,
        effect: write_effect,
        provenance: provenance.clone(),
    });

    policy.axes.filesystem.read_rules.push(PathAccessRule {
        pattern: "*".to_owned(),
        effect: if groups.contains("read") {
            PolicyEffect::Ask
        } else {
            PolicyEffect::Deny
        },
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.traversal_rules.push(PathAccessRule {
        pattern: "*".to_owned(),
        effect: if groups.contains("read") {
            PolicyEffect::Ask
        } else {
            PolicyEffect::Deny
        },
        provenance: provenance.clone(),
    });
    policy.axes.filesystem.write_rules.push(PathAccessRule {
        pattern: "*".to_owned(),
        effect: if groups.contains("edit") {
            PolicyEffect::Ask
        } else {
            PolicyEffect::Deny
        },
        provenance,
    });
}

fn push_command_rules(
    policy: &mut CanonicalPolicy,
    groups: &BTreeSet<String>,
    require_approval: bool,
) {
    policy.axes.commands.shell_rules.push(CommandAccessRule {
        pattern: "*".to_owned(),
        effect: effect_for_group(groups.contains("command"), require_approval),
        provenance: CanonicalRuleProvenance::approximate(
            "roo-mode",
            "command-group",
            "Roo command access is inferred from the selected mode and require-approval CLI state.",
        ),
    });
}

fn push_agent_rules(
    policy: &mut CanonicalPolicy,
    groups: &BTreeSet<String>,
    require_approval: bool,
) {
    let effect = effect_for_group(groups.contains("modes"), require_approval);
    let provenance = CanonicalRuleProvenance::approximate(
        "roo-mode",
        "modes-group",
        "Mode switching and delegated tasks are inferred from the Roo mode surface.",
    );
    policy.axes.agents.mode_switch_rules.push(ModeSwitchRule {
        target: None,
        effect,
        provenance: provenance.clone(),
    });
    policy
        .axes
        .agents
        .subagent_rules
        .push(crate::permissions::SubagentRule {
            name: None,
            effect,
            provenance,
        });
}

fn push_mcp_rules(
    policy: &mut CanonicalPolicy,
    layers: &[(PolicySource, RooLayerData)],
    groups: &BTreeSet<String>,
    require_approval: bool,
) {
    let default_effect = effect_for_group(groups.contains("mcp"), require_approval);

    for (source, layer) in layers {
        let RooLayerData::Mcp(config) = layer else {
            continue;
        };

        for (server_name, server) in &config.servers {
            if server.disabled {
                policy.axes.mcp.server_rules.push(McpServerRule {
                    server_id: server_name.clone(),
                    effect: PolicyEffect::Deny,
                    provenance: CanonicalRuleProvenance::exact(
                        source.id.clone(),
                        format!("mcpServers.{server_name}.disabled"),
                    ),
                });
            }

            for tool in &server.disabled_tools {
                policy.axes.mcp.tool_rules.push(McpToolRule {
                    server_id: server_name.clone(),
                    tool_name: tool.clone(),
                    effect: PolicyEffect::Deny,
                    provenance: CanonicalRuleProvenance::exact(
                        source.id.clone(),
                        format!("mcpServers.{server_name}.disabledTools"),
                    ),
                });
            }

            for tool in &server.always_allow {
                policy.axes.mcp.tool_rules.push(McpToolRule {
                    server_id: server_name.clone(),
                    tool_name: tool.clone(),
                    effect: if require_approval {
                        PolicyEffect::Ask
                    } else {
                        PolicyEffect::Allow
                    },
                    provenance: CanonicalRuleProvenance::approximate(
                        source.id.clone(),
                        format!("mcpServers.{server_name}.alwaysAllow"),
                        "Roo per-tool alwaysAllow depends on broader MCP approval gates.",
                    ),
                });
            }
        }
    }

    policy.axes.mcp.server_rules.push(McpServerRule {
        server_id: "*".to_owned(),
        effect: default_effect,
        provenance: CanonicalRuleProvenance::approximate(
            "roo-mode",
            "mcp-group",
            "Roo MCP default behavior is inferred from the mode surface and CLI approval state.",
        ),
    });
    policy.axes.mcp.tool_rules.push(McpToolRule {
        server_id: "*".to_owned(),
        tool_name: "*".to_owned(),
        effect: if groups.contains("mcp") {
            PolicyEffect::Ask
        } else {
            PolicyEffect::Deny
        },
        provenance: CanonicalRuleProvenance::approximate(
            "roo-mode",
            "mcp-group",
            "Without extension-global settings, Roo MCP tools fall back to ask-or-deny modeling.",
        ),
    });
}

fn effect_for_group(enabled: bool, require_approval: bool) -> PolicyEffect {
    match (enabled, require_approval) {
        (true, true) => PolicyEffect::Ask,
        (true, false) => PolicyEffect::Allow,
        (false, _) => PolicyEffect::Deny,
    }
}

fn has_cli(cli: &RooCliOverrides) -> bool {
    cli.require_approval.is_some() || cli.mode.is_some()
}

fn build_one_shot_plan(change: &PolicyChange) -> Option<OneShotMutationPlan> {
    let mut argv = Vec::new();
    for operation in &change.operations {
        match operation {
            PolicyChangeOp::SetApprovalMode(CanonicalApprovalMode::AlwaysAsk) => {
                argv.push("--require-approval".to_owned());
            }
            PolicyChangeOp::SetApprovalMode(_) => return None,
            _ => return None,
        }
    }
    Some(OneShotMutationPlan {
        argv,
        env: BTreeMap::new(),
        fidelity: crate::permissions::MappingFidelity::Approximate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{CommandQuery, ConfiguredPolicySnapshot, PolicyContext};

    fn setup_ctx() -> (tempfile::TempDir, PolicyContext) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let repo = dir.path().join("repo");
        fs::create_dir_all(home.join(".roo")).unwrap();
        fs::create_dir_all(repo.join(".roo")).unwrap();
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
    fn roo_backend_models_mode_ignore_and_mcp_rules() {
        let (_dir, ctx) = setup_ctx();
        fs::write(
            ctx.home_dir
                .as_ref()
                .unwrap()
                .join(".roo/cli-settings.json"),
            serde_json::to_string_pretty(&json!({
                "mode": "docs-writer",
                "requireApproval": false
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            ctx.repo_root.as_ref().unwrap().join(".roomodes"),
            r#"
customModes:
  - slug: docs-writer
    groups:
      - read
      - edit
      - mcp
"#,
        )
        .unwrap();
        fs::write(
            ctx.repo_root.as_ref().unwrap().join(".roo/mcp.json"),
            serde_json::to_string_pretty(&json!({
                "mcpServers": {
                    "filesystem": {
                        "alwaysAllow": ["read_file"],
                        "disabledTools": ["delete_file"]
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            ctx.repo_root.as_ref().unwrap().join(".rooignore"),
            "secret/**\n",
        )
        .unwrap();

        let backend = RooPolicyBackend;
        let sources = backend.discover_sources(&ctx).unwrap();
        let layers = backend.load_native_layers(&ctx, &sources).unwrap();
        let native = backend.compose_native_policy(&ctx, &layers, None).unwrap();
        let canonical = backend.canonicalize(&ctx, &native).unwrap();
        let snapshot =
            ConfiguredPolicySnapshot::from_parts(Provider::RooCode, native, canonical, &ctx);

        assert!(
            snapshot
                .can_write(ctx.repo_root.as_ref().unwrap())
                .is_allowed()
        );
        assert!(
            snapshot
                .can_read(ctx.repo_root.as_ref().unwrap().join("secret/file.txt"))
                .is_denied()
        );
        assert!(
            snapshot
                .can_execute(&CommandQuery::from_raw("git status"))
                .is_denied()
        );
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
        assert!(
            snapshot
                .can_write(ctx.repo_root.as_ref().unwrap().join("AGENTS.md"))
                .is_ask()
        );
    }

    #[test]
    fn roo_cli_require_approval_forces_ask() {
        let (_dir, ctx) = setup_ctx();
        let backend = RooPolicyBackend;
        let cli = vec![
            "--require-approval".to_owned(),
            "--mode".to_owned(),
            "ask".to_owned(),
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
        let canonical = backend.canonicalize(&ctx, &native).unwrap();
        let snapshot =
            ConfiguredPolicySnapshot::from_parts(Provider::RooCode, native, canonical, &ctx);

        assert!(snapshot.can_read(ctx.cwd.join("README.md")).is_ask());
        assert!(snapshot.can_write(ctx.cwd.join("README.md")).is_denied());
    }
}
