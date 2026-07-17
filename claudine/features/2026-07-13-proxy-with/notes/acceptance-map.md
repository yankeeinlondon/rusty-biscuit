# Acceptance map — `proxy.with` and canonical document handoffs

The Phase 13 sign-off artifact required by validation checkpoint 13: every
acceptance criterion in [`../spec.md`](../spec.md) mapped to at least one named
test that exists in the tree.

Written 2026-07-17 during Phase 13. **Status: 26 of 30 criteria mapped to
passing tests; 4 are blocked on Phase 9's R6 launch rebuild** (transitively via
Phases 10 and 11). Checkpoint 13 is therefore **not passed**. The blocked set is
named precisely in [Blocked criteria](#blocked-criteria) rather than mapped to a
test that does not assert what the criterion claims.

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
| 6 | `compose`/`inline-compose`/sequence-step remain command state | ✅ | `loop_control::tests::coordinator_adoption::inline_closure_ownership_follows_the_adopted_target`; L2 `wrap_compose_preflight::compose_dry_run_does_not_traverse_a_proxy_handoff` |
| 7 | **Proxied target acquires its own loop; same iteration count as direct** | ⛔ | `level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run` — **`#[ignore]`d**, Phase 10 |
| 8 | Body/frontmatter/lifecycle/schema/shell use the same stored `ComposeContext` | ✅ | `prepare::service::tests::the_prepared_document_stores_the_context_it_composed_against`; `..::current_is_not_a_prepare_time_fallback_for_the_stored_context`; `..::context_derivation_ignores_a_later_process_cwd_change`; `composition_seams::every_canonical_preparation_caller_supplies_explicit_context` |
| 9 | **`ctx.area`/`ctx.agent`/`ctx.model`, `env.AGENT`/`env.MODEL` match direct** | ⚠️ | `ctx.area`, `ctx.os`: L2 `level2_lifecycle_equivalence_probe_matches_direct_run`; `level2_lifecycle_initialize_proxy_target_resolves_ctx_not_in_source`. **`ctx.agent`/`ctx.model`/`env.MODEL`: `level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` — `#[ignore]`d, R6** |
| 10 | **Provider/model/MCP/argv/env/CWD/system prompt recalculated per target** | ⛔ | `level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` — **`#[ignore]`d**, R6 |
| 11 | Target `initialize` after the narrow gate, before full pre-flight; may chain | ✅ | `preflight::tests::initialize_scoped_audit_approves_only_the_initialize_command`; `prepare::entry::tests::only_a_new_active_document_emits_initialize`; L2 `level2_lifecycle_proxy_target_initialize_shell_is_gated_before_dispatch` |
| 12 | Full preparation rereads the stabilized target; no double `initialize` | ✅ | L2 `level2_lifecycle_proxy_target_rereads_after_initialize_mutation`; `loop_control::tests::overlay_layering::the_overlay_reaches_the_bootstrap_read_and_the_stabilized_reread` |
| 13 | Entry reasons obey the stage matrix; loops reuse the plan, retry/resume reread | ✅ | `prepare::entry::tests::stage_matrix_covers_every_entry_reason`; `..::retry_and_resume_fully_validate_but_a_loop_iteration_reuses_its_plan` |
| 14 | Retry refreshes canonically, fresh attempt, keeps overlay/provenance | ✅ | `coordinator::tests::retry_replaces_the_attempt_slice_and_drops_the_session`; `coordinator::tests::overlay_and_provenance_survive_a_canonical_refresh`; `..::retry_cannot_reset_its_own_budget_by_replacing_the_attempt` |
| 15 | **Resume retains only a session whose compatibility key matches; names facets** | ⛔ | Session compatibility key does not exist — Phase 11 tasks 1–4/6, blocked on R6. `coordinator::tests::resume_replaces_the_attempt_slice_but_retains_the_live_session` covers the retain half only |
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
| 26 | Overlay survives retry/resume/loop refresh; a downstream proxy replaces it | ⚠️ | Retry/resume/refresh: `overlay_layering::refreshing_the_same_target_reapplies_the_same_overlay`; `coordinator::tests::overlay_and_provenance_survive_a_canonical_refresh`. Replacement: `overlay_layering::{a_second_hop_replaces_the_overlay_rather_than_merging_it, a_hop_that_omits_with_installs_an_empty_overlay_rather_than_forwarding}`. **Loop-refresh survival is not end-to-end tested** — a proxied target has no loop until Phase 10 |
| 27 | No source/target byte or hash change from using `with:` | ✅ | `overlay_layering::an_overlay_never_writes_to_disk`; L2 `level2_lifecycle_proxy_with_overlay_loses_to_a_caller_set_and_beats_the_target` (asserts both documents' bytes unchanged) |
| 28 | Same typed diagnostic identity across direct / initialize-proxy / recovery-proxy | ✅ | `prepare::service::tests::a_schema_failure_has_one_typed_identity_across_every_entry`; `..::a_missing_required_property_is_typed_on_the_harness_route`; `..::an_invalid_optional_is_dropped_and_recorded_on_the_harness_route` |
| 29 | Failed handoff follows event-aware routing; no duplicate emission | ✅ | `lifecycle_ordering::{a_handoff_failure_after_the_terminal_event_still_runs_the_owed_finalize, a_handoff_failure_before_the_terminal_event_routes_blocked_then_finalize, a_handoff_failure_after_finalize_surfaces_without_re_emitting}`; `coordinator_adoption::adopt_rejects_a_missing_target_without_activating_it` |
| 30 | No overlay disclosure in status/tracing; new output uses `TerminalRenderable` | ✅ | `coordinator::tests::{an_evaluated_request_debug_names_properties_but_never_values, a_committed_handoff_debug_names_properties_but_never_values, a_prepared_document_debug_never_prints_overlay_values, redaction_does_not_hide_overlay_values_from_the_code_that_needs_them}`; `composition_seams::no_ad_hoc_printing_on_a_transition_path` |

## Blocked criteria

**All four trace to one root: Phase 9's R6 launch rebuild is unstarted.**
Re-verified at Phase 13: `LaunchInputs`, `TargetLaunchRebuilder`, and any
relaunch seam do not exist in the tree.

### AC 7 — loop ownership (Phase 10)

Loop recognition happens two frames above where a proxy target is adopted, so
giving a proxied target its own loop means inverting that ownership — the same
relaunch seam R6 needs. See plan checkpoint 10.

### AC 9 (launch half) and AC 10 — launch rebuild (R6)

Phase 13 **wrote the reproduction checkpoint 9 called unwritable**
(`level2_lifecycle_equivalence_target_pinned_model_matches_direct_run`,
`#[ignore]`d). Observed on today's tree:

```text
routed: ["sig=initialize", "provider-ran", "sig=success", "env.MODEL=",                       "sig=finalize"]
direct: ["sig=initialize", "provider-ran", "sig=success", "env.MODEL=llamacpp/probe-model-x", "sig=finalize"]
```

Its fixture check passes, so the target's own `model:` *does* reach the launch
environment when it is the invoked document. Reached through a proxy it resolves
empty: every launch decision is computed once, before the loop, from the router,
and `adopt` re-derives only prompt, frontmatter, lifecycle, harness plan,
timeouts, and closure plan.

The probe uses `model:` rather than `agent:` because the provider is pinned by
`--goose` on both arms — explicit CLI intent R6 must keep authoritative — so
`model:` is the one launch input free to move without a second provider stub or
an interactive selection prompt. The id rides Goose's declared `llamacpp`
namespace: frontmatter models are catalog-validated and an invalid one is
dropped silently, which would empty the facet for a reason unrelated to routing.

The remaining launch facets in AC 10 (MCP, argv, system prompt, child
environment, child CWD, interactivity, profile/binary, structured mode) have
**no matrix row**. One row reproducing the gap is sufficient to pin it; rows for
facets that all move together on the same rebuild would be redundant until that
rebuild exists. They should be added as R6 lands, using `equivalence_arms`.

### AC 15 — resume compatibility key (Phase 11)

The key's facets are pre-loop launch values, not document-derived ones, so no
facet can move and the diagnostic cannot be honestly written or tested. See plan
checkpoint 11.

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

Phase 13 took **none** of these: each is either an owner decision (1, 2) or a
policy reversal (3), and the plan's readiness rules do not authorize a phase to
make either unilaterally. Recorded here for checkpoint sign-off.

## Recorded gate results (2026-07-17, macOS host)

| Gate | Result |
|---|---|
| `just test` | see plan checkpoint 13 |
| `just test-l2` | 135 passed before this phase's rows; see checkpoint 13 for after |
| `just lint` | see checkpoint 13 |
| CI (Linux) | **not run — no `test-l2` leg exists for claudine** |
| CI (Windows) | **not applicable — L2 skips by ratified policy** |
| CI (macOS) | **not applicable — compile-check only by ratified policy** |

## Debt found by this phase

**One surviving R5 ambient-context site**, found by the new corpus guard, not by
review: `sequence::phase1c::build_template_preflight_options`
(`cli/src/commands/wrap/sequence/phase1c.rs:484`) builds its base
`ComposeContext` with the ambient argument-less `capture()`, so `ctx.*` in a
sequence step's template shell preflight resolves from the process CWD the
wrapper has already moved to the repo root.

Phase 5's task enumerated six capture sites and retired all six; this seventh was
never on that list, so it was never considered. Its own docblock names the hazard
and patches one face of it — `with_file_ref_fallback_dir` anchors *file-ref*
resolution on the launch area — while leaving the *context* ambient.

It is baselined in `AMBIENT_CONTEXT_CAPTURE_BASELINE` explicitly as **debt, not a
sanctioned owner**, so the guard still stops a second one. Fixing it means
choosing the correct anchor for a sequence step, which is a behavior change with
L2 blast radius that a guard phase should not make. Recommend folding it into
whatever picks up R6, which is already rebuilding per-document context.
