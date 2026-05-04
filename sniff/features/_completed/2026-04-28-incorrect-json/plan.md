---
phases: 6
created: 2026-04-29
start_phase: 1
---

# Execution Plan: Correct `sniff repo --json` Output

## Phase 1: Baseline And Contract Inventory

Goal: establish the current behavior, implementation touchpoints, and expected JSON contract before changing code.

1. Run targeted baseline commands from the repo root and capture representative output shapes for:
   - `sniff --json repo`
   - `sniff --json repo structure`
   - `sniff --json repo git-status`
   - `sniff --json repo deps`
   - each dirty/staged/unstaged package and package-area command
   - `sniff --json repo package-root`, `package-area-root`, `package`, and `package-area`
   - `sniff --json repo recent-commits`, `source-code-changes`, and `documentation-changes`
   - boolean commands with and without matching state where practical
2. Confirm the primary routing points:
   - `sniff/cli/src/commands.rs` decides when repo actions are fast-pathed or sent through full detection.
   - `sniff/cli/src/output/mod.rs::print_json` and `apply_filter_to_json` currently collapse most repo subcommands into `OutputFilter::Repo`.
   - `sniff/cli/src/output/filesystem.rs` already contains the text-mode selectors for package, package-area, dependency, locator, and boolean behavior.
   - `sniff/cli/src/output/recent_commits.rs` serializes unfiltered `CommitDescSet` before the text-only `CommitCentricFilter` is applied.
3. Inventory existing tests near `sniff/cli/src/args.rs`, `sniff/cli/src/output/filesystem.rs`, `sniff/cli/src/output/recent_commits.rs`, and any CLI integration tests to identify the smallest stable test layer for each command family.
4. Define a short JSON contract matrix in test code or fixture comments covering every affected command and the commands explicitly left unchanged.

Validation checkpoint:
- A maintainer can point from each affected subcommand to its planned serializer or existing unchanged path.
- Baseline confirms `structure` and bare `repo` remain the only intentionally identical repo JSON outputs.

Parallelizable:
- Baseline command capture and test inventory can run in parallel after the spec is read.

## Phase 2: Add Repo-Action-Aware JSON Routing

Goal: preserve existing global JSON filtering while giving `repo` subcommands access to their specific action before serialization.

1. Add a repo-specific JSON entry point, preferably in `sniff/cli/src/output/mod.rs` or a new focused module under `sniff/cli/src/output/`, with a signature that receives:
   - `SniffResult`
   - `RepoAction`
   - `base_dir`
   - `verbose`
   - repo action filters such as package filters
   - performance attachment requirements
2. Update `sniff/cli/src/commands.rs` so the full-detection repo path calls this repo-specific JSON entry point when `use_json` is true and `repo_action` is present.
3. Keep existing early-return JSON paths unchanged for commands already correct:
   - dirty/staged/source file list commands
   - `packages`
   - `package-areas`
   - `hash`
   - `root`
   - `remote`
   - `pr`
   - unstaged/untracked file list commands
   - `recent-commits`, until Phase 5 changes filtered variants
4. Preserve the default `output::print_json(&result, OutputFilter::Repo, ...)` behavior for bare `sniff --json repo` and `sniff --json repo structure`.
5. Ensure `--perf` handling remains centralized by reusing or adapting `attach_performance` instead of duplicating performance insertion logic.

Validation checkpoint:
- `sniff --json repo structure` and bare `sniff --json repo` still serialize full `RepoInfo`.
- `sniff --json repo git-status` no longer reaches the generic `OutputFilter::Repo` serialization path.
- Existing non-repo JSON commands still compile and route through their previous code paths.

Parallelizable:
- This phase blocks later serializers because they need a caller, but test scaffolding for expected shapes can start in parallel.

## Phase 3: Implement Shared JSON Builders For Repo Families

Goal: add reusable structured output functions that match the text-mode selectors without parsing rendered text.

1. Expose or refactor package selection helpers in `sniff/cli/src/output/filesystem.rs` so JSON builders can reuse the same data logic as text mode:
   - dirty package names
   - staged package names
   - unstaged package names
   - package-area derivation from selected package names
   - package/package-area locator resolution
   - package-area dirty and source-code-change boolean checks
2. Add focused serializable structs or `serde_json::json!` builders for:
   - `{ "scope": "...", "kind": "packages", "names": [...] }`
   - `{ "scope": "...", "kind": "package_areas", "names": [...] }`
   - `{ "root": "..." }`
   - `{ "name": "..." }`
   - `{ "dirty": bool }`
   - `{ "has_source_code_changes": bool }`
   - `{ "has_merge_conflict": bool }`
3. Implement dependency JSON builder for `deps`:
   - filter packages with the same repo filter used by text mode
   - include `name`, `depends_on`, `used_by`
   - include `dependencies`, `dev_dependencies`, `peer_dependencies`, and `optional_dependencies` only when populated, preserving existing `DependencyEntry` serialization
   - include external dependency entries with at least `name` and `targeted_version`; preserve `actual_version` when available
4. Implement `git-status` JSON as direct serialization of `filesystem.git`, with package scoping already applied by `commands.rs`.
5. Ensure all builders return `serde_json::Value` or serializable objects, not strings.

Validation checkpoint:
- Unit tests can construct minimal `SniffResult`/`RepoInfo` fixtures and assert exact JSON keys for each builder.
- No JSON builder depends on ANSI, `Prose`, `Terminal`, CSV text, or markdown rendering.

Parallelizable:
- Dependency JSON, package-family JSON, locator JSON, and boolean JSON builders can be implemented independently once shared selection helpers are decided.

## Phase 4: Wire Broken Full-Detection Repo Subcommands

Goal: connect every full-detection affected repo action to its focused JSON output and preserve text behavior.

1. Wire `RepoAction::GitStatus` to serialize `GitInfo` directly.
2. Wire `RepoAction::Deps` to the dependency JSON builder and honor existing package filter behavior.
3. Wire package-family actions:
   - `DirtyPackages`
   - `DirtyPackageAreas`
   - `StagedPackages`
   - `StagedPackageAreas`
   - `UnstagedPackages`
   - `UnstagedPackageAreas`
4. Wire locator actions:
   - `PackageRoot`
   - `PackageAreaRoot`
   - `Package`
   - `PackageArea`
5. Adjust current early text-only handling for `RepoAction::Package` and `RepoAction::PackageArea` so JSON mode returns `{ "name": ... }` and still honors `--no-error` / `--on-error` no-result behavior.
6. Wire boolean actions:
   - `IsCurrentPackageAreaDirty`
   - `PackageAreaHasSourceCodeChanges`
   - `HasMergeConflict`
7. For boolean JSON mode, print the boolean object and still exit `0` when true and `1` when false.
8. Preserve existing text-mode exit-code-only behavior for boolean commands, including verbose stderr output where it exists.

Validation checkpoint:
- Running the 16 previously full-`RepoInfo` commands with `--json` yields distinct, focused JSON shapes.
- Boolean commands keep backward-compatible shell semantics in both text and JSON mode.
- Locator commands return empty/no-result behavior consistently with their text counterparts.

Parallelizable:
- Git status/deps wiring, package-family wiring, locator wiring, and boolean wiring can be reviewed independently after Phase 3.

## Phase 5: Filter Commit-Family JSON

Goal: make `source-code-changes --json` and `documentation-changes --json` apply the same commit/file filtering as styled and plain text output.

1. Move the commit file-match logic behind `CommitCentricFilter` into reusable methods, or add a pure helper that takes `CommitDescSet` plus `CommitCentricFilter` and returns a filtered serializable value.
2. For `RecentCommitsMode::RecentCommits`, preserve current full `CommitDescSet` JSON exactly.
3. For `SourceCodeChanges`, return only commits containing source-code files and prune each commit's `files` array to source-code files.
4. For `DocumentationChanges`, return only commits containing documentation files and prune each commit's `files` array to documentation files.
5. Add `"filter": "source_code"` or `"filter": "documentation"` to filtered JSON outputs without adding it to `recent-commits`.
6. Apply no-result handling after filtering, so a period with commits but no matching source/docs files behaves like text mode.

Validation checkpoint:
- Tests with mixed commit files prove non-matching files are removed and commits with no remaining files are omitted.
- `recent-commits --json` remains backward-compatible.
- `--package`, `--package-area`, and `--action` filters still apply before source/docs file filtering.

Parallelizable:
- Commit filtering tests can be written in parallel with Phase 4 because they touch a separate handler.

## Phase 6: End-To-End Validation, Documentation, And Regression Guardrails

Goal: prove the implementation satisfies the spec and leave durable coverage for future repo subcommands.

1. Add or update CLI-level tests that execute representative commands against a temporary git workspace/monorepo fixture:
   - `git-status --json` has `repo_root`, `status`, and `recent`, and does not have `packages` as the top-level repo structure.
   - `deps --json` has top-level `packages` entries with internal edges and dependency sections.
   - dirty/staged/unstaged package commands return `{ scope, kind, names }`.
   - locator commands return only `root` or `name`.
   - boolean commands return descriptive boolean keys and expected exit codes.
   - `source-code-changes --json` and `documentation-changes --json` filter commits and files.
2. Add a regression assertion that affected repo subcommands do not all serialize to the same JSON object; allow only bare `repo` and `structure` to match.
3. Run focused checks first:
   - `cargo test -p sniff-cli repo`
   - `cargo test -p sniff-cli recent_commits`
   - any integration test package that owns CLI command execution
4. Run broader checks after focused tests pass:
   - `cargo fmt --check`
   - `cargo clippy -p sniff-cli --all-targets -- -D warnings`
   - `cargo test -p sniff-cli`
   - if time permits, `just test sniff` or the area-specific `sniff/justfile` test recipe
5. Manually verify the acceptance command matrix from Phase 1 using `jq type` and key checks, including `--perf` with at least one object output and one array output.
6. Update docs only where public JSON behavior is documented:
   - `sniff/cli/README.md`
   - relevant `sniff/docs/cli/repo_*.md` pages for changed JSON examples
   - `docs/dependencies.md` only if implementation adds or removes crates
   - `.claude/skills/` only if repo workflow or architecture guidance changes

Validation checkpoint:
- All acceptance criteria in `spec.md` are checked off against tests or manual command output.
- No existing correct JSON command regresses.
- `--perf` still attaches performance data without corrupting the command-specific JSON shape.

Parallelizable:
- Documentation updates can proceed while broader tests run after final JSON shapes are stable.
- Manual command matrix verification can be split by command family.

## Suggested Implementation Order

1. Land Phase 2 first with a minimal fallback that delegates unknown repo actions to existing full `RepoInfo` JSON.
2. Implement Phase 3 builders with unit tests before changing each command family.
3. Wire Phase 4 one family at a time and run focused tests after each family.
4. Complete Phase 5 commit filtering independently.
5. Finish Phase 6 with regression tests, command matrix verification, and docs.

## Risks And Mitigations

- Risk: boolean commands currently call `std::process::exit` inside output helpers, which makes JSON mode hard to test.
  Mitigation: refactor boolean logic into pure `bool` helpers and keep exiting only at the command boundary.
- Risk: package and package-area text paths currently use rendered strings for no-result behavior.
  Mitigation: resolve package/package-area as data first, then render or serialize from the same option.
- Risk: dependency JSON could accidentally expose the entire `Package` struct.
  Mitigation: create a narrow DTO for `deps --json` and assert excluded keys like `path`, `languages`, and `documentation` are absent unless intentionally added.
- Risk: `--perf` changes array outputs by wrapping them under `data`.
  Mitigation: preserve existing `attach_performance` semantics and test both object and non-object outputs.
