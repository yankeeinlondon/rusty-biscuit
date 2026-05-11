---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo packages` Subcommand

Outputs all package names in the monorepo in a configurable format. Designed for script consumption and shell automation. Runs a structure-only detection (no git scanning, no file inventory, no language detection), so it returns well under 100 ms even on large monorepos.

## Default Behavior

Outputs a single line of comma-separated package names:

```
sniff-cli, sniff, research-cli, research, homelab-cli, homelab, homelab-server
```

## Arguments and Flags

| Argument/Flag | Description |
|---------------|-------------|
| `[filter...]` | Optional substring filters to narrow the package list |
| `-p`, `--package <PKG>` | Restrict output to a single package (exact match on `Package.name`) |
| `--package-area <AREA>` | Restrict output to packages in the given area (prefix match on `Package.package_area`) |
| `--md` | Render as a Markdown unordered list (one `- name` per line) |
| `--list` | Render as a raw list (one name per line, no bullet) |
| `-v`, `--verbose` | Append each package's repo-relative root directory in dim italic |
| `--json` | Emit a JSON array of package names |
| `--plain` | Strip terminal escape codes |

`--md` and `--list` are mutually exclusive. `--verbose` is layered on top of any output format and shows the root dir as styled metadata, never as raw tracing.

## Package Filtering

Zero or more positional arguments act as substring filters on package names and areas:

```bash
sniff repo packages                  # All packages
sniff repo packages @homelab         # Packages in the homelab area (filter syntax)
sniff repo packages biscuit          # Packages whose name contains "biscuit"
sniff repo packages !test            # All packages except those matching "test"
```

| Pattern | Behavior |
|---------|----------|
| `name` | Includes packages whose name contains the substring |
| `@area` | Includes all packages in the named area |
| `!name` | Excludes packages matching the substring |

Multiple filters apply OR logic (a package is included if any filter matches).

### Scoping to a Package or Package Area

`-p/--package <PKG>` restricts output to a single package by exact (case-insensitive) name match:

```bash
sniff repo packages -p sniff-cli
# sniff-cli
```

`--package-area <AREA>` restricts output to packages whose area starts with the supplied prefix (case-insensitive). It composes with positional filters:

```bash
sniff repo packages --package-area homelab
# arcam-amp-integration, homelab-cli, eversolo-integration, homelab, homelab-server,
# sony-receiver-integration, unfolded-integration-helper
```

Passing both `-p` and `--package-area` produces the AND intersection. If the resolved package does not live within the resolved area, the command fails with an explicit error. Unknown values for either flag fail with an error listing the valid names.

Shell completion suggests known package and area names detected in the current repo.

## Output Formats

### Default (CSV)

```
agent-sandbox-cli, biscuit-file-cli, biscuit-file, ...
```

### Markdown (`--md`)

```
- agent-sandbox-cli
- biscuit-file-cli
- biscuit-file
```

### Raw list (`--list`)

```
agent-sandbox-cli
biscuit-file-cli
biscuit-file
```

### Verbose (`-v` / `--verbose`)

With `--verbose`, each entry is annotated with its repo-relative root directory, rendered in dim italic (stripped by `--plain`):

```
agent-sandbox-cli(./agent-sandbox/cli), biscuit-file-cli(./biscuit-file/cli), ...
```

Combines with every format:

```bash
sniff repo packages --md --verbose
# - agent-sandbox-cli(./agent-sandbox/cli)
# - biscuit-file-cli(./biscuit-file/cli)

sniff repo packages --list --verbose
# agent-sandbox-cli(./agent-sandbox/cli)
# biscuit-file-cli(./biscuit-file/cli)
```

`--verbose` is reserved for styled user-facing output and never drives tracing. Raw tracing is opt-in via `--debug` or `RUST_LOG`.

## JSON Output (`--json`)

```
sniff --json repo packages
```

Returns a JSON array of package name strings:

```json
["sniff-cli", "sniff", "research-cli", "research"]
```

`--json` is authoritative over `--md`/`--list`/`--verbose`: output is always a string array of names.

## Usage in Scripts

```bash
# Iterate with --list (one name per line — no splitting needed)
while read -r pkg; do
    echo "Processing $pkg"
done < <(sniff repo packages --list)

# Every package in an area, CSV
sniff repo packages --package-area homelab

# Pass to cargo
cargo build $(sniff repo packages --package-area sniff --list | sed 's/^/-p /')
```
