//! Level 3 (`terminal-tests`) integration tests for `claudine-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_placement.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

#[cfg(target_os = "macos")]
mod level3_auto_complete_chooser;
#[cfg(target_os = "linux")]
mod level3_linux_sequence_ctrl_c;
#[cfg(target_os = "macos")]
mod level3_sequence_ctrl_c;
#[cfg(windows)]
mod level3_windows_sequence_ctrl_c;
mod level3_wrap_ctrl_c;
