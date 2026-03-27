---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo dirty-source-code` Subcommand

Lists source code files with changes (staged, unstaged, and untracked). Exits with code 1 if there are no dirty source files.

Source code is defined as files with `ProgrammingLanguage`, `FrameworkFile`, or `Styling` associations, plus HTML/HTM files.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[FILTER...]` | Substring filters (OR logic, case-insensitive) |
| `--package <PKG>` | Scope to a specific package in a monorepo |
| `--package-area <AREA>` | Scope to a package area (prefix match) |
| `--list` | Output as bullet list (`- ` prefix) |
| `--csv` | Output as comma-separated values |
| `--no-path` | Show only filenames (hide directory path) |
| `--no-error` | Exit 0 when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more dirty source files found |
| `1` | No dirty source files (default) |
| `0` | No dirty source files with `--no-error` |

## Examples

```bash
sniff repo dirty-source-code                          # All dirty source files
sniff repo dirty-source-code --package sniff-cli      # Only in sniff/cli/
sniff repo dirty-source-code --list --plain            # Plain bullet list
sniff repo dirty-source-code blast_radius              # Filter by substring
```

## JSON Output

```json
{
  "scope": "Dirty",
  "kind": "SourceCode",
  "paths": ["sniff/lib/src/filesystem/blast_radius.rs"]
}
```

## Related Commands

| Command | Scope | File types |
|---------|-------|------------|
| `sniff repo dirty-source-code` | Dirty | Source code only |
| `sniff repo staged-source-code` | Staged | Source code only |
| `sniff repo unstaged-source-code` | Unstaged | Source code only |
| `sniff repo dirty-files` | Dirty | All files |
| `sniff repo staged-files` | Staged | All files |
