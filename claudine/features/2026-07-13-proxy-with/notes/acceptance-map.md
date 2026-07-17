# Acceptance map — `proxy.with` and canonical document handoffs

The Phase 13 sign-off artifact required by validation checkpoint 13: every
acceptance criterion in [`../spec.md`](../spec.md) mapped to at least one named
test that exists in the tree.

Written 2026-07-17 during Phase 13; **updated 2026-07-17 after review-3 findings
1-5.** **Status: 30 of 30 criteria mapped to passing tests.** The four criteria
Phase 13 recorded as blocked on the R6 launch rebuild (AC 7, AC 9/10, AC 15) are
now unblocked: findings 1-3 landed the target launch rebuild
(`harness_orch/loop_control/target_launch.rs`), moved loop ownership to the
adopted document, and added the session-compatibility key
(`harness_orch/session_key.rs`, 10 per-facet Level 1 tests). The two
previously-`#[ignore]`d reproductions are re-enabled and pass, and review-3
finding 5 added the missing matrix rows plus a real Level 2 pane assertion for
AC 30. See [Previously-blocked criteria](#previously-blocked-criteria-now-resolved).

Every test name below was verified to exist. Paths are relative to `claudine/`.

## Conventions

| Mark | Meaning |
|---|---|
| ✅ | Mapped to at least one passing test |
| ⚠️ | Partially mapped — one facet covered, another blocked |
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
| 6 | `compose`/`inline-compose`/sequence-step remain command state | ✅ | `loop_control::tests::coordinator_adoption::inline_closure_ownership_follows_the_adopted_target`; L2 `wrap_compose_preflight::compose_dry_run_does_not_traverse_a_proxy_handoff`; L2 `level2_lifecycle_proxy_inside_sequence_step_is_contained` (a proxy inside a sequence step runs its target once per step and neither advances nor restarts the step) |
| 7 | **Proxied target acquires its own loop; same iteration count as direct** | ✅ | L2 `level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run` (re-enabled, passes); L2 `level2_lifecycle_loop_router_initialize_proxy_is_honored` |
| 8 | Body/frontmatter/lifecycle/schema/shell use the same stored `ComposeContext` | ✅ | `prepare::service::tests::the_prepared_document_stores_the_context_it_composed_against`; `..::current_is_not_a_prepare_time_fallback_for_the_stored_context`; `..::context_derivation_ignores_a_later_process_cwd_change`; `composition_seams::every_canonical_preparation_caller_supplies_explicit_context` |
| 9 | **`ctx.area`/`ctx.agent`/`ctx.model`, `env.AGENT`/`env.MODEL` match direct** | ✅ | `ctx.area`, `ctx.os`: L2 `level2_lifecycle_equivalence_probe_matches_direct_run`; `level2_lifecycle_initialize_proxy_target_resolves_ctx_not_in_source`. `env.MODEL`: L2 `level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` (re-enabled, passes after the R6 rebuild) |
| 10 | **Provider/model/MCP/argv/env/CWD/system prompt recalculated per target** | ✅ | L2 `level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` (model→env launch facet); L2 `level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run` (workspace/CWD anchor across repos); L2 `level2_lifecycle_equivalence_stdout_stderr_routing_matches_direct_run` (output routing) |
| 11 | Target `initialize` after the narrow gate, before full pre-flight; may chain | ✅ | `preflight::tests::initialize_scoped_audit_approves_only_the_initialize_command`; `prepare::entry::tests::only_a_new_active_document_emits_initialize`; L2 `level2_lifecycle_proxy_target_initialize_shell_is_gated_before_dispatch` |
| 12 | Full preparation rereads the stabilized target; no double `initialize` | ✅ | L2 `level2_lifecycle_proxy_target_rereads_after_initialize_mutation`; `loop_control::tests::overlay_layering::the_overlay_reaches_the_bootstrap_read_and_the_stabilized_reread` |
| 13 | Entry reasons obey the stage matrix; loops reuse the plan, retry/resume reread | ✅ | `prepare::entry::tests::stage_matrix_covers_every_entry_reason`; `..::retry_and_resume_fully_validate_but_a_loop_iteration_reuses_its_plan` |
| 14 | Retry refreshes canonically, fresh attempt, keeps overlay/provenance | ✅ | `coordinator::tests::retry_replaces_the_attempt_slice_and_drops_the_session`; `coordinator::tests::overlay_and_provenance_survive_a_canonical_refresh`; `..::retry_cannot_reset_its_own_budget_by_replacing_the_attempt` |
| 15 | **Resume retains only a session whose compatibility key matches; names facets** | ✅ | `harness_orch::session_key::tests` — 10 per-facet tests (`a_model_env_overlay_projects_the_model_facet`, `changing_the_working_directory_projects_the_cwd_facet`, `toggling_yolo_projects_the_permission_facet`, `toggling_interactivity_projects_the_interactivity_facet`, `toggling_structured_output_projects_the_structured_facet`, `swapping_the_provider_changes_provider_binary_and_resume_protocol`, `a_changed_system_prompt_flag_projects_the_system_prompt_facet`, `a_changed_mcp_env_projects_the_mcp_facet`, …); `coordinator::tests::resume_replaces_the_attempt_slice_but_retains_the_live_session` covers the retain half |
| 16 | Budgets persist across attempts; reset at proxy / next loop iteration | ✅ | `loop_control::tests::budget_scoping` — all 5, incl. `adoption_resets_budgets_while_the_invocation_wide_chain_keeps_growing` |
| 17 | Every fresh target gets full shell discovery/approval; approved == executed | ✅ | `loop_control::tests::shell_approval` — all 6, incl. `approved_bytes_equal_the_bytes_a_with_value_resolves_to`; L2 `level2_lifecycle_proxy_target_later_event_shell_is_audited_after_stabilization` |
| 18 | Key/value proxy accepts optional mapping `with:` with static string keys | ✅ | `action_shape_control::{proxy_with_omitted_yields_empty_overlay, proxy_with_empty_mapping_equals_omission, static_keys_with_punctuation_are_accepted, rejects_dynamic_proxy_with_key}` |
| 19 | Positional proxy stays valid; positional + sibling `with:` stays ambiguous | ✅ | `action_shape_control::{positional_proxy_yields_empty_overlay, positional_proxy_plus_sibling_with_stays_ambiguous}` |
| 20 | `with:` resolves once through DM2; types preserved; no raw span deferred | ✅ | `proxy_with_evaluation::{whole_value_span_preserves_bool_rather_than_stringifying, whole_value_spans_preserve_every_resolved_type, mixed_string_resolves_to_a_string, nested_strings_follow_the_same_interpolation_rule, a_raw_span_stored_in_frontmatter_never_reaches_the_overlay}`; `composition_seams::subtree_compose_baseline_holds_the_line` |
| 21 | Malformed/unknown/illegal interpolation aborts atomically | ✅ | `proxy_with_evaluation::{evaluation_is_atomic_across_the_whole_mapping, malformed_expression_and_unknown_function_raise, out_of_scope_err_in_a_with_value_is_rejected_before_the_event_fires, no_error_does_not_suppress_an_overlay_failure}` |
| 22 | Precedence target < `with:` < caller; shallow replace; null removes | ✅ | `overlay_layering::{overlay_beats_the_targets_authored_frontmatter, caller_set_override_beats_a_conflicting_with_key, overlay_object_replaces_rather_than_deep_merging, overlay_null_removes_the_targets_authored_property, a_caller_override_restores_a_key_the_overlay_removed}`; L2 `level2_lifecycle_proxy_with_overlay_loses_to_a_caller_set_and_beats_the_target` |
| 23 | Stored overlay is the immutable pre-schema input; deterministically reapplied | ✅ | `overlay_layering::{schema_coercion_shapes_effective_frontmatter_but_not_the_stored_overlay, refreshing_the_same_target_reapplies_the_same_overlay}` |
| 24 | `with:` can satisfy a schema requirement; invalid overlay fails pre-launch | ✅ | `overlay_layering::{with_satisfies_a_required_schema_property_the_target_does_not_author, an_invalid_overlay_fails_the_targets_schema_before_any_launch}` |
| 25 | Control-plane overlay values reparsed/validated; cannot bypass policy | ✅ | `overlay_layering::{a_control_plane_overlay_is_reparsed_by_the_target, a_shell_command_installed_by_the_overlay_stays_subject_to_target_side_policy, a_malformed_control_plane_overlay_fails_as_the_targets_own_parse_error}` |
| 26 | Overlay survives retry/resume/loop refresh; a downstream proxy replaces it | ✅ | Retry/resume/refresh: `overlay_layering::refreshing_the_same_target_reapplies_the_same_overlay`; `coordinator::tests::overlay_and_provenance_survive_a_canonical_refresh`. Replacement/forwarding: `overlay_layering::{a_second_hop_replaces_the_overlay_rather_than_merging_it, a_hop_that_omits_with_installs_an_empty_overlay_rather_than_forwarding}`; L2 `level2_lifecycle_proxy_three_document_chain_forwards_only_explicit_keys` (end-to-end explicit-vs-omitted forwarding across two hops). Loop ownership now moves to the adopted target (AC 7), so loop-refresh survival is reachable |
| 27 | No source/target byte or hash change from using `with:` | ✅ | `overlay_layering::an_overlay_never_writes_to_disk`; L2 `level2_lifecycle_proxy_with_overlay_loses_to_a_caller_set_and_beats_the_target` (asserts both documents' bytes unchanged) |
| 28 | Same typed diagnostic identity across direct / initialize-proxy / recovery-proxy | ✅ | `prepare::service::tests::a_schema_failure_has_one_typed_identity_across_every_entry`; `..::a_missing_required_property_is_typed_on_the_harness_route`; `..::an_invalid_optional_is_dropped_and_recorded_on_the_harness_route` |
| 29 | Failed handoff follows event-aware routing; no duplicate emission | ✅ | `lifecycle_ordering::{a_handoff_failure_after_the_terminal_event_still_runs_the_owed_finalize, a_handoff_failure_before_the_terminal_event_routes_blocked_then_finalize, a_handoff_failure_after_finalize_surfaces_without_re_emitting}`; `coordinator_adoption::adopt_rejects_a_missing_target_without_activating_it` |
| 30 | No overlay disclosure in status/tracing; new output uses `TerminalRenderable` | ✅ | **L2 pane assertion (review-3 finding 5):** `level2_lifecycle_proxy_overlay_value_is_not_disclosed_in_rendered_status` — a `with:` secret-shaped value is consumed by the target lifecycle (stamped to `events.log`) yet never appears on the rendered tmux pane, while the `report_proxy_handoff` status *does* render through the terminal component. Backed by L1 `coordinator::tests::{an_evaluated_request_debug_names_properties_but_never_values, a_committed_handoff_debug_names_properties_but_never_values, a_prepared_document_debug_never_prints_overlay_values, redaction_does_not_hide_overlay_values_from_the_code_that_needs_them}` and `composition_seams::no_ad_hoc_printing_on_a_transition_path` |

## Previously-blocked criteria, now resolved

Phase 13 recorded AC 7, AC 9 (launch half), AC 10, and AC 15 as blocked on the
R6 launch rebuild being unstarted. review-3 findings 1-3 landed that work, and
the four are now mapped to passing tests (verified 2026-07-17, parallel
self-spawn `just test-l2`):

### AC 7 — loop ownership (was Phase 10)

The R7 loop-ownership move (`harness_orch/loop_control/target_launch.rs` +
`compose/prep.rs`) stabilizes initialize routing before loop recognition, so an
adopted target receives the same document loop as direct invocation. The two
reproductions are re-enabled and pass:
`level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run` (a
non-looping router that proxies to a looping target runs all three iterations)
and `level2_lifecycle_loop_router_initialize_proxy_is_honored` (a loop-owning
router's `initialize` proxy is honored, not refused).

### AC 9 (launch half) and AC 10 — launch rebuild (R6)

The R6 target launch rebuild (`harness_orch::loop_control::target_launch`)
recomputes the target's `model:` into the launch environment on hand-off, so
`level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` (re-enabled)
now resolves the same `env.MODEL` on the routed arm as on the direct arm.
review-3 finding 5 adds two further AC 10 launch-facet rows that move together on
the same rebuild:
`level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run`
(workspace/CWD anchor: a target in a different repository resolves a `file(...)`
schema reference against the launch area on both routes) and
`level2_lifecycle_equivalence_stdout_stderr_routing_matches_direct_run`
(stdout/stderr channel routing is route-independent).

### AC 15 — resume compatibility key (was Phase 11)

The `SessionCompatibilityKey` now exists (`harness_orch/session_key.rs`) and is
compared after resume refresh. `harness_orch::session_key::tests` proves each
launch facet projects into the key — model, CWD, permission (`yolo`),
interactivity, structured output, provider binary + resume protocol, system
prompt, and MCP — so a changed facet is detected and named before any provider
attempt.

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
- **No CI job runs `just test-l2` for claudine on any platform.** The only
  `test-l2` invocations in `.github/workflows/` are `test.yml` (sniff) and
  `_area-ci.yml`'s opt-in `l2` job, which claudine does not use.

So the matrix's CI result today is: **not run anywhere.** Local macOS success is
recorded below, and the plan itself says that "is not cross-platform sign-off".

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

## Recorded gate results (2026-07-17, macOS host, review-3 finding 5)

| Gate | Result |
|---|---|
| `cd claudine && just test` | **pass** (exit 0, all packages) |
| `just test-l2` (`level2_lifecycle_control` binary, `BISCUIT_L2_THREADS=6`) | **34 passed, 0 failed, 0 skipped** — includes the 5 new rows, the 3 previously-timing-out proxy tests, and the re-enabled equivalence/loop reproductions |
| `just test-l2` (`level2_lifecycle_{loop,dispatch,action_forms}`) | **18 passed, 0 failed** |
| `cd claudine && just lint` | **pass** (clippy + fmt-check + error-transport / lifecycle-doc guards clean) |

The full-area `just test-l2` (every L2 file) was not run end-to-end here; the
`level2_lifecycle*` + equivalence subset above is the faithful slice for the rows
this finding touches. Robustness change: `.config/nextest.toml` now gives
`package(claudine-cli) & test(/level2_/)` a `slow-timeout` of `30s × 3 = 90s`
(previously the default `5s × 6 = 30s`), which sits *below* these tests' own 40s
internal wait-for-marker deadline and was the concrete cause of the review's
"timed out on all four attempts" under host saturation.
| CI (Linux) | **not wired** — sanctioned mechanism (`_area-ci.yml l2: true`) documented as owner-enablement path; not switched on unvalidated (provider stubs + `BISCUIT_TEST_LEVEL_REQUIRED=2`) |
| CI (Windows) | **not applicable — L2 skips by ratified policy; Windows proxy coverage is L1** |
| CI (macOS) | **not applicable — compile-check only by ratified policy** |

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
