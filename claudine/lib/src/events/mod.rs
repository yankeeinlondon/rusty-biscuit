mod agentic_event;
mod config;
mod environment;
mod event_meta;
mod init_defaults;
mod matrix;
#[allow(deprecated)]
pub mod provider;
mod resolved_hook;

pub use agentic_event::AgenticEvent;
pub use config::{
    CanonicalProviderSettings, EventBinding, GlobalSettings, LinkingSettings, TtsSettings,
};
pub use environment::{
    EnvironmentContext, GitContext, HardwareContext, OsContext, RepoContext, detect_environment,
    detect_environment_fast, environment_context_from_sniff_result,
    environment_context_from_sniff_result_and_env,
};
pub use event_meta::{EventMeta, ToolName};
pub use init_defaults::{
    INIT_EVENT_DISPLAY_ORDER, INIT_RECOMMENDED_EVENTS, INIT_TTS_PROVIDERS, TtsProviderOption,
    default_speak_template, quick_start_supported_providers, recommended_sound,
};
pub use matrix::{
    EventNativeMappingCell, EventNativeMappingRow, EventSupportCell, EventSupportRow,
    NativeEventName, event_native_mapping_matrix, event_support_matrix,
};
pub use resolved_hook::ResolvedHook;

// Deprecated compatibility re-exports.
//
// `Provider`, `PROVIDERS_DISPLAY_ORDER`, and `EventSupportLevel` previously
// lived in `crate::events::provider` and are now owned by `crate::provider`.
// The `provider` submodule above survives one release cycle as a thin
// `#[deprecated]` re-export module per spec §"Migration Plan" and design
// §6.1. Removal is scheduled for the post-Phase-8 cleanup release.
//
// Internal consumers must import directly from `crate::provider`; the
// re-exports below exist solely to keep external consumers compiling
// during the deprecation window.
#[allow(deprecated)]
pub use provider::{EventSupportLevel, PROVIDERS_DISPLAY_ORDER, Provider};
