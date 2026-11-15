---
kind: fix
created: 2026-11-15
status: draft-spec
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
reviewed: false
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

Two prompt sequences in this repository (`prompts/complete.yaml` and
`prompts/_fix/fix-review.yaml`) declare a root
`$schema` expecting the document reading. No YAML sequence in the repository
relies on the step reading; it appears only in Claudine's tests and docs.

## Why it happens

`2026-07-11-sequence-plus` modeled a formal sequence document as a **data
source**: a list of same-shaped rows, a `template:` that fills fields on every
row, and a `$schema` that types each row. Its reviews required the direct and
referenced entry routes to agree, and the agreement was reached on the row
reading:

- `claudine/lib/src/composition/sequence/formal.rs`: `formal_keys` reads root
  `$schema` into `FormalKeys::schema`; `normalize_formal_plan` passes it to
  `validate_state_schema`, which validates every step's state.
- `claudine/lib/src/composition/resolve.rs`: `FORMAL_SEQUENCE_KEYS`
  (`["$schema", "template"]`) and `without_formal_sequence_keys` strip both
  keys from a directly invoked YAML document so Darkmatter's `$schema` stage
  never sees them.
- `claudine/lib/src/composition/sequence/source.rs`: a referenced formal
  document takes the same `normalize_formal_plan` path.

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
  fields may be declared with `generated`, as today.
- Both keys may appear together. Neither reads the other.
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

Here `reviewer` is asked of the caller once, and every step must carry
`color`, `rank`, and `desc`.

## Design decisions

### `$state` is the step-state key

Chosen by the author. The `$` prefix marks it as a schema declaration like
`$schema`, and `state` names its scope: the step state exposed through `seq`.

### `$schema` changes meaning without a transition error

The repository has no users of the step reading outside Claudine's own tests
and docs, and the repo convention is to remove rather than deprecate. A file
that relied on the step reading will now see its `$schema` judged against the
document, which reports the missing properties by name; that message points
at the cause. No `RemovedGrammar` diagnostic is added for `$schema`.

### `$state` is a reserved document key, not composition frontmatter

Like `template:`, `$state` belongs to the sequence plan. It is lifted out of a
directly invoked YAML document before composition (replacing `$schema` in
`FORMAL_SEQUENCE_KEYS`), never reaches a step's state, and may not be declared
as a step field or a `with:` key.

### Markdown sequence documents accept `$state` and `template:`

Ruled by the author. A Markdown sequence document's frontmatter accepts
`$state` and `template:` for its inline `sequence:` list, with the same
meaning and the same normalization order as in a YAML document. Its
frontmatter `$schema` keeps describing the document, as it does today. Both
formats' inline lists therefore take one path through `normalize_formal_plan`;
the YAML-only gate (`is_direct_formal_document`) no longer decides whether
`$state` and `template:` apply, only whether they must be lifted out of the
frontmatter before composition.

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

## Implementation and verification

### Phase 1: Separate the two keys

- `formal.rs`: `FormalKeys::schema` reads `$state`; rename the field and
  `validate_state_schema`'s docs to match. Root `$schema` is no longer a
  formal key.
- `resolve.rs`: `FORMAL_SEQUENCE_KEYS` becomes `["$state", "template"]`, so a
  directly invoked YAML document keeps its `$schema` for Darkmatter's schema
  stage.
- Referenced YAML sequence documents: their root `$schema` describes the
  referenced document wherever its frontmatter is read (events, conditions,
  `set`, steps; "What a referenced sequence reads" in `sequences.md`),
  including required-property collection.
- `reserved.rs`: the `GENERATED_STATE_KEYS` doc names `$state`.
- `mod.rs`: a Markdown sequence document's inline list goes through
  `normalize_formal_plan` with its frontmatter's `$state` and `template:`.
- Inline nested `sequence:` tasks: the task grammar accepts `$schema`,
  `$state`, and `template:` beside the list; `$state`/`template:` feed that
  container's normalization, and `$schema` is checked when the container is
  reached, before its `initialize`.

### Phase 2: Tests

Update the tests that pin the old reading, and add the missing cases, all L1:

- `cli/tests/l1/sequence_sources_cli.rs`:
  `a_referenced_formal_documents_schema_validates_the_step_state`,
  `a_directly_invoked_documents_schema_validates_the_step_state`, and
  `a_formal_document_normalizes_identically_through_both_entry_paths` move to
  `$state`.
- A directly invoked YAML sequence whose root `$schema` requires a caller
  input: succeeds with `key=value`, fails (non-TTY) naming the property
  without it, and coerces a `file(eager)` value before step 1.
- The same document referenced from a Markdown sequence: same outcomes.
- `$schema` and `$state` together: each judges only its own scope.
- `$state` written as a step field or a `with:` key is a typed error.
- A Markdown sequence document with `$state` and `template:`: steps are
  filled and typed exactly as the equivalent YAML document's.
- Ownership: a parent's `$state` does not judge a nested container's steps
  (inline or referenced); an inline nested task's own `$state` does; its
  `$schema` fails that task, not the preflight, when a value it requires is
  missing at its turn.

### Phase 3: Documentation

- `claudine/docs/topics/flow-control/sequences.md`, "Formal sequence
  documents": `$state` replaces `$schema` in the example and the
  normalization-order prose; state plainly that root `$schema` describes the
  document in both formats.
- `claudine/docs/topics/schemas.md` and any other topic page that describes
  the step-state `$schema`.
- The `use-claudine` caller skill (`claudine/docs/use-skills/use-claudine/`),
  then `just publish-skills`; the repo `claudine` skill if it describes the
  step reading.
- The editor schema for sequence documents, if it declares the formal keys.

## Acceptance

- `claudine sequence prompts/complete.yaml spec=<spec> --dry-run` gets past
  schema validation (later failures unrelated to this fix are out of scope).
- A root `$schema` behaves the same in a YAML sequence and a Markdown sequence
  document, run directly or referenced.
- `$state` reproduces today's step-state validation, error text included,
  with only the key renamed.
- `just test` and `just lint` pass in `claudine/`.

## Open Questions

None. Markdown `$state`/`template:` support and per-sequence ownership were
ruled by the author on 2026-11-15 (see Design decisions).
