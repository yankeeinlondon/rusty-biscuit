## Implementation of Review Findings #1

> **started at:** 2026-10-20T16:59:10-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-20-strict-equality/review-1.md'
- this is iteration 1 of the review-to-implement cycle

## Follow-up Pass for Review #1

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-20-strict-equality/review-1.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-20-strict-equality/spec.md'
- reproduced the review's single observation before repairing it: `cargo nextest run -p claudine --features prompt-tests --lib -E 'test(/shipped_implement_plan/)'` failed with `left: Some(String(""))`, `right: Some(Null)`
- repaired `claudine/lib/src/composition/schema/tests.rs`: `shipped_implement_plan_prepares_with_unset_optional_commit_message` now expects the shipped prompt's declared `commit_message: ""` default and says why; absent/null behavior stays covered by the CLI's `implement_plan_commit_message` fixture tests
        - kept the test's name: the caller still leaves the message unset, and closed implementation logs cite the name
- sibling sweep (defect class: an opt-in prompt test whose expectation went stale against the live prompt):
        - `claudine` lib `prompt-tests`, every shipped/prompt test: **348/348 passed**
        - root `just test-prompts`: `claudine` 2/2, `claudine-cli` prompts 87/87, `claudine-cli` L2 `shipped_implement` 3/3, `darkmatter` prompts 10/10, all passing
        - the root step "schema validation" still fails: 34 `is a required property` and 24 `invalid-reference` problems from validating draft invocation templates without their arguments. None involves equality or a test expectation, so this is not the same defect class (see ledger row O1)

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| O1 | Opt-in prompt test expects an obsolete null default | **fixed** | Reproduced first (failed on `""` vs `null`), then fixed. The focused run passes 2/2, lib `prompt-tests` 348/348, root `just test-prompts` test steps all pass. The review also mentions schema-validation failures, which are draft `prompts/` templates validated without required arguments. They are not test defects, and `prompts/` is draft territory that is never tested in CI. |

### Gates

- `claudine/` `just test`: **10606/10606 passed** (9 skipped)
- `claudine/` `just lint`: clean; also `cargo clippy -p claudine --features prompt-tests --lib --tests -- -D warnings`: clean (the edited test is feature-gated)
- `darkmatter/`: no files changed in this pass, so no gate rerun
- no cross-OS check: this is a test-expectation edit with no path, process, or platform logic

> **completed at:** 2026-10-20
