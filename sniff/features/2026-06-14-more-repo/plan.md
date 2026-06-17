---
agent: codex/
phases: 9
created: 2026-06-16
start_phase: 1
yolo: "true"
packages:
  - biscuit-clipboard
  - biscuit-visualized
  - claudine
  - messenger
  - renderable
  - sniff
  - sniff-cli
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - sniff/features/2026-06-14-more-repo/plan.md
docs_created_during_phase_1:
  - sniff/features/2026-06-14-more-repo/implementation-notes.md
  - sniff/features/2026-06-14-more-repo/baseline-repo.json
  - sniff/features/2026-06-14-more-repo/baseline-repo-deps.json
  - sniff/features/2026-06-14-more-repo/baseline-repo-structure.json
  - sniff/features/2026-06-14-more-repo/baseline-repo-git-status.json
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/topics.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/install_plan.rs
  - sniff/cli/tests/install_interview_cli.rs
  - sniff/cli/tests/snapshots/snapshots__help_output.snap
  - sniff/cli/tests/snapshots/snapshots__topics_table.snap
docs_updated_during_phase_2:
  - sniff/features/2026-06-14-more-repo/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/lib/src/programs/contract.rs
  - sniff/lib/src/programs/enums/categories.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/enums/mod.rs
  - sniff/lib/src/programs/install/command.rs
  - sniff/lib/src/programs/local_bin.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/test_runner.rs
  - sniff/lib/src/programs/test_runner_spec.rs
  - sniff/lib/src/programs/types.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/test_runners.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots/snapshots__help_output.snap
  - sniff/cli/tests/snapshots/snapshots__topics_table.snap
docs_updated_during_phase_3:
  - sniff/features/2026-06-14-more-repo/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_4:
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/test_runner_usage.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/test_runner_report.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots/snapshots__help_output.snap
docs_updated_during_phase_4:
  - sniff/features/2026-06-14-more-repo/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_5:
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/tests/integration.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/filesystem/deps.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots.rs
docs_updated_during_phase_5:
  - sniff/cli/README.md
  - sniff/docs/cli/repo.md
  - sniff/docs/cli/repo_deps.md
  - sniff/docs/topics/json-output.md
  - sniff/features/2026-06-14-more-repo/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_6:
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_6:
  - sniff/features/2026-06-14-more-repo/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_7:
  - sniff/cli/src/output/topics.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots/snapshots__topics_table.snap
docs_updated_during_phase_7:
  - sniff/features/2026-06-14-more-repo/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8:
  - just/devops.just
  - sniff/cli/tests/install_interactive_pty.rs
docs_updated_during_phase_8:
  - biscuit-clipboard/docs/research/clipboard-managers.md
  - biscuit-visualized/docs/dot-graph.md
  - claudine/features/_completed/2026-04-17-edit-command/design.md
  - messenger/features/2026-04-27-leveraging-notification-helpers/plan.md
  - messenger/features/2026-04-27-leveraging-notification-helpers/tech-design.md
  - messenger/reviews/slow-info.md
  - renderable/features/_completed/2026-05-16-iterative-improvement/components/GraphExpression.md
  - sniff/README.md
  - sniff/cli/README.md
  - sniff/docs/cli/repo.md
  - sniff/docs/cli/repo_is-monorepo.md
  - sniff/docs/cli/repo_package-count.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/docs/topics/json-output.md
  - sniff/features/2026-06-14-more-repo/plan.md
  - sniff/features/2026-06-14-more-repo/implementation-notes.md
  - sniff/fixes/2026-05-07-repo-package-consistency/plan.md
  - sniff/lib/README.md
  - sniff/reviews/2026-05-05-bench/review.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/sniff/SKILL.md
source_code:
  - just/devops.just
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/filesystem/deps.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/test_runner_report.rs
  - sniff/cli/src/output/test_runners.rs
  - sniff/cli/src/output/topics.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/install_interview_cli.rs
  - sniff/cli/tests/install_interactive_pty.rs
  - sniff/cli/tests/install_plan.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/cli/tests/snapshots/snapshots__help_output.snap
  - sniff/cli/tests/snapshots/snapshots__topics_table.snap
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/test_runner_usage.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/programs/contract.rs
  - sniff/lib/src/programs/enums/categories.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/enums/mod.rs
  - sniff/lib/src/programs/install/command.rs
  - sniff/lib/src/programs/local_bin.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/test_runner.rs
  - sniff/lib/src/programs/test_runner_spec.rs
  - sniff/lib/src/programs/types.rs
  - sniff/lib/tests/integration.rs
documentation:
  - biscuit-clipboard/docs/research/clipboard-managers.md
  - biscuit-visualized/docs/dot-graph.md
  - claudine/features/_completed/2026-04-17-edit-command/design.md
  - messenger/features/2026-04-27-leveraging-notification-helpers/plan.md
  - messenger/features/2026-04-27-leveraging-notification-helpers/tech-design.md
  - messenger/reviews/slow-info.md
  - renderable/features/_completed/2026-05-16-iterative-improvement/components/GraphExpression.md
  - sniff/README.md
  - sniff/cli/README.md
  - sniff/docs/cli/repo.md
  - sniff/docs/cli/repo_deps.md
  - sniff/docs/cli/repo_is-monorepo.md
  - sniff/docs/cli/repo_package-count.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/docs/topics/json-output.md
  - sniff/features/2026-06-14-more-repo/baseline-repo-deps.json
  - sniff/features/2026-06-14-more-repo/baseline-repo-git-status.json
  - sniff/features/2026-06-14-more-repo/baseline-repo-structure.json
  - sniff/features/2026-06-14-more-repo/baseline-repo.json
  - sniff/features/2026-06-14-more-repo/implementation-notes.md
  - sniff/features/2026-06-14-more-repo/plan.md
  - sniff/fixes/2026-05-07-repo-package-consistency/plan.md
  - sniff/lib/README.md
  - sniff/reviews/2026-05-05-bench/review.md
---

# Execution Plan — More Repo Feature

Derived from [`spec.md`](./spec.md) and [`test-runner-strategy.md`](./test-runner-strategy.md).

## Ground Rules

- Use the `sniff`, `cli`, `rust-devops`, and `biscuit-terminal` skills during implementation.
- Keep business logic in `sniff/lib`; keep `sniff/cli` limited to argument parsing, output selection, rendering, and exit behavior.
- Treat this as a coordinated hard break: remove old CLI surfaces outright, do not add compatibility aliases, legacy JSON flags, or schema-version fallback paths.
- Preserve focused subcommand richness where the spec says to; make the bare `sniff repo --json` aggregate a lean projection rather than a concatenation of focused payloads.
- Keep stdout as primary data and stderr for non-data hints. In `--json` mode, stdout must be valid JSON.
- Use pure Rust `gix`-based Git paths already present in Sniff; any optional network refresh must be explicit and non-interactive with `GIT_TERMINAL_PROMPT=0`.
- Remote commands (`repo issues`, `repo ci-cd` / `repo ci`) and focused `repo is-monorepo` redesign are out of scope.

## Phase 1 — Baseline Audit And Contract Inventory

Purpose: establish the current surfaces, tests, call sites, and JSON shapes before making the hard break.

- [x] Run `sniff repo` or `cargo metadata --no-deps --format-version 1` as needed to confirm workspace package names and targeted `-p` values.
- [x] Inspect current CLI command wiring for installed-program commands, repo subcommands, and aggregate JSON builder in `sniff/cli/src/args`, `sniff/cli/src/commands`, and `sniff/cli/src/output`.
- [x] Inspect library repo detection paths for `Package`, manifest parsing, package-manager detection, version detection, Git branch/worktree data, and dependency extraction.
- [x] Inspect existing software category architecture: category enums, `ProgramInfo`, `ProgramsInfo::detect`, JSON rendering, markdown/text rendering, and CLI action macros.
- [x] Audit test fixtures and snapshots for installed-program commands, repo JSON aggregate, repo deps, repo version, worktrees, git status, and package listing.
- [x] `git grep` in the monorepo for removed or renamed invocations: `sniff programs`, `sniff editors`, `sniff utilities`, `sniff language-package-managers`, `sniff os-package-managers`, `sniff tts-clients`, `sniff terminal-apps`, `sniff audio-players`, `sniff notification-helpers`, `sniff agents`, and `sniff repo deps`.
- [x] `git grep` for `repo --json` consumers, especially in `claudine`, and record the fields they consume.
- [x] Capture a baseline `sniff repo --json` fixture for this repo, including byte size, top-level keys, duplicated package catalogs, worktree duplication, and change-family envelopes.
- [x] Confirm with tests or manual inspection that `sniff repo version` currently fails or returns null for the Rust monorepo case described in the spec.
- [x] Document any pre-existing failing tests before code changes so later validation can distinguish regressions from baseline noise.

**Parallelizable**

- [x] Run call-site grep/audit in parallel with code inspection.
- [x] Capture baseline JSON metrics in parallel with snapshot inventory.

**Validation checkpoint 1**

- [x] Baseline command/JSON contracts are recorded in implementation notes or test fixtures.
- [x] All in-repo call sites requiring migration are identified.
- [x] Out-of-scope commands are explicitly left unimplemented.

## Phase 2 — Software Command Reparenting

Purpose: move the installed-program surface under `sniff software` and remove old top-level paths.

- [x] Add a `software` top-level command with an aggregate action equivalent to the current `sniff programs` behavior.
- [x] Move existing installed-program category commands under `sniff software`: `editors`, `utilities`, `language-package-managers`, `os-package-managers`, `tts-clients`, `terminal-apps`, `audio-players`, `notification-helpers`, and `agents`.
- [x] Remove old top-level installed-program command variants and their routing paths entirely, including help output and completion-visible command entries.
- [x] Preserve existing text, plain, and JSON output behavior for the reparented categories except for the command path.
- [x] Ensure `sniff software --json` emits the same aggregate data shape that `sniff programs --json` emitted before adding test runners.
- [x] Update CLI help/snapshot tests so removed top-level paths are absent and new `software` paths are present.
- [x] Add negative CLI tests showing old top-level command paths fail with clap usage errors rather than silently aliasing.

**Parallelizable**

- [x] Snapshot/help updates can run in parallel with call-site migration from Phase 8 after command wiring compiles.

**Validation checkpoint 2**

- [x] `cargo build -p sniff-cli` succeeds.
- [x] `sniff software --json` stdout is valid JSON.
- [x] Old top-level installed-program paths are not accepted.

## Phase 3 — Test Runner Catalog And Host Availability

Purpose: add the ninth software category and host availability model in the library.

- [x] Add the `TestRunner` enum with the full v1 catalog from `test-runner-strategy.md`.
- [x] Add `TEST_RUNNER_INFO` / metadata so `TestRunner` participates in the same category machinery as existing software categories.
- [x] Add `InvocationClass`, `RunnerKind`, `TestRunnerSpec`, and metadata fields for ecosystem, parent binary, dependency keys, config globs, and ecosystem defaults.
- [x] Extend `ExecutableSource` with `ProjectLocal { root }` and any required serialization/output support.
- [x] Implement `LocalBinIndex` or an `ExecutableIndex` extension for project-local bin search: Node `node_modules/.bin` walk-up, PHP `vendor/bin`, Python virtualenv directories including Windows `Scripts`, Ruby `bin`, and `$VIRTUAL_ENV`.
- [x] Implement host resolution order in the library: project-local bin, global PATH index, parent binary for class B/C, then not found.
- [x] Add `availability` to test-runner program output with `installed`, `local`, `via_parent`, and `not_found` cases.
- [x] Add `test_runners` to `ProgramsInfo` and detect it in parallel with the shared executable index.
- [x] Wire `sniff software test-runners` through CLI args, actions, JSON output, and terminal rendering.
- [x] Render availability details through `biscuit-terminal` components, not hand-written ANSI.
- [x] Add cross-platform tests for executable suffix handling and local-bin search roots, using fixture directories rather than host-global assumptions.

**Parallelizable**

- [x] Catalog metadata table work can proceed in parallel with local-bin resolver implementation.
- [x] CLI wiring can proceed after the `ProgramsInfo.test_runners` field shape is known.

**Validation checkpoint 3**

- [x] `cargo test -p sniff` passes for test-runner metadata and local-bin resolution.
- [x] `cargo test -p sniff-cli` or targeted CLI tests pass for `sniff software test-runners`.
- [x] `sniff software test-runners --json` stdout includes availability discriminators and remains valid JSON.

## Phase 4 — Repo Test Runner Usage And Shared Collapse Logic

Purpose: detect declared test runner usage per package and expose context-aware repo aggregation.

- [x] Add typed `TestRunnerUsage` and `TestRunnerSource` to the library with evidence sources: manifest key, config file, ecosystem default, and convention.
- [x] Add `test_runners: Vec<TestRunnerUsage>` to `Package` and update serde snapshots/fixtures as needed.
- [x] Extend manifest cache/index support for runner detection inputs not currently parsed: `composer.json`, `.csproj`, `pom.xml`, `build.gradle[.kts]`, `mix.exs`, `Gemfile`, `*.gemspec`, and Python requirements files where appropriate.
- [x] Implement `detect_test_runners(pkg_dir, cache)` as a sibling to `detect_package_managers()`.
- [x] Apply runner signal priority per package: config file, manifest dependency key, ecosystem default, then convention.
- [x] Record orchestrators such as `tox` and `nox` with `kind: orchestrator`, not as plain runner strings.
- [x] Add focused package-level tests for Rust, Go, JS/TS, Python, PHP, Ruby, JVM, .NET, and Elixir detection cases from the strategy table.
- [x] Add a shared library aggregation helper for package-context, package-area-context, and repo-root collapse logic.
- [x] Refactor existing package-manager reporting logic to use the shared aggregation helper.
- [x] Add `sniff repo test-runner` CLI with default text plus `--csv`, `--list`, `--md`, and `--json` output.
- [x] Ensure CLI output reports library-provided runner values and evidence where JSON supports it; do not re-detect in the CLI.

**Parallelizable**

- [x] Ecosystem-specific fixture tests can be implemented independently once `TestRunnerUsage` is defined.
- [x] Package-manager collapse refactor can proceed in parallel with runner signal parsing.

**Validation checkpoint 4**

- [x] `cargo test -p sniff` passes package runner detection and aggregation tests.
- [x] `sniff repo test-runner --json` reports typed usage from the current repo/package context.
- [x] `sniff repo package-manager` still matches previous behavior for uniform and variant package sets.

## Phase 5 — Repo Local Commands And Library Data

Purpose: add the new repo commands and fix existing local repo facts.

- [x] Implement library `BranchInfo` projection with `name`, `current`, `sha`, `remote_represented`, `upstream`, `ahead`, and `behind`.
- [x] Ensure branch detection uses locally known refs only by default and does not fetch.
- [x] If `--refresh-remotes` is added, route through existing non-interactive remote refresh code and set `GIT_TERMINAL_PROMPT=0`.
- [x] Add `sniff repo branches` CLI with JSON array output and terminal rendering through `biscuit-terminal`.
- [x] Keep branch list on stdout; suppress any legend in `--json` mode.
- [x] Rename focused internal workspace dependency graph command from `sniff repo deps` to `sniff repo package-dependencies`.
- [x] Remove `sniff repo deps` entirely; add negative tests proving it is not an alias.
- [x] Add `sniff repo dependencies` for external dependencies with filters for `--dependencies`, `--dev-dependencies`, `--peer-dependencies`, and `--optional-dependencies`.
- [x] Ensure `sniff repo package-dependencies` preserves current internal graph behavior, including Mermaid `--ui`.
- [x] Fix repo version detection in the library for Cargo root package/workspace-root package, Node `package.json`, Python `pyproject.toml [project].version`, and safe parser-backed ecosystem manifests.
- [x] Preserve focused `sniff repo version --json` shape `{ "version": string | null }`; missing versions are success with null.
- [x] Add targeted fixtures for Cargo workspace root version, package root version, Node version, Python version, and null ecosystems.

**Parallelizable**

- [x] Branch projection can proceed in parallel with dependency command rename.
- [x] Version detection fixtures can proceed in parallel with external dependency filter wiring.

**Validation checkpoint 5**

- [x] `sniff repo branches --json` emits an array of `BranchInfo` objects without fetching.
- [x] `sniff repo package-dependencies` matches the old `deps` behavior.
- [x] `sniff repo dependencies` filters external dependency classes correctly.
- [x] `sniff repo version --json` returns this Rust monorepo's manifest version or null only if the selected manifest truly has no version.

## Phase 6 — Bare `sniff repo --json` Aggregate Redesign

Purpose: replace the heavy, mixed-key aggregate with the consolidated `SniffRepo` shape.

- [x] Define aggregate-only serde/projection types for the new `SniffRepo`, `ScopeBucket`, lean `git_status`, flattened `worktrees`, `BranchInfo`, commit families, context, package dependencies, and external dependencies.
- [x] Standardize aggregate top-level keys to `snake_case`, including `is_monorepo`, `package_count`, `git_status`, `recent_commits`, `source_code_changes`, and `documentation_changes`.
- [x] Group cwd-relative facts under `context`: `package`, `package_area`, `area`, `package_root`, `package_area_root`, `worktree`, `is_current_package_area_dirty`, and `package_area_has_source_code_changes`.
- [x] Replace the 13 change-family wrapper entries with four `ScopeBucket` entries: `dirty`, `staged`, `unstaged`, and `untracked`.
- [x] Include `files`, `source_code`, `documentation`, `packages`, and `package_areas` arrays in each `ScopeBucket`, using empty arrays for no data.
- [x] Add top-level `branches` from library branch data.
- [x] Flatten `worktrees` to a single top-level array carrying the useful union of fields; remove double nesting and aggregate `git_status.worktrees`.
- [x] Slim aggregate `git_status` to `current_branch`, `config`, `file_changes`, `is_dirty`, `staged_count`, `unstaged_count`, and `untracked_count`.
- [x] Collapse each aggregate `file_changes` entry to a single status/action field with line counts; keep richer focused command shapes unchanged.
- [x] Strip `filter`, `repo_root`, and embedded package catalogs from aggregate commit families.
- [x] Use structured `period` data for commit families if available; otherwise keep the minimum non-duplicative period representation.
- [x] Fix documentation-change package/package-area attribution so markdown files under a package area map the same way source files do.
- [x] Keep `remote`, `pr`, `hash`, and recursive/default query surfaces excluded from the aggregate.
- [x] Add aggregate tests asserting no duplicated full package catalog under `structure`, `package_dependencies`, or `recent_commits`.
- [x] Add aggregate tests asserting `snake_case` keys and absence of old kebab-case aggregate keys.
- [x] Add byte-size regression test or fixture assertion showing a material reduction from the measured baseline.

**Parallelizable**

- [x] Scope bucket construction, worktree projection, and commit-family projection can be implemented independently once the aggregate type is defined.
- [x] JSON fixture assertions can be written in parallel from the expected `SniffRepo` contract.

**Validation checkpoint 6**

- [x] `sniff repo --json` stdout validates as JSON and matches the new consolidated shape.
- [x] The aggregate has one package-name list and no repeated full package catalog.
- [x] Worktrees and branches appear once at top level.
- [x] Change scopes are four flat `ScopeBucket` objects with empty arrays where appropriate.

## Phase 7 — CLI Output, Formatting, And Focused Command Contracts

Purpose: polish command behavior, output modes, and focused command isolation after the library and aggregate changes land.

- [x] Ensure every new or moved command supports existing global `--json` and `--plain` behavior.
- [x] Render new text outputs with `biscuit-terminal` `Renderable` components or `Prose`.
- [x] Add CSV/list/markdown output support for `repo package-manager` and `repo test-runner` where variance can produce multiple values.
- [x] Verify `--json` modes do not write legends, hints, or progress lines to stdout.
- [x] Update `sniff topics` or topic listings if they enumerate command groups.
- [x] Update shell completion generation expectations if command snapshots cover completions.
- [x] Ensure clap help shows `software` as the only installed-program parent and does not list removed top-level categories.
- [x] Ensure focused rich commands (`repo git-status --json`, `repo recent-commits --json`, `repo structure --json`, `repo package-dependencies --json`) keep their intended focused shape unless specifically renamed or fixed by the feature.

**Parallelizable**

- [x] Output snapshot rebaselining can proceed in parallel with docs updates after command behavior stabilizes.

**Validation checkpoint 7**

- [x] CLI tests pass for help, JSON, plain output, negative removed commands, and new command paths.
- [x] Manual smoke commands produce valid output: `sniff software`, `sniff software test-runners`, `sniff repo branches`, `sniff repo dependencies`, `sniff repo package-dependencies`, `sniff repo package-manager`, `sniff repo test-runner`, and `sniff repo --json`.

## Phase 8 — In-Repo Consumer Migration, Docs, And Skill Updates

Purpose: complete the coordinated hard break across the monorepo.

- [x] Update every in-repo invocation of removed installed-program top-level commands to the `sniff software` form.
- [x] Update every in-repo invocation of `sniff repo deps` to `sniff repo package-dependencies`.
- [x] Update every in-repo consumer of bare `sniff repo --json` to read `snake_case` keys and the new grouped `context` / `ScopeBucket` shape.
- [x] Update `claudine` consumers of `sniff repo --json` in the same change, removing reads of old kebab-case keys and removed embedded package catalogs.
- [x] Update `sniff/cli/README.md` and any command docs for `software`, `software test-runners`, `repo branches`, `repo package-manager`, `repo test-runner`, `repo dependencies`, `repo package-dependencies`, and `repo version`.
- [x] Update `sniff/lib/README.md` if public library types or package fields changed.
- [x] Update `sniff/docs/dependencies.md` or area dependency docs if new crates are added or removed.
- [x] Update `.claude/skills/sniff/SKILL.md` so the CLI examples and `sniff repo --json` aggregate description match the new hard-break contract.
- [x] Update feature docs or comments that mention old `programs`, `repo deps`, `workspace_tools`, `monorepo_tool`, or duplicated aggregate behavior.
- [x] Apply comment-quality discipline: fix or delete drifted docs/comments adjacent to changed symbols, but avoid unrelated cleanup.

**Parallelizable**

- [x] Consumer migration, README updates, and skill updates can run in parallel after final JSON and CLI command names are stable.

**Validation checkpoint 8**

- [x] `git grep` finds no remaining in-repo calls to removed command paths except documentation explicitly describing the break.
- [x] `git grep` finds no consumer reads of old aggregate kebab-case keys except migration notes or tests that assert absence.
- [x] Documentation and skill examples match executable command behavior.

## Phase 9 — Final Verification And Release Readiness

Purpose: prove the hard break is coherent, tested, and safe to hand to downstream implementers/users.

- [ ] Run formatting for touched Rust crates.
- [ ] Run targeted library tests: `cargo test -p sniff` or the repo's preferred `just`/`nextest` equivalent for Sniff.
- [ ] Run targeted CLI tests: `cargo test -p sniff-cli` or the repo's preferred `just`/`nextest` equivalent for Sniff CLI.
- [ ] Run targeted consumer tests for migrated in-repo packages, especially `claudine`, if they compile against the changed JSON shape.
- [ ] Run targeted builds: `cargo build -p sniff`, `cargo build -p sniff-cli`, and any migrated consumer package builds.
- [ ] Run command smoke checks with `--json` through a JSON validator for `sniff software test-runners`, `sniff repo branches`, `sniff repo dependencies`, `sniff repo package-dependencies`, `sniff repo package-manager`, `sniff repo test-runner`, `sniff repo version`, and bare `sniff repo --json`.
- [ ] Measure `sniff repo --json` byte size against the Phase 1 baseline and record the reduction.
- [ ] Confirm no network requests occur during bare `sniff repo --json` or default `sniff repo branches`.
- [ ] Confirm removed paths fail: old installed-program top-level commands and `sniff repo deps`.
- [ ] Review git diff for unrelated refactors, accidental formatting churn, stale comments, and unintended focused JSON contract changes.

**Validation checkpoint 9**

- [ ] All targeted tests/builds pass or any pre-existing failures are documented with exact commands and failure summaries.
- [ ] Hard-break acceptance criteria from the spec are satisfied.
- [ ] The feature plan, docs, and skill catalog agree with the implemented CLI/library surface.
