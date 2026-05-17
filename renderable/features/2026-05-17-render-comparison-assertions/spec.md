# Spec: Render Comparison Assertions

**Date:** 2026-05-17
**Status:** Approved
**Crates:** `biscuit-terminal`, `darkmatter`

## Problem

The render-tree engine produces output that drifts from the established bespoke
renderer for the seven components on the render-tree architecture (Section,
UnorderedList, TwoColumn, Progress, Table, BlockQuote in `biscuit-terminal`;
YamlBlock in `darkmatter`). The `layout_matrix` snapshot suite records that
drift but does not assert *correctness* — its snapshots pass with every bug
baked in, because a snapshot only asserts "output unchanged since last run."

There is currently no test that encodes the *correct* expected behavior and
fails because of a bug. As a result the bugs are invisible to `cargo test`:
every suite is green while the new engine is materially wrong.

## Goal

An assertion-based test suite that compares the new tree-rendering engine's
output against the bespoke renderer (the oracle) for every component under
every layout scenario, across the **complete** output — visible text, layout,
and ANSI styling. The suite genuinely asserts `tree == bespoke`. Known drift is
tracked in an expected-failures ledger so CI stays green while the ledger gives
an exact, self-maintaining count of remaining drift.

## Non-Goals

- Fixing any rendering drift. This suite *detects and tracks* drift; fixes are
  separate work driven by the ledger.
- Replacing the `layout_matrix` snapshot suite or harness. They coexist.
- Browser or Markdown render targets. Terminal only.
- Permanence. This suite is temporary scaffolding (see Exit Condition).

## Oracle

The bespoke `TerminalRenderable::render()` output is the oracle — treated as
correct. Each comparison asserts the **tree path equals the bespoke path**.

This differs fundamentally from the `layout_matrix` snapshots: those assert
"output unchanged"; this asserts "output correct." The `KNOWN_DRIFT` ledger is
purely XFAIL bookkeeping — it never makes drift acceptable. It records "this
bug is already known; do not report it as news, but DO report when it is
fixed."

## Facets

For each (component, scenario) pair the suite compares the bespoke and tree
outputs across six **facets**, ordered from catch-all to diagnostic. The
catch-all guarantees completeness; the five sub-facets localize which dimension
broke.

| Facet | Definition | Catches |
|-------|------------|---------|
| `exact` | Full output, ANSI bytes included, byte-exact `String` equality | Any difference whatsoever |
| `text` | ANSI-stripped visible text, exact equality | Layout + content drift, ignoring color |
| `indent` | Per-line count of leading visible spaces (`Vec<usize>`) | Left margin, centering, right alignment |
| `blank_lines` | Indices of blank lines within the output (`Vec<usize>`) | Top/bottom margin, inter-block spacing |
| `width` | Maximum visible line width — ANSI-stripped `.chars().count()` (`usize`) | Right margin, wrapping, width handling |
| `styling` | Ordered list of `(visible_offset, sgr_sequence)` pairs (`Vec<(usize, String)>`) | Color, bold, dim drift |

A facet **drifts** for a (component, scenario) when its extracted value differs
between the bespoke and tree outputs. `exact` is computed by direct string
comparison, never inferred from the sub-facets; it necessarily drifts whenever
a sub-facet does, and additionally catches differences — trailing whitespace,
ANSI ordering and position — that no sub-facet isolates. The sub-facets exist
to attribute `exact` drift to a dimension.

### Extraction semantics

Facet extraction operates on the rendered strings only — pure string analysis,
no component types.

- **Line splitting.** Extraction splits on `\n` (`str::split('\n')`), *not*
  `str::lines()`. `lines()` discards the empty final element produced by a
  trailing newline, which would hide top/bottom-margin and trailing-blank-line
  drift. The element after a trailing `\n` is a real, comparable line.
- **ANSI stripping.** Facets that work on visible text (`text`, `indent`,
  `blank_lines`, `width`) strip ANSI with the existing
  `biscuit_terminal::prelude::strip_escape_codes` helper — the same stripper the
  rest of the terminal test suite uses. No second ANSI stripper is introduced.
- **Width.** `width` is the ANSI-stripped `.chars().count()` of the widest
  line. The current scenario matrix is ASCII-only, so code-point count equals
  display width. If wide-Unicode scenarios are added later this must move to a
  terminal display-width helper.
- **Indent.** A leading space is U+0020; `indent` counts leading U+0020 on the
  ANSI-stripped line.
- **`styling` precision.** `styling` records `(visible_offset, sgr_sequence)`
  pairs — each SGR sequence and the visible-character offset at which it
  appears. This localizes color/emphasis drift better than a bare sequence
  list, but it still cannot prove two equal-offset SGRs style identical glyphs
  if surrounding text shifted. That residual case is always caught by `exact`;
  `styling` is a diagnostic aid, not an independent correctness guarantee.

## The Drift Ledger

Each crate's test file carries a committed `KNOWN_DRIFT` constant — a list of
`(component, scenario, facet)` triples, one per known-drifting facet:

```rust
const KNOWN_DRIFT: &[(&str, &str, Facet)] = &[
    ("Section", "left_margin_4", Facet::Indent),
    ("Section", "left_margin_4", Facet::Text),
    ("Section", "left_margin_4", Facet::Exact),
    // ... one entry per known-drifting (component, scenario, facet)
];
```

`component` and `scenario` are the `name` fields already defined by
`layout_matrix_support`. `Facet` is an enum (`Exact`, `Text`, `Indent`,
`BlankLines`, `Width`, `Styling`) deriving `Debug`, `Clone`, `Copy`, `PartialEq`,
`Eq`, `Ord`, `PartialOrd`.

The ledger length is the **remaining drift count** for that crate — the number
of known drifting `(component, scenario, facet)` observations, *not* the number
of underlying engine bugs. One engine bug typically produces several entries:
every diagnostic facet it disturbs, plus the `exact` facet. The ledger is the
drift backlog in data form; mapping it to discrete bugs (issue IDs, grouping)
is out of scope for this suite.

### Ledger integrity

`KNOWN_DRIFT` is normalized into a `BTreeSet` of drift keys at the start of the
test. If the constant contains duplicate triples the test panics immediately
(`duplicate KNOWN_DRIFT entry`) — duplicates would silently inflate the drift
count and usually indicate a bad merge or paste. Record mode (below) emits the
literal in deterministic sorted order so committed diffs stay minimal and
stable.

### Bootstrap

The initial ledger is generated, not hand-written. Running the test with the
environment variable `RECORD_DRIFT` set to a truthy value — `1`, `true`, or
`yes` (case-insensitive) — causes the test to print the complete current drift
set as a ready-to-paste `KNOWN_DRIFT` literal, sorted deterministically, and
pass without comparing against the existing ledger. The engineer pastes that
output into the test file and commits it.

Any other value, including `0` or `false`, leaves the test in normal compare
mode, so an inherited `RECORD_DRIFT=0` does not surprise CI or local shells.
Unset behaves the same as a non-truthy value: normal compare mode.

## Test Behavior

One `#[test]` function per crate — `render_matches_bespoke`. It:

1. Iterates every `ComponentCase` × every `Scenario` from `layout_matrix_support`.
2. For each pair, renders bespoke and tree (via the existing
   `(case.render)(&scenario)` which already returns `(bespoke, tree)` with ANSI
   retained).
3. Extracts all six facets from each side and records every drifting
   `(component, scenario, facet)` triple into a live drift set.
4. If `RECORD_DRIFT` is truthy: print the live set as a sorted `KNOWN_DRIFT`
   literal, pass.
5. Otherwise compare the live set against `KNOWN_DRIFT`:
   - **Triples in live but not in the ledger** → panic: `REGRESSION / unrecorded
     drift`, listing each triple. Either a real regression, or new drift that
     must be acknowledged in the ledger.
   - **Triples in the ledger but not live** → panic: `FIXED — remove from
     KNOWN_DRIFT`, listing each triple. The engine now matches bespoke for that
     facet; the ledger entry must be deleted.
   - **Equal** → pass.

Every failure panic opens with a one-line summary —
`live drift: N, known: M, unrecorded: X, fixed: Y` — followed by the offending
triples, so the required ledger edit (or the regression) is unambiguous. For up
to the first five **unrecorded** triples the panic also prints the bespoke and
tree facet values (rendered strings for `exact`/`text` truncated to a
reasonable cap, e.g. 200 chars), so newly introduced drift can be diagnosed
without rerunning under ad-hoc logging.

The test is green precisely when the live drift set equals the ledger — i.e.
when every known bug is still present and no new drift and no fixes have
appeared unrecorded. This keeps CI green while the ledger count tracks reality.

## File Layout

One new file per crate. No new shared support module — the suite reuses the
existing `layout_matrix_support` module (its `Scenario`, `scenarios()`,
`ComponentCase`, `component_cases()` already produce ANSI-retained
`(bespoke, tree)` pairs, which is exactly the input this suite needs).

| File | Responsibility |
|------|----------------|
| `biscuit-terminal/lib/tests/render_comparison.rs` | `mod layout_matrix_support;` reuse; `Facet` enum; six facet extractors; `KNOWN_DRIFT` ledger; `render_matches_bespoke` test (6 components) |
| `darkmatter/lib/tests/render_comparison.rs` | Same, for `YamlBlock` (1 component) |

The facet-extractor logic (~60 lines of pure string analysis) and the `Facet`
enum are duplicated across the two files. This is consistent with the
cross-crate no-test-sharing decision already accepted for `layout_matrix`:
cross-crate test-helper coupling is worse than the duplication.

`layout_matrix_support` currently lives at
`{crate}/lib/tests/layout_matrix_support/mod.rs` (a directory module) and is
included with `mod layout_matrix_support;`.

## Error Handling

- Facet extraction is total: it never panics on any string input (empty
  output, output with no newlines, output that is only ANSI codes all yield
  well-defined facet values).
- The tree path may already return a `<render error: ...>` sentinel string
  (produced by `layout_matrix_support`'s `render_tree_string` on a render
  error). Such a sentinel simply produces drift on the relevant facets and is
  recorded in the ledger like any other drift — it is not special-cased.
- `RECORD_DRIFT` is read once via `std::env::var`; record mode is enabled only
  when the value, lowercased and trimmed, is `1`, `true`, or `yes`. Every other
  value (including `0`, `false`, and unset) leaves the test in compare mode.

## Testing

The suite *is* the test. It is self-validating: with a correct committed
ledger, `cargo test --test render_comparison` passes in both crates.
Additionally:

- Verify `RECORD_DRIFT=1 cargo test -p biscuit-terminal --test render_comparison`
  prints a `KNOWN_DRIFT` literal and passes.
- Verify a plain run after committing that ledger passes.
- Verify that artificially adding a bogus triple to `KNOWN_DRIFT` produces the
  "FIXED — remove from KNOWN_DRIFT" failure, and removing a real triple
  produces the "REGRESSION / unrecorded drift" failure (manual one-off check
  during implementation; not committed).
- Both crates remain `clippy`-clean.

## Implementation Notes

- Represent triples internally as a named `DriftKey { component, scenario,
  facet }` struct deriving `Ord`/`Eq`/`Hash` for `BTreeSet` use. The committed
  `KNOWN_DRIFT` constant may stay tuple-shaped for compact literals; convert
  tuples into `DriftKey`s on load.
- Define an `ALL_FACETS: [Facet; 6]` constant and iterate it everywhere facets
  are enumerated, so record output ordering is stable and a newly added facet
  cannot be silently skipped.
- Compare every facet independently, `exact` included — never derive `exact`
  drift from the sub-facets. `exact` catches trailing-space and ANSI-position
  differences the sub-facets do not localize.
- Panic messages include both counts and the categorized triples so a single
  read of the failure tells the engineer exactly which ledger lines to add or
  remove.

## Exit Condition

This suite and the `layout_matrix` suite are temporary scaffolding for the
render-engine migration. When `KNOWN_DRIFT` is empty in both crates and stays
empty, the tree engine matches the bespoke renderer across every component,
scenario, and facet. At that point confidence is established and both suites
(`render_comparison` and `layout_matrix`, plus their support module and
harness examples) can be retired.
