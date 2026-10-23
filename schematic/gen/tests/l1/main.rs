//! Level 1 integration tests for `schematic-gen`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod api_inventory;
mod artifact_drift;
mod asyncapi_import;
mod definition_baseline;
mod definition_bundles;
mod discriminated_unions;
mod e2e_generation;
mod endpoint_auth_output;
mod extension_reimport;
mod extension_round_trip;
mod fixed_header_export;
mod fresh_output_compile;
mod http_client;
mod import_overlay;
mod openapi_import_test;
mod openapi_strict_completeness;
mod output_validation;
mod pagination_validation;
mod partial_generation;
mod path_substitution;
mod postman_artifact_validation;
mod postman_golden;
mod postman_schema;
mod postman_var_consistency;
mod query_param_detection;
mod query_params_codegen;
mod response_adapters;
mod test_layout;
mod typed_path_output;
mod upstream_hygiene;
mod vocabulary_exports;
mod ws_codegen;
mod ws_validation;
