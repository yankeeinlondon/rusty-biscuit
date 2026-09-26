---
ready: false
agent: codex
model: ""
---

# Review

## Findings

### High: image-capable graph output is still not verified at the required level

Acceptance criterion 5 requires no user-visible output change on image-capable terminals, including the graph. The new Level 2 test forces `TERM_PROGRAM=ghostty`, but it still runs inside `TmuxHarness` ([`worktree/cli/tests/level2_list_verbose.rs:139`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/tests/level2_list_verbose.rs:139), [`worktree/cli/tests/level2_list_verbose.rs:159`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/tests/level2_list_verbose.rs:159)). The test comment explicitly says tmux cannot display Kitty graphics and only checks that table/verbose text survives ([`worktree/cli/tests/level2_list_verbose.rs:153`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/tests/level2_list_verbose.rs:153), [`worktree/cli/tests/level2_list_verbose.rs:171`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/tests/level2_list_verbose.rs:171)).

Per the requested Level 1 / Level 2 / Level 3 rubric, this is still a verification mismatch: the strongest test for the image graph's user-visible rendering is not an actual image-capable terminal capture. It exercises the code path by environment spoofing, but it cannot prove that the graph renders, is framed correctly, or remains unchanged in WezTerm/Kitty/Ghostty.

Add a Level 2 test using an actual image-capable backend when available, or split the requirement explicitly: one test may spoof image detection to prove table/verbose survival, but a separate real image-terminal capture should verify the graph path itself. If exact raster comparison is too brittle, capture enough terminal output/state to prove an inline image was emitted and no surrounding text was suppressed.

### High: the performance SLA remains under-asserted

Acceptance criterion 7 requires `wt list` on a non-image terminal and `wt list -v` on an image terminal to meet the repo-wide 1-second SLA, excluding rasterization. The new `perf_subprocess_counts_meet_sla` test measures internal pieces, but it only asserts `list_worktrees()` is under 2 seconds ([`worktree/cli/src/commands/list.rs:563`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:563)). The docs repeat that generous 2-second bound and say the 1-second SLA applies elsewhere ([`worktree/docs/performance-testing.md:57`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/docs/performance-testing.md:57)).

That leaves the actual acceptance criterion unverified. On this machine, the internal measurement looked good (`list_worktrees: 488.20ms`, `gather_branch: 116.10ms`, `gather_base_graph: 174.74ms`), and a direct non-image `target/debug/wt list` run took `real 0.58`. But the test suite does not fail if the full command regresses past 1 second, and it does not measure the image-terminal verbose path with rasterization excluded.

Tighten the regression coverage so it asserts the stated contract: full non-image `wt list` under 1 second on warm cache, and a measured graph/verbose data-gather path for image-terminal `wt list -v` under 1 second with Mermaid rasterization excluded or mocked.

### Medium: the default test recipe now runs Level 2 tests, and the package still exposes no public `test-l2`

The worktree justfile exposes only `test`; it does not expose the canonical `test-l2` recipe even though the shared helper exists ([`worktree/justfile:35`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/justfile:35)). As a result, `just --justfile worktree/justfile --working-directory worktree test` ran the new `level2_*` tests as part of the default package test run.

That is not aligned with the repo testing contract from `rust-testing`: Level 2 tests should be run through `just test-l2`, which pre-spawns shared terminal panes and runs nextest with `-j 1`. The direct shared helper did pass when invoked manually, but the public package workflow should expose it and keep ordinary `test` as the L1 suite.

### Medium: one Level 1 graph test is flaky under nextest leak detection

During `just --justfile worktree/justfile --working-directory worktree test`, `commands::git_graph::tests::base_graph_is_deterministic_across_gather_runs` passed only after a retry. The first attempt exited successfully but nextest reported leaked handles. Since this test is part of the subprocess orchestration regression suite, it should be deterministic before the feature is considered production-ready.

Investigate the temporary-repo git setup around [`worktree/cli/src/commands/git_graph.rs:718`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:718). If git maintenance or hook/signing behavior is spawning detached work, disable it in the fixture config. If the leak is harness-related, isolate it from the pure graph tests.

### Low: new test code violates the repo comment/style rules

The new tests add section-marker comments and non-ASCII box-drawing separators, for example [`worktree/cli/src/commands/git_graph.rs:459`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:459) and [`worktree/cli/src/commands/list.rs:482`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:482). The repo instructions call section-marker comments an anti-pattern and ask edits to default to ASCII. This is not a functional blocker, but it should be cleaned up while touching these tests.

## Requirement Verification

| Requirement | Strongest verification found | Review result |
| --- | --- | --- |
| R1 skip graph path on non-image terminals | Level 1 recorder test | Present. |
| R2 compute default branch once | Level 1 recorder test with temp repo | Present. |
| R3 one merge-base per branch pair | Level 1 recorder test with temp repo | Present for current branch. |
| R4 eliminate `short_sha()` subprocess | Code removes helper; Level 1 recorder tests | Present for current and base graph paths. |
| R5 parallelize base graph per-branch queries | Code uses `thread::scope`; Level 1 determinism/count tests | Mostly present, but one related test was flaky under nextest leak detection. |
| R6 share current-branch graph/verbose gather | Code shares `gather_branch`; Level 1 count test | Present. |
| R7 use `git log --reverse` | Level 1 direct git-output comparison | Present. |
| R8 performance testing contract | Documentation | Present. |
| R9 subprocess-count regression coverage | Level 1 recorder tests | Present, but the suite needs the flake resolved. |
| AC5 image-capable user-visible graph unchanged | Level 2 tmux test with spoofed image env | Gap: not an actual image-capable rendering verification. |
| AC7 1-second SLA | Internal timing test with 2-second assertion | Gap: does not assert the specified SLA or full required paths. |

## Tested

- `sniff repo`
- `just --justfile worktree/justfile --working-directory worktree test` (passed, with one nextest retry/flaky leak report)
- `just --justfile worktree/justfile --working-directory worktree _test_l2 worktree-cli` (passed)
- `cargo test -p worktree-cli --lib -- commands::list::tests::perf_subprocess_counts_meet_sla --nocapture --test-threads=1 --color=never`
- `/usr/bin/time -p env -u TERM_PROGRAM -u KITTY_WINDOW_ID target/debug/wt list`
- `just --justfile worktree/justfile --working-directory worktree lint`

## Production Readiness

Not ready for production. The implementation has the right architecture and the subprocess-count coverage is much stronger than iteration 1, but the image-terminal graph requirement and the stated performance SLA are not yet verified at the required level, and the test suite showed one leak-detection flake.
