//! Level 1 integration tests for `simplified-schema`: one test binary.
//!
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod coercion_core;
mod context_construction_guard;
mod dependency_direction_guard;
mod expression_syntax;
mod foundations;
mod located_schema_errors;
mod path_lookup_guard;
mod resolution_unavailable;
mod root_union_assertions;
mod schema_file_selection;
mod schema_files;
mod schema_object;
mod schema_target_classification;
mod shipped_corpus;
mod test_layout;
mod validation_contract;
