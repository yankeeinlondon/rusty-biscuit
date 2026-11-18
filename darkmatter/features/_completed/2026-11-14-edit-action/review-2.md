---
$schema: feature-review.yaml
ready: false
findings:
  - "R2-F1 (medium): Whole-value edit operands bypass the approved scalar conversion"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-18T08:54:51-08:00
spec: 2026-11-14-edit-action/spec.md
implemented: true
description: "A **feature** review of `2026-11-14-edit-action/spec.md`"
feature: 2026-11-14-edit-action/review-2.md
previous: 2026-11-14-edit-action/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-edit-action/log.md
implemented_by: claude/opus
next: 2026-11-14-edit-action/review-3.md
---

# Edit action review 2

The feature is not ready. Both previous findings are fixed. One newly identified gap remains in the author's approved scalar conversion: a native numeric filename works, but the same number obtained from an expression fails. No author decision is needed.

This review applies the specification together with the author rulings recorded under “Review rulings” in the implementation log. The accepted timeout gap, common caller-repository write boundary, null/default rules, refusal of Claudine's unsupported `--output`, and shared process-tree ownership remain accepted. Cross-platform evidence is left to CI.

## Previous Finding Verification

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Recorded planner tests exceed the L1 time budget | R1-F1.1: documentation, full-scope, and changed-file planner cases, plus the case-list guard | verified fixed | Read the split tests and their guard. All three still feed the shipped planner's output to the public guard reader. The documentation fixture avoids an unrelated repository-wide reader scan and explicitly asserts its classification. Review package timings: 0.397, 0.498, and 0.441 s | Nothing |
| R1-F1: same finding | R1-F1.2: deletion and missing-file checks | verified fixed | Assertions still require the modified file, exclude the deleted file, and reject an unexpectedly missing listed file. Package execution: 0.424 s | Nothing |
| R1-F1: same finding | R1-F1.3–R1-F1.4: both archive-path guards | verified fixed | Both execute in the package run: 2.297 and 2.286 s | Nothing |
| R1-F1: same finding | R1-F1.5: repeated Kache initialization | verified fixed | First package run: 3.756 s. Second: 5.402 s on a loaded host; isolated clearance is recorded below | Nothing |
| R1-F1: same finding | R1-F1.6: worktree-base contract cases | verified fixed | All nine fixture shapes run separately; second package run: 0.645–0.824 s | Nothing |
| R1-F2: Sequence edit test describes an obsolete template limitation | R1-F2.1: computed sequence-root `fqn`, removal of obsolete comment, both branches and existing assertions | verified fixed | Read the exact diff and ran Claudine `just test lifecycle_edit_action`. The changed test passed in 0.666 s: one launch at the canonical computed path, saved bytes, success message, no task/provider; second invocation warns without another launch | Nothing |

All ledger children were checked. Neither repair is partial, and the new finding below is not a sibling of either previous defect class.

## Unblocked Findings

### R2-F1 (medium): Whole-value edit operands bypass the approved scalar conversion

**Authority: spec as amended by the author's “`edit` follows coercion” ruling**, recorded in [the implementation log](./implementation-log.md): “Overrides the spec's strict-string rule” and “`edit` operands go through the same conversion engine as every other lifecycle action's.” The ruling specifically asks to remove the number and Boolean refusals. This does not request a new conversion policy: it completes that existing decision.

**Defect class:** conversion is applied to authored scalar values before expression parsing, but not to the scalar values expressions produce; parser and runtime retain separate strict-string refusals.

A numeric property can plausibly name a numbered file, for example a year or issue number. It is unusual for the whole filename to be numeric, hence medium priority. This input requires no malformed file or filesystem race:

```yaml
---
number: 5
initialize:
  - edit: "{{ number }}"
  - skip
---
```

With the portable fake editor installed as `code`, `md compose --output json` and `claudine compose --goose` both exit 1 without opening it: “`edit` needs a string path, got number.” Changing only the operand to native `5` succeeds and opens file `5`. Changing it to a mixed string such as `"n-{{ number }}"` also succeeds. An explicitly constant expression, `"{{ 5 }}"`, fails earlier in both execution and `md schema validate`, although DMLS accepts it.

The cause is visible in Darkmatter's [action-shape handling](../../lib/src/lifecycle/action_shape.rs:725): `edit_file_value` converts native values, while `edit_file_problem` still rejects numeric and Boolean expression literals. Darkmatter's [executor](../../lib/src/lifecycle/executor.rs:1331), which both hosts use to run edits, then requires an evaluated `Value::String` without binding the result to the shared text parameter shape.

The sweep changed one operand at a time in the motivating create/edit/skip fixture. Every runtime/schema shape below was checked in scalar, one-item list, and long-form `file` spellings. The final service receives a resolved path, so it has no scalar-expression decision of its own.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Darkmatter short-form parser | Native number/Boolean; constant numeric/Boolean expression; property expression | Native values accepted; constant expressions refused; properties deferred | Convert supported scalar results, preserving passive deferral |
| R2-F1.2 | Darkmatter one-item positional-list parser | Same values inside one-item lists | Same split | Same conversion contract |
| R2-F1.3 | Darkmatter long-form parser | Same values under `file` | Same split | Same conversion contract |
| R2-F1.4 | Shared executor through `md compose` | Numeric/Boolean properties, all three spellings | Exit 1, no launch; native and text controls exit 0 and launch | Bind evaluated scalars as text before resolving the path |
| R2-F1.5 | Shared executor through `claudine compose --goose` | Same runtime shapes and controls | Same failure and successful controls | Same result as the shared conversion rule |
| R2-F1.6 | `BoundOperands` target projection through `md schema validate` | All three forms: native scalars, constant expressions, property expressions | Native accepted; constant number/Boolean rejected at the operand; properties deferred | Accept convertible constants; continue to defer event-time values |
| R2-F1.7 | DMLS, ordinary Darkmatter document | Constant number in scalar and long forms, constant Boolean in a one-item list, numeric property | Clean: no schema diagnostic, no editor launch or filesystem change | Keep acceptance and passive behavior; align CLI validation with it |
| R2-F1.8 | DMLS, registered Claudine document | Same protocol fixture as the preceding row | Clean: no schema diagnostic, no launch or filesystem change | Same |
| R2-F1.9 | Darkmatter lifecycle guide and error guide | Text explicitly declaring numeric/Boolean whole-value results invalid | Both describe the retained strict runtime rule | Update alongside conversion so the guides describe the author's decision |

For the same target validator, null and blank operands are refused in all three forms through both CLIs and schema validation. Text properties and mixed templates succeed through both CLIs. An array-valued expression is deferred by passive validation and refused at execution, without launching. Those controls remain valid: scalar-to-text conversion does not authorize collection targets or missing targets. The existing parser/schema matrix covers absent operands, invalid collections/elements, empty values, unknown fields, arity, and duplicate flag placement. Flag conversion is already shared and is not part of this defect.

**Recommended repair:** remove the retained numeric/Boolean expression-literal exclusions and bind the evaluated operand through the existing scalar-to-text conversion before the nonblank check and path resolution. Preserve null/collection refusals and passive checks. Extend the existing public parser/schema parity tests with native-versus-expression controls, add a runtime numeric-property case through the shared executor, and retain a small CLI wiring check. Update [the lifecycle guide](../../docs/topics/lifecycle/index.md:645) and [the error guide](../../docs/topics/lifecycle/error-handling.md:162), which currently say whole-value numeric results fail. No new escaping layer, syntax, process abstraction, or CI cell is needed.

## Blocked Findings

None.

## Recurrence

None. Review 1 reported planner-test cost and an obsolete sequence-template comment. R2-F1 concerns incomplete scalar conversion and was not part of either earlier finding. The previous review's broad acceptance of coercion did not test these whole-value results.

## Requirements and verification levels

| Requirement | Evidence examined or rerun | Assessment |
|---|---|---|
| Computed initialize workflow, external prompt, occupied-name race, create-new, skip | L1 process tests in both CLIs, including the repaired sequence-root test | Appropriate level; workflow passes |
| Operand forms, flags, located malformed-input errors, runtime/schema/tooling agreement | L1 public parser/schema tests, CLI matrix, DMLS protocol tests and temporary protocol probes | R2-F1 is the remaining conversion gap |
| File-reference resolution, containment, exclusive creation, preserved arbitrary bytes, replacement saves and post-exit checks | L1 service, filesystem and CLI tests; service and shared resolver inspection | Appropriate level |
| Shared editor precedence, requested surface, wait flags and non-blocking refusal | L1 resolver/catalog/dispatch tests and shared resolver inspection | Appropriate level |
| Eligibility, typed failures, `no_error`, execution order, cache invalidation, passive/no-launch paths | L1 service and both CLI suites | Appropriate level |
| Terminal input, blocking handoff and return to the next action | L2 detached-tmux tests in both CLIs | Both tests executed in each CLI; no window focus |
| Stream routing and structured output | L1 recorder/envelope tests; L2 terminal handoff | Appropriate level; Claudine's output-flag requirement remains superseded |
| Cancellation, process-tree termination/escalation, preserved saves and no continuation | L1 launcher and Claudine process tests; L2 interrupt tests in both CLIs | Appropriate level; accepted timeout gap not reopened |
| Terminal/foreground restoration on success, failure and cancellation | L1 controlling-PTY launcher test for all outcomes; L2 success and interruption | Appropriate level; no new terminal key-encoding behavior requires L3 |
| Passive completion/hover/diagnostics and artifact wiring | L1 DMLS tests; declared target and CI-feature inspection | Tests are compiled and selected; no new artifact change in this repair |

This work adds an action grammar, not a new file-format loader. No new document-loader matrix is required. Invalid target/flag shapes produce errors without an editor launch; exceptions are the approved scalar conversions and absent/null flags taking defaults. Target and flag shape validators are covered through public lifecycle parsing and schema validation; null flags deliberately equal omission under the author's ruling. The smell search found no silently filtered target collection: the `.ok()` in `edit_target_problem` leaves parse failures to the ordinary parser/schema checks, while the flags use the shared binder. R2-F1 follows the target value through the later runtime decision and passive projections rather than stopping at the reader.

## Verification performed

- Root `just test test-toolkit`: 569 passed, four skipped, none at five seconds in the first run. A second package run retained the repaired planner timings listed above; its one loaded-host slow test is addressed below.
- Darkmatter `just test edit_action`: 46 passed, no slow tests.
- Darkmatter `just test editor`: 334 passed, no slow tests; includes service, launcher and DMLS coverage.
- Darkmatter `just test lifecycle_edit_schema_parity`: five passed, no slow tests.
- Claudine `just test lifecycle_edit_action`: 77 passed, no slow tests.
- Darkmatter `just test-l2 level2_edit_action`: both CLI tests passed, 0.990 and 1.024 s. Other package legs selected no matching tests.
- Claudine `just test-l2 level2_edit_action`: both CLI tests passed, 1.063 and 1.083 s; the generator leg selected no matching tests.
- Direct CLI sweep: eleven operand shapes × three forms through `md compose`, `md schema validate`, and `claudine compose`; fixture-owned home, directory and fake editor, with launch-record checks. Results are summarized under R2-F1.
- Temporary DMLS probes in the already declared `edit_action_editor` test module checked both host profiles through the existing protocol fixture. Both passed, 0.188 and 0.580 s; recorder/tree assertions proved passivity. An earlier exploratory assertion expecting CLI-style rejection failed because both DMLS profiles accepted the constants. The probes were removed and the original source restored; their observed acceptance is the clean part of the sweep.
- Isolated nextest run of `ensure_is_idempotent_on_a_second_run`: 2.462 s. The load averages at measurement were 24.87, 28.54 and 32.26 on the 16-core host. This clears its second package reading of 5.402 s under the loaded-host rule; the first complete package run was already below five seconds throughout.
- The saved review passed `md schema validate`; `git diff --check` found no whitespace errors in the feature records. No production source was changed by this review.

## Implementation record corrections

Corrected repeated current-tense claims in the implementation log about the old document-directory write boundary, native scalar/flag refusals, and `non-null` markers. Dated notes explain the historical behavior, current behavior, and source/test evidence. The author rulings and specification prose were preserved. These corrections are not findings or requests for human review.
