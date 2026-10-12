---
deferred_perf_measurement: false
implementation_1: "2026-10-11T17:11:35-07:00"
---

# Worker Git Processes Log

## Implementation of Review Findings #1

> **started at:** 2026-10-11T17:11:35-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'worktree/fixes/2026-10-05-worker-git-processes/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'The process-count regression test does not cover the complete worker' at 17:11:47
        - defect class: a process-count regression test measures only one component of the execution path whose count it must protect, because work handed to other threads escapes the counting scope
        - class sweep: thread-escaping git work outside a count scope; sites checked: `refresh_worker::run_halves`, `list::gather` (local and history threads), `list` local gather and `regather`, `listing::compare_live`, `worktree` dirtiness and comparison threads, `graph` and `graph::topology` workers, `git` stdin writer, `live_remote::drain` pipe readers, `remove::safety::assess` PR lookup; fixed here: `refresh_worker::run_halves`; clean: every `list`/`listing`/`worktree`/`graph` site already carries `TaskHandle`, `git` stdin writer and `live_remote::drain` start no git, `remove::safety::assess` starts no git on its thread and no count scope covers `wt remove`
        - changed `run_halves` (`cli/src/commands/refresh_worker.rs`) to take `calls::TaskHandle::current()` and run each half through `handle.run`, merging the joined counts with `calls::joined`; its doc now states that a panicked half's count is lost; a no-op when no scope is open
        - added `refresh_worker::tests::a_fetching_worker_starts_only_the_checkout_lookup_ls_remote_and_fetch`: main checkout plus local bare origin, second commit pushed, `origin/main` moved back, `HOME`/`XDG_CACHE_HOME` pointed into the temp dir (with a drop guard that removes the worker's store files for Windows, where the cache ignores both), then the real `run`; asserts the tracking ref moved and exactly 3 git processes; it reported 1 with the propagation reverted; existing two-process component test kept
        - updated `docs/performance-testing.md` (Git processes) to describe the whole-worker test and the halves carrying the count scope
        - `just test`: 1104 passed, 14 skipped; `just lint`: passed
        - orchestrator cross-OS check of the new test (both compiled targets):
                - native Windows (`just cross-check worktree-cli --os windows`): 2 passed; this covers the Windows-only cleanup of the real per-user cache and confirms the count is 3 there, so `gix` did not fall back to git
                - Linux (`build-linux`): 2 passed on retry; the first attempt failed to compile under a different archive key, consistent with the shared dirty tree changing between runs, and it did not recur
- work completed for 'The process-count regression test does not cover the complete worker' at 17:29:22

### Successful Completion

The implementation of review cycle 1 has completed successfully in 18 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred, 0 were disputed (see reasons below):

- no findings were deferred
- no findings were disputed
