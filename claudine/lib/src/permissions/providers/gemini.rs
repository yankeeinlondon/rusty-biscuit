use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::fs;
use toml_edit::{DocumentMut, Item};

use crate::error::{ClaudineError, Result};
use crate::permissions::backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};
use crate::permissions::canonical::{
    CanonicalApprovalMode, CanonicalPolicy, CanonicalRuleProvenance, CanonicalSandboxMode,
    CommandAccessRule, MappingFidelity, McpServerRule, PathAccessRule, PathProtectionRule,
    PolicyEffect, PolicyMode, PolicyWarning, SubagentRule, TernaryState,
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
struct GeminiSettings {
    default_approval_mode: Option<String>,
    sandbox_enabled: Option<bool>,
    mcp_allowed: Vec<String>,
}

#[derive(Debug, Clone)]
struct GeminiRule {
    tool_names: Vec<String>,
    command_prefix: Option<String>,
    mcp_name: Option<String>,
    subagent: Option<String>,
    decision: PolicyEffect,
}

#[derive(Debug, Clone, Default)]
struct GeminiLayerData {
    settings: Option<GeminiSettings>,
    rules: Vec<GeminiRule>,
}

#[derive(Debug, Clone, Default)]
struct GeminiCliOverrides {
    approval_mode: Option<String>,
    sandbox: Option<bool>,
    allowed_tools: Vec<String>,
}

#[derive(Debug, Clone)]
struct GeminiState {
    layers: Vec<(PolicySource, GeminiLayerData)>,
    cli: GeminiCliOverrides,
}

#[derive(Debug, Default)]
pub(crate) struct GeminiPolicyBackend;

#[async_trait]
impl ProviderPolicyBackend for GeminiPolicyBackend {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            fidelity: BackendFidelity::High,
            filesystem_queries: true,
            command_queries: true,
            network_queries: false,
            mcp_queries: true,
            agent_queries: true,
            persistent_mutations: true,
            one_shot_mutations: true,
        }
    }

    async fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
        let mut sources = Vec::new();

        if let Some(system_root) = &ctx.system_root {
            let settings = system_root.join("etc/gemini-cli/settings.json");
            if fs::try_exists(&settings).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "gemini-system-settings".to_owned(),
                    kind: PolicySourceKind::SystemConfig,
                    path: Some(settings),
                    precedence: 10,
                    writable: false,
                });
            }
            let policy_dir = system_root.join("etc/gemini-cli/policies");
            if fs::try_exists(&policy_dir).await.unwrap_or(false) {
                let mut entries = fs::read_dir(&policy_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    if entry.path().extension().and_then(|ext| ext.to_str()) == Some("toml") {
                        sources.push(PolicySource {
                            id: format!(
                                "gemini-system-policy:{}",
                                entry.file_name().to_string_lossy()
                            ),
                            kind: PolicySourceKind::RuleFile,
                            path: Some(entry.path()),
                            precedence: 15,
                            writable: false,
                        });
                    }
                }
            }
        }

        if let Some(home) = &ctx.home_dir {
            let settings = home.join(".gemini/settings.json");
            if fs::try_exists(&settings).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "gemini-user-settings".to_owned(),
                    kind: PolicySourceKind::UserConfig,
                    path: Some(settings),
                    precedence: 40,
                    writable: true,
                });
            }
            let policy_dir = home.join(".gemini/policies");
            if fs::try_exists(&policy_dir).await.unwrap_or(false) {
                let mut entries = fs::read_dir(&policy_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    if entry.path().extension().and_then(|ext| ext.to_str()) == Some("toml") {
                        sources.push(PolicySource {
                            id: format!(
                                "gemini-user-policy:{}",
                                entry.file_name().to_string_lossy()
                            ),
                            kind: PolicySourceKind::RuleFile,
                            path: Some(entry.path()),
                            precedence: 35,
                            writable: true,
                        });
                    }
                }
            }
        }

        if ctx.trust.is_trusted == Some(true)
            && let Some(repo_root) = &ctx.repo_root
        {
            let settings = repo_root.join(".gemini/settings.json");
            if fs::try_exists(&settings).await.unwrap_or(false) {
                sources.push(PolicySource {
                    id: "gemini-repo-settings".to_owned(),
                    kind: PolicySourceKind::RepoConfig,
                    path: Some(settings),
                    precedence: 20,
                    writable: true,
                });
            }
            let policy_dir = repo_root.join(".gemini/policies");
            if fs::try_exists(&policy_dir).await.unwrap_or(false) {
                let mut entries = fs::read_dir(&policy_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    if entry.path().extension().and_then(|ext| ext.to_str()) == Some("toml") {
                        sources.push(PolicySource {
                            id: format!(
                                "gemini-repo-policy:{}",
                                entry.file_name().to_string_lossy()
                            ),
                            kind: PolicySourceKind::RuleFile,
                            path: Some(entry.path()),
                            precedence: 25,
                            writable: true,
                        });
                    }
                }
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
                    "Gemini source `{}` is missing a path",
                    source.id
                ))
            })?;
            let content = fs::read_to_string(path).await?;
            let data = if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                let value: Value = serde_json::from_str(&content).map_err(|error| {
                    ClaudineError::PolicyNativeParse {
                        source_id: source.id.clone(),
                        message: error.to_string(),
                    }
                })?;
                GeminiLayerData {
                    settings: Some(parse_gemini_settings(&value)),
                    rules: Vec::new(),
                }
            } else {
                let doc: DocumentMut = content.parse().map_err(|error: toml_edit::TomlError| {
                    ClaudineError::PolicyNativeParse {
                        source_id: source.id.clone(),
                        message: error.to_string(),
                    }
                })?;
                GeminiLayerData {
                    settings: None,
                    rules: parse_gemini_rules(&doc),
                }
            };
            layers.push(NativePolicyLayer::new(source.clone(), data));
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
                Provider::Gemini,
                GeminiCliOverrides::default(),
            )),
            CliPolicyInput::Parsed(parsed) => {
                if parsed.provider != Provider::Gemini {
                    return Err(ClaudineError::PolicyCliParse {
                        provider: Provider::Gemini,
                        message: "parsed CLI overrides belong to another provider".to_owned(),
                    });
                }
                let typed = parsed
                    .payload::<GeminiCliOverrides>()
                    .cloned()
                    .ok_or_else(|| ClaudineError::PolicyCliParse {
                        provider: Provider::Gemini,
                        message: "parsed CLI overrides had an unexpected payload type".to_owned(),
                    })?;
                Ok(ProviderCliOverrides::new(Provider::Gemini, typed))
            }
            CliPolicyInput::Argv(argv) => {
                let mut overrides = GeminiCliOverrides::default();
                let mut i = 0;
                while i < argv.len() {
                    match argv[i].as_str() {
                        "--approval-mode" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.approval_mode = Some(value.clone());
                                i += 1;
                            }
                        }
                        "--yolo" => overrides.approval_mode = Some("yolo".to_owned()),
                        "--sandbox" | "-s" => overrides.sandbox = Some(true),
                        "--allowed-tools" => {
                            if let Some(value) = argv.get(i + 1) {
                                overrides.allowed_tools.push(value.clone());
                                i += 1;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                Ok(ProviderCliOverrides::new(Provider::Gemini, overrides))
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
                let data = layer.payload::<GeminiLayerData>().cloned().ok_or_else(|| {
                    ClaudineError::PolicyNativeParse {
                        source_id: layer.source.id.clone(),
                        message: "Gemini layer payload type mismatch".to_owned(),
                    }
                })?;
                Ok((layer.source.clone(), data))
            })
            .collect::<Result<Vec<_>>>()?;

        let cli = cli
            .and_then(|value| value.payload::<GeminiCliOverrides>().cloned())
            .unwrap_or_default();
        let mut sources = typed_layers
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>();
        if cli.approval_mode.is_some() || cli.sandbox.is_some() || !cli.allowed_tools.is_empty() {
            sources.push(PolicySource {
                id: "gemini-cli".to_owned(),
                kind: PolicySourceKind::CliOverride,
                path: None,
                precedence: 5,
                writable: false,
            });
        }
        Ok(NativeEffectivePolicy::new(
            Provider::Gemini,
            sources,
            GeminiState {
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
                .payload::<GeminiState>()
                .ok_or_else(|| ClaudineError::PolicyNativeParse {
                    source_id: "gemini-effective".to_owned(),
                    message: "Gemini effective policy payload type mismatch".to_owned(),
                })?;

        let mut policy = CanonicalPolicy::empty(
            Provider::Gemini,
            if has_cli(&state.cli) {
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
                        description: "Gemini settings or policy file".to_owned(),
                        provenance: CanonicalRuleProvenance::approximate(
                            source.id.clone(),
                            "settings/policy",
                            "Policy files should be treated as protected configuration.",
                        ),
                    });
            }
        }

        let approval_mode = state
            .cli
            .approval_mode
            .clone()
            .or_else(|| {
                state.layers.iter().find_map(|(_, layer)| {
                    layer
                        .settings
                        .as_ref()
                        .and_then(|settings| settings.default_approval_mode.clone())
                })
            })
            .unwrap_or_else(|| "default".to_owned());

        policy.axes.runtime.approval_mode = Some(map_gemini_approval_mode(&approval_mode));
        policy.axes.runtime.sandbox_mode = Some(
            if state.cli.sandbox == Some(true)
                || state.layers.iter().any(|(_, layer)| {
                    layer
                        .settings
                        .as_ref()
                        .and_then(|settings| settings.sandbox_enabled)
                        == Some(true)
                })
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
        policy.axes.runtime.project_trust_required = TernaryState::Yes;

        for (source, layer) in &state.layers {
            if let Some(settings) = &layer.settings {
                for server in &settings.mcp_allowed {
                    policy.axes.mcp.server_rules.push(McpServerRule {
                        server_id: server.clone(),
                        effect: PolicyEffect::Allow,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            "mcp.allowed",
                        ),
                    });
                }
            }
            for rule in &layer.rules {
                if rule
                    .tool_names
                    .iter()
                    .any(|tool| tool == "shell" || tool == "execute")
                    && let Some(prefix) = &rule.command_prefix
                {
                    policy.axes.commands.shell_rules.push(CommandAccessRule {
                        pattern: prefix.clone(),
                        effect: rule.decision,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            "policy.rule.commandPrefix",
                        ),
                    });
                }
                if rule.tool_names.iter().any(|tool| tool == "read") {
                    policy.axes.filesystem.read_rules.push(PathAccessRule {
                        pattern: "*".to_owned(),
                        effect: rule.decision,
                        provenance: CanonicalRuleProvenance::approximate(
                            source.id.clone(),
                            "policy.rule.toolName",
                            "Gemini read rules are tool-scoped, not path-scoped.",
                        ),
                    });
                }
                if rule.tool_names.iter().any(|tool| tool == "edit") {
                    policy.axes.filesystem.write_rules.push(PathAccessRule {
                        pattern: "*".to_owned(),
                        effect: rule.decision,
                        provenance: CanonicalRuleProvenance::approximate(
                            source.id.clone(),
                            "policy.rule.toolName",
                            "Gemini edit rules are tool-scoped, not path-scoped.",
                        ),
                    });
                }
                if let Some(server) = &rule.mcp_name {
                    policy.axes.mcp.server_rules.push(McpServerRule {
                        server_id: server.clone(),
                        effect: rule.decision,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            "policy.rule.mcpName",
                        ),
                    });
                }
                if let Some(subagent) = &rule.subagent {
                    policy.axes.agents.subagent_rules.push(SubagentRule {
                        name: Some(subagent.clone()),
                        effect: rule.decision,
                        provenance: CanonicalRuleProvenance::exact(
                            source.id.clone(),
                            "policy.rule.subagent",
                        ),
                    });
                }
            }
        }

        policy.axes.filesystem.read_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect: PolicyEffect::Allow,
            provenance: CanonicalRuleProvenance::approximate(
                super::common::first_source_id(native, "gemini-derived"),
                "approval mode",
                "Gemini read operations are modeled as broadly allowed.",
            ),
        });
        policy.axes.filesystem.write_rules.push(PathAccessRule {
            pattern: "*".to_owned(),
            effect: match approval_mode.as_str() {
                "auto_edit" | "yolo" => PolicyEffect::Allow,
                _ => PolicyEffect::Ask,
            },
            provenance: CanonicalRuleProvenance::approximate(
                super::common::first_source_id(native, "gemini-derived"),
                "approval mode",
                "Gemini write operations are modeled from approval mode.",
            ),
        });
        policy.axes.commands.shell_rules.push(CommandAccessRule {
            pattern: "*".to_owned(),
            effect: match approval_mode.as_str() {
                "yolo" => PolicyEffect::Allow,
                _ => PolicyEffect::Ask,
            },
            provenance: CanonicalRuleProvenance::approximate(
                super::common::first_source_id(native, "gemini-derived"),
                "approval mode",
                "Gemini shell execution is modeled from approval mode when no explicit policy rule matches.",
            ),
        });

        if ctx.trust.is_trusted.is_none() && repo_policy_exists(ctx).await {
            policy.warnings.push(PolicyWarning {
                code: "gemini.trust_unknown".to_owned(),
                message: "Gemini repo settings are trust-gated and trust was not supplied."
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
        let (settings_source, settings_path, policy_source, policy_path) =
            choose_targets(ctx, current, change.target)?;

        let settings_before = fs::read_to_string(&settings_path).await.ok();
        let mut settings_value = settings_before
            .as_deref()
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .unwrap_or_else(|| json!({}));
        let policy_before = fs::read_to_string(&policy_path).await.ok();
        let mut generated_rules = String::new();

        for operation in &change.operations {
            match operation {
                PolicyChangeOp::SetApprovalMode(mode) => {
                    *ensure_json_value(&mut settings_value, &["general", "defaultApprovalMode"]) =
                        Value::String(map_approval_mode_to_setting(*mode).to_owned());
                }
                PolicyChangeOp::SetSandboxMode(mode) => {
                    *ensure_json_value(&mut settings_value, &["tools", "sandbox", "enabled"]) =
                        Value::Bool(!matches!(mode, CanonicalSandboxMode::None));
                }
                PolicyChangeOp::AllowCommand(CommandPattern { raw })
                | PolicyChangeOp::RequireApprovalForCommand(CommandPattern { raw })
                | PolicyChangeOp::DenyCommand(CommandPattern { raw }) => {
                    let decision = match operation {
                        PolicyChangeOp::AllowCommand(_) => "allow",
                        PolicyChangeOp::RequireApprovalForCommand(_) => "ask_user",
                        PolicyChangeOp::DenyCommand(_) => "deny",
                        _ => unreachable!(),
                    };
                    generated_rules.push_str(&format!(
                        "[[rule]]\ntoolName = \"shell\"\ncommandPrefix = \"{raw}\"\ndecision = \"{decision}\"\npriority = 100\n\n"
                    ));
                }
                PolicyChangeOp::AllowMcpServer(server) | PolicyChangeOp::DenyMcpServer(server) => {
                    let decision = if matches!(operation, PolicyChangeOp::AllowMcpServer(_)) {
                        "allow"
                    } else {
                        "deny"
                    };
                    generated_rules.push_str(&format!(
                        "[[rule]]\nmcpName = \"{server}\"\ndecision = \"{decision}\"\npriority = 100\n\n"
                    ));
                }
                other => {
                    return Err(ClaudineError::PolicyUnsupportedMutation {
                        provider: Provider::Gemini,
                        op: format!("{other:?}"),
                    });
                }
            }
        }

        let mut edits = Vec::new();
        if change.operations.iter().any(|op| {
            matches!(
                op,
                PolicyChangeOp::SetApprovalMode(_) | PolicyChangeOp::SetSandboxMode(_)
            )
        }) {
            edits.push(ConfigEditPlan {
                source_id: settings_source,
                path: settings_path,
                description: "Update Gemini settings".to_owned(),
                before_preview: settings_before,
                after_preview: serde_json::to_string_pretty(&settings_value)? + "\n",
            });
        }
        if !generated_rules.is_empty() {
            edits.push(ConfigEditPlan {
                source_id: policy_source,
                path: policy_path,
                description: "Update Gemini policy rules".to_owned(),
                before_preview: policy_before,
                after_preview: generated_rules,
            });
        }

        Ok(PolicyMutationPlan {
            provider: Provider::Gemini,
            persistent_plan: if change.persistence == PolicyPersistence::OneShot {
                None
            } else {
                Some(PersistentMutationPlan {
                    edits,
                    fidelity: MappingFidelity::Exact,
                })
            },
            one_shot_plan: build_one_shot_plan(change)?,
            warnings: Vec::new(),
            supported: true,
        })
    }
}

fn parse_gemini_settings(value: &Value) -> GeminiSettings {
    GeminiSettings {
        default_approval_mode: value
            .get("general")
            .and_then(Value::as_object)
            .and_then(|general| general.get("defaultApprovalMode"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        sandbox_enabled: value
            .get("tools")
            .and_then(Value::as_object)
            .and_then(|tools| tools.get("sandbox"))
            .and_then(Value::as_object)
            .and_then(|sandbox| sandbox.get("enabled"))
            .and_then(Value::as_bool),
        mcp_allowed: value
            .get("mcp")
            .and_then(Value::as_object)
            .and_then(|mcp| mcp.get("allowed"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    }
}

fn parse_gemini_rules(doc: &DocumentMut) -> Vec<GeminiRule> {
    let Some(rule_items) = doc.get("rule").and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };

    rule_items
        .iter()
        .map(|table| GeminiRule {
            tool_names: if let Some(name) = table.get("toolName").and_then(Item::as_str) {
                vec![name.to_owned()]
            } else {
                table
                    .get("toolName")
                    .and_then(Item::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_owned))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            },
            command_prefix: table
                .get("commandPrefix")
                .and_then(Item::as_str)
                .map(str::to_owned),
            mcp_name: table
                .get("mcpName")
                .and_then(Item::as_str)
                .map(str::to_owned),
            subagent: table
                .get("subagent")
                .and_then(Item::as_str)
                .map(str::to_owned),
            decision: match table.get("decision").and_then(Item::as_str) {
                Some("allow") => PolicyEffect::Allow,
                Some("deny") => PolicyEffect::Deny,
                _ => PolicyEffect::Ask,
            },
        })
        .collect()
}

fn map_gemini_approval_mode(mode: &str) -> CanonicalApprovalMode {
    match mode {
        "auto_edit" | "yolo" => CanonicalApprovalMode::AutoApprove,
        "plan" => CanonicalApprovalMode::SuggestOnly,
        _ => CanonicalApprovalMode::AlwaysAsk,
    }
}

fn map_approval_mode_to_setting(mode: CanonicalApprovalMode) -> &'static str {
    match mode {
        CanonicalApprovalMode::AlwaysAsk => "default",
        CanonicalApprovalMode::AutoApprove => "auto_edit",
        CanonicalApprovalMode::SuggestOnly => "plan",
    }
}

fn choose_targets(
    ctx: &PolicyContext,
    current: &NativeEffectivePolicy,
    target: PolicyChangeTarget,
) -> Result<(String, PathBuf, String, PathBuf)> {
    let repo_settings = ctx
        .repo_root
        .as_ref()
        .map(|root| root.join(".gemini/settings.json"));
    let repo_policy = ctx
        .repo_root
        .as_ref()
        .map(|root| root.join(".gemini/policies/claudine-policy.toml"));
    let user_settings = ctx
        .home_dir
        .as_ref()
        .map(|home| home.join(".gemini/settings.json"));
    let user_policy = ctx
        .home_dir
        .as_ref()
        .map(|home| home.join(".gemini/policies/claudine-policy.toml"));

    match target {
        PolicyChangeTarget::UserConfig => Ok((
            "gemini-user-settings".to_owned(),
            user_settings.ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Gemini user settings path is unavailable".to_owned(),
                )
            })?,
            "gemini-user-policy".to_owned(),
            user_policy.ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Gemini user policy path is unavailable".to_owned(),
                )
            })?,
        )),
        PolicyChangeTarget::RepoConfig => Ok((
            "gemini-repo-settings".to_owned(),
            repo_settings.ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Gemini repo settings path is unavailable".to_owned(),
                )
            })?,
            "gemini-repo-policy".to_owned(),
            repo_policy.ok_or_else(|| {
                ClaudineError::PolicyAmbiguousContext(
                    "Gemini repo policy path is unavailable".to_owned(),
                )
            })?,
        )),
        PolicyChangeTarget::LocalOverride => Err(ClaudineError::PolicyUnsupportedMutation {
            provider: Provider::Gemini,
            op: "LocalOverride target is not supported by Gemini (no local override concept)"
                .to_owned(),
        }),
        PolicyChangeTarget::Auto => {
            let has_repo = current.sources.iter().any(|source| {
                matches!(
                    source.kind,
                    PolicySourceKind::RepoConfig | PolicySourceKind::RuleFile
                )
            });
            if has_repo
                && ctx.trust.is_trusted == Some(true)
                && let (Some(settings), Some(policy)) = (repo_settings.clone(), repo_policy.clone())
            {
                return Ok((
                    "gemini-repo-settings".to_owned(),
                    settings,
                    "gemini-repo-policy".to_owned(),
                    policy,
                ));
            }
            Ok((
                "gemini-user-settings".to_owned(),
                user_settings.or(repo_settings).ok_or_else(|| {
                    ClaudineError::PolicyAmbiguousContext(
                        "Could not determine Gemini settings target".to_owned(),
                    )
                })?,
                "gemini-user-policy".to_owned(),
                user_policy.or(repo_policy).ok_or_else(|| {
                    ClaudineError::PolicyAmbiguousContext(
                        "Could not determine Gemini policy target".to_owned(),
                    )
                })?,
            ))
        }
    }
}

fn build_one_shot_plan(change: &PolicyChange) -> Result<Option<OneShotMutationPlan>> {
    let mut argv = Vec::new();
    for operation in &change.operations {
        match operation {
            PolicyChangeOp::SetApprovalMode(mode) => {
                argv.push("--approval-mode".to_owned());
                argv.push(
                    match mode {
                        CanonicalApprovalMode::AlwaysAsk => "default",
                        CanonicalApprovalMode::AutoApprove => "auto_edit",
                        CanonicalApprovalMode::SuggestOnly => "plan",
                    }
                    .to_owned(),
                );
            }
            PolicyChangeOp::SetSandboxMode(mode) => {
                if !matches!(mode, CanonicalSandboxMode::None) {
                    argv.push("--sandbox".to_owned());
                }
            }
            PolicyChangeOp::AllowCommand(CommandPattern { raw }) => {
                argv.push("--allowed-tools".to_owned());
                argv.push(raw.clone());
            }
            _ => return Ok(None),
        }
    }
    Ok(Some(super::common::one_shot_plan(
        argv,
        MappingFidelity::Approximate,
    )))
}

fn has_cli(cli: &GeminiCliOverrides) -> bool {
    cli.approval_mode.is_some() || cli.sandbox.is_some() || !cli.allowed_tools.is_empty()
}

async fn repo_policy_exists(ctx: &PolicyContext) -> bool {
    let Some(repo_root) = &ctx.repo_root else {
        return false;
    };

    let settings = repo_root.join(".gemini/settings.json");
    if fs::try_exists(&settings).await.unwrap_or(false) {
        return true;
    }

    let policy_dir = repo_root.join(".gemini/policies");
    if !fs::try_exists(&policy_dir).await.unwrap_or(false) {
        return false;
    }

    match fs::read_dir(&policy_dir).await {
        Ok(mut entries) => {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.path().extension().and_then(|ext| ext.to_str()) == Some("toml") {
                    return true;
                }
            }
            false
        }
        Err(_) => false,
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
}
