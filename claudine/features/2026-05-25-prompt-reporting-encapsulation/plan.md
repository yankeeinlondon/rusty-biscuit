---
phases: 5
created: 2026-05-25
start_phase: 1
source_files_during_phase_1:
  - claudine/lib/src/system_prompt/types.rs
  - claudine/lib/src/system_prompt/mod.rs
  - claudine/lib/src/system_prompt/prepare.rs
  - claudine/lib/src/prompt_reporting/system_prompt.rs
  - claudine/cli/src/output/mod.rs
  - claudine/cli/src/commands/wrap/system_prompt.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/system_prompt_perf_bench.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_2:
  - claudine/lib/src/prompt_reporting/types.rs
  - claudine/lib/src/prompt_reporting/frontmatter.rs
  - claudine/lib/src/prompt_reporting/precedence.rs
  - claudine/lib/src/prompt_reporting/mod.rs
  - biscuit-tui/lib/src/core/standalone/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - claudine
  - claudine-cli
  - tui-chrome
---

# Plan: Prompt Reporting Encapsulation (Stage 0)

Encapsulate the prompt reporting logic into two primary report types and a unified `ReportMode` enum, reducing public surface and simplifying call sites.

## Phase 1: Mechanical Rename of `EffectiveSystemPrompt`

Rename the core system prompt type to better reflect its role and align with future stages.

- [ ] Rename `EffectiveSystemPrompt` to `ResolvedSystemPrompt` in `claudine/lib/src/system_prompt/types.rs`.
- [ ] Update all references in `claudine/lib/src/system_prompt/prepare.rs`.
- [ ] Update all references in `claudine/lib/src/prompt_reporting/system_prompt.rs`.
- [ ] Update all references in `claudine/cli/src/commands/wrap/mod.rs`.
- [ ] Update all references in `claudine/cli/src/commands/wrap/system_prompt.rs`.
- [ ] Update all references in `claudine/cli/src/commands/wrap/composition/mod.rs`.
- [ ] Update all references in `claudine/cli/src/output/mod.rs`.
- [ ] Update all references in `claudine/cli/tests/system_prompt_perf_bench.rs`.
- [ ] **Validation:** Run `cargo check -p claudine` and `cargo check -p claudine-cli` to ensure no broken references.

## Phase 2: Unified Verbosity and Precedence

Introduce the `ReportMode` enum and update the precedence resolvers to return it.

- [ ] Define `ReportMode` enum in `claudine/lib/src/prompt_reporting/types.rs`.
    - Variants: `Silent`, `Summary`, `Partial { truncation: TruncationMode }`, `Full`.
- [ ] Update `claudine/lib/src/prompt_reporting/frontmatter.rs` to return `Option<ReportMode>`.
    - Map `"silent"` → `Silent`, `"quiet"` → `Summary`, `"verbose"` → `Full`.
- [ ] Update `claudine/lib/src/prompt_reporting/precedence.rs` to use `ReportMode`.
    - Implement `resolve_system_prompt_report_mode`.
    - Implement `resolve_agent_prompt_report_mode`.
- [ ] **Validation:** Run `cargo test -p claudine --lib prompt_reporting::precedence` and `prompt_reporting::frontmatter`.

## Phase 3: Report Type Implementation

Implement the encapsulated report types and move internal logic to private helpers.

- [ ] Implement `SystemPromptReport` in `claudine/lib/src/prompt_reporting/system_prompt.rs`.
    - Include `new` and `render` methods.
    - Encapsulate `report_system_prompt` and `report_system_prompt_empty` logic.
- [ ] Implement `AgentPromptReport` in `claudine/lib/src/prompt_reporting/user_prompt.rs`.
    - Include `new` and `render` methods.
    - Note: This replaces `UserPromptReportConfig` and associated helpers.
- [ ] Make all header/summary/body builders and block-quote helpers module-private in `prompt_reporting`.
- [ ] **Validation:** Ensure `claudine` library compiles with `cargo check -p claudine`.

## Phase 4: CLI Call-site Migration

Simplify the CLI output logic by using the new encapsulated API.

- [ ] Update `claudine/cli/src/output/mod.rs`:
    - Refactor `log_system_prompt_with_scope` to use `SystemPromptReport`.
    - Refactor `log_compose_prompt` to use `AgentPromptReport`.
    - Remove dual-entry-point branching and complex boolean logic.
- [ ] **Validation:** Run `cargo check -p claudine-cli`.

## Phase 5: Final Cleanup and Verification

Clean up public exports and verify the complete system.

- [ ] Update `claudine/lib/src/prompt_reporting/mod.rs` to limit re-exports to the 7 symbols identified in the spec.
- [ ] Remove deprecated types: `SystemPromptReportConfig`, `UserPromptReportConfig`, `PromptReportFormat`, `PromptVerbosity`.
- [ ] Run all tests in the `claudine` workspace.
- [ ] Verify documentation with `cargo doc -p claudine` (ensure no broken intra-doc links).
- [ ] **Checkpoint:** All acceptance criteria from the spec are met.
