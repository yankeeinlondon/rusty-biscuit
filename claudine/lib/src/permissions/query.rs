use std::path::{Component, Path, PathBuf};

use biscuit_file::to_portable_string;

use super::canonical::{
    CanonicalPolicy, MappingFidelity, PolicyCertainty, PolicyEffect, PolicyWarning, TernaryState,
};
use super::context::PolicyContext;
use super::explain::{ExplanationReason, PolicyExplanation};
use super::matchers;
use super::matchers::PathClassification;
use super::native::{NativeEffectivePolicy, ProviderCliOverrides};
use crate::provider::{Provider, provider_info};

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
    /// Performs shell-like tokenization for quotes and escapes, then extracts
    /// the executable from the first non-assignment token.
    pub fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let argv = tokenize_command_words(&raw);
        let executable = argv
            .iter()
            .find(|token| !looks_like_env_assignment(token))
            .map(|token| {
                std::path::Path::new(token)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(token)
                    .to_owned()
            });
        Self {
            raw,
            executable,
            argv,
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

    pub fn is_unknown(&self) -> bool {
        self.effect.is_none()
    }

    /// Creates an unknown result with a reason.
    pub fn unknown() -> Self {
        Self::no_match()
    }

    /// Creates a synthetic `Deny` result with exact certainty.
    pub fn denied(reason: &str) -> Self {
        Self {
            effect: Some(PolicyEffect::Deny),
            certainty: PolicyCertainty::Exact,
            stability: QueryStability::Stable,
            matched_rules: Vec::new(),
            explanation: PolicyExplanation::no_match(reason),
            warnings: Vec::new(),
        }
    }

    /// Creates a synthetic `Allow` result with exact certainty.
    pub fn allowed(reason: &str) -> Self {
        Self {
            effect: Some(PolicyEffect::Allow),
            certainty: PolicyCertainty::Exact,
            stability: QueryStability::Stable,
            matched_rules: Vec::new(),
            explanation: PolicyExplanation::no_match(reason),
            warnings: Vec::new(),
        }
    }

    /// Creates a synthetic `Ask` result with exact certainty.
    pub fn synthetic_ask(reason: &str) -> Self {
        Self {
            effect: Some(PolicyEffect::Ask),
            certainty: PolicyCertainty::Exact,
            stability: QueryStability::Stable,
            matched_rules: Vec::new(),
            explanation: PolicyExplanation::no_match(reason),
            warnings: Vec::new(),
        }
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
    query_context: SnapshotQueryContext,
}

impl ConfiguredPolicySnapshot {
    pub(crate) fn from_parts(
        provider: Provider,
        native: NativeEffectivePolicy,
        canonical: CanonicalPolicy,
        ctx: &PolicyContext,
    ) -> Self {
        let query_context = SnapshotQueryContext::from_parts(ctx, &native);
        Self {
            provider,
            native,
            canonical,
            query_context,
        }
    }

    /// Evaluates a policy query against this configured snapshot.
    pub fn query(&self, query: &PolicyQuery) -> QueryResult {
        resolve_query(&self.canonical, query, &self.query_context)
    }

    /// Can the given path be read?
    pub fn can_read(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::ReadPath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given path be written?
    pub fn can_write(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::WritePath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given path be traversed?
    pub fn can_traverse(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::TraversePath(PathQuery::directory(
            path.as_ref(),
        )))
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

    /// Can the provider switch into the given mode?
    pub fn can_switch_mode(&self, target: Option<&str>) -> QueryResult {
        self.query(&PolicyQuery::SwitchMode {
            target: target.map(String::from),
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
    query_context: SnapshotQueryContext,
}

impl EffectivePolicySnapshot {
    pub(crate) fn from_parts(
        provider: Provider,
        native: NativeEffectivePolicy,
        canonical: CanonicalPolicy,
        cli: ProviderCliOverrides,
        ctx: &PolicyContext,
    ) -> Self {
        let query_context = SnapshotQueryContext::from_parts(ctx, &native);
        Self {
            provider,
            native,
            canonical,
            cli,
            query_context,
        }
    }

    /// Evaluates a policy query against this effective snapshot.
    pub fn query(&self, query: &PolicyQuery) -> QueryResult {
        resolve_query(&self.canonical, query, &self.query_context)
    }

    /// Can the given path be read?
    pub fn can_read(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::ReadPath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given path be written?
    pub fn can_write(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::WritePath(PathQuery::unknown(path.as_ref())))
    }

    /// Can the given path be traversed?
    pub fn can_traverse(&self, path: impl AsRef<Path>) -> QueryResult {
        self.query(&PolicyQuery::TraversePath(PathQuery::directory(
            path.as_ref(),
        )))
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

    /// Can the provider switch into the given mode?
    pub fn can_switch_mode(&self, target: Option<&str>) -> QueryResult {
        self.query(&PolicyQuery::SwitchMode {
            target: target.map(String::from),
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
fn resolve_query(
    policy: &CanonicalPolicy,
    query: &PolicyQuery,
    query_context: &SnapshotQueryContext,
) -> QueryResult {
    let mut result = match query {
        PolicyQuery::ReadPath(pq) => resolve_path_query(
            policy,
            &policy.axes.filesystem.read_rules,
            pq,
            query_context,
        ),
        PolicyQuery::WritePath(pq) => resolve_path_query(
            policy,
            &policy.axes.filesystem.write_rules,
            pq,
            query_context,
        ),
        PolicyQuery::TraversePath(pq) => resolve_path_query(
            policy,
            &policy.axes.filesystem.traversal_rules,
            pq,
            query_context,
        ),
        PolicyQuery::ExecuteCommand(cq) => {
            resolve_command_query(policy, &policy.axes.commands.shell_rules, cq)
        }
        PolicyQuery::AccessDomain(dq) => resolve_domain_query(policy, &policy.axes.network, dq),
        PolicyQuery::UseMcpServer { server } => {
            resolve_mcp_server_query(policy, &policy.axes.mcp.server_rules, server)
        }
        PolicyQuery::UseMcpTool { server, tool } => {
            resolve_mcp_tool_query(policy, &policy.axes.mcp.tool_rules, server, tool)
        }
        PolicyQuery::SpawnSubagent { name } => {
            resolve_subagent_query(policy, &policy.axes.agents.subagent_rules, name.as_deref())
        }
        PolicyQuery::SwitchMode { target } => resolve_mode_switch_query(
            policy,
            &policy.axes.agents.mode_switch_rules,
            target.as_deref(),
        ),
        PolicyQuery::ModifyProviderConfig => {
            resolve_config_modify_query(policy, &policy.axes.filesystem.protected_config_paths)
        }
    };

    // Apply CLI sensitivity only for configured snapshots and only for axes
    // where the provider actually has CLI overrides that can change the answer.
    if result.stability == QueryStability::Stable
        && policy.mode == super::canonical::PolicyMode::Configured
        && is_cli_sensitive(policy.provider, query)
    {
        result.stability = QueryStability::MayChangeWithCli;
    }

    result
}

fn resolve_path_query(
    policy: &CanonicalPolicy,
    rules: &[super::canonical::PathAccessRule],
    pq: &PathQuery,
    query_context: &SnapshotQueryContext,
) -> QueryResult {
    let resolved = ResolvedPathQuery::from_query(pq, query_context);

    for (i, rule) in rules.iter().enumerate() {
        if matchers::path_matches(&resolved.normalized_path, &rule.pattern) {
            let mut result = build_single_match_result(
                policy,
                i,
                &format!("path rule: {}", rule.pattern),
                rule.effect,
                &rule.provenance,
                &format!(
                    "Path {} matched rule `{}` -> {:?}",
                    resolved.subject(),
                    rule.pattern,
                    rule.effect,
                ),
            );
            result
                .explanation
                .reasons
                .push(resolved.scope_reason("Query path resolved within"));
            return result;
        }
    }

    no_match_result(
        policy,
        format!("No matching rules found for path {}.", resolved.subject()),
        vec![resolved.scope_reason("Query path resolved within")],
    )
}

fn resolve_command_query(
    policy: &CanonicalPolicy,
    rules: &[super::canonical::CommandAccessRule],
    cq: &CommandQuery,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        if matchers::command_matches(&cq.raw, cq.executable.as_deref(), &rule.pattern) {
            return build_single_match_result(
                policy,
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
    no_match_result(
        policy,
        format!("No matching rules found for command `{}`.", cq.raw),
        Vec::new(),
    )
}

fn resolve_domain_query(
    policy: &CanonicalPolicy,
    network: &super::canonical::NetworkPolicy,
    dq: &DomainQuery,
) -> QueryResult {
    if network.enabled == TernaryState::No {
        return finalize_result(
            policy,
            QueryResult {
                effect: Some(PolicyEffect::Deny),
                certainty: PolicyCertainty::Exact,
                stability: QueryStability::Stable,
                matched_rules: Vec::new(),
                explanation: PolicyExplanation::new("Network access is disabled.", Vec::new()),
                warnings: Vec::new(),
            },
        );
    }

    for (i, rule) in network.domain_rules.iter().enumerate() {
        if matchers::domain_matches(&dq.domain, &rule.pattern) {
            return build_single_match_result(
                policy,
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

    match network.enabled {
        TernaryState::Yes => finalize_result(
            policy,
            QueryResult {
                effect: Some(PolicyEffect::Allow),
                certainty: PolicyCertainty::BestEffort,
                stability: QueryStability::Stable,
                matched_rules: Vec::new(),
                explanation: PolicyExplanation::new(
                    format!(
                        "Network access is enabled and no domain-specific rule matched `{}`.",
                        dq.domain
                    ),
                    Vec::new(),
                ),
                warnings: Vec::new(),
            },
        ),
        TernaryState::Unknown => no_match_result(
            policy,
            format!(
                "No matching rules found for domain `{}` and network enablement is unknown.",
                dq.domain
            ),
            Vec::new(),
        ),
        TernaryState::No => unreachable!(),
    }
}

fn resolve_mcp_server_query(
    policy: &CanonicalPolicy,
    rules: &[super::canonical::McpServerRule],
    server: &str,
) -> QueryResult {
    for (i, rule) in rules.iter().enumerate() {
        if rule.server_id == server || rule.server_id == "*" {
            return build_single_match_result(
                policy,
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
    no_match_result(
        policy,
        format!("No matching rules found for MCP server `{server}`."),
        Vec::new(),
    )
}

fn resolve_mcp_tool_query(
    policy: &CanonicalPolicy,
    rules: &[super::canonical::McpToolRule],
    server: &str,
    tool: &str,
) -> QueryResult {
    // Check server-level rules first: a server-level decision applies to all
    // tools on that server unless a more specific tool rule overrides it.
    let server_result = resolve_mcp_server_query(policy, &policy.axes.mcp.server_rules, server);

    // Server deny always wins — short-circuit before checking tool rules.
    if server_result.effect == Some(PolicyEffect::Deny) {
        return server_result;
    }

    for (i, rule) in rules.iter().enumerate() {
        let server_match = rule.server_id == server || rule.server_id == "*";
        let tool_match = rule.tool_name == tool || rule.tool_name == "*";
        if server_match && tool_match {
            return build_single_match_result(
                policy,
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

    // No tool-specific rule matched. If the server has an Allow or Ask rule,
    // inherit that as the fallback answer for the tool query.
    if server_result.effect.is_some() {
        return server_result;
    }

    no_match_result(
        policy,
        format!("No matching rules found for MCP tool `{server}::{tool}`."),
        Vec::new(),
    )
}

fn resolve_subagent_query(
    policy: &CanonicalPolicy,
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
                policy,
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
    no_match_result(
        policy,
        format!(
            "No matching rules found for subagent `{}`.",
            name.unwrap_or("(any)")
        ),
        Vec::new(),
    )
}

fn resolve_mode_switch_query(
    policy: &CanonicalPolicy,
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
                policy,
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
    no_match_result(
        policy,
        format!(
            "No matching rules found for mode switch `{}`.",
            target.unwrap_or("(any)")
        ),
        Vec::new(),
    )
}

fn resolve_config_modify_query(
    policy: &CanonicalPolicy,
    protected_paths: &[super::canonical::PathProtectionRule],
) -> QueryResult {
    if protected_paths.is_empty() {
        return no_match_result(
            policy,
            "No protected provider config paths are registered.".to_owned(),
            Vec::new(),
        );
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

    finalize_result(
        policy,
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
        },
    )
}

/// Builds a `QueryResult` for a single matched rule.
fn build_single_match_result(
    policy: &CanonicalPolicy,
    index: usize,
    description: &str,
    effect: PolicyEffect,
    provenance: &super::canonical::CanonicalRuleProvenance,
    summary: &str,
) -> QueryResult {
    finalize_result(
        policy,
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
        },
    )
}

#[derive(Debug, Clone)]
struct ResolvedPathQuery {
    original_path: PathBuf,
    normalized_path: PathBuf,
    classification: PathClassification,
}

impl ResolvedPathQuery {
    fn from_query(query: &PathQuery, query_context: &SnapshotQueryContext) -> Self {
        let candidate = if query.path.is_absolute() {
            query.path.clone()
        } else {
            query_context.cwd.join(&query.path)
        };
        let normalized_path = canonicalize_lossy(&candidate);
        let provider_roots = query_context
            .provider_config_roots
            .iter()
            .map(PathBuf::as_path)
            .collect::<Vec<_>>();
        let classification = matchers::classify_path_with_config(
            &normalized_path,
            Some(query_context.workspace_root.as_path()),
            query_context.home_dir.as_deref(),
            &provider_roots,
        );

        Self {
            original_path: query.path.clone(),
            normalized_path,
            classification,
        }
    }

    fn subject(&self) -> String {
        let normalized_path = to_portable_string(&self.normalized_path);
        if self.original_path == self.normalized_path {
            format!(
                "`{}` ({})",
                normalized_path,
                path_classification_label(self.classification),
            )
        } else {
            format!(
                "`{}` -> `{}` ({})",
                to_portable_string(&self.original_path),
                normalized_path,
                path_classification_label(self.classification),
            )
        }
    }

    fn scope_reason(&self, prefix: &str) -> ExplanationReason {
        ExplanationReason {
            source_id: "query".to_owned(),
            native_reference: None,
            message: format!(
                "{prefix} {} scope at `{}`.",
                path_classification_label(self.classification),
                to_portable_string(&self.normalized_path),
            ),
            fidelity: MappingFidelity::Exact,
        }
    }
}

#[derive(Debug, Clone)]
struct SnapshotQueryContext {
    cwd: PathBuf,
    workspace_root: PathBuf,
    home_dir: Option<PathBuf>,
    provider_config_roots: Vec<PathBuf>,
}

impl SnapshotQueryContext {
    fn from_parts(ctx: &PolicyContext, native: &NativeEffectivePolicy) -> Self {
        let mut provider_config_roots = Vec::new();
        for source in &native.sources {
            if let Some(path) = &source.path {
                provider_config_roots.push(canonicalize_lossy(path));
                if let Some(parent) = path.parent() {
                    provider_config_roots.push(canonicalize_lossy(parent));
                }
            }
        }
        provider_config_roots.sort();
        provider_config_roots.dedup();

        Self {
            cwd: canonicalize_lossy(&ctx.cwd),
            workspace_root: canonicalize_lossy(ctx.repo_root.as_deref().unwrap_or(&ctx.cwd)),
            home_dir: ctx.home_dir.as_deref().map(canonicalize_lossy),
            provider_config_roots,
        }
    }
}

fn no_match_result(
    policy: &CanonicalPolicy,
    summary: String,
    reasons: Vec<ExplanationReason>,
) -> QueryResult {
    finalize_result(
        policy,
        QueryResult {
            effect: None,
            certainty: PolicyCertainty::Unknown,
            stability: QueryStability::Unknown,
            matched_rules: Vec::new(),
            explanation: PolicyExplanation::new(summary, reasons),
            warnings: Vec::new(),
        },
    )
}

fn finalize_result(policy: &CanonicalPolicy, mut result: QueryResult) -> QueryResult {
    for warning in &policy.warnings {
        if !result
            .warnings
            .iter()
            .any(|existing| existing.code == warning.code && existing.message == warning.message)
        {
            result.warnings.push(warning.clone());
        }
    }

    if let Some(warning) = policy
        .warnings
        .iter()
        .find(|warning| warning.code.ends_with(".trust_unknown"))
    {
        result.effect = None;
        result.certainty = PolicyCertainty::Unknown;
        result.stability = QueryStability::Unknown;
        result.explanation.reasons.push(ExplanationReason {
            source_id: warning
                .source_id
                .clone()
                .unwrap_or_else(|| "policy".to_owned()),
            native_reference: None,
            message: warning.message.clone(),
            fidelity: MappingFidelity::Approximate,
        });
        result.explanation.summary = format!(
            "{} Trust is unknown, so repo-scoped policy may change this answer.",
            result.explanation.summary
        );
    }

    result
}

/// Returns whether a configured result should be marked as `MayChangeWithCli`
/// for the given provider and query type.
///
/// Each provider's CLI overrides only affect certain policy axes:
/// - **Claude**: `--settings`, `--allowedTools`, `--disallowedTools`, `--dangerously-skip-permissions`
///   can affect all axes.
/// - **Codex**: `--sandbox-mode`, `--approval-policy`, `--full-auto`, `--writable-roots`
///   affect filesystem and runtime axes.
/// - **Gemini**: `--approval-mode`, `--sandbox`, `--allowed-tools` affect runtime and commands.
/// - **OpenCode**: `--full-auto` and config content override affect runtime.
/// - **Qwen**: `--allowed-mcp-server-names`, `--allowed-tools`, `--excluded-tools`,
///   `--sandbox` affect MCP, commands, and runtime.
fn is_cli_sensitive(provider: Provider, query: &PolicyQuery) -> bool {
    let axes = provider_info(provider).cli_sensitive_axes;
    match query {
        PolicyQuery::ReadPath(_) => axes.read_path,
        PolicyQuery::WritePath(_) => axes.write_path,
        PolicyQuery::TraversePath(_) => axes.traverse_path,
        PolicyQuery::ExecuteCommand(_) => axes.execute_command,
        PolicyQuery::AccessDomain(_) => axes.access_domain,
        PolicyQuery::UseMcpServer { .. } => axes.use_mcp_server,
        PolicyQuery::UseMcpTool { .. } => axes.use_mcp_tool,
        PolicyQuery::SpawnSubagent { .. } => axes.spawn_subagent,
        PolicyQuery::SwitchMode { .. } => axes.switch_mode,
        PolicyQuery::ModifyProviderConfig => axes.modify_provider_config,
    }
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    normalize_path(path)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
            Component::Normal(segment) => normalized.push(segment),
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

fn path_classification_label(classification: PathClassification) -> &'static str {
    match classification {
        PathClassification::ProviderConfig => "provider-config",
        PathClassification::Workspace => "workspace",
        PathClassification::Home => "home",
        PathClassification::Temp => "temp",
        PathClassification::System => "system",
        PathClassification::External => "external",
    }
}

fn tokenize_command_words(raw: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = raw.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if !in_single => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

fn looks_like_env_assignment(token: &str) -> bool {
    let Some((name, value)) = token.split_once('=') else {
        return false;
    };

    !name.is_empty()
        && !value.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests;
