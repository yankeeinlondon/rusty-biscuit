---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Incomplete manifest discovery can produce an exact lockfile match
    - priority: high
      title: The uv parser treats missing membership data as an empty set
    - priority: high
      title: Rush silently drops invalid lockfile member paths
    - priority: high
      title: Styled terminal output has no real-terminal verification
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T21:17:40-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-1.md
next: 2026-09-26-lockfile-corroboration/review-2.md
---

# Review 1 — Lockfile corroboration

## Verdict

**Not ready for production.** The public result shape, source selection, request costs, real-tool fixtures, and CLI projections are substantially implemented. Three paths can still report stronger membership evidence than the files establish. The styled terminal report also lacks the required level of rendering verification. These findings can be resolved in code and tests without a new owner decision.

## Findings

### High — Incomplete manifest discovery can produce an exact lockfile match

The Sniff library's [DetectorOutcome](../../lib/src/filesystem/repo/topology.rs) carries only resolved package seeds. The [workspace glob expander](../../lib/src/filesystem/repo/glob.rs) drops unsupported patterns and filesystem walk errors without returning an incomplete-discovery signal. The [lockfile comparison](../../lib/src/filesystem/repo/lockfile/mod.rs) checks that an outcome exists, then compares its seeds with lockfile members. It cannot tell a complete manifest set from a partial one. Corroboration also runs in [repository detection](../../lib/src/filesystem/repo/detection.rs) before package enrichment parses member manifests, so a member manifest that fails to parse can still be counted as complete for npm, pnpm, Yarn, Bun, uv, or Rush. If the lockfile contains exactly the discovered subset, the result is `match` and package provenance is upgraded to `lockfile` even though the manifest-side set is not known to be complete.

The specification requires `unverifiable` with `incomplete_manifest_discovery` for an unresolved declared member, a failed member-manifest parse, or a reported glob bound. Carry completeness through detection and prevent both `match` and `mismatch` when it is false. Add Level 1 public-API cases for malformed member manifests and incomplete glob discovery, checking the full observation and that provenance remains manifest-derived. The existing Cargo-specific parse-failure test does not prove this behavior for the other authorities.

### High — The uv parser treats missing membership data as an empty set

The Sniff library's [uv lockfile parser](../../lib/src/filesystem/repo/lockfile/uv.rs) defaults a missing `[manifest].members` field to an empty list. An entirely absent `[manifest]` takes the same path. This makes a lockfile with no required membership record appear to contain a complete empty set: it can report `match` for an empty discovered workspace, or `mismatch` for an ordinary workspace. The [real-fixture integration test](../../lib/tests/l1/lockfile_fixtures.rs) explicitly expects `mismatch` from an edited workspace lockfile with `[manifest]` removed, so the test currently preserves the defect. The spec says missing membership fields are not empty sets; its root-only exception applies only when the document actually establishes that case.

Distinguish a supported root-only lockfile from a workspace lockfile whose membership record is absent or incomplete. Reject a present `[manifest]` without `members` as a parse failure, and classify an absent `[manifest]` conservatively when other local package records make root-only membership uncertain. Update the edited fixture expectation and add Level 1 cases for both forms, including a case where the manifest-side set is empty so a false `match` would be caught.

### High — Rush silently drops invalid lockfile member paths

The Sniff library's [Rush lockfile adapter](../../lib/src/filesystem/repo/lockfile/rush.rs) removes the synthetic `.` importer by filtering out every key whose path normalization fails. An absolute importer path or other invalid member path is therefore discarded before the shared comparison sees it. A lockfile with one invalid importer and otherwise matching members can report `match`, upgrading provenance. The specification requires `unverifiable` with `invalid_member_path` for any absolute or unrepresentable member path; dropping it also hides stale extras.

Remove only the actual synthetic root importer. Pass every other key to the shared path validator, then add a Level 1 Rush fixture with an invalid importer and assert the full observation, unchanged provenance, and no retry of another source.

### High — Styled terminal output has no real-terminal verification

The Sniff CLI's [lockfile renderer](../../cli/src/output/filesystem/lockfile.rs) adds status styling, nested lists, and selected-path/member lines. Its unit tests render with a default terminal and strip escape codes; the [shipped CLI integration tests](../../cli/tests/l1/lockfile_cli.rs) run `--plain` and collapse whitespace. Those tests prove the text content at Level 1. They do not prove that a terminal emulator renders the styled list, glyphs, wrapping, and fallback behavior promised by the CLI section of the spec. I found no lockfile or repository-structure capture in the Sniff CLI's Level 2 suite. Level 3 is unnecessary because this feature has no keyboard or mouse behavior.

Add a Level 2 test that runs the shipped CLI in the area's real-terminal harness, captures a mismatched layer with both extra and missing members at a constrained width, and checks the rendered lines and wrapping. Keep the existing Level 1 JSON and plain-text tests; they answer separate questions.

## Verification level by requirement

| User-observable requirement | Strongest evidence found | Assessment |
|---|---|---|
| All layer authorities report the required status, paths, and member differences | Level 1 complete-result fixture matrices and parser tests | Correct level, but the three false-evidence cases above are missing or asserted incorrectly |
| Only an exact match upgrades package provenance | Level 1 result and ownership-isolation tests | Correct level; incomplete inputs can still reach the exact-match branch |
| Standalone Poetry, PDM, and Composer files appear as repository observations | Level 1 library and shipped CLI tests | Appropriate level |
| Disabled requests avoid lockfile content reads, while full dependency enrichment can still read Cargo.lock | Level 1 work-counter and result tests | Appropriate level |
| JSON output is one valid document on stdout, and observations leave exit status unchanged | Level 1 spawned CLI tests | Appropriate level |
| Plain human output names status, paths, and differences | Level 1 spawned CLI tests using `--plain` | Appropriate for plain text |
| Styled terminal output renders lists, glyphs, and wrapping through a real terminal | Level 1 renderer unit tests only | Level 2 needed; finding above |

## Review checks

I inspected the feature specification, implementation, fixtures, and declared test targets. The new library and CLI Level 1 modules are declared by their consolidated test binaries. `just check-tier-coverage sniff` reported zero stranded tests. This review did not rerun the broad suites or cross-OS checks recorded in the implementation log; cross-OS proof is outside this review's readiness decision.

## Human review

No human review is required to resolve these findings. The specification already decides the relevant status and path semantics.
