---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Invalid workspace member entries can produce a false lockfile match
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T10:38:35-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-4.md
previous: 2026-09-26-lockfile-corroboration/review-3.md
next: 2026-09-26-lockfile-corroboration/review-5.md
---

# Review 4 — Lockfile corroboration

## Verdict

**Not ready for production.** Both unblocked findings from review 3 are fixed. Review 3 had no blocked findings, so none needed reassessment. One newly verified case lets malformed workspace declarations produce an exact lockfile match and stronger package provenance.

## Previous-review disposition

| Review 3 finding | Result |
|---|---|
| PHP-only Composer projects lose their standalone lockfile observation | Resolved. The Sniff library's [root-package fallback](../../lib/src/filesystem/repo/detection.rs) recognizes `composer.json` without a Node, Rust, Python, or Go manifest. The unmodified Composer fixture produces one root package, no workspace layer, and a repository-level lockfile observation under enabled and disabled requests. A shipped CLI Level 1 test checks JSON, plain output, stdout, stderr, and success. |
| A root-only uv workspace has no layer to report its lockfile match | Resolved. The Sniff library's [uv workspace detector](../../lib/src/filesystem/repo/uv.rs) distinguishes an explicit empty `members` array from an absent declaration. The real uv fixture now produces a layer with `match`, `uv.lock`, empty differences, and lockfile provenance; the disabled request reports `not_requested` without reading or parsing the lockfile. The repository remains a single-package project. |

Review 3 listed no blocked findings, and no earlier blocked finding became actionable in this iteration.

## Unblocked Findings

### High — Invalid workspace member entries can produce a false lockfile match

The Sniff library's [uv member reader](../../lib/src/filesystem/repo/uv.rs) and [pnpm member reader](../../lib/src/filesystem/repo/npm.rs) use `filter_map` to discard non-string values from a workspace's declared member array. A syntactically valid manifest such as `members = ["packages/alpha", 123]` or a pnpm `packages` list with `- 123` therefore becomes the same member set as a valid manifest. The lockfile comparison reports `match` and upgrades the layer and packages to `lockfile` provenance even though the declared membership was not fully understood. The same silent filtering pattern exists in the Sniff library's [Node workspace reader](../../lib/src/filesystem/repo/npm.rs) and [Cargo workspace reader](../../lib/src/filesystem/repo/cargo.rs).

I reproduced the public result with disposable copies of the checked-in uv 0.9.5 and pnpm 10.32.1 workspace fixtures, adding only a numeric entry to each manifest. Both shipped CLI JSON responses exited successfully and reported `status: "match"` with `provenance: "lockfile"`. This violates the specification's rule that incomplete manifest discovery cannot produce `match` or `mismatch`; consumers may treat a malformed workspace as independently corroborated.

Validate every declared member before comparing sets. An invalid workspace declaration should either fail detection as a manifest error or retain a layer marked `unverifiable` with `incomplete_manifest_discovery`; it must never be silently shortened and upgraded. Add Level 1 public-result tests for invalid member entries in uv, pnpm, Node, and Cargo manifests, asserting the status and unchanged provenance. Check both a valid list with one invalid entry and an all-invalid list, since the latter can currently suppress a layer entirely.

## Verification level by requirement

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Status, selected paths, member differences, and package provenance | Level 1 complete-result matrix and real-tool fixture tests | Appropriate level for data behavior, but invalid manifest member types have no Level 1 case and currently produce a false `match`. |
| Standalone Poetry, PDM, and Composer observations | Level 1 library fixture and shipped CLI tests | Appropriate; the PHP-only shape is now covered. |
| Root-only uv declaration and request costs | Level 1 public-result fixture and counter tests | Appropriate; enabled and disabled requests are covered. |
| JSON stdout, plain output, stderr separation, and exit status | Level 1 shipped CLI tests | Appropriate for noninteractive output. |
| Styled lists, wrapping, and status colors | Level 2 capture of the shipped CLI inside tmux | Appropriate; the focused real-terminal test passed. |
| Keyboard, mouse, paste, or input encoding | No such feature requirement | Level 3 is unnecessary. |

## Review checks

I read the specification, review 3, the detection and lockfile paths, the public fixture tests, the CLI renderer, and the target declarations. `just check-tier-coverage sniff` found zero stranded tests. `just test lockfile_` passed 80 selected Level 1 tests. `just test-l2 level2_lockfile_rendering::` passed the one Level 2 tmux test. The invalid-manifest reproductions used the shipped CLI and edited only disposable copies of pinned fixtures. Cross-OS results and human approval are outside this readiness decision, as requested.

## Human review

No human review is needed to resolve the finding. The specification already forbids an exact match when the manifest-side member set is incomplete.
