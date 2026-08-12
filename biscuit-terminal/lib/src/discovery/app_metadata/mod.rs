//! Per-app terminal configuration and environment metadata.
//!
//! This module holds a `&'static`, vendor-determined description of where each
//! supported terminal keeps its config (per OS target, with env overrides), what
//! settings live inside that config, and what runtime facts the terminal exports
//! into the environment. It is the config + environment slice of the future
//! `TerminalAppProfile`, shipped standalone.
//!
//! The type model lives in [`types`], OS-target resolution in [`os_target`], and
//! portable path-template expansion in the internal `expand` module. The seed
//! table (internal `seed`) holds the per-app `&'static` facts, and `resolver`
//! implements the host-aware `TerminalApp::get_config_file*` methods over it.
//!
//! See `features/2026-06-29-app-metadata/spec.md` for the full design.

mod expand;
mod extract;
mod os_target;
mod resolver;
mod seed;
mod types;

pub(crate) use resolver::default_config_paths;
pub use extract::{ConfigDocument, SettingValue};
pub use os_target::{ConfigOsTarget, current_config_os_target};
pub use types::{
    ConfigCandidate, ConfigEnvKind, ConfigFormat, ConfigLocationEnv, ConfigMetadata, ConfigSource,
    EnvFactMap, OsConfigLocations, ResolvedConfig, SettingLocator, SettingLocators,
    TerminalAppMetadata, ValueKind,
};
