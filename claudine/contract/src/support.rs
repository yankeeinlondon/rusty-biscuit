//! Provider gating and the v1 tool-free support matrix.
//!
//! A provider is enabled only when it has a non-interactive entrypoint, a
//! structured stream protocol Claudine can parse (so the final assistant text
//! can be captured deterministically), and has been curated into the v1
//! allowlist as verified-runnable tool-free for untrusted input. Everything
//! else is reported as `Unsupported` rather than run unsafely.

use claudine::provider::{EntrypointMode, EntrypointSpec, ProviderInfo, all_providers, provider_info};
use claudine::provider_id::Provider;

/// Providers enabled for tool-free inference in v1.
///
/// Limited to the providers whose non-interactive entrypoint, output format,
/// system-prompt channel, and tool-free behavior are documented in the crate
/// README's support matrix. Widening this set is a deliberate, reviewed change
/// — not a default — because each entry asserts a safety guarantee over
/// untrusted input.
const V1_ENABLED: &[Provider] = &[Provider::Claude, Provider::Codex];

/// Whether a provider can back tool-free inference in v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSupport {
    /// The provider is enabled for tool-free, isolated inference.
    Enabled,
    /// The provider is rejected; the string is a short, stable reason.
    Rejected(&'static str),
}

/// Classify a provider for tool-free inference.
pub fn provider_support(provider: Provider) -> ProviderSupport {
    let info = provider_info(provider);
    if non_interactive_entrypoint(info).is_none() {
        return ProviderSupport::Rejected("no non-interactive entrypoint");
    }
    if info.stream_protocol.is_none() {
        return ProviderSupport::Rejected("no structured stream protocol for tool-free capture");
    }
    if !V1_ENABLED.contains(&provider) {
        return ProviderSupport::Rejected("not yet verified tool-free for untrusted input in v1");
    }
    ProviderSupport::Enabled
}

/// The first non-interactive (or dual-mode) entrypoint for a provider.
pub(crate) fn non_interactive_entrypoint(info: &'static ProviderInfo) -> Option<&'static EntrypointSpec> {
    info.entrypoints
        .iter()
        .find(|spec| matches!(spec.mode, EntrypointMode::NonInteractive | EntrypointMode::Both))
}

/// Authentication environment variables forwarded for a provider.
///
/// Most agentic CLIs authenticate via files under `HOME`; these named keys are
/// the secondary, env-based path. They are forwarded only when present in the
/// caller's environment and are never echoed into diagnostics.
pub(crate) fn auth_env_vars(provider: Provider) -> &'static [&'static str] {
    match provider {
        Provider::Claude => &["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN"],
        Provider::Codex => &["OPENAI_API_KEY", "OPENAI_BASE_URL"],
        Provider::Gemini => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        Provider::Goose => &[],
        Provider::KimiCode => &["MOONSHOT_API_KEY", "KIMI_API_KEY"],
        Provider::OpenCode => &["OPENCODE_API_KEY"],
        Provider::QwenCode => &["DASHSCOPE_API_KEY", "QWEN_API_KEY"],
        Provider::Kilo => &["KILO_API_KEY"],
        // Pi has no dedicated key: it fronts many providers and reads each
        // provider's own auth var. These are the representative keys; Pi is
        // Rejected in contract v1, so this is informational.
        Provider::Pi => &[
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_OAUTH_TOKEN",
            "OPENAI_API_KEY",
            "GEMINI_API_KEY",
        ],
        // Antigravity authenticates via the OS keyring / Google Sign-In OAuth;
        // it honors no API-key env var, so there is nothing to forward.
        // Rejected in contract v1 regardless, so this is informational.
        Provider::Antigravity => &[],
        // `Provider` is non-exhaustive, but every known variant is enumerated
        // above. A new provider must update this match rather than silently
        // receiving no forwarded auth variables.
        _ => unreachable!("auth_env_vars must be updated for new Provider variants"),
    }
}

/// One row of the support matrix, for README generation and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportRow {
    /// Provider slug (e.g. `claude`).
    pub slug: &'static str,
    /// Provider display name.
    pub display_name: &'static str,
    /// Provider binary on `$PATH`.
    pub binary: &'static str,
    /// Whether v1 enables the provider for tool-free inference.
    pub support: ProviderSupport,
}

/// The full provider support matrix in display order.
pub fn support_matrix() -> Vec<SupportRow> {
    all_providers()
        .map(|info| SupportRow {
            slug: info.slug,
            display_name: info.display_name,
            binary: info.binary,
            support: provider_support(info.provider),
        })
        .collect()
}
