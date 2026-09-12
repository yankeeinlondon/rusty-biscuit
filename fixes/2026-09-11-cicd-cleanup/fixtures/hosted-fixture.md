---
kind: fixture
status: prepared-not-hosted
created: 2026-09-12
---

# Reduced hosted integration fixture

`hosted-fixture.py` prepares an overlay for an isolated full candidate checkout.
It does not commit, push, create a pull request, publish Git notes, or dispatch
anything. No hosted execution has been recorded. GitHub authentication and the
outgoing trigger/constraint review must be resolved before activation.

The overlay keeps `_area-ci.yml` and `_wsl-ci.yml` byte-for-byte. It retains the
package job graph, conditions, permissions, normalized gate step IDs, producer
status scripts, and artifact uploads. It substitutes inexpensive setup and a
real tiny assertion for product workloads. The top workflow uses explicit
fixture scope and the shipped `ci-gate` shell, with only scope and area jobs as
its dependencies. `fixture-validation.json` records every substituted step and
candidate input Git blob identity. Regenerate after candidate changes.

Prepare and verify from the candidate repository root, using a new empty path:

```sh
python3 fixes/2026-09-11-cicd-cleanup/fixtures/hosted-fixture.py prepare /tmp/cicd-fixture-overlay
cargo build --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup
python3 fixes/2026-09-11-cicd-cleanup/fixtures/hosted-fixture.py verify /tmp/cicd-fixture-overlay
actionlint /tmp/cicd-fixture-overlay/.github/workflows/ci.yml /tmp/cicd-fixture-overlay/.github/workflows/_area-ci.yml /tmp/cicd-fixture-overlay/.github/workflows/_package-ci.yml /tmp/cicd-fixture-overlay/.github/workflows/_wsl-ci.yml
```

Preparation executes exactly three local fixture assertions. Their reports,
measured outcomes, and `sniff` host identity are retained in `fixture-evidence/`.
Hosted scope consumes these outcomes without rerunning them. Reused cells keep
the measured local environment; executing cells are Ubuntu-only. Every matrix
has `wsl=false`. This proves no WSL runtime behavior and runs no product suite.
The local `verify` command runs seven cases through the shipped Bash status
script and Rust rollup/verdict; its output is `fixture-local-verification.json`.
It is not hosted evidence.

Apply only to an isolated **full candidate checkout**. Candidate
`fcf7a4f66977a3dba70f64438fdb757bb3dcb503` contains the committed inputs;
all 33 recorded input/workflow blobs match the prepared overlay. The original
manifest's `candidate_head` records the pre-commit preparation base, and
`committed-candidate.json` beside the durable overlay records the verified
committed identity. The apply command rejects differing candidate input blobs
before making changes:

```sh
python3 fixes/2026-09-11-cicd-cleanup/fixtures/hosted-fixture.py apply /tmp/cicd-fixture-overlay /absolute/path/to/isolated-candidate
```

Application removes every unrelated workflow YAML in that isolated checkout
and installs the overlay. It never edits this candidate checkout. Review all
remaining workflow triggers and every active execution constraint against the
outgoing fixture branch. Do not push `feat/unifi` to bootstrap this fixture:
its product plan schedules prohibited WSL cells. Fixture-baseline acceptances
belong only on the disposable fixture branch.

Once the isolated fixture branch is published through the reviewed route,
dispatch the existing `ci.yml` workflow file on that branch. The default branch
already declares `workflow_dispatch`; no default-branch workflow edit is needed:

```sh
gh workflow run ci.yml --ref codex/cicd-review7-fixture --field case=accepted
```

Run each case once and retain its run URL and candidate identity:

| Case | Expected gate | Distinguishing observation |
| --- | --- | --- |
| `accepted` | success | Failed Ubuntu assertion is accepted by its area's fixture baseline. |
| `unaccepted` | failure | The failed producer is normalized; its own rollup blocks. |
| `missing-report` | failure | A gate failing without JUnit remains missing/blocking. |
| `setup-failure` | failure | Setup fails the producer directly and cannot be baselined. |
| `reused-failure` | failure | Locally measured failure blocks without a producer for its environment. |
| `accepted-reused-failure` | success | The measured local failure is accepted only by its matching environment baseline. |
| `empty` | success | Area fan-out skips and the gate passes without a pending matrix. |

For every nonempty case capture the resolved `fixture/nested` label, the reused
result with no corresponding L1 producer, independent passing area slices, and
the `fixture-gap` neutral check. Confirm the gap publisher skips in other areas,
reads its plan artifact successfully, and alone receives `checks: write`.
Download the artifacts listed in `fixture-validation.json`, including the
retained fixture reports; record cell states and gate conclusions, not merely
the overall run conclusion.

Dispatch does not prove PR head-versus-merge check placement. The generated
workflow also permits pull requests targeting only
`codex/cicd-review7-fixture-base`. Prepare the base commit from the isolated
overlay with `fixture-case.json` set to `{"case":"empty"}`. Create a distinct
head commit changing only that file to `{"case":"accepted"}`. This supplies
the commit difference required to open a PR while both branches retain the
same narrow PR trigger. Verify the base and head SHAs differ and all workflow
Git blob identities match between them. Review both outgoing branch triggers,
publish the two fixture branches, and open the fixture PR. Its case comes from
`fixture-case.json`. Record the PR head SHA, merge SHA, neutral check SHA, nested check
names, artifact slices, and gate result; the neutral cell check must be on the
PR head. This dedicated PR scenario remains required before claiming that
boundary complete.

Receipt verification and source selection are intentionally outside this
reduced experiment: actual locally measured fixture outcomes enter the plan
directly, and no product receipt or Git note is manufactured. The local hook
and real scope-step regressions supply that evidence. This fixture cannot
replace product compilation, WSL execution, or the separately approved
`ci-verdict` to `ci-gate` ruleset change.
