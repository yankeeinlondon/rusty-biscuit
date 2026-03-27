---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo unstaged-source-code` Subcommand

Lists source code files that are modified in the working tree but not staged. Excludes untracked files.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[FILTER...]` | Substring filters (OR logic, case-insensitive) |
| `--package <PKG>` | Scope to a specific package |
| `--package-area <AREA>` | Scope to a package area (prefix match) |
| `--list` | Output as bullet list |
| `--csv` | Output as comma-separated values |
| `--no-path` | Show only filenames |
| `--no-error` | Exit 0 when no results |
| `--on-error <MESSAGE>` | Message when no results |

## Examples

```bash
sniff repo unstaged-source-code
sniff repo unstaged-source-code --package-area sniff --plain
```

## JSON Output

```json
{
  "scope": "Unstaged",
  "kind": "SourceCode",
  "paths": ["sniff/lib/src/filesystem/docs.rs"]
}
```
