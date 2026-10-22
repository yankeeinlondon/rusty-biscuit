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
status: finalized-spec
clarified: true
clarified_by: claude/opus
reviewed: true
reviewed_by: claude/opus
reviewed_on: 2026-10-22
review_note: "the clarification process served as a review"
review_iterations: 0
implemented: false
needs_rulings: false
depends-on:
    - 2026-10-21-merman
related:
    - 2026-10-21-merman
---

# Embedded font for Mermaid static SVG

An **opt-in** for Darkmatter's static-SVG HTML output that embeds the face the
diagrams were measured with into the page, so diagrams render with the same
label widths on every reader's machine. The face is the bundled Inter of
`2026-10-21-merman` (D40 there).

Ken made the rulings below on 2026-10-22, including D10, which settles how
the subset is made (Q6).

## Why

Under `2026-10-21-merman` (D37 there), static-SVG HTML embeds an SVG measured
with the author's host fonts. Each label's `font-family` leads with the
measured face, and label overflow is allowed, because the reader's fonts
cannot be known. On a reader's machine without that face, labels can crowd or
overrun their shapes. Embedding the face removes the guess.

The default HTML path (`<pre class="mermaid">`, rendered by Mermaid.js in the
reader's browser) is not affected.

## Scope

**In scope**

- An `HtmlOptions` field and an `md` CLI flag that turn the opt-in on (D3).
- Measuring with Inter whenever the opt-in is on (D6).
- Subsetting Inter to the code points a page uses with `allsorts`, and
  embedding the subset once per page as a TrueType data URI (D2, D4, D10).
- A first-task browser check of the subset, with a fixed fallback to
  unsubset Inter (D11).
- The failure fallback (D7) and the license notice (D8).

**Out of scope**

- The default `<pre class="mermaid">` path.
- Terminal rendering: settled by `2026-10-21-merman`, where terminal users
  select Inter with `MERMAID_FONT=Inter` (D5).
- Embedding any face other than the bundled Inter, including fallback faces
  for scripts Inter does not cover (CJK, emoji). Host faces are typically
  proprietary, and redistributing them is what this opt-in must not do.
- A frontmatter key for the opt-in (D3).
- WOFF2 encoding (D2).

## Key constraint: licensing

System fonts such as Trebuchet MS, Helvetica, and Arial are typically
proprietary, and embedding them in a published page redistributes them.
Therefore the opt-in measures with and embeds only the bundled Inter, which is
licensed under the SIL Open Font License 1.1. The OFL allows redistribution
and modification, subsetting included, provided the copyright notice and
license travel with the font (D8). Turning the opt-in on changes the measured
face, not only the output (D6).

## Owners

| Area | Responsibility |
| --- | --- |
| `biscuit-visualized` | Measuring with Inter for the opt-in render; subsetting Inter to the code points used; building the `@font-face` rule |
| `darkmatter` | The `HtmlOptions` field and the `md` flag; collecting the code points of every diagram on a page; emitting the rule once per page; keeping the sanitizer's rule against external URLs |

## Decisions

### D1. The font: the bundled Inter *(resolves Q1)*

The embedded face is the bundled Inter of `2026-10-21-merman` (D40 there),
regular and bold, licensed OFL 1.1, which allows redistribution. Each weight a
page uses is subset and embedded separately; a weight no label uses is not
embedded.

### D2. Subsetting and encoding *(resolves Q2; amended 2026-10-22 by D10)*

> **Amended by D10:** subsetting uses `allsorts`, not `subsetter`, keeping
> the `cmap` and `OS/2` tables a browser needs. D11 fixes what happens if the
> subset does not load in a browser.

- Subsetting uses the `allsorts` crate (D10).
- The subset is embedded as a **TrueType** data URI, `font/ttf` with
  `format("truetype")`, not WOFF2.
- **Budget:** at most 60 KB per weight per page, measured as the subset's
  TrueType bytes before base64 encoding. A subset above the budget is still
  embedded, and its size is logged.

### D3. How the opt-in is exposed *(resolves Q3)*

- A field on Darkmatter's `HtmlOptions`: `embed_mermaid_font: bool`, default
  `false`.
- An `md` CLI flag, `--embed-mermaid-font`, that sets it.
- No frontmatter key.
- The opt-in acts only on diagrams emitted as static SVG
  (`MermaidMode::Image`); with the default `<pre class="mermaid">` path it
  changes nothing.

### D4. Themes: no interaction *(resolves Q4)*

The embedded face does not interact with light and dark themes. One subset per
page serves both. The `@font-face` rule is emitted **once per page** and
shared by every diagram on it, so the subset covers the union of the code
points used by all the page's diagrams.

### D5. Terminal rendering *(resolves Q5)*

Settled by `2026-10-21-merman`: terminal users choose the same face with
`MERMAID_FONT=Inter`. This spec adds nothing for terminals.

### D6. The opt-in measures with Inter

When the opt-in is on, the render behaves as if `MERMAID_FONT=Inter` were set,
for that render only, so the measured face is the embedded face. Through the
"HTML static" target of `2026-10-21-merman` (D60 there), each label's
`font-family` therefore starts with `Inter`, which the page's `@font-face`
rule defines.

### D7. Failure behavior

If subsetting fails, the diagram is emitted as static SVG without an embedded
font (the behavior of `2026-10-21-merman` D37), and a warning is logged naming
the cause. The page still renders; only the width guarantee is lost.

### D8. The license notice travels with the subset

- The subset keeps Inter's `name` table, which carries the copyright and
  license entries (name IDs 0, 13, and 14); OFL 1.1 accepts machine-readable
  metadata inside the font as the way the notice travels. The `allsorts`
  subset keeps `name` along with `cmap` and `OS/2` (D10); the first task's
  check (D11) confirms it.
- The implementation confirms that the bundled `OFL.txt` declares no Reserved
  Font Name. If it declares one, the embedded subset is declared under the
  family name `bv-mermaid` instead of `Inter`, and labels lead with that
  name.

### D9. Cache identity

Output with the embedded font is a distinct output format in the artifact
cache key of `2026-10-21-merman` (D41 there), so a cached static SVG without
the font is never served for an opt-in render, or the reverse.

### D10. Subsetting with `allsorts` *(resolves Q6; amends D2)*

- Subsetting uses the **`allsorts`** crate (YesLogic; pure Rust;
  Apache-2.0).
- The subset keeps the `cmap` and `OS/2` tables, which a browser's
  `@font-face` needs: `cmap` maps characters to glyphs, and browsers' font
  sanitizers reject a font without `cmap` or `OS/2`. It also keeps `name`
  (D8).
- **Rejected: `subsetter`** (the Typst project's), the crate first ruled for
  D2. It removes `cmap` and `OS/2` with no option to keep them, and its own
  documentation calls its output "most likely unusable in any other
  contexts than PDF writing".

### D11. Browser check first, with a fixed fallback

- The **first implementation task** checks, in headless Chrome, that the
  `allsorts` subset loads as an `@font-face` and that every glyph the test
  page uses renders in Inter (not in a fallback face and not as a missing
  glyph).
- If the check fails, the implementation embeds **Inter unsubset** instead,
  one file per weight used, with the same `@font-face` shape, and logs the
  outcome and the embedded sizes in the implementation log. The 60 KB budget
  rule of D2 (embed and log) applies to the unsubset files. No further ruling
  is needed in either case.

### D12. Where the sanitizer admits a `data:` URL

Darkmatter's sanitizer admits `url(data:…)` **only** inside the `src`
descriptor of an `@font-face` rule, and only as a `data:font/ttf` URI. A
`data:` URL anywhere else (an attribute such as `fill` or `filter`, a
`style` attribute, another CSS property, or another at-rule) is rejected, as
today. The block on external `url()` is unchanged everywhere (R-E6).

## Requirements

- **R-E1.** `HtmlOptions::embed_mermaid_font` (default `false`) and the `md`
  flag `--embed-mermaid-font` turn the opt-in on; there is no frontmatter key
  (D3).
- **R-E2.** With the opt-in on, every diagram on the page is measured with
  Inter, as if `MERMAID_FONT=Inter` were set for that render (D6).
- **R-E3.** The page carries exactly one `@font-face` rule per weight used,
  shared by all its diagrams, with a TrueType data URI (`font/ttf`) whose
  subset maps every code point of the page's labels that Inter maps (D2, D4).
- **R-E4.** A subset over 60 KB per weight is still embedded, and its size is
  logged (D2).
- **R-E5.** If subsetting fails, the diagrams are emitted as static SVG without
  an embedded font, and a warning is logged (D7).
- **R-E6.** Darkmatter's sanitizer still rejects an external `url()`, inside
  `@font-face` and everywhere else. It admits `url(data:…)` **only** inside
  an `@font-face` rule's `src` descriptor, and only as a `data:font/ttf` URI;
  a `data:` URL anywhere else (`fill`, `style`, `filter`, other CSS
  properties, other at-rules) is still rejected (D12).
- **R-E11.** Subsetting uses `allsorts`, and the subset keeps the `cmap`,
  `OS/2`, and `name` tables (D10).
- **R-E12.** The first implementation task verifies, in headless Chrome, that
  the subset loads as an `@font-face` and every used glyph renders; if it
  does not, Inter is embedded unsubset, one file per weight used, and the
  outcome is logged (D11).
- **R-E7.** The embedded subset carries Inter's copyright and license notice
  in its `name` table (D8).
- **R-E8.** Code points Inter does not map are not embedded from any other
  face; they are drawn by the reader's fallback.
- **R-E9.** An opt-in render and a plain static-SVG render of the same diagram
  have different cache keys (D9).
- **R-E10.** The opt-in, its flag, and its limits (Inter's coverage, the
  budget, the failure fallback) are documented in Darkmatter's and
  `biscuit-visualized`'s `docs/`, for a reader new to both packages; the
  `allsorts` dependency is recorded in `biscuit-visualized/docs/dependencies.md`
  and the root `docs/dependencies.md`.

## Acceptance criteria

| ID | Statement | Validated by |
| --- | --- | --- |
| AC-E1 | A page with the opt-in renders identical label widths on two hosts with different installed fonts, for labels Inter covers | Render the same page in a headless browser on two hosts (for example macOS and Linux); compare every label's bounding box; widths equal within 0.5 px |
| AC-E2 | Darkmatter's sanitizer rejects an external `url()` in `@font-face` and elsewhere, admits a `data:font/ttf` URL inside `@font-face` `src`, and rejects a `data:` URL anywhere else | Sanitizer tests: positive (data URI in `@font-face` `src` kept) and negative (`https:` and protocol-relative URLs in `@font-face`, `fill`, and `style` removed; a `data:` URL in `fill`, in `style`, in `filter`, and in a non-`src` CSS property removed) |
| AC-E3 | The embedded subset maps every code point of the page's labels that Inter maps | Test comparing the label text's code points with the subset's `cmap`, on a page with two diagrams whose labels share and differ in characters |
| AC-E4 | A page with several diagrams, in light and dark themes, has exactly one `@font-face` per weight used | HTML test counting `@font-face` rules on a multi-diagram page in each theme |
| AC-E5 | With the opt-in on, the measured face is Inter whatever `MERMAID_FONT` and the host fonts are | Test with `MERMAID_FONT` set to another installed family; every label's measured face is Inter |
| AC-E6 | A subset over 60 KB is embedded and its size is logged | Test with a label set large enough to exceed the budget; captured log |
| AC-E7 | When subsetting fails, the output is the static SVG without a font, and a warning is logged | Test with an injected subsetting failure; output has no `@font-face`; captured log |
| AC-E8 | The subset's `name` table carries name IDs 0, 13, and 14 from Inter; the bundled `OFL.txt`'s Reserved Font Name status is recorded in the implementation log, and the declared family matches D8 | Test reading the subset's `name` table; log entry; HTML assertion on the family name |
| AC-E9 | `HtmlOptions::embed_mermaid_font` defaults to `false`; `--embed-mermaid-font` sets it; without static-SVG mode the output is unchanged | Unit test on the default; CLI test; HTML diff with and without the flag in the default mode |
| AC-E10 | An opt-in render and a plain static-SVG render of the same source have different cache keys | Cache test |
| AC-E11 | The docs describe the opt-in, the flag, Inter's coverage, the budget, and the failure fallback, with an example, and name no spec | Review; search for spec names in the touched docs |
| AC-E12 | The subset is produced by `allsorts` and keeps `cmap`, `OS/2`, and `name`; the headless Chrome check of D11 is recorded in the implementation log with its outcome (subset used, or unsubset fallback taken) | Test reading the subset's table directory; log entry with the Chrome result and, on fallback, the embedded file sizes |

## Example markup

```html
<style>@font-face{font-family:"Inter";src:url(data:font/ttf;base64,…) format("truetype")}</style>
<text font-family="Inter, &quot;trebuchet ms&quot;, verdana, arial, sans-serif">Animal</text>
```

## Resolved questions

Resolved questions: Q1→D1, Q2→D2, Q3→D3, Q4→D4, Q5→D5, Q6→D10 and D11; the
draft's unasked items (scripts outside Inter, cache identity) → R-E8 and D9;
the `data:` URL boundary in the sanitizer → D12.
