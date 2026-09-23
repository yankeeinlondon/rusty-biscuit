---
area: darkmatter
status: draft
created: 2026-09-20
owner: Ken Snyder <ken@ken.net>
origin: merge of `main` into `feat/dark-fixes`, 2026-09-20 (resolution "A" for `dmls/src/providers/frontmatter.rs`)
packages:
    - dmls
---

# Port alias and block-scalar analysis onto `ScalarProjection`

## Outcome

DMLS again analyzes an aliased frontmatter expression once, in the scalar
that defines it, and again places unknown-identifier findings inside block,
literal, and folded scalars at their own ranges — on top of `main`'s
`ScalarProjection` / `FmPathSegment` design rather than the decoder that
design replaced. Six ignored tests pass again and two removed ones are
re-created.

## Why this exists

`feat/dark-fixes` and `main` each rebuilt `ExpressionValue` in
`darkmatter/dmls/src/providers/frontmatter.rs`, on different foundations:

- **`feat/dark-fixes`** added YAML anchor/alias analysis. An alias shared one
  `Arc<DecodedScalar>` with its definition, so N aliases of one expression cost
  one parse and one decode; an alias whose anchor was redefined, or whose
  definition the decoder could not reproduce, fell back to an `AliasToken`
  whose every range collapsed onto the alias token. The decoder also produced
  a decoded→authored byte map for block scalars, so a finding inside a
  `|`/`>` value landed on its own bytes. This was built on `DecodedScalar`,
  `decode_scalar_node`, and `decode_alias_definition`.
- **`main`** (`feat/better-static-analysis`) deleted that decoder and replaced
  it with `expressions::ScalarProjection::for_value(entry, text)`: an exact
  map for untagged single-line plain or quoted scalars, and a deliberate
  whole-scalar range for block, tagged, and multi-line values. It also rewrote
  schema-shape navigation around `FmPathSegment` (keys *and* sequence
  indices) with an `ArrayCrossing` memo, and gave `ExpressionValue` an arena
  `index` for `FrontmatterAst::path_at`.

`main`'s side is load-bearing for `dsl.rs`, `nested_span.rs`, and the merged
`diagnostics/frontmatter.rs`, so the merge took it whole. Our alias analysis
could not be merged in; it has to be re-implemented on the new foundation.
That is this fix.

## What already survived the merge

Do not rebuild these; build on them.

- **Lowering.** `overlay/frontmatter.rs` records every anchor once, in
  document order, *after* the node's own alias lookup, so an alias sees
  exactly the definitions authored before it (`AnchorDefinition`,
  `record_anchor`, `record_anchors` for key nodes). Every `FmEntry` carries
  `alias_target: Option<Arc<str>>` (the defining scalar's text, shared by
  every alias of it) and `alias_target_start: Option<usize>` (the document
  offset of the defining scalar after its tag and anchor; `None` when the
  definition is a collection or the anchor was redefined before the alias).
  This is threaded through `main`'s `Parent` / `lower_entry` structure,
  including sequence descent.
- **Diagnostics scaffolding.** `diagnostics/frontmatter.rs::expression_diagnostics`
  keeps the alias dedup: `settled` and `unreported_errors` keyed on
  `value.expression_span()`, so whichever Expression-typed property reaches
  an anchored value first settles it and a later alias with the same span is
  skipped, and a malformed value only a union arm accepted stays unsettled for
  a later alias whose property rejects it. Today the keys never collide,
  because `main`'s `expression_values()` skips `FmValueKind::Alias` entries
  (`providers/frontmatter.rs`, the `entry.kind != FmValueKind::Scalar`
  guard); the port makes them collide again.
- **Whole-scalar fallback.** A finding in an inexact value now lands on
  `value.expression_span()` instead of being dropped, and the fix is
  withheld because the texts cannot match. Keep the warning; the port only
  restores precision.

## Scope

All production changes are in `darkmatter/dmls/src/`.

1. **Aliases are expression values again.** `expression_values()` admits
   `FmValueKind::Alias` entries whose `alias_target` is `Some`. Their
   expression text is `alias_target`; there is no re-decode per alias.
2. **A projection for an alias.** Give `ScalarProjection` (or a sibling that
   `ExpressionValue` holds by `Arc`) a constructor from a definition:
   `for_value` applied to the *defining* scalar located by
   `alias_target_start`, shared by every alias of that definition. Locate the
   defining `FmEntry` through the arena rather than by re-scanning text —
   the lowering guarantees the definition was lowered before any alias of it.
   The `AliasToken` fallback is the case `alias_target_start` is `None`: an
   inexact projection whose `range`/`project` collapse onto the alias entry's
   own `value_span`, the one place the value is certainly referenced.
3. **`is_exact` has one meaning.** A value is exact iff it is not an
   alias-token fallback *and* its projection is exact. Both `main`'s meaning
   (scalar style) and ours (not a token) were called `is_exact`; the merged
   predicate must be their conjunction, and the doc comment must say so.
4. **Block-scalar precision.** Restore a decoded→authored map for `|` and
   `>` scalars so a finding inside one projects to its own bytes. This is
   optional relative to the alias work and may be split out; if it is,
   `unknown_identifiers_in_block_and_multi_line_expression_values_warn_at_their_own_ranges`
   stays ignored and the split is recorded here. `main`'s choice to give
   block values a whole-scalar range for the *malformed* diagnostic is
   compatible with either outcome.
5. **The dash-separated-key fix on an alias.** `KeyReferenceFix` is an edit
   of authored bytes. It is offered only when the projected span's text equals
   the expression's; on an alias token, or across a YAML escape, it is
   withheld. The existing check in `expression_diagnostics` already does this
   once projections are real.

Out of scope: `main`'s `FmPathSegment` navigation, `ArrayCrossing`,
`nested_span`, and every caller outside `dmls`.

## Tests

Re-enable, by removing the `#[ignore]` whose reason names this fix:

- `dmls/src/diagnostics/frontmatter.rs::a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias`
- `dmls/tests/lsp_session.rs::a_dash_separated_key_behind_an_unprovable_alias_offers_no_fix`
- `dmls/tests/lsp_session.rs::an_aliased_dash_separated_key_fix_avoids_the_defining_scalars_quote`
- `dmls/tests/lsp_session.rs::aliases_are_analyzed_in_the_definition_the_yaml_parser_resolved`
- `dmls/tests/lsp_session.rs::tagged_anchored_and_aliased_expression_values_are_diagnosed_once_where_authored`
- `dmls/tests/lsp_session.rs::unknown_identifiers_in_block_and_multi_line_expression_values_warn_at_their_own_ranges`
  (only if item 4 lands here)

Re-create, in `dmls/src/diagnostics/frontmatter.rs`. Both were removed in the
merge because they read counters that lived in the deleted providers file;
the counters come back with the port:

- `distinct_expression_aliases_publish_without_searching_the_document` —
  publishing a document with N distinct aliased expressions performs one
  expression-value set build (`EXPRESSION_VALUE_SETS`) and finds every
  definition through the YAML tree, never by searching source text: at
  N ∈ {150, 300} the work is linear, not ~N²/2 lines twice.
- `many_aliases_of_one_long_expression_share_its_text_and_one_decode` — 200
  aliases of one long expression (100 and 400 terms) share its text and cost
  one decode (`ALIAS_TARGET_DECODES`) and one parse
  (`EXPRESSION_DIAGNOSTIC_PARSES`).

Pin these too:

- `main`'s `diagnostics/frontmatter/severity_tests.rs` and
  `nested_span_tests.rs` stay green unchanged: the port must not move a
  severity or a nested-span range.
- `dmls/tests/mapping_only_corpus.rs` stays green **without** re-blessing.
  The corpus documents are alias-free, so a baseline change means the port
  touched something it should not have.

## Acceptance criteria

1. Every test above passes; the six `#[ignore]` attributes are gone.
2. Two aliases of one anchored expression produce one parse, one decode, and
   one diagnostic, placed in the defining scalar.
3. An alias of a redefined anchor, or of a collection, is diagnosed on its
   own token with no fix offered.
4. `just test` and `just lint` pass in `darkmatter/`.
5. The `mapping_only_corpus` baseline is byte-identical before and after.

## Notes

- The merge's verification record is the resolution log for the
  `feat/dark-fixes` ↔ `main` merge, 2026-09-20. It lists the four dmls
  files merged by hand and why this one could not be.
- `overlay/frontmatter.rs` populates `alias_target` / `alias_target_start`
  today and nothing reads them. If `just lint` ever flags them as dead under
  a future toolchain, the answer is this fix, not removing the fields.
