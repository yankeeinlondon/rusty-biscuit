---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/cli/src/output/mod.rs
---

# The `sniff repo package-areas` Subcommand

Outputs all unique package area names in the monorepo in a configurable format. Designed for script consumption and shell automation. Runs a structure-only detection (no git scanning, no file inventory, no language detection), so it returns well under 100 ms even on large monorepos.

## Default Behavior

Outputs a single line of comma-separated package area names:

```
sniff, homelab, biscuit-terminal, claudine
```

## Arguments and Flags

| Argument/Flag | Description |
|---------------|-------------|
| `[filter...]` | Optional substring filters to narrow the area list |
| `-p`, `--package <PKG>` | Restrict output to the area that contains the named package (exact match on `Package.name`) |
| `--package-area <AREA>` | Restrict output to a specific package area (prefix match) |
| `--md` | Render as a Markdown unordered list (one `- name` per line) |
| `--list` | Render as a raw list (one name per line, no bullet) |
| `-v`, `--verbose` | Append each area's repo-relative root directory in dim italic |
| `--json` | Emit a JSON array of area names |
| `--plain` | Strip terminal escape codes |

`--md` and `--list` are mutually exclusive. `--verbose` is layered on top of any output format and shows the root dir as styled metadata, never as raw tracing.

## Area Filtering

Zero or more positional arguments act as substring filters on area names:

```bash
sniff repo package-areas                  # All areas
sniff repo package-areas biscuit          # Areas whose name contains "biscuit"
sniff repo package-areas !test            # All areas except those matching "test"
```

| Pattern | Behavior |
|---------|----------|
| `name` | Includes areas whose name contains the substring |
| `!name` | Excludes areas matching the substring |

Multiple filters apply OR logic (an area is included if any filter matches).

### Scoping to a Package or Package Area

`-p/--package <PKG>` restricts output to the area that contains the named package:

```bash
sniff repo package-areas -p sniff-cli
# sniff
```

`--package-area <AREA>` applies a case-insensitive prefix filter on the area name:

```bash
sniff repo package-areas --package-area homelab
# homelab
```

Passing both `-p` and `--package-area` produces the AND intersection. If the resolved package does not live within the resolved area, the command fails with an explicit error. Unknown values for either flag fail with an error listing the valid names.

## Output Formats

### Default (CSV)

```
sniff, homelab, biscuit-terminal, claudine
```

### Markdown (`--md`)

```
- sniff
- homelab
- biscuit-terminal
```

### Raw list (`--list`)

```
sniff
homelab
biscuit-terminal
```

### Verbose (`-v` / `--verbose`)

With `--verbose`, each entry is annotated with its repo-relative root directory, rendered in dim italic (stripped by `--plain`):

```
sniff(./sniff), homelab(./homelab), biscuit-terminal(./biscuit-terminal)
```

Combines with every format:

```bash
sniff repo package-areas --md --verbose
# - sniff(./sniff)
# - homelab(./homelab)

sniff repo package-areas --list --verbose
# sniff(./sniff)
# homelab(./homelab)
```

`--verbose` is reserved for styled user-facing output and never drives tracing. Raw tracing is opt-in via `--debug` or `RUST_LOG`.

## JSON Output (`--json`)

```bash
sniff --json repo package-areas
```

Returns a JSON array of package area name strings:

```json
["sniff", "homelab", "biscuit-terminal", "claudine"]
```

`--json` is authoritative over `--md`/`--list`/`--verbose`: output is always a string array of area names.

## Usage in Scripts

```bash
# Iterate with --list (one name per line)
while read -r area; do
    echo "Processing area: $area"
done < <(sniff repo package-areas --list)

# Build all packages in each area
for area in $(sniff repo package-areas --list); do
    just "$area" build
done
```
