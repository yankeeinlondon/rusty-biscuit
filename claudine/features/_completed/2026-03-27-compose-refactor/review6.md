# Compose Refactor Review 6

`just test` in `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine` passed while reviewing this refactor.

## Findings

### 1. [P2] Composition still derives part of its wrapper context from the caller's cwd instead of the resolved source document

- The refactor now carries `prepared.source_repo_root`, and provider selection correctly consults it in [`wrap/composition.rs`](../../cli/src/commands/wrap/composition.rs), but the wrapper env/shadow-home path still starts from `current_dir()`.
- [`execute_composition_request()`](../../cli/src/commands/wrap/composition.rs) calls `build_child_env(..., &cwd, ...)` at lines 140-151, builds MCP shadow HOME from `cwd` at lines 238-245, and derives the displayed environment context from `cwd` at line 448.
- `build_child_env()` then resolves package/repo context from that cwd in [`wrap/env.rs`](../../cli/src/commands/wrap/env.rs), and Codex repo prompt overlay still resolves repo-scoped resources from the same cwd in [`wrap/repo_home.rs`](../../cli/src/commands/wrap/repo_home.rs#L145-L168).

This means `claudine compose /other/repo/prompt.md` can still pick up the caller repo's prompt overlay, package metadata, and shadow-HOME state even though the composition source was resolved in a different repo. The favorite-provider lookup is now source-aware, but the rest of the wrapper context is not.

### 2. [P2] Non-harness `inline-compose` still skips provider-policy writability checks

- The design required inline prep validation to reject targets that are unwritable both on the filesystem and under the provider sandbox model when Claudine can determine that policy.
- The harness path already has the machinery for this: `run_harness_loop()` creates a `WrapperHarnessPermissionProbe` in [`wrap/mod.rs`](../../cli/src/commands/wrap/mod.rs#L2093-L2124), and `has_write_permission` ultimately consults provider policy in [`harness/validate.rs`](../../lib/src/harness/validate.rs#L347-L395).
- But the non-harness inline path in [`wrap/composition.rs`](../../cli/src/commands/wrap/composition.rs#L415-L419) still calls `validate_file_permissions()` directly, which only performs the raw OS read/write check from [`composition/resolve.rs`](../../lib/src/composition/resolve.rs#L50-L64).

So inline runs without harness frontmatter still do not enforce provider-side write policy before launch. The implementation already knows how to answer that question for Codex via `WrapperHarnessPermissionProbe`; it just is not reused on the non-harness path.

### 3. [P3] The composition docs still advertise a false migration path for retired `--prompt-file`

- [`docs/topics/composition.md`](../../docs/topics/composition.md#L145-L156) still tells users that `claudine <agent> --prompt-file <file>` maps to `claudine compose --<agent> <file>`.
- The actual CLI rejects `--prompt-file` as retired in [`wrap/mod.rs`](../../cli/src/commands/wrap/mod.rs#L553-L578).

That leaves acceptance criterion 8 incomplete: the code removed the surface, but the main composition doc still points users at a replacement that changes semantics instead of documenting that prompt-file support was intentionally removed by this refactor.

## Coverage Gaps

- The CLI suite covers explicit provider selection, ambiguous hints without a TTY, config favorites, harness activation, inline closure, and no cross-provider retry. What is still missing is the design's required end-to-end branch where an `agent` hint matches a known provider that is not installed and the command falls through to config/chooser/error instead of failing immediately.
- There is also no integration test for cross-repo composition proving that source-repo context, not caller-cwd context, drives wrapper env/shadow-home setup.
- There is no integration test for non-harness inline sandbox-policy denial; current tests only cover raw filesystem permissions and the harness-backed writability check.

## Dead Code / Cleanup

- [`CompositionError::ProviderLaunchFailed`](../../lib/src/composition/error.rs#L71-L73) is currently unused.
- [`ResolvedCompositionSource::original_ref`](../../lib/src/composition/types.rs#L20-L31) is only exercised in tests.
- [`SelectedProvider::reason`](../../lib/src/composition/types.rs#L48-L55) is populated and tested, but the executor never surfaces it in reporting or user-facing summaries.

Either wire these into reporting/diagnostics, or remove them to keep the post-refactor composition model smaller and easier to reason about.

## Ergonomics / Performance

- Thread `prepared.source_repo_root` through `build_child_env()`, `repo_home::build_repo_home_env()`, and `detect_environment_fast()` so composition of files in another repo becomes deterministic and avoids repeated cwd-based discovery.
- Reuse the existing `HarnessPermissionProbe`-backed write-policy check for non-harness inline runs instead of maintaining a separate filesystem-only validation path.
- If selection reason is useful product metadata, emit it in the composition session summary event; otherwise drop it from the public composition types.
