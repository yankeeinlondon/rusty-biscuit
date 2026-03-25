---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo package-area-root` Subcommand

Outputs the absolute path to the root directory of the current package area. Exits with code 1 if the current directory is not inside a recognized package area.

## Default Behavior

Outputs a single absolute path:

```
/Users/ken/projects/rusty-biscuit/sniff
```

For packages at the repository root level, returns the repository root.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Current directory is inside a package area |
| `1` | Not inside a recognized package area |

## Examples

```bash
# From inside sniff/cli/src/
sniff repo package-area-root
# → /Users/ken/projects/rusty-biscuit/sniff

# From inside sniff/lib/
sniff repo package-area-root
# → /Users/ken/projects/rusty-biscuit/sniff

# From inside homelab/server/src/
sniff repo package-area-root
# → /Users/ken/projects/rusty-biscuit/homelab

# Analyze a specific directory
sniff repo package-area-root -b /path/to/biscuit-hash/lib
# → /path/to/biscuit-hash
```

## Usage in Scripts

```bash
# Navigate to the area root from any nested location
cd "$(sniff repo package-area-root)"

# Run just commands scoped to the current area
cd "$(sniff repo package-area-root)" && just test
```

## Related Subcommands

| Subcommand | Returns |
|------------|---------|
| [`package-area`](./repo_package-area.md) | Area name |
| [`package-root`](./repo_package-root.md) | Absolute path to package root |
| `package-area-root` | Absolute path to area root |
| [`repo-root`](./repo_repo-root.md) | Absolute path to repository root |
