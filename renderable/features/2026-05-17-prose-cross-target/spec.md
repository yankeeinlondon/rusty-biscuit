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

As part of this work the **atomic-token grammar (`{{token}}`) is removed**.
`Prose` will accept only bracketed tags and the Markdown subset. This is a
deliberate breaking grammar change — see [Grammar Change](#grammar-change) and
[Migration](#migration-atomic-token-removal).

## Grammar Change

The atomic-token grammar is unscoped: `{{bold}}` turns a style on with no
matching close, and atomic tokens can outlive an enclosing bracketed tag
(`<b>x {{red}}y</b> z` leaves `z` red but not bold). That produces overlapping,
non-nestable style ranges, which cannot be represented as a tree and cannot be
emitted as open/close pairs for Browser or Markdown without a reconstruction
algorithm.

Bracketed tags (`<bold>…</bold>`) and the Markdown subset are well-nested by
construction. Removing atomic tokens lets the parsed representation be a pure
tree and removes the only construct that does not map cleanly to every target.

Call-site analysis (2026-05-17): atomic tokens remain in live use in ~218
matches across 12 non-test files, concentrated in Claudine hook/action UI
code. Bracketed tags are already the dominant authoring style across the
monorepo and the form emitted by the render-tree terminal projection. Every
atomic use has a bracketed equivalent (`{{bold}}x{{reset}}` → `<bold>x</bold>`).

## Non-Goals

- Replacing the render tree or changing its `NodeKind` vocabulary.
- Re-pointing `Prose::render()` through `TreeRenderable`.
- Making every Prose feature lossless in plain Markdown.
- Implementing a production `TreeRenderable for Prose` in this feature.
- Reworking higher-level components such as `Section`, `List`, or `Table`.
- Changing the bracketed-tag or Markdown-subset input grammar (only the atomic
  grammar is removed).
- Removing terminal capability-aware behavior.

## Current Behavior

`Prose` currently accepts three input forms. **This feature removes the first
one** (atomic tokens); the table below describes today's behavior:

| Input form | Examples | Notes |
|------------|----------|-------|
| Atomic tokens *(being removed)* | `{{bold}}`, `{{reset}}`, `{{bg-red}}` | Unscoped — set styles until an explicit reset. |
| Block tags | `<bold>text</bold>`, `<a href="url">text</a>`, `<rgb #ff0000>text</rgb>` | Nestable, auto-reset on close. |
| Markdown subset | `[desc](url)`, `**bold**`, `_italic_`, fenced code blocks | Pre-processed into block tags before parsing. |

After this feature, only **block tags** and the **Markdown subset** remain.

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

Because the atomic grammar is removed, every styled region is well-nested and
the representation is a **pure tree** — no flat style-operation or reset
variants are needed.

```rust
enum ProseNode {
    Text(String),
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
}

struct ProseDocument {
    children: Vec<ProseNode>,
}
```

The concrete names are not settled. The important requirement is that parsing
produces a target-neutral semantic model before any target emits output.

There is deliberately **no `Literal` variant**: unknown tags route to `Text`,
so every target escapes them by its own rules (a target-neutral IR cannot hold
target-specific escaped strings). `Text` holds **fully decoded** content —
backslash escapes resolved, no input syntax — and each emitter re-escapes for
its target on output (Terminal: none; Browser: HTML-escape; Markdown: escape
Markdown sigils).

### Style Model

`ProseStyle` should represent Prose intent, not emitted escape strings:

- text weight: bold, dim
- text style: italic, underline variants, strikethrough, blink, inverse, hidden
- foreground color: basic, bright, Tailwind, web color, RGB
- background color: basic, bright, Tailwind, web color, RGB

Each `Span` carries the styles its bracketed tag applies; closing the tag ends
the span. No standalone reset operations are needed — with the atomic grammar
removed, resets are implicit at span boundaries. The terminal emitter keeps its
existing layer-restoration behavior for *nested* spans.

### Parser Boundary

The existing Markdown pre-processor and tag parser should be refactored, not
replaced wholesale. The atomic-token parsing path is deleted (see
[Migration](#migration-atomic-token-removal)).

The new parser should:

- keep the bracketed-tag and Markdown-subset grammar and escaping rules,
- drop atomic-token recognition — `{{...}}` becomes ordinary literal text,
- route unknown tags to `Text` (escaped per target, never emitted verbatim),
- store `Text` as fully decoded content (backslash escapes resolved),
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

## Migration: Atomic Token Removal

Removing the atomic grammar is a breaking change to `Prose` input. Once removed,
a stray `{{bold}}` renders as the literal text `{{bold}}` — visible and easy to
spot, but wrong. Migration must therefore be sequenced as a **prerequisite
phase**, completed before the IR work lands:

1. **Migrate call sites first.** Convert every atomic-token use to the
   equivalent bracketed tag (`{{bold}}x{{reset}}` → `<bold>x</bold>`). Scope:
   ~218 matches across 12 non-test files, concentrated in Claudine hook/action
   UI code (`claudine/cli/src/commands/hooks/` and `actions.rs`).
2. **Audit for incremental construction.** Most uses are inline open/reset
   pairs and convert mechanically. Some build a styled region across multiple
   `push_str` / `format!` calls (e.g. `hooks/list.rs`); these need a real
   refactor to collect the region into one string before wrapping it in a tag.
   This is not a find-and-replace.
3. **Remove the atomic grammar.** Delete the atomic-token parsing path and the
   `atomic_token_*` tables. The reset tokens (`{{reset}}`, `{{reset-fg}}`,
   `{{not-italic}}`, `{{normal-font-weight}}`, …) are removed with it — closing
   a bracketed tag resets implicitly.
4. **Update `bt prose` CLI** help text and examples, and `prose.md` docs.

Keeping atomic-token *recognition* while internally rewriting it to bracketed
form is **not** an option: that rewrite is exactly the overlapping-range
reconstruction problem the removal exists to avoid. The cut must be genuine.

This is an internal monorepo with no external `Prose` consumers, so a clean cut
(no deprecation-warning release) is acceptable.

## Requirements

- **FR-1** — `Prose` MUST implement `BrowserRenderable`.
- **FR-2** — `Prose` MUST implement `MarkdownRenderable`.
- **FR-3** — Terminal rendering MUST preserve existing visible output and ANSI
  behavior for the current Prose test suite.
- **FR-4** — Browser rendering MUST escape user text and attribute values.
- **FR-5** — Markdown rendering MUST escape Markdown-significant literal text
  where needed to avoid changing content meaning.
- **FR-6** — MarkdownPlus MAY use inline HTML but MUST NOT require JavaScript.
- **FR-7** — Unknown Prose tags MUST remain visible across all targets as
  `Text`, escaped per target (never emitted verbatim). Former atomic-token
  syntax (`{{...}}`) is likewise treated as ordinary text.
- **FR-8** — The parser MUST keep target-specific capability decisions out of
  the parsed representation.
- **FR-9** — Layout remains applied by the terminal `TerminalRenderable` path.
  Browser layout mapping is limited to existing Prose layout fields that have
  clear CSS equivalents.

## Testing

Add tests at three levels.

### Parser tests

- Markdown subset converts into the expected IR.
- Backslash escapes resolve in the IR (e.g. `\_` parses to `Text("_")`).
- Unknown tags, and former atomic syntax (`{{...}}`), parse to `Text`.
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
- The atomic-token grammar is removed; all monorepo call sites are migrated to
  bracketed tags first (see [Migration](#migration-atomic-token-removal)).
- No bracketed-tag or Markdown-subset grammar changes are required by callers.
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
