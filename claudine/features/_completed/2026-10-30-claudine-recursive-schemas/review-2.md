---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (medium): Literal-token arguments are incorrectly deferred after decoding"
    - "R2-F2 (low): Expanded schema tests exceed the L1 time budget"
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-24T06:57:29-07:00"
spec: 2026-10-30-claudine-recursive-schemas/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-30-claudine-recursive-schemas/log.md
implemented_by: claude/opus
description: "A **feature** review of `2026-10-30-claudine-recursive-schemas/spec.md`"
feature: 2026-10-30-claudine-recursive-schemas/review-2.md
previous: 2026-10-30-claudine-recursive-schemas/review-1.md
next: 2026-10-30-claudine-recursive-schemas/review-3.md
---

# Review 2: Claudine recursive schemas

The feature is **not production ready**. The four previous findings' required
instances are repaired. This review found a separate argument-validation
regression and two test-duration overruns, grouped into two findings. Neither
requires a human decision.

## Previous Finding Verification

The required instances below include the repair log's additional child rows,
not just the original review's examples. The log's completion claims were
checked against source, declared test targets, assertions, and the selected
runs listed below.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — Passive argument validation omits runtime registration rules | R1-F1.1–R1-F1.11: numeric recovery in positional/named forms; boolean source exclusion in scalar/positional/named forms; ordinary controls; overload/variadic inventory; synthetic ambiguity; CLI, DMLS, launch prevalidation, event binding; authored boolean versus converted text | verified fixed | Darkmatter's `lifecycle_generated_operands` tests now bind through runtime registrations, including the subject/fallback matrix and instrumented ambiguous-overload test. The CLI registration-rule test, editor corpus, Claudine passive-parity cases, and event-binding tests passed. `BoundOperands` reads authored arguments before schema conversion. | None of these required instances remains. R2-F1 is a newly exposed provenance problem in that integration, rather than an unchanged boolean/recovery instance. |
| R1-F2 — Single-action dictionary long forms are rejected | R1-F2.1–R1-F2.13: ordinary/initialize/cancelled roots and both branches; loop loose action and gate; short/list controls and bundle rejection; four container paths; CLI/editor/launch; companion keys; md profile; schema-about layout; conditional-root rejection | verified fixed | The three families now use an action-only `entry` alternative. Category key counts exclude `when` and `no_error`; `loop-single` preserves one long form. Schema/parser tests cover every event and position, nested container tests cover all four paths, and CLI/editor/launch parity tests pass. The md-profile dictionary test passed in the lifecycle selection; the schema-about component test is retained in source, with a passing run recorded in the repair log. The `entry` alternatives contain no conditional, and the conditional-root fixture is rejected. | None. Existing parser-only placement rules remain the accepted boundary. |
| R1-F3 — Malformed templates pass validation for string operands | R1-F3.1–R1-F3.12: scalar/positional/named expression and effect operands; mixed text; numeric/structured operands; valid/unclosed/literal controls; CLI/editor/launch; parser; 22 hand-written action/overlay sites; CLI error-boundary reconciliation | verified fixed | `BoundOperands::report_malformed` creates positioned syntax errors independently of destination types. Generated-operand tests, all 22 hand-written rows and bundle controls, shared CLI/editor fixtures, Claudine passive parity, and portable launch rejection rows pass. Literal tokens containing malformed template text remain data. The mixed-span assertion now permits the earlier schema boundary while still requiring no launch. | None of these syntax-error instances remains. R2-F1 concerns valid expression-looking text stored as data, not malformed spans. |
| R1-F4 — New launch coverage excludes native Windows | R1-F4.1–R1-F4.8: rejection rows; initialization ordering; scalar/structured success; evaluated wrong types; expression failure and `no_error`; dry run; portable library and md/editor controls | verified fixed | The launch module is selected by `test-fixtures`, which CI and the local recipe enable. The affected cases use compiled `claudine-fake-goose` and portable file effects, without Unix gates. All 51 launch tests passed locally. Remaining Unix gates cover older prompt-recording/editing fixtures, not the required new instances. Library and md/editor modules remain declared and portable. | None. Native-Windows execution reported by the repair log was not independently repeated; a new cross-OS receipt is not required for review readiness. |

## Unblocked Findings

### R2-F1 (medium): Literal-token arguments are incorrectly deferred after decoding

**Authority — spec:** “A literal operand is checked with Darkmatter's shared
conversion rules, the same policy runtime conversion uses,” and “Validation
checks a converted copy and never rewrites the authored document or evaluates
a template.” This is also a regression in Darkmatter's existing public
`validate_with_options` contract: its existing
`validate_with_options_judges_a_literal_token_by_its_text` test rejects a token
holding `{{ x }}` for a numeric field. The new generated-argument binder
bypasses that rule for the equivalent numeric operand. Literal tokens are the
repository's existing representation of persisted data strings, including
agent-written values; they are not an invented input encoding.

**Defect class:** losing a value's stored-data origin before classifying it
as deferred lets expression-looking literal data bypass argument typing.

In Darkmatter,
[`BoundOperands::bind`](../../../darkmatter/lib/src/markdown/schemas/bound_operands.rs)
binds the decoded value through
[`validate_action_operands`](../../../darkmatter/lib/src/lifecycle/operands.rs).
That function uses the unconditional template predicate in `validate_operands`.
Although the walker retains `stored` to protect literal tokens from malformed
syntax scanning, binding does not consult that origin. An accepted deferred
call then settles its operands, removing the numeric/object type error.

For example, the public API's
`literal_token::encode("{{ fm.count }}")` returns
`{{!data:v1:e3sgZm0uY291bnQgfX0}}`. Change one generated-operand fixture field:

```yaml
start:
    - abs: "{{!data:v1:e3sgZm0uY291bnQgfX0}}"
```

The value is literal text `{{ fm.count }}`, which cannot convert to a number.
DMLS publishes no diagnostic. Darkmatter's `validate_with_options` returns
`valid: true`, with both `pending` and `deferred` empty. Claudine's public
`pre_validate_schema` also succeeds: its
[`preflight schema path`](../../../claudine/lib/src/composition/schema/mod.rs)
decodes stored text before validation. In contrast, `md schema validate
--no-baseline-schema --format json` rejects `/start/0/abs` at line 2,
column 16. The review probes used the existing isolated consumer fixtures;
they executed no provider or effect.

The bounded sweep followed the stored value through the shared binder's
argument forms, its acceptance decision, and all three schema consumers:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Generated scalar operand, public options API | `abs: encode("{{ fm.count }}")` | Accepted; no deferred path reported | Reject literal text at the operand |
| R2-F1.2 | Generated positional operands | `min: [encode("{{ fm.count }}"), 1]` | Accepted | Reject operand 1 |
| R2-F1.3 | Generated named operands | `min: {a: encode("{{ fm.count }}"), b: 1}` | Accepted | Reject `a` |
| R2-F1.4 | Generated structured operand | `merge_frontmatter: [doc.md, encode("{{ fm.count }}")]` | Accepted | Reject literal text where an object is required |
| R2-F1.5 | Shared acceptance/settlement decision | Above four forms | Binder's deferred success settles the path; public report has no pending/deferred entry | Stored data must be bound without template deferral |
| R2-F1.6 | String and literal controls | `trim` with the token; `abs` with encoded `three`; `abs` with encoded `3`; `abs` with an ordinary authored template | Accept; reject; accept; accept with deferred path, respectively | Preserve all four; clean |
| R2-F1.7 | DMLS diagnostics | Document above through published diagnostics | No diagnostic | Positioned numeric argument error |
| R2-F1.8 | Claudine public launch prevalidation | Same document in the embedded-bundle fixture | Success, no dropped optional | Reject the invalid literal argument |
| R2-F1.9 | Shipped md CLI | Same document with copied shipped bundle | Rejects `/start/0/abs`, line 2, column 16 | Reject; clean for this reproduction |
| R2-F1.10 | Hand-written operand syntax scanning | Stored literal tokens with malformed template text in `say`/`set`; generated `trim` counterpart | Existing public-API controls pass; scanner explicitly skips encoded leaves | Preserve data text without syntax scanning; clean |
| R2-F1.11 | Public Launch/Completion phase APIs and raw validation | Original encoded token, then the decoded string passed as authored text | Both phases reject the encoded token but accept the decoded string as deferred; raw validation rejects the encoded token | Preserve the original-data distinction through the caller; encoded/raw controls are clean |

Scalar, positional, named, and whole structured operands all reach the same
registration adapter. Event roots, both conditional branches, bundles, loop
keys, and nested container events have no separate registration policy: the
schema walker discovers their operand markers and calls that same adapter.
This repair needs no new syntax, escaping layer, or expression evaluator.
Retain stored origin when choosing the existing `validate_operands_with`
deferral predicate, and retain it through Claudine's schema call as well.
Keep numeric conversion, genuine authored deferral, string data, and the
malformed-token controls intact. One focused public-API matrix and consumer
wiring controls are sufficient; do not add a payload cross product.

### R2-F2 (low): Expanded schema tests exceed the L1 time budget

**Authority — regression:** review 1 recorded the affected schema/editor
selections below the five-second L1 limit. This repair expands the editor's
invalid-fixture batch and grows the resolved action schemas. Normal selected
runs now exceed that limit. The review's testing instructions explicitly say
“An L1 test over 5 s counts as failing”; a passing nextest exit does not waive
that budget.

Two measured instances need repair:

| Site | Normal selected run | Observed duration |
|---|---|---|
| DMLS `claudine_editor_parity::the_shared_corpus_invalid_fixtures_are_rejected_where_the_cli_rejects_them`, in `darkmatter/dmls/tests/l1/claudine_editor_parity.rs:167` | `cd darkmatter && just test claudine_` | 5.346 s, reported SLOW |
| Claudine `claudine_schemas_matrix::json_action_flow`, generated by the matrix in `claudine/lib/tests/l1/claudine_schemas_matrix.rs` | `cd claudine && just test-library claudine_schema` | 5.024 s, reported SLOW |

The DMLS case takes 4.231 s when selected alone, confirming inadequate margin
under normal nextest contention rather than a failing assertion. It opens 12
invalid documents serially in one test. Split that fixture batch into named
tests with a coverage guard, or reuse the expensive schema work where safe.
For the schema matrix, `observe` separately resolves the edited bundle for
the positive and negative probe documents. Reuse the effective schema when
those probes have the same discovery result, or split the shape rows into
named tests with a coverage guard. Preserve every
existing shape and assertion, and check the normal selections again. Do not
hide the overrun with a slow-tier marker, retry, ignore, or timeout increase.
This is test-runtime work, not a request for a new CI cell or a production
validation-time gate.

## Blocked Findings

None.

## Recurrence

R2-F1 repeats the passive binding/classification class of **review-1.md,
R1-F1 — Passive argument validation omits runtime registration rules**. It is
a regression introduced by connecting the binder, not an unchanged recovery,
boolean-exclusion, or ambiguity instance. The repair correctly retained the
original boolean value before conversion, but its sweep should also have
checked the already-supported stored literal-token origin through the
options API, DMLS, and Claudine's decoded-instance path. Those sites and their
clean controls are enumerated above. The required boundary is narrow: stored
data stays data when generated operands are bound. It does not call for a
new general protection mechanism.

R2-F2 is a new test-duration finding, not recurrence of the Windows test
selection finding. All original required tests are now selected portably.

## Observations

None.

## Verification and limits

Selected macOS runs used the canonical just recipes and nextest, sequentially.
The following existing tests passed; the two SLOW results remain defects under
this review's explicit budget even though nextest returned success:

| Command | Result |
|---|---|
| `cd claudine && just test-library claudine_schema` | 282 passed; one over five seconds |
| `cd claudine && just test-library container_schema_recursion` | 18 passed |
| `cd claudine && just test-cli launch_trigger_schemas` | 51 passed |
| `cd darkmatter && just test lifecycle_` | 247 passed, including generated operands, generation drift, registration parity, and event binding |
| `cd darkmatter && just test claudine_` | 106 passed, including CLI and editor parity; one over five seconds |
| `cd darkmatter && just test the_shared_corpus_invalid_fixtures_are_rejected_where_the_cli_rejects_them` | One passed in 4.231 s alone |

Temporary probes exercised Darkmatter's public options API, the shipped md
CLI, DMLS's published diagnostics, and Claudine's public prevalidation using
the existing isolated fixtures. Diagnostic probes deliberately failed to print their
returned results; an additional passing probe checked both phase APIs and
raw validation. These were reproduction probes, not failures of the
unchanged suite. All probes were removed afterward. No production code or
lasting test change is part of this review. `git diff --check` passed.

| User-facing requirement | Appropriate level and verification present |
|---|---|
| Recursive stacks, embedding, size limit, event restrictions, generated forms, corpus | L1 public API/schema/parser tests; selected Claudine tests passed |
| Container recursion and independent conditional-depth counts | L1 public schema/task-parser tests; all 18 container tests passed |
| Registration recovery/exclusions, overloads, deferred binding and no dispatch on failed binding | L1 public binder and subprocess tests; selected Darkmatter and launch tests passed, except the new literal-origin gap reproduced separately |
| Positioned CLI errors and editor diagnostic parity | L1 CLI and LSP protocol tests; existing fixtures pass, with R2-F1 documenting the newly reproduced consumer difference |
| Deep completion/hover, initialization/cancellation filtering, validation passivity | L1 LSP requests and instrumented recording counters; editor tests passed with `effects-instrumentation` enabled by the recipe |
| Regeneration and metadata inventory | L1 deterministic drift, catalog, and coverage tests in the lifecycle selection passed |
| Validation-time goal | Repair log records 115.40–121.12 ms on the representative depth-four container document, including command, host load, and dominant resolve stage; accepted without another timing sample |

No changed requirement depends on terminal-emulator rendering or OS keyboard
encoding, so Level 2 or Level 3 evidence is not required. The changed tests
are declared in consolidated L1 targets; instrumentation and compiled fake
features are enabled by the actual recipes/CI metadata. No terminal or
browser was focused.

For input robustness, this work changes generated field validation and schema
constraints, not a document loader. The Claudine schema selections exercise
the existing JSON/YAML loader matrices. The generated-operand selection
covers missing/empty calls, explicit null, wrong scalar/collection types,
invalid siblings, unknown names, optional arguments, and overload controls
through public reports. R2-F1 adds the missing stored-data classification
case. The source sweep of `operands.rs`, `schema_projection.rs`, and
`bound_operands.rs` found no additional permissive defaulting: unknown named
keys fall back to structural validation, and invalid schema regexes remain
schema-validator errors rather than silently accepted document fields.

The repair log's successful full suites, lint, tier audits, and Windows run
were reviewed as supporting records, not recharacterized as independent runs.
This review did not rerun full area suites, lint, remote OS checks, or the
release timing measurement. The recorded design choices are settled; none
needs renewed human approval.
