# CLI Contracts: Shell Wrapper, Exit Codes, Name Resolution

Load before touching `cli/src/shell_integration.rs`, `cli/src/env.rs`,
`cli/src/exit.rs`, or how `wt` resolves a worktree name.

## Shell wrapper protocol

- `wt --completions <bash|zsh|fish|powershell>` (`cli/src/shell_integration.rs`)
  is the **only** source of the `wt` shell function and the completion
  registration. The registration is generated in-process with clap's
  `EnvCompleter`, so nothing sources `COMPLETE=… wt` at startup.
- The wrapper sets `WT_SHELL_WRAPPER=1` on exactly the `wt` calls it makes.
  `cli/src/env.rs::shell_wrapper_active()` treats only that value as proof.
  **Never infer the wrapper from a captured stdout** — scripts, `just`, and
  agents capture it too.
- Protocol lines on stdout: `cd:<path>` and `remove-handoff:<token>`.
  - The wrapper acts only after `wt` exits 0, stops if `cd` fails, and never
    evaluates output.
  - The POSIX wrapper collects the values in its read loop and acts after it, so
    the handoff call keeps the terminal's stdin.
- PowerShell:
  - calls the absolute executable path captured at generation, because Windows
    Terminal ships its own `wt.exe` alias;
  - switches `[Console]::OutputEncoding` to UTF-8 for the call;
  - sets `[Environment]::CurrentDirectory` along with `Set-Location`.

### Tests

- `cli/tests/shell_wrapper_exec.rs` runs the generated bash wrapper (and zsh on
  macOS) against a stub `wt` on `PATH`.
- fish has no L1 execution test.
- PowerShell's is `cli/tests/powershell_wrapper_exec.rs` (Windows only).
- Wrapper snapshots live in `cli/tests/wrapper_protocol.rs` (see the
  dual-target snapshot trap in [SKILL.md](SKILL.md)).

## Exit codes and interactivity

| Code | Meaning |
| ---- | ------- |
| 0 | done, or `Cancelled` |
| 1 | failure |
| 2 | clap usage error (clap owns it) |
| 3 | `WorktreeError::RefusedToLoseWork`, `WorktreeError::NotARealDirectory` |
| 4 | `WorktreeError::BlockedByEnvironment`, `WorktreeError::DirectoryInUse` |

- Exit 3 and 4 promise **nothing removed**, not "nothing changed". `wt remove`
  repairs an unlinked worktree's `.git` link before any consent, and that
  repair (which can also touch other worktrees' links) is never rolled back.
  A refusal after a repair attempt says Git metadata may have changed, and a
  refusal or cancellation after a successful repair says the restored link was
  left in place. Never print "nothing was changed" on those paths.
- Mapping lives in `cli/src/exit.rs`.
- `RefusedToLoseWork` and `BlockedByEnvironment` carry Prose markup that
  `main.rs` prints as-is. Other errors (including `DirectoryInUse`, whose
  message names the processes) are escaped with `Prose::escape_text`.
- `env::is_interactive()` means stdin **and** stderr are terminals and `CI` is
  unset or empty. Stdout is never consulted.

## Name resolution

`worktree::worktree::resolve_worktree(entries, name)` and
`completion_names(entries)` are pure functions over parsed porcelain;
`find_worktree` and `worktree_names` only add the git call.

- `base` is always the main checkout.
- Any other name matches branch names on every checkout, and directory
  basenames (raw or dasherized input) on linked worktrees only. The main
  checkout's basename is the repository name.
- More than one distinct worktree returns `WorktreeError::AmbiguousWorktree`.
  **Never pick the first.**
