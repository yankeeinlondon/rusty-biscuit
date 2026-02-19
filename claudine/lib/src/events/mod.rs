mod agentic_event;
mod config;
mod environment;
mod event_meta;
mod init_defaults;
mod matrix;
mod provider;
mod resolved_hook;

pub use agentic_event::AgenticEvent;
pub use config::{
    CanonicalProviderSettings, EventBinding, GlobalSettings, HookerConfig, LinkingSettings,
    ProviderConfig, TtsSettings,
};
pub use environment::{
    EnvironmentContext, GitContext, HardwareContext, OsContext, RepoContext, detect_environment,
};
pub use event_meta::EventMeta;
pub use init_defaults::{
    INIT_EVENT_DISPLAY_ORDER, INIT_RECOMMENDED_EVENTS, INIT_TTS_PROVIDERS, TtsProviderOption,
    default_speak_template, quick_start_supported_providers, recommended_sound,
};
pub use matrix::{
    EventNativeMappingCell, EventNativeMappingRow, EventSupportCell, EventSupportRow,
    NativeEventName, PROVIDERS_DISPLAY_ORDER, event_native_mapping_matrix, event_support_matrix,
};
pub use provider::{EventSupportLevel, Provider};
pub use resolved_hook::ResolvedHook;
