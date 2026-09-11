# Deferred performance measurement

One performance clause from [`review-1.md`](review-1.md) could not be completed
in review cycle 1. It is recorded here so the next cycle can pick it up without
re-deriving the context.

**Important:** the reason is *not* host CPU load. The local half of the same
finding **was** measured successfully in this cycle — see
[`measurement/review1-alternating/summary.md`](measurement/review1-alternating/summary.md).
The log's `deferred_perf_measurement` frontmatter is therefore `false`.

## What is deferred

| Field | Value |
|---|---|
| Source review | [`review-1.md`](review-1.md) |
| Finding | **High — The recorded comparison does not prove the faster-tests outcome** |
| Deferred clause | "…then complete the matched CI samples and ratify the per-family timing budgets." |
| Cycle | Review-to-implement iteration 1, 2026-09-09 |
| Acceptance criterion | AC8 (per-family timing budgets); the CI half of AC6 is superseded by Ken's `DECISION` |

## Why it is deferred

Ratifying per-family timing budgets requires three consecutive candidate CI runs
for each declared environment (Ubuntu, macOS, native Windows, WSL2), compared
only within compatible OS/runner/request/counter versions. That needs the branch
pushed to `origin`.

The branch is not pushed, and pushing is a human-gated step this session is not
authorized to take — it runs non-interactively and cannot obtain the signing or
remote credentials a push would require. The newest branch CI run does not
contain the candidate, and baseline run `34008778001` is a single baseline
sample rather than a candidate one.

No local substitute exists: per-family budgets are defined against CI runner
cells, and this cycle's local measurement deliberately compares only within the
one macOS host it ran on.

## What was completed instead

The local half of the finding was run in full and is the more consequential
result:

- Clean, pinned baseline (`c2dee9217`, preserved detached worktree) against a
  freshly pinned candidate (`fe83e7481` plus this fix's Sniff diff only).
- Five alternating warm rounds per cohort, twenty measured runs, all
  compile-free, per-run load and sustained-idle samples recorded.
- **Result: negative.** The full L1 cohort shows no demonstrated change; the
  `sanity` cohort is slower in 5 of 5 paired rounds. Attribution and the drift
  bracket are in the summary artifact.

## How to close this

After a human pushes `fix/cli-slow-tests`:

1. Collect three consecutive green runs per declared environment leg.
2. Join them to the family tree with `tools/test-audit` and compare matched
   tests per environment — native Windows and WSL2 are distinct cells.
3. Ratify or revise the per-family budgets, and flip AC8 in
   [`results.md`](results.md).

Do not re-run the local gates first; they are green and recorded. Reconcile the
ledger instead.
