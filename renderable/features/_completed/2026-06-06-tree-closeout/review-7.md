---
ready: false
agent: codex
model: ""
---

# Review 7

## Findings

### High: The Level 2 probe is shipped as a normal Darkmatter binary

The real-terminal fix places `dm_l2_render_probe.rs` under
`darkmatter/lib/src/bin`, so Cargo auto-discovers it as a production `bin`
target (`darkmatter/lib/src/bin/dm_l2_render_probe.rs:1`). `cargo metadata`
confirms the `darkmatter` library package now exposes both the `darkmatter`
library and a `dm_l2_render_probe` binary.

This helper exists solely for one integration test, but ordinary
`cargo build -p darkmatter`, packaging, and binary discovery now include it.
That expands the public artifact surface of the library package and conflicts
with the monorepo convention that Darkmatter's user-facing binary belongs in
the separate `darkmatter-cli` package. A test fix should not introduce a
second, undocumented production executable.

Keep the subprocess test-only. For example, run a probe mode through the
integration-test executable itself, or explicitly gate a helper target behind
a non-default test feature enabled only by `test-l2` and excluded from normal
packaging/install flows. Preserve `CARGO_BIN_EXE_*` only if the target is
demonstrably absent from normal Darkmatter builds and published artifacts.

### Medium: The Level 2 module documentation still claims every test pre-renders to a tempfile

The module-level mechanism says each test renders in-process, writes a
temporary file, and runs `cat` in WezTerm
(`darkmatter/lib/tests/level2_render_tree_terminal.rs:13-22`). The new
capability test deliberately does the opposite: it runs the renderer process
inside the pane so terminal detection occurs against a real TTY
(`:386-413`).

This is behavior-relevant documentation drift in the exact area changed by
iteration 7. Rewrite the module overview to describe both mechanisms and state
that capability-detection tests must execute the renderer inside the terminal.

## Verification Levels

| Requirement | Strongest present verification | Assessment |
|---|---|---|
| Matched layout policy does not alter unrelated color/OSC8 capabilities | Level 1 capability-signature parity | Appropriate regression coverage |
| Same capability parity through a real terminal | Level 2 WezTerm render-in-pane with escaped OSC8 metadata and color comparison | Appropriate; review-6 gap closed |
| The matched policy actually applies | Level 2 captured table-position difference | Appropriate and non-vacuous |
| Page-frame width independence from unmatched policy | Level 1 discriminating parity | Appropriate |
| Terminal HR appearance and layout | Level 2 real-terminal capture | Appropriate |
| Browser HR and component layout/style | Browser computed style/geometry | Appropriate |
| Markdown/MarkdownPlus degradation | Level 1 | Appropriate |
| Keyboard, modifier, paste, IME, or mouse behavior | Not applicable | No Level 3 requirement |

## Focused Verification

- `cargo test -p darkmatter --lib terminal_matched_layout_policy_does_not_change_unrelated_capabilities --color=never`: 1 passed.
- `BISCUIT_TEST_LEVEL_REQUIRED=2 just -f darkmatter/justfile test-l2 level2_matched_layout_policy_matches_no_policy_capabilities_in_real_terminal`: the named Darkmatter test passed in WezTerm. The recipe was then terminated at the 60-second session limit while cold-building the unrelated `darkmatter-cli` package.
- `cargo metadata --no-deps --format-version 1`: identifies `dm_l2_render_probe` as a normal `bin` target of package `darkmatter`.
- `git diff --check` and `git diff --cached --check`: clean.

The real-terminal verification defect from review 6 is fixed, and the
performance percentage was correctly removed. The unintended production binary
target keeps the feature from being ready for production.
