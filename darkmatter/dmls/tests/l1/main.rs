//! Level 1 integration tests for `dmls`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod bench_index;
mod claudine_editor_parity;
mod claudine_schema_discovery;
mod content_policy_baseline;
mod context_construction_guard;
mod core_schema_types;
mod entry_point_parity;
mod call_check_agreement;
mod hashing_guard;
mod recursive_call_check;
mod recursive_schema_editor;
mod expression_call_diagnostics;
mod expression_scopes;
mod formatting_transaction;
mod frontmatter_response_contracts;
mod full_index_removal_guard;
mod host_function_diagnostics;
mod host_globals;
mod incremental_graph_equivalence;
mod index_build;
mod invalid_trigger_warnings;
mod index_store_concurrency;
mod index_store_facts;
mod index_store_keys;
mod index_store_live;
mod index_store_log_lines;
mod index_store_open;
mod index_store_oracle;
mod index_store_requests;
mod index_store_sessions;
mod index_store_stdio;
mod index_store_sweep;
mod level1_graph_index;
mod level1_wiki;
mod lifecycle_event_views;
mod lifecycle_host_profile;
mod lsp_session;
mod mapping_only_corpus;
mod named_type_references;
mod no_side_effects;
mod packaging_contract;
mod referenced_targets;
mod registry_observability;
mod registry_preparation;
mod repository_contexts;
mod schema_roots_parity;
mod schema_yaml_spellings;
mod searched_candidates;
mod shell_suffixes;
mod standalone_stdio;
mod startup_log_lines;
mod startup_preparation;
mod stdio_subprocess;
mod strict_mode_recovery_spike;
mod style_markdown_parity;
mod suggest_constraint_phase1;
mod test_layout;
mod type_scope_ownership_guard;
mod undeclared_property;
mod union_item_editor;
mod validation_timing_log;
mod wiki_universe_equivalence;
mod workspace_file_list;
mod workspace_formatting_settings;
mod workspace_schema_scopes;
mod zed_extension_contract;
