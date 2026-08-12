# Run record — f-cumulative-closeout / run-20260716T230028

Bracketed re-measurement of the four integrated compose regressions reported by
review-1's "The integrated compose regression gate does not pass" finding.

## Why this run exists

Review-1 rejected the closeout's attribution of the compose regressions to the
linked ReferenceGraph feature: "an ownership boundary does not make a failed
release gate pass." Separately, the review-1 raw-samples work (Finding 4) found
that F33's claimed −19 % control regressions were pure cross-run drift on a host
under load 29–64, and concluded that any cross-run comparison needs an explicit
same-code **drift bracket** to be trustworthy.

This run tested the resulting hypothesis: **are the four compose regressions
also measurement artifacts?**

**They are not.** The hypothesis was refuted. All four reproduce.

## Method

- **Pins** — baseline `51c1f16e1` (the audit commit), candidate `74e0fdc90`
  (the current integrated head; newer than the original closeout's
  `b425fb466`+worktree). Built in detached worktrees with isolated
  `CARGO_TARGET_DIR`, measured in one session.
- **Drift bracket** — each case runs `cand_A → base → cand_B` inside a single
  hyperfine invocation. `cand_A` vs `cand_B` is identical code measured twice,
  so their delta is the drift floor this host can resolve. A regression is only
  established when it clears that floor.
- **Command** — `hyperfine --shell=none --warmup 3 --runs 20`, `NO_COLOR=1`,
  non-TTY (piped).
- **Fixtures** — all five verified byte- and `darkmatter_hash`-identical to
  `manifest.yaml`; working tree git-clean for the fixture directory.
- **Statistic** — mean ± stddev over 20 runs; bootstrap 95 % CIs recomputed from
  the retained `times` vectors via `benchmarks/recompute.ts`.

## Host and tool versions

- Apple M4 Max, 16 cores, macOS 26.5.2 (Darwin arm64)
- rustc / cargo 1.96.0, release profile
- hyperfine 1.20.0
- Load average **6.63 → 6.22** across the run (`load.log`); quiet window, no
  concurrent agents.

## Results

Times in ms, mean ± stddev, n=20.

| Case | cand_A | base | cand_B | Drift (A→B) | Regression |
|---|---|---|---|---|---|
| `compose_trivial` | 13.99 ± 0.52 | 10.14 ± 0.70 | 13.17 ± 0.63 | −5.9 % | **+34.0 %** |
| `compose_schema_transclusion` | 19.70 ± 0.91 | 15.58 ± 1.06 | 20.00 ± 0.41 | +1.6 % | **+27.4 %** |
| `compose_interpolation_heavy` | 16.54 ± 0.38 | 15.02 ± 0.55 | 16.80 ± 0.69 | +1.6 % | **+11.0 %** |
| `compose_transclusion_heavy` | 58.41 ± 1.99 | 50.95 ± 1.60 | 58.19 ± 1.76 | −0.4 % | **+14.4 %** |
| `render_basic` *(control)* | 4.56 ± 0.23 | 4.47 ± 0.25 | 4.45 ± 0.42 | −2.3 % | +0.7 % *(flat)* |

Bootstrap 95 % CIs: all four targets separate **completely** from baseline on
both candidate arms; the `render_basic` control **overlaps** baseline on both
arms, as a compose-confined change predicts.

**Caveat on `compose_trivial`.** Its two candidate arms do not overlap each
other — a real −5.9 % drift on that case alone. Read it from the conservative
arm (`cand_B` 13.17 vs base 10.14 = **+29.9 %**) and the regression is still
~5× the measured drift and ~6× the gate threshold. The conclusion is unchanged
either way.

## Comparison to the original closeout

The original `run-20260716T050518` was **not** the corrupted methodology the
hypothesis assumed. It already interleaved all pins within a single hyperfine
invocation at load 5–7, carried controls that moved the opposite direction, and
was backed by a bisect plus a `--perf` segment split. Its only gap was the
absence of an explicit same-code drift bracket, which this run supplies.

Its quoted statistics recompute **exactly** from its own retained vectors
(10.16 ± 0.48 → 10.1620 ± 475 µs; 13.69 ± 0.91 → 13.6949 ± 911 µs).

The original numbers (+34.8 / +23.1 / +18.3 / +14.0 %) therefore stand. Only
`compose_interpolation_heavy` moved materially (+18.3 → +11.0 %), plausibly from
follow-up interpolation work landed between the two pins.

## The ReferenceGraph "load-neutral" re-measurement is not a contradiction

A project note records ReferenceGraph review-2 re-measuring its compose cost as
load-neutral. That re-measurement covers that feature's Criterion `construct`
**microbenchmark** (167 → 234 µs, **+67 µs**). This gate measures the whole
`md compose` **command**, where Command Setup grows **~+3 ms**. The two are
three orders of magnitude apart and are not substitutes. ReferenceGraph
review-2 independently reports a **+40.4 %** construction regression, which
corroborates this result rather than conflicting with it.

## Mechanism (reproduced at current head)

From `perf-segments.txt`:

- Command Setup **5.3 → 8.5 ms** — `validate references` 3.4 → 6.4 ms,
  `build options` 3.8 → 6.8 ms.
- Compose Pipeline **flat** (788 µs → 1.0 ms).

`compose_trivial` has no descendants yet still regresses ~3 ms, which locates
the cost in **graph/identity construction on the setup path**, not in the
descendant re-read.

## Retained raw data

- `compose_trivial.json`, `compose_schema_transclusion.json`,
  `compose_interpolation_heavy.json`, `compose_transclusion_heavy.json`,
  `render_basic.json` — hyperfine output including per-run `times` vectors.
- `load.log` — load average sampled across the run.
- `perf-segments.txt` — `md compose --perf` segment split for baseline and
  candidate.

Every statistic above is recomputable from these vectors with
`benchmarks/recompute.ts` (extended during this run to read hyperfine vectors,
not only Criterion `sample.json` — the closeout's hyperfine numbers were
previously not recomputable, a live gap in the evidence contract).

## Disposition

The integrated compose regression gate remains **FAILED** on independent,
bracketed evidence. The "it's measurement noise" explanation is closed off.

This needs an owner decision — no production code was changed to chase it:

1. **Fix** the ReferenceGraph setup-path cost (`build options` / `validate
   references`), or
2. **Re-threshold** with a recorded compatibility decision accepting +11–34 % as
   the price of graph-guard correctness, or
3. **Keep the feature blocked** on the linked ReferenceGraph work.
