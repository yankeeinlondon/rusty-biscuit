//! Level 1 integration tests for `biscuit-terminal-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod about;
mod dir_targets;
mod integration_test;
mod output_markers;
// The pure output-marker detector the Level 2 helpers use, compiled here so
// its regression tests run without the `terminal-tests` feature.
#[path = "../common/output_region.rs"]
mod output_region;
mod test_layout;
