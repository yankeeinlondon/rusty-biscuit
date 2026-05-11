---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo staged-packages` Subcommand

Lists package names that have files staged in the git index (ready to commit). Exits with code 1 if no staged packages are found.

## Default Behavior

Outputs a comma-separated list of package names:

```
sniff-cli, sniff
```

A package is included if any file within its directory tree is currently staged.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[filter...]` | Optional substring filters to narrow the package scope |
| `-p/--package <PKG>` | Restrict output to a single package (exact match on `Package.name`) |
| `--package-area <AREA>` | Restrict output to packages in the given area (prefix match on `Package.package_area`) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more packages with staged files found |
| `1` | No staged packages, or not in a monorepo |

## Package Filtering

```bash
sniff repo staged-packages              # All packages with staged files
sniff repo staged-packages @sniff       # Staged packages in sniff area
sniff repo staged-packages !vendor      # Staged packages excluding vendor
```

## Scoping to a Package or Package Area

`-p/--package` matches `Package.name` exactly (case-insensitive); `--package-area` matches `Package.package_area` as a case-insensitive prefix. Either can be combined with the positional filter (AND).

```bash
sniff repo staged-packages -p sniff-cli            # Just sniff-cli, when staged
sniff repo staged-packages --package-area homelab  # Staged packages in homelab/*
```

Passing both flags requires the resolved package to live inside the resolved area; otherwise the command fails with an explicit error. Unknown values for either flag fail with an error listing the valid names.

## JSON Output (`--json`)

```
sniff --json repo staged-packages
```

Returns a `{ scope, kind, names }` object. `scope` is `"staged"`, `kind`
is `"packages"`, and `names` is an array of package name strings
(empty when the repo is not a monorepo):

```json
{
  "scope": "staged",
  "kind": "packages",
  "names": ["sniff-cli", "sniff"]
}
```

## Related Subcommands

| Subcommand | Scope |
|------------|-------|
| [`staged-files`](./repo_staged-files.md) | Individual file paths |
| `staged-packages` | Package names with staged files |
| [`staged-package-areas`](./repo_staged-package-areas.md) | Area names with staged files |
