---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo is-current-package-area-dirty` Subcommand

Checks whether the current package area has any uncommitted changes (staged, unstaged, or untracked files). Communicates the result purely via exit code — no output is produced.

## Default Behavior

Produces no output. Use the exit code in shell conditionals.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Current package area has uncommitted changes (is dirty) |
| `1` | Package area is clean, or not inside a recognized package area |

A package area is considered dirty if any package within it has staged files, unstaged modifications, or untracked files.

## Examples

```bash
# Shell conditional
if sniff repo is-current-package-area-dirty; then
    echo "This area has uncommitted changes"
else
    echo "Working tree is clean"
fi

# In a justfile recipe
check-clean:
    @sniff repo is-current-package-area-dirty && echo "Dirty!" || echo "Clean"

# Use with set -e to abort on dirty state
set -e
sniff repo is-current-package-area-dirty && { echo "Commit your changes first"; exit 1; }
```

## Usage in CI/CD

```yaml
# GitHub Actions example
- name: Check for uncommitted changes
  run: |
    if sniff repo is-current-package-area-dirty; then
      echo "::warning::Uncommitted changes detected"
    fi
```

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| `is-current-package-area-dirty` | Exit code only |
| [`dirty-package-areas`](./repo_dirty-package-areas.md) | Area names as comma-separated list |
| [`dirty-packages`](./repo_dirty-packages.md) | Package names as comma-separated list |
| [`package-area-has-source-code-changes`](./repo_package-area-has-source-code-changes.md) | Exit code, filtered to source files only |
