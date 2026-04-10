# Implementation Plan: Recent Commits

Spec: `sniff/features/2026-04-09-recent-commits/spec.md`
Tech Design: `sniff/features/2026-04-09-recent-commits/tech-design.md`

## Confidence: HIGH

The tech design aligns closely with existing codebase patterns. Key validations:

- `get_commit_files(repo, full_sha)` already exists in `detection.rs` and returns `Vec<(PathBuf, DeltaKind)>`
- `detect_repo(root)` returns `Result<Option<RepoInfo>>` with package/area data
- `is_source_code_path()` exists in `blast_radius.rs`
- `handle_no_results()` already handles `--no-error` / `--on-error` in `commands.rs`
- Early-return dispatch pattern in `commands.rs` (used by `BlastRadius`, file-list commands)
- `darkmatter` + `biscuit-terminal` already wired in `sniff-cli/Cargo.toml`
- `chrono`, `git2`, `serde`, `thiserror` already in `sniff/lib/Cargo.toml`
- No new dependencies needed

## Step 1: Add `PeriodSpecifier` and `parse_period` to library

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs` (NEW)

- Create file with `PeriodSpecifier` enum
- Implement `parse_period(input: &str) -> Result<PeriodSpecifier>`
- Follow disambiguation order: keywords → ISO date → duration → hex hash → error
- Duration mapping: `h`/`d`/`w`/`mo`/`m`/`y` with long forms
- `m` maps to months (not minutes) per spec

**Confidence:** HIGH — pure string parsing, no external dependencies

## Step 2: Add `parse_commit_message` helper

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs`

- Private function `parse_commit_message(message: &str) -> (String, Vec<String>)`
- Split on `\n\n`, first paragraph = description, `- ` / `* ` lines = bullet_points
- Non-bullet body lines appended to description

**Confidence:** HIGH — pure string parsing

## Step 3: Add `CommitDesc` and `CommitDescSet` types

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs`

- `CommitDesc`: hash, datetime, packages, package_areas, files, description, bullet_points
- `CommitDescSet`: commits vec, period_label, repo_root
- Both derive `Debug, Clone, Serialize, Deserialize`

**Confidence:** HIGH — straightforward data types

## Step 4: Add `is_documentation_path` helper

**File:** `sniff/lib/src/filesystem/blast_radius.rs`

- New public function `is_documentation_path(path: &Path) -> bool`
- Check extensions: `.md`, `.mdx`, `.rst`, `.txt`, `.adoc`
- Also check file-type registry for `FileAssociation::Documentation`
- Follows same pattern as existing `is_source_code_path()`

**Confidence:** HIGH — mirrors existing `is_source_code_path`

## Step 5: Add `get_recent_commits_by_*` query functions

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs`

- Three public functions: `by_duration`, `by_date`, `by_hash`
- All call shared `collect_commits_since(repo, since, repo_info_opt)` helper
- Use `git2::Repository::discover(base_dir)` for repo open
- Revwalk from HEAD, compare commit timestamps to cutoff
- For each commit: call `get_commit_files()`, parse message, determine packages/areas
- Cache `detect_repo()` call once per query, pass through to all commits
- Hash variant: resolve hash → get commit timestamp → call shared walker

**Implementation detail:** Use `GitRepo` wrapper from `types.rs` which provides `open(path)` — check whether to use `GitRepo` or raw `git2::Repository`. The existing `detection.rs` functions accept `&Repository` so we should do the same internally, opening via `git2::Repository::discover()`.

**Confidence:** HIGH — `get_commit_files()` and `detect_repo()` already exist and are well-tested

## Step 6: Add rendering methods on `CommitDescSet`

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs`

- `describe(plain: bool) -> String` — Markdown, one section per commit
- `source_code_changes(plain: bool) -> String` — grouped by source file
- `documentation_changes(plain: bool) -> String` — grouped by doc file
- `describe_for_terminal(term: &Terminal, plain: bool) -> String` — darkmatter render
- `source_code_changes_for_terminal(term: &Terminal, plain: bool) -> String`
- `documentation_changes_for_terminal(term: &Terminal, plain: bool) -> String`
- Plain mode: skip hyperlink wrapping in Markdown
- Use `is_source_code_path()` and `is_documentation_path()` for classification

**Note:** The terminal methods should NOT live in the library (which doesn't depend on `darkmatter`). Instead, the library provides only the Markdown methods, and the CLI handles terminal rendering. This is a deviation from the tech design — the library should not depend on `darkmatter` or `biscuit-terminal`.

**Revised approach:**
- Library: `describe()`, `source_code_changes()`, `documentation_changes()` returning Markdown strings
- CLI: wraps with darkmatter for terminal output in `sniff/cli/src/output/recent_commits.rs`

**Confidence:** HIGH — follows existing separation of concerns

## Step 7: Add `InvalidPeriod` error variant

**File:** `sniff/lib/src/error.rs`

- Add `InvalidPeriod(String)` variant to `SniffError`

**Confidence:** HIGH — trivial addition following existing pattern

## Step 8: Update module visibility

**Files:**
- `sniff/lib/src/filesystem/git/mod.rs` — add `pub mod recent_commits` and re-exports
- `sniff/lib/src/filesystem/mod.rs` — add to `pub use git::` block

**Confidence:** HIGH — follows existing module pattern

## Step 9: Add CLI argument types

**File:** `sniff/cli/src/args.rs`

- Add three new `RepoSubcommand` variants: `RecentCommits`, `SourceCodeChanges`, `DocumentationChanges`
- Each with: `period: Option<String>`, `--package`, `--package-area`, `--no-error`, `--on-error`
- Add matching `RepoAction` variants
- Update `to_repo_action()` conversion method

**Confidence:** HIGH — follows exact pattern of existing variants like `BlastRadius`

## Step 10: Add CLI dispatch and output rendering

**Files:**
- `sniff/cli/src/commands.rs` — add early-return dispatch for the three new `RepoAction` variants
- `sniff/cli/src/output/recent_commits.rs` (NEW) — `handle_recent_commits_command()` function

**`handle_recent_commits_command` logic:**
1. Default period to `"3d"` if None
2. Call `parse_period()` to resolve specifier
3. Dispatch to appropriate library function
4. Apply `--package` / `--package-area` filtering on `CommitDescSet`
5. Handle empty results via `handle_no_results()`
6. Output: JSON serialization or terminal rendering via darkmatter

**Confidence:** HIGH — follows `BlastRadius` early-return pattern exactly

## Step 11: Unit tests — `parse_period` and `parse_commit_message`

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs` (inline `#[cfg(test)]`)

- `parse_period`: duration forms, keywords, ISO dates, hashes, invalid inputs
- `parse_commit_message`: simple, bullets, multi-paragraph, empty

**Confidence:** HIGH — pure function tests

## Step 12: Unit tests — `CommitDescSet` rendering

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs` (inline tests)

- `describe()` structure validation
- `source_code_changes()` grouping and filtering
- `documentation_changes()` grouping and filtering
- Plain mode hyperlink stripping

**Confidence:** HIGH — string output assertions

## Step 13: Integration tests — query functions

**File:** `sniff/lib/src/filesystem/git/recent_commits.rs` or separate test file

- Create temp git repo with known commits
- Test `by_duration`, `by_date`, `by_hash`
- Verify package/area attribution

**Confidence:** MEDIUM — requires temp git repos, but `tempfile` is already a dev-dependency

## Step 14: CLI integration tests

**File:** `sniff/cli/tests/` or inline

- `sniff repo recent-commits` default period
- `sniff repo recent-commits 1d`
- `sniff repo source-code-changes 1w`
- `sniff repo documentation-changes 1w`
- `--json`, `--plain`, `--package`, `--no-error` switches

**Confidence:** MEDIUM — requires test git repos, but `assert_cmd` + `tempfile` are existing dev-deps

## Risk Areas

1. **`m` ambiguity**: `3m` means months, not minutes. This matches the spec but could surprise users. Low risk since commit history at minute granularity isn't useful.

2. **Monorepo detection cost**: `detect_repo()` is expensive. Caching once per query is critical. The plan handles this.

3. **Terminal rendering placement**: The tech design puts `*_for_terminal` methods on `CommitDescSet` in the library, but the library doesn't depend on `darkmatter`. The plan corrects this: library produces Markdown, CLI renders to terminal.

4. **Large commit sets**: No pagination planned. For very long durations (e.g., `1y`), the output could be large. Acceptable for v1; can add limits later.

## Summary

| Step | Scope | Effort | Risk |
|------|-------|--------|------|
| 1 | `parse_period` + `PeriodSpecifier` | S | Low |
| 2 | `parse_commit_message` | S | Low |
| 3 | `CommitDesc` + `CommitDescSet` types | S | Low |
| 4 | `is_documentation_path` helper | S | Low |
| 5 | Query functions (`by_duration/date/hash`) | M | Low |
| 6 | Rendering methods (Markdown) | M | Low |
| 7 | `InvalidPeriod` error variant | XS | Low |
| 8 | Module visibility | XS | Low |
| 9 | CLI args (`RepoSubcommand` + `RepoAction`) | M | Low |
| 10 | CLI dispatch + output rendering | M | Low |
| 11 | Unit tests (parsing) | S | Low |
| 12 | Unit tests (rendering) | S | Low |
| 13 | Integration tests (query) | M | Medium |
| 14 | CLI integration tests | M | Medium |

**Estimated total:** ~14 files touched, 3 new files, no dependency changes.
