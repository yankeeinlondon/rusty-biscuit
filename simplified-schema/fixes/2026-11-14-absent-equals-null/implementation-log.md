---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/spec.md"
plan: "simplified-schema/fixes/2026-11-14-absent-equals-null/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - simplified-schema/lib/src/claims.rs
    - simplified-schema/lib/src/defaults.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/phase.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/recursive.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/shipped/registry.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/definitions.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/narrowing.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/triggers/envelope.rs
    - simplified-schema/lib/src/triggers/grammar.rs
    - simplified-schema/lib/src/triggers/lint.rs
    - simplified-schema/lib/src/triggers/matcher.rs
    - simplified-schema/lib/generated/shipped.json
    - simplified-schema/schemas/schema.yaml
    - simplified-schema/schemas/yaml-schema.yaml
    - simplified-schema/schemas/schema-trigger.yaml
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/required_presence.rs
    - simplified-schema/lib/tests/l1/not_empty_values.rs
    - simplified-schema/lib/tests/l1/non_null_properties.rs
    - simplified-schema/lib/tests/l1/claimed_values.rs
    - simplified-schema/lib/tests/l1/declaration_grammar.rs
    - simplified-schema/lib/tests/l1/envelope_admission.rs
    - simplified-schema/lib/tests/l1/required_defaults.rs
    - simplified-schema/lib/tests/l1/schema_identities.rs
    - simplified-schema/lib/tests/l1/schema_target_classification.rs
    - simplified-schema/lib/tests/l1/schema_target_inspection.rs
    - simplified-schema/lib/tests/l1/template_type_results.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/shipped.rs
    - darkmatter/lib/generated/shipped.json
    - darkmatter/schemas/darkmatter-config.yaml
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/not_empty_frontmatter.rs
    - darkmatter/lib/tests/l1/non_null_frontmatter.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
    - darkmatter/lib/tests/l1/function_call_binding.rs
    - darkmatter/lib/tests/l1/kind_semantic_checks.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
    - darkmatter/lib/tests/l1/schemas_effective_presence.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_union_schema_matrix.rs
    - darkmatter/lib/tests/l1/shared_acyclic_selection.rs
    - darkmatter/lib/tests/l1/trigger_input_matrix.rs
    - darkmatter/lib/tests/l1/yaml_document_selection.rs
    - darkmatter/lib/tests/l1/yaml_kind_registry.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/standalone_stdio.rs
    - darkmatter/dmls/tests/l1/value_hovers.rs
    - darkmatter/cli/tests/l1/compose_setters.rs
    - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
    - darkmatter/cli/tests/l1/schema_validate_core_types.rs
    - claudine/lib/src/composition/inline_prompt.rs
    - claudine/lib/src/composition/schema/shipped.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/config/persist_tests.rs
    - claudine/lib/src/dispatch/loader/tests.rs
    - claudine/lib/src/steering/automatic/tests.rs
    - claudine/lib/generated/shipped.json
    - claudine/schemas/claudine-config.yaml
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/config_schema_matrix.rs
    - claudine/cli/src/commands/init/mod.rs
    - claudine/cli/tests/l1/compose_presence_parity.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/simplified-schema/SKILL.md
    - .claude/skills/simplified-schema/declaration-narrowing.md
    - .claude/skills/simplified-schema/schema-documents.md
packages:
    - simplified-schema
    - darkmatter
    - darkmatter-lsp
    - darkmatter-cli
    - claudine
    - claudine-cli
decisions_for_review:
    - |-
      I moved the schema migration and artifact regeneration into Phase 1. The plan put
      the schema migration (Rule 16) in Phase 3 and said not to regenerate the shipped
      artifacts from this intermediate state. But once `non-null` is gone from the grammar,
      every shipped schema that spelled it stops parsing, which fails this package's own
      corpus and artifact tests. So Phase 1 also rewrote `non-null` in the three
      simplified-schema envelope schemas, Darkmatter's config schema, its hand-written
      lifecycle `edit` entries, and Claudine's config schema, regenerated all three
      artifacts, and raised every owner's preparation revision.
      Alternatives: keep `non-null` as a temporary alias until Phase 3 (contradicts
      Rule 10 and leaves dead code), or leave the suites red until Phase 3.
      If reversed: put `non-null` back in the grammar and revert the YAML edits. Phase 3
      still has to do the semantic config and lifecycle work either way.
    - |-
      Rule 12's reader change (a null `$schema`/`types` counts as absent) also landed in
      Phase 1, because the migrated envelope schemas now admit null. I added one choice the
      plan did not settle: an untagged document whose `$schema` is null is now plain YAML,
      not a schema document, the same as a document with no `$schema` at all. A tagged
      `kind: schema` with `$schema: null` and a `types` table is a types-only document.
      Alternative: treat the untagged `$schema: null` case as a types-only document, or as
      an error.
      If reversed: one condition in `standalone.rs::recognize_standalone`, plus three
      Darkmatter test cells.
    - |-
      Claudine config migration choices (Rule 16 applied by type):
      - the nine `model-override(non-null)` references became plain `model-override`. It is
        a named union, and a use-site constraint on a union reference accepts only
        `required`, `generated`, or `eager`. Dropping it accepts exactly what `non-null`
        accepted for non-null values.
      - the repository references `event-actions`, `event-matchers`, `repository-steering`,
        and `repository-automatic-steering` became `(not-empty)`. So an empty override
        mapping such as `steering: {}` or `automatic: {}` is now invalid in a repository
        file. It used to mean "override nothing".
      - repository `enabled` and the TTS `prewarm` booleans lost the marker.
      Alternative: leave the repository mapping references unconstrained so `{}` stays a
      legal no-op override.
      If reversed: edit `claudine-config.yaml`, regenerate the artifacts, and adjust
      `steering/automatic/tests.rs::repo_file_matrix`.
    - |-
      Launch keeps the declared value type. That is Rule 3 and the spec's phase paragraph,
      but it reverses an older acceptance test (AC9a). A required, non-eager `string` whose
      expression resolves to `null` now refuses the inline launch, because `null` is not a
      string. Before, the launch projection dropped `required` and tolerated the `null`.
      An omitted required non-eager property is still deferred to completion. I rewrote the
      two tests that pinned the old behavior:
      `claudine/cli/tests/l1/inline_completion_lifecycle.rs::a_conditional_required_property_resolved_to_null_fails_the_inline_launch`
      and
      `claudine/lib/src/composition/schema/tests.rs::required_expression_resolved_to_null_is_a_wrong_type_at_inline_launch`.
      Alternative: during a phase, treat a written `null` on a property that phase does not
      list as absent, by giving the projection's unlisted required properties the optional
      null wrapper. That keeps AC9a.
      If reversed: add that wrapper inside `convert.rs::Lowering` for phase lowering, and
      restore the two tests and
      `darkmatter/lib/tests/l1/schemas_effective_presence.rs::a_phase_that_relaxes_presence_keeps_the_declared_value_type`.
    - |-
      `declared-required` (a `type-definition` option) still also refuses any declaration
      with a null-admitting alternative, even though `required` alone no longer does. Its
      error message promises exactly that, and the trigger/kind `kind` claims rely on a
      non-null value.
      Alternative: let it accept `literal(a; required) | null` now that `required` means
      presence only.
      If reversed: one line in `format.rs`.
    - |-
      Two consumer fixes so the change ships working:
      (1) `claudine init` no longer writes an empty action list for events that have no
      action. `action[](not-empty)` would reject the file it writes. Hook registration still
      uses every event, read from the in-memory bindings.
      (2) Darkmatter's shared operand check now refuses `edit: null`. The schema's optional
      wrapper admits it once `non-null` is gone, and the parser already refused it.
      Alternative for (1): write the empty lists anyway and relax `action[]` in the user
      config. For (2): mark `edit` required in the verb dictionary, which the spec forbids.
      If reversed: `claudine/cli/src/commands/init/mod.rs::write_init_config` and
      `darkmatter/lib/src/markdown/schemas/bound_operands.rs`.
    - |-
      For `schema` / `type-definition` declarations, `not-empty` lowers to three keywords:
      `pattern: "\S"` for reference text, `minProperties: 1` for an inline mapping, and
      `minItems: 1` for a union list. Under the `mapping` option it adds only
      `minProperties`, so a list is reported once, by the mapping check. Without that
      special case the DMLS diagnostic range for `types: []` shrank to one character.
      Alternative: lower it inside the `x-darkmatter-schema` payload instead.
      If reversed: `convert.rs::semantic_type_fragment` and the matching branch in
      Darkmatter's `call_binding/translate.rs`.
message_to_agent: |-
    Phase 1 also pulled several later-phase items forward, because removing `non-null`
    broke parsing of every shipped schema that used it. Read "Schema migration pulled
    forward" in the Phase 1 log before you start.

    - Already done: the mechanical `non-null` to `not-empty` migration (Rule 16) in all
      shipped schemas; Rule 12's reader half (`standalone.rs`, `triggers/envelope.rs`);
      all three artifacts regenerated, with preparation revisions simplified-schema 3,
      Darkmatter 4, Claudine 4. Phase 3 still owns Rule 14's `default(false)` flags,
      Rules 17–20, and the parts of Rule 12 not done yet.
    - If Phase 2 changes what any owner's schemas prepare to, run the root
      `just shipped-schemas` again. Raise the revision only when the payload change
      comes from code alone; the artifacts were regenerated in Phase 1 after the
      revisions were raised.
    - Known red, owned by Phase 3 "Migrate configuration" (Rule 19): four Claudine tests
      in `claudine/lib/tests/l1/config_schema_matrix.rs`, namely
      `nested_variants_keep_their_accepted_shapes` (`tts.provider: null`),
      `sound_rule_guard_and_steering_paths_are_typed` (`steering.automatic.enabled: null`),
      `shared_validators_take_the_full_shape_walk` (`actions.session_start: null`), and
      `a_repository_file_accepts_only_its_override_keys` (repository `steering: null`).
      The schema now admits these nulls as unset, but Claudine's serde readers still
      refuse null ("the configuration cannot be read"). Rule 19's typed null-as-unset
      deserializers fix them; then update the cells to expect a load that inherits.
    - `defaults.rs` still has the old runtime `AmbiguousDefaults` and per-arm policy.
      Phase 2 replaces them; only one module-doc sentence was corrected.
    - No `docs/` page or caller skill was updated, because the plan schedules that for
      Phase 5. Many docs still describe `non-null` and "null cancels `required`". The
      repo skill `.claude/skills/simplified-schema/` was updated.
    - Commit `b9599c601` (author Ken Snyder) appeared mid-phase and holds part of Wave 1
      (`phase.rs`, `format.rs`, `definitions.rs`, Darkmatter `schemas/mod.rs`, and early
      hunks of other files). The rest of Phase 1 is uncommitted on top of it.
    - Claudine CLI tests need `--features test-fixtures`; without it,
      `document_environment_routes` fails spuriously. Use `just test` in `claudine/`.
---

# Implementation Log for 2026-11-14-absent-equals-null (6 phases)

## Phase 1

### Wave 1 — Separate presence (complete)

- `PropertyDef::is_required` / `PropertyAtom::is_required` now return the resolved `required` marker (written, inherited, or hoisted from a union arm); `admits_null` decides value typing only (`simplified/types.rs`).
- Lowering (`simplified/convert.rs`): a marked union keeps its parent `required` entry even with a `null`/`unknown` arm; a null-admitting atom lowers bare and is listed when marked (unless `generated`). The optional null wrapper is unchanged.
- The resolver no longer strips a use-site `required` from a reference to a null-admitting stored definition (`resolve/recursive.rs::apply_reference_postfix`).
- Phase projection: `phase::{project, project_definitions}` replaced by `phase::lower(schema, definitions, phase)`, implemented as a phase option on the lowering (`Lowering::lists_required`). Value fragments always use the authored marker; only parent `required` arrays follow launch (eager + required) / completion (every required, `generated` included) policy, including inside `$defs` bodies and match copies. `DefinitionTable::map_bodies` (its only user was the old projection) was removed. `PROJECTION_REVISION` 1 → 2; pinned projection identity in `tests/l1/schema_identities.rs` updated (`e89539b57da19d99` → `2da28dfe4c5dcd41`). `MERGE_ALGORITHM_REVISION` untouched.
- Darkmatter `EffectiveSchema::phase_projection` now keeps its original AST and definitions and lowers through `phase::lower`.
- Trigger matching: a written `null` meets `required` and then must satisfy the type (`triggers/matcher.rs`); `unknown(required)`/`null(required)` conditions are gates, not vacuous guards (`triggers/lint.rs`).
- `declared-required` (format option) keeps its stricter meaning: required **and** no null-admitting alternative (`format.rs`), since its message promises that.
- Darkmatter's call-binding mirror of the lowering (`compose/expression/call_binding/translate.rs`) got the same presence change so it stays in parity.
- Doc comments corrected: `claims.rs::ClaimCause` (a `null | literal(a; required)` is now `NullAdmitted`, not `NotRequired`), DMLS `is_required`, `defaults.rs` module doc sentence, `resolve/recursive.rs` bullet and `UseSummary::admits_null`.

Tests (requirement → test):

- presence table rows without defaults, strict + soft, missing vs written null, query view, nullable unions with hoisted markers, local and recursive named types, nested objects / list items, positional tuple null contract → new `simplified-schema/lib/tests/l1/required_presence.rs` (10 tests, registered in `tests/l1/main.rs`).
- required null-admitting with default filled when missing; without default needs key → `required_defaults.rs` (`a_required_null_admitting_property_is_filled_when_missing`, `…_without_a_default_needs_its_key`).
- lowering unit tests rewritten: `convert.rs::{unknown_accepts_null_and_every_value_and_required_demands_the_key, null_admitting_properties_lower_bare_and_enter_required_when_marked}`; `types.rs::required_marks_presence_independently_of_null_admission`.
- trigger presence → `triggers/matcher.rs::presence_follows_the_shared_calculation`, `triggers/lint.rs::required_conditions_are_gates_whether_or_not_they_admit_null`, Darkmatter `schemas_effective_presence::trigger_conditions_use_the_same_presence`.
- phases (launch/completion/generated/eager) and the launch-keeps-value-type regression → Darkmatter `schemas_effective_presence::{null_admitting_required_values_need_their_key_at_the_phases_that_demand_it, a_phase_that_relaxes_presence_keeps_the_declared_value_type, inherited_and_postfixed_required_survives_a_null_arm, composition_requires_written_keys_for_null_admitting_required_properties}` (F2, in-process host API).
- claimed values / template types updated: `claimed_values::problems::a_required_declaration_that_admits_null`, `template_type_results::required_suffix_over_null_admitting_types_requires_the_key`.

### Wave 2 — Replace `non-null` with `not-empty` (complete)

- `Constraint::NonNull`, `marks_non_null` (both), `non_null_problem`, `UnionInvariant::NonNull` (and `union_invariant_problem`'s `check_non_null` flag), `resolve.rs::check_non_null_use`, and the resolver's now-dead `UseSummary::admits_null` are gone. Serialization, `$constraints` export, narrowing, subtype markers, trigger match-safety, and the convert `non-null` lowering paths no longer mention it.
- `non-null` (any spelling: bare, with arguments, after `[]`, inside a union, or `$constraints: { non-null: true }`) is now a located `SchemaError::Grammar` carrying `grammar::NON_NULL_REMOVED`, which points authors at `not-empty` / `required`.
- `not-empty` lowering by family (`simplified/convert.rs`): text (`string`, `alphanumeric`, `yaml`, `json`) → `pattern: "\S"`; lists → `minItems` merged through the same `Bounds` as `min(n)` (so `string[](not-empty)` exports exactly as `string[](min(1))`, and `not-empty; max(0)` is a contradiction naming `not-empty`); `object`, inline objects, dictionaries, and `$constraints: { not-empty: true }` (root or nested) → `minProperties` via `apply_object_arity`; `schema` / `type-definition` → `pattern` + `minProperties` + `minItems` (each keyword applies only to its JSON type, so blank reference text, `{}` and `[]` all fail while declaration validation is unchanged); under the `mapping` option only `minProperties`, since the mapping check already refuses text and lists. `number`, `boolean`, `null`, `unknown` refuse it with a dedicated "\"empty\" has no meaning" message. Trigger condition matching judges `not-empty` by the written value's shape (text / mapping / list), including array-level `string[](not-empty)`.
- Darkmatter: the call-binding lowering mirror (`call_binding/translate.rs`) dropped `non-null`; the help/completion catalog (`markdown/schemas/about.rs`) dropped every `non-null` descriptor (type lists, constraint, block, match-safe) and now advertises `not-empty` on `object`, `schema`, `type-definition`, and at array level, plus a `not-empty: true` block descriptor. DMLS completion test updated to expect `not-empty` and withhold `non-null`.

**Schema migration pulled forward (see decisions_for_review).** Removing the grammar made every shipped schema that spelled `non-null` unparseable, which fails the library's own corpus and artifact tests. So this phase also migrated, mechanically per Rule 16:

- `simplified-schema/schemas/{schema,yaml-schema,schema-trigger}.yaml`: `description`/`categories` → `not-empty`; optional `types` → `schema(mapping; not-empty)`; types-only arm's `types` → `schema(mapping; required; not-empty)`; optional `$schema` carriers → `schema(not-empty)`.
- `darkmatter/schemas/darkmatter-config.yaml`: `file-read-pattern[](not-empty)`.
- `darkmatter/schemas/partials/lifecycle.yaml` (hand-written `edit-short`/`edit-long` only, outside generated regions): `non-null` dropped from the edit string, tuple, Boolean flags, and the long-form object (none of those has a meaningful "empty" beyond what `not-empty`/`required` already express). The `default(false)` flag work of Rule 14 is left to Phase 3.
- `claudine/schemas/claudine-config.yaml`: TTS `provider` and the 16 matcher strings → `string(not-empty)`; the 16 `action[]` event lists → `action[](not-empty)`; `prewarm` → `boolean(default(true))`; repository `enabled` → `boolean`; `repository-automatic-steering`, `event-actions`, `event-matchers`, `repository-steering` references → `(not-empty)`; the nine `model-override(non-null)` union references → bare `model-override` (a use-site constraint on a named *union* reference only admits `required`/`generated`/`eager`, and dropping preserves exactly what `non-null` used to accept for non-null values).
- Rule 12's reader half: `simplified/standalone.rs::RecognizedStandalone::{payload_key, payload, local_types}` now treat a null carrier as absent (a tagged `$schema: null` beside `types` is a `types`-only document; `exports_schema` means a usable export; a null optional `types` is absent); `triggers/envelope.rs` treats `$schema: null` as matcher-only instead of an error. `NotSchemaReason::TypesOnly` reads "with no `$schema` value".
- `PREPARATION_REVISION` bumped for all three owners (simplified-schema 2→3, Darkmatter 3→4, Claudine 3→4) because lowering code changed their payloads, and all three artifacts were regenerated with one root `just shipped-schemas` run (it reported all three regenerated).

Tests (requirement → test):

- every `not-empty` family (string, yaml, json, list, inline object, dictionary, `object`, `schema(mapping)`, `schema` text/mapping/union), null and missing accepted, list ≡ `min(1)`, merged/contradictory list bounds, item vs list constraint, invalid scalar families, required + not-empty, block form → new `simplified-schema/lib/tests/l1/not_empty_values.rs` (replaces `non_null_properties.rs`, registered in `main.rs`).
- old spellings refused with the located message (type string, `[]` suffix, arguments, union, YAML list union, `$constraints` block) → `declaration_grammar.rs::rejected::the_removed_non_null_marker_is_a_located_error_in_every_spelling`; round-trip and block export now use `not-empty`.
- template-returned `non-null` → `template_type_results::a_returned_type_spelling_the_removed_non_null_is_not_a_type_definition`; nested substitution parity → `not_empty_after_nested_substitution_matches_the_static_text`.
- trigger conditions → `triggers/matcher.rs::{not_empty_guard_passes_when_absent_or_null_and_defeats_on_empty_values, the_removed_non_null_marker_is_refused_in_a_condition}`.
- envelopes (null metadata/types absent, `$schema: null` + types = types-only, yaml-schema null export refused, no usable carrier refused, empty types refused, null-typed payload property kept) → `envelope_admission::{null_optional_metadata_and_types_read_as_absent, a_tagged_null_schema_beside_types_is_a_types_only_document, a_null_schema_without_a_usable_alternative_is_rejected, a_null_typed_property_inside_a_payload_is_a_declaration}`; field-shape rows updated in `schema_target_classification.rs`, `schema_target_inspection.rs`, `resolve.rs::malformed_local_types_are_schema_errors_with_authored_locations`.
- Darkmatter frontmatter path → new `darkmatter/lib/tests/l1/not_empty_frontmatter.rs` (replaces `non_null_frontmatter.rs`).

Additional library and host changes found while running the Darkmatter suite:

- `resolve.rs::is_array_container_constraint` now admits `not-empty`, so `action[](not-empty)` over the named union `action` resolves (Claudine's `event-actions` import failed without it).
- `simplified/standalone.rs::recognize_standalone`: an informal (untagged) document is claimed only by a non-null `$schema`; `$schema: null` / `$schema:` alone is ordinary YAML, like a document with no `$schema` (consistent with "null is absent"; a tagged `kind: schema` keeps its `types` fallback).
- Darkmatter's call-binding translator (`call_binding/translate.rs`) mirrors every `not-empty` family the lowering gained: array-level `minItems`, `object`/inline/dictionary/block `minProperties`, `yaml`/`json` blank-text assertion, and `schema`/`type-definition` text/mapping/list variants. New conformance rows: `not_empty_{string,list,union,object_property,yaml,json,object,inline_object,dictionary}`, `required_null`, `required_nullable_union`.
- `markdown/schemas/bound_operands.rs`: with `non-null` gone from `edit-short`, the schema's optional wrapper admitted `edit: null`; the shared operand check now refuses it (`EDIT_WITHOUT_TARGET`), keeping `lifecycle_edit_schema_parity` green without marking `edit` required in the verb dictionary.
- Darkmatter recursive-schema goldens re-blessed (`DARKMATTER_BLESS_RECURSIVE_GOLDEN=1`): the only differences are `edit`'s optional null wrappers and `edit-short` now being stored once under `$defs` (it is shareable once its top level carries no `non-null`).
- Pre-existing finding (not caused by this work, not changed): Darkmatter's call-binding translator converts a native number/boolean into `schema`/`type-definition` declaration text (`4` → `"4"`) while the library's JSON-Schema translator refuses it, for a plain `schema` declaration as much as for `schema(not-empty)`; there is no conformance row for declarations, so no row was added.

Darkmatter test updates (rules changed, assertions follow): `schemas_effective_presence.rs` (rewritten for Wave 1), `not_empty_frontmatter.rs` (replaces `non_null_frontmatter.rs`), `coercion_translator_conformance.rs`, `dynamic_schema_slot_surfaces.rs`, `function_call_binding.rs` (object-argument properties follow the presence rule; positional arguments keep the positional contract), `kind_semantic_checks.rs`, `meta_schema_phase3.rs`, `recursive_schema_types.rs`, `schema_embedded_trigger_bundles.rs`, `schema_enhancements_matrix/envelope.rs`, `schema_extraction_characterization.rs`, `schema_grammar_descriptors.rs`, `schemas_conjunctive_lowering.rs`, `schema_yaml_spellings.rs`, `schemas_named_import_errors.rs`, `schemas_source_projection.rs`, `schemas_union_schema_matrix.rs`, `shared_acyclic_selection.rs`, `trigger_input_matrix.rs`, `yaml_document_selection.rs`, `yaml_kind_registry.rs`; Darkmatter CLI `compose_setters.rs`; DMLS `providers/frontmatter.rs` completion test.

### Phase 1 verification

Gates (run after every change was in place, with no subagents):

- `simplified-schema`: `just test` exited 0. It ran 2177 L1 tests (`work-counters,terminal`) and 1821 minimal tests (`--no-default-features`), all passed, and the recipe reported "2177 test(s), none at or above 5.0s". `just lint` exited 0.
- `darkmatter`: `just lint` exited 0, after removing the `NotASchema` outcome variant that my envelope-matrix edit left unused. `cargo nextest run -p darkmatter`: 11656 passed. `-p darkmatter-lsp -p darkmatter-cli`: 4475 passed.
- `claudine`: `just lint` exited 0. `cargo nextest run -p claudine --features work-counters -p claudine-cli --features claudine-cli/test-fixtures`: 11787 passed, 4 failed. The four are the null-valued config cells listed in `message_to_agent`, which wait on Phase 3's Rule 19 work.
- Load-sensitive tests, measured alone (host load average 44–60 during the package runs): `claudine_schemas::{cancelled_conditional_depth_is_the_parsers_alone_to_limit, group_/nested_group_/nested_task_/task_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals, shared_payload_properties_have_identical_types}` took 5.1–6.4 s in the package run (under the 10 s allowance) and 1.7–2.2 s alone. `composition::preflight::tests::a_saved_choice_…`, `defaults_tests::launch::a_required_default_is_defaulted_rather_than_deferred`, `a_caller_supplied_prompt_satisfies_the_eager_launch_gate_transiently`, and `inline_launch_type_checks_a_present_required_non_eager_value` took 0.7–1.1 s alone. An early `claudine-cli` run without `test-fixtures` showed 15 spurious `document_environment_routes` failures and a 10.6 s `lifecycle_message_drain` run; both pass with the recipe's features.
- Cross-OS check: not run. This phase changes no OS-specific code (paths, processes, or `cfg` blocks); CI covers the other operating systems.
- An external commit, `b9599c601 fix(simplified-schema): keep required presence independent of null` (author Ken Snyder, 22:42), appeared during the phase. It contains a subset of the Wave 1 edits. I did not make it. The rest of Phase 1 sits uncommitted on top of it.
