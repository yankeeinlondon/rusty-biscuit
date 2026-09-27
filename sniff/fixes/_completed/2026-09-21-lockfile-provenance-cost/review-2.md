---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Complete repository results are not verified against the pre-change contract
    - priority: medium
      title: The required isolated corroboration measurement is missing
    - priority: high
      title: The structure-request default still awaits the author's decision
human_review: true
human_review_items:
    - |-
        Please choose the default for lockfile checking when an application asks Sniff only for repository structure. The implementation audit found that current structure callers use package paths and names, not the lockfile-derived provenance value. The current change provisionally uses option 1; your choice determines the public library behavior.

        1. Keep the current opt-in default. Structure detection avoids opening lockfiles and reports provenance from manifests. Applications that need lockfile confirmation request it explicitly.
        2. Restore the old default. Structure detection continues checking lockfiles unless an application opts out; Darkmatter must explicitly opt out to gain the measured saving.

        Please also confirm whether bare `sniff repo --json` should keep its existing lockfile-confirmed provenance. The current implementation opts that command in, preserving its output while retaining the lockfile cost for that command.
has_blocked_findings: true
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T13:59:35-07:00
spec: 2026-09-21-lockfile-provenance-cost/spec.md
implemented: true
implemented_by: claude/default
log: sniff/fixes/2026-09-21-lockfile-provenance-cost/implementation-log.md
description: "A **fix** review of `2026-09-21-lockfile-provenance-cost/spec.md`"
fix: 2026-09-21-lockfile-provenance-cost/review-2.md
previous: 2026-09-21-lockfile-provenance-cost/review-1.md
next: 2026-09-21-lockfile-provenance-cost/review-3.md
---

# Review 2 — Lockfile provenance cost

## Verdict

**Not ready for production.** The request setting, conditional lockfile reads, typed Cargo parser, and consumer audit address the first review's main implementation findings. The result-verification and performance evidence still fall short of explicit specification requirements. The author has not yet confirmed the public default chosen provisionally by the implementation.

## Previous review findings

| Finding from review 1 | Status in this review |
|---|---|
| Structure detection reads lockfiles without a provenance request | Implemented. The Sniff library's [`RepoRequest`](../../lib/src/request.rs) now selects corroboration, and [repository detection](../../lib/src/filesystem/repo/detection.rs) gates it. Fresh-counter fixture tests prove zero lockfile reads and parses for a declining structure request and preserve dependency-version reads. |
| Cargo lockfiles use the generic TOML parser | Implemented. The Sniff library's [`CargoLockVersions`](../../lib/src/filesystem/repo/manifest_index.rs) now retains names and ordered versions through typed deserialization. The old parser remains as a test-only comparison, including malformed entries and this checkout's lockfile. |
| Requested behavior and performance lack verification | Partly implemented. The new Level 1 matrix, request serialization tests, CLI JSON tests, and timed parser/detection/compose runs provide useful evidence. The two unblocked findings below identify what remains. |
| Compatibility policy lacks an audit and author decision | Audit implemented; author decision remains blocked. The [implementation log](implementation-log.md) inventories callers and explains the provisional opt-in default. No subsequent author choice appears in the record. |

The first review's blocked decision was therefore not unblocked before implementation. The implementation proceeded provisionally under its plan and retained a reversible default setting. This review does not treat the pending human choice or cross-OS CI evidence as a production-readiness defect by itself.

## Unblocked Findings

### High — Complete repository results are not verified against the pre-change contract

The spec requires complete `RepoInfo` output to remain unchanged when provenance is requested, across Cargo, pnpm, and uv lockfile states. The new [Level 1 fixture matrix](../../lib/tests/l1/lockfile_provenance.rs) checks a complete `MonorepoLayer`, but its `PackageIdentity` projection checks only seven package fields; it also checks only selected top-level fields. It does not compare the full serialized result or every field of each `Package` against an independent expectation or captured pre-change result. The [CLI JSON tests](../../cli/tests/l1/cli.rs) similarly inspect selected keys.

This matters because a request that opts in promises the same complete public result, not only the same provenance labels. A change to another package or repository field could pass these tests. Add a controlled Level 1 fixture that asserts the full result for the requested path against independently specified expected JSON or a reviewed pre-change fixture for each authority and lockfile state. Keep the current hand-written provenance assertions; they explain the intended Cargo versus pnpm/uv membership rule.

### Medium — The required isolated corroboration measurement is missing

The spec's performance protocol asks for `upgrade_provenance_with_lockfile` in isolation, before and after the typed parser change, in debug and release. The [results](results.md) explicitly say this was not measured and use “requested detection minus declined detection” as a proxy. That difference includes more than the corroboration function and compares two full detections, so it cannot establish the isolated function's cost or its change. The implementation plan also called for a pnpm-authoritative fixture to decide whether a typed pnpm parser is warranted; the log records no such measurement or decision.

Measure the corroboration step in isolation with the specified warmups, alternating samples, load readings, medians, and ranges. Record the pnpm-authoritative cost check and its decision, even if the decision is to leave pnpm parsing as it is. The existing parser, full-detection, and compose timings should remain separate.

## Blocked Findings

### High — The structure-request default still awaits the author's decision

**Blocked by human review; excluded from the readiness verdict.** The consumer audit supports the current opt-in default: direct structure callers do not use or serialize lockfile provenance, and the CLI's complete-result commands still request it. The [plan](plan.md) and [implementation log](implementation-log.md) nevertheless call this default provisional and say the author must choose. The author should select the structure default and confirm whether bare `sniff repo --json` should preserve its existing lockfile-confirmed output at the current cost. The precise choices and consequences are in `human_review_items` above.

WHY IS THIS HAPPENING. TOO MUCH. DO THE WORK. If you really need human involvement that you MUST add more context, provide examples, offer a recommendation and ideally express pros/cons of each option. 

## Verification level by requirement

| User-observable requirement | Strongest present verification | Assessment |
|---|---|---|
| Structure requests skip lockfile reads and report manifest provenance | Level 1 fresh-counter fixture matrix for all three authorities; new tests are declared by the Sniff library's `l1` target and selected by its tier | Appropriate level; passes |
| Requested provenance preserves complete Cargo, pnpm, and uv repository output | Level 1 checks for all layer fields and selected repository/package fields | Level 1 is appropriate, but assertions do not cover the complete result; high finding above |
| Legacy request plans and new request defaults retain their documented behavior | Level 1 serialization and constructor tests | Appropriate level; passes |
| Cargo duplicate-name order and malformed entries match the old parser | Level 1 test-only reference parser, synthetic cases, and this checkout's `Cargo.lock` read through `include_str!` | Appropriate level; passes |
| Dependency-version requests can read a lockfile while declining corroboration | Level 1 counter and resolved-version assertion | Appropriate level; passes |
| CLI JSON remains valid and reflects the chosen command's request | Level 1 spawned CLI tests using the declared `l1` target | Appropriate level; passes for tested commands |
| Darkmatter composition retains its fixed observation and benefits from cheaper detection | Three affected Level 1 observation tests passed according to the implementation log; timed whole-process compose evidence exists | Appropriate functional level; timing is directional evidence |

These requirements concern filesystem observation, serialization, and ordinary CLI JSON. No requirement depends on terminal rendering or keyboard input, so Level 2 and Level 3 tests are not applicable.

## Verification performed for this review

- Ran focused Sniff library tests: 8 passed, including the provenance matrix and typed-parser parity tests.
- Ran focused Sniff CLI tests: 3 passed, including JSON validity and lockfile provenance for full and aggregate commands.
- Checked that the new integration module is declared in the library's consolidated `l1` target and that the selected test names have no tier marker.
- Reviewed the implementation log's earlier broader runs: Sniff tests and lint passed; affected Claudine tests passed; Darkmatter's three observation tests passed, while two unrelated fixture tests failed because their prompt fixture lacks a referenced file. This review did not rerun those broad suites.
