---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: the non-image full-command SLA test does not enforce the 1-second SLA

Acceptance criterion 7 requires warm-cache `wt list` on a non-image terminal to meet the repo-wide 1-second SLA, excluding rasterization. The new integration test is named and documented as the SLA guard, but it asserts `best < 1500ms` instead of `< 1000ms` ([`worktree/cli/tests/perf_command_sla.rs:55`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/tests/perf_command_sla.rs:55), [`worktree/cli/tests/perf_command_sla.rs:57`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/tests/perf_command_sla.rs:57)). The performance doc repeats the same mismatch: it says the test asserts the 1-second SLA, then explains that the checked bound is 1.5 seconds ([`worktree/docs/performance-testing.md:60`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/docs/performance-testing.md:60), [`worktree/docs/performance-testing.md:62`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/docs/performance-testing.md:62)).

That means a regression to 1.4 seconds would pass the production gate even though it violates the spec. If parallel-test contention makes a strict wall-clock assertion too noisy, keep the tolerant measurement as an observability test but add a deterministic subprocess-count bound, a serial/non-parallel performance recipe, or a separately gated SLA check that actually fails above 1 second.

### High: the image-terminal `wt list -v` SLA path is conditional on ambient checkout state

The only asserted under-1-second check for the image-terminal verbose data-gather path runs inside `perf_subprocess_counts_meet_sla`, but it uses the current monorepo checkout and skips the assertion entirely when the current worktree is main ([`worktree/cli/src/commands/list.rs:470`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:470), [`worktree/cli/src/commands/list.rs:479`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:479), [`worktree/cli/src/commands/list.rs:509`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:509)). It also measures `gather_branch(...)` directly rather than the `wt list -v` orchestration case that decides graph/verbose sharing.

Acceptance criterion 7 specifically calls out `wt list -v` on an image terminal, with rasterization excluded. The strongest current coverage is Level 1, which is appropriate for subprocess orchestration and timing with rasterization omitted, but the fixture needs to be controlled: create a temporary non-main worktree fixture or a fake-git fixture so the graph+verbose gather path always runs and the test always asserts the SLA.

### Low: the new testing-strategy link points at the wrong relative path

`docs/topics/testing-strategy.md` links to `../worktree/docs/performance-testing.md` ([`docs/topics/testing-strategy.md:108`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/docs/topics/testing-strategy.md:108)). From `docs/topics/`, that resolves to `docs/worktree/docs/performance-testing.md`, not `worktree/docs/performance-testing.md`. Use `../../worktree/docs/performance-testing.md` so the documentation link is valid.

## Requirement Verification

| Requirement | Strongest verification found | Review result |
| --- | --- | --- |
| R1 skip graph path on non-image terminals | Level 1 recorder test | Present. |
| R2 compute default branch once | Level 1 recorder test with temp repo | Present. |
| R3 one merge-base per branch pair | Level 1 recorder test with temp repo | Present for current branch. |
| R4 eliminate `short_sha()` subprocess | Code removes helper; Level 1 recorder tests | Present for current and base graph paths. |
| R5 parallelize base graph per-branch queries | Code uses `thread::scope`; Level 1 determinism/count tests | Present. |
| R6 share current-branch graph/verbose gather | Level 1 recorder test | Present for subprocess count, but SLA timing uses ambient checkout state. |
| R7 use `git log --reverse` | Level 1 direct git-output comparison | Present. |
| R8 performance testing contract | Documentation | Present. |
| R9 subprocess-count regression coverage | Level 1 recorder tests | Present. |
| AC5 image-capable user-visible graph unchanged | Level 2 Kitty capture | Present: emits Kitty graphics bytes and keeps table text visible. |
| AC7 1-second SLA | Level 1/integration timing tests | Gap: non-image command allows 1.5s; image verbose path can skip on main checkout. |

## Tested

- `sniff repo`
- `cargo nextest run -p worktree -p worktree-cli -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/))' --no-tests=pass` (passed)
- `just test-l2` from `worktree/` (passed; 5 Level 2 tests, including Kitty graph capture)

## Production Readiness

Not ready for production. The functionality and Level 2 rendering coverage are now in good shape, but the stated 1-second performance acceptance criterion is still not enforced by the tests.
