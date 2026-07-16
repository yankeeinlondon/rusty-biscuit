# Run record — Phase 6 (Findings 11–14): frontmatter/body interpolation & replacement

- **Run id:** run-20260716T085358 (2026-07-16 08:53 UTC)
- **Host:** Apple M4 Max (Mac16,5), macOS (Darwin 25.5.0), single-host
  non-interactive sandbox.
- **Baseline commit:** `b425fb466` (branch `darkmatter` HEAD before this phase),
  built by reverting the seven Phase-6 source files to HEAD.
- **Candidate:** this phase's changes (F11 incremental fixpoint, F12 borrowed
  `ResolutionContext`, F13 leftmost-longest replacement automaton, F14 scan
  fast-path). Baseline and candidate share identical fixture/harness bytes;
  only the code under test differs.
- **Fixtures (immutable, hashed in `manifest.yaml`, generator 1.2.0):**
  `compose_interpolation_heavy` (wide 30-key + deep 15-link frontmatter graph,
  nested body interpolation, `{{{ }}}` literal, fenced code, `replace:` map;
  2881 bytes), `replace_heavy` (43-rule / ~1600-occurrence `replace:` map;
  40944 bytes), `toc_large` (1000-heading, interpolation-free body).

## Correctness gate (predeclared: byte-identical composed output)

Baseline vs candidate `md compose` over `compose_interpolation_heavy.md` and
`replace_heavy.md`: **byte-identical** (`diff` empty). Also byte-identical on
`compose_trivial`, `compose_schema_transclusion`, `render_basic`,
`render_code_heavy`. This is the hard gate for all four findings — every
optimization is required to preserve output exactly.

## F13 — text replacement matcher (isolated microbenchmark)

Direct `apply_replacements(body, state)` over the 43-rule `replace_heavy` body,
state built once so the number isolates the matcher from compose-context
capture. Criterion, 50 samples, warm-up 3 s.

| variant | mean |
|---|---|
| baseline (per-character `starts_with` scan, `O(pos × rules × keylen)`) | **2.371 ms** |
| candidate (Aho–Corasick `LeftmostLongest`, single linear pass) | **0.087 ms** |

**≈27× faster, decisive (p < 0.05, non-overlapping CIs).** Predeclared
threshold: any repeatable out-of-noise win with byte-identical output on the
canonical precedence — met. Raw: `criterion-apply_replacements-{baseline,candidate}.json`.

## F14 — scan fast-path (isolated microbenchmark)

For an interpolation-free body the baseline body-scan runs a full MarkdownAware
`ExpressionFinder::new(body).find_all()` (pulldown-cmark code-region parse); the
candidate short-circuits on `body.contains("{{")`. Over the `toc_large` body:

| operation | mean |
|---|---|
| `f14_baseline_markdown_scan` (the parse F14 skips) | **240.1 µs** |
| `f14_candidate_contains_guard` (what F14 does instead) | **2.3 µs** |

**≈104× less work per compose for every `{{`-free body**, byte-identical (a
`{{`-free body yields zero expressions and zero literals under either path). Raw:
`criterion-f14-*.json`.

## F11 / F12 — whole-pipeline wall-clock (control)

`md compose compose_interpolation_heavy.md`, hyperfine, warm-up 5, 80 runs,
`--shell=none`, `NO_COLOR=1`:

| binary | mean | σ | user |
|---|---|---|---|
| baseline | 19.0 ms | 0.5 ms | 14.5 ms |
| candidate | ~18.6 ms | ~0.6 ms | ~14.1 ms |

Ratio **1.02 ± 0.06× (within σ)**. Whole-pipeline compose wall-clock is
dominated by per-run setup (process start, context capture, rendering): the
phase6 Criterion pipeline benches measure ≈158 ms per compose *stage-invariant*
(the same order whether only `Cleanup`, `FrontmatterInterpolation`, or
`Interpolation` runs), so the F11/F12 per-key work sits below this floor. F11
(deps parsed once + `O(sweeps × keys)` re-parse and per-key seed/context/context
clones eliminated → `O(keys + edges)` fixpoint over a reused, in-place-mutated
lookup) and F12 (borrowed `ResolutionContext`, zero read-side clones) are
therefore retained as byte-identical **structural work-reductions**, matching the
Phase-5 precedent (Finding 35.1) for keeping a reduction whose wall-clock is
setup-dominated. Neither adds speculative machinery beyond the plan-requested F11
worklist. Raw: `cli-compose-interpolation-heavy.json`.

## Cross-platform

- F11, F12, F14: OS-identical — pure in-memory string/AST/allocation work, no
  `cfg`/filesystem/URL-runtime branch. Windows compile + macOS behavioral run +
  ordinary Linux CI sufficient.
- F13: OS-identical — Aho–Corasick operates on the in-memory body bytes; no
  platform branch. Same sufficiency.
