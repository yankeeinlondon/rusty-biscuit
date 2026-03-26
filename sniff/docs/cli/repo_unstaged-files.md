---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo unstaged-files` Subcommand

Lists files that are modified in the working tree but not yet staged. Exits with code 1 if there are no unstaged files.

## Default Behavior

Outputs a comma-separated list of file paths relative to the repository root:

```
sniff/cli/src/args.rs, sniff/lib/src/filesystem/git.rs
```

Only tracks modifications to already-versioned files. Untracked (new) files are not included — see [`sniff repo untracked-files`](./repo_untracked-files.md) for those.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-p/--package <PKG>` | Scope output to files within a specific package or package area |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more unstaged files found |
| `1` | No unstaged files in scope |

## Package Scoping (`-p`)

When `-p/--package` is provided, only unstaged files within that package's directory are returned:

```bash
sniff repo unstaged-files -p homelab      # Unstaged files in homelab/ area
sniff repo unstaged-files -p sniff-cli    # Unstaged files in sniff/cli/
```

## JSON Output (`--json`)

```
sniff --json repo unstaged-files
```

Returns a JSON array of file path strings:

```json
[
  "sniff/cli/src/output/filesystem.rs"
]
```

Returns an empty array `[]` when no unstaged files are found (exit code 1 still applies).

## Usage in Scripts

```bash
# Stage everything that's modified before committing
if sniff repo unstaged-files > /dev/null; then
    git add -u
fi
```
