---
agent: codex
model: ""
ready: true
---

# Review - Iteration 2

## Findings

No blocking findings.

The iteration-2 changes address the two iteration-1 production blockers:

- `inline-compose` loop seeding is now mode-aware. `compose` passes `CompositionMode::ChainedDocument`, while `inline-compose` passes `CompositionMode::InlineFrontmatterPrompt`, and `build_loop_seed` dispatches to the matching prepare path.
- `doc.<key>` condition references now lift the referenced frontmatter head into the loop-owned control state, so read-only values such as `doc.total` remain available during seeded loop condition evaluation.

## Test Rigor

- Spec requirement: expression-defined control variables resolve before loop mutation, `increment(phase)` advances typed state, iteration bodies see `phase = N`, and derived frontmatter such as `pass_icon` remains live. Strongest verification: Level 1 via `seeded_loop_repro_runs_to_completion_with_live_derived_variable`. Level 1 is appropriate because this is in-process composition state flow, not terminal rendering or input encoding.
- Spec requirement: inline loop seeding must match the CLI composition mode. Strongest verification: Level 1 via `build_loop_seed_inline_mode_resolves_prompt_frontmatter_with_empty_body` and `inline_compose_loop_with_prompt_frontmatter_and_empty_body_runs`. Level 1 is appropriate because the behavior is CLI orchestration plus composition mode selection.
- Spec requirement: loop conditions and actions operate on resolved control state, including `doc.<key>` condition reads. Strongest verification: Level 1 via `extract_control_variables_lifts_doc_namespace_head`, `extract_control_variables_doc_and_action_target_merge`, and `seeded_loop_doc_namespace_condition_retains_readonly_control_value`. Level 1 is appropriate because the requirement is expression/state evaluation.
- Spec requirement: invalid numeric mutation errors include the offending value and unresolved-template context when applicable. Strongest verification: Level 1 via the loop action error excerpt tests and `seeded_loop_reports_honest_error_for_non_numeric_control_variable`. Level 1 is appropriate because this is structured error data, not terminal styling.

No Level 2 or Level 3 verification is required for this feature as reviewed. The user-observable behavior is composition output and CLI control flow; it does not assert terminal emulator rendering, key input encoding, mouse behavior, IME, paste handling, or real-terminal scroll/styling behavior.

## Verification Run

- `cargo test -p claudine composition::loop_ --color=never` - passed: 119 loop-related tests passed.
- `cargo test -p claudine-cli --test loop_cli --color=never` - passed: 18 tests passed.
- `cargo test -p claudine-cli --test loop_cli inline_compose_loop_with_prompt_frontmatter_and_empty_body_runs --color=never` - passed.

Note: an initial attempted command, `cargo test -p claudine composition::loop_config composition::loop_engine --color=never`, failed before running tests because Cargo accepts only one test-name filter in that position. It was replaced by the valid `composition::loop_` filter above.

## Summary

The implementation now matches the spec's control-variable ownership model: loop state is seeded from resolved typed frontmatter, mutated state is passed back into per-iteration composition, derived presentation values continue to re-resolve each iteration, and both direct compose and inline-compose use the correct seed mode. I consider this feature ready for production.
