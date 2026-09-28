---
$schema: feature-review.yaml
ready: true
findings:
    - priority: high
      title: Small repositories pay a measured parallel-walk startup cost
      resolution: author chose a four-worker cap (option 2); implemented, measured, and verified
closed_on: 2026-09-26
disposition: resolved
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-26T12:08:07-07:00
spec: 2026-09-20-repo-perf/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/fixes/2026-09-20-repo-perf/implementation-log.md
description: "A **fix** review of `2026-09-20-repo-perf/spec.md`"
fix: 2026-09-20-repo-perf/review-2.md
previous: 2026-09-20-repo-perf/review-1.md
---

# Review 2 — Repository discovery performance

## Closure decision — 2026-09-26

**Closed as production ready by the author's decision on the blocked
finding.** The author chose option 2. They judged the 8-file probe tree
unrepresentatively small and a few milliseconds an acceptable price for the
large-repository gain, and asked for a smaller fixed worker count. The
fallback walk now uses `min(available_parallelism, 4)` workers
(`MAX_NESTED_WALK_WORKERS` in the Sniff library's
[nested-marker walk](../../lib/src/filesystem/repo/nested.rs)), rather than
`ignore`'s default cap of 12.

- **Measured** on the same host, with only the cap differing between the
  sides ([`evidence/worker-cap/`](evidence/worker-cap/README.md)):
  - checkout `detect_repo_structure`: 34.3 → 38.8 ms, still about 2× faster
    than the serial walk's 79.4 ms;
  - tiny tree: about 3.2 → 2.0 ms;
  - sequential tiny-tree throughput: +55%;
  - 14-way concurrent p95: about −20%; peak RSS: about −22%, at or below the
    serial baseline.
- **Work counters** are unchanged.
- **Verified:**
  - `just test` in `sniff/`: 2,875 passed, 32 skipped;
  - `just lint` and `cargo clippy --all-targets -D warnings` (`sniff`, with
    and without `remote`, and `sniff-cli`): clean;
  - `just check-tier-coverage sniff`: 0 stranded;
  - nested tests on the other operating systems: Linux 23/23, native Windows
    21/21, WSL2 23/23
    ([`cross-check-worker-cap.txt`](evidence/cross-os/cross-check-worker-cap.txt)).
- **Documentation observations** from this review are fixed: the fixture
  builder comment now says `vendor` is pruned, and `results.md` cites the
  full-output test for AC5.

The remaining small-tree cost is about +1.5 ms against the serial walk. The
author accepted it. The review text below is kept as written.

## Original verdict

**Not production ready.** The missing complete-output fixture test is now implemented and passes. The measured performance regression on small repositories remains unchanged. This verdict rests on that regression, not on missing cross-platform evidence or the mere need for a later human review.

## Prior review follow-through

The [first review](review-1.md) used a combined Findings section rather than separate blocked and unblocked sections. Its explicit blocking labels establish the following status:

| Prior finding | Result of this review |
|---|---|
| Medium — Controlled fixture lacks a complete before-and-after repository comparison | Resolved. The new Level 1 test compares complete serialized public repository results from serial and parallel fallback scans on the same disposable fixture. |
| High — Small repositories pay a measured parallel-walk startup cost | Still blocked. The specification, implementation log, and results contain no author choice that unblocked it before the last implementation. The log explicitly records deferral, not a fix. |

In the Sniff library, [the new full-result regression test](../../lib/src/filesystem/repo/nested.rs#L1852) exercises public structure detection twice. Separate counters prove the baseline used the serial fallback and the changed request used the production parallel fallback. It compares complete JSON and independently expects seven packages and four workspace layers. The serial reference matches the pre-change walker for this ordinary directory fixture; its documented root-entry exception only affects a symlinked starting root. The test-only selection is confined to the executing thread and reset on unwind. No production behavior changed in this review implementation.

## Unblocked Findings

None. The prior actionable testing gap is closed, and this review found no additional production defect or missing required verification level.

## Blocked Findings

### High — Small repositories pay a measured parallel-walk startup cost

The Sniff library's [fallback nested-marker scan](../../lib/src/filesystem/repo/nested.rs#L253), which discovers nested workspace manifests when no captured marker list is supplied, still starts a fresh parallel walk for every request. The [recorded comparisons](evidence/after/README.md) show release-mode public discovery rising from roughly 0.49 ms to 3.36 ms on an eight-file tree. Sequential throughput fell about 84%. On this monorepo, single-request discovery improved from roughly 79 ms to 36 ms, while fourteen concurrent large requests showed about 42% higher 95th-percentile latency and 14–31% more peak memory.

These are matched, warm-cache measurements from one busy Mac; they establish a local trade-off, not universal timing guarantees. The last implementation added a test only, so it did not remove the measured regression. The specification requires reconsidering the chosen worker policy when small-tree or concurrent-request costs regress materially.

**Blocked by the author's unresolved performance-policy choice.** Record acceptance of the existing trade-off, or select a changed strategy and repeat the affected correctness and performance checks. The existing recommendation is not an author decision. Level 1 results and counter tests establish correctness; measurements establish latency and resource costs. Neither decides what cost callers should accept.

## Requirement and verification level

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Marker names, root exclusion, grouping and ordering, sibling/deep trees, one/default/multiple workers | Level 1 candidate comparisons with independent expectations and twenty wide-tree repetitions | Appropriate and passing. |
| Ignore rules, hidden and pruned directories, marker-named directories, missing roots, symlinks, case rules, and permission errors | Level 1 filesystem fixtures; controlled child process for Git ignore configuration; platform-specific cases | Appropriate. Local applicable cases passed; other OS evidence is recorded separately. |
| One logical fallback walk, supplied-marker reuse including empty evidence, and worker counter propagation | Level 1 exact counter assertions | Appropriate and passing. |
| Complete public output on a controlled nested-workspace tree | New Level 1 serial/parallel full-JSON comparison with package and layer expectations | Prior gap closed. |
| Complete public output on this checkout | Recorded before/after full-JSON comparison in [checkout evidence](evidence/detection-output/README.md) | Manual corpus evidence; intentionally not a changing-checkout CI assertion. |
| Request costs and reuse of captured filesystem observations | Existing Level 1 detection observation tests | Appropriate; focused results below. |
| Preserve Darkmatter's observation timing | Existing Level 1 request/child observation tests, with passing results recorded in the implementation log | No observation ownership or timing change found; not rerun here. |
| Large-tree gain, tiny-tree cost, concurrent throughput and resource use | Recorded matched timing runs and work-counter comparisons | Regression remains the blocked finding above; no timing gate added. |
| Terminal rendering or keyboard, paste, mouse, and scrolling behavior | No changed requirement | Level 2 and Level 3 are not required for this filesystem-only fix. |

## Checks and limits

- `just test nested::tests`: **23 passed**, including the new public-output regression.
- `just test filesystem::repo::detection::tests`: **26 passed**.
- `just test filesystem::repo::detection::observation_index`: **24 passed**, including fallback costs, supplied observation reuse, and structure-only enrichment limits.
- `just check-tier-coverage sniff`: **zero stranded tests**.

The new test is compiled in the Sniff library's unit-test target, despite `autotests = false` for external integration files. It has no tier prefix or optional-feature gate. The package-area test recipe enables `remote`, matching the library's declared CI test features. An initial mistyped observation-test filter selected zero tests and is not counted as evidence.

This review did not repeat full-suite lint, the full Sniff suite, other-OS runs, Darkmatter's full suite, or the timing campaign. The latest implementation log reports 2,875 Sniff tests passing, 32 skipped, and clean lint. Earlier Darkmatter evidence records the three relevant observation tests passing alongside two unrelated prompt-file failures; that is not a clean full-suite result.

## Documentation observations

The Sniff library's [controlled fixture builder](../../lib/src/filesystem/repo/nested.rs#L1771) says its nested Cargo workspace under `vendor/inner` is suppressed by the same-standard nesting rule. In fact, the walker [prunes `vendor`](../../lib/src/filesystem/file_types/classify.rs#L224) before that rule can see the workspace. Correct the comment to describe pruning; this does not invalidate the complete-output comparison or require changing production behavior. The results document also still cites the older partial fixture test for complete-output coverage; the new test above is now the evidence. These are nonblocking documentation corrections, not additional missing functionality.

The requested previous-review location under `prompts/_reviews/sniff/fixes/` does not exist. The actual previous review is alongside this file; its `next` and `implemented` metadata were updated there. The specification's review iteration is now 2; it is not marked completed.
