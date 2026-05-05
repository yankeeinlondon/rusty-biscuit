---
feature: incorrect-json
review: review-4.md
created: 2026-05-02
phases: 1
start_phase: 1
source_files_during_phase_1:
  - sniff/cli/src/output/filesystem.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
packages:
  - sniff-cli
---

# Implementation Plan — Address `review-4.md` Gaps

## Scope

`review-4.md` flags exactly **one** production-blocking bug:

- **High**: `sniff repo package-area-has-source-code-changes --json` reports
  `{ "has_source_code_changes": false }` and exit `1` even when source files in
  the current package area are dirty in the normal (non-deep) git request path.

The root cause is that `package_area_source_code_change_count`
(`sniff/cli/src/output/filesystem.rs:2155`) reads only `git.status.dirty` and
`git.status.untracked` — both of which are empty in the normal CLI path —
while its sibling helper `current_package_area_is_dirty` already chains in
`git.file_changes` (`sniff/cli/src/output/filesystem.rs:2089`) for exactly
this reason.

The fix is small (one helper change) and the bulk of the work is locking the
behavior down with CLI integration tests so this regression cannot recur.

## Working Assumptions

- Repo root: `/Users/ken/.claudine/worktrees/rusty-biscuit/sniff`
- Sniff lib crate: `sniff/lib/` (`-p sniff-lib`)
- Sniff CLI crate: `sniff/cli/` (`-p sniff-cli`)
- All `cargo` invocations MUST be targeted (`-p` flags). Never bare
  `cargo build` / `cargo test` at repo root.
- All shape decisions match `spec.md` and remain stable from prior review
  plans.
- Existing unit tests in `output::filesystem::tests::boolean_helpers` already
  cover the pure-helper "true" branch; the gap is end-to-end CLI coverage.

## Recommendations Addressed

| # | Source | Recommendation |
|---|--------|----------------|
| 1 | review-4 §High, fix bullet 1 | Update `package_area_source_code_change_count` to include `git.file_changes` paths, matching `current_package_area_is_dirty`. |
| 2 | review-4 §High, fix bullet 2 | Add a Level 1 CLI integration test that creates a dirty `src/lib.rs` in the current package area and asserts `has_source_code_changes: true` plus exit `0`. |
| 3 | review-4 §High, fix bullet 3 | Add a sibling Level 1 CLI integration test for a dirty docs-only file in the current area asserting source filtering still returns `false` plus exit `1`. |
| 4 | review-4 §Test Rigor Matrix | Promote the "Insufficient" matrix row for `package-area-has-source-code-changes --json` to "Appropriate" by closing the true-branch CLI gap. |
| 5 | Plan-level constraint | Sniff package area has zero clippy warnings under `-D warnings` and is `cargo fmt --check` clean. |

---

## Phase 1 — Fix `package_area_source_code_change_count` + lock with CLI tests

**Goal.** Make `package-area-has-source-code-changes --json` correct in the
normal (non-deep) git request path and prove it with CLI integration tests
that mirror the reproduction in the review.

**Files modified.**

- `sniff/cli/src/output/filesystem.rs` (helper update + matching unit test
  additions)
- `sniff/cli/tests/cli.rs` (two new CLI integration tests)

### 1A. Update `package_area_source_code_change_count` to read `git.file_changes`

**File:** `sniff/cli/src/output/filesystem.rs`, lines `2155`–`2192`.

The helper currently builds an iterator from
`git.status.dirty` chained with `git.status.untracked`. Mirror the structure
already used by `current_package_area_is_dirty` (lines `2101`–`2127`) by
chaining a third source: `git.file_changes`.

Replace the iterator construction inside
`package_area_source_code_change_count` with:

```rust
let count = git
    .status
    .dirty
    .iter()
    .map(|d| d.filepath.to_str().unwrap_or(""))
    .chain(
        git.status
            .untracked
            .iter()
            .map(|u| u.filepath.to_str().unwrap_or("")),
    )
    .chain(
        git.file_changes
            .iter()
            .map(|fc| fc.path.to_str().unwrap_or("")),
    )
    .filter(|path| {
        let in_area = if area_prefix.is_empty() {
            !repo.packages.as_ref().is_some_and(|pkgs| {
                pkgs.iter()
                    .any(|p| p.package_area != "root" && path.starts_with(&p.package_area))
            })
        } else {
            path.starts_with(area_prefix)
        };
        in_area && is_source_code_file(path)
    })
    .count();
```

Also update the rustdoc on `package_area_source_code_change_count` to match
the rationale paragraph already on `current_package_area_is_dirty` so future
readers see why all three sources are consulted. Use a `## Notes` section per
the repo's rustdoc convention. Suggested wording:

```rust
/// Pure helper: returns `(has_changes, count, area_name)` for the current
/// package area, or `None` when the area cannot be resolved.
///
/// JSON consumers only need the boolean. The text/verbose path uses the
/// count and area name to print a human-readable summary.
///
/// ## Notes
///
/// Pulls dirty paths from `git.file_changes` (always populated) plus the
/// diff-rich `git.status.dirty` / `git.status.untracked` arrays (deep mode
/// only). Without the `file_changes` source, non-deep callers would always
/// see a count of `0` because `status.dirty`/`status.untracked` are empty
/// unless `--refresh-remotes` is set. This mirrors the same chain used by
/// [`current_package_area_is_dirty`].
```

### 1B. Add unit test for the `file_changes`-only true branch

**File:** `sniff/cli/src/output/filesystem.rs`, inside the existing
`mod boolean_helpers` test module (around line `4092`).

The existing tests
(`package_area_source_code_change_count_counts_source_files_only`,
`package_area_source_code_change_count_false_when_only_docs_dirty`) populate
`git.status.dirty`. Add one more test that populates **only** `file_changes`
to lock the new behavior at the unit layer.

Use the same fixture pattern as the existing tests (look at
`package_area_source_code_change_count_counts_source_files_only` for the
`SniffResult` builder shape used in the surrounding tests). The new test
should:

1. Build a `SniffResult` whose `git.status.dirty` and `git.status.untracked`
   are empty.
2. Push a single `FileChange` into `git.file_changes` whose `path` lives
   inside the resolved package area and is a source-code file (e.g.
   `pkg-a/lib/src/lib.rs`).
3. Call `package_area_source_code_change_count(&result, Some(&area_dir))`.
4. Assert `Some((true, 1, _))`.

Add a sibling test that puts a docs-only file (e.g.
`pkg-a/lib/README.md`) in `file_changes` only, and asserts
`Some((false, 0, _))` — confirming the source-code filter still applies when
the path arrives via `file_changes`.

Test names (snake_case, matching the existing module style):

- `package_area_source_code_change_count_counts_file_changes_source_files`
- `package_area_source_code_change_count_ignores_file_changes_docs`

### 1C. Add CLI integration test: dirty source file → true + exit 0

**File:** `sniff/cli/tests/cli.rs`, append after
`test_package_area_has_source_code_changes_json_clean` (around line `3771`)
or grouped with
`test_is_current_package_area_dirty_json_true_branch` (around line `4234`).
Match the style of the latter — it is the closest analog and was added in
prior review work.

```rust
/// `package-area-has-source-code-changes --json` from inside a package area
/// whose source files are dirty must emit
/// `{ "has_source_code_changes": true }` and exit 0, even in the normal
/// (non-deep) git request path where `RepoStatus.dirty` is empty.
///
/// Regression test for review-4 High finding: the helper used to read only
/// `git.status.dirty` / `git.status.untracked` and missed dirty files
/// surfaced via `git.file_changes`.
#[test]
fn test_package_area_has_source_code_changes_json_true_branch() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a() {}");
    std::fs::write(path.join("pkg-a/lib/src/lib.rs"), "pub fn a() { dirty }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .success()
        .code(0);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["has_source_code_changes"],
        Value::Bool(true),
        "dirty source file in the area should emit has_source_code_changes: true, got: {value}"
    );
}
```

### 1D. Add CLI integration test: dirty docs-only file → false + exit 1

Sibling test confirming the source-code filter still works when paths arrive
via `file_changes` (so we do not regress in the opposite direction).

```rust
/// `package-area-has-source-code-changes --json` must remain `false` when
/// only documentation files are dirty in the current package area, even
/// though those paths are reported via `git.file_changes` in the normal
/// CLI path.
#[test]
fn test_package_area_has_source_code_changes_json_docs_only_is_false() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/README.md", "# pkg-a");
    std::fs::write(path.join("pkg-a/lib/README.md"), "# pkg-a (dirty)").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["has_source_code_changes"],
        Value::Bool(false),
        "docs-only dirty file must not flip has_source_code_changes, got: {value}"
    );
}
```

Notes for the implementer on test fixtures:

- `create_cli_monorepo()` and `test_commit_file(..)` are existing helpers in
  `sniff/cli/tests/cli.rs` — re-use them. Do not invent new fixture builders.
- The `pkg-a/lib/README.md` file may or may not exist in the default fixture.
  If it does, `test_commit_file` will overwrite then commit; if it does not,
  it will be created and committed. The pattern follows
  `test_is_current_package_area_dirty_json_true_branch`.
- If `create_cli_monorepo` does not seed `pkg-a/lib/README.md`, the
  `test_commit_file` call will create it cleanly. Adjust the path only if
  the fixture's package layout differs.

### 1E. Verification

Run from the worktree root
(`/Users/ken/.claudine/worktrees/rusty-biscuit/sniff`):

1. Targeted test for the regression bug fix:

   ```bash
   cargo test -p sniff-cli --test cli test_package_area_has_source_code_changes_json
   ```

   Expect three passing tests:
   `test_package_area_has_source_code_changes_json_clean` (existing),
   `test_package_area_has_source_code_changes_json_true_branch` (new),
   `test_package_area_has_source_code_changes_json_docs_only_is_false` (new).

2. Targeted unit tests for the helper:

   ```bash
   cargo test -p sniff-cli output::filesystem::tests::boolean_helpers
   ```

   Expect existing tests plus the two new
   `package_area_source_code_change_count_*_file_changes_*` tests to pass.

3. Manual reproduction from the review:

   ```bash
   tmp=$(mktemp -d)
   # build a two-package Cargo workspace with pkg-a/lib and pkg-b/lib,
   # commit it, then dirty pkg-a/lib/src/lib.rs (mirroring the review's
   # repro). Then:
   sniff --base "$tmp/pkg-a/lib" repo package-area-has-source-code-changes --json
   ```

   Expect: `{ "has_source_code_changes": true }` on stdout, exit `0`.

4. Full sniff package-area test suite:

   ```bash
   cargo test -p sniff-lib -p sniff-cli
   ```

   All tests must pass; no skips, no ignores added.

5. Lint gate (the implementer owns ALL clippy warnings in the sniff area,
   regardless of which code introduced them):

   ```bash
   cargo clippy -p sniff-lib -p sniff-cli --all-targets -- -D warnings
   ```

   Must complete with zero warnings.

6. Format gate:

   ```bash
   cargo fmt --check -p sniff-lib -p sniff-cli
   ```

   Must complete cleanly.

### 1F. Done criteria

- [ ] `package_area_source_code_change_count` chains `git.file_changes`
      paths and the rustdoc explains why.
- [ ] Two new unit tests in `boolean_helpers` cover the
      `file_changes`-only source and docs branches.
- [ ] Two new CLI integration tests cover the true (source dirty) and
      false-but-not-source (docs dirty) branches end-to-end.
- [ ] `test_package_area_has_source_code_changes_json_clean` still passes
      unchanged.
- [ ] Manual repro from review-4 returns
      `{ "has_source_code_changes": true }` with exit `0`.
- [ ] `cargo test -p sniff-lib -p sniff-cli` is green.
- [ ] `cargo clippy -p sniff-lib -p sniff-cli --all-targets -- -D warnings`
      is clean.
- [ ] `cargo fmt --check -p sniff-lib -p sniff-cli` is clean.

---

## Risk and Ambiguity Notes

1. **Fixture content for the docs-only test (1D).** The CLI test assumes
   `create_cli_monorepo()` produces a workspace where
   `pkg-a/lib/` is a valid package area into which `README.md` can be
   committed and then dirtied. If the existing fixture already commits a
   `README.md` at that path with different content, `test_commit_file` will
   simply overwrite-and-commit which is fine. If the implementer finds the
   fixture's layout differs, swap the path to any other docs file inside
   `pkg-a/lib/` (e.g. `pkg-a/lib/CHANGELOG.md`) — the assertion is about
   the source/docs classification, not the specific filename.

2. **`is_source_code_file` classification.** The fix relies on
   `sniff::filesystem::blast_radius::is_source_code_path` correctly
   classifying `.rs` as source and `.md` as docs. This is already the basis
   of the existing `boolean_helpers` unit tests, so no further audit is
   required, but the docs-only CLI test (1D) doubles as a guardrail.

3. **No spec changes required.** `spec.md` already documents the boolean
   shape (`{ "has_source_code_changes": bool }`) and the exit-code contract.
   The fix is a behavioral bug, not a contract change.

4. **No new clippy noise expected.** The change is a single `.chain(...)`
   addition mirroring an existing pattern; no new lifetimes, types, or
   public APIs are introduced. If clippy flags pre-existing warnings under
   `-D warnings`, the implementer must still resolve them per the plan
   constraint, even if unrelated to this fix.

---

## Summary

| Phase | Focus | Blocker? | Files |
|-------|-------|----------|-------|
| **1** | Fix `package_area_source_code_change_count` to read `git.file_changes`; add unit and CLI tests for true and docs-only branches; full sniff lint/fmt/test gates. | **Yes** | `sniff/cli/src/output/filesystem.rs`, `sniff/cli/tests/cli.rs` |

**Total: 1 phase.** The review identifies a single, narrowly scoped
production bug whose fix and verification fit cleanly inside one sequential
unit of work. Splitting further would only fragment the
fix-then-lock-with-tests cycle.
