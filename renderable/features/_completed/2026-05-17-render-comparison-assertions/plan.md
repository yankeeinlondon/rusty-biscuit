# Implementation Plan: Render Comparison Assertions

**Derived from:** `spec.md` (approved 2026-05-17)  
**Scope:** Two new integration-test files, no production code changes.

---

## 1. Understand the Existing Harness

Both target crates already contain `layout_matrix_support/mod.rs` that provides exactly what this suite needs:

| Symbol | Type | Purpose |
|--------|------|---------|
| `Scenario` | struct `{ name: &'static str, layout: Layout, width: u32 }` | One matrix cell |
| `scenarios()` | `Vec<Scenario>` | 12 scenarios (baseline, margins, alignment, widths, etc.) |
| `ComponentCase` | struct `{ name: &'static str, render: RenderFn }` | One component under test |
| `component_cases()` | `Vec<ComponentCase>` | 6 components (biscuit-terminal) or 1 (darkmatter) |
| `(case.render)(&scenario)` | `(String, String)` | `(bespoke, tree)`, **ANSI retained** |

The render function already handles tree-render errors by returning `<render error: …>` sentinel strings. No error handling changes required.

---

## 2. Shared Types (duplicated in both test files)

### 2.1 `Facet` enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Facet {
    Exact,
    Text,
    Indent,
    BlankLines,
    Width,
    Styling,
}
```

Derive `Ord` so `DriftKey` can derive it. Order listed above is the required stable ordering.

### 2.2 `ALL_FACETS` constant

```rust
const ALL_FACETS: [Facet; 6] = [
    Facet::Exact,
    Facet::Text,
    Facet::Indent,
    Facet::BlankLines,
    Facet::Width,
    Facet::Styling,
];
```

Used everywhere facets are enumerated; guarantees record-mode output order and prevents skipping a facet.

### 2.3 `DriftKey` struct

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DriftKey {
    component: &'static str,
    scenario: &'static str,
    facet: Facet,
}
```

Internal representation only. Convert from the tuple-shaped `KNOWN_DRIFT` entries on load.

---

## 3. Facet Extractors (pure string analysis)

All six extractors are total (never panic) and operate on `&str`. They are **duplicated** in both test files per the cross-crate no-test-sharing decision.

### 3.1 `extract_exact(a: &str, b: &str) -> bool`
Direct byte-level `a == b`.

### 3.2 `extract_text(a: &str, b: &str) -> bool`
```rust
strip_escape_codes(a) == strip_escape_codes(b)
```
Use the existing `biscuit_terminal::prelude::strip_escape_codes`; no second stripper.

### 3.3 `extract_indent(a: &str, b: &str) -> bool`
Split on `'\n'` (not `lines()`). For each line, count leading `U+0020` after ANSI strip. Compare the two `Vec<usize>`.

### 3.4 `extract_blank_lines(a: &str, b: &str) -> bool`
Split on `'\n'`. Collect 0-based indices of lines where the ANSI-stripped line is empty. Compare the two `Vec<usize>`.

### 3.5 `extract_width(a: &str, b: &str) -> bool`
ANSI-strip, split on `'\n'`, take `.chars().count()` of each line, keep the max. Compare the two `usize` values.

### 3.6 `extract_styling(a: &str, b: &str) -> bool`
Scan for SGR sequences (`\x1b[` … `m`). For each match, record `(visible_offset, sequence)` where `visible_offset` is the count of non-ANSI bytes seen so far. Compare the two `Vec<(usize, String)>` vectors.

> **Note:** `visible_offset` is char-count since the current matrix is ASCII-only. This is the precision improvement requested in the review.

---

## 4. Drift Ledger

### 4.1 `KNOWN_DRIFT` constant shape

```rust
const KNOWN_DRIFT: &[(&str, &str, Facet)] = &[
    // ("ComponentName", "scenario_name", Facet::Exact),
];
```

Initially empty in both crates. The first run will be in `RECORD_DRIFT=1` mode to populate it.

### 4.2 Ledger loading & integrity check

```rust
let known: BTreeSet<DriftKey> = KNOWN_DRIFT
    .iter()
    .map(|(c, s, f)| DriftKey { component: c, scenario: s, facet: *f })
    .collect();

assert_eq!(
    known.len(),
    KNOWN_DRIFT.len(),
    "duplicate KNOWN_DRIFT entry"
);
```

Panics immediately if the committed constant contains duplicates.

---

## 5. Test Function: `render_matches_bespoke`

One `#[test]` per crate.

### 5.1 Build the live drift set

```rust
let mut live = BTreeSet::<DriftKey>::new();

for case in component_cases() {
    for scenario in scenarios() {
        let (bespoke, tree) = (case.render)(&scenario);
        for facet in ALL_FACETS {
            let drifts = match facet {
                Facet::Exact      => !extract_exact(&bespoke, &tree),
                Facet::Text       => !extract_text(&bespoke, &tree),
                Facet::Indent     => !extract_indent(&bespoke, &tree),
                Facet::BlankLines => !extract_blank_lines(&bespoke, &tree),
                Facet::Width      => !extract_width(&bespoke, &tree),
                Facet::Styling    => !extract_styling(&bespoke, &tree),
            };
            if drifts {
                live.insert(DriftKey {
                    component: case.name,
                    scenario: scenario.name,
                    facet,
                });
            }
        }
    }
}
```

### 5.2 `RECORD_DRIFT` mode

Read `std::env::var("RECORD_DRIFT")` once. Lowercase + trim. Enable record mode **only** for `1`, `true`, or `yes`. Every other value (unset, `0`, `false`) → normal compare mode.

In record mode:
1. Print the live set as a sorted `KNOWN_DRIFT` literal (deterministic order thanks to `BTreeSet` + `ALL_FACETS` ordering).
2. `return;` (pass).

### 5.3 Compare mode

```rust
let unrecorded: Vec<_> = live.difference(&known).collect();
let fixed: Vec<_> = known.difference(&live).collect();
```

**If `unrecorded` or `fixed` is non-empty:**

Panic with a one-line summary first:
```
live drift: {N}, known: {M}, unrecorded: {X}, fixed: {Y}
```

Then list categorized triples:
- `REGRESSION / unrecorded drift:` followed by each unrecorded `DriftKey` (up to 5, with facet values for `Exact`/`Text` truncated to ~200 chars).
- `FIXED — remove from KNOWN_DRIFT:` followed by each fixed `DriftKey`.

**If both are empty:** pass silently.

---

## 6. File Layout

| Path | Contents |
|------|----------|
| `biscuit-terminal/lib/tests/render_comparison.rs` | `mod layout_matrix_support;`, `Facet`, `DriftKey`, `ALL_FACETS`, 6 extractors, `KNOWN_DRIFT`, `render_matches_bespoke` |
| `darkmatter/lib/tests/render_comparison.rs` | Same structure, but only `YamlBlock` in `component_cases()` |

No new shared modules. Facet logic (~60 lines) is duplicated per the accepted convention.

---

## 7. Bootstrap Procedure

1. Create both files with empty `KNOWN_DRIFT`.
2. Run `RECORD_DRIFT=1 cargo test -p biscuit-terminal --test render_comparison` → copy printed literal into the file.
3. Run `RECORD_DRIFT=1 cargo test -p darkmatter --test render_comparison` → copy printed literal into the file.
4. Commit.
5. Verify plain `cargo test --test render_comparison` passes in both crates.

---

## 8. Verification Checklist

- [ ] `cargo test -p biscuit-terminal --test render_comparison` passes after committing ledger.
- [ ] `cargo test -p darkmatter --test render_comparison` passes after committing ledger.
- [ ] `RECORD_DRIFT=1 cargo test -p biscuit-terminal --test render_comparison` prints a sorted literal and passes.
- [ ] `RECORD_DRIFT=1 cargo test -p darkmatter --test render_comparison` prints a sorted literal and passes.
- [ ] Adding a bogus triple to `KNOWN_DRIFT` panics with `FIXED — remove from KNOWN_DRIFT`.
- [ ] Removing a real triple from `KNOWN_DRIFT` panics with `REGRESSION / unrecorded drift`.
- [ ] Both crates remain `clippy`-clean.

---

## 9. Exit Condition

When `KNOWN_DRIFT` is empty in both crates and stays empty, the tree engine matches the bespoke renderer. Both `render_comparison` and `layout_matrix` suites can then be retired.
