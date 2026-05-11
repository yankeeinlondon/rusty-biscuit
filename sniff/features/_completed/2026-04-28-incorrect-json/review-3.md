---
ready: true
agent: ""
model: ""
---

# Review 3: Incorrect JSON Feature Implementation

This review evaluates the implementation of the "Incorrect JSON" feature (2026-04-28), which aimed to ensure that every `sniff repo` subcommand returns tailored, filtered JSON instead of the full `RepoInfo` blob.

## Summary of Findings

The implementation is comprehensive and aligns well with the architectural goals. The use of a centralized dispatching seam in `repo_json.rs` and the introduction of the `BuildOutcome` pattern are strong improvements.

### 1. Functional Completeness
The implementation successfully addresses the core problem. All 18 subcommands mentioned in the specification now emit focused JSON instead of the full `RepoInfo` blob.

- **Structure family:** Now honors the `--filter` flag in JSON mode.
- **Git family:** `git-status` correctly serializes `GitInfo` directly.
- **Dependency family:** `deps` returns a hand-built, narrow dependency graph, avoiding data leaks from the internal `Package` struct.
- **Locator family:** `package-root`, `package-area-root`, `package`, and `package-area` return focused `{ root }` or `{ name }` objects.
- **Boolean family:** `is-current-package-area-dirty`, `package-area-has-source-code-changes`, and `has-merge-conflict` return descriptive boolean objects and maintain correct exit-code behavior.
- **Commit family:** `source-code-changes` and `documentation-changes` correctly apply filters and include the `"filter"` tag in their JSON output.

### 2. Implementation Quality & Ergonomics
- **Dispatch Pattern:** The routing in `repo_json.rs` is clean and extensible.
- **Exit Code Semantics:** The `BuildOutcome` struct effectively mirrors text-mode exit codes in JSON mode, which is critical for scriptable probes.
- **Early Returns:** High-performance subcommands correctly bypass the heavy detection pass where possible.

### 3. Gaps & Inconsistencies

#### JSON Shape Inconsistency (Packages Family)
There is a minor inconsistency between "already correct" subcommands and the new lifecycle-scoped ones:
- `sniff repo packages` returns a plain array: `["pkg-a", "pkg-b"]`.
- `sniff repo dirty-packages` returns a structured object: `{ "scope": "dirty", "kind": "packages", "names": ["pkg-a", ...] }`.
While the object shape is more future-proof, standardizing the "all packages" list to match this shape would improve consistency.

#### File-Listing Inconsistency (Legacy Paths)
`unstaged-files` and `untracked-files` are handled by legacy code paths in `commands.rs`. They return a raw `Vec<FileChange>` (array of objects with `path`, `status`, `action`, etc.). In contrast, `staged-files` and `dirty-files` return the `{ scope, kind, paths }` object. Unifying these would improve the predictability of the CLI's JSON contract.

#### Commit Family Verbosity
The `CommitDescSet` used by the commit family includes the full `packages` list (the same one used by `deps`). While the commits themselves are correctly filtered, the JSON output is significantly larger than the "Proposed JSON Shape" in the spec due to this extra metadata.

### 4. Test Coverage
Unit test coverage for the new JSON builders in `repo_json.rs` is excellent. Integration tests in `cli.rs` cover the primary subcommands like `git-status` and `repo` (structure). Adding integration tests for the boolean and locator families would ensure their exit-code semantics remain stable.

## Recommendations
1. **Unify File Listing:** Refactor `unstaged-files` and `untracked-files` to use the shared `handle_file_list_command` logic.
2. **Standardize Shapes:** Consider moving `packages` and `package-areas` to the same `{ names: [...] }` object shape for better consistency with their `dirty-*` / `staged-*` counterparts.
3. **Trim Commit JSON:** Optionally exclude the full `packages` metadata from `source-code-changes` and `documentation-changes` if brevity is a priority for these subcommands.

## Conclusion

The implementation is robust, follows the specification's intent, and is a significant improvement over the previous state.

**Status: Ready for Production**
