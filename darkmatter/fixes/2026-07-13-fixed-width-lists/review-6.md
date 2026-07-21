---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-20T22:57:29-07:00
spec: 2026-07-13-fixed-width-lists/spec.md
log: darkmatter/fixes/2026-07-13-fixed-width-lists/log.md
implemented: true
description: "A **fix** review of `2026-07-13-fixed-width-lists/spec.md`"
fix: 2026-07-13-fixed-width-lists/review-6.md
previous: 2026-07-13-fixed-width-lists/review-5.md
---

# Review 6 — Fixed-Width Lists

## Verdict

This fix is **ready for production**. Opaque shell-body ownership now comes from Darkmatter's shared
block-pair scanner, including mixed page/shell nesting, keyword boundaries, quoted ownership, code
region exclusions, and a source-preserving fallback for malformed structures. Exact Level-1 tests
cover the defect and all affected cleanup surfaces. The Criterion B1/B2/B3 timing vector remains
deferred and is explicitly non-blocking for completion; this review makes no performance claim from
the absent measurements.

## Findings

### Resolved — Opaque shell-body ownership diverges from the shared block scanner

`protect_opaque_directive_bodies` implements a second directive recognizer
(`cleanup/lists.rs:348-410`) instead of consuming the shared block-pair model. It increments depth
only for `::shell-block` and decrements it for every `::end-block` (`lists.rs:360-385`). The
authoritative scanner uses one stack for both `::block` and `::shell-block`, enforces keyword
boundaries and closer validity, distinguishes quoted ownership, and excludes code regions
(`compose/block_pairs.rs:97-160`).

This is a production defect, not only missing coverage. A valid mixed-stack input is:

```markdown
::shell-block
- first literal
::block condition
+ second literal
::end-block
- third source line
  continuation remains literal
::end-block

+ Actual item.
```

The first `::end-block` closes the nested page block in the shared scanner. The cleanup scanner
instead treats it as the shell closer, stops masking, and `md clean -` rewrites the remaining shell
payload to:

```markdown
::end-block

- third source line continuation remains literal
    ::end-block
```

Fixed-width cleanup also reflows that payload. The same duplicated recognizer treats
`::shell-blocker` as a real opener and accepts `::end-block trailing text` as a closer; a list under
the former is incorrectly exempted from incidental-newline cleanup. These cases directly
contradict Design Decision 1's shared-classification requirement and violate AC 5, 9, and 10 plus
the Goals 5 and 8 preservation contracts. Because `cleanup_content_internal` uses the mask before
the common event serialization path (`cleanup/mod.rs:308-417`), the defect reaches direct library
cleanup, compose Cleanup, CLI stdout/`--save`, and DMLS formatting.

The new Darkmatter-aware oracle is capable of catching this: it extracts shell payloads with
`scan_darkmatter_blocks` (`cleanup/tests/reflow.rs:125-139`). However, its fixtures and the compose,
CLI, and DMLS mirrors cover only a flat block and a quoted flat block
(`cleanup/tests/reflow.rs:1521-1594`, `cli/tests/clean.rs:277-370`,
`dmls/src/providers/formatting.rs:228-290`). No test crosses a page-block closer while a shell block
remains open or pins scanner-negative lookalikes.

**Suggested resolution:** derive opaque ownership from the shared block-pair authority, including
its mixed opener stack, token boundaries, quote rules, and code-region exclusions. Define an
explicit source-preserving fallback for malformed or unterminated blocks because cleanup returns a
`String`, not a `Result`. Add exact Level-1 tests for nested page blocks inside shell blocks,
`::shell-blocker`, trailing-content closers, quoted-ownership mismatches, fixed width, second-pass
idempotence, compose, CLI stdout/`--save`, and DMLS.

**Resolution:** `OpaqueBodyScan` now derives shell payload spans from `scan_block_pairs`; cleanup and
fixed-width reflow consume those spans instead of maintaining a second directive recognizer.
Malformed, unmatched, trailing-content, and unterminated block structures preserve the complete
source. Exact library coverage pins mixed nesting, scanner-negative lookalikes, quoted mismatches,
malformed fallbacks, fixed-width behavior, and idempotence. Compose, spawned CLI stdout/`--save`,
and DMLS fixtures pin the mixed-stack behavior across every affected surface.

### Deferred (non-blocking) — Required performance timing evidence remains deferred

The deterministic parse-count half of AC 15 passes, and the Criterion harness exists. There is
still no admissible baseline → candidate → baseline bracket, no per-case Criterion medians, no 3%
baseline-drift verdict, and no B1, B2, or B3 result. The implementation log and
`deferred-performance-tests.md` explicitly retain this as deferred work.

The host remains unsuitable for a 10% budget. During this review `sniff` identified the 16-core
Apple M4 Max host, while `uptime` reported eight users and load averages of
`13.97 19.85 23.52`; nine Codex and five Claudine processes were active. The one-minute load was
nearly seven times the documented ceiling of 2.0, so no timing sample was taken. A harness smoke
run or parse-count test cannot substitute for the specification's median comparisons.

**Follow-up:** on a quiet admissible host, run the documented fresh-target baseline → candidate →
baseline bracket. Record all eight baseline drift checks, all four B1 cases, all three B2 cases,
and all four B3 cases independently. The timing portion of AC 15 remains unverified, but is
explicitly non-blocking for this review's completion.

## Requirement-to-Verification Assessment

Fixed-width cleanup is a deterministic Markdown source transformation, so Level 1 is the correct
behavioral tier for AC 1–15. No requirement depends on a terminal emulator's rendering or input
encoder, and Level 3 is not applicable. AC 16 independently mandates the package-area Level-2
gate.

| AC | Requirement | Strongest verification | Assessment |
| --- | --- | --- | --- |
| 1 | Collapse eligible list-prose soft breaks at all nesting and quote depths | Level 1 exact library, compose, spawned-CLI, and DMLS tests | Pass for the represented list-prose matrix. |
| 2 | Remove continuation layout and retain only the Unicode join separator | Level 1 exact ASCII and Unicode tests | Pass. |
| 3 | Strip mode emits no synthesized hanging whitespace | Level 1 exact output and fixed-point tests | Pass. |
| 4 | Preserve mode performs no list collapse or fixed-width synthesis | Level 1 library, CLI, compose, and second-pass tests | Pass. |
| 5 | Fixed width unwraps logical prose while protecting child blocks | Level 1 exact output and fingerprints | Pass: shared scanner spans protect the complete mixed-stack shell payload. |
| 6 | Created lines carry complete aligned container prefixes | Level 1 exact nested/quote/task output and width checks | Pass. |
| 7 | Per-item digit/task/configured-indent/quote prefixes | Level 1 exact indent and prefix matrix with structural fingerprints | Pass. |
| 8 | Total display width respects only documented overflow exceptions | Level 1 display-width and indivisible-overflow assertions | Pass for reflowable prose. |
| 9 | Paragraph, item, child-block, and protected-block structure remains intact | Level 1 exact output and structural fingerprints | Pass: mixed page/shell nesting retains both closers and payload ownership. |
| 10 | Structural fingerprints preserve list and protected ownership | Level 1 pulldown-cmark and Darkmatter-aware payload oracles | Pass: mixed nesting, lookalikes, quoted mismatches, and malformed fallbacks are covered. |
| 11 | Normal, compact, and loose spacing retain compatibility | Level 1 exact mode matrix | Pass. |
| 12 | Equivalent library, compose, CLI stdout/save, and DMLS sequences agree | Level 1 cross-surface parity tests | Pass: all four surfaces cover the mixed-stack shell payload and fixed-width output. |
| 13 | Default, preserve, and fixed-width cleanup are idempotent | Level 1 second-pass tests | Pass for the represented outputs. |
| 14 | Public API, CLI schema, dependencies, and platform behavior remain stable | Source inspection plus Level 1 API/CLI tests | Pass. |
| 15 | Parse and timing budgets pass | Level 1 parse counters; Criterion timing absent | Deferred, non-blocking: parse counts pass; B1/B2/B3 remain unmeasured and no timing verdict is claimed. |
| 16 | Build/L1/L2/lint and bounded impact gates pass | Recorded exhaustive gates, fresh focused L1, GitNexus impact analysis | Pass: the implementation record covers build, exhaustive L1, lint, and a gap-free 19/69/3 Level-2 result for the current tree. |

## Verification Performed

- Read the complete specification, Review 5, implementation log, deferred-performance record,
  current cleanup implementation, shared block-pair scanner, and cross-surface tests.
- `sniff` confirms the affected scope as `darkmatter`, `darkmatter-cli`, and `dmls` in the
  `darkmatter` package area.
- GitNexus rates `cleanup_content_internal` **CRITICAL** with 171 upstream symbols,
  `reflow_to_width` **CRITICAL** with 65, `protect_opaque_directive_bodies` **CRITICAL** with 155,
  and `restore_opaque_directive_bodies` **CRITICAL** with 154. The common compose inline-post flow
  is affected.
- Fresh focused Level 1 passed 5/5 on the resolved tree: shared-classification and fallback library
  coverage plus library, compose, spawned-CLI, and DMLS mixed-stack fixtures.
- Package-area `just build` and `just lint` passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
  The full Level-1 run passed 5,785 tests before a pre-existing `slow_` compose fixture exhausted
  its retry timeout under host contention and canceled the remaining 119; that fixture and the
  related retried fixture then passed alone 2/2. The implementation record's earlier exhaustive
  Level-1 result covers the unaffected remainder.
- The implementation record reports package-area build, exhaustive Level 1, and lint passing, plus
  a gap-free current-tree Level-2 result of `darkmatter` 19/19, `darkmatter-cli` 69/69, and `dmls`
  3/3.
- No Criterion timing was run because the host failed the documented admissibility conditions.

This review ran on macOS. The implementation is pure Rust with no new platform branch; Windows and
Linux were not executed during this review.

## Production Readiness

**Ready.** Opaque shell-body ownership is unified with the shared Darkmatter block scanner and the
missing exact cross-surface regressions are present. The Criterion timing vector remains documented
as deferred follow-up and does not block completion.
