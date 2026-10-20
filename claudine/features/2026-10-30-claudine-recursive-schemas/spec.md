---
area: claudine
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-16
review_iterations: 0
human_review: false
pipeline_step: 20
depends-on:
    - 2026-10-21-claudine-schemas
    - 2026-10-27-darkmatter-lifecycle-ownership
    - 2026-10-29-recursive-schema-types
related:
    - 2026-10-20-lifecycle-grammar
    - 2026-10-24-sequence-containers
    - 2026-10-28-schema-enhancements
---

# Claudine Lifecycle Schemas Use Recursive Types

Claudine authors gain editor completion, argument typing, and typo diagnostics
throughout nested lifecycle actions. DMLS (the Darkmatter language server) and
`md schema validate` check the same recursive schema; the lifecycle parser
still limits conditionals to five levels before execution.

`2026-10-21-claudine-schemas` supplies the interim schemas: three fully typed
conditional levels, loosely checked bodies below them, and named generated
verbs with untyped values. `2026-10-29-recursive-schema-types` allows a named
type to refer back to itself without copying its definition at every depth.
This specification replaces the interim shape and types generated arguments.
It does not change which actions execute or grant new permissions.

**Reader's note:** the review replaces the original single merged item example
with separate recursive families. A merged object would allow conditional keys
beside actions and could admit shell actions during initialization. Separate
families preserve the grammar and event restrictions already decided by the
prerequisites. The review also preserves Darkmatter's ownership of shared
action definitions instead of introducing a second Claudine-owned copy.
There are three event profiles: ordinary events, initialization, and
cancellation. Container recursion and conditional recursion are independent;
only conditional nesting is subject to the five-level parser limit.

## Prerequisites and ownership

The `depends-on` identifiers in frontmatter name prerequisites, regardless of
whether their directories are active or completed.
`2026-10-27-darkmatter-lifecycle-ownership` is a direct prerequisite, and
`2026-10-29-recursive-schema-types` itself depends on
`2026-10-28-schema-enhancements`; implementation starts with their ownership
and catalog changes in place. The source currently checked out may precede
those changes: this document describes the target after they land.

- `2026-10-21-claudine-schemas` establishes event action unions, separate
  initialization types, short-form bundles, schema embedding, trigger
  selection, and strict launch validation for Claudine controls.
- `2026-10-27-darkmatter-lifecycle-ownership` moves the lifecycle parser and
  shared action definitions into Darkmatter. Claudine selects a closed host
  profile and references those definitions. Darkmatter never depends on a
  Claudine crate. Darkmatter's baseline remains tolerant of unknown host
  actions; Claudine's profile remains closed.
- `2026-10-28-schema-enhancements` supplies tuples and the typed function
  catalog at `darkmatter/schemas/partials/functions.yaml`, with
  `mutations.yaml` beside it. A catalog `function` describes a signature; it
  cannot itself be used as a frontmatter value schema.
- `2026-10-29-recursive-schema-types` supplies recursive references, local
  JSON Schema `$ref`/`$defs`, positioned errors, and editor traversal.

Use the ownership prerequisite's `kind: schema-trigger` and
`--no-schema-triggers` spellings in new fixtures and documentation. The older
trigger kind and flag in pre-prerequisite examples are not compatibility
aliases. This migration does not restore them.

Claudine's `action.yaml` and `lifecycle.yaml` retain their public imports and
exported type names unless a prerequisite has already moved them. Shared
shapes are changed under their Darkmatter owner; Claudine's files select and
reference them. Update the existing embedding/import manifest for all new
transitive references. Embedded validation must work without a checkout or
schema files on disk. Disk imports retain their disk origin and never silently
fall back to an embedded file with the same basename.

## Decided

### Recursive stacks preserve event restrictions

Remove the numbered item, conditional, and stack types and every loose body
below the third level. Keep one recursive item/conditional/stack family for
ordinary events and a separate family for `initialize`. Sharing a type is safe
only when the event profiles have identical permissions. The third,
cancellation family is described under [Recursive containers](#containers-recurse-and-every-container-event-is-typed)
and is also used by document-level `cancelled`.

The following abbreviated example shows the structure; the imported action
definitions remain owned by Darkmatter as described above:

```yaml
kind: schema
types:
    stack:
        - item[]
        - bundle@./action.yaml
    item:
        - communicate@./action.yaml
        - flow@./action.yaml
        - shell@./action.yaml
        - expression@./action.yaml
        - bundle@./action.yaml
        - conditional
        - bare-verb
    conditional:
        when: expression(required)
        then: stack(required)
        else: stack
    initialize-stack:
        - initialize-item[]
        - initialize-bundle@./action.yaml
    initialize-item:
        - communicate@./action.yaml
        - flow@./action.yaml
        - expression@./action.yaml
        - initialize-bundle@./action.yaml
        - initialize-conditional
        - initialize-bare-verb
    initialize-conditional:
        when: expression(required)
        then: initialize-stack(required)
        else: initialize-stack
```

`bare-verb` and `initialize-bare-verb` stand for the prerequisite's enums of
supported bare actions, filtered for the corresponding event profile. They
must not become unconstrained strings. The local action types named `shell`
and `expression` are imported with `@`; `when: expression` uses Darkmatter's
built-in expression value type.

Preserve these contracts at every depth:

- A conditional requires `when` and `then`, permits `else`, and rejects
  sibling verbs and sibling `no_error`. An `else` without `then` fails.
- A single-verb list item permits its supported short or long form. A bundle
  contains short forms only, with the existing `when` and `no_error` rules.
  Long-form parameter maps and action mappings stay closed to unknown keys.
  Preserve any parser-supported single-action dictionary long form, including
  a single-action branch body; the short-form-only bundle rule concerns
  mappings containing multiple actions. Do not use the bundle arm to reject
  a supported single-action form.
- `initialize` excludes direct `shell` actions in both branches at every
  depth. Shell expansion inside a `set` value remains a parser prohibition
  and runtime backstop; recursive typing does not replace that protection.
- A loop's `gate:` uses the ordinary recursive stack. Loop controls and loose
  verb keys retain their existing types and parser placement rules.
- Document frontmatter, user metadata, `with` overlays, and `set` destinations
  retain their existing open-mapping behavior. Closing action keys does not
  close application data.

Empty event stacks remain legal. Explicitly empty `then`/`else` bodies,
root-bundle placement rules, action availability by event, and other semantic
restrictions that the interim schema leaves to the parser stay parser checks.
Do not claim schema acceptance proves executability.

**Reader's note:** the lifecycle parser accepts a long-form action in a
single-action dictionary, including a branch body. Earlier wording described
all dictionary forms as short-only. Preserve that supported form through a
single-action schema alternative; only multi-action bundles require short
forms. This corrects an accidental schema restriction without changing the
execution grammar.

### Containers recurse, and every container event is typed

*(amended 2026-10-16)* This spec predates `2026-10-24-sequence-containers`,
which added `task`/`group` containers with their own lifecycle events and a
restricted `cancelled` event. Its reviewed interim design bounded the
schema: two typed container levels (`task`/`group`, then
`step-2`/`task-2`/`group-2`), `cancelled` typed through a hand-unrolled
`cancelled-*` family, and every other container event checked for shape only.
This spec removes that bound:

- One recursive `task` and one recursive `group` replace the numbered
  container types. A task's inline `sequence:` items and a group's `tasks:`
  legs reference them, so containers are typed at every depth.
- Every container event is typed: `initialize` with the initialization
  family, `start`/`blocked`/`success`/`failure`/`finalize` with the ordinary
  family, and `cancelled` with a third recursive family that admits only the
  `cancelled` action set (`stop` the only flow directive), as the document-level
  `cancelled` event already does.
- The `cancelled` conditional types `then` and `else` as two literal keys. The
  interim single pattern key existed only because literal keys doubled the
  inlined size per level; with shared definitions that cost is gone, and
  literal keys restore editor completion of `then`/`else`.

Preserve the task parser's accepted container forms and required fields;
recursion does not introduce a new task syntax. A scalar sequence step remains
valid where it is valid today. Retain string file-reference alternatives for
`sequence:` and `group:` alongside their inline forms. A referenced YAML file
is still checked by the task parser during preparation, not automatically by
Markdown trigger discovery. This work does not add recursive filesystem
validation, file loading for completion, or a container-depth limit.

An absent or empty event remains valid on every container. Test nested
`initialize` restrictions and ordinary event actions as well as `cancelled`;
typing only cancellation would leave the main container coverage gap open.

### Generated actions project catalog signatures into value schemas

Read the typed expression catalog through Darkmatter's library API. A
repository generator may read the authoritative YAML using the same grammar;
it must not parse Rust source or use a separate reduced signature vocabulary.
Read the side-effect inventory through Darkmatter's public descriptor API.
Today the Darkmatter library's
[`EffectDescriptor`](../../../darkmatter/lib/src/effects/catalog.rs) describes
capabilities and parameter names but does not supply parameter types. Those
types come from Darkmatter's shared action parameter definitions, as decided
under [Side-effect argument metadata](#side-effect-argument-metadata).

Generate ordinary value schemas from parameters, never import a `function`
type into the document schema. Preserve parameter types and constraints,
required versus optional arguments, explicit null acceptance, overloads, and
rest parameters. Return types describe the function's result, not the action's
input, and do not become action-value schemas.

- Positional arrays use tuples so each position has its own type and arity.
  A required argument cannot be omitted; an optional argument may be omitted
  but accepts explicit null only if its type does. Zero-argument null/empty
  array forms follow the parser. Scalar forms follow the existing grammar.
- Emit one arm per supported overload, deduplicating identical value shapes.
  Never combine unrelated overloads into one permissive map. For example,
  `ensure_file` retains both its one-argument and two-argument forms.
- Nonvariadic long forms use the catalog's parameter names, with required
  markers on required parameters and the existing `no_error: boolean` field.
  Variadic functions, including `and` and `or`, remain positional-only; their
  positional schema preserves any fixed prefix and final rest element.
- Bundles reference the corresponding short forms only. Categories,
  standalone actions, and bundles use the same generated definitions so
  their argument typing cannot drift.
- Preserve the parser's collision and alias policy. The lifecycle `set`
  action remains a destination-to-value mapping with no long form and no
  action-level `no_error`; the catalog's positional `set(key, value)` does
  not replace it. Record each other existing name collision and its winning
  action definition in the parity test. A newly ambiguous collision fails
  generation rather than silently dropping an entry. Completion lists
  canonical names; accepted aliases follow the existing parser contract.
  Preserve documented planned-only hand-written entries such as `break` and
  `prep` until their implementing work changes that status. Test these
  exceptions separately from generated catalog parity; schema acceptance
  still does not promise parser or runtime support.
- `unknown` remains valid where the catalog genuinely accepts arbitrary data.
  It is not a fallback for missing type metadata. No action-key catch-all is
  added to Claudine's profile.

Provide a maintained repository generator as an authoring tool; builds embed
checked-in YAML and do not regenerate it. The historical spike script is
reference material, not the production output location. Reuse its useful
logic without changing the frozen measurement inputs. Document a deterministic
regeneration command that works from any checkout without hardcoded absolute
paths or writes to historical spike
fixtures. Compare generated structures with the checked-in definitions to
catch drift, including short forms and bundle projections, not only names.

**Intended behavior change:** literal argument mistakes previously accepted by
untyped generated keys now produce positioned schema errors, including before
Claudine launches a provider. Preserve the prerequisite's validation phases
and declaration precedence; this change does not make optional events required
or authorize silent removal of invalid Claudine controls. Deferred values and
runtime conversions follow
[Deferred arguments and runtime conversions](#deferred-arguments-and-runtime-conversions).

### Deferred arguments and runtime conversions

A lifecycle operand can contain `{{ fm.count }}` and receive its value only
when the event runs. A catalog may require a number even though the author
supplied numeric text that Darkmatter's conversion engine accepts. Copying the
destination type into a YAML schema would reject supported inputs, and adding
an unrestricted string alternative would hide real mistakes. Generated argument
typing therefore reuses expression-aware validation and the shared conversion
rules:

- Apply argument validation after identifying the action form and binding its
  positional or named parameters. A template in one operand never defers the
  action name, parameter names, arity, required-argument presence, or checks
  of other literal operands. For example, an extra named argument is invalid
  even beside `count: "{{ fm.count }}"`.
- A literal operand is checked with Darkmatter's shared conversion rules, the
  same policy runtime conversion uses. For a number parameter, `3` and `"3"`
  pass; `"three"` is a positioned schema error.
- An operand containing `{{ … }}` is recognized as deferred through
  Darkmatter's existing template parser. Schema validation accepts it without
  judging its value; its evaluated value is checked against the parameter type
  before the action is dispatched when the event runs. This applies to
  deferred scalars and deferred structured values alike. A whole-value
  expression can supply a collection; a collection containing an expression
  defers only the affected child values, while literal siblings and the
  collection's known shape remain checked. Recognize expressions using the
  template parser, not a substring test. Malformed template syntax is a
  positioned diagnostic, not an exemption from validation.
- Schema validation, in the CLI, at Claudine's launch preflight, and in DMLS,
  never evaluates an expression, probes the host, or performs an effect to
  discover a value.
- No second conversion policy or expression evaluator is introduced; schema
  projections integrate with the existing deferred-value checks. Input
  acceptance is a projection of the shared parameter contract, distinct from
  the value delivered after conversion. Validation checks a converted copy
  and never rewrites the authored document or evaluates a template. Do not
  mark operands `eager` merely to enforce required call parameters; the
  Launch/Completion presence rules must not make an incomplete call valid.

**Reader's note:** ordinary JSON Schema argument types alone do not express
convertible inputs and deferred operands. The implementation must connect the
shared parameter binder and template classification to passive schema
validation, rather than widening every numeric argument to arbitrary text.
That integration belongs to Darkmatter so `md`, DMLS, and Claudine use one
verdict. If the prerequisites do not expose the necessary passive API, adding
that API is required work here and its cost must be recorded; it is not
permission to omit argument checks. Use the same overload-selection rules
at validation and execution, and never combine separate overloads' parameters
to accept a call that no overload admits. This intentionally replaces any
older merged-signature approximation for generated argument checks, while
retaining valid documented call forms.

### Side-effect argument metadata

The current side-effect descriptors provide names and signatures, not parameter
types, and the companion mutations catalog is not evidence that every effect
has a complete typed signature. Full typing cannot be inferred from parameter
names such as `file` or `body`. The source of effect parameter types is
therefore decided as follows:

- The generator reuses Darkmatter's shared action parameter definitions
  produced by `2026-10-27-darkmatter-lifecycle-ownership`.
- Where a coverage gap is proven, the fix is to extend that same
  Darkmatter-owned metadata, giving one reusable contract for generation and
  runtime checks, with overloads and optional parameters stated explicitly.
- A coverage test compares the shared definitions against Darkmatter's
  side-effect descriptors and fails when a descriptor has no typed parameter
  definition.
- Missing metadata never silently falls back to untyped values; it fails the
  coverage test and generation.
- There is no generator-local table of effect parameter types.

### The conditional depth limit remains parser-only

A root-list `when` counts as level one, whether it uses `then`/`else` or
guards sibling actions in a bundle. Each nested `when` in either branch adds
one level; guards beside each other in a list do not accumulate levels. A
guarded bundle inside a level-five conditional is therefore level six, even
without a `then` key. Container nesting itself does not consume this budget;
each event stack starts its own count. The parser validates both
branches and rejects level six with `LifecycleStackInvalidShape` and its path.
That parser is Darkmatter-owned after the ownership prerequisite; Claudine
surfaces its diagnostic using its existing error mapping.

The schema encodes no conditional depth limit. `depth6.md` is therefore
schema-valid but parser-invalid. `depth6-wrong-verb.md` has a misspelled verb at
level six and is schema-invalid with the offending key, path, and source
position. A syntax/type error and a conditional-depth error are separate
checks; do not require the parser to choose one diagnostic ahead of the other
when a fixture contains both defects.

### Fixture updates are explicit

Review each existing fixture against the grammar and record the intended
schema and parser verdicts separately. Do not regenerate expected verdicts by
blindly accepting actual output. Correct generated argument types may change
more fixtures than `depth6.md`; any such change is explained in the
implementation log and retains the parser's supported forms.

Use frozen copies under `tests/fixtures/frozen_prompts/` for behavior tests
that need internal prompts. Do not add CI reads of `prompts/`,
`claudine/prompts/`, or `darkmatter/prompts/`. Checking the live internal prompt
corpus remains opt-in through `just test-prompts`; it is not a new normal L1
or CI gate. Other shipped schema examples and lifecycle documents are ordinary
fixtures with declared repository inputs.

## Out of scope

| Work | Owner or boundary |
|---|---|
| Recursive reference resolution and generic editor traversal | `2026-10-29-recursive-schema-types` |
| Lifecycle grammar, execution semantics, and conditional depth limit | `2026-10-20-lifecycle-grammar` and the ownership prerequisite |
| Schema embedding, trigger selection, event typing, and launch policy | `2026-10-21-claudine-schemas`; preserve those contracts |
| Lifecycle event ownership, host profiles, and tolerant Darkmatter baseline | `2026-10-27-darkmatter-lifecycle-ownership`; integrate without reversing ownership |
| New coercion rules or a second expression evaluator | Reuse Darkmatter's existing mechanisms |
| DMLS caching and performance infrastructure (stage timings, benchmark) | Part A of `2026-10-29-recursive-schema-types` |
| New CI matrix cells | Not needed for this schema migration |

## Estimate

**2–3 engineer-days, likeliest 2.5**, provided shared typed action metadata and
existing deferred-value validation can be reused: catalog projection
(0.5–0.75), recursive families and removal of the unroll (0.25–0.5), fixture
updates and parser parity (0.5–0.75), DMLS parity (0.25–0.5), docs and skill
(0.5). Extending the Darkmatter-owned action metadata for gaps the coverage
test proves, and any new integration between schema projections and the
existing deferred-value checks, is not included here; record that added cost
in the implementation log when the coverage test identifies it. Recursive containers and the third `cancelled` family add
**0.75–1.25 days** *(amended 2026-10-16)*. The combined baseline estimate
is therefore **2.75–4.25 engineer-days**, before any missing shared metadata
or passive-validation API work is identified.

The recursive resolver prerequisite already measures duplication reduction.
Do not repeat its performance spike or promise an order-of-magnitude runtime
improvement for this different catalog. Retain the draft's 2 MB resolved-size
check for the final embedded Claudine payload as a deterministic regression
check. Measure the UTF-8 byte length of compact serialized resolved JSON
Schema (including `$defs`, without pretty printing), with a ceiling of
2,000,000 bytes. Resolve the final Claudine trigger payload with its complete
transitive bundle in isolation from user schemas and the document baseline;
also resolve each exported type independently for correctness. Record which
root was measured so unrelated user schemas cannot change this result. If it
fails, inspect duplicated definitions and record the cause; do not loosen it
or add timing thresholds without author review.

## Acceptance criteria

Use nextest through `just test` in `claudine/` and `darkmatter/`, and `just lint`
in both affected areas. Tests use the Test Toolkit, isolated command fixtures,
and explicit repository-input reads where needed. No test focuses a terminal
or browser or executes real effects, shell commands, audio, or providers.
Tests must be portable to macOS, Linux, native Windows, and WSL2. Add no CI
matrix cell or gate; the existing planner schedules changed code normally.

1. **Recursive shape and embedding.** All three event families recurse, with
   no numbered unroll or loose deep body, and the loop gate uses the ordinary
   family. The final embedded payload contains local `$ref`/`$defs`, resolves
   without disk schemas, and is at most 2 MB. Resolve every exported type and
   exercise initialization, ordinary, and cancellation event imports. Include
   nested task/group references in the size-checked trigger payload.
2. **Catalog and generated-definition parity.** Canonical generated names
   match the parser-admitted catalog inventory after documented collisions
   and host filtering. Value schemas match signatures, including overloads,
   optional/null distinctions, positional arity, variadic forms, long-form
   names, and bundle short forms. Test an invalid argument, unknown parameter,
   missing required argument, and extra argument as well as valid forms.
3. **Recursive restrictions.** At depths 1, 3, and 5, test valid `then` and
   `else`, bundle bodies, bare actions, and loop gates. Reject misspelled verbs,
   malformed long forms, conditional/sibling-action mixtures, and direct
   initialization shells, including in an unselected deep branch. Preserve
   `set` and open application mappings.
4. **Depth fixtures.** Valid depth 5 passes both schema and parser. Valid
   `depth6.md` passes `md schema validate` and DMLS, while parser validation
   rejects the sixth level by path. `depth6-wrong-verb.md` fails both schema
   surfaces with the verb, path, and source position. `md schema triggers`
   confirms the intended trigger was discovered; its listing alone is not
   a validation verdict. A guarded bundle inside a level-five conditional
   passes schema validation but fails the parser's depth check, just like a
   sixth `when` using `then`.
5. **Editor parity and passivity.** DMLS diagnostic verdicts match CLI schema
   validation for the shared fixtures. Completion and hover reach generated
   verbs and long-form parameters at depth 4 in `then` and `else`, filter shell
   from initialization, and preserve the loop gate's profile. Recording fake
   services prove schema validation and editor requests execute no actions.
6. **Launch integration and corpus.** With a fake provider, a malformed
   generated literal blocks launch without scrubbing its control key. A
   supported deferred argument is recognized as deferred, passes preflight,
   and is checked at its established execution boundary. Ordinary shipped examples
   and frozen prompt copies pass with declared verdicts. Live internal prompt
   checks are opt-in; existing DMLS parity tests still pass.
7. **Documentation and regeneration.** Update Claudine's schema topic pages
   and skill and the affected Darkmatter schema pages/skill for shared
   definitions. Describe recursive typing, generated argument forms, and the
   parser-only limit with compact examples. Remove current-doc claims of
   unrolled/untyped behavior; historical specs and spike fixtures remain
   snapshots. Docs never name a feature directory. The documented generator
   command reproduces the checked-in structures without changing frozen input
   fixtures.
8. **Deferred arguments and conversions.** For a number parameter, a numeric
   literal (`3`) and convertible numeric text (`"3"`) pass schema validation;
   invalid text (`"three"`) fails with its path and source position. A
   deferred scalar (`{{ fm.count }}`) and a deferred structured value pass
   schema validation and are checked at the runtime boundary, where an
   evaluated value of the wrong type fails. Recording fakes prove no
   expression is evaluated, no host probe runs, and no effect executes during
   schema validation. A deferred operand beside an unknown parameter, missing
   required argument, extra positional argument, or invalid literal sibling
   does not hide that error in either Launch or Completion validation.
   Malformed templates fail passively. Failed conversion leaves the input
   unchanged, and runtime binding failure prevents action dispatch.
9. **Side-effect metadata coverage.** A test fails when any side-effect
   descriptor lacks a typed parameter definition in Darkmatter's shared action
   metadata, and the generator rejects such an effect rather than emitting an
   untyped value. The generator contains no local table of effect parameter
   types.

10. **Recursive containers.** *(amended 2026-10-16)* No numbered container
   type (`step-2`, `task-2`, `group-2`) or loose container list remains. For
   each container path sequence → sequence, sequence → group, group →
   sequence, and group → group, nested to container depth 4, an unknown
   action in each of the seven container events is rejected with its path,
   and each forbidden `cancelled` action (`retry`, `resume`, `proxy`, `prep`,
   `skip`, `error`, `break`, `defer`) is rejected, including inside `then`
   and `else` at conditional level 4. `[stop]` and `[]` remain valid
   `cancelled` stacks. DMLS completes `then`/`else` inside a `cancelled`
   conditional. Preserve scalar steps and file-reference alternatives.
   At nested container paths, reject an initialization shell in each branch
   and a malformed generated argument in an ordinary event. Confirm each
   container event starts an independent conditional-depth count.
11. **Validation time goal.** *(amended 2026-10-16)* One-shot `md schema
   validate` of a Claudine document with nested containers targets **under
   200 ms** on the dev Mac. Take a quick sample of one representative nested
   container document using the prerequisite's existing stage timings; reuse
   an equivalent measurement if already available. No additional benchmark
   suite, repetition count, or OS run is required. The command, document,
   result, host load, and available stage breakdown are recorded in the
   implementation log. This is a recorded goal, not a test or gate; a miss is
   recorded with its dominant stage, not hidden by a relaxed target.

## Open questions

None. The passive argument-validation integration is required scope rather
than a choice to weaken typing. If implementation uncovers a contract that
cannot represent a documented valid call, stop that portion of the migration
and record the concrete call and alternatives for author review before
changing accepted syntax or conversion rules.
