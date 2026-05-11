---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo package` Subcommand

Outputs the package name of the current directory. Useful for scripts that need to identify which package they are operating in.

## Default Behavior

Outputs a single package name:

```
sniff-cli
```

Exits with code 1 if the current directory is not inside a recognized package.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |
| `-v/--verbose` | Show the package location alongside the name |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Verbose Mode (`-v`)

With `-v`, the output includes the relative path:

```
sniff-cli (located in sniff/cli)
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Current directory belongs to a package |
| `1` | Not inside a recognized package (unless `--no-error`) |

## Examples

```bash
# From inside sniff/cli/
sniff repo package
# → sniff-cli

# From inside sniff/lib/
sniff repo package
# → sniff

# Analyze a specific directory
sniff repo package -b /path/to/homelab/server
# → homelab-server
```

## Error Handling

By default, exits with code 1 when not inside a package. Use `--no-error` and `--on-error` to customize:

```bash
# Silently succeed with no output
sniff repo package --no-error

# Show a custom message on stderr and exit 1
sniff repo package --on-error "Not in a package"

# Show a custom message on stdout and exit 0
sniff repo package --no-error --on-error "Not in a package"
```

## Usage in Scripts

```bash
# Get the current package name for cargo
PKG=$(sniff repo package)
cargo build -p "$PKG"

# Conditional logic based on package
if [ "$(sniff repo package)" = "sniff-cli" ]; then
    echo "In the sniff CLI package"
fi

# Safe fallback when not in a package
PKG=$(sniff repo package --no-error)
if [ -n "$PKG" ]; then
    cargo test -p "$PKG"
fi
```

## JSON Output (`--json`)

```bash
sniff --json repo package
```

Returns a `{ name: "<pkg>" }` object. Exit code semantics still
honour `--no-error` / `--on-error`.

```json
{ "name": "sniff-cli" }
```
