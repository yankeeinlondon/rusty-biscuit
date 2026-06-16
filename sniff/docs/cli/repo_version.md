---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo version` Subcommand

Outputs the repository version read from a root manifest (e.g. the workspace `Cargo.toml`, `package.json`, or `pyproject.toml`), when one is present. Many monorepos have no single root version; in that case the subcommand reports absence via exit code (text) or a `null` body (JSON).

## Default Behavior

Prints the version string and exits `0`. When no root version is found, prints nothing and exits `1`.

```
1.4.2
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |
| `--no-error` | Exit `0` with no output when no version is found |
| `--on-error <MESSAGE>` | Message to display when no version is found |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | A root version was found (or `--no-error` was passed) |
| `1` | No root version found (unless `--no-error`) |

## Examples

```bash
# Print the version
sniff repo version
# → 1.4.2

# No root version: exits 1, no output
sniff repo version
# → (no output, exit code 1)

# Treat absence as success
sniff repo version --no-error

# Custom message when absent
sniff repo version --on-error "no root version"
```

## JSON Output (`--json`)

```bash
sniff repo version --json
```

Returns a single-key `{ "version": "..." | null }` object so JSON consumers always see a stable shape. Exit code mirrors the text path: `0` when a version is present, `1` when `null` (unless `--no-error`, which forces `0`).

```json
{ "version": "1.4.2" }
```

```json
{ "version": null }
```

```bash
# Absent version still emits valid JSON, but exits 1
sniff repo version --json
# → { "version": null }   (exit code 1)

# Force exit 0 while keeping the null body
sniff repo version --json --no-error
# → { "version": null }   (exit code 0)
```

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| [`name`](./repo_name.md) | Repository name |
| [`is-monorepo`](./repo_is-monorepo.md) | Monorepo label (e.g. `cargo`; `false` if not) |
| [`package-count`](./repo_package-count.md) | Number of discovered packages |
