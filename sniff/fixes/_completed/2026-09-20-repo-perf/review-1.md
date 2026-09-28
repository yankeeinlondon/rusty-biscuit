---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Small repositories pay a measured parallel-walk startup cost
    - priority: medium
      title: Controlled fixture lacks a complete before-and-after repository comparison
human_review: true
human_review_items:
    - |-
        Decide whether the faster scan on large repositories justifies slower scans on small repositories. The current implementation reduces this monorepo's repository-discovery time from about 79 ms to 36 ms, but raises an eight-file repository from about 0.5 ms to 3.4 ms per call. Among fourteen concurrent large scans, the slowest 5% of calls also took about 42% longer, and the process used more memory on the measured Mac. These are measured effects of starting threads for every scan.

        Please choose one direction:

        1. Accept the current 12-thread setting and its measured costs. The existing implementation and verification remain applicable.
        2. Use a smaller fixed thread count. This reduces startup cost but gives up some large-repository speed; the code and performance evidence must be updated.
        3. Start with a serial scan and switch to parallel work for a large tree. This avoids the small-tree cost but needs a new design, tests, and measurements.
        4. Restore the serial scan. This removes the regression but gives up the measured large-repository improvement.

        The review recommends option 1 if a roughly 3 ms cost per small request is acceptable for Sniff's callers. This decision is required by the specification's performance escalation rule and is the blocker for the first finding. An agent can address the separate fixture-test finding without this decision.
has_blocked_findings: true
blocked: false
reviewed_by: codex/default
created: 2026-09-25T19:08:56-07:00
spec: 2026-09-20-repo-perf/spec.md
implemented: true
next: 2026-09-20-repo-perf/review-2.md
implemented_by: claude/default
log: sniff/fixes/2026-09-20-repo-perf/implementation-log.md
description: "A **fix** review of `2026-09-20-repo-perf/spec.md`"
fix: 2026-09-20-repo-perf/review-1.md
---

# Review 1 — Repository discovery performance

## Verdict

**Not ready for production.** The parallel scan preserves candidate ordering and the tested ignore, symlink, and work-count behavior. It delivers a clear gain on this monorepo. Two matters remain: the measured cost on small repositories needs the author's explicit choice, and the required complete-output comparison on a controlled fixture has not been done.

The change affects Sniff's library discovery path. It adds no terminal output or interactive behavior, so in-process Level 1 tests are the appropriate verification level. No Level 2 terminal or Level 3 keyboard test is needed for this fix. Cross-operating-system evidence and a later human review are separate from the technical readiness assessment here.

## Findings

### High — Small repositories pay a measured parallel-walk startup cost

**Blocked by the author's performance-policy choice.** The Sniff library's [`walk_for_nested_markers`](../../lib/src/filesystem/repo/nested.rs#L252), which finds nested workspace markers when no request-scoped marker list is supplied, starts `ignore`'s default parallel workers on every call. The [before-and-after measurements](evidence/after/README.md) show public `detect_repo_structure` taking about 0.49 → 3.36 ms on an eight-file tree in release mode. A single caller repeating that request handles about 84% fewer requests per second. On this monorepo, the same change improves public discovery from about 79 → 36 ms, while 14 concurrent large scans show about 42% higher 95th-percentile latency and up to 31% more peak memory. These numbers come from one heavily loaded Mac with a warm filesystem cache; the magnitude on other hosts is unknown.

The specification says to bring a material small-tree or concurrent regression back for a decision. Its existing `human_review_items` presents four options. The author should choose one of those options before this performance policy is treated as accepted. If the choice changes the worker strategy, repeat the parity tests and matched measurements on that code. This is a readiness concern because the regression affects every structure-only caller on a small tree, rather than merely being an outstanding request for human approval.

Strongest verification present: Level 1 library tests for results and counters, plus repeatable local timing measurements for the performance trade-off. Level 1 is the correct level for this filesystem behavior; the tests cannot decide whether the measured trade-off is acceptable.

### Medium — Controlled fixture lacks a complete before-and-after repository comparison

The specification asks for complete detection and topology output before and after the change on both a controlled nested-workspace fixture and this checkout. The checkout comparison exists and its serialized [`RepoInfo`](evidence/detection-output/README.md) is byte-identical. For the controlled fixture, the cited [`supplied_evidence_starts_no_fallback_walk`](../../lib/src/filesystem/repo/nested.rs#L1602) test compares selected detector outcomes and seed counts between supplied evidence and the new fallback. The candidate tests compare ordered roots and standards. Neither runs the old and new implementation through public repository detection and compares the complete result on the same disposable fixture.

Add a Level 1 regression using one fixed nested-workspace fixture and both baseline and changed implementations, comparing the full public repository result while confirming each request enters the fallback. An equivalent recorded baseline fixture output is acceptable if it was captured from the old implementation. Keep the candidate-level tests: they isolate walk semantics, while the full-output check catches changes later in repository projection. The existing checkout comparison does not replace a controlled fixture because its source tree changes over time.

Strongest verification present: Level 1 candidate and selected-outcome tests on disposable fixtures, plus a manual complete-output comparison on this checkout. Level 1 is the correct verification level, but the controlled complete-output assertion is missing.

## Verification

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| Marker admission, ignore rules, pruning, symlinks, platform case rules, and deterministic candidate order | Level 1 library tests with independent expected candidates, including 20 wide-tree repeats | Appropriate level and covered. |
| One logical fallback walk, supplied-evidence reuse, and worker work-count propagation | Level 1 counter tests, including missing and empty roots | Appropriate level and covered. |
| Complete public repository output on this checkout | Baseline/changed JSON comparison, both entering the fallback | Present as manual corpus evidence. |
| Complete public repository output on a controlled nested-workspace fixture | Level 1 partial outcome and candidate assertions | **Gap:** complete before-and-after output is missing. |
| Large-tree gain, small-tree cost, and concurrent resource use | Matched local measurements on one Mac | Trade-off established locally; author choice remains open. |
| Terminal display, hotkeys, paste, mouse, or other interactive behavior | No changed requirement | Level 2 and Level 3 do not apply. |

I ran `just test nested::tests` in `sniff/`: 22 selected tests passed. `just check-tier-coverage sniff` reported zero stranded tests. The new tests are in the Sniff library's compiled unit-test target, have no tier prefix that removes them from Level 1, and do not depend on a disabled feature. The implementation report also records the broader Sniff suite and lint as passing and focused runs on macOS, Linux, native Windows, and WSL2; this review did not repeat those broader runs.

The implementation report documents one intentional deviation: a marker-named symlink used as the starting root no longer registers a candidate outside the requested tree. That is a sensible boundary correction and has a direct Level 1 regression test. It should be carried into the final spec wording if the author accepts the fix, since the outcome currently says no output change is intended.
