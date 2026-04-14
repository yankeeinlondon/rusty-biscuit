---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo documentation-changes` Subcommand

Lists documentation changes within a time period in a **commit-centric** layout: one block per commit, with only the commit's documentation files surfaced under **Files Impacted**. Commits that did not touch any documentation files are skipped entirely. This shares the same rendering template as `recent-commits` and `source-code-changes`, so the three commands differ only in which files they include.

## What Counts as Documentation

File classification uses the sniff file-type registry. A file is considered documentation if:

- Its association is **Documentation** (e.g., `README`, `CHANGELOG`, `CONTRIBUTING` without extension)
- Its extension matches a known documentation format: `.md`, `.mdx`, `.rst`, `.txt`, `.adoc`

Source code, configuration, and other non-documentation files are excluded.

## Default Behavior

When no period is specified, defaults to `3d` (last 3 days). Output is rendered as Markdown with a heading that includes the period label, followed by one block per commit:

```
### Documentation Changes (today)

- [e0a1034] docs(sniff) at 10:12am Today: document commit-centric changes layout

    **Description:**

    - Add examples matching the new per-commit block
    - Note the change-kind prefix in Files Impacted

    **Files Impacted:**

    - modified: sniff/docs/cli/repo_source-code-changes.md
    - modified: sniff/docs/cli/repo_documentation-changes.md

- [a1b5d77] docs(sniff) at 12:32pm Today: add program-installation documentation placeholder

    **Files Impacted:**

    - added: sniff/docs/topics/program-installation.md
```

Notes:

- **Files Impacted** lists only documentation files; any non-doc files touched by the same commit are hidden from this view.
- Each file is prefixed by its change kind (`added`, `modified`, `deleted`, `renamed`, `copied`).
- File paths are rendered as clickable OSC8 hyperlinks (pointing to `file://` URIs) in terminals that support them.
- Commit timestamps are displayed in the viewer's local timezone with `Today`/`Yesterday` labels.
- Terminal styling (bold hash, blue conventional prefix with dim scope, italic `at`, bold time) is shared with [`sniff repo recent-commits`](./repo_recent-commits.md#styled-terminal-output); see that document for the full styling table. `--plain` strips all ANSI escapes.

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
| Count | `10`, `25` | The last N commits reachable from HEAD (bare positive integer) |

An all-digit argument is always interpreted as a count, even when it is long enough to look like a SHA. See [`sniff repo recent-commits`](./repo_recent-commits.md#period-argument) for details.

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
| `--action <feat\|chore\|refactor\|test\|style\|fix>` | Filter to one or more conventional commit actions |
| `--package <PKG>` | Scope to commits touching a specific package |
| `--package-area <AREA>` | Scope to commits touching a specific package area |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Conventional Commit Action Filtering

Use `--action` to keep only commits whose summary matches one of these conventional commit actions: `feat`, `chore`, `refactor`, `test`, `style`, `fix`.

The flag may be repeated; multiple `--action` values are OR'd together. Non-conventional commits are excluded when `--action` filtering is active.

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
sniff repo documentation-changes 10                  # The last 10 commits
sniff repo documentation-changes --action feat      # Only conventional feat commits
sniff repo documentation-changes --action chore --action refactor
sniff repo documentation-changes 2w --package sniff # Last 2 weeks, sniff package only
```

## JSON Output (`--json`)

```bash
sniff --json repo documentation-changes 1w
```

Returns the same `CommitDescSet` object as `recent-commits` — JSON is not filtered down to documentation files, it always carries every changed file for the commit. The text-only rendering is the only view that prunes non-documentation files:

```json
{
  "commits": [
    {
      "hash": "5e8f2a1b...",
      "datetime": "2026-04-09T16:00:00+00:00",
      "packages": ["sniff"],
      "package_areas": ["sniff"],
      "files": [
        { "path": "sniff/docs/cli/repo_recent-commits.md",        "kind": "modified" },
        { "path": "sniff/lib/src/filesystem/git/recent_commits.rs", "kind": "modified" }
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

Each `files` item is a record of `{ path, kind }` where `kind` is one of `added`, `modified`, `deleted`, `renamed`, `copied`.

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
