# `wt list`

Lists all git worktrees along with their status. This is the default command -- running `wt` with no subcommand is equivalent to `wt list`.

The output is, in order: a caption, the worktree table, a two-line legend, an optional PR age line, the git graph (image-capable terminals only), and the verbose section (`-v` only). Everything is written to stderr.

## Output

```text
  main  is 7 commits behind local tracking ref  origin/main .  origin/main  matched the remote when checked 2 min ago.

┌───────────────────┬──────────────────────────────────┬──────────────────────┬───────────────────────────┐
│ Worktree          │ Branch                           │ ->  origin/main      │ -> parent                 │
├───────────────────┼──────────────────────────────────┼──────────────────────┼───────────────────────────┤
│ ○ base repo       │  main                            │ —                    │ —                         │
│ ● fix-wt-ux       │ ├─ fix/wt-ux                     │ clean +2 -1  PR #99  │ —                         │
│ ● feat-theme      │ ├─ feat/theme                    │ clean +3             │ —                         │
│ ○ feat-dark-fixes │ │  └─ feat/dark-fixes            │ clean +2 -1          │ conflicts +1 -3  PR #104  │
│ ○ old-cleanup     │ └─ chore/old-cleanup             │ clean -4             │ —                         │
│ ○ release-prep    │ release/prep  PR #7 → release/2  │ clean                │ —                         │
│                   │ experiments (deleted)            │                      │                           │
│ ● spike-parser    │ └┄ spike/parser                  │ conflicts +1 -3      │ parent deleted            │
│ ● bisect          │ detached @ a1b2c3d               │ —                    │ —                         │
└───────────────────┴──────────────────────────────────┴──────────────────────┴───────────────────────────┘

 Worktree   ○ clean    ● uncommitted files    ● uncommitted source files
 Branch     └─ merges cleanly into parent    └─ conflicts with parent    └┄ parent deleted
```

### The default-branch target

Ahead/behind and merge comparisons use one **target**: whichever of the local default branch and `origin/<default>` contains the other, or `origin/<default>` when the two have diverged. Without a remote-tracking ref the local branch is the target. `wt` never fetches, so `origin/<default>` is only as current as your last fetch.

### Caption

The caption describes the local default branch, its local tracking ref `origin/<default>`, and the last live answer from `origin`. It needs an `origin` remote: without one, leftover `origin/*` refs produce no caption.

When both the local default branch and `origin/<default>` exist, it first compares them: `is N commits behind`, `is N commits ahead of`, `is in sync with`, or `has diverged from`, each followed by `local tracking ref origin/main` (`main has diverged from local tracking ref origin/main: 3 commits ahead, 7 commits behind.`). Only the counts are colored. These counts come from your last fetch, never from the remote.

Then it says what `origin` answered the last time a background check asked it (`git ls-remote origin refs/heads/<default>`):

| Last answer | Caption text |
|---|---|
| Same commit as `origin/main` | `origin/main matched the remote when checked 2 min ago.` |
| Another commit | `origin/main differs from the remote head observed 2 min ago; run git fetch origin to update local tracking refs.` |
| Branch absent on `origin` | `main was absent on origin when checked 2 min ago.` |
| No answer yet, or one for another `origin` or default branch | `Remote state has not been verified.` |
| An answer, but no `origin/main` locally | `No local tracking ref origin/main; the remote branch was present (or absent) when checked 2 min ago.` (`…; remote state has not been verified.` without an answer) |

The age uses the PR age units: `less than 1 min ago`, then minutes, hours below two days, then days. The observation is dim. An old answer is still shown, always with its age. "Differs" only means the two commits differ; it does not say which is newer, since a fetch can be newer than the check. Without a local default branch, only the answer is shown.

`wt list` never asks `origin` itself. When the stored answer is missing or at least 60 seconds old, it starts the same detached background `wt` process that refreshes PRs (see below), which asks `origin` with a 10 second limit and stores the answer for the next run. A failed or timed-out check leaves the previous answer as it was.

### Columns

- **Worktree** -- a status dot and the directory name; the main checkout is `base repo`. The current worktree's name is bold and its whole row is highlighted.
    - `○` (dim): no uncommitted files
    - `●` (yellow): uncommitted files, none of them source code
    - `●` (red, the same red as `conflicts`): at least one uncommitted source file
- **Branch** -- a tree built from the fork records `wt create` keeps (see `--from` in the [README](../../README.md)):
    - the default branch comes first, as a badge; branches forked from another branch nest under it
    - the connector (`├─`, `└─`) is gray when the branch merges cleanly into its parent and red when it conflicts
    - a parent that has no worktree of its own still gets a row, with empty status cells
    - a deleted parent is shown dim, struck through, and marked `(deleted)`; its children hang from it with dotted connectors (`├┄`, `└┄`)
    - branches without a fork record sit at the root, in worktree order; siblings are ordered by creation time, then name
    - a detached worktree shows `detached @ <sha>` and comes last
- **`-> {target}`** -- the header names the target as a remote (`origin/main`) or local (`main`) badge. The cell compares the branch with it:
    - `clean` (dim italic): it would merge without conflicts, including when it has nothing to merge
    - `conflicts` (red): it would not
    - from 100 terminal columns, `clean` and `conflicts` are followed by `+N` (dim green, commits the branch has that the target lacks) and `-N` (dim red, commits the target has that the branch lacks); a zero side is left out, so equal tips read `clean` alone. Below 100 columns only the state word shows. The width is the terminal's; `-w` / `--width` sizes only the graph
    - `—`: the default branch's own row, and detached worktrees
    - `?`: git could not answer; `wt` never shows a guessed result
- **`-> parent`** -- the same comparison, counts included, against the recorded fork parent's local branch (never its `origin/*` copy). `—` when there is no non-default parent; `parent deleted` when the recorded parent is gone.

Comparison results are cached by the pair of commit SHAs, so a warm `wt list` runs no `rev-list` or `merge-base`; see [performance-testing.md](../performance-testing.md). Uncommitted-file status is always checked live.

### PR badges

Open pull requests on `origin` whose source is this repository show as a green `PR #n` badge:

- in the `-> {target}` cell when the PR targets the default branch, after any counts
- in the `-> parent` cell when it targets the branch's fork parent
- beside the branch name, as `PR #n → <target>`, otherwise

A PR from a fork with a same-named branch is never shown. In terminals that support OSC 8 hyperlinks the badge links to the PR; elsewhere it shows the number only, with no visible URL, so the table always fits.

A successful answer is stored with the `origin` it came from, and `wt list` shows a stored answer for the current `origin` at once, whatever its age. Once it is 60 seconds old a dim `PRs as of N min ago` line follows the legend, and `wt list` starts a detached `wt` process that asks again and replaces the stored answer for the next run; `wt list` exits without waiting for it. That one process also checks the caption's remote answer, and each check runs and stores its answer independently of the other. At most one such refresh makes a request at a time, and a refresh that fails leaves the stored answer as it was.

With no stored answer for the current `origin` (the first run, a cleared cache, or a changed or removed `origin`), the request runs in parallel with the git work and is given 300 ms. An answer stored for a different `origin` is never shown. A failure is never stored, and with no network and no usable stored answer the table shows no badges.

### Git Graph

When stderr is a terminal and `TERM_PROGRAM` (or `KITTY_WINDOW_ID`) names an image-capable terminal (Kitty, iTerm2, Ghostty, WezTerm, Warp, Konsole), a branch graph is drawn below the table. It is not drawn for a detached current worktree.

- **A feature branch is checked out**: the current branch forked from the default branch (or from its recorded parent branch, which then gets a line of its own), with two shared commits before the fork.
- **The default branch is checked out**: the default branch's 10 newest commits, and a line for every worktree branch. A branch that is already in the default branch is a tag rather than a line.

Each line shows its 5 newest commits; older ones fold into a `+N` square. Open PRs appear as `PR #n → target` tags. When `origin/<default>` has diverged from the local default branch it gets a line of its own.

There is no minimum terminal width: the graph is sized from its natural width and the terminal's cell size, trims commits into `+N` squares to fit, and shrinks only when even the trimmed graph is too wide. In the base view a graph taller than half the terminal keeps the most recently active lines and notes how many were left out. See [git-graph.md](../git-graph.md) for the design.

`-w` / `--width` sets the graph's width directly (`70`, `70ch`, or `50%` of the terminal); the graph is then never trimmed to fit.

### Verbose Mode

With `-v` / `--verbose`, and whenever the current branch is not the default branch (including when it is checked out in the base repo), two sections follow the graph:

- **Default branch section** -- the commit the branch forked from, formatted with SHA, conventional commit type/scope, timestamp, and any refs
- **Feature branch section** -- every commit on the branch that neither the local default branch nor `origin/<default>` has, oldest first, using the same format

Commits are formatted as conventional commits when possible (e.g. `feat(scope): description`), with fallback to a truncated first line for non-conventional messages.

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--width <WIDTH>` | `-w` | Set the graph width (e.g. `70`, `70ch`, `50%`) and turn off trimming |
| `--verbose` | `-v` | Show the commit history of the current branch |
| `--perf` | | Print a per-stage timing report to stderr |

## Examples

```bash
wt              # List worktrees (default command)
wt list         # Explicit list
wt -v           # List with verbose commit details
wt -w 100       # List with graph forced to 100 characters wide
wt -w 50%       # List with graph at 50% of terminal width
wt -v -w 120    # Verbose output with wider graph
```
