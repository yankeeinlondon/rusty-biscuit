//! `biscuit-tui-cli` integration tests that need the `terminal-tests` feature, one
//! test binary per execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! The binary groups tests by feature contract, not by tier: tier selection
//! follows each test's path, so the unmarked `windows_captured_stdout` tests
//! run in Level 1 and the `level2_` tests in Level 2. Each module was its own
//! test target before the consolidation and keeps that target's name, except
//! where an alias is noted below. `Cargo.toml` sets `autotests = false`, so a
//! file in this directory that is not declared below never compiles;
//! `test_layout` rejects one.

#[path = "../common/mod.rs"]
mod common;

// Formerly the `real_terminal_render` target: its tests are `level2_`, and a
// `real_` path segment would also put them in the stub `real` tier (R5).
#[cfg(unix)]
mod terminal_render;
#[cfg(windows)]
mod windows_captured_stdout;
