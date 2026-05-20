# Review: Render Comparison Assertions

## Summary

The spec is sound and should produce a useful migration safety net. Reusing the
existing `layout_matrix_support` modules is the right scope: those modules
already provide the stable component/scenario identifiers and ANSI-retained
`(bespoke, tree)` output the comparison suite needs.

The strongest part of the design is the live drift set compared against a
committed ledger. That gives CI a deterministic contract while making both new
drift and fixed drift visible.

## Feedback

### Clarify What the Ledger Count Means

The spec says the ledger length is the remaining-bug count. Because entries are
`(component, scenario, facet)` triples, the count is really the number of known
drifting facet observations. One underlying engine bug can produce many ledger
entries, especially because `Facet::Exact` is recorded in addition to every
diagnostic facet.

Suggestion: rename that language to "remaining drift count" or "known drift
facet count." If a true bug count is desired, it should be tracked separately
with issue IDs or grouped annotations.

### Define Line Splitting Semantics

The facet definitions depend on line boundaries, but Rust's `str::lines()`
drops the final empty line created by a trailing newline. That matters for
`blank_lines`, and possibly for top/bottom margin regressions.

Suggestion: explicitly define whether facet extraction uses `lines()` semantics
or a split that preserves trailing empty lines. For a render-comparison suite,
preserving line structure is safer:

```rust
fn logical_lines(s: &str) -> Vec<&str> {
    s.split('\n').collect()
}
```

If the final trailing newline should not count as a blank rendered row, say that
explicitly and trim only that case.

### Make ANSI Parsing Deliberate

The `styling` facet is defined as the ordered sequence of SGR escape sequences.
That is useful, but it has two limitations:

- It does not record where each SGR sequence appears in the visible output.
- Equal SGR sequences can still style different text if layout or text shifts.

`Facet::Exact` catches those cases, so this is not a correctness hole. It is a
diagnostic limitation.

Suggestion: either call this out in the spec, or make the `styling` facet return
visible offsets with the SGR sequence, such as `Vec<(usize, String)>`. If the
goal is only coarse attribution, the current `Vec<String>` is acceptable.

### Use the Existing ANSI Stripper

The support modules already import `biscuit_terminal::prelude::strip_escape_codes`.
The spec says the extractor logic is pure string analysis, but does not say how
ANSI stripping is performed.

Suggestion: require using the existing `strip_escape_codes` helper rather than
introducing a second ANSI stripper inside these tests. It keeps test behavior
aligned with the rest of the terminal test suite.

### Validate the Ledger Itself

The spec compares live drift against `KNOWN_DRIFT`, but it does not mention
guardrails for the ledger data.

Suggestion: normalize `KNOWN_DRIFT` into a `BTreeSet` and fail if duplicates are
present. Also consider requiring deterministic sorted output from record mode
and preserving that order in the committed constant. That makes review diffs
stable and prevents duplicate entries from inflating the drift count.

### Reconsider `RECORD_DRIFT` Enablement

The spec says any value other than unset enables record mode. That means
`RECORD_DRIFT=0` records drift instead of testing, which is surprising in CI or
local shells.

Suggestion: enable record mode only for clear truthy values such as `1`, `true`,
or `yes`. If the implementation keeps "any value means enabled," mention
`RECORD_DRIFT=0` explicitly in the docs or failure output.

### Add Failure Context for First-Class Debugging

The spec requires failures to list offending triples, which is necessary. For
fast debugging, it may also be worth printing a compact diff for the first few
unrecorded triples, especially for `Facet::Exact` and `Facet::Text`.

Suggestion: keep the ledger edit instructions concise, but include the bespoke
and tree facet values for a small capped number of failures. This avoids forcing
the engineer to rerun with ad hoc logging to understand a newly introduced
drift.

### Be Precise About Width

The spec defines `width` as maximum visible line width in `usize`, but "visible
width" can mean character count or terminal display-cell width. The current
matrix samples appear ASCII-only, so either choice works today.

Suggestion: define this as "ANSI-stripped `.chars().count()`" if that is the
intended simple behavior. If future cases include wide Unicode, use a terminal
display-width helper instead.

## Suggested Implementation Notes

- Use a small `DriftKey` struct instead of raw tuples internally. The committed
  constant can remain tuple-shaped, but a named type makes formatter and set
  code easier to read.
- Keep `Facet` order explicit with an `ALL_FACETS` constant so record output is
  stable and future facets are hard to forget.
- Compare every facet independently, including `Exact`; do not infer `Exact`
  from sub-facets. `Exact` can catch trailing-space and ANSI-position
  differences that the sub-facets may not localize.
- Consider including the live and known counts in panic messages:
  `live drift: N, known drift: M, unrecorded: X, fixed: Y`.

## Verdict

Proceed with the spec after tightening the wording around ledger counts,
line-splitting behavior, ANSI stripping, and `RECORD_DRIFT` semantics. Those are
small clarifications, not architectural objections.
