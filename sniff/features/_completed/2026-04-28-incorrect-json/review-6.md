---
ready: true
agent: gemini
model: ""
---

# Review: Incorrect JSON Output for `sniff repo` Subcommands

I have completed a comprehensive review of the implementation for the "incorrect-json" feature. The implementation successfully addresses the core problem where many `sniff repo` subcommands were returning an unfiltered `RepoInfo` blob instead of tailored JSON mirroring their text-mode output.

## Executive Summary

The feature is **ready for production**. The implementation is structurally sound, follows the architectural conventions of the `sniff` package, and provides strong verification across all requirements.

## Findings

### 1. Functional Completeness
All 18 subcommands identified as broken or partially broken in the specification have been fixed. 
- **Structure Family:** `sniff repo` and `sniff repo structure` now correctly honor the `--filter` flag in JSON mode.
- **Git Status:** Now returns a focused `GitInfo` object.
- **Dependency Diagram:** `deps --json` returns a hand-built, allow-listed per-package dependency object, ensuring a stable public contract.
- **Package Families:** `dirty-packages`, `staged-package-areas`, etc., now return the standardized `{ scope, kind, names }` shape.
- **Locator Family:** `package-root` and `package-area-root` return `{ root: "..." }`; `package` and `package-area` return `{ name: "..." }`.
- **Boolean Family:** `is-current-package-area-dirty`, `package-area-has-source-code-changes`, and `has-merge-conflict` return descriptive boolean objects.
- **Commit Family:** `source-code-changes` and `documentation-changes` now correctly filter both commits and files in their JSON output, including a `"filter": "..."` tag.

### 2. Exit Code Integrity
The implementation correctly maintains backward compatibility with shell scripts by mirroring text-mode exit codes for boolean and locator subcommands (e.g., `is-current-package-area-dirty` exits `0` if true, `1` if false). This is handled elegantly through the `BuildOutcome` struct in `repo_json.rs`.

### 3. Performance Data
Acceptance Criterion 9 is met. Performance data is either injected into the JSON (for commands using `print_json`) or emitted to `stderr` (for early-return commands like `package` or `has-merge-conflict`), ensuring `stdout` remains parseable JSON while providing timing information when requested via `--perf`.

### 4. Test Rigor
The feature is supported by a multi-layered testing strategy:

| Requirement | Verification Level | Status |
|---|---|---|
| JSON Shape Correctness | **Level 1** (Unit + Integration) | **Pass** |
| Distinctness Matrix | **Level 1** (Integration) | **Pass** |
| Exit Code Semantics | **Level 1** (Integration) | **Pass** |
| Commit/File Filtering | **Level 1** (Unit + Integration) | **Pass** |

- **Unit Tests:** Found in `repo_json.rs` and `commit_blocks.rs`, covering edge cases like non-monorepo behavior and sparse dependency objects.
- **Integration Tests:** `sniff/cli/tests/cli.rs` contains extensive tests, including a dedicated distinctness check (`test_repo_subcommand_json_shapes_are_distinct`) ensuring no two subcommands return identical JSON.
- **Level 2/3 Note:** As this feature is purely data-focused and non-interactive, Level 1 verification is the appropriate and sufficient level.

## Suggestions for Future Improvement

- **Consistent Perf Injection:** While emitting perf data to `stderr` is correct for keeping `stdout` clean, some early-return commands currently don't inject performance data *inside* the JSON, unlike the `print_json` path. While not a requirement for this phase, unifying this behavior would make the JSON contract even more consistent.
- **CommitDescSet Size:** For very large repositories, the `CommitDescSet` in `recent-commits --json` can be quite large. Future iterations might consider adding pagination or more aggressive property trimming if needed for performance.

## Closure

The implementation is high-quality, idiomatically consistent with the rest of the monorepo, and fully satisfies the specification.
