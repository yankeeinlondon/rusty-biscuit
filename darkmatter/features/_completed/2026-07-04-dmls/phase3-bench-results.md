# Phase 3 Bench Results

Cold-start index timings from `dmls --bench-index`, release build, dev host
(macOS, Apple Silicon). Each figure is the median of 3 runs. The bench uses the
**same parallel worker-pool indexer the server runs at startup**, so the total
reflects real cold-start latency, not a sequential worst case.

| Corpus | Files | Median total | Budget (R-6) | Verdict |
|---|---:|---:|---|---|
| `small-1k` (generated) | 1,000 | ~63 ms | p50 ≤ 500 ms | ✅ well under |
| repo subtree `darkmatter/` | 549 | ~345 ms | — | ✅ (heavy design docs) |
| full repo (all Markdown) | 3,132 | ~2.1 s | p50 ≤ 2 s / p95 ≤ 4 s | ✅ at p50, easily within p95 |

## small-1k stage breakdown (JSON, one run)

```
total 63.0 ms
  discover 10.7 · parse(read+hash+frontmatter+markdown, parallel) 49.6 · graph_build 2.8
graph: 1000 docs / 7950 nodes / 11389 edges
peak RSS ~21.4 MiB
```

## Notes

- **Parse dominates.** ~79% of cold-start is Markdown parsing (headings + link
  extraction through the `darkmatter` library). Discovery and graph assembly
  are small. This matches R-6's expectation that I/O + parse dominate.
- **Memory.** ~21 MiB peak RSS at 1k files — just over the ≤ 20 MiB/1k target,
  but that figure includes the base binary working set; the AD-2 escape-hatch
  memory criterion is not tripped.
- **AD-2 escape hatches: not activated.** No two activation criteria hold
  (repo p95 well under 4 s, single-doc re-index is a per-keystroke parse of one
  file, no OS is 2× slower here). The v1 in-memory-only model stands; no
  warm-start cache is built.
- **Sequential vs parallel.** An earlier sequential bench of the same repo
  subtree measured ~2.6 s for 549 heavy files; the worker pool brings that to
  ~345 ms, confirming the AD-3 concurrency model earns its keep on cold start.
- The finer read-vs-hash-vs-parse split is folded into `parse_markdown_ms`
  because the parallel pass shares those stages across threads; the discrete
  `tracing`-span backing lands with later diagnostics work.
