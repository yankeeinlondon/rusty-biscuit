---
blast_radius:
  - sniff/cli/src/args/recent_commits.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/lib/src/filesystem/git/recent_commits/options.rs
  - sniff/lib/src/filesystem/git/recent_commits/collect.rs
  - sniff/lib/src/filesystem/git/recent_commits/payload.rs
  - sniff/lib/src/filesystem/git/recent_commits/render.rs
  - sniff/lib/src/filesystem/git/commit_links.rs
---

# The `sniff repo recent-commits` Subcommand

Shows recent commits, one block per commit. Each block has a one-line header (short hash, conventional-commit prefix, local time with a relative-day label, and the commit's heading) followed by the files the commit changed and how it changed them (`added`, `modified`, `deleted`, `moved`). Flags select which commits appear, filter them, and choose how much of each commit to show.

The command is a thin layer over the `sniff` library: selection, filtering, remote linking, and every byte of the report come from `RecentCommits` (see the [Recent Commits topic](../topics/repo/recent-commits.md)). [`source-code-changes`](./repo_source-code-changes.md) and [`documentation-changes`](./repo_documentation-changes.md) share the same arguments and differ only in which files they list.

## Default Behavior

When no period is specified, the command shows the **last 10 commits** reachable from `HEAD`, at normal verbosity:

```
- [f89f844] refactor(sniff) at 1:01pm Today: improve Option chaining and narrow cfg guards
  Files Impacted:
  - modified: sniff/lib/src/filesystem/mod.rs
  - modified: sniff/lib/src/network/mod.rs

- [c8df5b9] test(sniff) at 12:32pm Today: apply cargo fmt to benchmarks and tests
  Files Impacted:
  - modified: sniff/lib/tests/bench_ids_sync.rs
  - added: sniff/lib/tests/uv_with_install_plan.rs

- [14bf472] at 8:27pm Yesterday: Merge branch 'main' into feat/better-sniff
  Files Impacted:
  - moved: sniff/docs/new-name.md (from sniff/docs/old-name.md)
```

Notes:

- Times and the `Today`/`Yesterday` labels use the host's local UTC offset. Commits older than yesterday show an absolute date (`9:30am 2026-04-01`).
- The short hash links to the commit's page on the remote host only when a locally recorded remote-tracking ref contains the commit and the provider has a browser URL. Nothing is fetched to decide this.
- File paths link to `file://` URIs, except deleted files. Terminals without OSC8 support show links in `[text](url)` form.
- Non-conventional commits (such as merges) have no operation or scope in their header.
- A commit that changed no files (such as a no-change merge) shows `Files Impacted: none`.
- Long lines wrap at word boundaries to the terminal width.

The styling (bold hash, blue operation, dimmed scope, italic `at`, bold time and labels) is defined by the library. See [Styling](../topics/repo/recent-commits.md#styling) in the topic doc.

## Period Argument

```
sniff repo recent-commits [PERIOD]
```

The optional `PERIOD` argument accepts several formats:

| Format | Example | Meaning |
|--------|---------|---------|
| Count | `10`, `25` | The newest N matching commits (bare positive integer; default `10`) |
| Duration | `3d`, `1w`, `2mo`, `6h` | Commits within that duration before now |
| Named | `today` | Since local midnight |
| Named | `yesterday` | Local midnight-to-midnight yesterday, that day only |
| Date | `2026-04-01` | That single local calendar day (YYYY-MM-DD) |
| Hash | `a1b2c3d` | From the starting tip back to and including that commit (at least 7 hex characters) |

Detection is ordered: `today`/`yesterday` → ISO date → bare number (count) → duration → hash. An all-digit argument is always treated as a count, so a SHA that happens to be entirely numeric must be disambiguated by supplying more of the hash (so that it includes a non-digit hex character). A zero count, or a value matching no format, is an error.

Calendar periods use the host's local UTC offset, so a commit selected as "today" is also labeled `Today`. Durations, counts, and hashes do not depend on the offset.

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
| `[PERIOD]` | Which commits (default: the last 10) |
| `--operation <OPERATION>` | Keep conventional commits with this operation (any word); repeat to match any of several |
| `--scope <SCOPE>` | Keep conventional commits with this scope (case-insensitive) |
| `--author <NAME\|EMAIL>` | Keep commits whose author name or email contains this text (case-insensitive) |
| `--branch <BRANCH>` | Walk history from this branch instead of `HEAD` (local first, then remote-tracking) |
| `--package <PKG>` | Keep commits touching this monorepo package |
| `--package-area <AREA>` | Keep commits touching this monorepo package area |
| `--source-code` | Keep commits that change source code |
| `--web` | Keep commits that change web assets (HTML, CSS, fonts) |
| `--images` | Keep commits that change images |
| `--documentation` | Keep commits that change documentation |
| `--configuration` | Keep commits that change configuration |
| `--cicd` | Keep commits that change CI/CD definitions |
| `--show-author` | Show each commit's author in its header line |
| `-v`, `--verbose` | Include each commit's description and bullet points |
| `-c`, `--compact` | Show only each commit's header line |

The global `--json`, `--plain`, `--perf`, `--debug`, and `-b/--base <DIR>` flags also apply.

> **Removed:** `--action` is replaced by `--operation`. `--no-error` and `--on-error` no longer exist on this command because an empty result is a success (see [Empty Results](#empty-results)).

## Filtering

Every filter is applied while history is walked. Filters of different kinds combine with AND, so `--operation fix --package sniff` keeps only `fix` commits that touched the `sniff` package. With a count period, the walk continues until that many commits _match_ (or history ends), so `sniff repo recent-commits 5 --operation fix` shows the last five fixes, not the fixes among the last five commits.

### Operation and Scope

`--operation` accepts any [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) operation word, not a fixed list, and matches it case-insensitively. Shell completion suggests common words such as `feat` and `fix`, but `--operation planning` works too. Repeated `--operation` values are OR'd together. Non-conventional commits are excluded when any operation filter is set.

`--scope` keeps conventional commits whose scope equals the given value, ignoring case.

### Author

`--author` is a case-insensitive substring match against the author's name **or** email, so `--author ada` and `--author example.com` both work. It only filters. To display authors, use `--show-author`, which adds `by {name}` before `at` in each header.

### Branch

`--branch` sets where the history walk starts; it is not a filter on which branch a commit belongs to. A local branch of that name wins; otherwise a remote-tracking branch (`origin/feature`, or `feature` on a configured remote) is used. An unknown branch is an error, and nothing is fetched.

```bash
sniff repo recent-commits --branch feature      # the last 10 commits on feature's tip
sniff repo recent-commits 1w --branch origin/main
```

### Package Scoping

In monorepos, `--package` and `--package-area` keep commits that changed at least one file owned by the given package or by a package in the given area:

```bash
sniff repo recent-commits 1w --package sniff-cli
sniff repo recent-commits --package-area homelab
```

- Ownership comes from the repository's package catalog (the same one `sniff repo packages` uses), and a file belongs only to its deepest owning package.
- A file inside a package area but outside every package (for example `sniff/README.md`) belongs to no package, so it matches neither filter.
- Unknown package or area names are errors that list the valid names. Using either flag outside a monorepo is also an error.

### File Categories

The file-category flags keep commits that changed at least one file of that category. Listing several requires **every** listed category (`--source-code --documentation` keeps commits that changed both). Every file belongs to exactly one category; see [File Categories](../topics/repo/recent-commits.md#file-categories) for the precedence. For example, `.html` and CSS files are web assets, and workflow files under `.github/workflows/` are CI/CD, not configuration.

These flags select commits and leave each commit's full file list intact. To show only one category's files, use [`source-code-changes`](./repo_source-code-changes.md) or [`documentation-changes`](./repo_documentation-changes.md).

## Verbosity

| Flag | Output |
|------|--------|
| `-c`, `--compact` | Header lines only, with no blank lines between commits |
| _(none)_ | Header plus `Files Impacted:` list |
| `-v`, `--verbose` | Header, description, `Details:` bullet points, then `Files Impacted:` |

`-v` and `-c` together at the subcommand level are a usage error. The subcommand's `-v` shares its identity with the global `-v`, so the position does not matter: `sniff -v repo recent-commits` also selects the verbose report, and repeating `-v` does not change anything further. When `-v` comes before the subcommand and `-c` after it, compact wins.

## Examples

```bash
sniff repo recent-commits                    # The last 10 commits (default)
sniff repo recent-commits 25                 # The last 25 commits
sniff repo recent-commits 1w                 # Last week
sniff repo recent-commits today              # Since local midnight
sniff repo recent-commits yesterday          # Yesterday only
sniff repo recent-commits 2026-04-01         # That local day only
sniff repo recent-commits a1b2c3d            # From HEAD back to a1b2c3d
sniff repo recent-commits --operation fix    # The last 10 conventional fix commits
sniff repo recent-commits --operation feat --operation refactor
sniff repo recent-commits 2w --package sniff # Last 2 weeks, sniff package only
sniff repo recent-commits --author ada --show-author
sniff repo recent-commits --cicd -c          # Recent CI/CD changes, headers only
sniff repo recent-commits -v 3               # The last 3 commits with full commentary
```

## JSON Output (`--json`)

```bash
sniff --json repo recent-commits 1w
```

Returns a **bare array** of commit objects, newest first:

```json
[
  {
    "author": { "email": "ada@example.com", "name": "Ada Lovelace" },
    "bullet_points": ["Replace nested if/match blocks with let-chains"],
    "datetime": "2026-09-16T14:32:00+00:00",
    "description": "",
    "file_types": {
      "cicd": false,
      "configuration": false,
      "documentation": false,
      "images": false,
      "source_code": true,
      "web_assets": false
    },
    "files": [
      {
        "added": 12,
        "kind": "modified",
        "path": "sniff/lib/src/filesystem/git/recent_commits/collect.rs",
        "removed": 4
      }
    ],
    "hash": "34b6d18a0c1e5f4b2d9a7e6c3b8f1a0d2e4c6b8a",
    "heading": "use let-chains in commit collection",
    "operation": "refactor",
    "package_areas": ["sniff"],
    "packages": ["sniff"],
    "remote": false,
    "scope": "sniff"
  }
]
```

The complete field reference, including which keys are omitted and when `remote` is `null`, is in the [Recent Commits Schema](../topics/repo/recent-commits-schema.md). In brief:

- `datetime` is always UTC (`+00:00`).
- `packages` and `package_areas` are present (possibly empty) only in a monorepo.
- `remote` is `true`, `false`, or `null` (undetermined), and `commit_url` appears only when a containing remote has a browser URL.
- `--verbose`, `--compact`, and `--show-author` never change the JSON.

With `--perf`, the array is wrapped so stdout stays valid JSON: `{ "data": [ ... ], "performance": { ... } }`.

> **Breaking change:** the previous `CommitDescSet` object (`commits`, `period_label`, `repo_root`) is gone, the file kinds `renamed` and `copied` are now `moved` (with `original_path`), and commits gained `author`, `heading`, `operation`, `scope`, `file_types`, `remote`, and `commit_url`.

## Plain Output (`--plain`)

`--plain` prints the same report as bare text: no escape codes, no hyperlinks, and no Markdown markers. Labels print as `Files Impacted:` and `Details:`, and file paths are plain text.

> **Behavior change:** earlier releases kept Markdown bold markers such as `**Description:**` in plain output. Plain output now strips all markup.

The plain output is exactly `RecentCommits::to_plain`, which is the library's
per-commit `RecentCommits::plain_blocks` joined by one blank line. A consumer
that wants commits as separate values — Darkmatter's `ctx.recent_commits` —
takes the blocks rather than splitting this output on blank lines, which the
verbose layout also uses inside a block.

## No-Result Behavior

## Empty Results

A valid query that matches no commits is a success:

| Mode | stdout | stderr | Exit |
|------|--------|--------|------|
| `--json` | `[]` | nothing | `0` |
| terminal or `--plain` | nothing | `No commits matched.` | `0` |

Because stdout stays empty, `$(sniff repo recent-commits --operation release)` is an empty string when nothing matches.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success, including a query that matched no commits |
| non-zero | An invalid period, unknown branch, unreachable hash, unknown package or area, package filter outside a monorepo, not a Git repository, or unreadable history. The error is printed to stderr and stdout stays empty. |

## Related Commands

| Command | Purpose |
|---------|---------|
| [`sniff repo source-code-changes`](./repo_source-code-changes.md) | The same commits, listing only their source-code files |
| [`sniff repo documentation-changes`](./repo_documentation-changes.md) | The same commits, listing only their documentation files |
| [`sniff repo hash`](./repo_hash.md) | Inspect a single commit by SHA |
| [`sniff repo`](./repo.md) | The `sniff repo --json` aggregate embeds this command's default JSON |
