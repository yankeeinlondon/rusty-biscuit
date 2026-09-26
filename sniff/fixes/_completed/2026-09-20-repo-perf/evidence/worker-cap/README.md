# Worker cap: 12 → 4 (review 2 decision)

**Verdict:** capping the fallback walk at `min(available_parallelism, 4)`
workers (the author's option 2, decided 2026-09-26) keeps the large-tree gain,
cuts the tiny-tree floor by about 40%, and removes most of the concurrent
tail-latency and memory regression. The remaining small-tree cost is accepted.

## Provenance

| Item | Value |
|---|---|
| Host | the Phase 4 host (M4 Max, 16 logical CPUs, macOS); see [`../environment.md`](../environment.md) |
| Code | the `fix/sniff` working tree on 2026-09-26 with `MAX_NESTED_WALK_WORKERS` in `nested.rs` |
| Sides | `w12` and `w4`: the same tree built twice, with only `MAX_NESTED_WALK_WORKERS` set to 12 or 4. `w12` reproduces the previous production policy (`ignore`'s default is the same `min(available_parallelism, 12)`) |
| Binaries | the Phase 4 probe ([`../after/probes/repo_perf_probe.rs.txt`](../after/probes/repo_perf_probe.rs.txt), copied as an untracked example and removed afterwards) in release and debug, and the `perf` Criterion bench in release; hashes in [`binaries.sha256`](binaries.sha256) |
| Corpus | this checkout (`/Volumes/coding/wt/rusty-biscuit/fix-sniff`) for both sides |
| Cache and load | warm cache. The 1-minute load average was 4–18 (logged per run); much quieter than Phase 4's 11–90 |
| Order | probes: p1 `w12`→`w4`, p2 `w4`→`w12`. Criterion brackets: r1 `w12`→`w4`, r2 `w4`→`w12`, r3 `w12`→`w4` |
| Script | [`campaign.sh.txt`](campaign.sh.txt); tables produced by [`summarize.py.txt`](summarize.py.txt) into [`summary-table.md`](summary-table.md) |
| Counters | `w4` on the corpus: `nested_marker_walks` 1, `read_dirs` 2, `manifest_parses` 83, `metadata_probes` 369, the same as Phase 4 ([`probes/w4-counters-corpus.txt`](probes/w4-counters-corpus.txt)) |

Debug corpus brackets were not rerun; the release brackets and the debug
tiny-tree probes answer the decision's questions.

## Results

Release unless noted. Pairs are p1 / p2; Criterion cells are the median of
three bracket medians.

| Measure | 12 workers | 4 workers | Change |
|---|---:|---:|---|
| Corpus walk alone (Criterion) | 21.77 ms | 24.09 ms | +2.3 ms |
| Corpus `detect_repo_structure` (Criterion) | 34.25 ms | 38.79 ms | +4.5 ms |
| Corpus serial reference, same binaries | 65.57 ms | 65.33 ms | unchanged |
| Tiny tree `detect_repo_structure` | 3.34 / 3.09 ms | 1.99 / 2.12 ms | about −1.2 ms |
| Tiny tree `detect_repo_structure`, debug | 3.98 / 3.66 ms | 2.70 / 2.57 ms | about −1.2 ms |
| Tiny tree, 1 × 400 throughput | 305 / 333 req/s | 498 / 500 req/s | +55% |
| Tiny tree, 14 × 400 throughput | 4,326 / 4,338 req/s | 4,608 / 4,618 req/s | +6% |
| Corpus, 1 × 20: throughput | 29.8 / 30.4 req/s | 27.6 / 28.1 req/s | −7% |
| Corpus, 1 × 20: CPU user+sys | 4.62 / 4.63 s | 2.19 / 2.15 s | −53% |
| Corpus, 14 × 10: throughput | 53.0 / 54.0 req/s | 55.0 / 54.4 req/s | flat |
| Corpus, 14 × 10: p95 latency | 427 / 381 ms | 315 / 333 ms | about −20% |
| Corpus, 14 × 10: peak RSS | 201 / 170 MB | 157 / 130 MB | about −22% |

Against the Phase 4 serial baseline (`43a08f94e`,
[`../after/README.md`](../after/README.md)), measured under heavier load:

- corpus `detect_repo_structure`: 79.40 → 38.79 ms, still about 2.0× faster;
- tiny tree: 0.49 → about 2.0 ms, +1.5 ms instead of +2.9 ms;
- tiny tree, sequential: about 1,940 → 500 req/s, −74% instead of −84%;
- corpus, 14 concurrent: p95 285–296 → 315–333 ms (+10%, previously +42%);
  peak RSS 150–152 → 130–157 MB (previously 174–196 MB).

The tiny-tree floor remains because any walk with two or more workers pays
thread spawn/join plus `ignore`'s 1 ms idle poll; see
[`../after/worker-diagnostics.md`](../after/worker-diagnostics.md). A host
with one available CPU gets one worker and no floor.
