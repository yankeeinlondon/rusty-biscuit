---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo unstaged-package-areas` Subcommand

Lists package area names that have at least one package with unstaged modifications. Exits with code 1 if no such areas are found.

## Default Behavior

Outputs a comma-separated list of area names:

```
sniff, homelab
```

An area is included if any package within it has modified (but not staged) files in the working tree. Untracked files are not included — use [`dirty-package-areas`](./repo_dirty-package-areas.md) to include those.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[filter...]` | Optional substring filters to narrow the area scope |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more areas with unstaged modifications found |
| `1` | No unstaged areas, or not in a monorepo |

## Package Filtering

```bash
sniff repo unstaged-package-areas              # All areas with unstaged changes
sniff repo unstaged-package-areas @homelab     # Only homelab area (if unstaged)
sniff repo unstaged-package-areas !biscuit     # Unstaged areas excluding biscuit
```

## JSON Output (`--json`)

```
sniff --json repo unstaged-package-areas
```

Returns a `{ scope, kind, names }` object. `scope` is `"unstaged"`,
`kind` is `"package_areas"`, and `names` is an array of area name
strings (empty when the repo is not a monorepo):

```json
{
  "scope": "unstaged",
  "kind": "package_areas",
  "names": ["sniff", "homelab"]
}
```

## Related Subcommands

| Subcommand | Scope |
|------------|-------|
| [`unstaged-files`](./repo_unstaged-files.md) | Individual file paths |
| [`unstaged-packages`](./repo_unstaged-packages.md) | Package names with unstaged files |
| `unstaged-package-areas` | Area names with unstaged files |
| [`dirty-package-areas`](./repo_dirty-package-areas.md) | All changed areas (staged + unstaged + untracked) |
