---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the clarify.md prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
clarified: false
reviewed: false
review_iterations: 0
implemented: false
needs_rulings: true
required_rulings:
    - "Q1 — which freely licensed font, and how it is chosen or bundled"
    - "Q2 — subsetting crate, and the size budget per weight"
    - "Q3 — how the opt-in is exposed (HtmlOptions field, md flag, frontmatter)"
    - "Q4 — interaction with light and dark themes"
    - "Q5 — whether the terminal raster path can use the same pinned font"
depends-on:
    - 2026-10-21-merman
related:
    - 2026-10-21-merman
---

# Embedded font for Mermaid static SVG

> **Draft.** This records a goal, a constraint, and the questions that block
> design. It has not been clarified or reviewed, and nothing here is decided
> beyond what `2026-10-21-merman` already decided.

## Goal

An **opt-in** for Darkmatter's static-SVG HTML output that embeds the font the
diagram was measured with into the page, as a subsetted WOFF2 `@font-face`
data URI, so diagrams render exactly as measured on any reader's machine.

## Why

Under `2026-10-21-merman` (D37), static-SVG HTML embeds an SVG measured with
the author's host fonts. Each label's `font-family` leads with the measured
face, and label overflow is allowed, because the reader's fonts cannot be
known. On a reader's machine without that face, labels can crowd or overrun
their shapes. Embedding the face removes the guess.

The default HTML path (`<pre class="mermaid">`, rendered by Mermaid.js in the
reader's browser) is not affected.

## Key constraint: licensing

System fonts such as Trebuchet MS, Helvetica, and Arial are typically
proprietary, and embedding them in a published page redistributes them.
Therefore the opt-in must **measure with and embed a freely licensed font**
(for example DejaVu, Inter, or Noto), not whatever face the host's font chain
happens to resolve. Turning the opt-in on changes the measured face, not only
the output.

*Inferred, to confirm:* the chosen font's license may require a notice to
travel with the page or the package. What it requires depends on Q1.

## Owners

| Area | Responsibility |
| --- | --- |
| `biscuit-visualized` | Selecting the face for measurement and drawing; subsetting it to the glyphs used; embedding the `@font-face` rule in the SVG |
| `darkmatter` | The opt-in switch; the SVG sanitizer admitting `@font-face` with `data:` URLs only, while keeping its rule against external URLs |

## Example markup

```html
<style>@font-face{font-family:"bv-mermaid";src:url(data:font/woff2;base64,…) format("woff2")}</style>
<text font-family="bv-mermaid, sans-serif">Animal</text>
```

## Open questions

1. **Q1. Which font.** Which freely licensed font is used, and is it bundled
   with `biscuit-visualized` or selected from the host? How does this relate
   to `MERMAID_FONT` (`2026-10-21-merman`, D32): does the opt-in override it,
   or require it to name a free font? Is bold a separate file?
2. **Q2. Subsetting and size.** Which crate subsets and encodes WOFF2, and
   what is the size budget per weight? A page with many diagrams may embed the
   subset once or once per diagram.
3. **Q3. How the opt-in is exposed:** an `HtmlOptions` field, an `md` flag,
   frontmatter, or several.
4. **Q4. Themes.** Does anything about the embedded font interact with light
   and dark themes (for example a page that switches theme after load)?
5. **Q5. The terminal raster path.** Should terminal rendering also be able to
   use the same pinned font? This ties to `2026-10-21-merman`'s golden-test
   font question (Q22 there), which may pin a free font for tests.

Not yet asked and possibly needed: scripts outside the chosen font's coverage
(CJK, emoji) and whether fallback faces are embedded too; how a cache entry
distinguishes embedded from non-embedded output.

## Acceptance criteria (sketch)

| ID | Statement | Validated by |
| --- | --- | --- |
| AC-1 | A page with the opt-in renders identical label widths on two hosts with different installed fonts | Render the same page in a headless browser on two hosts; compare label boxes |
| AC-2 | Darkmatter's sanitizer still rejects an external `url()`, in `@font-face` and elsewhere, while admitting a `data:` font URL | Sanitizer tests, positive and negative |
| AC-3 | The embedded subset covers every glyph the diagram's labels use | Test comparing the label text's code points with the subset's cmap |

Success metrics, performance limits, and failure behavior (for example a font
that cannot be subset) are not yet defined.
