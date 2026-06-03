---
status: ready for planning and implementation
reviewed: true
---

# Block Extension: HR-Attribute Lift

## Status

**Ready for planning and implementation — architecture approved, narrow
scope.** This spec covers exactly one thing: moving the HR-attribute
extension (`--- { kind: …, weight: … }`, plus legacy `style`) out of the
span-aware inline transport and into a dedicated offset-aware block
post-processor. No new block extensions are designed here.

Reader's note from review: the current implementation does **not** have the
HR branch inside `SpannedInlineStyleProcessor` itself. It already has a
separate `SpannedRuleProcessor`, but that processor still lives in
`darkmatter/lib/src/markdown/render_tree/span.rs`, consumes
`SpannedInlineEvent`, and is wired after `SpannedInlineStyleProcessor`.
That still couples the HR path to the inline-span transport that the sibling
inline-span spec needs to delete. This spec therefore lifts
`SpannedRuleProcessor` into a real block-extension processor over the
pulldown-cmark offset stream, rather than merely disabling a branch inside
`SpannedInlineStyleProcessor`.

Decision lineage:

| Question | Decision | Notes |
|---|---|---|
| Spec scope: speculative general architecture or narrow refactor? | **Narrow — HR attributes only.** | The current single block extension is HR attrs. Designing for hypothetical future extensions is premature. |
| Future generality reconsidered? | **Yes, when concrete needs arrive.** | If a second bespoke block extension is proposed, this spec is the precedent and we re-evaluate whether a general architecture is then justified. |
| Directory naming | **`block-extension`.** | Doesn't lock us out of future block-extension work, even though scope today is just HR. |
| Sequencing vs the inline-span spec | **This ships before the inline-span replacement deletes span transport.** | `SpannedInlineStyleProcessor` and `SpannedInlineEvent` cannot be deleted until HR attributes have moved off that transport. |
| Approach | **Offset-event post-process** | No source rewriting; no architectural reshape. Lift the existing simple-paragraph HR detection to a small module that preserves source ranges. |

The sibling spec
[`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md)
covers inline darkmatter extensions (mark, dim, future emoji, var,
tooltip). Performance context lived in the `2026-05-21-isolated-perf` spec
(since removed; its surviving items moved to the perf-gate and browser-perf
specs).
The render-side decision about HR graphical rendering is owned by
[`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md);
this spec is the parse-side story only.

## Background

CommonMark's thematic-break rule is strict: a line containing three or
more `-`, `_`, or `*` characters and only those characters, plus optional
whitespace, is an HR. Anything else on the line invalidates the rule.

| Source | What pulldown-cmark sees |
|---|---|
| `---` | `Event::Rule` — native, free |
| `*** ***` | `Event::Rule` — native, free |
| `--- { kind: waves }` | A paragraph containing the literal text `--- { kind: waves }` — the attribute block fails the HR rule |

Darkmatter wants the third form to be an HR with attached style, weight,
width, alignment, and color attributes. Today the render-tree path handles
this in `darkmatter/lib/src/markdown/render_tree/span.rs`:

1. `SpanningAdapter` converts `(Event, Range<usize>)` into
   `SpannedInlineEvent`.
2. `SpannedInlineStyleProcessor` handles `==mark==` and `⌄dim⌄`.
3. `SpannedRuleProcessor` buffers simple paragraphs, recognizes
   `---|***|___ { ... }`, drops the paragraph wrapper, and emits
   `InlineEvent::HorizontalRule(attrs)` with generated provenance.
4. `fold_markdown_spanned_with_frontmatter` lowers that event to
   `NodeKind::ThematicBreak` and stores the attributes as
   `darkmatter.hr.*` hints on the node.

This is a **block-level concern living in the inline-span module and
transport**. The coupling was convenient because the span-aware chain already
preserved byte ranges, but it is now the blocker for deleting
`SpannedInlineEvent` and `span.rs` in the inline-span replacement.

## Proposed Change

Create a small dedicated block-extension processor in
`darkmatter::markdown::render_tree` that runs directly after
`Parser::new_ext(...).into_offset_iter()` and before any inline-extension
processing.

### Pipeline Shape

```text
   raw source
       |
       v
   pulldown-cmark parser + offset iterator
       |
       v
   block-extension processor        <- HR-attribute recognition lives here
       |
       v
   inline-span dispatcher           <- mark / dim / future inline tokens
       |
       v
   fold -> Document
```

The block-extension processor runs before the inline-span dispatcher because
HR-attribute syntax is an entire-paragraph construct. Matching it first
removes the paragraph from the stream before inline extension logic can
split or rewrite its text.

### Event Shape

Use a small render-tree-local event enum rather than a side-channel map:

```rust
enum BlockExtensionEvent<'a> {
    Standard(Event<'a>),
    HorizontalRule(HorizontalRuleAttrs),
}
```

The processor's iterator item must also carry the source byte range and
provenance, matching the current span-aware range policy:

- Parsed standard events keep `Provenance::Parsed` and their
  pulldown-cmark offset range.
- Synthetic HR events use `Provenance::Generated` and point their location at
  the original paragraph **body** range: the buffered text event range, not
  the paragraph end range that may include a trailing newline.

This keeps the current `InlineEvent::HorizontalRule(attrs)` behavior without
requiring the new block processor to depend on inline event types.

### What The Processor Does

1. Wraps `Parser::new_ext(...).into_offset_iter()`.
2. Buffers only `Start(Paragraph)` to matching `End(Paragraph)` regions.
3. Matches exactly the current simple-paragraph policy: the paragraph must
   contain one `Event::Text` and no other inline events.
4. Trims the text body and recognizes `---|***|___ { ... }` with three or
   more identical marker characters, optional whitespace before the attribute
   block, and a single trailing `}`.
5. Parses the attribute block into `HorizontalRuleAttrs`.
6. When matched, drops the paragraph start/end and emits one
   `BlockExtensionEvent::HorizontalRule(attrs)` over the text-event body
   range.
7. When unmatched, flushes the buffered paragraph events unchanged.

The matcher must preserve the current behavior that a bare `---` is **not**
handled by this processor. Bare thematic breaks already arrive from
pulldown-cmark as `Event::Rule` and must continue through the standard fold
path.

### Attribute Parsing And Warnings

The implementation must consolidate, not fork, the HR attribute parser.
Today there are two relevant paths:

- `RuleProcessor::parse_attributes`, which parses YAML flow mappings, falls
  back to the legacy comma splitter on malformed YAML, emits `tracing::warn!`
  for malformed / unknown / non-scalar fields, and records a deprecation
  `StyleWarning` when legacy `style` is used.
- `try_parse_hr_attrs`, which is convenient for the span-aware fold but is a
  reduced free-function clone and does not preserve all warning/fallback
  behavior.

Design decision: introduce one shared parser helper in
`darkmatter::markdown::block` that returns both the match result and warning
data needed by callers. The legacy `RuleProcessor`, the new block-extension
processor, and `scan_inline_hr_warnings` must all call that helper. This is
slightly more work than continuing to call `try_parse_hr_attrs`, but it
prevents warning drift and keeps strict-style behavior consistent across the
legacy and tree paths.

### Fold Lowering

Move the existing HR lowering from
`fold_markdown_spanned_with_frontmatter` into a small helper shared by the
current span-aware fold and the new block-extension fold path:

- Build `RenderNode::thematic_break()`.
- Set `SourceSpan { provenance: Generated, location: body_range }`.
- Attach `darkmatter.hr.kind`, `alignment`, `weight`, `width`, and `color`
  hints via `NodeAttrs::set_hint`.
- Preserve precedence: `kind` wins over legacy `style`.

No renderable tree IR change is required.

## Preserved Behavior

- Byte-stable output for every existing HR-attribute fixture, including
  `mark_dim_hr`. The lift must not change what the render-tree pipeline emits.
  This is **not** a legacy-renderer-vs-tree byte-equality claim — those are two
  different renderers and equality between them is meaningless (see
  `darkmatter/lib/tests/render_tree_parity.rs`, which asserts only semantic
  invariants between the pipelines). The byte-stable contract is enforced by
  pinning the **tree pipeline's own** output for the HR fixtures with insta
  snapshots in `darkmatter/lib/tests/render_tree_hr_snapshots.rs`; any drift in
  the lifted `BlockExtensionProcessor` path fails an exact-match assertion.
- Same simple-paragraph policy. Paragraphs with emphasis, links, inline code,
  mark/dim spans, raw HTML, or any other nested inline events must not rewrite
  to an HR.
- Same marker set: `---`, `***`, and `___` with three or more identical
  marker characters.
- Same `HorizontalRuleAttrs` value type.
- Same hint namespace and keys: `darkmatter.hr.kind`, `alignment`, `weight`,
  `width`, `color`.
- Same source-span policy for generated HRs: generated provenance attached to
  the paragraph body byte range.
- Same malformed-attribute fallback and warnings as the legacy
  `RuleProcessor`.
- Same render-side behavior. Image-vs-text and SVG fidelity decisions remain
  owned by the graphics-policy spec.

## Goals

- Decouple HR-attribute handling from `SpannedInlineEvent`,
  `SpannedInlineStyleProcessor`, and `span.rs`.
- Enable the inline-span spec's eventual deletion of the span-aware inline
  transport.
- Preserve HR-attribute behavior, source ranges, and warnings.
- Keep the implementation deliberately small and specific to HR attributes.
- Provide a concrete precedent that a future block-extension spec can cite if
  a second block extension becomes necessary.

## Non-Goals

- Designing a general block-extension framework for admonitions, heading
  attributes, fenced containers, or front-of-block metadata.
- Changes to HR attribute syntax.
- Changes to `HorizontalRuleAttrs`.
- Changes to the renderable tree IR.
- Render-side HR decisions; those belong to
  [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
- Replacing pulldown-cmark.

## Migration Plan

Ordered so each step can land on a green tree:

1. **Extract shared HR parsing.** Add a shared helper in
   `darkmatter::markdown::block` that preserves `RuleProcessor` semantics:
   YAML flow parsing, legacy fallback, unknown/non-scalar warnings,
   `style` deprecation warning data, and `kind` over `style` precedence.
   Rewire `RuleProcessor`, `try_parse_hr_attrs`, and
   `scan_inline_hr_warnings` to use it.
2. **Create `darkmatter::markdown::render_tree::block_extension`.** Define
   the offset-aware event wrapper, provenance metadata, and HR-attribute
   processor over `(Event, Range<usize>)`.
3. **Add fold helpers.** Extract the current `InlineEvent::HorizontalRule`
   lowering into a helper that accepts `HorizontalRuleAttrs` plus the
   generated body range and appends the thematic-break node with
   `darkmatter.hr.*` hints.
4. **Wire block-extension before inline processing.** In
   `fold_markdown_spanned_with_frontmatter`, run:
   `Parser::new_ext(...).into_offset_iter() -> BlockExtensionProcessor ->
   inline-span/style processor -> fold`. During this transition the inline
   processor may still handle mark/dim; HR attributes must come only from the
   block-extension processor.
5. **Retire `SpannedRuleProcessor`.** Delete or make unreachable the old
   `SpannedRuleProcessor` path from `span.rs` once the new processor is
   verified. `span.rs` should then contain only inline-span compatibility
   code until the sibling spec removes it completely. This step retires the
   **span-aware** HR processor only; the legacy event-stream `RuleProcessor`
   is intentionally preserved — see [Legacy `RuleProcessor` Retention](#legacy-ruleprocessor-retention).
6. **Run tests and parity checks.** Verify the full test corpus,
   HR-attribute unit tests, strict-style warning tests, and the
   `migration_parity` bench outputs. Expected performance change is neutral;
   expected output change is none.
7. **Hand off to inline-span.** The sibling spec can then replace and delete
   `SpannedInlineStyleProcessor` without losing HR-attribute support.

## Legacy `RuleProcessor` Retention

This lift retires the **span-aware** `SpannedRuleProcessor` from the
render-tree chain. It deliberately does **not** retire the legacy
event-stream `RuleProcessor`
(`darkmatter/lib/src/markdown/block/rule_processor.rs`).

`RuleProcessor` remains the active HR-attribute processor for the legacy
renderers — `Markdown::as_html` (`output/html.rs`) and `for_terminal`
(`output/terminal.rs`) — which back darkmatter's current public render API.
The render-tree pipeline is still an internal/experimental entry point
(`render_tree/mod.rs` keeps `render_tree_html` / `render_tree_terminal`
`pub(crate)`); the public cutover to the tree path is owned by the
darkmatter-tree entry-point work, not this spec. Until that cutover lands,
deleting `RuleProcessor` would break the public renderers.

The two processors now share one parser (`parse_hr_attribute_block`), so there
is no behavioral fork to worry about while both exist — only one extra
event-stream iterator adapter that the legacy renderers still depend on.

**Follow-up removal gate.** `RuleProcessor` may be deleted once **all** of the
following hold:

1. `Markdown::as_html` and `for_terminal` are cut over to the render-tree
   pipeline (the darkmatter-tree entry-point cutover), or those legacy
   renderers are themselves retired.
2. No remaining caller constructs `RuleProcessor` or routes through the
   `InlineEvent::HorizontalRule` event-stream path.
3. The shared `parse_hr_attribute_block` helper and the
   `scan_inline_hr_warnings` preflight remain the single source of truth for
   HR-attribute parsing and warnings.

Tracking this as an explicit gate keeps the lift honest: the architectural
goal of removing the HR dependency on **inline-span transport** is met by this
spec; removing the legacy **event-stream** HR path is a separate, gated step.

## Performance Expectation

This is a correctness-and-architecture refactor, not a perf change.
HR-attribute recognition still buffers simple paragraphs and performs the
same attribute parse. Moving it earlier may avoid running inline extension
logic on matched HR paragraphs, but the signal is expected to be tiny next to
the sibling inline-span replacement.

Do not treat this spec as a perf win. The success criteria are behavioral
parity, source-span parity, and removal of the HR dependency on inline-span
transport.

## Open Questions

The architecture is locked; these are implementation-level sub-decisions to
pin during the lift.

### Module Placement

Two reasonable spots for the new processor:

- `darkmatter/lib/src/markdown/render_tree/block_extension.rs`
  - Pros: lives beside the fold that consumes it; keeps render-tree source
    range policy local.
  - Cons: block parsing helpers still live in `darkmatter::markdown::block`,
    so the module will import that shared parser.
- `darkmatter/lib/src/markdown/block/`
  - Pros: near `RuleProcessor` and the HR parser.
  - Cons: the existing `block` module backs legacy event-stream renderers;
    putting render-tree offset/provenance types there muddles the boundary.

Recommendation: use `render_tree/block_extension.rs`. The parser helper
belongs in `markdown::block`; the offset-aware processor belongs in
`markdown::render_tree`.

### Wrapper Iterator Shape

Two implementations are possible:

- A `Peekable` iterator plus match-on-current logic.
  - Pros: small diff and easy to read.
  - Cons: easier to mishandle paragraph flushing and end ranges.
- A small state machine that buffers the active paragraph.
  - Pros: matches the current `SpannedRuleProcessor` shape, preserves the
    simple-paragraph policy naturally, and makes unmatched flush behavior
    explicit.
  - Cons: a little more code.

Recommendation: state machine. The pattern is fixed-shape and the current
processor already proves this shape works.

### Event Type Name

The new event enum should not reuse `InlineEvent`.

- `BlockExtensionEvent`
  - Pros: clear about ownership and scope; future block specs can cite it.
  - Cons: sounds slightly more general than this HR-only implementation.
- `RenderTreeEvent`
  - Pros: describes the fold-facing role.
  - Cons: too broad for a processor that only adds one extension event.

Recommendation: `BlockExtensionEvent`. Pair the name with module docs that
state only HR attributes are implemented today and future extensions require
their own spec.

### Warning Surface

The current render-tree fold returns `Vec<renderable::tree::Diagnostic>`,
while legacy strict-style warning collection uses `StyleWarning`. HR
attribute parser warnings currently include tracing warnings plus
`StyleWarning` for legacy `style`.

Options:

- Keep `StyleWarning` collection outside the fold and only ensure
  `scan_inline_hr_warnings` uses the shared parser.
  - Pros: smallest surface change; preserves today's public warning model.
  - Cons: tree entry points still do not expose style warnings directly.
- Thread style warnings through the render-tree pipeline result.
  - Pros: one render-tree call can expose all warnings.
  - Cons: widens the tree entry-point contract and is larger than this lift.

Recommendation: keep the existing warning surface for this spec. The shared
parser must make warning behavior consistent, but exposing style warnings
through render-tree entry points should be a separate public-entrypoint
decision if needed.

### Fenced-Block Defensive Scan

The processor should not match HR-attribute text inside code fences. Because
pulldown-cmark emits fenced code as `Start(CodeBlock)` plus text inside that
container rather than as a top-level paragraph, the simple `Paragraph` matcher
is naturally safe.

Add a regression test anyway. The expected behavior is that:

````markdown
```text
--- { kind: waves }
```
````

remains a code block and does not produce a `ThematicBreak`.

## Decision Sequencing

Implementation gates:

1. Shared HR parser keeps legacy `RuleProcessor` tests green.
2. New block-extension processor passes focused unit tests for matched,
   unmatched, malformed, nested-inline, blockquote, list, and fenced-code
   cases.
3. Fold wiring produces byte-identical `mark_dim_hr` output and preserves
   generated source ranges.
4. Old `SpannedRuleProcessor` is removed from the active chain.
5. Inline-span replacement can delete `SpannedInlineStyleProcessor`.

## When To Revisit Generality

This spec is deliberately narrow. Reopen the general block-extension
architecture question only when one of these becomes concrete:

- A second bespoke block extension is proposed.
- HR-attribute syntax grows in a way that suggests shared block parser
  infrastructure.
- Repeated requests arrive for fenced-container syntax such as
  `::: warning ... :::`.

At that point, evaluate the second extension as real evidence. Do not
generalize from HR attributes alone.

## Out Of Scope

- Designing future block extensions or shared infrastructure.
- Inline darkmatter extensions, owned by
  [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md).
- Render-side HR rendering decisions, owned by
  [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
- Frontmatter parsing.
- Changes to pulldown-cmark or the renderable tree IR.

## Related Specs

- [`../2026-05-26-inline-span/spec.md`](../2026-05-26-inline-span/spec.md) —
  sibling spec. This HR lift is a hard prerequisite for that spec's
  `SpannedInlineStyleProcessor` deletion step.
- `2026-05-21-isolated-perf` (removed) — perf context. This refactor is not a
  perf win on its own.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  cross-target graphics policy. Owns render-side HR decisions; this spec's
  parsed `HorizontalRuleAttrs` are graphics-policy input.
- [`../_completed/2026-05-20-darkmatter-tree/spec.md`](../_completed/2026-05-20-darkmatter-tree/spec.md) —
  parent migration spec.
