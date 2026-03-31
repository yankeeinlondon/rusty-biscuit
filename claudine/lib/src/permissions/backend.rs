use crate::error::Result;
use crate::events::Provider;

use super::canonical::CanonicalPolicy;
use super::change::PolicyChange;
use super::context::{CliPolicyInput, PolicyContext};
use super::mutation::PolicyMutationPlan;
use super::native::{NativeEffectivePolicy, NativePolicyLayer, PolicySource, ProviderCliOverrides};

/// Trait implemented by provider-specific policy backends.
///
/// Each backend is responsible for:
/// 1. discovering config sources on disk
/// 2. loading typed native policy layers
/// 3. parsing CLI overrides
/// 4. composing layers into a native effective policy
/// 5. canonicalizing native policy into the shared model
/// 6. planning mutations
pub trait ProviderPolicyBackend: Send + Sync {
    /// Returns the provider this backend handles.
    fn provider(&self) -> Provider;

    /// Returns capability metadata for this backend.
    fn capabilities(&self) -> BackendCapabilities;

    /// Discovers relevant config sources on disk.
    fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>>;

    /// Loads typed native policy layers from discovered sources.
    fn load_native_layers(
        &self,
        ctx: &PolicyContext,
        sources: &[PolicySource],
    ) -> Result<Vec<NativePolicyLayer>>;

    /// Parses CLI overrides into typed provider-specific overrides.
    fn parse_cli_overrides(
        &self,
        ctx: &PolicyContext,
        input: CliPolicyInput<'_>,
    ) -> Result<ProviderCliOverrides>;

    /// Composes native layers into a single effective native policy.
    fn compose_native_policy(
        &self,
        ctx: &PolicyContext,
        layers: &[NativePolicyLayer],
        cli: Option<&ProviderCliOverrides>,
    ) -> Result<NativeEffectivePolicy>;

    /// Canonicalizes a native effective policy into the shared model.
    fn canonicalize(
        &self,
        ctx: &PolicyContext,
        native: &NativeEffectivePolicy,
    ) -> Result<CanonicalPolicy>;

    /// Plans a mutation against the current native policy.
    fn plan_change(
        &self,
        ctx: &PolicyContext,
        current: &NativeEffectivePolicy,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan>;
}

/// Capability metadata for a provider backend.
///
/// Allows callers and tests to inspect what a backend supports without
/// attempting operations that will fail.
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    /// Overall fidelity level of this backend.
    pub fidelity: BackendFidelity,
    /// Whether filesystem path queries are supported.
    pub filesystem_queries: bool,
    /// Whether command execution queries are supported.
    pub command_queries: bool,
    /// Whether network/domain queries are supported.
    pub network_queries: bool,
    /// Whether MCP server/tool queries are supported.
    pub mcp_queries: bool,
    /// Whether subagent queries are supported.
    pub agent_queries: bool,
    /// Whether persistent mutation planning is supported.
    pub persistent_mutations: bool,
    /// Whether one-shot CLI mutation planning is supported.
    pub one_shot_mutations: bool,
}

/// Overall fidelity level of a backend's policy modeling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendFidelity {
    /// High-fidelity backend with comprehensive policy modeling.
    High,
    /// Medium-fidelity backend with most policy axes covered.
    Medium,
    /// Partial backend with limited coverage.
    Partial,
    /// Stub backend with minimal or no real policy support.
    Stub,
}

impl BackendCapabilities {
    /// Creates capabilities for a fully-featured backend.
    pub fn full(fidelity: BackendFidelity) -> Self {
        Self {
            fidelity,
            filesystem_queries: true,
            command_queries: true,
            network_queries: true,
            mcp_queries: true,
            agent_queries: true,
            persistent_mutations: true,
            one_shot_mutations: true,
        }
    }

    /// Creates capabilities for a stub backend with no real support.
    pub fn stub() -> Self {
        Self {
            fidelity: BackendFidelity::Stub,
            filesystem_queries: false,
            command_queries: false,
            network_queries: false,
            mcp_queries: false,
            agent_queries: false,
            persistent_mutations: false,
            one_shot_mutations: false,
        }
    }
}
