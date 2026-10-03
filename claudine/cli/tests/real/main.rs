//! real-provider (`real-tests`) integration tests for `claudine-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_placement.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod real_codex_app_server;
mod real_inline_write_grant;
#[cfg(unix)]
mod real_native_interactive_startup;
#[cfg(unix)]
mod real_opencode_yolo_subagent;
#[cfg(unix)]
mod real_pi_interactive_startup;
mod real_pi_managed_rpc;
mod real_pi_steering;
