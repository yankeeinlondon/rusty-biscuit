---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - "For later author review: passing the current document into the shared masking helper keeps all three body-step paths consistent; retain this internal interface."
    - "For later author review: authored frontmatter plus existing overrides provide the event inventory; retain this approach while lifecycle stacks have no additional inheritance source."
    - "For later author review: accept the recorded debug-build comparison with temporary schema-load instrumentation as the required performance evidence; permanent instrumentation was unnecessary."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-29T13:00:04-07:00
spec: 2026-10-22-sequence-step-schema-reload/spec.md
implemented: false
description: "A **fix** review of `2026-10-22-sequence-step-schema-reload/spec.md`"
fix: 2026-10-22-sequence-step-schema-reload/review-1.md
---

# Review 1: Sequence step schema reload

The fix is production ready. No findings require repair, and no decision requires a human ruling before the cycle can finish.

## Findings

None.

## Scope and behavior

Reviewed implementation commit `6acf5ed9c` and the current versions of its shared helper and three callers. The current helper obtains event names from Claudine's lifecycle registry, skips `loop`, and adds an empty stack only when the source or existing overrides contain that event. The frontmatter and override lookups borrow their maps; they do not copy the document for each event.

The complete set of changed callers was checked:

| Site in claudine-cli | Source supplied to masking | Result of inspection |
| --- | --- | --- |
| [Normal sequence step](../../cli/src/commands/wrap/sequence/iterate.rs) | The document reloaded at the step boundary | Body steps mask the live document's events; executable steps keep their separate task behavior. |
| [Prompt-task validation](../../cli/src/commands/wrap/sequence/task_run.rs) | The document resolved for this request | Masking runs only for a body-only request, before preparation. |
| [Prompt-task execution](../../cli/src/commands/wrap/sequence/task_run.rs) | The document resolved again at execution time | Masking runs only for a body-only request, before composition. |

All three use [the same helper](../../cli/src/commands/wrap/sequence/container.rs). No sibling still inserts every event unconditionally. Present events retain the previous empty-stack behavior; absent events no longer create schema-trigger inputs. The helper's documentation explains that distinction and matches the code. The current [sequence documentation](../../docs/topics/flow-control/sequences.md#container-lifecycle-and-recovery) already states that body-only steps do not enter another lifecycle.

## Acceptance and verification level

| Requirement | Evidence | Appropriate level |
| --- | --- | --- |
| A1: absent events add no override; declared and supplied events are masked; `loop` is preserved | `only_present_events_are_masked` checks an event-free source, a declared `success`, an override-carried `finalize`, and a declared `loop`. Fresh review run passed. | Level 1, in-process. |
| A2: root `success` and `finalize` run once | `sequence_containers::body_only_steps_run_root_success_once` asserts one success, one cleanup, and two provider results through the normal CLI with a fake provider. The implementation log records a pass. | Level 1, hermetic subprocess. |
| A3: the three-item dry run makes one cold schema load | The implementation log records temporary `load_effective_schema` instrumentation: two cold loads before, one after, with total debug-build time decreasing from 1.95–2.42 seconds to 1.18–1.59 seconds. All three bodies composed in both builds. | Level 1 process measurement, as explicitly requested by the spec. |
| A4: claudine L1 suites pass | The implementation log records `just test`: 10,626 passing tests, plus 4,721 CLI L1 passes on Linux and clean Clippy. These are implementation-time results, not a fresh full-area run by this review. | Level 1. |

The new unit test is compiled in the CLI binary's test target. The existing lifecycle regression is declared by `tests/l1/main.rs`, which is the explicit `l1` target in `Cargo.toml`. Neither test name contains a higher-tier marker. None of this fix's requirements concerns terminal rendering or keyboard encoding, so Level 2 or Level 3 evidence is unnecessary.

The fix changes a presence-based override transformation, not a file loader or field validator. It introduces no new accepted file syntax or value coercion. Every present non-loop event is deliberately masked regardless of its original stack value; absent events remain absent. The changed helper contains none of the permissive parse patterns listed in the robustness instructions. An additional loader/validator shape matrix would test unchanged parsing outside this fix.

## Implementation decisions (informational)

Two agents independently read the specification and implementation log in parallel. The log contains neither a `decisions_for_review` list nor a “Decided during implementation” heading. Neither reader found an author-flagged decision or deferred work. The spec supplies no additional autonomy provision; the choices below are internal means of satisfying its stated behavior.

1. **Pass the current document to the shared helper and update all three callers.** This keeps the event-presence rule in one place and ensures a reread can change which events are masked. Recommended; it is internal and reversible.
2. **Use authored frontmatter plus existing overrides as the event inventory.** These are the inputs layered into step preparation. The emitter does not introduce additional frontmatter stacks. Recommended; no public or persisted format changes.
3. **Measure with temporary instrumentation in a debug build.** The implementer used `claudine sequence --goose --dry-run --yolo` on three items, recorded before/after cold-load counts and elapsed ranges, then removed the instrumentation. This directly answers A3 without adding a permanent measurement mechanism. Recommended.

These items are reported for later author review and do not block readiness. No work was deferred, no scope was transferred, and no human-only verification is required.
