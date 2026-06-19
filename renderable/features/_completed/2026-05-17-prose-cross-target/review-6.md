---
ready: false
agent: codex
model: ""
---

# Review: Prose Cross-Target Rendering

## Findings

### High: MarkdownPlus inline-HTML wrappers can turn escaped literal text back into raw HTML

The spec allows MarkdownPlus to use inline HTML for styles that plain Markdown cannot carry, but FR-6 says MarkdownPlus must not require JavaScript and FR-7 says unknown tags must remain visible as escaped text across all targets (`renderable/features/2026-05-17-prose-cross-target/spec.md:247`, `renderable/features/2026-05-17-prose-cross-target/spec.md:323`, `renderable/features/2026-05-17-prose-cross-target/spec.md:324`). The current MarkdownPlus renderer escapes text as Markdown first (`biscuit-terminal/lib/src/components/prose/to_markdown.rs:46`, `biscuit-terminal/lib/src/components/prose/to_markdown.rs:167`), then injects that already-rendered string directly inside raw inline-HTML wrappers for colors and underline (`biscuit-terminal/lib/src/components/prose/to_markdown.rs:103`, `biscuit-terminal/lib/src/components/prose/to_markdown.rs:111`, `biscuit-terminal/lib/src/components/prose/to_markdown.rs:114`).

That is not an HTML escape boundary. For example, a styled literal like `<red>\<script\>alert(1)\</script\></red>` is parsed as text, but MarkdownPlus wraps the Markdown-escaped `\<script\>...` inside `<span style="color: ...">...</span>`. Backslashes do not escape `<` in HTML, so a Markdown/HTML renderer that accepts inline HTML can see an actual nested `<script>` tag inside the span instead of inert visible text. The same applies to unknown tags such as `<red><unknown>x</unknown></red>`: the plain text path emits Markdown escapes, but the MarkdownPlus HTML wrapper can reintroduce raw `<unknown>` markup.

Fix by making the MarkdownPlus HTML-emitting path render children with an HTML-aware escape mode whenever those children become the body of an inline HTML element, or by avoiding raw HTML wrappers around arbitrary already-Markdown-rendered content. Add regression tests for colored and underlined literal `<script>`, unknown tags inside color spans, and `&` text inside MarkdownPlus style spans.

Verification level present: Level 1 tests cover that MarkdownPlus uses no JavaScript for a benign styled string (`biscuit-terminal/lib/src/components/prose/to_markdown.rs:367`), but there is no Level 1 coverage for user-controlled literal HTML inside MarkdownPlus inline-HTML wrappers. Required: Level 1 is sufficient because this is a string/serialization requirement, not terminal emulator behavior.

## Test Rigor Assessment

- Parser requirements: Level 1 coverage includes Markdown subset conversion, escaped text, unknown tags, former atomic syntax, nesting, href protection, fenced code-block opacity, and multiple fenced blocks.
- Browser requirements: Level 1 coverage checks escaped text, escaped code-block bodies, semantic tags, links, style spans, and unknown tags.
- Plain Markdown requirements: Level 1 coverage checks semantic styles, links with delimiter-bearing destinations, code blocks, style degradation, unknown tags, and Markdown sigil escaping.
- MarkdownPlus requirements: Level 1 coverage checks semantic styles and color/underline preservation for benign content, but misses the inline-HTML escape boundary above.
- Terminal requirements: Level 1 and Level 2 coverage now exists for SGR styling, OSC8, NO_COLOR, layout, rich styling, and the prior nested code-block restoration regression. Level 3 is not required because this feature has no OS-keyboard/input encoder behavior.

## Verification Performed

- Source review of the spec, the Prose IR/parser, terminal/browser/Markdown emitters, docs, and Level 2 terminal tests.
- `cargo test --color=never -p biscuit-terminal prose:: --lib` passed: 156 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test integration_test prose` passed: 5 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture` was abandoned after exceeding the non-interactive 60-second bound. Before termination, several tests passed, including WezTerm SGR, NO_COLOR, nested-emphasis visibility, layout width/wrap, and skip-clean Kitty cases. The remaining WezTerm rich-style, OSC8, and nested-code-block Level 2 tests had not completed.

## Production Readiness

Not ready. The terminal review-5 issue appears fixed, but MarkdownPlus can emit raw user-authored HTML inside style spans.
