# Phase 4: before/after performance evidence

**Verdict:** the gain on the representative checkout is real and well beyond
run-to-run variation. However, the spec's escalation rule (plan ruling R7) **is
triggered**: single-request latency on a tiny tree regressed materially, from
about 0.5 ms to about 3.4 ms in release. The evidence goes back to the author
for a decision before Phase 5; see [Decision rule](#decision-rule-and-escalation).

## Provenance

| Item | Value |
|---|---|
| Host, OS, toolchain, lockfile, ignore configuration | unchanged from [`../environment.md`](../environment.md); see its Phase 4 re-check section |
| Baseline side | `43a08f94e` ("land nested-marker walk measurement harness"): the Phase 1 harness with the serial `WalkBuilder::build()` walk |
| After side | `2f4eb5264` (`HEAD` of `fix/sniff` at Phase 4 start). This is the Phase 3 tested code. Its non-test `sniff/lib` code equals the implementation commit `f9af74815`; the only later `nested.rs` change is the `#[cfg(test)]` R2 hook |
| `sniff/lib` diff, baseline → after | `nested.rs` only (`git diff --stat 43a08f94e 2f4eb5264 -- sniff/lib Cargo.lock`) |
| Build location | one fixed-path measurement worktree, `/Volumes/coding/wt/rusty-biscuit/repo-perf-measure`, detached first at the baseline commit and then at the after commit, with the same commands for both. Binaries were copied out after each build; their hashes are in [`binaries.sha256`](binaries.sha256). The worktree was removed after the campaign; either side can be rebuilt from its commit |
| Corpus | `/Volumes/coding/wt/rusty-biscuit/fix-sniff` (this checkout, same root spelling as Phase 1) for **both** sides: 12,908 walked entries, 116 markers, 112 candidates, printed at every bench registration |
| Uncommitted state | the corpus checkout was clean apart from this fix directory's Phase 4 evidence. That was written to `/tmp` during timing and copied in afterwards, so the corpus did not change between brackets (every bracket logs 12,908 entries) |
| Features and profiles | `bench-internals` only; release = Cargo `bench` profile, debug = `--profile dev`, as in Phase 1 |
| Cache treatment | **warm** filesystem cache. Criterion does a 3 s warm-up per case, the probes warm up first, and the corpus was walked repeatedly beforehand. No cold-cache claim is made |
| Host load | **heavy and uncontrolled.** Other sessions pushed the 1-minute load average between 11 and 90 during the campaign; each log records its start and end load. The parallel side is more sensitive to this than the serial side, so it widens the after-side spread |
| Serialization | every timed invocation ran back to back from one shell. This implementation ran no builds or tests during a timed region |

### Deviation from ruling R3

R3 asks for `git switch` in a fixed-path worktree, measuring each side against
that worktree's own checkout. I built both sides in the fixed-path worktree
but measured both against **this** checkout. That keeps the corpus
byte-identical across sides; switching would have changed the walked tree by
the size of the `nested.rs` diff. R3 wants the root spelling and the contents
held constant, and this holds both constant.

## Isolated walk A/B and end-to-end A/B

The per-bracket data is in [`summary-table.md`](summary-table.md), and the raw
Criterion `sample.json` files and logs are in `raw/`. A sample's time is
`time / iters`. Each case had 3 s of warm-up and 20 flat samples.

The alternation order, three brackets:

- r1: baseline-release → after-release → baseline-debug → after-debug
- r2: after-debug → baseline-debug → after-release → baseline-release
  (reversed)
- r3: baseline-release → after-release → baseline-debug → after-debug

In the table below, the per-bracket medians are listed in r1, r2, r3 order.
The headline figure is the median of those three.

| profile | case | side | per-bracket medians (ms) | median (ms) |
|---|---|---|---|---:|
| release | walk: serial reference | after binary | 66.53, 68.52, 64.67 | 66.53 |
| release | walk: production | after binary | 22.81, 20.54, 22.57 | **22.57** |
| release | walk: production | baseline binary | 108.22¹, 65.18, 65.80 | 65.80 |
| release | `detect_repo_structure` | baseline | 80.98, 79.40, 78.11 | 79.40 |
| release | `detect_repo_structure` | after | 38.92, 33.30, 35.71 | **35.71** |
| debug | walk: serial reference | after binary | 173.50, 173.83, 177.55 | 173.83 |
| debug | walk: production | after binary | 53.00, 42.14, 64.61 | **53.00** |
| debug | walk: production | baseline binary | 182.67, 172.07, 169.11 | 172.07 |
| debug | `detect_repo_structure` | baseline | 243.66, 252.14, 238.88 | 243.66 |
| debug | `detect_repo_structure` | after | 109.26, 106.91, 106.51 | **106.91** |

¹ This bracket ran under a load average of about 51, the campaign peak; its
per-sample range was 77.5–166.2 ms.

### Isolated walk

The comparison is same-process: the serial reference against the parallel
walk, in the same binary, profile, and fixture.
- **release:** 66.53 → 22.57 ms, **2.95×** faster (−44.0 ms).
- **debug:** 173.83 → 53.00 ms, **3.28×** faster (−120.8 ms).

The baseline binary's own `production` row (the pre-change walk) agrees with
the serial reference: 65.80 ms release and 172.07 ms debug.

For context only, the historical figures were 61 → 18 ms release and
172 → 30 ms debug. The after-side debug walk is the most load-sensitive
number in the campaign: its bracket medians run from 42 to 65 ms.

### End-to-end (public `detect_repo_structure(<repo root>)`)

- **release:** 79.40 → 35.71 ms, **2.22×** faster (−43.7 ms).
- **debug:** 243.66 → 106.91 ms, **2.28×** faster (−136.8 ms).

Every measured request entered the fallback walk (`nested_marker_walks = 1`
on both sides; see [`../counters/README.md`](../counters/README.md)).

### Worker count

On the corpus, all **12** default workers participated on every run; see
[`worker-diagnostics.md`](worker-diagnostics.md).

## Re-attributed composition (after side)

"Other work" is `detect_repo_structure` minus the isolated `production` walk,
using the medians above:

| profile | side | `detect_repo_structure` | walk | other work | walk share |
|---|---|---:|---:|---:|---:|
| release | baseline | 79.40 ms | 65.80 ms | ≈13.6 ms | ≈83% |
| release | after | 35.71 ms | 22.57 ms | ≈13.1 ms | ≈63% |
| debug | baseline | 243.66 ms | 172.07 ms | ≈71.6 ms | ≈71% |
| debug | after | 106.91 ms | 53.00 ms | ≈53.9 ms | ≈50% |

- **Release:** "other work" is unchanged (13.6 → 13.1 ms), as expected when
  only the walk changed.
- **Debug:** the apparent drop in "other work" is **not** a real saving. The
  subtraction assumes the walk inside the request costs what the isolated
  walk costs. The after-side isolated debug walk varied from 42 to 65 ms
  between brackets, so the debug "other work" figure is uncertain by about
  ±11 ms.
- **Historical figure:** about 60 of 232 ms debug went to other work. Phase 1
  remeasured it at about 71.5 ms, and this phase at about 71.6 ms (baseline
  side).

## Tiny tree and concurrent detections

Two rounds ran, alternating the sides (p1: baseline then after; p2: after
then baseline). The raw output is in `probes/`. The disposable probe source
is [`probes/repo_perf_probe.rs.txt`](probes/repo_perf_probe.rs.txt); it was
built as an untracked example in the measurement worktree and never
committed. Resource use comes from `/usr/bin/time -l`.

Tiny tree: 8 files, a Cargo workspace with two member crates and one
`package.json` directory; 3 candidates. The fallback is entered on both
sides (`nested_marker_walks = 1`).

### Single-request latency

These are medians of 20 samples × 200 iterations. The case order rotated
every sample.

| profile | case | baseline p1 / p2 (µs) | after p1 / p2 (µs) | change |
|---|---|---:|---:|---|
| release | walk (production) | 227.8 / 228.1 | 3,056.1 / 3,008.1 | **+2.8 ms (≈13×)** |
| release | `detect_repo_structure` | 489.2 / 497.2 | 3,379.1 / 3,340.4 | **+2.87 ms (≈6.8×)** |
| debug | walk (production) | 470.0 / 459.1 | 3,699.6 / 3,629.0 | **+3.2 ms (≈7.9×)** |
| debug | `detect_repo_structure` | 815.8 / 786.2 | 4,059.1 / 4,031.4 | **+3.24 ms (≈5.0×)** |

In the same after binary, the serial reference stays at 233–235 µs release
and 460–462 µs debug. So the regression is the parallel walker's fixed cost,
not a host shift.

### Concurrent detections

These are release builds of `detect_repo_structure`. Here t14 means 14
threads, the local nextest concurrency (`test-threads = -2` on 16 CPUs).
Each row gives p1 / p2.

| tree | threads × requests | side | throughput (req/s) | median latency | p95 latency | CPU user+sys (s) | max RSS | involuntary ctx switches |
|---|---|---|---:|---:|---:|---:|---:|---:|
| tiny | 1 × 400 | baseline | 1,895 / 1,990 | 0.51 / 0.50 ms | 0.67 / 0.53 ms | 0.21 / 0.20 | 13.9 MB | 36 / 18 |
| tiny | 1 × 400 | after | 305 / 298 | 3.49 / 3.47 ms | 4.96 / 4.95 ms | 0.46 / 0.45 | 14.4 MB | 11,381 / 11,503 |
| tiny | 14 × 400 | baseline | 4,168 / 4,150 | 3.24 / 3.27 ms | 4.53 / 4.55 ms | 17.3 / 17.6 | 17.2 MB | 14,917 / 11,365 |
| tiny | 14 × 400 | after | 4,079 / 4,070 | 3.46 / 3.38 ms | 4.28 / 4.27 ms | 12.2 / 12.2 | 23.3 MB | 176,780 / 176,033 |
| corpus | 1 × 20 | baseline | 12.2 / 12.7 | 80.7 / 77.8 ms | 90.5 / 83.5 ms | 1.90 / 1.82 | 32.6 / 36.4 MB | 209 / 279 |
| corpus | 1 × 20 | after | 29.3 / 29.6 | 34.2 / 33.7 ms | 35.3 / 35.9 ms | 4.71 / 4.57 | 35.3 / 33.9 MB | 5,600 / 4,412 |
| corpus | 14 × 10 | baseline | 52.1 / 54.4 | 263.6 / 253.8 ms | 295.5 / 284.3 ms | 34.9 / 33.4 | 150.1 / 152.5 MB | 37,056 / 35,371 |
| corpus | 14 × 10 | after | 51.0 / 53.1 | 257.1 / 238.6 ms | 420.2 / 418.7 ms | 40.2 / 38.1 | 196.2 / 173.6 MB | 65,525 / 66,999 |

What the table shows:

- **Sequential tiny requests** lose about 84% of their throughput, since each
  request now waits out the walker's fixed cost.
- **At test-runner concurrency (tiny t14)**, throughput is flat (−2%, inside
  the p1/p2 spread). Median latency rises about 0.2 ms, p95 and max fall,
  and CPU drops. Baseline tiny t14 was already system-time bound (about
  16 s of sys across 14 threads). About 12× more involuntary context switches
  and about 6 MB more peak RSS go with it.
- **One large request at a time (corpus t1)** is 2.4× faster in throughput,
  but uses about 2.5× the CPU per request.
- **Concurrent large requests (corpus t14)** show:
  - throughput flat (−2%);
  - median latency flat to slightly better;
  - p95 latency **+42%** (about 285–296 → 419–420 ms);
  - CPU **+15%**;
  - peak RSS **+14–31%**.
  Fourteen concurrent walks, each spawning 12 workers, oversubscribe 16 CPUs.

## Mechanism of the tiny-tree floor

The mechanism is summarized in
[`worker-diagnostics.md`](worker-diagnostics.md). In `ignore` 0.4.25, idle
workers poll with `thread::sleep(1 ms)`, and the walk finishes only after
every worker goes idle. On top of that, each walk spawns its workers fresh.

The worker sweep on the tiny tree shows how the floor grows:

| workers | tiny-tree walk |
|---|---:|
| 1 | 0.27 ms |
| 2 | 1.58 ms |
| 4 | 1.70 ms |
| 8 | 2.18 ms |
| 12 (default) | 3.01 ms |

On the corpus, 4 workers give 23.1 ms against 19.3 ms for the default 12.

## Decision rule and escalation

**Gain against variation (representative checkout).** The rule requires the
gain to exceed the observed run-to-run variation. Here it does, by a wide
margin:
- The worst after-side bracket median (release walk 22.81 ms, debug walk
  64.61 ms) is far below the best baseline bracket median (64.67 ms and
  169.11 ms).
- The same holds end to end: release 38.92 against 78.11 ms, debug 109.26
  against 238.88 ms.
- The spread between the highest and lowest bracket median of one case
  reaches 3.9 ms for the release walk, 22.5 ms for the debug walk (after
  side), 5.6 ms release end to end, and 13.3 ms debug end to end. The
  exception is the baseline `production` release r1 bracket, which ran at
  load 51 (¹ above). Even that outlier only widens the baseline side, which
  strengthens the result.

**Claims kept separate:**
- The walk is 2.95× faster in release and 3.28× faster in debug.
- The command, `detect_repo_structure` on this checkout, is 2.22× faster in
  release and 2.28× faster in debug.
- No compose-suite claim is made; that suite was not measured.
- The historical 172 → 30 and 61 → 18 ms figures are context only. This
  host's debug after-side walk (53 ms) falls short of the historical 30 ms
  under a load average of 17–90.

**R7 material-regression condition: triggered.**
- Tiny-tree single-request latency regressed by an order of magnitude in
  relative terms: +2.9 ms release and +3.2 ms debug per
  `detect_repo_structure`, about 5–7× the baseline.
- Sequential small-request throughput fell about 84%.
- Concurrent throughput did not regress (−2%, within noise), but
  concurrent large-tree tail latency (p95 +42%) and resource use (CPU +15%,
  RSS up to +31%) did.

Every caller of the structure-only path pays this fixed cost per request.
These callers run the fallback walk rather than reusing walk evidence:
- `RepoRequest::structure()`;
- `sniff repo` commands;
- claudine completion and composition;
- darkmatter's compose snapshot;
- sniff recent-commits.

Per the spec's "Alternatives and open questions" section and ruling R7,
this phase **stops and escalates**:
- The implementation is unchanged.
- No adaptive or shared-pool scheduling was added.
- No worker cap was changed.

The options and a recommendation are recorded for the author in the spec's
`human_review_items`.
