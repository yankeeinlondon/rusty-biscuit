---
total_phases: 6
created: 2026-11-14
phase: 6
agent: codex/default
yolo: 'true'
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
source_files_during_phase_2:
- simplified-schema/lib/src/defaults.rs
- simplified-schema/lib/src/prepared.rs
- simplified-schema/lib/src/outcome.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/coerce.rs
- simplified-schema/lib/src/simplified/convert.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/tests/l1/required_defaults.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/compose/tests/schema.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/effective_default_preparation.rs
- darkmatter/lib/tests/l1/composition_defaults.rs
- darkmatter/lib/tests/l1/config_fields.rs
- darkmatter/lib/tests/l1/config_load.rs
- darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
- darkmatter/lib/tests/l1/recursive_schema_merge.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
- claudine/cli/src/commands/actions.rs
- claudine/cli/src/commands/config_tui/tabs/messenger/input.rs
- claudine/cli/src/commands/config_tui/tabs/messenger/modal.rs
- claudine/cli/src/commands/config_tui/tabs/messenger/render.rs
- claudine/cli/src/commands/config_tui/tabs/messenger/tests.rs
- claudine/cli/src/commands/hooks/list.rs
- claudine/cli/src/commands/hooks/variables.rs
- claudine/cli/src/commands/init/mod.rs
- claudine/cli/src/commands/init_wizard.rs
- claudine/cli/tests/common/drain_interrupt.rs
- claudine/cli/tests/common/webhook_listener.rs
- claudine/cli/tests/l1/agent_cwd.rs
- claudine/cli/tests/l1/command_routing.rs
- claudine/cli/tests/l1/detached_audio.rs
- claudine/cli/tests/l1/error_guards/transport-allow.toml
- claudine/cli/tests/l1/handle_blocking_output.rs
- claudine/cli/tests/l1/handle_message_drain.rs
- claudine/cli/tests/l1/handle_repo_config.rs
- claudine/cli/tests/l1/pi_managed_rpc.rs
- claudine/cli/tests/level2/level2_lifecycle_control.rs
- claudine/lib/benches/runtime_hot_paths.rs
- claudine/lib/generated/shipped.json
- claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
- claudine/lib/src/composition/lifecycle/mod.rs
- claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
- claudine/lib/src/composition/lifecycle/tests/grammar.rs
- claudine/lib/src/composition/preflight.rs
- claudine/lib/src/composition/prepare.rs
- claudine/lib/src/composition/prepare/service/tests.rs
- claudine/lib/src/composition/schema/classify.rs
- claudine/lib/src/composition/schema/defaults_tests.rs
- claudine/lib/src/composition/schema/launch_defaults.rs
- claudine/lib/src/composition/schema/mod.rs
- claudine/lib/src/composition/schema/tests.rs
- claudine/lib/src/composition/sequence/task/mod.rs
- claudine/lib/src/config/claudine_config.rs
- claudine/lib/src/config/claudine_config/tests.rs
- claudine/lib/src/config/json_conversion.rs
- claudine/lib/src/config/load.rs
- claudine/lib/src/config/merge.rs
- claudine/lib/src/config/messaging_block.rs
- claudine/lib/src/config/messaging_block/tests.rs
- claudine/lib/src/config/mod.rs
- claudine/lib/src/config/persist.rs
- claudine/lib/src/config/persist_tests.rs
- claudine/lib/src/config/tts.rs
- claudine/lib/src/dispatch/loader/tests.rs
- claudine/lib/src/steering/automatic.rs
- claudine/lib/src/steering/automatic/tests.rs
- claudine/lib/tests/config/repository-control.yaml
- claudine/lib/tests/config/user-control.yaml
- claudine/lib/tests/l1/claudine_schemas_matrix.rs
- claudine/lib/tests/l1/config_json_conversion.rs
- claudine/lib/tests/l1/config_schema_matrix.rs
- claudine/schemas/action.yaml
- claudine/schemas/claudine-config.yaml
- claudine/schemas/lifecycle.yaml
- darkmatter/cli/tests/l1/compose_edit_action.rs
- darkmatter/cli/tests/l1/compose_lifecycle.rs
- darkmatter/cli/tests/l1/compose_schema.rs
- darkmatter/dmls/src/overlay/event_views.rs
- darkmatter/dmls/src/providers/lifecycle_host_tests.rs
- darkmatter/dmls/tests/l1/edit_action_editor.rs
- darkmatter/dmls/tests/l1/lsp_session.rs
- darkmatter/dmls/tests/l1/required_defaults.rs
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/src/config/load.rs
- darkmatter/lib/src/lifecycle/action_shape.rs
- darkmatter/lib/src/lifecycle/executor/tests.rs
- darkmatter/lib/src/lifecycle/host_profile.rs
- darkmatter/lib/src/lifecycle/host_profile/tests.rs
- darkmatter/lib/src/lifecycle/md_host.rs
- darkmatter/lib/src/lifecycle/md_host/tests.rs
- darkmatter/lib/src/lifecycle/mod.rs
- darkmatter/lib/src/lifecycle/parse.rs
- darkmatter/lib/src/lifecycle/preflight/tests.rs
- darkmatter/lib/src/lifecycle/run_guard/tests.rs
- darkmatter/lib/src/lifecycle/schema_projection.rs
- darkmatter/lib/src/lifecycle/tests.rs
- darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/compose/tests/schema.rs
- darkmatter/lib/src/markdown/schemas/bound_operands.rs
- darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-action.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-claudine.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-lifecycle.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence-types.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/claudine-schemas-sequence.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-inline-compose.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-sequence.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-with-document-schema.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
- darkmatter/lib/tests/l1/absent_property_contract.rs
- darkmatter/lib/tests/l1/composition_defaults.rs
- darkmatter/lib/tests/l1/content_policy_compose_contract.rs
- darkmatter/lib/tests/l1/directive_target_analysis.rs
- darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
- darkmatter/lib/tests/l1/lifecycle_edit_action.rs
- darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs
- darkmatter/lib/tests/l1/lifecycle_generated_operands.rs
- darkmatter/lib/tests/l1/lifecycle_schema_parity.rs
- darkmatter/lib/tests/l1/not_empty_frontmatter.rs
- darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
- darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/trigger_input_matrix.rs
- darkmatter/lib/tests/l1/variable_type_projection.rs
- darkmatter/schemas/darkmatter-config.yaml
- darkmatter/schemas/partials/fm.yaml
- darkmatter/schemas/partials/lifecycle.yaml
- darkmatter/schemas/partials/md-lifecycle.yaml
- darkmatter/schemas/partials/style.yaml
- simplified-schema/lib/src/simplified/standalone.rs
- simplified-schema/lib/src/triggers/envelope.rs
- simplified-schema/lib/tests/l1/main.rs
- claudine/lib/src/config/wire.rs
- claudine/lib/src/config/wire_tests.rs
- darkmatter/lib/src/lifecycle/tests/authored_flags.rs
- simplified-schema/lib/tests/l1/envelope_null_carriers.rs
docs_updated_during_phase_3:
- claudine/cli/README.md
- claudine/docs/topics/automatic-steering.md
- claudine/docs/topics/configuration.md
- claudine/docs/topics/configuring-actions.md
- darkmatter/docs/inline/schema-validation.md
- darkmatter/docs/topics/configuration.md
- darkmatter/docs/topics/markdown-formatting.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/claudine/SKILL.md
- .claude/skills/claudine/cli-commands.md
- .claude/skills/claudine/hook-actions.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/simplified-schema/schema-documents.md
- claudine/docs/use-skills/use-claudine/config.md
packages:
- simplified-schema
- darkmatter
- darkmatter-lsp
- darkmatter-cli
- claudine
- claudine-cli
source_files_during_phase_4:
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/input_robustness.rs
- simplified-schema/lib/tests/l1/required_defaults.rs
- simplified-schema/lib/tests/l1/not_empty_values.rs
- simplified-schema/lib/tests/l1/envelope_null_carriers.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/frontmatter_extraction.rs
- darkmatter/lib/tests/l1/lifecycle_schema_parity.rs
- darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/compose_loader_wiring.rs
- claudine/lib/tests/l1/main.rs
- claudine/lib/tests/l1/config_keyed_entries.rs
- claudine/lib/tests/l1/config_schema_matrix.rs
- claudine/lib/tests/l1/config_json_conversion.rs
- claudine/lib/src/composition/prepare/service/tests.rs
- claudine/cli/tests/l1/main.rs
- claudine/cli/tests/l1/config_json5_wiring.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
- darkmatter/lib/tests/l1/schema_phase_validation.rs
docs_updated_during_phase_5:
- claudine/docs/topics/composition.md
- claudine/docs/topics/configuration.md
- claudine/docs/topics/configuring-actions.md
- claudine/docs/topics/flow-control/lifecycle.md
- claudine/docs/topics/flow-control/looping.md
- claudine/docs/topics/messaging.md
- claudine/docs/topics/schemas.md
- darkmatter/docs/cli/schema.md
- darkmatter/docs/cli/set.md
- darkmatter/docs/inline/schema-validation.md
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/docs/topics/frontmatter.md
- darkmatter/docs/topics/lifecycle/error-handling.md
- darkmatter/docs/topics/lifecycle/index.md
- darkmatter/docs/topics/numbers.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/docs/topics/schemas/schema-targeting.md
- simplified-schema/docs/topics/conversion.md
- simplified-schema/docs/topics/custom-vocabulary.md
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/foundations.md
- simplified-schema/docs/topics/global-type-constraints.md
- simplified-schema/docs/topics/language-core.md
- simplified-schema/docs/topics/language/index.md
- simplified-schema/docs/topics/language/local-type-references.md
- simplified-schema/docs/topics/language/objects-and-named-types.md
- simplified-schema/docs/topics/language/schema-documents.md
- simplified-schema/docs/topics/language/schema-types.md
- simplified-schema/docs/topics/language/slots.md
- simplified-schema/docs/topics/language/subtyping.md
- simplified-schema/docs/topics/language/types.md
- simplified-schema/docs/topics/language/unions.md
- simplified-schema/docs/topics/simplified-schema-struct.md
- simplified-schema/docs/topics/types/basic-types.md
- simplified-schema/docs/topics/validation.md
- simplified-schema/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/claudine/SKILL.md
- .claude/skills/claudine/architecture.md
- .claude/skills/claudine/cli-reference.md
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/compose.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/frontmatter.md
- .claude/skills/darkmatter/lifecycle.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/simplified-schema/declaration-narrowing.md
- .claude/skills/simplified-schema/schema-documents.md
- claudine/docs/use-skills/use-claudine/composition.md
- claudine/docs/use-skills/use-claudine/config.md
- claudine/docs/use-skills/use-claudine/library.md
- claudine/docs/use-skills/use-claudine/lifecycle.md
- darkmatter/docs/use-skills/use-darkmatter/SKILL.md
- darkmatter/docs/use-skills/use-darkmatter/authoring.md
- darkmatter/docs/use-skills/use-darkmatter/cli.md
- darkmatter/docs/use-skills/use-dmls/schemas.md
- .claude/skills/simplified-schema/defaults.md
- .claude/skills/simplified-schema/shipped-and-identities.md
---

# Absent values, defaults, and written presence

## Summary and Definition of Done

Implement `2026-11-14-absent-equals-null` in the shared SimplifiedSchema machinery and its Darkmatter and Claudine consumers. Missing properties and explicit nulls have the same value semantics; `required` independently checks written presence after default application. Apply optional and required defaults, remove `non-null`, extend `not-empty`, preserve positional argument rules, and align lifecycle parsing with soft conversion. Composition must stop manufacturing null-valued frontmatter bindings.

Completion means:

- Every row of the specification's presence/default table passes through the public strict and soft APIs, with correct ownership, default reports, locations, and repeat-validation stability.
- Union defaults are deterministic schema declarations, conflicting defaults fail preparation, and runtime `AmbiguousDefaults` no longer exists. Raw JSON Schema receives the same default application without altered assertions.
- Envelope admission, trigger payloads, runtime phases, configuration inheritance, composition, whole-YAML validation, CLI validation, and DMLS use the same rules. Unknown null-valued keys still fail closed-object checks.
- Lifecycle schema validation and execution binding agree on flags and `edit` operands. Missing targets, invalid arity, duplicate authored flags, and `set` flags remain errors. Verification launches no real editor, browser, speech, shell action, or messaging service.
- Shipped schemas, all three generated artifacts, relevant docs, and published caller skills agree with the implementation. The scoped nextest and lint checks pass.
- The implementation log records the migrations below, completed-value changes, commands and outcomes, and review evidence. Work remains in this worktree; no commit, push, lifecycle-directory move, or human gate belongs to this plan.

Paths below are repository-relative. Waves are globally numbered. Tasks within a wave may run concurrently only where their stated ownership is disjoint; each wave completes before the next starts. Subagent briefs must state that the entire session tree is non-interactive. Each code task includes its focused regression tests and a pass over documentation comments of changed symbols.

## Phase 1 — Presence and language contracts

### Necessary Rules

1. **Presence and null admission are independent.** `PropertyDef::is_required` and `PropertyAtom::is_required` return the resolved required marker, including hoisted union markers. Keep `admits_null` solely for value typing. Preserve pending-template presence. Evidence: `simplified-schema/lib/src/simplified/types.rs` currently returns `marked && !self.admits_null()` in both calculations; `convert.rs::union_property_to_schema` also computes presence separately and must change. Consumers include query/view APIs, `phase.rs`, `triggers/matcher.rs`, composition property projections, and DMLS completion.

2. **Keep ordinary optional null wrappers and authored raw assertions.** Export required names in their parent's `required` array and retain declared value schemas. Optional SimplifiedSchema properties keep their existing null wrapper; a non-null default removes null from the *completed runtime value*, not from the authored JSON Schema presence assertion. Raw schemas receive no SimplifiedSchema wrappers or rewritten `required` arrays. Evidence: the specification's export section and `convert.rs::{property_def_to_schema,atom_fragment_without_null_wrap,object_body_from_shape}`. A plain third-party validator still does not apply exported defaults.

3. **Phase projection relaxes presence only.** Replace AST-mutating `phase::{project,project_definitions}` with `phase::lower(schema: &SchemaRoot, definitions: &DefinitionTable, phase: SchemaPhase) -> Result<Value, SchemaError>`. Add an internal phase option to `convert.rs::Lowering`; use the original required marker for value fragments and the existing eager/generated phase policy for emitting parent `required` arrays, including definition bodies. `EffectiveSchema::phase_projection`, the only production caller of the old pair, retains its original AST/definitions and uses this lowered projection with the existing phase merge. Do not delete markers before null-wrapper lowering. Evidence: `phase.rs::project_atom` currently strips markers before lowering, which would make a temporarily optional required number nullable; the caller is in `markdown/schemas/mod.rs`. Increment `phase::PROJECTION_REVISION` from 1 to 2 and update its identity fixtures. Leave `MERGE_ALGORITHM_REVISION` at 1: this work changes projection, not whole-property layering.

4. **One compiled default plan, one application engine.** Add public `defaults::DefaultPlan` with `DefaultPlan::compile(schema: Arc<Value>, cache: &ValidatorCache) -> Result<Self, SchemaError>` and change application to `prepare_defaults<'a>(plan: &DefaultPlan, instance: Cow<'a, Value>, templates: &NestedTemplates) -> Result<DefaultPreparation<'a>, SchemaError>`. Store the plan alongside validators in `Prepared` and `EffectiveSchema`. Keep `AppliedDefault` and `DefaultPreparation` result shapes. Compile reachable schema structure and union consistency once; check a default's value only when used. Evidence: `defaults.rs::Walk` currently reconstructs per-arm candidates; the only production application callers are `prepared.rs` and `darkmatter/lib/src/markdown/schemas/mod.rs::complete`. Reuse `ValidatorCache` structural validators and existing identity facilities; add no hashing implementation or dependency.

5. **Default collection does not select union arms.** At each object location, collect the defaults of named properties across `anyOf`/`oneOf` arms and local references, in declaration order. Merge identical defaults and reject different values for the same property with new `SchemaError::ConflictingDefaults { pointer: String, property: String, declarations: Vec<String> }`, whose declaration entries are schema JSON Pointers. Check this during lowering/export and raw-schema preparation, including nested and referenced unions. Use a shared schema traversal for this check and plan compilation. Never choose a default by discriminant, successful validation, coercion cost, or arm order. Evidence: the author ruling explicitly extends the existing property-union conflict rule to object arms; `defaults.rs::{union,root_union,settle}` implements the superseded policy. Remove `Settled`, `settle`, `prepare_unaccepted`, and `AmbiguousDefaults`, including its matches in `errors.rs`, `triggers/finding.rs`, and Darkmatter's schema-error conversion.

6. **Completed values still satisfy all authored assertions.** Default collection ignores arm preference; validation then judges one completed candidate with the existing discriminant and root-sibling rules. A supplied non-null value always wins. A default that introduces a property forbidden by every viable completed arm produces an ordinary located validation failure; do not drop the default, widen a closed arm, or suppress a null exclusion. The specification's exclusive `timeout`/`retries` example therefore still uses its explicit null exclusions after completion. This follows its separate application/validation steps and its instruction to audit arms that reject another arm's defaults. Optional default filling must not be used to reward a conversion arm or to bypass an authored discriminant. Preserve conversion's existing supplied-value ranking in `coercion/union.rs`.

7. **Default validity is lazy, strict, and structural.** Validate a used default against the complete definition that declares it, with references in scope and no conversion, expression evaluation, or file access. A parent default must satisfy required children before optional reachable children are completed. Descend into existing objects and list items, and into a newly supplied valid parent default. Track active default expansion separately from ordinary reference traversal so recursive types terminate on finite input, while a default that keeps creating another defaulted parent raises `RecursiveDefault`. Evidence: `defaults.rs::{fill,visit,declared_default}` already provides structural checking and a same-depth reference guard, but does not descend into inserted defaults.

8. **Null defaults and falsey values have explicit outcomes.** Optional `default(null)` does nothing to absence or written null. Required null-admitting `default(null)` inserts and reports only a missing key. Required number with a used null default fails `InvalidDefault`. Preserve supplied `0`, `false`, `""`, `[]`, and `{}`. Missing/null parents without their own default remain untouched. Pending values and descendants remain pending. Report each changed path once, parents before children, arrays by index; a second soft call makes no changes. Evidence: the specification's default-preparation rules, `NestedTemplates`, and `defaults.rs::finish`.

9. **No success shortcut may bypass default preparation.** Strict validates the completed candidate but returns the original borrowed input and reports defaults. Soft applies defaults, converts, then completes newly reachable children; it borrows unchanged input and owns only a changed result. Remove validity-based early returns from `Prepared::{validate_strict,validate_soft}`, `EffectiveSchema::{coerce_frontmatter_with_pending,converted_candidate,data_copy}`, and the default engine. Keep diagnostic partial-copy behavior and never publish failed candidates. Rename Darkmatter's private `required_defaults` to `applied_defaults`; its call sites are in `markdown/compose/schema_validation.rs`. Evidence: these current methods return before visiting newly active optional defaults.

10. **Delete `non-null`; extend the existing constraint.** Remove `Constraint::NonNull`, `marks_non_null`, `non_null_problem`, the `UnionInvariant::NonNull` branch, parsing, serialization, resolution, export, trigger checks, help, and completion support. Unsupported text and `$constraints: { non-null: true }` produce useful located schema errors. `NotEmpty` lowers to the existing `\S` pattern for strings and YAML/JSON text, the same consolidated `minItems: 1` bound as `min(1)` for lists, and `minProperties: 1` for objects/mappings. Support `schema(not-empty)` on inline declaration mappings/unions and reference text, and `schema(mapping; not-empty)` on nonempty mappings; preserve declaration validation. Reject the constraint on scalar number, boolean, null, and unknown. Null acceptance comes from optionality, never from this constraint overriding a required type. Evidence: `convert.rs::{string_fragment_with_base,apply_array_constraints,apply_object_arity}`, declaration lowering/validation, `grammar.rs`, and the specification's migration table. Element constraints remain distinct from array constraints.

11. **Positional contracts stay positional.** Omitted optional arguments and trailing tuple items remain legal, while written null requires a null-admitting parameter/item type. No default pass inserts array items or function arguments. Nested object properties inside them do receive defaults. Retain optional-parameter subsets in `lifecycle/schema_projection.rs::long_arms`; mark every included parameter required even when nullable, add one optional `no_error: boolean(default(false))`, and delete the separate flag arms and `min-keys` presence workaround. Evidence: `long_arms` currently doubles nullable-parameter arms and counts keys; these alternatives are unnecessary once `required` means presence.

12. **Envelope carriers alone normalize null to absence.** Update `RecognizedStandalone::{payload_key,payload,local_types}` and category parsing so null `$schema` can fall back to a valid `types` mapping in a tagged schema, null optional `types`/metadata are omitted, and `exports_schema` means a usable export exists. A `yaml-schema` still requires a non-null export; a document with no usable carrier fails. `{ n: null }` inside a payload remains a declaration. A nonempty `types` table is required in the types-only alternative. Evidence: `simplified/standalone.rs` currently selects carriers by key existence; admission lives in `envelope.rs` and `schemas/{schema,yaml-schema,schema-trigger}.yaml`. Treat null trigger payload as matcher-only, without allowing defaults/imports/generated values in `match:`.

13. **Declared names already have a separate authority.** Delete `materialize_optional_document_bindings` and its call. Reuse `unknown_identifiers.rs::{undeclared_warnings,reconcile}` and `EffectiveSchema::declares_top_level_property`, which already consult the schema independently of actual keys. Do not add another declared-name registry. Keep inherited data, lifecycle inputs, and output frontmatter free of synthetic nulls. Change `schema_validation.rs::insert_defaults` to replace null as well as insert absence, retaining pending/excluded-key guards. Evidence: the synthesis helper currently inserts null into the working map; `unknown_identifiers.rs` already filters declared roots.

14. **Lifecycle conversion uses the library binder.** Centralize literal flag conversion in `lifecycle/action_shape.rs` using `coercion::{bind,check_exact}` and Boolean/String shapes; share it with `parse.rs` and `markdown/schemas/bound_operands.rs`. `no_error` and `create_new` default false; `gui` missing/null becomes `None`. Boolean-like text converts, native numbers do not become booleans. `edit: 5` and `edit: true` convert to file names `5` and `true`, following ordinary string coercion. Blank strings, null/missing mandatory `file`, extra keys, and arity errors remain refused by the passive action-shape check. Keep `edit` file templates deferred, and keep flag templates unsupported: the existing flags are literals, and the specification requests literal scalar coercion rather than deferred flag execution. Evidence: `action_shape.rs::{edit_file_problem,edit_flag_problem}`, `parse.rs::{parse_bundle,parse_long_form}`, `bound_operands.rs`, and `EditAction`'s concrete Boolean fields.

15. **Authored flag origin is data, not an inferred Boolean value.** Add `LifecycleParseOptions::authored_values: Option<&Value>` and a matching authored-value cursor for stack/branch/task traversal. `None` means the input itself is authored; hosts parsing completed values supply the pre-completion value. Track non-null presence at supported sibling and inner flag locations; null counts as omitted. Two authored placements fail even when equal. Defaulted false flags never count as authored, create verbs, or give `set` a flag. `set` continues to reject an authored `no_error` key, including null, because it accepts no such parameter; a synthesized false does not become a `set` parameter. Retain `AuthoredOrder` solely for key order. Add `authored_value: Option<&Value>` immediately after `raw` in `parse_single_action_with` and `parse_task_action_stack_with` for their non-root input, rather than guessing a pointer from display paths. Wire `lifecycle/{host_profile,md_host}.rs`, Claudine `composition/{prepare,preflight,lifecycle/mod,sequence/task/mod}.rs`, and their tests. Capture generated lifecycle values immediately before their first schema completion as that value's authored source. Evidence: `AuthoredOrder` stores keys but not values, so it cannot distinguish an authored null from a synthesized false; all these entry points currently parse only completed values or optional order metadata.

16. **Apply the migration by resolved type, preserving optionality.** In the three SimplifiedSchema envelope schemas, Darkmatter config/lifecycle schemas, and Claudine config schemas, replace existing `non-null` on strings, lists, and mappings with `not-empty`; remove it from booleans. Preserve defaults and add no inheritance-changing defaults to repository overrides. Empty event action lists formerly guarded by `non-null` become invalid as the specification explicitly requires. Preserve empty collections that have no such constraint, including required `default([])` and required `default({})` settings. Descriptive prose about non-null values and frozen benchmark text remain intact.

17. **Move provider-specific defaults into distinct parent properties.** The actual shipped conflict is `claudine/schemas/claudine-config.yaml::messenger-route`: Discord/Slack declare different `bot_token_env` defaults, and their webhook variants declare different `webhook_url_env` defaults. Also, its `action` union has defaults on `volume`, `speed`, and `params` absent from other closed action shapes. Use externally keyed configuration payloads: `{ discord: { channel_id: "…" } }`, `{ slack_webhook: {} }`, and `{ sound_effect: { effect: "…" } }`, `{ bash: { command: "…" } }`. Each union arm is a closed one-key mapping with that key required; the payload has no parent default. Keep the six route and six action payload defaults inside their respective keys. This preserves every capability and default while giving mutually exclusive payloads distinct paths; it does not exempt discriminated unions from the new rule. Existing standalone envelope root arms contain no defaults; generated lifecycle long-form arms will share only the same `no_error: false` default. Evidence: the config schema's `messenger-route` and `action` definitions, `config/messaging_block.rs::MessengerProviderConfig`, and `actions/hook_action.rs::HookAction`.

18. **Contain the config representation change at its boundary.** Keep runtime `MessengerProviderConfig` and `HookAction` enums and their existing non-config serialization. Add private `config/wire.rs` serde adapters on configuration `actions` and messenger `configurations`: unwrap the canonical one-key payload into the existing `type`/`provider` tagged value for deserialization, and reverse that transformation for config serialization. Adapters reject multiple keys, null payloads, wrong containers, and unknown tags; they contain no default logic. New YAML config uses the keyed representation; old flat YAML receives a migration diagnostic with a keyed example. The existing legacy JSON5-to-YAML converter upgrades its flat action/route representations before schema validation and writes the new representation. Evidence: `config/json_conversion.rs::prepare_scope` already owns that conversion boundary, `config/persist.rs` owns config writes, and the TUI/init wizard constructs the existing Rust enums. Update config fixtures, persistence tests, and examples, not unrelated provider event formats. This technical migration is part of the implementation and final review evidence, not a gate.

19. **Repository null overrides inherit, including messenger selection.** Change `RepoOverrideConfig::active_messenger` from `Option<Option<String>>` to `Option<String>`, remove `deserialize_optional_messenger_override`, and update `merge_repo_override` and its callers so missing/null retains the user's active route. Preserve explicit non-null overrides. For `actions`, `matchers`, `steering`, and nested event entries, use typed null-as-unset deserializers after schema validation; do not globally delete nulls from source or completed documents. No arbitrary map entry or `unknown` payload is pruned. Evidence: `config/merge.rs` already merges event entries individually but currently treats `Some(None)` as disable; `RepoOverrideConfig` and `SteeringConfig` have concrete map/struct fields whose derived deserializers cannot accept newly legal nulls. The specification explicitly supersedes the old null-versus-absence distinction for overrides.

20. **Keep newly active defaults, including observable output changes.** Preserve `fm.yaml`'s optional null defaults as no-ops. Existing `hash: {}` receives `kind: simple`; existing `style.markdown: {}` receives its declared cleanup defaults; absent/null parents do not appear. Existing Claudine TTS mappings receive `prewarm: true`, and user steering's `automatic` mapping receives `enabled: true`; repository steering receives no new default. Keep permissions defaults scope-specific and routed through Darkmatter. Evidence: `darkmatter/schemas/partials/{fm,style}.yaml`, Claudine's `tts-settings`, `user-steering`, `repository-steering`, and `config/load.rs::project`. Correct comments claiming these optional defaults are never inserted. Key-count constraints judge completed values; lifecycle schemas already exclude `when`/`no_error` using `uncounted-keys`, which remains their explicit policy.

21. **No spike is necessary.** The problem is answered by the existing walks, callers, schema inventory, and author rulings. Retain lazy validators and copy-on-change behavior; add semantic/work-counter regressions, not a benchmark or OS performance matrix. Evidence: the specification explicitly excludes a new performance spike, and the removed algorithm clones one candidate per arm while the replacement produces one candidate.

### Wave 1

- [x] **Separate presence**
  - Change the two `is_required` implementations and all lowering calculations under Rules 1–3. Resolve inherited/hoisted markers before presence decisions; retain pending-slot refusal.
  - Preserve declared value typing through launch/completion projection and update `schema_identities.rs` for projection revision 2.
  - Add focused tests for null/unknown required properties, nullable unions, imports, generated/eager properties, trigger match presence, and positional null behavior. Use the existing `required_defaults`, declaration, phase, and trigger modules.

### Wave 2

- [x] **Replace constraint**
  - Prerequisite: Wave 1. Remove `NonNull` and implement Rule 10 in shared `types.rs`/`convert.rs`; the separate wave prevents overlapping edits to those core files.
  - Cover `simplified/{grammar,mod,types,convert,serialize,narrowing,subtype,lint}.rs`, `resolve.rs`, `resolve/recursive.rs`, `schema/export.rs`, `triggers/{grammar,matcher}.rs`, declaration keyword validation, and completion catalogs. Update the existing DMLS completion assertions that explicitly list `non-null`.
  - Test unsupported old spellings, every supported `not-empty` family, invalid scalar families, merged array lower bounds, and null bypass for optional properties.

Checkpoint: focused nextest modules for presence, lowering, constraints, and phases pass. Shipped-schema-dependent suites are intentionally integrated after their sources are migrated in Phases 2–4; do not regenerate artifacts from this intermediate language state.

## Phase 2 — Deterministic default preparation

### Wave 3

- [x] **Compile defaults**
  - Implement Rules 4–8 in `simplified-schema/lib/src/defaults.rs`, with schema-location traversal over properties, local references, arrays/tuples, `allOf`, and unions. Compile reference graphs without expanding recursive schemas indefinitely.
  - Keep distinct default scopes below distinct parent properties. Do not create absent parent objects while collecting descendants. Existing `additionalProperties` object traversal remains supported; pattern-property dispatch remains owned by existing validation/conversion machinery and is not broadened by this fix.
  - Route lowered export and raw `Prepared::new` through the shared conflict check. Preserve `InvalidDefault` and `RecursiveDefault`; add `ConflictingDefaults` display/location handling and remove `AmbiguousDefaults` exhaustively.
  - Test equal defaults, conflicting defaults even on unused arms, one-arm defaults on already-valid inputs, root sibling assertions, closed arms, null exclusions, reference cycles, and recursive default expansion. Invalid but unused values remain accepted until their default is used; conflicting declarations always fail preparation.

### Wave 4

- [x] **Integrate preparation**
  - Prerequisite: the compiled-plan API is in place. Update `Prepared`, `EffectiveSchema`, `outcome.rs` documentation, and all Rule 9 shortcuts. Store plans with prepared schemas rather than recompiling them per document.
  - Make final validation and conversion operate on the one shared completed candidate; retain original values and positions for diagnostics and authored-presence consumers. Preserve structural-only default checking.
  - Extend `simplified-schema/lib/tests/l1/required_defaults.rs` with the complete presence table, raw-schema equivalents, unchanged-input assertions, borrowed/owned results, optional invalid defaults, null defaults, falsey controls, nested parents, arrays, imports, and pending templates. Replace obsolete runtime-ambiguity expectations with preparation-time conflicts.
  - Add a public plain-JSON-Schema validator test proving exported `required` still rejects an absent defaulted required property while library preparation accepts it. Register additional test modules in `tests/l1/main.rs`.

Checkpoint: direct library tests prove the complete contract without relying on migrated shipped artifacts. No validator/default API refers to runtime ambiguous defaults or promises that optional defaults are metadata-only.

## Phase 3 — Migrate schemas and consumers

### Wave 5

The following three tasks have disjoint ownership: envelope files, lifecycle files, and configuration files. Configuration owns both host config schemas; lifecycle owns only lifecycle schemas and generators. `markdown/schemas/mod.rs` remains owned by the completed Phase 2 task.

- [x] **Admit envelopes**
  - Implement Rule 12 in `simplified/standalone.rs`, `envelope.rs`, trigger envelope/assembly readers, and all three library schemas. Apply `not-empty` to metadata, optional schema carriers, and both types-table alternatives without making optional metadata mandatory.
  - Exercise public `parse_standalone_schema_document`, `SimplifiedSchema` factories, envelope admission, and whole-YAML loading. Include `$schema: null` plus valid types, null optional metadata/types, missing mandatory exports, empty declarations, malformed carriers, and a payload's null-typed property.
  - Keep schema-source spans and YAML numeric/duplicate-key admission intact. Test selected and uncached admission as well as ordinary factory loading.

- [x] **Align lifecycle inputs**
  - Implement Rules 11, 14, and 15 in `lifecycle/{schema_projection,parse,action_shape,operands,host_profile,md_host}.rs` and `markdown/schemas/bound_operands.rs`.
  - Update handwritten lifecycle definitions and the typed generator. Every supported sibling/bundle/long-form `no_error` is `boolean(default(false))`; `edit.create_new` is defaulted false and `edit.gui` is optional Boolean. Keep `set` outside that flag contract. Required operands must stay required inside long forms without requiring the verb in the shared dictionary.
  - In the generated long arms, remove the old key-count/flag variants; retain optional-operand subsets and the passive `operands(...)` marker. Run `just regen-lifecycle-schema` from `darkmatter/`, and `just regen-lifecycle-artifact` from `claudine/` after updating its generator inputs. Do not hand-edit generated regions.
  - Thread authored values through Claudine's lifecycle/sequence entry points listed in Rule 15. Preserve authored order independently. Missing/null flags use default values while duplicate authored non-null placements fail.
  - Extend `lifecycle_edit_schema_parity.rs`, `lifecycle_schema_parity.rs`, parser tests, and fake-editor executor tests. Cover `{ edit: { file: notes.md } }`, both defaulted flag locations, null-versus-written duplicates, strings/native flags, numeric/Boolean file names, empty/missing targets, generated functions, bundles, conditionals, cancellation, and `set`.

- [x] **Migrate configuration**
  - Implement Rules 16–20 in `darkmatter/schemas/darkmatter-config.yaml`, `darkmatter/lib/src/config/`, `claudine/schemas/claudine-config.yaml`, and `claudine/lib/src/config/`. Keep required nested default objects complete before optional descendant filling.
  - Reshape route/action schema unions to the keyed payload representation and add the configuration-only serde adapters. Preserve enum consumers in `messaging/{config,resolve,send}.rs`, dispatch, the init wizard, and config TUI; update their serialized configuration fixtures and persistence expectations.
  - Update the JSON5 converter to produce canonical keyed YAML before writing it, preserving its existing atomic write/backup behavior and refusing invalid input before publication. Use the biscuit-file skill for this conversion work.
  - Change messenger override representation and typed null handling; update `config/{claudine_config,merge,load,messaging_block}.rs`, `steering/automatic.rs::SteeringConfig`, and tests. Preserve non-null explicit values and unknown-field errors. Test through `ClaudineConfig::from_user_document`, `RepoOverrideConfig::from_repository_document`, and config loader/merge entry points.
  - Apply default reports to null replacements in both config loaders. Keep original documents/locations and permission ownership; do not duplicate the library's default walk or introduce generic null-key pruning.

### Wave 6

- [x] **Preserve written data**
  - Prerequisites: default preparation and lifecycle authored-input APIs. Remove synthesis in `markdown/compose/schema_validation.rs`, update default write-back, and use the existing declared-name warning reconciliation.
  - Ensure composition commits only an accepted completed copy and preserves pre-completion lifecycle flag origin through later composition passes. Failed conversions/defaults leave authored input and source files unchanged.
  - Update `darkmatter/schemas/partials/{fm,style}.yaml` commentary and host assumptions about newly inserted defaults. Exercise actual cleanup/style and hash readers with the new completed values; preserve absent parents.
  - Extend `darkmatter/lib/tests/l1/absent_property_contract.rs` with the parent/child table from the specification, both `unknown(required)` and `string(required)` children, output frontmatter absence, lifecycle-visible absence, and warning-free interpolation. Also test raw-schema, referenced-schema, root-union, and trigger-declared names through the existing declaration authority.

### Wave 7

- [x] **Prove host parity**
  - Prerequisite: written-data integration. Extend DMLS `tests/l1/required_defaults.rs`, Darkmatter whole-YAML/schema tests, and CLI schema/compose tests using `CliProcessFixture`.
  - Assert defaulted values are visible to subsequent composition reads, DMLS publishes correct located diagnostics without edits, and CLI validation does not rewrite source. Use fake lifecycle services to prove no action executes on invalid inputs.
  - Add one conversion-produced-empty YAML interpolation case under `not-empty`, and phase-specific tests for supplied null/wrong-type values during launch versus missing required values at completion.

Checkpoint: schema and lifecycle parser changes are integrated together. All host regressions use the new rules; no feature in this wave requires an interactive process or separate approval.

## Phase 4 — Robustness and generated artifacts

### Input Robustness Matrix

Default outcome: an invalid field or document returns a located public error, launches no action, writes no source/config/output document, and publishes no successful result. DMLS publishes diagnostics only. Field tests run once per shared validator through crate public APIs; each additional field sharing it gets a valid control and one invalid wiring case. Start from actual schema/config/action fixtures and make one mutation per case. Absence/null equivalence below is an explicit language rule, not an accidental `Option<T>` collapse.

**Field validator exceptions and coverage:**

| Shared validator / fixture | Exceptions to invalid-input rejection and required coverage |
|---|---|
| Defaulted/required mapping properties, `required_defaults.rs` | Missing/null follows the specification table and Rules 7–8. Exercise native wrong type, non-coercible text, empty strings/containers, one/every invalid collection item, and falsey non-null controls. Strict refuses coercion; soft accepts only library-supported conversions. Check public result ownership, reports, location, source immutability, and idempotence. |
| `not-empty` string/YAML/JSON/list/mapping | Missing/null is accepted only through optionality. Empty/whitespace text, empty lists/mappings fail. Use nonempty controls and whole-field wrong types; collections additionally contain one invalid item and all invalid items. YAML/JSON content must still parse. A declaration with unsupported scalar type fails schema preparation. |
| Envelope metadata and carriers | Missing/null optional metadata, categories, types, and trigger payload are omitted. A types-only document requires a nonempty valid table; `yaml-schema` requires a usable export. Wrong scalar/collection carriers fail. Category lists cover mixed/all non-string members and empty lists. Scalar metadata retains existing documented soft conversions, including numeric descriptions. Payload `{ n: null }` is a positive control. |
| Lifecycle flags and operands | Missing/null flags use false/none; Boolean-like text is soft-convertible; numbers are invalid flags. Numeric/Boolean file scalars convert to text. Null/missing/blank mandatory file values fail; zero/too many positional operands, one/every invalid item, unknown keys, and object operands fail. Null outer `edit` never creates an action. Two non-null authored flag placements fail; generated defaults do not count. |
| Claudine overrides and keyed config payloads | Missing/null overrides inherit. Nonempty migrated list/map/string overrides replace according to existing scope rules; newly constrained empty values fail. Root `{}` remains a valid defaults/inheritance control; a blank/null config root fails. Keyed action/route wrappers require exactly one known non-null object payload. Each of the six routes and six actions gets a positive control and one invalid field; shared wrapper checks get absent/null, scalar, zero keys, two keys, and unknown key cases once. |
| Raw JSON Schema defaults | Missing/null uses declared defaults; optional null defaults remain no-ops. Required assertions and null/type acceptance stay authored. Used invalid defaults return `Unprepared` with the default pointer; conflicting arm defaults fail `Prepared::new` before any document is judged. Preserve `oneOf` exclusivity and root sibling assertions. |

**Document loaders:**

- Admitted YAML: `numeric::load_admitted_yaml`, used by standalone recognition and `yaml_documents::load_yaml_document`, covers `.yaml`/`.yml` once. Prove duplicate-key rejection, malformed/trailing content rejection, second-document rejection, valid leading `---`/trailing `...`, BOM/CRLF locations, and both `k:` and `k: null`. Keep the existing whole-YAML public loader fixture as the control; host config and envelope calls add wiring cases rather than repeating this matrix.
- Markdown frontmatter extraction has its own framing path in `markdown/frontmatter.rs`. Cover valid fences and omitted/null properties, malformed YAML and duplicate keys inside fences, and unchanged body text after the closing fence. Do not apply the file-format matrix to Markdown body grammar.
- Raw JSON Schema text first calls `coercion::content_error(ContentFormat::Json, text)` in `resolve::{parse_raw_json_schema,classify_json_schema_text}`, then JSON parsing and numeric admission. Cover valid JSON, duplicate-member rejection, malformed/trailing-content rejection, and numeric admission. `coercion/engine.rs::content_error` already uses a duplicate-refusing JSON visitor. Validate resulting defaults through the public schema factory and `Prepared`.
- Claudine legacy JSON5 uses `biscuit_file::Json5::from_str` followed by `json5_scan` in `config/json_conversion.rs`. Cover valid comments/trailing commas, duplicate-member rejection with its pointer and offset, malformed/trailing-content rejection and non-finite-number refusal, then canonical keyed conversion. Invalid conversion creates neither YAML nor backup; successful conversion preserves its atomic publication contract.

CLI coverage is wiring only: valid control plus one malformed and one field-invalid case for whole YAML/config, Markdown frontmatter, raw JSON Schema, and the legacy JSON5 migration entry point. Assert rendered file/property locations and no action launch/publication. All other cells stay in-process. A review defect must be logged with the state of the other shapes for the affected validator and the other consumers of the affected loader.

### Wave 8

- [x] **Complete input coverage**
  - Close every applicable matrix row through public results and existing fixture helpers. Register all tests in their consolidated binaries. Preserve native fake editor/services and cross-platform path construction; no Unix-only stand-in for common behavior.
  - Search changed readers for `#[serde(default)]`, load-bearing `Option<T>`, `filter_map` plus `as_str`, `unwrap_or_default()`, and `.ok()`. Record the disposition in the implementation log: intentional null/unset handling, post-validation typed extraction, or fixed silent loss. Do not filter malformed collection entries.
  - Verify all retained key-count constraints against completed values, especially lifecycle `uncounted-keys` and new one-key config wrappers. Confirm no default creates an absent route/action payload.

### Wave 9

- [x] **Refresh shipped artifacts**
  - Prerequisite: all schema migrations and source fixes above are complete. Bump `simplified-schema/lib/src/shipped/registry.rs::PREPARATION_REVISION` 2→3, `darkmatter/lib/src/markdown/schemas/shipped.rs` 3→4, and `claudine/lib/src/composition/schema/shipped.rs` 3→4.
  - Run the single root `just shipped-schemas` recipe after lifecycle region regeneration. Do not hand-edit generated JSON. Check cached and uncached envelope, baseline, config, and lifecycle preparation against the same fixtures and completed values.
  - Add newly introduced preparation source files to affected `[package.metadata.ci.tests].source-inputs`, including `defaults.rs` dependencies and the configuration wire adapter where tests explicitly read it. Spell fixture reads in CI-indexable forms. Load the rust-devops skill before editing these evidence declarations; add no CI jobs or matrix cells.
  - Update affected lowering goldens, identity pins, freshness tests, and lifecycle generator parity expectations to the canonical regenerated output, preserving frozen benchmark inputs.

Checkpoint: all three shipped artifacts pass freshness checks, and cached/uncached paths produce identical verdicts, default reports, and completed values.

## Phase 5 — Documentation and caller guidance

### Wave 10

These tasks may run concurrently, each owning its area's docs and matching repo skill. Publication follows all three edits.

- [x] **Document library semantics**
  - Update SimplifiedSchema validation, conversion, language types, unions, schema documents, custom vocabulary, runtime phase guidance, and facade API docs. Separate default application from validation, show the required/null table, and explain plain JSON Schema export limitations.
  - Document compile-time default conflicts, recursive/default-null behavior, the new public default-plan API, and `not-empty` by type. Include examples for missing versus null parents and optional positional arguments.
  - Update `.claude/skills/simplified-schema/` guidance, especially required defaults and declaration narrowing. Update README claims only where summarized behavior changes; there is no SimplifiedSchema caller-skill publication recipe or directory to invent.

- [x] **Document Darkmatter behavior**
  - Update `docs/inline/schema-validation.md`, schema definition/validation topic pages, lifecycle actions/error handling, configuration, and cleanup/style guidance. Explain declared-but-unwritten bindings and the composed-copy/default write-back boundary.
  - Replace obsolete literal-only `edit` flag guidance with supported coercion and null behavior; retain nondeferred flag limits and mandatory target rules. Explain duplicate authored placement separately from completed defaults.
  - Update `.claude/skills/darkmatter/`, canonical `docs/use-skills/use-darkmatter/`, and `docs/use-skills/use-dmls/`. Include a compact flow diagram for authored values → default application → conversion → final validation → successful output, with authored metadata retained alongside it.

- [x] **Document Claudine migration**
  - Update configuration, schemas, lifecycle, messaging, and relevant README overview claims. Show canonical keyed route/action examples and old-flat-YAML diagnostic guidance; explain legacy JSON5 conversion and unchanged runtime enum APIs.
  - Document missing/null override inheritance, removal of null-as-disable messenger selection, newly rejected empty overrides, and newly inserted user defaults. Include before/after completed-value examples for TTS and steering.
  - Update `.claude/skills/claudine/` and canonical `docs/use-skills/use-claudine/{config,lifecycle,composition,library}.md`. Avoid links from topic docs to feature/fix directories.

### Wave 11

- [x] **Publish caller skills**
  - Run `just publish-skills` in `darkmatter/` and `claudine/` after canonical docs are finalized. Inspect composed outputs for interpolation corruption and stale rules; never directly edit installed user-scope copies.
  - Search active source, schemas, help, docs, and skills for `NonNull`, `non-null`, `AmbiguousDefaults`, null-cancels-required claims, and optional-defaults-as-metadata-only claims. Keep rejection tests, historical specs, frozen benchmarks, and accurate descriptive uses. Record remaining legitimate match categories in the implementation log.

## Phase 6 — Final verification and handoff

### Wave 12

- [ ] **Run scoped gates**
  - Run `just test` and `just lint` from each of `simplified-schema/`, `darkmatter/`, and `claudine/`. Run these area gates sequentially against the integrated tree; the package recipes own their feature sets and nextest concurrency. SimplifiedSchema includes no-default-features; Darkmatter includes CLI and DMLS; Claudine includes its CLI/config adapters.
  - Run the areas' `just doctest` recipes for changed public API examples. Do not invoke `cargo fmt` or `cargo test` unit/integration suites. No added test requires L2/L3: CLI wiring uses hermetic subprocesses and lifecycle execution uses fake services. No real GUI launch is part of acceptance.
  - Reuse focused passes already obtained; repeat a gate only after changes or a failure that invalidates its evidence. Keep macOS/Linux/native-Windows/WSL2 compatibility in fixtures and implementations; report actual tested environments precisely. Cross-environment execution follows the repository's OS skill and configured non-interactive build-host recipes, never an ad hoc interactive SSH flow or an inferred OS ban.
  - Confirm unchanged-input and no-publication assertions for failure paths, default visibility in subsequent reads, lifecycle schema/parser agreement, and all cached/uncached parity checks. Fix defects in this worktree and rerun the affected evidence.

### Wave 13

- [ ] **Record completion evidence**
  - Finish `implementation.md` beside this plan with commands/results, migrations, changed completed outputs, schema/API departures, generated artifact revisions, and the matrix's validation coverage. Include the keyed config migration rationale and the former messenger-null disable behavior explicitly.
  - Check task completion, phase count, test registration, generated-file freshness, documentation consistency, and diff scope. Preserve other work already present in the worktree.
  - End with “implementation complete, ready for review.” Leave the fix directory active and make no commit or push. Final human review occurs after all phases and does not block any task above.