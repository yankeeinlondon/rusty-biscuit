---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo package-area-has-source-code-changes` Subcommand

Checks whether the current package area has any source code file changes (staged, unstaged, or untracked). Unlike [`is-current-package-area-dirty`](./repo_is-current-package-area-dirty.md), this ignores non-source files such as documentation, config files, and lock files.

## Default Behavior

Produces no output. Use the exit code in shell conditionals.

With `-v/--verbose`, prints a human-readable message before exiting:
```
Source code changes detected in homelab
```

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |
| `-v/--verbose` | Print a human-readable message before exiting |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Source code files have changes in the current package area |
| `1` | No source code changes, or not inside a recognized package area |

## Source Code Extensions

The following file extensions are recognized as source code:

`rs`, `ts`, `tsx`, `js`, `jsx`, `vue`, `py`, `go`, `c`, `cpp`, `h`, `hpp`, `java`, `kt`, `kts`, `swift`, `rb`, `php`, `cs`, `fs`, `fsx`, `scala`, `sh`, `bash`, `zsh`, `fish`, `sql`, `zig`, `elm`, `ex`, `exs`, `erl`, `hrl`, `lua`, `r`, `jl`, `dart`, `toml`, `yaml`, `yml`, `json`, `graphql`

Files matching only documentation (`.md`, `.txt`, `.rst`) or build artifacts are excluded.

## Examples

```bash
# Gate a build on source changes
if sniff repo package-area-has-source-code-changes; then
    cargo test
else
    echo "No source changes, skipping tests"
fi

# Verbose mode shows context
sniff repo package-area-has-source-code-changes -v
# → Source code changes detected in sniff

# From a specific directory
sniff repo package-area-has-source-code-changes -b /path/to/homelab/lib
```

## Usage in CI/CD

```yaml
# Skip expensive steps when only docs changed
- name: Check for source changes
  id: source-check
  run: sniff repo package-area-has-source-code-changes && echo "changed=true" >> $GITHUB_OUTPUT || echo "changed=false" >> $GITHUB_OUTPUT

- name: Run tests
  if: steps.source-check.outputs.changed == 'true'
  run: cargo test
```

## JSON Output (`--json`)

```bash
sniff --json repo package-area-has-source-code-changes
```

Returns a `{ has_source_code_changes: bool }` object so JSON consumers
get a stable shape even when the underlying state is "no changes".
The exit code mirrors the boolean: `0` when `true`, `1` when `false`
(or when the current directory is outside any package area).

```json
{ "has_source_code_changes": true }
```

## Related Subcommands

| Subcommand | Behavior |
|------------|----------|
| [`is-current-package-area-dirty`](./repo_is-current-package-area-dirty.md) | Exit code, all file types |
| `package-area-has-source-code-changes` | Exit code, source files only |
