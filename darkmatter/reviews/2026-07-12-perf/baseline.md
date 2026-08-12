---
scope: "Phase 1 measurement baseline for the 2026-07-12 darkmatter perf review"
captured: "2026-07-14"
commit: "83aaecc8f"
branch: "darkmatter"
host: "Apple M4 Max, macOS 26.5.2 (arm64)"
rustc: "1.96.0 (2026-05-25)"
build: "release"
tool: "hyperfine 1.20.0 (--shell=none, stdout+stderr non-TTY)"
---

# Phase 1 Baseline — Darkmatter Performance Review (2026-07-12)

> **Not reproducible as captured (2026-07-16).** This baseline records fixture
> *sizes* but not the fixture **bytes or their hashes**, and instructs re-runs to
> use "any deterministic generator of the same sizes" — so it cannot be paired
> with [`results.md`](./results.md) as a gate. That hole is closed by the
> follow-up's immutable fixture manifest
> ([`benchmarks/manifest.yaml`](../../features/2026-07-15-performance-followup/benchmarks/manifest.yaml):
> committed bytes + a pinned generator + Darkmatter and xxHash identities) and
> the same-bytes reconstruction of this commit (`83aaecc8f`) against the audit
> commit in
> [`f4-historical-closeout`](../../features/2026-07-15-performance-followup/benchmarks/raw/f4-historical-closeout/run-20260715T232610/summary.md).
> Preserved unedited as the historical `codex/default` capture.

Reproducible before/after baseline captured on **this host** so every later
phase has a concrete checkpoint. All numbers are from a **release** build of the
`darkmatter` branch at commit `83aaecc8f`, run through `hyperfine` with no TTY
attached (both stdout and stderr are pipes — terminal detection short-circuits,
matching the review's "stdout piped" methodology).

> This host (Apple M4 Max) is substantially faster than the review's original
> capture host, so absolute times are lower than the spec's table. The
> **relationships** the review flagged reproduce exactly: the per-compose NTP
> stall dominates compose wall time, and `md toc` is quadratic. Compare later
> phases against *this* table, not the spec's.

## Command baseline

| Command | Mean ± σ | Runs | Notes |
|---------|----------|------|-------|
| `md --help` | 4.5 ms ± 0.5 ms | 576 | startup floor; no syntect/terminal work |
| `md small.md` (render) | 4.4 ms ± 0.4 ms | 668 | 133-byte doc |
| `md large.md` (render) | 7.0 ms ± 0.6 ms | 482 | 110 KB doc — render scales well |
| `md hash small.md` | 4.4 ms ± 0.4 ms | 749 | |
| `md compose small.md` | 84.4 ms ± 6.6 ms | 33 | trivial doc, no `ctx.*`, no transclusion |
| `md compose --no-trigger-schemas` | 83.2 ms ± 5.7 ms | 35 | trigger discovery ≈ 1 ms on this tree |
| `md compose --no-baseline-schema --no-trigger-schemas` | 79.4 ms ± 7.8 ms | 44 | baseline schema ≈ 4 ms wall |

**Compose is NTP-bound (Finding 1 confirmed).** On `md compose small.md` the
mean wall is **84.4 ms** but *user* time is only **22.9 ms** — the ~60 ms gap is
the blocking `sntp time.apple.com` round-trip from the always-on datetime
capture. The trigger-discovery and baseline-schema deltas that dominate the
spec's slower host are small here (~1 ms and ~4 ms) precisely because the NTP
wall swamps everything else. Phase 2 must collapse `md compose` toward the
~20 ms user-time floor.

### `md compose --perf` attribution (`md compose --perf small.md`)

Confirms the documented segments are all still reported (primary harness for
Findings 1, 5, 6, 7):

| Command Setup | | Compose Pipeline | |
|---|---|---|---|
| load input | 64 µs | frontmatter interpolation | 7 µs (2 calls) |
| resolve input | 55 µs | **schema validation** | **2.8 ms** |
| **capture context** | **62.9 ms** | frontmatter shell expansion | 5 µs (2 calls) |
| validate references | 4.4 ms | effective state build | 3 µs |
| build options | 4.9 ms | text replacement / page blocks / interpolation | ≤ 16 µs each |
| compose pipeline | 11.6 ms | transclusion / cleanup / normalization | ≤ 18 µs each |

`capture context` = **62.9 ms** on a document that references zero `ctx.*`
values — the NTP probe computed and thrown away (Finding 1). The ~2.8 ms
schema-validation segment vs the larger cross-pass baseline-schema cost
corroborates Findings 5–9.

## TOC scaling baseline (`md toc --json`)

| Fixture | Mean ± σ | Runs |
|---------|----------|------|
| 81 KB (`toc_81k.md`, 408 sections) | 128.3 ms ± 2.8 ms | 23 |
| 326 KB (`toc_326k.md`, 1 618 sections) | 1.772 s ± 0.033 s | 10 |
| 1.3 MB (`toc_1p3m.md`, 6 369 sections) | 27.724 s ± 0.154 s | 2 |

**Quadratic confirmed (Finding 4).** 4× the document size costs **13.8×** the
time (81 KB → 326 KB) and **~15.6×** again (326 KB → 1.3 MB). Phase 5 must make
this roughly linear — the 1.3 MB tier should drop from ~28 s to sub-second.

## Criterion benches (steady-state, in-process)

Two benches were added for the two regressed paths that had no coverage
(`darkmatter/lib/benches/`). Unlike the per-process hyperfine table, these run
many iterations in one process, so the one-time NTP probe amortizes into warm-up
— they isolate the *steady-state* compose/render work that Phases 4–9 target.

Run: `just bench` (all), or per-bench:

```text
cargo bench -p darkmatter --bench compose_schema_transclusion
cargo bench -p darkmatter --bench render_code_heavy
```

Indicative smoke numbers on this host (10-sample smoke, not the recorded
baseline — capture full baselines with `just bench-baseline phase1` before
Phase 2 work begins):

| Bench / function | Fixture | Time (smoke) |
|------------------|---------|--------------|
| `compose_schema_transclusion/compose_with` | fm-interp + `$schema` + baseline + 1 `::file` | ~7.9 ms |
| `render_code_heavy/as_terminal` | code-heavy ~100 KB doc | ~239 ms |
| `render_code_heavy/page_render` | code-heavy ~100 KB doc | ~187 ms |

## Fixtures

Deterministically generated for this capture (not committed — regenerate for a
re-run). Section chunk = heading + prose + one `rust` code block (~190 bytes):

- `small.md` — 133 bytes (trivial compose/render floor)
- `large.md` — 110 071 bytes / 553 sections (render scaling)
- `toc_81k.md` — 81 071 bytes / 408 sections
- `toc_326k.md` — 326 161 bytes / 1 618 sections
- `toc_1p3m.md` — 1 300 116 bytes / 6 369 sections

## How to re-run for Phase 11 closeout

1. `cargo build -p darkmatter-cli --release`
2. Regenerate the fixtures above (any deterministic generator of the same sizes).
3. Re-run the three `hyperfine` invocations and the two `cargo bench` targets.
4. Diff against this table; record deltas in `results.md`.
