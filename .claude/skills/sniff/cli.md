# Sniff CLI

Use this reference for command discovery and output-mode behavior.

## Output modes

- `sniff` with no subcommand shows help.
- `sniff --json` with no subcommand emits full system information.
- Focused subcommands default to terminal text and accept `--json` where
  supported.
- `--plain` disables styled output.

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
