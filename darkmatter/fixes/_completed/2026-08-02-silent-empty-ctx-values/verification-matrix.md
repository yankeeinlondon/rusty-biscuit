# Verification Matrix

Maps each of the spec's twelve verification items to the automated tests that
prove it. Paths are relative to `darkmatter/`. Every test listed runs in the L1
suite (`just test`). The CLI test is launched through `CliProcessFixture`.

Test locations:

- `missing_ctx_capture`: `lib/tests/missing_ctx_capture.rs`
- `ambient_ctx_capture`: `lib/tests/ambient_ctx_capture.rs`
- `request_context_epoch`: `lib/tests/request_context_epoch.rs`
- `checked`: `lib/src/markdown/compose/context/checked.rs`
- `authority`: `lib/src/markdown/compose/context/authority.rs`
- `conditions`: `lib/src/markdown/compose/conditions.rs`
- `ctx`: `lib/src/markdown/compose/expression/ctx.rs`

| # | Spec item | Tests | Landed |
|---|-----------|-------|--------|
| 1 | A caller-supplied context without the group fails with a typed error naming the variable, group, and source | `missing_ctx_capture::body_reference_to_an_uncaptured_group_names_variable_group_and_source` (asserts `SourceRef::OnDiskLine` file and line); Review 1 authored location: `missing_ctx_capture::a_body_failure_reports_its_authored_file_line_after_frontmatter` (CRLF, frontmatter, rendered file, line, and `>`-marked excerpt), `…::a_transcluded_child_failure_reports_the_child_file_and_line`, `…::a_generated_expression_is_not_reported_at_an_authored_line`, `interpolation::rewrite::tests::located_failure_*`, `inline::interpolation::tests::authored_line_*`; `missing_ctx_capture::the_regression_input_fails_instead_of_rendering_empty_values` (verbatim regression input, both `fail_fast` modes); `ambient_ctx_capture::caller_supplied_minimal_context_fails_instead_of_rendering_uncaptured_groups_empty`; `cli/tests/compose_interpolation.rs::test_compose_uncaptured_context_group_exits_nonzero_naming_variable_and_group` | Phase 3 |
| 2 | Same error through frontmatter interpolation, body interpolation, a condition, and `$()` branching | `missing_ctx_capture::frontmatter_interpolation_fails_in_whole_value_and_mixed_text_forms`, `…::frontmatter_interpolation_pass_two_fails`, `…::page_block_condition_fails_with_a_typed_cause`, `…::transclusion_condition_fails_with_a_typed_cause`, `…::frontmatter_shell_ternary_condition_and_branch_fail_with_a_typed_cause`, `…::a_transcluded_child_reading_an_uncaptured_group_fails_the_parent`; shared evaluator: `interpolation::fatality_characterization::fatality_matrix_is_locked` | Phase 3 |
| 3 | Captured null/empty renders with existing semantics and no missing-capture diagnostic | `checked::tests::captured_typed_null_and_empty_values_are_present`; `missing_ctx_capture::a_captured_group_without_evidence_renders_its_typed_projection` | Phases 2, 3 |
| 4 | Unavailable supplied evidence keeps `PartialRuntimeCapture` and is not "uncaptured" | `missing_ctx_capture::a_captured_group_without_evidence_renders_its_typed_projection` (asserts the typed empty projection, no missing-capture warning, and that the report's `Partial runtime capture` warnings equal the context's `PartialRuntimeCapture` diagnostics, once each); `ambient_ctx_capture::a_capture_without_supplied_evidence_projects_every_key_of_each_captured_group`. Child-only capture through the cache: `request_context_epoch::persistent_cache::a_child_only_partial_capture_warns_identically_on_cold_and_warm_runs` (same warning count and content cold and warm against one cache root); `request_context_epoch::a_run_local_hit_replays_a_child_only_partial_capture_warning` (a run-local hit reports the same warnings as `CacheAccessMode::Off`) | Phases 2, 3; review-1 |
| 5 | Unknown key produces only the unknown-context-variable diagnostic | `missing_ctx_capture::an_unknown_key_warns_once_and_is_not_a_missing_capture`; `checked::tests::an_unknown_key_keeps_the_unchecked_lookup` | Phases 2, 3 |
| 6 | Malformed projection omitting a cataloged key is an internal invariant violation | `checked::tests::a_captured_group_missing_a_cataloged_key_is_an_internal_invariant`, `checked::tests::composition_fails_on_a_malformed_projection`; standalone: `ctx::tests::a_capture_missing_a_cataloged_key_is_an_internal_invariant`; real captures project every key: `ambient_ctx_capture::a_repository_capture_projects_every_key_of_each_captured_group`, `…outside_any_repository…`, `…without_supplied_evidence…` | Phases 2, 3 |
| 7 | Ambient composition resolves a group first named by a local child and a nested child, with one captured value for every later read | `ambient_ctx_capture::ambient_child_first_reference_renders_the_full_capture_value`; `request_context_epoch::siblings_and_nested_children_read_one_capture_of_a_child_introduced_group`; `request_context_epoch::a_remote_child_first_naming_a_group_reads_the_request_capture`; `request_context_epoch::a_group_named_only_by_an_unreachable_child_is_never_captured`; `authority::tests::concurrent_sources_capture_a_group_once_and_read_one_projection` | Phase 4 |
| 8 | Caller-supplied minimal context fails for that child and performs no ambient fallback | `request_context_epoch::a_frozen_context_fails_when_a_child_first_names_a_group`; `authority::tests::a_frozen_authority_never_grows_the_request_context` | Phase 4 |
| 9 | A child-only context change invalidates the parent's cached output; cache hits keep the missing-group contract | `request_context_epoch::persistent_cache::a_changed_child_only_value_invalidates_the_cached_child`, `…::a_changed_grandchild_only_value_invalidates_the_cached_parent`, `…::a_persistent_entry_cannot_bypass_a_frozen_missing_capture` (Strict, Fallback, Optimistic, Forced) | Phase 4 |
| 10 | Removing the root upgrade or the transclusion handoff gives a named missing-group failure | Root half: `authority::tests::a_pipeline_without_the_root_extension_fails_with_the_named_group`. Handoff half: a logged mutation run in Phase 4 (6 of 7 epoch tests failed with `ContextNotCaptured { key: "repo_root", group: Repo }` naming the child) | Phase 4 |
| 11 | A graph naming no discovery-backed group captures none; no L1 test above 5 s | Constructor: `context::options::tests::new_captures_no_discovery_derived_group`. Graph: `request_context_epoch::a_graph_naming_no_discovery_backed_group_captures_none` (root, condition-gated child, nested child, unknown and literal `ctx.*`, cold and warm persistent cache). Timing: the Phase 5 `just test` run's slowest test took 3.4 s | Phase 5 |
| 12 | Standalone `evaluate_condition_against` captures only reached groups and keeps the unknown/invariant classification | Reached groups: `ctx::tests::checked_resolution_captures_only_the_groups_it_reaches`, `conditions::tests::shortcut_and_short_circuits_prevents_ctx_capture`, `…::shortcut_or_short_circuits_prevents_ctx_capture`, `…::shortcut_ternary_short_circuits_then_branch`, `…::shortcut_ternary_short_circuits_else_branch`, `…::shortcut_same_group_captured_only_once`, `…::shortcut_unknown_ctx_key_does_not_capture`. Classification: `conditions::tests::shortcut_lookup_reports_a_projection_invariant_failure`, `conditions::tests::evaluate_condition_against_keeps_unknown_names_falsy_and_skips_unreached_groups` (public entry point) | Phases 2, 5 |

## Mutation evidence

Each run below disabled one fix, recorded which tests failed, and then restored
and re-verified the source.

- **Phase 2:** broke the `os_version` projection. The completeness tests failed (#6).
- **Phase 3:** disabled five fixes in turn:
  - structural transclusion classification
  - body on-disk source
  - `defer_missing_runtime_context`
  - missing-capture-over-dynamic-shape
  - `upgraded_for`

  Each run failed its named test (#1, #2).
- **Phase 4:** disabled three fixes in turn:
  - the closure check (#9)
  - the child handoff (#10)
  - per-document pre-flight extension
- **Phase 5:** added `{{ ctx.os }}` to the nested child of the #11 graph test. It
  failed with `cold: a group was captured for a graph that names none`.
- **Review-1 (#4):** replaced the transclusion engine's merge of the cached
  child report (`report.merge(cached.report.clone())`) with a discard. Both new
  cache tests failed; restored and re-verified.
