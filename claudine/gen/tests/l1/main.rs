//! Level 1 integration tests for `claudine-gen`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod agent_errors_check;
mod drift;
mod fixtures_provenance;
mod generate_ux;
mod pipeline;
mod registry_coverage;
mod signals_sidecar_mirror;
mod signals_validation;
mod steering_check;
mod test_layout;
mod vocabulary;
