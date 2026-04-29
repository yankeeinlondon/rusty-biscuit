//! Centralized provider catalog.
//!
//! This module is the canonical home for the [`Provider`] enum and all
//! per-provider static facts. Each [`Provider`] variant has exactly one
//! [`ProviderInfo`] constant served from [`provider_info`](registry::provider_info)
//! that exposes identity, documentation links, and four focused behavior
//! traits ([`ProviderBehavior`], [`McpBehavior`], [`AdapterBehavior`],
//! [`ConfiguratorBehavior`]).
//!
//! Phase 1 of the centralized providers refactor establishes this module
//! as the home for `Provider` and the static-data scaffolding while leaving
//! existing per-domain dispatch (events, agents, linking, stream, MCP,
//! adapters, configurators) intact. Subsequent phases migrate per-domain
//! data into [`ProviderInfo`] fields and dynamic dispatch through the
//! behavior traits.

mod acp;
mod behavior;
mod claude;
mod cli_sensitivity;
mod codex;
mod errors;
mod event_mapping;
mod gemini;
mod goose;
mod identity;
mod kimi;
mod known_gap;
mod methods;
mod model_catalog_source;
mod opencode;
mod output_format;
mod path_template;
mod prompt_args;
mod qwen;
mod reasoning;
mod registry;
mod roo;
mod system_prompt;
mod yolo;

#[cfg(test)]
mod tests;

pub use acp::{AcpEvent, AcpServerMode, AcpSupport};
pub use behavior::{AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
pub use cli_sensitivity::CliSensitiveAxes;
pub use errors::{ConfigError, McpError};
pub use event_mapping::{EventMapping, EventMappingTable, EventSupportLevel};
pub use identity::{PROVIDER_COUNT, PROVIDERS_DISPLAY_ORDER, Provider};
pub use known_gap::{KnownGap, KnownGapArea};
pub use model_catalog_source::ModelCatalogSource;
pub use output_format::{
    EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSelector, OutputFormatSupport,
};
pub use path_template::{GlobKind, PathContext, PathSegment, PathTemplate};
pub use prompt_args::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
pub use reasoning::{ReasoningCustomTag, ReasoningSupport};
pub use registry::{all_providers, provider_info};
pub use system_prompt::{
    SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
pub use yolo::YoloSupport;

use serde::{Serialize, Serializer};
use sniff::programs::AiCli;

use crate::agents::AgentCapabilities;
use crate::linking::capabilities::ProviderCapabilities;
use crate::stream::StreamProtocol;

/// `serialize_with` adapter for [`ProviderInfo::resource_support_fn`].
///
/// Calls the fn pointer and forwards the resolved
/// `&'static ProviderCapabilities` into the supplied serializer so the JSON
/// describe surface emits the full resource portability descriptor under the
/// `resource_support` key.
fn serialize_resource_support<S: Serializer>(
    accessor: &fn() -> &'static ProviderCapabilities,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    accessor().serialize(serializer)
}

/// All static, serializable facts about a provider.
///
/// Populated once per provider variant as a `&'static ProviderInfo` accessed
/// through [`provider_info`](registry::provider_info). The four behavior
/// fields (`behavior`, `mcp`, `adapter`, `configurator`) carry the small set
/// of genuinely dynamic operations so a single registry lookup returns both
/// the data and the behavior halves of the catalog.
///
/// All static fields are `&'static` references or trivially copyable values
/// so a `ProviderInfo` lives in the binary's read-only data segment with no
/// heap allocation. Trait-object fields are skipped during serialization
/// because they are not data; the `fn`-pointer accessors
/// (`agent_capabilities_fn`, `resource_support_fn`) are not themselves data.
/// The legacy [`AgentCapabilities`] accessor is skipped so structured JSON
/// exposes only typed top-level catalog fields plus the `resource_support`
/// descriptor. Every other catalog field is serializable so the JSON output
/// round-trips the catalog without information loss. JSON is the authoritative
/// descriptive surface for the typed catalog half.
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    /// Canonical [`Provider`] identifier for this entry.
    pub provider: Provider,

    /// Friendly display name (e.g. "Claude", "Kimi Code").
    pub display_name: &'static str,

    /// Snake-case slug suitable for file paths and JSON keys (e.g. "claude").
    pub slug: &'static str,

    /// Short display name used in event logs and CLI output (e.g. "claude", "kimi").
    pub short_name: &'static str,

    /// Provider binary name on `$PATH` (e.g. "claude", "codex").
    pub binary: &'static str,

    /// Agent offset directory used for shadow-HOME isolation (e.g. ".claude").
    pub agent_offset: &'static str,

    /// CLI alias forms accepted on the command line.
    pub cli_aliases: &'static [&'static str],

    /// Provider documentation homepage.
    pub docs_url: &'static str,

    /// Usage / billing dashboard URL when one exists.
    pub usage_dashboard_url: Option<&'static str>,

    /// Typed binding to the [`sniff::programs::AiCli`] enum used for install
    /// detection.
    pub sniff_binding: AiCli,

    /// Whether this provider supports skill discovery.
    pub supports_skills: bool,

    /// Structured stream format used by the provider, when one is present.
    ///
    /// `None` for providers without a structured non-interactive output
    /// stream.
    pub stream_protocol: Option<StreamProtocol>,

    /// Per-provider event mapping table. Source of truth for event support
    /// level, provider-native event names, parse aliases, and which rows
    /// participate in standard hook registration.
    pub event_mapping: &'static EventMappingTable,

    /// Cross-cutting dynamic behavior (payload detection, parser construction).
    #[serde(skip)]
    pub behavior: &'static dyn ProviderBehavior,

    /// MCP import / state / inject / export behavior. Defaults to typed
    /// "not supported" results for providers without MCP support.
    #[serde(skip)]
    pub mcp: &'static dyn McpBehavior,

    /// Inbound payload parser behavior. Defaults to no-op detection for
    /// providers that cannot be detected from raw payloads.
    #[serde(skip)]
    pub adapter: &'static dyn AdapterBehavior,

    /// Hook configuration / registration behavior. Defaults to typed
    /// "not supported" results for providers without config-file hooks.
    #[serde(skip)]
    pub configurator: &'static dyn ConfiguratorBehavior,

    /// Accessor for the provider's full agent capability descriptor.
    ///
    /// The accessor returns a `'static` reference into a per-provider
    /// `LazyLock<AgentCapabilities>` defined alongside the provider's
    /// [`ProviderInfo`] constant. It exists only for compatibility with the
    /// legacy `agents::registry::agent_for` facade and is skipped during
    /// serialization so structured describe JSON cannot expose the old
    /// string-heavy capability tree.
    #[serde(skip)]
    pub agent_capabilities_fn: fn() -> &'static AgentCapabilities,

    /// Accessor for the provider's resource portability descriptor used by
    /// cross-provider linking.
    ///
    /// As with [`agent_capabilities_fn`](Self::agent_capabilities_fn), the
    /// underlying data is built once via a `LazyLock` and lives in the
    /// provider module so the linking facade can forward to it. The
    /// fn-pointer is serialized via `serialize_resource_support` under the
    /// canonical `resource_support` key so the typed catalog half of the
    /// JSON describe output round-trips without information loss.
    #[serde(
        rename = "resource_support",
        serialize_with = "serialize_resource_support"
    )]
    pub resource_support_fn: fn() -> &'static ProviderCapabilities,

    // -- Typed catalog data -------------------------------------------------
    //
    // These fields are the strongly typed half of the centralized provider
    // catalog. JSON serialization of the catalog (consumed by
    // `claudine providers --describe --format json`) is the authoritative
    // descriptive surface for these fields.
    /// Templates for per-session log files (e.g. JSONL transcripts).
    pub session_log_paths: &'static [PathTemplate],

    /// Templates for ancillary session-state directories.
    pub session_locations: &'static [PathTemplate],

    /// Templates for user / project / local config files.
    ///
    /// ## Notes
    ///
    /// The first element is treated as the **primary user-level config
    /// path** by [`crate::config::discover_agents_full`]. The catalog
    /// invariant test
    /// [`crate::provider::tests::config_paths_have_primary_user_entry`]
    /// asserts every provider declares at least one entry; the discovery
    /// helper relies on `config_paths[0]` to determine config-file
    /// presence and the path surfaced via [`crate::config::AgentInfo`].
    pub config_paths: &'static [PathTemplate],

    /// Templates for memory / instruction files contributing to the
    /// system prompt hierarchy.
    pub memory_files: &'static [PathTemplate],

    /// Output formats supported in non-interactive mode.
    pub output_formats: &'static [OutputFormatSupport],

    /// Available non-interactive (and selected interactive) entrypoints.
    pub entrypoints: &'static [EntrypointSpec],

    /// Typed system-prompt delivery descriptor.
    pub system_prompt: &'static SystemPromptSpec,

    /// Typed YOLO / auto-approve descriptor.
    pub yolo: YoloSupport,

    /// Typed reasoning / extended-thinking descriptor.
    pub reasoning: ReasoningSupport,

    /// Known gaps in provider capability data, classified by area.
    pub known_gaps: &'static [KnownGap],

    /// Typed ACP capability descriptor.
    ///
    /// Consumed by hook reporting (`claudine hooks --capture-method`) and
    /// by the invariant test that pairs `EventSupportLevel::Acp` rows with
    /// a non-`NotSupported` [`AcpSupport::server_mode`].
    pub acp: AcpSupport,

    /// Argv conventions describing how this provider's native CLI
    /// represents a prompt (prompt-carrying flags, optional entrypoint
    /// subcommand, and additional value-taking flags). Consumed by the
    /// CLI wrapper's prompt-extraction helpers.
    pub prompt_arg_conventions: PromptArgConventions,

    /// Static, compiled-in model catalog for providers with a known model
    /// enum. Empty for providers whose catalog is dynamic or unavailable.
    pub static_models: &'static [&'static str],

    /// Source of this provider's dynamic model catalog. See
    /// [`ModelCatalogSource`] for the variants.
    pub dynamic_source: ModelCatalogSource,

    /// Provider-specific environment variables consulted (in order) for the
    /// `MODEL` selection chain. Consumed by `composition::select`.
    pub model_env_vars: &'static [&'static str],

    /// CLI override sensitivity axes. Consumed by `permissions::query` to
    /// flag results that may flip due to provider CLI flag overrides.
    pub cli_sensitive_axes: CliSensitiveAxes,

    /// Root-level files in the repo home that must be preserved during
    /// shadow-HOME isolation. Empty for providers without such files.
    pub repo_home_root_files: &'static [&'static str],
}

impl ProviderInfo {
    /// Returns the provider's full [`AgentCapabilities`] descriptor.
    pub fn agent_capabilities(&self) -> &'static AgentCapabilities {
        (self.agent_capabilities_fn)()
    }

    /// Returns the provider's resource portability descriptor used by the
    /// cross-provider linking layer.
    pub fn resource_support(&self) -> &'static ProviderCapabilities {
        (self.resource_support_fn)()
    }
}

// Implementing [`crate::agents::Agent`] on `ProviderInfo` lets the legacy
// `agents::agent_for(provider)` registry forward straight to
// [`provider_info`](registry::provider_info) without keeping per-provider
// thin facade structs around.
impl crate::agents::Agent for ProviderInfo {
    fn id(&self) -> Provider {
        self.provider
    }

    fn capabilities(&self) -> &AgentCapabilities {
        self.agent_capabilities()
    }
}
