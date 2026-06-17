---
blast_radius:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/standard.rs
---

# The `sniff repo is-monorepo` Subcommand

Reports whether the current repository is a monorepo and, when it is, prints the
unified monorepo label derived from the primary authority and any orchestrators.
The determination uses the same repository-structure detection as `sniff repo
structure` and `sniff repo packages`, so it does not trigger network requests or
deep git analysis.

## Default Behavior

Prints a single line and exits `0` inside a monorepo:

```
cargo
```

With an orchestrator on top of an authority, the label is composed as
`{orchestrator_label} (using {authority_label})`:

```
Nx (using pnpm workspaces)
```

Outside a monorepo it prints `false` and exits non-zero:

```
false
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |
| `--no-error` | Exit `0` when not inside a monorepo (still prints `false`). Genuine failures still exit non-zero. |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Inside a monorepo, or not-a-monorepo with `--no-error` |
| non-zero | Not inside a monorepo (without `--no-error`), or a genuine detection failure |

Unlike the boolean exit-code leaves (`is-current-package-area-dirty`,
`has-merge-conflict`), the non-monorepo result is encoded in the exit code by
default. Use `--no-error` when you only need the printed value and want a stable
exit code.

## Examples

```bash
# Plain answer inside a monorepo
sniff repo is-monorepo
# → cargo

# Use in a conditional
if sniff repo is-monorepo >/dev/null 2>&1; then
    echo "Detected monorepo"
fi

# Stable exit code while still printing the predicate
sniff repo is-monorepo --no-error
# → false

# Analyze a different directory
sniff repo is-monorepo -b /path/to/repo
```

## JSON Output (`--json`)

```bash
sniff repo is-monorepo --json
```

Returns a focused object. The key is snake_case and the standard ids are
kebab-case, matching `MonorepoStandard::spec().id`.

Inside a monorepo:

```json
{
  "is_monorepo": true,
  "authority": "cargo-workspace",
  "orchestrators": []
}
```

With orchestrators:

```json
{
  "is_monorepo": true,
  "authority": "pnpm-workspaces",
  "orchestrators": ["nx"]
}
```

Outside a monorepo:

```json
{ "is_monorepo": false }
```

The object is printed to STDOUT first, then the process exits with the predicate
status. STDOUT stays valid JSON even when the exit code is non-zero.

## Bare `sniff repo --json`

Bare `sniff repo --json` exposes the same predicate as the top-level
`is_monorepo` boolean in its consolidated aggregate. The focused
`sniff repo is-monorepo --json` leaf uses the object shape documented above so
it can include authority and orchestrator details when the repository is a
monorepo.

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| [`package-count`](./repo_package-count.md) | Number of discovered packages |
| [`version`](./repo_version.md) | Repository version from the root manifest |
| [`structure`](./repo_structure.md) | Full hierarchical package overview |
