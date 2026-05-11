---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo package-area` Subcommand

Outputs the package area (top-level directory group) of the current directory. Exits with code 1 if the current directory is not inside a recognized package area.

## Default Behavior

Outputs a single area name:

```
sniff
```

For packages at the repository root level (no parent area), outputs `root`.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Current directory belongs to a package area |
| `1` | Not inside a recognized package area (unless `--no-error`) |

## Examples

```bash
# From inside sniff/cli/
sniff repo package-area
# → sniff

# From inside sniff/lib/src/
sniff repo package-area
# → sniff

# From inside homelab/server/
sniff repo package-area
# → homelab

# From a root-level package
sniff repo package-area
# → root

# Analyze a specific directory
sniff repo package-area -b /path/to/biscuit-hash/lib
# → biscuit-hash
```

## Error Handling

By default, exits with code 1 when not inside a package area. Use `--no-error` and `--on-error` to customize:

```bash
# Silently succeed with no output
sniff repo package-area --no-error

# Show a custom message on stderr and exit 1
sniff repo package-area --on-error "Not in a package area"

# Show a custom message on stdout and exit 0
sniff repo package-area --no-error --on-error "Not in a package area"
```

## Usage in Scripts

```bash
# Get the current area to scope cargo commands
AREA=$(sniff repo package-area)

# Use with dirty-packages to check the current area
if sniff repo dirty-packages "@$AREA" > /dev/null; then
    echo "Area $AREA has uncommitted changes"
fi

# Safe fallback when not in a package area
AREA=$(sniff repo package-area --no-error)
if [ -n "$AREA" ]; then
    echo "Working in area: $AREA"
fi
```

## JSON Output (`--json`)

```bash
sniff --json repo package-area
```

Returns a `{ name: "<area>" }` object. Exit code semantics still
honour `--no-error` / `--on-error`.

```json
{ "name": "sniff" }
```
