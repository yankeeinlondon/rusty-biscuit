# Run record — Finding 23, retained raw sample vectors

Supersedes the F23 half of
[`../run-20260716T120000/`](../run-20260716T120000/summary.md), which retained
Criterion **estimates only** and whose comparison was a cross-run
`--save-baseline` pair. Review-2 rejected both: estimates cannot be recomputed,
and this host's drift makes a cross-run ~0.6% delta unreadable. **This run
corrects that record's headline figure — see *Correction*.**

- **Run id:** run-20260717T012550-f23 (2026-07-17 01:25:50 UTC)
- **Host:** Apple M4 Max (Mac16,5), macOS (Darwin 25.5.0), shared
  non-interactive sandbox. Piped (non-TTY) process; no PTY measurement.
- **Host load (1/5/15 min):** `10.34 11.45 17.93` before, `22.41 14.69 18.62`
  after. High and volatile — which is exactly why nothing here is compared
  across processes.
- **Commit:** `a80e032c3` (branch `darkmatter`). Baseline and candidate are
  **both in this one binary**; there is no second commit to name (see *Method*).
- **Build profile:** `release`, stable toolchain per `rust-toolchain.toml`,
  workspace lockfile unchanged.
- **Tool versions:** `rustc 1.96.0 (ac68faa20 2026-05-25)`,
  `cargo 1.96.0 (30a34c682 2026-05-25)`,
  `cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)`, `bun 1.3.3`. Criterion and
  hyperfine are **not** used by this harness.
- **Fixture (immutable, hashed in `manifest.yaml`, generator 1.2.0):**
  `render_code_heavy` — 40 fenced blocks across 8 languages, 4157 bytes,
  `xxhash64: 2f5d9cca8c854cfd`. No new fixture was registered.
- **Harness (retained):**
  `darkmatter/lib/src/markdown/render_tree/code_renderer.rs`,
  `tests::f23_code_surface_raw_samples`, built on the shared
  `darkmatter/lib/src/perf_harness.rs`. Both are `#[cfg(test)]` — no public API
  was added or widened to measure. The test is `#[ignore]`d **and** gated on
  `DM_PERF_RAW_DIR`, so `just test` neither runs nor is slowed by it.

## Command

```
cd darkmatter
DM_PERF_RAW_DIR=darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260717T012550-f23 \
  cargo nextest run -p darkmatter --lib --release --run-ignored all \
  -E 'test(f23_code_surface_raw_samples)' --no-capture
```

Recomputation — every statistic below is produced by this command from the
vectors committed beside this file:

```
cd darkmatter/features/2026-07-15-performance-followup/benchmarks
bun recompute.ts raw/f23f25-render-cleanup/run-20260717T012550-f23
```

## Method

Baseline and candidate are sampled **interleaved, sample by sample, in one
process** (`Harness::interleaved_pair`), so both see the same thermal and
scheduling conditions. No drift bracket is needed: there is no cross-run
comparison to bracket.

The pre-F23 shape is reconstructed without a pinned code copy and without a
production seam. A `TerminalCodeRenderer` **rebuilt per block** reads
`CODE_THEME`/`THEME` and resolves the surface once per block — what the
per-block resolution did; the F23 renderer is built once per render and
resolves once. That reconstruction is **asserted, not assumed**: the
`surface_probe` counters require the baseline to report 40 environment reads +
40 resolutions and the candidate exactly 1 + 1, on both the terminal and
browser paths.

- **Warm-up:** 3 batches per arm. **Samples:** 300 per arm (40-fence pairs,
  batch 1); 300 per arm (control, batch 20).
- **Statistic:** mean, with a bootstrap 95% CI from `recompute.ts`; median and
  std dev also recomputed.
- `THEME=github` is pinned and `CODE_THEME`/`NO_COLOR` removed, so the measured
  resolution chain does not depend on the invoking shell.

### Deviations from the entry-point benchmark (disclosed)

1. **Entered at the code hook**, not at `as_terminal`/`as_html`. This excludes
   the fixture's prose, layout, and frame work, giving F23 its **best case**:
   the hoisted resolution is a larger share here than in any real render, so
   the entry-point effect is **at most** what is measured below. Bounding it
   this way needs no cross-run number.
2. The browser arm uses the **page-less** `HtmlOptions::default()` surface, so
   its per-block clone is of a default `HtmlOptions`, not a page-resolved one.
3. The baseline additionally reconstructs the (allocation-free) renderer struct
   per block. The environment reads it pays per block are pre-F23 behavior; the
   two `RefCell`/`OnceCell` initializations are not, and are a small charge
   against the baseline.

## Predeclared thresholds

Carried over **verbatim** from `run-20260716T120000/summary.md`, where they
were declared before that baseline was captured:

- **F23 target:** "a repeatable, out-of-noise reduction in `as_terminal` /
  `as_html` over the 40-block fixture, **with byte-identical output**."
- **F23 control:** "`code_block_direct` (the one-block, page-less surface,
  which has no per-render hoisting available) must not regress beyond noise. A
  control that moves with the target identifies build-to-build drift rather
  than a win."

The control is **re-specified** for an interleaved harness: at one block the
baseline and candidate shapes do identical work (one construction, one
resolution, one highlight), so their measured separation *is* the harness's
noise floor. That is a strictly stronger floor than a cross-build control,
which can only detect drift it happens to share.

## Correctness gate (runs before any timing)

| gate | result |
|---|---|
| terminal baseline vs candidate, 40 blocks | **byte-identical** |
| browser baseline vs candidate, 40 blocks | **byte-identical** |
| control baseline vs candidate, 1 block | **byte-identical** |
| baseline env reads / resolutions (terminal, browser) | **40 / 40** — the per-block shape is real |
| candidate env reads / resolutions (terminal, browser) | **1 / 1** — the contract holds |

## Results

| benchmark | n | mean | 95% CI (mean) | median | std dev |
|---|---|---|---|---|---|
| `f23-terminal-code-heavy-baseline` | 300 | 1.5055 ms | [1.4997 ms, 1.5118 ms] | 1.4947 ms | 53.0143 µs |
| `f23-terminal-code-heavy-candidate` | 300 | 1.4884 ms | [1.4829 ms, 1.4943 ms] | 1.4792 ms | 50.4594 µs |
| `f23-browser-code-heavy-baseline` | 300 | 1.5187 ms | [1.5113 ms, 1.5264 ms] | 1.4986 ms | 66.5446 µs |
| `f23-browser-code-heavy-candidate` | 300 | 1.5000 ms | [1.4924 ms, 1.5081 ms] | 1.4811 ms | 69.3977 µs |
| `f23-control-single-block-baseline` | 300 | 29.4233 µs | [29.3518 µs, 29.5044 µs] | 29.3604 µs | 677.3510 ns |
| `f23-control-single-block-candidate` | 300 | 29.4309 µs | [29.3675 µs, 29.5013 µs] | 29.3792 µs | 590.4872 ns |

Derived from the means above:

| arm | change | CIs |
|---|---|---|
| terminal, 40 fences | **−1.14%** | disjoint (1.4997 ms > 1.4943 ms) |
| browser, 40 fences | **−1.23%** | disjoint (1.5113 ms > 1.5081 ms) |
| **control**, 1 fence | **+0.03%** | fully overlapping → floor ≈ ±0.25% |

A discarded 100-sample pilot at load ~16.9 — its CIs abutted, which is why the
sample count was raised to 300 — gave −1.12% terminal, −1.07% browser, +0.17%
control. The effect reproduces across two runs at loads ~10 and ~17.

## Correction to the recorded disposition

**The recorded figure is contradicted.** `results.md` (Finding 23) and
`run-20260716T120000/summary.md` record `as_terminal` −0.61%, `as_html` −0.79%,
control `code_block_direct` −0.66%, concluding *"the control moved the same
amount: the shift is build-to-build drift, and the targets net ≈0.1% — noise."*

Interleaved in-process sampling — which the benchmark README prescribes for
exactly this reason — measures **−1.14% / −1.23%** with disjoint 95% CIs
against a **+0.03%** control whose CIs overlap. F23's work-reduction is **small
but real**, not a null.

The old number is explained rather than merely disagreed with: subtracting a
control's cross-build movement from a target's assumes the drift is uniform
across benchmarks, which is the assumption the README's drift warning says
cannot be made on this host. The −0.66% control shift and the −0.61% target
shift were two independent draws from a drifting distribution; netting them to
≈0.1% was not a valid operation.

**What does not change: the disposition.** −1.1% at F23's *best-case* boundary
bounds the entry-point effect at ≤ −1.1%, well under the "repeatable,
out-of-noise reduction in `as_terminal`/`as_html`" the checkpoint predeclared
as its target, and nowhere near user-visible. F23 remains **retained on
contract grounds** — plan-mandated, byte-identical, counter-verified — with the
honest claim upgraded from *"no measurable win"* to **"a ~1% reduction at the
code hook; contract satisfied; not a user-visible speed-up."**

Raw: `f23-{terminal,browser}-code-heavy-{baseline,candidate}-sample.json`,
`f23-control-single-block-{baseline,candidate}-sample.json`.

## Cross-platform

Unchanged from the superseded record: F23's diff is OS-identical (a struct
field, a `RefCell`/`OnceCell` memo, and the same resolution chain invoked once
instead of per block; no `cfg`, filesystem, terminal-detection, or process
branch added or moved). This run adds no platform surface — the harness is
`#[cfg(test)]` and calls the same code on the same frozen bytes.
