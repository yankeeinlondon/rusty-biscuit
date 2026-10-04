---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
  - |-
      Retain the review requested in review-2.md after the context-validation defect recurred. Inspect whether the implementation and regression tests check every captured directory and preserve an invalid originating context across document derivation. The tests now cover replacement launch directories, all six derivation methods, overwritten package anchors, both writers, and the public readers. This is a review of sweep completeness; no new design choice or reaffirmation of an earlier decision is requested.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-01T07:22:46-07:00"
spec: "2026-09-30-reusable-path/spec.md"
implemented: false
description: "A **feature** review of `2026-09-30-reusable-path/spec.md`"
feature: "2026-09-30-reusable-path/review-3.md"
previous: "2026-09-30-reusable-path/review-2.md"
---

The feature is **production ready**. The previous review's unblocked finding is implemented, the earlier fixes remain intact, and this review found no additional correctness, coverage, performance, or ergonomics issue that blocks readiness. The human inspection requested by review #2 remains an external follow-up and does not change this assessment.

This review examined the specification, both implementation logs, both earlier reviews, context selection and derivation, resolver boundaries and completion, portable strategy evaluation, path identity and text generation, Darkmatter normalization, consumer documentation, and test registration. Existing working-tree changes were preserved. No implementation code or tests were changed.

## Previous review findings

Review #2 has one unblocked finding: **Context validation still admits relative launch directories and hides invalid request settings**. Its blocked-findings section says “None”; there was no blocked implementation work to unblock. Its human-review request concerned sweep completeness rather than a missing design decision.

In the `biscuit-file` package, [FileResolutionContext::validate](../../lib/src/file_reference/context.rs) now checks the replacement launch scope's own request directory. The shared document-derivation method captures the originating context's typed validation failure before clearing explicit roots or reselecting repository/package anchors. Further derivations preserve that failure. A builder can correct a setting before derivation; a builder applied afterward cannot erase the originating error. The rustdoc, [file-reference topic page](../../docs/topics/file-references.md), and skill references describe that behavior consistently.

The [originating-context regression matrix](../../lib/tests/l1/portable_path/originating_context.rs) exercises the same isolated filesystem fixture through public results. All rows below passed with the expected outcome:

| Site | Shape tested | Observed and expected result |
| --- | --- | --- |
| Replacement launch scope | Relative request directory, repository root, package root, or package-area root; otherwise valid context | Typed relative-directory error before probing |
| All six ordinary/trusted directory, source, and reference-aware derivation methods | Relative original explicit root, in-tree and external destinations | Original relative-directory error retained |
| All six derivation methods | Absolute explicit root conflicting with the repository root | Original root-conflict error retained |
| All six derivation methods, with and without a repository catalog | Relative current package or package-area root, independently valid replacement launch scope | Original relative-directory error retained despite anchor clearing or reselection |
| Builders and repeated derivation | Correct invalid setting before deriving; attempt correction afterward; derive again | Correction before derivation succeeds; later correction and repeated derivation retain the original failure |
| Valid-context controls | Repository context, catalog context, valid replacement launch scope, accepted ordinary and trusted destinations | Both writers and the reader reach the target |
| Separate relative-value contracts | Relative environment values and configured magic/vault roots | Valid context remains valid |
| Process-directory independence | One captured relative launch scope; file available under only one of two process directories | Same configuration error from both directories |

For every invalid-context row, the shared assertion checks direct validation, both `PortablePath` constructors with an absolute-only strategy, convenience and detailed resolution, candidate planning under both ordering policies, bare and magic completion, and ordinary/recursive magic resolution. Writer attempts are empty, proving rejection precedes strategy execution. The existing [relative-context tests](../../lib/tests/l1/portable_path/relative_context.rs) additionally cover ordinary request/document directories, current and retained launch anchors, home, and relative destinations through every derivation wrapper.

The other defect classes from review #1 remain resolved:

| Earlier class | Sibling sites checked | Result |
| --- | --- | --- |
| Root-parent normalization disagreement | Shared identity/native normalization, resolver and context consumers, absolute/reference writer inputs, root and anchor selection, searched roots, catalog scope selection, relative projection, recursive resolution | Clean; [excess-parent regressions](../../lib/tests/l1/portable_path/excess_parent.rs) passed |
| Failed strategy omitted from attempt history | Shared evaluator/verification path, both searched preferences, earlier candidate rejections and preferences, separate target-preparation failure | Clean; [probe-failure regressions](../../lib/tests/l1/portable_path/probe_failure.rs) passed and retain typed path/error information |
| Missing absolute-fallback warning | Darkmatter's Markdown link/image and HTML hyperlink, image, video, audio, source, iframe, script, stylesheet, and font destinations | Clean; default-warning and explicit-suppression tables passed for all eleven forms; evaluation failures still warn independently |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No defect class recurs in this review, so `recurrence` is `false`. The context-validation class reported by reviews #1 and #2 is now resolved across the listed sibling sites. Review #2's human inspection request is retained rather than treated as completed without evidence.

## Requirement coverage

| User-facing requirement | Relevant verification present | Assessment |
| --- | --- | --- |
| Tree-root precedence, origin, repository conflicts, fallback exception, opening home/environment anchors, ordinary/trusted derivation, unchanged launch scope | Level 1 public filesystem tests in [file_tree](../../lib/tests/l1/file_tree.rs), [resolution_context](../../lib/tests/l1/resolution_context.rs), and originating/relative-context matrices | Appropriate |
| Effective relative boundaries after interpolation, bare fallback candidates, symlink landing and missing descendants, reader opt-in, repository-only sigils, recursive roots, completion | Level 1 public resolver tests in `file_tree` and portable platform/input tests | Appropriate |
| Default/reordered strategies, all route shapes, equal-to-CWD directories, filters, searched-root order, shadowing, missing/non-file targets, typed diagnostics | Level 1 public-result tests in [strategies](../../lib/tests/l1/portable_path/strategies.rs), [inputs](../../lib/tests/l1/portable_path/inputs.rs), and probe-failure tests | Appropriate |
| Authored intent, URL/recursive preservation and refusal, minimal churn, idempotence, candidate equality, stable failures, captured-state isolation | Level 1 input and [corpus property tests](../../lib/tests/l1/portable_path/properties.rs), plus process-directory regression | Appropriate |
| Portable-name declarations, unusable values, deepest-prefix/name ties, deduplication | Level 1 public-result [environment matrix](../../lib/tests/l1/portable_path/environment.rs) | Appropriate |
| Lossless identity, root clamping, non-Unicode names, literal backslashes/interpolation, Windows drives, UNC/verbatim spellings and distinct roots | Level 1 identity/text tests and [platform regressions](../../lib/tests/l1/portable_path/platform.rs), including Windows-gated cases | Appropriate; OS-specific execution remains CI's responsibility |
| Darkmatter rewriting, suffixes, child-document contexts, environment-reference recomposition, fallback warnings and suppression | Level 1 library/CLI integration tests, normalization tables, and [consumer tree-root tests](../../../darkmatter/lib/tests/l1/file_tree_roots.rs) | Appropriate |

These requirements concern file resolution and document text/report output. They introduce no terminal rendering, keyboard encoding, focus, or scrolling behavior requiring Level 2 or Level 3 tests. The file-format robustness matrix is not applicable: this feature introduces no manifest, lockfile, or serialized configuration reader. Environment string declarations and values have their own public-result matrix.

The portable-path modules are declared through the consolidated `l1` test target; their names select Level 1. The layout gate passed, CI metadata enables the separate fetch target, and tier coverage reported zero stranded tests in both affected areas.

The previously reviewed departures remain understood: richer candidate diagnostics, additional typed errors, boxed reference errors, spelling from the selected magic filter root, and Darkmatter finalization consuming already resolved absolute destinations. Preserving authored intent through the entire composition pipeline remains a separate provenance change, not a new defect here. Context capture is reused for document batches; no additional performance blocker was found.

## Verification

Executed on macOS against the current working tree:

- `just test` in `biscuit-file`: **1,011 Level 1 tests passed**, zero skipped, plus the recipe's six no-default-features path-text checks.
- `just lint` in `biscuit-file`: passed.
- `just lint` in `darkmatter`: passed, including its Zed WASM extension compile check.
- `just test link_normalization` in `darkmatter`: **21 selected tests passed**.
- `just test file_tree_roots` in `darkmatter`: **five selected consumer integration tests passed**.
- `just check-tier-coverage biscuit-file` and `just check-tier-coverage darkmatter`: passed, zero stranded tests.

Cross-OS execution evidence is left to CI and is not a condition of this review's readiness decision. The feature directory remains active for the author to close after review.
