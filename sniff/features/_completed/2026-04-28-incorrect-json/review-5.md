---
ready: false
agent: "codex"
model: ""
---

# Review 5: Incorrect JSON Implementation

## Summary

The review-4 blocker for `package-area-has-source-code-changes --json` has been fixed. The helper now reads `git.file_changes`, and Level 1 CLI regression tests cover the dirty source true branch, clean false branch, and docs-only false branch.

One production-blocking contract mismatch remains: the implementation changes the two subcommands the spec explicitly classified as already-correct (`packages` and `package-areas`) from JSON arrays into `{ "names": [...] }` objects. That is a user-facing JSON shape regression, and the current tests now assert the wrong shape.

## Findings

### High: `repo packages --json` and `repo package-areas --json` no longer match the specified array contract

- Requirement: `packages` and `package-areas` were listed as already-correct and should remain JSON arrays of strings (`["pkg-a", "pkg-b", ...]`).
- Spec reference: `sniff/features/2026-04-28-incorrect-json/spec.md:20` and `sniff/features/2026-04-28-incorrect-json/spec.md:21`.
- Implementation: `sniff/cli/src/commands.rs:1428` wraps packages in `{ "names": names }`; `sniff/cli/src/commands.rs:1490` does the same for package areas.
- Test coverage: `sniff/cli/tests/cli.rs:3070` and `sniff/cli/tests/cli.rs:3301` assert `json["names"]`, so Level 1 coverage exists but verifies a shape that conflicts with the spec.
- Required verification level: Level 1 CLI integration is sufficient for this JSON stdout contract, but the assertions need to match the specified array contract.
- Status: gap.

Manual check in this worktree confirms the object shape:

```text
target/debug/sniff --base . repo packages --json
{"names":["agent-sandbox-cli", ...]}

target/debug/sniff --base . repo package-areas --json
{"names":["agent-sandbox", ...]}
```

The feature goal was to stop broken repo subcommands from falling through to full `RepoInfo`, while preserving commands already called out as correct. Changing these two shapes will break scripts that consumed the documented/previous array output.

Recommended fix:

- Change `handle_repo_packages` and `handle_repo_package_areas` JSON branches back to serializing the names vector directly.
- Update `test_repo_packages_json_output`, `test_repo_package_areas_json_output`, and `test_repo_package_areas_json_perf_stdout_is_valid_json` to assert `json.is_array()` and compare the array values directly.
- Keep the `{ scope, kind, names }` object shape only for the dirty/staged/unstaged package and package-area families, where the spec explicitly requires that object.

## Test Rigor Matrix

Level 2 and Level 3 terminal tests are not required for this feature. The user-observable behavior is JSON stdout plus process exit status; there are no requirements around terminal rendering, keyboard input, paste/IME, mouse, or scroll behavior.

| Requirement | Strongest verification observed | Assessment |
|---|---:|---|
| `git-status --json` emits focused `GitInfo` | Level 1 CLI + unit | Appropriate |
| `deps --json` emits narrow `{ packages }` graph | Level 1 CLI + unit | Appropriate |
| dirty/staged/unstaged package and area commands emit `{ scope, kind, names }` | Level 1 CLI + unit | Appropriate |
| `packages --json` remains an array of strings | Level 1 CLI, but asserts `{ names }` | Broken contract |
| `package-areas --json` remains an array of strings | Level 1 CLI, but asserts `{ names }` | Broken contract |
| locator commands emit `{ root }` / `{ name }` | Level 1 CLI + unit | Appropriate |
| `is-current-package-area-dirty --json` preserves true/false exit semantics | Level 1 CLI + unit | Appropriate |
| `package-area-has-source-code-changes --json` preserves true/false exit semantics | Level 1 CLI + unit | Appropriate after review-4 fix |
| `has-merge-conflict --json` emits boolean and exit code | Level 1 CLI + unit | Appropriate |
| `source-code-changes --json` filters commits and files | Level 1 CLI + unit | Appropriate |
| `documentation-changes --json` filters commits and files | Level 1 CLI + unit | Appropriate |
| `--perf --json` remains parseable with new shapes | Level 1 CLI | Appropriate |

## Verification Run

Passing checks:

- `cargo test -p sniff-cli repo_json`
- `cargo test -p sniff-cli package_area_has_source_code_changes_json`
- `cargo test -p sniff-cli test_repo_subcommand_json_shapes_are_distinct`
- `cargo test -p sniff-cli source_code_changes`
- `cargo test -p sniff-cli documentation_changes`
- `cargo test -p sniff-cli git_status_json`

Manual contract checks:

- `target/debug/sniff --base . repo packages --json` currently emits `{ "names": [...] }`, but the spec requires an array.
- `target/debug/sniff --base . repo package-areas --json` currently emits `{ "names": [...] }`, but the spec requires an array.

## Production Readiness

Not ready. The review-4 blocker is resolved, but two already-correct public JSON contracts now have the wrong shape and are covered by tests that encode the mismatch.
