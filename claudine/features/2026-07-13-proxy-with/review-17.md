---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T20:24:45-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-17.md
previous: 2026-07-13-proxy-with/review-16.md
---

# Review 17: Proxy With

## Verdict

The feature is **not ready for production**. Review 16's Level 1
system-prompt-lifetime regression is now green, including in the same focused
four-row gate that covers the three supporting launch-rebuild projections.
The remaining blocker is the required Level 2 evidence: all five post-review-11
rows still fail at the tmux backend boundary before a feature assertion runs.

There is no implementation or test commit after review 16 at this HEAD. Source
inspection found no additional defect in handoff ownership, overlay evaluation,
canonical preparation, retry/resume launch rebuilding, closure ownership, or
diagnostic transport. The Level 1 library suite and the feature-owned structural
guards are green, but those results cannot substitute for real-terminal
verification of spawned-child state and pane-rendered diagnostics.

## Findings

### 1. High — Required current Level 2 evidence is still absent

The specification requires Level 2 verification for direct/proxy equivalence,
spawned-child launch state, system-prompt file lifetime, target-selected MCP
state, rendered capability warnings, shell approval/execution, diagnostics,
redaction, closure ownership, and dry-run pane behavior. The five rows added to
close review 11's remaining re-entry-boundary gaps are:

- `level2_lifecycle_direct_compose_delivers_a_readable_system_prompt_file`;
- `level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`;
- `level2_lifecycle_retry_keeps_an_interpolated_mcp_tag_at_child_launch`;
- `level2_lifecycle_switch_surfaces_unsupported_system_prompt_warning`; and
- `level2_lifecycle_switch_surfaces_unsupported_sandbox_warning`.

Targeted run `b40a8393-5d95-409b-9446-bf6822d78778` selected exactly those five
rows: 0 passed, 5 failed, and 2331 were skipped. Every failure occurred while
tmux tried to access `/private/tmp/tmux-501/default` and reported `Operation not
permitted`; no feature assertion ran.

An isolated-socket probe set `TMUX_TMPDIR` to a task-owned writable directory.
Run `052f38d7-9398-4a1c-bf14-4d051820fcdd` still failed before the selected
system-prompt assertion because this managed host denied creation of the tmux
Unix socket. This confirms a host/backend limitation, not a product failure,
but it supplies no Level 2 evidence.

Run these five rows and the existing proxy-with Level 2 matrix on the Linux CI
runner or another host where tmux can create and control its socket. Record the
tested revision, backend, command, pass/fail/skip counts, and any excluded rows
in `notes/acceptance-map.md` before changing `ready` to `true`.

## Review 16 Closure

1. **Level 2 execution evidence: not closed.** Both the normal and isolated
   tmux attempts failed before a feature assertion.
2. **Direct system-prompt lifetime Level 1 regression: closed.** Focused run
   `1d017bf3-d551-4e1b-a97c-9910fb130fdf` passed all four review-16 backing
   rows. The direct child-spawn regression passed in 0.565 seconds; the
   unavailable-provider, prepared-MCP-tag, and rebuilt-warning projections also
   passed.

## Verification-Level Audit

| User-observable requirement | Required | Strongest defined | Current evidence |
|---|:---:|:---:|---|
| Direct/proxy equivalence for content, context, lifecycle, loops, closure, and launch identity | L2 | L2 matrix | Historical rows exist; no current full-matrix run |
| Direct file-backed system-prompt bytes remain readable at child spawn | L2 | L2 plus L1 process regression | L1 green; **L2 gap** |
| Refreshed unavailable-provider selection refuses before spawn | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| Prepared/interpolated MCP tags reach the retried child | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| Rebuilt-provider capability warnings render to the terminal | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| `proxy.with` parsing, typed interpolation, precedence, lifetime, and immutability | L1 | L1 unit/integration matrix | Current library suite green |
| Shell approval/execution, diagnostics, redaction, inline/sequence closure, and dry-run pane behavior | L2 | L2 matrix | Correct tier is defined; current full-matrix evidence remains incomplete |
| OS keyboard encoding | L3 | Not applicable | No requirement depends on physical keyboard input |

Level 3 is not required because this feature defines no keyboard-, mouse-,
paste-, or IME-driven behavior.

## Validation Performed

- Confirmed that current HEAD `5019f6e5f` is the review-16 documentation commit
  and contains no later implementation or test change.
- Traced the production flow through the command-owned active-document
  coordinator, coordinator-only handoff commit, canonical preparation service,
  and per-attempt launch rebuild. No second target composer or optional proxy
  channel was found.
- Ran the four review-16 Level 1 rows: 4 passed, 2332 skipped.
- Ran the complete `claudine` library Level 1 suite: 3532 passed, 7 skipped.
  `claudine-catalog-types` passed 21/21 and `claudine-contract` passed 47/47
  with 5 skipped.
- Started the area-wide `just test` gate. It was stopped at the non-interactive
  60-second subprocess ceiling during unrelated CLI coverage; before the stop,
  412 CLI rows had passed, including the proxy composition-seam guards,
  handoff-ledger call-site guard, shipped-route drift guards, test-placement
  guard, and the direct system-prompt lifetime row. This is not claimed as a
  completed full Level 1 gate.
- Ran the five exact Level 2 blockers through `just test-l2`: 0 passed, 5
  backend failures, 2331 skipped; no feature assertion ran. Repeated one row
  with an isolated tmux socket directory; socket creation was still denied.
- Inspected all 30 acceptance-criterion mappings. User-observable requirements
  are assigned Level 2 rows; pure parser/state contracts are assigned Level 1
  rows. No Level 3 requirement applies.

## Production-Readiness Condition

Record a green `just test-l2` execution for the five review-11 rows and the
existing proxy-with Level 2 matrix on a reachable real-terminal backend. The
current implementation and Level 1 evidence reveal no additional production
code change required by this review.
