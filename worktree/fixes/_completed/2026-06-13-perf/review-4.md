---
ready: false
agent: codex
model: ""
---

# Review 4

## Findings

### Medium: graph display IDs are still delegated to git `%h`, not derived in-process

R4 says to fetch full SHAs for graph identity and derive display IDs in Rust with a shared helper, specifically to avoid carrying forward git's repository-dependent abbreviation behavior. The implementation does switch branch placement to full-SHA equality, but `query_commits` still asks git for both `%H` and `%h` and stores `%h` directly as `CommitId.display` ([`worktree/cli/src/commands/git_graph.rs:298`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:298), [`worktree/cli/src/commands/git_graph.rs:300`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:300), [`worktree/cli/src/commands/git_graph.rs:321`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:321)). The tests then lock in that old `%h` behavior by comparing graph output to direct `git log --format=%h` output ([`worktree/cli/src/commands/git_graph.rs:655`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:655), [`worktree/cli/src/commands/git_graph.rs:714`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:714)).

This does remove the extra `rev-parse --short` subprocess, so the main performance goal is met. It is still a designed behavior gap: the graph path should query `%H` only, build `CommitId { full, display: display_sha(&full) }` in Rust, and update the characterization tests to assert the in-process display contract rather than git `%h` parity.

### Medium: no full-`run()` subprocess-count guard for graph + verbose sharing

R9 asks for regression coverage proving that one `merge-base` is used for the current branch when graph and verbose data are both needed. The current binding count test calls `gather_branch(default, feature, verbose = true)` directly ([`worktree/cli/src/commands/git_graph.rs:601`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/git_graph.rs:601)), which proves the gather helper is efficient, but it does not prove `list::run()` only invokes that helper once when the image-capable `wt list -v` orchestration path is active. A future regression could accidentally call `gather_branch` twice from `run()` and this test would still pass.

The Level 2 tmux test exercises the image-detected verbose path and verifies table/verbose rendering survives ([`worktree/cli/tests/level2_list_verbose.rs:157`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/tests/level2_list_verbose.rs:157)), but it does not inspect git subprocess counts. Add a recorder-backed L1 integration/unit test around `run(..., verbose = true)` with image support forced and stderr treated as TTY, or factor the orchestration decision into a pure function that can be counted without terminal spoofing. The important assertion is one `merge-base` for the current branch across the graph and verbose surfaces.

## Verification Matrix

| Requirement | Strongest verification observed | Review assessment |
| --- | --- | --- |
| R1 skip graph path on non-image terminals | Level 1 recorder test around `run(None, false)` | Present. |
| R2 resolve default branch once per list snapshot | Level 1 recorder tests | Present for the list snapshot. |
| R3 one merge-base per branch pair | Level 1 recorder test for `gather_branch`; base graph count test | Present for helpers; full `run()` graph+verbose orchestration is not count-guarded. |
| R4 eliminate `short_sha()` subprocess and derive display IDs in-process | Level 1 recorder tests assert no `rev-parse --short` | Partial: subprocess removed, but display IDs still come from git `%h`. |
| R5 parallelize base graph per-branch queries | Code uses `thread::scope`; Level 1 determinism/count tests | Present. |
| R6 share graph and verbose current-branch gather | Code appears correct; helper count test | Partial test coverage: no full orchestration count guard. |
| R7 use `git log --reverse` | Unit test for oldest-first order | Present. |
| R8 performance testing contract | `worktree/docs/performance-testing.md` | Present. |
| R9 subprocess-count regression coverage | Level 1 recorder tests | Mostly present; missing full `run()` graph+verbose count guard. |
| Image-capable graph/table rendering | Level 2 Kitty/tmux tests | Present. |
| Non-image verbose rendering | Level 2 tmux test | Present. |

No Level 3 coverage is required here: the spec does not define OS keyboard-input behavior.

## Tests Run

- `cargo nextest run -p worktree -p worktree-cli -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/))' --no-tests=pass` — passed, 85 tests.
- `just --justfile worktree/justfile --working-directory worktree test-l2` — passed, 5 Level 2 tests.

## Summary

The implementation is close and the main performance regression gates now pass, including the 1-second non-image command SLA and Level 2 rendering checks. I would not mark it production-ready yet because R4's in-process display-SHA requirement is still not implemented, and the most important graph+verbose subprocess-sharing claim is not guarded at the `wt list -v` orchestration boundary.
