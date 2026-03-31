// Provider-agnostic permission policy engine.
//
// `PolicyEngine` is Claudine's canonical source of truth for provider
// permission state. It loads provider-native config, composes it with
// CLI/runtime overrides, normalizes the result into a canonical
// cross-provider model, answers structured permission queries with
// explanation and provenance, and plans permission mutations.
//
// The engine is independent from `ProtectService`. Protect remains a
// runtime decision layer that can consume `PolicyEngine` later.

pub mod backend;
pub mod canonical;
pub mod change;
pub mod context;
pub mod engine;
pub mod explain;
pub mod matchers;
pub mod mutation;
pub mod native;
pub mod providers;
pub mod query;

// --- Public re-exports ---

// Engine
pub use engine::{PolicyEngine, ProviderPolicyHandle};

// Backend trait
pub use backend::{BackendCapabilities, BackendFidelity, ProviderPolicyBackend};

// Context
pub use context::{CliPolicyInput, PolicyContext, ProjectTrustContext, TrustSource};

// Canonical model
pub use canonical::{
    AgentPolicy, CanonicalApprovalMode, CanonicalPolicy, CanonicalPolicyAxes,
    CanonicalRuleProvenance, CanonicalSandboxMode, CommandAccessRule, CommandPolicy,
    DomainAccessRule, ExecutionModeRule, FilesystemPolicy, MappingFidelity, McpAccessPolicy,
    McpServerRule, McpToolRule, ModeSwitchRule, NetworkPolicy, PathAccessRule, PathProtectionRule,
    PolicyCertainty, PolicyEffect, PolicyMode, PolicyWarning, RuntimePolicy, SubagentRule,
    TernaryState,
};

// Native types
pub use native::{
    NativeEffectivePolicy, NativePolicyLayer, PolicySource, PolicySourceKind, ProviderCliOverrides,
};

// Query types
pub use query::{
    CommandQuery, ConfiguredPolicySnapshot, DomainQuery, EffectivePolicySnapshot, MatchedRule,
    PathKindHint, PathQuery, PolicyQuery, QueryResult, QueryStability,
};

// Explanation
pub use explain::{ExplanationReason, PolicyExplanation};

// Change types
pub use change::{
    CommandPattern, PolicyChange, PolicyChangeOp, PolicyChangeTarget, PolicyPersistence,
};

// Mutation types
pub use mutation::{
    AppliedMutationReport, ConfigEditPlan, OneShotMutationPlan, PersistentMutationPlan,
    PolicyMutationPlan,
};
