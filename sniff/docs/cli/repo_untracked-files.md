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
| `-p/--package <PKG>` | Scope output to files within a specific package (exact match on `Package.name`) |
| `--package-area <AREA>` | Scope output to files within a specific package area (prefix match on `Package.package_area`) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more untracked files found |
| `1` | No untracked files in scope |

## Package Scoping (`-p` and `--package-area`)

`-p/--package` scopes the listing to a single package; `--package-area` scopes to all packages whose area starts with the supplied prefix. Passing both narrows to the intersection (and errors if the package is not inside the area).

```bash
sniff repo untracked-files -p sniff-cli                  # New files in sniff/cli/
sniff repo untracked-files --package-area sniff          # New files in sniff/* (sniff/cli, sniff/lib, …)
sniff repo untracked-files --package-area research       # New files in research/* areas
```

> Breaking change vs. previous releases: passing a **package-area name** (e.g., `sniff`) to `-p/--package` is now an error. Use `--package-area sniff` instead.

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
if sniff repo untracked-files --package-area sniff > /dev/null; then
    echo "New files detected in sniff/"
fi
```
