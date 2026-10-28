//! Level 1 integration tests for `darkmatter`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod coercion_core;
mod coercion_engine_parity;
mod coercion_translator_conformance;
mod function_call_binding;
#[cfg(feature = "effects-instrumentation")]
mod function_dispatch;
#[cfg(feature = "effects-instrumentation")]
mod pure_function_contracts;
#[cfg(feature = "effects-instrumentation")]
mod context_function_contracts;
#[cfg(feature = "effects-instrumentation")]
mod function_binding_registry;
#[cfg(feature = "effects-instrumentation")]
mod function_binding_conversions;
#[cfg(feature = "effects-instrumentation")]
mod function_catalog_parity;
mod compose_diagnostic_identity;
mod compose_expression_failure_contract;
mod context_construction_guard;
mod context_functions;
mod current_root_documentation_contract;
mod current_root_migration_guard;
mod dasherized_identifier_compose;
mod data_origin;
mod dependency_direction_guard;
mod directive_target_analysis;
mod empty_package_area;
mod entry_point_parity;
mod feature_review_incident;
mod file_tree_roots;
mod find_files_and_try_frontmatter;
// Helpers shared by more than one module are declared once, here.
#[path = "../image_test_support/mod.rs"]
mod image_test_support;
#[path = "../layout_matrix_support/mod.rs"]
mod layout_matrix_support;
#[path = "../request_support/mod.rs"]
mod request_support;
#[path = "../position_fixtures/mod.rs"]
mod position_fixtures;
#[path = "../union_fixtures/mod.rs"]
mod union_fixtures;
#[path = "../content_policy_fixtures/mod.rs"]
mod content_policy_fixtures;

mod absent_property_contract;
mod fm_namespace_contract;
mod hashing_guard;
mod host_globals_compose;
mod shell_child_environment;
mod schemas_effective_presence;
mod schemas_core_type_values;
mod schemas_runtime_parity;
mod side_effect_partial;
mod alias_lookup_once;
mod ambient_ctx_capture;
mod array_rendering_json;
mod as_block_error_registry;
mod backslash_escape_spans;
mod binding_contract;
mod base_schema_end_to_end;
mod benchmark_fixtures;
#[cfg(feature = "effects-instrumentation")]
mod blocking_policy;
mod blockquote_list_spacing;
mod clean_compose_equivalence;
mod clean_counters;
mod clean_parity;
mod clean_real_world_prompt;
mod clean_frontmatter_repair;
mod clean_rules;
mod code_link;
mod compose_phase6;
mod compose_reuse_phase5;
mod content_policy_compose_contract;
mod content_policy_disk_baseline;
mod content_policy_embedded_baseline;
mod content_policy_editor_schema;
mod cutover_reference;
mod debug_test;
#[cfg(windows)]
mod declined_path_transclusion;
mod diagnostic_positions;
mod disclosure_render_targets;
mod disclosure_transclusion_integration;
mod effects_integration;
mod error_snapshots;
mod expression_regression;
mod expression_strict_equality;
mod expression_type_scope;
mod host_functions;
#[cfg(feature = "effects-instrumentation")]
mod design_time_call_check;
#[cfg(feature = "effects-instrumentation")]
mod variable_type_projection;
#[cfg(feature = "effects-instrumentation")]
mod file_predicates;
#[cfg(feature = "effects-instrumentation")]
mod flow_sensitive_narrowing;
#[cfg(feature = "effects-instrumentation")]
mod compose_call_precheck;
#[cfg(feature = "effects-instrumentation")]
mod shared_diagnostics;
#[cfg(feature = "effects-instrumentation")]
mod report_diagnostics;
mod runtime_blocking;
mod frontmatter_coercion_engine;
mod frontmatter_surface_projection;
mod fs_capability;
mod git_context_integration;
mod global_catalog;
mod glob_consumers;
mod glob_implementation_guard;
mod horizontal_rule_integration;
mod horizontal_rule_snapshots;
mod html_inversion;
mod image_pixel_classification;
mod inline_document_text;
mod inline_envelope_prototype;
mod interpolation_literal_pipeline;
mod layout_matrix;
mod layout_snapshots;
mod lifecycle_boundary;
mod lifecycle_control_flow_spike;
mod lifecycle_event_binding;
mod lifecycle_generated_operands;
mod lifecycle_operand_validation;
mod lifecycle_run_protocol;
mod link_interpolation_integration;
mod literal_token;
mod match_keyword_input_matrix;
mod meta_schema_phase1;
mod meta_schema_phase3;
mod meta_schema_phase4;
mod meta_schema_phase5;
mod meta_schema_phase6;
mod meta_schema_reference_graph;
mod mermaid_automatic_mode;
mod mermaid_embedded_font;
mod mermaid_static_svg;
mod mermaid_syntax_highlighting;
mod mermaid_text_line_numbers;
mod meta_schema_repo_schemas;
mod missing_ctx_capture;
mod more_is_more_literals_and_indexes;
mod numeric_admission;
mod path_function_contracts;
mod path_lookup_guard;
mod nested_composition;
mod persistent_cache_disabled;
mod predict_conflicts;
mod prelude_exports;
mod prose_wrap_parity;
mod raw_schema_conversion;
mod recent_commits_shared_type;
mod reference_integration;
mod request_context_builder;
mod required_context;
mod render_comparison;
mod render_invariants;
mod render_tree_hr_snapshots;
mod render_tree_roundtrip;
mod request_context_epoch;
mod schema_phase_validation;
mod schema_quoting_safety;
mod schema_embedded_trigger_bundles;
mod schema_enhancements_matrix;
mod schema_grammar_descriptors;
mod schema_nested_template_deferral;
mod schema_roots;
mod schema_yaml_spellings;
mod schemas_conjunctive_lowering;
mod schemas_constraints_block_matrix;
mod schemas_convert_snapshots;
mod schemas_core_type_grammar;
mod schemas_detect_table;
mod schemas_grammar_proptest;
mod schemas_literal_expression;
mod schemas_named_import_errors;
mod schemas_named_type_merge;
mod schemas_required_count_matrix;
mod schemas_union_fixture_controls;
mod schemas_union_postfix;
mod schemas_union_report;
mod schemas_union_coerce;
mod schemas_union_rewrite;
mod schemas_union_clean;
mod schemas_union_completion;
mod schemas_union_typed_consumers;
mod schemas_union_schema_matrix;
mod schemas_source_projection;
mod schemas_validate_table;
mod schema_validation_timings;
mod schema_cache_validity;
mod scope_site_conformance;
mod semantic_results_never_persist;
mod serialized_code_neighbors;
mod recursive_schema_caller_files;
mod recursive_schema_golden;
mod recursive_schema_merge;
mod recursive_schema_types;
mod recursive_schema_validator_probe;
mod recursive_schema_reporting;
mod recursive_schema_clean;
mod recursive_schema_conversion;
mod recursive_schema_rewrite;
mod recursive_schema_completion;
mod recursive_schema_typed_consumers;
mod recursive_schema_frozen_fixture;
mod shared_acyclic_consumers;
mod shared_acyclic_definitions;
mod shared_acyclic_selection;
mod shipped_schema_corpus;
mod set_overlay_integration;
mod frontmatter_merge_order;
mod shell_block_integration;
mod shell_expansion_coordinates;
mod shell_result_values;
mod shell_template_policy;
mod shell_template_discovery;
mod shell_template_execution;
mod shell_template_guards;
mod shell_template_body_guards;
mod preflight_child_state_parity;
mod preflight_repository_sigils;
mod shell_probe_preflight;
mod span_compat;
mod strict_equality_composition;
mod style_features_baseline;
mod style_features_phase5;
mod style_frontmatter;
mod style_frontmatter_parity;
mod suggest_constraint_phase1;
mod suggest_constraint_phase2;
mod suggest_constraint_phase3;
mod suggest_constraint_phase4;
mod tab_indentation_repair_parity;
mod ternary_integration;
mod test_layout;
mod transcluded_shell_failure;
mod tree_features_characterization;
mod trigger_discovery_recovery;
mod trigger_envelope_claim;
mod trigger_input_matrix;
mod trigger_payload_load_errors;
mod trigger_match_grammar;
mod trigger_selection_parity;
mod trigger_spelling_guard;
mod type_scope_ownership_guard;
mod unknown_identifier_warning;
mod collection_analysis;
mod collection_execution;
#[cfg(feature = "effects-instrumentation")]
mod collection_effects;
mod url_root_identity;
mod yaml_block_parity;
mod lifecycle_passive_compose;
mod lifecycle_schema_generation;
mod lifecycle_schema_parity;
