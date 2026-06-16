---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo package-count` Subcommand

Reports the number of packages discovered in the repository. For a single-package repository this is `1`; for a monorepo it is the count of participating workspace members.

## Default Behavior

Prints the count as a single integer and exits `0`.

```
65
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Identity detection succeeded |

When no packages are detected the count is `0` (still exit `0`).

## Examples

```bash
# Plain count
sniff repo package-count
# → 65

# Analyze a different directory
sniff repo package-count -b /path/to/repo
```

## JSON Output (`--json`)

```bash
sniff repo package-count --json
```

Returns a single-key `{ "package-count": N }` object. The key is kebab-case, matching the bare `sniff repo --json` aggregate.

```json
{ "package-count": 65 }
```

```bash
# Read the value into a shell variable
COUNT=$(sniff repo package-count --json | jq '."package-count"')
echo "Workspace has $COUNT packages"
```

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| [`is-monorepo`](./repo_is-monorepo.md) | Monorepo label (e.g. `cargo`; `false` if not) |
| [`packages`](./repo_packages.md) | The package names themselves |
| [`structure`](./repo_structure.md) | Full hierarchical package overview |
