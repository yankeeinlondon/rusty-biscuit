---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-18T09:28:01-08:00
spec: 2026-11-14-edit-action/spec.md
implemented: false
description: "A **feature** review of `2026-11-14-edit-action/spec.md`"
feature: 2026-11-14-edit-action/review-3.md
previous: 2026-11-14-edit-action/review-2.md
---

# Edit action review 3

The feature is production ready under the specification and the author's
recorded amendments. R2-F1 is verified fixed in every required instance.
The earlier planner-test and sequence-fixture repairs remain fixed. There
are no new findings, partial repairs, blocked findings, or decisions requiring
the author.

This review applies the author decisions in the implementation log's
“Review rulings”: shared scalar conversion and null/default handling, the
caller-repository write boundary, shared process-tree ownership, the accepted
absence of a timeout covering an open editor, and refusal of Claudine's
unsupported `--output` flag. These decisions supersede conflicting original
specification text and are not reopened. Cross-OS evidence remains CI's work.

## Previous Finding Verification

The required instances below come from review 2 and the repair's
`### Finding Ledger` in [log.md](./log.md). The implementation was checked
independently of its completion claims. The table also reconfirms both
findings already closed by review 2.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1: Whole-value edit operands bypass the approved scalar conversion | R2-F1.1: short-form parser | verified fixed | `edit_file_problem` now rejects blank literals without rejecting numeric/Boolean expressions. Public parser/schema accepted-cell tests pass for native values, constants, and deferred properties. Temporary CLI sweep opens the expected numeric/Boolean filenames | Nothing |
| R2-F1: same finding | R2-F1.2: one-item positional-list parser | verified fixed | Same shared validator; parity cases accept numeric/Boolean constants and property expressions inside the list. CLI sweep confirms all six native/constant/property controls launch once | Nothing |
| R2-F1: same finding | R2-F1.3: long-form `file` parser | verified fixed | Same shared validator; parity cases accept constants and property expressions under `file`. CLI sweep confirms all six native/constant/property controls launch once | Nothing |
| R2-F1: same finding | R2-F1.4: shared executor through `md compose` | verified fixed | `dispatch_edit` evaluates once, then calls `edit_file_value` before the nonblank check and path resolution. Six new executor tests pass, including number, Boolean, fraction, constant, list, and long form. Permanent numeric-property CLI test and 18 temporary CLI controls pass with exact target and saved-byte assertions | Nothing |
| R2-F1: same finding | R2-F1.5: shared executor through `claudine compose --goose` | verified fixed | Claudine reaches the shared executor; no separate target-type decision exists. `compose_opens_the_file_a_numeric_property_spells` passes, asserting one launch at `5`, saved bytes, and no provider. All 78 lifecycle edit tests pass | Nothing |
| R2-F1: same finding | R2-F1.6: `BoundOperands` through schema validation | verified fixed | `edit_target_problem` delegates to the parser's conversion and blank check. Six public parity tests pass, including all three forms, both constant types, properties, malformed shapes, located errors, and completed/authored-value parity. Permanent CLI wiring test also runs `md schema validate` successfully | Nothing |
| R2-F1: same finding | R2-F1.7: ordinary Darkmatter DMLS document | verified fixed | Temporary protocol test sends constant numeric/Boolean targets in all three forms plus a numeric property. No schema diagnostic, recorder launch, or repository-file change. Existing malformed-input, completion, hover, and passivity tests pass | Nothing |
| R2-F1: same finding | R2-F1.8: registered Claudine DMLS document | verified fixed | Same protocol probe under the Claudine host profile passes with no schema diagnostic, launch, or file change. Existing registered-host tests also pass | Nothing |
| R2-F1: same finding | R2-F1.9: lifecycle and error guides | verified fixed | Read the corrected Darkmatter target rules and unsuppressible-failure section: numeric/Boolean results convert; null, collections, mappings, and blank text fail. Claudine's guide, the canonical caller-skill source, and repo lifecycle guidance describe the same conversion | Nothing |
| R2-F1: same finding | R2-F1.10: Claudine-side target decision, added by repair sweep | verified fixed | Claudine supplies the editor service and delegates parsing/execution to Darkmatter; there is no independent strict scalar refusal to repair | Nothing |
| R2-F1: same finding | R2-F1.11: `prep_target`, added by repair sweep | verified fixed | Separate action; it consistently refuses native and expression numeric/Boolean operands. The edit-specific conversion repair does not change its contract | Nothing |
| R1-F1: Recorded planner tests exceed the L1 time budget | R1-F1.1: three planner cases and case-list guard; R1-F1.2: deletion/missing-file assertions | verified fixed | Read the split case macro, list guard, documentation-classification assertion, and retained deletion/missing-file assertions. Fresh package run: documentation 0.202 s, full scope 0.244 s, changed file 0.222 s, deletion 0.216 s, guard 0.007 s | Nothing |
| R1-F1: same finding | R1-F1.3–R1-F1.6: archive guards, repeated Kache initialization, nine worktree-base cases | verified fixed | Fresh test-toolkit package run: archive guards 1.197/1.199 s; initialization 2.941 s; all nine base cases 0.373–0.505 s. Whole package has no test at five seconds | Nothing |
| R1-F2: Sequence edit test describes an obsolete template limitation | R1-F2.1: computed sequence-root target and retained workflow assertions | verified fixed | Read the computed `fqn`, both branches, launch/target/saved-byte/message assertions, and provider/task exclusions. Fresh Claudine run passes this test in 0.617 s; obsolete comment remains removed | Nothing |

R2-F1 is a completed repair, not a newly found sibling. No earlier finding
remains open, and no blocked prerequisite needs a new decision.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None. No new defect class or missed sibling was found. The scalar-conversion
class from review 2 is closed across its parser, runtime, schema, editor-tooling,
and documentation sites.

## Requirements and verification levels

The review checked the full action contract as amended, including the
existing implementation beyond the latest conversion repair. The service
resolves the editor and checks eligibility before creation, reserves missing
targets exclusively, launches with a separate absolute path argument, and
rechecks containment and regular-file status after exit. The executor retains
typed dispatch failures, unsuppressible evaluation/cancellation failures,
and cache invalidation after an attempted side effect.

| User-facing requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Computed initialize workflow, prompt outside launch directory, occupied-name handling, create-new, skip without provider | L1 portable fake-editor CLI tests in both hosts; repaired sequence-root workflow rerun | Appropriate; passes |
| Scalar, one-item list, long form, flags, arity, unknown keys, located errors, branch-independent passive checks | L1 public parser/schema matrix, completed/authored parity, both CLI suites, DMLS protocol | Appropriate; approved conversion now agrees across surfaces |
| Local reference forms, containment, exclusive creation, existing bytes, Unicode/metacharacter paths, replacement saves, nonregular/deleted targets | L1 service, filesystem, shared-executor and CLI tests; service/resolver inspection | Appropriate; no editor shell command or content rewriting |
| Shared editor precedence, surface matching, blocking metadata, non-blocking editor exclusion | L1 resolver/catalog and shared-launch tests; resolver inspection | Appropriate; action uses shared discovery |
| Noninteractive/parallel/output eligibility, typed failures, `no_error`, ordering, cancellation cleanup, post-edit reads | L1 service and both CLI suites | Appropriate; invalid target results remain unsuppressible and never launch |
| Terminal editor input, blocking wait, return to the next action | L2 detached-tmux tests in both CLIs | Executed and passed without window focus |
| Markdown output isolation and one valid structured result | L1 CLI envelope/stream tests, additional structured-output scalar sweep, L2 terminal handoff | Appropriate; terminal refusal and GUI output routing retained |
| Cancellation stops/reaps the owned tree, escalates for ignored termination, preserves saved data, never resumes even with `no_error` | L1 launcher and Claudine process tests, L2 interrupt tests in both CLIs | Appropriate; author-accepted timeout omission remains accepted |
| Terminal settings and foreground restoration on success, failure, cancellation | L1 controlling-PTY launcher test plus L2 normal completion and interruption | Appropriate; this feature adds no terminal key-encoding behavior requiring L3 |
| Passive completion, hover, diagnostics, no launch or writes | L1 DMLS protocol tests under both host profiles | Appropriate; repaired expressions checked directly |
| Test inclusion and tier selection | Explicit test targets, `main.rs` module declarations, CI feature declarations, actual L1/L2 listings | Relevant tests compile and run in their declared tiers |

The change concerns lifecycle action syntax and operand conversion, not a new
file-format/configuration loader. The Input Robustness Matrix's loader rules
therefore do not add a document-loader requirement. Existing public parity
tests cover missing/null/blank targets, collections and element shapes,
unknown fields, flags, and duplicate flag placement. Invalid targets fail
without a launch. Approved exceptions are numeric/Boolean scalar conversion
and absent/null flags taking defaults. The shared target helper's `.ok()`
defers expression parse errors to the ordinary parser/schema checks; it does
not silently filter target collections or default a failed target to empty.

## Verification performed

All runs used the canonical nextest-backed recipes. No measured L1 test
reached five seconds.

- Darkmatter `just test edit_action`: 47 passed.
- Darkmatter `just test lifecycle::executor::tests`: 59 passed.
- Darkmatter `just test lifecycle_edit_schema_parity`: six passed.
- Darkmatter `just test editor`: 334 passed, including launcher, service,
  resolver, DMLS malformed-input and passive-tooling checks.
- Claudine `just test lifecycle_edit_action`: 78 passed.
- Root `just test test-toolkit`: 569 passed, four skipped; all required
  planner/guard/Kache instances ran below five seconds.
- Darkmatter `just test-l2 level2_edit_action`: both CLI tests passed,
  0.898 and 0.908 s. Other package legs selected no matching tests.
- Claudine `just test-l2 level2_edit_action`: both CLI tests passed,
  0.999 and 1.033 s. Generator leg selected no matching tests.
- Temporary DMLS probes in the existing declared module: both host profiles
  passed (0.085 and 0.290 s), with recorder and before/after file-tree checks.
- Temporary CLI sweep in the existing declared module: 18 tests passed.
  Number/Boolean × native/constant/property × short/list/long form each
  produced a successful structured-output invocation, empty stderr, one launch
  at the expected path, and exact saved bytes. An initial probe mistakenly
  placed the prompt outside the fixture's writable directory and correctly
  hit containment refusal. Moving that probe into the fixture's `cwd`
  corrected the setup; all 18 controls then passed.

Both temporary probe additions were removed, restoring the exact original
test-file bytes. Production code was not changed by this review. The
implementer's broader package runs and their individually cleared loaded-host
timings were examined; this review did not repeat unrelated full suites or
cross-OS gates. The saved review passes `md schema validate`, and
`git diff --check` reports no whitespace errors in the feature records.

## Implementation record corrections

Corrected stale target/flag-refusal statements in the implementation log's
phase summaries, handoff message, and earlier correction notes. Dated notes
state the old claims, current conversion behavior, and verification. Author
rulings and specification prose were preserved. These factual corrections
are not findings or human-review items.
