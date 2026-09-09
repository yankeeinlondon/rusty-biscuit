---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-09T11:53:45-07:00
spec: 2026-09-07-faster-darkmatter-tests/spec.md
implemented: false
description: A **fix** review of `2026-09-07-faster-darkmatter-tests/spec.md`
fix: 2026-09-07-faster-darkmatter-tests/review-2.md
previous: 2026-09-07-faster-darkmatter-tests/review-1.md
---

# Review 2: Faster Darkmatter Tests

## Verdict

Not ready for production.

The implementation closes the prior review's CLI isolation-guard and DMLS
worker-ownership defects. The current Level-1 suite and lint gate pass, and the
Level-3 synchronization changes now have a clear rationale. Production
readiness remains blocked by the absence of the specification's discovery and
composition work-count evidence and by the fact that the four changed
Level-3 behaviors have never executed. Hosted budget/candidate comparison also
remains incomplete, but this review honors the recorded operator decision that
the hosted evidence is deferred rather than a readiness blocker.

## Findings

### High — Discovery and composition work reduction is still not measured

The specification requires compatible work counters for discovery,
composition, effects, and HTTP requests, and forbids extrapolating a speedup
from an unprofiled substage (`spec.md:160-166`). The implementation's own audit
ledger still marks the discovery and composition signals as `pending`
(`log.md:36-43`). Its replacement evidence establishes containment and shows
that hostile ambient inputs do not reach deterministic child processes, but it
does not count how often repository discovery or composition occurs in matched
baseline and candidate workloads.

That distinction matters because the full local cohorts did not become faster:
the candidate medians are reported as 9–10% higher, while only the selected
`darkmatter-cli` and loopback HTTP subcohorts improved (`results.md:146-162`).
Those focused improvements are useful, but they do not prove the spec's broader
claim that repeated discovery and composition work was reduced.

Add stable discovery and composition counters at test-only observation points,
run them over matched baseline/candidate identities, and record the deltas in
`results.md`. If either activity cannot be counted without changing production
behavior, narrow the outcome claim and document why the existing containment
or timing evidence is the strongest valid substitute.

### High — The changed Level-3 behavior still has no Level-3 execution evidence

The synchronization and skip semantics changed for three OS-input popover tests
and one real-terminal image-paint test. The added explanation is clear and
appropriately identifies the latency tradeoff, and the hard-fail behavior for a
required but unavailable screen capture closes a genuine false-green path.
However, all four user-observable claims remain explicitly unexecuted
(`results.md:334-360`). Compile/list reachability is not behavioral evidence.

The popover claims require Level 3 because they depend on OS keyboard/pointer
events passing through Chrome's real input path. The painted-image claim also
requires its Level-3 macOS screen-capture boundary because its assertion is
about pixels produced by a real terminal emulator. Under the review's rigor
rule, these claims cannot be treated as preserved until an attended run passes
with no runtime skips.

Run the documented strict invocation on a suitable attended macOS host:

```sh
cd darkmatter
BISCUIT_L3_TAKE_FOCUS=1 BISCUIT_TEST_LEVEL_REQUIRED=3 just test-l3
```

Retain stderr and the four per-test outcomes so an unavailable harness cannot
be mistaken for a pass.

### Medium — Required hosted performance acceptance remains deferred

The specification requires three consecutive candidate runs per configured CI
leg, ratified family budgets, and matched-identity comparisons
(`spec.md:168-174`, `spec.md:194-197`). The results still contain one baseline
sample per leg, no candidate samples, and no ratified budget
(`results.md:105-121`, `results.md:164-179`). Therefore AC7 and final CI
performance verification remain incomplete.

The previous review now records an operator decision that this evidence should
not block production readiness. This review respects that decision and does not
use this finding to determine `ready`; the evidence remains a specification gap
and should be completed through the sequence in
`deferred-performance-measurement.md` before the fix is archived as fully
verified.

## Prior Review Closure

| Prior finding | Review 2 assessment |
|---|---|
| CI comparison and budgets | Explicitly deferred with an owner and closing sequence; still incomplete, but operator-ruled non-blocking. |
| Level-3 synchronization and execution | Rationale and strict skip behavior fixed; execution half remains open as the High finding above. |
| DMLS server-thread cleanup | Fixed. All three in-memory suites share `LspFixture`; teardown is bounded, non-panicking during unwind, and covered by a failure-path ordering test. |
| Candidate isolation | Fixed. The owned commit range and path-level scope are documented separately from sibling fixes. |
| Protected environment overrides | Fixed. Builder declarations and the structural guard share one protected-key classification, with negative tests for each namespace. |

## Requirement Verification and Test Rigor

| Requirement family | Strongest verification present | Assessment |
|---|---:|---|
| Complete inventory and runner reconciliation | Level 1/static source and Nextest reconciliation across declared routes | Appropriate; the implementation reports every feature-unioned identity assigned once. |
| Deterministic `md` and `zed-dmls` isolation | Level 1 real subprocesses under hostile CWD/home/cache/Git/PATH/rendering/application inputs | Appropriate; focused fixture/guard tests passed 44/44. |
| Passive schema/LSP behavior and effect absence | Level 1 in-process protocol assertions with effect/network sentinels | Appropriate. |
| HTTP/process ownership | Level 1 loopback/subprocess failure-path tests | Appropriate. |
| In-memory DMLS worker cleanup | Level 1 unwind-order regression plus the shared bounded fixture | Appropriate; the three affected test binaries passed 109/109. |
| Terminal glyph/style/layout behavior retained by this fix | Level 2 results recorded from real terminal backends | Appropriate for rendering; no terminal encoder claim is involved. |
| Browser computed layout/style | Browser tier results recorded from headless Chrome | Appropriate. |
| OS Tab/Enter/pointer popover behavior | Compile/list only | Wrong level. Requires Level 3; High gap. |
| Real terminal image painting | Compile/list only | Wrong level for the changed screen-capture assertion; High gap. |
| Reduced discovery/composition work | Local timing and isolation proxies; no work counters | Insufficient for the explicit performance-evidence requirement. |
| Cross-platform performance acceptance | One hosted baseline per leg; no candidate comparison or budgets | Incomplete and operator-deferred. |

## Validation Performed

```text
cd darkmatter && just test
  7,755 passed, 6 skipped

cd darkmatter && just lint
  exit 0 for darkmatter, darkmatter-cli, dmls, zed-dmls-cli,
  including the wasm32-wasip2 Zed extension check

cargo nextest run -p darkmatter-cli \
  --test md_process_fixture --test spawn_site_guard
  44 passed, 0 skipped

cargo nextest run -p dmls --features effects-instrumentation \
  --test lsp_session --test no_side_effects \
  --test suggest_constraint_phase1
  109 passed, 0 skipped

git diff --check
  no whitespace diagnostics
```

No Level-2, browser, or Level-3 runtime was started during this review. The
implementation records prior Level-2/browser passes at the unchanged relevant
source state. Level 3 was intentionally not invoked from this non-interactive
session because it would require foreground OS input and focus changes.
