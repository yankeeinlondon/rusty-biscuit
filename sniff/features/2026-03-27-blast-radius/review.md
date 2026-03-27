# Blast Radius Review

Review scope:
- [spec.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/features/2026-03-27-blast-radius/spec.md)
- [tech-design.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/features/2026-03-27-blast-radius/tech-design.md)
- current implementation in `sniff/lib` and `sniff/cli`

Validation performed:
- Code review of parser, command wiring, library matching, and renderers
- `just test` from [/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff), which passed

## Findings

### 1. `sniff repo staged-files` still uses the legacy implementation, so most of the designed behavior is not actually implemented

Severity: High

Files:
- [commands.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/commands.rs#L290)
- [commands.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/commands.rs#L336)
- [commands.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/commands.rs#L345)

`RepoSubcommand::StagedFiles` was upgraded to parse `FileListArgs`, but command dispatch still routes it through the old `git.file_changes` path instead of `handle_file_list_command()`. That means `staged-files` does not honor the new feature surface:

- `--package` and `--package-area` are parsed but ignored
- `--list`, `--csv`, and `--no-path` are parsed but ignored
- `--no-error` and `--on-error` are parsed but ignored
- output still uses `render_git_file_list()` rather than the shared OSC8 path-list renderer
- JSON output is still raw `FileChange` objects instead of the new `{ scope, kind, paths }` shape

This is the clearest spec/design miss in the implementation.

Recommendation:
- Route `RepoAction::StagedFiles(args)` through `handle_file_list_command(args, ChangeScope::Staged, ChangedPathKind::AllFiles, ...)`
- Remove the duplicated staged-files branch once the shared path-list handler covers it

### 2. `--package-area` scoping is implemented as exact equality, not area-prefix scoping

Severity: High

File:
- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L145)

The tech design says package-area scoping should map to package roots that share the same area prefix. The current filter only matches `pkg.package_area.eq_ignore_ascii_case(area)`. That will miss nested areas when the user scopes to a parent area.

Example of the current behavior:
- `--package-area foo` matches packages in `foo`
- it does not match packages in `foo/bar`

Recommendation:
- Change package-area matching to prefix semantics on normalized area paths
- Add unit tests that cover both exact area matches and nested-area matches

### 3. `blast_radius` entries are not normalized before matching, so valid documents can be missed

Severity: Medium

Files:
- [docs.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/docs.rs#L181)
- [docs.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/docs.rs#L267)
- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L268)

The design calls for normalizing `blast_radius` paths to repo-relative paths when possible. The implementation currently stores them exactly as written in frontmatter and then does exact `PathBuf` intersection against repo-relative changed paths.

That means these documents will silently fail to match even when they refer to the same file:
- `./sniff/lib/src/filesystem/git.rs`
- `/absolute/path/to/repo/sniff/lib/src/filesystem/git.rs`
- any equivalent path using `.` segments

Recommendation:
- Normalize `blast_radius` entries during parse or immediately before matching
- Store normalized repo-relative paths in `MarkdownMeta`
- Add unit tests for `./`, absolute-path, and already-relative cases

### 4. The docs verbose renderer does not fully match the designed provenance output when title resolution falls back to “none”

Severity: Medium

File:
- [filesystem.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/output/filesystem.rs#L2652)

The spec/design describes a provenance line for title metadata, including the “none” case. The current renderer drops provenance entirely when `TitleSource::None` is used and renders just:

- `title: none`

instead of rendering the provenance consistently.

This is smaller than the staged-files gap, but it is still a spec mismatch in user-visible output.

Recommendation:
- Render a consistent `(<dim><i>from ...</i></dim>)` suffix for the `None` case too
- Add a renderer unit test for each title provenance variant

### 5. User-facing documentation for the new commands and flag changes was not updated

Severity: Medium

Files checked:
- [sniff/cli/README.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/README.md)
- [sniff/lib/README.md](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/README.md)
- [/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/docs/cli](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/docs/cli)

I found no references in those docs to:
- `sniff repo dirty-source-code`
- `sniff repo staged-source-code`
- `sniff repo unstaged-source-code`
- `sniff repo dirty-files`
- `sniff blast-radius`
- `sniff docs --blast-radius`
- the updated docs footer wording / verbose metadata layout

The tech design explicitly called out these documentation updates. This is drift rather than a runtime bug, but it is still an implementation gap.

Recommendation:
- Update the CLI README and `sniff/docs/cli/*` pages in the same change set

## Test Coverage Gaps

### 1. There are no CLI integration tests for any of the new blast-radius workflows

Severity: High

Files checked:
- [/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/cli.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/cli.rs)
- [/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/snapshots.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/tests/snapshots.rs)

There are parser tests in [args.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/args.rs#L1802), but there are no end-to-end CLI tests covering:

- `sniff repo dirty-source-code`
- `sniff repo staged-source-code`
- `sniff repo unstaged-source-code`
- `sniff repo dirty-files`
- upgraded `sniff repo staged-files`
- `sniff blast-radius dirty|staged|last-commit`
- `--no-error` exit handling
- `--on-error` stream placement
- `sniff docs` stdout/stderr split
- `sniff docs --blast-radius`

The only CLI snapshot related to this feature is that help text mentions `blast-radius`.

Recommendation:
- Add temp-repo based CLI integration tests that stage, modify, and untrack files explicitly
- Add stream-aware tests for stdout vs stderr behavior

### 2. The core matching function `find_blast_radius_documents()` has no direct tests

Severity: High

File:
- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L251)

The plan called for tests covering:
- matched document
- unmatched document
- empty `blast_radius`
- no changed files
- scope variants

Those tests do not exist. The current unit tests stop at `collect_changed_paths()`.

Recommendation:
- Add isolated tests that build a temp repo plus temp docs and assert exact matching behavior

### 3. `collect_changed_paths()` tests are shallow and rely on the live repo state

Severity: Medium

File:
- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L368)

Current tests only verify:
- the dirty query returns something sortable in this repo
- source-code filtering excludes non-source files
- substring filtering works
- `LastCommit` returns `Ok`
- `/tmp` is not a repo

They do not prove:
- staged scope semantics
- unstaged scope semantics
- deleted-file handling
- package scoping
- package-area scoping
- nested package-area behavior

Because they depend on whatever is currently dirty in this worktree, they also do not pin behavior very tightly.

Recommendation:
- Move these tests to temp-repo fixtures with explicit staged/unstaged/untracked/deleted files

### 4. The new renderers do not have direct unit or snapshot coverage

Severity: Medium

Files:
- [filesystem.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/output/filesystem.rs#L2620)
- [filesystem.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/output/filesystem.rs#L2764)

I did not find tests for:
- `render_docs_output()`
- `render_path_list()`
- basename-only output via `--no-path`
- the updated docs footer text
- docs verbose nested metadata layout

Recommendation:
- Add renderer unit tests for structural behavior
- Add snapshots for styled path output, `--no-path`, and verbose docs output

## Ergonomics / Performance Recommendations

### 1. Remove the duplicated changed-file implementation paths

The code now has two competing implementations for repo file listing:
- legacy git-file-change rendering
- new shared path-list rendering

That duplication already caused the staged-files behavior drift. Reusing one shared path-list code path for all compatible commands will make the CLI more ergonomic to maintain and less likely to regress.

### 2. Short-circuit `find_blast_radius_documents()` when there are no changed source files

File:
- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L268)

If `changed_set` is empty, the function can return `Ok(Vec::new())` immediately instead of walking the repo and parsing every Markdown file. That will make the “no changed source files” case cheaper, which is likely common.

### 3. Precompute lowercased substring filters once in `collect_changed_paths()`

File:
- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L169)

The current substring filter lowercases every filter string for every path being checked. Pre-lowercasing the filters once would slightly reduce allocation and simplify the closure.

### 4. Validate `--package` / `--package-area` names explicitly instead of silently collapsing to “no results”

Files:
- [blast_radius.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/lib/src/filesystem/blast_radius.rs#L145)
- [commands.rs](/Users/ken/.claudine/worktrees/feat-sniff-tuning/sniff/cli/src/commands.rs#L824)

If the user misspells a package or area, the current code retains no matching roots and then falls into generic no-result behavior. That is ambiguous: it looks the same as “there were no matching changed files”.

There is already a more helpful validation pattern in `resolve_package_path()`. Generalizing that behavior for the new commands would make the CLI much more ergonomic.

## Summary

The underlying library split is mostly in place, and the test suite still passes, but the implementation is not feature-complete relative to the spec/design yet. The largest functional miss is that `sniff repo staged-files` was only upgraded at the parser layer. The biggest quality risk is test coverage: none of the new end-to-end workflows are actually exercised in CLI integration tests, and the core blast-radius document matcher has no direct tests at all.
