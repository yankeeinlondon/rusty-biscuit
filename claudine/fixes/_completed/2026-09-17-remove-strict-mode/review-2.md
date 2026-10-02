---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-17-remove-strict-mode/spec.md`
fix: 2026-09-17-remove-strict-mode/review-2.md
spec: 2026-09-17-remove-strict-mode/spec.md
previous: 2026-09-17-remove-strict-mode/review-1.md
created: 2026-10-01T19:19:43-07:00
reviewed_by: codex/gpt-6.1-sol
implemented: false
ready: true
human_review: false
has_blocked_findings: false
blocked: false
recurrence: false
findings: []
---

# Review 2: Remove Strict Mode

The fix is **production ready within the scope approved on October 1**. All
four findings from review #1 are addressed. No new in-scope defect, missing
requirement, or material performance issue was found.

The specification and design explicitly park the larger schema platform,
editor lifecycle descriptors, and schema-location restrictions. Their historical
requirements are not incomplete work in this fix. The implementation preserves
Claudine's existing initialization shell prohibition and records its approved
binding API departures in the implementation log.

## Previous findings

Review #1 puts all four findings under `Findings`; it has no separate
`Unblocked Findings` or `Blocked Findings` section. All four are actionable,
and none required a human decision to become unblocked.

| Previous finding | Implementation checked | Verification and result |
| --- | --- | --- |
| High: lifecycle triple-brace escapes were evaluated | Darkmatter now skips excluded keys in both first-pass literal handling loops, leaving the authored escape for event-time evaluation. | The real `claudine compose` L1 tests in [lifecycle_literal_escapes.rs](../../cli/tests/l1/lifecycle_literal_escapes.rs) pass for mixed text, whole values, `set` read-back, top-level event fields, proxy overlays, and approved shell bytes. |
| Medium: the lifecycle topic taught triple braces as evaluation syntax | The five contradictory passages now use double braces for evaluation. The use-claudine skill was corrected too. | The [lifecycle topic](../../docs/topics/flow-control/lifecycle.md) consistently describes triple braces as literal syntax. Searches of current topic docs and skill references found no sibling teaching that triple braces inject a value. |
| Low: sequence dotted paths and the whole document disagreed | Claudine's [SourceExpressionLookup](../../lib/src/composition/sequence/expr.rs), used for sequence source and per-item template expressions, chooses the document layer by the top-level key. | `item_layer_replaces_top_level_keys_for_every_spelling` passes: `config.b`, `doc.config.b`, and indexed spellings all return `null` when the item replaces `config`; item and frontmatter-only values also agree. |
| Low: task-value error causes lacked public-outcome coverage | Claudine's task-value error now carries `LifecycleCause`, and diagnostic projection preserves it on `TaskOutcome.error.info.cause`. This fixes the actual loss discovered during implementation, beyond merely adding a wrapper test. | `a_task_value_failure_keeps_the_typed_darkmatter_cause` passes for prompt parameters, group variables, and timeout. It inspects the typed error through the public outcome and source chain, and proves neither shell nor provider starts. |

## Class sweeps

No failing instance remains in these sweeps. The tables include clean siblings
so the scope of each check is explicit.

### Escape handling across deferred evaluation

| Site | Shape checked | Observed and expected result |
| --- | --- | --- |
| Darkmatter excluded-key first pass | Triple-brace escape under an excluded key | Authored triple braces survive unchanged; the new unit regression passes. |
| Lifecycle messages and action operands | All three escape forms, whole and mixed strings | Literal template text reaches `stderr`; no property substitution. The CLI regression passes through preparation and execution. |
| Lifecycle `set` and read-back | All three forms stored, then interpolated through a document property | The stored text remains data and prints literally; CLI regression passes. |
| Top-level lifecycle communication | All three forms in `start.stderr` | Literal text; CLI regression passes. |
| Proxy overlay | Triple-brace whole and mixed values | Target receives literal data; CLI regression passes. |
| Lifecycle shell approval and execution | Triple braces in shell command text | Executed bytes contain literal double braces; Unix CLI regression passes. |
| Ordinary Darkmatter body and shared lifecycle/sequence interpolation | Triple braces and both backslash forms, whole and mixed strings | Ordinary escape semantics; Darkmatter escape tests and the shared conformance table pass. |
| Direct/inline preparation and sequence preflight | Deferred lifecycle keys | Source inspection confirms use of the shared excluded-key compose implementation. |

Backslash escapes remain backslash-prefixed data after interpolation; inline
Markdown rendering on `stderr` removes the backslash. Review #1's assertion
that every form has identical raw output was too broad. The updated tests and
docs correctly distinguish interpolation from rendering without changing the
language contract.

The loop action renderer's previously recorded triple-brace limitation remains
an explicit non-goal. It is documented rather than silently claimed as fixed.

### Layered document resolution

| Site | Shape checked | Observed and expected result |
| --- | --- | --- |
| Sequence source/per-item lookup | Item `config: {a: 1}` over document `config: {b: 2}`; dotted and indexed reads | Whole-key replacement for every spelling; regression passes. |
| Loop lookup and its size wrapper | Loop bindings over frontmatter; explicit document access | Loop bindings retain their own scope; `doc` uses the document map. Source inspection and lookup/conformance tests are clean. |
| Effective state, resolving wrapper, frontmatter seed, shortcut lookup | Bare missing property with a populated context namespace | No context fallback; the production lookup parity inventory and probes pass. These document surfaces read one assembled map. |
| Evaluation session and deferred lookup wrapper | Available, unavailable, and missing bindings | Resolution preserves the richer binding result; unavailable globals never fall through to document data. Binding and parity tests pass. |
| Hook event lookups | Event metadata and condition operands | Separate event policy, no implicit document-to-context fallback; dispatch tests pass. |

### Typed errors at the library boundary

| Consumer | Public result checked | Result |
| --- | --- | --- |
| Lifecycle messages, `set`, and proxy overlays | Lifecycle outcome and contextual error wrappers | Typed Darkmatter cause retained; library contract test passes. |
| Lifecycle shell approval | `CompositionError::LifecycleShellResolution` | Typed source retained; preparation tests pass. |
| Sequence shell approval | `SequenceShellLateBinding` and `SequenceShellResolution` | Typed binding/interpolation source retained; byte-parity tests pass. |
| Sequence task parameters, group variables, timeout | `TaskOutcome.error.info.cause` | Typed interpolation and unknown-function cause retained; new public-outcome regression passes. |
| Sequence source and template expressions | `SequenceExpressionFailed` | Typed preparation/evaluation cause retained; source-layer tests pass. |
| Task setup, teardown, and side effects | Shared lifecycle executor error path | Same retained lifecycle cause; task tests pass. |
| Loop expressions and actions | Typed loop error variants | Separate loop context retains typed parse/evaluation causes; loop tests pass. |

Some existing boxed error fields expose `Box<ErrorType>` in the standard source
chain. They remain inspectable as typed errors; the new task outcome also
provides `expression_error()`. This is not a loss of the cause or a new
readiness blocker.

## Requirement coverage and test levels

| Requirement | Strongest relevant verification |
| --- | --- |
| Missing properties are `null`; whole-value typing, mixed text, ternaries and defaults agree | L1 public compose/subtree contract tests and lifecycle conformance tests. |
| Bare properties never fall back to context; globals shadow document properties | L1 lookup inventory/probes and binding resolution tests. |
| Invalid registrations fail before providers run; lazy values keep their session lifetime | L1 registration, association, and lazy-cache contract tests. |
| Definitely unavailable globals fail passive validation even in inactive branches; runtime-dependent scope remains deferred | L1 matrix across all eleven lifecycle scopes, passive validation and runtime resolution. |
| Parse/evaluation failures prevent the affected effect; `set` and proxy batches remain atomic | L1 lifecycle error/atomicity tests and task failure tests. |
| Required schema errors and initialization shell restrictions still block execution | L1 real CLI schema tests and initialization/recovery regressions. |
| Shell approval fixes the exact execution bytes, including task setup/teardown | L1 approval and byte-parity tests; existing lifecycle dispatch L2 coverage also exercised. |
| The shipped implementation router reaches its authored error with absent optional inputs | L1 shipped-prompt CLI regression, route/hash drift checks, private home and audio spool. |
| Editor diagnostics are advisory for undeclared properties, errors for unknown functions, with shared classification | L1 DMLS diagnostics, hover/completion, corpus and session tests. |
| Escapes survive the real document pipeline | L1 real CLI regression, not just manufactured executor JSON. |

These requirements concern expression evaluation, effects, and editor results.
They introduce no keyboard, mouse, terminal color, or layout contract requiring
L2 or L3 proof. No requirement is supported only by an inappropriate test level.

The new CLI test file is declared by `tests/l1/main.rs`, compiled by the
manifest's `l1` target, and selected in the successful L1 run. The added library
tests likewise ran in L1. The optional area-wide `check-tier-coverage` command
was stopped when it began compiling unrelated nested Rendezvous/DuckDB targets;
its completion is not claimed as evidence.

The Input Robustness Matrix does not apply: this fix changes expression binding
and frontmatter value transformation, not a file-format reader or configuration
deserializer. The new handling does not change accepted field shapes, duplicate
keys, or trailing-content rules.

## Verification

| Command | Result |
| --- | --- |
| `just test` in `claudine` | 8,075 passed; 9 skipped. |
| `just test` in `darkmatter` | 8,775 passed; 12 skipped, including DMLS coverage. |
| `just lint` in both areas | Both exited 0. |
| `just test-l2 level2_lifecycle_dispatch` in `claudine` | All 11 selected CLI tests passed in detached tmux; the generator had zero matching tests. Other tests were excluded by the filter. |

The first L2 run also required tmux backend proof: all 11 CLI tests ran with
`run=11, skip=0, panic=0`. The area recipe then returned an error because it
applied that requirement to the generator, where the filter selected zero
tests. Repeating the canonical recipe without that override again passed all
11 CLI tests. The empty generator selection is not missing lifecycle coverage
and is unrelated to this fix.

Skipped tests are not counted as passing evidence. No production code was
changed during this review. Cross-OS results remain the CI process's
responsibility and do not determine this review's readiness flag.

## Unblocked Findings

None.

## Blocked Findings

None. No additional human design decision is needed for the approved scope.

## Recurrence

No finding recurs from review #1, the only earlier implementation review.
Earlier design reviews concern design checkpoints and the broader historical
scope; no new finding in this review repeats one of their defect classes.
