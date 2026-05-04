---
phases: 4
created: 2026-05-03
start_phase: 1
source_files_during_phase_1: [sniff/cli/src/args.rs, sniff/cli/tests/cli.rs]
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: [sniff/cli/src/args.rs]
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: [sniff/cli/src/commands.rs, sniff/cli/src/output/mod.rs, sniff/cli/src/output/filesystem.rs, sniff/cli/src/output/repo_json.rs]
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages: [sniff-cli]
---

# Execution Plan: Repo-Language Feature & `--base` Regression Fix

This plan addresses the functional specification for adding the `sniff repo language` command and fixing the `--base` CLI switch regression.

## Phase 1: Fix `--base` Flag Regression
**Objective:** Ensure the `--base` flag works as a global switch for all `sniff` subcommands, including `sniff repo` subcommands.

*   **Step 1:** Modify the `Cli` struct in `sniff/cli/src/args.rs`.
    *   Find the `base` field.
    *   Change the `#[arg(short, long)]` attribute to `#[arg(short, long, global = true)]`.
*   **Step 2:** Validate the fix.
    *   Run `cargo test` in the CLI crate to ensure no existing argument parsing tests break.
    *   Verify manually that `cargo run --bin sniff -- repo --base . root` and `cargo run --bin sniff -- --base . repo root` both work as expected.

## Phase 2: Add `sniff repo language` CLI Command
**Objective:** Expose the new `language` subcommand under the `repo` group.

*   **Step 1:** Update `RepoSubcommand` in `sniff/cli/src/args.rs`.
    *   Add a new variant `Language` to the `RepoSubcommand` enum with documentation: `/// Output the primary programming language for the repository`.
*   **Step 2:** Update `RepoAction` in `sniff/cli/src/args.rs`.
    *   Add a corresponding `Language` variant to the `RepoAction` enum.
*   **Step 3:** Map `RepoSubcommand::Language` to `RepoAction::Language`.
    *   Update the `impl From<RepoSubcommand> for RepoAction` conversion in `sniff/cli/src/args.rs`.

## Phase 3: Implement Backend Logic and Output Rendering
**Objective:** Wire up the CLI command to fetch language data and render it appropriately (both text and JSON).

*   **Step 1:** Update `DetectionPlan` mapping in `sniff/cli/src/commands.rs`.
    *   In `match repo_action`, add `crate::args::RepoAction::Language => plan.filesystem(FilesystemRequest::new().without_docs().without_formatting()),` to ensure language data is collected efficiently.
*   **Step 2:** Implement `render_repo_language` in `sniff/cli/src/output/filesystem.rs`.
    *   Create a function `pub fn render_repo_language(result: &sniff::SniffResult, base_dir: Option<&Path>) -> String`.
    *   Extract `result.filesystem.languages.primary`.
    *   Handle JSON output formatting (`{ "language": "Rust" }`).
    *   Handle standard text output (just the language name).
    *   Handle the case where no primary language is detected.
*   **Step 3:** Wire up rendering in `sniff/cli/src/output/mod.rs`.
    *   In the `match &cli_args.repo_subcommand` block (under `OutputFilter::Repo`), add `Some(RepoAction::Language) => { out.push_str(&filesystem::render_repo_language(result, base_dir)); }`.

## Phase 4: Final Validation
**Objective:** Confirm all features work as specified.

*   **Step 1:** Test text output.
    *   Run `cargo run --bin sniff -- repo language`. Expected: `Rust` (or the primary language).
*   **Step 2:** Test JSON output.
    *   Run `cargo run --bin sniff -- repo language --json`. Expected: `{"language": "Rust"}`.
*   **Step 3:** Test `--base` integration.
    *   Run `cargo run --bin sniff -- repo --base <some-other-repo> language` to ensure it targets the correct directory.
