---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/blast_radius.rs
---

# The `sniff repo source-code-changes` Subcommand

Lists source-code changes within a time period in a **commit-centric** layout: one block per commit, with only the commit's source-code files surfaced under **Files Impacted**. Commits that did not touch any source-code files are skipped entirely. This shares the same rendering template as `recent-commits` and `documentation-changes`, so the three commands differ only in which files they include.

## What Counts as Source Code

File classification uses the sniff file-type registry. A file is considered source code if its association is one of:

- **ProgrammingLanguage** — `.rs`, `.ts`, `.py`, `.go`, `.java`, etc.
- **FrameworkFile** — framework-specific files (e.g., `vite.config.ts`)
- **Styling** — `.css`, `.scss`, `.less`, etc.
- **HTML/HTM** — accepted as source code (historical convention)

Configuration files (`.toml`, `.json`, `.yaml`), documentation (`.md`, `.txt`), and other non-source files are excluded.

## Default Behavior

When no period is specified, defaults to `3d` (last 3 days). Output is rendered as Markdown with a heading that includes the period label, followed by one block per commit:

```
### Source Code Changes (today)

- [f89f844] refactor(sniff) at 1:01pm Today: improve Option chaining and narrow cfg guards

    **Description:**

    - Use and_then() instead of map().flatten() for more idiomatic Option chaining
    - Remove unnecessary as i32 cast on header.rtm_addrs (already the correct type)
    - Narrow parse_bsd_default_route_interface cfg from multi-platform to test-only

    **Files Impacted:**

    - modified: sniff/lib/src/filesystem/mod.rs
    - modified: sniff/lib/src/network/mod.rs

- [c8df5b9] test(sniff) at 12:32pm Today: apply cargo fmt to benchmarks and tests

    **Description:**

    - Reformat import ordering and line wrapping in benchmark cases
    - Add uv_with_install_plan integration test for UvWithInstall auto-append flow
    - Standardize assertion formatting across test files

    **Files Impacted:**

    - modified: sniff/lib/tests/bench_ids_sync.rs
    - added: sniff/lib/tests/uv_with_install_plan.rs
    - modified: sniff/lib/tests/windows_app_paths_orphan.rs
```

Notes:

- **Files Impacted** lists only source-code files; any non-source files touched by the same commit are hidden from this view.
- Each file is prefixed by its change kind (`added`, `modified`, `deleted`, `renamed`, `copied`).
- File paths are rendered as clickable OSC8 hyperlinks (pointing to `file://` URIs) in terminals that support them.
- Commit timestamps are displayed in the viewer's local timezone with `Today`/`Yesterday` labels.

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
sniff repo source-code-changes --action fix       # Only conventional fix commits
sniff repo source-code-changes --action feat --action refactor
sniff repo source-code-changes 2w --package sniff # Last 2 weeks, sniff package only
```

## JSON Output (`--json`)

```bash
sniff --json repo source-code-changes 1w
```

Returns the same `CommitDescSet` object as `recent-commits` — JSON is not filtered down to source-code files, it always carries every changed file for the commit. The text-only rendering is the only view that prunes non-source-code files:

```json
{
  "commits": [
    {
      "hash": "34b6d18a...",
      "datetime": "2026-04-09T14:32:00+00:00",
      "packages": ["sniff", "sniff-cli"],
      "package_areas": ["sniff"],
      "files": [
        { "path": "sniff/lib/src/filesystem/git/recent_commits.rs", "kind": "modified" },
        { "path": "sniff/cli/src/output/recent_commits.rs",        "kind": "modified" }
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

Each `files` item is a record of `{ path, kind }` where `kind` is one of `added`, `modified`, `deleted`, `renamed`, `copied`.

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
