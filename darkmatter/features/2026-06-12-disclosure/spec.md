---
created: 2026-06-12
status: draft
---

# Disclosure Blocks

Darkmatter's DSL should provide an ergonomic way to express a **disclosure
block** — a short summary line that, when clicked, reveals a larger hidden
body (it _discloses_ it). This is the terminal/web equivalent of the HTML
`<details>`/`<summary>` pair, but authored in portable Darkmatter Markdown and
lowered per render target.

This feature turns the description in
[`docs/rendering/disclosure.md`](../../docs/rendering/disclosure.md) into an
implementable plan. That doc is the **basis** for this spec; where the doc's
vocabulary ("markdown / markdown-plus / terminal / browser / json targets") and
Darkmatter's actual render-tree architecture (Terminal / Markdown / Browser /
MarkdownPlus folds) diverge, this spec reconciles them and the reconciliation
is called out explicitly.

## Status

**Draft.** Not implemented. The render-time disclosure DSL specified here is
being unified with the existing `disclosure="..."` option on `::file` and
`::code` transclusion directives
(`lib/src/markdown/compose/transclusion/wrappers.rs::wrap_disclosure`,
`transclusion/parser.rs`, `transclusion/types.rs::BlockOptions::disclosure`).
That option previously wrapped transcluded content in `<details>` **at compose
time**; it will now emit a `::disclosure / ::details / ::end-disclosure` block
around the content so the same render-time lowering applies. See
[Resolved Decisions](#resolved-decisions).

## Audience

Maintainers of Darkmatter's render pipeline. The reader is assumed to
understand the render-tree fold (`render_tree::fold_markdown_to_document`), the
block-extension processor
(`lib/src/markdown/render_tree/block_extension.rs`), the `renderable`
`NodeKind` model, and `style:` frontmatter wiring (`lib/src/style/`).

## Motivating the Render-Time Choice

A disclosure block has no portable CommonMark spelling. The only way a Markdown
viewer can render one is inline HTML (`<details>`/`<summary>`), and that is
target-specific and lossy to author. Two consequences fall out:

1. **Compose must not touch it.** During the
   [compose lifecycle](../../docs/darkmatter-compose-pipeline.md) the directive
   lines must survive **verbatim**, so a composed `.md` is still a clean
   Darkmatter document that can be re-rendered to any target. The legacy
   `::file` / `::code` `disclosure=` transclusion option is being migrated to
   emit the same DSL block rather than inline HTML, so it too passes through
   render-time lowering.
2. **Rendering owns the lowering.** Activation happens in the
   [rendering pipeline](../../docs/darkmatter-rendering-pipeline.md), where the
   target is known and each target can lower the block its own way.

## Syntax

```md
::disclosure
License Agreement
::details
Keep your dirty hands off my stuff. You have the right to leave immediately.
::end-disclosure
```

- `::disclosure` opens the block. The text **between** `::disclosure` and
  `::details` is the **summary** (the always-visible clickable label).
- `::details` separates summary from body. The text **between** `::details` and
  `::end-disclosure` is the **disclosed body**.
- `::end-disclosure` closes the block.

Both regions are parsed as Markdown, but with different constraints:

- The **summary** is all phrasing content from `::disclosure` up to
  `::details`. It may contain inline markup (emphasis, links, code spans,
  etc.) and soft line breaks. An **empty summary** region, a **hard line
  break**, or any **block-level element** (headings, lists, code blocks,
  paragraphs, nested disclosures, etc.) in the summary region is a
  malformed-block error.
- The **disclosed body** is arbitrary block content: paragraphs, lists, code
  blocks, and nested disclosures.

A malformed disclosure — missing `::details`, missing `::end-disclosure`, a
`::details` without a matching `::end-disclosure`, an empty summary region, or a
summary region that contains a hard line break or block-level element — is a
fatal `RenderError` handled by the block-extension processor.

Keyword recognition follows the conventions already established for
`::file-links` and `::block`: a directive keyword must be followed by ASCII
whitespace or end-of-line, and near-miss prose (`::disclosure-extra`,
`::detailsX`) is left as literal text. Directive lines inside fenced code blocks
are never interpreted (the block-extension processor already tracks fence
state).

## Render Targets

Darkmatter's CLI exposes five output choices for disclosure lowering. The
`json` output now exports the renderable `Document` tree IR (MDAST export is
removed); disclosure nodes appear natively in that IR.

| CLI output        | `OutputFormat`        | Render fold | Lowering |
|-------------------|-----------------------|-------------|----------|
| `terminal`        | `Terminal`            | Terminal    | Summary rendered normally; body rendered as a **block quote** whose text is **dim and italic** to separate it visually from the summary. This is the default CLI output. |
| `markdown`        | `Markdown`            | Markdown    | Emit the `::disclosure / ::details / ::end-disclosure` DSL verbatim. |
| `markdown-plus`   | `MarkdownPlus`        | Markdown    | Render summary and body to Markdown first, then wrap with inline HTML `<details><summary>…</summary>…</details>`. |
| `browser`         | `Browser`             | Browser     | Render summary and body to HTML first, then wrap with native `<details>`/`<summary>` elements. No JavaScript. |
| `json`            | `Json`                | —           | Export the renderable `Document` IR, including native `NodeKind::Disclosure` nodes. |

Notes:

- **Terminal is the default.** `md README.md` and `md render README.md` render
  to the terminal by default. The disclosure DSL is not mutated; both summary
  and body are shown, with the body styled dim and italic inside a block quote.
- The `markdown` target intentionally preserves the original DSL so the
  resulting `.md` remains a clean Darkmatter document that can be re-rendered.
- The `markdown-plus` target trades editability for portability: it produces
  inline HTML that CommonMark/GFM viewers render as collapsible details.
- The `json` target exports the renderable `Document` IR; the old `ast` alias is
  removed.

## Architecture & Lowering

Disclosure is a **block extension**, structurally analogous to the existing
HR-attribute lift documented in
[`renderable/features/_completed/2026-05-26-block-extension/spec.md`](../../../renderable/features/_completed/2026-05-26-block-extension/spec.md).

### Recognition

The `::disclosure … ::details … ::end-disclosure` triple is recognized at the
offset-event layer by the block-extension processor
(`lib/src/markdown/render_tree/block_extension.rs`). pulldown-cmark parses the
three directive lines as ordinary paragraph text; the processor matches the
opener, buffers events until the matching closer, splits on `::details`, and
emits one synthetic disclosure event over the original byte range. This keeps
recognition out of compose entirely and reuses the fence-state tracking the
processor already owns.

### Render-Tree Node

The fold lowers the synthetic event into a dedicated render-tree node:

```rust
NodeKind::Disclosure {
    summary: Vec<RenderNode>,
    children: Vec<RenderNode>,
}
```

- `summary` holds the always-visible label content (phrasing content, allowing
  soft line breaks but no hard line breaks or block-level elements).
- `children` holds the disclosed body content (arbitrary block content).

Each target fold (Terminal / Markdown / Browser / MarkdownPlus) lowers it per
the [Render Targets](#render-targets) table. For `MarkdownPlus` and `Browser`,
the summary and body are rendered to their respective target formats first,
then wrapped with `<details>`/`<summary>`. A target that does not
recognize the node must fall back to rendering summary-then-body so content is
never dropped.

## Style Frontmatter

A new optional `disclosure` bucket is added to `StyleFrontmatter`
(`lib/src/style/schema/mod.rs`), following the established per-bucket
`Option<…>` + kebab-case (with snake_case deprecation alias) pattern. The bucket
contains **layout properties only**, consistent with other block types.

```yaml
style:
  disclosure:
    margin-left: 2
    margin-right: 0
    width: 80
    max-width: 120
    min-width: 40
```

Supported properties:

- `margin-left` / `margin-right` — horizontal margins around the disclosure
  block.
- `width` — target width of the disclosure block.
- `max-width` / `min-width` — width bounds.

Kebab-case is canonical; snake_case is a deprecated alias and must still be
accepted. No `mode`, `indent`, or `color` properties are part of this bucket.
These values are overridden by instance-level `param=value` directives and
CLI base style switches per the precedence order in [CLI Scope](#cli-scope);
per-component CLI offset switches for disclosure do not exist. Disclosure
inherits the page defaults set by CLI base switches unless `style.disclosure`
or an instance-level parameter overrides them.

## CLI Scope

- **`--output {terminal,markdown,markdown-plus,browser,json}`** — selects the
  render target. `terminal` is the default. `markdown-plus` maps to the new
  `OutputFormat::MarkdownPlus` variant. The old `ast` alias and `auto`/`html`
  variants are removed.
- **Base style switches** (`--margin-left` / `--ml`, `--margin-right` / `--mr`,
  `--width`, `--max-width`, `--min-width`) set **page defaults** for the
  corresponding layout property. Disclosure inherits these page defaults unless
  overridden. This requires adding `min_width` to `renderable::Layout`.
- **Instance-level component styles** use Darkmatter directive `param=value`
  syntax attached to a specific disclosure instance, e.g.
  `::disclosure margin-left=1ch`.
- **Precedence order** for disclosure layout properties is:
  1. Instance-level `param=value` style on `::disclosure`
  2. `style.disclosure` frontmatter bucket
  3. CLI base style switch (page default)
  4. Built-in default
- **Per-component CLI offset switches** (e.g. `--disclosure-margin-right`) are
  **out of scope** for this spec.

### Breaking CLI Changes

- `--output auto` is removed; `terminal` is now the default.
- `--output html` is renamed to `--output browser`.
- `--output ast` is renamed to `--output json`.
- Pipes/scripts that previously received Markdown text from `Auto` on non-TTY
  now receive terminal output.

## Goals & Non-Goals

**Goals**

- Recognize the `::disclosure / ::details / ::end-disclosure` triple at render
  time without any compose-phase mutation.
- Lower the block correctly for Terminal (dim+italic body in a block quote),
  Markdown (verbatim DSL), MarkdownPlus (inline HTML `<details>`/`<summary>`),
  Browser (native `<details>`/`<summary>`), and JSON (native IR node).
- Add a `style.disclosure.*` bucket containing layout properties only:
  `margin-left`, `margin-right`, `width`, `max-width`, `min-width`.
- Add `OutputFormat::MarkdownPlus` and `OutputFormat::Json`, and wire
  `--output markdown-plus` and `--output json`.
- Unify the existing `::file ... disclosure="..."` and
  `::code ... disclosure="..."` transclusion options so they emit the
  render-time disclosure DSL block instead of wrapping content in inline HTML at
  compose time.
- Preserve content under every target — an unrecognized/disabled path must
  still emit summary + body.

**Non-Goals**

- Making the disclosure interactive in the terminal (no collapse/expand TUI).
  Terminal output is static; the body is always shown.
- JavaScript-driven browser behavior — the `<details>` element is sufficient.
- A separate alternate terminal style. The terminal presentation is fixed
  (dim+italic body in a block quote).
- Round-trip fidelity from the MarkdownPlus (inline-HTML) target back to the
  `::disclosure` DSL. The MarkdownPlus lowering is intentionally one-way and
  lossy.

## Migration / Implementation Plan

### Phase 1 — Recognition (compose-invariant)

- Extend the block-extension processor to match the disclosure triple and emit a
  synthetic event. Assert via test that the directive lines survive **compose**
  untouched.

### Phase 1b — Transclusion Unification

- Update the `::file ... disclosure="..."` and `::code ... disclosure="..."`
  transclusion paths to emit a
  `::disclosure / ::details / ::end-disclosure` block around the transcluded
  content instead of composing inline HTML `<details>`.
- Normalize the transclusion option `disclosure=true` (empty string summary) to
  the default summary `"Details"`.
- Remove the compose-time HTML wrapper (`wrap_disclosure`) so the same
  render-time lowering applies to both authored and transclusion-produced
  disclosure blocks.

### Phase 2 — Node + Folds

- Land `NodeKind::Disclosure { summary, children }` in `renderable`.
- Wire the Terminal fold (summary + dim/italic body inside a block quote), the
  Browser fold (`<details>`/`<summary>`), the Markdown fold (verbatim DSL), and
  the MarkdownPlus fold (inline HTML).
- Wire JSON export to emit the renderable `Document` IR, including native
  `NodeKind::Disclosure` nodes (the old MDAST `ast` export is removed).

### Phase 3 — Style + CLI

- Add the `style.disclosure` bucket and `apply_disclosure_style` with layout
  properties only.
- Add `OutputFormat::MarkdownPlus` and `OutputFormat::Json`; wire
  `--output markdown-plus` and `--output json`.
- Implement the disclosure style precedence order: instance-level
  `param=value` > `style.disclosure` frontmatter > CLI base style switch
  (page default) > built-in default.

### Phase 4 — Docs

- Promote `docs/rendering/disclosure.md` from "planned" to documented behavior;
  cross-link from the rendering-pipeline and style docs.

## Testing Requirements

- **Compose invariance:** a document containing a disclosure block, run through
  compose, is byte-identical in the disclosure region (directive lines intact).
- **Terminal target:** both summary and body appear; no inline HTML leaks; DSL
  not mutated; body text is dim and italic inside a block quote; layout
  overrides are honored with the precedence instance > frontmatter > CLI >
  default.
- **Markdown target:** emits the `::disclosure / ::details / ::end-disclosure`
  DSL verbatim.
- **MarkdownPlus target:** `--output markdown-plus` exists as a CLI value and
  routes through the `MarkdownPlus` fold, emitting well-formed
  `<details><summary>…</summary>…</details>`.
- **Browser target:** emits native `<details>`/`<summary>`; no script.
- **JSON target:** `--output json` exports the renderable `Document` IR; it
  contains native `NodeKind::Disclosure` nodes with `summary` and `children`
  populated.
- **Nested disclosures:** nested `::disclosure` blocks in the body render
  correctly (recursive lowerings) across Terminal, Markdown, MarkdownPlus,
  Browser, and JSON targets.
- **Summary constraint:** the summary may contain phrasing content and soft line
  breaks; an empty summary, hard line breaks, or block-level markup in the
  summary region are rejected with a fatal `RenderError`.
- **Transclusion unification:** the old `disclosure=` option on both `::file` and
  `::code` no longer calls `wrap_disclosure` and no longer emits `<details>`
  at compose time.
- **Transclusion default summary:** `::file ... disclosure=true` and
  `::code ... disclosure=true` normalize an empty-string summary to the default
  summary `"Details"`.
- **Empty summary:** an authored empty summary region is rejected as a fatal
  `RenderError`.
- **Instance-level style:** instance-level `param=value` style overrides
  (e.g. `::disclosure margin-left=1ch`) are honored.
- **Strict style:** `--strict-style` rejects unknown and deprecated keys inside
  `style.disclosure`.
- **Robustness:** near-miss keywords are literal text; directives inside fenced
  code blocks are ignored; a malformed disclosure (missing `::details`, missing
  `::end-disclosure`, `::details` without a matching closer, an empty summary
  region, or a summary region containing a hard line break or block-level
  element) raises a fatal `RenderError`.

## Resolved Decisions

The following questions were raised during drafting and are now decided:

- **Malformed-block policy:** malformed disclosures (missing `::details`,
  missing `::end-disclosure`, `::details` without a matching closer, an empty
  summary region, or a summary region containing a hard line break or
  block-level element) are a fatal `RenderError`, consistent with strict
  handling in the block-extension processor.
- **Nesting:** nested disclosures are **supported** inside the disclosed body.
  Each target fold must define a recursive lowering (Terminal block quote,
  MarkdownPlus nested inline HTML, Browser native nesting, etc.).
- **Summary richness:** the summary contains **phrasing content** up to
  `::details`; soft line breaks are allowed, but hard line breaks and
  block-level elements are rejected.
- **Name collision with `disclosure=` transclusion:** the existing
  `::file ... disclosure="..."` and `::code ... disclosure="..."` transclusion
  options are unified with this DSL. They now emit a
  `::disclosure / ::details / ::end-disclosure` block around the transcluded
  content at compose time and rely on the render-time lowering described above.
- **Empty transclusion summary:** the transclusion option `disclosure=true`
  (empty string summary) normalizes to the default summary `"Details"`. Authored
  empty summaries remain malformed and are rejected.

## Open Questions

No open questions remain.

## Success Criteria

- A disclosure block authored in Darkmatter Markdown renders correctly and
  distinctly across Terminal, Markdown, MarkdownPlus, Browser, and JSON targets.
- Compose never alters the disclosure DSL.
- `style.disclosure.*` layout properties, instance-level `param=value` styles,
  and CLI base style switches behave per the precedence rules above (instance
  > frontmatter > CLI (page default) > default).
- An authored empty summary is rejected as a fatal `RenderError`; the
  transclusion option `disclosure=true` normalizes an empty-string summary to
  `"Details"`.
- No content is ever dropped by any target or by a disabled/unrecognized path.
- A malformed disclosure raises a fatal `RenderError` rather than silently
  dropping or misrendering content.
- The `::file ... disclosure="..."` and `::code ... disclosure="..."`
  transclusion options produce the same render-time disclosure block as the
  authored DSL, with no separate compose-time `<details>` emitter remaining.
- `docs/rendering/disclosure.md` describes shipped behavior, not a plan.
