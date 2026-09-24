//! Level 1 integration tests for `biscuit-file`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod completion_round_trip;
mod detailed_resolution;
mod finalized_reference_resolution;
mod implicit_relative;
mod parse_count;
mod precedence_flip;
mod reference_grammar;
mod repository_scope_catalog;
mod resolution_context;
mod round_trip;
mod span_compat;
mod test_layout;
mod yaml_corpus;
mod yaml_mutation;
mod yaml_safety;
