---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - Retain incremental publication selected from the existing local 66-record measurement; this differs from the plan's unavailable-diagnostic fallback but is a bounded, reversible remedy supported by the spec.
    - Retain storage_sample.py as an importable Python module name.
    - Retain the JSON argument array for invoking the sampler without a shell.
    - Retain record-boundary sampling; the hosted coordinator currently produces a start sample and an after-record sample for each record.
    - Retain JavaScript action ownership of the coordinator so artifact credentials stay within that action.
    - Retain status-only finalization retries after status-upload failure and terminal failure reporting after archive-upload failure.
    - Retain compiler-work-owner-{producer} as the separate final measurement artifact name.
    - Retain Windows Bash lookup through PATH under the existing hosted-runner provisioning assumptions.
    - Retain existing Node provisioning rather than adding runner-tool metadata for the revised dependency-sharing fixture.
    - Retain unconditional WSL2 status validation alongside its existing unconditional archive download.
    - Retain rollup blocking rules and attribute absent status through the consumer check rather than guessing its cause in rollup.
    - Retain the correction to the README paragraph that incorrectly said the publisher suite was unregistered.
    - Retain scoped Clippy instead of repository-wide lint for these changes.
    - Retain previously passing planner-suite evidence where no subsequent source change invalidated it.
    - Leave temporary packaging cleanup and cache isolation disabled because the available measurement did not establish recoverable bytes in those categories.
    - Leave schematic guard output cleanup out of this change; the selected remedy does not require broadening the authorized cleanup boundary.
    - Keep larger runners, producer partitioning, and debug-profile changes unauthorized until hosted evidence establishes insufficient capacity.
    - Accept the explicitly unverified hosted acceptance record; after deployment, inspect a full Ubuntu producer and one consuming WSL2 cell before claiming the storage objective was achieved.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-09T14:35:00-08:00
spec: 2026-11-06-ubuntu-archive-disk/spec.md
implemented: false
description: A **fix** review of `2026-11-06-ubuntu-archive-disk/spec.md`
fix: 2026-11-06-ubuntu-archive-disk/review-1.md
---

# Review 1: Ubuntu archive disk

The implementation is production ready under this review's readiness rules. No findings were identified. This approves the implementation, not a claim that the hosted storage failure has been eliminated: full-workspace Ubuntu capacity, publication within the timeout, and a WSL2 consumer reaching tests with the new archives remain **unverified**. The spec explicitly permits that evidence state when no qualifying execution is available.

## Findings

None. This is the first review; there are no previous findings to reconcile.

## Scope and verification

Reviewed the spec, plan, implementation log, existing local storage measurement, producer sampling, sequential coordinator, publication ledger and cleanup, consumer status reader, both consumer workflows, runner-loss attribution, rollup changes, tests, and current documentation. Two independent parallel readers collected implementation decisions before the comparison with the spec.

The implementation preserves the resolved plan's scheduling and build identities, one owner per environment, shared compilation, archive verification, existing timeout, and merge-gate policy. Upload success is awaited and recorded before payload deletion; manifests, statuses, verifier, and shared compiled dependencies remain available. A failed record or upload does not prevent unrelated records from being attempted. Status-only retries do not upload an immutable archive twice.

Both native and WSL2 consumers validate the exact package, producer, key, and artifact before downloading the archive. Failure and cancellation stop the consumer with producer attribution; missing status reports an unknown outcome. WSL2 validation precedes guest provisioning. Successful status still requires the existing archive download and verifier to succeed. No replacement compilation, fabricated test report, or completion exemption was introduced. Runner-loss synthesis preserves uncertainty about individual records rather than asserting that no archive was uploaded.

| Requirement | Verification boundary and result |
| --- | --- |
| Upload before deletion; retain payload on upload failure; independent successful sibling; finalizer replay and status retries | Level 1: real filesystem fixtures with controlled asynchronous upload transport; Node suite passed all 14 tests. |
| Preserve shared dependency reuse and separate package feature graphs | Level 1: real Cargo builds through the coordinator and directory upload transport, followed by archive execution; passed in 3.669 seconds. |
| Sampler arguments, boundaries, refusal and failure behavior, and wrapper wiring | Level 1: public producer and shipped wrapper tests; selected tests passed. Python sampler suite passed all 17 tests. |
| Strict status parsing, matching identity, failed/cancelled/absent status, and CLI error location | Level 1: public Python parsing/check results and narrow CLI cases; all 21 status tests passed. |
| Consumer ordering, owner shape, verification, unchanged compile prohibition, and diagnostic preservation | Level 1: 11 focused test-toolkit workflow contracts passed. These establish workflow structure, not a hosted service execution. |
| Build-failure attribution without discarding actual test results or unrelated cells | Level 1: selected repo-deps rollup tests passed. |
| Every full-workspace Ubuntu record published within 180 minutes with positive observed disk space | Hosted execution required; unverified. Local fixtures do not establish runner capacity or hosted upload duration. |
| One WSL2 consumer verifies a newly produced archive and starts tests with actual reports | Hosted execution required; unverified. Later test failures remain outside this fix. |

Review execution used `just test test-toolkit` and `just test repo-deps` with explicit focused filters, totaling 11 and 13 passing Rust tests respectively. It also ran `node --test 'scripts/ci/artifacts/*.test.cjs'` and the two focused Python unittest suites. All selected Rust tests stayed below five seconds. Cargo emitted a nonfatal permission warning while cleaning its global registry cache; builds and tests succeeded. The implementation log records the earlier full package checks, companion suites, scoped Clippy, and actionlint; those were inspected rather than represented as fresh review runs. No terminal or keyboard behavior changed, so Level 2 or Level 3 terminal evidence is unnecessary.

## Input robustness

The new consumer build-status reader uses the existing strict JSON loader. Invalid documents and invalid load-bearing fields produce located errors and refuse download. Absence of the whole status file is separately reported as an unknown producer outcome.

The shared required-string validator is tested for absent, null, wrong-type, and empty values, with a control and invalid wiring case for each identity field, result, and stage. Identity mismatches and unknown results are rejected separately. Schema version validation rejects booleans, strings, floating-point values, unsupported integers, null, and absence. Optional detail intentionally treats absent and null as no diagnostic text and preserves empty strings. Duplicate keys, trailing garbage, malformed JSON, nonfinite constants, and a second document are rejected by the loader. CLI cases prove successful wiring and located malformed-document and mismatched-key failures. No collection matrix applies to this reader's decisions.

The sampler's plan read only annotates diagnostics; it neither schedules work nor authorizes cleanup. The coordinator and publisher consume existing producer-owned plan, manifest, status, and local ledger documents. Their changes do not introduce permissive defaults into the consumer authorization check.

## Implementation decisions (informational)

The following choices and deferrals were gathered independently from the implementation log and spec. I recommend retaining each; none needs a ruling before repair, and none blocks the cycle. The frontmatter provides a one-line author review item for each.

| Choice or deferred work | Review judgment and reason |
| --- | --- |
| Use the existing local 66-record replay to enable incremental publication | Accept the disclosed plan departure. The spec permits choosing a measured bounded remedy; approximately 6.2 GiB of retained archives supports releasing that category. The replay does not establish hosted capacity, and the implementation does not claim that it does. |
| Name the sampler `storage_sample.py` | Accept the importable module name and existing Python naming convention. |
| Supply sampler arguments as a JSON array | Accept exact argument passing without shell interpretation. The public parser rejects malformed argument arrays. |
| Sample at record boundaries rather than add redundant walks | Accept. In the final hosted path each keyed child samples at start and after its record, so publication and deletion occur between the previous after-record sample and the next start sample. The log's earlier once-per-owner description reflects the first phase, not the final coordinator. |
| Run the coordinator in a JavaScript action and launch the shell wrapper per record | Accept. The action retains the artifact-service credentials while children share the same target tree. The real-Cargo fixture exercises this direction. |
| Retry a missing status without re-uploading the archive; do not retry archives already reported as upload failures | Accept. This preserves immutable artifact names and failure attribution. The job remains failed even if finalization later publishes a missing status. |
| Publish the final compiler aggregate as `compiler-work-owner-{producer}` | Accept the internal diagnostic name. Per-record measurements remain accurate, and the total is emitted separately after production. |
| Find Windows Bash through PATH | Accept under existing hosted-runner provisioning, consistent with the former Bash workflow and fixture assumptions. Native Windows execution was not produced by this review; that evidence belongs to CI. |
| Rely on existing Node provisioning rather than new runner-tool metadata | Accept for the current runner setup: Ubuntu provisions Node through the registered companion suite, and the other producer images supply it. The fixture fails explicitly when Node is unavailable. |
| Leave WSL2 status checks unconditional | Accept because every WSL2 cell already unconditionally consumes an archive. Native consumers retain the archive-consumption condition. |
| Preserve rollup blocking behavior and place unknown-status wording in the consumer check | Accept. Rollup cannot infer a missing producer outcome from a missing consumer report. Known failure attribution and missing-completion enforcement remain intact. |
| Correct the stale publisher-suite registration paragraph | Accept. The code already registers the suite; the documentation was corrected to match it. |
| Use scoped Clippy instead of root-wide lint | Accept the plan's focused verification scope. |
| Reuse passing planner-suite results in the later documentation phase | Accept where the relevant source is unchanged; repeating that suite adds no new evidence. |
| Disable temporary packaging cleanup and cache isolation | Accept because no retained packaging scratch or restored cache bytes were established by the selected measurement. These conditional remedies are not mandatory additions. |
| Defer removal of the guard's approximately 6.5 GiB child target | Accept the conservative cleanup boundary. The measurement identifies a possible further saving but does not require introducing another deletion path alongside the selected archive remedy. |
| Leave capacity expansion, partitioning, and debug reduction unresolved | Accept. The hosted capacity premise has not been measured, so the spec's conditional escalation is not yet triggered. No speculative choice among those options is needed. |
| Finish the implementation phase with hosted acceptance unverified | Accept the evidence state explicitly permitted by the spec. Hosted free-byte/inode minima, duration, required-versus-published records, and WSL2 test start remain unperformed verification, not successful observations. Do not claim the storage objective achieved until those observations exist. |

No item meets the stopping-for-a-human test without an available recommendation inside the spec's boundaries. A future hosted execution proving insufficient capacity would supply new evidence for the spec's capacity decision; the present review does not authorize that change.
