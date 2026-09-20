---
title: ci-verdict — the single required check
status: spec (workflows not yet edited)
created: 2026-07-27
implements:
  - plan.md §0.2 (rollup)
  - plan.md §1.2 (skip budget)
  - plan.md §1.3 (baseline verdict + deliberate job graph)
source_code:
  - scripts/ci-rollup.rs
  - .github/ci/ci-baseline.toml
---

# `ci-verdict` — the single required check

`ci-rollup` is built and tested. **No workflow file has been edited.** This
document is the exact YAML to wire it in, written so the artifact layout it
describes and the layout `ci-rollup`'s walker expects cannot drift apart.

## Why the graph has this shape

Three constraints, each earned:

1. **The verdict must run even when every producer failed.** `if: always()` on
   the job, and `needs` listing every producer.
2. **Producers must stay visibly red but must not be required checks.** If a
   producer is a required check, its failure blocks the merge directly and the
   baseline never gets consulted — the entire mechanism is bypassed. Only
   `ci-verdict` goes in branch protection.
3. **A skipped producer emits nothing to key policy to.** Measured in run
   30323254931: when `needs: lint` skips a matrix job, GitHub never evaluates
   the matrix context and the whole matrix collapses into **one** skipped job
   named with the raw expression
   `test (${{ matrix.os }}${{ ... }})`. There is no artifact, no `os`, no
   `shard`. This is why every producer must upload an **explicit status
   artifact**, and why nothing in `ci-rollup` ever parses a job display name.

## Artifact contract

`ci-rollup rollup --artifacts <dir>` expects `<dir>` to contain one
subdirectory per artifact — exactly what `actions/download-artifact@v4`
produces when `name:` is omitted.

### JUnit artifacts

```
<dir>/junit-<area>-<tier>-<os>-<index>/
    manifest.jsonl              # one JSON object per nextest invocation
    <tier>/<package>.xml        # verbatim JUnit document
```

Produced by `just/devops.just`'s `_stage_junit`. `<tier>` is one of
`L1`, `L2`, `L3`, `browser`, `real`, `sanity`.

**The manifest is the identity source.** The artifact directory name is parsed
only when a staged XML has no covering manifest record, and such a record is
flagged `degraded: true` in the output. Name parsing anchors on the tier token
(the only closed-vocabulary field) because both area names and OS labels contain
hyphens.

### Producer status artifacts

```
<dir>/status-<area>-<job>/status.json
```

```json
{"area":"claudine","job":"lint","environment":"ubuntu-latest","result":"failure"}
```

`result` is GitHub's own `success` | `failure` | `cancelled` | `skipped`.

A status whose `job` is not a test tier (today: `lint`, `check`) becomes a cell
in its own right, so a job that produces no JUnit can still be baselined and can
still block. A status whose `job` *is* a test tier is used to explain a `MISSING`
cell — it is matched on `area` alone, deliberately: `lint` runs only on Linux,
but `needs: lint` deletes the test matrix for **every** environment.

## Changes required in `_area-ci.yml` (not made — you own this file)

### 1. Export CI identity into the test steps

Without these three variables `_stage_junit` writes `""` for area, environment,
and shard, and every record downgrades to `degraded`.

```yaml
      - name: L1 tests
        shell: bash
        env:
          BISCUIT_CI_AREA: ${{ inputs.area }}
          BISCUIT_CI_ENVIRONMENT: ${{ matrix.os }}
          BISCUIT_CI_SHARD: ${{ matrix.shard }}
        run: |
          args=(--no-fail-fast)
          if [ "${{ matrix.shard }}" != "1/1" ]; then
            args+=(--partition "count:${{ matrix.shard }}")
          fi
          cd "${{ inputs.area }}"
          just test "${args[@]}"
```

Same for the `l2` job (`BISCUIT_CI_ENVIRONMENT: ubuntu-latest`,
`BISCUIT_CI_SHARD: 1/1`) and the `browser` job.

### 2. Upload the whole staging directory, not one file

The current step uploads `target/nextest/ci/test-results.xml`, which is a single
file with no manifest — the walker would fall back to name parsing and record
the package as `test-results`.

```yaml
      - name: Upload L1 JUnit
        if: ${{ !cancelled() }}
        uses: actions/upload-artifact@v4
        with:
          name: junit-${{ inputs.area }}-L1-${{ matrix.os }}-${{ strategy.job-index }}
          path: target/nextest/ci-reports
          if-no-files-found: ignore
```

### 3. Emit a status artifact from every producer job

Add as the **last** step of `check`, `test`, `lint`, `l2`, and `browser`, with
`if: ${{ always() }}` so a failed job still reports.

```yaml
      - name: Emit producer status
        if: ${{ always() }}
        shell: bash
        env:
          AREA: ${{ inputs.area }}
          JOB: lint                       # test | lint | check | l2 | browser
          ENVIRONMENT: ubuntu-latest      # ${{ matrix.os }} in the test job
          RESULT: ${{ job.status }}       # success | failure | cancelled
        run: |
          set -euo pipefail
          mkdir -p "$RUNNER_TEMP/status"
          printf '{"area":"%s","job":"%s","environment":"%s","result":"%s"}\n' \
            "$AREA" "$JOB" "$ENVIRONMENT" "$RESULT" \
            > "$RUNNER_TEMP/status/status.json"
      - name: Upload producer status
        if: ${{ always() }}
        uses: actions/upload-artifact@v4
        with:
          name: status-${{ inputs.area }}-lint
          path: ${{ runner.temp }}/status
```

For the sharded `test` job the artifact name must stay unique:
`status-${{ inputs.area }}-test-${{ matrix.os }}-${{ strategy.job-index }}`.
The walker only needs the `status-` prefix, not the rest of the name.

> A job that is **skipped** never runs its steps, so it uploads nothing. That is
> correct and sufficient: the upstream job that caused the skip *did* run and
> *did* upload a `failure`, which is what explains the `MISSING` cell.

## The `ci-verdict` job

Add to `.github/workflows/ci.yml`, after the area fan-out.

```yaml
  ci-verdict:
    name: ci-verdict
    # Every producer. A failed producer must not prevent the verdict; that is
    # the whole point of `if: always()`.
    needs: [scope, preflight, area-ci]
    if: ${{ always() }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4

      - name: Set up the pinned Rust toolchain
        run: rustup show

      # Omitting `name:` downloads EVERY artifact in the run into one directory
      # per artifact — exactly the layout `--artifacts` expects.
      - name: Download all result and status artifacts
        uses: actions/download-artifact@v4
        with:
          path: ci-artifacts
          merge-multiple: false

      # `--no-default-features` deselects biscuit-terminal, sniff, and
      # cargo_metadata, which the other two bins in scripts/ need and this one
      # does not. Measured: 8s vs 40s+. Do not drop the flag.
      - name: Build ci-rollup
        run: |
          cargo build --release \
            --manifest-path scripts/Cargo.toml \
            --no-default-features --bin ci-rollup

      - name: Roll up results
        id: rollup
        # `!cancelled()`, not `always()`: a MISSING cell must fail the summary
        # gate here (exit 2) without preventing the verdict step below.
        continue-on-error: true
        run: |
          ./scripts/target/release/ci-rollup rollup \
            --artifacts ci-artifacts \
            --areas .github/ci/areas.json \
            --scope '${{ needs.scope.outputs.areas }}' \
            --browser-environments ubuntu-latest \
            --provisioned-backends 'ubuntu-latest=tmux' \
            --provisioned-backends 'macos-latest=tmux' \
            --provisioned-backends 'windows-latest=' \
            --provisioned-backends 'wsl2-ubuntu=' \
            --out ci-results.json \
            --run-id '${{ github.run_id }}'

      - name: Upload the machine-readable rollup
        if: ${{ always() }}
        uses: actions/upload-artifact@v4
        with:
          name: ci-results
          path: ci-results.json

      # THE required check. Non-zero blocks.
      - name: Verdict
        run: |
          ./scripts/target/release/ci-rollup verdict \
            --results ci-results.json \
            --baseline .github/ci/ci-baseline.toml
```

`--summary` is omitted on purpose: both subcommands append to
`$GITHUB_STEP_SUMMARY` automatically when it is set, which it always is in
Actions.

### How environments and tiers are derived

L1 and L2 environments come from each area's `environments` in `areas.json` —
there is no `--l2-environments` flag. An L2 tier is expected on **every**
environment the area declares, including ones with no backend: dropping the
unprovisioned ones would make them vanish from the grid, which is the failure
mode `POLICY GAP` exists to prevent. `--browser-environments` remains a flag
because the browser tier is explicitly Linux-only.

A `policy_gaps` record in `areas.json` is **authoritative** and needs no
provisioning data: the cell renders `POLICY GAP`.
`--provisioned-backends` is the second net — it catches an *undeclared* gap
(which `affected_scope.py` is supposed to reject at config time) and labels it
`UNDECLARED` in the reason. An environment omitted from the flag is treated as
*unknown*, not as unprovisioned, so no gap is asserted for it.

Rendering `POLICY GAP` and *blocking the merge* are separate decisions, taken in
separate subcommands. `rollup` renders and lists the cell under "Cells failing
the summary gate" unconditionally — that visibility is the whole point of the
state. `verdict` then decides, and an owned, unexpired gap is accepted as a
`note` rather than a block, exactly as a baselined failure is. An undeclared,
unowned, undated, or expired gap blocks; so does a gap whose cell produced real
failures, because `FAIL` outranks `POLICY GAP`. A gap is *not* rejected merely
for showing passing tests — `require_level!` early-returns render as JUnit
passes, so that is what a correctly-declared gap looks like from here, which is
also why the per-backend execution proof below is the missing piece. See
`.github/ci/README.md` → "A declared gap does not block the merge" for the rule
table and for why expiry is checked both here and in `affected_scope.py`.

Without this, `ci-verdict` could never exit 0 on any run touching one of the
eight areas that declare a Windows-L2 gap — which, given `biscuit-terminal`,
`darkmatter`, `sniff`, and `claudine` are among them, is essentially every run.

`sanity` is never a scheduled tier. It is a fast local-dev subset of L1 that CI
does not run, so a `sanity` record appearing in a CI artifact means a recipe is
mis-wired; it surfaces as an unscheduled cell rather than being counted as L1
coverage.

`environment` is not `os`: `wsl2-ubuntu` gets its own cells, never merged into
the native Windows column. `full_os` is accepted as a serde alias for
`environments` so an artifact produced before the rename still rolls up.

### nextest exit codes are raw

`exit_code` in the manifest is nextest's own code, and the rollup reads all of
them distinctly:

| exit | `report_present` | Cell | Reason text |
|---|---|---|---|
| `101` | `false` | `MISSING` | the crate failed to **build**; no test ran, so the tier's test set is unknown and it can never be `N/A` |
| `100` | `false` | `MISSING` | tests failed but staged no report; the failing tests cannot be identified |
| `0` | `false` | `MISSING` | exited clean and staged nothing |
| `100` | `true`, report shows no failure | `MISSING` | the exit code and the report disagree; the report is incomplete |

A build failure is therefore **not** baselineable as a known test failure — per
plan §1.3, an entry that emits no test result stays blocking.

### `--scope` is load-bearing

`--scope` is the **only** way `ci-rollup` learns that an area was scheduled but
produced nothing. Omit it and scope is inferred from the artifacts on disk —
which by construction cannot see an area that produced no artifact at all, i.e.
exactly the `MISSING` case. The tool marks such a run `scope_degraded: true` and
prints a warning in the summary, but do not ship it that way.

It accepts a comma-separated list, repeatable. It must be the same area set the
`scope` job fanned out. If `needs.scope.outputs.areas` is a JSON array rather
than a comma list, convert it:

```yaml
        run: |
          scope="$(printf '%s' '${{ needs.scope.outputs.areas }}' | jq -r 'join(",")')"
          ./scripts/target/release/ci-rollup rollup --scope "$scope" ...
```

### Branch protection

Set **`ci-verdict`** as the only required status check. Remove every
`<area> / test (...)`, `<area> / lint (...)`, and `<area> / check (...)` entry.
Producers stay red and visible in the run; only the verdict gates.

## Not wired: the expected-test manifest

`--expected-manifest` is implemented and tested but has **no producer**.

Without it, a test present on the target environment that produced no result is
indistinguishable from one `#[cfg]`-compiled out. `ci-rollup` refuses to guess:
it records `skip_evidence_degraded: true` on the cell rather than inferring
either answer. That is plan §1.2's explicit requirement — "compile-time `#[cfg]`
absence is N/A and must not be inferred from JUnit alone".

Format (JSON), one file per environment × tier, **generated on that
environment**:

```json
{
  "schema_version": 1,
  "environment": "windows-latest",
  "tier": "L1",
  "packages": {
    "biscuit-file":     ["biscuit-file::path::abs", "biscuit-file::path::rel"],
    "biscuit-file-cli": ["biscuit-file-cli::cli::help"]
  }
}
```

Identities are `<testsuite name>::<testcase name>`, matching what the JUnit
parser produces. Source: `cargo nextest list --message-format json`, run in the
same job, before the tests.

Wiring it needs a `just` recipe (owned by another agent) and a step per test
job; both are out of scope here. Until then, treat `skip-evidence-degraded`
notes in the verdict as expected.

## Also not wired

- **Specialized workflows.** `messenger-desktop`, `rendezvous`, the Claudine
  generator drift check, and coverage emit no `manifest.jsonl` and are not in
  `areas.json`, so they cannot be in an affected scope. Their migrated baseline
  entries are reported `baseline-out-of-scope` and **ignored** — they neither
  block nor pass. Plan §3.5 is what closes this.
- **`--provisioned-backends` for WSL.** `wsl2-ubuntu` is listed above with an
  empty value, which asserts "provisions nothing" and produces a `POLICY GAP`
  for any L2 area scheduled there. Update it as plan §3.3 provisions `tmux`
  inside the distro. `windows-latest` is likewise empty, and stays that way
  until the §2.3 headless-`wezterm-mux-server` spike proves a backend.
  `macos-latest` is **not** in that set: it is listed as `macos-latest=tmux`,
  matching `_area-ci.yml`'s `brew install tmux` step and `affected_scope.py`'s
  `L2_PROVISIONED_ENVIRONMENTS = {ubuntu-latest, macos-latest}`. An environment
  *omitted* from the flag entirely means "unknown", and no policy-gap judgement
  is made for it — which is different again from an empty value.
- **Per-backend execution proof** (plan §1.1). `ci-rollup` can see that a tier
  executed zero tests, and can see that no compatible backend was provisioned,
  but it cannot see that a test early-returned from `require_level!` — that
  renders as a JUnit *pass*. `BISCUIT_TEST_REQUIRED_BACKENDS` is the fix, and it
  is a different phase.
