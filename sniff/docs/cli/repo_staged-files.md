---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo staged-files` Subcommand

Lists all files that are staged in the git index (ready to commit). Exits with code 1 if there are no staged files.

## Default Behavior

Outputs one file path per line, relative to the repository root. Paths are rendered with OSC8 hyperlinks and styled with dim directory segments and bold filenames. Use `--plain` to strip escape codes.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[FILTER...]` | Substring filters (OR logic, case-insensitive) |
| `--package <PKG>` | Scope output to files within a specific package |
| `--package-area <AREA>` | Scope output to files within a package area (prefix match) |
| `--list` | Output as bullet list (`- ` prefix) |
| `--csv` | Output as comma-separated values on a single line |
| `--no-path` | Show only the filename (hide directory path) |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more staged files found |
| `1` | No staged files in scope (default behavior) |
| `0` | No staged files with `--no-error` |

## Package Scoping

In monorepos, `--package` and `--package-area` limit which staged files are shown:

```bash
sniff repo staged-files --package sniff-cli       # Files staged in sniff/cli/
sniff repo staged-files --package-area homelab    # Files staged in homelab/
```

## Output Formats

```bash
sniff repo staged-files                  # One path per line (default)
sniff repo staged-files --list           # Bullet list
sniff repo staged-files --csv            # Comma-separated
sniff repo staged-files --no-path        # Basenames only
sniff repo staged-files --plain          # Strip ANSI/OSC8 codes
```

## JSON Output (`--json`)

```bash
sniff --json repo staged-files
```

Returns a JSON object with scope, kind, and paths:

```json
{
  "scope": "Staged",
  "kind": "AllFiles",
  "paths": [
    "sniff/cli/src/args.rs",
    "sniff/lib/src/filesystem/git.rs"
  ]
}
```

## No-Result Behavior

When no staged files are found, the default is to exit with code 1 and produce no output. This can be customized:

```bash
sniff repo staged-files --no-error
sniff repo staged-files --on-error "<yellow>Nothing staged</yellow>"
sniff repo staged-files --no-error --on-error "All clear"
```

## Usage in Scripts

```bash
# Check if anything is staged before committing
if sniff repo staged-files --plain > /dev/null 2>&1; then
    git commit -m "..."
fi

# Get staged files as an array (bash)
mapfile -t FILES < <(sniff repo staged-files --plain)
```
