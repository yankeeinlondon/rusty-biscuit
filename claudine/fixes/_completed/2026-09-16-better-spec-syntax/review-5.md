---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T19:53:22-07:00
spec: 2026-09-16-better-spec-syntax/spec.md
implemented: false
description: A **fix** review of `2026-09-16-better-spec-syntax/spec.md`
fix: 2026-09-16-better-spec-syntax/review-5.md
previous: 2026-09-16-better-spec-syntax/review-4.md
---

# Review 5: Make Key-to-Value Mappings the Only Claudine `set` Action Syntax

## Verdict

The fix is **ready for production**. Both Review 4 findings are implemented in
the final working tree, their regressions discriminate the previously broken
paths, and this review found no new functional, diagnostic, performance, or
test-coverage gap attributable to the specification.

Human review is not required. The implementation follows the already-ratified
mapping syntax, snapshot semantics, diagnostic paths, and authored-order
contract without introducing a new design decision.

## Prior Review Closure

### Unblocked Findings

1. **Primary `side_effect` set parse failures report a synthetic,
   task-unrooted path with no excerpt: implemented.**
   `TaskExecution::run_side_effect` now parses at the task's source-rooted
   `diagnostic.action_property`, enriches parse failures from the owning
   document, and `parse_single_action_with_order` removes the synthetic
   `action[0]` segment used internally by the shared stack parser. The three
   new Level 1 cases cover a top-level sequence task, an inline group member,
   and an external `kind: task` document. They assert the exact source and
   semantic property, projection parity, excerpt behavior, and absence of
   mutation/output.
2. **Authored order is not consistently applied before validation or in
   JSON/JSON5 task documents: implemented.** `RuntimeSet::new_with_order`
   restores the authored entries before destination-key validation. A
   format-neutral `MappingOrders` deserializer records JSON and JSON5 object
   order at the same JSON pointers used for YAML, and sequence loading obtains
   that index through the typed `Json5::deserialize_raw` boundary. The new
   Level 1 cases distinguish authored from canonical order for invalid YAML
   keys, JSON tasks, JSON5 tasks, and JSON group members.

### Blocked Findings

Review 4 contained no blocked findings, so none could have become unblocked
before this implementation.

## Findings

No findings.

## Requirement Verification Levels

| User-facing or contract requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Mapping-only syntax works in event stacks, nested action lists, task setup/teardown, and sequence side-effect tasks | Level 1 parser, executor, sequence, and hermetic CLI tests | Appropriate. |
| Native scalar/null/array/object values and recursive execution-time interpolation keep their types | Level 1 parser and executor tests | Appropriate. |
| One mapping reads one snapshot and commits atomically, with and without shared runtime state | Level 1 executor/runtime tests | Appropriate. |
| Prior-value objects and textual sequence output keep authored assignment order across YAML, JSON, and JSON5 task/group sources | Level 1 exact-output tests | Appropriate. |
| Removed forms, malformed mappings, reserved keys, and false guards produce canonical behavior without partial writes | Level 1 parser, executor, and diagnostic tests | Appropriate. |
| Failures name the owning source and full semantic value path consistently in terminal, `err.*`, and machine projections | Level 1 event, primary side-effect, setup, and teardown tests | Appropriate. |
| Frontmatter excerpts are attached when the owning source can locate them and omitted for data documents | Level 1 TTY-policy and task diagnostic tests | Appropriate. |
| Duplicate YAML keys fail at every nesting level and through fallback parsing | Level 1 shared-parser tests | Appropriate. |
| The shipped implementation prompt exercises the mapping through normal CLI invocation without provider or audio side effects | Level 1 hermetic CLI regression | Correct level. The test is currently blocked by unrelated committed `has_skill(ctx.area)` prompt behavior; the failure occurs after this fix's initialization mapping succeeds and before provider launch. |
| Loop-control `set(...)`, Darkmatter's positional API, state isolation, other verbs, and unrelated serializers remain unchanged | Level 1 unit and integration tests | Appropriate. |

Level 2 is not required because no requirement depends on rendering through a
real terminal emulator. Level 3 is not required because no requirement depends
on OS keyboard/mouse injection or terminal input encoding.

## Verification Performed

- Read the specification, Review 4, the cycle-4 implementation log, commits
  `86ae7eebc`, `8c8b2eb8b`, and `6d7c63041`, and the final uncommitted delta
  that replaces the intermediate raw `json_five` re-export with the typed
  `Json5::deserialize_raw` API.
- Traced the current source paths through `RuntimeSet::new_with_order`,
  `MappingOrders`, `sequence::data::load_document`,
  `parse_single_action_with_order`, and `TaskExecution::run_side_effect`.
- Bound GitNexus to `better-static-analysis`. Its index was 24 commits stale;
  the required `just gitnexus` refresh waited ten minutes for the existing
  analyzer process (`pid 78755`) and exited on the repository lock timeout.
  Stale graph results were not treated as evidence; the current source and
  tests were inspected directly.
- Focused Claudine Level 1 slice: 150 tests run, 150 passed.
- Darkmatter mapping-order slice: 5 tests run, 5 passed.
- Biscuit-file raw-deserialization regression: 1 test run, 1 passed.
- `just lint` passed in `claudine`, `darkmatter`, and `biscuit-file`.
- Full Claudine Level 1 gate: 7,308 tests ran; 7,298 passed, 10 failed, and 9
  were skipped. Nine failures are the independently changed shipped
  `implement-plan.md` prompt/fixture (`has_skill(ctx.area)` and fixture drift),
  and one is the independently moved `prompts/cross-platform.md`. The focused
  rerun of the real-artifact regression confirms it reaches the unrelated
  `has_skill(ctx.area)` condition after initialization and fails there.
- No Level 2 or Level 3 run was performed because neither tier observes a
  boundary changed by this specification. No formatting command or Git commit
  was performed.

## Production Readiness Closure

The canonical mapping grammar, typed values, snapshot/all-or-nothing mutation,
prior-value result, authored ordering, migration diagnostics, and owning-source
diagnostics are implemented and have verification at the appropriate level.
There are no remaining findings for this specification.
