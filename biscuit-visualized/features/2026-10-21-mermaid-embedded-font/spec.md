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
    - 2026-10-22-merman-text-fallback
related:
    - 2026-10-21-merman
    - 2026-10-22-merman-text-fallback
---

# Embedded font for Mermaid static SVG

An **opt-in** for Darkmatter's static-SVG HTML output that embeds the face the
diagrams were measured with into the page, so diagrams render with the same
label widths on every reader's machine. The face is the one the normal font
chain of `2026-10-21-merman` (D32 there) resolves, when that face's own
embedding permission allows it; otherwise it is the bundled Inter (D40 there).

Ken made the rulings below on 2026-10-22, including D10, which settles how
the subset is made (Q6). D13–D16, also Ken's of 2026-10-22, amend the font
choice (D1, D6), the license notice (D8), and the first task's check (D11).

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

- An `HtmlOptions` field and an `md` CLI feature, `--feature embed-mermaid-font`, that turn the opt-in on (D3).
- Choosing the face to measure with and embed: the face the font chain
  resolves when its embedding permission allows it, the bundled Inter
  otherwise (D13).
- Subsetting the chosen face to the code points a page uses with `allsorts`,
  and embedding the subset once per page as a TrueType data URI (D2, D4,
  D10, D16).
- A first-task browser check of the subset, including its `name` table where
  the license requires it, with a fixed fallback to the unsubset face (D11,
  D16).
- A page comment naming the embedded font and why (D14).
- The failure fallback (D7) and the license notice (D8, D15).

**Out of scope**

- The default `<pre class="mermaid">` path.
- Terminal rendering: settled by `2026-10-21-merman`, where terminal users
  select Inter with `MERMAID_FONT=Inter` (D5).
- Embedding fallback faces for code points the chosen face does not map
  (CJK, emoji, and the Linux-only bundled Noto Emoji of `2026-10-21-merman`
  D65); those are drawn by the reader's own fallback (R-E8).
- A frontmatter key for the opt-in (D3).
- WOFF2 encoding (D2).

## Key constraint: licensing

Embedding a font in a published page **redistributes** it: every reader
downloads a copy. Common OS fonts, such as Helvetica, Trebuchet MS, Arial, and
Segoe UI, are usually proprietary, licensed for use on the machine they were
installed on, not for redistribution.

Every OpenType or TrueType font carries its own declaration of whether it may
be embedded: the **`fsType`** field of its `OS/2` table, set by the font's
vendor. In plain terms it says one of:

| `fsType` permission | Plain meaning | This opt-in |
| --- | --- | --- |
| Installable (`0x0000`) | May be embedded, and installed by the reader | Embeds it |
| Editable (`0x0008`) | May be embedded and used to edit the document | Embeds it |
| Preview & print (`0x0004`) | May be embedded for viewing and printing only | Embeds it |
| Restricted (`0x0002`) | Must not be embedded | Uses Inter instead |

The opt-in embeds a face only when its `fsType` permits it (D13); otherwise
it measures with and embeds the bundled Inter, which is licensed under the SIL
Open Font License 1.1. The OFL allows redistribution and modification,
subsetting included, provided the copyright notice and license travel with
the font (D8, D15). The face embedded is always the face measured, so
choosing a different face changes the layout, not only the output (D6, D13).

## Owners

| Area | Responsibility |
| --- | --- |
| `biscuit-visualized` | Choosing the face for the opt-in render (reading the resolved face's `fsType`, D13); measuring with it; subsetting it to the code points used; building the `@font-face` rule; the explanation for the page comment (D14) |
| `darkmatter` | The `HtmlOptions` field and the `md` feature `embed-mermaid-font`; collecting the code points of every diagram on a page; emitting the rule, the page comment, and the OFL attribution once per page; keeping the sanitizer's rule against external URLs |

## Decisions

### D1. The font: the resolved face, or the bundled Inter *(resolves Q1; amended 2026-10-22 by D13)*

> **Amended by D13:** the embedded face is no longer always Inter. It is the
> face the normal font chain resolves when that face's `fsType` permits
> embedding, and the bundled Inter otherwise or when `MERMAID_FONT=Inter`
> selects it. The per-weight rule below applies to whichever face is
> embedded.

The embedded face, when it is Inter, is the bundled Inter of
`2026-10-21-merman` (D40 there), regular and bold, licensed OFL 1.1, which
allows redistribution. Each weight a page uses is subset and embedded
separately; a weight no label uses is not embedded.

### D2. Subsetting and encoding *(resolves Q2; amended 2026-10-22 by D10)*

> **Amended by D10:** subsetting uses `allsorts`, not `subsetter`, keeping
> the `cmap` and `OS/2` tables a browser needs. D11 fixes what happens if the
> subset does not load in a browser. D16 applies these rules to whichever
> face is embedded.

- Subsetting uses the `allsorts` crate (D10).
- The subset is embedded as a **TrueType** data URI, `font/ttf` with
  `format("truetype")`, not WOFF2.
- **Budget:** at most 60 KB per weight per page, measured as the subset's
  TrueType bytes before base64 encoding. A subset above the budget is still
  embedded, and its size is logged.

### D3. How the opt-in is exposed *(resolves Q3)*

- A field on Darkmatter's `HtmlOptions`: `embed_mermaid_font: bool`, default
  `false`.
- An `md` CLI feature, `--feature embed-mermaid-font`, that sets it. It uses the
  `--feature` convention introduced by `2026-10-22-merman-text-fallback` (D4
  there), which is implemented first: repeatable, comma-separated,
  case-insensitive, and an unknown name is a usage error. *(Amended
  2026-10-22: originally a standalone `--embed-mermaid-font` flag; Ken moved it
  under the `--feature` convention and ordered this spec after the text
  fallback.)*
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

### D6. The opt-in measures with the face it embeds *(amended 2026-10-22 by D13; amended 2026-10-24 during implementation under D17)*

> **Amended during implementation (2026-10-24, D17):** the first task's
> browser check found that the `allsorts` subset carries no `GPOS` or `GSUB`
> table, so the browser draws the subset without kerning, ligatures, or
> contextual alternates, about 1.3% wider than a kerned measurement. With the
> opt-in on, labels are therefore measured with the embedded face's plain
> glyph advances (no OpenType layout features), which is exactly what the
> subset draws. The implementation log records the evidence; flagged for
> review.

> **Amended by D13:** the opt-in no longer forces Inter. It measures with the
> face D13 chooses: the resolved face when its `fsType` permits embedding,
> and Inter, as if `MERMAID_FONT=Inter` were set for that render, otherwise.

The measured face is always the embedded face. Through the "HTML static"
target of `2026-10-21-merman` (D60 there), each label's `font-family`
therefore starts with the embedded face's family name, which the page's
`@font-face` rule defines.

### D7. Failure behavior

If subsetting fails, the diagram is emitted as static SVG without an embedded
font (the behavior of `2026-10-21-merman` D37), and a warning is logged naming
the cause. The page still renders; only the width guarantee is lost.

### D8. The license notice travels with the subset *(amended 2026-10-22 by D15 and D16)*

> **Amended:** these rules apply when Inter is the embedded face. D15 adds a
> one-line OFL attribution comment beside Inter's `@font-face`; D16 makes the
> first task verify that the subset keeps the `name` entries below.

- The subset keeps Inter's `name` table, which carries the copyright and
  license entries (name IDs 0, 13, and 14); OFL 1.1 accepts machine-readable
  metadata inside the font as the way the notice travels. The `allsorts`
  subset keeps `name` along with `cmap` and `OS/2` (D10); the first task's
  check (D11, D16) confirms it.
- The implementation confirms that the bundled `OFL.txt` declares no Reserved
  Font Name. If it declares one, the embedded subset is declared under the
  family name `bv-mermaid` instead of `Inter`, and labels lead with that
  name.

### D9. Cache identity

Output with the embedded font is a distinct output format in the artifact
cache key of `2026-10-21-merman` (D41 there), so a cached static SVG without
the font is never served for an opt-in render, or the reverse. The face
chosen by D13 is covered by that key's resolved-face identity (D63 there):
the `fsType` decision depends only on the resolved face's contents, which the
key already carries, and an Inter substitution changes the measured face.

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

### D11. Browser check first, with a fixed fallback *(amended 2026-10-22 by D16)*

> **Amended by D16:** the same first task also verifies that the subset keeps
> the `name` table's copyright and license entries whenever the license
> requires it (OFL, so Inter); a failure there takes the same fallback.

- The **first implementation task** checks, in headless Chrome, that the
  `allsorts` subset loads as an `@font-face` and that every glyph the test
  page uses renders in the embedded face (not in a fallback face and not as
  a missing glyph).
- If the check fails, the implementation embeds the face **unsubset**
  instead, one file per weight used, with the same `@font-face` shape, and
  logs the outcome and the embedded sizes in the implementation log. The
  60 KB budget rule of D2 (embed and log) applies to the unsubset files. No
  further ruling is needed in either case.

### D12. Where the sanitizer admits a `data:` URL

Darkmatter's sanitizer admits `url(data:…)` **only** inside the `src`
descriptor of an `@font-face` rule, and only as a `data:font/ttf` URI. A
`data:` URL anywhere else (an attribute such as `fill` or `filter`, a
`style` attribute, another CSS property, or another at-rule) is rejected, as
today. The block on external `url()` is unchanged everywhere (R-E6).

### D13. Choosing the face: `fsType` decides, Inter is the fallback *(amends D1 and D6)*

- **By default**, the opt-in measures with and embeds the face the normal
  font chain of `2026-10-21-merman` (D32 there) resolves for each requested
  family: `MERMAID_FONT`, then Mermaid's CSS stack, then the OS generic
  family, then the OS/2 sans-serif rule (D50 there). This is the face the
  chain resolves for the requested family, not a fallback position used for
  code points that face does not map (R-E8).
- That face is embedded **only if** its `OS/2` `fsType` permits embedding:
  Installable (`0x0000`), Editable (`0x0008`), or Preview & print
  (`0x0004`). When more than one of these permission bits is set (older
  fonts), the least restrictive applies, as the OpenType specification
  directs.
- **Inter instead:** if the face is Restricted (`0x0002` as its only
  permission bit), or the chain reaches no installed face, the opt-in
  measures with and embeds the bundled Inter, as if `MERMAID_FONT=Inter`
  were set for that render. A face with no readable `OS/2` table, or whose
  `fsType` sets "bitmap embedding only" (`0x0200`), is treated as Restricted,
  because this opt-in embeds outlines.
- A face whose `fsType` sets "no subsetting" (`0x0100`) and otherwise permits
  embedding is embedded **unsubset** (the D11 fallback's shape).
- The decision is per requested family: if any weight the page uses resolves
  to a face that may not be embedded, the whole family is measured and
  embedded as Inter, so one family never mixes two faces' metrics.
- `MERMAID_FONT=Inter` selects Inter explicitly (step 1 of D32 there matches
  the bundled Inter by name).

### D14. A page comment says which font was embedded, and why

- For each embedded family, the page carries one HTML comment immediately
  before the `<style>` element holding the `@font-face` rules that names the embedded face and the reason, which is
  one of:
    - the face the font chain resolved, with its `fsType` permission;
    - Inter, selected by `MERMAID_FONT=Inter`;
    - Inter, because the resolved face (named) is Restricted or otherwise may
      not be embedded (with the reason: Restricted, bitmap only, or no
      readable `OS/2` table);
    - Inter, because the chain reached no installed face.
- It also says when the face was embedded unsubset, and why (the D11
  fallback or the "no subsetting" bit).
- Font names written into the comment have `--` and `>` removed, so a name
  cannot end the comment early.
- For example:

  ```html
  <!-- mermaid font: "Inter" embedded; resolved face "Arial" is Restricted (fsType 0x0002) -->
  ```

### D15. OFL attribution only when Inter is embedded *(amends D8)*

When, and only when, Inter is embedded, the page carries a one-line OFL
attribution comment beside Inter's `@font-face`: a CSS comment inside the
same `<style>` element, immediately before the rule, giving Inter's copyright
line from the bundled `OFL.txt` and naming the SIL Open Font License 1.1. A
page that embeds a host face carries no OFL comment.

### D16. The subset check covers the `name` table, for whichever face *(amends D2 and D11)*

- The headless-Chrome first task (D11) also verifies that the subset keeps
  the `name` table, including its copyright and license entries (name IDs 0,
  13, and 14), whenever the license requires it: OFL, so Inter. If it does
  not, the existing fixed fallback applies: embed the face unsubset.
- The subsetting rules (`allsorts`, the kept tables of D10, the budget of D2,
  the fallback of D11) apply to whichever face is embedded, Inter or a host
  face.

### D17. Bounded implementer autonomy *(pre-authorization)*

Ken ruled on 2026-10-22 that the rule `2026-10-21-merman` records as its
D72 applies to this work too. When implementation evidence invalidates a
decision's premise, the implementer may proceed with an option that meets
**all three** of these conditions:

1. it preserves the decision's stated purpose;
2. it stays within every other decision and limit of this spec and of the specs it depends on;
3. it adds no new dependency, no outward-facing action (pushing, posting,
   filing), and no scope.

The implementer then amends this spec (marking the decision amended),
records the evidence and the choice under a **"Decided during
implementation"** heading in the implementation log, and flags the choice
for the review cycle. If no option meets all three conditions, or the
options trade off things Ken would weigh, the implementer stops and sets
`human_review` in the implementation log.

## Requirements

- **R-E1.** `HtmlOptions::embed_mermaid_font` (default `false`) and the `md`
  feature `--feature embed-mermaid-font` turn the opt-in on; there is no frontmatter key
  (D3).
- **R-E2.** With the opt-in on, every diagram on the page is measured with the
  face D13 chooses, which is the face embedded (D6, D13).
- **R-E3.** The page carries exactly one `@font-face` rule per embedded face
  and weight used, shared by all its diagrams, with a TrueType data URI
  (`font/ttf`) whose subset maps every code point of the page's labels that
  the face maps (D2, D4).
- **R-E4.** A subset over 60 KB per weight is still embedded, and its size is
  logged (D2).
- **R-E5.** If subsetting fails, the diagrams are emitted as static SVG without
  an embedded font, and a warning is logged (D7).
- **R-E6.** Darkmatter's sanitizer still rejects an external `url()`, inside
  `@font-face` and everywhere else. It admits `url(data:…)` **only** inside
  an `@font-face` rule's `src` descriptor, and only as a `data:font/ttf` URI;
  a `data:` URL anywhere else (`fill`, `style`, `filter`, other CSS
  properties, other at-rules) is still rejected (D12).
- **R-E7.** When Inter is embedded, the subset carries Inter's copyright and
  license notice in its `name` table (D8).
- **R-E8.** Code points the embedded face does not map are not embedded from
  any other face; they are drawn by the reader's fallback.
- **R-E9.** An opt-in render and a plain static-SVG render of the same diagram
  have different cache keys (D9).
- **R-E10.** The opt-in, its flag, the face choice and `fsType` rule, and its
  limits (the embedded face's coverage, the budget, the failure fallback) are
  documented in Darkmatter's and `biscuit-visualized`'s `docs/`, for a reader
  new to both packages, including a plain explanation of `fsType` and of why
  embedding redistributes a font; the `allsorts` dependency is recorded in
  `biscuit-visualized/docs/dependencies.md` and the root
  `docs/dependencies.md`.
- **R-E11.** Subsetting uses `allsorts`, and the subset keeps the `cmap`,
  `OS/2`, and `name` tables (D10).
- **R-E12.** The first implementation task verifies, in headless Chrome, that
  the subset loads as an `@font-face`, every used glyph renders, and, for
  Inter, the `name` table keeps name IDs 0, 13, and 14; if any check fails,
  the face is embedded unsubset, one file per weight used, and the outcome is
  logged (D11, D16).
- **R-E13.** The resolved face is embedded only when its `fsType` is
  Installable, Editable, or Preview & print; Restricted, bitmap-only, a
  missing `OS/2` table, or no installed face selects Inter; "no subsetting"
  embeds the face unsubset; the decision is per requested family (D13).
- **R-E14.** The page carries one comment per embedded family naming the face
  and the reason, with `--` and `>` removed from names (D14).
- **R-E15.** The page carries the one-line OFL attribution comment when, and
  only when, Inter is embedded (D15).

## Acceptance criteria

| ID | Statement | Validated by |
| --- | --- | --- |
| AC-E1 | A page with the opt-in renders identical label widths on two reader hosts with different installed fonts, for labels the embedded face covers | Render the same page in a headless browser on two hosts (for example macOS and Linux); compare every label's bounding box; widths equal within 0.5 px |
| AC-E2 | Darkmatter's sanitizer rejects an external `url()` in `@font-face` and elsewhere, admits a `data:font/ttf` URL inside `@font-face` `src`, and rejects a `data:` URL anywhere else | Sanitizer tests: positive (data URI in `@font-face` `src` kept) and negative (`https:` and protocol-relative URLs in `@font-face`, `fill`, and `style` removed; a `data:` URL in `fill`, in `style`, in `filter`, and in a non-`src` CSS property removed) |
| AC-E3 | The embedded subset maps every code point of the page's labels that the embedded face maps | Test comparing the label text's code points with the subset's `cmap`, on a page with two diagrams whose labels share and differ in characters |
| AC-E4 | A page with several diagrams, in light and dark themes, has exactly one `@font-face` per embedded face and weight used | HTML test counting `@font-face` rules on a multi-diagram page in each theme |
| AC-E5 | With the opt-in on, the measured face is the embedded face: a permitted resolved face is measured and embedded; a Restricted one, and an empty chain, give Inter; `MERMAID_FONT=Inter` gives Inter | Tests with an injected font set: one face per `fsType` case; `MERMAID_FONT` set to a permitted installed family, to a Restricted one, and to `Inter`; an injected set with no installed face; each asserts the measured and the embedded face |
| AC-E6 | A subset over 60 KB is embedded and its size is logged | Test with a label set large enough to exceed the budget; captured log |
| AC-E7 | When subsetting fails, the output is the static SVG without a font, and a warning is logged | Test with an injected subsetting failure; output has no `@font-face`; captured log |
| AC-E8 | When Inter is embedded, the subset's `name` table carries name IDs 0, 13, and 14 from Inter; the bundled `OFL.txt`'s Reserved Font Name status is recorded in the implementation log, and the declared family matches D8 | Test reading the subset's `name` table; log entry; HTML assertion on the family name |
| AC-E9 | `HtmlOptions::embed_mermaid_font` defaults to `false`; `--feature embed-mermaid-font` sets it; without static-SVG mode the output is unchanged | Unit test on the default; CLI test; HTML diff with and without the flag in the default mode |
| AC-E10 | An opt-in render and a plain static-SVG render of the same source have different cache keys | Cache test |
| AC-E11 | The docs describe the opt-in, the flag, the face choice with a plain explanation of `fsType` and of redistribution, the embedded face's coverage, the budget, and the failure fallback, with an example, and name no spec | Review; search for spec names in the touched docs |
| AC-E12 | The subset is produced by `allsorts` and keeps `cmap`, `OS/2`, and `name`; the headless Chrome check of D11 and D16 (glyphs, and Inter's name IDs 0, 13, and 14) is recorded in the implementation log with its outcome (subset used, or unsubset fallback taken) | Test reading the subset's table directory; log entry with the Chrome result and, on fallback, the embedded file sizes |
| AC-E13 | `fsType` handling: Installable, Editable, and Preview & print embed the face; Restricted, bitmap-only, and a missing `OS/2` table give Inter; a font with several permission bits uses the least restrictive; "no subsetting" embeds the face unsubset; a family whose bold resolves to a Restricted face while its regular is permitted is embedded entirely as Inter | Unit tests over fonts (or `OS/2` tables) built with each `fsType` value |
| AC-E14 | The page comment names the embedded face and the reason for each case of D14, including unsubset embedding, and a font name containing `--` or `>` cannot end the comment | HTML assertions per case; a test with a crafted family name |
| AC-E15 | A page embedding Inter has the one-line OFL attribution comment immediately before Inter's `@font-face`; a page embedding only a host face has none | HTML assertions on both pages |

## Example markup

A page whose diagrams resolved to a Restricted host face:

```html
<!-- mermaid font: "Inter" embedded; resolved face "Arial" is Restricted (fsType 0x0002) -->
<style>
/* Inter: Copyright (c) 2016 The Inter Project Authors; SIL Open Font License 1.1 */
@font-face{font-family:"Inter";src:url(data:font/ttf;base64,…) format("truetype")}
</style>
<text font-family="Inter, &quot;trebuchet ms&quot;, verdana, arial, sans-serif">Animal</text>
```

The HTML comment sits immediately before the `<style>` element (inside
`<style>`, `<!--` is not a comment). The copyright line comes from the
bundled `OFL.txt`; the wording above is illustrative.

## Risks

- **`fsType` is the vendor's declaration, not a license.** It is the rule this
  opt-in honors, and it is what font tools and browsers read, but a font's
  license text can be stricter than its `fsType`. The page comment (D14)
  names the embedded face so a publisher can check it.
- **Host faces can be large.** A face embedded unsubset ("no subsetting", or
  the D11 fallback) can far exceed the 60 KB budget; D2's rule is to embed and
  log, so the size is visible but not prevented.

## Resolved questions

Resolved questions: Q1→D1 (amended by D13), Q2→D2, Q3→D3, Q4→D4, Q5→D5,
Q6→D10 and D11; the draft's unasked items (scripts outside the embedded face,
cache identity) → R-E8 and D9; the `data:` URL boundary in the sanitizer →
D12. Ken's amendments of 2026-10-22 (the face choice, the page comment, the
OFL attribution, and the `name` table check) → D13–D16.
