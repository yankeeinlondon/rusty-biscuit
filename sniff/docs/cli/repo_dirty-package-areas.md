---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo dirty-package-areas` Subcommand

Lists package area names (top-level directory groups) that have at least one package with uncommitted changes. Exits with code 1 if no dirty areas are found.

## Default Behavior

Outputs a comma-separated list of area names:

```
sniff, homelab
```

An area is reported as dirty if any package within it has staged, unstaged, or untracked file changes.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[filter...]` | Optional substring filters to narrow the area scope |
| `-p/--package <PKG>` | Restrict output to the area that contains the named package (exact match on `Package.name`) |
| `--package-area <AREA>` | Restrict output to a specific package area (prefix match) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more dirty areas found |
| `1` | No dirty areas, or not in a monorepo |

## Package Filtering

```bash
sniff repo dirty-package-areas            # All dirty areas
sniff repo dirty-package-areas homelab    # Only the homelab area (if dirty)
sniff repo dirty-package-areas !biscuit   # All dirty areas except biscuit
```

## Scoping to a Package or Package Area

`-p/--package` reduces the result to the area containing the resolved package; `--package-area` applies a case-insensitive prefix filter on the area name. Both can be combined with the positional filter (AND).

```bash
sniff repo dirty-package-areas -p sniff-cli            # The area sniff-cli lives in (when dirty)
sniff repo dirty-package-areas --package-area homelab  # Dirty homelab/* areas
```

Passing both flags requires the resolved package to live inside the resolved area; otherwise the command fails with an explicit error. Unknown values for either flag fail with an error listing the valid names.

## JSON Output (`--json`)

```
sniff --json repo dirty-package-areas
```

Returns a `{ scope, kind, names }` object. `scope` is `"dirty"`, `kind`
is `"package_areas"`, and `names` is an array of area name strings
(empty when the repo is not a monorepo):

```json
{
  "scope": "dirty",
  "kind": "package_areas",
  "names": ["sniff", "homelab"]
}
```

## Relationship to `dirty-packages`

- [`dirty-packages`](./repo_dirty-packages.md) — returns individual package names
- `dirty-package-areas` — returns deduped area names (useful for area-level CI gates)

```bash
# These may return different granularity:
sniff repo dirty-packages       # → sniff-cli, sniff
sniff repo dirty-package-areas  # → sniff
```
