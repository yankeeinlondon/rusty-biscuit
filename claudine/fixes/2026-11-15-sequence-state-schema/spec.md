---
kind: fix
created: 2026-11-15
status: finalized-spec
area: claudine
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
human_review: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-15
review_iterations: 0
needs_rulings: false
---

# A sequence document's `$schema` describes the document; `$state` types its steps

## The defect

In a YAML sequence document (`kind: sequence`, or any YAML file with a root
`sequence:` list), the root `$schema` does not describe the document. Claudine
applies it to **each step's state**, one step at a time, and removes it from
the document before composition. The same `$schema` in a Markdown sequence
document's frontmatter describes the document, as it does in every other
Darkmatter document.

So a YAML sequence cannot declare its caller inputs:

```yaml
# prompts/complete.yaml
kind: sequence
$schema:
    spec: file(required;match(**/*spec*.md);eager) -> the specification to complete
plan: "{{ dirname(spec) + '/plan.md' }}"
sequence:
    - name: planning
      sequence:
          - name: plan
            when: "!file_exists(plan)"
            prompt: ./plan.md
```

```text
$ claudine sequence prompts/complete.yaml spec=…/spec.md --dry-run
sequence step 0 (planning) failed schema validation at spec: "spec" is a
required property
```

The caller supplied `spec`, but the schema was judged against step 0's state
(`{name: planning, …}`), which has no `spec`. A schema written for the
document cannot be satisfied by steps of different shapes, and every
non-trivial task list has steps of different shapes.

Two prompt sequences in this repository ([`complete.yaml`](../../../prompts/complete.yaml)
and [`fix-review.yaml`](../../../prompts/_fix/fix-review.yaml)) declare a root
`$schema` expecting the document reading. No YAML sequence in the repository
relies on the step reading; it appears only in Claudine's tests and docs.

## Why it happens

`2026-07-11-sequence-plus` modeled a formal sequence document as a **data
source**: a list of same-shaped rows, a `template:` that fills fields on every
row, and a `$schema` that types each row. Its reviews required the direct and
referenced entry routes to agree, and the agreement was reached on the row
reading:

- In the Claudine library, [`formal_keys`](../../lib/src/composition/sequence/formal.rs)
  reads root `$schema` into the schema field of `FormalKeys`;
  `normalize_formal_plan` passes it to `validate_state_schema`, which validates
  every step's state. These are the schema reader and normalizer that must
  change to separate document inputs from step data.
- In the Claudine library, [`FORMAL_SEQUENCE_KEYS`](../../lib/src/composition/resolve.rs)
  (`["$schema", "template"]`) and `without_formal_sequence_keys` strip both
  keys from a directly invoked YAML document so Darkmatter's `$schema` stage
  never sees them.
- The Claudine library's [referenced-source loader](../../lib/src/composition/sequence/source.rs)
  takes the same normalization path for a referenced formal document.

The data-source use is real and worth keeping. The defect is that it borrowed
`$schema`, a key whose scope everywhere else is the document it sits in, so the
two meanings cannot coexist. Every other root key of a sequence document
(`initialize`, `success`, `loop:`, derived values such as `plan:`) already
describes the document.

## What should happen

- **`$schema` at a sequence document's root is the document schema**, in YAML
  exactly as in Markdown frontmatter. It types the document's own properties
  and caller inputs, takes part in required-property collection ("Requiring
  caller input" in `sequences.md`), and coerces values (`file(eager)`,
  `number`, …) before the steps run.
- **`$state` at a sequence document's root types each step's state.** It keeps
  today's step-state behavior unchanged: SimplifiedSchema syntax, applied after
  scalar shorthand, `template:`, and generated fields, judged per step, with
  failures naming the step index, its id, and the property path. Generated
  fields may be declared with `generated`, as today. Executables and their
  options are excluded from the state being judged.
- Both keys may appear together. They validate separate values: `$schema`
  validates document properties, and `$state` validates normalized step data.
  A required `$state` field never becomes a request for caller input.
- The direct and referenced entry routes stay identical. A referenced YAML
  sequence document's `$schema` describes that document, the way a referenced
  prompt document's `$schema` describes it; its `$state` types its steps.

```yaml
kind: sequence
$schema:
    reviewer: string(required) -> who signs off on the colors
$state:
    color: string(required) -> the color being evaluated
    rank: number(required)  -> a 1-5 ranking, 5 being best
    desc: string(required)
template:
    desc: "{{ color }}({{ rank }})"
sequence:
    - name: blue
      color: blue
      rank: 5
```

Here a missing `reviewer` joins the existing caller-input collection pass,
and every step must carry `color`, `rank`, and `desc`. A non-interactive run
reports the missing input instead of prompting.

### Scope and compatibility

This deliberately changes the meaning of root `$schema` in formal sequence
documents; it does not introduce a second schema language or a migration
mode. Markdown gains the author's requested `$state` and `template:` support,
and inline nested sequence tasks gain their own declarations. Ordinary prompt
documents and group objects keep their existing contracts.

Referenced whole sequence documents already support Markdown, YAML, JSON, and
JSON5 through the existing loaders. The key split applies to all of those
whole-document routes, without adding direct CLI support for new file formats.
A reference with `-> offset` or `::operator` still selects data rather than a
document: it does not activate that file's document schema, lifecycle,
`template:`, or `$state`. Existing strict authored-list and lenient data-list
normalization rules remain in force.

## Design decisions

### `$state` is the step-state key

Chosen by the author. The `$` prefix marks it as a schema declaration like
`$schema`, and `state` names its scope: the step state exposed through `seq`.

### `$schema` changes meaning without a transition error

The repository has no users of the step reading outside Claudine's own tests
and docs, and the repo convention is to remove rather than deprecate. A file
that relied on the step reading will now see its `$schema` judged against the
document and may fail or may pass if the document happens to satisfy it. There
is no reliable way to infer the author's old intent from that result. Update
all repository examples and tests that deliberately type rows to `$state`,
and document the rename explicitly; do not rely on a validation failure to
explain migration. No removed-grammar diagnostic is added for `$schema`.

### `$state` is a reserved document key, not composition frontmatter

Like `template:`, `$state` belongs to the sequence plan. Retain both in the
authored representation used to build the plan, and exclude both from ordinary
document composition in **YAML and Markdown**, including later source rereads.
Keep root `$schema` in document composition. Preserve authored key order and
source locations while making these views.

On an inline nested sequence task, `$schema`, `$state`, and `template:` are
container options. Extract them before separating task options from step
state; a parent's `$state` never judges these declarations as data. They are
accepted beside an inline `sequence: [ … ]`, including in a group leg or a
referenced task file. On a prompt, shell, side-effect, body-only, or
file-referencing sequence task, these container options are typed placement
errors. Put a referenced sequence's declarations in its target document.

Within sequence execution, neither `$state` nor `template` may be supplied as a caller setter, a prompt
task's `with:` key, a runtime `set` destination, or a template-generated state
field. Those routes provide values, not changes to the sequence declaration.
Use the shared reserved-key checks for these collisions. Root `$schema`
continues to follow the existing document-schema rules.

### Markdown sequence documents accept `$state` and `template:`

Ruled by the author. A Markdown sequence document's frontmatter accepts
`$state` and `template:` for its inline `sequence:` list, with the same
meaning and the same normalization order as in a YAML document. Its
frontmatter `$schema` keeps describing the document, as it does today. Both
formats' inline lists therefore use the Claudine library's
[`normalize_formal_plan`](../../lib/src/composition/sequence/formal.rs), the
shared step normalizer. The YAML-only gate must no longer control either
normalization or exclusion of these declarations from ordinary composition.

> **Review note:** the earlier draft kept a YAML-only extraction rule. That
> would let Markdown interpolate `template.desc: "{{ color }}"` against the
> document before an individual step supplies `color`. Both formats must retain
> the declaration for per-step evaluation and keep it out of the ordinary
> frontmatter interpolation pass. Apply the same protection to declarations
> on inline nested containers.

Ruled by the author (2026-11-15): for this fix, `$state` and `template:` are
declarations for an authored inline list. If the same owner instead writes a file reference, expression, or shell
source in `sequence:`, reject these sibling declarations with a typed placement
error explaining that they belong beside an inline list (or inside the
referenced whole document). Do not silently ignore them or apply a wrapper's
declarations to a referenced document. Existing dynamic sources without these
declarations are unchanged.

### Every sequence owns its own `$schema` and `$state`

Ruled by the author. Each sequence, root or nested, declares its own; nothing
is inherited:

- A parent's `$state` types only the parent's own steps. It never reaches the
  steps of a nested sequence, inline or referenced.
- A parent's `$schema` describes only the parent document. A referenced
  sequence document is judged by its own root `$schema` and `$state`.
- An inline nested `sequence:` task may declare `$schema` and `$state` beside
  its list, as it already may declare lifecycle events and `loop:`:
    - `$state` types that container's steps;
    - `$schema` is checked against the values the container sees when it is
      reached (its document's properties, caller setters, and runtime `set`
      writes), before its `initialize`, and a failure fails that task. It is
      a check, not a declaration of new caller inputs: it adds nothing to the
      root's required-property collection.
- `template:` follows the same ownership rule: an inline nested `sequence:`
  task may declare its own `template:`, and a parent's `template:` never
  reaches a nested container's steps.

```yaml
kind: sequence
$schema:
    spec: file(required) -> the spec being completed
sequence:
    - name: rank-colors
      $state:
          color: string(required)
          rank: number(required)
      sequence:
          - name: blue
            color: blue
            rank: 5
            prompt: ./rank.md
    - name: ship
      shell: just ship   # not judged by rank-colors' `$state`
```

Ruled by the author (2026-11-15): a parent's `$state` still judges the
immediate nested-container **step's own state**, such as its `name` and authored data. It stops at that boundary; it
does not judge the container's children. Consequently a parent contract that
requires `color` on every immediate step requires it on a container step too.

### Document preparation uses the document's values

Keep the existing document composition stages and launch/completion rules;
do not add a strict whole-document check that rejects all runtime-supplied
required values before `initialize` can set them. The root schema must also
be considered for an empty sequence, which has no step-composition pass to
discover missing caller inputs or invalid values.

Before evaluating a sequence's template, use its prepared document values,
including supplied caller values and normal document coercion, rather than
the raw file alone. For a referenced whole document, lowest precedence first:

1. that document's own properties and derived values;
2. invocation setters, preserving their caller-relative file context;
3. runtime `set` writes available at the relevant preparation boundary.

The referring document's authored properties do not enter this list. Inline
containers use their enclosing document's effective values instead. For each
template expression, the immediate item's fields shadow those document values,
as they do today. File reads in authored expressions and schema references use
the declaring document's context, derived from the invocation's captured
context; they must not capture a fresh working directory or use the referring
document's directory. This makes direct and referenced execution agree even
when caller and child files live in different folders.

`$state` accepts the same schema declaration forms that the old state schema
accepted: an inline SimplifiedSchema declaration or a referenced schema,
including supported root unions and named-type imports. Resolve it with the
shared schema machinery, not a new parser. Malformed or unloadable declarations
are errors even for an empty step list. Step validation remains passive: this
rename does not start evaluating state expressions or writing coerced/defaulted
schema results back into the step plan. Document `$schema` still performs its
normal composition-time coercion and defaulting.

### Nested checks run at the task boundary

An inline container's `$schema` is an additional check on a snapshot of the
values visible at its turn. It does not replace the enclosing document's
schema, insert new caller-input requirements, mutate that document, or apply
to another prompt document invoked by a child. Use ordinary passive schema
validation; any defaulted or converted validation copy is discarded. A value
needed by this check must already be available before this container's own
`initialize`.

Parse and check declaration syntax during the existing recursive grammar and
preflight walk (the walk that discovers all tasks and approves their commands
before execution). Do not judge this inline container's value requirements
against the invocation's initial values. At execution, evaluate its `when:`
guard first; a false guard skips the check and the container. Otherwise check
the snapshot before its run opens. A failed check is a typed task failure:
that container runs no `initialize`, children, or cleanup events, and the
enclosing container handles the failed task through its existing failure rules.
This follows the existing rule for checks made before a run opens.

Referenced whole documents instead follow normal document preparation,
including their own schema and caller-input collection. Their required
properties must join the existing combined collection pass, including when
the document is reached through a nested sequence or group leg and has no
prompt tasks of its own. Do not accidentally treat their `$schema` as the
inline container's check-only declaration.

Keep the existing frozen task graph: once commands and tasks have been
discovered and approved, later preparation must not add, reorder, or replace
children, regenerate their identities, or refill their template-derived state.
Templates are filled when that graph is built: runtime writes already present
at that point can participate, but writes by later tasks do not refill them.
Later document schema checks use current document values through the existing
reread path. If a reread changes a container's check, validate it at the next
entry without reopening approval. Retries and loop passes retain their existing
preparation boundaries; no new check runs between children of the same pass.

Errors must distinguish an invalid schema declaration from a value that fails
it. Preserve typed causes and source locations. A nested error names the
declaring file and full container property path, so two containers both holding
step 0 can be distinguished. Keep the existing step index, step id, property
path, and state-validation message for `$state` value failures; references to
the declaration key in help text change to `$state`.

## Implementation and verification

### Phase 1: Separate the two keys

Implement the behavior above through the existing Claudine library and CLI
paths. These are the affected responsibilities, not independent alternative
implementations:

- In the Claudine library's [formal normalizer](../../lib/src/composition/sequence/formal.rs),
  rename `FormalKeys::schema` to `state_schema`, read `$state`, and update the
  docs on `validate_state_schema`, the function that checks normalized step
  data. Root `$schema` is no longer a formal normalization key. Prepare the
  schema even when the list is empty, without resolving it afresh for every
  row.
- In the Claudine library's [source preparation](../../lib/src/composition/resolve.rs),
  change `FORMAL_SEQUENCE_KEYS` to `["$state", "template"]` and generalize
  `without_formal_sequence_keys`, which removes declarations from the
  composition view, to both document formats. Retain declarations in the plan
  view and protect nested declarations from premature interpolation. Apply
  the same separation on initial preparation and rereads.
- In the Claudine library's [root sequence resolver](../../lib/src/composition/sequence/mod.rs)
  and [referenced-source loader](../../lib/src/composition/sequence/source.rs),
  route authored inline lists through the shared formal normalizer. In
  coordination with the [CLI sequence entry](../../cli/src/commands/sequence.rs),
  supply the effective owning document's values for templates, not just its
  raw authored map. Preserve per-item shadowing and source context.
- In the Claudine library's [reserved-key catalog](../../lib/src/composition/sequence/reserved.rs)
  and [task grammar](../../lib/src/composition/sequence/grammar.rs),
  classify the new container options before extracting step data, enforce
  their allowed placements and value-write restrictions, and update the docs
  on `GENERATED_STATE_KEYS`, the list of read-only generated state fields.
  Keep a single reserved-key authority for setters, `with:`, and runtime `set`.
- In the Claudine library's [recursive preflight loader](../../lib/src/composition/sequence/preflight/mod.rs)
  and [nested sequence executor](../../lib/src/composition/sequence/task/sequence.rs),
  carry the owning declarations and their source paths, normalize each
  container's own children, and run inline `$schema` checks at the boundary
  defined above. Preserve frozen children and existing lifecycle error routing.
- Extend the Claudine CLI's [combined missing-input pass](../../cli/src/commands/wrap/sequence/phase1c.rs)
  to cover owning sequence documents as well as prompt documents. Use the
  existing passive preparation and caller-input provenance; do not run a
  lifecycle, shell, or provider while collecting requirements. Ensure an empty
  sequence still receives document validation.
- Update the actual shipped contracts in [`sequence.yaml`](../../schemas/sequence.yaml)
  and [`sequence-types.yaml`](../../schemas/sequence-types.yaml): declare
  `$state` with the shared `schema` meta-type, and declare inline task
  `$schema`, `$state`, and `template` with their appropriate types. These files
  serve both runtime validation and editor help; do not invent a separate
  editor-only definition. The task parser still enforces placement rules that
  the open task schema cannot express. Run `just shipped-schemas` from the
  repository root after changing the bundle.

### Phase 2: Tests

Update tests that pin the old reading, including their names and comments.
Use library tests for the detailed scope and declaration cases, and a small
number of CLI cases for setter ownership, input collection, and actual
composition. All are L1: they need no real provider, network, terminal window,
or focus changes. Use the repository's Test Toolkit and Claudine's existing
isolated CLI fixture. Extend consolidated test modules rather than creating
an unregistered test binary.

- `cli/tests/l1/sequence_sources_cli.rs`:
  `a_referenced_formal_documents_schema_validates_the_step_state`,
  `a_directly_invoked_documents_schema_validates_the_step_state`, and
  `a_formal_document_normalizes_identically_through_both_entry_paths` move to
  `$state`.
- A directly invoked YAML sequence whose root `$schema` requires a caller
  input: succeeds with `key=value`, fails non-interactively naming the property
  without it, and proves the normalized `file(eager)` value is what a derived
  property and step actually read. Include a schema-aware setter after the
  file, preserving the CLI's order-independent argument contract.
- The same document referenced from Markdown: same document and state verdicts.
  Cover both a root `sequence: child.yaml` reference and a nested
  `sequence: [{ name: child, sequence: child.yaml }]` task; these are separate
  implementation paths. Extend existing referenced-state coverage for setter
  precedence and the absence of referring-document properties.
- `$schema` and `$state` together: each judges only its own scope.
- An empty sequence still rejects a missing required document input, a wrong
  document value, or a malformed/unloadable `$state`. A valid empty sequence
  retains its existing lifecycle behavior.
- Declaration forms and errors: inline schema, relative schema-file reference,
  and named-type import resolve in the declaring document's context. Put the
  referrer and declaring document in different directories so a wrong base
  cannot accidentally pass. Preserve ordinary schema-reference error causes.
- Misplaced declarations and attempts to provide `$state` or `template`
  through state, setters, `with:`, runtime `set`, or template defaults produce
  typed errors. An inline nested sequence's own declarations are accepted
  as container options, including through a group leg or task file.
- A Markdown sequence document with `$state` and `template:`: steps are
  filled and typed exactly as the equivalent YAML document's. Make a template
  depend on an item-only value, and another on a caller-supplied document
  value, to expose premature interpolation and missing setter layering.
- Ownership: a parent's `$state` does not judge a nested container's steps
  (inline or referenced), but does judge the container step's own data. A
  nested container's template fills only its own children.
- Inline container checks: an earlier task's runtime `set` satisfies a later
  container's `$schema`; a missing value fails at that task's turn, before its
  `initialize`, with the enclosing failure handler observing the error. A
  false `when:` skips value checking. A malformed declaration still fails the
  early syntax check. The check's defaults/conversions do not mutate shared
  state or replace the enclosing document schema.
- Referenced containers' document requirements join missing-input collection,
  while inline checks and required `$state` fields never join it. Assert the
  combined report contains the owning documents and properties without running
  their lifecycle effects.
- Extend existing frozen-graph and retry/loop coverage where the new declaration
  handling crosses those paths: rereading document values must not change
  already planned children, ids, or template-derived state, and a failed check
  must not reopen command approval.
- Extend the existing embedded/disk schema parity and freshness tests to cover
  the new declarations. JSON/JSON5 whole documents use the same semantics;
  offset/operator data selection does not activate the selected file's
  declarations. Test loader-independent combinations at the library boundary,
  rather than multiplying CLI subprocess cases by every format.

Freeze the minimal regression under `tests/fixtures/frozen_prompts/` if it is
derived from `prompts/complete.yaml`. Automated tests must not read the live
internal `prompts/` tree, which is deliberately excluded from CI test inputs.
Assert the schema outcome and values reaching the step, rather than merely
accepting an arbitrary later error as proof of the fix.

### Phase 3: Documentation

- `claudine/docs/topics/flow-control/sequences.md`, "Formal sequence
  documents": `$state` replaces `$schema` in the example and the
  normalization-order prose; state plainly that root `$schema` describes the
  document in both formats. Also update the nested-container grammar,
  reserved-key rules, referenced-state behavior, and caller-input collection
  sections to reflect the scope and timing rules above.
- `claudine/docs/topics/schemas.md` and any other topic page that describes
  the step-state `$schema`.
- The `use-claudine` caller skill (`claudine/docs/use-skills/use-claudine/`),
  then `just publish-skills`; the repo `claudine` skill if it describes the
  step reading.
- The schema bundle's README and any schema reference examples. The package
  README changes only if its public overview/examples describe this behavior;
  detailed rules remain in the topic pages. Do not link current docs back to
  this fix. Record implementation departures in an implementation log while
  leaving this specification as the design snapshot.

## Acceptance

- A frozen, otherwise valid reproduction of
  `claudine sequence prompts/complete.yaml spec=<spec> --dry-run` proves that
  the supplied `spec` is judged as a document input, coerced correctly, and
  read by the nested planning step. The live internal prompt may have unrelated
  grammar errors; fixing those is outside this schema change.
- A root `$schema` behaves the same in a YAML sequence and a Markdown sequence
  document, run directly or referenced.
- `$state` reproduces today's step-state validation, error text included,
  with declaration-key references renamed and nested locations made unambiguous.
- Markdown and YAML both protect template declarations from ordinary
  interpolation. Each sequence normalizes and types only its own immediate
  steps; inline container checks see current state before their run opens.
- Root and referenced document inputs participate in the existing collection
  pass even for an empty list. Required step-state fields and inline checks
  never cause caller prompts.
- The shipped runtime schema bundle and disk schema definitions agree, and
  the generated shipped artifact is fresh.
- `just test` and `just lint` pass in `claudine/`.

No timing spike or benchmark matrix is needed: the fix changes schema scope
and preparation wiring, not an identified OS-specific cost. Reuse existing
schema preparation and parsed-source caching rather than resolving an
unchanged declaration separately for every step. The implementation remains
portable across macOS, Linux, native Windows, and WSL2.

## Open Questions

None. Markdown `$state`/`template:` support and per-sequence ownership were
ruled by the author on 2026-11-15 (see Design decisions). This review preserves
those choices and resolves the remaining placement, preparation, and checking
details through the existing document, lifecycle, and frozen-plan contracts.
The author also ruled on two of the review's additions on 2026-11-15:
`$state`/`template:` beside a file-reference, expression, or shell source is a
typed placement error, and a parent's `$state` judges a nested-container
step's own state (but not its children).
