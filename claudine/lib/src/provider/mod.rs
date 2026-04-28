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

mod behavior;
mod claude;
mod codex;
mod errors;
mod event_mapping;
mod gemini;
mod goose;
mod identity;
mod kimi;
mod known_gap;
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

pub use behavior::{AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior};
pub use errors::{ConfigError, McpError};
pub use event_mapping::{EventMapping, EventMappingTable};
pub use identity::{Provider, PROVIDERS_DISPLAY_ORDER};
pub use known_gap::{KnownGap, KnownGapArea};
pub use output_format::{EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport};
pub use path_template::{GlobKind, PathContext, PathSegment, PathTemplate};
pub use prompt_args::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
pub use reasoning::{ReasoningCustomTag, ReasoningSupport};
pub use registry::{all_providers, provider_info};
pub use system_prompt::{
    SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
pub use yolo::YoloSupport;

use serde::Serialize;
use sniff::programs::AiCli;

use crate::agents::AgentCapabilities;
use crate::linking::capabilities::ProviderCapabilities;
use crate::stream::StreamProtocol;

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
/// heap allocation. Trait-object fields are skipped during serialization.
#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    /// Canonical [`Provider`] identifier for this entry.
    pub provider: Provider,

    /// Friendly display name (e.g. "Claude", "Kimi Code").
    pub display_name: &'static str,

    /// Snake-case slug suitable for file paths and JSON keys (e.g. "claude").
    pub slug: &'static str,

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
    /// stream. Skipped during serialization to keep the
    /// `claudine providers --describe --format json` payload byte-for-byte
    /// identical with prior phases.
    #[serde(skip)]
    pub stream_protocol: Option<StreamProtocol>,

    /// Per-provider event mapping table. Source of truth for event support
    /// level, provider-native event names, parse aliases, and which rows
    /// participate in standard hook registration.
    ///
    /// Skipped during serialization because Phase 3 of the centralized
    /// providers refactor deliberately preserves the existing
    /// `claudine providers --describe --format json` payload bit-for-bit;
    /// Phase 5 introduces typed event-mapping serialization as an additive
    /// change.
    #[serde(skip)]
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
    /// [`ProviderInfo`] constant. Phase 2 of the centralized providers
    /// refactor moves the data definitions into `provider/<name>.rs`;
    /// the legacy `agents::registry::agent_for` facade now forwards to
    /// this accessor.
    #[serde(skip)]
    pub agent_capabilities_fn: fn() -> &'static AgentCapabilities,

    /// Accessor for the provider's resource portability descriptor used by
    /// cross-provider linking.
    ///
    /// As with [`agent_capabilities_fn`](Self::agent_capabilities_fn), the
    /// underlying data is built once via a `LazyLock` and lives in the
    /// provider module so the linking facade can forward to it.
    #[serde(skip)]
    pub resource_support_fn: fn() -> &'static ProviderCapabilities,

    // -- Phase 5: typed catalog data ----------------------------------------
    //
    // These fields are additive for Phase 5 of the centralized providers
    // refactor. They live alongside the legacy `Vec<&'static str>` fields
    // on `RuntimeCapabilities` (kept through Phase 8) and are skipped
    // during serialization so the existing `claudine providers --describe
    // --format json` payload is unchanged.
    /// Templates for per-session log files (e.g. JSONL transcripts).
    #[serde(skip)]
    pub session_log_paths: &'static [PathTemplate],

    /// Templates for ancillary session-state directories.
    #[serde(skip)]
    pub session_locations: &'static [PathTemplate],

    /// Templates for user / project / local config files.
    #[serde(skip)]
    pub config_paths: &'static [PathTemplate],

    /// Templates for memory / instruction files contributing to the
    /// system prompt hierarchy.
    #[serde(skip)]
    pub memory_files: &'static [PathTemplate],

    /// Output formats supported in non-interactive mode.
    #[serde(skip)]
    pub output_formats: &'static [OutputFormatSupport],

    /// Available non-interactive (and selected interactive) entrypoints.
    #[serde(skip)]
    pub entrypoints: &'static [EntrypointSpec],

    /// Typed system-prompt delivery descriptor.
    #[serde(skip)]
    pub system_prompt: &'static SystemPromptSpec,

    /// Typed YOLO / auto-approve descriptor.
    #[serde(skip)]
    pub yolo: YoloSupport,

    /// Typed reasoning / extended-thinking descriptor.
    #[serde(skip)]
    pub reasoning: ReasoningSupport,

    /// Known gaps in provider capability data, classified by area.
    #[serde(skip)]
    pub known_gaps: &'static [KnownGap],

    /// Argv conventions describing how this provider's native CLI
    /// represents a prompt (prompt-carrying flags, optional entrypoint
    /// subcommand, and additional value-taking flags). Consumed by the
    /// CLI wrapper's prompt-extraction helpers.
    #[serde(skip)]
    pub prompt_arg_conventions: PromptArgConventions,
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
