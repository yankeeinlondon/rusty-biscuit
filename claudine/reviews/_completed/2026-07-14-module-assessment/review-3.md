# Claudine Module-Assessment Implementation Review, Round 3

**Date:** 2026-07-15

**Source assessment:** [`review-2.md`](review-2.md)

**Scope:** the current uncommitted implementation relative to `HEAD` (`475a54554`), covering the 19 changed files under `claudine/{lib,cli,gen,docs}` and `.claude/skills/claudine/architecture.md`.

## Executive assessment

The implementation is a meaningful improvement, but `review-2.md` is **not fully closed**. The generator assembler, visibility-qualified test-module detection, and governed dispatch-count documentation are addressed. The harness preparation god-function is also substantially decomposed: `prepare_attempt_phase` fell from 487 to 105 effective sloc and now coordinates named transition-boundary helpers.

The highest-priority lifecycle finding remains partially open. The new `LifecycleCatchProtocol` correctly moves the three edited CLI paths onto a provider-neutral step protocol, but it coexists with the old precedence router and with manual failure→finalize sequences elsewhere. There are therefore still multiple behavioral owners for the ordering and error-precedence contract that the change set was intended to centralize.

No behavior regression was observed in the test suite. The remaining findings concern incomplete ownership, residual coupling, and documentation drift.

| Severity | Count | Summary |
|---|---:|---|
| High | 1 | Lifecycle catch/finalize sequencing still has multiple implementations. |
| Medium | 1 | Harness preparation was split, but most phase dependencies remain loose and broadly coupled. |
| Low | 1 | The architecture guide still documents the old test-placement limitation and omits the new lifecycle owner. |

## Remaining findings

### 1. High — `LifecycleCatchProtocol` is not yet the single lifecycle sequencing owner

The new protocol in `lib/src/composition/lifecycle/runtime.rs:99-277` is a sound direction. It requests `failure`/`finalize` effects, carries the active error, identifies the blocked-slot redesignation operation, and selects evaluation-error precedence. The edited composition preflight, initialize, and harness setup paths now execute the protocol's requested steps rather than each spelling out the full sequence.

The intended ownership change remains incomplete in three ways:

- `lib/src/composition/lifecycle/runtime.rs:394-477` retains `TerminalRoutingDecision` plus `route_blocked_finalize`, `route_failure_finalize`, and `route_loop_gate`. `TerminalRoutingDecision::new` independently implements the same `finalize > failure > origin` precedence and origin-control selection that `LifecycleCatchProtocol::finish` implements at lines 245-275. The provider-neutral library now has two precedence authorities.
- `lib/src/composition/looping/engine.rs:349-380` still manually runs initialize evaluation failures through `failure` and `finalize`, threads the active error, and selects the surfaced error. `route_init_failure_with` repeats the clean/action-error variant at lines 767-808 and calls the old `route_failure_finalize` helper. Its own comments describe this as mirroring the non-loop path, which is precisely the duplication the protocol was introduced to remove.
- `cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs:233-315` retains `surface_catch_evaluation_error` and `handle_terminal_evaluation_error`, which manually execute `finalize` and consult the old router instead of feeding the outcome through the new protocol.

The protocol also does not accept terminal-slot or `finalize_emitted` state. It always requests `finalize` for applicable origins and relies on `LifecycleRunGuard` to turn a duplicate request into a no-op. That preserves current exactly-once behavior, but it means the protocol itself cannot yet own the exactly-once decision its rustdoc claims to centralize.

This is not merely dead compatibility surface: the old router is still used by production loop and terminal-evaluation paths. A future change to error precedence, catch-event control handling, or finalize eligibility would still require coordinated edits across both implementations.

GitNexus classifies the implementation diff as HIGH risk and traces the changed lifecycle paths into the main composition execution flow. Existing integration tests cover the current behavior and pass, but they do not remove the duplicated source of truth.

**Recommended closure:** migrate the loop-engine initialize routes and terminal-evaluation route to `LifecycleCatchProtocol`; include the guard facts needed to decide finalize eligibility in the protocol input; then remove the old terminal routers or make them thin delegates to the protocol's single precedence calculation. Add protocol matrix tests for initialize, start, failure, success, and loop origins—not only blocked.

### 2. Medium — The harness split reduced function size without fully reducing state coupling

The implementation fixes the most visible part of Finding 2. `prepare_attempt_phase` at `cli/src/commands/wrap/harness_orch/loop_control.rs:263-375` is now a 105-effective-sloc coordinator, and proxy preflight, materialization, proxy adoption, plan preparation, and lifecycle start have distinct functions and results.

The residual coupling called out by `review-2.md` is still visible:

- `prepare_attempt_phase` begins by aliasing 17 values out of `HarnessLoopState` and its nested run context.
- All five extracted transition helpers carry `#[allow(clippy::too_many_arguments)]`.
- The helpers take 9–13 loose arguments; `start_lifecycle_phase` remains 142 effective sloc and `prepare_harness_plan_phase` 121 effective sloc.
- `loop_control.rs` remains High-risk in the current `hug` scan at 1,254 physical lines and 1,147 effective sloc.

This is still materially better than the previous 487-effective-sloc phase, and the implementation should retain the new boundaries. However, the review asked for phases over deliberately narrower attempt state, not only extraction into long parameter lists. Lifecycle guard, prompt state, effect engine, repository context, terminal, timing, and proxy/budget state remain repeatedly threaded as independent values.

**Recommended closure:** keep the current phase boundaries but give each phase a cohesive input/state contract derived from the invariants it owns. Avoid a wrapper that merely hides the same argument list; the useful reduction is to separate prompt-preparation state, lifecycle execution state, and retry/proxy control state so a phase cannot mutate unrelated run state.

### 3. Low — Architecture documentation was not updated alongside the behavior changes

The visibility parser implementation and its fixtures satisfy Finding 4: `skip_visibility` recognizes private, `pub`, `pub(crate)`, and `pub(super)` inline test modules before the analyzer matches `mod`.

The authoritative architecture guide still says the opposite. `.claude/skills/claudine/architecture.md:459` states that visibility-qualified modules are not recognized and must not be used to hide tests. That sentence is now stale and should have been changed with `cli/tests/test_placement.rs`.

The same guide documents the composition and harness boundaries but does not mention `LifecycleCatchProtocol` or explain which lifecycle sequencing it owns. Because this implementation introduces a new public provider-neutral state protocol, the absence makes the intended ownership harder to assess and contributes to Finding 1's dual-authority state.

**Recommended closure:** update the test-placement paragraph to list the supported visibility forms, and document `LifecycleCatchProtocol` in the composition/lifecycle architecture section with an explicit statement of which paths must use it and which responsibilities remain in CLI adapters.

## Review-2 findings now closed

The following items are implemented sufficiently:

- **Generator assembler:** `emit_data_file` is now a short coordinator over typed `EmissionFragment`s. Stable domains own their fields and supporting items, the coordinator validates a complete 47-field order, and generated-artifact drift remains byte-stable. `claudine-gen` tests pass.
- **Visibility-qualified test modules:** the analyzer skips optional Rust visibility and has fixtures for the four requested forms. The repository placement gate passes with no exceptions.
- **Dispatch-count drift:** both provider-metadata documents now avoid freezing a transient number and direct readers to the live `GUARD_ALLOWLIST` count. The regenerated dispatch inventory and its guard pass.
- **Harness god-function size:** the 487-effective-sloc preparation function is gone. Finding 2 above is about the remaining dependency shape, not a claim that the extraction failed.
- **Direct CLI catch-sequence duplication:** composition preflight, composition initialize, and harness setup routing now consume `LifecycleCatchProtocol`. Finding 1 concerns the production paths and old authority that were not migrated.

## Verification

The review used the current working-tree diff, the Claudine architecture guidance, `hug god-files --json claudine`, targeted source inventories, and GitNexus change detection, impact, and symbol context.

The following gates passed on macOS:

- `just test` across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and `claudine-gen`, including lifecycle adapter tests, generator drift, dispatch inventory, and test-placement enforcement;
- `just lint` across all five crates, including the error-transport and lifecycle-doc-facets guards; and
- `git diff --check`.

Nextest reported retry-recovered timeouts in several slow tests, but no final failures. The rendezvous suite was not rerun because this implementation changes no rendezvous files. Windows and Linux behavior was reviewed structurally but not executed on this macOS host.

No implementation source was modified during this assessment. The only new workspace file from this review is `review-3.md`; the existing untracked `.claude/settings.local.json` and `review-2.md` were left untouched.

## Closing assessment

The implementation is **mostly complete but still needs lifecycle ownership work before the architectural review can be considered closed**. The generator and structural-guard changes are ready. The harness refactor should be retained and followed by a narrower state-contract pass. The documentation correction is small and should land with the lifecycle cleanup so the architecture guide names the actual single owner once that owner exists.
