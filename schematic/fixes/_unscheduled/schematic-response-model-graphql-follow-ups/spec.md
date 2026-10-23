---
area: schematic
status: unscheduled
created: 2026-10-22
owner: Ken Snyder <ken@ken.net>
origin: review 2 of 2026-10-16-schematic-response-model-graphql
related:
    - 2026-10-16-schematic-response-model-graphql
packages:
    - schematic-define
    - schematic-gen
    - schematic-definitions
---

# Schematic Response Model GraphQL Follow-ups

Items still open after the final review of
`2026-10-16-schematic-response-model-graphql`. That review found the feature
production ready with no findings. It recorded the two observations below,
which are known limitations left for separately scheduled work. See
[review 2](../../../features/2026-10-16-schematic-response-model-graphql/review-2.md)
for the full context.

## R2-O1: Unions without a declared discriminator still use first-match decoding

**Defect.** The OpenAPI importer emits a tag-dispatching `Deserialize` only
when a `oneOf`/`anyOf` declares a `discriminator`. A union whose members can
be told apart by a common required string-enum property, but which declares
no `discriminator`, stays untagged. It decodes as the first member that fits,
so an input can land on a structurally compatible but wrong variant. For
example, `{"type":"json_object"}` decodes as `ResponseFormatText` today.

**Where.** The discriminator resolution is in
`define/src/openapi/import/mappings/discriminator.rs`, and the dispatch is
emitted in `gen/src/model_gen.rs`. In the vendored OpenAI document, 9
top-level unions are affected. Five are named-member unions with a required,
disjoint `type`: `TextResponseFormatConfiguration`,
`BetaTextResponseFormatConfiguration`,
`ChatCompletionRequestUserMessageContentPart`, `Filters`, and `BetaFilters`.
Three are Assistants event unions of inline members keyed on `event`:
`MessageStreamEvent`, `RunStreamEvent`, and `RunStepStreamEvent`.
`RealtimeAudioFormats` has an optional `type`. The current behavior is
documented in `docs/io/import-openapi.md` under "Unions and discriminators".

**Why not fixed now.** Choosing a dispatch property the document does not
declare is an import policy decision, not a repair. It changes request-side
public decoding outside the `GetResponse` stream contract the feature covered,
and some unions could qualify on more than one property.

**What fixing it looks like.**

- Decide on the inference rule: a property that is required on every member,
  string-valued with a single `enum`/`const` value per member, and disjoint
  across members. Also decide how to choose between properties when more than
  one qualifies, or whether to refuse in that case.
- Feed the inferred tag through the same `TypeAlias` discriminator metadata,
  so `model_gen` needs no new path, and emit a located warning naming the
  inferred property.
- Regenerate `definitions/src/openai/`, and add decode tests for the five
  named-member unions, including the `json_object` case.
- Decide separately whether inline-member event unions (currently
  `serde_json::Value` variants) should get named members first.

## R2-O2: Inline array alternatives lose their schema on re-export

**Defect.** An inline array member of a union, which is the form the exporter
writes for a `Vec<File>` alternative, imports as free-form JSON
(`serde_json::Value`) tagged `Array`. Re-exporting keeps the alternative but
not its array schema. A second import therefore cannot separate the union by
its array/object shape the way the first import did.

**Where.** Classification of unnamed members (the `JsonTag::Array` fallback)
is in `define/src/openapi/import/mappings/classify.rs`. The round-trip table
in `docs/io/import-openapi.md` describes the behavior. Inline and referenced
unions behave the same way.

**Why not fixed now.** The behavior predates the feature, was not changed by
its repair, and falls under the accepted deferral of broader import/export
fidelity.

**What fixing it looks like.**

- Import an inline array member as a typed `Vec<T>` alternative (with `T`
  resolved from `items`) rather than as `Value`, or keep the original member
  schema alongside the shape so the exporter can write it back.
- Add an export, re-import, and re-export round-trip test that asserts the
  array member's `items` schema survives and the second import still yields
  separated shapes.
