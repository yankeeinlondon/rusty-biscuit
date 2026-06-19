# Specification: `sniff repo git-status` Performance & UI Touch-up

This specification outlines the performance optimizations and user interface (UI) enhancements for the `sniff repo git-status` subcommand.

## Problem Statement
The `sniff repo git-status` command currently computes the ahead/behind status of *all* linked git worktrees by default. Walking the commit histories for all worktrees is an extremely expensive operation, resulting in high latency (>1 second) on large repositories.
Additionally, the section layout lacks sufficient vertical spacing, making it visually cramped.

## Key Changes & Requirements

### 1. Worktrees Section Layout and Summary Model

To eliminate unnecessary commit walks and clean up the UI, we simplify the reporting of linked worktrees.

#### Case A: Current Directory is in a Linked Worktree
> Clarified during review-1: case selection is by **physical location** (is the
> process running inside the main worktree or a linked one), not by branch
> spelling. Selecting on `branch == "main"` misclassifies detached HEAD,
> `master`-default repos, and a non-main branch checked out in the main worktree.

When `sniff repo git-status` is executed inside a linked worktree (not the main worktree):
- Display the location of the `main` worktree.
- Display details for the **current** worktree (including branch name and its ahead/behind status relative to `main`).
- Display a count-only summary for all other active worktrees (without performing ahead/behind checks on them).

**Target Output Format:**
- `<b>main:</b> _the main worktree for this repo is located at <blue><a href="{absolute-path}">{relative-path}</a></blue>_`
- `<b>Current Worktree:</b>`
  - `  - you are in the <b>{worktree}</b> located at <blue><a href="{absolute-path}">{relative-path}</a></blue>`
  - `  - this worktree is on the <b>{branch}</b> branch and is {ahead-behind-main}`
- `<b>Other Worktrees:</b>`
  - `  - there are {#} other active worktrees in this repo`

#### Case B: Current Directory IS the Main Worktree
When `sniff repo git-status` is executed inside the main worktree (regardless of which branch is checked out there, including detached HEAD):
- Display details for the **current** worktree (main).
- Display a count-only summary of the other active worktrees. This section is **always rendered**, including a `there are 0 other active worktrees in this repo` line when the repository has no linked worktrees.

**Target Output Format:**
- `<b>Current Worktree:</b>`
  - `  - you are in the <b>main</b> worktree located at <blue><a href="{absolute-path}">{relative-path}</a></blue>`
- `<b>Other Worktrees:</b>`
  - `  - there are {#} other active worktrees in this repo`

---

### 2. Layout, Header Styling, and Vertical Spacing

Every major section in the `git-status` report (`Status`, `Worktrees`, and `Meta`) must be styled consistently to improve visual breathing room.

- **Header Styling:** Use the `<uu>` tag for genuine double-underlining of header titles (e.g., `<b><uu>Status</uu></b>`). On unsupported terminals, this degrades gracefully using `biscuit-terminal`'s standard capability-aware degradation logic.
- **Section Spacing:** 
  - Place exactly one blank row **before** and **after** each section header (separating the title from its content list).
  - Consolidate trailing/leading whitespace between sections to ensure exactly **one blank row** of separation between adjacent sections (no double blank lines).

**Layout Example:**
```
[blank line]
<b><uu>Status</uu></b>
[blank line]
- [Status/Commit list items]
[blank line]
<b><uu>Worktrees</uu></b>
[blank line]
- [Worktrees list items]
[blank line]
<b><uu>Meta</uu></b>
[blank line]
- [Meta list items]
```

---

### 3. Performance & Library Optimization

- **Performance Target:** Execution time of `sniff repo git-status` must be **under 500ms** on standard repositories.
- **Library-level Lazy Loading:**
  - Optimize `GitRequest` and the underlying `sniff` library's detection logic to only compute ahead/behind tracking for the *current* checked-out branch/worktree by default.
  - Do not walk commit histories or perform ahead/behind calculations for any non-current worktrees.
  - Introduce an option in `GitRequest` to explicitly request full worktree details if needed (for backwards compatibility/explicit consumers).
  - This ensures both the text mode and the default JSON response (`sniff repo git-status --json`) benefit from the performance optimization.
- **Network Exemption:** The 500ms performance threshold applies to local repository scans. Invoking with `--refresh-remotes` is exempt from the 500ms limit, as it contacts external git servers.
