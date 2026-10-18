//! Level 1 integration tests for `dmls`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod claudine_schema_discovery;
mod content_policy_baseline;
mod context_construction_guard;
mod core_schema_types;
mod entry_point_parity;
mod formatting_transaction;
mod full_index;
mod index_store_concurrency;
mod index_store_facts;
mod index_store_keys;
mod index_store_live;
mod index_store_open;
mod index_store_oracle;
mod index_store_requests;
mod index_store_sessions;
mod index_store_stdio;
mod index_store_sweep;
mod level1_graph_index;
mod level1_wiki;
mod lsp_session;
mod mapping_only_corpus;
mod named_type_references;
mod no_side_effects;
mod packaging_contract;
mod referenced_targets;
mod registry_preparation;
mod repository_contexts;
mod schema_roots_parity;
mod schema_yaml_spellings;
mod shell_suffixes;
mod standalone_stdio;
mod startup_log_lines;
mod stdio_subprocess;
mod strict_mode_recovery_spike;
mod style_markdown_parity;
mod suggest_constraint_phase1;
mod test_layout;
mod undeclared_property;
mod union_item_editor;
mod wiki_universe_equivalence;
mod workspace_file_list;
mod workspace_formatting_settings;
mod workspace_schema_scopes;
mod zed_extension_contract;
