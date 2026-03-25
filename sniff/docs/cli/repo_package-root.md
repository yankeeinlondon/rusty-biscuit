---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo package-root` Subcommand

Outputs the absolute path to the root directory of the current package. Exits with code 1 if the current directory is not inside a recognized package.

## Default Behavior

Outputs a single absolute path:

```
/Users/ken/projects/rusty-biscuit/sniff/cli
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Current directory is inside a package |
| `1` | Not inside a recognized package |

## Examples

```bash
# From inside sniff/cli/src/
sniff repo package-root
# → /Users/ken/projects/rusty-biscuit/sniff/cli

# From inside sniff/lib/
sniff repo package-root
# → /Users/ken/projects/rusty-biscuit/sniff/lib

# Analyze a specific directory
sniff repo package-root -b /path/to/homelab/server
# → /path/to/homelab/server
```

## Usage in Scripts

```bash
# Navigate to package root from any nested subdirectory
cd "$(sniff repo package-root)"

# Get package root for use in justfile
PKG_ROOT=$(sniff repo package-root)
```

## Related Subcommands

| Subcommand | Returns |
|------------|---------|
| [`package`](./repo_package.md) | Package name |
| `package-root` | Absolute path to package root |
| [`package-area-root`](./repo_package-area-root.md) | Absolute path to area root |
| [`repo-root`](./repo_repo-root.md) | Absolute path to repository root |
