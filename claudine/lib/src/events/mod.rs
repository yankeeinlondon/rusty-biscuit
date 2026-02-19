mod agentic_event;
mod config;
mod environment;
mod event_meta;
mod hook_action;
mod hook_response;
mod provider;
mod resolved_hook;

pub use agentic_event::AgenticEvent;
pub use config::{EventBinding, GlobalSettings, HookerConfig, ProviderConfig, TtsSettings};
pub use environment::{
    EnvironmentContext, GitContext, HardwareContext, OsContext, RepoContext, detect_environment,
};
pub use event_meta::EventMeta;
pub use hook_action::{CompiledMapper, HookAction, LogTarget, Mapper, ReportFormat, ReportHandler};
pub use hook_response::{HookDecision, HookResponse};
pub use provider::{EventSupportLevel, Provider};
pub use resolved_hook::ResolvedHook;
