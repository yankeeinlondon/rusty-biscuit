---
created: 2026-07-17
phase: 5
purpose: same-session Criterion evidence for the redundant-walk fix
verdict: improvement threshold NOT met under the original spec; superseded by the 2026-07-18 spec amendment (review 1) — evidence satisfies the amended guards
---

# Results — Eliminate Redundant Reference-Graph Verification

## Amendment (2026-07-18, review 1)

`spec.md`'s §"Performance acceptance" and acceptance criterion 8 were amended
per review 1's High finding, replacing the disproven ≥10%/≥500 µs threshold
with a mechanism-based requirement plus guards calibrated to the measured
effect. Evaluated against the amended guards, the evidence recorded below
satisfies them all:

- mechanism guard ✔ — named seams (`validate_fresh_graph` /
  `validate_with_graph` / `validate_graph_contents`) plus the
  `fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph` test;
- improvement guard ✔ — 461 µs quiet-window median delta ≥ 100 µs
  (≈159 µs by same-run decomposition);
- regression guard ✔ — small −16.6%, large −2.1%, multi −4.4% (all improved
  or flat);
- prebuilt-gap guard ✔ — small 6.6×, large ≈15×, multi 2.4×.

The historical verdict and measurements below are preserved as recorded
against the original, superseded threshold.

## Verdict (read first)

The fix is functionally correct and removes the redundant descendant walk from
the one-step path (the Phase-4 mechanism test and the decomposition below both
confirm it). However, the spec's acceptance threshold for
`multi_transclusion/build_and_validate` — improve by **both ≥10% and ≥500 µs**
— is **not met and is not reachable on this fixture**. Direct same-run
decomposition shows the redundant walk costs ≈ **160 µs (≈1.5% of
`build_and_validate`)**, not the ~4.15 ms the spec assumed:

- `validate_prebuilt` (unchanged checked path) = verify + flatten + engine =
  **4.1616 ms**
- `build_and_validate` (fixed path) − `construct` = flatten + engine alone =
  10.066 − 6.063 = **4.003 ms**
- ⇒ descendant re-verification ≈ **159 µs**. The 4.15 ms floor is the *shared
  validation engine* (flattening, fragment preparation — which composes each
  referenced document for heading slugs — and per-record validation), which
  this fix intentionally keeps in both paths.

The spec's premise ("the approximately 4.15 ms prebuilt-validation floor is
dominated by reopening and hashing those 12 children") is falsified: reopening
and hashing the 12 children is ~4% of that floor. The best measured
improvement on `multi_transclusion/build_and_validate` is ≈ **160–460 µs
(1.5–4.4%)**, below the ≥10% and ≥500 µs bar on both criteria. The regression
guard and the `validate_prebuilt` gap requirement both pass (below).

The fix remains worthwhile independent of the missed threshold: it eliminates
the one-step hard-error race (spec §"Secondary behavior defect") and removes
provably redundant disk I/O. The threshold was derived from the same
misattribution it tests; meeting it would require the redundant walk to cost
6× more than it does.

## Commit / worktree state

| Item | Baseline (pre-fix) | Candidate (with fix) |
|---|---|---|
| `git rev-parse HEAD` | `d672388dd0fed4196295e7f21514cac6fa59f0ae` | `d672388dd0fed4196295e7f21514cac6fa59f0ae` |
| Fix source changes | stashed (`git stash push` of the two files below) | applied |
| Fix files | `darkmatter/lib/src/markdown/reference/validate.rs`, `darkmatter/lib/src/markdown/reference/file_tree/mod.rs` | same |
| Bench source xxHash (`bh --file …/benches/reference_graph.rs`) | `10346693882688625444` (verified unchanged before every timed run) | `10346693882688625444` |

Pre-existing dirty paths (unrelated to the fix), present at both baseline and
candidate captures, confirmed benchmark-irrelevant (Markdown docs only):

- `CLAUDE.md` (M)
- `darkmatter/features/2026-07-15-performance-followup/{performance-compliance.md,review-8.md,spec.md}` (M), `review-9.md` (untracked)
- `darkmatter/fixes/2026-07-16-redundant-walk/plan.md` (M — this fix's own plan checkoffs)
- `darkmatter/fixes/2026-07-16-redundant-walk/phase-1-evidence.md` (untracked)

No unrelated Rust source, manifest, lockfile, or benchmark input changed
between any baseline and candidate run. The bench source hash was re-verified
before each timed run; all timed numbers below come from source hash
`10346693882688625444`. (After all timed runs, the Phase-5.5 comment-only
correction changed the bench file's comments only; new hash
`8842533234797574318`; no timed code moved.)

Candidate runs were executed by stashing/restoring exactly the two fix files,
so pre-fix and candidate measurements are same-working-tree, minutes apart.

## Toolchain / parameters

| Item | Value |
|---|---|
| OS / arch (`uname -mrs`) | `Darwin 25.5.0 arm64` |
| `rustc --version` | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| `cargo --version` | `cargo 1.96.0 (30a34c682 2026-05-25)` |
| Hardware | 16 cores, 128 GiB RAM |
| Criterion params | sample count 30, warm-up 1 s, measurement 4 s |
| Named baselines | `redundant-walk-before` (Phase 1, quiet, pre-fix), plus paired-session controls `redundant-walk-paired{,-b}` and `redundant-walk-control` (see host-load notes) |

## Commands

```text
# baseline (Phase 1, quiet host window; saved before any implementation edit)
cargo bench -p darkmatter --bench reference_graph -- build_and_validate \
  --save-baseline redundant-walk-before --warm-up-time 1 --measurement-time 4

# candidate, paired against that baseline
cargo bench -p darkmatter --bench reference_graph -- build_and_validate \
  --baseline redundant-walk-before --warm-up-time 1 --measurement-time 4

# decomposition + final unfiltered coverage (candidate code)
cargo bench -p darkmatter --bench reference_graph -- 'reference_graph/multi_transclusion/' \
  --warm-up-time 1 --measurement-time 4
cargo bench -p darkmatter --bench reference_graph -- 'reference_graph/(small|large)/' \
  --warm-up-time 1 --measurement-time 4

# same-evening pre-fix control attempt (stash → save → restore → compare)
git stash push -- darkmatter/lib/src/markdown/reference/validate.rs \
                  darkmatter/lib/src/markdown/reference/file_tree/mod.rs
cargo bench -p darkmatter --bench reference_graph -- \
  'reference_graph/multi_transclusion/build_and_validate' \
  --save-baseline redundant-walk-control --warm-up-time 1 --measurement-time 4
git stash pop
cargo bench -p darkmatter --bench reference_graph -- \
  'reference_graph/multi_transclusion/build_and_validate' \
  --baseline redundant-walk-control --warm-up-time 1 --measurement-time 4
```

## Host-load observations

The host was shared with concurrent agent workloads all evening (other
worktrees' `cargo` builds/test runs, Spotlight `mds_stores`, a Logitech
updater, a VM). One-minute load oscillated between ~16 and ~116 on
multi-minute timescales. Consequences, handled per plan 5.1/5.2:

- The first candidate run (load ~15→) produced a wide-CI, noncomparable
  `multi_transclusion` reading (+14.8% bogus "regression"); discarded.
- Two fully paired recaptures (`redundant-walk-paired`, `-paired-b`) each
  caught a load burst on exactly one side (bogus −51% and +289% swings on
  identical code); both discarded as noncomparable.
- A load-gated watch (`vm.loadavg` 1-min < 20–22) then yielded genuinely
  quiet windows. All numbers in the primary table below come from tight-CI
  quiet-window runs. `construct` and `validate_prebuilt` in the candidate
  quiet window reproduce Review 4's quiet-host medians within 0.3%, which
  corroborates the window's quietness.
- The same-evening pre-fix control (`redundant-walk-control`) caught a rising
  burst on the pre-fix side (16.061 ms, wide CI) versus a quiet candidate
  side (10.181 ms, tight CI); its −35.7% comparison is bogus and recorded
  only as provenance.
- Criterion's "Unable to complete 30 samples in 4.0s" warning appeared on
  most `multi_transclusion` runs (collection overran to ~4.2–5.3 s); 30
  samples were still collected each time.

## Numbers

### Primary: `build_and_validate`, quiet-window runs (sample count 30 each)

| Fixture | Baseline (pre-fix, Phase-1 saved `redundant-walk-before`) | Candidate (fix) | Δ median | Δ % |
|---|---|---|---|---|
| small | 249.85 µs [241.78, 263.86] | 208.35 µs [204.91, 214.30] | −41.5 µs | −16.6% (µs-scale scheduler noise; empty manifest) |
| large | 6.4407 ms [6.4118, 6.4723] | 6.3065 ms [6.2681, 6.3479] | −134 µs | −2.1% (empty manifest; within drift) |
| multi_transclusion | 10.527 ms [10.446, 10.608] | 10.066 ms [9.968, 10.169] | −461 µs | −4.4% |

A second quiet candidate run measured `multi_transclusion` at 10.181 ms
[10.110, 10.258]; a third (moderate load, tight CI) at 11.031 ms
[10.936, 11.126]. Phase-1 measured the *pre-fix* code a second time at
10.989 ms [10.854, 11.126] under similar moderate load — i.e. identical code
drifts ±4–5% across host states, and the pre-fix and candidate quiet
distributions overlap. The decomposition below, not any single pair of
medians, is the reliable effect estimate.

### Decomposition (candidate code, one quiet window, all three functions)

| Function | Median | CI |
|---|---|---|
| `multi_transclusion/construct` | 6.0632 ms | [6.0103, 6.1165] |
| `multi_transclusion/build_and_validate` | 10.066 ms | [9.9682, 10.169] |
| `multi_transclusion/validate_prebuilt` | 4.1616 ms | [4.1355, 4.1903] |

- flatten + shared engine = b&v − construct = 10.066 − 6.063 = **4.003 ms**
- descendant re-verification (removed by the fix) = vp − (b&v − construct) =
  4.162 − 4.003 = **159 µs ≈ 1.5% of b&v**
- consistency check (pre-fix identity): construct + vp = 6.063 + 4.162 =
  10.225 ms ≈ the pre-fix quiet b&v of 10.527 ms (3% drift) ✓
- `construct` and `validate_prebuilt` match Review 4's quiet-host 6.0571 ms /
  4.1522 ms within 0.3%, confirming both that the checked path is unchanged
  and that the window was quiet.

### Acceptance-threshold check (spec §"Performance acceptance")

| Requirement | Result | Evidence |
|---|---|---|
| `multi_transclusion/build_and_validate` improves by **both ≥10% and ≥500 µs** at the median | **FAIL** — best estimate of the true effect is ≈159 µs (1.5%) by same-run decomposition; best quiet-vs-quiet median delta is −461 µs (−4.4%), which still misses both bars and is partly drift | decomposition table above; primary table |
| No `build_and_validate` fixture regresses by both >5% and >100 µs at the median | **PASS** — small −16.6%, large −2.1%, multi −4.4% (all improved or flat) | primary table |
| Final unfiltered run: `validate_prebuilt` still materially faster than `build_and_validate` for each fixture | **PASS** — small: 31.46 µs vs 208.35 µs (6.6×); large: 426.7 µs vs 6.31 ms (≈15×); multi: 4.162 ms vs 10.066 ms (2.4×) | small/large from the unfiltered candidate run (moderate load, wide CIs on large; verdict insensitive to that noise), multi from the quiet decomposition run |

The unfiltered benchmark was executed as two filtered invocations covering all
nine `reference_graph/{fixture}/{function}` groups on the candidate code
(equivalent coverage to one unfiltered invocation; noted for exactness).

## Interpretation

1. **The redundant walk was real but small.** Removing `verify_descendants`
   from the one-step path saves ≈160 µs on the 12-child fixture — the 12
   re-opens + rehashes of tiny cached files are cheap. The spec's attribution
   of the whole 4.15 ms `validate_prebuilt` floor to that walk is falsified
   by the same-run decomposition above.
2. **The 4.15 ms floor is the shared validation engine** (flattening,
   fragment preparation incl. composing referenced documents for heading
   slugs, per-record local/remote/fragment matching, report construction) —
   work both paths must keep doing, explicitly out of this fix's scope
   (spec §Non-goals).
3. **The fix is correct and its behavioral goals stand**: one graph build with
   no compatibility re-walk in the one-step path (AC1), the FileTree fresh
   seam (AC2), the checked public path untouched (AC3 — `validate_prebuilt`
   numbers unchanged), one shared engine (AC4), the snapshot-vs-stale
   mechanism test (AC5), all existing suites green (AC6), no public surface
   change (AC7). The one-step hard-error race is eliminated.
4. **AC8 is the failing criterion**: the threshold encodes the falsified
   premise. Recommended follow-up (for the user/orchestrator, outside this
   phase): amend the spec's performance-acceptance numbers to the measured
   ≈160 µs / ≈1.5% effect, or enlarge `TRANSCLUSION_CHILD_COUNT` by an order
   of magnitude if a larger signal is wanted — the fixture was frozen for
   this phase, so no such change was made here.
