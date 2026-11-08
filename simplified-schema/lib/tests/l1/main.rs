//! Level 1 integration tests for `simplified-schema`: one test binary.
//!
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod claimed_values;
mod closed_roots;
mod coercion_core;
mod container_contracts;
mod context_construction_guard;
mod declaration_grammar;
mod declaration_narrowing;
mod dependency_direction_guard;
mod envelope_admission;
mod expression_regions;
mod expression_scanner;
mod expression_syntax;
mod foundations;
mod located_failures;
mod located_schema_errors;
mod non_null_properties;
mod path_lookup_guard;
mod property_paths;
mod resolution_unavailable;
mod root_union_assertions;
mod schema_file_selection;
mod schema_files;
mod scanner_corpus;
mod schema_object;
mod schema_target_classification;
mod schema_target_inspection;
mod setter_values;
mod slot_spans;
mod slot_templates;
mod shipped_corpus;
mod soft_validation_repair;
mod test_layout;
mod validation_contract;
mod yaml_schema_envelope;
