---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T12:04:25-07:00
spec: 2026-09-16-better-spec-syntax/spec.md
implemented: true
implemented_by: codex/gpt-5.6-sol
log: claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
description: A **fix** review of `2026-09-16-better-spec-syntax/spec.md`
fix: 2026-09-16-better-spec-syntax/review-2.md
previous: 2026-09-16-better-spec-syntax/review-1.md
next: 2026-09-16-better-spec-syntax/review-3.md
findings:
    - "[high] Global preserve-order activation breaks established Claudine serialization contracts"
    - "[high] Group-member set failures report a non-source-rooted document and task path"
    - "[high] Runtime set evaluation errors cannot attach the required frontmatter excerpt"
---

# Review 2: Make Key-to-Value Mappings the Only Claudine `set` Action Syntax

## Verdict

The fix is **not ready for production**. Review 1's authored-order regression
now passes at the lifecycle result boundary, and execution-time failures in an
event stack or a top-level sequence side-effect task now retain their nested
value suffix across terminal, `err.*`, and machine projections. Those changes
do not close the review safely, however: the order fix changes every
`serde_json::Map` in Claudine's dependency graph and breaks two existing
Claudine tests, while the diagnostic fix computes the wrong document and task
path for group members and still cannot attach a frontmatter excerpt.

Human review is not required. The specification already requires a scoped
ordered representation, a source-rooted semantic path, and an excerpt whenever
the existing infrastructure can locate the authored property.

## Prior Review Closure

### Unblocked Findings

1. **Lifecycle set discards authored assignment order before result
   serialization — partially implemented.** The new Level 1 regression proves
   that a top-level sequence side-effect result retains non-lexical authored
   order. The implementation is not production-safe because it activates
   `serde_json/preserve_order` for the entire Claudine graph and breaks existing
   serialization contracts (Finding 1).
2. **Runtime set failures do not retain the required full semantic value path —
   partially implemented.** Event-stack and top-level sequence tests now carry
   the nested object/array suffix and preserve diagnostic identity. Group-member
   tasks still report the wrong source document and index (Finding 2), and the
   typed runtime variant is absent from excerpt selection (Finding 3).

### Blocked Findings

Review 1 contained no blocked findings, so none could have become unblocked
before this implementation.

## Findings

### 1. Global preserve-order activation breaks established Claudine serialization contracts (high)

The order fix enables `serde_json`'s `preserve_order` feature on Claudine's
ordinary dependency (`claudine/lib/Cargo.toml:30-32`). Cargo features are
additive across the package graph, so this changes the backing representation
and iteration order of every `serde_json::Map` compiled into Claudine, not only
the nested lifecycle `set` payload parsed by Darkmatter. `cargo tree -p
claudine -i serde_json -e features` confirms that `preserve_order` and its
`indexmap` dependency are active throughout that graph.

The full Claudine library Level 1 run exposes two direct regressions:

- `opencode_config::tests::yolo_permission_block_serializes_independent_of_insertion_order`
  now emits different bytes for logically identical permission objects. The
  test explicitly protects byte-stable, cross-platform serialization
  (`lib/src/opencode_config.rs:220-235`).
- `provider::tests::serialized_field_list_matches_catalog` now returns struct
  declaration order instead of the canonical sorted field inventory
  (`lib/src/provider/tests.rs:237-255`). Its comment still states that
  `serde_json` is compiled without `preserve_order`, so the implementation also
  leaves behavior-changing documentation drift.

This violates the specification's narrow lifecycle-grammar scope and leaves a
mandatory Level 1 gate red. The targeted lifecycle order test passing is not
sufficient evidence when the mechanism changes unrelated serializers.

**Required change:** preserve authored order at the frontmatter/lifecycle value
boundary without globally changing Claudine's JSON map semantics. Carry ordered
mapping data or source-order metadata through Darkmatter's single validated
parse into `RuntimeSet`. If a broader shared representation change is chosen,
it must be treated as separately scoped work: audit and intentionally
canonicalize every byte-stable serializer, update its contracts and comments,
and make the complete Claudine and Darkmatter Level 1 suites green.

### 2. Group-member set failures report a non-source-rooted document and task path (high)

`run_side_effect` always builds `tasks[n].side_effect.set` from a flattened
preflight walk (`lib/src/composition/sequence/task/mod.rs:680-725`). The walk
increments the counter for a group container before visiting its members. The
first member of the first group therefore reports `tasks[1]`, although an
external `kind: group` document authors that member at `tasks[0]`.

The file identity is wrong at the same boundary. The CLI creates the stack
context from the top-level preflight task's `origin_path`
(`cli/src/commands/wrap/sequence/task_run.rs:125-127`). Group scheduling then
reuses that context for every member (`lib/src/composition/sequence/task/group.rs:141-165`),
even though each member has its own `origin_path`. For a referenced group file,
the diagnostic consequently names the invoking sequence document while its
reported `tasks[n]` path belongs to neither that document nor the group file.

The new test covers only a single top-level task, where the flattened index is
zero and the fixture's stack source happens to equal the task source. It cannot
detect either defect. This leaves R5 and acceptance criterion 10 incomplete for
one of R1's explicitly supported sequence-task surfaces.

**Required change:** carry authored diagnostic provenance in `PreflightTask`
instead of rediscovering it by pointer identity at execution time. It should
identify the owning document and the exact source-rooted property path for a
top-level task, inline-group member, external-group member, and externalized
task. Build the runtime-set path from that provenance and execute each member
with the matching source document. Add Level 1 failures for at least inline and
external group members, asserting the file, exact property, projection parity,
and atomic rollback.

### 3. Runtime set evaluation errors cannot attach the required frontmatter excerpt (high)

R5 requires the existing frontmatter excerpt to be attached when the
diagnostic infrastructure can locate the authored value. The implementation
now gives `LifecycleEvaluationError` an exact `property`, but
`frontmatter_block_spec` does not include that variant
(`lib/src/composition/error/mod.rs:3184-3237`). Calling
`enrich_frontmatter[_text]` therefore returns the error unchanged even for a
direct event-stack failure whose source and exact path are both known.

The new tests render the error or restored snapshot directly and assert only
that the path and filename appear in prose. None calls the normal excerpt
enrichment seam or asserts a highlighted frontmatter block, so the explicit R5
requirement remains unverified and unimplemented.

**Required change:** make `LifecycleEvaluationError` select its exact property
for frontmatter enrichment, retaining the same effective diagnostic identity.
Add a Level 1 event-stack regression using real frontmatter text and assert that
the nested `set` value line is selected. Sequence/group coverage should assert
an excerpt when that execution path retains the typed error; if snapshots are
the intentional boundary, preserve enough source/excerpt data in the snapshot
to render the same diagnostic rather than silently dropping it.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Mapping-only syntax works in event stacks, nested action lists, task setup/teardown, and sequence side-effect tasks | Level 1 parser, executor, sequence, and hermetic CLI tests | Appropriate and green for the accepted surfaces. |
| Native scalar/null/array/object values and recursive execution-time interpolation retain types | Level 1 parser and executor tests | Appropriate and green. |
| One mapping reads one snapshot and commits atomically with and without shared runtime state | Level 1 executor/runtime tests | Appropriate and green. |
| Prior-value objects and textual sequence output retain authored assignment order | Level 1 exact-output regression | The targeted behavior is green, but its global mechanism breaks unrelated Level 1 serialization contracts (Finding 1). |
| Removed forms, malformed mappings, reserved keys, and false guards produce the canonical behavior | Level 1 parser and diagnostic tests | Appropriate and green. |
| Runtime failures name the source document and full semantic value path in terminal, `err.*`, and machine projections | Level 1 event and top-level sequence tests | **Gap:** inline/external group members are untested and report incorrect provenance (Finding 2). |
| Runtime failures attach a frontmatter excerpt when the source and property are locatable | No discriminating test; Level 1 tests render prose/snapshots without enrichment | **Gap:** the typed variant is omitted from excerpt selection (Finding 3). |
| Duplicate YAML keys fail at every nesting level and through fallback parsing | Level 1 shared-parser tests and Darkmatter package tests recorded in Review 1 | Appropriate boundary; unchanged by this iteration. |
| The shipped implementation prompt runs normally without source mutation, provider use, or audio side effects | Level 1 hermetic CLI regression recorded in Review 1 | Appropriate boundary; cross-OS result collection is external to readiness. |
| Loop-control `set(...)`, Darkmatter's positional API, state isolation, and other verbs remain unchanged | Level 1 unit/integration coverage | The retained APIs are covered, but unrelated JSON serializers regress under the order mechanism (Finding 1). |

Level 2 is not required because no requirement depends on real-terminal glyphs,
width, styling, scrolling, or emulator behavior. Level 3 is not required
because no requirement depends on OS keyboard/mouse injection, terminal input
encoding, hotkeys, paste, or IME behavior.

## Verification Performed

- Read the complete specification, Review 1, implementation log, focused diff,
  and the sequence preflight/task/group and diagnostic-enrichment paths.
- GitNexus was current at `4399b3a`. Branch-wide comparison is capped at 1,408
  changed symbols and reports critical aggregate risk because this worktree
  contains several unrelated initiatives. Text tracing was used where symbol
  lookup resolved same-named symbols from other package areas or returned
  `UNKNOWN`.
- The four focused Level 1 regressions for authored order, event error paths,
  top-level sequence error paths, and `proxy.with` order passed individually.
- Full Claudine library Level 1 with `--no-fail-fast`: 4,298 tests ran; 4,293
  passed, four failed, and one timed out. Two failures are caused by this
  iteration's global `preserve_order` feature (Finding 1). Two shipped-prompt
  failures come from unrelated dirty prompt edits already recorded by the
  implementer. The preflight reference-spelling test timed out under the full
  run and had previously passed alone according to the implementation log.
- Both feature-caused failures reproduced individually with exact value/order
  diffs.
- `just lint` in `claudine/` passed for all five packages, the eight transport
  error guards, and the lifecycle documentation facet guard.
- No Level 2 or Level 3 test was run because neither tier verifies an
  additional boundary for this syntax, state, serialization, or diagnostic
  behavior. No formatting command or Git commit was performed.

## Production Readiness Closure

Production readiness requires an order-preserving lifecycle representation
that does not alter unrelated JSON serialization, source-authored provenance
for every sequence task including group members, and frontmatter excerpt
selection for runtime `set` evaluation errors. The complete relevant Level 1
suite must then pass, including regressions that distinguish top-level tasks
from inline and external group members.
