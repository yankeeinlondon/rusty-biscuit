---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo is-monorepo` Subcommand

Reports whether the current repository is a monorepo. In text mode it prints `yes` or `no`; the determination comes from the same repository-identity detection used by the rest of `sniff repo`.

## Default Behavior

Prints a single line, `yes` or `no`, and exits `0`.

```
yes
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Identity detection succeeded (regardless of the `yes`/`no` answer) |

Unlike the boolean exit-code leaves (`is-current-package-area-dirty`, `has-merge-conflict`), this subcommand does not encode the answer in the exit code — read the printed value (text) or the JSON body instead.

## Examples

```bash
# Plain answer
sniff repo is-monorepo
# → yes

# Analyze a different directory
sniff repo is-monorepo -b /path/to/repo
```

## JSON Output (`--json`)

```bash
sniff repo is-monorepo --json
```

Returns a single-key `{ "is-monorepo": bool }` object. The key is kebab-case, matching the bare `sniff repo --json` aggregate.

```json
{ "is-monorepo": true }
```

```bash
# Branch on the value
if sniff repo is-monorepo --json | jq -e '."is-monorepo"'; then
    echo "Detected monorepo"
fi
```

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| [`package-count`](./repo_package-count.md) | Number of discovered packages |
| [`version`](./repo_version.md) | Repository version from the root manifest |
| [`structure`](./repo_structure.md) | Full hierarchical package overview |
