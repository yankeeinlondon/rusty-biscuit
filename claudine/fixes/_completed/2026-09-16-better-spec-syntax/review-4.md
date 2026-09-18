---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T18:50:47-07:00
spec: 2026-09-16-better-spec-syntax/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
next: 2026-09-16-better-spec-syntax/review-5.md
description: A **fix** review of `2026-09-16-better-spec-syntax/spec.md`
fix: 2026-09-16-better-spec-syntax/review-4.md
previous: 2026-09-16-better-spec-syntax/review-3.md
findings:
    - "[high] Primary side_effect set parse failures report a synthetic, task-unrooted path with no excerpt"
    - "[medium] Authored order is not consistently applied before validation or in JSON/JSON5 task documents"
---

# Review 4: Make Key-to-Value Mappings the Only Claudine `set` Action Syntax

## Verdict

The fix is **not ready for production**, but it is close. Both Review 3
findings are implemented in the working tree. Their regressions discriminate
the paths Review 3 named, and the focused Level 1 verification is green.

One more surface has the diagnostic defect Review 3 fixed for
`setup`/`teardown`: a **parse-time** failure of a task's primary
`side_effect: {set: …}`. It still reports a synthetic property with no task
root and no frontmatter excerpt. R5 and acceptance criterion 10 use that exact
surface as their example (`tasks[2].side_effect.set.ready`). No test covers it,
so the gap was not visible.

Authored-order handling also remains incomplete at two edges: invalid
destination keys are validated in canonical map order before the authored
order is restored, and accepted JSON/JSON5 task documents carry no order
metadata at all.

Human review is not required. The specification already defines the required
path shape, and the remaining work is mechanical.

## Prior Review Closure

### Unblocked Findings

1. **Authored set order is still lost outside top-level inline side-effect
   steps: implemented.** An `AuthoredOrder` cursor
   (`lib/src/composition/authored_order.rs`) now moves through
   `load_step_task` → `load_task` → `load_group`/`build_group` →
   `load_external_task`. When the walk enters another file,
   `load_data_document` resets the cursor, the same way it resets the semantic
   property path (`sequence/preflight/mod.rs:377-720, 997-1010`). A catalog
   group re-roots the cursor at `/groups/{k}` for the selected entry.
   Referenced formal sequences re-root at `/sequence`. That happens only when
   `is_formal_sequence` has already ruled out an offset or operator
   (`sequence/source.rs:150-167, 192-198`), so a reshaped list cannot pick up
   another step's order. Task `setup`/`teardown` now pass
   `authored.child(stage)` into `parse_task_action_stack_with_order`
   (`sequence/task/mod.rs:775-800`). Six Level 1 tests in
   `task::tests::authored_set_order` assert the exact serialized
   `{"z_last_lexically":…,"a_first_lexically":…}` output for these sources:
   - inline-group members
   - external-group members
   - catalog-group members, with a decoy entry at index 0
   - externalized task documents
   - referenced formal sequences

   A setup-stack case shows that the first *authored* failing assignment is
   the one diagnosed.
2. **Task setup and teardown set failures still lose source-rooted
   diagnostics and excerpts: implemented.** Stacks are rooted as
   `StackRoot::Task(property)` (`lifecycle/executor.rs:418-445`). Event stacks
   keep their byte-identical `{signal}.stack[n]` spelling, and a guard test
   covers that. `run_stack` and the parse branch of `run_stages` both enrich
   from the owning document through `with_owning_excerpt`
   (`sequence/task/mod.rs:367-512`). The `task_stack_diagnostics` module checks
   the exact file, the full property, terminal/`err.*`/snapshot parity, the
   absence of `start.stack`/`finalize.stack`, the excerpt, and atomic
   rollback. It covers inline, group-member, and external-task sources for
   malformed, removed, and evaluation failures.

### Blocked Findings

Review 3 contained no blocked findings, so none could have become unblocked
before this implementation.

## Findings

### 1. Primary side_effect set parse failures report a synthetic, task-unrooted path with no excerpt (high)

`TaskExecution::run_side_effect` parses the action with the bare literal
property `"side_effect"` (`lib/src/composition/sequence/task/mod.rs:703-716`),
not with `self.task.diagnostic.action_property`. On a parse error it returns
`TaskDiagnostic::from_composition(TaskStage::Primary, &error)` directly and
never calls `with_owning_excerpt`. `parse_single_action_with_order` treats the
object as stack item 0, so the reported path also gains an `action[0]` segment
that the author never wrote.

The typed error projection produced by that path is therefore:

| Authored source | Reported `detail.property` | Excerpt |
| --- | --- | --- |
| `sequence[0].group.tasks[0].side_effect: {set: ready}` | `side_effect.action[0].set` | none |
| `sequence[0].side_effect: {set: [ready, "yes"]}` (removed form) | `side_effect.action[0].set` | none |

The expected properties are `tasks[0].group.tasks[0].side_effect.set` and
`tasks[0].side_effect.set`. The runtime (evaluation-failure) path for the same
action already reports `tasks[0].side_effect.set.metadata.files[0]`
(`task/tests.rs:2381, 3713, 3753`). So parse and runtime failures of one
authored mapping currently name two different, incompatible locations. This
fails these requirements:

- R5: "a stable semantic action path, for example
  `tasks[2].side_effect.set.ready`"
- acceptance criterion 10: "Diagnostics name the source and full semantic
  value path"

It is the same class of defect as Review 3 Finding 2. The primary side-effect
parse path was not included in that fix, and no test for it exists. Every
existing primary side-effect diagnostic test is an evaluation failure.

**Required change:** parse the primary action at
`self.task.diagnostic.action_property`. Make that root produce
`<task>.side_effect.set…` with no synthetic `action[0]` segment, consistent
with the runtime path. Route the parse error through `with_owning_excerpt`
before snapshotting it. Add Level 1 cases using the existing
`assert_stack_failure` helper (or an equivalent) for:

- a malformed mapping on a top-level step
- a removed form in an inline-group member
- a malformed mapping in an external `kind: task` document, where the root
  is the document itself and no excerpt is expected

Each case must assert the exact property, source, projection parity, and
excerpt.

### 2. Authored order is not consistently applied before validation or in JSON/JSON5 task documents (medium)

`RuntimeSet::new_with_order` validates empty and dynamic destination keys while
iterating the canonical `serde_json::Map`, then restores authored order
afterward (`lifecycle/actions.rs:282-295`). A YAML mapping with two invalid
keys therefore diagnoses the lexically first key rather than the first authored
key. That contradicts R2's requirement to retain assignment order for stable
diagnostics, and the current order regression only exercises value-evaluation
failures after reordering.

The order channel is also format-dependent. `sequence::data::load_document`
records `MappingOrders` only for YAML (`sequence/data.rs:64-121`), although the
same loader accepts JSON and JSON5 task/group documents. A `set` in one of
those documents serializes its prior-value result in canonical map order rather
than source order. R1 applies the shared lifecycle grammar wherever accepted,
and R2/R4 do not exempt these accepted document formats.

**Required change:** reorder the assignment entries before destination-key
validation and add a discriminating YAML test with multiple invalid keys. Carry
source order through JSON/JSON5 task and group documents as well, with exact
result-order tests, or narrow the specification and public documentation so
those formats explicitly promise canonical rather than authored order.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Mapping-only syntax works in event stacks, nested action lists, task setup/teardown, and sequence side-effect tasks | Level 1 parser, executor, sequence, and hermetic CLI tests | Appropriate and green. |
| Native scalar/null/array/object values and recursive execution-time interpolation keep their types | Level 1 parser and executor tests | Appropriate and green. |
| One mapping reads one snapshot and commits atomically, with and without shared runtime state | Level 1 executor/runtime tests, including event, side-effect, and task-stack failures | Appropriate and green. |
| Prior-value objects and textual sequence output keep authored assignment order | Level 1 exact-output tests for top-level, inline-group, external-group, catalog-group, external-task, and referenced-formal YAML sources | **Gap:** JSON/JSON5 task documents use canonical map order, and no test covers that accepted source path (Finding 2). |
| Removed forms, malformed mappings, reserved keys, and false guards produce the canonical behavior | Level 1 parser and diagnostic tests | Appropriate for event stacks and task stacks. **Gaps:** primary `side_effect` parse failures have no path/excerpt test and are wrong (Finding 1); multiple invalid destination keys are not diagnosed in authored order (Finding 2). |
| Failures name the source document and full semantic value path in terminal, `err.*`, and machine projections | Level 1 event, primary side-effect evaluation, and setup/teardown parse and evaluation tests | **Gap:** primary `side_effect` parse failures report `side_effect.action[0].set` (Finding 1). |
| Failures attach a frontmatter excerpt when the source and property can be located | Level 1 event, primary side-effect evaluation, and setup/teardown tests | **Gap:** primary `side_effect` parse failures have no excerpt (Finding 1). |
| Duplicate YAML keys fail at every nesting level and through fallback parsing | Level 1 shared-parser tests plus the full Darkmatter area gate | Appropriate and green. |
| The shipped implementation prompt runs normally without source mutation, provider use, or audio side effects | Level 1 hermetic CLI regression | Correct level, but it is red in this checkout because of the unrelated `has_skill(ctx.area)` prompt edit. It is not passing readiness evidence yet. |
| Loop-control `set(...)`, Darkmatter's positional API, state isolation, other verbs, and unrelated JSON serializers stay unchanged | Level 1 unit and integration coverage | Appropriate and green. |

Level 2 is not required: no requirement depends on how a real terminal renders
glyphs, widths, styling, or scrolling. Level 3 is not required: no requirement
depends on OS keyboard or mouse injection, the terminal's input encoder,
hotkeys, paste, or IME behavior. The TTY-gated excerpt is correctly exercised
at Level 1 by setting `term.is_tty` and asserting on the restored rendering.

## Verification Performed

- Read the specification, Review 3, the cycle-3 implementation-log entries,
  commits `14f444ab6` and `3268e59ab`, and the current working-tree diff.
- Bound GitNexus to `better-static-analysis`. Its index remained 14 commits
  behind: the required `just gitnexus` refresh waited ten minutes for another
  analyzer process (`pid 78755`) and exited on the repository lock timeout.
  Stale graph results were not treated as evidence; private-method callers and
  the diagnostic path were confirmed directly in current source.
- Traced `AuthoredOrder` through sequence planning (`sequence/mod.rs`,
  `source.rs`), preflight (`preflight/mod.rs`, `data.rs`), task execution
  (`task/mod.rs`), the lifecycle parser (`parse.rs`), and
  `RuntimeSet::new_with_order`. Traced `StackRoot` and the owning-document
  enrichment through `lifecycle/executor.rs` and `task/mod.rs`.
- Inspected the primary parse and diagnostic projection path end to end. The
  parser is called with literal `side_effect`, constructs `action[0].set`, and
  snapshots the error without the owning-document enrichment used by the
  runtime path.
- The focused Level 1 slice for `task_stack_diagnostics`,
  `authored_set_order`, `runtime_set`, and `action_shape_control` passed: 129
  tests run, 129 passed.
- `cd claudine && just test --no-fail-fast`: 7,301 tests ran. 7,291 passed,
  10 failed, and 9 were skipped.
  - Nine failures are the known `has_skill(ctx.area)` failures from the
    independently edited shipped `implement-plan.md` prompt and fixture.
  - One failure, `cross_platform_prompt_composes_cleanly`, happens because
    `prompts/cross-platform.md` is deleted in the working tree. That deletion
    is unrelated work by another session.
- `cd claudine && just lint`: passed.
- Darkmatter was not rerun because it has not changed since Review 3; that
  review recorded 7,945 passing Level 1 tests, 7 skips, and a passing lint
  gate for the shared duplicate-key parser work.
- No Level 2 or Level 3 test was run, because neither tier checks any boundary
  this change touches. No formatting command or Git commit was performed.

## Production Readiness Closure

Production readiness requires three things:

- A primary `side_effect: {set: …}` parse failure must report the owning
  document, the task-rooted property `<task>.side_effect.set…`, projection
  parity, and a frontmatter excerpt, with discriminating Level 1 tests for
  top-level, group-member, and external-task sources.
- Destination-key validation must follow authored order, and accepted
  JSON/JSON5 task sources must either preserve authored order or have their
  canonical-order exception explicitly ratified in the specification and
  documentation.

The corrected `parse_stacks` line is present in the reviewed working tree.
Unrelated shipped-prompt and prompt-move failures are reported as verification
noise, not as readiness findings for this specification.
