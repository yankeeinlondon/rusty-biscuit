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

## Platform exclusions

Every `requiredTests` entry lives in a `#![cfg(unix)]` binary, so the
`windows-latest` leg cannot execute any of them — 2,105 identities against
2,466 on the three Unix legs (372 Unix-only, 11 Windows-only). `expectations.json`
declares those eleven under `platformExclusions["windows-latest"]`; the gate
reports them in their own table instead of as `missing-test`, and fails with
`stale-exclusion` if one of them ever *does* run on Windows. The three Unix legs
carry identical identity sets.

## Status (2026-09-08)

The predecessor merged to `main` as `444213eb5` (PR #69, 00:27 UTC). Collected:

| Run | Event | Source | Legs | Gate |
|---|---|---|---|---|
| [`34173378609/`](34173378609/) | push to `main` | `444213eb5` | four, all green | exit 0 — **baseline run 1 of 3** |
| [`34159725015/`](34159725015/) | `pull_request` on PR #69 | `a9e88c069` — tree-identical to `444213eb5` (`git diff a9e88c069 444213eb5` is empty) | four, all green | exit 0 — **supplementary**, same tree, not counted toward the three `main` runs |

Each run directory also holds the gate's verbatim output (`junit-metrics.txt`).
`local-gates/` holds attribution only and establishes no target.

**Still pending: two more consecutive green runs on `main` at a comparable
source state.** `main` moved 13 hours after the merge (PR #70, `6504747e2`),
touching `claudine/lib`, four `claudine/cli/tests` files and the `test-real`
recipe, so later `main` pushes are a different source state; run `34232285291`
at `6504747e2` is recorded in `../log.md` § Phase 9 with that caveat. The one
way to get two more samples at `444213eb5` itself is `gh run rerun 34173378609`,
which is an operator call — the `main` concurrency group cancels in-flight runs.
