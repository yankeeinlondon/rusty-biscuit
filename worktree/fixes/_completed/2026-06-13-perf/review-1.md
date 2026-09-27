---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: subprocess-count regression tests are racy and currently fail

The new recorder is a single process-global `Mutex<Option<GitCallLog>>`
([`worktree/lib/src/git.rs:133`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/lib/src/git.rs:133)), but the tests that start and finish recording are not serialized
([`worktree/lib/src/git.rs:182`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/lib/src/git.rs:182),
[`worktree/cli/src/commands/list.rs:344`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:344),
[`worktree/cli/src/commands/git_graph.rs:477`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:477),
[`worktree/cli/src/commands/git_graph.rs:640`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:640)).
Under the default Rust test runner, one test can replace or finish another test's recording window, so the subprocess-count assertions see unrelated calls or no calls at all.

This is not only a test hygiene problem: R9 is the required regression mechanism for the optimization, and its strongest intended verification level is Level 1. Right now that Level 1 coverage is unstable and failing, so the subprocess-count requirements are not verified.

Evidence:

- `cargo test -p worktree --color=never` failed in `git::tests::recorder_captures_default_branch_call`.
- `cargo test -p worktree-cli --color=never` failed in:
  - `commands::list::tests::list_worktrees_resolves_default_branch_once`
  - `commands::git_graph::tests::gather_branch_uses_one_merge_base_and_no_short_sha`
  - `commands::git_graph::tests::base_graph_collects_multiple_branches`

The fix should serialize every recorder-using test, or make the recorder scoped/thread-local enough that unrelated tests cannot interfere. The existing `run_skips_graph_git_calls_when_image_unavailable` test is serialized, but the other recorder tests need the same treatment or a stronger isolation design.

### High: image-capable user-visible output is only covered at Level 1

Acceptance criterion 5 requires unchanged user-visible output on image-capable terminals: table, graph, and verbose section. The current tests validate generated strings and in-process behavior, but there is no Level 2 test that runs `wt list` in a real terminal and captures rendered output through tmux/WezTerm/Kitty.

Per the requested test-rigor rules, graph rendering and terminal-visible table/verbose output are user-observable terminal behavior. The strongest coverage present here is Level 1, so this is a verification-level mismatch and the feature cannot be marked production-ready on that requirement.

At minimum, add a Level 2 capture for an image-capable or image-detected terminal path that verifies the status table and verbose text still render, and that the graph path is exercised without suppressing the table/verbose output. If exact raster/image bytes are impractical, assert the captured text around the graph and use the recorder to prove the graph gather path ran.

### High: base-branch graph gathering performs unnecessary per-branch git logs

`gather_base_graph` calls the full `gather_branch` for every branch
([`worktree/cli/src/commands/git_graph.rs:355`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:355)), and `gather_branch` always collects `default_context`, `default_after_base`, and `branch_after_base`
([`worktree/cli/src/commands/git_graph.rs:56`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:56)).
For the base graph renderer, only the merge-base identity/index and `branch_after_base` are consumed
([`worktree/cli/src/commands/git_graph.rs:407`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:407)).

That means the base-branch overview now pays two extra `git log` subprocesses per branch for data that is discarded. The work is parallel, but it still increases total process churn and CPU/I/O in the exact path R5 is trying to improve. This also weakens the subprocess-count regression suite because `base_graph_collects_multiple_branches` only asserts "at least one merge-base per branch" rather than bounding the expected graph-data calls.

Use a base-graph-specific gather path, or parameterize `gather_branch` so base graph collection requests only the fields it actually renders. Then add a subprocess-count assertion for the expected base graph command shape.

### High: subprocess-count tests depend on this checkout's ambient branch state

Several new tests call real `git` in the current monorepo and branch their assertions on whichever worktree happens to be current
([`worktree/cli/src/commands/git_graph.rs:485`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:485),
[`worktree/cli/src/commands/git_graph.rs:654`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:654)).
On the main worktree, the current-branch tests return early; in a checkout with no linked branches, base-graph tests can fail or stop exercising the intended path. This falls short of R9's requested "test-only instrumentation around the git boundary, or a narrow fake-git integration test" because the assertions are not backed by a controlled repository shape.

The strongest verification here is Level 1, which is appropriate for subprocess orchestration, but the fixture is too ambient to prove the requirements consistently. A narrow fake-git script or temporary repository fixture should cover:

- non-image `run()` emits zero graph-only commands;
- one default-branch resolution per list snapshot, including fallback behavior;
- one current-branch `merge-base` when graph and verbose data are both needed;
- base graph uses one merge-base per branch and no discarded per-branch log queries.

### Medium: the 1-second SLA is undocumented by measurements

Acceptance criterion 7 requires `wt list` non-image and `wt list -v` image-terminal performance to meet the repo-wide 1-second SLA, excluding rasterization. The implementation adds `worktree/docs/performance-testing.md`, and its frontmatter hash is valid, but there is no benchmark, trace, or recorded measurement attached to this change.

Because the spec is performance-driven and the implementation still adds extra base-graph logs, this should not be considered verified. A follow-up Criterion bench is not strictly required by R8, but the review needs at least a reproducible command/result or subprocess-count measurement showing the optimized paths meet the SLA.

## Requirement Verification

| Requirement | Strongest verification found | Review result |
| --- | --- | --- |
| R1 skip graph path on non-image terminals | Level 1 recorder test | Present but currently unstable because recorder tests race. |
| R2 compute default branch once | Level 1 recorder test | Present but currently failing/racy. |
| R3 one merge-base per branch pair | Level 1 recorder test | Present for current branch, but currently failing/racy. |
| R4 eliminate `short_sha()` subprocess | Code removes helper; Level 1 recorder test | Present for current branch, but currently failing/racy and not asserted for all graph paths. |
| R5 parallelize base graph per-branch queries | Code uses `thread::scope`; Level 1 determinism test | Partial: parallelized, but does unnecessary per-branch logs and lacks bounded subprocess-count coverage. |
| R6 share current-branch graph/verbose gather | Code shares `gather_branch`; Level 1 recorder intent | Partial: merge-base is shared, but test is failing/racy. |
| R7 use `git log --reverse` | Level 1 tests | Present, but tests mostly compare against the same command shape rather than independent expected fixtures. |
| R8 performance testing contract | Documentation | Present. |
| R9 subprocess-count regression coverage | Level 1 recorder tests | Not acceptable until the recorder tests are isolated and deterministic. |

## Tested

- `sniff repo`
- `cargo test -p worktree --color=never` (failed)
- `cargo test -p worktree-cli --color=never` (failed)
- `md hash worktree/docs/performance-testing.md`

## Production Readiness

Not ready for production. The implementation has the right broad shape, but required regression tests are failing, image-capable output lacks the required Level 2 verification, and the base graph path still performs avoidable git work.
