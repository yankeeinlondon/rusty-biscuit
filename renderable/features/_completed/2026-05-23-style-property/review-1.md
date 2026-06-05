---
ready: false
agent: codex
model: ""
---

# Review: `style:` Frontmatter Sub-Spec #1

## Verdict

Not ready for production.

The implementation covers the broad parser shape: a public `darkmatter::style`
module exists, the fixture parses to the expected typed values, unknown keys are
reported without aborting, and strict mode ignores `KnownButInactive`. However,
there are still correctness and verification gaps against the spec's explicit
acceptance criteria.

## Findings

### High: Nested snake-case aliases under deprecated containers can be reported as `UnknownKey`

Spec requirement: test #8 says `style.block_quote.max_width: 50%` must parse
successfully and emit two `Deprecated` warnings: `block_quote -> block-quote`
and `max_width -> max-width`.

Implementation: `walker::deprecated_container()` maps `block_quote` to
`block-quote`, then recurses with the canonical path:

- `darkmatter/lib/src/style/walker.rs:56-64`
- `darkmatter/lib/src/style/descriptor.rs:50-51`

That means the nested raw key is checked as `block-quote.max_width`, but the
descriptor only recognizes canonical `block-quote.max-width` or alias
`block_quote.max_width`. The mixed path is neither, so the walker emits
`UnknownKey` instead of the required nested `Deprecated` warning.

This same path-shape risk applies to deprecated nested containers like
`hyperlinks.local_style` and `images.local_style` when their children also use
snake-case aliases.

Suggested fix: preserve both raw and canonical traversal paths, or let the
descriptor canonicalize segment-by-segment so a deprecated parent plus a
deprecated child produces two `Deprecated` warnings and no `UnknownKey`.

Verification level: Level 1 is appropriate. Add direct parser tests for
`block_quote.max_width`, `hyperlinks.local_style.max_width`, and
`images.local_style.max_width`.

### High: Typed parse errors do not use the public `StyleParseError` variants required by the spec

Spec requirement: invalid structure, length, percent, and color failures should
surface as `StyleParseError::{Structure, InvalidLength, InvalidPercent,
InvalidColor}`. Test #4 specifically requires `top-margin: 2ch` to fail with
`StyleParseError::Structure`.

Implementation: `from_json_value()` delegates typed parsing directly to
`serde_json::from_value(value.clone())?`, and the docs/tests now expect a
generic serde error:

- `darkmatter/lib/src/style/parse.rs:17-31`
- `darkmatter/lib/src/style/parse.rs:177-183`
- `darkmatter/lib/src/style/error.rs:14-50`

The custom variants are effectively dead outside their own display tests. That
breaks the public API contract and loses the path-specific diagnostic shape
the spec designed.

Suggested fix: either implement explicit path-aware validation before serde, or
make custom field deserializers return an internal typed error that
`from_json_value()` maps into the corresponding `StyleParseError` variant with
the full YAML path.

Verification level: Level 1 is appropriate. Add parser-entry tests matching on
the concrete variants for invalid horizontal length, invalid percent, invalid
vertical row count, invalid color, and non-object `style:`.

### High: Required alias and descriptor coverage tests are incomplete

Spec acceptance requires every documented snake-case key from `style.md` to be
accepted and emit `Deprecated`, and says the descriptor test should prove
coverage of every leaf.

Current tests only sample a few aliases (`page.left_margin`, `max_width`,
`local_style`, `block_quote`) and do not prove descriptor/schema drift is
prevented:

- `darkmatter/lib/src/style/descriptor.rs:141-156` only checks uniqueness.
- `darkmatter/lib/src/style/schema/page.rs:104-106` samples one page alias.
- `darkmatter/lib/src/style/schema/common.rs:57-62` samples one common alias.
- `darkmatter/lib/src/style/schema/inline.rs:44-47` samples container alias
  parsing but not warning behavior.

This gap allowed the `block_quote.max_width` issue above to ship.

Suggested fix: add a table-driven parser test over every `SchemaLeaf::alias`
entry asserting parse success plus one `Deprecated` warning at the raw path and
a `KnownButInactive` warning at the canonical path. Add a descriptor/schema
coverage test, even if initially hand-maintained, that fails when a field is
added without a descriptor entry.

Verification level: Level 1 is appropriate.

### Medium: Color coverage does not meet the specified matrix

Spec test #2 requires every Tailwind family x level combination to parse, plus
opacity bounds, hex, alpha hex, invalid hex, and web-name routing.

Current color tests cover representative values (`red-500`, opacity,
specials, hex, `orange`) but not the full Tailwind matrix:

- `darkmatter/lib/src/style/color.rs:489-591`

The implementation uses a large manual match table, which is exactly the sort
of code that needs exhaustive tests against the source enum.

Suggested fix: add a table that exercises every supported family and level,
including multi-family neutral colors (`slate`, `zinc`, `neutral`, `stone`),
and assert all parse to the expected `renderable::color::Tailwind` variants.

Verification level: Level 1 is appropriate.

## Verification Level Assessment

This sub-spec is parser/schema only. It explicitly excludes render-pipeline
integration and CLI behavior, so Level 1 unit/integration tests are sufficient
for all user-observable requirements in this slice.

No Level 2 or Level 3 terminal verification is required until later sub-specs
wire these parsed values into terminal/browser rendering or keyboard/terminal
interaction.

## What Looks Implemented

- Public `darkmatter::style::{from_frontmatter, from_json_value, into_strict}`
  exists.
- The example fixture parses to the acceptance-criteria values in the
  integration test.
- Unknown keys are accumulated instead of aborting immediately.
- `KnownButInactive` is emitted for known leaves and ignored by `into_strict`.
- The schema stores lengths as `renderable::layout::Length`, alignments as
  `renderable::layout::Alignment`, and colors as `renderable::color::Color`
  inside `StyleColor`.

## Production Readiness

`ready: false`

The parser is close, but the documented alias behavior and concrete error API
are part of sub-spec #1's public contract. Those should be fixed and covered
before this feature is considered production-ready.
