---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/git.rs
  - sniff/lib/src/filesystem/mod.rs
---

# The `sniff repo git-status` Subcommand

Shows a compact git status for the current repository: recent commit history, working tree changes (staged, unstaged, untracked), worktree summaries, and repository meta (branches, remotes, git config).

## Usage

```
sniff repo git-status [OPTIONS]
sniff [--json] [--plain] [-v] repo git-status [--history <N>] [--refresh-remotes] [-p <PKG>]
```

## Flags and Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--history <N>` | | `10` | Number of recent commits to display |
| `--refresh-remotes` | | off | Fetch remotes to check if branches are out of sync (enables `--deep` detection) |
| `--package <PKG>` | `-p` | | Scope the git view to a specific package or package area |
| `--json` | | off | Emit JSON instead of styled text (global flag) |
| `--plain` | | off | Strip all ANSI escape codes from text output (global flag) |
| `--verbose` / `-v` | `-v` | 0 | Increase output verbosity; repeatable (`-vv`) |

### `--history <N>`

Controls how many recent commits appear in the Status section. Commits are displayed oldest-first within the list so the most recent commit is at the bottom. Defaults to `10`.

### `--refresh-remotes`

Enables deep detection: fetches remote refs to populate tracking data. Without this flag, ahead/behind counts for remotes come from locally-cached ref data only. When enabled, `RemoteInfo.branches` and `RemoteInfo.default_branch` are also populated, and the `is_behind` field on `RepoStatus` is computed.

### `-p` / `--package <PKG>`

Scopes all git output to a single package or package area. Accepts:

- A package name (e.g., `homelab-cli`) — matched against `Package.name` case-insensitively
- A package area (e.g., `homelab`) — matched against `Package.package_area` case-insensitively

When scoped, recent commits are filtered to paths under the package directory, and file changes are filtered to that path prefix. Staged, unstaged, and untracked counts are recomputed after filtering. If the name matches nothing, an error lists valid package names and areas.

## Default Output

The output is divided into three labeled sections: **Status**, **Worktrees** (only when worktrees exist), and **Meta**.

### Status Section

Lists up to `--history` recent commits followed by any working tree changes. Items are rendered as a `UnorderedList`.

#### Commit Lines

Each commit renders as a one-liner using `format_commit_line`. The SHA (first 7 characters) is shown as an OSC8 hyperlink when the commit has been pushed to `origin`; unpushed commits show the SHA as plain bold text. The "pushed" boundary is derived from the `origin` remote's ahead count.

Conventional commit format (`type(scope): description`) is parsed and displayed with the type and scope styled separately from the description.

At `verbose > 0`, the author name is appended to each commit line.

#### File Change Lines

After commits, file changes are listed in three groups (staged, then unstaged, then untracked):

- **Staged** — rendered in lime: `staged(<action>): [dir/]<filename>` with diff stats for modified files
- **Unstaged** — rendered in yellow: `unstaged(<action>): [dir/]<filename>` with diff stats
- **Untracked** — rendered in red: `untracked: [dir/]<filename>`

Diff stats appear as `<N added, M removed>` and are omitted for created or deleted files.

When there are no changes at all, the section shows `No changes` in dim text.

### Worktrees Section

Only rendered when `git.worktrees` is non-empty (the repository has linked worktrees). Each item in this section is a `UnorderedList` entry.

#### Base Repo Line

The first item describes the base repository. The rendering differs based on whether the current working directory is inside a worktree or the base repo:

- **In the base repo:** `Base Repo: you are in the base repo which is on the <branch> branch`
- **Inside a worktree:** `Base Repo: the base repo is located at <base_repo_root_path>` (dim text)

#### Worktree Lines

One line per linked worktree. Each line shows the worktree's branch name and its relationship to its base branch. The rendering differs based on whether the user is currently inside that particular worktree:

- **Current worktree** (bold branch name): `<branch>: you are <status> · <merge_status><uncommitted>`
- **Other worktrees** (normal weight): `<branch>: is <status> · <merge_status><uncommitted>` (status in dim)

The `<status>` phrase is derived from the worktree's ahead/behind counts relative to its base branch:

| Condition | Status Text |
|-----------|-------------|
| `merged && ahead == 0` | `merged into <base_branch>` |
| `ahead > 0, behind == 0` | `N ahead of <base_branch>` |
| `ahead == 0, behind > 0` | `N behind <base_branch>` |
| both nonzero | `N ahead, M behind of <base_branch>` |
| both zero | `up to date with <base_branch>` |

The `<merge_status>` suffix is either ` · conflicts` (red) or ` · clean` (green), derived from `WorktreeInfo.has_conflicts`.

When `WorktreeInfo.changed_files > 0`, an `<uncommitted>` suffix is appended: ` · <N> uncommitted file[s]` (red count, dim label).

### Meta Section

Shows repository metadata as a nested `UnorderedList`.

#### Local / Branches

- **Default (non-verbose):** A single line showing the current branch in blue, followed by up to 3 other branch names in parentheses. If there are more than 4 branches total, a `+N more` suffix is appended.
- **Verbose (`-v`):** A nested `Branches:` list where the current branch is shown with its short hash and `(current)` label (bold blue), and each other branch shows its short hash and ahead/behind counts relative to the current branch. Ahead/behind arrows use Nerd Font glyphs when the terminal supports them.

#### Remotes

One line per configured remote showing its name, ahead/behind tracking status (from `git.tracking`), optional default branch (only when `--refresh-remotes` was used), and a hyperlinked `owner/repo` path to the remote's browser URL.

At `verbose > 0`, remote branches (excluding the default branch) are shown as a nested dim list under each remote.

#### Config

Shows git user identity (`user.name` and `user.email`) when configured. At `verbose > 0`, expands to include:

- **Crypto:** GPG use-agent, program, credential helper, signing key, commit signing, tag signing
- **Pager:** configured pager; when `delta`, also shows syntax theme, light mode, and side-by-side mode

## Verbose Mode

`-v` (verbosity 1) expands:
- Commit lines to include the author name
- Local branches to a nested per-branch list with hashes and ahead/behind counts
- Remote sections to show remote branch lists
- Config section to include crypto and pager details

`-vv` (verbosity 2) is accepted but does not add additional output beyond `-v` for this subcommand.

## JSON Output (`--json`)

```
sniff --json repo git-status [--history <N>] [--refresh-remotes] [-p <PKG>]
```

Returns the full `FilesystemInfo` JSON object. The `git` field contains the `GitInfo` structure:

```json
{
  "filesystem": {
    "git": {
      "repo_root": "/absolute/path/to/repo",
      "org": "owner",
      "repo": "repo-name",
      "current_branch": "main",
      "branches": [
        {
          "name": "main",
          "short_hash": "a1b2c3d4",
          "ahead": 0,
          "behind": 0
        }
      ],
      "in_worktree": false,
      "base_repo_root": null,
      "recent": [
        {
          "sha": "a1b2c3d4e5f6...",
          "message": "feat(cli): add new flag",
          "author": "Ken Snyder",
          "timestamp": "2026-03-24T12:00:00Z",
          "refs": [
            {
              "name": "main",
              "kind": "LocalBranch",
              "is_head": true
            }
          ]
        }
      ],
      "status": {
        "is_dirty": true,
        "staged_count": 1,
        "unstaged_count": 2,
        "untracked_count": 0,
        "dirty": [
          {
            "filepath": "src/main.rs",
            "absolute_filepath": "/path/to/repo/src/main.rs",
            "diff": "--- a/src/main.rs\n+++ b/src/main.rs\n...",
            "last_local_commit": "a1b2c3d4...",
            "origin_commit": "a1b2c3d4..."
          }
        ],
        "untracked": [],
        "is_behind": false
      },
      "remotes": [
        {
          "name": "origin",
          "url": "https://github.com/owner/repo.git",
          "provider": "GitHub",
          "branches": null,
          "default_branch": null
        }
      ],
      "worktrees": {
        "feat/my-feature": {
          "branch": "feat/my-feature",
          "filepath": "/path/to/worktrees/feat-my-feature",
          "sha": "b2c3d4e5f6a1...",
          "dirty": false,
          "ahead": 3,
          "behind": 0,
          "base_branch": "main",
          "has_conflicts": false,
          "merged": false,
          "changed_files": 0
        }
      },
      "config": {
        "user_name": "Ken Snyder",
        "user_email": "ken@example.com"
      },
      "tracking": [
        {
          "remote": "origin",
          "ahead": 2,
          "behind": 0
        }
      ],
      "file_changes": [
        {
          "path": "src/main.rs",
          "status": "Staged",
          "action": "Modified",
          "lines_added": 5,
          "lines_removed": 2
        }
      ]
    }
  }
}
```

Key notes on the JSON schema:

- `is_behind` — omitted unless `--refresh-remotes` was used; serializes as `false` when not behind any remote, or as an array of remote names when behind one or more
- `remotes[].branches` — omitted unless `--refresh-remotes` was used
- `remotes[].default_branch` — omitted unless `--refresh-remotes` was used
- `recent[].remotes` — omitted unless `--refresh-remotes` was used
- `worktrees` — keyed by branch name; empty object `{}` when no linked worktrees exist
- `base_repo_root` — only present when `in_worktree` is `true`
- `file_changes[].status` — one of `"Staged"`, `"Modified"`, `"Both"`, `"Untracked"`
- `file_changes[].action` — one of `"Created"`, `"Modified"`, `"Deleted"`

When `-p`/`--package` is used, `recent` commits, `file_changes`, `status.dirty`, `status.untracked`, and the derived counts are all filtered to the package path before serialization.

## Plain Output (`--plain`)

Adding `--plain` strips all ANSI escape codes (colors, bold, OSC8 hyperlinks) from text output. Useful for piping or logging.
