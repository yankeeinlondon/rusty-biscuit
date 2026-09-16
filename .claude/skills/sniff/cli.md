# Sniff CLI

Use this reference for command discovery and output-mode behavior.

## Output modes

- `sniff` with no subcommand shows help.
- `sniff --json` with no subcommand emits full system information.
- Focused subcommands default to terminal text and accept `--json` where
  supported.
- `--plain` disables styled output.

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

Focused commands such as `repo git-status --json`, `repo structure --json`, and
`repo recent-commits --json` retain their richer command-specific shapes.
