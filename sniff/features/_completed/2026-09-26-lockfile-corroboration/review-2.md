---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Malformed Node member manifests abort repository detection instead of yielding an incomplete observation
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T07:12:17-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-2.md
previous: 2026-09-26-lockfile-corroboration/review-1.md
next: 2026-09-26-lockfile-corroboration/review-3.md
---

# Review 2 — Lockfile corroboration

## Verdict

**Not ready for production.** Three of the four findings from the first review are resolved. The incomplete-discovery fix works for uv member parse failures and dropped workspace patterns, but malformed npm and pnpm member manifests still make repository detection fail before Sniff can return the required lockfile observation. No finding from the first review was blocked, and no human decision is needed for this remaining behavior.

## Unblocked Findings

### High — Malformed Node member manifests abort repository detection instead of yielding an incomplete observation

The Sniff library's [nested workspace discovery](../../lib/src/filesystem/repo/nested.rs) dispatches each `package.json` it finds as a possible workspace root. The [npm workspace detector](../../lib/src/filesystem/repo/npm.rs) reads that file through `required_npm`, and a parse error propagates out of repository detection. This happens when an npm or pnpm workspace has a malformed member `package.json`, even if the workspace root and selected lockfile are readable. The new [integration tests](../../lib/tests/l1/lockfile_isolation.rs) explicitly accept that error as an alternative to `unverifiable` with `incomplete_manifest_discovery`; they therefore do not establish the public result required by the spec or the first review.

The spec requires an incomplete manifest-side set to produce `unverifiable` with `incomplete_manifest_discovery`, while preserving manifest-derived provenance. A caller currently receives no `RepoInfo` at all for this input, so it also loses the otherwise available repository and lockfile facts. Handle a malformed member manifest as incomplete membership without treating it as a fatal root-manifest error. Make the Level 1 npm and pnpm cases require the complete public observation and unchanged provenance; keep a separate test for a malformed workspace root if that must remain fatal.

## First-review disposition

| First-review finding | Result |
|---|---|
| Incomplete manifest discovery can produce an exact lockfile match | Partly resolved. Dropped patterns, uv member parse failures, and missing Rush project folders now block comparison. Malformed Node members still abort detection instead of returning the required observation. |
| The uv parser treats missing membership data as an empty set | Resolved. A present `[manifest]` without `members` is a parse failure; an absent table with other local packages reports `no_membership_data`. Root-only and empty-set cases have Level 1 checks. |
| Rush silently drops invalid lockfile member paths | Resolved. Only the synthetic `.` importer is removed; invalid paths reach validation. Level 1 checks the full result and provenance. |
| Styled terminal output has no real-terminal verification | Resolved. A declared Level 2 test runs the shipped CLI in tmux at a constrained width and checks rendered nesting, wrapping, and status styling. |

## Verification level by requirement

| User-observable requirement | Strongest verification | Assessment |
|---|---|---|
| Layer status, paths, differences, and package provenance | Level 1 complete-result and isolation tests | Appropriate level for data behavior; malformed Node members still need a complete-result assertion. |
| Standalone Poetry, PDM, and Composer observations; request costs; JSON and plain CLI output | Level 1 library and shipped CLI tests | Appropriate level for these noninteractive results. |
| Styled CLI lists, width, and colors | Level 2 tmux pane capture | Appropriate level; the focused test passed locally. |
| Keyboard, mouse, paste, or other input behavior | No such requirement in this feature | Level 3 is not needed. |

## Review checks

I read the spec, first review, changed implementation, and relevant test declarations. `just check-tier-coverage sniff` reported zero stranded tests. The focused `just test lockfile_isolation::` run passed 22 Level 1 tests; `just test-l2 level2_lockfile_rendering::` passed its one tmux test. The latter is declared by the CLI's consolidated Level 2 target, enabled by `test-fixtures`, and selected by a live `test-l2` recipe. Cross-OS proof and human review are outside this readiness decision, as the request specifies.

## Human review

No human review is required to resolve the finding. The specification already defines the expected status and provenance.
