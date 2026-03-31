use std::path::{Path, PathBuf};

use crate::events::Provider;

use super::canonical::{
    CanonicalPolicy, MappingFidelity, PolicyCertainty, PolicyEffect, PolicyWarning, TernaryState,
};
use super::explain::{ExplanationReason, PolicyExplanation};
use super::matchers;
use super::native::{NativeEffectivePolicy, ProviderCliOverrides};

// --- Query types ---

/// A policy query to evaluate against a snapshot.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PolicyQuery {
    /// Can a path be read?
    ReadPath(PathQuery),
    /// Can a path be written?
    WritePath(PathQuery),
    /// Can a directory tree be traversed?
    TraversePath(PathQuery),
    /// Can a shell command be executed?
    ExecuteCommand(CommandQuery),
    /// Can a network domain be accessed?
    AccessDomain(DomainQuery),
    /// Can an MCP server be used?
    UseMcpServer {
        /// Server identifier.
        server: String,
    },
    /// Can a specific MCP tool be used?
    UseMcpTool {
        /// Server identifier.
        server: String,
        /// Tool name.
        tool: String,
    },
    /// Can a subagent be spawned?
    SpawnSubagent {
        /// Subagent name, or `None` for any.
        name: Option<String>,
    },
    /// Can a mode switch be performed?
    SwitchMode {
        /// Target mode, or `None` for any.
        target: Option<String>,
    },
    /// Can the provider's own config be modified?
    ModifyProviderConfig,
}

/// Path query with optional kind hint.
#[derive(Debug, Clone)]
pub struct PathQuery {
    /// The path being queried.
    pub path: PathBuf,
    /// Hint about whether the path is a file or directory.
    pub path_kind: PathKindHint,
}

impl PathQuery {
    /// Creates a path query for a file.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            path_kind: PathKindHint::File,
        }
    }

    /// Creates a path query for a directory.
    pub fn directory(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            path_kind: PathKindHint::Directory,
        }
    }

    /// Creates a path query with unknown kind.
    pub fn unknown(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            path_kind: PathKindHint::Unknown,
        }
    }
}

/// Hint about the kind of path being queried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathKindHint {
    /// Known to be a file.
    File,
    /// Known to be a directory.
    Directory,
    /// Unknown.
    #[default]
    Unknown,
}

/// Shell command query.
#[derive(Debug, Clone)]
pub struct CommandQuery {
    /// Raw command string as the user would type it.
    pub raw: String,
    /// Extracted executable name, if available.
    pub executable: Option<String>,
    /// Parsed argument vector, if available.
    pub argv: Vec<String>,
}

impl CommandQuery {
    /// Creates a command query from a raw command string.
    ///
    /// Extracts the executable name from the first whitespace-delimited token.
    pub fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let executable = raw.split_whitespace().next().map(String::from);
        Self {
            raw,
            executable,
            argv: Vec::new(),
        }
    }
}

/// Network domain query.
#[derive(Debug, Clone)]
pub struct DomainQuery {
    /// Domain name to check.
    pub domain: String,
}

impl DomainQuery {
    /// Creates a domain query.
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
        }
    }
}

// --- Query result ---

/// Result of evaluating a policy query against a snapshot.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Resolved effect, or `None` if the engine cannot determine it.
    pub effect: Option<PolicyEffect>,
    /// Confidence level of the answer.
    pub certainty: PolicyCertainty,
    /// Whether the answer might change with CLI args or at runtime.
    pub stability: QueryStability,
    /// Rules that matched the query, in precedence order.
    pub matched_rules: Vec<MatchedRule>,
    /// Structured explanation of the result.
    pub explanation: PolicyExplanation,
    /// Warnings about the query or answer.
    pub warnings: Vec<PolicyWarning>,
}

/// Whether a query answer might change under different conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryStability {
    /// The answer is stable across invocations.
    Stable,
    /// The answer may change if different CLI args are used.
    MayChangeWithCli,
    /// The answer may change at runtime due to dynamic factors.
    MayChangeAtRuntime,
    /// Stability is unknown.
    Unknown,
}

/// A rule that matched during query evaluation.
#[derive(Debug, Clone)]
pub struct MatchedRule {
    /// Index of the matched rule within its rule list, if applicable.
    pub rule_index: Option<usize>,
    /// Human-readable description of the matched rule.
    pub description: String,
    /// Effect of the matched rule.
    pub effect: PolicyEffect,
    /// Provenance of the matched rule.
    pub provenance: super::canonical::CanonicalRuleProvenance,
}

impl QueryResult {
    /// Creates a result indicating no matching rules were found.
    pub fn no_match() -> Self {
        Self {
            effect: None,
            certainty: PolicyCertainty::Unknown,
            stability: QueryStability::Unknown,
            matched_rules: Vec::new(),
            explanation: PolicyExplanation::no_match("No matching rules found for this query."),
            warnings: Vec::new(),
        }
    }

    /// Returns `true` if the effect is `Allow`.
    pub fn is_allowed(&self) -> bool {
        self.effect == Some(PolicyEffect::Allow)
    }

    /// Returns `true` if the effect is `Deny`.
    pub fn is_denied(&self) -> bool {
        self.effect == Some(PolicyEffect::Deny)
    }

    /// Returns `true` if the effect is `Ask`.
    pub fn is_ask(&self) -> bool {
        self.effect == Some(PolicyEffect::Ask)
    }

    /// Returns `true` if the effect could not be determined.
    pub fn is_unknown(&self) -> bool {
        self.effect.is_none()
    }
}

// --- Snapshot types ---

/// Snapshot of a provider's configured policy (filesystem only).
///
/// Represents the durable, on-disk policy state before any CLI or runtime
/// overrides are applied. All convenience query methods route through
/// [`query`](Self::query).
pub struct ConfiguredPolicySnapshot {
    /// Provider this snapshot belongs to.
    pub provider: Provider,
    /// Native effective policy (composed from filesystem layers only).
    pub native: NativeEffectivePolicy,
    /// Canonical policy produced from the native state.
    pub canonical: CanonicalPolicy,
}

impl ConfiguredPolicySnapshot {
    /// Evaluates a policy query against this configured snapshot.
    pub fn query(&self, query: &PolicyQuery) -> QueryResult {
        resolve_query(&self.canonical, query)
    }

    /// Can the given path be read?
    pub fn can_read(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::ReadPath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given path be written?
    pub fn can_write(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::WritePath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given command be executed?
    pub fn can_execute(&self, query: &CommandQuery) -> QueryResult {
        self.query(&PolicyQuery::ExecuteCommand(query.clone()))
    }

    /// Can the given domain be accessed?
    pub fn can_access_domain(&self, domain: &str) -> QueryResult {
        self.query(&PolicyQuery::AccessDomain(DomainQuery::new(domain)))
    }

    /// Can the given MCP server be used?
    pub fn can_use_mcp_server(&self, server: &str) -> QueryResult {
        self.query(&PolicyQuery::UseMcpServer {
            server: server.to_owned(),
        })
    }

    /// Can the given MCP tool be used?
    pub fn can_use_mcp_tool(&self, server: &str, tool: &str) -> QueryResult {
        self.query(&PolicyQuery::UseMcpTool {
            server: server.to_owned(),
            tool: tool.to_owned(),
        })
    }

    /// Can a subagent be spawned?
    pub fn can_spawn_subagent(&self, name: Option<&str>) -> QueryResult {
        self.query(&PolicyQuery::SpawnSubagent {
            name: name.map(String::from),
        })
    }

    /// Can the provider's own config be modified?
    pub fn can_modify_own_config(&self) -> QueryResult {
        self.query(&PolicyQuery::ModifyProviderConfig)
    }
}

impl std::fmt::Debug for ConfiguredPolicySnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfiguredPolicySnapshot")
            .field("provider", &self.provider)
            .field("canonical", &self.canonical)
            .finish_non_exhaustive()
    }
}

/// Snapshot of a provider's effective policy (config + CLI overrides).
///
/// Represents the runtime-effective policy state after CLI and environment
/// overrides are applied. All convenience query methods route through
/// [`query`](Self::query).
pub struct EffectivePolicySnapshot {
    /// Provider this snapshot belongs to.
    pub provider: Provider,
    /// Native effective policy (composed from all layers including CLI).
    pub native: NativeEffectivePolicy,
    /// Canonical policy produced from the native state.
    pub canonical: CanonicalPolicy,
    /// CLI overrides that were applied.
    pub cli: ProviderCliOverrides,
}

impl EffectivePolicySnapshot {
    /// Evaluates a policy query against this effective snapshot.
    pub fn query(&self, query: &PolicyQuery) -> QueryResult {
        resolve_query(&self.canonical, query)
    }

    /// Can the given path be read?
    pub fn can_read(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::ReadPath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given path be written?
    pub fn can_write(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::WritePath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given command be executed?
    pub fn can_execute(&self, query: &CommandQuery) -> QueryResult {
        self.query(&PolicyQuery::ExecuteCommand(query.clone()))
    }

    /// Can the given domain be accessed?
    pub fn can_access_domain(&self, domain: &str) -> QueryResult {
        self.query(&PolicyQuery::AccessDomain(DomainQuery::new(domain)))
    }

    /// Can the given MCP server be used?
    pub fn can_use_mcp_server(&self, server: &str) -> QueryResult {
        self.query(&PolicyQuery::UseMcpServer {
            server: server.to_owned(),
        })
    }

    /// Can the given MCP tool be used?
    pub fn can_use_mcp_tool(&self, server: &str, tool: &str) -> QueryResult {
        self.query(&PolicyQuery::UseMcpTool {
            server: server.to_owned(),
            tool: tool.to_owned(),
        })
    }

    /// Can a subagent be spawned?
    pub fn can_spawn_subagent(&self, name: Option<&str>) -> QueryResult {
        self.query(&PolicyQuery::SpawnSubagent {
            name: name.map(String::from),
        })
    }

    /// Can the provider's own config be modified?
    pub fn can_modify_own_config(&self) -> QueryResult {
        self.query(&PolicyQuery::ModifyProviderConfig)
    }
}

impl std::fmt::Debug for EffectivePolicySnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectivePolicySnapshot")
            .field("provider", &self.provider)
            .field("canonical", &self.canonical)
            .finish_non_exhaustive()
    }
}

// --- Query resolution ---

/// Resolves a policy query against a canonical policy.
///
/// Uses first-match-wins against rules in precedence order. If no rule
/// matches, returns [`QueryResult::no_match()`].
pub(crate) fn resolve_query(policy: &CanonicalPolicy, query: &PolicyQuery) -> QueryResult {
    match query {
        PolicyQuery::ReadPath(pq) => resolve_path_query(&policy.axes.filesystem.read_rules, pq),
        PolicyQuery::WritePath(pq) => resolve_path_query(&policy.axes.filesystem.write_rules, pq),
        PolicyQuery::TraversePath(pq) => {
            resolve_path_query(&policy.axes.filesystem.traversal_rules, pq)
        }
        PolicyQuery::ExecuteCommand(cq) => {
            resolve_command_query(&policy.axes.commands.shell_rules, cq)
        }
        PolicyQuery::AccessDomain(dq) => resolve_domain_query(&policy.axes.network, dq),
        PolicyQuery::UseMcpServer { server } => {
            resolve_mcp_server_query(&policy.axes.mcp.server_rules, server)
        }
        PolicyQuery::UseMcpTool { server, tool } => {
            resolve_mcp_tool_query(&policy.axes.mcp.tool_rules, server, tool)
        }
        PolicyQuery::SpawnSubagent { name } => {
            resolve_subagent_query(&policy.axes.agents.subagent_rules, name.as_deref())
        }
        PolicyQuery::SwitchMode { target } => {
            resolve_mode_switch_query(&policy.axes.agents.mode_switch_rules, target.as_deref())
        }
        PolicyQuery::ModifyProviderConfig => {
            resolve_config_modify_query(&policy.axes.filesystem.protected_config_paths)
        }
    }
}

fn resolve_path_query(
    rules: &[super::canonical::PathAccessRule],
    pq: &PathQuery,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        if matchers::path_matches(&pq.path, &rule.pattern) {
            return build_single_match_result(
                i,
                &format!("path rule: {}", rule.pattern),
                rule.effect,
                &rule.provenance,
                &format!(
                    "Path `{}` matched rule `{}` -> {:?}",
                    pq.path.display(),
                    rule.pattern,
                    rule.effect,
                ),
            );
        }
    }
    QueryResult::no_match()
}

fn resolve_command_query(
    rules: &[super::canonical::CommandAccessRule],
    cq: &CommandQuery,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        if matchers::command_matches(&cq.raw, cq.executable.as_deref(), &rule.pattern) {
            return build_single_match_result(
                i,
                &format!("command rule: {}", rule.pattern),
                rule.effect,
                &rule.provenance,
                &format!(
                    "Command `{}` matched rule `{}` -> {:?}",
                    cq.raw, rule.pattern, rule.effect,
                ),
            );
        }
    }
    QueryResult::no_match()
}

fn resolve_domain_query(
    network: &super::canonical::NetworkPolicy,
    dq: &DomainQuery,
) -> QueryResult {
    if network.enabled == TernaryState::No {
        return QueryResult {
            effect: Some(PolicyEffect::Deny),
            certainty: PolicyCertainty::Exact,
            stability: QueryStability::Stable,
            matched_rules: Vec::new(),
            explanation: PolicyExplanation::new("Network access is disabled.", Vec::new()),
            warnings: Vec::new(),
        };
    }

    for (i, rule) in network.domain_rules.iter().enumerate() {
        if matchers::domain_matches(&dq.domain, &rule.pattern) {
            return build_single_match_result(
                i,
                &format!("domain rule: {}", rule.pattern),
                rule.effect,
                &rule.provenance,
                &format!(
                    "Domain `{}` matched rule `{}` -> {:?}",
                    dq.domain, rule.pattern, rule.effect,
                ),
            );
        }
    }
    QueryResult::no_match()
}

fn resolve_mcp_server_query(
    rules: &[super::canonical::McpServerRule],
    server: &str,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        if rule.server_id == server || rule.server_id == "*" {
            return build_single_match_result(
                i,
                &format!("MCP server rule: {}", rule.server_id),
                rule.effect,
                &rule.provenance,
                &format!(
                    "MCP server `{server}` matched rule `{}` -> {:?}",
                    rule.server_id, rule.effect,
                ),
            );
        }
    }
    QueryResult::no_match()
}

fn resolve_mcp_tool_query(
    rules: &[super::canonical::McpToolRule],
    server: &str,
    tool: &str,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        let server_match = rule.server_id == server || rule.server_id == "*";
        let tool_match = rule.tool_name == tool || rule.tool_name == "*";
        if server_match && tool_match {
            return build_single_match_result(
                i,
                &format!("MCP tool rule: {}::{}", rule.server_id, rule.tool_name),
                rule.effect,
                &rule.provenance,
                &format!(
                    "MCP tool `{server}::{tool}` matched rule `{}::{}` -> {:?}",
                    rule.server_id, rule.tool_name, rule.effect,
                ),
            );
        }
    }
    QueryResult::no_match()
}

fn resolve_subagent_query(
    rules: &[super::canonical::SubagentRule],
    name: Option<&str>,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        let matches = match (&rule.name, name) {
            (None, _) => true,
            (Some(rn), Some(qn)) => rn == qn,
            (Some(_), None) => false,
        };
        if matches {
            let label = rule.name.as_deref().unwrap_or("*");
            return build_single_match_result(
                i,
                &format!("subagent rule: {label}"),
                rule.effect,
                &rule.provenance,
                &format!(
                    "Subagent `{}` matched rule `{label}` -> {:?}",
                    name.unwrap_or("(any)"),
                    rule.effect,
                ),
            );
        }
    }
    QueryResult::no_match()
}

fn resolve_mode_switch_query(
    rules: &[super::canonical::ModeSwitchRule],
    target: Option<&str>,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        let matches = match (&rule.target, target) {
            (None, _) => true,
            (Some(rt), Some(qt)) => rt == qt,
            (Some(_), None) => false,
        };
        if matches {
            let label = rule.target.as_deref().unwrap_or("*");
            return build_single_match_result(
                i,
                &format!("mode switch rule: {label}"),
                rule.effect,
                &rule.provenance,
                &format!(
                    "Mode switch `{}` matched rule `{label}` -> {:?}",
                    target.unwrap_or("(any)"),
                    rule.effect,
                ),
            );
        }
    }
    QueryResult::no_match()
}

fn resolve_config_modify_query(
    protected_paths: &[super::canonical::PathProtectionRule],
) -> QueryResult {
    if protected_paths.is_empty() {
        return QueryResult::no_match();
    }

    let reasons: Vec<ExplanationReason> = protected_paths
        .iter()
        .map(|rule| ExplanationReason {
            source_id: rule.provenance.source_id.clone(),
            native_reference: rule.provenance.native_reference.clone(),
            message: format!("Protected config: {} -- {}", rule.pattern, rule.description),
            fidelity: rule.provenance.fidelity,
        })
        .collect();

    QueryResult {
        effect: Some(PolicyEffect::Ask),
        certainty: PolicyCertainty::BestEffort,
        stability: QueryStability::Stable,
        matched_rules: Vec::new(),
        explanation: PolicyExplanation::new(
            format!(
                "{} protected config path(s) registered.",
                protected_paths.len(),
            ),
            reasons,
        ),
        warnings: Vec::new(),
    }
}

/// Builds a `QueryResult` for a single matched rule.
fn build_single_match_result(
    index: usize,
    description: &str,
    effect: PolicyEffect,
    provenance: &super::canonical::CanonicalRuleProvenance,
    summary: &str,
) -> QueryResult {
    QueryResult {
        effect: Some(effect),
        certainty: match provenance.fidelity {
            MappingFidelity::Exact => PolicyCertainty::Exact,
            _ => PolicyCertainty::BestEffort,
        },
        stability: QueryStability::Stable,
        matched_rules: vec![MatchedRule {
            rule_index: Some(index),
            description: description.to_owned(),
            effect,
            provenance: provenance.clone(),
        }],
        explanation: PolicyExplanation::new(
            summary,
            vec![ExplanationReason {
                source_id: provenance.source_id.clone(),
                native_reference: provenance.native_reference.clone(),
                message: format!("Rule `{description}` with effect {effect:?}"),
                fidelity: provenance.fidelity,
            }],
        ),
        warnings: Vec::new(),
    }
}
