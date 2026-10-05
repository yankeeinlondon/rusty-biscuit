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
mod symbols_adapter;
mod symbols_coverage;
mod symbols_decl_text;
mod symbols_declarator_wrappers;
mod symbols_export;
mod symbols_field_lists;
mod symbols_fixtures;
mod symbols_function_locals;
mod symbols_header;
mod symbols_kinds;
mod symbols_member_wrappers;
mod symbols_query;
mod symbols_signatures;
mod symbols_static;
mod symbols_visibility;
mod test_layout;
mod tree_file;
mod tree_package;
