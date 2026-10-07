---
name: worktree
description: Expert knowledge for the `worktree` package area of rusty-biscuit — the `worktree` library and the `wt` CLI (`worktree-cli`) for managing git worktrees. Use when working in `worktree/`, changing `wt create`, `wt list`, `wt remove`, or `wt go`, the shell wrapper and completions, `.worktreeinclude` copying, fork-origin records, the PR and live-head refresh worker, the `wt list` caption, table, or git graph, or when writing or debugging worktree tests.
---

# Worktree

The `worktree` package area has two crates:

| Crate | Path | Role |
| ----- | ---- | ---- |
| `worktree` | `worktree/lib/` | Library holding the git-worktree logic (listing, include copying, removal safety, remote stores) |
| `worktree-cli` | `worktree/cli/` | The `wt` binary, plus rendering, the shell wrapper, and the refresh worker |

User-facing behavior is documented in `worktree/README.md`,
`worktree/docs/cli/list.md`, `worktree/docs/cli/remove.md`,
`worktree/docs/git-graph.md`, and
`worktree/docs/performance-testing.md`. The pages below hold implementation
facts and traps, not user docs.

## Pick a topic page

| You are touching | Load |
| ---------------- | ---- |
| Shell wrapper (`--completions`, `WT_SHELL_WRAPPER`, `cd:` protocol), exit codes, interactivity, worktree name resolution | [cli-contracts.md](cli-contracts.md) |
| `wt create`, fork-origin records, `.worktreeinclude`, include copying and copy records | [create-and-include.md](create-and-include.md) |
| `wt remove`: inventory fingerprint, safety tiers, handoff, `--force-remote` endpoint checks | [remove.md](remove.md) |
| `wt list` local pipeline, comparison cache, caption, table, columns, `--ff`, `--ignore-api` | [list.md](list.md) |
| `wt list` background worker, PR store, live-head store, receipts, the wait and its budgets | [list-remote.md](list-remote.md) |
| The `wt list` git graph: gathering, classification, layout, Kitty L2, graph perf | [git-graph.md](git-graph.md) |
| Any worktree test: stand-ins (`ProxyStub`, `FakeGitea`, `HoldingOrigin`), fixtures, L2 recipes, perf gates | [testing.md](testing.md) |
| Building a fixture repository: config writes, bulk commits, template copies, counting `git` spawns | [fixtures.md](fixtures.md) |

## Rules that bite everywhere

- **`worktree-cli` compiles shared modules twice.** It has a binary target
  (`src/main.rs`, binary `wt`) and a library target (`src/lib.rs`). Shared
  modules such as `commands` and `perf` are declared in both so integration
  tests (library target) and the binary can use them. Their `#[cfg(test)]`
  unit tests therefore run twice under different module paths
  (`worktree_cli::…` and `wt::…`), so an `insta` snapshot in a shared unit
  test gets two names and one always fails. **Put snapshot tests in an
  integration test**, as `cli/tests/wrapper_protocol.rs` does.
- **Never infer the shell wrapper from captured stdout.** Only
  `WT_SHELL_WRAPPER=1` proves it ([cli-contracts.md](cli-contracts.md)).
- **Never pick the first match** of an ambiguous worktree name; return
  `WorktreeError::AmbiguousWorktree`.
- **Git output is bytes.** Parse `-z` output with `git::git_from_bytes` and
  hash names as OS bytes. A lossy decoding once let a changed symlink pass the
  remove handoff ([remove.md](remove.md#inventory)).
- **Only the refresh worker writes the PR and live-head stores.** `wt list`
  never makes a foreground request ([list-remote.md](list-remote.md)).
- **Network code is bounded and noninteractive.** It goes through
  `live_remote`, which disables credential prompts and kills the whole process
  tree at its deadline.
- **A thread that runs Git must carry the call-count scope.** Take
  `git::calls::TaskHandle::current()` before spawning, run the task through
  `handle.run(..)`, and pass each joined result through `calls::joined`.
  Without it the thread's Git calls silently drop out of a `git_calls` count
  (scopes are per thread). Any new helper that starts Git calls
  `calls::started()` once, after a successful spawn, or uses `calls::output`.
- **Read repository metadata through `git_metadata`, not a git process.**
  Origin URL, default branch, branch-name validity, `core.sshCommand`, and ref
  targets are read in-process with `gix`, falling back to git only when `gix`
  cannot read the repository. A git process costs ~47 ms on Windows, and a
  dozen redundant ones once pushed the refresh worker past `wt list`'s 3 s
  wait there. A new read gets a parity test against the git command it
  replaces ([performance-testing](../../../worktree/docs/performance-testing.md#git-processes)).
- **Tests never reach the network.** Use the stand-ins in
  [testing.md](testing.md) and `example.invalid` origins.
- **The graph never substitutes what it cannot draw.** It marks the plan
  `incomplete` instead ([git-graph.md](git-graph.md)).

## Commands

Run these from `worktree/`:

```sh
just test         # L1
just test-l2      # L2 (name substring filter only; see testing.md)
just test-perf    # perf gates
just lint
just install      # installs wt
```

## Maintaining this skill

Add each fact to the topic page it belongs to, and keep this file a router.
When a new topic would push a page past roughly 300 lines, give it a page of
its own and add a row to the table above.
