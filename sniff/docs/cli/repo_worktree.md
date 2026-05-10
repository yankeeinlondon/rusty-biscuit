---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/lib/src/filesystem/git.rs
  - sniff/cli/src/output/repo_json.rs
---

# The `sniff repo worktree` Subcommand

Outputs the name of the current Git linked worktree. Exits with code 1 when inside the main worktree, outside any repository, or when the worktree path has no valid basename.

## Default Behavior

Outputs a single worktree name:

```
sniff
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |
| `-v/--verbose` | Include the fully qualified path alongside the worktree name |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Verbose Mode (`-v`)

With `-v`, the output includes the absolute path to the worktree's root directory:

```
sniff [/Users/ken/.claudine/worktrees/rusty-biscuit/sniff]
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Current directory is inside a linked worktree |
| `1` | Not inside a linked worktree (unless `--no-error`) |

## Examples

```bash
# From inside a linked worktree
sniff repo worktree
# → sniff

# From inside a linked worktree with verbose output
sniff repo worktree -v
# → sniff [/Users/ken/.claudine/worktrees/rusty-biscuit/sniff]

# Analyze a specific directory
sniff repo worktree -b /path/to/my-worktree
# → my-worktree

# From the main worktree (exits 1 by default)
sniff repo worktree
# → (no output, exit code 1)
```

## Error Handling

By default, exits with code 1 when not inside a linked worktree. Use `--no-error` and `--on-error` to customize:

```bash
# Silently succeed with no output
sniff repo worktree --no-error

# Show a custom message on stderr and exit 1
sniff repo worktree --on-error "Not in a worktree"

# Show a custom message on stdout and exit 0
sniff repo worktree --no-error --on-error "Not in a worktree"
```

## Usage in Scripts

```bash
# Get the current worktree name
WT=$(sniff repo worktree)
echo "Working in worktree: $WT"

# Conditional logic based on worktree
if [ "$(sniff repo worktree)" = "main" ]; then
    echo "In the main worktree"
fi

# Safe fallback when not in a worktree
WT=$(sniff repo worktree --no-error)
if [ -n "$WT" ]; then
    echo "In worktree: $WT"
fi
```

## JSON Output (`--json`)

```bash
sniff --json repo worktree
```

Returns a `{ worktree: "<name>" }` object on success, or `{ worktree: null }` when not inside a linked worktree. Exit code semantics honour `--no-error` / `--on-error`.

```json
{ "worktree": "sniff" }
```

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| [`root`](./repo_repo-root.md) | Absolute path to the repository root |
| [`package-root`](./repo_package-root.md) | Absolute path to the current package root |
| [`git-status`](./repo_git-status.md) | Full git status with worktree summaries |
