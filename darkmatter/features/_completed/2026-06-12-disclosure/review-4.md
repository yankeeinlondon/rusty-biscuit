---
ready: false
agent: codex
model: ""
---

# Review: Disclosure Blocks

## Findings

### High: Terminal disclosure body still does not emit dim + italic in the library render path

The spec requires terminal output to render the disclosed body as a block quote
whose text is dim and italic. The focused Level 1 render-target test still fails:
`darkmatter/lib/tests/disclosure_render_targets.rs:129` expects SGR 2 and
`:130` expects SGR 3, but `Markdown::as_terminal(TerminalOptions::default())`
renders plain text:

```text
License Agreement

│  Keep your hands off. 
```

Relevant implementation is
`biscuit-terminal/lib/src/render_tree/render.rs:677`, which dispatches
`NodeKind::Disclosure` to `render_disclosure(summary, children)`. That helper
builds a dim+italic style at `:698`, enters inherited style at `:706`, and calls
`style::apply_style` at `:715`, but the resulting output observed by the test
contains no dim or italic escape codes. This means the direct library terminal
surface is still missing required user-observable styling.

Verification level present: Level 1 failing. The requirement is also
user-observable terminal styling, so it needs Level 2 real-terminal capture after
the Level 1 bug is fixed. The current Level 2 tests passed in this review run,
but they do not override the failing library-path evidence.

Command run:

```sh
cargo test -p darkmatter --test disclosure_render_targets --color=never
```

Result: 12 passed, 2 failed. Failed test:
`terminal_target_renders_summary_and_dim_italic_body`.

### High: Inline disclosure opener color is parsed but not rendered on the terminal target

The spec requires instance-level opener styles such as
`::disclosure color=red-500 max-width=24ch ...` to override frontmatter and
render visibly where supported. The new regression test proves parsing succeeds:
`darkmatter/lib/tests/disclosure_render_targets.rs:138` captures style hints on
the node and verifies the style tokens do not leak into summary text. However,
the terminal render test at `:165` still fails at `:172`: the rendered disclosure
is centered and wrapped, but it contains no `red-500` foreground SGR.

Observed output excerpt:

```text
                            A Title

                            │  This disclosed body
                            │ is
                            │ comfortably longer
                            │ than
                            │ twenty-four columns
                            │ wide. 
```

The policy merge in
`darkmatter/lib/src/markdown/render_tree/build_context.rs:267` reads inline
`NodeKind::Disclosure.style`, sets layout at `:290`, and sets style colors at
`:303`. The failed output suggests terminal rendering applies disclosure layout
but loses the inline color attribute before or during rendering.

Verification level present: Level 1 failing. This is user-observable terminal
paint behavior, so it also needs a reliable Level 2 real-terminal capture once
the Level 1 render path is fixed.

Command run:

```sh
cargo test -p darkmatter --test disclosure_render_targets --color=never
```

Failed test: `terminal_target_honors_inline_opener_style`.

### High: The Level 2 disclosure tests can pass against `PATH`'s installed `md`, not the just-built binary

The Level 2 disclosure tests are intended to verify this implementation in a
real terminal. Instead, `darkmatter/cli/tests/level2_layout.rs:157` builds the
command as:

```rust
let cmd = format!("md {} {}", file_path.display(), extra_args);
```

In this review environment that resolves to `/Users/ken/.cargo/bin/md`, as
confirmed by `which md`. That means the real-terminal disclosure tests can pass
against whatever binary is installed on the host, even while the current
workspace's focused Level 1 render-target tests fail. This is not production
confidence for the code under review.

Verification level present: nominal Level 2, but pointed at the wrong executable
boundary. The tests should invoke Cargo's built test binary path, for example
via `CARGO_BIN_EXE_md` or the existing assert-cmd pattern, then run that path
inside WezTerm. Until then, Level 2 coverage for terminal disclosure behavior is
not trustworthy.

Command run:

```sh
cargo test -p darkmatter-cli level2_disclosure --test level2_layout --color=never
```

Result: 2 passed, but this does not clear the Level 2 requirement because the
test subject is ambiguous and currently resolves through `PATH`.

## Verification Matrix

- Compose invariance and transclusion unification: Level 1 coverage present in
  compose tests.
- Markdown, MarkdownPlus, Browser, JSON, nested disclosure rendering: Level 1
  coverage present in `darkmatter/lib/tests/disclosure_render_targets.rs`.
- CLI `markdown-plus`, `browser`, `ast` compatibility: Level 1 CLI coverage
  present; focused MarkdownPlus CLI tests passed.
- Malformed disclosure handling, near-miss keywords, fenced-code ignores:
  Level 1 parser coverage present in `block_extension.rs` tests.
- Terminal dim/italic body: Level 1 failing; Level 2 currently not reliable.
- Terminal inline opener color: Level 1 failing; Level 2 currently not reliable.

## Production Readiness

Not ready for production. Required terminal styling still fails in the focused
library render-target suite, and the Level 2 disclosure coverage is not a
reliable check of the current workspace binary.

## Resolution (2026-06-13)

- **Finding #1 (dim + italic body) — resolved.** Already fixed by the disclosure
  styling commits preceding this review. The focused Level 1 suite now passes:
  `cargo test -p darkmatter --test disclosure_render_targets` → 14 passed, 0
  failed, including `terminal_target_renders_summary_and_dim_italic_body`.
- **Finding #2 (inline opener color) — resolved.** Same suite now passes
  `terminal_target_honors_inline_opener_style` and
  `inline_opener_style_is_parsed_off_the_summary`.
- **Finding #3 (Level 2 ran the host `PATH` `md`) — fixed.** The two disclosure
  Level 2 tests now run the just-built binary through a new `run_md_built`
  helper. It invokes a `md` symlink (created under the system temp dir, pointing
  at `CARGO_BIN_EXE_md`) rather than the absolute `target/debug/md` path,
  because that path contains the substring `rust`, which would otherwise leak
  into the captured command echo and corrupt the `find(|l| l.contains("rust"))`
  code-block anchors used elsewhere in the file. The shim path lives under
  `/var/folders/…`, so the visible command carries no `rust`. Verified with
  `BISCUIT_TEST_LEVEL_REQUIRED=2 cargo test -p darkmatter-cli --test level2_layout
  level2_disclosure` → 2 passed against the built binary (command echo now shows
  `…/dm-md-shim-<pid>/md`, not `~/.cargo/bin/md`).

### Out of scope / pre-existing

The broader `level2_layout.rs` suite still invokes the bare `md` for non-
disclosure tests (the same `PATH` anti-pattern, scoped out of this disclosure
review). Two code-theme tests —
`level2_code_block_inverts_to_dark_in_light_terminal` and
`level2_cli_code_theme_overrides_style_page_code_theme` — fail in a dark WezTerm
pane on the **unmodified** code as well (verified via `git stash`): the code-
block contrast fix derives the code-panel color mode from the real terminal, so
their forced-`COLORFGBG` light/dark expectations no longer hold. These are
independent of the disclosure work and warrant a separate follow-up.
