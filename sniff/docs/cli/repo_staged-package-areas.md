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
