---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo untracked-files` Subcommand

Lists new files in the working tree that are not yet under version control. Exits with code 1 if there are no untracked files.

## Default Behavior

Outputs a comma-separated list of file paths relative to the repository root:

```
sniff/docs/cli/repo.md, sniff/docs/cli/repo_structure.md
```

Only includes files that git considers untracked (not ignored). Ignored files (matching `.gitignore` patterns) are excluded.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-p/--package <PKG>` | Scope output to files within a specific package or package area |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more untracked files found |
| `1` | No untracked files in scope |

## Package Scoping (`-p`)

When `-p/--package` is provided, only untracked files within that package's directory are returned:

```bash
sniff repo untracked-files -p sniff       # New files in sniff/ area
sniff repo untracked-files -p research    # New files in research/ area
```

## JSON Output (`--json`)

```
sniff --json repo untracked-files
```

Returns a JSON array of file path strings:

```json
[
  "sniff/docs/cli/repo.md",
  "sniff/docs/cli/repo_structure.md"
]
```

Returns an empty array `[]` when no untracked files are found (exit code 1 still applies).

## Usage in Scripts

```bash
# List newly created files before committing
sniff repo untracked-files | tr ',' '\n'

# Check if new files exist in a package area
if sniff repo untracked-files -p sniff > /dev/null; then
    echo "New files detected in sniff/"
fi
```
