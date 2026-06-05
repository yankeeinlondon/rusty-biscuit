---
phases: 6
created: 2026-06-04
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - phase-1-inventory.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/composition/select.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/tests/argv_normalization.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/sequence.rs
  - claudine/cli/tests/wrap_commands.rs
  - claudine/cli/tests/level2_dry_run_metadata_capture.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/cli/src/commands/wrap/composition/dry_run.rs
  - claudine/cli/tests/level2_dry_run_metadata_capture.rs
docs_updated_during_phase_6:
  - claudine/docs/topics/composition.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - claudine
  - claudine-cli
---

# Implementation Plan -- `--dry-run` Agent Resolution Touch-up

Converts [`spec.md`](spec.md) into an ordered execution plan. The core success condition is that the dry-run metadata table becomes a faithful prediction of live provider selection for every `agent` frontmatter state, while preserving the requested dry-run formatting.

## Phase 1 -- Grounding and State Inventory

- [ ] Inspect the current composition flow in `claudine/lib/src/composition/prepare.rs`, `claudine/cli/src/commands/wrap/composition/mod.rs`, and `claudine/cli/src/commands/wrap/composition/dry_run.rs`; record where agent frontmatter is parsed, where invalid values currently become fatal, where provider selection prompts, and where dry-run returns.
- [ ] Inspect existing dry-run L1 and L2 tests, especially the `level2_dry_run_*` pattern, and identify reusable fixtures for compose, inline-compose, and sequence.
- [ ] Confirm how installed providers are represented today and how provider validity differs from host installation state.
- [ ] Confirm the channel used by the existing `choose_one` prompt and styled status/error output so the new TTY gate can key off stderr rather than stdout.
- [ ] Confirm whether the biscuit-terminal table component preserves embedded newlines and bulleted lists inside one value cell without breaking two-column alignment.
- [ ] Validation checkpoint: write a short implementation note in the PR or feature branch summary naming the exact call sites that will change and the exact test files that will cover them.

## Phase 2 -- Agent Resolution Data Model

- [x] Add an `AgentResolutionState` enum that can represent selected provider, no-agent, single-invalid, single-not-installed, list with one installed provider, list with multiple installed providers, list with invalid suggestions, and zero-installed-list.
- [x] Add structured fields for selected provider, suggested installed providers, suggested not-installed providers, invalid suggestions, and source context needed for user-facing messages.
- [x] Replace the dry-run renderer's current `agent: Option<Provider>` input with `AgentResolutionState`.
- [x] Add pure helper functions that classify frontmatter `agent` values against the known provider catalog and the installed-provider set.
- [x] Make invalid frontmatter agent values non-fatal in `prepare.rs` for both single values and list entries, routing them into the new resolution state instead of `CompositionError::AgentHintInvalid`.
- [x] Preserve existing behavior for valid installed single-agent selection so already-working direct cases do not regress.
- [x] Validation checkpoint: targeted L1 tests cover classification for single valid installed, single invalid, single valid not-installed, mixed lists, all-invalid lists, all-not-installed lists, and mixed invalid plus not-installed lists.

## Phase 3 -- Live Provider Selection Behavior

- [x] Move or short-circuit the dry-run seam so `--dry-run` never invokes the interactive provider picker and instead records unresolved states for rendering.
- [x] Implement the TTY-only prompting gate for every prompting state using the prompt/output channel's TTY status, not `InteractiveSchemaOptions::allowed()`.
- [x] Implement live no-agent behavior: TTY prompts over all installed agents; no TTY emits the styled no-agent message to stderr and exits non-zero.
- [x] Implement live single-invalid behavior: invalid value is non-fatal; TTY emits the styled `Invalid Agent:` message then prompts over all installed agents; no TTY emits the same message to stderr and exits non-zero.
- [x] Implement live single-not-installed behavior: TTY prompts over all installed agents; no TTY emits the styled `Agent Not Installed:` message to stderr and exits non-zero.
- [x] Implement live list behavior for exactly one valid installed provider: silently auto-select that provider with no prompt.
- [x] Implement live list behavior for two or more valid installed providers: TTY prompts scoped to suggested installed providers; no TTY emits the styled state message to stderr and exits non-zero.
- [x] Implement live zero-installed-list behavior: TTY emits the styled zero-installed-list message then prompts over all installed agents; no TTY emits the same message to stderr and exits non-zero.
- [x] Verify `--silent` has no effect on whether these states prompt, report, or abort.
- [x] Validation checkpoint: L1 tests cover every live-path state, picker scope, no-TTY non-zero exit behavior, stderr messaging, no generic resolver error leakage, and `--silent` invariance.

## Phase 4 -- Dry-run Agent Cell Rendering

- [x] Render a selected valid installed agent in the `Agent` metadata row by provider name.
- [x] Render the no-agent state as the specified unordered list inside the single `Agent` value cell.
- [x] Render single-invalid as `Invalid Agent(<agent>)` plus the specified explanatory italic text inside the single `Agent` value cell.
- [x] Render single-not-installed as `Agent Not Installed:(<agent>)` plus the specified explanatory italic text inside the single `Agent` value cell.
- [x] Render list-with-multiple-installed as the green interactive-choice header plus a bulleted suggested-agent list, dimming valid but not-installed suggestions.
- [x] Render list-with-one-installed as the green auto-select header plus explanatory line, including invalid suggestions when present.
- [x] Render invalid list entries with the `NOT valid` header and invalid-suggestion bullets wherever applicable.
- [x] Render zero-installed-list as the zero-installed header, dimmed suggested-but-not-installed bullets, and invalid suggestions where present.
- [x] Keep all multiline breakdown content inside the existing `Agent` table row's value cell; do not add extra metadata rows.
- [x] Validation checkpoint: L1 dry-run renderer tests assert semantic content and ordering for every agent state, including multiline cell content and one-row table shape.

## Phase 5 -- Structural Dry-run Formatting

- [x] Add a full-width horizontal rule after the composed prompt output and before resolved frontmatter/metadata in single-document dry-run output.
- [x] Add per-prompt horizontal-rule delimiters for sequence dry-run output.
- [x] Add the `Frontmatter (resolved):` heading before the YAML block with bold `Frontmatter`, italic `resolved`, and bottom margin of 1.
- [x] Render resolved YAML frontmatter with inverse-theme code highlighting and a `1ch` left margin.
- [x] Render the metadata table with a top margin of 1 after the YAML frontmatter block.
- [x] Confirm stdout remains the composed prompt/body data and stderr contains the frontmatter, horizontal rules, metadata table, and status/error presentation.
- [x] Validation checkpoint: L1 structural tests assert presence and ordering of prompt, horizontal rule, heading, YAML block, metadata table, and sequence delimiters.

## Phase 6 -- L2 Styling, Regression Coverage, and Documentation

- [x] Add or update real-terminal L2 captures for red invalid-agent styling using `frame.raw` with semantic SGR assertions.
- [x] Add or update L2 captures for yellow and dim not-installed styling using `frame.raw` with semantic SGR assertions.
- [x] Add or update L2 captures for horizontal-rule rendering in single-document and sequence dry-run output. *(Note: tmux strips Kitty graphics images from pane captures; the hr is covered by L1 integration tests.)*
- [x] Add or update L2 captures for inverse-theme YAML highlighting and the `Frontmatter (resolved):` heading spacing.
- [x] Add regression tests proving multiline styled `Agent` cells preserve two-column table alignment and the `1ch` visual offset.
- [x] Update user-facing docs only if public dry-run behavior or live agent-resolution behavior is documented outside this feature spec.
- [x] Run targeted validation: `cargo test -p claudine-cli dry_run`, targeted composition/agent-resolution tests, and the relevant L2 dry-run tests.
- [x] Run a final targeted build for the affected package area, normally `cargo build -p claudine-cli`.
- [x] Validation checkpoint: all acceptance criteria from the spec are mapped to passing L1 or L2 tests, and any skipped L2 assertions are documented with the reason and replacement coverage.

## Parallelization Notes

- [ ] Phase 1 investigation tasks can run in parallel across CLI flow, library prepare flow, and existing tests.
- [ ] After Phase 2 defines `AgentResolutionState`, Phase 3 live behavior and Phase 4 dry-run rendering can proceed in parallel if both use the same enum contract.
- [ ] Phase 5 formatting can proceed in parallel with Phase 3 live behavior once the dry-run seam location is decided.
- [ ] Phase 6 L2 styling work can start as soon as the corresponding Phase 4 and Phase 5 render paths exist; it does not need to wait for all live-path behavior tests.
