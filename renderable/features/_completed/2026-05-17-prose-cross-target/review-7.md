---
ready: false
agent: codex
model: ""
---

# Review: Prose Cross-Target Rendering

## Findings

### High: Escaped `>` inside Prose link attributes still terminates the opening tag

The spec says the parser should keep the bracketed-tag escaping rules, preserve link href values, and let each target escape user text/attributes at its own boundary (`spec.md:172-180`, `spec.md:320`). The implementation adds `Prose::quoted_attr()` as the public helper for safely interpolating user-controlled attribute values, and its docs explicitly say it escapes `<`, `>`, and `\` so the attribute parser does not mis-identify the value boundary (`biscuit-terminal/lib/src/components/prose/prose.rs:134-145`, `biscuit-terminal/lib/src/components/prose/prose.rs:164-180`).

That contract is not actually met. The tag scanner in `parse_nodes` stops the opening tag at the first `>` byte without quote or backslash awareness (`biscuit-terminal/lib/src/components/prose/tokens.rs:326-335`). As a result, even a value produced by the documented escaping scheme is split before `parse_opening_tag` or the Browser/Markdown/Terminal emitters can preserve and escape it. Reproduction:

```sh
cargo run --color=never -q -p biscuit-terminal-cli -- prose '<a href="https://example.com/a\>b">x</a>'
```

Expected output is a link with visible text `x` and destination `https://example.com/a>b`. Actual non-TTY Markdown fallback is:

```text
[b">x](https://example.com/a\)
```

This affects all targets because the IR is already wrong: Browser never reaches its `escape_attribute` call with the intended href (`biscuit-terminal/lib/src/components/prose/browser.rs:57-63`), Markdown emits a malformed link, and Terminal OSC8/fallback receives the truncated destination. It also leaves a test gap: existing link tests cover underscores, parentheses, whitespace, and bracket escaping, but not parser-safe quoted attributes containing escaped tag delimiters.

Fix by making the tag-declaration scanner quote-aware and escape-aware, then add Level 1 parser plus Browser/Markdown/Terminal tests for `Prose::quoted_attr("a>b")` and a value containing both quote types. Level 1 is the correct verification level because this is deterministic parsing/serialization behavior, not terminal emulator behavior.

## Test Rigor Assessment

- Parser requirements: Level 1 coverage exists for Markdown conversion, escaped text, unknown tags, former atomic syntax, nesting, href emphasis protection, and opaque fenced code blocks. It does not cover escaped tag delimiters inside quoted attribute values.
- Browser requirements: Level 1 coverage checks escaped text, escaped code-block bodies, semantic tags, links, style spans, and unknown tags. The quoted-attribute failure means FR-4 is not fully verified for realistic user-controlled href values.
- Plain Markdown requirements: Level 1 coverage checks semantic styles, links with several destination edge cases, code blocks, style degradation, unknown tags, and Markdown sigil escaping. It misses the malformed-link case above.
- MarkdownPlus requirements: the review-6 inline-HTML escape issue is covered by new Level 1 tests for literal `<script>`, unknown tags, `&`, and nested semantic spans inside HTML wrappers.
- Terminal requirements: Level 1 and partial Level 2 coverage exists for SGR styling, OSC8, NO_COLOR, layout, rich styling, and nested code-block restoration. Level 3 is not required because this feature has no OS keyboard/input encoder behavior.

## Verification Performed

- Source review of the spec, Prose IR/parser, Browser/Markdown/Terminal emitters, docs, and Level 2 terminal tests.
- `cargo test --color=never -p biscuit-terminal prose:: --lib` passed: 162 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test integration_test prose` passed: 5 passed, 0 failed.
- Focused MarkdownPlus review-6 regression filter passed: 5 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture` was abandoned at the non-interactive 60-second bound. Before termination, WezTerm rich styling, wrapping, padding, SGR red, and nested code-block restoration passed; Kitty-dependent tests skipped cleanly because `KITTY_LISTEN_ON` was not set. Some WezTerm OSC8/NO_COLOR/nested-emphasis cases had not completed.
- Manual reproduction of the quoted-attribute parser bug with `bt prose '<a href="https://example.com/a\>b">x</a>'` produced malformed Markdown fallback output.

## Production Readiness

Not ready. The previous MarkdownPlus HTML escaping issue is fixed, but a public safe-attribute path can still corrupt link hrefs before any target renderer can preserve or escape them.
