# The `sniff repo` Subcommand

Provides monorepo and git repository analysis. With no subcommand, `sniff repo` prints the repository name (matching [`sniff repo name`](./repo_name.md) in text mode); when invoked as `sniff repo --json`, it returns the consolidated repository aggregate described below.

## Global Flags

These flags apply to all `sniff repo` subcommands:

| Flag | Description |
|------|-------------|
| `--json` | Output JSON instead of styled text |
| `-v` / `--verbose` | Increase output verbosity (stackable: `-vv`) |
| `--plain` | Strip all ANSI escape codes from text output |
| `-b/--base <DIR>` | Analyze a specific directory instead of current |

`--latest-versions` (on `structure`) and `--refresh-remotes` (on `git-status`) are **command-local** flags, not global ones. They affect only the subcommand that declares them and are **not** inputs to the bare `sniff repo --json` aggregate, which never performs network requests.

## Subcommands

### Identity

| Subcommand | Description |
|------------|-------------|
| [`name`](./repo_name.md) | Repository name (default when no subcommand is given) |
| [`language`](./repo_language.md) | Primary programming language for the repository |
| [`is-monorepo`](./repo_is-monorepo.md) | Monorepo label (e.g. `cargo`; `false` if not). Exits non-zero when false unless `--no-error`. `--json` emits `{ "is_monorepo": true, "authority": "...", "orchestrators": [...] }` / `{ "is_monorepo": false }` |
| [`package-count`](./repo_package-count.md) | Number of discovered packages (`{ "package-count": N }` with `--json`) |
| [`version`](./repo_version.md) | Repository version from the root manifest (`{ "version": "..." \| null }` with `--json`) |
| [`worktree`](./repo_worktree.md) | Name of the current Git linked worktree |

### Repository Structure

| Subcommand | Description |
|------------|-------------|
| [`structure`](./repo_structure.md) | Display hierarchical repo/monorepo package overview |
| `branches` | List local branches from known refs; `--refresh-remotes` opts into a non-interactive remote refresh |
| [`package-dependencies`](./repo_deps.md) | Show internal package dependency graph (text or `--ui` Mermaid diagram) |
| `dependencies` | Show external package dependencies with dependency-family filters |
| [`remote <REMOTE>`](./repo_remote.md) | Inspect a remote repository via URL, name, or `owner/repo` shorthand |

### Git Status

| Subcommand | Description |
|------------|-------------|
| [`git-status`](./repo_git-status.md) | Branch, commits, working tree status, and worktree info |
| [`hash <SHA>`](./repo_hash.md) | Inspect a specific commit by hash (full or short) |

### Worktree

| Subcommand | Description |
|------------|-------------|
| [`worktree`](./repo_worktree.md) | Name of the current Git linked worktree |

### File Listing

| Subcommand | Description |
|------------|-------------|
| [`staged-files`](./repo_staged-files.md) | List files staged in the index (ready to commit) |
| [`unstaged-files`](./repo_unstaged-files.md) | List modified files in the working tree (not staged) |
| [`untracked-files`](./repo_untracked-files.md) | List new files not yet under version control |

### Package Names

| Subcommand | Description |
|------------|-------------|
| [`packages`](./repo_packages.md) | All package names (CSV, `--md`, or `--list`); `--package-area` scopes by area |
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
| [`has-merge-conflict`](./repo_has-merge-conflict.md) | Exit 0 if there are merge conflicts, otherwise 1, `-v` lists conflict files |

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

## Package and Package-Area Scoping (`-p` / `--package-area`)

Most package- and area-aware subcommands also accept two structured scoping flags that compose with the positional filter (AND):

| Flag | Match Semantics |
|------|-----------------|
| `-p/--package <PKG>` | Exact (case-insensitive) match on `Package.name` |
| `--package-area <AREA>` | Case-insensitive prefix match on `Package.package_area` (so `homelab` matches both `homelab` and `homelab/server`) |

Passing both flags produces the AND intersection: the resolved package must live within the resolved area. Mismatches and unknown values fail with explicit errors that list the valid names. See each subcommand page for the full coverage matrix.

## JSON Output (`--json`)

All display subcommands support `--json`. Path subcommands emit a `{ "root": "..." }` (or `{ "name": "..." }`) object under `--json` while still honoring exit codes. Most exit-code leaves such as `version`, `is-current-package-area-dirty`, `package-area-has-source-code-changes`, and `has-merge-conflict` emit a stable single-key object under `--json` (e.g. `{ "version": "0.1.0" }`) alongside their exit code, so scripts can branch on `$?` without parsing the body. The focused `is-monorepo` leaf emits the object documented in [`repo_is-monorepo.md`](./repo_is-monorepo.md). Without `--json` subcommands produce plain text or no output.

```bash
sniff --json repo git-status
sniff --json repo structure
sniff --json repo branches
sniff --json repo dependencies --dev-dependencies
sniff --json repo package-dependencies
```

### Aggregate output for bare `sniff repo --json`

When `sniff repo` is invoked without a subcommand and with `--json`, the output is a consolidated `SniffRepo` projection with snake_case keys:

- Identity fields stay top-level: `name`, `version`, `language`, `is_monorepo`, `package_count`, and `root`.
- Cwd-relative fields are grouped under `context`, including `package`, `package_area`, `area`, `package_root`, `package_area_root`, `worktree`, `is_current_package_area_dirty`, and `package_area_has_source_code_changes`.
- Worktrees and branches appear once as top-level `worktrees` and `branches` arrays.
- Change data is grouped into four `ScopeBucket` objects: `dirty`, `staged`, `unstaged`, and `untracked`. Each bucket contains `files`, `source_code`, `documentation`, `packages`, and `package_areas` arrays.
- `git_status` is a lean aggregate projection with current branch, config, compact file changes, and dirty/staged/unstaged/untracked counts. Use `sniff repo git-status --json` for the focused rich shape.
- **Excluded children:** `hash` (requires a parameter), `remote`, and `pr` (network-primary) are omitted from the aggregate. No network requests are made by the aggregate.

```bash
sniff repo --json
```

The returned object uses stable empty shapes (`null`, `""`, `[]`, or `{ ... }`) when a given value is absent. It intentionally does not embed full package catalogs under structure, dependency, or commit-family sections. Network-only fields such as remote branch lists or latest dependency versions are not included unless the relevant opt-in flag is used on the specific leaf subcommand.
