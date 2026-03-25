---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/git.rs
---

# The `sniff repo hash <SHA>` Subcommand

Looks up a single commit by SHA and displays its metadata and changed files. Useful for quickly inspecting what a commit touched without leaving the terminal.

## Argument: `<SHA>`

```
sniff repo hash <SHA>
```

The `<SHA>` argument is resolved via `git2::Repository::revparse_single()`, which accepts:

- `HEAD` — the current commit
- Short hash — any unambiguous abbreviated SHA (e.g., `a1b2c3d`)
- Full hash — the complete 40-character SHA

If the SHA cannot be resolved to a valid commit, the command exits with an error: `Commit not found: <SHA>`.

If run outside a git repository, the command exits with: `Not a git repository: <reason>`.

The repository is discovered from the current directory unless `--base <dir>` is passed.

## Default Output

Output is divided into two sections.

### Commit

A bold, underlined **Commit** heading is followed by a single-item list containing the commit rendered as a one-liner. The format depends on whether the message follows conventional commit style.

**Conventional commit** (e.g. `feat(scope): description`):

```
[<sha>] <type>(<scope>) at <time> on <date> [<refs>]: <description>
```

**Non-conventional commit** (truncated at 50 characters if longer):

```
[<sha>] <message> <date> [<refs>]
```

In both cases:

- The 7-character short SHA is shown in bold inside brackets
- If the commit's origin remote URL maps to a known hosting provider (GitHub, GitLab, Bitbucket, Forgejo, Gitea), the SHA is rendered as an **OSC8 hyperlink** pointing to the commit's browser URL
- The timestamp is split into a time portion and a date portion; dates from today omit the "on" prefix
- Refs (local branches, remote tracking branches, tags) are shown as decorations after the date when present

### Files changed

A bold, underlined **Files changed** heading is followed by a list of every file the commit touched. Each entry shows the delta kind and the file path, color-coded by kind:

| Delta kind | Color |
|------------|-------|
| added | lime (green) |
| modified | yellow |
| deleted | red |
| renamed | cyan |
| copied | cyan |

The filename component of each path is rendered in bold; the directory prefix is rendered without additional styling.

If the commit has no file changes (e.g., an empty initial commit), the **Files changed** section is omitted entirely.

## Verbose Mode (`-v`)

Passing `-v` (or `--verbose`) adds the author name to the commit one-liner:

```
[<sha>] <type>(<scope>) at <time> on <date> [<refs>] by <author>: <description>
```

The author is styled in bold indigo. There is no additional verbosity at `-vv` or higher for this subcommand.

## JSON Output (`--json`)

```
sniff --json repo hash <SHA>
```

Emits a single JSON object with two top-level keys: `commit` and `files`.

```json
{
  "commit": {
    "sha": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
    "message": "feat(sniff): add hash subcommand",
    "author": "Ken Snyder",
    "timestamp": "2026-03-24T12:34:56Z",
    "refs": [
      {
        "name": "main",
        "kind": "LocalBranch",
        "is_head": true
      },
      {
        "name": "origin/main",
        "kind": "RemoteBranch"
      }
    ]
  },
  "files": [
    {
      "path": "sniff/cli/src/commands.rs",
      "kind": "Modified"
    },
    {
      "path": "sniff/lib/src/filesystem/git.rs",
      "kind": "Modified"
    }
  ]
}
```

### `commit` object

| Field | Type | Notes |
|-------|------|-------|
| `sha` | string | Full 40-character SHA |
| `message` | string | Full commit message, trimmed |
| `author` | string | Author display name |
| `timestamp` | string | ISO 8601 UTC timestamp |
| `refs` | array | Omitted when empty |
| `remotes` | array\|null | Omitted from output (not populated by this subcommand) |

### `refs` items

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | Ref name, e.g. `"main"`, `"origin/main"`, `"v1.0.0"` |
| `kind` | string | `"LocalBranch"`, `"RemoteBranch"`, or `"Tag"` |
| `is_head` | boolean | Omitted when `false`; only set for local branches |

### `files` items

| Field | Type | Notes |
|-------|------|-------|
| `path` | string | Repo-relative file path |
| `kind` | string | `"Added"`, `"Modified"`, `"Deleted"`, `"Renamed"`, or `"Copied"` |

## Plain Output (`--plain`)

Adding `--plain` strips all ANSI escape codes (colors, bold, OSC8 hyperlinks) from the text output.
