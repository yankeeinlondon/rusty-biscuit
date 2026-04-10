# Tech Design: Recent Commits

Feature spec: `sniff/features/2026-04-09-recent-commits/spec.md`

## Overview

This feature adds three library functions for querying recent commits by duration, hash, or date, a `CommitDesc` value object that carries structured commit data with Markdown/terminal rendering, and three new CLI subcommands under `sniff repo`.

## Architecture

```
sniff/lib/src/filesystem/
├── git/
│   ├── mod.rs               ← re-export CommitDesc, CommitDescSet, period types
│   ├── recent_commits.rs    ← NEW: period parsing + commit querying
│   ├── types.rs             ← CommitInfo (existing, unchanged)
│   └── detection.rs         ← existing low-level git helpers
sniff/cli/src/
├── args.rs                  ← new RepoSubcommand variants
├── commands.rs              ← dispatch for new subcommands
└── output/
    └── recent_commits.rs    ← NEW: CLI output rendering helpers
```

## Part 1: Library — `CommitDesc` and `CommitDescSet`

### File: `sniff/lib/src/filesystem/git/recent_commits.rs`

The spec describes `CommitDesc` as a single commit, but the `describe()`, `source_code_changes()`, and `documentation_changes()` methods inherently operate over a _set_ of commits (they produce a single Markdown document spanning multiple commits). We split into:

- **`CommitDesc`** — data for a single commit (pure data, `Serialize`/`Deserialize`).
- **`CommitDescSet`** — owns a `Vec<CommitDesc>` plus period metadata; provides the `describe()` / `source_code_changes()` / `documentation_changes()` rendering methods.

### `CommitDesc`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDesc {
    pub hash: String,
    pub datetime: String,       // ISO 8601 UTC: "2026-04-09T14:30:00Z"
    pub packages: Option<Vec<String>>,
    pub package_areas: Option<Vec<String>>,
    pub files: Vec<String>,     // relative paths from repo root
    pub description: String,
    pub bullet_points: Vec<String>,
}
```

Notes:

- `datetime` is an ISO 8601 UTC string for JSON portability. Parsing/formatting uses `chrono`.
- `packages` and `package_areas` are `None` for non-monorepos, `Some(vec![])` for monorepos where no packages were affected.
- `description` is the first paragraph of the commit message (up to the first blank line or bullet point).
- `bullet_points` are lines from the commit body that start with `- ` or `* `, trimmed of the leading marker.

### `CommitDescSet`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDescSet {
    pub commits: Vec<CommitDesc>,
    pub period_label: String,   // human-readable, e.g. "last 3 days", "since 2025-12-04"
    pub repo_root: PathBuf,
}
```

Rendering methods on `CommitDescSet`:

| Method | Returns | Notes |
|--------|---------|-------|
| `describe(plain: bool)` | `String` | Markdown — one section per commit; `plain` strips hyperlinks |
| `source_code_changes(plain: bool)` | `String` | Markdown — grouped by source file; `plain` strips hyperlinks |
| `documentation_changes(plain: bool)` | `String` | Markdown — grouped by doc file; `plain` strips hyperlinks |

**Note:** Terminal rendering via `darkmatter` is handled in the CLI layer, not in the library. This avoids a cyclic dependency (darkmatter depends on sniff).

### File classification for `source_code_changes` / `documentation_changes`

Reuse `is_source_code_path()` from `blast_radius.rs`. A file is "documentation" if it has a `.md`, `.mdx`, `.rst`, `.txt`, or `.adoc` extension, or if the file-type registry classifies it as `FileAssociation::Documentation`. All other files that are not source code are excluded from both views.

A new helper `is_documentation_path()` will live alongside `is_source_code_path()` in `blast_radius.rs` (or in a shared location):

```rust
pub fn is_documentation_path(path: &Path) -> bool {
    // .md, .mdx, .rst, .txt, .adoc
    // or FileAssociation::Documentation from registry
}
```

### Commit message parsing

A private helper `parse_commit_message(message: &str) -> (String, Vec<String>)` will:

1. Split the message on `\n\n` to separate subject from body.
2. The first paragraph (before blank line or first bullet) becomes `description`.
3. Lines starting with `- ` or `* ` become `bullet_points`.
4. Non-bullet body lines are appended to `description` (separated by a space).

## Part 2: Library — Period Parsing and Query Functions

### `PeriodSpecifier`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PeriodSpecifier {
    Duration(chrono::Duration),
    Date(chrono::NaiveDate),
    Hash(String),
    Today,
    Yesterday,
}
```

### `parse_period(input: &str) -> Result<PeriodSpecifier>`

Parses a user-supplied period string:

| Input | Result |
|-------|--------|
| `3d`, `3 days` | `Duration(3 days)` |
| `6h`, `6 hours` | `Duration(6 hours)` |
| `1w`, `1wk`, `1 week` | `Duration(7 days)` |
| `3m`, `3mo`, `3 months` | `Duration(90 days)` (approximate) |
| `today` | `Today` |
| `yesterday` | `Yesterday` |
| `2025-12-04` | `Date(2025-12-04)` |
| `a1b2c3d` (7+ hex chars) | `Hash("a1b2c3d")` |

Disambiguation order:

1. Check for exact keywords `today` / `yesterday` (case-insensitive).
2. Try ISO date parse (`YYYY-MM-DD`).
3. Try duration parse (`<number><unit>` or `<number> <unit>`).
4. Assume hash if input contains only hex characters and is at least 7 characters long.
5. Otherwise return `SniffError::InvalidPeriod`.

### Duration unit mapping

```
h, hour, hours    → hours
d, day, days      → days
w, wk, week, weeks → weeks (× 7 days)
mo, m, month, months → months (× 30 days, approximate)
y, yr, year, years → years (× 365 days, approximate)
```

Note: `m` is ambiguous with minutes in some contexts, but the spec explicitly includes `3m` for months. Since commit history queries at minute granularity are not useful, `m` maps to months.

### New error variant

Add to `SniffError`:

```rust
#[error("invalid period specifier: '{0}'. Expected duration (e.g., 3d, 1w), date (YYYY-MM-DD), hash, 'today', or 'yesterday'.")]
InvalidPeriod(String),
```

### Three public query functions

All three functions open a `git2::Repository` via `GitRepo::discover()`, walk commits from HEAD, and build `CommitDescSet`.

```rust
pub fn get_recent_commits_by_duration(
    base_dir: &Path,
    duration: chrono::Duration,
    period_label: &str,
) -> Result<CommitDescSet>

pub fn get_recent_commits_by_date(
    base_dir: &Path,
    date: chrono::NaiveDate,
) -> Result<CommitDescSet>

pub fn get_recent_commits_by_hash(
    base_dir: &Path,
    hash: &str,
) -> Result<CommitDescSet>
```

### Implementation strategy — shared walker

All three share an internal `collect_commits_since(repo: &Repository, since: DateTime<Utc>) -> Vec<CommitDesc>` helper:

1. `repo.revwalk().push_head()` — iterate from HEAD.
2. For each commit, compare `commit.time()` to the cutoff.
3. Stop walking once we pass the cutoff (commits are newest-first).
4. For each matching commit:
   - Get files via `get_commit_files()` (existing).
   - Parse message via `parse_commit_message()`.
   - Determine affected packages/areas from file paths + `detect_repo()`.
5. Return the list.

The hash variant resolves the hash to a commit, gets its timestamp, then calls the same walker.

### Package/area attribution

After collecting files for a commit, determine which packages and package areas were affected:

1. Call `detect_repo(repo_root)` once (cache per query, not per commit).
2. For each file path, check if it falls under a known package's `relative` path.
3. Collect unique package names and package area names.

If the repo is not a monorepo (`repo_info.is_monorepo == false`), `packages` and `package_areas` remain `None`.

### Performance: repo detection caching

`detect_repo()` walks the filesystem and is expensive. The internal helper accepts an `Option<&RepoInfo>` parameter. The three public functions call `detect_repo()` once and pass it through.

## Part 3: Rendering

### `describe()` Markdown format

For each commit in the set:

```markdown
## 2026-04-09 at 14:30

- **Commit:** a1b2c3d4
- **Files:**
    - [sniff/lib/src/lib.rs](file:///abs/path/sniff/lib/src/lib.rs)
- **Description:** feat(sniff): add recent commits query
    - added period parsing
    - added CommitDesc struct
```

When `--plain` is active, hyperlinks are stripped: `[{relative-file}]({absolute-file})` becomes just `{relative-file}`.

### `source_code_changes()` Markdown format

```markdown
### Source Code Changes (_since last 3 days_)

- [sniff/lib/src/lib.rs](file:///abs/path/sniff/lib/src/lib.rs)
    - 2026-04-09 - _feat(sniff): add recent commits query_ as part of commit **a1b2c3d4**
        - added period parsing
        - added CommitDesc struct
- [sniff/cli/src/commands.rs](file:///abs/path/sniff/cli/src/commands.rs)
    - 2026-04-08 - _feat(cli): wire up recent-commits command_ as part of commit **e5f6a7b8**
```

Grouped by file path. For each file, commits are listed chronologically (newest first).

### `documentation_changes()` Markdown format

Same structure as `source_code_changes()` but filtered to documentation files only.

### Terminal rendering

Terminal rendering is handled in the CLI layer (`sniff/cli/src/output/recent_commits.rs`) using `darkmatter`:

1. The library's Markdown method is called (e.g., `describe(false)`) to generate Markdown with hyperlinks.
2. The Markdown string is passed to `darkmatter::markdown::Markdown::from()` and then `darkmatter::markdown::output::terminal::for_terminal()`.
3. When `--plain` is set, the library's plain method is used directly (e.g., `describe(true)`) which omits hyperlinks.

This separation avoids a cyclic dependency between `sniff` and `darkmatter`.

## Part 4: CLI Integration

### New `RepoSubcommand` variants

Add to `RepoSubcommand` in `args.rs`:

```rust
/// Show recent commits for a period
#[command(name = "recent-commits")]
RecentCommits {
    /// Period: duration (3d, 1w), date (2025-12-04), hash, 'today', 'yesterday'
    period: Option<String>,

    /// Scope to a specific package
    #[arg(long, value_name = "PKG", add = ...)]
    package: Option<String>,

    /// Scope to a specific package area
    #[arg(long, value_name = "AREA", add = ...)]
    package_area: Option<String>,

    /// Exit 0 with no output when no results found
    #[arg(long)]
    no_error: bool,

    /// Message to display when no results found
    #[arg(long, value_name = "MESSAGE")]
    on_error: Option<String>,
},

/// Show source code changes for a period
#[command(name = "source-code-changes")]
SourceCodeChanges {
    /// Period: duration (3d, 1w), date (2025-12-04), hash, 'today', 'yesterday'
    period: Option<String>,

    /// Scope to a specific package
    #[arg(long, value_name = "PKG", add = ...)]
    package: Option<String>,

    /// Scope to a specific package area
    #[arg(long, value_name = "AREA", add = ...)]
    package_area: Option<String>,

    /// Exit 0 with no output when no results found
    #[arg(long)]
    no_error: bool,

    /// Message to display when no results found
    #[arg(long, value_name = "MESSAGE")]
    on_error: Option<String>,
},

/// Show documentation changes for a period
#[command(name = "documentation-changes")]
DocumentationChanges {
    /// Period: duration (3d, 1w), date (2025-12-04), hash, 'today', 'yesterday'
    period: Option<String>,

    /// Scope to a specific package
    #[arg(long, value_name = "PKG", add = ...)]
    package: Option<String>,

    /// Scope to a specific package area
    #[arg(long, value_name = "AREA", add = ...)]
    package_area: Option<String>,

    /// Exit 0 with no output when no results found
    #[arg(long)]
    no_error: bool,

    /// Message to display when no results found
    #[arg(long, value_name = "MESSAGE")]
    on_error: Option<String>,
},
```

### New `RepoAction` variants

Add matching variants to `RepoAction`:

```rust
RecentCommits {
    period: Option<String>,
    package: Option<String>,
    package_area: Option<String>,
    no_error: bool,
    on_error: Option<String>,
},
SourceCodeChanges {
    period: Option<String>,
    package: Option<String>,
    package_area: Option<String>,
    no_error: bool,
    on_error: Option<String>,
},
DocumentationChanges {
    period: Option<String>,
    package: Option<String>,
    package_area: Option<String>,
    no_error: bool,
    on_error: Option<String>,
},
```

### Dispatch in `commands.rs`

Add an early-return handler for the three new `RepoAction` variants, before the full detection plan. These commands don't need `SniffResult` — they use the library functions directly.

```rust
RepoAction::RecentCommits { period, package, package_area, no_error, on_error }
| RepoAction::SourceCodeChanges { period, package, package_area, no_error, on_error }
| RepoAction::DocumentationChanges { period, package, package_area, no_error, on_error } => {
    return handle_recent_commits_command(
        action, base_dir.as_deref(), cli.json, cli.plain, cli.verbose,
    ).await;
}
```

### `handle_recent_commits_command`

A new function in `commands.rs` (or a new submodule `commands/recent_commits.rs` if it grows large):

1. Resolve period: if `None`, default to `"3d"`.
2. Call `parse_period(period_str)?`.
3. Dispatch to the appropriate library function:
   - `Duration` / `Today` / `Yesterday` → `get_recent_commits_by_duration()`
   - `Date` → `get_recent_commits_by_date()`
   - `Hash` → `get_recent_commits_by_hash()`
4. If `--package` or `--package-area` is specified, filter the `CommitDescSet`:
   - For each `CommitDesc`, retain only files under the matching package/area path.
   - Remove commits with zero remaining files.
   - Retain only matching packages/areas in the metadata.
 5. If the result is empty, delegate to `handle_no_results(no_error, on_error, plain)`.
 6. Output:
    - `--json`: serialize `CommitDescSet` to JSON.
    - `RecentCommits`: call `describe(plain)`, render via darkmatter for terminal (unless `--plain`).
    - `SourceCodeChanges`: call `source_code_changes(plain)`, render via darkmatter for terminal.
    - `DocumentationChanges`: call `documentation_changes(plain)`, render via darkmatter for terminal.

### Default period

The default period is `3d` (3 days). This is handled at the CLI layer — the library functions require explicit parameters.

### `--plain` behavior

The global `--plain` flag already strips ANSI escape codes via `emit_text()`. For these new commands, `--plain` additionally:

- Removes hyperlink syntax from Markdown before rendering.
- Falls back to the plain `describe()` / `source_code_changes()` / `documentation_changes()` methods which include raw file paths without hyperlink wrapping.

The Markdown generation methods on `CommitDescSet` accept a `plain: bool` parameter (or a `RenderOptions` struct) to control whether hyperlinks are included.

### `--no-error` / `--on-error`

Uses the existing `handle_no_results()` function, consistent with `blast-radius` and file-list commands.

## Part 5: Error Handling

| Scenario | Error |
|----------|-------|
| Invalid period string | `SniffError::InvalidPeriod(input)` |
| Not a git repo | `SniffError::NotARepository(path)` |
| Hash not found | `SniffError::Git(git2::Error)` — wrapped with context |
| No results found | CLI exits 1 (or 0 with `--no-error`), prints `on_error` message if set |
| `--package` on non-monorepo | `SniffError::NotAMonorepo(path)` — reuses existing pattern |

## Part 6: Module Visibility

### `sniff/lib/src/filesystem/git/mod.rs`

Add:

```rust
pub mod recent_commits;
pub use recent_commits::{
    CommitDesc, CommitDescSet, PeriodSpecifier,
    get_recent_commits_by_date, get_recent_commits_by_hash,
    get_recent_commits_by_duration, parse_period,
};
```

### `sniff/lib/src/filesystem/mod.rs`

Add to the `pub use git::` block:

```rust
    CommitDesc, CommitDescSet, PeriodSpecifier,
    get_recent_commits_by_date, get_recent_commits_by_hash,
    get_recent_commits_by_duration, parse_period,
```

## Part 7: Testing Strategy

### Unit tests — `recent_commits.rs`

1. `parse_period` tests:
   - Duration forms: `"3d"`, `"3 days"`, `"6h"`, `"1w"`, `"1wk"`, `"3mo"`, `"3 months"`.
   - Keywords: `"today"`, `"yesterday"`.
   - ISO date: `"2025-12-04"`.
   - Hash: `"a1b2c3d"`, `"a1b2c3d4e5f6"`.
   - Invalid: `"foo"`, `"3x"`, `"123"`.

2. `parse_commit_message` tests:
   - Simple message → description only.
   - Message with bullets → description + bullet_points.
   - Multi-paragraph message → first paragraph as description, bullets from body.
   - Empty message.

3. Integration tests (require git repo):
   - Create a temp repo with known commits at known timestamps.
   - `get_recent_commits_by_duration` returns expected commits.
   - `get_recent_commits_by_date` returns commits on/after the date.
   - `get_recent_commits_by_hash` returns commits from the hash to HEAD.
   - Package/area attribution works for monorepo layout.

### Unit tests — `CommitDescSet` rendering

1. `describe()` produces valid Markdown with expected structure.
2. `source_code_changes()` groups by file, filters to source code.
3. `documentation_changes()` groups by file, filters to docs.
4. Plain mode strips hyperlinks.

### CLI integration tests

Using `assert_cmd` and `tempfile` (existing dev-dependencies):

1. `sniff repo recent-commits` — default 3-day period.
2. `sniff repo recent-commits 1d` — duration.
3. `sniff repo recent-commits 2025-12-04` — date.
4. `sniff repo recent-commits abc1234` — hash.
5. `sniff repo source-code-changes 1w` — source code view.
6. `sniff repo documentation-changes 1w` — documentation view.
7. `--json` flag serializes correctly.
8. `--plain` strips hyperlinks and escape codes.
9. `--package` and `--package-area` filter results.
10. No results → exit 1 (or 0 with `--no-error`).

## Part 8: Dependencies

No new dependencies required:

- `chrono` — already in `sniff/lib/Cargo.toml` for duration/date handling.
- `git2` — already used for repository access.
- `darkmatter` — already in `sniff-cli/Cargo.toml` for terminal rendering.
- `serde` / `serde_json` — already used for serialization.
- `thiserror` — already used for error types.

## Part 9: Implementation Order

 1. **`parse_period` + `PeriodSpecifier`** — pure parsing, easy to test.
 2. **`parse_commit_message`** — pure string parsing.
 3. **`CommitDesc` + `CommitDescSet`** — data types.
 4. **`get_recent_commits_by_*`** — library query functions.
 5. **Rendering methods** — `describe()`, `source_code_changes()`, `documentation_changes()`.
 6. **`is_documentation_path`** — file classification helper.
 7. **CLI args** — new `RepoSubcommand` variants.
 8. **CLI dispatch** — `handle_recent_commits_command`.
 9. **Terminal rendering** — darkmatter rendering in CLI layer (not in library to avoid cyclic dep).
10. **Tests** — unit + integration.
