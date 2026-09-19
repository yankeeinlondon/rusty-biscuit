---
blast_radius:
  - sniff/cli/src/args/recent_commits.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits/render.rs
  - sniff/lib/src/filesystem/git/recent_commits/collect.rs
  - sniff/lib/src/filesystem/path_kind.rs
---

# The `sniff repo source-code-changes` Subcommand

Lists recent source-code changes in a **commit-centric** layout: one block per commit, with only the commit's source-code files surfaced under **Files Impacted**. Commits that did not touch any source-code files are skipped entirely.

This is a preset over the same pipeline as [`sniff repo recent-commits`](./repo_recent-commits.md). It accepts the same period, filters, and verbosity flags, selects the same commits, and then _projects_ them onto source code. The three commit-family commands differ only in which files they list.

## What Counts as Source Code

File classification uses the library's canonical classifier (`sniff::filesystem::path_kind::classify_path`), which places every file in exactly one category. A file is source code when it is not a CI/CD definition (for example under `.github/workflows/`) and its file-type registry association is one of:

- **ProgrammingLanguage** — `.rs`, `.ts`, `.py`, `.go`, `.java`, etc.
- **FrameworkFile** — framework files such as `.vue`, `.svelte`, `.astro`, and Angular `.component.html` templates

Web assets (`.html`, `.htm`, `.css`, `.scss`, `.less`, fonts), images, configuration files (`.toml`, `.json`, `.yaml`), documentation (`.md`, `.txt`), CI/CD definitions, and other files are excluded.

> **Behavior change:** CSS and plain `.html` files used to count as source code. They are now web assets and no longer appear here.

## Selection, Then Projection

The command first selects commits exactly as `recent-commits` would, then prunes each commit's files to source code and drops commits left with none:

- With no period, the selection is the **last 10 commits**, so the output can show fewer than 10 commits (or none) when some of them changed no source code.
- A moved file is kept when either its new path or its original path is source code.
- To _select_ the last 10 commits that changed source code, add `--source-code`: `sniff repo source-code-changes --source-code`.

## Default Behavior

```
Source Code Changes

- [f89f844] refactor(sniff) at 1:01pm Today: improve Option chaining and narrow cfg guards
  Files Impacted:
  - modified: sniff/lib/src/filesystem/mod.rs
  - modified: sniff/lib/src/network/mod.rs

- [c8df5b9] test(sniff) at 12:32pm Today: apply cargo fmt to benchmarks and tests
  Files Impacted:
  - modified: sniff/lib/tests/bench_ids_sync.rs
  - added: sniff/lib/tests/uv_with_install_plan.rs
```

Notes:

- The `Source Code Changes` heading appears only when at least one commit remains.
- **Files Impacted** lists only source-code files; any other files touched by the same commit are hidden from this view.
- Headers, links, local-time labels, wrapping, and styling are the same as in [`sniff repo recent-commits`](./repo_recent-commits.md#default-behavior). The styling is defined in the [Recent Commits topic](../topics/repo/recent-commits.md#styling).

## Period Argument and Flags

```
sniff repo source-code-changes [PERIOD] [OPTIONS]
```

`PERIOD` accepts a count (`10`), duration (`3d`, `1w`), date (`YYYY-MM-DD`), `today`, `yesterday`, or a hash; the default is the last 10 commits. See [Period Argument](./repo_recent-commits.md#period-argument) for the exact semantics and duration units.

The flags are identical to `recent-commits`:

| Flag | Description |
|------|-------------|
| `--operation <OPERATION>` | Keep conventional commits with this operation (any word); repeat to match any of several |
| `--scope <SCOPE>` | Keep conventional commits with this scope |
| `--author <NAME\|EMAIL>` | Keep commits whose author name or email contains this text |
| `--branch <BRANCH>` | Walk history from this branch instead of `HEAD` (local first, then remote-tracking) |
| `--package <PKG>` / `--package-area <AREA>` | Keep commits touching this monorepo package or package area |
| `--source-code`, `--web`, `--images`, `--documentation`, `--configuration`, `--cicd` | Keep commits that change every listed file category |
| `--show-author` | Show each commit's author in its header line |
| `-v`, `--verbose` / `-c`, `--compact` | Add descriptions and bullet points / show header lines only |

See [Filtering](./repo_recent-commits.md#filtering) and [Verbosity](./repo_recent-commits.md#verbosity) for details. `--action`, `--no-error`, and `--on-error` have been removed.

## Examples

```bash
sniff repo source-code-changes                    # Source files in the last 10 commits
sniff repo source-code-changes --source-code      # The last 10 commits that changed source code
sniff repo source-code-changes 1w                 # Last week
sniff repo source-code-changes today              # Since local midnight
sniff repo source-code-changes yesterday          # Yesterday only
sniff repo source-code-changes a1b2c3d            # From HEAD back to a1b2c3d
sniff repo source-code-changes --operation fix    # Only conventional fix commits
sniff repo source-code-changes 2w --package sniff # Last 2 weeks, sniff package only
```

## JSON Output (`--json`)

```bash
sniff --json repo source-code-changes 1w
```

Returns the same **bare array** as `recent-commits --json`, projected so it matches the text view: only commits with at least one source-code file are included, and each kept commit's `files` array holds only source-code files. There is no `filter` field or envelope.

Commit-level fields still describe the **whole commit**: `file_types`, `packages`, `package_areas`, `remote`, and `commit_url` are not recomputed from the pruned files. A kept commit can therefore report `"documentation": true` in `file_types` even though no documentation file is listed.

See the [Recent Commits Schema](../topics/repo/recent-commits-schema.md) for every field. With `--perf`, the array is wrapped as `{ "data": [ ... ], "performance": { ... } }`.

## Plain Output (`--plain`)

`--plain` prints the same report as bare text, with no escape codes, hyperlinks, or Markdown markers. The heading prints as a bare `Source Code Changes` line.

## Empty Results and Exit Codes

When no selected commit touched source code, the command still succeeds: `--json` prints `[]`, and text modes print nothing to stdout and `No commits matched.` to stderr. The exit code is `0`.

Invalid periods, unknown branches, unreachable hashes, unknown packages or areas, and package filters outside a monorepo are errors: the message goes to stderr and the exit code is non-zero.

## Related Commands

| Command | Purpose |
|---------|---------|
| [`sniff repo recent-commits`](./repo_recent-commits.md) | The same commits with every changed file |
| [`sniff repo documentation-changes`](./repo_documentation-changes.md) | The same commits, listing only their documentation files |
| [`sniff repo dirty-source-code`](./repo_dirty-source-code.md) | Uncommitted source code changes |
| [`sniff repo staged-source-code`](./repo_staged-source-code.md) | Staged source code changes |
