---
agent: gemini
phases: 5
start_phase: 1
created: 2026-05-04T19:55:31
source_files_during_phase_1:
  - sniff/lib/src/programs/contract.rs
  - sniff/lib/src/programs/types.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/programs/enums/categories.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/enums/mod.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/find_program.rs
  - sniff/lib/src/programs/ai_cli.rs
  - sniff/lib/src/programs/notification_helpers.rs
  - sniff/lib/src/programs/tts_clients.rs
  - sniff/lib/src/programs/install_plan.rs
  - sniff/lib/src/programs/installer.rs
  - sniff/lib/src/programs/install_interview.rs
  - sniff/lib/src/programs/editors.rs
  - sniff/cli/src/perf.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/main.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/filesystem/path_kind.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/blast_radius.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/package/dependency.rs
  - sniff/lib/src/package/mod.rs
  - sniff/lib/src/package/network.rs
  - sniff/lib/src/executable_index.rs
  - sniff/lib/src/lib.rs
  - sniff/lib/src/programs/find_program.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/types.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/lib/src/programs/notification_helpers.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/programs/windows_apps.rs
  - sniff/lib/src/os/package_manager.rs
  - sniff/lib/src/os/mod.rs
  - sniff/cli/src/output/commit_blocks.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/benches/cases/inventory.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - sniff
  - sniff-cli
---

# Refactoring Plan: Sniff Package Area (2026-05-04)

This plan addresses the 20 suggestions from the Sentrux structural review, focusing on breaking critical cycles, resolving layer inversions, and decomposing "god modules" in both the library and CLI.

## Phase 1: Breaking Critical Cycles
*Goal: Resolve immediate acyclicity violations that hinder stability and testing.*

- **1.1: Resolve `programs` Library Cycle**
  - Create `sniff/lib/src/programs/contract.rs` as a leaf module.
  - Move `InstallationMethod`, `SystemPrerequisite`, `ProgramMetadata`, and `ProgramError` into `contract.rs`.
  - Update `types.rs` and `schema.rs` to import from `contract.rs`.
- **1.2: Resolve `programs` Enums Cycle**
  - Move `CategoryEnum` into `programs/contract.rs`.
  - Update `types.rs` to depend only on `contract.rs` for category traits.
- **1.3: Resolve CLI Commands/Output Cycle**
  - Create `sniff/cli/src/perf.rs` (or `runtime.rs`).
  - Move `CliPerf` and `handle_no_results` from `commands.rs` to `perf.rs`.
  - Update `commands.rs` and `output/recent_commits.rs` to import from the new leaf.

## Phase 2: Resolving Layer Inversions & Cross-Module Leaks
*Goal: Restore natural layering and clean boundaries between core modules.*

- **2.1: Fix `git` / `blast_radius` Inversion**
  - Create `sniff/lib/src/filesystem/path_kind.rs`.
  - Move `is_documentation_path` and `is_source_code_path` from `blast_radius.rs` to `path_kind.rs`.
  - Update `git/recent_commits.rs` and `blast_radius.rs` to use the new leaf.
- **2.2: Fix `package` / `filesystem` Leak**
  - Move `DependencyEntry` from `filesystem/repo/types.rs` to `sniff/lib/src/package/dependency.rs`.
  - Update `filesystem/repo` to import from `package`.
- **2.3: Fix `os` / `programs` Inversion**
  - Move `ExecutableIndex` from `programs` to a neutral leaf (e.g., `sniff/lib/src/executable_index.rs`).
  - Update `os/package_manager.rs` and `programs` modules to import from the root leaf.

## Phase 3: Decomposing Library God Modules
*Goal: Reduce mass concentration and improve modularity in the core library.*

- **3.1: Split `filesystem/repo/detection.rs`**
  - Create `filesystem/repo/` submodule tree.
  - Extract logic into `cargo.rs`, `npm.rs`, `python.rs`, `go.rs`, `nx_turbo.rs`, and `manifest_index.rs`.
- **3.2: Split `filesystem/git/detection.rs`**
  - Move logic into `discovery.rs`, `status.rs`, `diff.rs`, and `remote_refresh.rs`.
- **3.3: Split `programs/types.rs`**
  - Decompose into `category_detector.rs`, `install_method.rs`, `prerequisite.rs`, and `source.rs`.
- **3.4: Split `services/mod.rs`**
  - Extract init-specific logic into `launchd.rs`, `systemd.rs`, `openrc.rs`, and `runit.rs`.

## Phase 4: Restructuring Programs & Installers
*Goal: Group related logic and collapse redundant category definitions.*

- **4.1: Migrate to `programs/install/` Submodule**
  - Group `installer.rs`, `install_plan.rs`, and `install_interview.rs` into `programs/install/`.
  - Split into `plan.rs`, `command.rs`, `execute.rs`, `interview.rs`, and `options.rs`.
- **4.2: Consolidate Program Categories**
  - Create `programs/categories.rs` and collapse near-empty per-category files.
  - Delete/Inline redundant boolean accessors.
- **4.3: Collapse `programs/pkg_mngrs.rs`**
  - Inline aliases into `programs/mod.rs` or `categories.rs`.

## Phase 5: Decomposing CLI God Modules
*Goal: Restructure the CLI binary to match modern modularity standards.*

- **5.1: Split `cli/src/output/filesystem.rs`**
  - Decompose into `repo.rs`, `packages.rs`, `package_areas.rs`, `deps.rs`, `language.rs`, `files.rs`, and `docs.rs`.
- **5.2: Split `cli/src/args.rs`**
  - Group clap definitions into `args/` submodule (e.g., `repo.rs`, `files.rs`, `docs.rs`, `install.rs`).
- **5.3: Split `cli/src/commands.rs`**
  - Extract command-family handlers into `commands/` submodule (e.g., `repo.rs`, `files.rs`, `shorthand.rs`).
- **5.4: Refactor `cli/src/output/repo_json.rs`**
  - Co-locate JSON logic with text rendering or introduce `RepoView` abstraction.
- **5.5: Slim down `cli/src/output/mod.rs`**
  - Move helper functions to `output/render.rs`.
