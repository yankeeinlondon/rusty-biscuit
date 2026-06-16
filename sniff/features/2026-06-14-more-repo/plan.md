---
agent: codex/
phases: 9
created: 2026-06-16
start_phase: 1
yolo: "true"
packages:
  - sniff
  - sniff-cli
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

- [ ] Run `sniff repo` or `cargo metadata --no-deps --format-version 1` as needed to confirm workspace package names and targeted `-p` values.
- [ ] Inspect current CLI command wiring for installed-program commands, repo subcommands, and aggregate JSON builder in `sniff/cli/src/args`, `sniff/cli/src/commands`, and `sniff/cli/src/output`.
- [ ] Inspect library repo detection paths for `Package`, manifest parsing, package-manager detection, version detection, Git branch/worktree data, and dependency extraction.
- [ ] Inspect existing software category architecture: category enums, `ProgramInfo`, `ProgramsInfo::detect`, JSON rendering, markdown/text rendering, and CLI action macros.
- [ ] Audit test fixtures and snapshots for installed-program commands, repo JSON aggregate, repo deps, repo version, worktrees, git status, and package listing.
- [ ] `git grep` in the monorepo for removed or renamed invocations: `sniff programs`, `sniff editors`, `sniff utilities`, `sniff language-package-managers`, `sniff os-package-managers`, `sniff tts-clients`, `sniff terminal-apps`, `sniff audio-players`, `sniff notification-helpers`, `sniff agents`, and `sniff repo deps`.
- [ ] `git grep` for `repo --json` consumers, especially in `claudine`, and record the fields they consume.
- [ ] Capture a baseline `sniff repo --json` fixture for this repo, including byte size, top-level keys, duplicated package catalogs, worktree duplication, and change-family envelopes.
- [ ] Confirm with tests or manual inspection that `sniff repo version` currently fails or returns null for the Rust monorepo case described in the spec.
- [ ] Document any pre-existing failing tests before code changes so later validation can distinguish regressions from baseline noise.

**Parallelizable**

- [ ] Run call-site grep/audit in parallel with code inspection.
- [ ] Capture baseline JSON metrics in parallel with snapshot inventory.

**Validation checkpoint 1**

- [ ] Baseline command/JSON contracts are recorded in implementation notes or test fixtures.
- [ ] All in-repo call sites requiring migration are identified.
- [ ] Out-of-scope commands are explicitly left unimplemented.

## Phase 2 — Software Command Reparenting

Purpose: move the installed-program surface under `sniff software` and remove old top-level paths.

- [ ] Add a `software` top-level command with an aggregate action equivalent to the current `sniff programs` behavior.
- [ ] Move existing installed-program category commands under `sniff software`: `editors`, `utilities`, `language-package-managers`, `os-package-managers`, `tts-clients`, `terminal-apps`, `audio-players`, `notification-helpers`, and `agents`.
- [ ] Remove old top-level installed-program command variants and their routing paths entirely, including help output and completion-visible command entries.
- [ ] Preserve existing text, plain, and JSON output behavior for the reparented categories except for the command path.
- [ ] Ensure `sniff software --json` emits the same aggregate data shape that `sniff programs --json` emitted before adding test runners.
- [ ] Update CLI help/snapshot tests so removed top-level paths are absent and new `software` paths are present.
- [ ] Add negative CLI tests showing old top-level command paths fail with clap usage errors rather than silently aliasing.

**Parallelizable**

- [ ] Snapshot/help updates can run in parallel with call-site migration from Phase 8 after command wiring compiles.

**Validation checkpoint 2**

- [ ] `cargo build -p sniff-cli` succeeds.
- [ ] `sniff software --json` stdout is valid JSON.
- [ ] Old top-level installed-program paths are not accepted.

## Phase 3 — Test Runner Catalog And Host Availability

Purpose: add the ninth software category and host availability model in the library.

- [ ] Add the `TestRunner` enum with the full v1 catalog from `test-runner-strategy.md`.
- [ ] Add `TEST_RUNNER_INFO` / metadata so `TestRunner` participates in the same category machinery as existing software categories.
- [ ] Add `InvocationClass`, `RunnerKind`, `TestRunnerSpec`, and metadata fields for ecosystem, parent binary, dependency keys, config globs, and ecosystem defaults.
- [ ] Extend `ExecutableSource` with `ProjectLocal { root }` and any required serialization/output support.
- [ ] Implement `LocalBinIndex` or an `ExecutableIndex` extension for project-local bin search: Node `node_modules/.bin` walk-up, PHP `vendor/bin`, Python virtualenv directories including Windows `Scripts`, Ruby `bin`, and `$VIRTUAL_ENV`.
- [ ] Implement host resolution order in the library: project-local bin, global PATH index, parent binary for class B/C, then not found.
- [ ] Add `availability` to test-runner program output with `installed`, `local`, `via_parent`, and `not_found` cases.
- [ ] Add `test_runners` to `ProgramsInfo` and detect it in parallel with the shared executable index.
- [ ] Wire `sniff software test-runners` through CLI args, actions, JSON output, and terminal rendering.
- [ ] Render availability details through `biscuit-terminal` components, not hand-written ANSI.
- [ ] Add cross-platform tests for executable suffix handling and local-bin search roots, using fixture directories rather than host-global assumptions.

**Parallelizable**

- [ ] Catalog metadata table work can proceed in parallel with local-bin resolver implementation.
- [ ] CLI wiring can proceed after the `ProgramsInfo.test_runners` field shape is known.

**Validation checkpoint 3**

- [ ] `cargo test -p sniff` passes for test-runner metadata and local-bin resolution.
- [ ] `cargo test -p sniff-cli` or targeted CLI tests pass for `sniff software test-runners`.
- [ ] `sniff software test-runners --json` stdout includes availability discriminators and remains valid JSON.

## Phase 4 — Repo Test Runner Usage And Shared Collapse Logic

Purpose: detect declared test runner usage per package and expose context-aware repo aggregation.

- [ ] Add typed `TestRunnerUsage` and `TestRunnerSource` to the library with evidence sources: manifest key, config file, ecosystem default, and convention.
- [ ] Add `test_runners: Vec<TestRunnerUsage>` to `Package` and update serde snapshots/fixtures as needed.
- [ ] Extend manifest cache/index support for runner detection inputs not currently parsed: `composer.json`, `.csproj`, `pom.xml`, `build.gradle[.kts]`, `mix.exs`, `Gemfile`, `*.gemspec`, and Python requirements files where appropriate.
- [ ] Implement `detect_test_runners(pkg_dir, cache)` as a sibling to `detect_package_managers()`.
- [ ] Apply runner signal priority per package: config file, manifest dependency key, ecosystem default, then convention.
- [ ] Record orchestrators such as `tox` and `nox` with `kind: orchestrator`, not as plain runner strings.
- [ ] Add focused package-level tests for Rust, Go, JS/TS, Python, PHP, Ruby, JVM, .NET, and Elixir detection cases from the strategy table.
- [ ] Add a shared library aggregation helper for package-context, package-area-context, and repo-root collapse logic.
- [ ] Refactor existing package-manager reporting logic to use the shared aggregation helper.
- [ ] Add `sniff repo test-runner` CLI with default text plus `--csv`, `--list`, `--md`, and `--json` output.
- [ ] Ensure CLI output reports library-provided runner values and evidence where JSON supports it; do not re-detect in the CLI.

**Parallelizable**

- [ ] Ecosystem-specific fixture tests can be implemented independently once `TestRunnerUsage` is defined.
- [ ] Package-manager collapse refactor can proceed in parallel with runner signal parsing.

**Validation checkpoint 4**

- [ ] `cargo test -p sniff` passes package runner detection and aggregation tests.
- [ ] `sniff repo test-runner --json` reports typed usage from the current repo/package context.
- [ ] `sniff repo package-manager` still matches previous behavior for uniform and variant package sets.

## Phase 5 — Repo Local Commands And Library Data

Purpose: add the new repo commands and fix existing local repo facts.

- [ ] Implement library `BranchInfo` projection with `name`, `current`, `sha`, `remote_represented`, `upstream`, `ahead`, and `behind`.
- [ ] Ensure branch detection uses locally known refs only by default and does not fetch.
- [ ] If `--refresh-remotes` is added, route through existing non-interactive remote refresh code and set `GIT_TERMINAL_PROMPT=0`.
- [ ] Add `sniff repo branches` CLI with JSON array output and terminal rendering through `biscuit-terminal`.
- [ ] Keep branch list on stdout; suppress any legend in `--json` mode.
- [ ] Rename focused internal workspace dependency graph command from `sniff repo deps` to `sniff repo package-dependencies`.
- [ ] Remove `sniff repo deps` entirely; add negative tests proving it is not an alias.
- [ ] Add `sniff repo dependencies` for external dependencies with filters for `--dependencies`, `--dev-dependencies`, `--peer-dependencies`, and `--optional-dependencies`.
- [ ] Ensure `sniff repo package-dependencies` preserves current internal graph behavior, including Mermaid `--ui`.
- [ ] Fix repo version detection in the library for Cargo root package/workspace-root package, Node `package.json`, Python `pyproject.toml [project].version`, and safe parser-backed ecosystem manifests.
- [ ] Preserve focused `sniff repo version --json` shape `{ "version": string | null }`; missing versions are success with null.
- [ ] Add targeted fixtures for Cargo workspace root version, package root version, Node version, Python version, and null ecosystems.

**Parallelizable**

- [ ] Branch projection can proceed in parallel with dependency command rename.
- [ ] Version detection fixtures can proceed in parallel with external dependency filter wiring.

**Validation checkpoint 5**

- [ ] `sniff repo branches --json` emits an array of `BranchInfo` objects without fetching.
- [ ] `sniff repo package-dependencies` matches the old `deps` behavior.
- [ ] `sniff repo dependencies` filters external dependency classes correctly.
- [ ] `sniff repo version --json` returns this Rust monorepo's manifest version or null only if the selected manifest truly has no version.

## Phase 6 — Bare `sniff repo --json` Aggregate Redesign

Purpose: replace the heavy, mixed-key aggregate with the consolidated `SniffRepo` shape.

- [ ] Define aggregate-only serde/projection types for the new `SniffRepo`, `ScopeBucket`, lean `git_status`, flattened `worktrees`, `BranchInfo`, commit families, context, package dependencies, and external dependencies.
- [ ] Standardize aggregate top-level keys to `snake_case`, including `is_monorepo`, `package_count`, `git_status`, `recent_commits`, `source_code_changes`, and `documentation_changes`.
- [ ] Group cwd-relative facts under `context`: `package`, `package_area`, `area`, `package_root`, `package_area_root`, `worktree`, `is_current_package_area_dirty`, and `package_area_has_source_code_changes`.
- [ ] Replace the 13 change-family wrapper entries with four `ScopeBucket` entries: `dirty`, `staged`, `unstaged`, and `untracked`.
- [ ] Include `files`, `source_code`, `documentation`, `packages`, and `package_areas` arrays in each `ScopeBucket`, using empty arrays for no data.
- [ ] Add top-level `branches` from library branch data.
- [ ] Flatten `worktrees` to a single top-level array carrying the useful union of fields; remove double nesting and aggregate `git_status.worktrees`.
- [ ] Slim aggregate `git_status` to `current_branch`, `config`, `file_changes`, `is_dirty`, `staged_count`, `unstaged_count`, and `untracked_count`.
- [ ] Collapse each aggregate `file_changes` entry to a single status/action field with line counts; keep richer focused command shapes unchanged.
- [ ] Strip `filter`, `repo_root`, and embedded package catalogs from aggregate commit families.
- [ ] Use structured `period` data for commit families if available; otherwise keep the minimum non-duplicative period representation.
- [ ] Fix documentation-change package/package-area attribution so markdown files under a package area map the same way source files do.
- [ ] Keep `remote`, `pr`, `hash`, and recursive/default query surfaces excluded from the aggregate.
- [ ] Add aggregate tests asserting no duplicated full package catalog under `structure`, `package_dependencies`, or `recent_commits`.
- [ ] Add aggregate tests asserting `snake_case` keys and absence of old kebab-case aggregate keys.
- [ ] Add byte-size regression test or fixture assertion showing a material reduction from the measured baseline.

**Parallelizable**

- [ ] Scope bucket construction, worktree projection, and commit-family projection can be implemented independently once the aggregate type is defined.
- [ ] JSON fixture assertions can be written in parallel from the expected `SniffRepo` contract.

**Validation checkpoint 6**

- [ ] `sniff repo --json` stdout validates as JSON and matches the new consolidated shape.
- [ ] The aggregate has one package-name list and no repeated full package catalog.
- [ ] Worktrees and branches appear once at top level.
- [ ] Change scopes are four flat `ScopeBucket` objects with empty arrays where appropriate.

## Phase 7 — CLI Output, Formatting, And Focused Command Contracts

Purpose: polish command behavior, output modes, and focused command isolation after the library and aggregate changes land.

- [ ] Ensure every new or moved command supports existing global `--json` and `--plain` behavior.
- [ ] Render new text outputs with `biscuit-terminal` `Renderable` components or `Prose`.
- [ ] Add CSV/list/markdown output support for `repo package-manager` and `repo test-runner` where variance can produce multiple values.
- [ ] Verify `--json` modes do not write legends, hints, or progress lines to stdout.
- [ ] Update `sniff topics` or topic listings if they enumerate command groups.
- [ ] Update shell completion generation expectations if command snapshots cover completions.
- [ ] Ensure clap help shows `software` as the only installed-program parent and does not list removed top-level categories.
- [ ] Ensure focused rich commands (`repo git-status --json`, `repo recent-commits --json`, `repo structure --json`, `repo package-dependencies --json`) keep their intended focused shape unless specifically renamed or fixed by the feature.

**Parallelizable**

- [ ] Output snapshot rebaselining can proceed in parallel with docs updates after command behavior stabilizes.

**Validation checkpoint 7**

- [ ] CLI tests pass for help, JSON, plain output, negative removed commands, and new command paths.
- [ ] Manual smoke commands produce valid output: `sniff software`, `sniff software test-runners`, `sniff repo branches`, `sniff repo dependencies`, `sniff repo package-dependencies`, `sniff repo package-manager`, `sniff repo test-runner`, and `sniff repo --json`.

## Phase 8 — In-Repo Consumer Migration, Docs, And Skill Updates

Purpose: complete the coordinated hard break across the monorepo.

- [ ] Update every in-repo invocation of removed installed-program top-level commands to the `sniff software` form.
- [ ] Update every in-repo invocation of `sniff repo deps` to `sniff repo package-dependencies`.
- [ ] Update every in-repo consumer of bare `sniff repo --json` to read `snake_case` keys and the new grouped `context` / `ScopeBucket` shape.
- [ ] Update `claudine` consumers of `sniff repo --json` in the same change, removing reads of old kebab-case keys and removed embedded package catalogs.
- [ ] Update `sniff/cli/README.md` and any command docs for `software`, `software test-runners`, `repo branches`, `repo package-manager`, `repo test-runner`, `repo dependencies`, `repo package-dependencies`, and `repo version`.
- [ ] Update `sniff/lib/README.md` if public library types or package fields changed.
- [ ] Update `sniff/docs/dependencies.md` or area dependency docs if new crates are added or removed.
- [ ] Update `.claude/skills/sniff/SKILL.md` so the CLI examples and `sniff repo --json` aggregate description match the new hard-break contract.
- [ ] Update feature docs or comments that mention old `programs`, `repo deps`, `workspace_tools`, `monorepo_tool`, or duplicated aggregate behavior.
- [ ] Apply comment-quality discipline: fix or delete drifted docs/comments adjacent to changed symbols, but avoid unrelated cleanup.

**Parallelizable**

- [ ] Consumer migration, README updates, and skill updates can run in parallel after final JSON and CLI command names are stable.

**Validation checkpoint 8**

- [ ] `git grep` finds no remaining in-repo calls to removed command paths except documentation explicitly describing the break.
- [ ] `git grep` finds no consumer reads of old aggregate kebab-case keys except migration notes or tests that assert absence.
- [ ] Documentation and skill examples match executable command behavior.

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
