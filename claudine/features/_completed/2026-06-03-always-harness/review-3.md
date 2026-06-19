---
ready: true
agent: codex
model: ""
---

# Review: Always-Harness

## Verdict

Ready for production.

I found no blocking or follow-up findings in this iteration. The prior review's live-code comment drift is fixed in `SingleCompositionOutcome::iteration_signals`, and the implementation now matches the spec's single non-dry-run composition route through `run_harness_loop`.

## Findings

None.

## Requirement Verification Matrix

| Requirement | Strongest verification found | Status |
|---|---:|---|
| Bare direct compose and parsed-harness direct compose both execute through the unified harness route and preserve stdout body behavior. | Level 1 integration: `compose_direct_non_harness_and_harness_produce_identical_stdout_body`; static review of the single non-dry-run `run_harness_loop` call in `execute_composition_request_inner`. | OK |
| Bare inline compose and parsed-harness inline compose both execute through the unified harness route. | Level 1 integration: `inline_compose_non_harness_and_harness_write_identical_final_body`; static review of `HarnessPromptMode::Inline` routing. | OK |
| Inline body replacement uses only the post-tool-call final response in bare and parsed-harness runs. | Level 1 integration with manufactured structured stream containing interstitial narration and post-tool-call final text: `inline_compose_non_harness_and_harness_write_identical_final_body`; Level 1 unit coverage in `LiveSemanticSink` for final-response accumulation. | OK |
| Interactive composition routes through the unified path and preserves the resolved interactive controls. | Level 1 integration for `interactive: true` resolved-mode behavior and timeout conflict/no-launch guarantees in `compose_interactive_timeout_cli`; static review of interactive execution branch in `execute_harness_attempt`. | OK |
| Provider non-zero exit code remains observable at the CLI boundary. | Level 1 integration through loop failure behavior: `compose_loop_step_timeout_surfaces_as_iteration_failure`; static review of provider failure path returning `Ok((outcome.exit_code, ...))`. | OK |
| Dry-run stays before provider launch, still performs preflight, and does not mutate inline source files. | Level 1 integration: `inline_compose_dry_run_fails_on_read_only_source`, plus existing dry-run CLI coverage. | OK |
| Lifecycle ownership is single-owner for non-dry-run composition. | Static review: outer guard is defused before `run_harness_loop`; harness loop owns start/success/failure emission. | OK |
| Timeout precedence remains CLI > frontmatter > env > built-in for parsed plans, and CLI > env > built-in for bare plans. | Level 1 integration around resolved interactive timeout conflicts and existing timeout tests; static review of `build_harness_launch` using plan timeouts from parsed/finalized harness plans. | OK |
| `compose --loop` receives terminal-attempt rate-limit and exit-reason signals from bare and parsed-harness direct composition. | Level 1 integration: `compose_loop_rate_limit_abort_exits_75`, `compose_loop_rate_limit_abort_exits_75_on_harness_doc`, and `compose_loop_step_timeout_surfaces_as_iteration_failure`. | OK |
| Inline composition evaluates exactly one system-owned writability pre-check per effective plan. | Level 1 unit tests: `direct_bare_plan_unchanged`, `inline_bare_plan_adds_writability_pre_check`, `inline_parsed_plan_preserves_author_order`; Level 1 integration: `inline_compose_dry_run_fails_on_read_only_source`. | OK |
| Removed-path cleanup is complete for live code and current docs. | Static scan. Remaining matches are historical timeline entries and the current library `composition::closure::apply_inline_closure`, not the removed wrapper `inline_guards::apply_inline_closure` path. | OK |

No Level 2 or Level 3 gap applies to this feature. The user-observable behavior under review is provider process routing, stream-summary interpretation, exit-code propagation, file mutation, dry-run behavior, and frontmatter-driven preflight. These are correctly exercised with Level 1 in-process/CLI tests using deterministic provider stubs. The spec does not require terminal emulator rendering, terminal input encoding, hotkeys, modifier presses, paste, IME, mouse, or scroll behavior.

## Verification Run

- `cargo test -p claudine-cli --test compose_cli --test inline_compose_cli --color=never -- --nocapture` passed.
- `cargo test -p claudine-cli --test compose_interactive_timeout_cli --color=never -- --nocapture` passed.
- `cargo test -p claudine-cli --test loop_cli compose_loop_rate_limit_abort_exits_75 --color=never -- --nocapture` passed.
- `cargo test -p claudine-cli --test loop_cli compose_loop_rate_limit_abort_exits_75_on_harness_doc --color=never -- --nocapture` passed.
- `cargo test -p claudine-cli --test loop_cli compose_loop_step_timeout_surfaces_as_iteration_failure --color=never -- --nocapture` passed.
- `cargo test -p claudine --lib bare_plan --color=never -- --nocapture` passed.
- `cargo test -p claudine --lib inline_parsed_plan_preserves_author_order --color=never -- --nocapture` passed.
- `cargo check -p claudine -p claudine-cli --color=never` passed.
