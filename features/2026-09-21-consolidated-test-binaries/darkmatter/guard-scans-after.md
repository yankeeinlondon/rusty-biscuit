---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 4
package: darkmatter
host: macOS aarch64-apple-darwin
---

# Guard after-scan — `darkmatter`

The one path-sensitive guard that scans `darkmatter/lib/tests/` is the
repository archive-path guard (`tools/test-toolkit/tests/archive_path_guard.rs`).
Its population after the move is compared by path with
`../baseline/guard-scans.md` (acceptance 7). Each before path was mapped through
`../darkmatter-migration.json`, plus the two helper moves the manifest does not
carry: the nested `error_snapshots/` crate, which moved whole under `l1/`, and
`level2_render_tree_terminal/`, which moved beside `level2/level2_render_tree_terminal.rs`
(R2). Produced the Phase 1 way: a temporary probe test appended to the guard's
file, calling `test_toolkit::archive_guard::{collect_rust_files, is_eligible}`.
It was run once, and the file was restored with `git checkout --` and verified clean.

| Guard | Before | After | Extra after mapping | Missing after mapping | Eligibility changed |
|---|---:|---:|---|---|---|
| archive-path guard, `darkmatter/lib/tests/**` | 100 | 106 | the 5 new `main.rs` roots and `l1/test_layout.rs` | none | none (all `true`) |

The guard's own tests passed in the same run (`every_allowlist_entry_still_names_a_live_site`,
`no_archive_executed_target_bakes_in_a_producer_path`). Its summary line reads
`mode=full-tree files-checked=3488 ineligible=0 violations=5`, against the
baseline's 3,476. The difference is 12 files: the 4 `claudine-cli` roots (Phase 3),
these 6, and `tools/test-toolkit/src/test_layout.rs` + `test_layout/tests.rs`. The
5 violations are the baseline's 5 allow-listed live sites. `ALLOWED` names no file
under `darkmatter/lib/tests/`, before or after.

darkmatter has no spawn-site guard. That is `darkmatter-cli`'s (Phase 5).

## Population after (repository-relative; `path<TAB>eligible`)

```text
darkmatter/lib/tests/browser/browser_render.rs	true
darkmatter/lib/tests/browser/main.rs	true
darkmatter/lib/tests/image_test_support/mod.rs	true
darkmatter/lib/tests/l1/ambient_ctx_capture.rs	true
darkmatter/lib/tests/l1/array_rendering_json.rs	true
darkmatter/lib/tests/l1/as_block_error_registry.rs	true
darkmatter/lib/tests/l1/backslash_escape_spans.rs	true
darkmatter/lib/tests/l1/base_schema_end_to_end.rs	true
darkmatter/lib/tests/l1/benchmark_fixtures.rs	true
darkmatter/lib/tests/l1/blockquote_list_spacing.rs	true
darkmatter/lib/tests/l1/clean_counters.rs	true
darkmatter/lib/tests/l1/compose_phase6.rs	true
darkmatter/lib/tests/l1/compose_reuse_phase5.rs	true
darkmatter/lib/tests/l1/cutover_reference.rs	true
darkmatter/lib/tests/l1/debug_test.rs	true
darkmatter/lib/tests/l1/declined_path_transclusion.rs	true
darkmatter/lib/tests/l1/disclosure_render_targets.rs	true
darkmatter/lib/tests/l1/disclosure_transclusion_integration.rs	true
darkmatter/lib/tests/l1/effects_integration.rs	true
darkmatter/lib/tests/l1/error_snapshots/condition.rs	true
darkmatter/lib/tests/l1/error_snapshots/ctx_merge.rs	true
darkmatter/lib/tests/l1/error_snapshots/deferred_set.rs	true
darkmatter/lib/tests/l1/error_snapshots/editor.rs	true
darkmatter/lib/tests/l1/error_snapshots/file_tree.rs	true
darkmatter/lib/tests/l1/error_snapshots/helpers.rs	true
darkmatter/lib/tests/l1/error_snapshots/image_ref.rs	true
darkmatter/lib/tests/l1/error_snapshots/link.rs	true
darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs	true
darkmatter/lib/tests/l1/error_snapshots/mermaid_theme.rs	true
darkmatter/lib/tests/l1/error_snapshots/mod.rs	true
darkmatter/lib/tests/l1/error_snapshots/normalization.rs	true
darkmatter/lib/tests/l1/error_snapshots/page_block.rs	true
darkmatter/lib/tests/l1/error_snapshots/reference.rs	true
darkmatter/lib/tests/l1/error_snapshots/shell_expansion.rs	true
darkmatter/lib/tests/l1/error_snapshots/stylesheet.rs	true
darkmatter/lib/tests/l1/error_snapshots/toc_linking.rs	true
darkmatter/lib/tests/l1/error_snapshots/transclusion.rs	true
darkmatter/lib/tests/l1/expression_regression.rs	true
darkmatter/lib/tests/l1/frontmatter_surface_projection.rs	true
darkmatter/lib/tests/l1/git_context_integration.rs	true
darkmatter/lib/tests/l1/horizontal_rule_integration.rs	true
darkmatter/lib/tests/l1/horizontal_rule_snapshots.rs	true
darkmatter/lib/tests/l1/html_inversion.rs	true
darkmatter/lib/tests/l1/image_pixel_classification.rs	true
darkmatter/lib/tests/l1/inline_document_text.rs	true
darkmatter/lib/tests/l1/inline_envelope_prototype.rs	true
darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs	true
darkmatter/lib/tests/l1/layout_matrix.rs	true
darkmatter/lib/tests/l1/layout_snapshots.rs	true
darkmatter/lib/tests/l1/link_interpolation_integration.rs	true
darkmatter/lib/tests/l1/main.rs	true
darkmatter/lib/tests/l1/meta_schema_phase1.rs	true
darkmatter/lib/tests/l1/meta_schema_phase3.rs	true
darkmatter/lib/tests/l1/meta_schema_phase4.rs	true
darkmatter/lib/tests/l1/meta_schema_phase5.rs	true
darkmatter/lib/tests/l1/meta_schema_phase6.rs	true
darkmatter/lib/tests/l1/meta_schema_reference_graph.rs	true
darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs	true
darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs	true
darkmatter/lib/tests/l1/predict_conflicts.rs	true
darkmatter/lib/tests/l1/prelude_exports.rs	true
darkmatter/lib/tests/l1/prose_wrap_parity.rs	true
darkmatter/lib/tests/l1/reference_integration.rs	true
darkmatter/lib/tests/l1/render_comparison.rs	true
darkmatter/lib/tests/l1/render_invariants.rs	true
darkmatter/lib/tests/l1/render_tree_hr_snapshots.rs	true
darkmatter/lib/tests/l1/render_tree_roundtrip.rs	true
darkmatter/lib/tests/l1/schema_phase_validation.rs	true
darkmatter/lib/tests/l1/schema_quoting_safety.rs	true
darkmatter/lib/tests/l1/schemas_convert_snapshots.rs	true
darkmatter/lib/tests/l1/schemas_detect_table.rs	true
darkmatter/lib/tests/l1/schemas_grammar_proptest.rs	true
darkmatter/lib/tests/l1/schemas_literal_expression.rs	true
darkmatter/lib/tests/l1/schemas_required_count_matrix.rs	true
darkmatter/lib/tests/l1/schemas_source_projection.rs	true
darkmatter/lib/tests/l1/schemas_validate_table.rs	true
darkmatter/lib/tests/l1/set_overlay_integration.rs	true
darkmatter/lib/tests/l1/shell_block_integration.rs	true
darkmatter/lib/tests/l1/shell_expansion_coordinates.rs	true
darkmatter/lib/tests/l1/span_compat.rs	true
darkmatter/lib/tests/l1/style_features_baseline.rs	true
darkmatter/lib/tests/l1/style_features_phase5.rs	true
darkmatter/lib/tests/l1/style_frontmatter.rs	true
darkmatter/lib/tests/l1/style_frontmatter_parity.rs	true
darkmatter/lib/tests/l1/suggest_constraint_phase1.rs	true
darkmatter/lib/tests/l1/suggest_constraint_phase2.rs	true
darkmatter/lib/tests/l1/suggest_constraint_phase3.rs	true
darkmatter/lib/tests/l1/suggest_constraint_phase4.rs	true
darkmatter/lib/tests/l1/ternary_integration.rs	true
darkmatter/lib/tests/l1/test_layout.rs	true
darkmatter/lib/tests/l1/tree_features_characterization.rs	true
darkmatter/lib/tests/l1/yaml_block_parity.rs	true
darkmatter/lib/tests/layout_matrix_support/mod.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal/basic_spans.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal/code_panel.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal/file_links.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal/images.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal/layout_policy.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal/public_entry_points.rs	true
darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs	true
darkmatter/lib/tests/level2/main.rs	true
darkmatter/lib/tests/level3-browser/level3_popover.rs	true
darkmatter/lib/tests/level3-browser/main.rs	true
darkmatter/lib/tests/level3-terminal/level3_image_painting.rs	true
darkmatter/lib/tests/level3-terminal/main.rs	true
```
