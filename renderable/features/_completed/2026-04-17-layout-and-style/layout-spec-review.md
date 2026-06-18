# Review: Layout Primitive Spec

## Summary

The spec is directionally right. The current system has three layout contracts
that cannot round-trip through the tree: `renderable::layout::Layout`,
`tree::LayoutHints`, and Darkmatter's page layout types. Promoting one typed
`Layout` onto block `RenderNode`s is the correct architectural center of
gravity, and the "components declare layout, renderers apply layout" rule is
the right boundary.

The main risk is that the spec currently settles a large API shape without
pinning down several details that will determine whether the migration is
clean or whether another compatibility layer appears immediately. I would
proceed, but tighten the points below before implementation.

## Feedback

### 1. Define `Length` Precisely

The unit model is the most consequential part of the spec, but `Length` is not
specified as a concrete type. The implementation needs to know:

- whether values are `f32`, `f64`, integer cells, or a fixed decimal type;
- whether negative lengths are valid;
- whether percentages are stored as `0.0..=1.0` or `0.0..=100.0`;
- how `0` is represented independently of unit;
- how invalid values such as `NaN`, infinity, or `101%` are rejected.

Suggestion: add a settled `Length` sketch before implementation. For example,
`Length::zero()`, `Length::ch(u32)`, `Length::percent(f32)`, and
`Length::css(CssLength)` for target-native browser values. If fractional `ch`
is allowed for Browser but not Terminal, that should only be expressible in the
per-target branch.

### 2. Keep Terminal Resolution Explicitly Integer

The spec says `Margin::resolve` turns lengths into concrete values for a target
and available width. For Terminal, that concrete value should be a cell count,
and the rounding policy must be part of the contract.

Current `Layout::resolve_margin` rounds percentages. If the new implementation
changes to floor or ceil, existing visual output may drift. Pick the policy
explicitly and test it, especially for small widths and asymmetric margins.

Suggestion: define terminal resolution as:

- `ch` resolves to whole cells;
- `%` resolves against the current available width, using a named rounding
  rule;
- overflow saturates rather than panics;
- vertical margins resolve to line counts, not CSS-like physical lengths.

### 3. Reconsider Vertical `ch`

Universal `ch` is reasonable for horizontal lengths, but it is awkward for top
and bottom margins. A terminal top margin of `2ch` actually means two lines,
while CSS `margin-top: 2ch` is based on glyph width, not line height.

The spec mentions that Browser may translate vertical `ch` to `lh`, but that
creates target-specific behavior inside the supposedly universal form.

Suggestion: either make that translation a requirement, or add a universal
`Line` / `lh`-like unit for vertical layout. If the desired author model is
"two blank rows in terminal and two line-heights in browser," name that unit
directly.

### 4. Separate Block Layout From Inline Node Kinds in Validation

The spec says inline components carry no `Layout`, but it should also say how
the tree enforces that. Because `NodeAttrs` can attach to every `RenderNode`,
without validation a `Span`, `Text`, or `InlineCode` can accidentally carry a
layout and each renderer will need to decide whether to ignore it.

Suggestion: add a tree validation rule: layout attributes are allowed only on
block-level nodes. Under Warn/Lossy, renderers may ignore invalid inline layout
with a diagnostic; under Strict, validation should fail.

### 5. Specify Layout Inheritance and Composition

The spec says nested block nodes may each carry their own `Layout`, but it does
not define whether a child layout replaces, composes with, or inherits from its
parent. This matters for margins, available width, and `word_wrap`.

Suggestion: make layout non-inherited by default, with only available width
flowing downward after the parent is applied. If any field should inherit
(`word_wrap` is the likely candidate), call that out explicitly. Otherwise,
renderers will diverge.

### 6. Be Careful With `TargetValue<T>` Maps

The per-target escape hatch is useful, but it needs deterministic fallback
rules:

- If the current target is `MarkdownPlus`, does it first check `MarkdownPlus`
  then `Markdown`, or does the enum not include `MarkdownPlus` at all?
- If a per-target map omits the current target, is the property absent or does
  it fall back to the universal default?
- Are empty per-target maps invalid?
- Can a map contain both `Markdown` and `MarkdownPlus`?

The current `renderable::target::RenderTarget` still includes `Ast`; the spec
should explicitly remove or ignore that variant as part of the migration.

### 7. Do Not Leave Markdown Behavior Too Implicit

I agree that plain Markdown body output should stay clean. The risky part is
saying Markdown "carries layout via the `style` frontmatter" while frontmatter
schema is deferred to Spec B and this spec emits nothing.

That creates a temporary state where Markdown silently drops layout by design,
which is acceptable only if tests assert that behavior and the public docs say
Markdown layout requires the future style frontmatter path.

Suggestion: add an explicit acceptance criterion:

- plain Markdown rendering drops `Layout` without diagnostics;
- MarkdownPlus may use HTML only when the dialect opts into it;
- frontmatter emission is not implemented in Spec A.

This prevents an implementer from half-adding frontmatter before Spec B settles
the schema.

### 8. Define Browser Lowering More Exactly

`alignment` lowering is underspecified. Browser block centering and text
alignment are different operations:

- `text-align: center` centers inline content inside the block;
- `margin-left: auto; margin-right: auto` centers the block itself when it has a
  constrained width.

Suggestion: define Browser lowering per field:

- `margin` lowers to CSS margin longhands;
- `max_width` lowers to `max-width`;
- `alignment` lowers to block alignment only when `max_width` is present, and
  to `text-align` only for text-flow nodes if that is intended;
- `word_wrap` lowers to a small, explicit CSS mapping or is ignored with a
  documented reason.

### 9. Preserve Existing Terminal Extension Ergonomics

Today `biscuit-terminal` owns `LayoutTerminalExt` and many components use
`layout.apply_layout` / `apply_block_layout`. The spec says Terminal tree
rendering absorbs page decoration behavior, but it does not say what happens to
those existing component APIs.

Suggestion: add a migration rule:

- `renderable::layout::Layout` remains the shared data type;
- terminal-only layout application stays in `biscuit-terminal`;
- the extension trait adapts to the new field names and `TargetValue` model;
- bespoke terminal renderers keep working during migration.

That will reduce churn outside the seven tree-migrated components.

### 10. Avoid a Big-Bang Darkmatter Type Deletion

Deleting `PageMargin`, `PagePadding`, `PageFill`, and `PageAlignment` may be
right eventually, but the spec should say whether this is an internal-only
break or a public API break. If these are public Darkmatter types, the migration
should include deprecation aliases or conversion impls unless the package is
comfortable with a hard breaking change.

Suggestion: add a compatibility subsection:

- identify which removed types are public;
- decide whether to provide `From`/`TryFrom` conversions;
- update frontmatter parsing separately from render-tree layout application.

### 11. Add Serialization Requirements

Because `NodeAttrs` serializes with the tree, `Layout`, `Margin`,
`TargetValue`, and `Length` should have stable serde shapes. Without that, tree
snapshots and future tooling will become brittle.

Suggestion: include a sample serialized layout in the spec. That will force
decisions about unit spelling, enum casing, and per-target map keys before
implementation.

### 12. Add Tests Around the Actual Drift Modes

The success criteria mention burning down layout-related drift, but the test
shape should be more concrete.

Suggestion: require focused tests for:

- margins on `Section`, lists, `Table`, `Progress`, `TwoColumn`, and
  `YamlBlock`;
- alignment with and without `max_width`;
- percentage margins at several terminal widths;
- a per-target browser-only value that does not affect terminal output;
- invalid universal `px` / `em` / `rem` values;
- Markdown body output remaining unchanged when layout is present.

## Suggested Spec Edits Before Implementation

- Add concrete definitions for `Length`, `TargetValue<T>`, and target fallback
  behavior.
- Specify terminal rounding, saturation, and vertical-unit semantics.
- Add validation rules for layout on block nodes only.
- Define parent/child layout composition.
- Tighten Browser lowering rules for `alignment`, `max_width`, and
  `word_wrap`.
- Clarify that Spec A does not emit Markdown frontmatter.
- Add a compatibility plan for existing `biscuit-terminal` layout extension
  APIs and public Darkmatter page-layout types.
- Include a serde example for a node carrying layout.

## Verdict

Proceed after tightening the above details. The core architectural move is the
right one, but the unit model and renderer contracts need to be precise before
code changes begin; otherwise the migration will likely replace three layout
models with one type plus several undocumented renderer-specific behaviors.
