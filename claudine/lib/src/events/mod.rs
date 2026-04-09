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
    CanonicalProviderSettings, EventBinding, GlobalSettings, LinkingSettings, TtsSettings,
};
pub use environment::{
    detect_environment, detect_environment_fast, EnvironmentContext, GitContext, HardwareContext,
    OsContext, RepoContext,
};
pub use event_meta::{EventMeta, ToolName};
pub use init_defaults::{
    default_speak_template, quick_start_supported_providers, recommended_sound, TtsProviderOption,
    INIT_EVENT_DISPLAY_ORDER, INIT_RECOMMENDED_EVENTS, INIT_TTS_PROVIDERS,
};
pub use matrix::{
    event_native_mapping_matrix, event_support_matrix, EventNativeMappingCell,
    EventNativeMappingRow, EventSupportCell, EventSupportRow, NativeEventName,
    PROVIDERS_DISPLAY_ORDER,
};
pub use provider::{EventSupportLevel, Provider};
pub use resolved_hook::ResolvedHook;
