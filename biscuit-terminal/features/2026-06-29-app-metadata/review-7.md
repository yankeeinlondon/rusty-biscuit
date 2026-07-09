---
ready: false
implemented: true
agent: codex/default
created: "2026-07-01T23:21:11"
---

# Review 7: App Metadata

Not production ready. The review-6 graph example leak has been fixed and is now covered by a focused Level 1 integration test, but the global `--plain` contract is still not true for all subcommands.

## Findings

### High: `--plain` still emits ANSI escapes on non-diagram render-tree commands

The spec requires `--plain` to force `ColorDepth::None` for all subcommands and to override `FORCE_COLOR` unconditionally. The latest patch threads `ctx.plain` through graph and Mermaid diagram commands, but other render-tree commands still ignore `CliContext::plain` and call `detect_terminal_honoring_force_color()` directly.

- [spec.md](spec.md:505) says `--plain` forces `ColorDepth::None`, and [spec.md](spec.md:525) says the result must be correct for all subcommands.
- [spec.md](spec.md:528) says `--plain` suppresses color unconditionally, overriding `FORCE_COLOR`.
- [block.rs](../../cli/src/commands/block.rs:242) discards the context as `_ctx`, then [block.rs](../../cli/src/commands/block.rs:271) calls `detect_terminal_honoring_force_color()`.
- [shared.rs](../../cli/src/commands/shared.rs:44) turns `FORCE_COLOR` / `CLICOLOR_FORCE` into `Terminal::new_forced()`, so any command that uses this helper without consulting `ctx.plain` can still produce SGR.
- [integration_test.rs](../../cli/tests/integration_test.rs:129) covers only `bt --plain graph-expression --example`; it does not cover another previously styled command such as `block`, `table`, `quote`, `list`, or `progress`.

I reproduced the remaining leak with:

```sh
FORCE_COLOR=1 CLICOLOR_FORCE=1 cargo run -q -p biscuit-terminal-cli -- --plain block hello --bold
```

The command prints:

```text
\x1b[1mhello\x1b[0m
```

This is a Level 1 verification gap and behavioral bug. No real terminal emulator is needed: the requirement is deterministic CLI rendering under explicit environment variables. The fix should make every terminal-rendering subcommand use `terminal_for_render(ctx.plain)` or an equivalent context-derived terminal, and add at least one Level 1 regression for a non-diagram render-tree path with `FORCE_COLOR=1 --plain`.

## Verification Run

- `cargo check -p biscuit-terminal-cli --color never`: passed.
- `cargo nextest run -p biscuit-terminal app_metadata --color never`: passed, 33/33 focused Level 1 tests.
- `cargo nextest run -p biscuit-terminal-cli --test about --color never`: passed, 19/19 Level 1 CLI integration tests.
- `cargo nextest run -p biscuit-terminal-cli test_graph_expression_example_plain_overrides_force_color --color never`: passed, 1/1 focused Level 1 regression.
- Manual regression probe: `FORCE_COLOR=1 CLICOLOR_FORCE=1 cargo run -q -p biscuit-terminal-cli -- --plain block hello --bold` emitted ANSI escapes, confirming the finding.

No Level 2 or Level 3 coverage is required for the app-metadata resolver/extractor or for the `--plain` precedence contract. These are file/env/rendering decisions that can be verified at Level 1.
