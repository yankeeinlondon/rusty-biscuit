# `wt list`

Lists all git worktrees along with their status. This is the default command -- running `wt` with no subcommand is equivalent to `wt list`.

## Output

### Status Line

Each worktree is displayed as a bulleted list with the following information:

- **Branch name** -- bold for the current worktree, dimmed for others; detached HEADs show as `(detached)`
- **Merge status** -- `clean` (mergeable without conflicts) or `conflict`; green/red for the current worktree, dimmed for others
- **Ahead/behind counts** -- `+N` commits ahead (green) and `-N` commits behind (yellow) relative to the default branch; only shown when non-zero

### Git Graph

When the terminal supports inline images (Kitty, iTerm2, Ghostty, WezTerm, Warp, Konsole), a Mermaid `gitGraph` diagram is rendered below the status list.

The graph varies based on which worktree is current:

- **On a feature branch**: shows the 2-branch view -- up to 2 context commits before the merge-base, up to 5 commits on the feature branch since divergence, and up to 5 commits on the default branch since divergence.
- **On the base/main branch**: shows up to 10 recent commits on main with all active worktree branches forking off at their divergence points, each showing up to 10 commits.

The graph is suppressed when the terminal width is less than **80 characters**.

#### Graph Width Sizing

When no explicit `-w` flag is provided, the graph width is chosen automatically based on the number of commits in the generated diagram:

| Commits | Default Width |
|---------|---------------|
| 1--4    | 40 characters |
| 5--8    | 80 characters |
| 9-11    | 100 characters (if terminal > 100 cols), otherwise 100% of terminal width |
| 12-15   | 120 characters (if terminal > 120 cols), otherwise 100% of terminal width |
| 16+     | 160 characters (if terminal >= 160 cols), otherwise 100% of terminal width |

The `-w` / `--width` flag overrides this automatic sizing. It accepts character values (`70`, `70ch`) or percentages (`50%`).

### Verbose Mode

With `-v` / `--verbose`, additional detail is shown below the graph (only when on a feature branch):

- **Default branch section** -- the merge-base commit formatted with SHA, conventional commit type/scope, timestamp, and any refs
- **Feature branch section** -- all commits since the branch point, in oldest-first order, using the same format

Commits are formatted as conventional commits when possible (e.g. `feat(scope): description`), with fallback to a truncated first line for non-conventional messages.

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--width <WIDTH>` | `-w` | Override graph width (e.g. `70`, `70ch`, `50%`) |
| `--verbose` | `-v` | Show detailed commit history for the current worktree |

## Examples

```bash
wt              # List worktrees (default command)
wt list         # Explicit list
wt -v           # List with verbose commit details
wt -w 100       # List with graph forced to 100 characters wide
wt -w 50%       # List with graph at 50% of terminal width
wt -v -w 120    # Verbose output with wider graph
```
