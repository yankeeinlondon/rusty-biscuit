---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo unstaged-packages` Subcommand

Lists package names that have modified files in the working tree (not yet staged). Exits with code 1 if no unstaged packages are found.

## Default Behavior

Outputs a comma-separated list of package names:

```
sniff-cli, sniff
```

A package is included if any file within its directory tree is modified but not staged. Untracked (new) files are not included — use [`dirty-packages`](./repo_dirty-packages.md) to catch those as well.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[filter...]` | Optional substring filters to narrow the package scope |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more packages with unstaged modifications found |
| `1` | No unstaged packages, or not in a monorepo |

## Package Filtering

```bash
sniff repo unstaged-packages              # All packages with unstaged changes
sniff repo unstaged-packages @biscuit     # Unstaged in biscuit area
sniff repo unstaged-packages !test        # Unstaged packages excluding test
```

## JSON Output (`--json`)

```
sniff --json repo unstaged-packages
```

Returns a `{ scope, kind, names }` object. `scope` is `"unstaged"`,
`kind` is `"packages"`, and `names` is an array of package name strings
(empty when the repo is not a monorepo):

```json
{
  "scope": "unstaged",
  "kind": "packages",
  "names": ["sniff-cli", "sniff"]
}
```

## Related Subcommands

| Subcommand | Scope |
|------------|-------|
| [`unstaged-files`](./repo_unstaged-files.md) | Individual file paths |
| `unstaged-packages` | Package names with unstaged modifications |
| [`unstaged-package-areas`](./repo_unstaged-package-areas.md) | Area names with unstaged modifications |
| [`dirty-packages`](./repo_dirty-packages.md) | All changed packages (staged + unstaged + untracked) |
