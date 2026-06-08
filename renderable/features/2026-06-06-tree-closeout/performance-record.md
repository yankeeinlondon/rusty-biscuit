---
status: complete
date: 2026-06-07
owner: ken
spec: renderable/features/2026-06-06-tree-closeout/spec.md
phase: 3
---

# Post-Cutover Performance Record

Trend record for the production render-tree path after the CSS Box Architecture
cutover (closeout spec section 6, acceptance criterion 7). The **authoritative
gate is structural, not wall-clock**: the styled corpus must fold with zero
first-class extension-bag access (which also entails zero per-node formatted hint
keys, since those are only produced inside the bag accessors). The third spec
property — zero typed-attr serde round-trips — is **not** directly measured by the
counter; the gate is a strong backstop for it (the likely regression re-routes a
hint through the serializing `data` bag and trips the counter), but a direct
typed-attr serialization that bypassed the bag would not be caught. See the
`structural_gate` module doc for the precise derivation. Timings here exist for
trend visibility only — no flaky wall-clock threshold is an acceptance gate.

## Structural gate (authoritative)

| Gate | Location | Result |
|---|---|---|
| Terminal fold, zero `renderable.*` bag round-trips | `biscuit-terminal/lib/tests/perf_gate.rs::terminal_fold_does_zero_renderable_owned_hint_roundtrips` | **pass** |
| Counter liveness (anti-vacuity) | `biscuit-terminal/lib/tests/perf_gate.rs::hint_access_counter_is_live_in_this_build` | **pass** |
| Styled terminal path, zero bag round-trips (both counters) | `darkmatter` `structural_gate::styled_terminal_path_does_zero_renderable_owned_hint_roundtrips` | **pass** |
| Styled browser path, zero bag round-trips | `darkmatter` `structural_gate::styled_browser_path_does_zero_renderable_owned_hint_roundtrips` | **pass** |
| Corpus non-vacuity (layout, paint, text-layout, browser attrs present pre-fold) | `darkmatter` `structural_gate::styled_corpus_populates_every_typed_group` | **pass** |

Two of the three spec properties (zero extension-bag access, zero per-node
formatted hint keys) collapse onto one observable — a `renderable.*` bag access —
so the zero-access gate certifies both simultaneously. The third (zero typed-attr
serde round-trips) is a code-review invariant for which this gate is a backstop,
not a proof (see the `structural_gate` module doc for the derivation).

## Corpus expansion (this phase)

Both structural corpora were expanded to the spec section 6 feature list:

- **Terminal** (`perf_gate.rs::styled_corpus_document`): added alpha
  foreground+background paint, a box with `padding` + `border` +
  `Width::Fixed` + `max_width` + center alignment, a separate `Width::FitContent`
  block, an ordered list, and a link + image carrying width-dependent
  `TextLayoutHints`.
- **Browser/Terminal/Markdown production path**
  (`structural_gate.rs::STYLED_CORPUS`): added alpha foreground, component
  fixed/max width (`table.width`, `block-quote.max-width`), page padding/
  max-width, and an ordered list with `ol.max-width`.

## Criterion timings (trend only, non-gating)

Short local runs, release profile, optimistic 120-col terminal, macOS
(Darwin 25.5.0). These are warm-up=1s, measurement=2–3s, sample-size=10–20
runs — deliberately short for trend capture, **not** statistically authoritative
baselines.

Command (new styled-production group through the real production entry points
`DarkmatterPage::render` / `render_to_browser`):

```text
cargo bench -p darkmatter --bench render_pipeline_steps -- styled_production
```

| Benchmark | Median |
|---|---|
| `styled_production/page_styled/terminal` (full `style:` pipeline + fold) | ~412 µs |
| `styled_production/page_styled/browser` (full `style:` pipeline + fold) | ~197 µs |
| `render_pipeline_terminal/render` (fold-only → terminal, unstyled corpus) | ~357 µs |
| `render_pipeline_terminal/full` (parse+fold+render, unstyled corpus) | ~410 µs |
| `render_pipeline_browser/render` (fold-only → browser, unstyled corpus) | ~283 µs |
| `render_pipeline_browser/full` (parse+fold+render, unstyled corpus) | ~296 µs |
| `darkmatter_components/darkmatter_page/terminal` (unstyled page) | ~81 µs |
| `darkmatter_components/darkmatter_page/browser` (unstyled page) | ~139 µs |

## Comparison rationale

- The new `styled_production` group measures the **real production entry
  points** (`DarkmatterPage::render` / `render_to_browser`) with the full
  `style:` pipeline applied, so its cost includes `from_frontmatter` +
  `apply_*_style` + the typed-tree fold + the target fold — the path the
  production CLI actually pays. The `render_pipeline_*` groups isolate the
  fold/render stages on a larger unstyled corpus and stay for stage-localization
  of regressions.
- Styled terminal (~412 µs) and the unstyled `full` terminal path (~410 µs) are
  similar in magnitude, but these are **different workloads** (styled corpus +
  `style:` pipeline vs. a larger unstyled corpus), so this is an order-of-
  magnitude sanity check, not a like-for-like comparison. The styled browser path
  (~197 µs) carries more measurement variance (small absolute time, allocation-
  sensitive); treat its band as indicative, not a threshold.
- **No same-corpus before/after baseline exists for the styled-production group**:
  it was introduced in this phase, so these numbers are the *first* trend anchor
  for it, not evidence of a regression or its absence. A claim of "no regression"
  would require re-running the identical styled corpus against a pre-change build;
  that has not been done. The numbers above establish the baseline future runs
  compare against. Because the structural gate — not wall-clock — is the
  acceptance authority, future wall-clock movement is investigated and documented
  here rather than failing the build.

## How to reproduce

```text
# Authoritative structural gate
cargo test -p biscuit-terminal --test perf_gate
cargo test -p darkmatter --lib structural_gate

# Trend timings (non-gating)
cargo bench -p darkmatter --bench render_pipeline_steps
```
