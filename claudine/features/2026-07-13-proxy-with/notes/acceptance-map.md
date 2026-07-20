# Acceptance map — `proxy.with` and canonical document handoffs

The Phase 13 sign-off artifact required by validation checkpoint 13: every
acceptance criterion in [`../spec.md`](../spec.md) mapped to at least one named
test that exists in the tree.

Written 2026-07-17 during Phase 13; updated after review-3 findings 1-5,
review-5 findings 1-6, review-6 findings 1-5, review-7 findings 1-4, and
**review-8 findings 1-5 (current)**.

## Status: 30 of 30 complete

The count below is **derived from the rows**, not asserted ahead of them: it is
the number of ✅ and ⚠️ marks in the [criteria matrix](#criteria). Review-7
finding 4 found the previous headline overstated, so it is now recomputed
whenever a row's mark changes.

| | Count | Which |
|---|---:|---|
| ✅ Complete | 30 | every criterion |
| ⚠️ Partial | 0 | — |
| ⛔ Blocked | 0 | — |

**AC 11 closed in review-8 finding 1.** It was partial because a document that
owned a `loop:` reached its schema verdict on iteration 1, before the loop
engine emitted `initialize` — the residue of the bug review-7 finding 1 fixed
everywhere else. The loop branch now threads the selected `SchemaStage` like the
single-execution branch does, and the L2 ordering matrix gained a looping arm on
all three routes plus a still-invalid converse. Review-7 finding 4's framing is
worth keeping even though the row closed: AC 11 described behavior a user could
reach and was not qualified as to path or document shape, so it was never
waivable as a scoped exception.

**AC 10 closed in review-7 finding 3.** It was partial only because the direct
provider wrappers adopted a hand-off against a borrowed launch bundle. They now
refuse it with a typed diagnostic, so no path borrows one — and the borrowed
bundle turned out to be unreachable anyway, for the reason recorded below. Its
closure is by **refusal**, not by an equivalence proof on that path; the row
says so.

Nothing below claims a criterion is proven at a level it is not.

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

### What review-7 changed

- **Finding 1 — the R4 stage order was restored.** Review-6 finding 4 fixed the
  cross-route diagnostic divergence by moving the proxied route onto the
  *caller's* already-early validation seam, which made the routes agree at the
  cost of validating a target before its own `initialize` could repair the
  document. The verdict is now a **stage of canonical preparation** rather than
  a decision each caller makes: `SchemaStage::{Validate, DeferToStabilizedReread}`
  (`lib/src/composition/prepare/service.rs`) selects who owns it, and
  `BootstrapStage::{Full, StabilizeOnly}`
  (`cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs`) selects
  which half of the staged boot runs. A deferred read withholds the verdict at
  **both** layers that can reach one — Darkmatter's compose-time stage and this
  crate's post-shell re-validation — so skipping one of the two cannot leak an
  early failure. Target `initialize` now precedes the verdict on the direct,
  initialize-proxy, and recovery-proxy routes — for a single-execution document;
  the looping shape was left uncovered and was closed by review-8 finding 1.
- **Finding 2 — the whole launch identity is rebuilt, not just the env.**
  `rebuild_launch_env` (review-6) still exists as the env-projection half, but
  the compatibility comparison no longer derives from it. `rebuild_launch_identity`
  now recomputes provider, profile/binary, resume protocol, interactivity,
  permission mode, structured-output mode, MCP tag set, and env from
  `(document, LaunchRebuildIntent)` at every retry/resume fresh-read boundary,
  and **both sides** of the comparison derive from it. Seven further facets
  moved from projection-only to a real end-to-end refusal, bringing the total to
  eight of ten.
- **Finding 3 — the wrapper hand-off is refused rather than borrowed.** See
  [the two paths](#the-surfaced-coordinator-versus-the-direct-provider-wrappers).
- **Finding 4 — this document.** The headline count was overstated (29/30
  against rows that did not support it) and AC 11 was marked complete against
  three tests, none of which asserted the order the criterion names. Every row
  below was re-derived from the tests that exist at this HEAD, and every test
  name cited anywhere in this document was re-verified. The count is now
  computed from the marks.

### What review-8 changed

- **Finding 1 — a looping document no longer outruns its own `initialize`.**
  `compose/prep.rs::execute_loop_or_single` threads the selected `SchemaStage`
  into `loop_prepare_options` and `build_and_run_loop`, so every read the loop
  route takes before the engine emits `initialize` — the seed compose and the
  iteration-1 compose — withholds the verdict when the deferred stage was
  chosen. `CompositionKind::prepare_with_schema` was deleted in favor of the
  stage-aware `prepare_staged`, leaving one way to reach a verdict. This closes
  AC 11, the last partial row.
- **Finding 2 — the rebuilt bundle is now the launch.** A new re-entrant,
  side-effect-free builder (`cli/src/commands/wrap/launch_plan.rs`,
  `build_launch_plan(inputs, facets)`) re-derives argv and the environment
  overlay from `(invocation-recorded inputs, refreshed document facets)`.
  `execute_attempt_phase` reads provider, profile, binary, argv, session mode,
  structured-output shape, permission mode, and MCP injection from that one
  bundle, which also feeds the compatibility key; the invocation-fixed launch
  state (`binary_path`, `base_args`, `use_structured`) is gone from the harness
  loop. A retry whose refresh moved a facet now *launches* under the refreshed
  plan instead of the invocation's.
- **Finding 3 — the last two AC 15 facets were resolved by ratification, not by
  deletion.** `workspace/child CWD` and `system-prompt content` were the two
  projection-only facets. Both were re-examined against the tree and found
  **structurally unreachable**: no document surface exists for either, and for
  the system prompt the one mutation a lifecycle stack could attempt (rewriting
  the discovered `system-prompt.md`) provably moves nothing. R8 now defines them
  as **immutable invocation inputs** and says why, AC 15 names them, the
  `target_launch` / `session_key` / `launch_plan` module docs agree, and a new
  L1 test holds the system-prompt claim. AC 15 is complete. See its row.
- **Finding 3 also split delivery from content.** System-prompt *delivery* is
  provider-shaped and rebuilt per provider by finding 2's builder, so it is not
  immutable — it moves exactly when the provider facet moves, which already
  refuses end-to-end. The spec now says that rather than lumping
  "delivery/content" together.
- **Finding 4 — this document, plus `docs/topics/composition.md`.** The
  composition topic still described the deleted in-harness adoption fallback and
  a resume-reachability split (`model` only, eight facets projection-only) that
  three findings had since moved. Both are now written against the tree.
- **Finding 5 — the wrapper's `mcp_enabled` matches its own flags.**
  `LaunchRebuildIntent.mcp_enabled` is derived from `--mcp` / `--mcp-use` on the
  direct-wrapper path instead of hardcoded `false`. This closes the second of
  the two [open items](#open-items-carried-out-of-review-7--both-closed-by-review-8) carried out of review-7.

### The surfaced coordinator versus the direct provider wrappers

These are two different paths and their limits are **not** the same. Conflating
them was the substance of review-6 finding 5; review-7 finding 3 then closed the
second path.

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
- **Direct provider wrappers (`claudine claude`, `claudine goose`, …).** These
  prepare no active document — the prompt comes from argv or stdin and the
  harness source is a provider *memory file* — so they own no coordinator that
  could re-enter that pipeline. A handoff there is now **refused** with
  `CompositionError::LifecycleProxyWithoutOwningCoordinator`, routed through the
  source's own terminal/`finalize` events (AC29) and rendered as a `StatusBlock`
  naming the target, the wrapper that cannot host it, and `claudine compose` as a
  command that can. There is no longer an in-harness adoption, and therefore no
  reduced launch path.

#### The borrowed-bundle divergence was latent, not live

Review-7 finding 3 states the borrowed bundle is "user-observable whenever a
wrapper handoff's target-owned launch configuration differs from the source."
**That premise does not hold at this HEAD**, and the correction matters for how
AC10 is scored.

The wrapper passthrough builds its lifecycle guard from
`LifecycleConfig::default()` (`wrapper_stages.rs::run_execution_stage`) and never
re-points it: the only `set_config` calls are in the staged proxy bootstrap,
which no passthrough run reaches. A memory file authoring
`initialize`/`start`/`failure`/`finalize` therefore fires **none** of them, so it
can never raise a `proxy` control, so the in-harness adoption arm was
unreachable. Verified end-to-end on the shipped binary by
`level2_lifecycle_wrapper_passthrough_raises_no_proxy_handoff`.

The fix therefore removes a *latent* reduced path and converts it into a loud
refusal. That is still the right change — R3 forbids the reduced path existing at
all, and the refusal is the diagnostic the spec's "Errors and Diagnostics"
section already names ("any supported transition returned without an owning
coordinator able to consume it"). But no shipped behavior changed for any
reachable input, and the L2 row is a **guard** rather than an equivalence proof:
it fails if lifecycle is ever wired into the passthrough without an owning
coordinator arriving with it.

**Every test name in this document — not only in the matrix below — was
re-verified against the tree at this HEAD on 2026-07-18** (review-7 finding 4),
by extracting every backticked identifier, expanding the `{a,b,c}` shorthands,
and matching each leaf against the `fn` and `mod` names the area actually
defines. Two stale citations were found and corrected: `stage_equivalence_arm`
(the arm-staging helper is `stage_proxy_pair`) and the AC 15 facet arithmetic.
Paths are relative to `claudine/`.

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

- `stage_proxy_pair` — stages one arm. Both arms execute the **same file**
  (`target.md`); only the route to it differs. The direct arm invokes it, the
  routed arm invokes `doc.md` (`EQUIV_ROUTER`) and is handed off at
  `initialize`. Pointing the direct arm at the target file itself, rather than a
  copy under a second name, is what keeps document-path-derived facets
  comparable.
- `normalize_arm` — rewrites each arm's own workspace path and basename to
  placeholders. Each arm owns a separate tempdir, so path-derived facets carry a
  random component; without this the matrix would compare randomness.
- `equivalence_arms` — runs both arms and returns their normalized event logs.
- `equivalence_arms_configured` — the same, with the router document, the
  `claudine` flags, and a per-arm staging hook spelled out. The provider-switch
  and MCP rows go through this one, because they need to install a recording
  stub or seed a catalog into each freshly staged arm.

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
| 10 | **Provider/model/MCP/argv/env/CWD/system prompt recalculated per target** | ✅ | Target-driven rows (no CLI provider flag, so the selection asserted on can only come from the target's frontmatter): `level2_lifecycle_equivalence_target_authored_provider_matches_direct_run` (provider, plus explicit-CLI precedence); `level2_lifecycle_equivalence_target_launch_bundle_matches_direct_run` (profile/binary, entrypoint, argv flags, effective child environment, interactivity, structured-output mode, dispatch/correlation configuration — router `goose` → target `codex`, and the router's stub must never launch); `level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run` (MCP runtime injection under a provider switch, router `codex` → target `gemini`, with the server set selected by the target's own prompt tag against an empty-defaults catalog). Invocation-level rows: `level2_lifecycle_equivalence_target_pinned_model_matches_direct_run` (model→env); `level2_lifecycle_equivalence_child_cwd_matches_direct_run` (child CWD); `level2_lifecycle_equivalence_cli_system_prompt_survives_the_proxy` (system-prompt delivery); `level2_lifecycle_sequence_step_proxy_rebuilds_target_launch_bundle` (full bundle in a step); `level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run` (workspace/CWD anchor); `level2_lifecycle_equivalence_stdout_stderr_routing_matches_direct_run` (output routing). **Direct provider wrappers (review-7 finding 3):** every facet named by the criterion is recalculated, and proven at L2, on the **surfaced command coordinator** — the path all three composition commands take. The direct provider wrappers have no coordinator to recalculate *with*, so they no longer adopt a target at all: `surface_or_adopt_terminal_proxy`'s unowned arm refuses the hand-off with the typed `LifecycleProxyWithoutOwningCoordinator` instead of borrowing the invocation's profile/argv/MCP. The reduced launch path R3 forbids is gone rather than documented. L1: `loop_control::tests::unowned_handoff::*` (typed identity, `err.*` projection, source stays active, terminal→`finalize` routing); `composition::error::tests::proxy_without_owning_coordinator_names_the_command_that_can_host_it` (the rendered block names a command that *can* host it). L2 guard: `level2_lifecycle_wrapper_passthrough_raises_no_proxy_handoff` — the wrapper installs no lifecycle config, so the refusal is currently unreachable and the divergence the finding predicted never shipped; see [the two paths](#the-surfaced-coordinator-versus-the-direct-provider-wrappers) |
| 11 | Target `initialize` after the narrow gate, before full pre-flight; may chain | ✅ | **The order is proven at L2 on every route, for both document shapes.** `level2_lifecycle_initialize_precedes_schema_verdict_{direct,initialize_proxy,recovery_proxy}` (review-7 finding 1) and `level2_lifecycle_initialize_precedes_schema_verdict_loop_{direct,initialize_proxy,recovery_proxy}` (review-8 finding 1) run one target whose `initialize` supplies a schema-`required` property the document does not author — the second trio being the same document plus a `loop:` of its own. Reaching the provider at all *is* the assertion: a verdict taken before `initialize` would have failed on the missing property. Each row also pins `initialize` firing exactly once, no schema diagnostic on the pane, target-owned `finalize`, and exit 0. The still-invalid converse — deferring the verdict must not *drop* it — is `level2_lifecycle_still_invalid_{target,loop_target}_runs_initialize_and_closure_first`, each running all three routes and asserting the owed `initialize` → `blocked` → `finalize` order *precedes* the rendered typed diagnostic, and that the invalid target never reaches a provider. **The loop gap is closed:** `compose/prep.rs::execute_loop_or_single` now threads the chosen `SchemaStage` into `loop_prepare_options` and into `build_and_run_loop`, so every read the loop route takes before the engine emits `initialize` withholds the verdict when the deferred stage was selected; the hard-coded `defer_schema_verdict: false` that made a looping document fail where the identical document without `loop:` succeeded is gone. L1 stage cover: `looping::engine::tests::seed_state::loop_seed_read_honors_the_deferred_schema_verdict`, which pins both halves — the undeferred seed read fails on the unauthored required property, the deferred one does not. Narrow-gate and entry-stage rows: `preflight::tests::initialize_scoped_audit_approves_only_the_initialize_command` (the audit approves only the `initialize` command); `prepare::entry::tests::only_a_new_active_document_emits_initialize`; L2 `level2_lifecycle_proxy_target_initialize_shell_is_gated_before_dispatch` (gate before dispatch, on a schema-*valid* target — the narrow-gate row, not the ordering row). Chaining: L2 `level2_lifecycle_proxy_three_document_chain_forwards_only_explicit_keys` (two `initialize` hops) and `level2_lifecycle_initialize_proxy_hop_limit_routes_source_blocked_finalize` (a 16-document `initialize` chain) |
| 12 | Full preparation rereads the stabilized target; no double `initialize` | ✅ | L2 `level2_lifecycle_proxy_target_rereads_after_initialize_mutation` (an `initialize` that rewrites its own frontmatter delivers the **mutated** body; the pre-`initialize` bootstrap body never reaches the provider; `initialize` fires once). Extended across all three routes by review-7 finding 1's `level2_lifecycle_initialize_precedes_schema_verdict_{direct,initialize_proxy,recovery_proxy}`, whose delivered prompt carries the value only the stabilized reread can see. Overlay half: `loop_control::tests::overlay_layering::the_overlay_reaches_the_bootstrap_read_and_the_stabilized_reread`. Stage semantics at L1: `prepare::service::tests::{a_deferred_read_withholds_the_verdict_the_validating_read_reaches, a_deferred_read_still_coerces_declared_types}` — the second is the one that matters, because a deferred read that stopped coercing would change the document the reread judges. Extended again to the looping shape by review-8 finding 1's `level2_lifecycle_initialize_precedes_schema_verdict_loop_{direct,initialize_proxy,recovery_proxy}` |
| 13 | Entry reasons obey the stage matrix; loops reuse the plan, retry/resume reread | ✅ | `prepare::entry::tests::stage_matrix_covers_every_entry_reason`; `..::retry_and_resume_fully_validate_but_a_loop_iteration_reuses_its_plan` |
| 14 | Retry refreshes canonically, fresh attempt, keeps overlay/provenance | ✅ | `coordinator::tests::retry_replaces_the_attempt_slice_and_drops_the_session`; `coordinator::tests::overlay_and_provenance_survive_a_canonical_refresh`; `..::retry_cannot_reset_its_own_budget_by_replacing_the_attempt` |
| 15 | **Resume retains only a session whose compatibility key matches; names facets** | ✅ | **Every document-reachable facet drives a real refusal end-to-end.** Both sides of the comparison derive from `target_launch::rebuild_launch_identity`, which recomputes provider / profile+binary / resume protocol / model / interactivity / permission mode / structured-output mode / MCP tag set from the document each fresh read re-materializes — and, since review-8 finding 2, that same rebuilt bundle *is* the launch the harness spawns (argv, profile, binary, session mode, structured-output shape, permission mode, MCP injection), so the key can no longer describe a plan the child did not receive. **Five isolating L2 refusal rows**, each proving no second provider launch, the changed facet(s) named on the pane, and `retry` recommended: `level2_lifecycle_resume_refuses_when_refresh_changes_{model,provider,interactivity,permission_mode,mcp_server_set}`. **Three further facets are named by those same rows** rather than by a row of their own, because none has a document surface independent of the facet that determines it: `profile/binary` and `resume protocol` are asserted by name on the `provider` row, and `structured-output mode` on the `interactivity` row. Converse (no false refusal): `level2_lifecycle_resume_with_dropped_launch_flag_stays_compatible`. L1 identity rebuild: `target_launch::tests::{frontmatter_agent_moves_the_provider_and_its_binary, frontmatter_interactive_moves_the_mode_and_structured_output, the_permission_mode_records_what_yolo_achieved_not_what_was_asked, body_mcp_tags_are_lexed_from_disk_and_only_when_mcp_is_enabled, an_explicit_cli_provider_pins_the_rebuilt_provider, an_unchanged_document_rebuilds_to_an_identical_identity}`. **The remaining two facets are immutable invocation inputs**, ratified as such in R8 (review-8 finding 3) rather than left as an unreachable requirement: `workspace/child CWD` is resolved from the process launch directory before any document is read, and the only document surfaces over launch identity are `agent:`/`model:`/`interactive:` — none names a directory — with the resolver deliberately preferring the launch repository over the document's own (`prep_context::tests::prep_context_launch_workspace_split_contract_unit`, verified non-vacuous); `system-prompt content` is composed once at invocation and captured, so the one mutation a lifecycle stack could attempt — rewriting the discovered `system-prompt.md` — moves neither delivery path (`launch_plan::tests::rewriting_the_discovered_system_prompt_moves_no_delivered_content`, verified non-vacuous). System-prompt *delivery* is not in that bucket: it is rebuilt per provider, so it moves exactly when the provider facet moves, which already refuses. Projections retained at L1: `session_key::tests::{changing_the_working_directory_projects_the_cwd_facet, a_changed_system_prompt_flag_projects_the_system_prompt_facet}`. Retain half: `coordinator::tests::resume_replaces_the_attempt_slice_but_retains_the_live_session`. **Lifecycle tail (review-9 finding 3, settled):** the refusal is a post-`start`, pre-spawn failure, so it now takes the shared typed catch protocol — `route_incompatible_resume` snapshots the `LifecycleResumeIncompatible` diagnostic into `err.*` and routes through `failure` then exactly one `finalize`, matching the ratified lifecycle contract and every other failure in that window. The no-second-spawn guarantee is unchanged: the re-run `failure` stack's `resume` control action is not dispatched from this path. All five L2 refusal rows assert the full `start → provider-ran → initial-prompt-ok → failure → start → failure → finalize` trace; L1 routing cover is `retry_resume::a_refused_resume_routes_through_failure_then_finalize_with_err`. The previously carried "is `finalize` owed here" question is closed. The wrapper-path `mcp_enabled` gap this row previously carried was closed by review-8 finding 5 — see [open items](#open-items-carried-out-of-review-7--both-closed-by-review-8) |
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

## Previously-blocked criteria — all four resolved

Phase 13 recorded AC 7, AC 9 (launch half), AC 10, and AC 15 as blocked on the
R6 launch rebuild being unstarted. review-3 findings 1-3 opened that work and
review-5 findings 1-6 completed it on the **surfaced-coordinator** path,
review-6 findings 2-3 closed the remaining verification gaps, review-7
findings 2-3 closed AC 10 and moved AC 15 from one reachable facet to eight, and
review-8 findings 2-3 closed AC 15 — finding 2 by making the rebuilt bundle the
launch the harness actually spawns, finding 3 by ratifying the last two facets
as immutable invocation inputs on structural evidence. All four are now
complete (verified 2026-07-18, `just test` + `just lint` + `just test-l2 resume`).

AC 11 was never on this list — it was recorded complete throughout until
review-7 finding 4 found that mark unsupported, and review-8 finding 1 then
earned it. Its history is in [the AC 11 row](#criteria), not here.

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

### AC 10 — resolved

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

**review-7 finding 3 closed the remaining gap.** The criterion is unqualified as
to path, and the one path that did not rebuild the bundle — the direct provider
wrappers — no longer adopts a target at all. It refuses the hand-off with
`CompositionError::LifecycleProxyWithoutOwningCoordinator` rather than borrowing
profile/binary, argv entrypoint, or MCP runtime injection from the invocation.
Every path that *runs* a proxied target now rebuilds its complete launch bundle;
the path that cannot says so. See
[the two paths](#the-surfaced-coordinator-versus-the-direct-provider-wrappers),
including why the borrowed bundle was latent rather than shipped.

### AC 15 — complete: eight facets reachable, two immutable by construction

review-6 finding 2 made the refusal reachable rather than latent, by rebuilding
the launch env at every retry/resume fresh-read boundary instead of once at proxy
adoption. That moved exactly one facet — `model`.

**review-7 finding 2 moved seven more.** The comparison no longer derives from
the env projection: `rebuild_launch_identity` recomputes the whole typed launch
identity from `(document, LaunchRebuildIntent)` at each fresh read, and both
sides of the key are taken from it. Five isolating L2 refusal rows plus three
facets named on those same rows bring the total to eight of ten.

**review-8 finding 2 removed the retry residue.** Provider argv, MCP runtime
injection, and the spawned profile are now rebuilt at a same-document boundary
too, from the re-entrant `launch_plan` builder, so the key and the child can no
longer describe different launches.

**review-8 finding 3 resolved the last two facets by ratification.** Each was
re-examined for reachability rather than assumed:

- `workspace/child CWD` is resolved from the process launch directory before any
  document is read. The complete set of document surfaces over launch identity
  is `agent:`/`model:`/`interactive:` — none names a directory — and the
  resolver deliberately prefers the launch repository over the document's own,
  so even a proxy into a sibling clone leaves the child where it started.
- `system-prompt content` is composed once at invocation and captured; the
  builder re-delivers that capture rather than re-reading its source, so a
  lifecycle stack rewriting the discovered `system-prompt.md` between attempts
  moves neither delivery path.

Neither can move across a same-document resume, so an end-to-end refusal row for
either would be unwritable, not merely missing. R8 now defines them as immutable
invocation inputs and states the structural reason; both claims carry a
non-vacuity-verified L1 test. The per-facet evidence, the empty
`SessionCompatibilityKey::extra`, and the settled `failure`/`finalize`-on-refusal
routing are in the [AC 15 row](#criteria) — that row, not this section, is the
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

## Open items carried out of review-7 — both closed by review-8

Two gaps were found while closing review-7 and were recorded here rather than in
a row, because neither invalidated the criterion it touched. Review-8 closed
both. They are kept for the trail, not as live items.

1. **A looping document reached its schema verdict before its `initialize`** —
   `compose/prep.rs::execute_loop_or_single` consumed the chosen `SchemaStage`
   on its single-execution branch only, so a document that owned a `loop:` and
   whose `initialize` supplied a schema-`required` property failed on iteration
   1, before the loop engine routed that `initialize`. **Closed by review-8
   finding 1:** the stage is threaded into `loop_prepare_options` and
   `build_and_run_loop`, and the L2 ordering matrix gained the looping arm the
   item asked for. See [the AC 11 row](#criteria).

2. **The wrapper's `LaunchRebuildIntent` hardcoded `mcp_enabled: false`** —
   contradicting `--mcp` / `--mcp-use` on `WrapperArgs`, so a passthrough
   document's `#tag`s could never move the MCP facet of the resume
   compatibility key. **Closed by review-8 finding 5:** `wrapper_stages.rs`
   derives it as `args.mcp || !args.mcp_use.is_empty()`, matching the
   composition path, and the derivation is pinned by a switch-matrix assertion
   in that module's tests. The blast radius was always small (the wrapper path
   raises no proxy hand-off at all — see [the two paths](#the-surfaced-coordinator-versus-the-direct-provider-wrappers)); what is
   removed is a constant that contradicted its own flags.

## Recorded gate results (2026-07-18, macOS host, review-7 finding 4)

All three gates run from the `claudine/` package area, **full area, not a
subset**, at the HEAD that carries review-7 findings 1-4.

| Gate | Exit | Result |
|---|:-:|---|
| `just test` | **0** | `claudine-catalog-types` 21/21 · `claudine` 3530/3530 (7 skipped) · `claudine-contract` 47/47 (5 skipped) · `claudine-cli` 2054/2054 (202 skipped) · `claudine-gen` 152/152 (4 skipped) |
| `just test-l2` | **0** | **175 passed, 0 failed**, 2081 skipped (tiers/backends not available on this host), **0 flaky** |
| `just lint` | **0** | clippy + fmt-check + error-transport / lifecycle-doc guards clean |

The L2 run was clean on the first attempt. The three rows documented as
flaky-under-load — `level2_lifecycle_failure_proxy_to_looping_target_matches_direct_iterations`,
`level2_lifecycle_sequence_step_proxy_to_looping_target_owns_the_loop`, and
`level2_malformed_frontmatter_renders_highlighted_diagnostic_in_tmux` (note the
name: it carries no `lifecycle_` segment and ends `_in_tmux`) — all passed
without a retry, as did the two WezTerm capture tests the review-6 record
saw flake (`level2_perf_tree_renders_styled_in_wezterm`,
`level2_wezterm_removed_key_renders_yaml_codeblock`). Those remain
load-sensitive rather than fixed; this host was quiet.

Every test review-7 added ran (did **not** skip) and passed:

| Test | Finding |
|---|---|
| `level2_lifecycle_initialize_precedes_schema_verdict_direct` | 1 |
| `level2_lifecycle_initialize_precedes_schema_verdict_initialize_proxy` | 1 |
| `level2_lifecycle_initialize_precedes_schema_verdict_recovery_proxy` | 1 |
| `level2_lifecycle_still_invalid_target_runs_initialize_and_closure_first` | 1 |
| `prepare::service::tests::a_deferred_read_withholds_the_verdict_the_validating_read_reaches` | 1 |
| `prepare::service::tests::a_deferred_read_still_coerces_declared_types` | 1 |
| `level2_lifecycle_resume_refuses_when_refresh_changes_{provider,interactivity,permission_mode,mcp_server_set}` | 2 |
| `target_launch::tests::{frontmatter_agent_moves_the_provider_and_its_binary, frontmatter_interactive_moves_the_mode_and_structured_output, the_permission_mode_records_what_yolo_achieved_not_what_was_asked, body_mcp_tags_are_lexed_from_disk_and_only_when_mcp_is_enabled, an_explicit_cli_provider_pins_the_rebuilt_provider, an_unchanged_document_rebuilds_to_an_identical_identity}` | 2 |
| `level2_lifecycle_wrapper_passthrough_raises_no_proxy_handoff` | 3 |
| `loop_control::tests::unowned_handoff::{an_unowned_handoff_is_refused_with_a_typed_diagnostic, a_refused_unowned_handoff_routes_the_sources_terminal_then_finalize, the_refusal_projects_its_typed_identity_into_err}` | 3 |
| `composition::error::tests::proxy_without_owning_coordinator_names_the_command_that_can_host_it` | 3 |

The review-6 rows this record supersedes
(`level2_lifecycle_dry_run_fires_no_lifecycle_events_and_no_proxy_traversal`,
`level2_lifecycle_resume_refuses_when_refresh_changes_model`,
`level2_lifecycle_resume_with_dropped_launch_flag_stays_compatible`, the three
`level2_lifecycle_equivalence_*` rows, and the three
`level2_lifecycle_diagnostic_matrix_*` rows) all still exist and still pass;
they are cited from their own criteria rows rather than repeated here.

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
