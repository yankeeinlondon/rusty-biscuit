---
ready: false
agent: codex
model: ""
---

# Review: `claudine context` Iteration 2

## Verdict

Not ready for production. The first-review blockers around embedding docs, `ctx.` display, and status-component footer rendering were mostly addressed, but this iteration still has user-visible correctness gaps and the verification level is below the review bar for terminal-rendered behavior.

## Findings

### High: `--expressions` tables are parsed with the first data row as the header

- Requirement: `claudine context --expressions` should provide a well-structured report of the expression engine's operations and functions.
- Implementation: `parse_expressions_content()` starts table collection only after the separator row. The header row is never pushed into `table_lines`, but `build_expr_table()` treats `lines[0]` as the headers and skips it from the row set.
- Impact: every expression-doc table rendered through this path loses its actual headers and first data row. For example, the Truthiness table is expected to have `Value` / `Falsy` headers, but the renderer will use the first row, `` `null` `` / `yes`, as the headers and omit that row from the body.
- Evidence: [context.rs](../../cli/src/commands/context.rs:555), [context.rs](../../cli/src/commands/context.rs:567), [context.rs](../../cli/src/commands/context.rs:712).
- Fix direction: keep the header row when a potential table starts, or store it separately before consuming the separator. Add assertions on actual parsed table headers and representative rows from `darkmatter-expressions.md`.
- Verification level: strongest present coverage is Level 1 unit parsing, but it only asserts section presence. This requirement needs Level 1 content assertions at minimum.

### High: narrow terminal rendering fix is tested on a different table than production uses

- Requirement: default and `--values` reports render organized tables in the terminal.
- Implementation: the production context tables define `Property`, `Type`, and `Description`/`Value` columns without marking the `Type` column droppable. The regression test constructs a separate table that does call `drop_when_space_is_limited`, so it proves biscuit-terminal behavior but not the `claudine context` table configuration.
- Impact: on narrow terminals the actual command can still hit the table width failure path or render poorly, while the test remains green because it does not render the production report table.
- Evidence: production columns in [context.rs](../../cli/src/commands/context.rs:363) and [context.rs](../../cli/src/commands/context.rs:407); test-only droppable column in [context.rs](../../cli/src/commands/context.rs:1242).
- Fix direction: apply the same droppable-column configuration in `render_default_report()` and `render_values_report()`, then test through those render paths or a shared table-construction helper.
- Verification level: this is terminal-rendered, user-observable table behavior. The current strongest test is Level 1 and not wired to production. Per the requested rigor, add Level 2 coverage in a real terminal or multiplexer for narrow and normal widths before calling this ready.

### High: required terminal styling has no Level 2 verification

- Requirement: footer messages use blue flag names and styled emphasis; reports use styled headings and tables. The spec explicitly defines visible terminal styling for the two footer messages.
- Implementation tests: coverage is limited to in-process parsing/unit checks in `context.rs`. There is no integration test invoking `claudine context`, no PTY coverage for this command, and no Level 2 capture via WezTerm, Kitty, or tmux.
- Impact: regressions in SGR styling, table widths, wrapping, margins, or stdout/stderr placement can ship unnoticed. This matters here because the feature is almost entirely a terminal report.
- Evidence: tests are local to [context.rs](../../cli/src/commands/context.rs:1098), [context.rs](../../cli/src/commands/context.rs:1133), [context.rs](../../cli/src/commands/context.rs:1181), [context.rs](../../cli/src/commands/context.rs:1210), and the only PTY tests in `claudine/cli/tests/pty_tests.rs` are unrelated and ignored.
- Fix direction: add Level 1 CLI assertions for routing/content/stdout/stderr, plus Level 2 terminal-capture tests for styled footer and table rendering at representative widths.
- Verification level: mismatch. Per the review instructions, styled terminal output with specific colors needs Level 2 verification.

### Medium: documented alias values render as `null` under `--values`

- Requirement: `--values` renders the same report with the current host's values.
- Implementation: the command reads variables from `context-variables.md` and looks them up directly in `ComposeContext::values()`. The docs include backward-compatible aliases `utc`, `dow`, and `dow_abbr`, but Darkmatter's runtime capture inserts `now_utc`, `day`, and `day_abbr` without those alias keys.
- Impact: `claudine context --values` reports `ctx.utc`, `ctx.dow`, and `ctx.dow_abbr` as `null`, even though their descriptions say they are aliases for populated values.
- Evidence: direct lookup in [context.rs](../../cli/src/commands/context.rs:419); alias docs in [context-variables.md](../../../darkmatter/docs/topics/context-variables.md:73) and [context-variables.md](../../../darkmatter/docs/topics/context-variables.md:91); runtime inserts only canonical keys in [capture.rs](../../../darkmatter/lib/src/markdown/compose/context/capture.rs:635) and [capture.rs](../../../darkmatter/lib/src/markdown/compose/context/capture.rs:655).
- Fix direction: either add aliases to `ComposeContext::values()` in Darkmatter or have the report resolve documented aliases explicitly. The stronger fix is in Darkmatter so interpolation and reporting agree.
- Verification level: Level 1 content tests are enough for this specific value mapping, but none currently assert representative live values or alias values.

## Test Rigor Classification

- `claudine context`: Level 1 parser/unit coverage only. Missing Level 1 CLI assertions and Level 2 terminal capture for styled table rendering.
- `claudine context --values`: Level 1 parser/unit coverage only. Missing Level 1 assertions for the `Value` column, no `Description` column, and non-null values for representative keys and aliases.
- `claudine context --expressions`: Level 1 parser/unit coverage only. Existing assertions miss the broken table-header behavior and do not verify representative operations/functions content.
- `claudine context --side-effects`: placeholder implementation exists, but there is no CLI-level assertion that stdout contains `not implemented yet` and stderr contains the required footer.
- Footer messages: implemented with `Status`/`StatusState::Info`, but not verified at Level 2 for styled blue flag names or at Level 1 for exact stdout/stderr routing.

## Notes

I attempted `cargo test -p claudine-cli context --color=never` and direct `cargo run -p claudine-cli --bin claudine -- context ...` checks, but the cold build was still compiling after the non-interactive session's practical timeout threshold, so I stopped those commands and did not treat them as verification.
