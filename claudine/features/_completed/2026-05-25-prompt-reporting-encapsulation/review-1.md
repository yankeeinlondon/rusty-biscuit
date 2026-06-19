---
ready: true
agent: codex
model: ""
---

# Review: Prompt Reporting Encapsulation

## Verdict

Not ready for production. The implementation is close on the core refactor, and the targeted Level 1 tests pass, but two acceptance criteria are not satisfied: the `prompt_reporting` public surface is still larger than the specified seven items, and user-visible terminal rendering has no Level 2 verification.

## Findings

### High: Public surface is still larger than the seven-symbol contract

The spec says the public surface must contract to exactly:

- `SystemPromptReport`, `AgentPromptReport`
- `ReportMode`, `TruncationMode`
- `resolve_system_prompt_report_mode`, `resolve_agent_prompt_report_mode`
- `parse_frontmatter_verbosity`

However, `prompt_reporting` still exposes public modules at [claudine/lib/src/prompt_reporting/mod.rs:13](../../lib/src/prompt_reporting/mod.rs), [claudine/lib/src/prompt_reporting/mod.rs:17](../../lib/src/prompt_reporting/mod.rs), and [claudine/lib/src/prompt_reporting/mod.rs:18](../../lib/src/prompt_reporting/mod.rs):

```rust
pub mod state;
pub mod types;
pub mod user_prompt;
```

Those are public items under `claudine::prompt_reporting::*`, so consumers can still import `state`, `types`, and `user_prompt`, and can reach extra APIs such as `prompt_reporting::state::check_and_record`, `prompt_reporting::state::compute_prompt_hash`, and `prompt_reporting::user_prompt::AgentPromptReport`. This misses the explicit acceptance criterion and weakens the encapsulation goal.

Recommended fix: make these modules private and keep only the seven re-exports public. If `state` must remain callable by the CLI, move that state helper behind a crate-level public API outside `prompt_reporting` or expose a narrow wrapper that is included in the spec.

### High: Visual terminal behavior is only verified at Level 1

The spec explicitly says visual output must not change: header glyphs, block-quote colors, summary prose, truncation thresholds, and chrome-width arithmetic must stay identical. The current tests for these user-observable rendering requirements are Level 1 only:

- Unit tests render with `Terminal::new()` and inspect strings in-process, for example [claudine/lib/src/prompt_reporting/system_prompt.rs:279](../../lib/src/prompt_reporting/system_prompt.rs) and [claudine/lib/src/prompt_reporting/user_prompt.rs:122](../../lib/src/prompt_reporting/user_prompt.rs).
- CLI integration tests run the binary with `assert_cmd`, force `NO_COLOR=1`, then strip ANSI before asserting text, for example [claudine/cli/tests/prompt_reporting.rs:67](../../cli/tests/prompt_reporting.rs), [claudine/cli/tests/prompt_reporting.rs:68](../../cli/tests/prompt_reporting.rs), and [claudine/cli/tests/prompt_reporting.rs:82](../../cli/tests/prompt_reporting.rs).
- There are no `level2_*` prompt-reporting tests that run `compose` inside a real terminal and capture the pane via tmux or WezTerm. The existing Level 2 tests are for other surfaces such as perf, context, dry-run, and schema prompts.

Per the requested review rubric, requirements like colored `System Prompt` / `Agent Prompt` badges, block-quote styling, OSC8 link rendering, wrapping, and scroll/capture-visible layout require at least Level 2 real-terminal capture. Current strongest coverage is Level 1, so this is a readiness gap.

Recommended fix: add `level2_prompt_reporting_capture.rs` covering at minimum:

- tmux capture for orange system prompt block quote and green agent prompt block quote SGR.
- WezTerm capture for the system-prompt file OSC8 hyperlink.
- Narrow-width capture for prompt body wrapping/chrome width and front/back truncation.

### Medium: `cargo doc -p claudine --no-deps` still emits rustdoc warnings

The spec requires `cargo doc -p claudine` to produce no broken intra-doc-link warnings. I ran:

```bash
cargo doc -p claudine --no-deps --color=never
```

It completed, but emitted two `rustdoc::private_intra_doc_links` warnings:

- `claudine/lib/src/composition/agent_message.rs:7` links to private `super::error`.
- `claudine/lib/src/composition/loop_engine.rs:40` links to private `PAUSE_RESET_MARGIN`.

These appear unrelated to prompt reporting, but the acceptance criterion is repository-command based, not module-scoped. The criterion is not currently satisfied.

## Coverage Notes

Strong Level 1 coverage exists for the refactor mechanics:

- `cargo test -p claudine --lib prompt_reporting --color=never`: 114 passed.
- `cargo test -p claudine-cli --test prompt_reporting --color=never`: 12 passed.

Requirement-to-level summary:

| Requirement | Strongest observed verification | Ready? |
|---|---:|---|
| `ReportMode` precedence and frontmatter parsing | Level 1 unit tests | Yes |
| `SystemPromptReport::render` dispatches `Ready` / `None` / `Disabled` | Level 1 unit tests | Yes |
| CLI call sites use one resolver and one report constructor | Level 1 compile/integration tests plus code review | Yes |
| Header glyphs, colors, block quotes, OSC8 links, width/truncation rendering | Level 1 only | No, needs Level 2 |
| Public surface is exactly seven symbols | Code review shows extra public modules | No |
| `cargo doc -p claudine` has no broken intra-doc-link warnings | Command completed with warnings | No |

## Positive Notes

The main call-site migration matches the design: `log_compose_prompt` constructs `AgentPromptReport` after one resolver call, and `log_system_prompt_with_scope` constructs `SystemPromptReport` after one resolver call at [claudine/cli/src/output/mod.rs:162](../../cli/src/output/mod.rs) and [claudine/cli/src/output/mod.rs:231](../../cli/src/output/mod.rs).

`EffectiveSystemPrompt` no longer appears in the source paths I checked; the implementation consistently uses `ResolvedSystemPrompt`.

## Resolution

All three findings are resolved and verified.

### High: Public surface contracted to the seven-symbol contract

`prompt_reporting/mod.rs` now declares every submodule private (`mod formatting; mod frontmatter; mod precedence; mod system_prompt; mod tokens; mod truncation; mod types; mod user_prompt;`) and re-exports exactly the seven specified items: `SystemPromptReport`, `AgentPromptReport`, `ReportMode`, `TruncationMode`, `resolve_system_prompt_report_mode`, `resolve_agent_prompt_report_mode`, `parse_frontmatter_verbosity`.

The state helper was moved out of `prompt_reporting` entirely. `prompt_reporting::state` is deleted; its logic now lives in the private `claudine/lib/src/system_prompt/change_state.rs`, with only `check_and_record` re-exported via `claudine::system_prompt::check_and_record`. The CLI calls that single narrow API at `claudine/cli/src/output/mod.rs:197`. No source path references `prompt_reporting::state`, `prompt_reporting::types`, or `prompt_reporting::user_prompt` any longer.

### High: Visual terminal behavior now verified at Level 2

Added `claudine/cli/tests/level2_prompt_reporting_capture.rs` driving the real `claudine compose --goose` binary inside real terminal emulators:

- `level2_prompt_reporting_block_quote_colors_in_tmux` — tmux capture asserts the Tailwind Orange500 system-prompt and Green500 agent-prompt block-quote SGR.
- `level2_prompt_reporting_body_wraps_and_reserves_chrome_in_tmux` — narrow-width tmux capture asserts body wrapping and the 2-cell `┃ ` chrome arithmetic.
- `level2_prompt_reporting_front_back_truncation_in_tmux` — tmux capture asserts 20/10 front/back truncation of an over-length agent prompt.
- `level2_prompt_reporting_system_link_osc8_in_wezterm` — WezTerm capture asserts the system-prompt file OSC8 hyperlink stays inside the control bytes.

All four pass against real backends: `BISCUIT_TEST_LEVEL_REQUIRED=2 cargo test -p claudine-cli --test level2_prompt_reporting_capture` → 4 passed.

### Medium: `cargo doc -p claudine --no-deps` is warning-free

A fresh `cargo doc -p claudine --no-deps` build completes with no `rustdoc::private_intra_doc_links` (or any) warnings. The `agent_message.rs` and `loop_engine.rs` doc comments no longer emit private intra-doc links.

### Verification summary

- `cargo test -p claudine --lib prompt_reporting`: 111 passed.
- `cargo test -p claudine-cli --test prompt_reporting`: 12 passed.
- `BISCUIT_TEST_LEVEL_REQUIRED=2 cargo test -p claudine-cli --test level2_prompt_reporting_capture`: 4 passed.
- `cargo doc -p claudine --no-deps`: no warnings.
