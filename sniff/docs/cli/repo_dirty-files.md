---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo dirty-files` Subcommand

Lists all files with changes (staged, unstaged, and untracked), regardless of file type. Unlike `dirty-source-code`, this includes non-source files such as documentation, configuration, and data files.

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
sniff repo dirty-files
sniff repo dirty-files --list --plain
sniff repo dirty-files .md                # Filter to markdown files
```

## JSON Output

```json
{
  "scope": "Dirty",
  "kind": "AllFiles",
  "paths": ["sniff/cli/src/commands.rs", "sniff/docs/cli/repo.md", "config.json"]
}
```
