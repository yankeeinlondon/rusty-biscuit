use crate::events::Provider;

/// Provider-agnostic canonical policy model.
///
/// Produced by canonicalizing provider-native policy state. Every rule
/// retains provenance back to the native source that produced it.
#[derive(Debug, Clone)]
pub struct CanonicalPolicy {
    /// Provider this policy was produced from.
    pub provider: Provider,
    /// Whether this represents configured or effective policy.
    pub mode: PolicyMode,
    /// Policy rules organized by security axis.
    pub axes: CanonicalPolicyAxes,
    /// Provenance chain linking canonical rules to native sources.
    pub provenance: Vec<CanonicalRuleProvenance>,
    /// Warnings emitted during canonicalization.
    pub warnings: Vec<PolicyWarning>,
}

/// Whether a policy snapshot represents on-disk config or runtime-effective state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    /// Represents filesystem-configured state only.
    Configured,
    /// Represents effective state including CLI/runtime overrides.
    Effective,
}

/// Policy rules organized by cross-provider security axis.
#[derive(Debug, Clone, Default)]
pub struct CanonicalPolicyAxes {
    /// Filesystem read, write, and traversal rules.
    pub filesystem: FilesystemPolicy,
    /// Shell command execution rules.
    pub commands: CommandPolicy,
    /// Network and domain access rules.
    pub network: NetworkPolicy,
    /// MCP server and tool access rules.
    pub mcp: McpAccessPolicy,
    /// Subagent and mode-switch rules.
    pub agents: AgentPolicy,
    /// Approval, sandbox, and bypass state.
    pub runtime: RuntimePolicy,
}

// --- Filesystem axis ---

/// Filesystem access policy.
#[derive(Debug, Clone, Default)]
pub struct FilesystemPolicy {
    /// Path read access rules in precedence order.
    pub read_rules: Vec<PathAccessRule>,
    /// Path write access rules in precedence order.
    pub write_rules: Vec<PathAccessRule>,
    /// Directory traversal rules in precedence order.
    pub traversal_rules: Vec<PathAccessRule>,
    /// Protected config paths that should not be modified.
    pub protected_config_paths: Vec<PathProtectionRule>,
}

/// A rule governing access to a path pattern.
#[derive(Debug, Clone)]
pub struct PathAccessRule {
    /// Path pattern (exact path, prefix, or glob).
    pub pattern: String,
    /// Effect when the pattern matches.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

/// A rule protecting a specific config path from modification.
#[derive(Debug, Clone)]
pub struct PathProtectionRule {
    /// Path pattern identifying the protected path.
    pub pattern: String,
    /// Human-readable description of why this path is protected.
    pub description: String,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

// --- Command axis ---

/// Command execution policy.
#[derive(Debug, Clone, Default)]
pub struct CommandPolicy {
    /// Shell command access rules in precedence order.
    pub shell_rules: Vec<CommandAccessRule>,
    /// Execution mode rules.
    pub execution_modes: Vec<ExecutionModeRule>,
}

/// A rule governing execution of a shell command pattern.
#[derive(Debug, Clone)]
pub struct CommandAccessRule {
    /// Command pattern (executable name, prefix, or regex).
    pub pattern: String,
    /// Effect when the pattern matches.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

/// A rule governing a named execution mode.
#[derive(Debug, Clone)]
pub struct ExecutionModeRule {
    /// Mode name (e.g., "full-auto", "yolo").
    pub mode_name: String,
    /// Effect for this mode.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

// --- Network axis ---

/// Network and domain access policy.
#[derive(Debug, Clone)]
pub struct NetworkPolicy {
    /// Whether network access is enabled overall.
    pub enabled: TernaryState,
    /// Domain-specific access rules in precedence order.
    pub domain_rules: Vec<DomainAccessRule>,
    /// Whether local network binding is permitted.
    pub local_binding: TernaryState,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            enabled: TernaryState::Unknown,
            domain_rules: Vec::new(),
            local_binding: TernaryState::Unknown,
        }
    }
}

/// A rule governing access to a network domain.
#[derive(Debug, Clone)]
pub struct DomainAccessRule {
    /// Domain pattern (exact domain or wildcard like `*.example.com`).
    pub pattern: String,
    /// Effect when the pattern matches.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

// --- MCP axis ---

/// MCP server and tool access policy.
#[derive(Debug, Clone, Default)]
pub struct McpAccessPolicy {
    /// MCP server access rules.
    pub server_rules: Vec<McpServerRule>,
    /// MCP tool access rules.
    pub tool_rules: Vec<McpToolRule>,
}

/// A rule governing access to an MCP server.
#[derive(Debug, Clone)]
pub struct McpServerRule {
    /// Server identifier or pattern.
    pub server_id: String,
    /// Effect when matched.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

/// A rule governing access to a specific MCP tool.
#[derive(Debug, Clone)]
pub struct McpToolRule {
    /// Server identifier.
    pub server_id: String,
    /// Tool name or pattern.
    pub tool_name: String,
    /// Effect when matched.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

// --- Agents axis ---

/// Subagent and mode-switch policy.
#[derive(Debug, Clone, Default)]
pub struct AgentPolicy {
    /// Subagent spawning rules.
    pub subagent_rules: Vec<SubagentRule>,
    /// Mode switch rules.
    pub mode_switch_rules: Vec<ModeSwitchRule>,
}

/// A rule governing subagent spawning.
#[derive(Debug, Clone)]
pub struct SubagentRule {
    /// Subagent name filter. `None` matches any subagent.
    pub name: Option<String>,
    /// Effect when matched.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

/// A rule governing mode switches.
#[derive(Debug, Clone)]
pub struct ModeSwitchRule {
    /// Target mode name. `None` matches any mode switch.
    pub target: Option<String>,
    /// Effect when matched.
    pub effect: PolicyEffect,
    /// Provenance linking this rule to a native source.
    pub provenance: CanonicalRuleProvenance,
}

// --- Runtime axis ---

/// Runtime approval, sandbox, and bypass state.
#[derive(Debug, Clone, Default)]
pub struct RuntimePolicy {
    /// Canonical approval mode if known.
    pub approval_mode: Option<CanonicalApprovalMode>,
    /// Canonical sandbox mode if known.
    pub sandbox_mode: Option<CanonicalSandboxMode>,
    /// Whether the agent can bypass permission checks entirely.
    pub can_bypass_permissions: TernaryState,
    /// Whether project trust is required for repo-scoped policy.
    pub project_trust_required: TernaryState,
}

/// Canonical approval mode across providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalApprovalMode {
    /// Every tool invocation requires user approval.
    AlwaysAsk,
    /// Tool invocations are approved automatically.
    AutoApprove,
    /// Suggestions are shown but not enforced.
    SuggestOnly,
}

/// Canonical sandbox mode across providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalSandboxMode {
    /// Full OS-level sandboxing is active.
    Full,
    /// Partial sandboxing (e.g., network only, or FS only).
    Partial,
    /// No sandboxing.
    None,
}

// --- Decision vocabulary ---

/// Effect of a matched policy rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    /// The action is permitted.
    Allow,
    /// The action requires user approval.
    Ask,
    /// The action is denied.
    Deny,
}

/// Confidence level of a policy answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyCertainty {
    /// The answer is authoritative from the provider's config.
    Exact,
    /// The answer is a best-effort interpretation.
    BestEffort,
    /// The answer is not determinable from available information.
    Unknown,
}

/// Fidelity of a native-to-canonical rule mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingFidelity {
    /// The canonical rule exactly represents the native rule.
    Exact,
    /// The canonical rule is stricter than the native rule.
    Narrowed,
    /// The canonical rule is more permissive than the native rule.
    Broadened,
    /// The canonical rule is an approximation.
    Approximate,
}

/// Three-valued state for boolean policy fields that may be unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TernaryState {
    /// Affirmative.
    Yes,
    /// Negative.
    No,
    /// Not determinable.
    #[default]
    Unknown,
}

// --- Provenance and warnings ---

/// Links a canonical rule back to the native source that produced it.
#[derive(Debug, Clone)]
pub struct CanonicalRuleProvenance {
    /// Identifier of the source that produced this rule.
    pub source_id: String,
    /// Provider-native reference (e.g., config key path or field name).
    pub native_reference: Option<String>,
    /// Fidelity of the native-to-canonical mapping.
    pub fidelity: MappingFidelity,
    /// Optional human-readable note about the mapping.
    pub note: Option<String>,
}

/// Warning emitted during policy resolution or canonicalization.
#[derive(Debug, Clone)]
pub struct PolicyWarning {
    /// Machine-readable warning code.
    pub code: String,
    /// Human-readable warning message.
    pub message: String,
    /// Source identifier related to this warning, if applicable.
    pub source_id: Option<String>,
}

impl CanonicalPolicy {
    /// Creates an empty canonical policy for a provider.
    pub fn empty(provider: Provider, mode: PolicyMode) -> Self {
        Self {
            provider,
            mode,
            axes: CanonicalPolicyAxes::default(),
            provenance: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl CanonicalRuleProvenance {
    /// Creates provenance with exact fidelity.
    pub fn exact(source_id: impl Into<String>, native_reference: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            native_reference: Some(native_reference.into()),
            fidelity: MappingFidelity::Exact,
            note: None,
        }
    }

    /// Creates provenance with approximate fidelity and a note.
    pub fn approximate(
        source_id: impl Into<String>,
        native_reference: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            native_reference: Some(native_reference.into()),
            fidelity: MappingFidelity::Approximate,
            note: Some(note.into()),
        }
    }
}
