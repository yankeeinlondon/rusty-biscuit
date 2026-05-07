# `sniff repo worktree` Specification

The `sniff repo worktree` subcommand identifies if the current directory is part of a linked Git worktree and returns its name.

## Functional Requirements

- **Worktree Identification**: The command returns the **base directory name** of the worktree path.
- **Scope**: This applies **only to linked worktrees**. The main/original Git worktree is treated as "not in a worktree".
- **Non-Worktree/Non-Git Behavior**: If the directory is not in a linked worktree (this includes being in the main worktree or not being in a Git repository at all), the command:
    - Exits with code `1` by default.
    - Produces no text output by default.
    - Respects the `--no-error` and `--on-error` switches.

## CLI Switches

| Argument               | Description                                                                         |
| ---------------------- | ----------------------------------------------------------------------------------- |
| `--no-error`           | Exit `0` even when no linked worktree is found.                                     |
| `--on-error <MESSAGE>` | Display `<MESSAGE>` to stderr when no linked worktree is found (and exit with code 1). |

## Output Formats

### Plain Text (Default)
- **Success**: The name of the linked worktree (e.g., `my-feature-fix`).
- **Failure**: No output (unless `--on-error` is used).

### JSON (Global `--json` flag)
- **Success**: 
  ```json
  { "worktree": "worktree-name" }
  ```
- **Failure (with `--no-error`)**: 
  ```json
  { "worktree": null }
  ```
- **Failure (Default)**: If the command exits with code 1, it should still provide the null structure if JSON was requested.
