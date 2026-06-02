---
ready: true
agent: codex
model: ""
---

# Review: Sub-Spec #2 Page-Level Wiring

## Findings

### High: `style.page.alignment` overrides component-specific CLI alignment flags

Spec lines 107-112 define `style.page.alignment` as a broadcast default that must not override a component-specific CLI alignment flag. The implementation applies CLI component flags first in `apply_cli_layout_flags` (`--align-images`, `--align-lists`, `--align-block-quotes`, `--align-tables`, `--align-code-blocks`) at `darkmatter/cli/src/output.rs:143` through `darkmatter/cli/src/output.rs:156`, but `page_style_overrides_from_cli` only marks `alignment` as claimed when the global `--alignment` flag is set at `darkmatter/cli/src/output.rs:208`. Then `apply_page_style` calls `use_alignment_for_all` at `darkmatter/lib/src/style/apply.rs:168` through `darkmatter/lib/src/style/apply.rs:171`, which overwrites any component-specific CLI alignment already applied.

User impact: `md --align-tables right doc.md` can be silently changed back to the frontmatter page default when the document contains `style.page.alignment: center`. That violates the CLI-over-frontmatter precedence rule and the explicit component-specific exception in spec test #7.

Verification level: strongest current coverage is Level 1 and does not include this case. Add a Level 1 regression around `apply_cli_layout_flags` plus `apply_style_frontmatter` or `apply_page_style` showing component-specific CLI alignment survives the page broadcast. The implementation likely needs component-level override tracking for alignment, or it needs to reapply component-specific CLI alignment after the page broadcast.

### High: terminal rendering acceptance is not verified at the required level

The spec's acceptance criteria require `md darkmatter/example-docs/rendering/style-prop.md` to visibly apply page margins and require rendered output to show margins/padding/max-width behavior. The current CLI test named `style_fixture_renders_terminal_successfully` invokes `md_cmd().arg(...).output()` at `darkmatter/cli/tests/cli.rs:3283`, which captures stdout as a pipe. In `run_render`, `OutputFormat::Auto` only calls `render_terminal_output` when stdout is a TTY; captured stdout takes the markdown artifact path at `darkmatter/cli/src/commands.rs:418` through `darkmatter/cli/src/commands.rs:426`. That means the test does not exercise the terminal rendering path it claims to verify.

There are useful Level 1 tests for the applicator and in-process render calls, but no Level 2 run-in-real-terminal capture for the user-visible terminal layout. For this feature, the observable requirements include leading whitespace margins, wrapping/max-width, and alignment behavior through the real terminal renderer. Per the review rubric, those should not be called production-ready with only in-process assertions and pipe-captured CLI output.

Verification level: strongest present coverage is Level 1. Add a Level 2 test using tmux, WezTerm, or Kitty to run `md style-prop.md` in a real pane and capture text, asserting the expected top margin and leading columns on non-empty content lines. Keep the existing Level 1 tests, but rename or adjust the pipe-based CLI test so it does not imply terminal coverage.

### High: strict mode drops `KnownButInactive` tracing events

The spec requires parsed-but-unwired keys to emit tracing events so `RUST_LOG=darkmatter=info` surfaces them, and it separately says `KnownButInactive` remains informational under `--strict-style`. In `apply_style_frontmatter`, the strict branch calls `into_strict(parsed)` and then replaces the warning list with `Vec::new()` at `darkmatter/cli/src/output.rs:223` through `darkmatter/cli/src/output.rs:227`. As a result, a strict-style document that only has future-phase keys succeeds, but the future-phase `KnownButInactive` warnings are never passed to `log_style_warnings` at `darkmatter/cli/src/output.rs:231`.

User impact: `md --strict-style` hides exactly the informational wiring-status events the spec says should remain visible. This is especially relevant for schema-clean documents such as the fixture, where strict mode is expected to succeed while still surfacing future-phase keys when tracing is enabled.

Verification level: no direct logging test is present. Add a Level 1 tracing/subscriber test or CLI stderr/log capture test that runs strict mode with a future-phase key and asserts the `style key parsed but not yet wired` event is emitted.

## Requirement Coverage

Page margin, padding, percent resolution, max-width lowering, background mapping, active warning suppression, and global CLI override handling have useful Level 1 coverage through unit and integration tests. The implementation also updates the page-level docs and wires both terminal and HTML artifact builders through `apply_style_frontmatter`.

Level 3 is not applicable here because the feature has no OS keyboard input requirement. Level 2 is still missing for the terminal-visible layout requirements.

## Verification

Attempted: `cargo nextest run -p darkmatter --no-fail-fast`.

Result: not completed. Cargo waited on the artifact directory lock and was still running after roughly 60 seconds, so I stopped the darkmatter nextest process per the non-interactive session guidance. No test pass/fail result should be inferred from this run.
