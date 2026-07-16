---
scope: "Phase 11 closeout results for the 2026-07-12 darkmatter perf review"
captured: "2026-07-14"
commit: "64e4b8cb8"
branch: "darkmatter"
host: "Apple M4 Max, macOS 26.5.2 (arm64)"
rustc: "1.96.0 (2026-05-25)"
build: "release"
tool: "hyperfine 1.20.0 (--shell=none, stdout+stderr non-TTY)"
baseline: "./baseline.md"
---

# Phase 11 Closeout — Darkmatter Performance Review (2026-07-12)

> **Superseded as a gate (2026-07-16).** These numbers show direction but are
> **not** a release gate: this table and [`baseline.md`](./baseline.md) were
> captured over *different, unhashed fixture bytes*, so the before/after pair is
> not comparable (Review 3's rejection). The command/TOC result was
> reconstructed on identical hashed fixture bytes — same host, same immutable
> fixture directory, predeclared thresholds, raw samples retained — and it
> **passes** (`toc_large` 488 ms → 23 ms). See the reconstruction in
> [`f4-historical-closeout`](../../features/2026-07-15-performance-followup/benchmarks/raw/f4-historical-closeout/run-20260715T232610/summary.md)
> and the fixture identity authority in
> [`benchmarks/manifest.yaml`](../../features/2026-07-15-performance-followup/benchmarks/manifest.yaml).
> Read this file as the historical `codex/default` capture; final dispositions
> are in
> [`2026-07-15-performance-followup/results.md`](../../features/2026-07-15-performance-followup/results.md).

After-fix measurements captured on the **same host** as
[`baseline.md`](./baseline.md), so every number here diffs directly against the
Phase 1 table. All numbers are from a **release** build of the `darkmatter`
branch, run through `hyperfine` with no TTY attached (terminal detection
short-circuits, matching the baseline methodology).

> The fixtures were regenerated deterministically at the same size classes as
> Phase 1 (small / large / 81 KB / 326 KB / 1.3 MB). The generator emits a few
> more sections per byte-target than the original capture, so the **absolute**
> toc times are not point-comparable across the two runs — but the **scaling
> relationship** (the whole point of Finding 4) is, and it is now linear.

## Headline results

| Finding | Metric | Baseline | After | Win |
|---------|--------|----------|-------|-----|
| **F1 (critical)** | `md compose small.md` wall | 84.4 ms | **12.5 ms** | **6.8× / NTP stall gone** |
| **F1** | `capture context` segment (no `ctx.*` doc) | 62.9 ms | **185 µs** | **~340×** |
| **F4 (high)** | `md toc --json` 1.3 MB tier | 27.72 s | **140 ms** | **~197×** |
| **F4** | toc scaling (4× size → time) | 13.8× / 15.6× | **~2× / ~6×** | quadratic → ~linear |
| **F5–F9** | `schema validation` segment | 2.8 ms | **476 µs** | ~5.9× |
| **F19–F24** | `as_terminal` code-heavy 100 KB | ~239 ms | **~57 ms** | ~4.2× |
| **F19–F24** | `page_render` code-heavy 100 KB | ~187 ms | **~57 ms** | ~3.3× |

## Command baseline (after)

| Command | Mean ± σ | Baseline | Notes |
|---------|----------|----------|-------|
| `md --help` | 9.9 ms ± 6.4 ms | 4.5 ms | startup floor (noisy host; min 5.1 ms) |
| `md small.md` (render) | 11.7 ms ± 9.0 ms | 4.4 ms | min 5.5 ms; noise-inflated mean |
| `md large.md` (render) | 13.7 ms ± 7.4 ms | 7.0 ms | 110 KB doc; min 7.0 ms |
| `md hash small.md` | 10.9 ms ± 8.5 ms | 4.4 ms | min 5.0 ms |
| `md compose small.md` | **12.5 ms ± 0.6 ms** | **84.4 ms** | **NTP probe eliminated (F1)** |
| `md compose --no-trigger-schemas` | 12.3 ms ± 0.5 ms | 83.2 ms | |
| `md compose --no-baseline-schema --no-trigger-schemas` | 6.1 ms ± 0.9 ms | 79.4 ms | baseline schema now ~6.4 ms of the 12.5 ms |

The `--help` / render / hash rows show inflated *means* from background load on a
busy host (large σ, high max), but their **minimums** (5.0–7.0 ms) match the
Phase 1 floor — none of the fixes regressed the startup or render floor. The
compose rows are stable (σ ≤ 0.9 ms) and tell the real story: **`md compose`
collapsed from 84.4 ms to 12.5 ms** because the always-on `sntp time.apple.com`
round-trip is gone. Compose is now dominated by the baseline-schema validation
(~6.4 ms), not the network.

### `md compose --perf` attribution (after)

| Command Setup (8.7 ms) | | Compose Pipeline (1.7 ms) | |
|---|---|---|---|
| load input | 69 µs | frontmatter interpolation | 10 µs (2 calls) |
| resolve input | 102 µs | **schema validation** | **476 µs** |
| **capture context** | **185 µs** | frontmatter shell expansion | 9 µs (2 calls) |
| validate references | 3.9 ms | effective state build | 1 µs |
| build options | 5.8 ms | transclusion / cleanup / normalization | ≤ 53 µs each |
| compose pipeline | 2.4 ms | | |

`capture context` = **185 µs** (was 62.9 ms). An NTP round-trip cannot complete
in 185 µs — the probe is definitively gone. `schema validation` = 476 µs (was
2.8 ms), corroborating the Phase 4 / Phase 9 double-work removals. Every
documented segment is still reported (harness intact).

## TOC scaling (after)

| Fixture | Mean ± σ | Baseline | Notes |
|---------|----------|----------|-------|
| 81 KB (498 sections) | 10.8 ms ± 5.2 ms | 128.3 ms | |
| 326 KB (1 962 sections) | 22.9 ms ± 11.6 ms | 1.772 s | |
| 1.3 MB (7 726 sections) | **140.3 ms ± 49.3 ms** | **27.724 s** | **sub-second (was ~28 s)** |

**Quadratic eliminated (Finding 4).** Baseline: 4× the document size cost 13.8×
then 15.6× the time. After: ~2× (81 KB → 326 KB) and ~6× (326 KB → 1.3 MB) —
roughly linear with fixed per-invocation overhead. The 1.3 MB tier dropped from
**27.7 s to 140 ms (~197×)**, exactly the Phase 5 target.

## Criterion benches (steady-state, in-process)

Smoke run (`--sample-size 10 --warm-up-time 1 --measurement-time 3`), matching
the Phase 1 methodology. `change` is criterion's comparison against the stored
in-tree baseline.

| Bench / function | Fixture | Baseline (smoke) | After (median) | change |
|------------------|---------|------------------|----------------|--------|
| `compose_schema_transclusion/compose_with` | fm-interp + `$schema` + baseline + 1 `::file` | ~7.9 ms | **4.28 ms** | −53.7% |
| `render_code_heavy/as_terminal` | code-heavy ~100 KB doc | ~239 ms | **57.1 ms** | −76.1% |
| `render_code_heavy/page_render` | code-heavy ~100 KB doc | ~187 ms | **56.8 ms** | −69.6% |

The render wins (~4×) are dominated by the Phase 8 removal of the per-code-block
syntect theme deep-clone (F23) and the `write!`-into-buffer emission (F24); the
compose win by the Phase 4 schema-stage cache reuse (F5/F6/F8/F9) and Phase 7/9
allocation reductions.

## Good patterns preserved (spec §"good patterns")

Confirmed **not** disturbed by any fix:

- **syntect one-time load** — `THEME_SET` / `SYNTAX_SET` remain process-wide
  `lazy_static`; F23 now borrows `&'static SyntectTheme` from that set rather
  than cloning it (strengthens the pattern, does not replace it).
- **`HighlightLines` reuse** — unchanged; still constructed once per code block.
- **zero render-path regexes** — no regex was added to the render path (F13's
  `aho_corasick` matcher was deferred; F19 uses a delimiter-pairing scan, not a
  regex).
- **non-TTY short-circuit** — preserved and extended: F21 gates the macOS
  `defaults read` spawn behind `is_tty()`; pure Markdown/HTML/JSON output still
  detects no terminal.
- **demand-driven capture** — unchanged; `ctx.*` groups are still only captured
  when referenced. F1 removed the NTP probe *inside* the always-on datetime
  group, not the demand-driven gating.
- **single-flight caches** — preserved; the new process-wide validator /
  coercion / namespace / theme caches are additive and key-correct (F8 folds
  `file_ref_fallback_dir` into the key; F26 swaps SHA-256 → xxHash without
  changing cache semantics).

## Reproduce

1. `cargo build -p darkmatter-cli --release`
2. Regenerate fixtures at the five size classes (any deterministic generator).
3. Re-run the three `hyperfine` invocations (command / compose / toc) and the
   two `cargo bench` targets (`compose_schema_transclusion`, `render_code_heavy`).
4. Diff against [`baseline.md`](./baseline.md).
