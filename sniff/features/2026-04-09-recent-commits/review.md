# Review

## Findings

1. High: `today` and `yesterday` are both implemented as the same trailing 24 hour window, so both keywords return the wrong commit set.

   Evidence:
   `parse_period()` distinguishes `Today` and `Yesterday`, but the CLI maps both variants to `get_recent_commits_by_duration(dir, Duration::days(1), ...)` with no day-boundary logic at all in [sniff/cli/src/output/recent_commits.rs:33](../../cli/src/output/recent_commits.rs#L33) and [sniff/cli/src/output/recent_commits.rs:41](../../cli/src/output/recent_commits.rs#L41). That means:
   `today` includes late commits from the previous day, and `yesterday` includes commits from today while excluding older commits from yesterday that fall outside the trailing 24 hours.

   Why this matters:
   The spec explicitly calls out `today` and `yesterday` as supported period specifiers, which implies calendar-day semantics rather than "last 24 hours".

   Suggested fix:
   Resolve `today` and `yesterday` to explicit `[start, end)` boundaries and query against those boundaries instead of reusing the duration path.

2. High: `--package` and `--package-area` filtering only removes whole commits; it does not narrow the files within surviving commits, so cross-package commits leak unrelated paths.

   Evidence:
   `filter_by_package()` and `filter_by_package_area()` retain or drop commits based on commit-level metadata only, and they never rewrite `commit.files` in [sniff/lib/src/filesystem/git/recent_commits.rs:467](../../lib/src/filesystem/git/recent_commits.rs#L467) and [sniff/lib/src/filesystem/git/recent_commits.rs:483](../../lib/src/filesystem/git/recent_commits.rs#L483). Rendering then prints `commit.files` directly in [sniff/lib/src/filesystem/git/recent_commits.rs:355](../../lib/src/filesystem/git/recent_commits.rs#L355) and groups files directly in [sniff/lib/src/filesystem/git/recent_commits.rs:398](../../lib/src/filesystem/git/recent_commits.rs#L398).

   Why this matters:
   In a monorepo commit that touches package `A` and package `B`, `sniff repo recent-commits --package A` still renders `B`'s files. The same leak affects `source-code-changes`, `documentation-changes`, and the JSON payload. That does not satisfy the spec's requirement to "narrow results to just the specified package" or package area.

   Suggested fix:
   Filter files by package/package-area roots first, then recompute `packages` and `package_areas` from the surviving file set before rendering or serializing.

3. Medium: package-area matching is inconsistent with the rest of `sniff repo`, and package attribution uses raw string-prefix matching instead of path-aware matching.

   Evidence:
   Existing repo-scoped file commands use prefix semantics for package areas, so `--package-area foo` matches both `foo` and `foo/bar` in [sniff/lib/src/filesystem/blast_radius.rs:178](../../lib/src/filesystem/blast_radius.rs#L178). The recent-commits filter is exact-only in [sniff/lib/src/filesystem/git/recent_commits.rs:483](../../lib/src/filesystem/git/recent_commits.rs#L483). Separately, commit attribution uses `path_str.starts_with(pkg.relative.as_str())` in [sniff/lib/src/filesystem/git/recent_commits.rs:307](../../lib/src/filesystem/git/recent_commits.rs#L307) even though package roots are real paths, not string prefixes.

   Why this matters:
   `--package-area apps` will miss commits attributed to `apps/browser`, which is a regression from existing repo behavior. The raw `starts_with` check can also misattribute `foo-bar/...` to package root `foo`.

   Suggested fix:
   Keep package-area matching consistent with the existing prefix semantics, and switch attribution/filtering to `Path`-aware prefix checks instead of string-prefix checks.

4. Medium: `documentation_changes()` misses registry-classified documentation files that do not have one of the hard-coded extensions.

   Evidence:
   `is_documentation_path()` only checks a small extension allowlist and `lookup_extension()` in [sniff/lib/src/filesystem/blast_radius.rs:71](../../lib/src/filesystem/blast_radius.rs#L71). The file-type registry already classifies exact filenames such as `README`, `CHANGELOG`, and `CONTRIBUTING` as `Documentation` in [sniff/lib/src/filesystem/file_types/registry.rs:92](../../lib/src/filesystem/file_types/registry.rs#L92), but that path is never consulted here.

   Why this matters:
   A commit that changes a bare `README` or `CHANGELOG` will be omitted from `sniff repo documentation-changes`, even though the tech design explicitly says documentation detection should also use the file-type registry's documentation classification.

   Suggested fix:
   Mirror `is_source_code_path()` and check `lookup_exact_filename()` before falling back to extension-based classification.

5. Medium: the tests for this feature are largely happy-path smoke tests and do not exercise the behaviors most likely to break.

   Evidence:
   The library tests validate parsing and rendering shape, and the integration tests mostly assert presence/absence on single-commit repos in [sniff/lib/tests/integration.rs:1088](../../lib/tests/integration.rs#L1088). The CLI tests mostly assert exit success or that JSON contains `"commits"` in [sniff/cli/tests/cli.rs:1780](../../cli/tests/cli.rs#L1780). The only multi-commit helper, `create_multi_commit_repo()`, is unused in [sniff/lib/tests/integration.rs:1048](../../lib/tests/integration.rs#L1048).

   Why this matters:
   The current suite goes green while missing the calendar-boundary bug, the cross-package leakage bug, package-area prefix semantics, and the bare-README documentation case.

   Suggested fix:
   Add temp-repo tests with controlled commit timestamps for `today`/`yesterday`, a monorepo fixture with a cross-package commit, package-area prefix cases like `apps` vs `apps/browser`, and a docs-only commit on a bare `README`.

## Verification

- `cargo test -p sniff --lib recent_commits -- --nocapture`
- `cargo test -p sniff --test integration recent_commits -- --nocapture`
