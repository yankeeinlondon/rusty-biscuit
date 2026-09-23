//! Level 2 (`terminal-tests`) integration tests for `biscuit-tui-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout` rejects one.

#[path = "../common/mod.rs"]
mod common;

// Formerly the `real_terminal_render` target: its tests are `level2_`, and a
// `real_` path segment would also put them in the stub `real` tier (R5).
#[cfg(unix)]
mod terminal_render;
#[cfg(windows)]
mod windows_captured_stdout;
