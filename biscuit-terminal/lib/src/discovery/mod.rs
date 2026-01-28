//! Terminal capability discovery utilities.
//!
//! This module provides functions for detecting terminal capabilities,
//! operating system information, and configuration file paths.
//!
//! ## Sub-modules
//!
//! - [`detection`] - Terminal app detection, color depth, image support, etc.
//! - [`os_detection`] - Operating system and Linux distribution detection
//! - [`config_paths`] - Terminal configuration file path discovery
//! - [`eval`] - Terminal capability evaluation utilities
//! - [`osc_queries`] - OSC color queries (10/11/12) for background/foreground detection
//! - [`mode_2027`] - Unicode grapheme cluster width support detection
//! - [`clipboard`] - OSC52 clipboard support for terminal applications
//! - [`fonts`] - Font detection utilities (font name, size, ligatures)

pub mod clipboard;
pub mod config_paths;
pub mod detection;
pub mod eval;
pub mod mode_2027;
pub mod osc_queries;
pub mod os_detection;
pub mod fonts;
pub mod locale;
pub mod service_manager;
