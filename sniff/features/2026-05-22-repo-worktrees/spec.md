We already have a command `sniff repo worktree` which returns the current worktree the user is in. What we don't have currently is a way to list all of the worktrees in the current repo. This feature will add `sniff repo worktrees` which will list all of the worktrees using the same pretty file printing that you find in `sniff repo dirty-files`.

### Requirements

*   **Inclusion:** The list must include all worktrees, including the main worktree.
    *   The main worktree should be named after its directory.
    *   The output must visually highlight the worktree the user is currently "in" (e.g., via an asterisk or color).
*   **Sorting:** The default output should be sorted alphabetically by the worktree name.

### CLI Switches

The default output is a plain list but we should include the following CLI switches:

```sh
--json          -- Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
--list          -- Output as bullet list (one item per line with `- ` prefix)
--csv           -- Output as comma-separated values on a single line

--verbose       -- Show not only the worktree name but `<b>{worktree}</b> (<dim><i>on</i> {branch} <i>branch, located at </i><blue>{path}</blue></dim>)`
```

### Verbose Output Details

*   The "path" in the verbose output should be an OSC8 link.
*   The displayed path should use the `~` alias for the user's home directory if the worktree is located inside the user's home directory.
*   **Detached HEAD:** If a worktree is in a detached HEAD state (not on a specific branch), the verbose output should fall back to: `<b>{worktree}</b> (<dim><i>on</i> detached HEAD <i>located at </i><blue>{path}</blue></dim>)`.