---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo staged-files` Subcommand

Lists files that are staged in the git index (ready to commit). Exits with code 1 if there are no staged files.

## Default Behavior

Outputs a comma-separated list of file paths relative to the repository root:

```
sniff/cli/src/args.rs, sniff/lib/src/filesystem/git.rs, sniff/docs/cli/repo.md
```

Each path reflects the file's action: created, modified, or deleted files are all included.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-p/--package <PKG>` | Scope output to files within a specific package or package area |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more staged files found |
| `1` | No staged files in scope |

## Package Scoping (`-p`)

When `-p/--package` is provided, only staged files within that package's directory are returned:

```bash
sniff repo staged-files -p sniff        # Files staged in sniff/ area
sniff repo staged-files -p sniff-cli    # Files staged in sniff/cli/
```

## JSON Output (`--json`)

```
sniff --json repo staged-files
```

Returns a JSON array of file path strings:

```json
[
  "sniff/cli/src/args.rs",
  "sniff/lib/src/filesystem/git.rs"
]
```

Returns an empty array `[]` when no staged files are found (exit code 1 still applies).

## Plain Output (`--plain`)

Adding `--plain` has no effect on this subcommand since its output contains no ANSI codes.

## Usage in Scripts

```bash
# Check if anything is staged before committing
if sniff repo staged-files > /dev/null; then
    git commit -m "..."
fi

# Get staged files as an array (bash)
IFS=',' read -ra FILES <<< "$(sniff repo staged-files)"
```
