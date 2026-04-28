# Review: Implicit Relative Path Feature

**Reviewer:** Claude Code (automated review)
**Date:** 2026-04-12
**Scope:** biscuit-file lib + CLI, comparing spec/tech-design to implementation

## Summary

The core feature is well-implemented and well-documented. Parsing, resolution, and the CWD-then-git-root fallback all match the spec and tech design. The topic doc (`docs/topics/file-references.md`) was updated comprehensively. Integration tests cover the primary scenarios. The issues below are about coverage gaps, a stale doc, and minor edge cases.

---

## 1. Stale Skill Reference Doc

**File:** `.claude/skills/biscuit-file/references/file-references.md` line 11

The quick-reference table still groups bare paths with `./` as a single **Relative** kind:

```
| _(none)_ or `./`     | **Relative** | Current working directory |
```

This no longer matches the implementation. Bare paths are `ImplicitRelative` (CWD + git root fallback), while `./` is `Relative` (CWD only). The topic doc at `docs/topics/file-references.md` was updated correctly — this reference doc was missed.

**Fix:** Split the row into two rows matching the topic doc's table (lines 23-27).

---

## 2. No CLI Integration Tests for Implicit Relative

**File:** `cli/tests/cli_tests.rs`

The CLI tests for the `reference` subcommand only exercise explicit `./` paths. There are no tests confirming `bf reference Cargo.toml` (bare, no prefix) resolves correctly, or that `bf reference` with a file only present at the git root succeeds. The tech design's verification plan calls out CLI-based manual verification, but automated coverage is missing.

**Suggested tests:**

- `bf reference Cargo.toml` (bare filename, should resolve from CWD) — confirms implicit relative works end-to-end through the CLI
- `bf reference CLAUDE.md` (file at repo root, not in `cli/` CWD) — confirms git-root fallback through CLI

---

## 3. No Integration Test Outside a Git Repo

**File:** `lib/tests/implicit_relative.rs`

All six integration tests create a temp git repo. There's no test for the scenario where implicit relative resolution happens outside any git repository (e.g., resolving `foo.md` in `/tmp/some-dir/` with no `.git` anywhere). The unit test in `resolve.rs` tests `collect_roots` with `/tmp` but doesn't exercise full resolution.

**Suggested test:** Create a temp dir (no `git_init`), place a file in it, resolve an implicit relative ref via `resolve_from` pointing at that dir. Confirm it resolves from CWD only.

---

## 4. No Test for `resolve_relative()` with Implicit Relative

**File:** `lib/tests/implicit_relative.rs`

All integration tests use `resolve()` or `resolve_from()`. The `resolve_relative()` method is not exercised with an implicit relative reference. This matters because `resolve_relative` has its own code path (calls `diff_paths`).

**Suggested test:** Resolve `"file_in_root.md"` via `resolve_relative(Some(&subdir))` and confirm the returned relative path is correct (e.g., `../../file_in_root.md` or similar).

---

## 5. No Test for Implicit Relative at Git Root (Dedup Check)

**File:** `lib/tests/implicit_relative.rs`

The `collect_roots` implementation has a deduplication guard:

```rust
if let Some(git_root) = find_git_root(&ctx.cwd)?
    && git_root != ctx.cwd
{
    roots.push(git_root);
}
```

No test confirms this works correctly when CWD *is* the git root. In that case only one root should be produced and the file should still resolve.

**Suggested test:** Set up a git repo, place a file at the root, resolve from the root itself. Confirm it resolves (and doesn't check the same root twice).

---

## 6. No Test for Recursive Implicit Relative with Subdir Filter

**File:** `lib/tests/implicit_relative.rs`

The existing recursive test (`recursive_implicit_relative_finds_file_under_git_root`) uses `%deep.md` — a bare filename. The tech design mentions `%file_deep_in_repo.md` similarly. But there's no test for recursive search with a subdir path like `%docs/spec.md`, which exercises the `subdir_filter` branch in `resolve_recursive`.

**Suggested test:** Create `a/docs/spec.md` under the git root. From a sibling CWD, resolve `%docs/spec.md`. Confirm it finds the file and that the parent path constraint is enforced.

---

## 7. Edge Case: `%` Alone is Accepted

**File:** `lib/src/file_reference/parse.rs`

`FileReference::new("%")` succeeds — `strip_recursive` produces `(true, "")`, which becomes `ImplicitRelative` with an empty template. It resolves to `Ok(None)` (directories fail `is_file()`), but arguably this should be rejected at parse time like `""` is.

**Severity:** Low — no crash or wrong behavior, just a confusing silent no-op.

**Suggested fix:** After stripping the `%` prefix, check if the remainder is empty and return `InvalidSyntax`.

---

## 8. Env-Var Interpolation Not Tested for ImplicitRelative Resolution

**File:** `lib/src/file_reference/parse.rs`, `lib/tests/implicit_relative.rs`

The parse test `interpolation_single_var` confirms `{{DIR}}/foo.md` parses as `ImplicitRelative`. But there's no resolution test confirming that `{{DIR}}/foo.md` actually resolves correctly when `DIR` is set in the environment — specifically that it resolves against both CWD and git root with the interpolated value.

**Severity:** Low — the interpolation and resolution are independent code paths so this is unlikely to break, but it exercises the full pipeline.

---

## 9. Performance Note: Git Root Discovery

**File:** `lib/src/file_reference/context.rs`

`find_git_root` opens a `git2::Repository` via `discover()` on every call. Within a single `resolve()` call for `ImplicitRelative`, this is called once — fine. But if a caller resolves many `ImplicitRelative` references in sequence (e.g., batch-resolving a list of bare paths), each call rediscovers the repo.

**Not a bug** — this matches the existing behavior for `Magic` and `Package` kinds. But if batch resolution becomes a use case, caching the git root in `ResolutionContext` (or a shared resolver struct) would avoid redundant discovery. No action needed now.

---

## Items Verified as Correct

- Parsing: bare paths → `ImplicitRelative`, `./` paths → `Relative` (4 unit tests)
- Resolution priority: CWD checked before git root (integration test)
- Explicit `./` does NOT fall back to git root (integration test)
- Subdir paths resolve against git root (integration test)
- `Ok(None)` when file missing from both roots (integration test)
- Recursive search includes git root as traversal root (integration test)
- `find_git_root` handles bare repos, missing repos gracefully
- `collect_roots` dedup guard exists (git_root != cwd)
- Topic doc updated with new kind, quick reference table, sections, algorithm table
- No changes to public API surface (`ReferenceKind` remains `pub(crate)`)
- Error types unchanged — implicit relative reuses existing error variants correctly
