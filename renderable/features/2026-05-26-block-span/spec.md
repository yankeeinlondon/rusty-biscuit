---
status: draft
---

# Block Span Extensions: Line-Anchored Block-Prefix Scanner

## Status

**Draft — brainstorming.** This spec captures a candidate architecture for
darkmatter's block-level syntax extensions (HR attributes today, future
admonitions / containers / heading attrs / fenced extensions). It is not
an implementation contract. Open design questions are surfaced explicitly
so the team can push on them before committing.

The sibling spec
[`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md)
covers inline-level extensions (mark, dim, emoji, var, math, tooltips).
The two were originally muddled together inside one span-aware processor
for historical reasons; this spec pair separates them.

Performance context lives in
[`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md).

## Background

Pulldown-cmark recognizes CommonMark plus GFM block constructs:
paragraphs, headings (ATX + setext), lists, block quotes, code fences,
HTML blocks, link reference definitions, and thematic breaks. Anything
darkmatter wants to extend at the block level falls outside that set.

### The HR-attribute case

The only block-level extension darkmatter ships today is HR attributes:

```
--- { style: waves, weight: thick }
```

Plain `---` is a CommonMark thematic break — pulldown-cmark emits
`Event::Rule` for it natively, no extension machinery required. But the
moment trailing content appears on the line, the construct **stops being
a CommonMark HR**: CommonMark's HR rule requires the line to contain only
rule characters and whitespace. Pulldown-cmark treats
`--- { style: waves }` as an ordinary paragraph containing literal text.

Today's solution (the same `SpannedInlineStyleProcessor` that handles
inline extensions) post-processes the event stream: it watches for
paragraphs whose text matches `--- { ... }`, drops the paragraph events,
synthesizes an `Event::Rule` with the parsed attributes carried as
hints (`HorizontalRuleAttrs`).

**This is a block-level concern living in an inline-level processor**
because both were "post-process pulldown-cmark's event stream for
darkmatter extensions" and shared infrastructure. The architectural
coupling is convenience, not design.

### Future block-level extensions (hypothetical)

Likely future block-level extensions, none currently shipped:

- **Heading attributes**: `# Heading { id: foo, class: bar }` — attach
  attributes to a native-recognized block.
- **Admonitions / callouts**: `::: warning` / `::: tip` paired containers.
- **Custom fenced containers**: `:::component\nprops\n:::` — generic
  shape for invoking custom components in Markdown.
- **Front-of-block metadata**: `{ class: highlight }\n# Heading` style
  prefix blocks.
- **Paragraph attributes**: `{ .lead }\nA leading paragraph.`

The shapes are heterogeneous: some are line-prefix (`---`), some are
fenced (`:::warning ... :::`), some attach attributes to a native
construct (`{ id: foo }` decorating a heading). They share the property
of being block-level (line-anchored, separated from surrounding content
by blank lines or block boundaries) and unrecognized by pulldown-cmark.

## Proposed Architecture

A **dedicated block-prefix scanner** that runs at the source-text layer,
parallel to (and independent from) the inline rewriter in the sibling
spec.

### Pipeline shape

```
                    ┌─────────────────────────┐
                    │  raw source             │
                    └─────────────────────────┘
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
  ┌────────────────────┐         ┌──────────────────────┐
  │ inline rewriter    │         │ block-prefix scanner │
  │ (sibling spec)     │         │ (this spec)          │
  └────────────────────┘         └──────────────────────┘
            │                                 │
            └────────────────┬────────────────┘
                             ▼
                    ┌────────────────────────┐
                    │ pulldown-cmark parser  │
                    └────────────────────────┘
                             │
                             ▼
                    ┌────────────────────────┐
                    │  block-extension fold  │
                    │   post-processor       │
                    └────────────────────────┘
                             │
                             ▼
                    ┌────────────────────────┐
                    │  render-tree Document  │
                    └────────────────────────┘
```

Where each block extension does its work depends on its shape (see
[Per-extension strategy](#per-extension-strategy) below) — but the two
common patterns are:

1. **Source-layer normalization**: the scanner rewrites a block extension
   to a form pulldown-cmark recognizes natively, with a sentinel marker
   that the fold post-processor can pick up to attach the extension's
   semantics.
2. **Event-stream post-processing**: pulldown-cmark sees the literal
   text; a dedicated post-processor watches the event stream for the
   block-extension pattern at *block boundaries only* (not inside text
   events) and synthesizes the right events.

The current HR-attribute handling is approach (2) but tangled with the
inline processor. A cleaner shape: approach (2) but scoped to block
boundaries, in a small dedicated processor.

### Per-extension strategy

Different block extensions suit different strategies. The spec doesn't
prescribe one for all; it gives a decision rule.

| Shape | Example | Strategy | Rationale |
|-------|---------|----------|-----------|
| Line-prefix with trailing attrs | `--- { ... }` | Event-stream post-process | Pulldown-cmark already emits a paragraph; pick it off at the block boundary, parse attrs, synthesize Rule + hints. |
| Attribute decorator on native block | `# H { id: foo }` | Source rewrite + sentinel | Strip `{ id: foo }` from heading line; emit sentinel comment; pulldown-cmark parses heading normally; post-processor consumes sentinel and attaches attrs. |
| Fenced container | `::: warning\n...\n:::` | Source rewrite to HTML block | Rewrite `:::warning` and closing `:::` to `<darkmatter-admonition class="warning">` and `</darkmatter-admonition>`; pulldown-cmark emits HTML block events; post-processor synthesizes typed tree nodes. |
| Front-of-block metadata | `{ .lead }\n# H` | Source rewrite + sentinel | Same as attribute decorator but attaches to the next block instead of modifying the current line. |

The unifying principle: **the scanner runs once over the source, line-
anchored, and either rewrites in place or emits a sentinel for the fold
to pick up.** No per-event walking of text content (that's the inline
spec's job).

### Why not push everything through the inline rewriter?

Because block constructs are line-anchored, not text-event-anchored. The
inline rewriter scans text events for paired delimiters. The block
scanner scans source lines (separated by `\n`, respecting fenced-code
boundaries) for block-level patterns. The two are structurally different
scans:

- Inline: byte walks within text events, opener-stack across events.
- Block: line iteration over source, with fenced-block boundary tracking
  so we don't false-positive inside code fences.

Trying to do both in one pass conflates the abstraction. Keeping them in
sibling specs lets each pick the right primitive.

## Goals

- Separate block-level darkmatter extensions from the inline span-aware
  processor.
- Provide a clear architectural slot for future block extensions
  (admonitions, container syntax, attribute decorators) without
  re-entangling the inline path.
- Preserve byte-range provenance for diagnostics on block extensions.
- Keep multi-target lowering intact: block extensions produce typed tree
  nodes that each target lowers in its own vocabulary.
- Cheap on documents without block extensions (the common case): the
  scanner exits quickly when no block-prefix patterns appear.

## Non-Goals

- Inline darkmatter extensions — owned by
  [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md).
- Frontmatter parsing — already handled separately by darkmatter.
- Graphics policy for block-level rendering (e.g. styled HR images) —
  owned by
  [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
  This spec governs the *parse-side* recognition of HR attributes; the
  *render-side* graphics decision lives in graphics-policy.
- Changes to the renderable tree IR (`NodeKind`).
- Designing every future block extension. Only HR attributes ship today;
  others are listed to make sure the architecture accommodates them
  without re-design.

## Performance Hypothesis

Block-prefix scanning is inherently cheaper than inline scanning because:

- It iterates source by **lines**, not by every text event.
- Line iteration uses `memchr(b'\n', …)` — extremely fast.
- Most lines aren't block extensions; the scanner makes O(1) decisions
  per line and only does real work on candidate lines.
- Block extensions are sparse in real documents — typically a handful
  per document.

**Expected cost on documents without block extensions:** approaches the
cost of one `memchr` pass over the source — negligible (microseconds
per MB).

**Expected cost on documents with HR attributes:** dominated by the
attribute parser (`{ style: waves, weight: thick }`) and the few
candidate lines that need it; not by the scan itself.

**Gating decision:** less profile-sensitive than the inline spec —
block-prefix scanning is cheap enough to be worth doing on architectural
grounds (clean separation from inline) even if it doesn't show up as a
big perf win.

## Open Questions

### Where does the scanner run?

Three options, in increasing invasiveness:

1. **Before pulldown-cmark** — source rewriter, possibly chained with the
   inline rewriter.
2. **After pulldown-cmark** — event-stream post-processor that watches
   for paragraphs / HTML blocks that match block-extension shapes.
3. **Both, depending on extension** — per-extension choice via the
   strategy table above.

Recommendation TBD. (3) is the most flexible and likely correct, but the
spec should commit to a default unless an extension has a clear reason to
deviate.

### How do we handle native-recognized blocks that grow attributes?

`# Heading { id: foo }` — pulldown-cmark sees the whole line as the
heading content (text "Heading { id: foo }"). To extract attributes
without losing the heading's native parsing:

- **Pre-strip**: source rewriter removes ` { id: foo }` from the heading
  line and emits a sentinel comment `<!--dm-attrs:{ id: foo }-->`
  immediately after; pulldown-cmark parses the heading normally; the
  post-processor consumes the sentinel and attaches attrs to the
  preceding heading. **Drawback:** comments inside paragraphs / lists
  may interact strangely.
- **Post-extract**: pulldown-cmark parses the heading with attrs as
  literal text in its content; the post-processor regexes the trailing
  `{ … }` out of the heading text and attaches it as attrs. **Drawback:**
  has to find the attribute block by string scan after parsing, which is
  fragile if the heading content happens to *contain* `{ ... }`
  legitimately.

Neither is clearly better. Pre-strip is more reliable; post-extract is
simpler.

### How do fenced container extensions interact with code fences?

`::: warning` looks like a fence to the human eye, but pulldown-cmark
only recognizes `` ``` `` and `~~~` fences. A `:::` line is just a
paragraph. Approaches:

- **Source rewrite to HTML block**: `:::warning` → `<dm-admonition class="warning">`,
  closing `:::` → `</dm-admonition>`. Pulldown-cmark emits HTML block
  events; the post-processor consumes them. **Pro:** uses native HTML
  block infrastructure; **Con:** content inside is parsed as HTML block,
  not as Markdown — would break nested formatting.
- **Source rewrite to blockquote-like construct**: same idea but pick a
  construct that allows nested Markdown.
- **Direct line-scan post-processing**: the scanner finds `:::tag` and
  matching closing `:::` lines, slices the source into outer + inner
  pieces, recursively parses the inner content as Markdown, wraps in a
  tree node. **Pro:** preserves nested Markdown; **Con:** more
  bookkeeping; needs careful interaction with fenced code blocks (`:::`
  inside a code fence isn't a container marker).

Recommendation: deferred until a concrete fenced-container extension is
actually proposed.

### Provenance translation

If the scanner rewrites the source, byte positions shift — same
provenance concern as the inline spec. The translation strategy should be
shared with the inline spec (one combined table) if both rewriters run.

### Frontmatter interaction

Darkmatter extracts YAML frontmatter before parsing. The block scanner
runs on the post-frontmatter body, so frontmatter-vs-extension conflicts
shouldn't arise. Verify this assumption holds for any future extension
that wants to influence frontmatter.

### Conflict with existing CommonMark / GFM

Some hypothetical extensions could conflict with existing constructs:

- `{ ... }` attribute blocks — already used by some Markdown flavors
  (Pandoc, kramdown). Risk of confusion with existing user expectations.
- `:::` fenced containers — Pandoc uses this for divs. If darkmatter
  adopts the syntax, the semantics should match Pandoc as closely as
  possible to avoid surprises.

Each new block extension needs a compatibility review against established
Markdown flavors before shipping.

### Diagnostics

Block extensions should produce diagnostics at the same fidelity as
inline extensions:

- Malformed attribute block (`--- { style: }`) → diagnostic pointing at
  the attribute value.
- Unclosed fenced container (`:::warning` with no `:::`) → diagnostic at
  the opener.
- Unknown attribute key → warning, not error (forward-compat for keys
  the consumer doesn't yet know about).

This is a parity requirement with the current span-aware processor's
diagnostics, not a new ask.

### Naming

"Block span" is the directory name but is somewhat oxymoronic — block
constructs aren't spans. Open: rename to "block-extension" or
"block-syntax" or similar. Current name is fine for the brainstorming
phase.

## Decision Sequencing

1. **Lock the HR-attribute-only case first.** Migrate the existing
   handling out of the inline `SpannedInlineStyleProcessor` into a small
   dedicated block post-processor. Don't add any new block extensions
   yet. This is a refactor, not new functionality, and should be
   measurably cheaper because it skips the per-event wrapping.
2. **Profile** the refactored HR-attribute path against the current
   span-aware-fold path on `mark_dim_hr`. Confirm the architectural win.
3. **Defer new block extensions** (admonitions, heading attrs) until a
   concrete need surfaces. Each new extension goes through its own small
   design pass picking from the strategy table.

## Cross-Bucket Summary

The Phase-1 refactor (HR attributes out of the inline processor) is a
near-term Bucket-1 quick win in
[`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md)
once this spec stabilizes — it removes one of the consumers of the
expensive `SpannedInlineEvent` wrapping. Phase-2+ extensions are deferred
until concrete needs surface.

## Out of Scope

- Inline darkmatter extensions — owned by
  [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md).
- Designing specific future extensions (admonitions, heading attrs,
  fenced containers). This spec defines the *architectural slot*; each
  extension gets its own design pass.
- Graphics / image policy for HR rendering. Owned by
  [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
- Frontmatter parsing.
- Replacing pulldown-cmark.

## Related Specs

- [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md) —
  sibling spec for inline extensions.
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) —
  the perf spec; owns scheduling.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  cross-target graphics policy; owns the render-side HR styling decision.
- [`../_completed/2026-05-20-darkmatter-tree/spec.md`](../_completed/2026-05-20-darkmatter-tree/spec.md) —
  the parent migration spec.
- [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md) —
  recorded `mark_dim_hr` baselines.
