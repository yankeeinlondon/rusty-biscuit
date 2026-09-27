---
$schema: feature-review.yaml
ready: false
findings:
    - priority: medium
      title: The measured pnpm lockfile cost meets the plan's parser trigger, but the parser remains unchanged
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T16:02:45-07:00
spec: 2026-09-21-lockfile-provenance-cost/spec.md
implemented: false
description: "A **fix** review of `2026-09-21-lockfile-provenance-cost/spec.md`"
fix: 2026-09-21-lockfile-provenance-cost/review-3.md
previous: 2026-09-21-lockfile-provenance-cost/review-2.md
---

# Review 3 — Lockfile provenance cost

## Verdict

**Not ready for production.** The two unblocked findings from review 2 are addressed, and the request default is settled. One measured performance requirement remains unimplemented for callers that explicitly request pnpm lockfile provenance.

## Previous review findings

| Review 2 finding | Result |
|---|---|
| Complete repository results are not verified against the pre-change contract | Addressed. The Sniff library's [Level 1 lockfile matrix](../../lib/tests/l1/lockfile_provenance.rs) compares the complete serialized `RepoInfo` with independently written expected documents for 15 Cargo, pnpm, and uv fixture states under both corroborating structure and full requests. The implementation log records a successful comparison against the pre-change implementation. The [Sniff CLI test](../../cli/tests/l1/cli.rs) also compares a complete JSON result. |
| The required isolated corroboration measurement is missing | Addressed. The [results](results.md) separate the corroboration step from Cargo parsing, full detection, and Darkmatter composition. They report debug and release medians, ranges, load, alternating runs, and a pnpm-authoritative cost check. |
| The structure-request default still awaits the author's decision | Resolved before this review. The author's instruction in review 2 was to finish the work. The [implementation log](implementation-log.md) records the opt-in structure default and preserves lockfile-confirmed output for bare `sniff repo --json`; the [specification](spec.md) records the same decision. No human choice remains blocked. |

## Unblocked Findings

### Medium — The measured pnpm lockfile cost meets the plan's parser trigger, but the parser remains unchanged

The [implementation plan](plan.md) says to replace the generic pnpm lockfile parse with a typed importer-keys parse if pnpm corroboration takes at least 10% of the corroboration step on a pnpm-authoritative release fixture. The new [measurement](results.md) says that threshold was met: the pnpm parse accounts for essentially the whole step. On its 3 MB fixture, parsing took about 45 ms in release mode; a typed importer-keys probe took about 34 ms, a measured saving of about 12 ms. Yet the Sniff library's [`ManifestStore::pnpm_lock`](../../lib/src/filesystem/repo/detection.rs) still parses the entire YAML file into a generic value. The implementation log explicitly leaves this optimization to a possible later fix.

This affects `RepoRequest::full()`, explicit provenance requests, and Sniff CLI commands that preserve lockfile-confirmed output. The opt-in structure default removes the cost for ordinary structure callers, but it does not fulfill the plan's conditional optimization for those who still request provenance. The suggested typed parser needs to keep the current importer-key matching behavior and include Level 1 parity tests for matching, extra, missing, and malformed lockfiles. If the measured saving is intentionally outside this fix, amend the specification and plan with a replacement acceptance decision before treating the fix as complete; the current documents require the parser when the measured condition is true.

## Blocked Findings

None.

## Verification level by requirement

| User-observable requirement | Strongest verification | Assessment |
|---|---|---|
| Structure requests avoid lockfile reads and keep manifest provenance | Level 1 fixture matrix with fresh read and parse counters | Appropriate and passes |
| Requested provenance preserves complete Cargo, pnpm, and uv results | Level 1 complete-result matrix across 15 lockfile states, plus documented comparison with the pre-change implementation | Appropriate and passes |
| Legacy serialized requests and new constructor defaults retain their documented choices | Level 1 request serialization and constructor tests | Appropriate and passes |
| Cargo version lookup retains duplicate-name order and malformed-entry behavior | Level 1 typed-versus-reference parser tests, including this checkout's `Cargo.lock` | Appropriate and passes |
| CLI JSON remains valid and reports the chosen request's provenance | Level 1 spawned CLI tests and a complete JSON expectation for `repo structure --json` | Appropriate and passes |
| Darkmatter compose uses the cheaper structure request without changing observation count | Level 1 affected tests recorded in the implementation log; separate compose timing evidence | Appropriate functional level |
| Requested pnpm provenance has the conditional parsing optimization | Release and debug cost measurements, but no typed parser or parity tests in the shipped code | Implementation gap above; Level 1 parity tests will be needed with the parser |

These behaviors concern filesystem observation and JSON output. None asks a terminal emulator to render text or encode keyboard input, so Level 2 and Level 3 verification do not apply.

## Verification performed for this review

- Ran `just test lockfile_provenance::` in `sniff/`: all three selected Level 1 tests passed.
- Confirmed the matrix module is declared by the Sniff library's consolidated `l1` test target and its test names are selected by the Level 1 tier.
- Reviewed the isolated measurement protocol and the previous implementation log. Broader Sniff and affected-package test and lint results are recorded there; this review did not rerun those suites.
