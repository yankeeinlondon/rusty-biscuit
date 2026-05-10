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
| `-p/--package <PKG>` | Restrict output to a single package (exact match on `Package.name`) |
| `--package-area <AREA>` | Restrict output to packages in the given area (prefix match on `Package.package_area`) |

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

## Scoping to a Package or Package Area

`-p/--package` matches `Package.name` exactly (case-insensitive); `--package-area` matches `Package.package_area` as a case-insensitive prefix. Either can be combined with the positional filter (AND).

```bash
sniff repo unstaged-packages -p sniff-cli              # Just sniff-cli, when unstaged
sniff repo unstaged-packages --package-area homelab    # Unstaged packages in homelab/*
```

Passing both flags requires the resolved package to live inside the resolved area; otherwise the command fails with an explicit error. Unknown values for either flag fail with an error listing the valid names.

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
