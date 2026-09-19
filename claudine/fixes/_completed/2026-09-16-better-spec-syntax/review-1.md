---
$schema: feature-review.yaml
ready: false
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T11:00:50-07:00
spec: 2026-09-16-better-spec-syntax/spec.md
log: claudine/fixes/2026-09-16-better-spec-syntax/implementation-log.md
implemented: true
implemented_by: codex/gpt-5.6-sol
next: 2026-09-16-better-spec-syntax/review-2.md
description: A **fix** review of `2026-09-16-better-spec-syntax/spec.md`
fix: 2026-09-16-better-spec-syntax/review-1.md
findings:
    - "[high] Lifecycle set discards authored assignment order before result serialization"
    - "[high] Runtime set failures do not retain the required full semantic value path"
---

# Review 1: Make Key-to-Value Mappings the Only Claudine `set` Action Syntax

## Verdict

The fix is **not ready for production**. The mapping-only grammar, recursive
typed values, snapshot evaluation, atomic batch commit, explicit-null prior
state, removed-form diagnostics, active-artifact migration, and shared
Darkmatter duplicate-key rejection are implemented and have relevant Level 1
coverage. Two explicit contracts remain incomplete: authored assignment order
is discarded before result serialization, and execution-time value failures do
not carry the full semantic value path consistently across diagnostic
projections.

Human review is not required. The specification already decides both outcomes:
retain authored order and report the full source-rooted semantic value path.

## Findings

### 1. Lifecycle set discards authored assignment order before result serialization (high)

R2 requires assignment order to be retained for stable diagnostics and result
serialization, and R4 requires the side-effect result to serialize the prior
values as one object. The implementation cannot currently preserve that order.
Darkmatter converts frontmatter to the workspace's ordinary
`serde_json::Map`, and `parse_runtime_set` then iterates that map into an
`IndexMap` (`lib/src/composition/lifecycle/parse.rs:1153-1156`). Neither the
Claudine nor Darkmatter dependency graph enables `serde_json`'s
`preserve_order` feature, so the source map is key-sorted before `RuntimeSet`
receives it. The later `IndexMap` in `RuntimeSet` and `set_batch` preserves that
normalized order, not the author's YAML order (`actions.rs:263-292`,
`runtime_state.rs:157-171`).

This is user-observable for sequence `side_effect` tasks because their object
result is converted directly to text. For an authored mapping ordered `z`, then
`a`, the output is serialized as `a`, then `z`. That contradicts the explicit
ordering contract even though snapshot semantics make evaluation order
independent.

The tests do not detect the loss. The multi-key prior-value assertion compares
JSON objects structurally, where order is irrelevant, while the textual result
test serializes only one key (`sequence/task/tests.rs:2276-2301`).

Required change: preserve YAML mapping order through the shared frontmatter
representation or carry source order through a dedicated ordered parsing path
that still uses Darkmatter's single parser and duplicate-key validation. Add a
multi-key sequence side-effect test whose authored order differs from lexical
order and assert the exact output and appended `outputs` string.

### 2. Runtime set failures do not retain the required full semantic value path (high)

R5 and acceptance criterion 10 require execution-time failures to name the
source document and full semantic value path, including nested object keys and
array indexes, with the same effective identity in terminal, `err.*`, and
machine projections. `dispatch_runtime_set` currently puts only
``set.{key}{suffix}`` into free-form message text
(`lifecycle/executor.rs:1403-1409`). When the stack catches the evaluation
failure, it assigns the coarser action location
`event.stack[i].action[j]` (`executor.rs:1035-1054`). It never combines that
location with `.set.{key}{suffix}`.

The machine projection loses even the action location:
`CompositionError::LifecycleEvaluationError` writes its `property` detail as
the event name alone (`composition/error/render/mod.rs:802-806`). Sequence
side-effect dispatch is coarser still: it calls `dispatch_runtime_set` without
a task/action location and returns the resulting `LifecycleErrorInfo` directly
(`executor.rs:572-609`, `sequence/task/mod.rs:651-698`). It therefore cannot
produce the specified path shape such as
`tasks[2].side_effect.set.metadata.files[0]`.

Existing coverage proves atomic rollback for a failed expression but asserts
only that an error occurred and state stayed unchanged. The diagnostic parity
test covers parse-time shape errors, not an execution-time value failure. As a
result, the required path and projection behavior can regress—or remain absent—
while every targeted test stays green.

Required change: make runtime-set resolution failures carry a typed semantic
path separately from prose, root it at the event stack or sequence task site,
and project that same path into terminal rendering, `err.*`, and machine
detail. Add Level 1 tests for nested object/array evaluation failures in an
event stack and a sequence side-effect task, asserting source, exact path,
catalog identity, and atomic rollback.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Mapping-only syntax works in lifecycle stacks, nested action lists, task setup/teardown, and sequence side-effect tasks | Level 1 parser, executor, sequence, and hermetic CLI tests | Appropriate level and broadly green. |
| Values retain scalar/null/array/object types and resolve recursively at execution time | Level 1 parser and executor tests | Appropriate and green. |
| A mapping evaluates from one snapshot and commits all-or-nothing, with or without shared runtime state | Level 1 executor/runtime tests | Appropriate and green. |
| Prior values and sequence output retain authored assignment order | Level 1 tests compare unordered multi-key objects or one-key serialized output | **Gap:** the required observable order is neither implemented nor tested. |
| Removed forms and malformed mappings produce canonical migration guidance | Level 1 parser and diagnostic-rendering tests | Appropriate and green for parse-time failures. |
| Failed runtime expressions name the source and full semantic value path consistently across terminal, `err.*`, and machine output | Level 1 rollback-only test; no full-path projection test | **Gap:** implementation and verification stop at a coarse action/event location. |
| Duplicate YAML keys fail at every nesting level and through fallback parsing | Level 1 shared-parser tests plus full Darkmatter L1 | Appropriate and green. |
| The real shipped implementation prompt works through normal invocation without file mutation or real provider/audio side effects | Level 1 hermetic CLI test on Unix | Appropriate boundary. Cross-OS result collection remains external to readiness. |

Level 2 is not required because no requirement depends on real-terminal glyph,
width, styling, scrolling, or emulator behavior. Level 3 is not required
because no requirement depends on OS keyboard/mouse injection, terminal input
encoding, hotkeys, paste, or IME behavior.

## Verification Performed

- GitNexus index was current at `4399b3a`. Upstream impact classified
  Darkmatter's `parse_yaml_with_fallbacks` as **CRITICAL**: 46 impacted symbols,
  four processes, and eight modules. `RuntimeState::set_batch`,
  `dispatch_runtime_set`, and sequence `run_side_effect` were LOW risk.
- Fix-focused Claudine Level 1 selection: **24/24 passed**. This included
  mapping parsing, snapshot/atomic behavior, explicit null, reserved keys,
  removed forms, diagnostics, and sequence result plumbing.
- Darkmatter package-area Level 1: **7,941/7,941 passed**, with seven tests
  excluded by the canonical tier filter.
- Full Claudine Level 1 reached **1,396 passes** before fail-fast stopped on two
  failures caused by pre-existing dirty edits to `prompts/implement.md` outside
  this fix. Both failures report the unrelated `has_skill(ctx.area)` path-name
  validation error; no reviewed file was changed to address them.

No Level 2 or Level 3 run was needed for this review because those tiers do not
verify an additional boundary for the specified syntax/state behavior.
