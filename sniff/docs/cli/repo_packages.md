---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo packages` Subcommand

Outputs all package names in the monorepo as a comma-separated list. Designed for script consumption and shell automation.

## Default Behavior

Outputs a single line of comma-separated package names:

```
sniff-cli, sniff, research-cli, research, homelab-cli, homelab, homelab-server
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[filter...]` | Optional substring filters to narrow the package list |

## Package Filtering

Zero or more positional arguments act as substring filters on package names and areas:

```bash
sniff repo packages                  # All packages
sniff repo packages @homelab         # Packages in the homelab area
sniff repo packages biscuit          # Packages whose name contains "biscuit"
sniff repo packages !test            # All packages except those matching "test"
```

| Pattern | Behavior |
|---------|----------|
| `name` | Includes packages whose name contains the substring |
| `@area` | Includes all packages in the named area |
| `!name` | Excludes packages matching the substring |

Multiple filters apply OR logic (a package is included if any filter matches).

## JSON Output (`--json`)

```
sniff --json repo packages
```

Returns a JSON array of package name strings:

```json
["sniff-cli", "sniff", "research-cli", "research"]
```

## Usage in Scripts

```bash
# Iterate over all packages
for pkg in $(sniff repo packages | tr ',' ' '); do
    echo "Processing $pkg"
done

# Get packages in the homelab area as a space-separated list
HOMELAB_PKGS=$(sniff repo packages @homelab | tr -d ' ' | tr ',' ' ')

# Pass to cargo
cargo build $(sniff repo packages @sniff | tr -d ' ' | sed 's/,/ -p /g' | sed 's/^/-p /')
```
