---
ready: false
implemented: true
agent: codex/default
created: "2026-07-01T23:05:57"
---

# Review 6: App Metadata

Not production ready. The app-metadata resolver, extraction, `bt about`, Warp locator-only handling, and focused Level 1 test suites pass. However, the feature's global `--plain` contract is still broken outside `about`, so a user-visible requirement from the spec is not met.

## Findings

### High: `--plain` still emits ANSI escapes on graph example output

The spec requires `--plain` to force `ColorDepth::None` for all subcommands and to override `FORCE_COLOR`, explicitly including graph paths that used to be `CliStyles`-styled. The implementation only proves the `about` path is escape-free; graph example output still bypasses the caller's `CliContext::plain`.

- [spec.md](spec.md:503) says `--plain` applies to all subcommands, not just `about`.
- [spec.md](spec.md:525) specifically names graph as part of the all-subcommands guarantee, and [spec.md](spec.md:528) says `--plain` overrides `FORCE_COLOR`.
- [graph.rs](../../cli/src/commands/graph.rs:167) calls `display_graph(...)` without passing `ctx.plain`, and [graph.rs](../../cli/src/commands/graph.rs:186) constructs `Terminal::new()` directly.
- [graph.rs](../../cli/src/commands/graph.rs:169) calls `print_example_command(...)`, and [shared.rs](../../cli/src/commands/shared.rs:165) hard-codes `terminal_for_render(false)`, explicitly ignoring `--plain`.
- [about.rs](../../cli/tests/about.rs:53) covers only `bt about kitty --plain`; there is no regression test for a formerly legacy-styled graph/default/content-analysis command with `FORCE_COLOR=1`.

I reproduced the leak with:

```sh
FORCE_COLOR=1 cargo run -q -p biscuit-terminal-cli -- --plain graph-expression --example
```

The first diagram fallback is plain, but the example footer prints:

```text
\e[1mCommand:\e[0m
\e[2mbt graph-expression "Start -> Validate -> Render; Validate -> Retry"\e[0m
```

This violates a user-observable CLI requirement. The correct verification level is Level 1 because the behavior is deterministic CLI rendering and environment precedence; no real terminal emulator or OS keyboard injection is needed. Add a Level 1 integration regression that runs a previously legacy-styled path with `FORCE_COLOR=1 --plain` and asserts stdout/stderr contain no `\x1b`, then thread the plain/color-depth choice through graph/mermaid/example helpers instead of reconstructing a default terminal.

## Verification Run

- `cargo check -p biscuit-terminal-cli --color never`: passed.
- `cargo nextest run -p biscuit-terminal app_metadata --color never`: passed, 33/33 focused Level 1 tests.
- `cargo nextest run -p biscuit-terminal-cli --test about --color never`: passed, 19/19 Level 1 CLI integration tests.
- `cargo nextest run -p biscuit-terminal-cli about --color never`: passed, 27/27 focused Level 1 tests.
- Manual regression probe: `FORCE_COLOR=1 cargo run -q -p biscuit-terminal-cli -- --plain graph-expression --example` emitted ANSI escapes, confirming the finding.

No Level 2 or Level 3 coverage is required for the app-metadata resolver/extractor itself: it reads static metadata, process environment, and files. The global `--plain` promise is also Level 1, but it must be asserted on the non-`about` paths named by the spec before this feature is ready.
