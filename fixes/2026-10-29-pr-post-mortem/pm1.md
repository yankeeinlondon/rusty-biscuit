---
branch: feat/pipeline
---
These were primarily integration-readiness, portability, and test-harness failures. The most expensive part was discovering inexpensive defects late—and my handling of the validation sequence made that worse.

| Failure | Why it happened | Characterization |
|---|---|---|
| Scope tests expected README edits to schedule nothing | A new guard read Darkmatter’s README, but the scope-test expectations still assumed it had no readers. | Stale test assumptions; the planner behaved correctly. |
| Archive-path guard failed | Three test/benchmark sites used compile-time CARGO_MANIFEST_DIR, capturing the producer’s checkout path. | A genuine portability defect in verification code. |
| Darkmatter CLI lint failed, also affecting Claudine CLI | run_render grew beyond Clippy’s argument limit without the allowance used by neighboring handlers. | A deterministic readiness failure that should have been caught locally. |
| Consolidation self-test failed | Claudine’s CI features gained work-counters, while the reviewed feature-capture table remained unchanged. | Metadata drift between two maintained contracts. |
| WezTerm commands timed out | The failures were concurrency-sensitive; the complete suite passed with two workers and independent panes. | Harness resource contention. We did not isolate the exact GUI bottleneck. |
| Serial tmux runs timed out | Setting one worker also enabled shared-pane reuse. Tests inherited state from earlier tests. | An isolation problem, exposed by changing execution mode. |

The editor fixture now owns its TTY, and its previously failing two-test sequence passes. Shared-pane serial mode still exposed another timeout, however. We did not establish that mode as generally reliable. The complete 289-test suite passed with independent panes, including both required terminal backends.

I made two avoidable process mistakes:

- After the first local failure, I switched to scope-only instead of completing strict validation. That published scope information without passing validation evidence, so macOS correctly ran in CI.
- I treated one worker as a concurrency adjustment before checking that it also changed pane isolation. That introduced another failure mode into the investigation.

I also missed the consolidation failure printed earlier in a local run that continued for 74 minutes. The hook preserved useful passing evidence, but the blocker should have received attention immediately.

Evidence reuse itself worked: the final CI plan reused 20 macOS cells and three narrowed test-input cells.

I would prioritize these improvements:

1. Establish a cheap readiness phase before expensive validation. Check scope expectations, feature-table consistency, archive paths, and lint early. Stop on invalid planning/evidence prerequisites; retain completed cell evidence when ordinary package tests fail.
2. Separate concurrency from isolation. Worker count should control parallelism. Shared-pane reuse should be an explicit, separate choice. Until then, use the documented two-worker independent-pane configuration on this host, and address shared-pane reliability as a distinct harness issue.
3. Make contract maintenance part of feature changes. Changes to CI features should include their capture-table update. Generic “unread documentation schedules nothing” tests should use controlled fixture workspaces; real-repository tests should pin intentional readers. This reduces accidental coupling to whichever README happens to be unread today.
4. Let eligible lint checks run earlier in CI. The final Linux archive build took about 76 minutes, and area jobs currently wait for the build (.github/workflows/ci.yml:817). Planned lint cells that need no archive could run alongside archive production, preserving the same scope and ownership without adding duplicate coverage.
5. Make failure status immediate and explicit. Print each infrastructure suite’s start, finish, elapsed time, and retained log location. Progress updates should lead with the current blocker—“PR red; fixes committed locally; push blocked by L2”—so passing checks do not obscure the unresolved failure.
