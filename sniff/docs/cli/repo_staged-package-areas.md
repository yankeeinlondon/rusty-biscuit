---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo staged-package-areas` Subcommand

Lists package area names that have at least one package with staged files. Exits with code 1 if no staged areas are found.

## Default Behavior

Outputs a comma-separated list of area names:

```
sniff
```

An area is included if any package within it has files in the git index.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[filter...]` | Optional substring filters to narrow the area scope |
| `-p/--package <PKG>` | Restrict output to the area that contains the named package (exact match on `Package.name`) |
| `--package-area <AREA>` | Restrict output to a specific package area (prefix match) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more areas with staged files found |
| `1` | No staged areas, or not in a monorepo |

## Package Filtering

```bash
sniff repo staged-package-areas              # All areas with staged files
sniff repo staged-package-areas @homelab     # Only homelab area (if staged)
sniff repo staged-package-areas !biscuit     # All staged areas except biscuit
```

## Scoping to a Package or Package Area

`-p/--package` reduces the result to the area containing the resolved package; `--package-area` applies a case-insensitive prefix filter on the area name. Both can be combined with the positional filter (AND).

```bash
sniff repo staged-package-areas -p sniff-cli            # The area sniff-cli lives in (when staged)
sniff repo staged-package-areas --package-area homelab  # Staged homelab/* areas
```

Passing both flags requires the resolved package to live inside the resolved area; otherwise the command fails with an explicit error. Unknown values for either flag fail with an error listing the valid names.

## JSON Output (`--json`)

```
sniff --json repo staged-package-areas
```

Returns a `{ scope, kind, names }` object. `scope` is `"staged"`, `kind`
is `"package_areas"`, and `names` is an array of area name strings
(empty when the repo is not a monorepo):

```json
{
  "scope": "staged",
  "kind": "package_areas",
  "names": ["sniff"]
}
```

## Related Subcommands

| Subcommand | Scope |
|------------|-------|
| [`staged-files`](./repo_staged-files.md) | Individual file paths |
| [`staged-packages`](./repo_staged-packages.md) | Package names with staged files |
| `staged-package-areas` | Area names with staged files |
