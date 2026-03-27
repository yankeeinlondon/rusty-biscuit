---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/docs.rs
---

# The `sniff blast-radius` Command

Finds markdown documents whose `blast_radius` frontmatter intersects with changed source code files. Use this to identify which documentation may need updating after code changes.

## How It Works

1. Collects changed **source code** files for the chosen scope (dirty, staged, or last commit).
2. Scans all markdown documents in the repository for a `blast_radius` frontmatter key.
3. Returns documents where at least one path in their `blast_radius` list matches a changed source file.

A document's `blast_radius` frontmatter declares which source files it covers:

```yaml
---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/lib/src/filesystem/git.rs
---
```

If any of those files appear in the changed set, the document is returned as a candidate for review.

## Default Behavior

Outputs one document path per line, relative to the repository root:

```
sniff/docs/cli/repo_staged-files.md
sniff/docs/cli/repo_dirty-packages.md
```

Paths are rendered with OSC8 hyperlinks and styled with dim directory segments and bold filenames. Use `--plain` to strip escape codes.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[SCOPE]` | Which changes to inspect: `dirty` (default), `staged`, `last-commit` |
| `--package <PKG>` | Scope changed files to a specific package |
| `--package-area <AREA>` | Scope changed files to a specific package area |
| `--list` | Output as bullet list (`- ` prefix) |
| `--csv` | Output as comma-separated values on a single line |
| `--no-path` | Show only the filename (hide directory path) |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more matching documents found |
| `1` | No matching documents (default behavior) |
| `0` | No matching documents with `--no-error` |

## Scope Variants

```bash
sniff blast-radius              # Dirty files (staged + unstaged + untracked)
sniff blast-radius dirty        # Same as above (explicit)
sniff blast-radius staged       # Only staged files
sniff blast-radius last-commit  # Files changed in the HEAD commit
```

## Package Scoping

In monorepos, `--package` and `--package-area` limit which changed files are considered. The document scan is always repo-wide -- only the source file set is narrowed:

```bash
sniff blast-radius --package sniff-cli       # Only changes in sniff/cli/
sniff blast-radius --package-area homelab    # Only changes in homelab/
```

## Output Formats

```bash
sniff blast-radius                  # One path per line (default)
sniff blast-radius --list           # Bullet list
sniff blast-radius --csv            # Comma-separated
sniff blast-radius --no-path        # Basenames only
sniff blast-radius --plain          # Strip ANSI/OSC8 codes
```

## JSON Output (`--json`)

```bash
sniff blast-radius --json
```

Returns a JSON object with the scope and matched documents:

```json
{
  "scope": "Dirty",
  "documents": [
    {
      "relative": "sniff/docs/cli/repo_staged-files.md",
      "title": "The sniff repo staged-files Subcommand",
      "blast_radius": [
        "sniff/cli/src/args.rs",
        "sniff/cli/src/commands.rs"
      ]
    }
  ]
}
```

## No-Result Behavior

When no documents match, the default is to exit with code 1 and produce no output. This can be customized:

```bash
# Silent success when nothing matches
sniff blast-radius --no-error

# Show a styled message on stderr when nothing matches
sniff blast-radius --on-error "<yellow>No docs need updating</yellow>"

# Combined: message to stdout, exit 0
sniff blast-radius --no-error --on-error "All clear"
```

## Working with `sniff docs --blast-radius`

The `sniff docs --blast-radius` flag is a complementary feature that lists all documents which *have* a `blast_radius` frontmatter key, regardless of what source files have changed. This is useful for auditing which documents are wired up for blast-radius tracking.

| Command | What it shows |
|---------|---------------|
| `sniff docs --blast-radius` | All documents with a `blast_radius` key (inventory) |
| `sniff blast-radius` | Documents whose `blast_radius` intersects current changes (actionable) |

A typical workflow:

```bash
# 1. See which docs are blast-radius aware
sniff docs --blast-radius

# 2. After making code changes, find docs that may need updating
sniff blast-radius

# 3. Scope to only staged changes before committing
sniff blast-radius staged
```

Combined with `--verbose`, `sniff docs --blast-radius -v` shows metadata for each tracked document including its title, last-updated date, and frontmatter properties.

## Usage in Scripts

```bash
# Gate a doc review step on blast radius
if sniff blast-radius staged 2>/dev/null; then
    echo "These docs may need updating:"
    sniff blast-radius staged --list --plain
fi

# Get affected doc paths as an array
mapfile -t DOCS < <(sniff blast-radius --plain)

# CI: warn but don't block
sniff blast-radius staged --no-error --on-error "<yellow>Consider reviewing docs</yellow>"
```

## Usage in CI/CD

```yaml
- name: Check for affected documentation
  id: blast-radius
  run: |
    if sniff blast-radius staged --plain > affected-docs.txt 2>/dev/null; then
      echo "affected=true" >> $GITHUB_OUTPUT
    else
      echo "affected=false" >> $GITHUB_OUTPUT
    fi

- name: Warn about stale docs
  if: steps.blast-radius.outputs.affected == 'true'
  run: |
    echo "::warning::The following docs may need updating:"
    cat affected-docs.txt
```

## Related Commands

| Command | Purpose |
|---------|---------|
| [`sniff docs --blast-radius`](#working-with-sniff-docs---blast-radius) | List all blast-radius-aware documents |
| [`sniff repo dirty-source-code`](./repo_dirty-source-code.md) | List dirty source code files |
| [`sniff repo staged-source-code`](./repo_staged-source-code.md) | List staged source code files |
| [`sniff repo package-area-has-source-code-changes`](./repo_package-area-has-source-code-changes.md) | Check if package area has source changes |
