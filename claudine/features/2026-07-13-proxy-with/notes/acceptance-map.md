# Acceptance map — `proxy.with` and canonical document handoffs

The Phase 13 sign-off artifact required by validation checkpoint 13: every
acceptance criterion in [`../spec.md`](../spec.md) mapped to at least one named
test that exists in the tree.

Written 2026-07-17 during Phase 13; updated after review-3 findings 1-5,
review-5 findings 1-6, and **review-6 findings 1-5 (current)**.

## Status: 28 of 30 complete, 2 partial

| | Count | Which |
|---|---:|---|
| ✅ Complete | 28 | all except the two below |
| ⚠️ Partial | 2 | **AC 10** — complete on the composition commands, not on the in-harness fallback; **AC 15** — one of nine compatibility facets is reachable end-to-end |
| ⛔ Blocked | 0 | — |

The two partials are **scoped, not headline** gaps, and each is stated in full in
its own row. Nothing below claims a criterion is proven at a level it is not.

### What review-6 changed

- **Finding 1 — dry run is now genuinely side-effect-free.** The seam moved out
  of `runner.rs` (where it sat *after* `initialize` had already fired) up into
  `wrap/composition/pipeline.rs::execute_composition_request_inner_with_guard`,
  landing after `resolve_selection_and_launch` and before
  `prepare_environment_and_mcp`. `--dry-run` now fires **no** lifecycle event and
  performs no filesystem side effect of its own, matching `spec.md:85-87`.
- **Finding 2 — resume compatibility is checked against a rebuilt bundle.**
  `rebuild_launch_env` was extracted in
  `harness_orch/loop_control/target_launch.rs`, and
  `materialize_attempt_prompt_phase` now rebuilds the launch env from the freshly
  materialized document at **every** retry/resume fresh-read boundary rather than
  re-applying an adoption-time snapshot. The dead `target_env_overrides`
  coordinator field was removed. This makes the AC 15 refusal reachable for
  `model` — see the AC 15 row for the eight facets it does **not** make reachable.
- **Finding 3 — the AC 9/AC 10 L2 matrix is complete.** A pure verification gap;
  no production bug was found. Three rows added, including two provider-*switch*
  rows that pin the launch bundle and MCP injection.
- **Finding 4 — a real production fix.** `prepare_and_run_active_document`
  performed no schema pre-validation ahead of its pre-flight compose, so a proxied
  target rendered Darkmatter's raw `MarkdownError::SchemaValidationFailed` where
  the direct route rendered the typed `CompositionError`. That is an AC 28 / R10
  violation on **both** proxy routes, and it was found by the new three-route L2
  matrix rather than by review.
- **Finding 5 — this document,** plus `target_launch.rs` / `session_key.rs`
  module docs, `docs/topics/composition.md`, `docs/topics/lifecycle.md`, the
  Claudine skill, and `plan.md`.

### The surfaced coordinator versus the in-harness fallback

These are two different paths and their limits are **not** the same. Conflating
them was the substance of review-6 finding 5.

- **Surfaced coordinator (the composition commands).** `compose`,
  `inline-compose`, and `sequence` each carry a run ledger, so a handoff on any
  proxy route surfaces up to the command-owned coordinator, which re-prepares the
  target as a fresh document through
  `compose/prep.rs::prepare_and_run_active_document`. That **re-enters the
  production selection/MCP/argv pipeline**, so provider, profile/binary
  sub-selection, argv entrypoint and flags, MCP runtime injection, effective child
  environment, interactivity, structured-output mode, dispatch configuration,
  model, loop ownership, child CWD, and system-prompt delivery are all recomputed
  from the target's own frontmatter under explicit-CLI precedence. No launch facet
  is borrowed here.
- **In-harness fallback (the direct provider wrappers).** A handoff with no
  ledger to surface to — `claudine claude`, `claudine goose`, … — is adopted
  inside the harness by `target_launch.rs::rebuild_target_launch`, which is
  env-only (`AGENT`/`MODEL`/`YOLO` plus the early-binding context). Profile/binary
  sub-selection, the argv entrypoint, and MCP runtime injection are **not**
  re-selected on this path and stay as the invocation resolved them. They diverge
  only when a proxy changes the provider itself, which explicit CLI provider
  selection pins.

Every test name below was verified to exist at HEAD. Paths are relative to
`claudine/`.

## Conventions

| Mark | Meaning |
|---|---|
| ✅ | Mapped to at least one passing test |
| ⚠️ | Partially mapped — the row states exactly which part is proven and which is not |
| ⛔ | Blocked; no honest test can be written yet |

L1 tests live beside their production module; L2 tests are in `cli/tests/` and
run via `just test-l2`.

## The matrix harness

Phase 13's equivalence harness lives at the end of
`cli/tests/level2_lifecycle_control.rs`:

- `stage_equivalence_arm` — stages one arm. Both arms execute the **same file**
  (`target.md`); only the route to it differs. The direct arm invokes it, the
  routed arm invokes `doc.md` (`EQUIV_ROUTER`) and is handed off at
  `initialize`. Pointing the direct arm at the target file itself, rather than a
  copy under a second name, is what keeps document-path-derived facets
  comparable.
- `normalize_arm` — rewrites each arm's own workspace path and basename to
  placeholders. Each arm owns a separate tempdir, so path-derived facets carry a
  random component; without this the matrix would compare randomness.
- `equivalence_arms` — runs both arms and returns their normalized event logs.

Rows are `level2_lifecycle_equivalence_*`. A probe document stamps the facets
under comparison into `events.log` from its own lifecycle surfaces; the
assertion is that the two arms' logs are identical.

## Criteria

| # | Criterion (abbreviated) | | Tests |
|---:|---|:-:|---|
| 1 | Proxied target bootstrapped/prepared by the same canonical service | ✅ | `prepare::entry::tests::proxy_target_runs_the_same_stages_as_a_direct_document`; `prepare::service::tests::direct_and_proxy_entry_prepare_equivalent_documents`; L2 `level2_lifecycle_equivalence_probe_matches_direct_run` |
| 2 | Only the coordinator changes identity; harness returns `Proxy` | ✅ | `loop_control::tests::coordinator_adoption::adopt_commits_identity_and_discards_source_execution_state`; `loop_control::tests::proxy::dispatch_proxy_requests_a_handoff_without_touching_active_document` |
| 3 | All four proxy routes return one typed handoff, always consumed or rejected | ✅ | `composition_seams::every_production_proxy_route_carries_the_typed_handoff` (censuses all 5 endpoints); `looping::engine::tests::lifecycle_control::loop_initialize_proxy_hands_off_without_iterating`; `composition_seams::no_new_optional_proxy_target_channel` (baseline zero) |
| 4 | Evaluation produces a request; the coordinator alone resolves and commits | ✅ | `executor::tests::proxy_with_evaluation::a_handoff_assembles_into_an_evaluated_request_losslessly`; `coordinator::tests::commit_takes_its_path_from_the_approval_not_the_authored_string`; `looping::engine::tests::lifecycle_control::loop_initialize_proxy_defers_resolution_to_the_coordinator` |
| 5 | Clean proxy emits no synthetic source terminal/finalize; target owns closure | ✅ | `loop_control::tests::coordinator_adoption::a_committed_handoff_synthesizes_no_source_finalize`; `coordinator::tests::a_proxy_hands_off_source_ownership_but_a_retry_does_not`; L2 `level2_lifecycle_proxy_target_lifecycle_parse_failure_fires_no_catch_events` |
| 6 | `compose`/`inline-compose`/sequence-step remain command state | ✅ | `loop_control::tests::coordinator_adoption::inline_closure_ownership_follows_the_adopted_target`; L2 `level2_lifecycle_proxy_inside_sequence_step_is_contained` (a proxy inside a sequence step runs its target once per step and neither advances nor restarts the step); L2 `level2_lifecycle_inline_compose_proxy_closure_rewrites_only_final_target` (only the final target is rewritten inline). **Dry run (review-6 finding 1):** L2 `level2_lifecycle_dry_run_fires_no_lifecycle_events_and_no_proxy_traversal` — a real-terminal dry run over a document whose `initialize`/`blocked`/`finalize` stacks all write to `events.log` asserts the file is *never created*, and that the router's own body is rendered rather than the proxy target's. Backed by L1 `wrap_compose_preflight::compose_dry_run_fires_no_lifecycle_side_effects` and `..::compose_dry_run_does_not_traverse_a_proxy_handoff` (both `cli/tests/`, no `level2_` prefix — L1 by tier, not L2 as this row previously said) |
| 7 | **Proxied target acquires its own loop; same iteration count as direct** | ✅ | L2 `level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run` (re-enabled, passes); L2 `level2_lifecycle_loop_router_initialize_proxy_is_honored`; L2 `level2_lifecycle_failure_proxy_to_looping_target_matches_direct_iterations` (terminal-route proxy to a looping target); L2 `level2_lifecycle_sequence_step_proxy_to_looping_target_owns_the_loop` (a sequence step's proxy acquires the target loop while staying contained in the step) |
| 8 | Body/frontmatter/lifecycle/schema/shell use the same stored `ComposeContext` | ✅ | `prepare::service::tests::the_prepared_document_stores_the_context_it_composed_against`; `..::current_is_not_a_prepare_time_fallback_for_the_stored_context`; `..::context_derivation_ignores_a_later_process_cwd_change`; `composition_seams::every_canonical_preparation_caller_supplies_explicit_context` |
| 9 | **`ctx.area`/`ctx.agent`/`ctx.model`, `env.AGENT`/`env.MODEL` match direct** | ✅ | All five facets × all three surfaces (prompt body, effective frontmatter, lifecycle): L2 `level2_lifecycle_equivalence_ac9_context_facets_match_direct_run` (no CLI provider flag, so agent/model are target-owned; the router authors no `model:`, so a borrowed launch bundle fails the fixture check before the arms are compared). Signal order, computed properties, `ctx.os`: L2 `level2_lifecycle_equivalence_probe_matches_direct_run`; `level2_lifecycle_initialize_proxy_target_resolves_ctx_not_in_source`. `env.MODEL` under a pinned provider: L2 `level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` |
| 10 | **Provider/model/MCP/argv/env/CWD/system prompt recalculated per target** | ⚠️ | Target-driven rows (no CLI provider flag, so the selection asserted on can only come from the target's frontmatter): `level2_lifecycle_equivalence_target_authored_provider_matches_direct_run` (provider, plus explicit-CLI precedence); `level2_lifecycle_equivalence_target_launch_bundle_matches_direct_run` (profile/binary, entrypoint, argv flags, effective child environment, interactivity, structured-output mode, dispatch/correlation configuration — router `goose` → target `codex`, and the router's stub must never launch); `level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run` (MCP runtime injection under a provider switch, router `codex` → target `gemini`, with the server set selected by the target's own prompt tag against an empty-defaults catalog). Invocation-level rows: `level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` (model→env); `level2_lifecycle_equivalence_child_cwd_matches_direct_run` (child CWD); `level2_lifecycle_equivalence_cli_system_prompt_survives_the_proxy` (system-prompt delivery); `level2_lifecycle_sequence_step_proxy_rebuilds_target_launch_bundle` (full bundle in a step); `level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run` (workspace/CWD anchor); `level2_lifecycle_equivalence_stdout_stderr_routing_matches_direct_run` (output routing). **Why partial:** every facet named by the criterion is recalculated, and proven at L2, on the **surfaced command coordinator** — the path all three composition commands take. The **in-harness fallback** used by the direct provider wrappers does not re-select profile/binary sub-selection, the argv entrypoint, or MCP runtime injection (`target_launch.rs::rebuild_target_launch` is env-only). Those three diverge on that path only under a provider *switch*, which explicit CLI selection pins. The criterion is unqualified as to path, so this is recorded partial rather than complete |
| 11 | Target `initialize` after the narrow gate, before full pre-flight; may chain | ✅ | `preflight::tests::initialize_scoped_audit_approves_only_the_initialize_command`; `prepare::entry::tests::only_a_new_active_document_emits_initialize`; L2 `level2_lifecycle_proxy_target_initialize_shell_is_gated_before_dispatch` |
| 12 | Full preparation rereads the stabilized target; no double `initialize` | ✅ | L2 `level2_lifecycle_proxy_target_rereads_after_initialize_mutation`; `loop_control::tests::overlay_layering::the_overlay_reaches_the_bootstrap_read_and_the_stabilized_reread` |
| 13 | Entry reasons obey the stage matrix; loops reuse the plan, retry/resume reread | ✅ | `prepare::entry::tests::stage_matrix_covers_every_entry_reason`; `..::retry_and_resume_fully_validate_but_a_loop_iteration_reuses_its_plan` |
| 14 | Retry refreshes canonically, fresh attempt, keeps overlay/provenance | ✅ | `coordinator::tests::retry_replaces_the_attempt_slice_and_drops_the_session`; `coordinator::tests::overlay_and_provenance_survive_a_canonical_refresh`; `..::retry_cannot_reset_its_own_budget_by_replacing_the_attempt` |
| 15 | **Resume retains only a session whose compatibility key matches; names facets** | ⚠️ | **Both directions live at L2, for `model` only:** `level2_lifecycle_resume_refuses_when_refresh_changes_model` (review-6 finding 2) drives a real refusal — a canonical resume refresh whose re-read document resolves a different `model:` is refused, the changed facet is named, and `retry` is recommended; `level2_lifecycle_resume_with_dropped_launch_flag_stays_compatible` proves the converse, that a deliberately dropped resume-only flag is *not* a false refusal. Together they pin both over- and under-eager guards. `model` became reachable because finding 2 made `rebuild_launch_env` run at every fresh-read boundary instead of once at adoption. **Projection (L1):** `harness_orch::session_key::tests` — `the_model_facet_reads_the_effective_child_env`, `changing_the_working_directory_projects_the_cwd_facet`, `toggling_yolo_projects_the_permission_facet`, `toggling_interactivity_projects_the_interactivity_facet`, `toggling_structured_output_projects_the_structured_facet`, `swapping_the_provider_changes_provider_binary_and_resume_protocol`, `a_changed_system_prompt_flag_projects_the_system_prompt_facet`, `a_changed_mcp_env_projects_the_mcp_facet`. Retain half: `coordinator::tests::resume_replaces_the_attempt_slice_but_retains_the_live_session`. **Why partial — read this before signing off:** the criterion says *complete* compatibility key. Only **1 of 9** facets is reachable end-to-end. The other eight (provider, profile/binary, resume protocol, system prompt, MCP set, workspace CWD, permission mode, interactivity, structured-output mode) are **projection-only (L1)** because they are argv-derived or CLI-resolved and cannot change across a same-document refresh on the current tree; a proxy that would change them re-prepares through the command coordinator, which opens a new session rather than resuming, so there is nothing for the key to refuse. `SessionCompatibilityKey::extra` is intentionally empty — no provider adapter has a precise resume identity to contribute. **Open question:** on refusal, `start` fires *before* the key comparison and `success`/`finalize` never fire — the refusal propagates as a hard error rather than routing through the lifecycle recovery stacks. The spec does not rule on whether `finalize` is owed here; flagged for owner decision, not silently accepted |
| 16 | Budgets persist across attempts; reset at proxy / next loop iteration | ✅ | `loop_control::tests::budget_scoping` — all 5, incl. `adoption_resets_budgets_while_the_invocation_wide_chain_keeps_growing` |
| 17 | Every fresh target gets full shell discovery/approval; approved == executed | ✅ | `loop_control::tests::shell_approval` — all 6, incl. `approved_bytes_equal_the_bytes_a_with_value_resolves_to`; L2 `level2_lifecycle_proxy_target_later_event_shell_is_audited_after_stabilization`; L2 `level2_lifecycle_proxy_shell_approved_bytes_equal_executed_bytes` (approved bytes equal executed bytes end-to-end) |
| 18 | Key/value proxy accepts optional mapping `with:` with static string keys | ✅ | `action_shape_control::{proxy_with_omitted_yields_empty_overlay, proxy_with_empty_mapping_equals_omission, static_keys_with_punctuation_are_accepted, rejects_dynamic_proxy_with_key}` |
| 19 | Positional proxy stays valid; positional + sibling `with:` stays ambiguous | ✅ | `action_shape_control::{positional_proxy_yields_empty_overlay, positional_proxy_plus_sibling_with_stays_ambiguous}` |
| 20 | `with:` resolves once through DM2; types preserved; no raw span deferred | ✅ | `proxy_with_evaluation::{whole_value_span_preserves_bool_rather_than_stringifying, whole_value_spans_preserve_every_resolved_type, mixed_string_resolves_to_a_string, nested_strings_follow_the_same_interpolation_rule, a_raw_span_stored_in_frontmatter_never_reaches_the_overlay}`; `composition_seams::subtree_compose_baseline_holds_the_line` |
| 21 | Malformed/unknown/illegal interpolation aborts atomically | ✅ | `proxy_with_evaluation::{evaluation_is_atomic_across_the_whole_mapping, malformed_expression_and_unknown_function_raise, out_of_scope_err_in_a_with_value_is_rejected_before_the_event_fires, no_error_does_not_suppress_an_overlay_failure}` |
| 22 | Precedence target < `with:` < caller; shallow replace; null removes | ✅ | `overlay_layering::{overlay_beats_the_targets_authored_frontmatter, caller_set_override_beats_a_conflicting_with_key, overlay_object_replaces_rather_than_deep_merging, overlay_null_removes_the_targets_authored_property, a_caller_override_restores_a_key_the_overlay_removed}`; L2 `level2_lifecycle_proxy_with_overlay_loses_to_a_caller_set_and_beats_the_target` |
| 23 | Stored overlay is the immutable pre-schema input; deterministically reapplied | ✅ | `overlay_layering::{schema_coercion_shapes_effective_frontmatter_but_not_the_stored_overlay, refreshing_the_same_target_reapplies_the_same_overlay}` |
| 24 | `with:` can satisfy a schema requirement; invalid overlay fails pre-launch | ✅ | `overlay_layering::{with_satisfies_a_required_schema_property_the_target_does_not_author, an_invalid_overlay_fails_the_targets_schema_before_any_launch}` |
| 25 | Control-plane overlay values reparsed/validated; cannot bypass policy | ✅ | `overlay_layering::{a_control_plane_overlay_is_reparsed_by_the_target, a_shell_command_installed_by_the_overlay_stays_subject_to_target_side_policy, a_malformed_control_plane_overlay_fails_as_the_targets_own_parse_error}` |
| 26 | Overlay survives retry/resume/loop refresh; a downstream proxy replaces it | ✅ | Retry/resume/refresh: `overlay_layering::refreshing_the_same_target_reapplies_the_same_overlay`; `coordinator::tests::overlay_and_provenance_survive_a_canonical_refresh`. Replacement/forwarding: `overlay_layering::{a_second_hop_replaces_the_overlay_rather_than_merging_it, a_hop_that_omits_with_installs_an_empty_overlay_rather_than_forwarding}`; L2 `level2_lifecycle_proxy_three_document_chain_forwards_only_explicit_keys` (end-to-end explicit-vs-omitted forwarding across two hops). Loop ownership now moves to the adopted target (AC 7), so loop-refresh survival is reachable and asserted end-to-end: L2 `level2_lifecycle_proxy_with_overlay_survives_a_retry`, `..survives_a_loop_refresh`, `..survives_a_resume` |
| 27 | No source/target byte or hash change from using `with:` | ✅ | `overlay_layering::an_overlay_never_writes_to_disk`; L2 `level2_lifecycle_proxy_with_overlay_loses_to_a_caller_set_and_beats_the_target` (asserts both documents' bytes unchanged) |
| 28 | Same typed diagnostic identity across direct / initialize-proxy / recovery-proxy | ✅ | **L2 three-route matrix (review-6 finding 4):** `level2_lifecycle_diagnostic_matrix_{schema_failure,invalid_overlay,preparation_failure}_is_route_equivalent` — each fixture runs all three routes through the shipped binary in a real tmux pane and asserts exit status, rendered typed identity, styled block, source attribution, exactly-once rendering, proxy provenance, and byte-equal diagnostics across routes. These caught a real divergence: `prepare_and_run_active_document` had no schema pre-validation ahead of its pre-flight compose, so a proxied target rendered Darkmatter's raw `MarkdownError: schema validation failed` where the direct route rendered the typed `CompositionError: schema validation`. Backed by L1 `prepare::service::tests::a_schema_failure_has_one_typed_identity_across_every_entry`; `..::a_missing_required_property_is_typed_on_the_harness_route`; `..::an_invalid_optional_is_dropped_and_recorded_on_the_harness_route` |
| 29 | Failed handoff follows event-aware routing; no duplicate emission | ✅ | `lifecycle_ordering::{a_handoff_failure_after_the_terminal_event_still_runs_the_owed_finalize, a_handoff_failure_before_the_terminal_event_routes_blocked_then_finalize, a_handoff_failure_after_finalize_surfaces_without_re_emitting}`; `coordinator_adoption::adopt_rejects_a_missing_target_without_activating_it`; L2 (initialize-route refusal through the source's `blocked`/`finalize` with typed `err.*`, no target activation) `level2_lifecycle_initialize_proxy_missing_target_routes_source_blocked_finalize`, `level2_lifecycle_initialize_proxy_cycle_routes_source_blocked_finalize`, `level2_lifecycle_initialize_proxy_hop_limit_routes_source_blocked_finalize` |
| 30 | No overlay disclosure in status/tracing; new output uses `TerminalRenderable` | ✅ | **L2 pane assertion (review-3 finding 5):** `level2_lifecycle_proxy_overlay_value_is_not_disclosed_in_rendered_status` — a `with:` secret-shaped value is consumed by the target lifecycle (stamped to `events.log`) yet never appears on the rendered tmux pane, while the `report_proxy_handoff` status *does* render through the terminal component. Backed by L1 `coordinator::tests::{an_evaluated_request_debug_names_properties_but_never_values, a_committed_handoff_debug_names_properties_but_never_values, a_prepared_document_debug_never_prints_overlay_values, redaction_does_not_hide_overlay_values_from_the_code_that_needs_them}` and `composition_seams::no_ad_hoc_printing_on_a_transition_path` |

## Previously-blocked criteria — two resolved, two partial

Phase 13 recorded AC 7, AC 9 (launch half), AC 10, and AC 15 as blocked on the
R6 launch rebuild being unstarted. review-3 findings 1-3 opened that work and
review-5 findings 1-6 completed it on the **surfaced-coordinator** path, and
review-6 findings 2-3 closed the remaining verification gaps. AC 7 and AC 9 are
now complete; AC 10 and AC 15 are **partial**, for the scoped reasons recorded
below and in their rows (verified 2026-07-17, parallel self-spawn `just test-l2`):

### AC 7 — loop ownership (was Phase 10)

Loop recognition reruns for the adopted target: an `initialize` proxy is hoisted
to the command-owned coordinator, which commits the handoff and re-prepares the
target as a fresh document through `compose/prep.rs::prepare_and_run_active_document`,
so an adopted target receives the same document loop as direct invocation. The two reproductions are re-enabled and pass:
`level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run` (a
non-looping router that proxies to a looping target runs all three iterations)
and `level2_lifecycle_loop_router_initialize_proxy_is_honored` (a loop-owning
router's `initialize` proxy is honored, not refused). Findings 2-6 extend the
same ownership to the terminal and sequence-step routes
(`level2_lifecycle_failure_proxy_to_looping_target_matches_direct_iterations`,
`level2_lifecycle_sequence_step_proxy_to_looping_target_owns_the_loop`).

### AC 9 — resolved

Complete. review-6 finding 3 closed the last verification gap with
`level2_lifecycle_equivalence_ac9_context_facets_match_direct_run`, which stamps
**all five** named facets (`ctx.area`, `ctx.agent`, `ctx.model`, `env.AGENT`,
`env.MODEL`) across **all three** required surfaces (prompt body, effective
frontmatter, lifecycle) and compares the routed arm against the direct one. It
runs with no CLI provider flag, so agent and model can only come from the
target's own frontmatter. No production bug was found; the behavior already
worked.

### AC 10 — resolved on the surfaced coordinator, still partial on the fallback

review-6 finding 3 added the two provider-*switch* rows the earlier map admitted
were missing by construction:
`level2_lifecycle_equivalence_target_launch_bundle_matches_direct_run` (router
`goose` → target `codex`; asserts profile/binary, entrypoint, argv flags,
effective child environment, interactivity, structured mode, dispatch config, and
that the router's stub never launches) and
`level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run` (router
`codex` → target `gemini`; MCP runtime injection with the server set chosen by
the target's own prompt tag against an empty-defaults catalog). Together with the
existing model, child-CWD, system-prompt, cross-repo, and output-routing rows,
**every facet AC 10 names is now rebuilt and L2-verified on the surfaced
coordinator path.**

It stays ⚠️ only because the criterion is unqualified as to path and the
in-harness fallback (direct provider wrappers) still does not re-select
profile/binary, argv entrypoint, or MCP runtime injection. See
[the two paths](#the-surfaced-coordinator-versus-the-in-harness-fallback).

### AC 15 — reachable for `model`, projection-only for the rest

review-6 finding 2 made the refusal reachable rather than latent, by rebuilding
the launch env at every retry/resume fresh-read boundary instead of once at proxy
adoption. `level2_lifecycle_resume_refuses_when_refresh_changes_model` now drives
a live refusal end-to-end.

This is a genuine improvement and **not** a completion. One of nine compatibility
facets is reachable. The full reasoning, the eight projection-only facets, the
empty `SessionCompatibilityKey::extra`, and the open `finalize`-on-refusal
question are in the [AC 15 row](#criteria) — that row, not this section, is the
one to read before signing off.

## Cross-platform status — the plan's tasks 2 and 3 conflict with ratified policy

Phase 13 task 2 requires the matrix to "run on macOS, Windows, and Linux", and
task 3 requires it to run "in the repository's existing macOS, Windows, and Linux
CI coverage". **Neither is achievable as written, and the obstruction is repo
policy rather than an oversight in this feature.**

`docs/testing-strategy.md` → "Platform Coverage (CI)" ratifies:

| Concern | Linux | Windows | macOS |
|---|---|---|---|
| L2 (`test-l2`) | yes (tmux) | **skips (harness absent)** | opt-in |

The equivalence matrix is an L2 tmux matrix. `cli/tests/level2_lifecycle_control.rs`
is `#![cfg(unix)]` at the file level and its fake providers are `#!/bin/sh`
scripts, so the whole suite — not just the new rows — is Unix-only **by
construction**, and `require_level!` skips it cleanly where tmux is absent.

Two further facts, both verified:

- **`claudine-tests.yml` runs `ubuntu-latest` only.** It does not call the
  reusable `_area-ci.yml`, so claudine has no Windows or macOS test leg at all.
  `claudine-windows-ctrl-c.yml` is `windows-latest` but runs one specific L3
  test.
- ~~**No CI job runs `just test-l2` for claudine on any platform.**~~
  **Superseded — this is no longer true.** `claudine-tests.yml` now defines a
  dedicated `test-l2` job on `ubuntu-latest` that runs `just test-l2`. The
  observation held when Phase 13 wrote it and is kept for the record, struck
  through rather than deleted so the reasoning below stays readable.

So the matrix's CI result today is: **Linux, via the dedicated `test-l2` job** —
not "not run anywhere", as this section originally concluded. Windows and macOS
remain uncovered for L2 by ratified policy. Local macOS success is recorded
below, and the plan is right that it "is not cross-platform sign-off".

**Recommended resolution, for the owner.** These are alternatives, not steps:

1. **Wire claudine into `_area-ci.yml` with `l2: true`** (Linux). This is the
   sanctioned mechanism, gives the matrix a real CI leg, and matches the ratified
   policy. It is the smallest change that makes task 3 meaningful. It does not
   satisfy the letter of "Windows and macOS", because policy says L2 cannot run
   there.
2. **Amend the plan** to state the matrix is Linux-CI + macOS-opt-in, and that
   Windows coverage of proxy behavior is L1-only. This makes the plan agree with
   `docs/testing-strategy.md`.
3. **Rewrite the L2 suite to be cross-platform** (drop `#![cfg(unix)]`, replace
   the `sh` fakes with portable stubs, adopt a Windows-capable harness). This
   contradicts ratified policy ("Windows skips — harness absent") and is a
   package-area-wide change far outside this feature.

Phase 13 took none of these and recorded them for owner sign-off.

**review-3 finding 5 took option 2 — reconcile the plan with ratified policy.**
The spec's Test Strategy now states the equivalence matrix is a Unix (tmux) L2
suite that runs on Linux CI + macOS opt-in per `docs/testing-strategy.md`, with
Windows proxy coverage carried by the platform-neutral L1 suite. Option 1
(wiring `_area-ci.yml` with `l2: true`) is left as the owner-enablement path
rather than switched on here: that reusable job sets
`BISCUIT_TEST_LEVEL_REQUIRED=2` (a missing harness hard-fails) and does **not**
install the AI-CLI provider stubs `claudine-tests.yml` provisions, so turning it
on without validating the ubuntu tmux + provider-stub combination in CI would
produce exactly the red L2 leg review-3 warns against. Enabling it is a one-line
owner change once that combination is validated on a runner. Option 3 remains a
policy reversal outside this feature.

**Resolved since: the intent of option 1 has landed, by a different mechanism.**
`claudine-tests.yml` now carries a purpose-built `test-l2` job on
`ubuntu-latest`. That satisfies what option 1 was after — a real Linux CI leg for
the matrix — while avoiding the two hazards that made `_area-ci.yml l2: true`
unsafe to switch on: it does not set `BISCUIT_TEST_LEVEL_REQUIRED=2`, and it runs
in the workflow that already provisions the AI-CLI provider stubs. Options 1 and
2 are therefore both satisfied; option 3 remains a policy reversal outside this
feature. **This section's premise — that the matrix has no CI home — no longer
holds; it is retained for the reasoning, not the conclusion.**

## Recorded gate results (2026-07-18, macOS host, review-6 finding 5)

All three gates run from the `claudine/` package area, **full area, not a
subset** — unlike the review-3 record this replaces, which ran only the
`level2_lifecycle*` slice.

| Gate | Exit | Result |
|---|:-:|---|
| `just test` | **0** | `claudine-catalog-types` 21/21 · `claudine` 3527/3527 (7 skipped) · `claudine-contract` 47/47 (5 skipped) · `claudine-cli` 2044/2044 (193 skipped) · `claudine-gen` 152/152 (4 skipped) |
| `just test-l2` | **0** | **166 passed, 0 failed**, 2071 skipped (tiers/backends not available on this host), 2 flaky |
| `just lint` | **0** | clippy + fmt-check + error-transport / lifecycle-doc guards clean |

**The 2 L2 flakes were `level2_perf_tree_renders_styled_in_wezterm` and
`level2_wezterm_removed_key_renders_yaml_codeblock`** — both WezTerm-backend
capture tests, unrelated to proxy behavior, that failed `TRY 1` and passed on
retry. This is the documented WezTerm capture flakiness, not a regression from
this work; `retries` is the sanctioned backstop for it.

Every test this review's findings added or changed ran (did **not** skip) and
passed:

| Test | Finding |
|---|---|
| `level2_lifecycle_dry_run_fires_no_lifecycle_events_and_no_proxy_traversal` | 1 |
| `level2_lifecycle_resume_refuses_when_refresh_changes_model` | 2 |
| `level2_lifecycle_resume_with_dropped_launch_flag_stays_compatible` | 2 (converse) |
| `level2_lifecycle_equivalence_ac9_context_facets_match_direct_run` | 3 |
| `level2_lifecycle_equivalence_target_launch_bundle_matches_direct_run` | 3 |
| `level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run` | 3 |
| `level2_lifecycle_diagnostic_matrix_schema_failure_is_route_equivalent` | 4 |
| `level2_lifecycle_diagnostic_matrix_invalid_overlay_is_route_equivalent` | 4 |
| `level2_lifecycle_diagnostic_matrix_preparation_failure_is_route_equivalent` | 4 |

Still standing from review-3 finding 5: `.config/nextest.toml` gives
`package(claudine-cli) & test(/level2_/)` a `slow-timeout` of `30s × 3 = 90s`
(vs. the default `5s × 6 = 30s`), which sits *above* these tests' own 40s
internal wait-for-marker deadline. The earlier default sat below it and was the
concrete cause of the "timed out on all four attempts" symptom under host
saturation.

### CI status

These three rows were previously orphaned below the gate table (they rendered as
loose text, not table rows) and the Linux row was stale. Both corrected here.

| Platform | Status |
|---|---|
| CI (Linux) | **wired** — `.github/workflows/claudine-tests.yml` now has a dedicated `test-l2` job on `ubuntu-latest` running `just test-l2`. This supersedes the earlier "not wired / owner-enablement path" record: the matrix *does* have a real CI leg. It is a purpose-built job rather than `_area-ci.yml l2: true`, which sidesteps that reusable job's `BISCUIT_TEST_LEVEL_REQUIRED=2` hard-fail and its lack of AI-CLI provider stubs |
| CI (Windows) | not applicable — L2 skips by ratified policy; Windows proxy coverage is L1 |
| CI (macOS) | not applicable — compile-check only by ratified policy |

## Debt found by this phase — RESOLVED (review-3 finding 4)

**One surviving R5 ambient-context site**, found by the new corpus guard, not by
review: `sequence::phase1c::build_template_preflight_options` built its base
`ComposeContext` with the ambient argument-less `capture()`, so `ctx.*` in a
sequence step's template shell preflight resolved from the process CWD the
wrapper has already moved to the repo root. It patched one face of the hazard —
`with_file_ref_fallback_dir` anchored *file-ref* resolution on the launch area —
while leaving the *context* itself ambient.

**Fixed by review-3 finding 4.** The step now captures its early-binding context
once via `build_step_prepared_context`
(`capture_for_document(launch_cwd, &step_source.markdown)`, anchor precedence
launch-area → document dir → `"."`, never the process CWD) and reuses that one
snapshot through both the template shell preflight and the step's
`PrepareOptions.prepared_context`. The commands the audit discovers are therefore
the commands the step's execution expands, matching the compose path's single
`prepared_context`. `AMBIENT_CONTEXT_CAPTURE_BASELINE` is now empty; the guard
still stops any new ambient `capture()` from appearing. A CWD-anchoring
regression test (`preflight_context_anchors_on_launch_cwd_not_process_cwd`) pins
the behavior: with the process CWD moved to an unrelated git repo, the preflight
resolves `ctx.repo_root` to the launch CWD's repo, not the process CWD's.
