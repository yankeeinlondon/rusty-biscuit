use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{ClaudineError, Result};
use crate::events::Provider;

/// Posture controls baseline aggressiveness for medium/high risk actions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectPosture {
    /// Never block in-line. Only collect findings and recommend next actions.
    Advisory,
    /// Ask for risky actions, block critical actions.
    #[default]
    Balanced,
    /// Prefer hard stops for any high-confidence dangerous behavior.
    Strict,
}

impl std::fmt::Display for ProtectPosture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtectPosture::Advisory => write!(f, "advisory"),
            ProtectPosture::Balanced => write!(f, "balanced"),
            ProtectPosture::Strict => write!(f, "strict"),
        }
    }
}

/// Runtime mode for a single evaluation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectRuntimeMode {
    #[default]
    Normal,
    Yolo,
}

impl ProtectRuntimeMode {
    #[allow(dead_code)]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ProtectRuntimeMode::Normal => "normal",
            ProtectRuntimeMode::Yolo => "yolo",
        }
    }
}

/// High-level event phase where a protect decision is being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectPhase {
    BeforePrompt,
    BeforeTool,
    AfterTool,
    McpResponse,
    Completion,
    SubagentStart,
    SubagentStop,
    Runtime,
}

/// Risk level assigned by upstream detection logic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Serializable top-level configuration for the Protect service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectConfig {
    /// Global on/off switch for protect evaluations.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Baseline policy strictness.
    #[serde(default)]
    pub posture: ProtectPosture,

    /// Allow repository-level config to weaken stricter user posture.
    #[serde(default)]
    pub allow_repo_posture_downgrade: bool,

    /// Behavior overrides when the provider is in bypass/YOLO posture.
    #[serde(default)]
    pub yolo: YoloPolicy,

    /// Generic command/path/secret rules.
    ///
    /// **Deprecation notice:** `blocked_command_patterns`, `ask_command_patterns`,
    /// and `protected_paths` duplicate policy truth that should live in
    /// PolicyEngine. Only `secret_patterns` is actively consumed by the
    /// runtime redaction pipeline. The other fields will be removed in a
    /// future version.
    #[serde(default)]
    pub rules: ProtectRules,

    /// Completion gate behavior.
    #[serde(default)]
    pub completion: CompletionPolicy,

    /// MCP trust and redaction policy.
    #[serde(default)]
    pub mcp: McpPolicy,

    /// Subagent policy controls.
    #[serde(default)]
    pub subagents: SubagentPolicy,

    /// Privilege/runtime hardening controls.
    #[serde(default)]
    pub privilege: PrivilegePolicy,

    /// Optional provider-specific overrides.
    #[serde(default)]
    pub providers: HashMap<Provider, ProviderProtectOverride>,

    /// Max in-memory decision records retained for forensic inspection.
    #[serde(default = "default_max_recent_decisions")]
    pub max_recent_decisions: u16,
}

impl Default for ProtectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            posture: ProtectPosture::Balanced,
            allow_repo_posture_downgrade: false,
            yolo: YoloPolicy::default(),
            rules: ProtectRules::default(),
            completion: CompletionPolicy::default(),
            mcp: McpPolicy::default(),
            subagents: SubagentPolicy::default(),
            privilege: PrivilegePolicy::default(),
            providers: HashMap::default(),
            max_recent_decisions: default_max_recent_decisions(),
        }
    }
}

impl ProtectConfig {
    /// Reject deprecated fields that have migrated to PolicyEngine.
    ///
    /// Previously these emitted warnings; they now produce validation errors
    /// to prevent silent acceptance of config that has no runtime effect.
    fn validate_deprecated_fields(&self) -> Result<()> {
        if !self.rules.blocked_command_patterns.is_empty() {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.rules.blocked_command_patterns is removed; migrate to PolicyEngine rules".to_string(),
            ));
        }
        if !self.rules.ask_command_patterns.is_empty() {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.rules.ask_command_patterns is removed; migrate to PolicyEngine rules".to_string(),
            ));
        }
        if !self.rules.protected_paths.is_empty() {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.rules.protected_paths is removed; migrate to PolicyEngine rules".to_string(),
            ));
        }
        if !self.mcp.allowlist.is_empty() {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.mcp.allowlist is removed; use MCP catalog trust instead".to_string(),
            ));
        }
        if !self.mcp.denylist.is_empty() {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.mcp.denylist is removed; use MCP catalog trust instead".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate semantic constraints and regex configuration.
    ///
    /// Deprecated fields that formerly emitted warnings now produce hard errors.
    /// Only `secret_patterns` and MCP `redact_patterns`/`block_instruction_payloads`
    /// remain active runtime controls.
    pub fn validate(&self) -> Result<()> {
        self.validate_deprecated_fields()?;

        if self.max_recent_decisions == 0 {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.max_recent_decisions must be > 0".to_string(),
            ));
        }

        if self.max_recent_decisions > 10_000 {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.max_recent_decisions must be <= 10000".to_string(),
            ));
        }

        if self.completion.enabled && self.completion.max_retries == 0 {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.completion.max_retries must be > 0 when completion is enabled"
                    .to_string(),
            ));
        }

        validate_patterns(&self.rules.secret_patterns)?;
        validate_patterns(&self.mcp.redact_patterns)?;

        for (provider, override_cfg) in &self.providers {
            if let Some(rules) = override_cfg.rules.as_ref() {
                if let Some(patterns) = rules.secret_patterns.as_ref() {
                    validate_patterns(patterns)?;
                }
            }
            if let Some(mcp) = override_cfg.mcp.as_ref()
                && let Some(patterns) = mcp.redact_patterns.as_ref()
            {
                validate_patterns(patterns)?;
            }
            if let Some(completion) = override_cfg.completion.as_ref()
                && completion.enabled.unwrap_or(false)
                && completion
                    .max_retries
                    .unwrap_or(default_completion_max_retries())
                    == 0
            {
                return Err(ClaudineError::ProtectInvalidPolicy(format!(
                    "settings.protect.providers.{provider}.completion.max_retries must be > 0 when completion is enabled"
                )));
            }
        }

        Ok(())
    }

    /// Merge another protect config on top of this config.
    pub fn merge_with(&self, overlay: &ProtectConfig) -> ProtectConfig {
        let mut merged = self.clone();
        merged.enabled = overlay.enabled;
        merged.posture = overlay.posture;
        merged.allow_repo_posture_downgrade = overlay.allow_repo_posture_downgrade;
        merged.yolo = overlay.yolo.clone();
        merged.rules = overlay.rules.clone();
        merged.completion = overlay.completion.clone();
        merged.mcp = overlay.mcp.clone();
        merged.subagents = overlay.subagents.clone();
        merged.privilege = overlay.privilege.clone();
        merged.max_recent_decisions = overlay.max_recent_decisions;

        for (provider, provider_override) in &overlay.providers {
            merged
                .providers
                .insert(*provider, provider_override.clone());
        }

        merged
    }

    /// Merge one provider override onto this config.
    pub fn merge_provider_override(
        &self,
        provider_override: &ProviderProtectOverride,
    ) -> ProtectConfig {
        let mut merged = self.clone();

        if let Some(enabled) = provider_override.enabled {
            merged.enabled = enabled;
        }
        if let Some(posture) = provider_override.posture {
            merged.posture = posture;
        }

        if let Some(yolo) = provider_override.yolo.as_ref() {
            merged.yolo = merge_yolo_policy(&merged.yolo, yolo);
        }
        if let Some(rules) = provider_override.rules.as_ref() {
            merged.rules = merge_rules_policy(&merged.rules, rules);
        }
        if let Some(completion) = provider_override.completion.as_ref() {
            merged.completion = merge_completion_policy(&merged.completion, completion);
        }
        if let Some(mcp) = provider_override.mcp.as_ref() {
            merged.mcp = merge_mcp_policy(&merged.mcp, mcp);
        }
        if let Some(subagents) = provider_override.subagents.as_ref() {
            merged.subagents = merge_subagent_policy(&merged.subagents, subagents);
        }
        if let Some(privilege) = provider_override.privilege.as_ref() {
            merged.privilege = merge_privilege_policy(&merged.privilege, privilege);
        }

        merged
    }

    /// Build provider-aware defaults for init flows.
    pub fn provider_aware_defaults(
        installed: &[Provider],
        posture: ProtectPosture,
    ) -> ProtectConfig {
        let mut config = ProtectConfig {
            posture,
            ..ProtectConfig::default()
        };

        for provider in installed {
            if matches!(
                provider,
                Provider::Codex | Provider::Goose | Provider::QwenCode
            ) {
                config.providers.insert(
                    *provider,
                    ProviderProtectOverride {
                        posture: Some(ProtectPosture::Advisory),
                        completion: Some(CompletionPolicyOverride {
                            enabled: Some(false),
                            ..CompletionPolicyOverride::default()
                        }),
                        ..ProviderProtectOverride::default()
                    },
                );
            }
        }

        config
    }
}

pub(crate) fn validate_patterns(patterns: &[String]) -> Result<()> {
    for pattern in patterns {
        Regex::new(pattern).map_err(|source| ClaudineError::ProtectRuleParse {
            pattern: pattern.clone(),
            source,
        })?;
    }
    Ok(())
}

fn merge_yolo_policy(base: &YoloPolicy, overlay: &YoloPolicyOverride) -> YoloPolicy {
    YoloPolicy {
        allow_critical_blocking: overlay
            .allow_critical_blocking
            .unwrap_or(base.allow_critical_blocking),
        force_advisory_for_medium_risk: overlay
            .force_advisory_for_medium_risk
            .unwrap_or(base.force_advisory_for_medium_risk),
        collect_forensic_trail: overlay
            .collect_forensic_trail
            .unwrap_or(base.collect_forensic_trail),
    }
}

fn merge_rules_policy(base: &ProtectRules, overlay: &ProtectRulesOverride) -> ProtectRules {
    ProtectRules {
        blocked_command_patterns: overlay
            .blocked_command_patterns
            .clone()
            .unwrap_or_else(|| base.blocked_command_patterns.clone()),
        ask_command_patterns: overlay
            .ask_command_patterns
            .clone()
            .unwrap_or_else(|| base.ask_command_patterns.clone()),
        protected_paths: overlay
            .protected_paths
            .clone()
            .unwrap_or_else(|| base.protected_paths.clone()),
        secret_patterns: overlay
            .secret_patterns
            .clone()
            .unwrap_or_else(|| base.secret_patterns.clone()),
    }
}

fn merge_completion_policy(
    base: &CompletionPolicy,
    overlay: &CompletionPolicyOverride,
) -> CompletionPolicy {
    CompletionPolicy {
        enabled: overlay.enabled.unwrap_or(base.enabled),
        max_retries: overlay.max_retries.unwrap_or(base.max_retries),
        check_commands: overlay
            .check_commands
            .clone()
            .unwrap_or_else(|| base.check_commands.clone()),
        secret_scan: overlay.secret_scan.unwrap_or(base.secret_scan),
    }
}

/// Merge an MCP policy overlay onto a base policy (public for cross-module use).
pub(crate) fn merge_mcp_policy_pub(base: &McpPolicy, overlay: &McpPolicyOverride) -> McpPolicy {
    merge_mcp_policy(base, overlay)
}

fn merge_mcp_policy(base: &McpPolicy, overlay: &McpPolicyOverride) -> McpPolicy {
    McpPolicy {
        allowlist: overlay
            .allowlist
            .clone()
            .unwrap_or_else(|| base.allowlist.clone()),
        denylist: overlay
            .denylist
            .clone()
            .unwrap_or_else(|| base.denylist.clone()),
        redact_patterns: overlay
            .redact_patterns
            .clone()
            .unwrap_or_else(|| base.redact_patterns.clone()),
        block_instruction_payloads: overlay
            .block_instruction_payloads
            .unwrap_or(base.block_instruction_payloads),
    }
}

fn merge_subagent_policy(
    base: &SubagentPolicy,
    overlay: &SubagentPolicyOverride,
) -> SubagentPolicy {
    SubagentPolicy {
        enabled: overlay.enabled.unwrap_or(base.enabled),
        tighten_permissions: overlay
            .tighten_permissions
            .unwrap_or(base.tighten_permissions),
        default_profile: overlay.default_profile.unwrap_or(base.default_profile),
    }
}

fn merge_privilege_policy(
    base: &PrivilegePolicy,
    overlay: &PrivilegePolicyOverride,
) -> PrivilegePolicy {
    PrivilegePolicy {
        deny_when_root_without_sandbox: overlay
            .deny_when_root_without_sandbox
            .unwrap_or(base.deny_when_root_without_sandbox),
        require_ask_for_network_writes: overlay
            .require_ask_for_network_writes
            .unwrap_or(base.require_ask_for_network_writes),
        require_ask_for_broad_fs_writes: overlay
            .require_ask_for_broad_fs_writes
            .unwrap_or(base.require_ask_for_broad_fs_writes),
    }
}

fn default_enabled() -> bool {
    true
}

pub(crate) fn default_max_recent_decisions() -> u16 {
    256
}

/// Optional provider-specific overrides layered on top of [`ProtectConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProtectOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub posture: Option<ProtectPosture>,
    #[serde(default)]
    pub yolo: Option<YoloPolicyOverride>,
    #[serde(default)]
    pub rules: Option<ProtectRulesOverride>,
    #[serde(default)]
    pub completion: Option<CompletionPolicyOverride>,
    #[serde(default)]
    pub mcp: Option<McpPolicyOverride>,
    #[serde(default)]
    pub subagents: Option<SubagentPolicyOverride>,
    #[serde(default)]
    pub privilege: Option<PrivilegePolicyOverride>,
}

/// Controls how policy behaves in bypass/YOLO runtime mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YoloPolicy {
    #[serde(default = "default_allow_critical_blocking")]
    pub allow_critical_blocking: bool,
    #[serde(default = "default_force_advisory_for_medium_risk")]
    pub force_advisory_for_medium_risk: bool,
    #[serde(default = "default_collect_forensic_trail")]
    pub collect_forensic_trail: bool,
}

impl Default for YoloPolicy {
    fn default() -> Self {
        Self {
            allow_critical_blocking: default_allow_critical_blocking(),
            force_advisory_for_medium_risk: default_force_advisory_for_medium_risk(),
            collect_forensic_trail: default_collect_forensic_trail(),
        }
    }
}

/// Partial provider override for [`YoloPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YoloPolicyOverride {
    #[serde(default)]
    pub allow_critical_blocking: Option<bool>,
    #[serde(default)]
    pub force_advisory_for_medium_risk: Option<bool>,
    #[serde(default)]
    pub collect_forensic_trail: Option<bool>,
}

fn default_allow_critical_blocking() -> bool {
    true
}

fn default_force_advisory_for_medium_risk() -> bool {
    true
}

fn default_collect_forensic_trail() -> bool {
    true
}

/// Shared tool/prompt risk matching patterns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectRules {
    #[serde(default)]
    pub blocked_command_patterns: Vec<String>,
    #[serde(default)]
    pub ask_command_patterns: Vec<String>,
    #[serde(default)]
    pub protected_paths: Vec<String>,
    #[serde(default)]
    pub secret_patterns: Vec<String>,
}

/// Partial provider override for [`ProtectRules`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectRulesOverride {
    #[serde(default)]
    pub blocked_command_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub ask_command_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub protected_paths: Option<Vec<String>>,
    #[serde(default)]
    pub secret_patterns: Option<Vec<String>>,
}

/// Completion validation controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionPolicy {
    #[serde(default = "default_true_value")]
    pub enabled: bool,
    #[serde(default = "default_completion_max_retries")]
    pub max_retries: u8,
    #[serde(default)]
    pub check_commands: Vec<String>,
    #[serde(default = "default_true_value")]
    pub secret_scan: bool,
}

impl Default for CompletionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: default_completion_max_retries(),
            check_commands: Vec::new(),
            secret_scan: true,
        }
    }
}

/// Partial provider override for [`CompletionPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionPolicyOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub max_retries: Option<u8>,
    #[serde(default)]
    pub check_commands: Option<Vec<String>>,
    #[serde(default)]
    pub secret_scan: Option<bool>,
}

pub(crate) fn default_completion_max_retries() -> u8 {
    3
}

/// MCP trust and response handling policy.
///
/// **Deprecation notice:** `allowlist` and `denylist` duplicate MCP server
/// trust that should be managed by the MCP catalog and PolicyEngine.
/// Only `redact_patterns` and `block_instruction_payloads` are runtime
/// redaction controls. The list fields will be removed in a future version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpPolicy {
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub denylist: Vec<String>,
    #[serde(default)]
    pub redact_patterns: Vec<String>,
    #[serde(default = "default_true_value")]
    pub block_instruction_payloads: bool,
}

impl Default for McpPolicy {
    fn default() -> Self {
        Self {
            allowlist: Vec::new(),
            denylist: Vec::new(),
            redact_patterns: Vec::new(),
            block_instruction_payloads: true,
        }
    }
}

/// Partial provider override for [`McpPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpPolicyOverride {
    #[serde(default)]
    pub allowlist: Option<Vec<String>>,
    #[serde(default)]
    pub denylist: Option<Vec<String>>,
    #[serde(default)]
    pub redact_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub block_instruction_payloads: Option<bool>,
}

/// Subagent defaults used when providers allow subagent-specific controls.
///
/// **Deprecation notice:** Subagent permission policy should be managed by
/// PolicyEngine. This struct will be removed in a future version; retain
/// only runtime behavior knobs like `tighten_permissions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubagentPolicy {
    #[serde(default = "default_true_value")]
    pub enabled: bool,
    #[serde(default = "default_true_value")]
    pub tighten_permissions: bool,
    #[serde(default)]
    pub default_profile: SubagentProfile,
}

impl Default for SubagentPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            tighten_permissions: true,
            default_profile: SubagentProfile::ReadMostly,
        }
    }
}

/// Partial provider override for [`SubagentPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubagentPolicyOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub tighten_permissions: Option<bool>,
    #[serde(default)]
    pub default_profile: Option<SubagentProfile>,
}

/// Named subagent permission profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentProfile {
    #[default]
    ReadMostly,
    Default,
    Isolated,
}

/// Hardening knobs for elevated privilege and weak isolation environments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivilegePolicy {
    #[serde(default = "default_true_value")]
    pub deny_when_root_without_sandbox: bool,
    #[serde(default = "default_true_value")]
    pub require_ask_for_network_writes: bool,
    #[serde(default = "default_true_value")]
    pub require_ask_for_broad_fs_writes: bool,
}

impl Default for PrivilegePolicy {
    fn default() -> Self {
        Self {
            deny_when_root_without_sandbox: true,
            require_ask_for_network_writes: true,
            require_ask_for_broad_fs_writes: true,
        }
    }
}

/// Partial provider override for [`PrivilegePolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivilegePolicyOverride {
    #[serde(default)]
    pub deny_when_root_without_sandbox: Option<bool>,
    #[serde(default)]
    pub require_ask_for_network_writes: Option<bool>,
    #[serde(default)]
    pub require_ask_for_broad_fs_writes: Option<bool>,
}

fn default_true_value() -> bool {
    true
}
