# More Repo Feature — Phase 1 Audit Notes

Generated during Phase 1 baseline audit (2026-06-16).

## Workspace Package Names

Confirmed via `cargo metadata --no-deps --format-version 1`. Relevant packages for this feature:

- `sniff` (library)
- `sniff-cli` (CLI)

Both are workspace members. `schematic/schema` remains excluded from the workspace per `AGENTS.md`.

## Current CLI Command Wiring

### Installed-program surface (`sniff/cli/src/args/mod.rs`)

- Top-level `Commands` variants: `Programs`, `Editors`, `Utilities`, `LanguagePackageManagers`, `OsPackageManagers`, `TtsClients`, `TerminalApps`, `AudioPlayers`, `Agents`, `NotificationHelpers`.
- `Commands::is_programs_mode()` returns true for all except `NotificationHelpers` (handled separately).
- `Commands::to_output_filter()` maps each to `OutputFilter::*`.
- `define_program_action!` macro generates per-category `Install`/`InstallPlan` subcommands for: `EditorAction`, `UtilityAction`, `LangPkgMgrAction`, `OsPkgMgrAction`, `TtsClientAction`, `TerminalAppAction`, `AudioAction`, `AgentAction`.
- `AllProgramAction` handles aggregate `sniff software install/install-plan`.

### Repo surface (`sniff/cli/src/args/repo.rs`, `sniff/cli/src/output/repo_json.rs`)

- `RepoSubcommand::Deps` exists with `--ui`, `--svg`, `--filter`, `--package`, `--package-area`, `--width`, `--orientation`.
- `RepoAction::Deps` mirrors it.
- No `Branches`, `TestRunner`, `PackageManager`, or `Dependencies` subcommands yet.
- `RepoAction::Version` exists; JSON shape is `{ "version": string | null }`.
- Bare `sniff repo --json` aggregate is built in `sniff/cli/src/output/repo_json.rs::build_aggregate_value`.

## Library Repo Detection Paths

### Package and version (`sniff/lib/src/filesystem/repo/detection.rs`)

- `resolve_package_version()` reads `Cargo.toml`, `package.json`, `pyproject.toml` only.
- For Cargo it calls `cargo_package_version()` which reads `[package].version` only.
- Workspace-root `Cargo.toml` (with `[workspace]` but no `[package]`) therefore returns `None`.
- `detect_package_managers()` looks at `Cargo.toml`, `package.json` + lockfiles, `requirements.txt`/`pyproject.toml`, `go.mod`.

### Manifest cache (`sniff/lib/src/filesystem/repo/detection.rs`)

- `ManifestCache` currently caches `cargo`, `npm`, `pyproject`, `go_mod`.
- Test-runner detection will need `composer.json`, `*.csproj`, `pom.xml`, `build.gradle[.kts]`, `mix.exs`, `Gemfile`, `*.gemspec`, `requirements*.txt`.

### Manifest index (`sniff/lib/src/filesystem/repo/manifest_index.rs`)

- `ManifestKind` enum: `Cargo`, `Node`, `Python`, `Go`.
- Index walks the tree for `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`.
- Will need extension for PHP/JVM/Ruby/Elixir/.NET manifests.

## Existing Software Category Architecture

### Enums and metadata (`sniff/lib/src/programs/enums/`)

- `categories.rs`: 8 category enums (`Editor`, `Utility`, `LanguagePackageManager`, `OsPackageManager`, `TtsClient`, `TerminalApp`, `HeadlessAudio`, `AiCli`) plus `NotificationHelper`.
- `metadata.rs`: static `ProgramInfo` tables for each category.
- Each enum impls `ProgramMetadata` + `CategoryEnum`.

### Detector (`sniff/lib/src/programs/category_detector.rs`)

- Generic `CategoryDetector<E>` backed by `Vec<Option<(PathBuf, ExecutableSource)>>`.
- Uses shared `ExecutableIndex` (`new_with_index`).
- Serialization produces a map of `serde_key -> ProgramEntry`.

### ProgramsInfo (`sniff/lib/src/programs/mod.rs`)

- 9 fields: `editors`, `utilities`, `language_package_managers`, `os_package_managers`, `tts_clients`, `terminal_apps`, `headless_audio`, `ai_clients`, `notification_helpers`.
- `detect()` builds one `ExecutableIndex` and detects categories in parallel via `rayon::join`.
- New field `test_runners` will be added in Phase 3.

### ExecutableSource (`sniff/lib/src/programs/contract.rs`)

- Variants: `Path`, `MacOsAppBundle`, `WindowsAppPaths`, `WindowsInstallRoot`.
- Phase 3 will add `ProjectLocal { root }`.

## Test Fixtures and Snapshots Inventory

- `sniff/cli/src/output/repo_json.rs` — extensive inline unit tests for aggregate, deps, structure, git-status, package families, locators, booleans.
- `sniff/cli/tests/cli.rs` — end-to-end tests for repo subcommands, program subcommands, aggregate JSON keys.
- `sniff/cli/tests/snapshots/` — help output, structure text/JSON, monorepo snapshots.
- `sniff/lib/src/filesystem/repo/types.rs` — helper tests for `RepoInfo`/`Package`.
- `sniff/lib/src/filesystem/repo/detection.rs` — detection tests (version, package name, etc.).

## In-Repo Call Sites Requiring Migration

### Removed installed-program top-level commands

`git grep` found invocations/docs in:

- `.claude/skills/sniff/SKILL.md`
- `.claude/skills/sniff/programs.md`
- `.claude/skills/sniff/extending.md`
- `.claude/skills/just/SKILL.md`
- `.claude/agents/documenter.md`
- `.claude/agents/just-scripter.md`
- `biscuit-clipboard/docs/research/clipboard-managers.md`
- `claudine/features/_completed/2026-04-17-edit-command/design.md`
- `messenger/features/2026-04-27-leveraging-notification-helpers/plan.md`
- `messenger/features/2026-04-27-leveraging-notification-helpers/tech-design.md`
- `renderable/features/_completed/2026-05-16-iterative-improvement/components/GraphExpression.md`
- `sniff/README.md`
- `sniff/cli/README.md`
- `sniff/docs/cli/repo_deps.md`
- `sniff/reviews/` (historical, completed)

No runtime code invocations (shell scripts, Rust, TS, Python) were found outside documentation/tests.

### Internal dependency graph rename

Found in:

- `.claude/skills/biscuit-visualized/SKILL.md`
- `.claude/skills/biscuit-visualized/graph-rendering.md`
- `renderable/features/_completed/2026-05-16-iterative-improvement/components/GraphExpression.md`
- `sniff/cli/README.md`
- `sniff/cli/src/args/mod.rs` (help text)
- `sniff/cli/src/output/filesystem/deps.rs`
- `sniff/cli/src/output/repo_json.rs`
- `sniff/cli/tests/cli.rs`
- `sniff/docs/cli/repo_deps.md`
- `sniff/features/_completed/2026-04-28-incorrect-json/review-1.md`
- `sniff/features/_completed/2026-04-28-incorrect-json/review-plan-1.md`
- `sniff/features/_completed/2026-04-28-incorrect-json/spec.md`
- `sniff/features/_completed/2026-05-07-repo-package-consistency/plan.md`
- `sniff/reviews/2026-05-05-bench/review.md`

### `repo --json` consumers

No in-repo consumers parsing bare `sniff repo --json` keys were found. Claudine uses `sniff repo` and `sniff repo packages` as text/shell commands, not JSON parsing. No code references to old kebab-case aggregate keys (`is-monorepo`, `package-count`, `package-areas`, `recent-commits`, etc.) outside `sniff/cli` tests and documentation.

## Baseline `sniff repo --json` Metrics

Captured on `rusty-biscuit` repo at commit under `sniff/features/2026-06-14-more-repo/baseline-repo.json`.

- Total bytes: ~2,150,698
- Top-level keys: 37
- Heaviest keys:
  - `recent-commits`: ~677,813 bytes
  - `structure`: ~552,178 bytes
  - `deps`: ~128,406 bytes
  - `source-code-changes`: ~81,994 bytes
  - `documentation-changes`: ~55,186 bytes

### Duplication observed

- Full package catalog serialized in `structure.packages`, `deps.packages`, and `recent-commits.packages`.
- `recent-commits.packages` embeds the full 67-package catalog (~600 KB contributor).
- Worktrees double-nested: top-level `worktrees.worktrees[]`.
- `git-status.worktrees` duplicates worktree data in a different shape (15 entries).
- `git-status.branches` holds 39 branches, buried inside git-status.
- 13 change-family envelope keys (`dirty-files`, `dirty-source-code`, `dirty-packages`, `dirty-package-areas`, `staged-*`, `unstaged-*`, `untracked-files`).

## `sniff repo version` Behavior

Confirmed current failure for Rust monorepo:

```json
{ "version": null }
```

Exit code: 1 (unless `--no-error`). Root cause: `cargo_package_version()` only reads `[package].version`; the workspace root `Cargo.toml` has `[workspace]` but no `[package]`.

## Pre-existing Test Status

`just test sniff` baseline run: **705 tests passed, 0 failed, 2 skipped**. No pre-existing failures to document.

## Out-of-Scope Commands

Per plan and spec, the following remain explicitly out of scope:

- `sniff repo issues`
- `sniff repo ci-cd` / `sniff repo ci`
- Focused `sniff repo is-monorepo` redesign (owned by `2026-06-16-monorepo-cli`)
