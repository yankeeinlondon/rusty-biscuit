---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Wrong-type workspace member fields silently remove lockfile observations
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T10:56:33-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-5.md
previous: 2026-09-26-lockfile-corroboration/review-4.md
next: 2026-09-26-lockfile-corroboration/review-6.md
---

# Review 5 — Lockfile corroboration

## Verdict

**Not ready for production.** The unblocked finding from review 4 is fixed for invalid entries *inside* member arrays. Review 4 had no blocked findings, and none became unblocked before this implementation. A whole member field with the wrong type still causes a workspace layer, and its lockfile observation, to disappear.

## Previous-review disposition

The Sniff library now records a non-string array entry as incomplete discovery in the [uv workspace reader](../../lib/src/filesystem/repo/uv.rs), [Node and pnpm workspace readers](../../lib/src/filesystem/repo/npm.rs), and [Cargo workspace reader](../../lib/src/filesystem/repo/cargo.rs). The Level 1 public-result fixture test covers mixed and all-invalid arrays for uv, pnpm, npm, and Cargo, plus invalid Cargo exclusions and Rush projects. It checks `unverifiable` with `incomplete_manifest_discovery` and confirms that neither layers nor packages gain lockfile provenance. That test passes. Review 4 had no blocked findings.

## Unblocked Findings

### High — Wrong-type workspace member fields silently remove lockfile observations

The Sniff library's [pnpm workspace reader](../../lib/src/filesystem/repo/npm.rs) treats a present `packages` field that is not a sequence as an empty declaration; its detector then returns no layer. The [uv workspace reader](../../lib/src/filesystem/repo/uv.rs) likewise treats a present, non-array `members` field as absent. The [Cargo workspace reader](../../lib/src/filesystem/repo/cargo.rs) and the Node `workspaces` reader in the same npm source file also convert a wrong-type whole field to an empty declaration. These paths bypass the new invalid-entry tracking, so there is no `unverifiable` result to tell consumers that membership could not be understood.

I reproduced this with a disposable pnpm project containing a root and member `package.json`, `pnpm-workspace.yaml` with `packages: 123`, and a valid pnpm lockfile naming that member. The shipped `sniff repo structure --json` command exited successfully with no stderr, but `monorepo_layers` was omitted and only one root package remained. The lockfile was never reported. The fixture test added for review 4 uses only arrays or sequences, so it cannot catch this case.

Treat a present member field of the wrong type as an invalid declaration. Keep enough workspace identity to report `unverifiable` with `incomplete_manifest_discovery`, or return a manifest error rather than silently dropping the layer. Add Level 1 public-result cases for wrong-type whole fields in pnpm, uv, Node, and Cargo manifests, including the lockfile observation and package provenance. This matters because the feature promises a truthful observation for each detected workspace and forbids treating an incomplete manifest-side set as established membership.

## Verification level by requirement

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Status, selected paths, member differences, and provenance | Level 1 complete-result matrix and real-tool fixtures | Appropriate level for data behavior; wrong-type whole member fields are untested and currently suppress the observation. |
| Standalone Poetry, PDM, and Composer observations | Level 1 library and shipped CLI tests | Appropriate for repository data. |
| JSON stdout, plain output, stderr separation, and exit status | Level 1 shipped CLI tests | Appropriate for noninteractive output. |
| Styled lists, wrapping, and status colors | Level 2 capture of the shipped CLI inside tmux | Appropriate for real-terminal rendering; the focused test passes. |
| Keyboard, mouse, paste, or input encoding | No such feature requirement | Level 3 is unnecessary. |

## Review checks

I read the specification, review 4, the workspace readers and lockfile comparison, the fixture tests, CLI output tests, and declared test targets. `just check-tier-coverage sniff` found zero stranded tests. `just test lockfile_` passed all 81 selected Level 1 tests, and the Level 2 real-terminal rendering test passed. I also reproduced the wrong-type pnpm field through the shipped CLI on a disposable project. Cross-OS results and human approval are outside this readiness decision, as requested.

## Human review

No human review is needed to resolve the finding. The specification already says incomplete manifest discovery must not establish a lockfile match.
