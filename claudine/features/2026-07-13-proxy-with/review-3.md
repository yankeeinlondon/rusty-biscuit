---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T08:06:51-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
next: 2026-07-13-proxy-with/review-4.md
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-3.md
previous: 2026-07-13-proxy-with/review-2.md
---

# Review 3: Proxy With

## Verdict

The feature is **not ready for production**. The `proxy.with` authoring surface,
typed handoff, overlay evaluation/layering, canonical preparation service, and a
substantial part of the coordinator are now implemented. However, the runtime
still fails the specification's defining equivalence contract: a proxied target
keeps the router's launch state and does not acquire the target's loop. Resume
compatibility is also absent, and the required Level 2 gate is red.

The implementation's own plan and acceptance map acknowledge these gaps: only
26 of 30 acceptance criteria are mapped to passing tests, with AC 7, AC 9/10,
AC 15, and part of AC 26 blocked. The two most important failing behaviors are
preserved as ignored reproduction tests rather than active regression tests.

## Findings

### 1. Critical: A proxied target still runs with the router's launch state

`execute_composition_request_inner_with_guard` resolves selection, provider,
environment/MCP, argv/system prompt, and lifecycle before entering
`provider_run_handoff` (`cli/src/commands/wrap/composition/pipeline.rs:142-184`).
Those frozen `SelectionPhase`, `EnvironmentPhase`, and `CommandPhase` values are
then passed into the attempt harness (`pipeline.rs:1220-1266`).
`ActiveDocumentCoordinator::adopt` replaces prompt identity, overlay, session,
budgets, and lifecycle state, but does not rebuild provider/model, MCP, argv,
child environment/CWD, system prompt, interactivity, or output mode
(`cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs:108-147`).

The ignored Level 2 reproduction
`level2_lifecycle_equivalence_target_pinned_model_matches_direct_run`
documents the observable result: direct execution exports
`MODEL=llamacpp/probe-model-x`, while the routed execution exports an empty
model (`cli/tests/level2_lifecycle_control.rs:2323-2380`). This violates R6 and
AC 9-10 and can launch the wrong provider configuration.

**Required change:** introduce the target launch rebuild described by R6. Keep
only invocation-level CLI intent and command/sequence output policy; derive all
target-dependent launch inputs again from the stabilized target. Re-enable the
pinned-model Level 2 test and add matrix rows for the remaining launch facets.

### 2. Critical: Loop ownership still follows the router, so the motivating bug remains

Loop selection occurs in `execute_loop_or_single` from the initially invoked
document (`cli/src/commands/compose/prep.rs:687-766`). The code explicitly
rejects a proxy produced by a looping router because loop ownership has not yet
moved to the adopted document (`prep.rs:794-810`). In the inverse headline case,
a non-looping router that proxies to a looping target enters the single-run path,
so the target executes once instead of owning its declared loop.

The ignored Level 2 test
`level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run`
captures exactly this three-iteration direct-versus-routed mismatch and states
that Phase 10 must re-enable it
(`cli/tests/level2_lifecycle_control.rs:1830-1906`). This violates R7 and AC 7,
and leaves the feature's motivating `implement.md` routing failure unresolved.

**Required change:** stabilize initialize routing before loop recognition, then
give the adopted target the same document-loop coordinator used by direct
execution. A loop-emitted proxy must end the source document and transfer
ownership, not be refused or reduced to a single attempt.

### 3. High: Resume retains a live session without the required compatibility key

No `SessionCompatibilityKey` or equivalent launch-plan fingerprint exists.
The resume branch checks only provider resume support and the presence of a
session ID, then carries that ID into the next attempt
(`cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:165-194`).
It does not compare provider, profile/binary, model, MCP, CWD, environment,
system prompt, interactivity, structured mode, or other launch-affecting facets
after canonical refresh.

This violates R8 and AC 15. Once the R6 rebuild exists, a changed target launch
plan could incorrectly reuse a session created under incompatible settings.

**Required change:** put the specified compatibility key on the prepared launch
plan, compare it after resume refresh, and return a typed diagnostic naming the
changed facets before any provider attempt. Add Level 1 key-diff tests and a
Level 2 resume case that changes a launch facet.

### 4. High: Sequence template preflight still derives context from ambient process state

`build_template_preflight_options` uses argumentless
`ComposeContext::capture()` (`cli/src/commands/wrap/sequence/phase1c.rs:478-489`).
Its file-reference fallback is explicitly anchored, but `ctx.*` discovery still
depends on the process CWD after the wrapper may have changed it. The preflight
can therefore approve shell bytes composed from different context than the
eventual sequence-step execution.

This is a surviving R5/R9 violation and weakens AC 8 and AC 17 for sequence
steps. It is already baselined as known debt by the ambient-capture corpus guard;
the guard prevents growth but does not make this site safe.

**Required change:** pass the sequence step's explicit launch/document context
into preflight and reuse that stored snapshot through schema, shell discovery,
and execution. Add a Level 2 sequence fixture that changes process CWD and
asserts approved bytes equal executed bytes.

### 5. High: Required Level 2 evidence is incomplete and the current L2 gate fails

The direct/proxy equivalence harness is useful, but its main assertions compare
fixture event logs. Pane captures are included primarily in failure messages;
there is no Level 2 pane assertion proving AC 30's overlay-aware status/error
redaction and terminal rendering. Mapping AC 30 to Level 1 `Debug` redaction
tests plus a static no-`println!` guard is the wrong verification level for
user-visible terminal output.

The matrix also lacks the specification's three-document forwarding/omission
chain, cross-repository context/file resolution, proxy-inside-sequence behavior,
stdout/stderr routing comparison, and most launch facets. The two matrix rows
that expose loop and launch-state inequivalence are ignored. In this review's
macOS run, three active Level 2 proxy lifecycle tests timed out on all four
attempts:

- `level2_lifecycle_proxy_target_initialize_shell_is_gated_before_dispatch`
- `level2_lifecycle_proxy_target_later_event_shell_is_audited_after_stabilization`
- `level2_lifecycle_proxy_target_lifecycle_parse_failure_fires_no_catch_events`

Those tests are cited as evidence for AC 11, AC 17, and AC 5 respectively, so
the acceptance map currently overstates their passing status. Finally,
Claudine has no CI job that runs `just test-l2`; the suite is Unix-only and its
shell fixtures are not portable to Windows. This does not satisfy the feature
plan's cross-platform sign-off.

**Required change:** make every required matrix row active and green; add real
pane-text assertions for rendered/redacted output; diagnose the three hangs;
and add a supported CI L2 leg. Either revise the cross-platform acceptance plan
to match the repository's ratified L2 policy or provide a portable harness for
the platforms the plan names.

### 6. Medium: The intended active-document state model is not wired into the runtime

The library defines and tests `ActiveDocumentState`, `DocumentIteration`, and
`ProviderAttempt` (`lib/src/composition/coordinator/active.rs`), but production
CLI code does not use them. The harness maintains parallel prompt, budget,
session, and attempt state instead. Unit tests of the library model can therefore
pass without proving the runtime enforces the same ownership and reset rules.

This duplication has already contributed to the missing loop and resume
transitions. Wire the state model into the coordinator or remove it and test the
actual runtime state machine directly; maintaining two sources of truth is not
a durable design.

## Requirement Verification Levels

| User-observable requirement | Acceptance criteria | Strongest verification present | Assessment |
|---|---:|---|---|
| Canonical target preparation, launch state, lifecycle, loop ownership, and closure | 1-17 | Level 2 tmux/event-log cases | **Gap:** AC 7 and AC 9-10 fail in ignored reproductions; AC 15 has no implementation; three cited L2 cases time out. |
| `proxy.with` parsing, typed evaluation, precedence, schema/policy participation, lifetime, and no-write behavior | 18-27 | Level 1 plus selected Level 2 provider fixtures | Mostly present, but AC 26 lacks end-to-end loop refresh and the forwarding-chain/cross-repo matrix rows are absent. |
| Typed handoff failure identity and event-aware closure | 28-29 | Level 1 cross-route identity plus selected Level 2 errors | Partial: the lifecycle-parse Level 2 case times out, and full rendered cross-route comparison is absent. |
| Redacted status and diagnostics through terminal components | 30 | Level 1 redaction/static guards; isolated Level 2 styled errors | **Wrong level:** no overlay-aware real-terminal pane assertion verifies the user-visible output. |

Level 3 is not applicable. The feature has no requirement involving physical
keyboard or mouse events, terminal input encoding, paste, or IME behavior.

## Verification Performed

- `just test`: **failed** at `claudine-gen::drift::committed_generated_artifacts_match_phase_1_byte_baseline`
  because its required baseline file is missing. Earlier package runs completed:
  catalog types 21 passed; library 3,518 passed and 7 skipped; contract 47 passed
  and 5 skipped; CLI 2,023 passed and 165 skipped; generator 109 passed before
  the failure.
- `just test-l2`: **failed** with 133 passed, 3 timed out, and 2,052 skipped.
  Each timeout reproduced across four attempts before the recipe exited 100.
- `just lint`: **passed** for all Claudine package-area crates, including the
  error-transport and lifecycle-document guards.
- Static review covered the specification, plan, acceptance map, canonical
  preparation and handoff pipeline, coordinator adoption, loop ownership,
  retry/resume dispatch, sequence preflight, and the Level 1/Level 2 inventory.

## Closure Criteria

Complete R6-R8 and the loop-ownership move; eliminate the remaining ambient
context capture; make all 30 acceptance criteria map to active passing tests at
the appropriate level; make `just test`, `just test-l2`, and `just lint` green;
and establish the agreed CI/platform coverage before requesting another
production-readiness review.
