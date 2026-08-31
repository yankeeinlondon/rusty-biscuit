# Phase 5 requirement-to-test map

Before changing implementation code, the twelve mandatory merged seams were
mapped to the public results below. Eleven seams already had merged-tree
coverage at their required boundary. `MM-S10` was the only uncovered
interaction: the existing parallel-group tests did not route a prompt task
through a proxy before the target failed. Running the mapped tests then exposed
the caller eager-file provenance regression fixed in this phase.

| Seam | Public result and dependent state | Concrete targeted evidence |
|---|---|---|
| MM-S01 | The exact bare proxy input resolves repository-first. A missing input retains repository/source candidate order, probe status, source, code, message, and detail across terminal, `err.*`, and JSON snapshot projections. | `level2_implicit_reference_resolves_repository_first_in_tmux`; `level2_implicit_no_match_lists_two_ordered_candidates_in_tmux`; `diagnostic_projection_parity_across_render_err_and_snapshot`; `snapshot_round_trips_every_facet_detail_message_and_cause` |
| MM-S02 | The exact explicit-relative input has one source-local candidate and never falls back; a child derives its own source/repository context while invocation state remains fixed. POSIX and Windows separators are covered. | `level2_explicit_reference_stays_source_relative_and_fails_in_tmux`; `explicit_relative_yields_one_base_candidate_and_never_falls_back`; `windows_backslash_explicit_relative`; `level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run`; `child_document_context_reanchors_without_moving_launch` |
| MM-S03 | `proxy.with` enters one sequence step, the target owns its loop, and each step advances once while JIT state/output remain sequence-owned. | `level2_lifecycle_sequence_step_proxy_to_looping_target_owns_the_loop`; `a_later_step_composes_the_previous_steps_output`; `the_reserved_overlay_outranks_a_runtime_mutation` |
| MM-S04 | A target schema failure has direct/proxy diagnostic parity; native and quoted/coerced YAML values take the same staged route; target `initialize`, `blocked`, and `finalize` are owed exactly once. | `level2_lifecycle_diagnostic_matrix_schema_failure_is_route_equivalent`; `level2_lifecycle_still_invalid_target_runs_initialize_and_closure_first`; `a_deferred_read_still_coerces_declared_types` |
| MM-S05 | Initialize-time and terminal-time handoff failures retain distinct triggering events, closure ownership, and typed identity without duplicate emission. | `a_handoff_failure_before_the_terminal_event_routes_blocked_then_finalize`; `a_handoff_failure_after_the_terminal_event_still_runs_the_owed_finalize`; `a_handoff_failure_after_finalize_surfaces_without_re_emitting`; `level2_lifecycle_initialize_proxy_missing_target_routes_source_blocked_finalize` |
| MM-S06 | Native overlay values survive retry, resume, and immediate loop refresh; an absent downstream `with` drops them, while explicit forwarding preserves only named keys; source and target bytes remain unchanged. | `level2_lifecycle_proxy_with_overlay_survives_a_retry`; `level2_lifecycle_proxy_with_overlay_survives_a_resume`; `level2_lifecycle_proxy_with_overlay_survives_a_loop_refresh`; `level2_lifecycle_proxy_three_document_chain_forwards_only_explicit_keys`; `an_overlay_never_writes_to_disk` |
| MM-S07 | Provider, model, MCP set/tag, permission, interactivity, and credential policy rebuild from the refreshed document; incompatible resume fails before spawn with named typed facets. | `level2_lifecycle_retry_rebuilds_mcp_injection_from_the_refreshed_body`; `level2_lifecycle_retry_readmits_credentials_the_refreshed_provider_admits`; `level2_lifecycle_retry_strips_credentials_the_refreshed_provider_rejects`; `level2_lifecycle_resume_refuses_when_refresh_changes_model`; `level2_lifecycle_resume_refuses_when_refresh_changes_provider`; `level2_lifecycle_resume_refuses_when_refresh_changes_mcp_server_set`; `level2_lifecycle_resume_refuses_when_refresh_changes_permission_mode` |
| MM-S08 | The compatibility key is projected from the effective child environment, provider/profile, CWD, permission, interactivity, structured-output, MCP, and system-prompt facets that the attempt launches; resume-only argv is excluded. | `session_key::tests` (`identical_inputs_produce_compatible_keys` through `a_changed_document_mcp_tag_projects_the_mcp_facet`); `level2_lifecycle_resume_with_dropped_launch_flag_stays_compatible` |
| MM-S09 | A handoff audits the exact overlay-resolved shell bytes that later execute; the source-relative/repository context used for approval is the one used for materialization. | `approved_bytes_equal_the_bytes_a_with_value_resolves_to`; `level2_lifecycle_proxy_shell_approved_bytes_equal_executed_bytes`; `level2_lifecycle_overlay_installed_initialize_shell_runs_the_approved_bytes` |
| MM-S10 | A failing proxied prompt target in a parallel group remains attributed to its task, preserves per-channel ordering and correct merged-pane attribution, lets siblings settle, merges surviving state in declaration order, and leaves no descendant. | New `level2_mega_merge_s10_parallel_proxy_failure_task_integrity`; channel split test `parallel_prompt_task_splits_data_and_status_across_channels`; supporting `a_failed_parallel_member_lets_its_siblings_finish`, `a_contested_key_in_a_parallel_group_warns_and_resolves_by_declaration_order`, `level3_sequence_ctrl_c_fans_out_to_parallel_children`, and native variants |
| MM-S11 | A target in another repository re-anchors authoring/nested resolution while the launched provider retains invocation-fixed child CWD; diagnostics and direct/proxy behavior distinguish those contexts. | `level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run`; `level2_lifecycle_proxy_target_keeps_invocation_child_cwd`; `child_document_context_reanchors_without_moving_launch` |
| MM-S12 | Dry-run renders static source selection while firing no lifecycle event, traversing no proxy, launching no provider, mutating no document, creating no side-effect file, injecting no MCP runtime, and disclosing no overlay value. | `level2_lifecycle_dry_run_fires_no_lifecycle_events_and_no_proxy_traversal`; `inline_compose_dry_run_leaves_file_unchanged_and_prints_prompt`; `codex_wrapper_mcp_dry_run_shows_cleaned_prompt_and_shadow_file`; overlay-redaction assertions in `dry_run` and `level2_lifecycle_proxy_overlay_value_is_not_disclosed_in_rendered_status` |

## Representation, error, corpus, and persistence coverage

- Bare and explicit-relative references include the original motivating path,
  missing targets, ordered missing/error probes, POSIX paths, Windows
  backslashes, and rooted-magic rejection.
- Overlay/schema coverage includes missing and present values, native YAML
  bool/number/null/object/array values, quoted strings, coercible values, and
  invalid values that abort before launch.
- The passive `shipped_prompt_corpus_parses_frontmatter` test covers every
  shipped Markdown prompt/config artifact. The real-artifact
  `level2_lifecycle_shipped_implement_route_matches_direct_run` test follows
  the normal CLI invocation path.
- Diagnostic persistence performs serialize/read/deserialize/write/read
  round trips. Proxy overlays are intentionally transient; their equivalent
  repeated-read contract is exercised across retry, resume, and loop refresh,
  with source/target bytes asserted unchanged.

## Targeted tests added in Phase 5

- `level2_mega_merge_s10_parallel_proxy_failure_task_integrity` — the missing
  real-terminal combined seam for a proxied target failure inside a parallel
  sequence group.
- `eager_file_set_override_keeps_launch_area_across_target_repository_boundary`
  — quoted/unquoted eager-file schemas with the exact `spec.md` caller input
  across a repository boundary.
- `pre_validate_schema_defers_caller_eager_file_to_canonical_preparation` —
  the Claudine-side launch-boundary deferral required by that same input.
- `pre_validate_schema_keeps_unresolved_caller_file_partials_interactive` —
  the exact `everywhere` partial as scalar `file`, scalar-to-`file[]`, and
  array `file[]`, ensuring invalid inputs retain the interactive typed error.
