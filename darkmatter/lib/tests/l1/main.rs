//! Level 1 integration tests for `darkmatter`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

// Helpers shared by more than one module are declared once, here.
#[path = "../image_test_support/mod.rs"]
mod image_test_support;
#[path = "../layout_matrix_support/mod.rs"]
mod layout_matrix_support;

mod ambient_ctx_capture;
mod array_rendering_json;
mod as_block_error_registry;
mod backslash_escape_spans;
mod base_schema_end_to_end;
mod benchmark_fixtures;
mod blockquote_list_spacing;
mod clean_counters;
mod compose_phase6;
mod compose_reuse_phase5;
mod cutover_reference;
mod debug_test;
#[cfg(windows)]
mod declined_path_transclusion;
mod disclosure_render_targets;
mod disclosure_transclusion_integration;
mod effects_integration;
mod error_snapshots;
mod expression_regression;
mod frontmatter_surface_projection;
mod git_context_integration;
mod horizontal_rule_integration;
mod horizontal_rule_snapshots;
mod html_inversion;
mod image_pixel_classification;
mod inline_document_text;
mod inline_envelope_prototype;
mod interpolation_literal_pipeline;
mod layout_matrix;
mod layout_snapshots;
mod link_interpolation_integration;
mod meta_schema_phase1;
mod meta_schema_phase3;
mod meta_schema_phase4;
mod meta_schema_phase5;
mod meta_schema_phase6;
mod meta_schema_reference_graph;
mod meta_schema_repo_schemas;
mod more_is_more_literals_and_indexes;
mod predict_conflicts;
mod prelude_exports;
mod prose_wrap_parity;
mod reference_integration;
mod render_comparison;
mod render_invariants;
mod render_tree_hr_snapshots;
mod render_tree_roundtrip;
mod schema_phase_validation;
mod schema_quoting_safety;
mod schemas_convert_snapshots;
mod schemas_detect_table;
mod schemas_grammar_proptest;
mod schemas_literal_expression;
mod schemas_required_count_matrix;
mod schemas_source_projection;
mod schemas_validate_table;
mod set_overlay_integration;
mod shell_block_integration;
mod shell_expansion_coordinates;
mod span_compat;
mod style_features_baseline;
mod style_features_phase5;
mod style_frontmatter;
mod style_frontmatter_parity;
mod suggest_constraint_phase1;
mod suggest_constraint_phase2;
mod suggest_constraint_phase3;
mod suggest_constraint_phase4;
mod ternary_integration;
mod test_layout;
mod tree_features_characterization;
mod yaml_block_parity;
