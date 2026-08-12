//! Terminal capability discovery utilities.
//!
//! This module provides functions for detecting terminal capabilities,
//! operating system information, and configuration file paths.
//!
//! ## Sub-modules
//!
//! - [`detection`] - Terminal app detection, color depth, image support, etc.
//!   (Split into capability-aligned submodules: `app`, `color`, `dimensions`,
//!   `image`, `osc8`, `styling_caps`, `multiplex`, `connection`.)
//! - [`os_detection`] - Operating system and Linux distribution detection
//!   (Submodules: `types`, `os_type`, `family`, `linux`, `ci`.)
//! - [`config_paths`] - Terminal configuration file path discovery
//! - [`eval`] - Terminal capability evaluation utilities
//! - [`osc_queries`] - OSC color queries (10/11/12) for background/foreground detection
//!   (Submodules: `types`, `parse`, `query`, `support`.)
//! - [`mode_2027`] - Unicode grapheme cluster width support detection
//! - [`clipboard`] - OSC52 clipboard support for terminal applications
//! - [`fonts`] - Font detection utilities (font name, size, ligatures)
//!   (Per-terminal parsers under `wezterm`, `kitty`, `alacritty`, `ghostty`, `iterm2`,
//!   plus `nerd`, `window_size`, `parser`, and `types` modules.)
//! - [`app_metadata`] - Per-app config-location and environment metadata model
//!   (`&'static` facts + OS-target resolution + portable path-template expansion).

pub mod app_metadata;
pub mod clipboard;
pub mod config_paths;
pub mod cursor_position;
pub mod detection;
pub mod eval;
pub mod fonts;
pub mod locale;
pub mod mode_2027;
pub mod os_detection;
pub mod osc_queries;
pub mod raw_mode;
