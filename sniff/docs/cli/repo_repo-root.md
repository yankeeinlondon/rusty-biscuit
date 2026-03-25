---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo repo-root` Subcommand

Outputs the absolute path to the root of the git repository. Exits with code 1 if the current directory is not inside a git repository.

## Default Behavior

Outputs a single absolute path:

```
/Users/ken/projects/rusty-biscuit
```

This is the directory containing the `.git` folder (or the worktree's linked git directory).

## Arguments and Flags

This subcommand takes no positional arguments. The global `-b/--base <DIR>` flag may be used to analyze from a different starting directory.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Current directory is inside a git repository |
| `1` | Not inside a git repository |

## Examples

```bash
# From anywhere in the monorepo
sniff repo repo-root
# → /Users/ken/projects/rusty-biscuit

# From a deeply nested directory
cd sniff/cli/src/output/
sniff repo repo-root
# → /Users/ken/projects/rusty-biscuit

# From inside a worktree
cd /Users/ken/.claudine/worktrees/feat-sniff-tuning/
sniff repo repo-root
# → /Users/ken/projects/rusty-biscuit (the base repo root)
```

## Usage in Scripts

```bash
# Navigate to repo root from any location
cd "$(sniff repo repo-root)"

# Set REPO_ROOT for use in scripts
REPO_ROOT=$(sniff repo repo-root)
echo "Repo root: $REPO_ROOT"
```

## Related Subcommands

| Subcommand | Returns |
|------------|---------|
| [`package-root`](./repo_package-root.md) | Absolute path to current package root |
| [`package-area-root`](./repo_package-area-root.md) | Absolute path to current area root |
| `repo-root` | Absolute path to repository root |
