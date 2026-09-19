---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-15T21:47:11-07:00
spec: corrected-perf-flag/spec.md
implemented: false
description: "A **fix** review of `corrected-perf-flag/spec.md`"
fix: corrected-perf-flag/review-3.md
previous: corrected-perf-flag/review-2.md
---

# Review 3 — Corrected Perf Flag

## Verdict

The fix is **ready for production**. Review 2's only unblocked finding is
implemented: both ordinary Level 1 width-sweep regressions now retain the
recorded failure band and boundary cases without exhaustively rendering every
larger width, and the component test has the missing non-vacuity guard.
Both tests remain below the five-second slow-test boundary in this review's
isolated-target runs.

The implementation remains complete against R1–R4. The full Sniff and
`biscuit-terminal` Level 1 suites and both lint gates pass. The Level 2 tmux
coverage accepted in review 2 is unchanged and remains the strongest evidence
for terminal-emulator rendering. No requirement concerns keyboard, mouse,
paste, or another OS-input encoder path, so Level 3 is not applicable. No human
review is required.

## Prior Review Closure

- **Regression width sweeps make two ordinary L1 tests slow — closed.** The
  component and Sniff tests now render 48 cases each instead of 242: Unicode
  and ASCII at width 20 and widths 27 through 49
  ([`metrics_tree.rs:681`](../../../biscuit-terminal/lib/src/components/metrics_tree.rs#L681),
  [`perf_tree.rs:888`](../../cli/src/output/perf_tree.rs#L888)). The component
  test now asserts that at least one retained case truncates immediately after
  an underscore; the Sniff test retains its equivalent guard. In an isolated
  target, the focused tests completed in 1.25 seconds and 2.49 seconds.
- The closure is non-vacuous. Temporarily reinstating the original unescaped
  label path made the component regression fail at width 39 with unequal row
  widths and the Sniff production-key regression fail at width 27. Restoring
  the escape made the complete suites green.
- Review 2 had no `## Blocked Findings` section and no deferred finding, so
  nothing became newly unblocked before this implementation cycle.

## Findings

No findings.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| R1 — Preserve collection, worker propagation, and request isolation | Level 1 library unit and integration tests in the complete Sniff suite | Correct level; all pass and the collector seam is unchanged. |
| R2.1 — Root, dotted hierarchy, aliases, values, calls, and ordering | Level 1 pure projection tests plus Level 2 real-tmux hierarchy capture | Correct level. |
| R2.2 — Wall-clock shares and overlap note | Level 1 calculation/render tests plus Level 2 real-tmux italic-note capture | Correct level. |
| R2.3 — Exactly one deterministic HOT measured stage | Level 1 selection tests plus Level 2 real-tmux SGR/HOT-row capture | Correct level. |
| R3 — Separate generic counter tree with count ordering and unknown shares | Level 1 projection and real-binary tests plus Level 2 real-tmux tree/alignment capture | Correct level. |
| R4 — stdout/stderr routing, valid JSON stdout, and ANSI-free `--plain` | Level 1 real-binary subprocess tests | Correct level; these are stream and serialization contracts, not emulator behavior. |
| R2/R4 — Glyphs, SGR styling, display-column alignment, wrapping, and narrow-width truncation | Level 2 real-tmux pane capture over widths 30–50 | Correct level; the Level 2 target is unchanged by this cycle. |
| Keyboard, hotkey, paste, IME, or mouse behavior | No changed requirement | Level 3 is not applicable. |

## Validation Performed

- `CARGO_TARGET_DIR=/tmp/codex-corrected-perf-review3-target just test` in
  `biscuit-terminal`: 3,262 tests passed, 0 failed, 55 tier skips.
- The same isolated-target `just test` in `sniff`: 2,661 tests passed, 0
  failed, 24 tier skips.
- `just lint` passed in both affected package areas with the isolated target.
- Focused post-fix runs passed in 1.25 seconds for the component regression and
  2.49 seconds for the Sniff regression. Mutation runs with the escaping fix
  removed made both tests fail on retained widths, proving the narrowed cases
  still distinguish the original defect.
- Review 2's required tmux Level 2 run remains applicable: four tmux tests
  passed with backend proof, including both perf-tree tests. This cycle did not
  change production rendering or the Level 2 target, so rerunning that
  unchanged external boundary was unnecessary.
- GitNexus was refreshed at `79ba35f`. It reports HIGH upstream impact for
  `MetricsTree::build_markup`: three direct renderer entry points and the
  indirect Claudine and Worktree CLI perf consumers. Review 2 already recorded
  green targeted downstream suites; the implementation-cycle diff changes
  only the two Level 1 test bodies and review metadata.
- `git diff --check` passed for every file owned by this review.

## Production Readiness

Ready. The specified behavior is implemented, the prior findings are closed,
and every user-observable requirement has verification at the appropriate
level.
