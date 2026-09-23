# Control-flow baseline

This is a source-derived baseline for `2026-09-22-lifecycle-events`, recorded
2026-09-19. It separates the catch protocol from normal terminal recovery because
those paths intentionally have different control semantics. It is not a claim
that a single current function implements the whole lifecycle.

## Authorities in the current implementation

Paths below are repository-relative. These are source anchors rather than a
complete graph-derived caller inventory.

| Authority | Source | Responsibility |
|---|---|---|
| Emission accounting | `claudine/lib/src/composition/lifecycle/mod.rs`, `LifecycleRunGuard` | Initialize/start flags, one effective terminal slot, finalize eligibility, retry/loop and proxy resets |
| Catch protocol | `claudine/lib/src/composition/lifecycle/runtime.rs`, `LifecycleCatchProtocol` | Setup catches, blocked redesignation, error threading, evaluation precedence |
| Pure decision | Same file, `decide_lifecycle_transition` | Decision given supplied state; it does not update state or execute the full CLI protocol |
| Control admission | `claudine/lib/src/composition/lifecycle/control.rs`, `decide_control` | Attempt ceilings, launch-dependent retry classification, session requirement, backoff |
| Terminal downgrade | `claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs`, `execute_terminal_event` | Success/blocked communication first, then explicit-error downgrade and failure stack |
| Recovery scheduling | Same directory, `control_dispatch.rs`, `drive_terminal_recovery` | Terminal control before finalize; finalize recovery; abort cleanup; proxy transfer |
| Refusal classification | Same directory, `error_routing.rs` | Refused handoff versus incompatible resume; cleanup without another recovery attempt |
| Preparation/identity | `claudine/lib/src/composition/coordinator/` and CLI composition pipeline | Active document, proxy adoption, canonical retry/resume preparation, initialization and shell boundary |
| Iteration | `claudine/lib/src/composition/looping/engine.rs` | Gate ordering, retained initialization, iteration mutations |

## Normal progress and operation facts

| Trigger | Required facts | Next work | State/effect constraint |
|---|---|---|---|
| Adopt document | Bootstrap succeeded and target owns lifecycle | Initialize, then preparation | New document starts shell-free |
| Preparation succeeds | Composition and applicable preflight passed | Start (`composed` alias in proposed API), then host operation | Opening the lifecycle shell boundary is a checked transition |
| Preparation blocked | Active lifecycle exists | Blocked, then eligible cleanup | No invented start; early shells stay disabled |
| Operation and completion verdict pass | Host supplies both results | Success | Composition alone is insufficient |
| Operation or completion verdict fails | Selected diagnostic | Failure | Same selected error feeds `err` and reporting |
| Launch/continuation refusal | Failure stage and entry reason | Host-specific blocked or failure route | `provider_launched == false` alone is not sufficient to classify the event |

In particular, `route_incompatible_resume` uses **failure → finalize** after
start but before spawn, while refused handoff routing can select blocked. A
neutral outcome envelope therefore needs failure stage/entry information or an
explicit validated classification in addition to a launched flag.

## Handler outcomes

| Origin and outcome | Next events/decision | Error and control behavior |
|---|---|---|
| Initialize/start: evaluation failure | Failure → finalize | Original evaluation halts; later catch evaluation supersedes it |
| Initialize/start: routing action failure or explicit error | Failure → finalize | Setup error is carried separately from evaluation failure |
| Blocked in catch path: evaluation/action/explicit error | Redesignate slot → failure → finalize | Keep previously emitted blocked communication |
| Success/blocked in normal terminal recovery: explicit error | Redesignate slot → failure | Failure's resulting control is eligible for normal recovery |
| Success/failure: evaluation failure | Finalize once, then halt | Do not retroactively emit another failure event |
| Finalize: evaluation failure | Halt | Never finalize recursively |
| Loop: evaluation failure after finalize | Halt | No second finalize |
| Terminal event: dispatch failure only | Existing terminal routing continues | `routes_to_failure` is setup-specific; do not silently change terminal dispatch-error policy |
| Normal terminal event: retry/resume admitted | Recovery request, then next attempt | **Skip this attempt's finalize** |
| Normal terminal event: proxy selected | Handoff request | **Skip source finalize**; adoption belongs to coordinator |
| Normal terminal event: control aborts | Finalize if eligible, then propagate abort | Finalize evaluation can supersede abort; cleanup controls do not recover it |
| Normal terminal event: no effective recovery/exhausted budget | Finalize | Recovery can still be selected by normal finalize |
| Normal finalize: admitted retry/resume/proxy | Recovery request | Finalize already ran; never repeat it for the old attempt |
| Catch-only finalize: retry/resume/proxy | Finish catch protocol | Catch protocol does not dispatch the cleanup handler's control |

`LifecycleCatchProtocol::finish()` returns the **originating** control only when
no evaluation failed. It does not return failure/finalize controls. This is a
property of that protocol, not a global rule that finalize cannot recover.

## Re-entry and permissions

| Entry | Initialize | Preparation | Accounting |
|---|---|---|---|
| Retry | Retained | Canonical reread/audit | New attempt, same iteration; retain retry ceiling; drop session |
| Resume | Retained | Canonical reread/audit plus compatibility check | New attempt, same iteration; retain resume ceiling and session |
| Next loop iteration | Retained | Reuse audited structural plan | New iteration and attempt budget; re-enter start/composed |
| Proxy adoption | Fresh for target | Fresh target preparation | New document identity, target budgets, reset shell boundary |
| Rejected handoff | No target initialization | No committed target | Source remains authoritative; route its refusal cleanup |

`decide_control` reports `reenter_preflight` based on launch state. The CLI's
actual retry implementation also sets the canonical document-entry reason and
clears the document epoch, including for a post-launch retry. Do not equate the
pure decision's flag with a complete preparation schedule.

Retry and resume ceilings are distinct, established on first firing, and
cannot be increased by re-firing a control. Proxy chain identity and hop budget
are invocation-level host facts. The prototype deliberately uses opaque document
numbers; it does not resolve paths or reproduce proxy cycle detection.

## Implications for the new boundary

1. Event identity, transition role, execution permission, and recovery mode are
   separate concepts. Before/after position cannot encode all four.
2. Make normal recovery versus catch-only cleanup an explicit mode owned by the
   engine. Do not let each adapter infer it from the event name.
3. Represent finalize as optional closure on a specific exit path, not an
   unconditional `finally` block. Preserve recovery/handoff exceptions.
4. Keep an operation request pending until its matching response arrives. Report
   refusal separately from successful re-entry/adoption.
5. Require stage-aware host failure facts. An unstarted operation can still
   require failure rather than blocked.
6. Separate a pure decision oracle from a complete state machine. Matching the
   former is necessary evidence, not proof of end-to-end orchestration parity.
