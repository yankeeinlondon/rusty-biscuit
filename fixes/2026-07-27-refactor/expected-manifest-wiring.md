---
title: Wiring the expected-test manifest producer into CI
status: handoff
created: 2026-07-28
owner: "@yankeeinlondon"
produces_for:
  - scripts/ci-rollup.rs (`rollup --expected-manifest`)
implements:
  - plan.md §1.2
---

# `just _expected_manifest` — the producer for `ci-rollup --expected-manifest`

`ci-rollup` has implemented `--expected-manifest` since plan §1.2 landed, but
nothing produced the file, so every skip-budgeted cell reported
`skip-evidence-degraded`. `just/devops.just` now has the producer. **Nothing in
`.github/workflows/**` has been changed** — another agent owns those files. This
document is the exact invocation to add.

## Why it cannot be inferred, restated

A JUnit report cannot distinguish a test that does not exist on an environment
(compile-time `#[cfg]`) from a test that silently stopped running. Both are
absent. §1.2 therefore forbids inferring `N/A` from JUnit and requires the
expected set to be generated **on the target environment**, where
`cargo nextest list` can report what the compiled binaries actually contain.

The hazard is not hypothetical. `_test` passes `--no-tests=pass`, so a tier that
selects zero tests exits `0` and stages a valid, empty report. The rollup renders
that `N/A`, which is right in isolation — "a tier with no tests is N/A, not
PASS" — but it means a drifted filter expression silently downgrades a real area
to `N/A` with nothing, anywhere, failing. Measured on `worktree` (36 L1 tests in
the lib alone):

| | cell state | `skip_evidence_degraded` | skipped | reason |
|---|---|---|---|---|
| without a manifest | `N/A` 0/0/0 | `true` | 0 | *(none — the gap is invisible)* |
| with a manifest | `N/A` 0/0/36 | `false` | 36 | `36 expected test(s) compiled on this environment produced no result` |

## The recipe

```
just _expected_manifest <tier> "<package spec>" [extra nextest args…]
```

- `tier` — `L1`, `L2`, `L3`, `browser`, `real`, or `sanity`.
- `<package spec>` — identical to `_test_all` / `_run_all`: whitespace-separated
  package names, or, when it contains a `;`, `;`-separated
  `<package> [extra args…]` entries. Pass the area's own spec string verbatim.
- The tier's filter expression comes from `_tier_filter`, the **same** recipe the
  tier's own runner reads. Expected and observed are therefore selected by one
  expression, not two copies that drift.

Required environment:

| variable | why |
|---|---|
| `BISCUIT_CI_ENVIRONMENT` | The manifest is keyed `{environment, tier}` and must join to the manifest records the same job stages. The recipe **hard-errors** when it is unset rather than emitting an unjoinable document. |
| `BISCUIT_JUNIT_STAGE_DIR` | Where to write. Defaults to `<workspace-root>/target/nextest/ci-reports`, i.e. the same `$STAGE` `_stage_junit` uses. |

Optional: `BISCUIT_JUNIT_WORKSPACE_ROOT` (skips `cargo metadata`, needed in the
WSL guest) and `BISCUIT_NEXTEST_BIN`.

Output: `$STAGE/expected-<tier>.json`, alongside `manifest.jsonl` and
`<tier>/<package>.xml`. Uploading the whole staging directory — which
`_area-ci.yml` already does — carries it without a new upload step.

## Emitted format

Matched to `ExpectedManifest` in `scripts/ci-rollup.rs`:

```json
{
  "schema_version": 1,
  "environment": "macos-latest",
  "tier": "L1",
  "packages": {
    "worktree": ["worktree::cache::tests::atomic_write_concurrent_writers_last_rename_wins", "…"],
    "worktree-cli": ["worktree-cli::bin/wt::commands::create::tests::reuse_notice_names_branch_commit_and_flags_no_fork", "…"]
  }
}
```

Two details are load-bearing and were verified against the consumer rather than
assumed:

- **Test identity is `<nextest binary-id>::<test name>`.** The rollup rebuilds
  identities from JUnit as `<testsuite name>::<testcase name>`, and nextest sets
  `testsuite name` to the binary id. The producer therefore joins
  `rust-suites[].binary-id` to the testcase key, which is why an identity can
  contain a slash (`worktree-cli::bin/wt::…`).
- **`cargo nextest list` reports every discovered test with a `filter-match`
  verdict, not just the matching ones.** The producer keeps only
  `filter-match.status == "matches"`, which also drops `#[ignore]`d tests — a
  test nextest will not run is not expected to produce a result.

`tier` is written verbatim and round-trips through `Tier::parse` (case-insensitive).
`packages` keys are package names, matching the `package` field of the manifest
records — the rollup only judges a package it has evidence for, so the two must
agree.

A package that compiled zero tests for the tier is recorded as an **empty
array**, not omitted. That is the whole point: it is a positive assertion that
this environment has nothing here, and it clears `skip_evidence_degraded` for
the cell.

## Invocation to add to `.github/workflows/_area-ci.yml`

In the `test` job, **after** the L1 run (so a compile failure fails the run
rather than the listing) and **before** the artifact upload:

```yaml
      - name: Record the expected test set for this environment
        if: always()
        shell: bash
        env:
          BISCUIT_CI_AREA: ${{ inputs.area }}
          BISCUIT_CI_ENVIRONMENT: ${{ matrix.os }}
          BISCUIT_JUNIT_STAGE_DIR: ${{ github.workspace }}/target/nextest/ci-reports
        run: |
          cd "${{ inputs.area }}"
          just _expected_manifest L1 "<the area's package spec>"
```

The package spec is the string the area's own `test` recipe passes to
`_test_all`. It is not currently exposed as workflow input; the two options are
to add a `test_packages` field to `.github/ci/areas.json` (preferred — one
source, validated by `affected_scope.py`) or to derive it from `check_args` by
stripping `-p`. **Do not hand-copy it into the workflow**; that is the same
second-copy drift the shared `_tier_filter` exists to prevent.

Do **not** pass `--partition`. The rollup aggregates every shard of a cell, so
each shard's expectation is the cell's whole compiled test set; the manifests
merge idempotently.

For `l2` / `browser` areas, add a second invocation with the matching tier in
those jobs.

Consumer side, in the rollup job:

```yaml
          --expected-manifest artifacts/junit-<area>-L1-<os>-<n>/expected-L1.json
```

`--expected-manifest` is repeatable, so the rollup step should glob every
`expected-*.json` under the artifacts root and pass one flag per file.

## WSL2

The recipe runs unchanged inside the guest — verified against a simulated
cargo-free environment (see `archive-passthrough-implementation.md`):

```bash
BISCUIT_NEXTEST_BIN="cargo-nextest nextest" \
BISCUIT_JUNIT_WORKSPACE_ROOT=/home/runner/rusty-biscuit \
BISCUIT_CI_ENVIRONMENT=wsl2-ubuntu \
just _expected_manifest L1 "<spec>" \
  --archive-file "$archive" --workspace-remap /home/runner/rusty-biscuit
```

Archive mode moves package selection into the filterset and drops build-only
flags, exactly as `_test` does, because `cargo nextest list --archive-file`
rejects `-p` for the same reason `run` does.

## Unwired

- No workflow file touched.
- No `areas.json` field added for the per-area package spec.
- The rollup step's `--expected-manifest` glob is not written.
