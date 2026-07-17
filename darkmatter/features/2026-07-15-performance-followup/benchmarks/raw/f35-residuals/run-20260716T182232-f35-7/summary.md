# Finding 35.7 — link/image policy appliers borrow instead of clone (retained re-capture)

Run id: `run-20260716T182232-f35-7`

Supersedes `run-20260716T160000/f35_7-link-policy-profile.txt`, whose harness
(`f35_7_profile`) was **deleted after capture**, leaving its numbers
unreproducible. This run re-measures the same four cases through a **retained**
harness and retains the per-observation vectors.

## Harness (retained)

- `darkmatter/lib/src/markdown/render_tree/build_context.rs` →
  `finding_35_7::f35_7_link_policy_raw_samples`
- Pinned baseline: `finding_35_7::baseline_apply_link_policy` (already committed
  alongside the differential tests).
- `#[ignore]`d **and** gated on `Harness::from_env` returning `Some`, so `just
  test` neither runs nor is slowed by it. Adds no public API (`#[cfg(test)]`).

`apply_link_policy` is private; exposing it for a Criterion bench would be the
public-API addition the feature's compatibility invariant 2 bars, so it is
measured in-crate.

Each timed batch applies the policy to all 1000 nodes in place. The two arms own
separate node vectors, so neither observes the other's mutations; because the
appliers are equivalent (gated below), both vectors evolve through identical
states across samples.

## Commits

| role | commit |
|---|---|
| baseline (what the pinned copy reproduces) | `1993665f4` — parent of the F35.7 change |
| candidate | working tree at `a80e032c3` (appliers unchanged since `8f604c5a3`) |

Baseline fidelity re-verified for this run: `baseline_apply_link_policy` is
verbatim `git show 1993665f4:darkmatter/lib/src/markdown/render_tree/build_context.rs`
(`fn apply_link_policy`, lines 254–286). No drift.

## Commands

```
cd darkmatter
DM_PERF_RAW_DIR=darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T182232-f35-7 \
  cargo nextest run -p darkmatter --lib --release --run-ignored all \
  -E 'test(f35_7_link_policy_raw_samples)' --no-capture
```

Recomputation — every statistic below is reproducible by this command from the
vectors committed beside this file:

```
cd darkmatter/features/2026-07-15-performance-followup/benchmarks
bun recompute.ts raw/f35-residuals/run-20260716T182232-f35-7
```

## Environment

- Profile: **release** (`--release`; workspace release profile).
- Host: macOS 26.5.2 (Darwin 25.5.0), Apple M4 Max, arm64, 16 CPUs. Shared host.
- TTY mode: piped (nextest `--no-capture` into a pipe); non-interactive session.
- Tool versions: rustc 1.96.0 (`ac68faa20`), cargo 1.96.0 (`30a34c682`),
  cargo-nextest 0.9.136, bun 1.3.3. No Criterion / hyperfine (in-crate harness).
- Host load average: **11.70 / 25.72 / 30.97 before and after** (this host was
  heavily contended during the session, with excursions to 148; this capture was
  taken in the quietest available window).
- Warm-up: 3 batches per arm. Samples: **50** per arm, batch size 1 (one batch =
  one pass over all 1000 nodes).
- Statistic: **median** (per-iteration). Dispersion: std dev + bootstrap 95 % CI
  of the mean, both recomputed by `recompute.ts`.

No drift bracket is needed: baseline and candidate are sampled **interleaved
sample-by-sample in one process**, so both see the same thermal and scheduling
conditions and the ratio survives the host load.

## Fixture

1000 synthetic link nodes shaped like `toc_large`'s TOC entries
(`./docs/chapter-N/section-N.md#heading-N`), built in-harness per the deleted
record's fixture description. Titles, where present, are plain `Section N`
strings (non-directive). No fixture file was added.

## Predeclared threshold

Carried over **verbatim** from `run-20260716T160000/f35_7-link-policy-profile.txt`,
where it was declared *before* the original baseline capture:

> Target operation: `apply_link_policy` / `apply_image_policy` over a document's
> link and image nodes.
> Minimum repeatable win: >= 5% on the empty-policy target (the common shape: most
> documents carry no `style.hyperlinks.*` policy).
> Maximum permitted control regression: 0% on any populated-policy shape.

## Equivalence gate

Every one of the 4 × 1000 measured nodes is applied by both appliers and compared
by `Debug` equality **before any timing runs**. A ratio between two appliers that
disagree is not a result.

## Results — `apply_link_policy` over 1000 link nodes (median, recomputed)

| case | baseline | candidate | change | recorded claim |
|---|---|---|---|---|
| empty policy, no title | 80.5835 µs | 62.8745 µs | **−22.0 %** | −19.9 % — **confirmed** (win slightly larger) |
| empty policy, with title | 255.0210 µs | 223.5835 µs | **−12.3 %** | −12.1 % — **confirmed** |
| hyperlink policy, no title | 116.7090 µs | 114.2085 µs | **−2.1 %** | −7.9 % — **CONTRADICTED** |
| hyperlink policy, with title | 946.0000 µs | 913.0630 µs | **−3.5 %** | −3.7 % — confirmed, but at the floor |

Bootstrap 95 % CIs of the mean:

| case | baseline CI | candidate CI | separated? |
|---|---|---|---|
| empty policy, no title | [77.65, 81.53] µs | [61.37, 62.71] µs | yes, decisively |
| empty policy, with title | [255.70, 260.41] µs | [226.42, 230.12] µs | yes, decisively |
| hyperlink policy, no title | [116.50, 119.52] µs | [113.82, 116.22] µs | **overlapping — not separated** |
| hyperlink policy, with title | [938.43, 949.52] µs | [908.57, 918.91] µs | yes |

## Honest reading vs the recorded claim

- The **two empty-policy cases — the shapes the predeclared floor is written
  against — reproduce**: −22.0 % vs a recorded −19.9 %, and −12.3 % vs a recorded
  −12.1 %. Both clear the >= 5 % floor decisively with disjoint CIs. This is the
  finding's actual claim and it stands.
- **CONTRADICTION (hyperlink policy, no title):** measured **−2.1 %**, against a
  recorded **−7.9 %** — the recorded figure overstates the win by ~4×. Worse, the
  mean CIs **overlap** ([116.50, 119.52] vs [113.82, 116.22] µs), so on this host
  this shape is **below the measurement floor**: the retained vectors do not
  establish a difference at all. Reported plainly as a structural no-win —
  a 1000-node pass here costs ~117 µs and the two removed clones are a
  vanishing share of it once a populated policy adds real per-node work. An
  earlier capture of the same case at a different load gave −3.9 % with narrowly
  disjoint CIs, which reinforces rather than rescues the reading: the effect, if
  any, is small enough to be load-dominated. **−7.9 % should not be quoted.**
- **hyperlink policy, with title:** −3.5 % vs a recorded −3.7 %. The CIs are
  disjoint here, so a small real win is present, but it is a ~3 % effect on this
  host's noise floor and should be quoted as "small, ~3 %", not leaned on.
- Absolute baselines drifted up from the record on the populated shapes (116.7 vs
  97.0 µs). Under interleaved sampling that does not touch the ratios, but it
  does mean the recorded absolutes are not reproducible as absolutes.
- **Scope gap:** like the original record's results table, this harness measures
  `apply_link_policy` only. `apply_image_policy` is named in the predeclared
  target operation but was **not measured** in either run and has **no retained
  samples**. Its equivalence is proven by
  `finding_35_7::image_policy_matches_the_pre_optimization_applier`; its
  performance is unmeasured.
- **Not reproduced here:** the record's end-to-end figure (`full
  as_terminal(toc_large, 1000 links)` = 3237.0 µs and the derived "~0.44 % of a
  render"). It came from the same deleted harness and still has **no retained
  samples**.

## Disposition

**ACCEPTED (implementation win at the target operation)** — unchanged, but two of
the four supporting numbers are corrected.

The predeclared floor is written against the **empty-policy target**, and that is
where the evidence is strongest and reproduces: −22.0 % and −12.3 %, both well
past the >= 5 % floor with disjoint CIs. The populated-policy shapes carry a 0 %
permitted-regression budget, and neither regressed — so the budget holds — but
the recorded win on the hyperlink/no-title shape (−7.9 %) is an **overclaim**
that this run contradicts and should be restated as "no detectable difference".

The qualitative case for retention is unaffected: two clones removed, no new
state, no new branch, no downside case measured.
