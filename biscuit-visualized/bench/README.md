# biscuit-visualized benchmark

Times Mermaid rendering for every diagram family and checks the timings
against fixed limits and a stored baseline. It is a standalone Cargo project,
not a member of the rusty-biscuit workspace, so it never runs in CI.

```sh
just bench                                        # from biscuit-visualized/
just bench --families flowchart,pie --iterations 3   # a quick run
```

The corpus is the family fixtures the golden tests use
(`../src/src/tests/fixtures/merman/families/*.mmd`), compiled into the binary.
Each diagram renders to PNG with the default request and the `Default`
palette.

## What is measured

For each family, the run reports the median and the p95 (nearest rank) of:

- **warm**: `MermaidDiagram::render` to PNG (measure, layout, SVG,
  post-process, raster) in a process that has already rendered that family
  once;
- **cold**: a fresh process doing one render, timed from spawn to exit (the
  benchmark runs itself with a hidden `__one-shot` subcommand).

Every iteration, warm and cold, gets its own empty
`BISCUIT_VISUALIZED_CACHE_DIR`, created and checked empty before the timer
starts. A render that reports a cache hit, or that writes nothing into its
directory, stops the run with an error.

## Rules

| Rule | Flags a family when |
|---|---|
| warm limit | the warm median is over 50 ms (`--warm-limit-ms`) |
| cold limit | the cold median is 500 ms or more (`--cold-limit-ms`) |
| regression | the warm or the cold median is more than 25% above the baseline median (`--regression-pct`) |
| no baseline | the family has no entry in the baseline |

The rules apply only on the **reference host**: a run whose hostname matches
the baseline's `reference_host` (first label, case-insensitive), or any run
passed `--reference`. Elsewhere the same findings print as "would flag" and
the run exits 0. Exit codes: 0 no rule fired (or informational), 1 a rule
flagged, 2 an error (unknown family, unreadable or invalid baseline, a failed
render, a cache hit).

## The baseline

`baseline.json` holds, per family, the warm and cold `median_ms` and `p95_ms`
recorded on the Linux build host. The loader is strict: a missing, `null`,
non-numeric, zero, or negative value, an unknown key or family, a duplicate
key at any depth, or text after the document is an error naming the JSON
path, and nothing is compared. An empty `families` object is valid; every
family then reports "no baseline".

Rewrite it only in a change that knowingly accepts the new timings, on the
reference host:

```sh
just bench --write-baseline                  # on the reference host
just bench --reference --write-baseline      # first recording on a new host
```

`--write-baseline` updates the families the run measured, keeps the others,
records this host as `reference_host`, and refuses to replace a baseline that
fails to load. It still flags the limits, but not regressions or missing
entries.

## Tests

```sh
CARGO_TARGET_DIR=/tmp/bv-bench-target cargo nextest run
```

The loader's tests run in-process; `tests/wiring.rs` spawns the binary for
the control, doctored-baseline, and doctored-limit runs.
