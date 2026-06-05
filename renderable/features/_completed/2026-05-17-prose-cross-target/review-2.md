---
ready: false
agent: codex
model: ""
---

# Review: Prose Cross-Target Rendering

## Findings

### High: MarkdownPlus drops the styling it is supposed to preserve

The spec calls out MarkdownPlus as the richer Markdown target and says color and underline styling should be preserved with inline HTML where specified (`renderable/features/2026-05-17-prose-cross-target/spec.md:245`, `:255`, `:256`, `:351`). The implementation currently makes `render_markdown_plus()` return exactly the same output as plain Markdown (`biscuit-terminal/lib/src/components/prose/to_markdown.rs:23`), and the test locks that behavior in (`biscuit-terminal/lib/src/components/prose/to_markdown.rs:179`).

Impact: `<red>x</red>`, `<bg-coral>x</bg-coral>`, and `<underline>x</underline>` all lose their target-specific styling in MarkdownPlus even though this target exists specifically to carry richer presentation. Plain Markdown degradation is fine; MarkdownPlus should emit safe inline HTML such as `<span style="color: ...">x</span>` / `<span style="text-decoration: ...">x</span>`.

Verification level present: Level 1 unit tests, but they assert the wrong behavior. Required: Level 1 target tests that verify MarkdownPlus preserves foreground color, background color, RGB color, and underline variants without JavaScript.

### High: Terminal styling parity is still only partially verified at the required level

Most terminal behavior is covered by in-process unit assertions over emitted bytes and Level 1 PTY tests (`biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs:54`). The only Level 2 Prose coverage is Apple Terminal degradation for OSC8 fallback and double-underline visibility (`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs:59`, `:138`). There is no Level 2 WezTerm/Kitty/tmux capture for the normal rich path: nested bold/italic, foreground/background/RGB color, strikethrough, underline variants, code-block indentation, and layout after the new IR-backed renderer.

Impact: the feature changes the terminal rendering engine from direct parsing to IR-backed emission, but production-visible SGR styling and layout still rely on Level 1 byte tests. Per the requested rigor model, real-terminal rendering of SGR styling, glyph widths, and scrolling/layout needs Level 2 capture before the feature can be marked production-ready.

Verification level present: Level 1 for rich terminal styling; Level 2 only for Apple Terminal fallback visibility. Required: Level 2 capture for representative `bt prose` cases in a terminal emulator that preserves rich styling captures, plus the existing Level 1 byte tests.

### Medium: Public docs still teach removed atomic-token syntax

The implementation removes atomic-token parsing, and the `bt prose` no-content hint was updated, but public docs still advertise `{{...}}` as supported syntax. Examples remain in `biscuit-terminal/cli/README.md:81`, `biscuit-terminal/cli/README.md:84`, `biscuit-terminal/cli/README.md:93`, `biscuit-terminal/docs/components/compose.md:15`, `biscuit-terminal/docs/components/compose.md:22`, `biscuit-terminal/docs/components/compose.md:29`, and `claudine/cli/README.md:295`. The spec explicitly includes updating `bt prose` help/examples and Prose docs in the atomic-token removal cutover (`renderable/features/2026-05-17-prose-cross-target/spec.md:285`).

Impact: users will copy examples that now render literal `{{bold}}` / `{{reset}}` text. This is especially confusing because some Prose docs correctly state the grammar was removed while nearby README material says the opposite.

Verification level present: a source/docs grep finds the stale examples. Required: update public examples to bracketed tags and add a scoped negative check for Prose-style atomic tokens in public docs/examples.

### Medium: The shared `TextEmphasis` style leaf was not implemented

The spec says `ProseStyle` is built from shared leaf primitives, with bold/italic/dim/underline/strikethrough/blink represented by `renderable::style::TextEmphasis` and shared terminal/browser emitters (`renderable/features/2026-05-17-prose-cross-target/spec.md:145`, `:150`, `:153`, `:157`). The current `ProseStyle` keeps a parallel set of boolean fields plus a local `UnderlineKind` (`biscuit-terminal/lib/src/components/prose/ir.rs:38`).

Impact: the implementation works functionally today, but it keeps the duplicated style model the spec was trying to avoid. Future render-tree and Prose styling changes can drift because terminal and browser style emission are still separate local implementations.

Verification level present: none for the architectural contract. Required: either implement the shared leaf in `renderable` and use it from Prose, or revise the spec to make the local style container an explicit accepted compromise.

## Test Rigor Assessment

- Parser requirements: Level 1 unit coverage exists for Markdown conversion, escapes, unknown tags, former atomic syntax, nesting, href protection, and opaque code blocks.
- Browser requirements: Level 1 string/fragment tests exist for escaping, semantic tags, links, style spans, code blocks, and unknown tags. That level is appropriate for non-terminal HTML generation.
- Plain Markdown requirements: Level 1 unit tests exist for readable Markdown output and literal escaping. Add link-destination edge cases if URLs with `)` or whitespace are considered supported input.
- MarkdownPlus requirements: Level 1 exists but verifies degradation to plain Markdown, so the strongest test is for the wrong behavior.
- Terminal rich rendering: Level 1 byte/unit and PTY coverage exists. Level 2 coverage is limited to Apple Terminal fallback visibility, not normal rich styling/color/layout through a real terminal.
- Keyboard/input UX requirements: none in this spec, so Level 3 is not required.

## Verification Performed

- `cargo test --color=never -p biscuit-terminal prose:: --lib` passed: 132 passed, 0 failed.
- Searched for stale Prose atomic tokens in live source/docs with `rg`.

## Production Readiness

Not ready. The core IR and cross-target trait implementations are now present, and atomic parsing is removed, but MarkdownPlus does not meet the richer-target behavior, real-terminal verification is below the requested bar for rich SGR styling, and public docs still advertise removed syntax.
