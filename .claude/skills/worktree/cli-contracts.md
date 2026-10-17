# CLI Contracts: Shell Wrapper, Exit Codes, Base Directory, Name Resolution

Load before touching `cli/src/shell_integration.rs`, `cli/src/env.rs`,
`cli/src/exit.rs`, `lib/src/config.rs`, base-directory error rendering, or how
`wt` resolves a worktree name.

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
| 1 | failure, including `WorktreeError::RemovalFailed` (a removal that stopped after the move, or failed before it) |
| 2 | clap usage error (clap owns it) |
| 3 | `WorktreeError::RefusedToLoseWork` |
| 4 | `WorktreeError::BlockedByEnvironment` |

- Exit 3 and 4 promise **nothing removed**, not "nothing changed". `wt remove`
  repairs an unlinked worktree's `.git` link before any consent, and that
  repair (which can also touch other worktrees' links) is never rolled back.
  A refusal after a repair attempt says Git metadata may have changed, and a
  refusal or cancellation after a successful repair says the restored link was
  left in place. Never print "nothing was changed" on those paths.
- Mapping lives in `cli/src/exit.rs`.
- `RefusedToLoseWork`, `BlockedByEnvironment`, and `RemovalFailed` carry
  Prose markup that `main.rs` prints as-is after one blank line. Other errors
  are escaped with `Prose::escape_text`.
- `wt remove` maps the lifecycle's `ExitClass`, never its outcome, to these
  variants (`commands/remove/recovery.rs::refusal_error`): `Refused` → 3,
  `Blocked` → 4, `Failure` → `RemovalFailed`.
- `env::is_interactive()` means stdin **and** stderr are terminals and `CI` is
  unset or empty. Stdout is never consulted.

## Base directory

User rules are in `worktree/docs/base-directory.md`; these are the code
contracts behind them.

- **Precedence:** `WT` (read with `var_os`), then `~/.worktree.json`. Unset and
  exactly empty `WT` both mean "no override"; any other `WT` (whitespace,
  relative, foreign, non-Unicode, missing) is a hard error and **never falls
  back** to a valid config.
- **Validation order:** `config::validate_base_dir(raw, origin)` (pure, no
  filesystem) → metadata probe (follows symlinks: missing → `NotFound`, file →
  `NotDirectory`, other I/O → `BaseDirectoryUnreadable` keeping the cause) →
  `.git` directly inside (file or folder; root-only, deeper paths allowed).
  Spelling is checked first so a foreign value can never touch the disk.
- **Acceptance is the host's `Path::is_absolute()`.** The foreign-syntax helper
  only labels a rejection. **WSL is POSIX:** `b:\wt` is a relative file name
  there, and `//server/share/wt` is absolute on POSIX. Never trim, unquote,
  expand `~`/`$VAR`, or join to the cwd; only typed prompt text is trimmed.
- **Origin:** every error carries `BaseDirSource` (`Env`, `ConfigFile(path)`,
  `Prompt`) under the field name `origin`, never `source` (thiserror reserves
  it). Remedy text is per origin and uses `config::example_base_dir()`.
- **Config parsing:** read bytes, then decode (invalid UTF-8 →
  `ConfigInvalidFormat`; real I/O keeps its cause). The derived deserializer
  rejects a **duplicate `base_dir`** (`ConfigInvalidFormat`, not last-wins).
- **Writer:** `save_config` checks `to_str` and syntax before opening the file
  and creates no directories; a bad value leaves the old config byte-for-byte.
- **Setup** runs only for `BaseDirectoryNotConfigured` **and**
  `env::is_interactive()`. `choose_base_dir` validates suggested and typed
  choices before any `create_dir_all`/`save_config`; value errors re-ask
  (`can_choose_again`), I/O errors stop (exit 1), cancel exits 0.
- **Rendering:** `create::run` returns every other resolution error; `main`
  renders it once with `commands::base_dir_error_markup` (exit 1 via
  `exit.rs`'s `_ => FAILURE`, stderr, no `cd:` line). Do not render in the
  command and also return the error.
- **WSL hint:** `config::wsl_drive_hint(value, runtime)` is pure;
  `detect_runtime_environment()` is called only in the `ForeignPath` arm, so a
  valid resolution pays no discovery cost.
- **Home isolation:** only `config_path()` and `considered_dirs()` use
  biscuit-file's `home_dir()` (`HOME`, `USERPROFILE` on Windows). The user
  cache, stores, and `~/.wt.json` (`api_preference`) still use `dirs`; see
  [testing.md](testing.md) for fixture consequences.

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
