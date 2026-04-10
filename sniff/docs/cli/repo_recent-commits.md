---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
---

# The `sniff repo recent-commits` Subcommand

Shows commits within a time period, grouped by date. Each commit displays its short hash, changed files, and a structured description parsed from the commit message.

## Default Behavior

When no period is specified, defaults to `3d` (last 3 days). Output is rendered as Markdown passed through a terminal renderer:

```
## 2026-04-09 at 14:32

- Commit: 34b6d18a
- Files:
    - sniff/lib/src/filesystem/git/recent_commits.rs
    - sniff/cli/src/output/recent_commits.rs
- Description: refactor(sniff): use let-chains and simplify parse_commit_message
    - Replaced nested if/match blocks with let-chains
    - Simplified scope extraction logic

## 2026-04-08 at 09:15

- Commit: 43b3be02
- Files:
    - sniff/lib/src/filesystem/git/range_walker.rs
- Description: fix(sniff): preserve empty commits and use time-monotonic sort
    - Empty commits are no longer dropped during range walking
    - Commits are sorted by timestamp instead of topological order
```

File paths are rendered as clickable OSC8 hyperlinks (pointing to `file://` URIs) in terminals that support them.

## Period Argument

```
sniff repo recent-commits [PERIOD]
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
| `--action <feat\|chore\|refactor\|test\|style\|fix>` | Filter to one or more conventional commit actions |
| `--package <PKG>` | Scope to commits touching a specific package |
| `--package-area <AREA>` | Scope to commits touching a specific package area |
| `--no-error` | Exit 0 with no output when no results found |
| `--on-error <MESSAGE>` | Message to display when no results found |

## Conventional Commit Action Filtering

Use `--action` to keep only commits whose summary matches one of these conventional commit actions:

- `feat`
- `chore`
- `refactor`
- `test`
- `style`
- `fix`

The flag may be repeated. When more than one `--action` is provided, the matches are logically OR'd together.

Non-conventional commits are excluded when `--action` filtering is active.

## Package Scoping

In monorepos, `--package` and `--package-area` filter commits to only those that touched files within the specified scope:

```bash
sniff repo recent-commits 1w --package sniff-cli
sniff repo recent-commits --package-area homelab
```

Filtering works by matching each commit's changed files against the package or area path. Commits with no matching files are excluded entirely.

## Examples

```bash
sniff repo recent-commits                    # Last 3 days (default)
sniff repo recent-commits 1w                 # Last week
sniff repo recent-commits today              # Since midnight
sniff repo recent-commits yesterday          # Yesterday only
sniff repo recent-commits 2026-04-01         # Specific date
sniff repo recent-commits a1b2c3d            # From hash to HEAD
sniff repo recent-commits --action fix       # Only conventional fix commits
sniff repo recent-commits --action feat --action refactor
sniff repo recent-commits 2w --package sniff # Last 2 weeks, sniff package only
```

## JSON Output (`--json`)

```bash
sniff --json repo recent-commits 1w
```

Returns a `CommitDescSet` object:

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
        "Replaced nested if/match blocks with let-chains",
        "Simplified scope extraction logic"
      ]
    }
  ],
  "period_label": "last 1w",
  "repo_root": "/absolute/path/to/repo"
}
```

### `commits` items

| Field | Type | Notes |
|-------|------|-------|
| `hash` | string | Full commit SHA |
| `datetime` | string | ISO 8601 timestamp |
| `packages` | array\|null | Package names touched by this commit |
| `package_areas` | array\|null | Package area names touched by this commit |
| `files` | array | Repo-relative file paths changed |
| `description` | string | Commit summary line |
| `bullet_points` | array | Parsed bullet points from commit body |

### Top-level fields

| Field | Type | Notes |
|-------|------|-------|
| `period_label` | string | Human-readable period description |
| `repo_root` | string | Absolute path to the repository root |
| `packages` | array\|null | Full package list (present when package filtering is active) |

## Plain Output (`--plain`)

Adding `--plain` strips all ANSI escape codes and OSC8 hyperlinks from the text output. File paths are rendered as plain text instead of clickable links.

## No-Result Behavior

When no commits match the period (or after filtering), the default is to exit with code 1. This can be customized:

```bash
# Silent success when nothing matches
sniff repo recent-commits --no-error

# Custom message when nothing matches
sniff repo recent-commits --on-error "No commits in this period"

# Combined: message to stdout, exit 0
sniff repo recent-commits --no-error --on-error "All quiet"
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more commits found |
| `1` | No commits found (default behavior) |
| `0` | No commits found with `--no-error` |

## Related Commands

| Command | Purpose |
|---------|---------|
| [`sniff repo source-code-changes`](./repo_source-code-changes.md) | Source code files changed in a period, grouped by file |
| [`sniff repo documentation-changes`](./repo_documentation-changes.md) | Documentation files changed in a period, grouped by file |
| [`sniff repo hash`](./repo_hash.md) | Inspect a single commit by SHA |
