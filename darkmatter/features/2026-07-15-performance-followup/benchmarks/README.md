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
- the predeclared minimum repeatable win and maximum permitted control
  regression (declared **before** the baseline is captured);
- raw result files (retained, not just summaries).

`../results.md` links each disposition to its run record. Interactive (PTY) and
piped (redirected CLI) measurements are reported separately.
