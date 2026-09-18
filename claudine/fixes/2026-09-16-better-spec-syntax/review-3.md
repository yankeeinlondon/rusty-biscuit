---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T13:23:13-07:00
spec: 2026-09-16-better-spec-syntax/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
description: A **fix** review of `2026-09-16-better-spec-syntax/spec.md`
fix: 2026-09-16-better-spec-syntax/review-3.md
previous: 2026-09-16-better-spec-syntax/review-2.md
next: 2026-09-16-better-spec-syntax/review-4.md
findings:
    - "[high] Authored set order is still lost outside top-level inline side-effect steps"
    - "[high] Task setup and teardown set failures still lose source-rooted diagnostics and excerpts"
---

# Review 3: Make Key-to-Value Mappings the Only Claudine `set` Action Syntax

## Verdict

The fix is **not ready for production**. Review 2's global JSON-order
regression is removed, primary side-effect failures now use group-member source
provenance, and the typed runtime evaluation error can retain a frontmatter
excerpt across the in-process snapshot boundary. The focused regressions for
those changes pass.

The scoped order channel is not carried through all of the task sources that
accept the shared lifecycle grammar, however, and task `setup`/`teardown`
failures still use event-rooted paths without excerpt enrichment. These are
explicit R1/R2/R4/R5 surfaces and need Level 1 regressions that distinguish
them from the covered top-level side-effect path.

Human review is not required. The specification already defines the required
ordering and diagnostic behavior.

## Prior Review Closure

### Unblocked Findings

1. **Global preserve-order activation breaks established Claudine
   serialization contracts — partially implemented.** The global
   `serde_json/preserve_order` feature is gone, and both unrelated serializers
   identified by Review 2 pass. Darkmatter now records authored mapping order
   separately and Claudine consumes it for lifecycle event stacks and a
   top-level inline sequence side-effect. The metadata is not propagated to
   nested/external task sources or task action stacks (Finding 1).
2. **Group-member set failures report a non-source-rooted document and task
   path — implemented for primary side-effect evaluation failures.** Inline
   groups, external groups, and externalized group tasks now carry the expected
   source document and semantic value path, and the focused Level 1 tests pass.
   Task `setup`/`teardown` still bypass that provenance for parse and execution
   diagnostics (Finding 2).
3. **Runtime set evaluation errors cannot attach the required frontmatter
   excerpt — partially implemented.** `LifecycleEvaluationError` now selects
   its exact property for enrichment, and event-stack plus primary side-effect
   tests prove the excerpt path. Task action-stack failures are returned without
   enrichment (Finding 2).

### Blocked Findings

Review 2 contained no blocked findings, so none could have become unblocked
before this implementation.

## Findings

### 1. Authored set order is still lost outside top-level inline side-effect steps (high)

The new order side channel is populated only when the invoking document's
`sequence` value is itself an array, and only for each top-level step whose
executable is `side_effect` (`lib/src/composition/sequence/mod.rs:182-195`).
Every side-effect task created while recursively loading an inline group,
external group, or external task starts with `authored_set_order: None`
(`lib/src/composition/sequence/preflight/mod.rs:427-430`). External task and
group documents are loaded as plain `serde_json::Value` values
(`lib/src/composition/sequence/preflight/mod.rs:493-524, 536-558`), so their
YAML order metadata is unavailable by the time `RuntimeSet` is built. Task
`setup` and `teardown` have the same problem: `parse_task_action_stack` always
passes `None` for authored metadata (`lib/src/composition/lifecycle/parse.rs:319-341`).

Without metadata, the ordinary JSON object has canonical key order. A group
member or external task authored as `z: ...` then `a: ...` therefore executes
and serializes as `a` then `z`. This violates R2's requirement to retain
assignment order for stable diagnostics and result serialization and R4's
observable prior-value object contract. For setup/teardown, it can also change
which of two failing authored values is diagnosed first. The new exact-output
test covers only a top-level inline `sequence[].side_effect.set`, so it cannot
detect any of these paths.

**Required change:** carry ordered mapping metadata through every recursively
loaded authored task document and through `setup`/`teardown`, while keeping
ordinary `serde_json::Map` behavior unchanged. Add Level 1 exact-order tests
for an inline-group member, an external group member, an externalized task, and
a referenced formal sequence. Add a task-stack case where authored and lexical
order differ and the first failing assignment proves diagnostic order.

### 2. Task setup and teardown set failures still lose source-rooted diagnostics and excerpts (high)

`TaskExecution::parse_stacks` parses task stacks with the bare properties
`setup` and `teardown` (`lib/src/composition/sequence/task/mod.rs:736-752`),
not the `TaskDiagnosticProvenance` path added for the primary executable.
At execution, the shared stack executor constructs locations from the synthetic
signals, yielding `start.stack[n]` or `finalize.stack[n]`
(`lib/src/composition/lifecycle/executor.rs:1025-1060, 1187-1189`).
`run_stack` then copies the resulting `LifecycleErrorInfo` directly into the
task diagnostic (`lib/src/composition/sequence/task/mod.rs:446-465`). It does
not rebase the property to `tasks[n].setup`/`tasks[n].teardown` and does not run
the source-text enrichment used by primary side-effect failures
(`lib/src/composition/sequence/task/mod.rs:712-730`).

Consequently, a malformed mapping or nested expression failure in a top-level,
inline-group, external-group, or externalized task stack cannot satisfy R5 and
acceptance criterion 10: the rendered, `err.*`, and machine projections name a
synthetic event location rather than the authored task property, and the
terminal diagnostic has no frontmatter excerpt even when the source is a
locatable Markdown frontmatter value. The new provenance and excerpt tests all
exercise the primary `side_effect`, so they do not distinguish this defect.

**Required change:** give setup and teardown stacks their source-rooted task
property and authored-order metadata, use that root while constructing both
parse-time and execution-time paths, and enrich the resulting typed diagnostic
from the owning document before snapshotting it. Add Level 1 cases for
malformed and evaluation failures in setup/teardown, including at least one
group or external-task source, asserting exact file, full property, projection
parity, excerpt, and atomic rollback.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Mapping-only syntax works in event stacks, nested action lists, task setup/teardown, and sequence side-effect tasks | Level 1 parser, executor, sequence, and hermetic CLI tests | Appropriate for syntax/execution. Covered shapes pass, but ordering and failure diagnostics are incomplete on task-stack and nested/external task sources (Findings 1–2). |
| Native scalar/null/array/object values and recursive execution-time interpolation retain types | Level 1 parser and executor tests | Appropriate and green. |
| One mapping reads one snapshot and commits atomically with and without shared runtime state | Level 1 executor/runtime tests, including event and sequence failures | Appropriate and green on covered surfaces. |
| Prior-value objects and textual sequence output retain authored assignment order | Level 1 exact-output test for a top-level inline side-effect | **Gap:** no discriminating test for group members, external task/group/formal documents, or task-stack diagnostic order (Finding 1). |
| Removed forms, malformed mappings, reserved keys, and false guards produce the canonical behavior | Level 1 parser and diagnostic tests | Appropriate for event/top-level parsing; task-stack diagnostics do not retain their source-rooted path (Finding 2). |
| Runtime failures name the source document and full semantic value path in terminal, `err.*`, and machine projections | Level 1 event, top-level side-effect, inline-group, external-group, and external-task tests | Primary side-effect coverage is appropriate and green. **Gap:** setup/teardown failures remain event-rooted (Finding 2). |
| Runtime failures attach a frontmatter excerpt when the source and property are locatable | Level 1 event and primary side-effect tests | Primary side-effect coverage is appropriate and green. **Gap:** setup/teardown bypass enrichment (Finding 2). |
| Duplicate YAML keys fail at every nesting level and through fallback parsing | Level 1 shared-parser tests plus full Darkmatter package-area tests | Appropriate and green. |
| The shipped implementation prompt runs normally without source mutation, provider use, or audio side effects | Level 1 hermetic CLI regression | Correct level, but the current checkout's unrelated prompt edit makes this test red; this run is not passing readiness evidence. |
| Loop-control `set(...)`, Darkmatter's positional API, state isolation, other verbs, and unrelated JSON serializers remain unchanged | Level 1 unit/integration coverage | Appropriate; focused serializer regressions and the complete Darkmatter Level 1 gate pass. |

Level 2 is not required because no requirement depends on real-terminal glyphs,
width, styling, scrolling, or emulator behavior. Level 3 is not required
because no requirement depends on OS keyboard/mouse injection, terminal input
encoding, hotkeys, paste, or IME behavior.

## Verification Performed

- Read the complete specification, Reviews 1 and 2, implementation log,
  current implementation diff, and the lifecycle parser/executor, recursive
  sequence preflight, task execution, diagnostics, and frontmatter-order paths.
- GitNexus was current at `4399b3a`; the relevant concept query reached the
  lifecycle stack execution context. Text tracing was used for the order and
  task-provenance paths that the graph does not distinguish.
- Eight focused Level 1 regressions passed: scoped JSON serialization, event
  path/excerpt, top-level authored output order, top-level atomic failure, and
  inline/external group/task source provenance.
- Full Claudine Level 1 with `--no-fail-fast`: 7,285 tests ran; 7,276 passed,
  nine failed, and nine were skipped. All nine failures are confined to the
  independently modified shipped implementation prompt/fixture: the new
  `has_skill(ctx.area)` expression rejects a slash-bearing value and the
  fixture no longer matches the shipped prompt. The feature-focused tests
  passed, but the area gate is still red in this checkout.
- Full Darkmatter Level 1: 7,942 tests passed and seven were skipped.
- `just lint` passed in both `claudine/` and `darkmatter/`, including the eight
  transport error guards and lifecycle documentation facet guard.
- No Level 2 or Level 3 test was run because neither tier verifies an
  additional boundary for this syntax, ordering, state, or diagnostic behavior.
  No formatting command or Git commit was performed.

## Production Readiness Closure

Production readiness requires authored-order metadata to survive every task
source accepted by the shared grammar and task setup/teardown failures to carry
the owning document, exact task-rooted property, typed projection parity, and
frontmatter excerpt. The corresponding Level 1 regressions must distinguish
those paths from the already-green top-level primary side-effect cases.
