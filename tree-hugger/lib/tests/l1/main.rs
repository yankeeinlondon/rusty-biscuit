//! Level 1 integration tests for `tree-hugger`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod adapter_tests;
mod cache_tests;
mod corpus_tests;
mod lint_diagnostics;
mod phase1_diagnostics;
mod phase6_neovim_query_reuse;
mod query_compile;
mod resolver_tests;
mod test_layout;
mod tree_file;
mod tree_package;
