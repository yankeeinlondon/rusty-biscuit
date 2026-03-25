---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo dirty-packages` Subcommand

Lists package names that have uncommitted changes (staged, unstaged, or untracked files). Exits with code 1 if no dirty packages are found.

## Default Behavior

Outputs a comma-separated list of package names:

```
sniff-cli, sniff
```

A package is considered dirty if any file within its directory tree has:

- Staged changes (in the index)
- Unstaged changes (modified working tree)
- Untracked files (new files not yet versioned)

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[filter...]` | Optional substring filters to narrow the package scope |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more dirty packages found |
| `1` | No dirty packages, or not in a monorepo |

## Package Filtering

```bash
sniff repo dirty-packages              # All dirty packages
sniff repo dirty-packages @homelab     # Dirty packages in homelab area
sniff repo dirty-packages biscuit      # Dirty packages matching "biscuit"
sniff repo dirty-packages !test        # Dirty packages excluding "test"
```

Filters apply to the full package set before checking dirty status. Use [`sniff repo dirty-package-areas`](./repo_dirty-package-areas.md) to get area-level results instead.

## JSON Output (`--json`)

```
sniff --json repo dirty-packages
```

Returns a JSON array of package name strings:

```json
["sniff-cli", "sniff"]
```

## Usage in Scripts

```bash
# Build only dirty packages
for pkg in $(sniff repo dirty-packages | tr ',' ' '); do
    cargo build -p "$pkg"
done

# Check if a specific package area has changes
if sniff repo dirty-packages @sniff > /dev/null; then
    echo "Changes detected in sniff area"
fi
```
