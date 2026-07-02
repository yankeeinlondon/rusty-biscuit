//! Operating system and distribution detection.
//!
//! This module provides types and functions for detecting the current operating
//! system, Linux distribution details, and CI environment status.
//!
//! ## Submodules
//!
//! - [`types`] — `OsType`, `LinuxFamily`, `LinuxDistro`
//! - [`os_type`] — `detect_os_type`
//! - [`family`] — `infer_linux_family` (id → family classification)
//! - [`linux`] — Linux distro and WSL detection
//! - [`ci`] — CI environment detection
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::discovery::os_detection::{detect_os_type, detect_linux_distro, is_ci, OsType};
//!
//! let os = detect_os_type();
//! match os {
//!     OsType::Linux => {
//!         if let Some(distro) = detect_linux_distro() {
//!             println!("Running on {} ({})", distro.name, distro.family);
//!         }
//!     }
//!     OsType::MacOS => println!("Running on macOS"),
//!     OsType::Windows => println!("Running on Windows"),
//!     _ => println!("Running on {:?}", os),
//! }
//!
//! if is_ci() {
//!     println!("Running in CI environment");
//! }
//! ```

pub mod ci;
pub mod family;
pub mod linux;
pub mod os_type;
pub mod types;

pub use ci::is_ci;
pub use family::infer_linux_family;
pub use linux::{detect_linux_distro, is_wsl1, is_wsl2};
pub use os_type::detect_os_type;
pub use types::{LinuxDistro, LinuxFamily, OsType};
