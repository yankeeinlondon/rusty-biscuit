---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-15T00:45:01-07:00
spec: 2026-07-12-perf/spec.md
implemented: false
description: "A **feature** review of `2026-07-12-perf/spec.md`"
feature: 2026-07-12-perf/review-1.md
previous: "/"
---

# Review 1 — Performance

## Verdict

Not ready for production. The implementation contains valuable measured wins—the always-on
Darkmatter NTP probe is removed, TOC construction is no longer quadratic, schema work is reduced,
and the current Darkmatter library Level 2 suite renders successfully in real terminals. However,
two public API compatibility violations directly contradict the specification, and the terminal
performance and benchmark closeout evidence does not meet the specification's required rigor.

The production blockers are the public `EffectiveSchema::json_schema` type change, the changed
semantics of `sniff::detect_timezone()`, and the absence of a Level 2 OSC-query measurement. The
performance results also cannot serve as the specified before/after regression evidence because
the fixtures changed and neither their generator nor their content hashes are checked in.

## Findings

### High — `EffectiveSchema::json_schema` breaks the public Rust API

The specification requires public Rust APIs to remain source-compatible. Before Finding 29,
`EffectiveSchema::json_schema` was a public `serde_json::Value`; it is now a public
`Arc<serde_json::Value>` in `darkmatter/lib/src/markdown/schemas/mod.rs:591-601`. Deref coercion
helps borrowed read sites, but it does not preserve source compatibility for consumers that move
the field, destructure the public struct, require `Value`, or expose the field through their own
API. Workspace compilation only proves that current in-repository callers do not exercise those
valid public use cases.

Keep the public field as `Value` or replace it through a separately reviewed compatibility design.
The allocation win can remain internal by sharing the baseline/merge layers and materializing the
owned public facade only at the API boundary. Add a compile-time downstream compatibility fixture
covering a moved `Value` and public-struct destructuring.

### High — The implementation changes `sniff::detect_timezone()` despite an explicit non-goal

Finding 1 says Darkmatter must call `detect_timezone_with_options(false)` while the bare
`sniff::os::detect_timezone()` API remains the full NTP-reporting convenience surface. Darkmatter
does make the correct explicit call, but `sniff/lib/src/os/time.rs:483-509` also changes the bare
API from `detect_timezone_with_options(true)` to `false`. Existing callers now receive
`NtpStatus::Unknown` without opting into that semantic change.

Revert the bare `sniff` API to its prior behavior and keep only Darkmatter's explicit local-only
call. If the `sniff` default should change, do it under a separate specification with a caller audit
and compatibility decision, exactly as this specification requires.

### High — Terminal caching has no requirement-matched Level 2 verification

Findings 2, 3, and 21 concern latency and probe behavior in an actual terminal. The specification
therefore requires a real PTY/Level 2 measurement. The new tests
`text_color_is_stable_across_calls`, `bg_color_is_stable_across_calls`, and
`color_mode_is_stable_across_calls` are Level 1 and only compare returned values. They cannot prove
that a terminal emitted one OSC query rather than several, or that repeated `Terminal`
construction avoids tty round trips. The existing Level 2 frame tests verify rendered content but
do not count OSC traffic or measure the repeated-construction path.

Add an L2 test or checked-in L2 benchmark that runs inside a real supported terminal, instruments
the OSC request path, constructs multiple terminals/renders, and proves one default OSC 10 query
with no repeated timeout/round-trip cost. A corresponding CLI case should cover the lazy
per-invocation terminal reuse. No Level 3 test is needed because no OS keyboard/input-encoder
behavior is specified.

### High — The recorded performance closeout is not reproducible under the specification's contract

The specification requires identical fixture bytes before and after, committed fixtures or a
checked-in deterministic generator, recorded byte sizes and content hashes, and predeclared
thresholds. `results.md:21-25` acknowledges that the after-run generator emitted different content;
`results.md:127-132` says to use "any deterministic generator"; and neither baseline nor results
records fixture hashes. The TOC values are therefore explicitly not point-comparable. The terminal
findings were measured with stdout and stderr non-TTY, so those measurements cannot establish the
interactive OSC gain either.

Check in the generator and fixture manifest (size plus Darkmatter/xxHash content identity), rerun
baseline and after measurements on byte-identical inputs and TTY modes, and record the declared
pass/fail thresholds plus dispersion/sample counts. The large reported wins are plausible, but the
current artifacts do not satisfy the feature's own release gate.

### Medium — Several findings are marked complete after only partial implementation or are deferred

The plan transparently defers Findings 11, 12, 13, 17, 32, and most of 35. It also marks Finding 14
complete while deferring the specified combined expression/literal scan and span-emission rewrite,
and marks Finding 33 complete while deferring the specified single forward line pass. Some
deferrals are prudent—especially retaining source-ordered body-shell execution—but archiving the
whole review as completed overstates delivery.

Either narrow the completed feature scope to the changes actually delivered and move the residual
items into linked unscheduled specs, or finish each remaining finding with its required checkpoint.
Do not parallelize body shell execution without a new ruling that preserves the side-effect-order
invariant.

### Medium — `validate_references_with_graph` exposes an unchecked public graph/document contract

The new public method accepts any `ReferenceGraph` and documents that it "must correspond" to the
document and options, but it cannot enforce that relationship. A caller can accidentally pair a
graph from another document or different graph options and receive a successful but incorrect
validation report. The only new test compares the two entry points with a matching graph.

Keep this optimization crate-private for the CLI path, or introduce a graph artifact that carries
and validates its document/options identity. Add negative tests for mismatched document content,
source path, and graph options before exposing the reuse path publicly.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| F1 — Darkmatter compose performs no NTP probe | Level 1 source/compose timing, non-TTY end-to-end measurement | Darkmatter's explicit call is correct; the unrelated bare `sniff` API was broken |
| F2/F3/F21 — terminal probes are cached and CLI reuses detection | Level 1 cache-stability tests; unrelated Level 2 render captures | Gap: spec requires Level 2 OSC query/latency evidence |
| F4 — TOC line lookup is no longer quadratic and preserves line semantics | Level 1 line-ending/span tests plus non-TTY end-to-end timing | Appropriate mechanism coverage; benchmark fixtures violate the reproducibility gate |
| F5/F6/F8/F9/F26-F29 — schema work/cache reuse remains behavior-compatible | Level 1 schema and compose tests | Runtime behavior is well exercised; public `EffectiveSchema` source compatibility is broken |
| F7/F16/F18 — graph/preflight work is reused | Level 1 graph, fragment, and compose tests | Appropriate for non-rendering behavior; mismatched public graph reuse is untested |
| F10-F15/F30-F35 — allocation and scan reductions | Level 1 unit/integration tests and Criterion smoke measurements | Several specified sub-items are deferred or only partially implemented |
| F19/F20/F23/F24 — render-path reductions preserve output | Level 1 renderer tests plus Level 2 real-terminal captures | Appropriate for visible rendering; current library L2 slice passed |
| F22/F25 — directory pruning and cleanup reductions | Level 1 unit/integration tests | Appropriate; no OS input or terminal encoder involved |
| Cross-platform compile/probe contract | macOS execution plus target-gated source inspection | No current Windows/Linux execution evidence was re-established in this review |

No requirement concerns keyboard, modifier presses, mouse, paste, or IME behavior, so Level 3 is
not applicable.

## Verification performed for this review

- Read the complete performance specification, plan, baseline, results, implementation commits,
  and relevant Darkmatter, biscuit-terminal, and sniff code paths.
- Used GitNexus to locate the compose/preflight, schema-validation, and reference-validation
  surfaces. Its index predates the latest `EffectiveSchema` type change, so current source and git
  history were used for that compatibility finding.
- `just test`: bounded at the non-interactive ceiling after 2,699 of 5,615 tests passed; no failure
  occurred before the clean interrupt, but a complete Level 1 result is not claimed.
- `just test-l2`: the Darkmatter library real-terminal slice passed 19/19 across the configured
  harnesses. The subsequent CLI L2 build exceeded the bounded window and was interrupted, so a
  complete area Level 2 result is not claimed.
- The passing L2 tests exercise terminal rendering but do not instrument the OSC query-count or
  latency requirement described above.
- Review-frontmatter validation could not run because `schemas/feature-review.yaml` is rejected by
  the current standalone-schema parser: its tagged form contains unsupported `description` and
  `$schema` keys. The requested frontmatter is retained exactly.

There is no previous review file in this feature directory: the requested `previous: "/"` sentinel
is therefore retained, and no nonexistent predecessor was modified.
