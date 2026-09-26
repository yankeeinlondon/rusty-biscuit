---
ready: false
agent: codex
model: ""
---

# Review 5

## Findings

### High: narrow image terminals still gather graph data that cannot render

R1 requires width fitting to be checked before building Mermaid instructions, and specifically says that for character widths graph data should be skipped when `terminal.width() < MIN_GRAPH_TERMINAL_WIDTH`. The current orchestration only applies that early skip when the user supplied an explicit character width (`Some(ImageWidth::Characters(_))`) ([`worktree/cli/src/commands/list.rs:35`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:35), [`worktree/cli/src/commands/list.rs:36`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:36)). With no `--width`, `parsed_width` is `None`, so `needs_graph` remains true on an image-capable terminal even if the terminal is narrower than 80 columns.

That path gathers graph data through `gather_data(...)` before deriving the default graph width ([`worktree/cli/src/commands/list.rs:46`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:46), [`worktree/cli/src/commands/list.rs:63`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:63)). The later `fits` check then discards rendering for character widths on narrow terminals ([`worktree/cli/src/commands/list.rs:66`](/Users/ken/.claudine/worktrees/rusty-biscuit/worktree/worktree/cli/src/commands/list.rs:66)). In other words, an image-capable 79-column terminal with the default width still pays the graph-only `git_graph.rs` subprocess cost and throws the result away.

This is both an implementation gap and a coverage gap. The existing Level 1 recorder test covers non-image terminals, but there is no recorder-backed test for the narrow image-terminal case. Add a Level 1 test that forces image support and a terminal width below `MIN_GRAPH_TERMINAL_WIDTH`, then asserts zero graph-only `merge-base`/`log` calls unless the width spec is `Percent` or `Fill`. The simplest implementation fix is to include the default/no-width case in the early character-width skip, or otherwise decide default graph sizing before graph-data collection without needing the graph instructions.

## Verification Matrix

| Requirement | Strongest verification observed | Review assessment |
| --- | --- | --- |
| R1 skip graph path when graph cannot render | Level 1 recorder test for non-image; Level 2 image rendering test | Partial: non-image is covered, but narrow image terminals still gather graph data before the late `fits` discard. |
| R2 resolve default branch once per list snapshot | Level 1 recorder tests | Present. |
| R3 one merge-base per branch pair | Level 1 recorder tests for `gather_branch`, `gather_data`, and base graph | Present. |
| R4 eliminate `short_sha()` subprocess and derive display IDs in-process | Level 1 recorder and graph-output tests | Present. `query_commits` now uses `%H` and derives display IDs with `display_sha`. |
| R5 parallelize base graph per-branch queries | Code uses `thread::scope`; Level 1 determinism/count tests | Present. |
| R6 share graph and verbose current-branch gather | Level 1 `gather_data` orchestration count test | Present. |
| R7 use `git log --reverse` | Level 1 oldest-first tests | Present. |
| R8 performance testing contract | `worktree/docs/performance-testing.md` | Present. |
| R9 subprocess-count regression coverage | Level 1 recorder tests | Partial: missing the narrow image-terminal skip case from R1. |
| Image-capable graph/table rendering | Level 2 Kitty/tmux tests | Present. |
| Non-image verbose rendering | Level 2 tmux test | Present. |

No Level 3 coverage is required for this spec because it does not define OS keyboard-input behavior.

## Tests Run

- `cargo nextest run -p worktree -p worktree-cli -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/))' --no-tests=pass --color=never` - passed, 89 tests.
- `just --justfile worktree/justfile --working-directory worktree test-l2` - passed, 5 Level 2 tests.

## Summary

The Review 4 issues are addressed: graph display IDs are now derived in process, and graph+verbose sharing has a dedicated orchestration-count guard. I would not mark this production-ready yet because R1 still has a discarded graph-data path for narrow image-capable terminals, and that path is not covered by subprocess-count regression tests.
