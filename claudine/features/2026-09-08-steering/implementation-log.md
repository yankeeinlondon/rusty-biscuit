---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/features/2026-09-08-steering/spec.md
plan: claudine/features/2026-09-08-steering/plan.md
implemented_by: claude/opus
started_phase: 1
---
# Implementation Log for 2026-09-08-steering (8 phases)

## Phase 1

Started 2026-09-28.

### Design decisions

- **Standalone generated artifact, not a `ProviderInfo` field.** Steering facts
  are emitted into `lib/src/steering/generated.rs` through the same full-scope
  artifact path as `lib/src/stream/providers/vocabulary.rs`. The mapping
  registry is exhaustive over serialized `ProviderInfo` fields; steering is a
  runtime selection input, not a describable provider property, so its field
  ownership is declared by the steering loader (`gen/src/steering_catalog.rs`)
  exactly as the vocabulary loader declares its own. This departs from the
  plan's literal "extend `registry.rs`" wording; the registry is unchanged and
  `catalog.json` gains nothing.
- **Shared rule functions live in `claudine-catalog-types`.** Operation/state
  compatibility, loop-rescue suitability, prompt-return receipt timing, and
  required assertion coverage are pure methods on the shared vocabulary, so the
  generator's checker and the library's runtime eligibility evaluate one
  implementation.
- **Typed assertions required a research contract change (revision 4).**
  Verification rows were prose-only, which cannot support deterministic
  activation. Revision 4 adds `id` and `assertion_kinds` to verification rows.
  Only Pi carries verification rows; its seven records were backfilled from
  their existing prose. The other nine documents change only their revision
  number and a `changes` entry.
- **Activation grants and reviewed adapter bindings are hand-owned policy**
  (`docs/providers/steering-activation.yaml`), separate from research facts.
  The generator validates every grant deterministically, then emits the grants
  into the generated file as a separate static. The library's hand-written
  `IMPLEMENTED_ADAPTERS` must equal the reviewed adapter set (L1 guard), and
  runtime eligibility requires both. Phase 1 ships with no adapters and no
  grants, so every case is unavailable with a specific reason.
