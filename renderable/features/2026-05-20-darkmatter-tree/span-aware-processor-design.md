---
feature: "@renderable/features/2026-05-20-darkmatter-tree"
prompt: |-
    Before we kickoff the Darkmatter implementation of the new [tree-rendering](@renderable/docs/tree-rendering.md)
    we need to design how InlineStyleProcessor and RuleProcessor will preserve ranges.
    
    This is the most important technical blocker. The fold wants into_offset_iter(), but the Darkmatter processors currently synthesize events without offsets. We should decide the event type and range policy before coding.
    
    Key decision: for split text like foo ==bar== baz, do child spans use exact byte subranges, whole original text range, or synthetic spans tied back to the original range?

    ## Task

    Write up a mini-design to the body of this Markdown document that address this design concern and adds text
    fixtures for mark, dim, escaped delimiters, and HR attributes
    
    > **Note:** this mini-design is meant to compliments the existing '{{ feature }}/spec.md'
last_updated: 2026-05-20
---
Now let me explore the actual Darkmatter processor implementations and the fold code to understand the current architecture.Now let me read the InlineEvent type and the existing rendering pipelines to understand the full picture:Now I have a thorough understanding of the architecture. Let me also check the existing HR feature spec for context on attributes:I now have comprehensive understanding of the architecture. Let me compose the mini-design document body.

## Span-Aware Processor Design

### The Problem

The fold in `darkmatter/lib/src/markdown/render_tree/fold.rs:199` consumes `Parser::new_ext(input, options).into_offset_iter()`, which yields `(Event, Range<usize>)` pairs. Every node gets a `SourceSpan` with `Provenance::Parsed` and a concrete byte range.

`InlineStyleProcessor` and `RuleProcessor` are iterator adapters over `Iterator<Item = Event<'a>>` (no ranges). They synthesize events — splitting `Event::Text` into segments, replacing whole paragraphs with `InlineEvent::HorizontalRule(HorizontalRuleAttrs)` — and have no mechanism to carry or compute byte offsets for the synthesized events.

You cannot stack these processors on top of `into_offset_iter()` because their item type (`InlineEvent<'a>`) is incompatible with `((Event, Range<usize>)`.

### Proposed Solution: SpannedInlineEvent

Introduce a new span-carrying wrapper type:

```rust
struct SpannedInlineEvent<'a> {
    event: InlineEvent<'a>,
    range: Range<usize>,
}
```

Both processors gain a second mode that consumes and emits `SpannedInlineEvent<'a>`. The existing non-spanned path remains the default for the legacy renderers — no behavior change.

### Range Policy for Split Text Events

**Decision: exact byte subranges of the original `Event::Text` range.**

When `InlineStyleProcessor` splits a text event like `foo ==bar== baz` (original range `0..17`), each synthesized child gets an exact byte subrange computed from the original text content:

| Event          | Range    | Rationale                           |
|----------------|----------|-------------------------------------|
| `Text("foo ")` | `0..4`   | Leading text, verbatim from source  |
| `Start(Mark)`  | `4..6`   | The `==` opening delimiter bytes    |
| `Text("bar")`  | `6..9`   | The highlighted content             |
| `End(Mark)`    | `9..11`  | The `==` closing delimiter bytes    |
| `Text(" baz")` | `11..17` | Trailing text, verbatim from source |

The `Start`/`End` container markers take the range of their delimiters. The container node's `SourceSpan` in the fold is computed as `start.start..end.end` (i.e. `4..11`), matching how the fold already computes container spans in `fold.rs:343`.

This is the simplest policy that preserves useful provenance:

- Diagnostics can point to the exact `==bar==` span, not the whole paragraph.
- Each text leaf maps to real source bytes.
- No "whole original range" lie where child spans overlap siblings.

### Range Policy for Synthetic Events

**Decision: `Provenance::Generated` with the parent paragraph's range as a fallback location.**

`RuleProcessor` replaces `--- { style: waves }` paragraphs with `InlineEvent::HorizontalRule(HorizontalRuleAttrs)`. The original paragraph's byte range is known. The synthesized HR event carries:

- `provenance: Provenance::Generated`
- `location: Some(SourceLocation { source, bytes: original_paragraph_range })`

This tells downstream consumers "this node was not in the source literally, but it came from this span." The fold already has `Provenance::Generated` in its enum for this purpose.

### Range Policy for Unclosed Delimiters

When an unclosed `==` reverts to literal text, the processor emits `Text("==")` with the exact byte range of the literal `==` characters. No special handling needed — it is a regular text event.

### Range Policy for Escaped Delimiters

Escaped `\==` is preprocessed into sentinel characters before parsing. After the processor finishes, the sentinels are converted back. The range policy:

- The `Text("==")` event for the escaped literal spans the original `\==` bytes (range includes the backslash).
- This matches the current behavior where escaped characters consume their escape prefix in the source.

### Implementation Strategy

Rather than modifying `InlineStyleProcessor` and `RuleProcessor` to operate on `(Event, Range)` tuples, introduce a thin **span-propagation adapter** layer:

1. **`SpanningAdapter`** wraps `into_offset_iter()` and emits `SpannedInlineEvent` items. It passes standard events through with their ranges unchanged.
2. **`SpannedInlineStyleProcessor`** wraps `SpanningAdapter` and performs the same delimiter logic as `InlineStyleProcessor`, but computes subranges during text splitting. It receives `SpannedInlineEvent`, emits `SpannedInlineEvent`.
3. **`SpannedRuleProcessor`** wraps `SpannedInlineStyleProcessor` and performs the same paragraph-buffer check. When it replaces a paragraph with an HR event, it uses the paragraph's range. When it re-emits the buffered events, ranges pass through unchanged.
4. The **fold** consumes the spanned processor chain instead of bare `into_offset_iter()`.

The non-spanned processors (`InlineStyleProcessor`, `RuleProcessor`) remain untouched for the legacy renderers. This avoids any risk to existing `as_html` / `for_terminal` behavior.

### Fold Integration Point

The fold currently has this loop (simplified):

```rust
for (event, range) in Parser::new_ext(input, options).into_offset_iter() {
    match event { /* ... */ }
}
```

With spanned processors, it becomes:

```rust
let parser = Parser::new_ext(input, options).into_offset_iter();
let adapter = SpanningAdapter::new(parser);
let inline = SpannedInlineStyleProcessor::new(adapter);
let rules = SpannedRuleProcessor::new(inline);
for spanned in rules {
    let SpannedInlineEvent { event, range } = spanned;
    match event {
        InlineEvent::Standard(ev) => { /* fold as before */ }
        InlineEvent::Start(InlineTag::Mark) => { fold.push_container(Mark, range); }
        InlineEvent::End(InlineTag::Mark) => { fold.pop_container(&range); }
        InlineEvent::Start(InlineTag::Dim) => { fold.push_container(Dim, range); }
        InlineEvent::End(InlineTag::Dim) => { fold.pop_container(&range); }
        InlineEvent::HorizontalRule(attrs) => { fold.thematic_break_with_attrs(attrs, range); }
    }
}
```

### Node Mapping

| Darkmatter Construct                   | Render Tree Node                                                                                                                       |
|----------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
| `==text==` (mark)                      | `NodeKind::Span { children }` with `attrs.classes = vec!["mark"]`                                                                      |
| `⌄text⌄` (dim)                         | `NodeKind::Emphasis { children }` with `attrs.set_style(Style { emphasis: TextEmphasis { dim: true, .. }, .. })`                       |
| `--- { style: waves }` (HR with attrs) | `NodeKind::ThematicBreak` with attrs stored in `NodeAttrs` namespaced data (`HintNamespace::HINTS` or a dedicated `HintNamespace::HR`) |

The `ThematicBreak` variant is currently a bare unit variant. HR attributes are carried entirely through `NodeAttrs` — no kind-level change needed. The renderer checks `node.attrs` for HR-specific hints and applies them.

### Subrange Computation During Text Splitting

When `InlineStyleProcessor` splits `Event::Text("foo ==bar== baz")` at range `10..27`, it:

1. Scans the `CowStr` for delimiter positions.
2. For each segment, computes `range.start + segment_byte_offset .. range.start + segment_byte_offset + segment_byte_len`.
3. Assigns the computed subrange to the synthesized `Text` event.
4. Assigns the delimiter's byte span to `Start`/`End` events.

The offset arithmetic is straightforward because the original `Event::Text` from `into_offset_iter()` always covers exactly the bytes that the `CowStr` contains. No special Unicode handling is needed — byte offsets are the native currency.

### Text Fixtures

These fixtures are designed for parity testing between legacy and tree rendering paths. Each is a self-contained Markdown fragment with a description of the expected tree structure.

#### Fixture 1: Mark — Basic

```markdown
plain ==highlighted== after
```

Expected tree (structural):

```text
Root
  Paragraph [0..28]
    Text "plain " [0..6]
    Span .mark [6..20]
      Text "highlighted" [8..18]
    Text " after" [20..28]
```

Key assertions:

- The `Span` node's span covers `==highlighted==` (bytes 6..20, including delimiters).
- The inner `Text` node covers only `highlighted` (bytes 8..18, excluding delimiters).
- Provenance is `Parsed` for all nodes.

#### Fixture 2: Mark — Multiple in One Paragraph

```markdown
==one== and ==two== end
```

Expected tree:

```text
Root
  Paragraph [0..23]
    Span .mark [0..7]
      Text "one" [2..5]
    Text " and " [7..12]
    Span .mark [12..19]
      Text "two" [14..17]
    Text " end" [19..23]
```

#### Fixture 3: Mark — Nested Inside Emphasis

```markdown
*this is ==important**==
```

Note: pulldown-cmark parses `*...*` first; the `==` inside is processed by `InlineStyleProcessor` after the emphasis text is emitted. The exact nesting depends on processor ordering. Expected tree:

```text
Root
  Paragraph
    Emphasis
      Text "this is "
      Span .mark
        Text "important*"
```

(The trailing `*` is consumed by the emphasis closer before mark sees it; the trailing `==` becomes literal.)

#### Fixture 4: Mark — Escaped Delimiter

```markdown
this is \== not highlighted
```

After escape preprocessing, `\==` becomes a literal `==`. Expected tree:

```text
Root
  Paragraph
    Text "this is == not highlighted"
```

Key assertion: no `Span .mark` node appears. The escaped `==` is plain text.

#### Fixture 5: Mark — Unclosed Delimiter

```markdown
this ==is not closed
```

Expected tree:

```text
Root
  Paragraph
    Text "this ==is not closed"
```

Key assertion: no `Span .mark` node. The unclosed `==` reverts to literal text.

#### Fixture 6: Mark — Empty Mark

```markdown
before ==== after
```

The `====` is two adjacent `==` pairs with zero-length content between them. Expected tree:

```text
Root
  Paragraph
    Text "before "
    Span .mark [7..11]
      Text "" [9..9]
    Text " after"
```

Key assertion: the empty `Span .mark` is emitted but its text child is an empty string. Renderers may collapse or skip it.

#### Fixture 7: Dim — Basic

```markdown
normal ⌄dimmed⌄ after
```

Expected tree:

```text
Root
  Paragraph [0..21]
    Text "normal " [0..7]
    Emphasis { dim: true } [7..15]
      Text "dimmed" [8..14]
    Text " after" [15..21]
```

Key assertions:

- Dim maps to an inline container with `TextEmphasis { dim: true }`.
- The `⌄` delimiters are each 3 bytes (U+2304 is 3 bytes in UTF-8), so the opening delimiter is bytes 7..10 and closing is 14..17 in source. Wait — actually `⌄` is U+2304 which encodes as `E2 8C 84` (3 bytes). The paragraph bytes would be `normal ⌄dimmed⌄ after`.
- Actual byte ranges depend on the UTF-8 encoding: `normal ` = 7 bytes, `⌄` = 3 bytes, `dimmed` = 6 bytes, `⌄` = 3 bytes, ` after` = 6 bytes. Total = 25 bytes.

Corrected tree:

```text
Root
  Paragraph [0..25]
    Text "normal " [0..7]
    Emphasis { dim: true } [7..19]
      Text "dimmed" [10..16]
    Text " after" [19..25]
```

#### Fixture 8: Dim — With Emphasis

```markdown
⌄*dim and italic*⌄
```

Expected tree:

```text
Root
  Paragraph
    Emphasis { dim: true }
      Emphasis { italic: true }
        Text "dim and italic"
```

Key assertion: dim and italic stack. The outer container carries `dim`, the inner carries `italic`.

#### Fixture 9: Dim — Escaped Delimiter

```markdown
not \⌄ dimmed here
```

Expected tree:

```text
Root
  Paragraph
    Text "not ⌄ dimmed here"
```

Key assertion: the escaped `⌄` becomes literal. No dim container.

#### Fixture 10: Dim — Inside Inline Code

```markdown
use `⌄not dim⌄` in code
```

Expected tree:

```text
Root
  Paragraph
    Text "use "
    InlineCode "⌄not dim⌄"
    Text " in code"
```

Key assertion: dim delimiters inside inline code are literal.

#### Fixture 11: HR Attributes — Basic Styled Rule

```markdown
--- { style: waves }
```

Expected tree:

```text
Root
  ThematicBreak [0..19]
    attrs.hints.hr.style = "waves"
```

Key assertions:

- `RuleProcessor` recognizes the paragraph as an HR with attributes.
- The `ThematicBreak` node carries the parsed attrs via `NodeAttrs` hints.
- Provenance is `Generated` (the HR attribute block is a Darkmatter extension).

#### Fixture 12: HR Attributes — Multiple Attributes

```markdown
*** { style: dots, width: "50%", alignment: centered }
```

Expected tree:

```text
Root
  ThematicBreak
    attrs.hints.hr.style = "dots"
    attrs.hints.hr.width = "50%"
    attrs.hints.hr.alignment = "centered"
```

#### Fixture 13: HR Attributes — Plain HR (No Attributes)

```markdown
---
```

Expected tree:

```text
Root
  ThematicBreak [0..3]
```

Key assertion: plain `---` passes through as a standard `ThematicBreak` with `Provenance::Parsed`. No `InlineStyleProcessor` or `RuleProcessor` involvement needed for this case — `pulldown-cmark` emits `Event::Rule` directly.

#### Fixture 14: HR Attributes — Inside Block Quote

```markdown
> --- { style: gradient }
```

Expected tree:

```text
Root
  BlockQuote
    ThematicBreak
      attrs.hints.hr.style = "gradient"
```

Key assertion: `RuleProcessor` transforms the paragraph inside the block quote per the existing spec.

#### Fixture 15: HR Attributes — Inside List Item (Not Transformed)

```markdown
- --- { style: waves }
```

Expected tree:

```text
Root
  List { ordered: false }
    ListItem
      Paragraph
        Text "--- { style: waves }"
```

Key assertion: `RuleProcessor` does not transform HR patterns inside list items. The content remains a plain paragraph.

#### Fixture 16: Mixed Mark and Dim

```markdown
==highlighted and ⌄dim within mark⌄==
```

Expected tree:

```text
Root
  Paragraph
    Span .mark
      Text "highlighted and "
      Emphasis { dim: true }
        Text "dim within mark"
```

Key assertion: mark is the outer container, dim is inner. Both ranges are exact subranges of the source.

#### Fixture 17: Mark in Table Cell

```markdown
| Header |
|--------|
| ==val== |
```

Expected tree:

```text
Root
  Table { align: [None] }
    TableRow
      TableCell
        Text "Header"
    TableRow
      TableCell
        Span .mark
          Text "val"
```

Key assertion: mark works inside table cells. The `InlineStyleProcessor` processes text events regardless of their container context.

### HR Attribute Storage

HR attributes are stored on `NodeAttrs` using a namespaced hint map. A new `HintNamespace::HR` (or reuse `HintNamespace::HINTS`) carries a typed `HorizontalRuleAttrs` struct:

```rust
// In renderable/src/tree/attrs.rs
namespace HR {
    pub fn set_hr_attrs(attrs: &mut NodeAttrs, hr: HorizontalRuleAttrs);
    pub fn hr_attrs(attrs: &NodeAttrs) -> Option<&HorizontalRuleAttrs>;
}
```

This keeps `NodeKind::ThematicBreak` as a bare unit variant. Renderers that understand HR hints apply them; renderers that do not ignore them and render a plain `<hr>` / `---`.

### Migration Path

1. Introduce `SpannedInlineEvent` and the two spanned processor wrappers.
2. Add `ContainerKind::Mark` and `ContainerKind::Dim` to the fold's stack.
3. Wire the fold to consume the spanned processor chain.
4. Add the text fixtures above as parity tests.
5. Verify all existing fold tests still pass (mark/dim/HR are additive — the fold already ignores them).
6. Legacy renderers continue using the non-spanned processors. No behavior change until cutover.
