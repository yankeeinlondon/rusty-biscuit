---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T18:28:26-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: false
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-16.md
previous: 2026-07-13-proxy-with/review-15.md
---

# Review 16: Proxy With

## Verdict

The feature is **not ready for production**. Review 15's acceptance-map
correction is complete, but its high-severity Level 2 evidence gap remains
open. The five real-terminal rows added for review 11 still have no recorded
successful execution, and the targeted run in this review again failed at the
tmux backend boundary before any feature assertion ran.

The current Level 1 evidence also regressed during this review:
`direct_compose_keeps_its_file_backed_system_prompt_readable_at_spawn` timed
out on every 30-second attempt, including a separate isolated run. Three
sibling Level 1 projections for refreshed provider selection, prepared MCP
tags, and rebuilt-provider warnings passed. The timeout does not by itself
prove a production defect, but a red feature-owned regression cannot support a
production-ready verdict.

There is no implementation or test commit after review 15 at this HEAD; the
only intervening commit records review 15 and updates its documentation. This
review found no additional source-level defect in the handoff, overlay,
launch-rebuild, loop, or diagnostic implementation.

## Findings

### 1. High — The required post-review-11 Level 2 evidence is still absent

Review 15 required a green real-terminal execution for these rows:

- `level2_lifecycle_direct_compose_delivers_a_readable_system_prompt_file`;
- `level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`;
- `level2_lifecycle_retry_keeps_an_interpolated_mcp_tag_at_child_launch`;
- `level2_lifecycle_switch_surfaces_unsupported_system_prompt_warning`; and
- `level2_lifecycle_switch_surfaces_unsupported_sandbox_warning`.

They are correctly classified as Level 2 because they observe spawned-child
state or terminal-rendered diagnostics. In targeted run
`abd3eec5-650e-41f9-b4e4-57792e77f0a8`, all five exhausted their retries while
opening a tmux session. Every failure reported
`error connecting to /private/tmp/tmux-501/default (Operation not permitted)`;
zero feature assertions ran.

This is a harness/backend failure rather than evidence of a product failure.
It nevertheless leaves the user-observable requirements unverified at their
required level. Run the five rows and the existing proxy-with Level 2 matrix on
the Linux CI runner or another host where tmux is reachable, and record the
tested revision, backend, command, result counts, and skipped rows in
`notes/acceptance-map.md`.

### 2. High — The direct file-backed system-prompt lifetime Level 1 regression times out

The Level 1 regression
`direct_compose_keeps_its_file_backed_system_prompt_readable_at_spawn` spawns
the shipped `claudine` binary against fake Gemini and Codex executables, then
requires each child to read the bytes at its delivered system-prompt path. It
timed out on all four configured attempts in focused run
`e6a14f07-57b8-46a1-8d46-5ff55873fb9c`. A second run containing only that row,
with retries disabled, timed out again after 30 seconds
(`8fa29f4b-7357-4f28-a230-d316c563a2af`).

The first focused run's other three selected tests passed:

- `a_refreshed_unavailable_scalar_agent_refuses_like_direct_selection`;
- `body_mcp_tags_come_from_the_prepared_document_and_only_when_mcp_is_enabled`;
  and
- `the_rebuilt_bundle_carries_warnings_that_the_output_policy_gates`.

The timeout occurs after both temporary repositories are initialized and
leaves the Codex fixture workspace behind without an `events.log`, so the fake
Codex child did not record that it received the launch. Process inspection is
blocked by this managed host (`ps: operation not permitted`), and no production
or test code changed after the last recorded green Level 1 gate. The next
implementation pass should reproduce this row on an unrestricted host,
determine whether the stall is in selection/preparation, spawn, stdin closure,
or child reaping, and make the regression reliably green before rerunning the
full Level 1 gate.

## Review 15 Closure

1. **Level 2 execution evidence: not closed.** The five exact rows were selected
   in this review, but the backend denied tmux socket access before assertions.
2. **Acceptance-map currency: closed.** `notes/acceptance-map.md` now labels the
   review-15 gate as current, records its exact revision and gate results, and
   retains the older review-10 run as historical evidence.

## Verification-Level Audit

| User-observable requirement | Required | Strongest defined | Current evidence |
|---|:---:|:---:|---|
| Direct/proxy equivalence for content, context, lifecycle, loops, closure, and launch identity | L2 | L2 matrix | Historical rows exist; no current full-matrix run |
| Direct file-backed system-prompt bytes remain readable at child spawn | L2 | L2 plus L1 process regression | **Gap:** L2 cannot reach assertions; L1 currently times out |
| Refreshed unavailable-provider selection refuses before spawn | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| Prepared/interpolated MCP tags reach the retried child | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| Rebuilt-provider capability warnings render to the terminal | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| `proxy.with` parsing, typed interpolation, precedence, lifetime, and immutability | L1 | L1 unit/integration matrix | Recorded green at the unchanged implementation revision |
| Shell approval/execution, diagnostics, redaction, inline/sequence closure, and dry-run pane behavior | L2 | L2 matrix | Correct tier is defined; current full-matrix evidence remains incomplete |
| OS keyboard encoding | L3 | Not applicable | No requirement depends on physical keyboard input |

Level 3 is not required because this feature defines no keyboard-, mouse-,
paste-, or IME-driven behavior.

## Validation Performed

- Confirmed with Git history that the current HEAD after review 15 contains no
  implementation or test change; it only records review 15 and its metadata.
- Ran the five exact Level 2 blockers through `just test-l2`: 0 passed, 5
  backend failures, 2331 skipped; no feature assertion ran.
- Ran four focused Level 1 backing tests through `just test-cli`: 3 passed and
  the direct system-prompt lifetime regression timed out on all 4 attempts.
- Reran that timeout alone with retries disabled: 1 timed out after 30 seconds.
- Inspected the implementation and acceptance map against all 30 acceptance
  criteria and found no additional unmapped requirement or wrong-tier test.
- Did not rerun the complete Level 1 or lint gates: the focused Level 1 gate was
  already red, Level 2 was backend-blocked, and production/test source is
  unchanged from review 15's recorded green full Level 1 and lint runs.

## Production-Readiness Conditions

Before setting `ready: true`:

1. make the direct system-prompt lifetime Level 1 regression reliably green and
   rerun the complete `just test` and `just lint` gates; and
2. record a green `just test-l2` execution for the five review-11 rows and the
   existing proxy-with Level 2 matrix on a reachable real-terminal backend.
