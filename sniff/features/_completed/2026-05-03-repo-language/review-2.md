---
ready: true
agent: gemini
model: ""
---

# Feature Review: Repo Language Command and Base Directory Regression Fix

I have reviewed the implementation of the `sniff repo language` command and the fix for the `--base` directory regression.

## Findings

### 1. `sniff repo language` Command
The new subcommand is correctly implemented across the CLI layers:
- **Argument Parsing:** Added as a `RepoSubcommand` in `src/args.rs`.
- **Command Dispatch:** Correctly mapped to `RepoAction::Language` and integrated into the `DetectionPlan` to minimize resource usage (uses `GitRequest::summary()` and disables unnecessary sections like OS, hardware, and network).
- **Output Rendering:** Implemented in `src/output/filesystem.rs` and `src/output/repo_json.rs`.
- **Contract Fulfillment:**
    - **Text Mode:** Returns exactly `{Language}\n` (e.g., `Rust\n`) or empty with exit code 1 when no language is detected.
    - **JSON Mode:** Returns `{"language": "Rust"}` or `{"language": null}` with exit code 1 for the null case.
- **Language Logic:** Uses the existing `primary_language` detection which correctly excludes markup and configuration files by leveraging the `FileAssociation` classification.

### 2. `--base <dir>` Regression Fix
The regression where `--base` was not working for `repo` subcommands has been resolved:
- **Global Flag:** The `base` flag is correctly defined as `global = true` in the top-level `Cli` struct.
- **Unified Handling:** `src/commands.rs` now consistently canonicalizes the base directory and passes it through to both the detection library and the output renderers.
- **Subcommand Support:** Verified that `base_dir` is correctly utilized by both "fast-path" handlers (like `packages`, `package-root`) and the full-detection path used by `structure` and `git-status`.

### 3. Test Rigor
The feature has been validated with a strong suite of Level 1 integration tests in `sniff/cli/tests/cli.rs`:
- **Requirement Verification:**
    - `test_repo_language_text_returns_rust_for_rust_repo` (Level 1): Verifies text output contract.
    - `test_repo_language_json_returns_rust_for_rust_repo` (Level 1): Verifies JSON output contract.
    - `test_repo_language_base_flag_all_three_placements` (Level 1): Verifies the `--base` flag works when placed before `repo`, between `repo` and `language`, or after `language`.
    - `test_repo_language_text_empty_repo_exits_one_with_no_stdout` (Level 1): Verifies failure behavior.
    - `test_repo_subcommand_json_shapes_are_distinct` (Level 1): Batch test verifying that multiple `repo` subcommands (including `git-status`, `deps`, `packages`, etc.) respect the `--base` flag and produce valid JSON.
- **Rigor Level Assessment:** Since these commands primarily involve data retrieval and plain text/JSON output without complex terminal UI behaviors (like modifier-press badges or real-terminal rendering edge cases), **Level 1 tests are appropriate and sufficient** for this feature.

## Recommendations

- **Ergonomics:** The `sniff repo language` command is highly ergonomic for scripting. For human-readable summaries with full breakdowns, users should continue to use the top-level `sniff language` command.
- **Performance:** The command is performant, utilizing a tailored `DetectionPlan` that avoids heavy git operations. While it still performs a full recursive scan of the filesystem to determine language percentages, this is necessary for accurate primary language detection.

## Conclusion

The feature is implemented correctly according to the specification, addresses the reported regression, and is supported by comprehensive tests.

**Ready for production.**
