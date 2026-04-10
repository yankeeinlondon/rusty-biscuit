---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo source-code-changes` Subcommand

Lists source code files changed within a time period, grouped by file. Each file entry shows which commits touched it and why. This is the file-centric complement to `recent-commits`, which is commit-centric.

## What Counts as Source Code

File classification uses the sniff file-type registry. A file is considered source code if its association is one of:

- **ProgrammingLanguage** — `.rs`, `.ts`, `.py`, `.go`, `.java`, etc.
- **FrameworkFile** — framework-specific files (e.g., `vite.config.ts`)
- **Styling** — `.css`, `.scss`, `.less`, etc.
- **HTML/HTM** — accepted as source code (historical convention)

Configuration files (`.toml`, `.json`, `.yaml`), documentation (`.md`, `.txt`), and other non-source files are excluded.

## Default Behavior

When no period is specified, defaults to `3d` (last 3 days). Output is rendered as Markdown with a heading that includes the period label:

```
### Source Code Changes (last 3d)

- sniff/lib/src/filesystem/git/recent_commits.rs
    - 2026-04-09 at 14:32 - refactor(sniff): use let-chains and simplify parse_commit_message as part of commit 34b6d18a
        - Replaced nested if/match blocks with let-chains
    - 2026-04-08 at 09:15 - fix(sniff): preserve empty commits and use time-monotonic sort as part of commit 43b3be02
        - Empty commits are no longer dropped during range walking
- sniff/cli/src/output/recent_commits.rs
    - 2026-04-09 at 14:32 - refactor(sniff): use let-chains and simplify parse_commit_message as part of commit 34b6d18a
```

File paths are rendered as clickable OSC8 hyperlinks (pointing to `file://` URIs) in terminals that support them.

## Period Argument

```
sniff repo source-code-changes [PERIOD]
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
sniff repo source-code-changes 1w --package sniff-cli
sniff repo source-code-changes --package-area homelab
```

## Examples

```bash
sniff repo source-code-changes                    # Last 3 days (default)
sniff repo source-code-changes 1w                 # Last week
sniff repo source-code-changes today              # Since midnight
sniff repo source-code-changes yesterday          # Yesterday only
sniff repo source-code-changes 2026-04-01         # Specific date
sniff repo source-code-changes a1b2c3d            # From hash to HEAD
sniff repo source-code-changes 2w --package sniff # Last 2 weeks, sniff package only
```

## JSON Output (`--json`)

```bash
sniff --json repo source-code-changes 1w
```

Returns the same `CommitDescSet` object as `recent-commits`. The file-grouped rendering is a text-only view — JSON always returns the full commit list:

```json
{
  "commits": [
    {
      "hash": "34b6d18a...",
      "datetime": "2026-04-09T14:32:00+00:00",
      "packages": ["sniff", "sniff-cli"],
      "package_areas": ["sniff"],
      "files": [
        "sniff/lib/src/filesystem/git/recent_commits.rs",
        "sniff/cli/src/output/recent_commits.rs"
      ],
      "description": "refactor(sniff): use let-chains and simplify parse_commit_message",
      "bullet_points": [
        "Replaced nested if/match blocks with let-chains"
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

When no source code files are found in the period (either no commits exist or no commits touched source code), the default is to exit with code 1. This can be customized:

```bash
sniff repo source-code-changes --no-error
sniff repo source-code-changes --on-error "No source changes in this period"
sniff repo source-code-changes --no-error --on-error "All quiet"
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more source code files found |
| `1` | No source code changes (default behavior) |
| `0` | No source code changes with `--no-error` |

## Related Commands

| Command | Purpose |
|---------|---------|
| [`sniff repo recent-commits`](./repo_recent-commits.md) | Commits in a period, grouped by commit |
| [`sniff repo documentation-changes`](./repo_documentation-changes.md) | Documentation files changed in a period, grouped by file |
| [`sniff repo dirty-source-code`](./repo_dirty-source-code.md) | Uncommitted source code changes |
| [`sniff repo staged-source-code`](./repo_staged-source-code.md) | Staged source code changes |
