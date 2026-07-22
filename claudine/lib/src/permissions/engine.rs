use std::collections::HashMap;

use tracing::info_span;

use super::backend::{BackendCapabilities, ProviderPolicyBackend};
use super::change::PolicyChange;
use super::context::{CliPolicyInput, PolicyContext};
use super::mutation::PolicyMutationPlan;
use super::providers::{
    ClaudePolicyBackend, CodexPolicyBackend, GeminiPolicyBackend, GoosePolicyBackend,
    KimiPolicyBackend, OpenCodePolicyBackend, QwenPolicyBackend,
};
use super::query::{ConfiguredPolicySnapshot, EffectivePolicySnapshot};
use crate::error::{ClaudineError, Result};
use crate::provider::Provider;

/// Cross-provider permission policy engine.
///
/// `PolicyEngine` is Claudine's source of truth for provider permission state.
/// It delegates provider-specific work to registered backends and provides a
/// uniform query and mutation surface.
///
/// ## Examples
///
/// ```ignore
/// let mut engine = PolicyEngine::new();
/// engine.register(Box::new(my_claude_backend));
///
/// let ctx = PolicyContext::new(std::env::current_dir()?);
/// let snapshot = engine.configured(Provider::Claude, &ctx).await?;
///
/// let result = snapshot.can_read("/workspace/src/main.rs");
/// assert!(result.is_allowed());
/// ```
pub struct PolicyEngine {
    backends: HashMap<Provider, Box<dyn ProviderPolicyBackend>>,
}

impl PolicyEngine {
    /// Creates a new engine with the built-in high-value provider backends.
    pub fn new() -> Self {
        let mut engine = Self::empty();
        engine.register(Box::new(ClaudePolicyBackend));
        engine.register(Box::new(CodexPolicyBackend));
        engine.register(Box::new(GeminiPolicyBackend));
        engine.register(Box::new(QwenPolicyBackend));
        engine.register(Box::new(GoosePolicyBackend));
        engine.register(Box::new(KimiPolicyBackend));
        engine.register(Box::new(OpenCodePolicyBackend));
        engine
    }

    /// Creates a new engine with no backends registered.
    pub fn empty() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /// Registers a provider backend.
    ///
    /// Replaces any existing backend for the same provider.
    pub fn register(&mut self, backend: Box<dyn ProviderPolicyBackend>) {
        let provider = backend.provider();
        self.backends.insert(provider, backend);
    }

    /// Returns a handle scoped to a specific provider.
    pub fn provider(&self, provider: Provider) -> ProviderPolicyHandle<'_> {
        ProviderPolicyHandle {
            engine: self,
            provider,
        }
    }

    pub fn has_backend(&self, provider: Provider) -> bool {
        self.backends.contains_key(&provider)
    }

    pub fn registered_providers(&self) -> Vec<Provider> {
        self.backends.keys().copied().collect()
    }

    /// Returns capability metadata for the given provider backend.
    pub fn capabilities(&self, provider: Provider) -> Result<BackendCapabilities> {
        Ok(self.backend(provider)?.capabilities())
    }

    /// Produces a configured policy snapshot (filesystem-only).
    ///
    /// ## Pipeline
    ///
    /// 1. Discover sources
    /// 2. Load native layers
    /// 3. Compose without CLI overrides
    /// 4. Canonicalize
    /// 5. Return snapshot
    pub async fn configured(
        &self,
        provider: Provider,
        ctx: &PolicyContext,
    ) -> Result<ConfiguredPolicySnapshot> {
        let _span = info_span!("permissions_configured", %provider).entered();
        let backend = self.backend(provider)?;

        let sources = backend.discover_sources(ctx).await?;
        let layers = backend.load_native_layers(ctx, &sources).await?;
        let native = backend.compose_native_policy(ctx, &layers, None)?;
        let canonical = backend.canonicalize(ctx, &native).await?;

        Ok(ConfiguredPolicySnapshot::from_parts(
            provider, native, canonical, ctx,
        ))
    }

    /// Produces an effective policy snapshot (config + CLI overrides).
    ///
    /// ## Pipeline
    ///
    /// 1. Discover sources
    /// 2. Load native layers
    /// 3. Parse CLI overrides
    /// 4. Compose with CLI overrides
    /// 5. Canonicalize
    /// 6. Return snapshot
    pub async fn effective(
        &self,
        provider: Provider,
        ctx: &PolicyContext,
        cli: CliPolicyInput<'_>,
    ) -> Result<EffectivePolicySnapshot> {
        let _span = info_span!("permissions_effective", %provider).entered();
        let backend = self.backend(provider)?;

        let sources = backend.discover_sources(ctx).await?;
        let layers = backend.load_native_layers(ctx, &sources).await?;
        let cli_overrides = backend.parse_cli_overrides(ctx, cli)?;
        let native = backend.compose_native_policy(ctx, &layers, Some(&cli_overrides))?;
        let canonical = backend.canonicalize(ctx, &native).await?;

        Ok(EffectivePolicySnapshot::from_parts(
            provider,
            native,
            canonical,
            cli_overrides,
            ctx,
        ))
    }

    fn backend(&self, provider: Provider) -> Result<&dyn ProviderPolicyBackend> {
        self.backends
            .get(&provider)
            .map(|b| b.as_ref())
            .ok_or(ClaudineError::PolicyBackendUnavailable(provider))
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PolicyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyEngine")
            .field("providers", &self.backends.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Handle for provider-scoped engine operations.
///
/// Created by [`PolicyEngine::provider`]. Provides convenience methods
/// that avoid repeating the provider argument.
pub struct ProviderPolicyHandle<'a> {
    engine: &'a PolicyEngine,
    provider: Provider,
}

impl ProviderPolicyHandle<'_> {
    /// Produces a configured policy snapshot.
    pub async fn configured(&self, ctx: &PolicyContext) -> Result<ConfiguredPolicySnapshot> {
        self.engine.configured(self.provider, ctx).await
    }

    /// Produces an effective policy snapshot.
    pub async fn effective(
        &self,
        ctx: &PolicyContext,
        cli: CliPolicyInput<'_>,
    ) -> Result<EffectivePolicySnapshot> {
        self.engine.effective(self.provider, ctx, cli).await
    }

    /// Plans a policy change against the current native policy.
    pub async fn plan_change(
        &self,
        ctx: &PolicyContext,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan> {
        let backend = self.engine.backend(self.provider)?;
        let sources = backend.discover_sources(ctx).await?;
        let layers = backend.load_native_layers(ctx, &sources).await?;
        let native = backend.compose_native_policy(ctx, &layers, None)?;
        backend.plan_change(ctx, &native, change).await
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// Returns capability metadata for this provider backend.
    pub fn capabilities(&self) -> Result<BackendCapabilities> {
        self.engine.capabilities(self.provider)
    }
}

#[cfg(test)]
mod tests;
