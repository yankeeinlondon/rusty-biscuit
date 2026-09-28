---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Structure detection still reads lockfiles without a request for provenance
    - priority: high
      title: Cargo lockfiles still use the generic TOML parser
    - priority: high
      title: Requested behavior and performance have no new verification
    - priority: high
      title: Public compatibility policy has no consumer audit or author decision
human_review: true
human_review_items:
    - |-
        Choose the default for lockfile provenance after the implementation records which callers read or serialize the result. Sniff currently checks lockfiles even for its cheapest repository-structure request. Changing that default saves work for existing callers but changes their reported provenance.

        Please choose one approach:

        1. Make lockfile provenance opt-in for structure requests. Existing structure callers, including Darkmatter, become faster and their reported provenance changes. Full requests retain the current result.
        2. Keep the current structure default and let callers explicitly opt out. Existing output remains stable, but Darkmatter and other cost-sensitive callers must change their requests to gain the saving.

        The specification recommends option 1 if the consumer audit finds no caller that requires the old structure result. This decision blocks the request-default portion of the first finding; the parser and verification work can proceed independently.
has_blocked_findings: true
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T12:52:46-07:00
spec: 2026-09-21-lockfile-provenance-cost/spec.md
implemented: true
implemented_by: claude/default
log: sniff/fixes/2026-09-21-lockfile-provenance-cost/implementation-log.md
description: "A **fix** review of `2026-09-21-lockfile-provenance-cost/spec.md`"
fix: 2026-09-21-lockfile-provenance-cost/review-1.md
next: 2026-09-21-lockfile-provenance-cost/review-2.md
---

# Review 1 — Lockfile provenance cost

## Verdict

**Not ready for production.** This review found the specification and existing lockfile behavior, but no implementation of the two proposed changes. The specification is still marked `draft-spec`, and this fix directory has no implementation log, test fixtures, or performance results. This is a review of the current worktree, not a claim that existing behavior regressed.

## Findings

### High — Structure detection still reads lockfiles without a request for provenance

The Sniff library's [`RepoRequest`](../../lib/src/request.rs#L776), which chooses repository detection detail, has no lockfile-provenance setting. Its structure and focused constructors therefore cannot decline corroboration, and legacy serialized requests cannot exercise the specified compatibility rule. Repository detection calls [`upgrade_provenance_with_lockfile`](../../lib/src/filesystem/repo/detection.rs#L747) for every detected layer regardless of the request. The public [`detect_repo_structure`](../../lib/src/filesystem/repo/types.rs#L532) path reaches that same call. On a workspace with a recognized lockfile, the cheapest structure request still reads and parses it and reports lockfile-derived provenance.

Add the request setting, gate only corroboration on it, preserve dependency-version reads when requested, and document the new meaning of an omitted `lockfile_match`. Add a stable read-attempt counter at each lockfile read path, including the Cargo parser, so a fresh work collector can prove that structure-only detection attempts no lockfile reads. Update the public request and result docs, Sniff's request-cost guidance, and affected README descriptions. The choice of structure's default is blocked by the author decision below; either choice needs an explicit way to decline the work.

### High — Cargo lockfiles still use the generic TOML parser

The Sniff library's [`CargoLockVersions::parse`](../../lib/src/filesystem/repo/manifest_index.rs#L26) still deserializes the complete lockfile into `toml::Value`, retaining data such as checksums and dependency lists before extracting names and versions. That is the allocation cost the second change aims to remove. Use a typed TOML representation that keeps only the necessary fields while preserving the current behavior for malformed entries and duplicate names in lockfile order. Retain the current parser as a test-only reference for parity checks. The specification makes a typed pnpm parser conditional on measurement; its absence is not a separate finding yet.

### High — Requested behavior and performance have no new verification

Existing Sniff [lockfile integration tests](../../lib/tests/l1/integration.rs#L728) assert that structure detection *does* upgrade pnpm and uv provenance. They do not exercise requested versus declined corroboration, legacy request deserialization, zero read and parse counts, complete `RepoInfo` parity across Cargo/pnpm/uv cases, or the typed Cargo parser's ordered versions. No implementation log records the consumer audit or the specified alternating before-and-after timing samples for detection and a Darkmatter compose.

Add controlled Level 1 fixtures with independent expected results and work-counter assertions. Update the existing structure tests for the chosen default, and verify full and explicitly requested provenance still match the old result. Run Sniff's Level 1 tests and lint, then the affected Darkmatter and Claudine tests. Any new Sniff integration test must be declared in the consolidated `l1` target and selected by the running tier; the existing `integration` module meets that placement rule. Performance results should distinguish parser, detection, and compose costs. Cross-operating-system CI evidence and human approval are separate from this readiness verdict.

### High — Public compatibility policy has no consumer audit or author decision

**Blocked by the author's choice after a consumer audit.** The spec intentionally leaves open whether existing structure callers should receive a cheaper result with changed provenance or retain the old result unless they opt out. The Sniff CLI calls structure detection for repository commands, and Darkmatter's [`repository capture`](../../../darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs#L447) calls it for ambient composition. No implementation log records which callers actually use the provenance fields or serialize a complete result, including Claudine and any CI inputs. Without that audit, choosing the public default could silently change a consumer's output.

Record each consumer, its request tier, and whether it needs the old value. Then adopt the author's selected default and add request and CLI JSON tests for that policy. This decision blocks only the default; it does not block implementing the opt-out mechanism or the typed parser.

## Verification level by requirement

| User-observable requirement | Strongest evidence now | Needed level |
|---|---|---|
| Structure callers can avoid lockfile work and see manifest-derived provenance | No test; existing Level 1 tests expect lockfile provenance | Level 1 library and CLI tests with fresh work counters |
| Explicit provenance preserves complete Cargo, pnpm, and uv results | Level 1 tests for selected pnpm and uv fields only | Level 1 controlled complete-result fixtures |
| Legacy requests and new request defaults serialize as specified | Existing request round-trip tests lack the new field | Level 1 serialization tests |
| Cargo version resolution retains duplicate-name ordering and malformed-entry behavior | Existing parser test covers direct resolution, not old/new parity | Level 1 parser parity tests |
| Darkmatter compose remains functionally identical while costing less | Existing compose tests predate this change; no matched timings | Level 1 affected tests and measured performance evidence |
| CLI JSON reflects the selected request tier | Existing CLI snapshots predate this change | Level 1 CLI process tests |

These changes concern filesystem reads, data serialization, and ordinary CLI JSON. They do not require real-terminal rendering or keyboard input, so Level 2 and Level 3 tests are not applicable. I did not run tests or lint: the requested implementation is absent, and a passing baseline suite would not verify it.
