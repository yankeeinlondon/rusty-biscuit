---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T17:56:00-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-15.md
previous: 2026-07-13-proxy-with/review-14.md
next: 2026-07-13-proxy-with/review-16.md
---

# Review 15: Proxy With

## Verdict

The feature is **not ready for production**. Review 14's two Level 1 failures
are closed, the complete Claudine Level 1 gate and lint gate pass, and this
review found no remaining implementation defect in the specified handoff,
overlay, launch-rebuild, loop, or diagnostic behavior.

The remaining blocker is verification rather than implementation. Review 11
added behavior at user-observable process boundaries together with the correct
Level 2 tests, but no repository review or gate record shows those new rows
passing. Reviews 12 through 14 did not complete a Level 2 run. This review also
could not execute the Level 2 suite because the managed sandbox rejects the
tmux and WezTerm socket creation required by the real-terminal harness. Test
definitions at the correct tier are not a substitute for a recorded successful
execution, so the affected acceptance criteria do not yet have current Level 2
evidence.

## Findings

### 1. High — Review 11's user-observable re-entry closures have no recorded green Level 2 execution

Review 11 changed four behaviors that are only fully observable at child
launch or through surfaced terminal output:

- the initial direct composition keeps a file-backed system prompt readable by
  the spawned provider;
- retrying into an unavailable refreshed provider follows normal typed
  selection and does not spawn a child;
- interpolated/composed MCP tags reach the retried provider launch; and
- warnings are recalculated and surfaced for the rebuilt provider.

The corresponding Level 2 rows are present in
`cli/tests/level2_lifecycle_control.rs`:

- `level2_lifecycle_direct_compose_delivers_a_readable_system_prompt_file`;
- `level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`;
- `level2_lifecycle_retry_keeps_an_interpolated_mcp_tag_at_child_launch`;
- `level2_lifecycle_switch_surfaces_unsupported_system_prompt_warning`; and
- `level2_lifecycle_switch_surfaces_unsupported_sandbox_warning`.

That is the appropriate verification level, but these rows were added after
the acceptance map's recorded review-10 Level 2 run. Review 12 stopped before
Level 2 after a cold build, review 13 explicitly did not rerun Level 2 because
no Level 2 implementation or test had changed in that iteration, and review 14
stopped at the red Level 1 gate. Consequently, the current implementation has
no recorded run proving these process-boundary behaviors in a real terminal.

Run `just test-l2` from `claudine/` on the Linux CI runner or an unsandboxed
host with a reachable tmux backend. Record a green result for all five rows
above and the existing proxy-with Level 2 matrix before changing `ready` to
`true`. A backend or harness failure is not an implementation failure, but it
does leave the production-readiness evidence incomplete.

### 2. Medium — The acceptance map identifies pre-review-11 evidence as current

`notes/acceptance-map.md` still labels review 10 findings as “current” and its
recorded gate section is explicitly for the HEAD carrying review-10 findings
1–5. The individual acceptance rows were extended with review-11 test names,
but the recorded execution evidence was not. This makes the document appear to
close newer acceptance evidence with an older run that could not have executed
those tests.

After the required Level 2 run, update the acceptance map's current-review
label and recorded gate results with the tested revision, backend, command,
counts, and disposition of any skipped or backend-unreachable rows. Keep the
review-10 result as historical evidence if useful, but do not present it as the
current feature gate.

## Review 14 Closure

Both prior findings are resolved:

1. `loop_and_lifecycle_agree_on_shared_syntax` now passes alone in 3.085s, and
   `loop_initialize_stop_proceeds_into_iterations` passes alone in 6.106s,
   both well inside nextest's 30-second timeout.
2. `nested_ctx_refs_resolve_against_one_captured_snapshot` now passes in
   6.071s and uses the capture-count hook to assert one shared discovery while
   also resolving both `ctx.os` and `ctx.agent`. It would fail if the
   implementation regressed to one capture per overlay leaf.

## Verification-Level Audit

| User-facing requirement | Required | Strongest present | Review result |
|---|:---:|:---:|---|
| Direct/proxy equivalence for composed content, frontmatter, context, lifecycle, loops, and child launch identity | L2 | L2 test matrix | Existing rows have historical L2 evidence; current full-suite rerun is blocked by the harness |
| Direct file-backed system-prompt lifetime at spawned child | L2 | L2 test defined | **Gap:** no recorded successful run after the row was added |
| Refreshed unavailable-provider selection and no child spawn | L2 | L2 test defined | **Gap:** no recorded successful run after the row was added |
| Prepared/interpolated MCP tags at retried child launch | L2 | L2 test defined | **Gap:** no recorded successful run after the row was added |
| Rebuilt-provider capability warnings in terminal output | L2 | L2 tests defined | **Gap:** no recorded successful run after the rows were added |
| `with:` overlay parsing, interpolation, merge, one-capture snapshot, explicit `null`, list replacement, and immutability | L1 | L1 unit/integration tests | Green in the current full Level 1 run |
| Proxy/sequence/closure diagnostics, dry-run output, shell safety, and redaction | L2 | L2 test matrix | Correct tier is present; no new implementation gap found |
| Physical keyboard/modifier encoding | L3 | Not applicable | The feature specifies no keyboard-driven behavior |

The Level 2 classifications above cover real child process behavior and
terminal-rendered diagnostics. Level 3 is not required because no acceptance
criterion depends on an OS keyboard event or terminal input encoder.

## Validation Performed

- `just test` from `claudine/`: exit 0. The library reported 3532/3532 passed
  with 7 skipped, the CLI reported 2105/2105 passed with 231 skipped, and the
  generator reported 152/152 passed with 4 skipped; the remaining area crates
  also completed successfully.
- `just lint` from `claudine/`: exit 0 for all Claudine crates, including the
  error-transport and lifecycle-documentation guards.
- Focused review-14 regressions: all three rows listed in “Review 14 Closure”
  passed without retry.
- `just test-l2` from `claudine/`: the harness failed before feature
  assertions. Six initial tests failed to create tmux sessions, WezTerm could
  not create its mux socket or pid file under the managed home directories,
  and fail-fast canceled the remaining 198 tests. An isolated tmux probe using
  `/private/tmp` was also rejected with `Operation not permitted`. These are
  sandbox backend failures, not proxy-with assertion failures, so they neither
  prove nor disprove the implementation; they prevent closing Finding 1 here.

## Production-Readiness Condition

No further code change is recommended from this review. The feature can be
marked production-ready once an unsandboxed `just test-l2` run records the five
review-11 rows and the existing proxy-with Level 2 matrix green, and the
acceptance map is updated to identify that current evidence.
