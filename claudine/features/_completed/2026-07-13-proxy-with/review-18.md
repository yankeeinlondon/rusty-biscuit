---
$schema: feature-review.yaml
ready: true
verification: blocked/awaiting Linux L2
agent: codex/default
created: 2026-07-20T20:50:07-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: false
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-18.md
previous: 2026-07-13-proxy-with/review-17.md
---

# Review 18: Proxy With

## Verdict

The feature is **not ready for production**. The implementation and test source
have not changed since review 17, and the required current Level 2 evidence is
still unavailable. At current HEAD, the five post-review-11 rows and the full
94-row proxy-with matrix again failed at the tmux backend boundary before any
feature assertion ran.

The focused Level 1 backing tests and structural drift guards are green. Source
and call-graph inspection found no new implementation defect in coordinator
ownership, handoff evaluation/commit, canonical preparation, retry/resume launch
rebuilding, closure ownership, or typed diagnostic transport. Those Level 1
results cannot substitute for the specification's real-terminal requirements.

## Findings

### 1. High — Required Level 2 acceptance evidence remains absent

The specification requires Level 2 verification for direct/proxy equivalence,
spawned-child launch state, system-prompt file lifetime, target-selected MCP
state, rendered capability warnings, shell approval/execution, diagnostics,
redaction, closure ownership, and dry-run pane behavior.

The five rows added after the last recorded green lifecycle matrix are:

- `level2_lifecycle_direct_compose_delivers_a_readable_system_prompt_file`;
- `level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`;
- `level2_lifecycle_retry_keeps_an_interpolated_mcp_tag_at_child_launch`;
- `level2_lifecycle_switch_surfaces_unsupported_system_prompt_warning`; and
- `level2_lifecycle_switch_surfaces_unsupported_sandbox_warning`.

Current run `61ce3b11-93b7-46b9-8682-92e2ad5607ed` selected exactly these five:
0 passed, 5 failed, and 2331 were skipped. Every row exhausted its four
attempts while tmux reported `Operation not permitted` for
`/private/tmp/tmux-501/default`; no feature assertion ran.

Current run `fb31f5e0-be4a-4c21-9934-4e87fffefd30` selected all 94 rows in
`level2_lifecycle_control`: 0 passed, 94 failed, and 0 were skipped within the
selected binary. Every failure occurred at the same tmux boundary. These are
host/backend failures, not product assertion failures, but they provide no
acceptance evidence.

Run both selections on the Linux CI runner or another host where tmux can
create and control its socket. Record the tested revision, backend, command,
pass/fail/skip counts, and excluded rows in `notes/acceptance-map.md` before
changing `ready` to `true`.

### 2. Low — The acceptance map misstates the Linux L2 gate

`notes/acceptance-map.md` says the purpose-built Linux job avoids
`BISCUIT_TEST_LEVEL_REQUIRED=2` and runs in a job that provisions the AI-CLI
provider stubs. The current workflow does the opposite on the first point:
`.github/workflows/claudine-tests.yml` sets
`BISCUIT_TEST_LEVEL_REQUIRED: "2"` on the dedicated `test-l2` job and installs
tmux explicitly. Its provider-stub step belongs to the separate Level 1 matrix,
not the L2 job. The map also still calls review 15 the current Level 2 run in
one historical-summary paragraph.

The workflow is the correct authority and is appropriately fail-closed for a
missing L2 harness. Update the acceptance map so the evidence record describes
the gate that actually runs; stale test-infrastructure claims make future
readiness decisions harder to audit.

## Review 17 Closure

1. **Level 2 execution evidence: not closed.** Both exact selections again
   failed before assertions because the managed host denied tmux socket access.
2. **Level 1 backing evidence: remains green.** Current run
   `815791b0-4dfc-4f8b-b58e-d334bee5395d` passed the direct file-backed
   system-prompt lifetime regression and the unavailable-provider, prepared-MCP,
   and rebuilt-warning projections (4/4).
3. **No implementation delta exists to review.** HEAD `fc88cc9e5` is the
   review-17 documentation commit; no production or test commit follows it.
   Review 17 did not identify a code change that could replace the missing L2
   evidence.

## Verification-Level Audit

| User-observable requirement | Required | Strongest defined | Current evidence |
|---|:---:|:---:|---|
| Direct/proxy equivalence for content, context, lifecycle, loops, closure, and launch identity | L2 | L2 matrix | Correct tier exists; current 94-row run reached no assertions |
| Direct file-backed system-prompt bytes remain readable at child spawn | L2 | L2 plus L1 process regression | L1 green; **L2 gap** |
| Refreshed unavailable-provider selection refuses before spawn | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| Prepared/interpolated MCP tags reach the retried child | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| Rebuilt-provider capability warnings render to the terminal | L2 | L2 plus L1 projection | L1 green; **L2 gap** |
| `proxy.with` parsing, typed interpolation, precedence, lifetime, and immutability | L1 | L1 unit/integration matrix | Feature-owned rows green in the current partial library run |
| Shell approval/execution, diagnostics, redaction, inline/sequence closure, and dry-run pane behavior | L2 | L2 matrix | Correct tier exists; current run reached no assertions |
| OS keyboard encoding | L3 | Not applicable | No requirement depends on physical keyboard input |

Level 3 is not required because this feature defines no keyboard-, mouse-,
paste-, or IME-driven behavior.

## Validation Performed

- Read the specification, review 17, the complete 30-criterion acceptance map,
  the lifecycle/composition documentation, and the Claudine architecture and
  test-placement guidance.
- Confirmed that no production or test commit follows review 17 and preserved
  the unrelated pre-existing `CLAUDE.md` worktree edit.
- GitNexus compared the branch with `main`: the long-lived branch-wide diff is
  critical (6007 changed symbols across 481 files and 42 affected flows). The
  narrowed coordinator `adopt` path is high risk with 13 upstream dependents;
  direct active-document preparation and target launch rebuilding are low-risk
  individually. Direct callers and feature-owned tests are present for each
  narrowed path.
- Ran the five exact Level 2 blockers: 0 passed, 5 backend failures, 2331
  skipped; no feature assertion ran.
- Ran the complete 94-row proxy-with Level 2 binary: 0 passed, 94 backend
  failures; no feature assertion ran.
- Ran four focused Level 1 backing rows: 4 passed, 2332 skipped.
- Ran six structural guards covering the typed handoff census, explicit
  preparation context, optional-target-channel baseline, handoff-ledger wiring,
  transition-path rendering, and test placement: 6 passed, 2330 skipped.
- Started the complete `claudine` library Level 1 suite. It was interrupted at
  the non-interactive 60-second ceiling after 937/3532 rows passed with no
  failures; 2595 did not run. This is partial evidence, not a completed gate.

## Production-Readiness Conditions

Before setting `ready: true`:

1. record a green run of the five post-review-11 rows and the complete
   proxy-with Level 2 matrix on a reachable real-terminal backend; and
2. correct the stale Linux L2 workflow description in
   `notes/acceptance-map.md` and record the successful run there.
