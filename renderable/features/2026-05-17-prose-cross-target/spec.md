# Spec: Prose Cross-Target Rendering

**Date:** 2026-05-17
**Status:** Draft
**Crates:** `biscuit-terminal`, `renderable`

## Problem

`Prose` is the primary inline styling component in `biscuit-terminal`. It is
widely used by higher-level terminal components and currently renders through a
terminal-only pipeline:

1. accept raw Prose input,
2. pre-process the supported Markdown subset,
3. parse atomic tokens and block tags,
4. emit ANSI/OSC8 output with terminal capability degradation.

The near-term rendering model needs `Prose` to implement
`renderable::browser::BrowserRenderable` and `renderable::MarkdownRenderable`.
Adding those targets by cloning the current parser into independent HTML and
Markdown string emitters would duplicate grammar rules and make drift likely.

At the same time, routing `Prose` primarily through the render tree is not a
good fit. The render tree is a canonical document-structure model. `Prose` is a
compact inline styling language with terminal-specific features: reset tokens,
capability-aware underline degradation, OSC8 behavior, foreground/background
colors, RGB colors, and fenced code-block terminal presentation.

## Goal

Give `Prose` one shared parsed representation that can render faithfully to
Terminal, Browser, Markdown, and MarkdownPlus, while preserving current
terminal behavior.

`Prose` should become a cross-target inline component without making the
render tree the source of truth for the Prose grammar.

## Non-Goals

- Replacing the render tree or changing its `NodeKind` vocabulary.
- Re-pointing `Prose::render()` through `TreeRenderable`.
- Making every Prose feature lossless in plain Markdown.
- Implementing a production `TreeRenderable for Prose` in this feature.
- Reworking higher-level components such as `Section`, `List`, or `Table`.
- Changing the public Prose input grammar.
- Removing terminal capability-aware behavior.

## Current Behavior

`Prose` accepts three input forms:

| Input form | Examples | Notes |
|------------|----------|-------|
| Atomic tokens | `{{bold}}`, `{{reset}}`, `{{bg-red}}` | Some tokens set styles until reset. |
| Block tags | `<bold>text</bold>`, `<a href="url">text</a>`, `<rgb #ff0000>text</rgb>` | Nestable, auto-reset on close. |
| Markdown subset | `[desc](url)`, `**bold**`, `_italic_`, fenced code blocks | Pre-processed into block tags before token parsing. |

The terminal renderer also handles target-specific behavior:

- OSC8 links when supported, Markdown-style link fallback otherwise.
- Double underline degraded to straight underline or plain text based on
  terminal support.
- Fenced code blocks rendered as dim, indented terminal text.
- Unknown tags and unknown tokens preserved as literal text.
- Backslash escaping for Prose-significant characters.

## Proposed Design

Introduce an internal parsed Prose representation and render all Prose targets
from it.

```rust
enum ProseNode {
    Text(String),
    Sequence(Vec<ProseNode>),
    Span {
        style: ProseStyle,
        children: Vec<ProseNode>,
    },
    Link {
        href: String,
        children: Vec<ProseNode>,
    },
    CodeBlock {
        lang: Option<String>,
        value: String,
    },
    Literal(String),
}

struct ProseDocument {
    children: Vec<ProseNode>,
}
```

The concrete names are not settled. The important requirement is that parsing
produces a target-neutral semantic model before any target emits output.

### Style Model

`ProseStyle` should represent Prose intent, not emitted escape strings:

- text weight: bold, dim, normal reset
- text style: italic, underline variants, strikethrough, blink, inverse, hidden
- foreground color: basic, bright, Tailwind, web color, RGB
- background color: basic, bright, Tailwind, web color, RGB
- reset semantics needed by atomic tokens

The parser must preserve enough structure for the terminal renderer to keep
its existing layer behavior. If atomic reset semantics cannot be represented
cleanly as nested spans, they should remain explicit operations in the IR
rather than being approximated.

### Parser Boundary

The existing Markdown pre-processor and token parser should be refactored, not
replaced wholesale.

The new parser should:

- keep the existing input grammar and escaping rules,
- preserve unknown tags/tokens as literal text,
- keep fenced code-block contents opaque,
- preserve link href values without markdown emphasis interpretation,
- build `ProseDocument` once per render call,
- avoid terminal-specific decisions while parsing.

Terminal capability decisions happen only in the terminal emitter.

## Target Rendering

### Terminal

`TerminalRenderable for Prose` remains the behavioral oracle.

The new terminal emitter must preserve current output for the existing Prose
test suite, including:

- ANSI style open/close and layer restoration,
- final reset behavior,
- OSC8 link output and fallback,
- double-underline degradation,
- code-block dim indentation,
- layout application after inline rendering.

Existing Prose terminal tests should pass unchanged except where the test is
intentionally updated to exercise the new parser directly.

### Browser

`BrowserRenderable for Prose` should emit a `BrowserFragment<Ready>`.

Recommended shape:

- root element: a small block wrapper such as `<span>` or `<div>` depending on
  current Prose layout semantics,
- text nodes: escaped by the browser fragment renderer,
- links: `<a href="...">...</a>`,
- semantic styles: `<strong>`, `<em>`, `<s>` where direct semantic HTML exists,
- presentational styles: `<span style="...">...</span>` or scoped CSS classes,
- code blocks: `<pre><code>` with optional language class or data attribute.

Raw HTML should be avoided for ordinary Prose output. Use `RawHtml` only if the
typed fragment API cannot express the needed shape.

Browser rendering is infallible by trait contract. Invalid or unsupported
Prose constructs must degrade to escaped literal text, matching terminal's
literal-preservation behavior.

### Markdown

`MarkdownRenderable::render_markdown()` should emit idiomatic plain Markdown
for semantic constructs and readable literal text for target-specific styling.

Recommended mapping:

| Prose construct | Markdown output |
|-----------------|-----------------|
| plain text | escaped Markdown text |
| bold | `**text**` when safe |
| italic | `_text_` when safe |
| link | `[text](href)` |
| strikethrough | `~~text~~` if accepted for plain Markdown in this repo; otherwise literal text |
| code block | fenced code block |
| colors/backgrounds/underline variants | inner text only |
| hidden/blink/inverse | inner text only |

Plain Markdown should prefer portable readability over visual fidelity.

### MarkdownPlus

`MarkdownRenderable::render_markdown_plus()` may use inline HTML for styles
that plain Markdown cannot carry.

Recommended mapping:

| Prose construct | MarkdownPlus output |
|-----------------|---------------------|
| semantic bold/italic/link/code | Markdown when safe |
| foreground/background colors | `<span style="...">...</span>` |
| underline variants | `<span style="text-decoration: ...">...</span>` |
| strikethrough | Markdown or `<s>` |
| hidden/blink/inverse | `<span style="...">...</span>` only when useful and accessible |

MarkdownPlus must not rely on JavaScript.

## Relationship To Render Tree

`Prose` should not use `TreeRenderable` as its primary rendering path.

The terminal tree renderer already projects inline tree nodes into Prose
markup, then asks `Prose` to render terminal output. That direction should
remain:

```text
RenderNode inline subtree -> Prose markup -> terminal output
```

A future `TreeRenderable for Prose` may be useful as an adapter, but it should
be explicitly lossy or metadata-bearing:

- semantic constructs project to `Text`, `Strong`, `Emphasis`, `Delete`,
  `Link`, `InlineCode`, and `Code`,
- Prose-specific styling projects to `Span` classes/data where possible,
- terminal-only reset/capability semantics are not guaranteed to round-trip.

That adapter is optional and should be gated by a concrete use case and parity
tests.

## Requirements

- **FR-1** — `Prose` MUST implement `BrowserRenderable`.
- **FR-2** — `Prose` MUST implement `MarkdownRenderable`.
- **FR-3** — Terminal rendering MUST preserve existing visible output and ANSI
  behavior for the current Prose test suite.
- **FR-4** — Browser rendering MUST escape user text and attribute values.
- **FR-5** — Markdown rendering MUST escape Markdown-significant literal text
  where needed to avoid changing content meaning.
- **FR-6** — MarkdownPlus MAY use inline HTML but MUST NOT require JavaScript.
- **FR-7** — Unknown Prose tags and tokens MUST remain visible as literal text
  across all targets.
- **FR-8** — The parser MUST keep target-specific capability decisions out of
  the parsed representation.
- **FR-9** — Layout remains applied by the terminal `TerminalRenderable` path.
  Browser layout mapping is limited to existing Prose layout fields that have
  clear CSS equivalents.

## Testing

Add tests at three levels.

### Parser tests

- Markdown subset converts into the expected IR.
- Backslash escapes remain literal.
- Unknown tags/tokens are literal.
- Nested spans preserve order and nesting.
- Links protect href contents from emphasis parsing.
- Fenced code blocks are opaque.

### Target tests

- Terminal output matches existing snapshots/assertions.
- Browser output contains escaped text, semantic tags, links, and style spans.
- Markdown output is readable and portable.
- MarkdownPlus preserves color/underline styling via HTML where specified.

### Parity tests

Terminal parity is mandatory: old Prose terminal output vs new IR-backed
terminal output.

Browser and Markdown do not have an old oracle. Their tests should assert
explicit expected strings or structural fragment properties.

## Acceptance Criteria

- `Prose` implements `BrowserRenderable` and `MarkdownRenderable`.
- Existing terminal Prose tests pass.
- New Browser and Markdown tests cover at least:
  - plain text,
  - bold,
  - italic,
  - nested bold/italic,
  - link,
  - strikethrough,
  - foreground color,
  - background color,
  - RGB color,
  - code block,
  - unknown tag,
  - escaped Markdown sigils.
- No public Prose grammar changes are required by callers.
- No render-tree `NodeKind` changes are required.

## Open Questions

- Should the parsed Prose representation be public, crate-public, or private?
- Should Browser output use inline styles initially, or scoped component CSS
  classes with a `ComponentStylesheet`?
- Should plain Markdown include GFM strikethrough (`~~text~~`) or reserve that
  for MarkdownPlus?
- How much of Prose `Layout` should map to Browser CSS in the first pass?
- Should the parser cache the parsed document inside `Prose`, or parse per
  render call as it does today?

## Future Work

- Optional `TreeRenderable for Prose` projection with documented lossiness.
- Shared color/style conversion helpers between Prose, render tree, and
  `renderable::color`.
- Browser accessibility review for hidden, blink, inverse, and low-contrast
  color combinations.
- Broader adoption by higher-level components once `Prose` is cross-target.
