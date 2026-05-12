---
phases: 6
created: 2026-05-11
start_phase: 6
source_files_during_phase_1:
  - claudine/lib/src/lib.rs
  - claudine/lib/src/prompt_reporting/mod.rs
  - claudine/lib/src/prompt_reporting/types.rs
  - claudine/lib/src/prompt_reporting/frontmatter.rs
  - claudine/lib/src/prompt_reporting/precedence.rs
  - claudine/lib/src/prompt_reporting/tokens.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .opencode/skill/claudine/SKILL.md
source_files_during_phase_2:
  - claudine/lib/src/prompt_reporting/mod.rs
  - claudine/lib/src/prompt_reporting/formatting.rs
  - claudine/lib/src/prompt_reporting/truncation.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/prompt_reporting/mod.rs
  - claudine/lib/src/prompt_reporting/system_prompt.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .opencode/skill/claudine/SKILL.md
source_files_during_phase_4:
  - claudine/lib/src/prompt_reporting/user_prompt.rs
  - claudine/lib/src/prompt_reporting/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/output/mod.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/wrap_commands.rs
  - claudine/cli/tests/snapshots/wrap_commands__wrapper_reports_removed_sensitive_env_names.snap
  - claudine/cli/tests/prompt_reporting.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/lib/src/prompt_reporting/formatting.rs
  - claudine/lib/src/prompt_reporting/system_prompt.rs
  - claudine/lib/src/prompt_reporting/user_prompt.rs
  - claudine/lib/src/prompt_reporting/precedence.rs
  - claudine/lib/src/prompt_reporting/mod.rs
  - claudine/cli/tests/prompt_reporting.rs
  - claudine/cli/tests/snapshots/wrap_commands__wrapper_reports_removed_sensitive_env_names.snap
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - claudine
---
# Prompt Reporting Execution Plan

## Phase 1: Foundation and Types
- [x] Define `PromptReportFormat` enum (`Summary`, `PartialPrompt`, `FullPrompt`) and truncation variants (`Truncate`, `FrontBack`).
- [x] Add parsing for `verbosity` property in frontmatter of `system-prompt.md` files.
- [x] Implement precedence logic for System Prompt resolution: CLI Switches > `CLAUDINE_SYSTEM_PROMPT` env > Prompt Length > Frontmatter > Default (`Summary`).
- [x] Implement token estimation using the `FileTree` utility from `biscuit-terminal` (measuring composed content, omitting agent base prompt).
- [x] Validation Checkpoint: Unit tests for configuration precedence logic and token count estimation.

## Phase 2: Common Formatting Utilities (Parallelizable)
- [x] Implement text truncation utility for `FrontBack` mode (x lines front, 10 lines back with an `hr` marker) ensuring the first/last lines of truncated sections are not blank lines.
- [x] Enhance markdown rendering configuration using `darkmatter` to strictly enforce the constraint of no more than two consecutive blank lines.
- [x] Implement `BlockQuote` styling logic leveraging `Prose` from `biscuit-terminal` that supports custom colored vertical lines (orange/green) and centered icon alignment.
- [x] Validation Checkpoint: Unit tests covering `FrontBack` boundary edge cases and darkmatter blank line collapsing.

## Phase 3: System Prompt Reporting
- [x] Implement Line 1 header logic using `Prose`: `📕 <b>System Prompt(<dim><i>{action}</i></dim>)</b>` where action is `appended` or `replaced`.
- [x] Implement the `Summary` view rendering, including the OS8 hyperlink to the prompt source and formatted token counts.
- [x] Implement the `Partial Prompt` and `Full Prompt` rendering applied inside the orange `BlockQuote`.
- [x] Create the top-level System Prompt reporter that wires together header, resolution precedence, and the selected body mode (suppressing output if `Silent`).
- [x] Validation Checkpoint: Output format tests against various configurations (Env vars, flags like `--silent`, `--quiet`, `--verbose`, frontmatter).

## Phase 4: User Prompt Reporting
- [x] Implement the User Prompt header logic: `🗣️ Agent Prompt`.
- [x] Create User Prompt logic to strip all leading whitespace from the prompt body.
- [x] Implement the 40-line threshold behavior (Full if <=40, `FrontBack` 20/10 if >40 lines) overridden by `--verbose`.
- [x] Wire `--quiet` (suppresses entirely) and `--silent` (suppresses entirely) for the User Prompt.
- [x] Render the User Prompt body within a green `BlockQuote` using the shared darkmatter utilities.
- [x] Validation Checkpoint: Unit tests for User Prompt with both short (<40 lines) and long (>40 lines) bodies, validating correct `FrontBack` usage and flag behavior.

## Phase 5: CLI Integration and Validation
- [x] Integrate the new System and User Prompt reporting structures into the main `claudine` output pipeline.
- [x] Ensure `ComposeArgs` and other command arguments properly propagate `--quiet`, `--verbose`, and `--silent` into the reporting context.
- [x] Validation Checkpoint: End-to-end test verifying visual layout, correct token counting, markdown formatting, and suppression across standard `compose` workflows.

## Phase 6: Spec Revisions (Addendum)

This phase implements the four spec revisions added on 2026-05-12 in response to visual review of the Phase 5 output. The changes refine rendering of the System and User prompts and adjust `--quiet` semantics for the User Prompt. The spec source of truth is `claudine/features/2026-05-10-prompt-reporting/spec.md` (sections "System Prompt → Body" and "User Prompt").

### 6.1 — Single BlockQuote with icon-centered border (System + User)

**Goal:** the entire below-header section of each prompt is rendered as one `BlockQuote` whose `│` border sits at column 1 (centered under the 2-column emoji on the header line).

- [ ] In `formatting.rs`, change `create_system_prompt_blockquote` and `create_user_prompt_blockquote` to set `layout_mut().left_margin = Margin::Chars(1)` so the border lands at column 1. Confirm `WordWrap::WrapProse` remains the default (regression guard: if the BlockQuote default changes, set it explicitly here).
- [ ] In `system_prompt.rs`, refactor `report_system_prompt_with_base` so the **Summary sentence and the optional Partial/Full body are composed into one string** that is wrapped in a single orange `BlockQuote`. Today (line ~286-320) the summary is pushed as a bare `Prose` and only the partial/full branches construct a `BlockQuote` — this must change so a continuous orange bar runs from beneath the icon to the end of the body.
- [ ] In `user_prompt.rs`, mirror the same single-BlockQuote pattern (green) so the bar runs from beneath the 🗣️ icon to the end of the body.
- [ ] **Validation:** add a unit test per crate-side renderer that asserts the rendered output's first body line begins with one space, then the colored `│` glyph (i.e., `left_margin = 1`). Add a test that verifies the summary line is *inside* the BlockQuote rather than emitted as a sibling line.

### 6.2 — Display-label resolution and blue OSC8 styling

**Goal:** the hyperlink label in the System Prompt summary resolves to one of three forms — a Nerd Font glyph, a `./relpath`, or an absolute path — and is always styled blue.

- [ ] In `system_prompt.rs`, replace `relative_or_absolute` + `path_hyperlink_display` with a `resolve_display_label(absolute, base, term) -> String` helper returning the styled visible label. Logic:
    1. If `term.is_nerd_font == Some(true)` **and** `absolute` strips cleanly under `base`, return the single character `'\u{f02a2}'`.
    2. Else if `absolute` strips cleanly under `base`, return `format!("./{}", rel.display())`.
    3. Else return `absolute.display().to_string()`.
- [ ] Wrap the resolved label in blue styling via `Prose::new(format!("<color=blue-400>{label}</color>")).render(term)` (or the project's idiomatic Tailwind blue token — confirm against `biscuit-terminal::utils::color::Tailwind::Blue400`). The styled label is then embedded inside the OSC8 escape pair so the absolute path remains the link target.
- [ ] Confirm OSC8 fallback path (when `!term.osc_link_support`) still renders the blue-styled label as plain text.
- [ ] **Validation:** unit tests for all three label branches, asserting:
    - Nerd Font branch produces the single `\u{f02a2}` glyph and no path text in the visible portion.
    - Relative branch produces `./` + path with at least one subdir example (e.g., `./.claude/system-prompt.md`).
    - Absolute branch produces the absolute path string.
    - All three branches contain a Tailwind Blue 400 ANSI sequence in the visible label region.
    - OSC8 target in all branches is the absolute `file://` URL.

### 6.3 — `--quiet` becomes a no-op for the User Prompt

**Goal:** `--quiet` no longer suppresses the Agent Prompt; only `--silent` does.

- [ ] In `precedence.rs` (and any consumer in `mod.rs` / `user_prompt.rs`), locate the User-Prompt-side handling of the `--quiet` flag. Adjust it so `Quiet` maps to the **same configuration as default** for User Prompt (header shown, body driven by length and `--verbose`).
- [ ] Leave System Prompt `--quiet` semantics untouched (still forces `Summary` mode).
- [ ] **Validation:** add a CLI integration test (in `claudine/cli/tests/prompt_reporting.rs`) that runs a `compose`-style flow with `--quiet` and asserts the Agent Prompt header (`🗣️ Agent Prompt`) and at least one body line appear in stdout. Re-run the existing `--silent` test to confirm suppression is unchanged.

### 6.4 — Snapshot + integration test refresh

**Goal:** existing snapshots and integration tests reflect the new rendering.

- [ ] Re-run the relevant snapshot test(s) and update `claudine/cli/tests/snapshots/wrap_commands__wrapper_reports_removed_sensitive_env_names.snap` (and any other affected snapshots) after manual visual review.
- [ ] Update or extend `claudine/cli/tests/prompt_reporting.rs` to cover the four behaviors above end-to-end, including a `--quiet` run that still shows the Agent Prompt.
- [ ] **Validation Checkpoint:** `just test -p claudine` and `just lint -p claudine` are clean; manual visual check inside a Nerd-Font terminal confirms the glyph variant renders; manual visual check inside a non-Nerd-Font terminal confirms the `./relpath` variant renders.

### Dependencies

- 6.1 should land before 6.2 because 6.2's tests assume the Summary line is inside the BlockQuote.
- 6.3 is independent of 6.1/6.2 and may proceed in parallel.
- 6.4 lands last and consolidates the snapshot/integration coverage for all three.
