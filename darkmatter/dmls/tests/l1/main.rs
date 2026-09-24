//! Level 1 integration tests for `dmls`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod level1_graph_index;
mod level1_wiki;
mod lsp_session;
mod mapping_only_corpus;
mod no_side_effects;
mod packaging_contract;
mod stdio_subprocess;
mod strict_mode_recovery_spike;
mod suggest_constraint_phase1;
mod test_layout;
mod zed_extension_contract;
