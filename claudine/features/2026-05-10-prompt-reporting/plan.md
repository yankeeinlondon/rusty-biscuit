---
phases: 5
created: 2026-05-11
start_phase: 1
---
# Prompt Reporting Execution Plan

## Phase 1: Foundation and Types
- [ ] Define `PromptReportFormat` enum (`Summary`, `PartialPrompt`, `FullPrompt`) and truncation variants (`Truncate`, `FrontBack`).
- [ ] Add parsing for `verbosity` property in frontmatter of `system-prompt.md` files.
- [ ] Implement precedence logic for System Prompt resolution: CLI Switches > `CLAUDINE_SYSTEM_PROMPT` env > Prompt Length > Frontmatter > Default (`Summary`).
- [ ] Implement token estimation using the `FileTree` utility from `biscuit-terminal` (measuring composed content, omitting agent base prompt).
- [ ] Validation Checkpoint: Unit tests for configuration precedence logic and token count estimation.

## Phase 2: Common Formatting Utilities (Parallelizable)
- [ ] Implement text truncation utility for `FrontBack` mode (x lines front, 10 lines back with an `hr` marker) ensuring the first/last lines of truncated sections are not blank lines.
- [ ] Enhance markdown rendering configuration using `darkmatter` to strictly enforce the constraint of no more than two consecutive blank lines.
- [ ] Implement `BlockQuote` styling logic leveraging `Prose` from `biscuit-terminal` that supports custom colored vertical lines (orange/green) and centered icon alignment.
- [ ] Validation Checkpoint: Unit tests covering `FrontBack` boundary edge cases and darkmatter blank line collapsing.

## Phase 3: System Prompt Reporting
- [ ] Implement Line 1 header logic using `Prose`: `📕 <b>System Prompt(<dim><i>{action}</i></dim>)</b>` where action is `appended` or `replaced`.
- [ ] Implement the `Summary` view rendering, including the OS8 hyperlink to the prompt source and formatted token counts.
- [ ] Implement the `Partial Prompt` and `Full Prompt` rendering applied inside the orange `BlockQuote`.
- [ ] Create the top-level System Prompt reporter that wires together header, resolution precedence, and the selected body mode (suppressing output if `Silent`).
- [ ] Validation Checkpoint: Output format tests against various configurations (Env vars, flags like `--silent`, `--quiet`, `--verbose`, frontmatter).

## Phase 4: User Prompt Reporting
- [ ] Implement the User Prompt header logic: `🗣️ Agent Prompt`.
- [ ] Create User Prompt logic to strip all leading whitespace from the prompt body.
- [ ] Implement the 40-line threshold behavior (Full if <=40, `FrontBack` 20/10 if >40 lines) overridden by `--verbose`.
- [ ] Wire `--quiet` (suppresses entirely) and `--silent` (suppresses entirely) for the User Prompt.
- [ ] Render the User Prompt body within a green `BlockQuote` using the shared darkmatter utilities.
- [ ] Validation Checkpoint: Unit tests for User Prompt with both short (<40 lines) and long (>40 lines) bodies, validating correct `FrontBack` usage and flag behavior.

## Phase 5: CLI Integration and Validation
- [ ] Integrate the new System and User Prompt reporting structures into the main `claudine` output pipeline.
- [ ] Ensure `ComposeArgs` and other command arguments properly propagate `--quiet`, `--verbose`, and `--silent` into the reporting context.
- [ ] Validation Checkpoint: End-to-end test verifying visual layout, correct token counting, markdown formatting, and suppression across standard `compose` workflows.
