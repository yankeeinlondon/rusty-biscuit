---
ready: false
agent: codex
model: ""
---

# Review: Prose Cross-Target Rendering

## Findings

### High: Markdown link destinations are emitted without escaping

`MarkdownRenderable for Prose` writes links as `[desc](resolved)` and inserts the resolved href directly (`biscuit-terminal/lib/src/components/prose/to_markdown.rs:59-69`). That breaks valid link output for hrefs containing Markdown destination delimiters. For example:

```bash
cargo run --quiet --color=never -p biscuit-terminal-cli -- prose --md '<a href="https://example.com/a)b">go</a>'
```

currently emits:

```markdown
[go](https://example.com/a)b)
```

The visible target is now malformed Markdown: the destination is truncated at `a`, and `b)` becomes literal trailing text. The parser already supports escaped parens in Markdown input, and links are one of the required Markdown acceptance cases, so the emitter needs to preserve the destination by escaping/encoding `)` and backslash or by using a valid angle-bracket destination form with the needed escaping. Add Level 1 target tests for `)`, `(`, backslash, whitespace, and `]` in the description/destination.

Verification level present: Level 1 tests cover only a simple URL (`to_markdown.rs:174-179`). Required: Level 1 Markdown target tests for delimiter-bearing hrefs.

### High: Rich terminal styling still has a Level 2 verification mismatch

The spec requires terminal rendering to preserve visible output and ANSI behavior (`renderable/features/2026-05-17-prose-cross-target/spec.md:300-302`, `:337-340`). The new Level 2 rich-styling tests run inside WezTerm/Kitty, but the core rich assertion deliberately accepts `bt prose --print-bytes` hex output as the proof (`biscuit-terminal/cli/tests/level2_prose_styling.rs:422-455`). That proves the renderer generated SGR bytes, but it does not prove the terminal emulator rendered/captured those styled cells. The simpler red tests also pass if either raw terminal capture contains SGR or the byte dump contains SGR (`level2_prose_styling.rs:25-95`, `:149-209`), so a broken real-terminal capture path can still pass through the Level 1-style backstop.

Impact: nested bold/italic, strikethrough, foreground/background RGB color, underline, and code-block indentation are still not strictly verified at Level 2. Per the requested rigor model, user-visible SGR styling and layout through a real terminal require `wezterm cli get-text --escapes`, `kitty @ get-text --ansi`, or equivalent captured-pane assertions that fail when the terminal capture does not contain the rendered style evidence. `--print-bytes` is useful as a diagnostic backstop, but it should not satisfy the Level 2 requirement.

Verification level present: Level 1 byte evidence for rich styling, plus partial Level 2 visible-text checks. Required: strict Level 2 capture assertions for representative rich Prose styling, with byte-dump checks kept separate.

## Test Rigor Assessment

- Parser requirements: Level 1 unit coverage exists for Markdown conversion, escapes, unknown tags, former atomic syntax, nesting, href protection, and opaque code blocks.
- Browser requirements: Level 1 output tests exist for escaping, semantic tags, links, style spans, code blocks, and unknown tags. That level is appropriate for HTML string/fragment generation.
- Plain Markdown requirements: Level 1 coverage exists for simple links and styling, but link destinations with Markdown delimiters are not covered and currently fail.
- MarkdownPlus requirements: Level 1 tests now verify foreground color, background color, RGB color, underline variants, semantic styles, and no JavaScript in generated style wrappers.
- Terminal requirements: Level 1 unit/PTY coverage is strong. Level 2 coverage exists, but rich styling tests currently allow `--print-bytes` to satisfy assertions, so the strongest strict verification for those user-visible style requirements remains Level 1.
- Keyboard/input UX requirements: none in this spec, so Level 3 is not required.

## Verification Performed

- `cargo test --color=never -p biscuit-terminal prose:: --lib` passed: 139 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test integration_test prose` passed: 5 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test integration_test public_docs_do_not_advertise_removed_atomic_tokens` passed: 1 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture` passed locally: 20 passed, 0 failed; Kitty-dependent cases skipped cleanly because `KITTY_LISTEN_ON` was unavailable.
- Manual repro for malformed Markdown href output: `bt prose --md '<a href="https://example.com/a)b">go</a>'` emitted `[go](https://example.com/a)b)`.

## Production Readiness

Not ready. The core IR, Browser, MarkdownPlus, atomic-token removal, docs migration, and broad Level 1 coverage are in much better shape, but Markdown link output can still be malformed and rich terminal styling has not met the requested strict Level 2 verification bar.
