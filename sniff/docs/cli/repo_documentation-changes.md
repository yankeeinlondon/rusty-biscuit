---
blast_radius:
  - sniff/cli/src/args/recent_commits.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits/render.rs
  - sniff/lib/src/filesystem/git/recent_commits/collect.rs
  - sniff/lib/src/filesystem/path_kind.rs
---

# The `sniff repo documentation-changes` Subcommand

Lists recent documentation changes in a **commit-centric** layout: one block per commit, with only the commit's documentation files surfaced under **Files Impacted**. Commits that did not touch any documentation files are skipped entirely.

This is a preset over the same pipeline as [`sniff repo recent-commits`](./repo_recent-commits.md). It accepts the same period, filters, and verbosity flags, selects the same commits, and then _projects_ them onto documentation. The three commit-family commands differ only in which files they list.

## What Counts as Documentation

File classification uses the library's canonical classifier (`sniff::filesystem::path_kind::classify_path`), which places every file in exactly one category. A file is documentation when it is not a CI/CD definition and its file-type registry association is **Documentation**:

- bare `README`, `CHANGELOG`, and `CONTRIBUTING` files (with or without `.md`)
- the extensions `.md`, `.mdx`, `.rst`, `.txt`, `.adoc`, `.org`, and `.tex`

A registry file name keeps its own category despite its extension, so `requirements.txt` is configuration. Source code, web assets, images, configuration, CI/CD definitions, and other files are excluded.

> **Behavior change:** `.html` and `.htm` files used to count as documentation. They are now web assets and no longer appear here.

## Selection, Then Projection

The command first selects commits exactly as `recent-commits` would, then prunes each commit's files to documentation and drops commits left with none:

- With no period, the selection is the **last 10 commits**, so the output can show fewer than 10 commits (or none) when some of them changed no documentation.
- A moved file is kept when either its new path or its original path is documentation.
- To _select_ the last 10 commits that changed documentation, add `--documentation`: `sniff repo documentation-changes --documentation`.

## Default Behavior

```
Documentation Changes

- [e0a1034] docs(sniff) at 10:12am Today: document commit-centric changes layout
  Files Impacted:
  - modified: sniff/docs/cli/repo_source-code-changes.md
  - modified: sniff/docs/cli/repo_documentation-changes.md

- [a1b5d77] docs(sniff) at 12:32pm Today: add program-installation documentation placeholder
  Files Impacted:
  - added: sniff/docs/topics/program-installation.md
```

Notes:

- The `Documentation Changes` heading appears only when at least one commit remains.
- **Files Impacted** lists only documentation files; any other files touched by the same commit are hidden from this view.
- Headers, links, local-time labels, wrapping, and styling are the same as in [`sniff repo recent-commits`](./repo_recent-commits.md#default-behavior). The styling is defined in the [Recent Commits topic](../topics/repo/recent-commits.md#styling).

## Period Argument and Flags

```
sniff repo documentation-changes [PERIOD] [OPTIONS]
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
sniff repo documentation-changes                    # Documentation files in the last 10 commits
sniff repo documentation-changes --documentation    # The last 10 commits that changed documentation
sniff repo documentation-changes 1w                 # Last week
sniff repo documentation-changes today              # Since local midnight
sniff repo documentation-changes yesterday          # Yesterday only
sniff repo documentation-changes 2026-04-01         # That local day only
sniff repo documentation-changes --operation docs   # Only conventional docs commits
sniff repo documentation-changes 2w --package sniff # Last 2 weeks, sniff package only
```

## JSON Output (`--json`)

```bash
sniff --json repo documentation-changes 1w
```

Returns the same **bare array** as `recent-commits --json`, projected so it matches the text view: only commits with at least one documentation file are included, and each kept commit's `files` array holds only documentation files. There is no `filter` field or envelope.

Commit-level fields still describe the **whole commit**: `file_types`, `packages`, `package_areas`, `remote`, and `commit_url` are not recomputed from the pruned files. A kept commit can therefore report `"source_code": true` in `file_types` even though no source file is listed.

See the [Recent Commits Schema](../topics/repo/recent-commits-schema.md) for every field. With `--perf`, the array is wrapped as `{ "data": [ ... ], "performance": { ... } }`.

## Plain Output (`--plain`)

`--plain` prints the same report as bare text, with no escape codes, hyperlinks, or Markdown markers. The heading prints as a bare `Documentation Changes` line.

## Empty Results and Exit Codes

When no selected commit touched documentation, the command still succeeds: `--json` prints `[]`, and text modes print nothing to stdout and `No commits matched.` to stderr. The exit code is `0`.

Invalid periods, unknown branches, unreachable hashes, unknown packages or areas, and package filters outside a monorepo are errors: the message goes to stderr and the exit code is non-zero.

## Related Commands

| Command | Purpose |
|---------|---------|
| [`sniff repo recent-commits`](./repo_recent-commits.md) | The same commits with every changed file |
| [`sniff repo source-code-changes`](./repo_source-code-changes.md) | The same commits, listing only their source-code files |
| [`sniff repo dirty-files`](./repo_dirty-files.md) | All uncommitted file changes |
| [`sniff blast-radius`](./repo_blast-radius.md) | Docs whose blast radius intersects changed code |
