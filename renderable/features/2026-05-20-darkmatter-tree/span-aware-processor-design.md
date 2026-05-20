---
feature: "@renderable/features/2026-05-20-darkmatter-tree"
prompt: |-
    Before we kickoff the Darkmatter implementation of the new [tree-rendering](@renderable/docs/tree-rendering.md)
    we need to design how InlineStyleProcessor and RuleProcessor will preserve ranges.

    This is the most important technical blocker. The fold wants into_offset_iter(), but the Darkmatter processors currently synthesize events without offsets. We should decide the event type and range policy before coding.

    Key decision: for split text like foo ==bar== baz, do child spans use exact byte subranges, whole original text range, or synthetic spans tied back to the original range?

    ## Task

    Write up a mini-design to the body of this Markdown document that addresses this design concern and adds text
    fixtures for mark, dim, escaped delimiters, and HR attributes.

    > **Note:** this mini-design complements the existing '{{ feature }}/spec.md'
last_updated: 2026-05-20
---

# Span-Aware Darkmatter Processors

## Problem

The render-tree fold currently consumes:

```rust
Parser::new_ext(input, options).into_offset_iter()
```

That gives the fold `(Event, Range<usize>)` pairs. The fold converts those
ranges into `SourceSpan { provenance: Parsed, location: Some(...) }` so every
node can point back to source bytes.

Darkmatter's custom processors do not preserve ranges:

- `InlineStyleProcessor` consumes `Iterator<Item = Event<'a>>` and splits text
  events into mark/dim events.
- `RuleProcessor` consumes `Iterator<Item = InlineEvent<'a>>`, buffers a whole
  paragraph, and may replace it with `InlineEvent::HorizontalRule`.

Those processors are required for Darkmatter parity, but the fold cannot use
them as-is without losing source provenance.

## Decision

Add a span-aware processor chain for the fold. Keep the existing non-spanned
processors for legacy renderers until public cutover.

```rust
pub(crate) struct SpannedInlineEvent<'a> {
    pub event: InlineEvent<'a>,
    pub range: std::ops::Range<usize>,
    pub provenance: SpannedEventProvenance,
}

pub(crate) enum SpannedEventProvenance {
    Parsed,
    GeneratedFrom { source: std::ops::Range<usize> },
}
```

This is an internal Darkmatter transport type. It is not the final
`renderable::tree::SourceSpan`. The fold still owns conversion from event
ranges into `SourceSpan` because it has the `SourceId`.

Mapping to tree provenance:

- `Parsed` becomes `SourceSpan { provenance: Provenance::Parsed, location:
  Some(SourceLocation { source, bytes: range }) }`.
- `GeneratedFrom { source }` becomes `SourceSpan { provenance:
  Provenance::Generated, location: Some(SourceLocation { source, bytes: source
  }) }`.

The `range` field remains the event's operational range. For generated events,
it should normally be the same as `source`; the separate enum makes the
provenance explicit.

## Range Policy

### Split Text

For split text events, use exact byte subranges of the original text event.

Example source:

```text
foo ==bar== baz
```

Byte ranges:

| Event | Range |
|-------|-------|
| `Text("foo ")` | `0..4` |
| `Start(Mark)` | `4..6` |
| `Text("bar")` | `6..9` |
| `End(Mark)` | `9..11` |
| `Text(" baz")` | `11..15` |

The resulting `Span.mark` node's range is computed by the fold from container
start/end events, so it covers `4..11`.

### Unicode Delimiters

All offsets are byte offsets. The dim delimiter `⌄` is three bytes in UTF-8.

Example:

```text
normal ⌄dimmed⌄ after
```

Byte ranges:

| Segment | Range |
|---------|-------|
| `Text("normal ")` | `0..7` |
| `Start(Dim)` delimiter | `7..10` |
| `Text("dimmed")` | `10..16` |
| `End(Dim)` delimiter | `16..19` |
| `Text(" after")` | `19..25` |

The dim container covers `7..19`.

### Escaped Delimiters

Escaped delimiters become literal text and include the escape prefix in their
source span.

Examples:

- `\==` emits literal `Text("==")` with a range covering the original `\==`
  bytes.
- `\⌄` emits literal `Text("⌄")` with a range covering the original `\⌄`
  bytes.

This mirrors normal parser behavior: the rendered text excludes the escape
marker, but diagnostics should still point at the source bytes that produced
the literal.

### Unclosed Delimiters

Unclosed mark or dim delimiters revert to literal text. The literal delimiter
text carries its exact source range.

Do not emit an empty or partially-open container for an unclosed delimiter.

### Empty Delimiters

Empty mark pairs such as `====` should not create an empty mark node in the
first implementation. Treat them as literal text unless the existing legacy
processor demonstrably emits a mark pair. This avoids introducing a new public
semantic while the migration is trying to preserve behavior.

### Generated Horizontal Rules

`RuleProcessor` replaces a simple paragraph such as:

```markdown
--- { style: waves }
```

with a horizontal-rule event. That event should use:

```rust
SpannedEventProvenance::GeneratedFrom { source: paragraph_range }
```

The eventual `ThematicBreak` node receives `Provenance::Generated` and a
location pointing back to the original paragraph bytes.

Plain CommonMark rules emitted directly by `pulldown-cmark` remain
`Provenance::Parsed`.

## Processor Shape

The fold pipeline becomes:

```text
Parser::new_ext(input, options).into_offset_iter()
  -> SpanningAdapter
  -> SpannedInlineStyleProcessor
  -> SpannedRuleProcessor
  -> fold
```

Responsibilities:

- `SpanningAdapter` converts `(Event, Range<usize>)` into
  `SpannedInlineEvent { event: InlineEvent::Standard(event), range,
  provenance: Parsed }`.
- `SpannedInlineStyleProcessor` mirrors `InlineStyleProcessor` but emits exact
  subranges for split text and delimiters.
- `SpannedRuleProcessor` mirrors `RuleProcessor` but preserves buffered event
  ranges and emits generated HR events with paragraph provenance.
- Legacy `InlineStyleProcessor` and `RuleProcessor` remain untouched until the
  tree path is public.

This duplicates some processor mechanics in the short term. That is acceptable
for the migration because it avoids destabilizing legacy renderers. Once the
tree path becomes public, the two processor implementations should be
converged or the non-spanned path retired.

## Fold Mapping

Add fold support for the Darkmatter inline events:

| Event | Tree mapping |
|-------|--------------|
| `InlineEvent::Start(InlineTag::Mark)` / `End` | `NodeKind::Span` with class `mark` |
| `InlineEvent::Start(InlineTag::Dim)` / `End` | `NodeKind::Span` with a `Style` hint whose `TextEmphasis.dim = true` |
| `InlineEvent::HorizontalRule(attrs)` | `NodeKind::ThematicBreak` with HR attrs stored in `NodeAttrs` |

Do not use `NodeKind::Emphasis` for dim. In the tree vocabulary,
`Emphasis` is semantic italic emphasis. Dim is presentational style and should
ride in `Style`.

## HR Attribute Storage

Keep `NodeKind::ThematicBreak` as a unit variant. Store Darkmatter HR
attributes in `NodeAttrs::data` under a namespaced key.

Recommended first implementation:

```rust
const DARKMATTER_HR: HintNamespace = HintNamespace("darkmatter.hr");

attrs.set_hint(DARKMATTER_HR, "style", json!("waves"));
attrs.set_hint(DARKMATTER_HR, "width", json!("50%"));
attrs.set_hint(DARKMATTER_HR, "alignment", json!("centered"));
```

Do not add `HintNamespace::HR` to `renderable` until more than Darkmatter needs
it. If HR attrs become a cross-package feature, promote them to typed helpers
later.

Renderers that do not understand the hint should render a normal thematic
break.

## Text Fixtures

These fixtures are for fold and parity tests. Byte ranges are zero-based and
refer to UTF-8 bytes in the fixture source.

### Mark: Basic

```markdown
plain ==highlighted== after
```

Expected tree:

```text
Root
  Paragraph
    Text "plain " [0..6]
    Span.mark [6..21]
      Text "highlighted" [8..19]
    Text " after" [21..27]
```

### Mark: Multiple

```markdown
==one== and ==two== end
```

Expected tree:

```text
Root
  Paragraph
    Span.mark [0..7]
      Text "one" [2..5]
    Text " and " [7..12]
    Span.mark [12..19]
      Text "two" [14..17]
    Text " end" [19..23]
```

### Mark: Escaped

```markdown
this is \== not highlighted
```

Expected:

- No `Span.mark`.
- Text contains `this is == not highlighted`.
- The literal `==` span includes the original backslash byte.

### Mark: Unclosed

```markdown
this ==is not closed
```

Expected:

- No `Span.mark`.
- The delimiter remains literal text.

### Dim: Basic

```markdown
normal ⌄dimmed⌄ after
```

Expected tree:

```text
Root
  Paragraph
    Text "normal " [0..7]
    Span(style.dim = true) [7..19]
      Text "dimmed" [10..16]
    Text " after" [19..25]
```

### Dim: With Italic

```markdown
⌄*dim and italic*⌄
```

Expected:

```text
Root
  Paragraph
    Span(style.dim = true)
      Emphasis
        Text "dim and italic"
```

### Dim: Escaped

```markdown
not \⌄ dimmed here
```

Expected:

- No dim style span.
- Text contains `not ⌄ dimmed here`.

### Dim: Inline Code

```markdown
use `⌄not dim⌄` in code
```

Expected:

```text
Root
  Paragraph
    Text "use "
    InlineCode "⌄not dim⌄"
    Text " in code"
```

### HR Attributes: Basic

```markdown
--- { style: waves }
```

Expected:

```text
Root
  ThematicBreak provenance=Generated location=0..20
    data.darkmatter.hr.style = "waves"
```

### HR Attributes: Multiple Attributes

```markdown
*** { style: dots, width: "50%", alignment: centered }
```

Expected:

```text
Root
  ThematicBreak provenance=Generated
    data.darkmatter.hr.style = "dots"
    data.darkmatter.hr.width = "50%"
    data.darkmatter.hr.alignment = "centered"
```

### HR Attributes: Plain Rule

```markdown
---
```

Expected:

- `ThematicBreak` with `Provenance::Parsed`.
- No `darkmatter.hr` attrs.

### HR Attributes: Block Quote

```markdown
> --- { style: gradient }
```

Expected:

```text
Root
  BlockQuote
    ThematicBreak provenance=Generated
      data.darkmatter.hr.style = "gradient"
```

### HR Attributes: List Item

```markdown
- --- { style: waves }
```

Expected behavior must match the legacy `RuleProcessor`. If legacy does not
transform HR-attribute paragraphs inside list items, the tree fold must leave
it as paragraph text. If legacy does transform it, the tree fold must do the
same. Add an explicit test before implementation; do not infer this from the
desired tree shape.

### Mixed Mark and Dim

```markdown
==highlighted and ⌄dim within mark⌄==
```

Expected:

```text
Root
  Paragraph
    Span.mark
      Text "highlighted and "
      Span(style.dim = true)
        Text "dim within mark"
```

### Mark in Table Cell

```markdown
| Header |
|--------|
| ==val== |
```

Expected:

```text
Root
  Table
    TableRow
      TableCell
        Text "Header"
    TableRow
      TableCell
        Span.mark
          Text "val"
```

## Migration Path

1. Add `SpannedInlineEvent`, `SpannedEventProvenance`, and the spanning
   adapter.
2. Add `SpannedInlineStyleProcessor` and `SpannedRuleProcessor`.
3. Add fold support for mark, dim, and HR attributes.
4. Add the fixtures above as fold tests first, then legacy-vs-tree parity
   tests.
5. Keep legacy renderers on the existing processors until public cutover.
6. After cutover, decide whether to retire the non-spanned processors or make
   all render paths use the spanned chain.
