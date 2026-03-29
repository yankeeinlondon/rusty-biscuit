# The `sniff repo` Subcommand

Provides monorepo and git repository analysis. The default subcommand (`sniff repo` with no arguments) is an alias for [`sniff repo structure`](./repo_structure.md).

## Global Flags

These flags apply to all `sniff repo` subcommands:

| Flag | Description |
|------|-------------|
| `--json` | Output JSON instead of styled text |
| `-v` / `--verbose` | Increase output verbosity (stackable: `-vv`) |
| `--plain` | Strip all ANSI escape codes from text output |
| `--latest-versions` | Query package registries for dependency update info |
| `--refresh-remotes` | Fetch remotes before reporting (for `git-status`) |
| `-b/--base <DIR>` | Analyze a specific directory instead of current |

## Subcommands

### Repository Structure

| Subcommand | Description |
|------------|-------------|
| [`structure`](./repo_structure.md) | Display hierarchical repo/monorepo package overview (default) |
| [`deps`](./repo_deps.md) | Show internal package dependency graph (text or `--ui` Mermaid diagram) |
| [`remote <REMOTE>`](./repo_remote.md) | Inspect a remote repository via URL, name, or `owner/repo` shorthand |

### Git Status

| Subcommand | Description |
|------------|-------------|
| [`git-status`](./repo_git-status.md) | Branch, commits, working tree status, and worktree info |
| [`hash <SHA>`](./repo_hash.md) | Inspect a specific commit by hash (full or short) |

### File Listing

| Subcommand | Description |
|------------|-------------|
| [`staged-files`](./repo_staged-files.md) | List files staged in the index (ready to commit) |
| [`unstaged-files`](./repo_unstaged-files.md) | List modified files in the working tree (not staged) |
| [`untracked-files`](./repo_untracked-files.md) | List new files not yet under version control |

### Package Names

| Subcommand | Description |
|------------|-------------|
| [`packages`](./repo_packages.md) | All package names as a comma-separated list |
| [`package`](./repo_package.md) | Package name of the current directory |
| [`package-area`](./repo_package-area.md) | Package area (top-level directory) of the current directory |
| [`dirty-packages`](./repo_dirty-packages.md) | Packages with uncommitted changes |
| [`dirty-package-areas`](./repo_dirty-package-areas.md) | Package areas with uncommitted changes |
| [`staged-packages`](./repo_staged-packages.md) | Packages with staged files |
| [`staged-package-areas`](./repo_staged-package-areas.md) | Package areas with staged files |
| [`unstaged-packages`](./repo_unstaged-packages.md) | Packages with unstaged (modified) files |
| [`unstaged-package-areas`](./repo_unstaged-package-areas.md) | Package areas with unstaged files |

### Directory Paths

| Subcommand | Description |
|------------|-------------|
| [`package-root`](./repo_package-root.md) | Absolute path to root of current package |
| [`package-area-root`](./repo_package-area-root.md) | Absolute path to root of current package area |
| [`root`](./repo_repo-root.md) | Absolute path to repository root |

### Exit Code Checks

| Subcommand | Description |
|------------|-------------|
| [`is-current-package-area-dirty`](./repo_is-current-package-area-dirty.md) | Exit 0 if package area has uncommitted changes |
| [`package-area-has-source-code-changes`](./repo_package-area-has-source-code-changes.md) | Exit 0 if package area has source code file changes |

## Package Filtering

Many subcommands accept an optional `filter` positional argument:

```bash
sniff repo dirty-packages @homelab   # Only packages in the homelab area
sniff repo packages biscuit          # Only packages whose name contains "biscuit"
sniff repo packages !vendor          # Exclude packages matching "vendor"
```

- Prefix `@` selects by area name
- Prefix `!` negates the filter
- Multiple filters are combined with OR logic
- Matching is case-insensitive

## JSON Output (`--json`)

All display subcommands support `--json`. Path and exit-code subcommands always produce plain text or no output.

```bash
sniff --json repo git-status
sniff --json repo structure
sniff --json repo deps
```
