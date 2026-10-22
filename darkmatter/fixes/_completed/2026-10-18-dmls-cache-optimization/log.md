## Implementation of Review Findings #1

> **started at:** 2026-10-18T18:52:52-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-18-dmls-cache-optimization/review-1.md'
- this is iteration 1 of the review-to-implement cycle

## Follow-up Pass for Review #1

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-18-dmls-cache-optimization/review-1.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-18-dmls-cache-optimization/spec.md'
- the review's `findings` list is empty: there are no high, medium, or low findings to repair
- the review's `observations` list holds one entry, which this pass carries to the backlog without repairing it

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| O1 | Existing fingerprint limitation for newly shadowing files | carried to backlog (observation) | Observations are not repaired in this pass. The spec also required the fingerprint to stay unchanged (R1), and this fix did not introduce the limitation. `inputs_fingerprint` in `lib/src/markdown/schemas/triggers/assemble.rs` hashes dependency bytes plus the listings of the schema roots and dependency folders only. |

### Outcome

- no production code, tests, or docs changed in this pass, so no focused tests, `just test`, or `just lint` run was needed; the review's own `just test` run (10,533 passed, 10 skipped) still describes the tree
- no cross-OS check was needed, since nothing changed
- backlog entry written: `darkmatter/fixes/_unscheduled/dmls-cache-optimization-follow-ups/spec.md`
        - it is the first spec in `darkmatter/fixes/_unscheduled/`, so its frontmatter follows the sibling `darkmatter/features/_unscheduled/` specs (`promote-cli-process-fixture`)
- review frontmatter set: `implemented: true`, `implemented_by: claude/opus`
