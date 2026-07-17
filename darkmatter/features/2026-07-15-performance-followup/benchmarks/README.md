# Benchmark Evidence Home (Architecture Decision A)

This directory is the feature-local evidence home for the performance
follow-up. It separates **immutable fixture identity** (this directory's
`manifest.yaml` + `fixtures/`) from **per-run measurement facts** (`raw/`), so a
fixture can never silently change under a checkpoint's baseline.

```
benchmarks/
├── README.md        # this file — manifest schema, runners, run-record contract
├── generate.sh      # deterministic fixture generator (version + command)
├── manifest.yaml    # single authority for fixture identity (recomputed-verified)
├── fixtures/        # committed, byte-identical fixtures (generate.sh output)
└── raw/<checkpoint>/<run-id>/   # dated run records: one per measurement
```

## Fixture manifest schema (`manifest.yaml`)

`manifest.yaml` is the single authority for fixture identity across every
checkpoint. It is regenerated only through the committed generator and verified
by `darkmatter/lib/tests/benchmark_fixtures.rs`
(`benchmark_manifest_matches_recorded_identities`), which recomputes each field
from the committed bytes and fails on any drift.

```yaml
generator:
  version: <semver>          # bump on any change to emitted fixture bytes
  command: <exact command>   # `bash generate.sh`
fixtures:                    # ordered collection (stable id order; do not reorder)
  - id: <file stem>
    path: fixtures/<name>.md
    provenance: generated    # every fixture is generate.sh output
    bytes: <exact byte size>
    lines: <newline count>            # structural size
    headings: <parser heading count>  # structural count
    frontmatter_hash: <hex>  # Darkmatter Markdown-aware `hash_frontmatter`
    body_hash: <hex>         # Darkmatter Markdown-aware `hash_body`
    darkmatter_hash: <hex-hex>   # `md hash <file>` frontmatter-body identity
    xxhash64: <hex>          # biscuit-hash xxHash whole-file byte identity
```

Hashing authority: Markdown identities use Darkmatter's Markdown-aware hashing
(`hash_frontmatter` / `hash_body`, byte-identical to `md hash <file>`); the
whole-file byte identity uses `biscuit-hash` xxHash. No ad hoc hashing.

### Regenerating fixtures + manifest

1. Edit `generate.sh` and bump `generator.version` (in the script and the
   `benchmark_fixtures.rs` emitter).
2. `bash generate.sh` — rewrites `fixtures/` deterministically.
3. `DM_BENCH_EMIT=1 cargo nextest run -p darkmatter --test benchmark_fixtures` —
   rewrites `manifest.yaml` from the new bytes.
4. `cargo nextest run -p darkmatter --test benchmark_fixtures` — verifies.

A checkpoint-specific fixture may be added only by registering and hashing it
here **before** that checkpoint captures its baseline.

## Fixture set (Phase 2)

| id | measurement case |
|----|------------------|
| `render_basic` | render (prose + headings + list + code block) |
| `hash_basic` | `md hash` (frontmatter + body) |
| `compose_trivial` | trivial compose (interpolation only) |
| `compose_schema_transclusion` (+ `compose_child`) | schema validation + `::file` transclusion |
| `toc_small` / `toc_medium` / `toc_large` | three TOC size tiers (12 / 120 / 1000 headings) |
| `render_code_heavy` | code-heavy render (40 fenced blocks) |

`md --help` is a command-level case with **no** file fixture; it is measured by
the release CLI runner and recorded in a run record, not the manifest.

## Runner contracts

One manifest does **not** imply one universal runner. `just bench` is a
Criterion runner only; CLI and PTY evidence must not be forced through it.

1. **Criterion microbenchmarks** — the existing `darkmatter/lib/benches/*`
   targets, run via the area `just bench` recipe. Establish mechanism-level
   deltas for library paths. Each records commands, environment, and raw
   Criterion sample output under `raw/<checkpoint>/<run-id>/`.
2. **Release CLI runner** — a release `md` binary invoked on manifest fixtures
   (and the fixture-less `md --help` case) for command-level, user-visible
   impact. Records baseline/candidate commits, release profile, host,
   environment, TTY mode, warm-up, sample count, statistic/dispersion, and raw
   timing files under `raw/<checkpoint>/<run-id>/`.
3. **Biscuit Terminal probe / PTY path** — the existing
   `biscuit-terminal/lib/examples/discovery_probe.rs` +
   `biscuit-terminal/lib/tests/common/pty.rs` path, extended in Phase 3 for
   interactive OSC-request and repeated-construction latency evidence. Not
   routed through `just bench`.

Each runner writes a dated run record linked from `../results.md` and consumes
this shared manifest wherever it uses file fixtures.

## Run-record contract (`raw/<checkpoint>/<run-id>/`)

Per-run facts stay out of the immutable fixture identity. Each measurement
writes a dated run record recording:

- baseline and candidate commits;
- exact commands and build profile (release);
- host facts, environment, and TTY mode (interactive vs piped);
- warm-up count, sample count, statistic, and dispersion;
- tool versions (`rustc`, `cargo`, `criterion`, `hyperfine`) and host load;
- the predeclared minimum repeatable win and maximum permitted control
  regression (declared **before** the baseline is captured);
- raw result files (retained, not just summaries) — see *Raw samples* below.

### Raw samples (mandatory)

"Raw result files" means the **per-observation vectors**, not Criterion's
derived statistics. Retaining `estimates.json` is **not** sufficient and was
rejected by review-1: it carries only `mean` / `median` / `slope` / `std_dev`,
from which nothing can be recomputed or independently checked.

- **Criterion:** retain `target/criterion/<bench>/new/sample.json` as
  `<bench>-sample.json` (or `<bench>-{baseline,candidate}-sample.json`). It holds
  parallel `iters` / `times` vectors; per-iteration time is `times[i]/iters[i]`.
- **Hyperfine:** retain the `--export-json` file (its `times` array is already
  per-run).
- **Any other harness:** retain the individual observations, not a median.

`recompute.ts` beside this file regenerates mean / median / std dev / min / max
/ bootstrap 95 % CI from a run record's retained vectors:

```
bun recompute.ts raw/<checkpoint>/<run-id>
```

Every statistic quoted in a `summary.md` must be reproducible by that command
from the vectors committed next to it.

### Harnesses are retained, not deleted

A benchmark harness that carries a **pinned baseline copy** of a replaced
algorithm must be committed and kept (see `f13_scan_and_replace` in
`darkmatter/lib/benches/phase6_interpolation.rs`), gated by an in-process
equivalence assertion so the pinned copy cannot drift from what it represents.

Deleting the harness after capture — the Phase-8 "temporary in-crate harness"
precedent — is what left Findings 25, 35.3, 35.5, 35.6 and 35.7 with
unrecoverable observations and no way to reproduce their claims. Do not repeat
it. Measuring a private function is not a reason to delete the harness; a bench
target is test-tier code and adds no public API.

### Where to put a harness (private vs public target)

The Phase-8 deletions were rationalized as unavoidable: "exposing a private
function purely to benchmark it would be the public API addition the standing
contract bars." That is correct about the API and wrong about the remedy — the
choice was never "widen the API or delete the harness". Pick by what the target's
visibility is:

| Target visibility | Home | Why |
|---|---|---|
| `pub` | `darkmatter/lib/benches/*.rs` (Criterion) | A bench is a separate compilation unit; it sees only `pub` items. |
| crate-private | `darkmatter/lib/src/perf_harness.rs` + a `#[cfg(test)]` caller in the target's own module | Lives *inside* the crate, so it reaches private items, and `#[cfg(test)]` ships in no artifact — **no public API is added**. |

Never widen a production item's visibility to measure it.

In-crate harness tests are `#[ignore]`d **and** gated on `Harness::from_env`
(`DM_PERF_RAW_DIR`), so the ordinary `just test` gate neither runs nor is slowed
by them:

```
DM_PERF_RAW_DIR=<abs run-record dir> \
  cargo nextest run -p darkmatter --lib --release \
  --run-ignored all -E 'test(<harness_test_name>)' --no-capture
```

They emit Criterion's `sample.json` shape, so `recompute.ts` reads them through
the same path as Criterion's own vectors.

### Cross-run comparison requires a drift bracket

This host is shared. Identical unmodified code has been observed drifting
**+50 %** across runs under load (Phase 10), and a cross-run F33 comparison
under load ~29–30 manufactured a systematic-looking **−19 %** shift across
*every* benchmark in the binary that vanished at load ~8.

Prefer sampling baseline and candidate **interleaved in one process** (pinned
baseline). Where that is impossible (the baseline is a whole private function),
**bracket** the baseline with a candidate run on each side, record the observed
drift, and record the host load average per run. A delta smaller than the
measured bracket drift is not a result.

`../results.md` links each disposition to its run record. Interactive (PTY) and
piped (redirected CLI) measurements are reported separately.
