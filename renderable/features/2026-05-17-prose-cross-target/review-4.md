---
ready: false
agent: codex
model: ""
---

# Review: Prose Cross-Target Rendering

## Findings

### High: fenced code blocks are not opaque when their body contains the synthetic closing tag

The spec requires fenced code-block contents to remain opaque (`renderable/features/2026-05-17-prose-cross-target/spec.md`: parser tests and FR coverage), but the implementation restores lifted fences as synthetic `<code-block>...</code-block>` markup before passing them into the normal tag parser (`biscuit-terminal/lib/src/components/prose/markdown.rs:102-114`). The parser then finds the first literal `</code-block>` while scanning inner content (`biscuit-terminal/lib/src/components/prose/tokens.rs:198-244`), so a code fence body containing that text prematurely terminates the code node and lets the remaining text be parsed as Prose markup.

Manual repro:

```bash
cargo run --quiet --color=never -p biscuit-terminal-cli -- prose --html $'```\n</code-block><red>x</red>\n```'
```

Observed output:

```html
<span class="prose"><pre><code></code></pre><span style="color: rgb(128, 0, 0)">x</span>&lt;/code-block&gt;</span>
```

The expected behavior is one escaped `<pre><code>` body containing the literal `</code-block><red>x</red>`. This affects every target that consumes the shared IR: Browser renders styled HTML outside the code block, Markdown produces a broken empty fence plus escaped tail text, and Terminal can style content that was authored inside a fence.

The underlying issue is that code-block opacity is implemented by placeholder restoration into the same string grammar rather than by constructing a `ProseNode::CodeBlock` directly, or by using an unambiguous placeholder that the tag scanner cannot collide with user code.

Verification level present: Level 1 tests cover code blocks containing Markdown emphasis, but not code blocks containing Prose tag delimiters or the synthetic `</code-block>` sentinel. Required: Level 1 parser and target tests for fenced bodies containing `</code-block>`, `<red>`, unknown tags, and backslash-heavy text.

## Test Rigor Assessment

- Parser requirements: Level 1 coverage exists for Markdown conversion, escapes, unknown tags, former atomic syntax, nesting, href protection, and basic opaque code blocks. The strongest test for code-block opacity is incomplete because it does not cover Prose tag delimiters inside the fence.
- Browser requirements: Level 1 output tests cover escaping, semantic tags, links, style spans, code blocks, and unknown tags. The code-block sentinel case above is missing and currently fails.
- Plain Markdown requirements: Level 1 tests now cover delimiter-bearing link destinations and common styles. The code-block sentinel case is missing and currently fails.
- MarkdownPlus requirements: Level 1 tests cover colors, background colors, RGB, underline variants, semantic styles, and no JavaScript in generated style wrappers.
- Terminal requirements: Level 1 unit coverage is broad. The updated Level 2 rich-styling tests now assert captured terminal SGR rather than `--print-bytes`, which addresses the previous verification-level mismatch for rich styling. In this run, the Level 2 suite did not complete because several WezTerm tests exceeded 60 seconds and had to be terminated, so I am not treating the full Level 2 suite as passing locally.
- Keyboard/input UX requirements: none in this spec, so Level 3 is not required.

## Verification Performed

- `cargo test --color=never -p biscuit-terminal prose:: --lib` passed: 144 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test integration_test prose` passed: 5 passed, 0 failed.
- `cargo test --color=never -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture` did not complete; after several tests passed, multiple WezTerm tests were still running past 60 seconds, so I terminated the process.
- Manual repro for fenced-code opacity failure with `bt prose --html` produced styled `<red>` output outside the code block.
- Manual check for the prior Markdown href issue now passes: `bt prose --md '<a href="https://example.com/a)b">go</a>'` emits `[go](https://example.com/a\)b)`.

## Production Readiness

Not ready. The prior Markdown href and strict Level 2 rich-styling concerns appear to have been addressed, but fenced code-block opacity is still broken for a user-authored code body that contains the parser's synthetic closing tag.
