---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo documentation-changes` Subcommand

Lists documentation files changed within a time period, grouped by file. Each file entry shows which commits touched it and why. This is the documentation-focused complement to `source-code-changes`.

## What Counts as Documentation

File classification uses the sniff file-type registry. A file is considered documentation if:

- Its association is **Documentation** (e.g., `README`, `CHANGELOG`, `CONTRIBUTING` without extension)
- Its extension matches a known documentation format: `.md`, `.mdx`, `.rst`, `.txt`, `.adoc`

Source code, configuration, and other non-documentation files are excluded.

## Default Behavior

When no period is specified, defaults to `3d` (last 3 days). Output is rendered as Markdown with a heading that includes the period label:

```
### Documentation Changes (last 3d)

- sniff/docs/cli/repo_recent-commits.md
    - 2026-04-09 at 16:00 - docs(sniff): add CLI docs for recent-commits subcommand as part of commit 5e8f2a1b
        - Documented period argument formats
        - Added JSON output examples
- CHANGELOG.md
    - 2026-04-08 at 11:30 - docs: update changelog for v0.5.0 as part of commit 7c3d9e4f
```

File paths are rendered as clickable OSC8 hyperlinks (pointing to `file://` URIs) in terminals that support them.

## Period Argument

```
sniff repo documentation-changes [PERIOD]
```

The optional `PERIOD` argument accepts several formats:

| Format | Example | Meaning |
|--------|---------|---------|
| Duration | `3d`, `1w`, `2mo`, `6h` | Relative duration from now |
| Named | `today` | Since midnight UTC today |
| Named | `yesterday` | Midnight-to-midnight UTC yesterday |
| Date | `2026-04-01` | All commits on that date (YYYY-MM-DD) |
| Hash | `a1b2c3d` | All commits from that hash to HEAD |

### Duration Units

| Unit | Aliases |
|------|---------|
| Hours | `h`, `hour`, `hours` |
| Days | `d`, `day`, `days` |
| Weeks | `w`, `wk`, `week`, `weeks` |
| Months | `mo`, `m`, `month`, `months` (30 days) |
| Years | `y`, `yr`, `year`, `years` (365 days) |

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `[PERIOD]` | Time period (default: `3d`) |
| `--package <PKG>` | Scope to commits touching a specific package |
| `--package-area <AREA>` | Scope to commits touching a specific package area |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Package Scoping

In monorepos, `--package` and `--package-area` filter commits before file grouping. Only commits that touched files within the specified scope are considered:

```bash
sniff repo documentation-changes 1w --package sniff
sniff repo documentation-changes --package-area homelab
```

## Examples

```bash
sniff repo documentation-changes                    # Last 3 days (default)
sniff repo documentation-changes 1w                 # Last week
sniff repo documentation-changes today              # Since midnight
sniff repo documentation-changes yesterday          # Yesterday only
sniff repo documentation-changes 2026-04-01         # Specific date
sniff repo documentation-changes a1b2c3d            # From hash to HEAD
sniff repo documentation-changes 2w --package sniff # Last 2 weeks, sniff package only
```

## JSON Output (`--json`)

```bash
sniff --json repo documentation-changes 1w
```

Returns the same `CommitDescSet` object as `recent-commits`. The file-grouped rendering is a text-only view — JSON always returns the full commit list:

```json
{
  "commits": [
    {
      "hash": "5e8f2a1b...",
      "datetime": "2026-04-09T16:00:00+00:00",
      "packages": ["sniff"],
      "package_areas": ["sniff"],
      "files": [
        "sniff/docs/cli/repo_recent-commits.md",
        "sniff/lib/src/filesystem/git/recent_commits.rs"
      ],
      "description": "docs(sniff): add CLI docs for recent-commits subcommand",
      "bullet_points": [
        "Documented period argument formats",
        "Added JSON output examples"
      ]
    }
  ],
  "period_label": "last 1w",
  "repo_root": "/absolute/path/to/repo"
}
```

## Plain Output (`--plain`)

Adding `--plain` strips all ANSI escape codes and OSC8 hyperlinks from the text output. File paths are rendered as plain text instead of clickable links.

## No-Result Behavior

When no documentation files are found in the period (either no commits exist or no commits touched documentation), the default is to exit with code 1. This can be customized:

```bash
sniff repo documentation-changes --no-error
sniff repo documentation-changes --on-error "No doc changes in this period"
sniff repo documentation-changes --no-error --on-error "All quiet"
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more documentation files found |
| `1` | No documentation changes (default behavior) |
| `0` | No documentation changes with `--no-error` |

## Related Commands

| Command | Purpose |
|---------|---------|
| [`sniff repo recent-commits`](./repo_recent-commits.md) | Commits in a period, grouped by commit |
| [`sniff repo source-code-changes`](./repo_source-code-changes.md) | Source code files changed in a period, grouped by file |
| [`sniff repo dirty-files`](./repo_dirty-files.md) | All uncommitted file changes |
| [`sniff blast-radius`](./repo_blast-radius.md) | Docs whose blast radius intersects changed code |
