---
name: worktree
description: Details on the `worktree` package area of the **rusty-biscuit** repo
---
# Worktree Package Area

Like many package areas in Rusty Biscuit, the `worktree` package area has two distinct packages:

1. Library: `worktree` is the library where the business logic for how we handle git worktrees is regulated
2. CLI: `worktree-cli` (binary name of `wt`) is the CLI which leverages the worktree library for most of it's functionality

The `worktree-cli` crate contains both a binary target (`src/main.rs`, binary `wt`) and a library target (`src/lib.rs`). Shared modules such as `commands` and `perf` are declared in both `lib.rs` and `main.rs` so they are available to integration tests (which use the library target) and to the binary.

Because every shared module compiles twice, its `#[cfg(test)]` unit tests also run twice, once per target, under different module paths (`worktree_cli::…` and `wt::…`). An `insta` snapshot in such a unit test therefore gets two different snapshot names, and one of them always fails. Keep snapshot tests in an integration test (`cli/tests/wrapper_protocol.rs` holds the shell-wrapper snapshots).

## Shell wrapper protocol

- `wt --completions <bash|zsh|fish|powershell>` (`cli/src/shell_integration.rs`) is the only source of the `wt` shell function and the completion registration. The registration is generated in-process with clap's `EnvCompleter`, so nothing sources `COMPLETE=… wt` at startup.
- The wrapper sets `WT_SHELL_WRAPPER=1` on exactly the `wt` calls it makes. `cli/src/env.rs::shell_wrapper_active()` treats only that value as proof. Never infer the wrapper from a captured stdout, because scripts, `just`, and agents capture it too.
- Protocol lines on stdout: `cd:<path>` and `remove-handoff:<token>`. The wrapper acts only after `wt` exits 0, stops if `cd` fails, and never evaluates output. The POSIX wrapper collects the values in its read loop and acts after it, so the handoff call keeps the terminal's stdin.
- The PowerShell wrapper calls the absolute executable path captured at generation, because Windows Terminal ships its own `wt.exe` alias. It switches `[Console]::OutputEncoding` to UTF-8 for the call and sets `[Environment]::CurrentDirectory` along with `Set-Location`.
- `cli/tests/shell_wrapper_exec.rs` runs the generated bash wrapper (and zsh on macOS) against a stub `wt` on `PATH`. fish and PowerShell have no L1 execution test.

## Exit codes and interactivity

- `cli/src/exit.rs` maps errors to 0 (done or `Cancelled`), 1 (failure), 3 (`WorktreeError::RefusedToLoseWork`), and 4 (`WorktreeError::BlockedByEnvironment`); clap owns 2. Those two variants carry Prose markup that `main.rs` prints as-is. Other errors are escaped with `Prose::escape_text`.
- `env::is_interactive()` means stdin and stderr are terminals and `CI` is unset or empty. Stdout is never consulted.

## Name resolution

`worktree::worktree::resolve_worktree(entries, name)` and `completion_names(entries)` are pure functions over parsed porcelain; `find_worktree` and `worktree_names` only add the git call.

- `base` is always the main checkout.
- Any other name matches branch names on every checkout, and directory basenames (raw or dasherized input) on linked worktrees only. The main checkout's basename is the repository name.
- More than one distinct worktree returns `WorktreeError::AmbiguousWorktree`. Never pick the first.

## Fork-origin records

`worktree::fork_origin` stores `{ base_branch, base_sha, created_at }` per branch in `<repo hash>.fork-origins.json`. The file sits beside the comparison cache (`cache::repo_cache_file`) and is keyed by the main worktree's path. `create_worktree(branch, base, from)` records only newly created branches, as a best-effort write that never fails a create. `--from` must name a local branch (`refs/heads/…`), and a detached HEAD requires `--from`. `ForkOriginStore::prune` drops records of deleted branches and keeps records whose parent was deleted.

`worktree::worktree::list_worktrees` uses `worktree::cache` to persist SHA-pair branch comparison results under the user cache directory. The cache key is `(default_tip_sha, branch_tip_sha, CACHE_FORMAT_VERSION)`, so branch/default tip movement self-invalidates ahead/behind and clean-merge results. Dirty working-tree status is never cached and remains a live `git status` check.

`list_worktrees` is a thin wrapper over `parse_worktree_state` and `fill_worktree_statuses`. The CLI uses that seam to start graph/verbose data gathering from the parsed branch names while the expensive per-worktree status pass runs.
