//! Centralized provider catalog.
//!
//! This module is the canonical home for the [`Provider`] enum and all
//! per-provider static facts. Each [`Provider`] variant has exactly one
//! [`ProviderInfo`] constant served from [`provider_info`]
//! that exposes identity, documentation links, and four focused behavior
//! traits ([`ProviderBehavior`], [`McpBehavior`], [`AdapterBehavior`],
//! [`ConfiguratorBehavior`]).
//!
//! ## Generated data, hand-written behavior
//!
//! Each `provider/<slug>/` module splits cleanly along a generated /
//! hand-written seam:
//!
//! - `data.rs` — the `&'static ProviderInfo` constant, the named
//!   `EventMappingTable` static, the shared memory-files const, and the
//!   `LazyLock`-backed [`ProviderCapabilities`](crate::linking::capabilities::ProviderCapabilities)
//!   builder. This file is
//!   **generated** by `claudine-gen` from the field-source matrix (roster +
//!   facts + sidecar-validated research + human overrides) and is **never**
//!   hand-edited; drift is caught by the generator's `check` path (the
//!   nextest drift test). Shape changes flow through `claudine-gen`'s emitter,
//!   never this crate.
//! - `behavior.rs` — the hand-written zero-sized struct implementing the four
//!   behavior traits above (plus the parser/adapter/configurator/wrapper
//!   wiring). This is the only per-provider file authored by hand.
//!
//! The per-domain migration that once lived across events, linking, stream,
//! MCP, adapters, and configurators is complete: provider-varying *data* is a
//! [`ProviderInfo`] field, provider-varying *behavior* is a trait method, and
//! decentralized `match Provider` dispatch is held in check by the unified
//! dispatch-inventory guard (`claudine-cli/tests/dispatch_inventory.rs`). See
//! `docs/topics/provider-metadata.md` for the field-source matrix and the
//! generation pipeline.

mod acp;
mod antigravity;
mod behavior;
mod billing_model;
mod cap_policy;
mod claude;
mod cli_sensitivity;
mod codex;
mod display_policy;
mod errors;
mod event_mapping;
mod gemini;
mod goose;
mod identity;
mod kilo;
mod kimi;
mod known_gap;
mod methods;
mod model_catalog_source;
mod offering;
mod opencode;
mod output_format;
mod path_template;
mod pi;
mod platform_kind;
mod prompt_args;
mod qwen;
mod reasoning;
mod registry;
mod resume_support;
mod system_prompt;
mod unmapped_native_event;
mod yolo;

#[cfg(test)]
mod tests;

pub use acp::{AcpEvent, AcpServerMode, AcpSupport};
pub use behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
pub use billing_model::BillingModel;
pub use cap_policy::CapPolicy;
pub use cli_sensitivity::CliSensitiveAxes;
pub use display_policy::{DisplayPolicy, EventClass, ToolResultSummary};
pub use errors::{ConfigError, McpError};
pub use event_mapping::{EventMapping, EventMappingTable, EventSupportLevel};
pub use identity::PROVIDER_COUNT;
pub use known_gap::{KnownGap, KnownGapArea};
pub use model_catalog_source::ModelCatalogSource;
pub use offering::{ExpectedOffering, LocalRunnerIntegration, OfferingClass, OfferingSource};
pub use output_format::{EntrypointMode, EntrypointSpec, OutputFormat, OutputFormatSupport};
// Temporary shim: Provider, OutputFormatSelector and friends now live in
// `crate::provider_id`.  These re-exports keep existing importers working
// during the migration.
pub use crate::provider_id::{OutputFormatSelector, PROVIDERS_DISPLAY_ORDER, Provider};
pub use path_template::{GlobKind, PathContext, PathSegment, PathTemplate};
pub use platform_kind::PlatformKind;
pub use prompt_args::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
pub use reasoning::{ReasoningCustomTag, ReasoningSupport};
pub use registry::{all_providers, provider_info};
pub use resume_support::ResumeSupport;
pub use system_prompt::{
    SystemPromptCustomTag, SystemPromptDelivery, SystemPromptDeliveryByMode, SystemPromptSpec,
};
pub use unmapped_native_event::UnmappedNativeEvent;
pub use yolo::YoloSupport;

use serde::{Serialize, Serializer};
use sniff::programs::AiCli;

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
/// through [`provider_info`]. The four behavior
/// fields (`behavior`, `mcp`, `adapter`, `configurator`) carry the small set
/// of genuinely dynamic operations so a single registry lookup returns both
/// the data and the behavior halves of the catalog.
///
/// All static fields are `&'static` references or trivially copyable values
/// so a `ProviderInfo` lives in the binary's read-only data segment with no
/// heap allocation. Trait-object fields are skipped during serialization
/// because they are not data; the `fn`-pointer accessor
/// (`resource_support_fn`) is not itself data and serializes through the
/// descriptor it resolves. Every other catalog field is serializable so the
/// JSON output round-trips the catalog without information loss. JSON is the
/// authoritative descriptive surface for the typed catalog half.
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

    /// Accessor for the provider's resource portability descriptor used by
    /// cross-provider linking.
    ///
    /// The underlying data is built once via a `LazyLock` and lives in the
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

    /// Templates for user / project / local config files.
    ///
    /// ## Notes
    ///
    /// The first element is treated as the **primary user-level config
    /// path** by [`crate::config::discover_agents_full`]. A catalog
    /// invariant test (`config_paths_have_primary_user_entry`) asserts
    /// every provider declares at least one entry; the discovery helper
    /// relies on `config_paths[0]` to determine config-file presence and
    /// the path surfaced via [`crate::config::AgentInfo`].
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
    /// `server_mode` records the provider's own ACP posture (research-fed,
    /// acp topic); `events_via_acp` records which events Claudine captures
    /// through ACP. The two are decoupled: a provider can speak ACP without
    /// Claudine wiring any `EventSupportLevel::Acp` rows to it.
    pub acp: AcpSupport,

    /// Argv conventions describing how this provider's native CLI
    /// represents a prompt (prompt-carrying flags, optional entrypoint
    /// subcommand, and additional value-taking flags). Consumed by the
    /// CLI wrapper's prompt-extraction helpers.
    pub prompt_arg_conventions: PromptArgConventions,

    /// Typed expected-offering records from agent-models research,
    /// classified and joined to the unchained-ai models-catalog artifact
    /// by identity key. The runtime validation baseline: ids plus
    /// aliases feed `model_catalog::expected_baseline`.
    pub expected_offerings: &'static [ExpectedOffering],

    /// Offering-source namespaces (local runners today): id prefixes a
    /// user can route models through beyond the expected set, from
    /// model-config research.
    pub offering_sources: &'static [OfferingSource],

    /// Source of this provider's model catalog. See
    /// [`ModelCatalogSource`] for the variants.
    pub model_catalog_source: ModelCatalogSource,

    /// Provider-specific environment variables consulted (in order) for the
    /// `MODEL` selection chain. Consumed by `composition::select`.
    pub model_env_vars: &'static [&'static str],

    /// CLI override sensitivity axes. Consumed by `permissions::query` to
    /// flag results that may flip due to provider CLI flag overrides.
    pub cli_sensitive_axes: CliSensitiveAxes,

    /// Root-level files in the repo home that must be preserved during
    /// shadow-HOME isolation. Empty for providers without such files.
    pub repo_home_root_files: &'static [&'static str],

    /// Overall resume support level for session continuation.
    ///
    /// Support LEVEL only — session-id capture and resume argv mechanics
    /// deliberately stay out of the catalog (2026-07-04 resume-parity
    /// ruling); the wrapper profiles own those.
    pub resume: ResumeSupport,

    /// Bare CLI flag that selects the model at launch (e.g. `--model`),
    /// when the provider exposes one.
    pub model_cli_flag: Option<&'static str>,

    /// Provider flags that conflict with Claudine's non-interactive
    /// wrapping strategy. Bare flag tokens only; annotated research
    /// entries are excluded at generation.
    pub non_interactive_conflicting_flags: &'static [&'static str],

    /// Billing models the provider offers.
    pub billing_models: &'static [BillingModel],

    /// Cap policies the provider imposes (Layer A of the provider-neutral cap
    /// model): the static 1:M set of `(model-scope, timeframe)` windows, from
    /// research. The runtime cap event (`SignalEvent::UsageCapped` /
    /// `UsageCapApproaching`) is one firing of a policy; this catalog is the
    /// declared universe of them. Empty until research supplies a provider's
    /// policies.
    pub cap_policies: &'static [CapPolicy],

    /// Hand-ruled security allowlist of provider env keys that bypass the
    /// wrapper's sensitive-key sanitizer. Never auto-widened from
    /// research.
    pub allowed_env_keys: &'static [&'static str],

    /// Render policy consumed by shared render components (single owner
    /// of the curated stdout/stderr noise-prefix suppression lists).
    pub display_policy: DisplayPolicy,

    /// Whether structured non-interactive runs buffer filtered stderr and
    /// surface it only when the provider exits with an error.
    pub suppress_structured_stderr_on_success: bool,

    /// Whether a final assistant body is recoverable after an interactive
    /// session ends, closing inline composition.
    pub supports_interactive_inline_closure: bool,

    /// Whether the provider hard-requires an explicit model selection in
    /// non-TTY sessions.
    pub model_required_in_non_tty: bool,

    /// What kind of CLI product this is: a vendor platform predominantly
    /// fronts the vendor's own models (rolling-alias handling matters),
    /// while an agent aggregator is a model-agnostic front-end where
    /// provider/model pair selection is the central UX. Human-owned facts
    /// (spec 2026-07-02 classification; values ratified at Checkpoint D).
    pub platform_kind: PlatformKind,

    /// Provider-native hook events that fire at phases the 16-event model
    /// cannot represent. Claudine cannot dispatch these; each entry carries
    /// how to configure the event directly in the provider.
    pub unmapped_native_events: &'static [UnmappedNativeEvent],
}

impl ProviderInfo {
    /// Returns the provider's resource portability descriptor used by the
    /// cross-provider linking layer.
    pub fn resource_support(&self) -> &'static ProviderCapabilities {
        (self.resource_support_fn)()
    }
}
