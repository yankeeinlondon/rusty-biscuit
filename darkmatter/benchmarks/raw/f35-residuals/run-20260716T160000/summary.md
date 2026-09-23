# Phase 10 — Finding-35 residuals: run record

- **Run id:** `run-20260716T160000`
- **Checkpoint:** Phase 10 (35.2, 35.3, 35.5, 35.6, 35.7)
- **Baseline commit:** `b425fb466` + the Phase 1–9 working tree (this branch's
  pre-Phase-10 state). Each sub-item's baseline is the algorithm it replaced,
  carried as a pinned copy next to the candidate — see *Measurement method*.
- **Candidate:** the Phase-10 working tree.
- **Profile:** `release`.
- **Host:** macOS (Darwin 25.5.0), aarch64. **Shared and heavily loaded**
  (load average ~29–30; concurrent Spotlight index + parallel `rustc`).
- **TTY mode:** none — every measurement is a non-interactive process.

## Measurement method (and why it deviates from `--save-baseline`)

The evidence contract requires baseline and candidate to differ only in the code
under test. The usual Criterion approach — `--save-baseline` then `--baseline` —
also requires a *quiet* host, because it compares two separate runs separated in
time. This host is not quiet: **identical, unmodified code re-measured across
runs drifted by +50%** (`f35_2_relevel_no_headings` read 290 µs → 397 µs →
420 µs on three consecutive runs of the same binary). A first attempt at 35.2
used cross-run baselines and reported a spurious "+5% control regression" that
this method later disproved (the control is at parity).

Every number below is therefore captured with **baseline and candidate sampled
in the same process, interleaved**, so both see the same thermal and scheduling
conditions and the *ratio* remains sound. The baseline copies are pinned to the
pre-change algorithms by differential equivalence tests, so they cannot silently
drift from what they claim to represent.

Where the target function is private, it is measured by a **temporary in-crate
harness** whose output is retained here and whose code was deleted (the
precedent Phase 8 set for F25). Exposing a private function purely to benchmark
it would be exactly the public API addition the standing contract bars.

## Dispositions (one per sub-item — no aggregate number)

| Sub-item | Target operation | Result | Disposition |
|---|---|---|---|
| **35.2** `relevel_with_overflow` | re-level `toc_large` (1001 headings, ~80 KB) | prefix **25.351 ms → 314.93 µs (−98.8 %)**; overflow **24.347 ms → 361.50 µs (−98.5 %)**; extract-only **18.463 ms → 277.80 µs (−98.5 %)**; heading-free control 230.59 → 220.62 µs (parity) | **Accepted** |
| **35.3** `Arc<str>` fetch bodies | register + fetch + `get_content` | net **pessimization** (+1.29 µs `::file`, +0.50 µs `::code` per typical document); whole copy budget = **0.125 %** of a loopback fetch vs a ≥5 % floor | **No-win → reverted** |
| **35.5** `md hash --diff`/`--save` | one CLI invocation | CLI **17.2 ms → 14.1 ms (−18.0 %)** on large `detailed --diff`; library sequence −29.3 %; controls within σ | **Accepted** (residual recorded) |
| **35.6** `normalize_body_rhythm` | rhythm pass over a decorated body | decorated **164.8 → 14.8 µs (−91.1 %)**; code panel −93.3 %; escape-free control −30.3 % (no regression); decorated render ≈**20 % faster** overall | **Accepted** |
| **35.7** link/image policy appliers | `apply_link_policy` over a document's links | empty policy/no title **72.6 → 58.2 µs (−19.9 %)**; all four shapes improved, none regressed; ≈0.44 % of a full render | **Accepted** on the target-operation win |

Thresholds were declared before each capture and are recorded in the per-item
files below. No sub-item's number is derived from another's; 35.3's rejection is
visible on its own row rather than being absorbed into a phase-level total.

## Files

- `criterion-f35_2_relevel_*-{baseline,candidate}.json` — Criterion estimates for
  the four 35.2 groups (`cargo bench -p darkmatter --bench phase10_residuals`).
- `f35_3-copy-cost-model.txt` — 35.3 copy-cost model + fetch proportion; the
  no-win record and the reason the code was removed.
- `f35_5-hash-artifact-profile.txt` — 35.5 library sequence timings + hyperfine
  CLI runs, and the recorded residual.
- `f35_6-rhythm-profile.txt` — 35.6 interleaved rhythm-pass timings + render share.
- `f35_7-link-policy-profile.txt` — 35.7 interleaved applier timings + render share.

## Cross-platform classification (per sub-item, from the actual diff)

All five are **OS-identical**, each confirmed against the shipped diff rather
than the finding number:

- **35.2** — byte/line scanning and string assembly. No `cfg`, no filesystem.
- **35.3** — nothing shipped. The inspected path (per the plan's instruction not
  to preclassify it) is slot storage plus an in-memory copy; the Tokio/reqwest
  runtime below it was never touched.
- **35.5** — call-graph restructuring only. The CLI still owns `fs::write`; no
  path handling, no clock, no `cfg`.
- **35.6** — in-memory regex predicate over a `&str`. The regex is the same
  shared `ANSI_ESCAPE_RE` static used on every platform.
- **35.7** — borrow-vs-clone in two in-memory node appliers.

Per the Verification Matrix, OS-identical findings are satisfied by Windows
compile evidence + the macOS behavioral run + ordinary Linux CI. **No Phase-10
sub-item adds an OS-divergent path**, so this phase contributes no new
Linux/Windows behavioral-run obligation to the Phase 11 closeout (the F17 and
F22 gaps recorded there are unaffected).
