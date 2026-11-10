---
kind: fix
name: ubuntu-archive-disk
date: 2026-11-06
status: planned
next_spec: 2026-11-06-windows-wsl-test-failures
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-08
review_iterations: 1
completed: true
implemented: true
implemented_by: "claude/opus"
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
---

# The Ubuntu archive build must finish, so WSL2 can run

## Why

WSL2 cells run archives built by `build (ubuntu-latest)`. On the Nov 5 and Nov 6
nightlies that job died after about 2.5 hours with `No space left on device`
(Nov 4 lost its runner, possibly the same cause). All 66 WSL2 cells then failed
with `Artifact not found`, so **WSL2 tests did not run at all**.

The run looks like a wall of WSL2 failures, but none of them is a test result.

## Current behavior and constraints

The [build job](../../.github/workflows/ci.yml) runs one producer per planned
producer environment, with a 180-minute timeout. The producer builds package
archives serially, sharing Cargo's `target/` directory so later packages can
reuse compiled dependencies. The number of build records comes from the
resolved plan; 26 is an earlier full-workspace observation, not a limit.

In the `repo-deps` package, [run_produce](../../scripts/ci-build-archive.rs)
builds each record and writes its status. The
[publication helper](../../scripts/ci/artifacts/publish.cjs) runs afterward,
uploading each successful archive, manifest, declared sidecars, and verifier.
Consequently, completed archives accumulate alongside the compiled output
until all records have been attempted. A failed record does not intentionally
prevent unrelated records from being published.

The workspace already uses line-table-only debug information for its own
dev builds and no debug information for external dependencies in
[Cargo.toml](../../Cargo.toml). Do not assume full debug information is the
cause or remove the remaining source-line information without evidence.

> **Reader's note:** “clear each key's output” is not a safe cleanup rule.
> Compiled dependencies are shared, and completed archives are still needed
> by the later upload step. This review separates those lifetimes so a disk
> remedy does not introduce repeated compilation or missing uploads.

## Outcome

- Every Ubuntu build record required by a full-workspace plan is produced and
  published within the existing timeout, with remaining disk space recorded.
- A WSL2 cell with a valid archive reaches test execution and produces actual
  reports. Passing tests are not required by this storage fix; failures in
  those tests belong to the follow-up below.
- A cell blocked by its producer clearly says “not run: producer failed” and
  identifies the package, build key, and failed stage when available. It is
  never represented as a passing test, accepted capability gap, or completed
  cell.
- Successful records remain consumable even when another record fails.

## Scope and design decisions

### Keep scheduling and archive identity unchanged

The resolved plan remains the only source of build records and executing
cells. Preserve package/environment/gate identities, per-record artifact
names, build-key and digest checks, source identity, declared sidecars, and
Linux/WSL2 compatibility checks. Consumers must never compile a replacement
for a missing archive. Do not expand nightly scope, disable tests, increase
the timeout, or weaken archive verification to make this fix appear green.

Keep one producer per environment and shared compiled dependencies as the
baseline design. A different producer layout requires the decision described
under Open Questions; it must not be introduced as a local workflow-only
matrix split that disagrees with the plan.

### Measure before selecting the storage remedy

Use one Ubuntu producer run as a quick storage diagnostic, preferably the next
already-required nightly. Add lightweight logging before and after cache
restore, the schematic fresh-output guard (when selected), each build record,
and publication. Record filesystem free bytes and inodes for both the target
and archive-output locations; they may share a filesystem. Record directory
sizes for compiled output, completed archives/sidecars, and temporary
extraction or packaging output, plus elapsed time and the cache state. Keep
these categories distinct; summing directory sizes is not a free-space measure.

Log at record boundaries and on handled failures, and preserve available logs
with an always-run diagnostic step. A lost runner may prevent final logging;
retain earlier samples rather than requiring a final observation as proof of
the original cause. Boundary samples are not a claim to have measured the
instantaneous peak during linking or compression.

This is one-host diagnosis, not a performance benchmark. Do not add repeated
full-workspace runs, other hosts, or statistical thresholds. If existing run
logs already identify the storage consumer, use them and omit the diagnostic
run. A full-workspace validation is justified by accumulated storage across
records, not by a need to rerun every test suite.

Select the smallest remedy supported by those observations:

- If stale restored build output dominates, narrow the producer cache or its
  restore policy. Preserve a useful shared target tree during the current run;
  document the resulting cold-build cost.
- If completed archives dominate, publish each completed record before deleting
  its local archive and copied sidecars. Await upload success before deletion,
  retain manifests/statuses needed for reporting, and preserve the verifier.
  Publication must happen during production; deleting files before the current
  end-of-job publisher reads them is invalid. Avoid publishing the same
  immutable artifact twice in the finalizer.
- If temporary packaging/extraction files dominate, release them after their
  last reader on success and handled failure. Cleanup must be limited to
  directories owned by this producer; never broadly delete runner tools,
  source fixtures, Cargo's registry, or shared dependencies as a disk heuristic.

Record the measured cause, chosen remedy, observed remaining space, and any
tradeoff in the implementation log. If the live compiled dependency set alone
cannot fit, use Open Questions rather than silently changing debug profiles or
producer layout. Further debug-information reduction is not the default fix.

### Attribute blocked consumers without hiding failures

Use the existing per-record build-status artifacts and audit attribution before
attempting to download a known-failed archive. Apply this to every consumer of
the affected build record, including native Linux cells when selected, rather
than only the WSL2 workflow. Keep successful records usable if a sibling record
or its upload failed.

A missing status after a runner loss is an infrastructure failure with an
unknown per-record outcome, not evidence that every record failed to compile.
Use the existing owner/job attribution to explain it. A reported success with
no downloadable artifact remains an artifact-publication failure. Never infer
success from a missing status, an empty test report, or a skipped download.

The existing build failure must continue to block `ci-gate`. Keep that gate's
policy-free fold of job results, and keep the area's coverage audit responsible
for missing completion records. Human-readable “not run” wording does not add
a new passing result state or excuse missing test completion. If implementation
requires a machine-readable schema change, update its validators, readers, and
version history together as required by `docs/cicd/schema-versions.md`.

## Verification and documentation

Add focused checks through the existing `repo-deps` producer/publication tests
and `test-toolkit` workflow contracts, using the canonical Just/nextest recipes.
Cover the selected cleanup lifetime, upload failure, a successful record beside
a failed record, and consumer attribution for failed or absent build status.
Use small fixtures or simulated failures for these cases; do not fill a hosted
runner's disk deliberately. Preserve the existing shared-dependency reuse check.

For storage acceptance, inspect one full-workspace Ubuntu producer execution
with the remedy applied. Record its plan/run identity, cache state, duration,
free-space samples, and successful publication of every required record. The
minimum observed free space must stay positive; report its actual value and
sampling limitations rather than inventing a benchmark threshold. Confirm that
a planned WSL2 consumer reaches tests using one of those verified archives;
report subsequent test failures separately. If no such execution is available,
mark hosted storage/consumer acceptance unverified rather than claiming the
local fixture proves it.

Update `docs/topics/ci-cd.md` with the implemented storage lifetime and blocked
consumer behavior, and the `rust-devops` skill if the producer workflow changes.
The topic page describes current behavior directly and does not link back to
this dated specification.

## Open Questions

### What if the live compiled output still cannot fit?

Decide only if the Ubuntu sample rules out stale cache, retained archives, and
temporary output as sufficient remedies:

1. **Use a larger Ubuntu producer runner (recommended in that case).**
   Preserves shared compilation, archive identity, and source-line diagnostics.
   Its costs are runner charges and possible availability/configuration work.
   Prefer this when the required live build set exceeds the current disk: it
   addresses capacity without repeating dependency builds.
2. **Partition records across multiple Ubuntu producers.** Bounds each runner's
   accumulated output and permits parallel builds. It duplicates dependency
   compilation and cache storage, and requires coordinated planner, owner
   identity, prerequisite, publication, and reporting changes. Each record must
   still have exactly one planned producer; artifact identity cannot collide.
3. **Reduce producer debug information further.** May reduce link and disk
   costs without more runners, but loses useful failure diagnostics and changes
   the compile configuration. It requires build-identity/cache separation from
   the old configuration and an explicit decision about that diagnostic loss.

No option is authorized merely by listing it here. The implementation log must
identify this as an unresolved capacity decision if smaller remedies fail.

## Not in scope

Fixing whatever WSL2 tests fail once they run; that is
`2026-11-06-windows-wsl-test-failures`. This fix also does not alter existing
capability gaps, evidence-reuse rules, OS scheduling, or the merge gate policy.
