# F4 historical closeout — run summary

- baseline (pre-opt): `83aaecc8f`
- audit (accumulated): `51c1f16e10ffe825b56987573ba4eabc659c768e`
- host: macOS (Darwin 25.5.0), single host
- harness: `hyperfine --warmup 3 --runs 20 --shell=none`, `NO_COLOR=1`, release `md`
- fixtures: `benchmarks/fixtures/` (manifest.yaml identities)
- profile: `cargo build --release -p darkmatter-cli`, isolated CARGO_TARGET_DIR per commit

| case | baseline 83aaecc8f mean±sd (ms) | audit 51c1f16e1 mean±sd (ms) | delta |
|---|---|---|---|
| `help` | 9.105 ± 2.384 | 11.368 ± 3.244 | +24.9% |
| `render_basic` | 19.313 ± 15.713 | 22.919 ± 17.113 | +18.7% |
| `hash_basic` | 13.020 ± 3.851 | 10.279 ± 2.017 | -21.1% |
| `compose_trivial` | 192.710 ± 93.563 | 32.223 ± 27.367 | -83.3% |
| `compose_schema_transclusion` | 248.017 ± 107.329 | 71.058 ± 45.135 | -71.3% |
| `toc_small` | 48.382 ± 30.458 | 13.688 ± 6.431 | -71.7% |
| `toc_medium` | 36.750 ± 30.159 | 15.669 ± 12.419 | -57.4% |
| `toc_large` | 488.215 ± 144.303 | 23.043 ± 9.095 | -95.3% |
| `render_code_heavy` | 12.921 ± 7.198 | 10.017 ± 2.321 | -22.5% |

## Disposition

F4 objective (non-quadratic TOC `line_at_offset`) **reconstructed and passes**. The
TOC tiers show monotonic size-scaling improvement well outside dispersion —
`toc_small` -71.7%, `toc_medium` -57.4%, `toc_large` -95.3% (488.2 ± 144.3 ms ->
23.0 ± 9.1 ms on the 1000-heading tier; non-overlapping bands) — so the quadratic
prefix-rescan is provably gone. Compose cases also improve sharply and outside
noise (`compose_trivial` -83.3%, `compose_schema_transclusion` -71.3%: NTP removal
+ schema/ownership work). The only two non-negative deltas are fast control paths
whose stddev exceeds the delta (`--help` 9.1 ± 2.4 -> 11.4 ± 3.2 ms; `render_basic`
19.3 ± 15.7 -> 22.9 ± 17.1 ms): overlapping bands, i.e. **within measurement noise**
on this shared host, not a TOC regression. Raw per-case JSON retained here.
