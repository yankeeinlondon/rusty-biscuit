---
$schema: feature-review.yaml
ready: false
findings:
  - title: Context validation still admits relative launch directories and hides invalid request settings
    priority: high
human_review: true
human_review_items:
  - |-
      Before restarting the automated review/fix loop, inspect the context-validation sweep and its regression tests. The previous fix rejected relative directories in ordinary contexts but missed the separately replaceable launch directory and settings discarded during document derivation. Confirm that every captured directory is checked and that deriving a document cannot turn an invalid originating request into a valid one. This asks for a review of the implementation and test completeness; the existing design does not need a new decision.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-01T03:15:03-07:00"
spec: "2026-09-30-reusable-path/spec.md"
implemented: true
implemented_by: claude/opus
log: biscuit-file/features/2026-09-30-reusable-path/log.md
description: "A **feature** review of `2026-09-30-reusable-path/spec.md`"
feature: "2026-09-30-reusable-path/review-2.md"
previous: "2026-09-30-reusable-path/review-1.md"
next: "2026-09-30-reusable-path/review-3.md"
---

The feature is **not production ready**. Three earlier findings are resolved. The context-validation finding is partly resolved, but the same defect class remains in the launch snapshot and document derivation. One high-priority finding below carries the complete reproduced sweep. Its implementation is unblocked; human review is requested because the class recurred, not because the fix requires a new design decision.

This review covers the specification, both implementation logs, the earlier review, path identity and spelling, strategy evaluation and diagnostics, context selection and derivation, resolver boundaries and completion, Darkmatter composition, affected documentation, and test registration. Existing implementation edits were preserved. Temporary public-API probes used the existing isolated filesystem fixture and were removed afterward. No production code was changed.

## Previous review findings

The earlier review has a single `Findings` section rather than separate unblocked and blocked sections. All four findings were unblocked. It declares no blocked findings or outstanding human decisions, so there was nothing to unblock before this implementation.

| Earlier finding | Result in this iteration | Verification |
| --- | --- | --- |
| Path normalization disagrees at filesystem roots | Resolved for the reported class | The resolver now shares the identity module's component-collapse rule. The [root-parent regression tests](../../lib/tests/l1/portable_path/excess_parent.rs) verify absolute path and reference inputs, convenience and detailed resolution, context/root containment, vault selection, home/environment generation and opening anchors, isolated magic roots, catalog selection, relative projection, and recursive resolution. Windows adds an excess-parent drive-root case. Existing identity, catalog-validation, planning, and completion controls remain consistent with their contracts. |
| Relative context directories bypass configuration validation | Partly resolved; recurring finding below | The [new directory-validation tests](../../lib/tests/l1/portable_path/relative_context.rs) correctly reject direct relative directories and relative derived destinations. They omit a relative directory supplied through a replacement launch scope and invalid settings removed during derivation. |
| Failed search strategies disappear from attempt history | Resolved | The shared verification path now returns a typed failure outcome; evaluation records the attempt before returning the error. [Public-error regressions](../../lib/tests/l1/portable_path/probe_failure.rs) cover both searched forms, earlier shadowed candidates, earlier preferences, and the distinct target-preparation failure. All target preferences use this recording path; authored intent retains lookup problems as findings. |
| Absolute fallback links omit the required consumer warning | Resolved | Darkmatter's [normalization tests](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs) exercise default warnings and explicit suppression for Markdown links/images and HTML hyperlinks, images, video, audio, sources, iframes, scripts, stylesheets, and fonts. Destination text is preserved. Evaluation-failure warnings remain separate. The suppression option participates in composition identity and the current docs describe it. |

## Unblocked Findings

### High — Context validation still admits relative launch directories and hides invalid request settings

**Defect class:** Captured-context validation omits an independently supplied directory and forgets invalid originating settings when deriving a document, allowing invalid snapshots to succeed through public readers and writers.

In `biscuit-file`, [FileResolutionContext::validate_absolute_anchors](../../lib/src/file_reference/context.rs:1318) checks the request and document directories, current anchors, launch repository/package anchors, and home. It never checks the launch scope's own request directory. The public [with_launch_magic_scope](../../lib/src/file_reference/context.rs:1099) builder can replace that scope independently, so checking the context's request directory does not validate the directory that `@` actually searches.

Separately, the shared [document derivation implementation](../../lib/src/file_reference/context.rs:791) clears the original explicit tree setting when entering an external tree and can replace package anchors from a catalog. [FileResolutionContext::validate](../../lib/src/file_reference/context.rs:1293) subsequently validates only the remaining settings and the originating selected tree's containment. It does not retain the original explicit-root conflict or all original absolute-directory failures. A request that fails validation can therefore produce a derived context that succeeds. This contradicts the specification's requirement to validate the originating request independently and the new rustdoc promise that derivation cannot make an invalid request valid.

This matters because captured contexts promise repeatable resolution without using the live process directory. A relative launch root reaches filesystem probes relative to that live directory. Losing an invalid original setting also makes validation depend on which document is visited first.

**Reproduction:** Use the existing portable-path `Fixture`, with a canonical temporary root, an absolute `repo/docs` document directory, and existing `repo/docs/x.md` and `other/x.md` files. Keep target and fixture unchanged while making one of these edits:

1. Capture a launch scope from a snapshot whose directory is `relative`, then attach it to the valid absolute document context using the replacement-scope builder.
2. Add an explicit tree root of `relative` to a valid repository context, then derive the already accepted external `other/x.md` document. The initial context reports a relative-directory error; the derived context validates and both writer constructors return the absolute target. An absolute explicit root of `repo/docs`, which conflicts with the repository root, disappears in the same way.
3. Set a relative package or package-area anchor after installing a valid repository catalog, attach an independently valid launch scope, then derive a document. Catalog reselection removes the invalid current anchor; ordinary as well as trusted derivations validate successfully.

For the first shape, create `first/relative/x.md`, leave a second directory empty, and change only the process directory between calls using the same captured context and `@x.md`. From `first`, resolution reports a match at `<fixture>/repo/docs/relative/x.md`, which does not exist: the relative candidate was probed from the process directory, then reported relative to the captured document directory. From the empty directory, resolution reports no match. Both calls should instead reject the context before probing. This was asserted through the public resolver in a nextest-isolated probe.

The following table includes every derivation wrapper and the independently replaceable launch anchors. “Reject” means a typed configuration error before strategy execution; successful controls have valid absolute directories. The package containing every context method in this table is `biscuit-file`.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| [Direct validation](../../lib/src/file_reference/context.rs:1293), valid fixture | Unedited absolute context | Accepted; both writers return the target | Same; clean control |
| [Replacement launch scope](../../lib/src/file_reference/context.rs:1099) | Relative launch request directory, valid absolute document/request directories | Accepted; magic root chain and candidate plan contain relative paths | Reject relative launch directory |
| Replacement launch scope | Relative launch repository root | Typed relative-directory error | Same; clean |
| Replacement launch scope | Relative launch package root | Typed relative-directory error | Same; clean |
| Replacement launch scope | Relative launch package-area root | Typed relative-directory error | Same; clean |
| [for_cwd](../../lib/src/file_reference/context.rs:767) | Original relative explicit root, then a valid in-tree directory | Original error retained | Same; clean |
| [for_source](../../lib/src/file_reference/context.rs:691) | Same invalid original root, in-tree file | Original error retained | Same; clean |
| [for_source_reference](../../lib/src/file_reference/context.rs:731) | Same invalid original root and accepted in-tree file | Original error retained | Same; clean |
| [for_trusted_external_cwd](../../lib/src/file_reference/context.rs:779) | Same invalid original root, external directory | Accepted after original explicit setting is cleared | Retain originating configuration error |
| [for_trusted_external_source](../../lib/src/file_reference/context.rs:713) | Same invalid original root, external file | Accepted; absolute writer succeeds | Retain originating configuration error |
| [for_trusted_external_source_reference](../../lib/src/file_reference/context.rs:750) | Same invalid original root and accepted external file | Accepted; absolute writer succeeds | Retain originating configuration error |
| All three trusted wrappers above | Same invalid original root, destination still in repository | Original error retained | Same; clean |
| All six wrappers above | Absolute explicit root differs from original repository root | Ordinary and trusted in-tree forms reject; all three trusted external forms accept | Original conflict remains an error for every derivation |
| All six wrappers above | Relative original request directory, repository root, or home; ordinary captured launch scope | All reject, including external derivations | Same; clean |
| All six wrappers above | Relative original package or package-area root; ordinary captured launch scope | All reject through retained launch anchor | Same; clean |
| Three trusted wrappers above, no catalog | Relative original package or package-area root, independently valid replacement launch scope | External forms accept after clearing the current anchor | Retain originating relative-directory error |
| All six wrappers above, catalog installed before invalid anchor | Relative original package root, independently valid replacement launch scope | Every wrapper accepts after catalog reselection | Retain originating relative-directory error |
| All six wrappers above, catalog installed before invalid anchor | Relative original package-area root, independently valid replacement launch scope | Every wrapper accepts after catalog reselection | Retain originating relative-directory error |
| All six wrappers above, catalog installed before invalid anchor | Relative original repository root, independently valid replacement launch scope | All reject through retained original tree containment | Rejected; clean, although error changes from relative-directory to containment |
| All six wrappers above | Absolute explicit non-repository root does not contain the originating request | All retain the originating containment error | Same; clean |
| All six wrappers above | Unedited valid request, accepted in-tree/external destinations | Valid ordinary in-tree and trusted external contexts succeed | Same; clean |
| [New destination directory validation](../../lib/tests/l1/portable_path/relative_context.rs) | Relative new directory, through all six wrappers | Typed relative-directory errors | Same; clean |
| [Relative values with separate contracts](../../lib/tests/l1/portable_path/relative_context.rs) | Relative environment values and configured magic/vault roots in valid contexts | Supported under their existing rules | Same; clean; these are not required absolute context directories |

Every accepted/rejected context in the sweep was checked through the same public-result projections:

| Site in `biscuit-file` | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| [PortablePath::from_path](../../lib/src/file_reference/portable/evaluate.rs:65), supplied context, absolute-only strategy | Each defective context above | Returns target, with a matched absolute attempt | Configuration error with no attempts |
| [PortablePath::from_reference](../../lib/src/file_reference/portable/evaluate.rs:73), supplied context, same strategy | Absolute reference to the same target, each defective context | Returns target | Configuration error with no attempts |
| [FileReference::resolve_in_context](../../lib/src/file_reference/mod.rs:710) | Existing absolute reference, each defective context | Match | Relative-directory/originating-configuration error |
| [FileReference::resolve_detailed](../../lib/src/file_reference/mod.rs:731) | Same input and context | Matched outcome | Failed outcome with retained typed configuration error |
| [FileReference::candidate_plan](../../lib/src/file_reference/mod.rs:787) | Same input and context | Candidate list | Configuration error |
| [FileReference::candidate_plan_with_order](../../lib/src/file_reference/mod.rs:812) | Absolute reference, both available ordering policies, relative launch and invalid original explicit-root contexts | Both ordered plans accepted | Configuration error before ordering |
| [FileReference::complete_partial_in_context](../../lib/src/file_reference/mod.rs:926) | Bare completion token `x`, each defective context | Successful completion plan | Configuration error |
| Context-aware completion | Magic token `@x`, relative launch and invalid original explicit-root contexts | Accepted; relative launch directory appears in completion roots | Configuration error |
| Context-aware recursive resolution | `%@x.md`, relative launch and invalid original explicit-root contexts | No match, with no configuration error | Configuration error before traversal |
| [Magic search roots](../../lib/src/file_reference/context.rs:1159) and candidate planning | Relative launch directory, `@x.md` | Relative roots/candidates returned | Context rejected by fallible planning; never probe relative launch candidates |
| Context-aware resolution, same snapshot | Relative launch directory, `@x.md`, file present only under one process directory | Match to nonexistent reported path from one directory; no match from the other | Same configuration error regardless of process directory |

The clean contexts exercised by the temporary sweep were also clean through both writers and every fallible projection in the second table. Ambient resolver entry points prepare their own scopes and do not accept these replacement snapshots. Darkmatter's [source_file_context](../../../darkmatter/lib/src/markdown/compose/context/options.rs:93) delegates to the four source derivation wrappers already swept; it needs no separate derivation algorithm or consumer-specific workaround.

**Required change:** Validate the launch scope's request directory alongside its other absolute anchors. Preserve validation of the entire originating context when deriving a document, before original settings are discarded or recomputed. Keep builders fallible at evaluation as designed; explicitly replacing a setting before derivation can correct it, but document derivation must not erase an originating error. Preserve valid trusted external behavior and the separate relative environment/magic-root contracts.

Add persistent public-result regressions for the complete instance table, especially all three trusted external wrappers with an invalid explicit root, all six catalog derivations with invalid current package anchors and an independently valid launch scope, and process-directory independence of launch resolution. Reuse a table-driven fixture instead of adding one isolated test for the first missed method. Review the context validation/derivation docs alongside the fix.

## Blocked Findings

None. The recurring finding can be implemented without a new design decision or human-only test.

## Recurrence

This repeats **“Relative context directories bypass configuration validation”** from `review-1.md`. Its required change called for central validation of captured directories and absolute tree anchors, including trusted derivations, before preferences execute.

That fix should have swept both request-directory stores, the replacement launch-scope builder, and original settings across all six derivation wrappers. It checked the context request directory but missed the launch request directory. It tested relative destinations and retained launch repository anchors, but missed relative explicit roots cleared by trusted external derivation and current package anchors overwritten by catalog reselection. This review enumerates those sites together with the clean controls and their public-reader/writer projections.

`recurrence` is `true`. A human should inspect sweep completeness before another automated cycle starts. This does not block implementing the listed fix and is not itself the reason for `ready: false`.

## Verification and requirement coverage

Executed on macOS against this working tree:

- `just test` in `biscuit-file`: 1,005 Level 1 tests passed; the recipe's six no-default-features path-text checks also passed.
- `just test link_normalization` in `darkmatter`: 21 selected Level 1 tests passed, including both eleven-form fallback-warning tables.
- `just test file_tree_roots` in `darkmatter`: five consumer tree-root integration tests passed.
- `just lint` in `biscuit-file` and `darkmatter`: passed, including Darkmatter's WASM extension compile check.
- `just check-tier-coverage biscuit-file` and `just check-tier-coverage darkmatter`: zero stranded tests.
- Temporary nextest public-API sweeps reproduced the context tables above; a final isolated process-directory probe asserted the exact match/no-match and nonexistent-reported-path behavior. The temporary module registration and files were removed.

| User-facing requirement | Strongest relevant verification present | Assessment |
| --- | --- | --- |
| Tree-root precedence and origin; explicit/repository conflicts; fallback behavior; home/environment opening anchors; ordinary/trusted derivation; unchanged launch scope | Level 1 filesystem integration tests in [file_tree](../../lib/tests/l1/file_tree.rs), [resolution_context](../../lib/tests/l1/resolution_context.rs), and Darkmatter's [file_tree_roots](../../../darkmatter/lib/tests/l1/file_tree_roots.rs) | Appropriate level; original-context validation and replacement launch-directory cases remain missing |
| Relative boundaries after interpolation; escaping bare fallback; symlink landing and missing descendants; reader opt-in; repository-only sigils; recursive roots; completion | Level 1 public resolver and filesystem integration tests in `file_tree` | Appropriate level; all context-aware projections inherit the finding above |
| Strategy ordering, route shapes, equal-to-CWD directories, filters, searched-root ordering, shadowing, missing/non-file targets, diagnostics | Level 1 public-result tests in [strategies](../../lib/tests/l1/portable_path/strategies.rs), [inputs](../../lib/tests/l1/portable_path/inputs.rs), and [probe_failure](../../lib/tests/l1/portable_path/probe_failure.rs) | Appropriate level; earlier failure-history gap is closed |
| Authored intent; URL/recursive preservation and refusal; minimal churn; idempotence; candidate equality; captured-state isolation | Level 1 public-input and corpus tests in `inputs` and [properties](../../lib/tests/l1/portable_path/properties.rs) | Appropriate level; captured-state isolation fails for the omitted launch-directory shape |
| Portable declarations, name/value eligibility, absent/relative/foreign values, deepest-prefix and name ties, deduplication | Level 1 public-result matrix in [environment](../../lib/tests/l1/portable_path/environment.rs) | Appropriate level |
| Root-parent normalization; lossless identity; non-Unicode names; literal backslashes/interpolation; Windows drives, UNC and verbatim spelling | Level 1 platform/public-result tests plus identity/text unit tests, including Windows-gated cases | Appropriate level; reported root-parent mismatch is closed |
| Darkmatter destination rewriting, suffix retention, child-document contexts, declared variables, default and suppressed fallback warnings | Level 1 library and CLI integration tests; this review executed normalization and tree-root selections | Appropriate level for document text/report behavior; fallback-warning gap is closed |

The new portable-path test files are declared from the existing consolidated `l1` target; their module names do not exclude them from Level 1. The area test metadata enables the separate fetch target. The normalization tables compile in Darkmatter's library test target. No requirement introduced by this feature depends on terminal rendering, keyboard encoding, focus, scrolling, or OS keyboard injection, so Level 2/3 evidence is not required for these contracts. Cross-OS execution evidence is left to CI and does not affect this readiness decision.

The file-format input robustness matrix is not applicable: this feature does not add or change a serialized configuration, manifest, or lockfile reader. Its environment-string matrix is already exercised through public outcomes.

The implementation log's documented departures remain understood: richer rejected-candidate records, additional typed errors, filter-root spelling, and Darkmatter finalization receiving already resolved absolute destinations. Maintaining authored link intent across the entire compose pipeline would require additional provenance; it is not a new finding in this iteration. No separate performance or ergonomics change was found that warrants blocking readiness.
