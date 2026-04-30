Most `sniff repo` subcommands return identical, unfiltered JSON — the full `RepoInfo` blob — regardless of which subcommand was invoked. This breaks the contract that `--json` should mirror text-mode output. Only a few subcommands currently produce tailored JSON.

## Problem Statement

When `--json` is passed to a `sniff repo` subcommand, the output should be a JSON representation of what the text-mode output shows. Instead, 18 of 33 subcommands return the same unfiltered `RepoInfo` object, making them indistinguishable from each other and useless for scripting.

## Current Behavior

### Already Correct (no changes needed)

These subcommands produce tailored JSON that matches their text-mode output:

| Subcommand | JSON Shape |
|---|---|
| `dirty-files` | `{ "scope": "dirty", "kind": "all_files", "paths": [...] }` |
| `dirty-source-code` | `{ "scope": "dirty", "kind": "source_code", "paths": [...] }` |
| `staged-files` | `{ "scope": "staged", "kind": "all_files", "paths": [...] }` |
| `staged-source-code` | `{ "scope": "staged", "kind": "source_code", "paths": [...] }` |
| `unstaged-source-code` | `{ "scope": "unstaged", "kind": "source_code", "paths": [...] }` |
| `packages` | `["pkg-a", "pkg-b", ...]` (JSON array of strings) |
| `package-areas` | `["area-a", "area-b", ...]` (JSON array of strings) |
| `hash` | `{ "commit": {...}, "files": [...] }` |
| `root` | `{ "root": "/path/to/repo" }` |
| `remote` | `RemoteReport` JSON |
| `pr` | Array of `PullRequest` JSON objects |
| `unstaged-files` | Array of `FileChange` objects |
| `untracked-files` | Array of `FileChange` objects |

### Broken — Returns Full RepoInfo Blob (16 subcommands)

All 16 of these return the identical `RepoInfo` JSON from `apply_filter_to_json(result, OutputFilter::Repo, ...)`. The subcommand is effectively ignored for JSON output:

| Subcommand | What text mode shows |
|---|---|
| `structure` (and bare `sniff repo`) | Repo structure table |
| `git-status` | Git status + commit history |
| `deps` | Dependency diagram |
| `dirty-packages` | CSV of dirty package names |
| `dirty-package-areas` | CSV of dirty area names |
| `staged-packages` | CSV of staged package names |
| `staged-package-areas` | CSV of staged area names |
| `unstaged-packages` | CSV of unstaged package names |
| `unstaged-package-areas` | CSV of unstaged area names |
| `package-root` | Path to current package root |
| `package-area-root` | Path to current area root |
| `package` | Name of current package |
| `package-area` | Name of current area |
| `is-current-package-area-dirty` | Exit code only |
| `package-area-has-source-code-changes` | Exit code only |
| `has-merge-conflict` | Exit code only |

### Partially Broken — Returns Unfiltered CommitDescSet (2 subcommands)

| Subcommand | What text mode shows | What JSON returns |
|---|---|---|
| `recent-commits` | All commits (correct) | Full `CommitDescSet` (correct) |
| `source-code-changes` | Only source-code commits + files | Full `CommitDescSet` (broken — unfiltered) |
| `documentation-changes` | Only documentation commits + files | Full `CommitDescSet` (broken — unfiltered) |

`recent-commits` is already correct — it should return the full set. The other two are broken because they serialize the same unfiltered `CommitDescSet` even though their text-mode output applies `CommitCentricFilter::SourceCode` or `CommitCentricFilter::Documentation`.

## Expected Behavior

Every `sniff repo` subcommand with `--json` must return JSON that mirrors its text-mode output.

### Subcommand JSON Shapes

#### Structure Family

**`sniff repo` / `sniff repo structure --json`**

Returns the full `RepoInfo` JSON. This is already correct — it's the default `OutputFilter::Repo` path.

```json
{
  "is_monorepo": true,
  "root": "/path/to/repo",
  "packages": [...],
  "dependencies": [...],
  ...
}
```

#### Git Status

**`sniff repo git-status --json`**

Serialize `GitInfo` directly. The struct already derives `Serialize`, so the fix is to serialize `GitInfo` instead of the full `RepoInfo`.

```json
{
  "repo_root": "/path/to/repo",
  "org": "rusty-biscuit",
  "repo": "sniff",
  "current_branch": "main",
  "branches": [
    { "name": "main", "short_hash": "a1b2c3d", "ahead": 0, "behind": 2 }
  ],
  "in_worktree": false,
  "base_repo_root": null,
  "recent": [
    { "sha": "a1b2c3d", "message": "fix: ...", "author": "Ken", "timestamp": "...", "remotes": [], "refs": [] }
  ],
  "status": {
    "is_dirty": true,
    "staged_count": 2,
    "unstaged_count": 1,
    "untracked_count": 3,
    "dirty": [{ "path": "src/main.rs", "kind": "modified" }],
    "untracked": [{ "path": "new_file.rs" }],
    "is_behind": true
  },
  "remotes": [
    { "name": "origin", "url": "git@github.com:...", "provider": "github", "branches": [], "default_branch": "main" }
  ],
  "worktrees": {},
  "config": { "user_name": "...", "user_email": "...", ... },
  "tracking": [{ "remote": "origin", "ahead": 0, "behind": 2 }],
  "file_changes": [{ "path": "src/main.rs", "status": "M", "action": "modified", "lines_added": 5, "lines_removed": 2 }]
}
```

#### Dependency Diagram

**`sniff repo deps --json`**

Per-package internal + external deps. Each package entry includes internal graph edges (`depends_on`, `used_by`) and full external dependency tables.

```json
{
  "packages": [
    {
      "name": "sniff-lib",
      "depends_on": [],
      "used_by": ["sniff-cli"],
      "dependencies": [
        { "name": "serde", "targeted_version": "1.0", "actual_version": "1.0.210" }
      ],
      "dev_dependencies": [
        { "name": "tempfile", "targeted_version": "3.0", "actual_version": "3.12.0" }
      ]
    },
    {
      "name": "sniff-cli",
      "depends_on": ["sniff-lib"],
      "used_by": [],
      "dependencies": [
        { "name": "clap", "targeted_version": "4.4", "actual_version": "4.5.13" },
        { "name": "sniff-lib", "targeted_version": "0.1", "actual_version": null }
      ],
      "dev_dependencies": [],
      "peer_dependencies": [],
      "optional_dependencies": []
    }
  ]
}
```

`depends_on` and `used_by` contain workspace-internal package names. `dependencies`, `dev_dependencies`, `peer_dependencies`, and `optional_dependencies` contain external dependency entries with at minimum `name` and `targeted_version`. The `peer_dependencies` and `optional_dependencies` keys are omitted from packages that have none.

#### Dirty / Staged / Unstaged Package Families

These use the `{ scope, kind, names }` pattern (parallel to the file-family `{ scope, kind, paths }`):

**`dirty-packages` / `dirty-package-areas` / `staged-packages` / `staged-package-areas` / `unstaged-packages` / `unstaged-package-areas`**

```json
{
  "scope": "dirty",
  "kind": "packages",
  "names": ["sniff-lib", "sniff-cli", "..."]
}
```

```json
{
  "scope": "dirty",
  "kind": "package_areas",
  "names": ["sniff", "homelab"]
}
```

The `scope` value is `"dirty"`, `"staged"`, or `"unstaged"`. The `kind` is `"packages"` or `"package_areas"`. The `names` array contains the filtered package or area names.

#### Path / Name Locator Family

**`package-root` / `package-area-root`**

Returns a path object. *(proposed shape)*

```json
{
  "root": "/abs/path/to/package"
}
```

**`package` / `package-area`**

Returns a name object. *(proposed shape)*

```json
{
  "name": "sniff-lib"
}
```

```json
{
  "name": "sniff"
}
```

#### Boolean Exit-Code Family

**`is-current-package-area-dirty` / `package-area-has-source-code-changes` / `has-merge-conflict`**

These are exit-code-only commands in text mode. For JSON, return a boolean object. *(proposed shape)*

```json
{
  "dirty": true
}
```

```json
{
  "has_source_code_changes": true
}
```

```json
{
  "has_merge_conflict": false
}
```

The key should be a descriptive boolean matching the subcommand semantics. Exit code should also reflect the boolean (0 = true, 1 = false) for backward compatibility with scripts that check exit code.

#### Commit Family

**`recent-commits`** — Already correct. Returns full `CommitDescSet`.

**`source-code-changes`** — Return filtered `CommitDescSet` where:
- Only commits with at least one source-code file are included
- Only source-code files appear in each commit's `files` array
- Add a `"filter": "source_code"` field for clarity

```json
{
  "filter": "source_code",
  "commits": [ { "hash": "...", "files": [ { "path": "src/main.rs", "kind": "modified" } ] } ],
  "period_label": "last 3d",
  "repo_root": "/path"
}
```

**`documentation-changes`** — Return filtered `CommitDescSet` where:
- Only commits with at least one documentation file are included
- Only documentation files appear in each commit's `files` array
- Add a `"filter": "documentation"` field for clarity

```json
{
  "filter": "documentation",
  "commits": [ { "hash": "...", "files": [ { "path": "README.md", "kind": "modified" } ] } ],
  "period_label": "last 3d",
  "repo_root": "/path"
}
```

## Affected Subcommands Summary

| Subcommand | Bug | Fix |
|---|---|---|
| `git-status` | Full RepoInfo | Serialize `GitInfo` directly |
| `deps` | Full RepoInfo | Per-package internal + external deps |
| `dirty-packages` | Full RepoInfo | `{ scope, kind, names }` |
| `dirty-package-areas` | Full RepoInfo | `{ scope, kind, names }` |
| `staged-packages` | Full RepoInfo | `{ scope, kind, names }` |
| `staged-package-areas` | Full RepoInfo | `{ scope, kind, names }` |
| `unstaged-packages` | Full RepoInfo | `{ scope, kind, names }` |
| `unstaged-package-areas` | Full RepoInfo | `{ scope, kind, names }` |
| `package-root` | Full RepoInfo | `{ root: "..." }` |
| `package-area-root` | Full RepoInfo | `{ root: "..." }` |
| `package` | Full RepoInfo | `{ name: "..." }` |
| `package-area` | Full RepoInfo | `{ name: "..." }` |
| `is-current-package-area-dirty` | Full RepoInfo | `{ dirty: bool }` + exit code |
| `package-area-has-source-code-changes` | Full RepoInfo | `{ has_source_code_changes: bool }` + exit code |
| `has-merge-conflict` | Full RepoInfo | `{ has_merge_conflict: bool }` + exit code |
| `structure` / bare `sniff repo` | Correct | No change needed |
| `source-code-changes` | Unfiltered CommitDescSet | Filter to source-code commits/files |
| `documentation-changes` | Unfiltered CommitDescSet | Filter to documentation commits/files |

## Acceptance Criteria

1. Every `sniff repo` subcommand with `--json` returns JSON that semantically matches its text-mode output
2. No two different subcommands return identical JSON (except `structure` and bare `sniff repo`, which are the same command)
3. Dirty/staged/unstaged package and package-area commands return `{ scope, kind, names }` objects
4. `source-code-changes --json` returns only source-code commits with only source-code files
5. `documentation-changes --json` returns only documentation commits with only documentation files
6. Boolean subcommands (`is-current-package-area-dirty`, `package-area-has-source-code-changes`, `has-merge-conflict`) return JSON with a descriptive boolean key and maintain exit-code behavior
7. Locator subcommands (`package-root`, `package-area-root`, `package`, `package-area`) return focused JSON objects
8. All existing subcommands that already return correct JSON remain unchanged
9. `--perf` output continues to work alongside the new JSON shapes
