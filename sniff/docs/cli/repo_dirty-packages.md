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
| `-p/--package <PKG>` | Restrict output to a single package (exact match on `Package.name`) |
| `--package-area <AREA>` | Restrict output to packages in the given area (prefix match on `Package.package_area`) |

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

## Scoping to a Package or Package Area

`-p/--package` matches `Package.name` exactly (case-insensitive); `--package-area` matches `Package.package_area` as a case-insensitive prefix. Either can be combined with the positional filter (AND).

```bash
sniff repo dirty-packages -p sniff-cli           # Just sniff-cli, when dirty
sniff repo dirty-packages --package-area homelab # All dirty homelab/* packages
```

Passing both flags requires the resolved package to live inside the resolved area; otherwise the command fails with an explicit error. Unknown values for either flag fail with an error listing the valid names.

## JSON Output (`--json`)

```
sniff --json repo dirty-packages
```

Returns a `{ scope, kind, names }` object so JSON consumers can branch on
lifecycle and granularity without hardcoding the subcommand name. `scope`
is `"dirty"`, `kind` is `"packages"`, and `names` is an array of package
name strings (empty when the repo is not a monorepo — JSON consumers
never see a prose error string):

```json
{
  "scope": "dirty",
  "kind": "packages",
  "names": ["sniff-cli", "sniff"]
}
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
