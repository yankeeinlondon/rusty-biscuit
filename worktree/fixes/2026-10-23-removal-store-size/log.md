## Implementation of Review Findings #1

> **started at:** 2026-10-23T14:44:11-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/review-1.md'
- this is iteration 1 of the review-to-implement cycle 

## Follow-up Pass for Review #1

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/review-1.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/spec.md'
- the review's `findings` list is empty: there are no medium or low findings to repair
- its `observations` list has two entries; this pass does not repair observations, so both are carried to the backlog

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| O1 | Existing unreadable-journal diagnostics omit the journal filename | backlog | Observation; the behavior predates this fix (`lib/src/remove/recovery/journal.rs`, `JournalInvalid::Malformed`), and the exit-class mismatch (1 vs 3) needs a decision on which class is right |
| O2 | Existing graph tests exceed the L1 time budget | backlog | Observation; the 21 slow tests are in graph code outside this fix's changes |

### Outcome

- no code changed, so there were no focused tests to run and no package area to re-run `just test` / `just lint` in; the review's own run (1,636 passed, lint clean) still describes the tree
- no cross-OS check is needed: nothing changed
- backlog entry written at `worktree/fixes/_unscheduled/removal-store-size-follow-ups/spec.md` with one section per observation
- set `implemented: true` and `implemented_by: claude/opus` on `review-1.md`
