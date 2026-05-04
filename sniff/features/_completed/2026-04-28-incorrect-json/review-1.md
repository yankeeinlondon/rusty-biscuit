---
ready: false
agent: ""
model: ""
---

# Feature Review: Correct `sniff repo --json` Output

Review of the implementation of the `incorrect-json` feature as defined in the [specification](./spec.md) and [execution plan](./plan.md).

## Summary

The feature is currently **NOT READY** for production. While the documentation (specification and plan) is excellent and clearly identifies the issues, the actual implementation is largely missing from the codebase. Most `sniff repo` subcommands still return the unfiltered `RepoInfo` blob or simple arrays, failing the contract defined in the specification.

## Gaps in Functionality

### 1. Missing Repo-Action-Aware JSON Routing
The core of the plan (Phase 2) involves making `output::print_json` aware of the `RepoAction`. This has not been implemented.
- `sniff/cli/src/output/mod.rs`: `print_json` and `apply_filter_to_json` still lack a `repo_action` parameter.
- `OutputFilter::Repo` continues to return `fs.repo` (the full `RepoInfo` blob) regardless of the specific subcommand invoked.

### 2. Incomplete Shared JSON Builders
Phase 3 of the plan intended to add reusable structured output functions.
- **Missing:** JSON builders for `packages`, `package_areas`, `deps`, and `git-status`.
- **Partial:** `handle_file_list_command` in `sniff/cli/src/commands.rs` implements the `{ "scope": "...", "kind": "...", "paths": [...] }` pattern for file-family commands, but this is an isolated implementation rather than a shared builder.

### 3. Broken `git-status` JSON
The specification requires `sniff repo git-status --json` to return a `GitInfo` object.
- **Current Behavior:** It returns the full `RepoInfo` blob (which contains `GitInfo` but also many other fields).
- **Impact:** Breaks the contract that `--json` should mirror the text-mode focus.

### 4. Broken Package-Family JSON
Subcommands like `dirty-packages`, `staged-package-areas`, etc., should return `{ "scope": "...", "kind": "...", "names": [...] }`.
- **Current Behavior:** They return the full `RepoInfo` blob.

## Broken or Incomplete Features

- **`sniff repo packages --json`**: Still returns a simple array `["pkg1", "pkg2"]` instead of the specified `{ "names": ["pkg1", "pkg2"] }`.
- **`sniff repo deps --json`**: Returns the full `RepoInfo` blob instead of the focused dependency object.
- **Recent Commits / Source Changes**: These subcommands do not yet respect the `RecentCommitsMode` for JSON output, returning full commit sets instead of filtered ones.

## Test Coverage

- **Zero new test coverage:** No new tests were found in `sniff/cli/tests/cli.rs` or elsewhere that verify the new JSON shapes.
- **Outdated tests:** Existing tests (e.g., `test_git_status_subcommand_json_output`) still assert the old "full blob" behavior, which now contradicts the specification.

## Recommendations for Improvement

### Ergonomics
- **Centralize JSON logic:** Instead of subcommands like `handle_file_list_command` building their own JSON, move this logic into `sniff/cli/src/output/filesystem.rs` as reusable functions that return `serde_json::Value`.
- **Refactor `print_json`**: Update the signature to `print_json(result, filter, docs_filter, files_filter, repo_action)` to allow contextual filtering.

### Performance
- **Avoid unnecessary serialization:** The current fallback to `serde_json::to_value(result)` in `apply_filter_to_json` is expensive for large repositories if only a small subset of data is needed. Centralizing the builders will allow for more targeted serialization.

## Final Verdict

**Ready for Production:** No.

The implementation is in its very early stages, with only the file-family commands showing signs of the new format. The majority of the repo-specific subcommands still exhibit the "Incorrect" behavior described in the specification.
