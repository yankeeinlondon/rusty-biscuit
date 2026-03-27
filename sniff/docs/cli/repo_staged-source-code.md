---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo staged-source-code` Subcommand

Lists source code files that are staged in the git index. Same as `dirty-source-code` but scoped to staged changes only.

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
sniff repo staged-source-code
sniff repo staged-source-code --package sniff-cli --plain
```

## JSON Output

```json
{
  "scope": "Staged",
  "kind": "SourceCode",
  "paths": ["sniff/cli/src/commands.rs"]
}
```
