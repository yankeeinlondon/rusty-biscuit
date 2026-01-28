//! # biscuit-terminal
//!
//! Terminal capability detection and utilities for Rust applications.
//!
//! This crate provides comprehensive terminal environment detection including:
//!
//! - **OS Detection**: Identify operating system and Linux distribution
//! - **Terminal App Detection**: Recognize 12+ terminal emulators
//! - **Color Support**: Query color depth, mode (light/dark), and background color
//! - **Escape Code Analysis**: Calculate visual line widths, detect escape codes
//! - **Clipboard**: OSC52 clipboard support for compatible terminals
//! - **Config Paths**: Find terminal configuration files
//!
//! ## Quick Start
//!
//! ```
//! use biscuit_terminal::terminal::Terminal;
//!
//! let term = Terminal::new();
//!
//! println!("Running in {:?}", term.app);
//! println!("Terminal size: {}x{}", Terminal::width(), Terminal::height());
//!
//! if term.supports_italic {
//!     println!("\x1b[3mItalic text!\x1b[0m");
//! }
//! ```
//!
//! ## Modules
//!
//! - [`terminal`] - Main `Terminal` struct with all capabilities
//! - [`discovery`] - Low-level detection functions
//!   - [`discovery::detection`] - Terminal app, color depth, image support
//!   - [`discovery::os_detection`] - OS and Linux distribution detection
//!   - [`discovery::config_paths`] - Terminal config file paths
//!   - [`discovery::osc_queries`] - Terminal color queries
//!   - [`discovery::clipboard`] - OSC52 clipboard support
//!   - [`discovery::mode_2027`] - Unicode grapheme cluster support
//!   - [`discovery::eval`] - Escape code analysis utilities
//! - [`components`] - Renderable terminal components (sections, lists, tables)
//! - [`utils`] - Utility functions (colors, styling, escape codes)

pub mod components;
pub mod discovery;
pub mod terminal;
pub mod utils;
