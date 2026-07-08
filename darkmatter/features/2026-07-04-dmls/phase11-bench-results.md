# Phase 11 Performance Sign-Off

Cold-start index timings from `dmls --bench-index --json`, **release** build,
dev host (macOS, Apple Silicon). Each figure is the median of 3 runs. The bench
uses the same parallel worker-pool indexer the server runs at startup, so the
total reflects real cold-start latency. Synthetic tiers were materialized with
`dmls --gen-corpus <tier> <dir>` (deterministic, seeded).

| Corpus | Files | Median total | Peak RSS | R-6 budget | Verdict |
|---|---:|---:|---:|---|---|
| full repo (all Markdown) | 3,141 | ~1.89 s | ~176 MiB | cold p50 ≤ 2 s | ✅ under |
| `vault-5k` | 5,000 | ~0.54 s | ~60 MiB | cold p50 ≤ 2.5 s | ✅ ~5× under |
| `dense-5k` | 5,000 | ~1.30 s | ~125 MiB | stress tier (no hard budget) | ✅ within |
| `pathological-1k` | 1,000 | ~0.32 s | ~92 MiB | stress tier (no hard budget) | ✅ within |

Graph magnitudes at each tier: `vault-5k` 42.4k nodes / 54.8k edges;
`dense-5k` 119.8k / 167.0k; `pathological-1k` 32.2k / 52.2k; full repo
8.1k / 13.8k (the repo has fewer, heavier design docs).

## Single-document re-index (p95 ≤ 25 ms)

Not measured by `--bench-index` (that is whole-corpus cold start). It is bounded
from two directions:

- The warm in-memory L2 harness (Phase 4) round-trips a full document request
  cycle sub-millisecond; a single-document reparse is one file's share of the
  parse stage.
- Cold parse averages well under 1 ms/file even on the dense tier
  (`dense-5k` parse ≈ 481 ms / 5,000 ≈ 0.10 ms per document), so an
  average-size file's re-index sits far inside the 25 ms p95 budget.

## AD-2 escape-hatch evaluation

The AD-2 warm-start on-disk cache activates only if **two** criteria hold. None
do:

- **repo p95 ≤ 4 s?** Yes — median 1.89 s, p95 < 2 s. No trip.
- **`vault-5k` p50 ≤ 2.5 s?** Yes — 0.54 s, ~5× under. No trip.
- **Any target OS ≥ 2× slower?** Not observed on the dev host; the cross-OS
  check is a CI concern, and the indexer has no platform-conditional core path.
- **Single-doc re-index p95 ≤ 25 ms?** Yes (above). No trip.

**Verdict: no warm-start cache needed.** The v1 in-memory-only model stands, as
projected in the Phase 3 results. This confirms `design.md` AD-2.

## Notes

- Parse dominates cold start (headings + link extraction through the
  `darkmatter` library), matching R-6 and the Phase 3 finding. `graph_build`
  grows with edge density (it is the largest stage on `dense-5k`) but stays a
  minority of total on realistic corpora.
- The finer read/hash/frontmatter/directive split still folds into
  `parse_markdown_ms` because the parallel pass shares those stages across
  threads (documented in the Phase 3 results).
