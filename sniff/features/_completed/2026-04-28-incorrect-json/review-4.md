---
ready: false
agent: ""
model: ""
---

# Review 4: Incorrect JSON Implementation

## Summary

The implementation now covers most of the specification well: repo-action-aware JSON routing exists, focused shapes are emitted for the package, locator, dependency, git-status, boolean, and filtered commit families, and there is meaningful Level 1 coverage through unit tests and CLI integration tests.

One production-blocking gap remains in the `package-area-has-source-code-changes` boolean command. It reports `false` in normal non-deep CLI execution even when a source file in the current package area is dirty.

## Findings

### High: `package-area-has-source-code-changes --json` misses normal dirty source files

- Requirement: `package-area-has-source-code-changes --json` must return `{ "has_source_code_changes": true }` and exit `0` when the current package area has source-code changes.
- Location: `sniff/cli/src/output/filesystem.rs:2155`
- Verification level present: Level 1 pure helper tests for the true branch, plus Level 1 CLI integration only for the false/clean branch.
- Required verification level: Level 1 full CLI integration is sufficient for this CLI JSON/exit-code behavior.
- Status: gap.

`current_package_area_is_dirty` explicitly chains `git.file_changes` with `git.status.dirty` and `git.status.untracked` because the status lists are empty in the normal non-deep git request path (`sniff/cli/src/output/filesystem.rs:2084`). `package_area_source_code_change_count` still only reads `git.status.dirty` and `git.status.untracked` (`sniff/cli/src/output/filesystem.rs:2167`), so it returns zero for ordinary unstaged source edits detected through `git.file_changes`.

I reproduced this with a temporary two-package Cargo workspace:

```text
sniff --base "$tmp/pkg-a/lib" repo package-area-has-source-code-changes --json
{
  "has_source_code_changes": false
}
exit=1
```

The fixture had an unstaged edit to `pkg-a/lib/src/lib.rs`, so the expected output is `{"has_source_code_changes": true}` with exit code `0`.

The test suite hints at the missing full-command branch: `test_package_area_has_source_code_changes_json_clean` covers only false/exit-1 (`sniff/cli/tests/cli.rs:3744`), while a true-branch CLI test was added for `is-current-package-area-dirty` (`sniff/cli/tests/cli.rs:4234`) but not for `package-area-has-source-code-changes`.

Recommended fix:

- Update `package_area_source_code_change_count` to include `git.file_changes` paths, matching `current_package_area_is_dirty`.
- Add a Level 1 CLI integration test that creates a dirty `src/lib.rs` in the current package area and asserts `has_source_code_changes: true` plus exit `0`.
- Add a sibling test for a dirty docs-only file in the current area to ensure source filtering still returns false.

## Test Rigor Matrix

Level 2 and Level 3 terminal tests are not required for this feature because the user-observable contract is JSON stdout and process exit status, not terminal rendering, keyboard input, paste/IME, mouse, or scroll behavior.

| Requirement | Strongest verification observed | Assessment |
|---|---:|---|
| `git-status --json` emits focused `GitInfo` | Level 1 CLI + unit | Appropriate |
| `deps --json` emits narrow `{ packages }` graph | Level 1 CLI + unit | Appropriate |
| dirty/staged/unstaged package and area commands emit `{ scope, kind, names }` | Level 1 CLI + unit | Appropriate |
| locator commands emit `{ root }` / `{ name }` | Level 1 CLI + unit | Appropriate |
| `is-current-package-area-dirty --json` preserves true/false exit semantics | Level 1 CLI + unit | Appropriate |
| `package-area-has-source-code-changes --json` preserves true/false exit semantics | Level 1 unit true branch, Level 1 CLI false branch only | Insufficient and currently broken |
| `has-merge-conflict --json` emits boolean and exit code | Level 1 CLI + unit | Appropriate |
| `source-code-changes --json` filters commits and files | Level 1 CLI + unit | Appropriate |
| `documentation-changes --json` filters commits and files | Level 1 CLI + unit | Appropriate |
| `--perf --json` remains parseable with new shapes | Level 1 CLI | Appropriate |

## Verification Run

Passing focused checks:

- `cargo test -p sniff-cli repo_json`
- `cargo test -p sniff-cli source_code_changes`
- `cargo test -p sniff-cli git_status_json`

Additional manual check failed as described above:

- `sniff --base "$tmp/pkg-a/lib" repo package-area-has-source-code-changes --json`

## Production Readiness

Not ready. The remaining bug is in a specified user-facing boolean command and affects both JSON payload and exit-code semantics in the normal CLI path.
