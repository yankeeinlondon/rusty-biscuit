//! Narrow façade re-exporting only the types the dispatch loader consumes.
//!
//! This collapses the loader's inbound edge count (13 distinct `crate::*`
//! paths) into a single module import.

pub use crate::actions::{CompiledMapper, HookAction, Mapper};
pub use crate::config::atomic::atomic_write;
pub use crate::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};
pub(crate) use crate::config::merge::merge_repo_override;
pub use crate::config::messaging_block::{ClaudineMessengerConfig, MessengerProviderConfig};
pub use crate::config::migration;
pub use crate::dispatch::matcher::{compile_many, RuntimeMatcher};
pub use crate::error::{ClaudineError, ConfigCause, Result};
pub use crate::events::{AgenticEvent, GlobalSettings};
pub use crate::messaging::{
    MessagingRouteConfig, RuntimeMessagingSettings, ScopedMessagingSettings,
};
pub use crate::protect::catalog::ProtectPlatform;
pub use crate::protect::service::ProtectService;
