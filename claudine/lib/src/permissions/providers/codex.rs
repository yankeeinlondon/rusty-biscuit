use std::collections::BTreeMap;
use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;
use toml_edit::{Array, DocumentMut, Item, value};

use crate::error::{ClaudineError, PolicyParseCause, Result};
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalApprovalMode, CanonicalPolicy, CanonicalRuleProvenance, CanonicalSandboxMode,
    CommandAccessRule, DomainAccessRule, MappingFidelity, McpServerRule, McpToolRule,
    PathAccessRule, PathProtectionRule, PolicyEffect, PolicyMode, PolicyWarning, TernaryState,
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
use crate::provider::Provider;

#[derive(Debug, Clone, Default)]
struct CodexProfile {
    network_enabled: Option<bool>,
    allowed_domains: Vec<String>,
    denied_domains: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct CodexMcpServer {
    enabled: Option<bool>,
    enabled_tools: Vec<String>,
    disabled_tools: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct CodexConfig {
    sandbox_mode: Option<String>,
    approval_policy: Option<String>,
    writable_roots: Vec<String>,
    network_access: Option<bool>,
    default_permissions: Option<String>,
    permissions_profiles: BTreeMap<String, CodexProfile>,
    mcp_servers: BTreeMap<String, CodexMcpServer>,
}

#[derive(Debug, Clone, Default)]
struct CodexCliOverrides {
    sandbox_mode: Option<String>,
    approval_policy: Option<String>,
    yolo: bool,
    add_dirs: Vec<String>,
}

#[derive(Debug, Clone)]
struct CodexState {
    layers: Vec<(PolicySource, CodexConfig)>,
    cli: CodexCliOverrides,
}

#[derive(Debug, Default)]
pub(crate) struct CodexPolicyBackend;

#[async_trait]
impl ProviderPolicyBackend for CodexPolicyBackend {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            fidelity: BackendFidelity::Medium,
            filesystem_queries: true,
            command_queries: true,
            network_queries: true,
            mcp_queries: true,
            agent_queries: false,
            persistent_mutations: true,
            one_shot_mutations: true,
        }
    }

    async fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
        let mut sources = Vec::new();

        if let Some(system_root) = &ctx.system_root {
            let path = system_root.join("etc/codex/config.toml");
            if fs::try_exists(&path).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "codex-system".to_owned(),
                    kind: PolicySourceKind::SystemConfig,
                    path: Some(path),
                    precedence: 10,
                    writable: false,
                });
            }
        }

        if ctx.trust.is_trusted == Some(true)
            && let Some(repo_root) = &ctx.repo_root
        {
            let path = repo_root.join(".codex/config.toml");
            if fs::try_exists(&path).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "codex-repo".to_owned(),
                    kind: PolicySourceKind::RepoConfig,
                    path: Some(path),
                    precedence: 20,
                    writable: true,
                });
            }
        }

        if let Some(home) = &ctx.home_dir {
            let path = home.join(".codex/config.toml");
            if fs::try_exists(&path).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "codex-user".to_owned(),
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

    async fn load_native_layers(
        &self,
        _ctx: &PolicyContext,
        sources: &[PolicySource],
    ) -> Result<Vec<NativePolicyLayer>> {
        let mut layers = Vec::new();
        for source in sources {
            let path = source.path.as_ref().ok_or_else(|| {
                ClaudineError::PolicySourceDiscovery(format!(
                    "Codex source `{}` is missing a path",
                    source.id
                ))
            })?;
            let content = fs::read_to_string(path).await?;
            let doc: DocumentMut = content.parse().map_err(|error: toml_edit::TomlError| {
                ClaudineError::PolicyNativeParse {
                    source_id: source.id.clone(),
                    message: error.to_string(),
                    source: Some(PolicyParseCause::Toml(Box::new(error))),
                }
            })?;
            layers.push(NativePolicyLayer::new(
                source.clone(),
                parse_codex_config(&doc),
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
                Provider::Codex,
                CodexCliOverrides::default(),
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::Codex {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::Codex,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                        source: None,
                    });
                }
                let typed = parsed
                    .payload::<CodexCliOverrides>()
                    .cloned()
                    .ok_or_else(|| ClaudineError::PolicyCliParse {
                        provider: Provider::Codex,
                        message: "parsed CLI overrides had an unexpected payload type".to_owned(),
                        source: None,
                    })?;
                Ok(ProviderCliOverrides::new(Provider::Codex, typed))
            }
            CliPolicyInput::Argv(argv) => {
                let mut overrides = CodexCliOverrides::default();
                let mut i = 0;
                while i < argv.len() {
                    match argv[i].as_str() {
                        "--sandbox" | "-s" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.sandbox_mode = Some(value.clone());
                                i += 1;
                            }
                        }
                        "--ask-for-approval" | "-a" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.approval_policy = Some(value.clone());
                                i += 1;
                            }
                        }
                        "--full-auto" => {
                            overrides.sandbox_mode = Some("workspace-write".to_owned());
                            overrides.approval_policy = Some("on-request".to_owned());
                        }
                        "--dangerously-bypass-approvals-and-sandbox" | "--yolo" => {
                            overrides.yolo = true;
                        }
                        "--add-dir" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.add_dirs.push(value.clone());
                                i += 1;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                Ok(ProviderCliOverrides::new(Provider::Codex, overrides))
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
                let config = layer.payload::<CodexConfig>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "Codex layer payload type mismatch".to_owned(),
                        source: None,
                    }
                })?;
                Ok((layer.source.clone(), config))
            })
            .collect::<Result<Vec<_>>>()?;
        let cli = cli
            .and_then(|value| value.payload::<CodexCliOverrides>().cloned())
            .unwrap_or_default();
        let mut sources = typed_layers
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        if cli.sandbox_mode.is_some()
            || cli.approval_policy.is_some()
            || cli.yolo
            || !cli.add_dirs.is_empty()
        {
            sources.push(PolicySource {
                id: "codex-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            });
        }
        Ok(NativeEffectivePolicy::new(
            Provider::Codex,
            sources,
            CodexState {
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
                .payload::<CodexState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "codex-effective".to_owned(),
                    message: "Codex effective policy payload type mismatch".to_owned(),
                    source: None,
                })?;

        let mut policy = CanonicalPolicy::empty(
            Provider::Codex,
            if has_cli(&state.cli) {
                PolicyMode::Effective
            } else {
                PolicyMode::Configured
            },
        );

        let effective_sandbox = effective_sandbox_mode(state);
        let effective_approval = effective_approval_policy(state);
        let repo_root = ctx.repo_root.as_ref().unwrap_or(&ctx.cwd);

        policy.axes.runtime.sandbox_mode = Some(match effective_sandbox.as_str() {
            "danger-full-access" => CanonicalSandboxMode::None,
            "workspace-write" => CanonicalSandboxMode::Partial,
            _ => CanonicalSandboxMode::Full,
        });
        policy.axes.runtime.approval_mode = Some(match effective_approval.as_str() {
            "never" => CanonicalApprovalMode::AutoApprove,
            _ => CanonicalApprovalMode::AlwaysAsk,
        });
        policy.axes.runtime.can_bypass_permissions = if state.cli.yolo
            || (effective_sandbox == "danger-full-access" && effective_approval == "never")
        {
            TernaryState::Yes
        } else {
            TernaryState::No
        };
        policy.axes.runtime.project_trust_required = TernaryState::Yes;

        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect: PolicyEffect::Allow,
            provenance: CanonicalRuleProvenance::approximate(
                super::common::first_source_id(native, "codex-derived"),
                "sandbox_mode",
                "Codex sandboxing constrains writes more strongly than reads.",
            ),
        });

        let protected_paths = [
            repo_root.join(".git"),
            repo_root.join(".codex"),
            repo_root.join(".agents"),
        ];
        for path in protected_paths {
            let display = path.to_string_lossy().into_owned();
            policy.axes.filesystem.write_rules.push(PathAccessRule {
                pattern: display.clone(),
                effect: PolicyEffect::Deny,
                provenance: CanonicalRuleProvenance::approximate(
                    super::common::first_source_id(native, "codex-derived"),
                    "sandbox protected paths",
                    "Protected paths remain read-only under Codex sandboxing.",
                ),
            });
            policy
                .axes
                .filesystem
                .protected_config_paths
                .push(PathProtectionRule {
                    pattern: display,
                    description: "Codex protected path".to_owned(),
                    provenance: CanonicalRuleProvenance::approximate(
                        super::common::first_source_id(native, "codex-derived"),
                        "sandbox protected paths",
                        "Protected path query surface is broader than native config.",
                    ),
                });
        }

        match effective_sandbox.as_str() {
            "read-only" => {
                policy.axes.filesystem.write_rules.push(PathAccessRule {
                    pattern: "*".to_owned(),
                    effect: PolicyEffect::Deny,
                    provenance: CanonicalRuleProvenance::exact(
                        super::common::first_source_id(native, "codex-derived"),
                        "sandbox_mode",
                    ),
                });
            }
            "workspace-write" => {
                policy.axes.filesystem.write_rules.push(PathAccessRule {
                    pattern: repo_root.to_string_lossy().into_owned(),
                    effect: PolicyEffect::Allow,
                    provenance: CanonicalRuleProvenance::exact(
                        super::common::first_source_id(native, "codex-derived"),
                        "sandbox_mode",
                    ),
                });
                for root in effective_writable_roots(state) {
                    policy.axes.filesystem.write_rules.push(PathAccessRule {
                        pattern: root,
                        effect: PolicyEffect::Allow,
                        provenance: CanonicalRuleProvenance::exact(
                            super::common::first_source_id(native, "codex-derived"),
                            "sandbox_workspace_write.writable_roots",
                        ),
                    });
                }
                policy.axes.filesystem.write_rules.push(PathAccessRule {
                    pattern: "*".to_owned(),
                    effect: if effective_approval == "never" {
                        PolicyEffect::Deny
                    } else {
                        PolicyEffect::Ask
                    },
                    provenance: CanonicalRuleProvenance::approximate(
                        super::common::first_source_id(native, "codex-derived"),
                        "approval_policy",
                        "Outside-workspace writes map to sandbox escalation prompts or denial.",
                    ),
                });
            }
            _ => {
                policy.axes.filesystem.write_rules.push(PathAccessRule {
                    pattern: "*".to_owned(),
                    effect: PolicyEffect::Allow,
                    provenance: CanonicalRuleProvenance::exact(
                        super::common::first_source_id(native, "codex-derived"),
                        "sandbox_mode",
                    ),
                });
            }
        }

        policy.axes.commands.shell_rules.push(CommandAccessRule {
            pattern: "*".to_owned(),
            effect: if effective_approval == "never" {
                PolicyEffect::Allow
            } else {
                PolicyEffect::Ask
            },
            provenance: CanonicalRuleProvenance::approximate(
                super::common::first_source_id(native, "codex-derived"),
                "approval_policy",
                "Codex execution rules are not fully parsed in phase 4.",
            ),
        });

        if let Some(profile) = effective_profile(state) {
            if let Some(enabled) = profile.network_enabled {
                policy.axes.network.enabled = if enabled {
                    TernaryState::Yes
                } else {
                    TernaryState::No
                };
            }
            for domain in &profile.allowed_domains {
                policy.axes.network.domain_rules.push(DomainAccessRule {
                    pattern: domain.clone(),
                    effect: PolicyEffect::Allow,
                    provenance: CanonicalRuleProvenance::exact(
                        super::common::first_source_id(native, "codex-derived"),
                        "permissions.<profile>.network.allowed_domains",
                    ),
                });
            }
            for domain in &profile.denied_domains {
                policy.axes.network.domain_rules.push(DomainAccessRule {
                    pattern: domain.clone(),
                    effect: PolicyEffect::Deny,
                    provenance: CanonicalRuleProvenance::exact(
                        super::common::first_source_id(native, "codex-derived"),
                        "permissions.<profile>.network.denied_domains",
                    ),
                });
            }
        } else {
            policy.axes.network.enabled = match effective_sandbox.as_str() {
                "danger-full-access" => TernaryState::Yes,
                "workspace-write" => {
                    if state
                        .layers
                        .iter()
                        .find_map(|(_, cfg)| cfg.network_access)
                        .unwrap_or(false)
                    {
                        TernaryState::Yes
                    } else {
                        TernaryState::No
                    }
                }
                _ => TernaryState::No,
            };
        }

        for (source, config) in &state.layers {
            for (server, server_config) in &config.mcp_servers {
                let server_enabled = server_config.enabled.unwrap_or(true);
                policy.axes.mcp.server_rules.push(McpServerRule {
                    server_id: server.clone(),
                    effect: if server_enabled {
                        PolicyEffect::Allow
                    } else {
                        PolicyEffect::Deny
                    },
                    provenance: CanonicalRuleProvenance::exact(
                        source.id.clone(),
                        format!("mcp_servers.{server}.enabled"),
                    ),
                });

                if !server_enabled {
                    policy.axes.mcp.tool_rules.push(McpToolRule {
                        server_id: server.clone(),
                        tool_name: "*".to_owned(),
                        effect: PolicyEffect::Deny,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            format!("mcp_servers.{server}.enabled"),
                        ),
                    });
                    continue;
                }

                for tool in &server_config.disabled_tools {
                    policy.axes.mcp.tool_rules.push(McpToolRule {
                        server_id: server.clone(),
                        tool_name: tool.clone(),
                        effect: PolicyEffect::Deny,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            format!("mcp_servers.{server}.disabled_tools"),
                        ),
                    });
                }
                for tool in &server_config.enabled_tools {
                    policy.axes.mcp.tool_rules.push(McpToolRule {
                        server_id: server.clone(),
                        tool_name: tool.clone(),
                        effect: PolicyEffect::Allow,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            format!("mcp_servers.{server}.enabled_tools"),
                        ),
                    });
                }
                policy.axes.mcp.tool_rules.push(McpToolRule {
                    server_id: server.clone(),
                    tool_name: "*".to_owned(),
                    effect: if server_config.enabled_tools.is_empty() {
                        PolicyEffect::Allow
                    } else {
                        PolicyEffect::Deny
                    },
                    provenance: CanonicalRuleProvenance::approximate(
                        source.id.clone(),
                        format!("mcp_servers.{server}"),
                        "Codex server-level MCP policy is modeled from enabled state and tool filters.",
                    ),
                });
            }
        }

        if ctx.trust.is_trusted.is_none() && repo_config_exists(ctx).await {
            policy.warnings.push(PolicyWarning {
                code: "codex.trust_unknown".to_owned(),
                message: "Repo-scoped Codex config is trust-gated and trust was not supplied."
                    .to_owned(),
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
        let (config_source_id, config_path) = choose_config_target(ctx, current, change.target)?;
        let before_text = fs::read_to_string(&config_path).await.ok();
        let mut doc = before_text
            .as_deref()
            .unwrap_or("")
            .parse::<DocumentMut>()
            .unwrap_or_default();

        let mut generated_rules = String::new();
        for operation in &change.operations {
            match operation {
                PolicyChangeOp::GrantWrite(path) => {
                    push_toml_array_string(
                        &mut doc,
                        "sandbox_workspace_write",
                        "writable_roots",
                        path.to_string_lossy().as_ref(),
                    );
                }
                PolicyChangeOp::SetApprovalMode(mode) => {
                    doc["approval_policy"] = value(map_approval_mode(mode));
                }
                PolicyChangeOp::SetSandboxMode(mode) => {
                    doc["sandbox_mode"] = value(map_sandbox_mode(mode));
                }
                PolicyChangeOp::AllowMcpServer(server) => {
                    set_toml_bool(&mut doc, &format!("mcp_servers.{server}"), "enabled", true);
                }
                PolicyChangeOp::DenyMcpServer(server) => {
                    set_toml_bool(&mut doc, &format!("mcp_servers.{server}"), "enabled", false);
                }
                PolicyChangeOp::AllowMcpTool { server, tool } => {
                    push_toml_array_string(
                        &mut doc,
                        &format!("mcp_servers.{server}"),
                        "enabled_tools",
                        tool,
                    );
                    remove_toml_array_string(
                        &mut doc,
                        &format!("mcp_servers.{server}"),
                        "disabled_tools",
                        tool,
                    );
                }
                PolicyChangeOp::DenyMcpTool { server, tool } => {
                    push_toml_array_string(
                        &mut doc,
                        &format!("mcp_servers.{server}"),
                        "disabled_tools",
                        tool,
                    );
                    remove_toml_array_string(
                        &mut doc,
                        &format!("mcp_servers.{server}"),
                        "enabled_tools",
                        tool,
                    );
                }
                PolicyChangeOp::AllowCommand(CommandPattern { raw })
                | PolicyChangeOp::RequireApprovalForCommand(CommandPattern { raw })
                | PolicyChangeOp::DenyCommand(CommandPattern { raw }) => {
                    let decision = match operation {
                        PolicyChangeOp::AllowCommand(_) => "allow",
                        PolicyChangeOp::RequireApprovalForCommand(_) => "prompt",
                        PolicyChangeOp::DenyCommand(_) => "forbidden",
                        _ => unreachable!(),
                    };
                    generated_rules.push_str(&format!(
                        "prefix_rule(\n    pattern = [\"{raw}\"],\n    decision = \"{decision}\",\n    justification = \"Generated by Claudine\"\n)\n\n"
                    ));
                }
                other => {
                    return Err(ClaudineError::PolicyUnsupportedMutation {
                        provider: Provider::Codex,
                        op: format!("{other:?}"),
                    });
                }
            }
        }

        let mut edits = vec![ConfigEditPlan {
            source_id: config_source_id,
            path: config_path.clone(),
            description: "Update Codex config".to_owned(),
            before_preview: before_text,
            after_preview: doc.to_string(),
        }];

        let one_shot_plan = build_one_shot_plan(change)?;

        if !generated_rules.is_empty() {
            let rules_dir = config_path
                .parent()
                .map(|path| path.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            let rules_path = rules_dir.join("rules/claudine-generated.rules");
            let rules_before = fs::read_to_string(&rules_path).await.ok();
            edits.push(ConfigEditPlan {
                source_id: "codex-rules".to_owned(),
                path: rules_path,
                description: "Update Codex execution rules".to_owned(),
                before_preview: rules_before,
                after_preview: generated_rules,
            });
        }

        Ok(PolicyMutationPlan {
            provider: Provider::Codex,
            persistent_plan: if change.persistence == PolicyPersistence::OneShot {
                None
            } else {
                Some(PersistentMutationPlan {
                    edits,
                    fidelity: MappingFidelity::Exact,
                })
            },
            one_shot_plan,
            warnings: Vec::new(),
            supported: true,
        })
    }
}

fn parse_codex_config(doc: &DocumentMut) -> CodexConfig {
    let mut config = CodexConfig {
        sandbox_mode: doc
            .get("sandbox_mode")
            .and_then(Item::as_str)
            .map(str::to_owned),
        approval_policy: parse_approval_policy(doc),
        writable_roots: doc
            .get("sandbox_workspace_write")
            .and_then(Item::as_table)
            .and_then(|table| table.get("writable_roots"))
            .and_then(Item::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        network_access: doc
            .get("sandbox_workspace_write")
            .and_then(Item::as_table)
            .and_then(|table| table.get("network_access"))
            .and_then(Item::as_bool),
        default_permissions: doc
            .get("default_permissions")
            .and_then(Item::as_str)
            .map(str::to_owned),
        permissions_profiles: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
    };

    if let Some(permissions) = doc.get("permissions").and_then(Item::as_table_like) {
        for (name, item) in permissions.iter() {
            if let Some(table) = item.as_table_like() {
                let network = table.get("network").and_then(Item::as_table_like);
                let profile = CodexProfile {
                    network_enabled: network
                        .and_then(|table| table.get("enabled"))
                        .and_then(Item::as_bool),
                    allowed_domains: string_array(
                        network.and_then(|table| table.get("allowed_domains")),
                    ),
                    denied_domains: string_array(
                        network.and_then(|table| table.get("denied_domains")),
                    ),
                };
                config.permissions_profiles.insert(name.to_owned(), profile);
            }
        }
    }

    if let Some(mcp_servers) = doc.get("mcp_servers").and_then(Item::as_table_like) {
        for (name, item) in mcp_servers.iter() {
            if let Some(table) = item.as_table_like() {
                config.mcp_servers.insert(
                    name.to_owned(),
                    CodexMcpServer {
                        enabled: table.get("enabled").and_then(Item::as_bool),
                        enabled_tools: string_array(table.get("enabled_tools")),
                        disabled_tools: string_array(table.get("disabled_tools")),
                    },
                );
            }
        }
    }

    config
}

fn parse_approval_policy(doc: &DocumentMut) -> Option<String> {
    let item = doc.get("approval_policy")?;
    if let Some(value) = item.as_str() {
        return Some(value.to_owned());
    }
    if item.is_inline_table() || item.is_table() {
        return Some("on-request".to_owned());
    }
    None
}

fn string_array(item: Option<&Item>) -> Vec<String> {
    item.and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn effective_sandbox_mode(state: &CodexState) -> String {
    if state.cli.yolo {
        return "danger-full-access".to_owned();
    }
    state
        .cli
        .sandbox_mode
        .clone()
        .or_else(|| {
            state
                .layers
                .iter()
                .find_map(|(_, cfg)| cfg.sandbox_mode.clone())
        })
        .unwrap_or_else(|| "read-only".to_owned())
}

fn effective_approval_policy(state: &CodexState) -> String {
    if state.cli.yolo {
        return "never".to_owned();
    }
    state
        .cli
        .approval_policy
        .clone()
        .or_else(|| {
            state
                .layers
                .iter()
                .find_map(|(_, cfg)| cfg.approval_policy.clone())
        })
        .unwrap_or_else(|| "on-request".to_owned())
}

fn effective_writable_roots(state: &CodexState) -> Vec<String> {
    let mut roots = state
        .layers
        .iter()
        .flat_map(|(_, cfg)| cfg.writable_roots.clone())
        .collect::<Vec<_>>();
    roots.extend(state.cli.add_dirs.iter().cloned());
    roots
}

fn effective_profile(state: &CodexState) -> Option<CodexProfile> {
    let name = state
        .layers
        .iter()
        .find_map(|(_, cfg)| cfg.default_permissions.clone())?;
    state
        .layers
        .iter()
        .find_map(|(_, cfg)| cfg.permissions_profiles.get(&name).cloned())
}

fn choose_config_target(
    ctx: &PolicyContext,
    current: &NativeEffectivePolicy,
    target: PolicyChangeTarget,
) -> Result<(String, PathBuf)> {
    let repo_path = ctx
        .repo_root
        .as_ref()
        .map(|root| root.join(".codex/config.toml"));
    let user_path = ctx
        .home_dir
        .as_ref()
        .map(|home| home.join(".codex/config.toml"));
    match target {
        PolicyChangeTarget::UserConfig => user_path
            .map(|path| ("codex-user".to_owned(), path))
            .ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Codex user config path is unavailable".to_owned(),
                )
            }),
        PolicyChangeTarget::RepoConfig => repo_path
            .map(|path| ("codex-repo".to_owned(), path))
            .ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Codex repo config path is unavailable".to_owned(),
                )
            }),
        PolicyChangeTarget::LocalOverride => Err(ClaudineError::PolicyUnsupportedMutation {
            provider: Provider::Codex,
            op: "LocalOverride target is not supported by Codex (no local override concept)"
                .to_owned(),
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
                return Ok(("codex-repo".to_owned(), path));
            }
            user_path
                .map(|path| ("codex-user".to_owned(), path))
                .or_else(|| repo_path.map(|path| ("codex-repo".to_owned(), path)))
                .ok_or_else(|| {
                    ClaudineError::PolicyAmbiguousContext(
                        "Could not determine Codex mutation target".to_owned(),
                    )
                })
        }
    }
}

fn build_one_shot_plan(change: &PolicyChange) -> Result<Option<OneShotMutationPlan>> {
    let mut argv = Vec::new();
    for operation in &change.operations {
        match operation {
            PolicyChangeOp::GrantWrite(path) => {
                argv.push("--add-dir".to_owned());
                argv.push(path.to_string_lossy().into_owned());
            }
            PolicyChangeOp::SetApprovalMode(mode) => {
                argv.push("--ask-for-approval".to_owned());
                argv.push(map_approval_mode(mode).to_owned());
            }
            PolicyChangeOp::SetSandboxMode(mode) => {
                argv.push("--sandbox".to_owned());
                argv.push(map_sandbox_mode(mode).to_owned());
            }
            PolicyChangeOp::AllowMcpServer(server) => {
                argv.push("-c".to_owned());
                argv.push(format!("mcp_servers.{server}.enabled=true"));
            }
            PolicyChangeOp::DenyMcpServer(server) => {
                argv.push("-c".to_owned());
                argv.push(format!("mcp_servers.{server}.enabled=false"));
            }
            _ => {}
        }
    }
    if argv.is_empty() {
        return Ok(None);
    }
    Ok(Some(super::common::one_shot_plan(
        argv,
        MappingFidelity::Approximate,
    )))
}

fn map_approval_mode(mode: &CanonicalApprovalMode) -> &'static str {
    match mode {
        CanonicalApprovalMode::AlwaysAsk => "on-request",
        CanonicalApprovalMode::AutoApprove => "never",
        CanonicalApprovalMode::SuggestOnly => "untrusted",
    }
}

fn map_sandbox_mode(mode: &CanonicalSandboxMode) -> &'static str {
    match mode {
        CanonicalSandboxMode::Full => "read-only",
        CanonicalSandboxMode::Partial => "workspace-write",
        CanonicalSandboxMode::None => "danger-full-access",
    }
}

fn push_toml_array_string(
    doc: &mut DocumentMut,
    table_key: &str,
    array_key: &str,
    value_to_add: &str,
) {
    let table = ensure_toml_table_path(doc, table_key);
    if table.get(array_key).is_none() || !table[array_key].is_array() {
        table[array_key] = value(Array::default());
    }
    if let Some(array) = table[array_key].as_array_mut()
        && !array
            .iter()
            .any(|entry| entry.as_str() == Some(value_to_add))
    {
        array.push(value_to_add);
    }
}

fn remove_toml_array_string(
    doc: &mut DocumentMut,
    table_key: &str,
    array_key: &str,
    value_to_remove: &str,
) {
    let table = ensure_toml_table_path(doc, table_key);
    if let Some(array) = table.get_mut(array_key).and_then(Item::as_array_mut) {
        let retained = array
            .iter()
            .filter_map(|entry| entry.as_str())
            .filter(|entry| *entry != value_to_remove)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mut next = Array::default();
        for entry in retained {
            next.push(entry);
        }
        table[array_key] = value(next);
    }
}

fn set_toml_bool(doc: &mut DocumentMut, table_key: &str, value_key: &str, enabled: bool) {
    let table = ensure_toml_table_path(doc, table_key);
    table[value_key] = value(enabled);
}

fn ensure_toml_table_path<'a>(
    doc: &'a mut DocumentMut,
    table_key: &str,
) -> &'a mut toml_edit::Table {
    let mut current = doc.as_table_mut();
    for segment in table_key.split('.') {
        if current.get(segment).is_none() || !current[segment].is_table() {
            current[segment] = Item::Table(Default::default());
        }
        current = current[segment]
            .as_table_mut()
            .expect("ensure_toml_table_path: segment was just set to Item::Table above");
    }
    current
}

async fn repo_config_exists(ctx: &PolicyContext) -> bool {
    match &ctx.repo_root {
        Some(root) => fs::try_exists(root.join(".codex/config.toml"))
            .await
            .unwrap_or(false),
        None => false,
    }
}

fn has_cli(cli: &CodexCliOverrides) -> bool {
    cli.sandbox_mode.is_some()
        || cli.approval_policy.is_some()
        || cli.yolo
        || !cli.add_dirs.is_empty()
}

#[cfg(test)]
mod tests;
