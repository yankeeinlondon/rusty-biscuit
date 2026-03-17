# CLI Parsing And Reporting Review

## Scope

Reviewed the implementation against `claudine/features/cli-parsing-and-reporting/spec.md`, focusing on:

- wrapper argument parsing and session-mode selection
- composition flows with `--interactive`
- execution-line reporting
- test coverage and drift

Targeted validation I ran:

- `cargo test -p claudine-cli --test wrap_commands -- --nocapture` → passed
- `cargo test -p claudine-cli --test pty_tests -- --nocapture` → 2 failing tests

## Recommendations

### 1. Replace the current prompt heuristic with provider-aware prompt detection

The default interactive/non-interactive decision is currently driven by `has_prompt_source()` in [`claudine/cli/src/commands/wrap/mod.rs:476`](../../cli/src/commands/wrap/mod.rs#L476) and [`claudine/cli/src/commands/wrap/mod.rs:1541`](../../cli/src/commands/wrap/mod.rs#L1541). The helper only checks for “any non-switch arg”, and `extract_user_prompt()` in [`claudine/cli/src/commands/wrap/mod.rs:1530`](../../cli/src/commands/wrap/mod.rs#L1530) uses the same heuristic for the execution line.

That creates two concrete problems:

- Provider flags that take values can be mistaken for a prompt.
  Example repro: `claudine gemini -- --approval-mode default`
  Current result: Claudine flips into non-interactive mode and errors with `--non-interactive for gemini requires a prompt...`, even though no prompt was provided.
- Inline prompt forms are missed.
  Example repros: `claudine gemini -- --prompt=hello` and `claudine qwen -- --prompt=hello`
  Current result: the session is still tagged `INTERACTIVE=true`, the execution line omits the prompt, and the non-interactive default never activates.

This violates the spec’s “order should have no impact on outcome” rule and makes provider passthrough ergonomics brittle.

Recommendation:

- Reuse a single provider-aware prompt locator for all of:
  - session-mode selection
  - execution-line prompt display
  - MCP tag stripping
- Base that logic on `find_prompt_location()` / `takes_value()` instead of “first non-switch arg”.
- Explicitly support `--prompt=value` and `-p=value` forms in the default-mode logic.

Tests to add:

- provider flag with value but no prompt stays interactive
- `--prompt=value` is treated as a prompt for Gemini and Qwen
- execution line shows the real prompt for native prompt-flag forms

### 2. Make `--interactive` actually preserve interactive composition

The spec explicitly says `--compose`, `--frontmatter-prompt`, and `--prompt-file` should default to non-interactive, but `--interactive` should allow them to become the first prompt in an interactive session.

That is not true today for several providers:

- Gemini always injects `--prompt` in [`claudine/cli/src/commands/wrap/profile.rs:689`](../../cli/src/commands/wrap/profile.rs#L689)
- Qwen always injects `--prompt` in [`claudine/cli/src/commands/wrap/profile.rs:877`](../../cli/src/commands/wrap/profile.rs#L877)
- Claude rejects interactive prompt-file delivery outright in [`claudine/cli/src/commands/wrap/profile.rs:367`](../../cli/src/commands/wrap/profile.rs#L367)
- Kimi rejects interactive prompt-file delivery outright in [`claudine/cli/src/commands/wrap/profile.rs:764`](../../cli/src/commands/wrap/profile.rs#L764)

Concrete repros:

- `claudine gemini -i --compose claudine/features/cli-parsing-and-reporting/spec.md`
  Current result: the child still receives `--prompt <content>`, so this is still a headless-style launch, not an interactive startup prompt.
- `claudine claude -i --prompt-file <file>`
  Current result: hard error, even though the spec says `-i` should make this interactive.

Recommendation:

- Split prompt delivery by session type instead of a single `apply_prompt_body()` path.
- A small enum like `PromptDeliveryMode::{Interactive, NonInteractive}` would make the provider profiles harder to misuse than a boolean.
- For providers that genuinely cannot seed an interactive session from a composed prompt, decide one of these explicitly:
  - implement a provider-native interactive startup path
  - reject `-i` early with a provider-specific message that documents the limitation
  - narrow the spec if this behavior is impossible

Tests to add:

- `-i + --compose` for Gemini/Qwen does not emit `--prompt`
- `-i + --prompt-file` / `-i + --frontmatter-prompt` behavior is covered for every wrapped provider
- provider matrix snapshot for composition delivery behavior in interactive vs non-interactive mode

### 3. Tighten execution-line behavior to match the spec

The execution-line renderer in [`claudine/cli/src/output.rs:28`](../../cli/src/output.rs#L28) is close, but still misses a few spec details:

- It prints a leading blank line but not a blank line after the header.
  Current code: [`claudine/cli/src/output.rs:100`](../../cli/src/output.rs#L100)
- Prompt display still depends on the raw positional heuristic from `extract_user_prompt()`, so native prompt forms can be omitted or displayed incorrectly.
- Unsupported wrapper features are currently surfaced as warnings, but the spec calls for `Info: {msg}` items after the execution line.

Recommendation:

- Emit an explicit blank separator after the header when it is visible.
- Derive prompt display from the same provider-aware prompt parser used for mode selection.
- Downgrade unsupported-capability notices from warning to info when the request is being safely ignored rather than rejected.
  Examples: unsupported YOLO on OpenCode, unsupported `--output`, unsupported `--sandbox`, unsupported `--system-prompt`.

Tests to add:

- snapshot of header-only output with one blank line before and after
- multiline prompt is rendered with literal `\n`, never as multiple terminal lines
- long prompt truncation stays on one line
- native prompt-flag forms appear in the execution line without leaking provider-specific wrapper switches

### 4. Repair stale PTY coverage and wrapper docs

There is confirmed drift in both tests and docs:

- [`claudine/cli/tests/pty_tests.rs:26`](../../cli/tests/pty_tests.rs#L26) still uses `-n`, which the spec removed.
- Both PTY tests currently fail with `ExpectTimeout`.
- [`claudine/cli/README.md:130`](../../cli/README.md#L130) still documents `-n, --non-interactive, --ni`.

Recommendation:

- Update PTY coverage to the new semantics:
  - prompt present => implicit non-interactive
  - prompt + `-i` => interactive override badge
  - no prompt => interactive default
- Keep at least one PTY test that validates the visible badge set end-to-end, since that is part of the feature spec rather than just internal logic.
- Update the wrapper README and any related docs/prompts/skills that still reference `--non-interactive`.

## Priority Order

1. Fix provider-aware prompt detection.
2. Fix `--interactive` composition delivery.
3. Tighten execution-line formatting and prompt display.
4. Repair PTY/doc drift and add coverage for the missing cases above.
