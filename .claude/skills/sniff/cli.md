# Sniff CLI

Use this reference for command discovery and output-mode behavior.

## Output modes

- `sniff` with no subcommand shows help.
- `sniff --json` with no subcommand emits full system information.
- Focused subcommands default to terminal text and accept `--json` where
  supported.
- `--plain` disables styled output.
- Global `-v`/`--verbose` is a repeatable counter for styled output detail
  only; raw tracing is `--debug` (or `RUST_LOG`).

### Subcommand flags that reuse a global name

Clap merges global argument values across command levels by argument id, so a
subcommand cannot independently shadow a global flag such as `-v`:

- same id with a different type (e.g. `verbose: bool`) panics on every parse;
- a different id with the same `-v`/`--verbose` spelling loses to the global;
- only the same id **and** type (`verbose: u8`, `ArgAction::Count`) works, and
  the two values unify, making the flag position-independent. `conflicts_with`
  on it is enforced only when both flags appear at the subcommand level.

`sniff/cli/src/args/recent_commits_flag_shadowing.rs` pins all three shapes.

### Commit-family commands

`repo recent-commits`, `repo source-code-changes`, and
`repo documentation-changes` share one clap shape, `args/recent_commits.rs`
(`RecentCommitsArgs::to_options`). They are presets over the library pipeline:
collect, apply `RecentCommits::projected(...)`, then emit `to_json()`,
`to_plain(...)`, or one `Prose` render of `to_prose(...)` with
`WordWrap::WrapProse(None, None)`. Do not add CLI-side filtering, styling,
URL building, or JSON surgery.

- Sibling commands **project** a selection; they do not filter collection.
  `source-code-changes` takes the last 10 commits and prunes them to
  source-code files, so it can show fewer than 10 commits. Use `--source-code`
  to filter instead.
- An empty result exits 0. `--json` prints `[]`. Text modes print nothing to
  stdout and `No commits matched.` to stderr.
- `-v` given before the subcommand plus `-c` after it resolves to compact.

## `--perf` output

`--perf` renders a hierarchical timing tree rooted at `Total` and, when the
report holds counters, a separate `Counters` tree below it. Rich terminal
commands emit it to stdout; scriptable text commands and `--json` emit it to
stderr so stdout stays machine-readable.

Three traps when asserting against it:

- Bare `sniff --perf` shows help and emits **no** performance section. Name a
  subcommand or pass `--json`.
- Rows are labelled by the **last dotted segment**, so a full key such as
  `filesystem.shared_walk.docs` never appears. Match a row by segment, and read
  its value as the cell immediately after the label — the last cell is the
  share, which folds from `—` to `-` without Unicode.
- `--plain` strips ANSI but does not force ASCII. Connector and marker glyphs
  follow the detected terminal's locale-derived Unicode capability, so a CLI
  test must never assert one.

The structured `performance` field in `--json` is unaffected by any of this.

## Common host commands

```text
sniff runtime
sniff hardware
sniff cpu
sniff audio-devices
sniff software
sniff software editors
sniff software test-runners
sniff services
sniff docs
sniff topics
sniff just
```

Program installation commands may be interactive. Do not invoke them from a
non-interactive agent session.

## Repository commands

```text
sniff repo
sniff repo name
sniff repo is-monorepo
sniff repo packages
sniff repo package-areas
sniff repo package-dependencies
sniff repo package-manager
sniff repo test-runner
sniff repo version
sniff repo git-status
sniff repo worktree
sniff repo worktrees
sniff repo branches
sniff repo remote origin
sniff repo pr
sniff repo recent-commits 1w
sniff repo source-code-changes today
sniff blast-radius
```

Use `--package`, `--package-area`, or `--all` only where the command exposes the
scope. Prefer the narrowest scope that answers the question.

## Aggregate JSON

Bare `sniff repo --json` returns the consolidated `SniffRepo` projection. It
contains top-level identity, a nested `context`, worktrees and branches, and
four change buckets. It excludes network-primary commands and does not fetch.

Focused commands such as `repo git-status --json` and `repo structure --json`
retain their richer command-specific shapes. `repo recent-commits --json` is a
bare array (`--perf` wraps it as `{ data, performance }`).
The aggregate's `recent_commits`, `source_code_changes`, and
`documentation_changes` keys are exactly the three commit-family commands'
default `--json` arrays; `cli.rs::test_repo_aggregate_commit_families_match_the_focused_commands`
pins that equality.
