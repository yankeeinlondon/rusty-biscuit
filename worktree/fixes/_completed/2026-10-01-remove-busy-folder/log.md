---
fix: 2026-10-01-remove-busy-folder
implementation_1: "2026-10-11T17:09:52-07:00"
---

# Remove Busy Folder Log

## Implementation of Review Findings #1

> **started at:** 2026-10-11T17:09:52-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'worktree/fixes/2026-10-01-remove-busy-folder/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Automatic recovery success lacks the specified public-path tests' at 17:11:30
        - defect class: automated tests do not verify a successful outcome of the failed-git recovery decision through the removal entry point
        - discovered: `recorder::fail_matching` skips Git entirely, so it cannot make Git delete and unregister the worktree and then report failure; the removal code calls Git through `git_from`
        - discovered: `worktree-cli` enables `worktree`'s `count-git` feature in normal dependencies, so `recorder` ships in `wt`; the new seam is therefore `#[cfg(test)]`, not behind `count-git`
        - added `git::recorder::fail_after_running(predicate, aftermath)` (test-only): runs the real Git, then the aftermath, then returns an error; portable and timing-free
        - added library L1 tests through `remove_worktree` (`worktree/lib/src/remove/mod.rs`):
                - `a_failed_git_removal_that_unregistered_and_deleted_the_folder_succeeds`
                - `a_failed_git_removal_that_left_deletable_remnants_is_finished`
                - `a_failed_git_removal_that_left_the_worktree_registered_is_an_error`
        - added CLI L1 test `remove::a_folder_refilled_while_git_removes_it_is_finished_and_the_branch_deleted` (`worktree/cli/tests/remove.rs`, Unix only): a `git` script on the spawned `wt`'s `PATH` runs the real removal, recreates `target/flycheck0/x`, and exits 1; asserts exit 0, folder and registration gone, branch deleted
                - Unix only because `wt` resolves only `git.exe` on Windows; the library tests cover the same decision on every OS
        - mutation proof: making `remove_worktree` return Git's error after a successful cleanup failed all three success-path tests
        - class sweep: failed-git recovery outcomes lacking entry-point tests; sites checked: registered → error, unregistered + gone → Ok, unregistered + deletable remnants → Ok, unregistered + undeletable → `FolderNotFullyRemoved`, never registered → error with nothing deleted, CLI route into the branch step; fixed here: unregistered + gone, unregistered + deletable remnants, registered → error (now asserts registration retained), CLI branch step; clean: unregistered + undeletable (`a_folder_git_unregistered_but_left_behind_is_reported_then_finished`, Unix only because it relies on permissions), never registered (`an_unregistered_folder_is_never_deleted`)
        - `just test` (worktree): 1102 passed, 14 skipped; `just lint`: passed (one earlier run exited 1 with no output, then passed twice unchanged); `just check-tier-coverage worktree`: 0 stranded
- work completed for 'Automatic recovery success lacks the specified public-path tests' at 17:17:31
- starting the work on 'Current removal documentation contradicts the implemented recovery' (observation) at 17:17:40
        - defect class: a current-behavior section of a `docs/` page describes removal behavior that the implementation has since replaced
        - handled directly by the orchestrator (documentation only, no code change)
        - rewrote "Background programs writing into the worktree" in `worktree/docs/cli/remove.md`: the all-OS process check (exit 4, names programs, no flag overrides, checked before the report and again before Git), its visibility limits, the post-failure decision table (registered / unregistered and gone / unregistered with remnants), and the `FolderNotFullyRemoved` exit 1 with its manual cleanup command
        - replaced the Windows-only busy-folder sentence in "Standing in the worktree you remove" with the all-OS process refusal plus the Windows open-file probe
        - updated the exit-code table's exit 4 row, which said a folder in use was a Windows-only case
        - checked the documented error text against `in_use_message` and `FolderNotFullyRemoved` in `worktree/lib/src/error.rs`
        - left the "Planned: process usage and recoverable removal" section unchanged, as the review directed
        - class sweep: current-behavior docs contradicting the busy-folder fix; sites checked: `worktree/docs/cli/remove.md` (background programs section, standing-in section, exit-code table, planned section), `worktree/README.md` remove bullets; fixed here: the three current-behavior sites in `remove.md`; clean: README (already describes the fix), planned section (not current behavior)
        - no test reads `docs/cli/remove.md`, so no test scope changes
- work completed for 'Current removal documentation contradicts the implemented recovery' (observation) at 17:18:18

### Successful Completion

The implementation of review cycle 1 has completed successfully in about 9 minutes (17:09:52 → 17:18:18). During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred, 0 were disputed (see reasons below):

- no findings were deferred
- no findings were disputed
- the 2 items are the high finding "Automatic recovery success lacks the specified public-path tests" and the observation "Current removal documentation contradicts the implemented recovery"
- the files changed in this cycle are `worktree/lib/src/git.rs`, `worktree/lib/src/remove/mod.rs`, `worktree/cli/tests/remove.rs`, and `worktree/docs/cli/remove.md`
