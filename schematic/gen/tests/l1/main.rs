//! Level 1 integration tests for `schematic-gen`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod artifact_drift;
mod e2e_generation;
mod http_client;
mod openapi_import_test;
mod openapi_strict_completeness;
mod path_substitution;
mod postman_artifact_validation;
mod postman_golden;
mod postman_schema;
mod postman_var_consistency;
mod query_param_detection;
mod query_params_codegen;
mod test_layout;
mod ws_codegen;
