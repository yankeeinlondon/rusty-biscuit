//! Level 2 (`test-fixtures`) integration tests for `sniff-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout` rejects one.

#[path = "../common/mod.rs"]
mod common;

#[cfg(feature = "test-fixtures")]
mod level2_cicd_styling;
#[cfg(feature = "test-fixtures")]
mod level2_git_status_styling;
#[cfg(feature = "test-fixtures")]
mod level2_perf_tree_rendering;
#[cfg(feature = "test-fixtures")]
mod level2_recent_commits_rendering;
