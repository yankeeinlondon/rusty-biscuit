//! Level 3 (`terminal-tests`) integration tests for `darkmatter`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[cfg(target_os = "macos")]
#[path = "../image_test_support/mod.rs"]
mod image_test_support;

#[cfg(target_os = "macos")]
mod level3_image_painting;
