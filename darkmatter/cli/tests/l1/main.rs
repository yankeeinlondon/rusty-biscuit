//! Level 1 integration tests for `darkmatter-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;
// Shared with the darkmatter lib and DMLS suites; declared in `Cargo.toml`
// `source-inputs`, and each test that uses it spells it with `include_str!`.
#[path = "../../../lib/tests/union_fixtures/mod.rs"]
mod union_fixtures;
#[path = "../../../lib/tests/position_fixtures/mod.rs"]
mod position_fixtures;
#[path = "../../../lib/tests/content_policy_fixtures/mod.rs"]
mod content_policy_fixtures;
// Only its bundles are used here; the module allows its own dead code.
#[path = "../../../lib/tests/host_fixtures/mod.rs"]
mod host_fixtures;

mod approval_prompt;
mod claudine_schema_bundle;
mod collection_blocks;
mod compose_collection_controls;
mod compose_combined_report;
mod clean;
mod clean_fidelity;
mod clean_frontmatter;
mod frontmatter_key_order;
mod clean_json;
mod clean_parity;
mod clean_rules;
mod clean_schema;
mod code_block;
mod host_registration;
mod compose_foreign_documents;
mod compose_host_scope;
mod host_runtime_scope;
mod compose_array_rendering;
mod compose_base_schema;
mod compose_basic;
mod entry_point_parity;
mod feature_flag;
mod mermaid_features;
mod compose_context_probes;
mod compose_dasherized_identifiers;
mod compose_diagnostic_identity;
mod compose_document_identity;
mod compose_expression_failures;
mod expression_recovery;
mod compose_icmp;
mod compose_interpolation;
mod compose_layout;
mod compose_lifecycle;
mod compose_loader_wiring;
mod compose_lazy_roots;
mod compose_literal_only;
mod compose_missing_target_recovery;
mod compose_nested_markdown;
mod compose_page_blocks;
mod compose_perf;
mod compose_refs_and_missing;
mod compose_report_diagnostics;
mod compose_data_output;
mod config_notices;
mod init_command;
mod link_titles;
mod compose_remote_caching;
mod compose_schema;
mod compose_schema_enhancements;
mod compose_schema_file_rewrite;
mod compose_scope_matrix;
mod compose_shell;
mod compose_shell_template;
mod compose_setters;
mod path_completion;
mod property_paths;
mod compose_state_set;
mod compose_terminal_detection;
mod compose_transclusion;
mod compose_file_interpolate;
mod compose_file_predicates;
mod compose_call_warnings;
mod compose_member_warnings;
mod compose_unknown_identifiers;
mod compose_value_provenance;
mod context_construction_guard;
mod delta;
mod docs_style_examples;
mod diagnostic_positions;
mod dynamic_schema_slot_template_types;
mod dynamic_schema_slots;
mod get_set_rm;
mod graph;
mod hash;
mod hash_directory;
mod hash_entry_precedence;
mod hash_kind_save_diff;
mod hashing_guard;
mod help;
mod invalid_trigger_warnings;
mod layout_alignment;
mod magic_root;
mod layout_fill;
mod layout_flags;
mod layout_style_frontmatter;
mod literal_glob_hint;
mod load_data;
mod md_process_fixture;
mod path_lookup_guard;
mod policy_verbs;
mod prose_escape_guard;
mod reference_bare_schema;
mod reference_report;
mod reference_report_baselines;
mod render_basic;
mod render_alias_parity;
mod render_dispatch;
mod render_style_acceptance;
mod render_style_carried_warnings;
mod render_style_files;
mod render_style_matrix;
mod render_style_wiring;
mod rm;
mod schema_about;
mod schema_detect;
mod schema_trigger_rename;
mod schema_triggers;
mod schema_union_matrix;
mod schema_union_matrix_cells;
mod schema_union_pipeline;
mod schema_validate;
mod schema_validate_timings;
mod schema_validate_baseline;
mod schema_validate_baseline_selection;
mod schema_robustness_matrix;
mod schema_target_recognition;
mod schema_validate_core_types;
mod schema_grammar_types;
mod schema_validate_shipped_artifacts;
mod schema_validate_phase;
mod schema_validate_summary;
mod schema_validate_yaml;
mod spawn_site_guard;
mod standalone_baseline;
mod output_commands;
mod output_envelope;
mod output_width_overrides;
mod test_layout;
mod editor_support;
mod editor_resolution;
mod editor_launcher;
mod editor_service;
mod compose_edit_action;
mod toc;
mod validate_refs;
mod compose_lifecycle_approval;
mod compose_lifecycle_refusals;
mod lifecycle_passive_surfaces;
mod lifecycle_schema_fallback;
mod schema_recursive;
mod yaml_serializer_guard;
mod schema_key_hints;
