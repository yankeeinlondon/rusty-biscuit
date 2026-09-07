---
status: draft
created: 2026-09-07
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-07
review_iterations: 2
area: darkmatter
packages:
  - darkmatter
  - dmls
  - claudine
---

# Required and eager must remain independent in DMLS

## Summary

Darkmatter now treats SimplifiedSchema's `required` and `eager` constraints as
independent axes:

- `required` controls whether a property must be present;
- `eager` makes a supplied value launch-critical without making the property
  required; and
- `required; eager` combines both behaviors.

The parser, schema conversion, and runtime phase projection implement that
contract, and DMLS accepts eager-only declarations. DMLS nevertheless presents
the old contract in hover and completion documentation: its catalog-derived
description says `eager` requires the containing property at launch and
completion. An LSP integration test pins that incorrect wording.

This fix must align DMLS's authoring experience with Darkmatter's authoritative
semantics without adding a second phase engine to the language server. DMLS
should accept and explain every valid `required`/`eager` combination, diagnose
invalid supplied values through the shared Darkmatter validator, and avoid
claiming that eager-only properties must exist.

> **Reader note — review correction:** The original draft scoped the fix to
> the descriptor, one DMLS protocol assertion, the Darkmatter schema skill,
> and one conversion comment. The review found the same obsolete presence
> semantics in the public schema-definition topic, one sentence in the inline
> validation topic, another code comment, and the still-active upstream plan.
> It also found that DMLS hover detects only item-level constraints and that a
> default-mode absence test cannot distinguish eager from required because
> DMLS suppresses every missing-required diagnostic outside strict mode. The
> reviewed design covers those surfaces, exercises absence in strict mode with
> a required control, and requires array-level constraint metadata to appear in
> both schema-definition and schema-bound instance hover.

## Background

SimplifiedSchema constraints serve two different purposes during an agentic
composition run. Presence is an obligation on the producing actors, while
eager validation determines whether a supplied value is usable before a
provider starts. Conflating the two removes the useful declaration "optional,
but validate it eagerly when supplied," such as:

```yaml
$schema:
  spec: file(eager; match(**/*spec*.md))
```

Here a caller may omit `spec`. If it supplies `spec`, the reference must be
resolved and validated before launch. Authors add `required` only when omission
itself is an error:

```yaml
$schema:
  plan: file(required; eager; match(**/*plan*.md))
```

Darkmatter's runtime phase projection is authoritative. At launch it adds a
presence requirement only when a property carries both constraints; at
completion it adds a presence requirement whenever the property carries
`required`. Eager-only properties remain optional in both phases.

DMLS intentionally does not know whether an open editor document represents a
launch or completion instance. It provides passive authoring feedback by
projecting Darkmatter's parser, descriptor catalogs, completion data, and
validation problems into LSP responses. It must describe the runtime contract
accurately, but it must not invent a separate phase-aware validator or publish
runtime-only missing-property errors while a user is editing.

## Current behavior

### Correct foundations

- The SimplifiedSchema grammar accepts `eager` with or without `required`.
- `eager` is available for all supported property types and at its valid array
  placements.
- Serialization preserves the two constraints independently.
- Schema conversion has a four-cell `file` matrix proving that eager-only
  properties permit absence while validating supplied references eagerly.
- Runtime `SchemaPhase::{Launch, Completion}` projection makes presence depend
  on `required`, not on `eager` alone.
- DMLS completion is catalog-driven and offers `eager` as a constraint.

These authorities must be reused rather than copied into DMLS.

### Drifted authoring surfaces

The `eager` entry in
`darkmatter/lib/src/markdown/schemas/about.rs` currently says:

> Requires the containing property at stabilized launch and completion. On
> file items it also requires each present reference to exist.

DMLS reads that descriptor directly when building schema-definition hover, so
the incorrect presence claim reaches every supported editor. The integration
test `eager_schema_fixture_is_clean_and_catalog_driven` asserts the same text,
turning the drift into a test requirement.

Several additional explanatory surfaces retain the old model:

- `darkmatter/docs/topics/schema-definition.md` says both that eager-only is
  optional and that it is equivalent to `required; eager`; its universal
  constraint, union-hoisting, and nested-property prose still describes eager
  as a presence rule;
- `darkmatter/docs/inline/schema-validation.md` has the correct phase table but
  later says an eager-only `null` fails at launch;
- the Darkmatter schema skill says launch requires eager properties and
  completion requires required or eager properties;
- the still-active `inline-flow-and-validations` plan repeats that old phase
  rule even though its reviewed specification and implementation use
  independent axes; and
- comments in SimplifiedSchema conversion and trigger grammar describe
  `eager` as a universal presence constraint even though the implementation no
  longer makes it one.

DMLS also has two coverage gaps. Schema-definition hover recognizes eager only
in an atom's item-level constraint list, so `file[](eager)` omits the eager
explanation. Schema-bound instance hover displays `Required` but does not
display eager timing at all. Finally, default DMLS mode suppresses every
missing-required diagnostic. A default-mode fixture that omits an eager-only
property would therefore pass even if eager were accidentally compiled as
required; strict mode and a missing `required; eager` control are necessary to
prove the distinction.

The phase table in the public inline schema-validation topic and D1 of the
reviewed `inline-flow-and-validations` specification are the semantic baseline
for this fix. The contradictory prose surrounding that table is drift, not a
new design decision.

## Required semantic contract

The following matrix is authoritative for SimplifiedSchema runtime consumers:

| Declaration | Launch | Completion |
|---|---|---|
| neither | absence allowed; a present value must satisfy ordinary validation | absence allowed; a present value must satisfy ordinary validation |
| `eager` | absence allowed; a present value must satisfy eager validation | absence allowed; a present value must remain valid |
| `required` | absence may be deferred; a present value must satisfy ordinary validation | must be present and valid |
| `required; eager` | must be present and valid under eager validation | must be present and valid |

For `file`, eager validation includes resolving the supplied reference and
requiring the resolved target to exist according to the existing file-schema
contract. For other types, `eager` still communicates launch timing even when
ordinary type validation produces the same immediate result.

Explicit `null` follows the same presence rules as omission. Therefore an
eager-only `null` is allowed as absence, while `required; eager` rejects it at
launch and `required` rejects it at completion.

Array placement retains its existing ownership rules:

- `file(eager)[]` applies the eager file existence contract to each supplied
  item without making the array property launch-required; and
- `file[](eager)` applies eager timing to the supplied array property without
  changing the file items from lazy references.

Neither form implies that an absent property is required. `required` must be
declared independently when presence is mandatory. For an array property,
`file[](required; eager)` is the clearest spelling of property-level presence
and timing, although the converter's existing hoisting rules remain
authoritative for every accepted placement and union arm.

## DMLS behavior

### Descriptor, completion, and hover

The typed `eager` constraint descriptor must explain both axes directly. Its
description should communicate that:

- a supplied value is validated at stabilized launch;
- eager alone does not require the property to be present;
- combining eager with required makes presence mandatory at launch; and
- eager file values or items must resolve to existing files under the existing
  file constraint.

The descriptor's JSON Schema effect must likewise state that eager does not add
the property to `required`. `file(eager)` continues to select the eager file
format.

DMLS completion detail and schema-definition hover must continue to derive
their wording from this descriptor. Do not add DMLS-local prose for these
rules. Schema-bound instance hover must use the same descriptor when it exposes
eager timing. A property carrying only eager should show the eager explanation
but must not show the standalone `Required` marker. A property carrying both
should show both.

Constraint-marker detection must inspect both an atom's item constraints and
its postfix array constraints, across every union arm. Thus `file(eager)[]` and
`file[](eager)` both disclose eager behavior, while their hover text must not
claim either spelling makes the array property required. The same complete
constraint inspection applies to the `Required` marker so array-level
`required` does not disappear from hover.

### Diagnostics

DMLS must keep using Darkmatter's passive schema preparation and validation
authorities. It must not call `validate_for_phase`, choose a synthetic phase,
or duplicate the `project_atom` rules.

For an open document:

- an absent eager-only property produces no missing-required diagnostic;
- an absent optional property without eager likewise remains valid;
- an invalid supplied eager value produces the same typed, source-ranged
  diagnostic as the shared Darkmatter validator;
- `required` markers and missing-required behavior retain DMLS's existing
  authoring-mode policy; and
- incomplete constraint syntax remains recoverable for completion.

The absence regression must run with DMLS schema strict mode enabled and place
an eager-only declaration beside a `required; eager` declaration. The former
must remain clean while the latter produces
`dm.schema.missing_required`. A default-mode assertion may be retained as a
policy check, but it is not evidence that eager and required are independent.

The language server may explain when runtime validation occurs, but it must not
claim that the editor snapshot has passed a launch or completion verdict.

### Diagnostic ranges

Any regression fixture added for this change must continue to assert stable LSP
ranges. Schema-definition failures belong on the exact property definition,
including nested dotted paths. Instance failures belong on the supplied value.
The fix must not fall back to the whole `$schema` block merely because a
property carries both `required` and `eager`.

## Implementation boundaries

The expected implementation is deliberately small:

1. Correct the authoritative `eager` descriptor in
   `darkmatter/lib/src/markdown/schemas/about.rs`.
2. Make DMLS's shared hover metadata inspection cover item constraints,
   postfix array constraints, and union arms; expose the catalog-derived eager
   explanation in both schema-definition and schema-bound instance hover.
3. Update the DMLS protocol test that pins completion detail and add real LSP
   coverage for strict-mode eager-only absence, a missing `required; eager`
   control, eager-only invalid supplied values, both hover surfaces, and both
   array placements.
4. Correct the stale public schema documentation, active upstream plan,
   Darkmatter schema skill, and relevant conversion and trigger-grammar
   comments. Historical completed specifications and review records remain
   unchanged.
5. Preserve the existing parser, converter, phase projection, and DMLS
   descriptor-driven architecture unless a new regression proves a behavioral
   defect in one of them.

No new public type or DMLS configuration option is expected.

## Acceptance criteria

- **AC1 — independent syntax:** DMLS accepts `string(eager)`,
  `file(eager)`, `string(required)`, and `string(required; eager)` as valid
  declarations.
- **AC2 — eager-only absence:** with DMLS schema strict mode enabled, a
  document omitting an eager-only property receives no
  `dm.schema.missing_required` diagnostic while an omitted `required; eager`
  control in the same fixture does.
- **AC3 — supplied eager validation:** a present eager-only property with an
  invalid value receives the normal typed diagnostic at the value's source
  range.
- **AC4 — hover distinction:** schema-definition and schema-bound instance
  hover for eager-only properties include the catalog-derived eager timing
  explanation and omit the `Required` marker; `required; eager` hover includes
  both.
- **AC5 — accurate completion detail:** the `eager` completion item explicitly
  says absence is allowed unless `required` is also declared.
- **AC6 — one wording authority:** hover and completion obtain the eager
  explanation from `schema_constraint_descriptors()`; no duplicate DMLS copy is
  introduced.
- **AC7 — no phase fork:** DMLS does not implement or select a runtime schema
  phase. Darkmatter remains the sole phase-projection authority.
- **AC8 — matrix regression:** Darkmatter retains direct tests for all four
  `required`/`eager` combinations, including explicit `null`, supplied invalid
  values, and array ownership. DMLS hover tests cover `file(eager)[]`,
  `file[](eager)`, and array-level `required` without conflating timing and
  presence.
- **AC9 — documentation parity:** both public schema topics, the descriptor
  catalog, the active upstream plan, Darkmatter schema skill, DMLS tests, and
  relevant code comments all describe the same independent-axis contract.
- **AC10 — passive editor behavior:** the DMLS tests perform no shell execution,
  remote fetch, file mutation outside their temporary workspace, or host-window
  activation.

## Verification

Run from the `darkmatter/` package area:

```sh
just test
just lint
```

The L1 suite must include a real LSP session that opens documents through the
normal DMLS server path and verifies diagnostics, completion, and hover. No L2
editor test is required because this change does not depend on terminal or GUI
behavior.

Run the downstream package gate from the repository root:

```sh
just test claudine
```

Downstream Claudine schema-phase tests must remain green because the runtime
contract is already ratified and this fix must not change it.

## Out of scope

- Adding launch/completion state to the Language Server Protocol.
- Making DMLS execute composition, interpolation, shell expressions, or remote
  fetches to predict a future runtime instance.
- Changing raw JSON Schema behavior; raw schemas do not carry SimplifiedSchema
  phase metadata.
- Redesigning eager file resolution or `FileReference` candidate ordering.
- Changing Claudine's interactive collection policy or completion verdict.
- Introducing a new constraint to represent optional eager validation; `eager`
  already has that meaning.
- Rewriting completed specifications, reviews, or other historical records
  whose role is to record an earlier decision or implementation state.

## Open questions

None. The runtime semantics were resolved by the
`inline-flow-and-validations` review. This spec records the remaining DMLS and
documentation alignment work.
