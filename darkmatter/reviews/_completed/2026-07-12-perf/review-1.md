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
and the complete current Level 1 and Level 2 package-area suites pass. The Finding 29 ownership
question is now resolved: the specification explicitly approves the public `Arc<Value>` exception,
and [`results-2.md`](./results-2.md) supplies same-source Criterion evidence for both the ownership
change and the subsequent zero-deep-clone baseline path.

The remaining production blockers are the changed semantics of `sniff::detect_timezone()`, the
absence of a Level 2 OSC-query measurement, and the original performance closeout's non-identical,
unhashed fixtures, plus the unapproved residual work represented as complete in the plan. The new
schema microbenchmarks are reproducible and requirement-matched for Finding 29, but they do not
retroactively repair the separate CLI, TOC, or terminal evidence or close other findings.

## Findings

### Resolved — `EffectiveSchema::json_schema` Arc ownership is an approved exception

The original finding was correct against review iteration 1: changing the public field from
`serde_json::Value` to `Arc<serde_json::Value>` was not source-compatible and contradicted the
then-current blanket compatibility invariant. Review iteration 2 now makes a narrow, explicit
exception for Finding 29. This repository has no established external users, so the measured
ownership benefit is accepted over preserving the old field type.

The A/B checkpoint in [`results-2.md`](./results-2.md) shows the Darkmatter baseline-only path
improving 69.4%, baseline-plus-document improving 28.6%, and effective-schema cloning improving
99.7% (125.53 µs → 340.21 ns). The 512-property synthetic cases also improve across all three
ownership-sensitive operations. The follow-up shared-baseline work then reduces built-in baseline
configuration from 391.12 µs to 26.42 ns and configuration plus resolution from 450.91 µs to
56.47 µs.

The independently mutable `darkmatter_base_json_schema() -> Value` accessor remains intact, while
the borrowed accessor and built-in-baseline builder expose the zero-clone path. This finding is no
longer a production blocker.

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
current CLI/TOC/terminal artifacts do not satisfy the feature's own release gate. The saved
Criterion baselines in `results-2.md` do satisfy this contract for Finding 29 specifically.

### Medium — Unapproved deferrals and partial implementations remain open

The plan defers Findings 11, 12, 13, 17, 32, and most of 35 without an approved scope reduction. It
also marks Findings 7, 14, 16, 23, 25, and 33 complete while explicitly retaining part of their
specified work. The safe subsets are useful, but a completed checkbox cannot represent the whole
finding while its residual work remains unimplemented and unmeasured.

Several stated rationales justify a separate implementation phase, not closeout. Broad blast
radius, ordering sensitivity, regression risk, an opportunistic phase boundary, or an assertion
that a path is rare are scheduling considerations. They do not demonstrate that the optimization
lacks value. In particular:

- Finding 12 can stop cloning `ResolutionContext` for pure functions by checking whether the
  function is context-aware before requesting the context; that portion does not require changing
  the public `EvaluationLookup` signature.
- Finding 13 can preserve longest-then-lexicographic replacement semantics through deterministic
  pattern ordering while retaining the existing owned public facade around an internal borrowed or
  `Cow` path.
- Finding 32 correctly rejects a stale once-per-stage `allow_once` snapshot, but compatible designs
  remain possible, including shared immutable policy sets with live approval-state checks.
- Finding 35's `RemoteFetchRuntime::get_content` rationale incorrectly describes an external public
  API break: the type is exposed only from the crate-private `remote_fetch` module. Other internal
  Finding 35 sub-items likewise require measurement and implementation decisions rather than a
  blanket plumbing deferral.

Two proposed approaches do have a substantive correctness objection. Carrying a fully prepared
body for Findings 7/16 can violate condition-aware rendering, so that literal approach should not
land without a cache identity/design that preserves those semantics. Parallel body-shell execution
from Finding 17 is explicitly rejected because it violates the source-order side-effect invariant.
Neither decision closes the underlying duplicated-work problem or Finding 17's independent 10 ms
process-polling issue.

Reopen every residual item in the active review. A finding or independently tracked sub-item may be
closed only after it is implemented and passes its specified checkpoint, a requirement-matched
benchmark shows that the gain is indistinguishable from noise, or the repository owner explicitly
approves a documented rejection or deferral. Any approved deferral must move to a linked active or
unscheduled spec; it must not be counted as delivered work in this review.

### Medium — `validate_references_with_graph` exposes an unchecked public graph/document contract

> **Superseded 2026-07-15 by the Opaque Reference Graph feature**
> (`darkmatter/features/2026-07-15-reference-graph/`). `ReferenceGraph` became the
> identity-carrying, builder-produced artifact this finding asked for: its private
> provenance records document/source/mode/options identity plus a visited-descendant
> manifest, and `validate_references_with_graph` now rejects any mismatched pairing
> (including a changed/missing/unreadable descendant) with
> `ReferenceError::ReferenceGraphMismatch` before flattening. The negative tests for
> mismatched document content, source path, and graph options ship with that feature.
> The historical finding below is retained unchanged.

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
| F5/F6/F8/F9/F26-F29 — schema work/cache reuse remains behavior-compatible | Complete Level 1 schema/compose coverage; Criterion A/B evidence and named zero-clone baseline in `results-2.md` | Runtime behavior passes; F29's public Arc ownership is an explicit measured compatibility exception and its default path now has zero deep clones |
| F7/F16/F18 — graph/preflight work is reused | Level 1 graph, fragment, and compose tests | Safe graph reuse landed; deep cross-pass work remains open, and mismatched public graph reuse is untested |
| F10-F15/F30-F35 — allocation and scan reductions | Level 1 unit/integration tests and Criterion smoke measurements | F11-F14, F17, F32, F33, and F35 retain open work |
| F19/F20/F23/F24 — render-path reductions preserve output | Level 1 renderer tests plus Level 2 real-terminal captures | Visible output passes; F23's environment-hoist sub-item remains open |
| F22/F25 — directory pruning and cleanup reductions | Level 1 unit/integration tests | F22 is covered; F25's line-pass fusion remains open |
| Cross-platform compile/probe contract | macOS execution plus target-gated source inspection | No current Windows/Linux execution evidence was re-established in this review |

No requirement concerns keyboard, modifier presses, mouse, paste, or IME behavior, so Level 3 is
not applicable.

## Verification performed for this review

- Read the complete performance specification, plan, baseline, results, implementation commits,
  `results-2.md`, and relevant Darkmatter, biscuit-terminal, and sniff code paths.
- Used GitNexus to locate the compose/preflight, schema-validation, and reference-validation
  surfaces before the follow-up implementation, then ran `detect_changes` after it. The
  uncommitted follow-up scope was LOW risk with no affected indexed execution flows.
- `effective_schema_ownership`: saved and compared `arc-value-54e2f6b65` and
  `pre-zero-clone-cf90c1582` under the sniff-backed host preflight; 100 Criterion samples per
  function with identical benchmark source across the zero-clone comparison.
- `just test`: 5,616 Darkmatter library, 555 CLI, and 566 DMLS tests passed (6,737 total).
- `just test-l2`: 19 Darkmatter, 69 CLI, and 3 DMLS tests passed in the configured real-terminal
  harnesses (91 total).
- `just lint` passed for the Darkmatter library, CLI, and DMLS. `cargo check -p darkmatter -p dmls`
  and `git diff --check` also passed.
- The complete L2 suite exercises terminal rendering but still does not instrument the OSC
  query-count or latency requirement described above.
- Review-frontmatter validation could not run because `schemas/feature-review.yaml` is rejected by
  the current standalone-schema parser: its tagged form contains unsupported `description` and
  `$schema` keys. The requested frontmatter is retained exactly.

There is no previous review file in this feature directory: the requested `previous: "/"` sentinel
is therefore retained, and no nonexistent predecessor was modified.
