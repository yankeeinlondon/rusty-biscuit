mod agentic_event;
mod config;
mod environment;
mod event_meta;
mod init_defaults;
mod matrix;
mod resolved_hook;

pub use agentic_event::AgenticEvent;
pub use config::{
    CanonicalProviderSettings, EventBinding, GlobalSettings, LinkingSettings, TtsSettings,
};
pub use environment::{
    EnvironmentContext, GitContext, HardwareContext, OsContext, RepoContext, detect_environment,
    detect_environment_fast, environment_context_from_sniff_result,
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

// Phase 8: `Provider`, `PROVIDERS_DISPLAY_ORDER`, and `EventSupportLevel`
// previously lived in `crate::events::provider`. They are now owned by
// `crate::provider`. Internal consumers must import directly from there.
