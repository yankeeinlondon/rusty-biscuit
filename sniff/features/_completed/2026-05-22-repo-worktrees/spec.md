We already have a command `sniff repo worktree` which returns the current worktree the user is in. What we don't have currently is a way to list all of the worktrees in the current repo. This feature adds `sniff repo worktrees` which lists all of the worktrees using the same pretty file printing that you find in `sniff repo dirty-files`.

> **Status:** Implemented. This spec describes the shipped behavior.

### Requirements

*   **Inclusion:** The list must include all worktrees, including the main worktree.
    *   The main worktree is named after its directory basename.
    *   The output visually highlights the worktree the user is currently "in" with a leading `* ` marker (non-current entries are space-padded to align).
*   **Sorting:** The default output is sorted alphabetically by the worktree name.

### CLI Switches

The default output is the pretty, highlighted list. The following CLI switches select alternate output formats:

```sh
--json          -- Output as JSON instead of text
--md            -- Output as a Markdown unordered list (one `- name` per line)
--list          -- Output as a newline-delimited list (one name per line)
--csv           -- Output as comma-separated values on a single line

--verbose       -- Show the worktree name plus branch and path detail (see below)
```

`--md`, `--list`, and `--csv` are mutually exclusive. The current-worktree marker (`* `) is retained for `--md` and `--list`; `--csv` emits names only.

### Verbose Output Details

With `--verbose` (or `-v`), each entry renders as:

```
{marker}<b>{worktree}</b> (on <green>{branch}</green> branch, located at <a href="{absolute}">{path}</a>)
```

*   The "path" in the verbose output is an OSC8 link whose target is the worktree's absolute path.
*   The displayed path uses the `~` alias for the user's home directory when the worktree is located inside it.
*   **Detached HEAD:** If a worktree is in a detached HEAD state, the branch text falls back to `detached HEAD`:

    ```
    {marker}<b>{worktree}</b> (on <green>detached HEAD</green> branch, located at <a href="{absolute}">{path}</a>)
    ```
