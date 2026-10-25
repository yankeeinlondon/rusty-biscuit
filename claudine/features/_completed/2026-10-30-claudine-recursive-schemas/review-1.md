---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Passive argument validation omits runtime registration rules"
    - "R1-F2 (high): Single-action dictionary long forms are rejected"
    - "R1-F3 (high): Malformed templates pass validation for string operands"
    - "R1-F4 (high): New launch coverage excludes native Windows"
observations: []
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-24T04:20:00-07:00"
spec: 2026-10-30-claudine-recursive-schemas/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-30-claudine-recursive-schemas/log.md
description: "A **feature** review of `2026-10-30-claudine-recursive-schemas/spec.md`"
feature: 2026-10-30-claudine-recursive-schemas/review-1.md
next: 2026-10-30-claudine-recursive-schemas/review-2.md
---

# Review 1: Claudine recursive schemas

The feature is **not production ready**. Four high findings remain. No human
decision is needed, and no finding is blocked. This is the first implementation
review, so there are no previous findings to carry forward.

## Decisions reviewed

Two agents independently read the spec and implementation log before the code
comparison. The log has `decisions_for_author: []`. Its 26 decisions cover
Darkmatter ownership of effect metadata, a checked-in schema generator,
per-overload forms, optional versus nullable operands, template recognition,
recursive event families, editor tests, and declared frozen-prompt verdicts.
These choices do not require author approval.

The temporary postponement of typed Claudine projections was completed when
the recursive families landed. Using block syntax for parameter completion
tests preserves the requested behavior; completion within flow mappings was
already absent. Keeping frozen prompts' own-schema errors as declared verdicts
is permitted by acceptance criterion 6. The separate schema/parser verdicts for
literal collections and null operands remain settled.

The Unix-only launch-test decision drops explicitly requested portable test
scope and is reported as R1-F4. The choice to restrict loop loose verbs to
short forms also leaves a promised supported form out, included in R1-F2.
Review decisions, alternatives, reasons, and downsides are recorded under
`Review iteration 1` → `Design Decisions` in the implementation log.

## Findings

### R1-F1 (high): Passive argument validation omits runtime registration rules

**Authority — spec:** “The binder decides overloads; the value schema follows
it” explicitly requires binding generated operands through
`FunctionRegistration`; it rejects relying on schema unions alone. The deferred
arguments section also requires the same conversion and overload rules as
execution. Valid documented calls now fail passive validation.

**Defect class:** projecting parameter types without the runtime registration's
binding rules makes schema acceptance disagree with execution.

In Darkmatter, [ProjectedVerb::from_function_types](/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/lib/src/lifecycle/schema_projection.rs:245)
builds scalar, tuple, and named forms from catalog types. It does not preserve
registered numeric recovery or source exclusions. The public passive binder
exists in [validate_operands](/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/lib/src/lifecycle/operands.rs:60),
but the schema-validation path does not call it. Runtime registrations add
recovery to `number` and `round`, and exclude booleans from `length`.

For example, each action below is a separate document's `start` action:

```yaml
start:
    - number: [pear, 7]  # valid: fallback supplies 7
```

`md schema validate` rejects `/start/0/number/0`; DMLS reports the same error;
Claudine's public `pre_validate_schema` rejects the document. Binding those
operands through the runtime registration succeeds. Conversely,
`start: [{length: [true]}]` passes all three schema consumers although the
runtime registration rejects it.

The sweep covered every registered recovery/exclusion site, all existing
overloaded and variadic parity rows, and the downstream consumers:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | `number`, positional and named | `[pear, 7]`; `{x: pear, fallback: 7}` | Schema rejects; binder accepts | Accept |
| R1-F1.2 | `round`, positional and named | `[pear, 2.7]`; `{x: pear, fallback: 2.7}` | Schema rejects; binder accepts | Accept |
| R1-F1.3 | `length`, scalar/positional/named projections | Boolean operand; positional and named public reproductions | Schema accepts; binder rejects `SourceExcluded` | Reject |
| R1-F1.4 | Ordinary controls for those three registrations | `number: [3, 7]`, `round: [3.7]`, `length: [hi]` | Both accept | Accept; clean |
| R1-F1.5 | Existing overload/variadic parity inventory | `code_link`, `link`, `frontmatter`, `validate_schema`, `branch_exists_on_remote`, `pr_list`, `cicd_list`, `ensure_file`, `and`, `or`, plus `min`/`ping` controls | Existing table passes | Agreement for tested shipped calls; clean |
| R1-F1.6 | Generator's synthetic ambiguous-overload test | `pick(number) \| pick(boolean)` with `"1"` | Test deliberately asserts schema acceptance and binder rejection | Passive binder must decide; this test currently pins the missing integration |
| R1-F1.7 | `md schema validate --format json` | Three failing cases above, plus a valid `min` control | Rejects valid fallbacks; accepts boolean `length` | Match runtime registration |
| R1-F1.8 | DMLS published diagnostics | Same documents | Same disagreement | Match runtime registration |
| R1-F1.9 | Claudine `pre_validate_schema` | Same documents | Same disagreement | Match runtime registration |
| R1-F1.10 | Event-time expression binding | Same operand vectors through `function_registration`/`validate_operands` | Applies recoveries and exclusion correctly | Preserve; clean |

The tuple/named matrix also checked null, boolean, number, numeric text,
malformed text, empty text, empty/nonempty arrays and objects, mixed arrays,
and all-numeric arrays. With a valid fallback, `number` and `round` bind every
tested subject shape, while schema validation accepts only number/numeric
text. `length` agrees on all those shapes except boolean. Missing arguments
fail both; omitted fallback succeeds; null and invalid fallback fail both;
extra `length` operands fail both. These are shared validators, so there is
no need for a field-by-format cross product.

**Repair:** connect passive schema validation to the existing runtime
registrations after identifying the action and argument form, preserving
source paths and deferral. Extend parity coverage to all three registered
exceptions and require the ambiguous fixture to agree. Widening individual
YAML types would create another policy and would not fulfill the decided
integration.

### R1-F2 (high): Single-action dictionary long forms are rejected

**Authority — spec:** “Preserve any parser-supported single-action dictionary
long form, including a single-action branch body”; only mappings containing
multiple actions are restricted to short forms. Loop loose keys must also
retain existing supported forms.

**Defect class:** choosing the short-only bundle alternative for every
dictionary rejects supported single-action dictionaries.

The [stack families in Claudine](/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/schemas/lifecycle.yaml:20) offer a
list or a bundle. A category's single-action long form is reachable through
a list item, but no corresponding dictionary alternative exists at stack
roots or branch bodies. The loop's loose generated keys likewise reference
only `<verb>-short`.

```yaml
start:
    ensure_file: { file: notes.md }
```

The public lifecycle parser accepts this document. Schema validation rejects
`/start/ensure_file`; the CLI, editor, and public launch prevalidation all
repeat that rejection. Putting the action inside a one-element list passes.
The same difference occurs inside either conditional branch.

Each family was reproduced with `ensure_file: {file: review.txt}`, its list
and short-form controls, and `when: 'true'`/`'false'` branch forms:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | Ordinary `stack`: `start`, `blocked`, `success`, `failure`, `finalize` | Root dictionary and `then`/`else` dictionary | Schema rejects; parser accepts, for every event | Accept |
| R1-F2.2 | `initialize-stack` | Same three positions | Schema rejects; parser accepts | Accept without admitting shells |
| R1-F2.3 | `cancelled-stack` | Same three positions | Schema rejects; parser accepts | Accept without admitting recovery directives |
| R1-F2.4 | Loop loose verb | `loop: {until: '{{ true }}', ensure_file: {file: review.txt}}` | Schema rejects; parser accepts | Accept the supported single loose long form |
| R1-F2.5 | Loop `gate` ordinary stack | List long form; dictionaries in its `then`/`else` | List passes; branch dictionaries fail schema and pass parser | Accept all supported single-action forms |
| R1-F2.6 | All seven events and loop controls | Short dictionary and one-element-list long form | Schema and parser accept | Preserve; clean |
| R1-F2.7 | Multi-action bundles | Existing `long-form-in-bundle` and `long-form-in-root-dictionary` fixtures | Both reject long forms | Preserve; clean |
| R1-F2.8 | Nested sequence task and group events | Same family imports in `sequence-types.yaml` | No independent validator: every event directly references the affected family | Repair through the shared family; preserve event restrictions |
| R1-F2.9 | CLI, DMLS, launch prevalidation | Root `start` dictionary above | All reject | Accept |

Conditional bodies recurse into the same stack type, so this defect has no
separate depth-specific implementation. Container events likewise import
these exact families; changing only a top-level trigger would leave siblings
broken.

**Repair:** add event-appropriate single-action dictionary alternatives,
separate from short-only bundles, and preserve the single loose loop long
form. Test roots and both branches across the three families, with multi-action
bundle rejection controls. Do not require authors to rewrite valid syntax.

### R1-F3 (high): Malformed templates pass validation for string operands

**Authority — spec:** the deferred arguments section requires “Malformed
template syntax is a positioned diagnostic,” and acceptance criterion 8 says
“Malformed templates fail passively.” Numeric operands satisfy this only
because their destination type rejects the malformed text.

**Defect class:** treating an invalid template as ordinary text suppresses its
syntax error whenever the parameter's value type accepts text.

Darkmatter's [is_deferred_template](/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/lib/src/markdown/compose/expression/deferred.rs:29)
correctly refuses to defer an invalid span, but refusal to defer does not
produce a diagnostic. [validate_converted](/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/lib/src/markdown/schemas/mod.rs:1135)
can accept the original string immediately, and the later schema checks also
accept it. The malformed-template filters only retain errors that already
exist; they do not create the missing syntax error.

```yaml
start:
    - ensure_file: "{{ 1 + }}"
```

The shipped CLI reports `valid: true`, DMLS publishes no diagnostic, and
Claudine's schema prevalidation returns success. The lifecycle parser later
rejects the syntax, so this is a passive-check contract failure rather than
evidence that the malformed effect executes.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F3.1 | Generated expression string operand, `trim` | Scalar, positional, and named `text`; `{{ 1 + }}` | All schema-valid | Positioned syntax error |
| R1-F3.2 | Generated effect string operand, `ensure_file` | Scalar, positional, and named `file`; same invalid span | All schema-valid | Positioned syntax error |
| R1-F3.3 | Mixed-text string operands | `trim: 'hello {{ 1 + }}'`; `ensure_file: 'out/{{ 1 + }}.md'` | Schema-valid | Positioned syntax error |
| R1-F3.4 | Numeric generated operands | Positional/named `min` with the invalid span | Rejected at the operand | Reject; clean verdict, currently an incidental type error |
| R1-F3.5 | Structured generated operand | `merge_frontmatter: [d.md, '{{ 1 + }}']` | Rejected at operand 2 | Reject; clean |
| R1-F3.6 | Template/literal controls | Valid `{{ fm.name }}`, ordinary text, unclosed `{{ unclosed` | Accepted for string parameters | Preserve; clean under the settled scanner contract |
| R1-F3.7 | `md` CLI | `ensure_file` document above | `valid: true`, no problems | Positioned syntax error |
| R1-F3.8 | DMLS | Same document | No diagnostic | Positioned syntax error |
| R1-F3.9 | Claudine schema prevalidation | Same document | Success | Reject |
| R1-F3.10 | Lifecycle parser's `action_value_to_expr` | Complete invalid whole/mixed spans | Rejects without evaluation | Preserve; clean |

Scalar, tuple, and named string operands share the same gap; all recursive
profiles reuse those definitions. Unknown application data is not a request
for a new global template-validation pass. The repair should stay within the
authored action operands this spec requires validating.

**Repair:** reuse the existing scanner/parser to return a located passive
syntax problem for malformed operand spans, independently of whether their
destination type accepts the text. Preserve valid deferred values,
interpolation literals, and literal tokens. This requires no evaluation,
host probing, effects, or new expression grammar.

### R1-F4 (high): New launch coverage excludes native Windows

**Authority — spec:** the acceptance preamble says “Tests must be portable to
macOS, Linux, native Windows, and WSL2.” The implementation log explicitly
chooses Unix-only process tests to avoid building portable provider fixtures.
Deferring requested scope is forbidden by this review's instructions.

**Defect class:** platform-gating newly required boundary tests removes that
coverage entirely from a supported target.

Claudine's [L1 target declaration](/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/cli/tests/l1/main.rs:99) puts
`launch_trigger_schemas` behind `#[cfg(unix)]`. Its marker provider and
recording helpers use shell scripts. The tests therefore cannot prove launch
wiring on native Windows, regardless of how many Windows CI runs occur.

| Child ID | Site | Shape tested or inspected | Observed result | Expected result |
|---|---|---|---|---|
| R1-F4.1 | New table rows for `compose` and `inline-compose` | `generated_verb_wrong_type`, `generated_verb_malformed_template`, `generated_verb_bad_sibling_beside_deferred`, `optional_nonnullable_null` | Eight tests pass on macOS; all excluded on native Windows | Portable launch-rejection tests |
| R1-F4.2 | Initialization ordering | `compose_runs_initialize_before_a_generated_verb_verdict` | Unix-only | Portable ordering test |
| R1-F4.3 | Successful deferred operands | `a_deferred_scalar_argument_passes_preflight_and_binds_at_event_time`, `deferred_structured_arguments_pass_preflight_and_bind_at_event_time` | Unix-only | Portable success controls |
| R1-F4.4 | Evaluated wrong type | `compose_fails_a_wrong_evaluated_type_before_dispatch`, `inline_compose_fails_a_wrong_evaluated_type_before_dispatch` | Unix-only | Portable no-launch/no-dispatch tests |
| R1-F4.5 | Expression failure and suppression | `a_failed_expression_verb_conversion_maps_like_any_raising_start_expression`, `no_error_does_not_suppress_a_binding_failure` | Unix-only | Portable failure-mapping tests |
| R1-F4.6 | Dry run | `dry_run_shows_the_deferred_operand_as_authored` | Unix-only | Portable dry-run wiring test |
| R1-F4.7 | Library schema and event-binding tests | Portable declared L1 modules | No Unix gate; public API coverage exists | Preserve; clean, but does not replace CLI wiring |
| R1-F4.8 | `md` CLI and DMLS parity modules | Portable declared L1 modules | No Unix gate; selected tests pass | Preserve; clean |

**Repair:** use a portable compiled fake provider or the repository's existing
portable fake mechanisms for these new tests, and select them on Windows.
The smallest repair may separate the new cases into a portable module while
leaving unrelated older Unix tests alone. No new CI cell, terminal window,
or fresh cross-OS receipt is requested by this finding.

## Observations

None.

## Verification and limits

Review covered the implementation commits from catalog generation through the
final consumer-parity and documentation changes. Temporary in-process, CLI,
and in-process LSP probes reproduced the findings using the existing isolated
fixtures; they were removed afterward. No production code or lasting tests
were changed by this review.

The following selected existing tests passed on macOS through `just`/nextest:

| Requirement | Verification level and evidence |
|---|---|
| Recursive families, embedding, schema/parser verdicts, corpus, input-shape validators | L1: 265 selected Claudine schema tests passed |
| All four container nesting paths and event profiles | L1: 14 `container_schema_recursion` tests passed |
| Catalog projection, regeneration drift, operand binder, event binding | L1: 169 existing Darkmatter `lifecycle_` integration tests passed; the 170-test selection included one temporary probe |
| Generated operand forms and existing overload parity | L1: 16 existing `lifecycle_generated_operands` tests passed; the 17-test selection included one temporary probe |
| Launch rejection, event-time binding, initialization ordering | L1 subprocess: 45 `launch_trigger_schemas` tests passed; native-Windows selection gap is R1-F4 |
| Shipped CLI verdicts and positioned errors | L1 subprocess: 7 `claudine_schema_bundle` tests passed |
| Editor diagnostics, deep completion/hover, event filtering, no evaluation | L1 in-process LSP: 6 `claudine_editor_parity` tests passed |
| Validation-time goal | Implementation log records 60.7–64.6 ms with command, document, load, and stages; accepted without repeating the measurement |

The changed functionality requires passive API, CLI, and LSP verification;
it makes no terminal-rendering or keyboard-encoder promise requiring Level 2
or Level 3. The inspected added test modules are declared in their consolidated
L1 targets, use ordinary L1 names, and the instrumentation-dependent editor
test was run with `effects-instrumentation`. Tested cases remained under the
five-second L1 budget.

For input robustness, no document loader was added or changed. Existing
Claudine JSON/YAML loader matrices were included in the schema selection.
Generated field validators were checked at the public result: normally wrong
types and arity must fail with a location, while optional omission, declared
nullability, arbitrary-data parameters, and deferred operands retain their
explicit contracts. R1-F1 records the recovery/exclusion shape exceptions and
the complete tested rows; R1-F3 covers template syntax independently of value
type. The code-smell sweep of new projection, operand, signature, and effect
metadata code found no permissive field-defaulting parse to add to the findings.

The implementation log records successful area lint and tier-coverage gates;
this review did not rerun lint or the full area suites. Cross-OS execution
results are left to CI and do not independently affect readiness.
