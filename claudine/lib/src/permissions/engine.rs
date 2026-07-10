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
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::permissions::backend::{BackendCapabilities, BackendFidelity};
    use crate::permissions::canonical::*;
    use crate::permissions::change::{PolicyChange, PolicyChangeOp};
    use crate::permissions::native::*;
    use crate::permissions::query::CommandQuery;

    /// Stub backend for testing the engine pipeline.
    struct StubBackend {
        target_provider: Provider,
        policy: CanonicalPolicy,
    }

    impl StubBackend {
        fn new(provider: Provider) -> Self {
            let mut policy = CanonicalPolicy::empty(provider, PolicyMode::Configured);

            policy.axes.filesystem.read_rules.push(PathAccessRule {
                pattern: "/workspace".to_owned(),
                effect: PolicyEffect::Allow,
                provenance: CanonicalRuleProvenance::exact("user-config", "permissions.read"),
            });

            policy.axes.filesystem.write_rules.push(PathAccessRule {
                pattern: "/etc".to_owned(),
                effect: PolicyEffect::Deny,
                provenance: CanonicalRuleProvenance::exact("user-config", "permissions.write"),
            });

            policy.axes.commands.shell_rules.push(CommandAccessRule {
                pattern: "rm -rf /".to_owned(),
                effect: PolicyEffect::Deny,
                provenance: CanonicalRuleProvenance::exact("user-config", "commands.deny"),
            });

            policy.axes.network.domain_rules.push(DomainAccessRule {
                pattern: "*.internal.corp".to_owned(),
                effect: PolicyEffect::Allow,
                provenance: CanonicalRuleProvenance::exact("user-config", "network.allow"),
            });

            policy.axes.mcp.server_rules.push(McpServerRule {
                server_id: "filesystem".to_owned(),
                effect: PolicyEffect::Allow,
                provenance: CanonicalRuleProvenance::exact("user-config", "mcp.allow"),
            });

            policy.axes.agents.subagent_rules.push(SubagentRule {
                name: None,
                effect: PolicyEffect::Ask,
                provenance: CanonicalRuleProvenance::exact("user-config", "agents.ask"),
            });

            Self {
                target_provider: provider,
                policy,
            }
        }
    }

    #[async_trait]
    impl ProviderPolicyBackend for StubBackend {
        fn provider(&self) -> Provider {
            self.target_provider
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::full(BackendFidelity::Stub)
        }

        async fn discover_sources(&self, _ctx: &PolicyContext) -> Result<Vec<PolicySource>> {
            Ok(vec![PolicySource {
                id: "user-config".to_owned(),
                kind: PolicySourceKind::UserConfig,
                path: None,
                precedence: 0,
                writable: true,
            }])
        }

        async fn load_native_layers(
            &self,
            _ctx: &PolicyContext,
            sources: &[PolicySource],
        ) -> Result<Vec<NativePolicyLayer>> {
            Ok(sources
                .iter()
                .map(|s| NativePolicyLayer::new(s.clone(), ()))
                .collect())
        }

        fn parse_cli_overrides(
            &self,
            _ctx: &PolicyContext,
            _input: CliPolicyInput<'_>,
        ) -> Result<ProviderCliOverrides> {
            Ok(ProviderCliOverrides::none(self.target_provider))
        }

        fn compose_native_policy(
            &self,
            _ctx: &PolicyContext,
            _layers: &[NativePolicyLayer],
            _cli: Option<&ProviderCliOverrides>,
        ) -> Result<NativeEffectivePolicy> {
            Ok(NativeEffectivePolicy::new(
                self.target_provider,
                vec![PolicySource {
                    id: "user-config".to_owned(),
                    kind: PolicySourceKind::UserConfig,
                    path: None,
                    precedence: 0,
                    writable: true,
                }],
                (),
            ))
        }

        async fn canonicalize(
            &self,
            _ctx: &PolicyContext,
            _native: &NativeEffectivePolicy,
        ) -> Result<CanonicalPolicy> {
            Ok(self.policy.clone())
        }

        async fn plan_change(
            &self,
            _ctx: &PolicyContext,
            _current: &NativeEffectivePolicy,
            _change: &PolicyChange,
        ) -> Result<PolicyMutationPlan> {
            Ok(PolicyMutationPlan::unsupported(
                self.target_provider,
                "Stub backend does not support mutations.",
            ))
        }
    }

    fn test_ctx() -> PolicyContext {
        PolicyContext::new(std::path::PathBuf::from("/workspace"))
    }

    #[tokio::test]
    async fn engine_returns_error_for_missing_backend() {
        let engine = PolicyEngine::empty();
        let ctx = test_ctx();
        let result = engine.configured(Provider::Claude, &ctx).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ClaudineError::PolicyBackendUnavailable(Provider::Claude),
        ));
    }

    #[test]
    fn new_registers_rollout_backends() {
        let engine = PolicyEngine::new();

        assert!(engine.has_backend(Provider::Claude));
        assert!(engine.has_backend(Provider::Codex));
        assert!(engine.has_backend(Provider::Gemini));
        assert!(engine.has_backend(Provider::QwenCode));
        assert!(engine.has_backend(Provider::Goose));
        assert!(engine.has_backend(Provider::KimiCode));
        assert!(engine.has_backend(Provider::OpenCode));
    }

    #[test]
    fn provider_handle_exposes_capabilities() {
        let engine = PolicyEngine::new();

        let qwen = engine.provider(Provider::QwenCode).capabilities().unwrap();
        assert_eq!(qwen.fidelity, BackendFidelity::Medium);

        let goose = engine.provider(Provider::Goose).capabilities().unwrap();
        assert_eq!(goose.fidelity, BackendFidelity::Partial);
    }

    #[tokio::test]
    async fn engine_configured_snapshot_pipeline() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        assert_eq!(snapshot.provider, Provider::Claude);
        assert_eq!(snapshot.canonical.mode, PolicyMode::Configured);
    }

    #[tokio::test]
    async fn engine_effective_snapshot_pipeline() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine
            .effective(Provider::Claude, &ctx, CliPolicyInput::None)
            .await
            .unwrap();

        assert_eq!(snapshot.provider, Provider::Claude);
    }

    #[tokio::test]
    async fn snapshot_path_read_query() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        let result = snapshot.can_read("/workspace/src/main.rs");
        assert!(result.is_allowed());
        assert_eq!(result.certainty, PolicyCertainty::Exact);
        assert_eq!(result.matched_rules.len(), 1);
    }

    #[tokio::test]
    async fn snapshot_path_write_deny() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        let result = snapshot.can_write("/etc/hosts");
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn snapshot_command_deny() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        let cmd = CommandQuery::from_raw("rm -rf /");
        let result = snapshot.can_execute(&cmd);
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn snapshot_domain_query() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        let result = snapshot.can_access_domain("api.internal.corp");
        assert!(result.is_allowed());

        let result = snapshot.can_access_domain("evil.com");
        assert!(result.is_unknown());
    }

    #[tokio::test]
    async fn snapshot_mcp_server_query() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        let result = snapshot.can_use_mcp_server("filesystem");
        assert!(result.is_allowed());

        let result = snapshot.can_use_mcp_server("unknown-server");
        assert!(result.is_unknown());
    }

    #[tokio::test]
    async fn snapshot_subagent_query() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        // Wildcard rule matches any subagent with Ask effect
        let result = snapshot.can_spawn_subagent(Some("researcher"));
        assert!(result.is_ask());
    }

    #[tokio::test]
    async fn snapshot_no_match_returns_unknown() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        let result = snapshot.can_read("/unknown/path");
        assert!(result.is_unknown());
    }

    #[tokio::test]
    async fn provider_handle_convenience() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let handle = engine.provider(Provider::Claude);

        let snapshot = handle.configured(&ctx).await.unwrap();
        assert_eq!(snapshot.provider, Provider::Claude);
    }

    #[tokio::test]
    async fn provider_handle_effective() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let handle = engine.provider(Provider::Claude);

        let snapshot = handle.effective(&ctx, CliPolicyInput::None).await.unwrap();
        assert_eq!(snapshot.provider, Provider::Claude);
    }

    #[tokio::test]
    async fn mutation_plan_unsupported_for_stub() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let handle = engine.provider(Provider::Claude);

        let change = PolicyChange::persistent(vec![PolicyChangeOp::GrantRead(
            std::path::PathBuf::from("/foo"),
        )]);
        let plan = handle.plan_change(&ctx, &change).await.unwrap();
        assert!(!plan.supported);
        assert!(!plan.warnings.is_empty());
    }

    #[test]
    fn registered_providers_list() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));
        engine.register(Box::new(StubBackend::new(Provider::Codex)));

        let mut providers = engine.registered_providers();
        providers.sort();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&Provider::Claude));
        assert!(providers.contains(&Provider::Codex));
    }

    #[test]
    fn has_backend_check() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        assert!(engine.has_backend(Provider::Claude));
        assert!(!engine.has_backend(Provider::Codex));
    }

    #[tokio::test]
    async fn effective_snapshot_queries_work() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Gemini)));

        let ctx = test_ctx();
        let snapshot = engine
            .effective(Provider::Gemini, &ctx, CliPolicyInput::None)
            .await
            .unwrap();

        let result = snapshot.can_read("/workspace/file.txt");
        assert!(result.is_allowed());

        let result = snapshot.can_write("/etc/passwd");
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn query_result_has_explanation() {
        let mut engine = PolicyEngine::empty();
        engine.register(Box::new(StubBackend::new(Provider::Claude)));

        let ctx = test_ctx();
        let snapshot = engine.configured(Provider::Claude, &ctx).await.unwrap();

        let result = snapshot.can_read("/workspace/src/lib.rs");
        assert!(!result.explanation.summary.is_empty());
        assert!(!result.explanation.reasons.is_empty());
        assert_eq!(result.explanation.reasons[0].source_id, "user-config",);
    }
}
