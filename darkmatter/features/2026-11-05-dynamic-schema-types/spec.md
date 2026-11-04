---
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
kind: feature
created: 2026-11-04
status: draft-spec
area: darkmatter
reviewed: false
clarified: false
implemented: false
review_iterations: 0
related:
    - 2026-11-04-loading-data
    - 2026-10-28-extract-simplified-schema
---

# Dynamic Schema Types

Let the expression engine take part in defining a schema. At minimum,
`enum(...)` members and `suggest(...)` candidates must be computable from an
expression. The goal is **broad** support: anywhere a schema author writes a
type expression should eventually accept a `{{ … }}` expression.

This is a **draft**. Its main job is to frame the spikes that decide how
dynamic types can run with good performance. Narrowing where expressions may
appear is the **last resort**. It is considered only if the spikes show that
no caching or resolution strategy keeps validation and DMLS responsive.

## Motivation

A schema often repeats a list that already lives in a data file. The
motivating case is `claudine/docs/research/sandboxing/providers/sandboxing.yaml`:

```yaml
types:
    os: enum({{ as_csv(claudine/docs/os.yaml) }}; required)
```

Today this fails when the schema loads:

```text
invalid SimplifiedSchema for property os:
expected `,`, `;`, or `)` in enum members, found `a`
```

`parse_enum_members` (`simplified/grammar.rs`) accepts only literal members,
and schema handling is deliberately passive: parsing, validation, trigger
matching, completion, and hover perform no I/O and evaluate no expressions.
With 2026-11-04-loading-data, the intended spelling becomes:

```yaml
types:
    os: enum({{ load_data("claudine/docs/os.yaml", string[]) }}; required)
```

## The tension to resolve

Four existing guarantees push against dynamic types. The spikes must keep
each one or replace it with something equally strong:

1. **Passive schemas.** Schema parsing and validation do not execute anything.
   A schema found by trigger discovery — even in DMLS, just by opening a
   folder — must not be able to run code or touch the network unasked.
2. **Validator caching.** `DarkmatterSchemas` compiles each schema once into a
   process-wide `SHARED_VALIDATOR_CACHE` keyed on the schema and its resolution
   context. A long-running host (Claudine) composes many documents against a
   few schemas and relies on this.
3. **DMLS latency.** Completion, hover, and diagnostics run on every edit and
   must not wait on file reads or expression evaluation on the hot path.
4. **The extraction boundary.** 2026-10-28-extract-simplified-schema moves the
   schema language into the host-neutral `simplified-schema` library. That
   library owns the *passive* expression parser but has **no evaluator**:
   evaluation, function registration, and lookup stay in Darkmatter.

## Proposed model (to be validated by the spikes)

Split schema loading into a passive phase and an explicit, cached
**resolution** phase:

```mermaid
flowchart LR
    A[Schema source] -->|passive parse| B[Schema template<br/>static parts + expression slots]
    B -->|resolve: evaluate slots<br/>via host resolver| C[Concrete SimplifiedSchema]
    C -->|compile| D[Validator]
    B -. fingerprint .-> K[(Resolution cache<br/>key = schema hash + input hashes)]
    K -. hit .-> C
```

- **Parse stays passive.** The grammar accepts a `{{ … }}` slot where a member
  list (and later, other type positions) is allowed, checks its syntax, and
  records its span. Nothing is evaluated. Editor features keep working on the
  template.
- **Resolution is the only effectful step.** `simplified-schema` defines a
  host hook (a resolver trait). Darkmatter implements it with its evaluator.
  The result must have the shape the slot requires (`enum` and `suggest` slots
  need an array of scalars), or schema loading fails with the slot's span.
- **Resolution is cached by its inputs.** The cache key is the schema content
  hash plus a fingerprint of everything evaluation read (file content hashes
  via `biscuit-hash`, environment values, `ctx.*` groups). A warm hit costs a
  hash comparison, not an evaluation.
- **Evaluation context is the schema's, not the document's.** Relative paths
  resolve against the schema file. One schema then resolves once for every
  document that uses it, which preserves validator sharing (guarantee 2).

## Spikes

Each spike reports measured numbers on the shipped schema corpus and on one
synthetic heavy case. Each one ends with a recommendation, not a prototype
merged to `main`.

### S1. Baseline and budget

Measure today's cold and warm cost for schema load, validator compile, and
validation (`md schema validate`, `md compose`, DMLS diagnostics). From those
numbers, set the budgets the other spikes are judged against. A proposed
default: warm overhead within noise of a static schema, and a cold resolution
cost no larger than one validator compile.

### S2. Input-fingerprinted resolution cache

Prototype the resolution cache. Record every input an evaluation touches
through the existing demand-driven context capture, and key on those inputs.
Measure hit rate and cost across a Claudine-style many-document run. Decide
whether the cache is process-wide (like the validator cache), persisted on
disk, or both.

### S3. Invalidation and DMLS

Determine how DMLS learns that an input changed (the watch-directories
mechanism, file events, or a fingerprint check on save). Resolution must run
off the request path: DMLS serves the last resolved schema and marks it stale
until a fresh one is ready. Measure added latency on completion and hover.

### S4. Function determinism classes

Classify catalog functions as **pure** (same inputs → same output),
**input-reading** (fingerprintable: `load_data`, `file_exists`), or
**volatile** (time, git state, network, `$()`). Pure and input-reading slots
are cacheable. Decide whether volatile slots get a TTL, resolve on every use,
or are rejected. This classification is metadata for caching, not a scope
limit. Only rejecting volatile functions would count as narrowing.

### S5. Safety and approval

Decide when resolution is allowed to run. Options to evaluate: resolution is
allowed only for slots whose functions are pure or local-input-reading, while
shell and remote reads go through the existing approval and `--allow host=`
flows; or a schema with an effectful slot can be discovered but stays
unresolved until an explicit request (for example `md compose`) approves it.
Trigger discovery and DMLS must never cause an unapproved effect.

### S6. Lazy and partial resolution

Measure whether resolving only the slots a given document reaches — and
compiling static parts once, with dynamic parts substituted later — is a
material win over resolving the whole schema up front.

### S7. Extraction boundary

Specify the resolver trait in `simplified-schema` and the template type it
operates on, and coordinate the sequencing with
2026-10-28-extract-simplified-schema. This spec should land after that
extraction, or define its hook in a form the extraction can adopt unchanged.

### S8. Breadth

Starting from the `enum`/`suggest` slots, evaluate which other positions can
accept a slot at no extra cost: `literal(...)`, `default(...)`, numeric bounds
(`min`/`max`), pattern constraints, and whole type expressions (a slot that
yields a `type-definition`). Only positions that the S1–S6 results show cannot
meet the budget are candidates for narrowing.

## Minimum requirements

Whatever the spikes conclude, the finalized spec must deliver:

- **M1.** `enum(...)` members from a `{{ … }}` slot that yields an array of
  scalars, mixable with the `; required` / `default(...)` tail.
- **M2.** `suggest(...)` candidates from a slot, on every type where
  `suggest` is allowed today.
- **M3.** Slot syntax errors and wrong result shapes are positioned schema
  errors, reported in `md schema validate` and DMLS like any other schema
  error.
- **M4.** `sandboxing.yaml` (in the M1 spelling above) loads and validates its
  documents.
- **M5.** No guarantee from "The tension to resolve" is lost without an
  explicit, documented replacement.

## Open questions

- **Q1.** Is a slot the whole member list, or may literal and dynamic members
  mix (`enum(none, {{ … }})`)?
- **Q2.** Should JSON Schema output (`md schema` emitters) contain the resolved
  values, or the template plus an `x-darkmatter-*` extension?
- **Q3.** Does any case truly need document-relative evaluation (for example
  `fm.*` inside a schema)? If so, how does it coexist with validator sharing?
- **Q4.** Should `kind` catalogs and baseline schemas be allowed dynamic slots,
  given that they load on every composition?
