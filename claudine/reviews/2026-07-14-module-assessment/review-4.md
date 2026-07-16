---
implemented: true
---
# Claudine Module-Assessment Implementation Review, Round 4

**Date:** 2026-07-15

**Source assessment:** [`review-3.md`](review-3.md)

**Scope:** the uncommitted implementation snapshot present at review start, relative to `HEAD` (`475a54554`), covering the initial 20 changed files under `claudine/{lib,cli,gen,docs}` and `.claude/skills/claudine/architecture.md`. Unrelated Claudine skill/topic-document edits appeared concurrently after verification began; those late-arriving files are outside this assessment.

## Executive assessment

The Round 3 implementation is substantially complete and preserves current behavior. The old terminal-routing helpers are gone, the loop-engine and harness terminal-evaluation paths now consume `LifecycleCatchProtocol`, the protocol receives terminal-slot and finalize state, and its tests cover initialize, start, blocked, failure, success, and loop origins. The harness preparation coordinator now builds three named state-family contracts and has fallen to 65 effective sloc. The architecture guide also documents both the supported test-module visibility forms and the intended lifecycle owner.

Two gaps remain. The more important one is that `LifecycleCatchProtocol` is still not the only policy authority: production adapters duplicate its setup-catch eligibility predicate before deciding whether to invoke it, and an unused public error helper retains a second implementation of the evaluation-error precedence rule. The other is localized comment drift left by removal of the pre-run snapshot path.

No runtime regression was observed in the package test suite.

| Severity | Count | Summary |
|---|---:|---|
| Medium | 1 | Setup-catch eligibility and precedence still have residual policy owners outside `LifecycleCatchProtocol`. |
| Low | 1 | Harness routing rustdoc and tests still describe the removed snapshot path. |

## Remaining findings

### 1. Medium — `LifecycleCatchProtocol` is the sequence executor, but not yet the single policy authority

`LifecycleCatchProtocol::after_origin` in `lib/src/composition/lifecycle/runtime.rs:178-215` now contains the provider-neutral rules for whether initialize/start/blocked must enter `failure`, whether a terminal origin must enter `finalize`, blocked-slot redesignation, and finalize eligibility. Its `finish` method at lines 263-293 owns the intended `finalize > failure > origin` evaluation-error precedence.

The production adapters still decide setup-catch eligibility themselves before invoking the protocol:

- `cli/src/commands/wrap/composition/pipeline.rs:1111-1143` independently checks initialize evaluation errors, action errors, and explicit `error(...)` control before calling `execute_initialize_catch`.
- `lib/src/composition/looping/engine.rs:355-381`, its explicit-error branch, and its action-error route select the initialize catch in separate branches before `execute_loop_catch_protocol` is reached.
- `cli/src/commands/wrap/harness_orch/loop_control.rs:393-505` separately branches for start evaluation errors, explicit `error(...)`, and action errors before calling `run_catch_protocol`.

This is already observable as disagreement between provider-neutral policies: `LifecycleCatchProtocol::after_origin` classifies setup-phase `StackControl::Error` as a failure catch, while `decide_lifecycle_transition` in the same module resolves that control directly to `TerminalFailure`. Current callers compensate with their manual branches, so the tests pass, but adding or changing a setup-catch criterion still requires coordinated edits across the library, composition pipeline, loop engine, and harness.

The old routers were removed, but `CompositionError::catch_evaluation_error` remains at `lib/src/composition/error/mod.rs:1589-1619`. It has no call sites and independently reimplements the same `finalize > failure > origin` selection now owned by `LifecycleCatchProtocol::finish`. Leaving this public compatibility surface in place preserves a second precedence authority that a future caller can accidentally revive.

**Recommended closure:** run the protocol for every relevant origin outcome and let its requested steps/result be the sole setup-catch classification, with adapters branching only on the returned control/error result. Reconcile or delegate the overlapping `decide_lifecycle_transition` setup-control decision. Remove `CompositionError::catch_evaluation_error`, or make it a thin consumer of a completed protocol result rather than retaining its own precedence logic.

### 2. Low — Source documentation still refers to the removed snapshot path

The harness split removed pre-run snapshot capture, and `loop_control.rs:357-358` explicitly says so. The documentation surrounding `emit_failure_finalize_with_err` was not updated with that behavior change:

- `cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs:173-185` still says the helper covers “snapshot capture” and is used by the “snapshot / launch / attempt” sites.
- `cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs:328-362` and `tests/terminal_evaluation.rs:377` repeat the same obsolete path description.

The implementation now routes launch construction and pre-spawn attempt failures, but there is no snapshot capture site. This is comment drift under the repository's behavior-change documentation rule.

**Recommended closure:** remove the historical marker in `prepare_attempt_phase` and update the helper/test documentation to name only the live launch and pre-spawn attempt paths.

## Round 3 findings now closed

The following requested changes are implemented sufficiently:

- **Lifecycle sequence migration:** `TerminalRoutingDecision`, `route_blocked_finalize`, `route_failure_finalize`, and `route_loop_gate` are removed. Composition initialize/preflight, loop initialize/gate, harness setup, and harness terminal-evaluation routes execute the protocol's requested steps.
- **Exactly-once protocol state:** `LifecycleCatchState` carries `terminal_slot` and `finalize_emitted`; the protocol decides blocked-slot redesignation and suppresses an ineligible duplicate finalize request.
- **Protocol matrix coverage:** library tests cover initialize, start, blocked, failure, success, and post-finalize loop origins, including error threading and precedence.
- **Harness preparation contracts:** `AttemptPromptPreparation`, `AttemptLifecycleExecution`, and `AttemptRetryProxyControl` replace the former loose preparation argument lists. `prepare_attempt_phase` is now a 65-effective-sloc coordinator, with no `too_many_arguments` allowances on its extracted phase helpers.
- **Architecture documentation:** the guide names `LifecycleCatchProtocol`, states adapter responsibilities, and correctly documents private, `pub`, `pub(crate)`, and `pub(super)` inline test-module detection.
- **Previously closed Round 2 work:** the generator assembler, visibility-aware placement guard, and governed dispatch-count documentation remain intact.

## Verification

The review inspected the complete current working-tree diff, traced all `LifecycleCatchProtocol` and legacy-routing references, compared the migrated paths with their `HEAD` implementations, ran the current module-size scan, and used GitNexus change detection. GitNexus classifies the change set as HIGH risk because the modified lifecycle routes participate in the main composition execution flow; no unexpected affected process was found.

The following gates passed on macOS:

- `just test` across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and `claudine-gen`;
- `just lint` across all five crates, including the error-transport and lifecycle-doc-facets guards; and
- `git diff --check`.

The current `hug god-files --json claudine` scan reports `loop_control.rs` at 1,251 physical lines / 1,141 effective sloc. The preparation coordinator is 65 effective sloc; the largest remaining functions in that file are classification (196), attempt execution (158), lifecycle start (145), and plan preparation (124). Those broader module-assessment opportunities are not regressions against the specific Round 3 closure request.

The rendezvous suite was not rerun because the implementation changes no rendezvous files. Windows and Linux behavior was reviewed structurally but not executed on this macOS host.

No implementation source was modified during this assessment. The only file created by this review is `review-4.md`; existing files, untracked files, and the unrelated documentation changes that arrived concurrently were left untouched.

## Closing assessment

The implementation is behaviorally sound and closes the concrete Round 3 migration, state-contract, and architecture-documentation tasks. The assessment should remain open for one focused ownership cleanup: make the protocol the actual entry-point policy for every setup origin and retire the remaining duplicate precedence helper. The stale snapshot references can be corrected in the same pass.
