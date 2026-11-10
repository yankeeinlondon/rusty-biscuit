# Slots and templates: code map

Loaded from the `simplified-schema` skill. The user-facing reference is
[slots.md](../../../simplified-schema/docs/topics/language/slots.md); this
page maps its rules to the code.

## Rules

- **Templates are passive and fail closed.** A `{{ … }}` element of
  `enum`/`suggest`, or the whole argument of `literal`/`default`/`min`/`max`/
  `pattern`, parses to `Constraint::Dynamic` / `Constraint::TemplateConstraint`
  (a **template constraint**; only when a slot is present; static lists stay `Members`/`Suggest`). The
  grammar reads a slot with `scan::scan_expression_at`; `source.rs`'s
  argument scanner skips it whole and `project_atom_suggestions` projects its
  spans. Every lowering entry returns `SchemaError::UnresolvedSlots` first
  (`slots::reject_unresolved`), so a reader that does not know slots never
  sees one; queries/completion set `pending`. A new `Constraint` reader that
  could see a template must decide its slot case (see
  [slots.md](../../../simplified-schema/docs/topics/language/slots.md)).
  Tests: `src/simplified/grammar/slot_tests.rs`,
  `tests/l1/{slot_spans,slot_templates}.rs`, `schema_object.rs::templates`.
- **Resolution replays, never patches.** A host (`slots::SlotHost`: passive
  `admit`, effectful `resolve`, `template_reads`, `is_canceled`) supplies values; the
  library reruns the same resolution inside `slots::with_values`, and every
  lowering site calls `slots::apply_active` right after import expansion
  (`substitute`: result shapes, dedupe, stricter-bound/equal-default merge,
  pattern cap). Public entry points (`resolving_entry` in `resolve.rs`)
  attach `UnresolvedSlots::pending` (`PendingSchema`, replaying the recorded
  `read_sources()` texts); `SimplifiedSchema::resolve_slots` reruns the
  layer's own `Rebuild`. Expansion stamps `Slot::origin` with the file a
  slot was written in (`DefNamespace::file`); a new lowering site must call
  `apply_active` too (`apply_active_to_arm` for a root-union arm, never
  pruned by `omitting`; the union then marks the arm's template
  `is_union_arm` with the names every arm declares, `resolve.rs::pending_union_arm`,
  read by `declared_properties()`). Tests: `tests/l1/dynamic_slots.rs`.
- **A template type is the pending type.** A slot supplying a property's
  whole definition parses to the atom `TypeExpr::Slot` (suffix read like a
  reference's), allowed only as a property type, union arm, or tuple element;
  every other position is refused by `grammar::SlotPosition` (one message
  shape, rewrite sentence only for names/roots/paths). Readers answer it
  explicitly, never through a `_ =>` arm: no shape facts, `pending` in queries
  and completion, `PropertyDef::requiredness()` = `Pending` (the boolean
  presence helpers assume a resolved definition). Resolution
  (`slots/template_type.rs::substitute_types`, run by `apply_active` before
  constraint substitution, so `apply_active` takes an `ExpansionSite` and
  returns late `imports`/`searched`) parses the value once
  (`definition_from_value`: string or array only; `NestedSlot`,
  `LocalReference`), then expands it twice through `expand_imports` as the
  named type `resolve::TEMPLATE_TYPE_NAME` of the slot's defining file
  (`resolve::expand_template_definition`): bare, for `ReachableSlot`,
  slot-pattern and lowering checks, then with the written suffix (merge
  failures are `SuffixConflict`/`InvalidExpansion`). Arms are spliced like
  `expand_union_arms` does, in property unions and in grouped unions at every
  depth (`TypeSubstitution::atom` substitutes a group, never only traverses
  it); a slot in an array item union leaves `requiredness()` `Pending`. Every read the expansion causes passes
  `SlotHost::template_reads` (`slots/reads.rs`: `ReadGate`, captured by
  `ImportEngine::new` only inside `reads::expanding`; each probe is admitted
  by `select::select_schema_file_checked` immediately before it is made, in
  the ungated selector's order with its first-match stop (biscuit-file's
  `resolve_detailed_with_presence` gates each candidate and lets an open
  buffer (`buffers::is_open`) match at its own candidate, gated and ungated
  alike), so an unreached root or candidate is never asked about; a refusal
  is sticky and becomes `SlotResolutionError::Refused`). Tests:
  `tests/l1/template_types_passive.rs`, `template_type_{results,imports,refusals}.rs`.
  The library never evaluates a slot and never reads its targets; approval,
  caching, and evaluation are the host's (Darkmatter's `schemas/slots/`).

## Slot and template APIs

| Need | API |
|---|---|
| Resolve a template's `{{ … }}` slots | `schema.resolve_slots(&host)`, or `UnresolvedSlots::pending` → `PendingSchema::resolve(&host, &SlotRunRequest { context })` |
| Resolve only the top-level properties a higher layer leaves | `pending.without_properties(&names)` (drops them and the definitions only they reach; `omitted_properties()` belongs in a cache key), then `resolve` or, with no slot left, `resolve_without_slots()` |
| The top-level names a template declares in every alternative (a root-union arm's: every arm's), unevaluated | `pending.declared_properties()` |
| Read a slot's data file honoring editor buffers | `resolve::buffers::read(path)` |
| Resolve an already-parsed inline `$schema` value with its slots spanned in the file it came from | `resolve::resolve_authored_yaml_schema(value, owner, Some(&AuthoredSchemaValue { file, text, mapping }), …)` (`resolve_authored_schema_with_roots` for JSON); the value is located and verified first, else no position is claimed |
| The file and text a slot's spans index, without reading anything | `pending.slot_text(&slot)`, `SlotResolutionFailure::of_pending(error, &pending).slot_text(&slot)` |
| Judge each schema source before it is read (a host's slot approvals) | `resolve::buffers::guarded(guard, \|\| …)`; a refusal fails the read as `PermissionDenied`; `unguarded` suspends the guards |
