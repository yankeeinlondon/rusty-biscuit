# Candidate evidence — `2026-09-07-faster-claudine-tests`

Phase 9's half of the CI tranche: what this fix's branch produces once pushed,
in the shape [`../baseline/`](../baseline/README.md) established for the
predecessor's runs.

## Layout

```text
candidate/
├── README.md            this file
├── expectations.json    baseline's expectations with the timeout floors ENFORCED
├── pr-body.md           the pull-request description, ready for `gh pr create --body-file`
├── local-gates/         verbatim output of the local gates run before the handoff
│   ├── check-windows.log
│   └── ci-local.log
└── <run-id>/            one directory per CI run, exactly as `baseline/<run-id>/`
    ├── ubuntu-latest/
    ├── macos-latest/
    ├── windows-latest/
    └── wsl2-ubuntu/
```

## Collecting a candidate run

```bash
run=<run-id>
for env in ubuntu-latest macos-latest windows-latest wsl2-ubuntu; do
  gh run download "$run" -R yankeeinlondon/rusty-biscuit \
    -n "junit-claudine-cli-L1-$env" -D "candidate/$run/$env"
done
npx tsx junit-metrics.ts "candidate/$run" --label "candidate $run" \
  --expect candidate/expectations.json \
  --baseline baseline/34173378609 --baseline-expect baseline/expectations.json \
  | tee "candidate/$run/junit-metrics.txt"
```

The gate exits non-zero on a violation in *either* tree. The comparison it
prints matches identities **within each environment** and lists additions and
removals apart from the matched summed durations; it never compares one leg's
identity set with another's.

Collect every run, green or not. The plan needs three consecutive green
candidate runs per leg with every intervening failure recorded and its cause
named; selecting the green ones is disallowed.

## Handoff — what has to happen before a candidate run can exist

All of it is operator action; this session cannot sign a commit or push.

1. **Merge `origin/main` into `fix/cli-slow-tests`.** `git merge-tree` reports
   thirteen conflicting files (PR #70 landed on `main` after this branch
   diverged): eight `claudine/cli/tests` files (`common/mod.rs`,
   `compose_schema_cli.rs`, `composition_outputs.rs`, `handle_deadline.rs`,
   `inline_compose_cli.rs`, `loop_cli.rs`, `sequence_groups.rs`,
   `sequence_prompt_property.rs`), `claudine/justfile` (`test-real`: this
   branch moved it onto nextest, `main` added a `real_inline_write_grant`
   invocation to the old `cargo test` form), `claudine/lib/src/diagnostics/registry.rs`,
   `claudine/docs/providers/dispatch-inventory.json`,
   `.claude/skills/claudine/timeline.md`, and the completed
   `2026-09-05-inline-flow-and-validations/spec.md`. Twenty-one hunks in all.
   After resolving, re-run `just test` and `just lint` in `claudine/` — the
   merge changes the test population this branch measured.
2. **Commit** (signed, `Ken Snyder <ken@ken.net>`, no agent trailers) and
   `git verify-commit HEAD`.
3. **Push** and open the PR: `gh pr create --base main --body-file candidate/pr-body.md`.
4. **Read the first run on every leg for correctness**, then let normal CI
   accumulate three consecutive green candidate runs per leg; request extra
   runs only for missing samples.
5. **Collect** each run with the recipe above.

## What the candidate comparison can and cannot say

- Post-merge, the candidate tree contains PR #70's test additions, so the
  per-environment comparison will show *additions* the baseline run never had.
  That is what the separate additions column is for; matched-identity summed
  duration is the number to read.
- The baseline itself is one run short of three (see `../baseline/README.md`),
  so Phase 3's budgets are still underivable and any candidate-vs-budget
  comparison remains pending until they exist.
