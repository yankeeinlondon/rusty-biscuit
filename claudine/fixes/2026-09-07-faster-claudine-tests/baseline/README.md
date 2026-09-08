# Baseline artifacts — `2026-09-07-faster-claudine-tests`

Everything Phase 1 collects, and the shape Phase 9 will mirror for the
candidate runs.

## Layout

```text
baseline/
├── README.md            this file
├── expectations.json    what a complete artifact set must contain
├── local-gates/         verbatim local gate output (attribution only)
└── <run-id>/            one directory per CI run
    ├── ubuntu-latest/   the uploaded `junit-claudine-cli-L1-<env>` artifact
    │   ├── manifest.jsonl
    │   └── L1/claudine-cli.xml
    ├── macos-latest/
    ├── windows-latest/
    └── wsl2-ubuntu/
```

Each `<env>` directory is the artifact **as downloaded** — the whole
`target/nextest/ci-reports` staging tree that `just/devops.just`'s `_stage_junit`
builds, not just the XML. The `manifest.jsonl` is load-bearing: it carries the
per-invocation `exit_code`, `duration_s` and `report_present` that the XML alone
cannot express, and a tree without it is rejected as not-a-staging-directory.

## Collecting a run

```bash
run=<run-id>
for env in ubuntu-latest macos-latest windows-latest wsl2-ubuntu; do
  gh run download "$run" -R yankeeinlondon/rusty-biscuit \
    -n "junit-claudine-cli-L1-$env" -D "baseline/$run/$env"
done
npx tsx ../junit-metrics.ts "baseline/$run" \
  --label "run $run" --expect baseline/expectations.json
```

The script exits non-zero on any violation. **Do not** collect only the runs
that pass: the plan requires every intervening failed attempt to be recorded
with its cause.

## The three cost columns

They are kept separate because they move independently and answer different
questions.

| Column | Source | What it measures |
|---|---|---|
| Build/setup | `manifest.duration_s` − `<testsuites time>` | compile and fixture setup inside the invocation |
| Runner elapsed | `<testsuites time>` | nextest's own wall time, sensitive to runner core count |
| Summed duration | Σ `<testcase time>` | total test work, the only column comparable across runners |

## Status

Phase 1's CI tranche is **not yet collected** — see `../log.md`. Committing,
pushing and merging are operator actions, so no post-merge run exists to
download. `local-gates/` holds attribution only and establishes no target.
