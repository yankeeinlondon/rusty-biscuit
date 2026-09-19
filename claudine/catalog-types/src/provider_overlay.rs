//! Provider-overlay vocabulary shared by the catalog generator and the
//! Claudine library.
//!
//! A *provider overlay* is Claudine-controlled storage carrying the config
//! and resource view for one wrapped launch. It is reached through a
//! **provider-owned selector** — a documented environment variable whose
//! effects are limited to that provider — never by replacing the child's
//! `HOME`.
//!
//! Four vocabularies live here because both halves of the catalog pipeline
//! need them: `claudine-gen` validates the facts records against these
//! variant names without linking the library, and the generated `data.rs`
//! references the types directly.
//!
//! The ground truth behind every value is the per-provider audit at
//! `fixes/2026-09-12-shadow-home/audit.md`.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Why a launch needs a provider overlay.
///
/// A launch can carry several reasons at once (`--repo` plus `--mcp` is the
/// common case), which is why the capability verdict is recorded per reason
/// rather than per provider.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, EnumIter, IntoStaticStr,
    VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OverlayReason {
    /// `--repo`: repository-biased isolation of the provider's user-scoped
    /// resources.
    RepoResources,
    /// Repository prompt discovery (Codex today).
    RepoPrompt,
    /// MCP delivery requires a Claudine-provided provider config root.
    Mcp,
}

/// Per-(provider, reason) verdict: how — or whether — Claudine can satisfy
/// an overlay reason for a provider.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OverlayCapability {
    /// Satisfied by a filesystem overlay reached through the provider's own
    /// root selector.
    NativeRoot,
    /// Satisfied with **no** filesystem overlay — the provider accepts the
    /// configuration inline (env-carried content or argv).
    ComposableInjection,
    /// No verified provider-owned mechanism exists. Claudine refuses the
    /// reason before spawning rather than replacing `HOME`.
    Unsupported,
}

/// What a selector's value names, relative to the directory the provider
/// actually reads its configuration from.
///
/// Getting this backwards silently produces a doubly nested config path and
/// a provider that reads the user's real configuration, so the shape is
/// recorded per provider from observed evidence and applied in exactly one
/// place.
///
/// `ParentOfProviderDir` carries the child segment because the appended name
/// varies by provider (Gemini appends `.gemini`); encoding it in data is
/// what keeps the offset from being hard-coded at each injection site.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OverlaySelectorShape {
    /// The selector's value IS the directory holding the provider's config
    /// file (`$CODEX_HOME/config.toml`).
    ProviderDir,
    /// The provider creates `child` inside the selector's value
    /// (`$GEMINI_CLI_HOME/.gemini/settings.json`).
    ///
    /// Authored as the externally tagged object
    /// `{"parent_of_provider_dir": {"child": ".gemini"}}`.
    ParentOfProviderDir {
        /// Directory segment the provider appends to the selector's value.
        child: &'static str,
    },
    /// The selector carries configuration *content*, not a directory, so no
    /// filesystem overlay exists to point at.
    Inline,
}

/// A class of provider-owned resource a selector relocates.
///
/// Deliberately coarse: provider-specific resource kinds (agents, commands,
/// modes, skills, plugins, prompts) live under the provider's config tree
/// and are recorded as [`Config`](OverlayResourceClass::Config). The
/// distinction that matters downstream is settings versus credentials
/// versus history versus disposable cache versus live mutable state —
/// [`State`](OverlayResourceClass::State) is the class that must never be
/// mirrored with per-file links.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, EnumIter, IntoStaticStr,
    VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OverlayResourceClass {
    /// Settings and the resource definitions that live beside them.
    Config,
    /// Credentials, tokens, and auth sidecars.
    Auth,
    /// Session transcripts and history.
    Sessions,
    /// Regenerable caches.
    Cache,
    /// Live mutable state: databases, journals, lock files, sockets.
    State,
}

#[cfg(test)]
mod tests {
    use strum::VariantNames;

    use super::*;

    /// The facts records are keyed and valued by these snake_case names;
    /// the generator validates against the same lists.
    #[test]
    fn variant_names_are_snake_case_wire_forms() {
        assert_eq!(
            OverlayReason::VARIANTS,
            &["repo_resources", "repo_prompt", "mcp"]
        );
        assert_eq!(
            OverlayCapability::VARIANTS,
            &["native_root", "composable_injection", "unsupported"]
        );
        assert_eq!(
            OverlaySelectorShape::VARIANTS,
            &["provider_dir", "parent_of_provider_dir", "inline"]
        );
        assert_eq!(
            OverlayResourceClass::VARIANTS,
            &["config", "auth", "sessions", "cache", "state"]
        );
    }

    /// Unit variants serialize as bare member strings; `ParentOfProviderDir`
    /// serializes as the externally tagged object. Both forms ARE the
    /// authoring shape used in `docs/providers/facts/<slug>.yaml`.
    #[test]
    fn serde_wire_forms_match_catalog_shape() {
        assert_eq!(
            serde_json::to_value(OverlayCapability::ComposableInjection).unwrap(),
            serde_json::json!("composable_injection")
        );
        assert_eq!(
            serde_json::to_value(OverlaySelectorShape::ProviderDir).unwrap(),
            serde_json::json!("provider_dir")
        );
        assert_eq!(
            serde_json::to_value(OverlaySelectorShape::ParentOfProviderDir { child: ".gemini" })
                .unwrap(),
            serde_json::json!({"parent_of_provider_dir": {"child": ".gemini"}})
        );
        assert_eq!(
            serde_json::to_value(OverlayResourceClass::State).unwrap(),
            serde_json::json!("state")
        );
        assert_eq!(
            serde_json::to_value(OverlayReason::RepoResources).unwrap(),
            serde_json::json!("repo_resources")
        );
    }

    #[test]
    fn into_static_str_covers_the_data_variant() {
        let parent = OverlaySelectorShape::ParentOfProviderDir { child: ".gemini" };
        assert_eq!(<&'static str>::from(parent), "parent_of_provider_dir");
        assert_eq!(
            <&'static str>::from(OverlaySelectorShape::ProviderDir),
            "provider_dir"
        );
    }
}
