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
    - |-
      Phase 2 keeps one "already valid" shortcut the plan listed for removal:
      `EffectiveSchema::coerce_frontmatter_with_pending`. It only converts and never
      applies defaults; a value its validators accept as written has nothing to
      convert, and composition already applies every default before calling it.
      Every shortcut that skipped default preparation (`Prepared::validate_strict`,
      `validate_soft`, `EffectiveSchema::converted_candidate`, `data_copy`, the old
      engine's own early return) is gone.
      Alternative: remove it too, at the cost of a full conversion walk for every
      valid composed document.
      If reversed: delete the `accepts_as_is` check at the top of that function.
    - |-
      `EffectiveSchema::data_copy` (the copy `md set`, the YAML kind checks, and DMLS
      read) now converts the default-completed candidate, then takes each applied
      default back out (an inserted key removed, a replaced `null` restored). The plan
      said only to drop its early return. Returning the completed copy as is would make
      `md set` write defaults into authored files, because it writes the value it
      reads back from this copy.
      Alternative: return the completed copy and change `md set` to read elsewhere;
      or keep converting the raw value, which a missing required child can block.
      If reversed: `darkmatter/lib/src/markdown/schemas/mod.rs::{data_copy,withdraw_default}`.
    - |-
      Composition's `insert_defaults` now replaces a `null` as well as a missing
      value (one line). Plan Rule 13 places this in Phase 3, but once preparation
      reports defaults for `null` values, leaving it out makes the composed
      frontmatter keep `null` while the verdict counts the default. The rest of
      Rule 13 (deleting `materialize_optional_document_bindings`) is still Phase 3.
      If reversed: `schema_validation.rs::insert_defaults`, plus
      `composition_defaults::a_written_null_takes_the_default`.
    - |-
      Darkmatter configuration: with null meaning absent, `permissions: null`,
      `file-read: null`, `shell-commands: null`, `network: null`, and `register: null`
      now load with their scope's schema default instead of the located "null" error
      the config input matrix expected. I moved those cells out of the error rows into
      a new `config_fields::a_null_setting_takes_its_scope_default`, and renamed
      `config_load::a_key_with_no_value_is_an_explicit_null` to
      `a_key_with_no_value_takes_the_default`. `[null]` inside a list is still an
      error.
      Alternative: give the Darkmatter config schema explicit non-null types so null
      stays an error, which the spec's ruling argues against.
      If reversed: `darkmatter/schemas/darkmatter-config.yaml` and those two tests.
    - |-
      Default-plan details the plan left open:
      (1) a property counts as required for a `null` default when any schema
      describing its location lists it in `required`;
      (2) a used default must satisfy every property definition that declares it
      (equal defaults on several arms are checked against each arm's definition);
      (3) a `$ref` chain that loops without describing a child (`a -> b -> a`) is
      `RecursiveDefault { path: None }` when the plan compiles, so `Prepared::new`
      now refuses such a raw schema instead of failing only when a value reached it.
      Alternatives: (1) require the property on every arm; (2) check only the first
      declaration; (3) skip such loops silently.
      If reversed: `simplified-schema/lib/src/defaults.rs::{Compiler::scope,
      Compiler::declared, Compiler::expand}`.
    - |-
      Claudine's launch-phase helper `complete_launch_defaults` is deleted rather than
      deduplicated. It re-judged a launch document against the full contract to find defaults
      of non-eager required properties, because the launch view used to hide them. Defaults
      now apply whether or not a property is required, so the launch verdict already lists
      them and refuses a broken one; the helper only reported the refusal a second time.
      Alternative: keep it and drop duplicate problems. If reversed: restore
      `claudine/lib/src/composition/schema/launch_defaults.rs` and its three call sites.
    - |-
      Expression typing now treats an optional property with a non-null literal default as
      never null (for example `retries: number(default(3))` is `number`, so no "may be null"
      warning), because composition fills a missing or null value with the default. Required
      properties were already non-null. A default written as a `{{ }}` template does not
      count. Visible effects: DMLS stops warning about such properties, and the nullable
      `::file` target diagnostic no longer fires for a defaulted target. Alternative: keep
      them nullable, which contradicts what composition does. If reversed:
      `darkmatter/lib/src/markdown/compose/expression/scope/shape.rs::has_non_null_default`
      and the updated tests (`composition_defaults`, `directive_target_analysis`, DMLS
      `required_defaults` and `lsp_session`).
    - |-
      A schema trigger with an empty `$schema` (`{}`, `[]`, blank text) is now refused with a
      located message. Before, discovery quietly layered nothing while whole-YAML validation
      of the same file refused it (`schema(not-empty)` in `schema-trigger.yaml`). `$schema:
      null` or omitting it still gives a matcher-only trigger. Separately, `$schema: {}` alone
      in a `kind: schema` document stays a valid, empty export, because the plan applied `not-
      empty` only to optional carriers. Alternatives: drop `not-empty` from the trigger
      definition, or also refuse the empty `kind: schema` export. If reversed: the
      `is_empty_carrier` guard in `simplified-schema/lib/src/triggers/envelope.rs`, plus the
      empty-payload rows in `trigger_input_matrix.rs`, `schema_embedded_trigger_bundles.rs`,
      Claudine's `claudine_schemas_matrix.rs`, and `envelope_null_carriers.rs`.
    - |-
      Lifecycle conditionals (`when`/`then`/`else` items) now declare `no_error:
      literal(false)`, in Claudine's `conditional`, `initialize-conditional` and `cancelled-
      conditional` and Darkmatter's `md-conditional-*` / `md-initialize-conditional-*` (the md
      ones with `uncounted-keys: [no_error]`). Defaults are collected from every arm at a
      location, so an item could receive `no_error: false` from an action arm, and the closed
      conditional arms would otherwise reject every conditional. The first review's repair
      now refuses an authored `no_error: false` beside `then` in both schema validation and
      parsing; a completed false remains accepted. Alternatives considered originally: declare
      `boolean` there (also admits `true`), or drop the sibling default (contradicts the
      spec). To support this, the lifecycle work also edited
      `claudine/schemas/{action,lifecycle}.yaml` and `darkmatter/schemas/partials/md-
      lifecycle.yaml`, which the plan did not list.
    - |-
      Where lifecycle flags came from: Claudine treats the source frontmatter plus the proxy
      overlay and layered overrides as what the author wrote
      (`prepare.rs::authored_lifecycle_values`), not a value captured inside Darkmatter's
      schema stage. Limitation: a `loop:` value rewritten by the first interpolation pass
      loses its authored flags. Sequence task `side_effect:`/`setup:`/`teardown:` stacks are
      parsed as authored (`None`), because sequence data is not schema-completed first. A
      `null` flag now counts as omitted on actions that accept that flag, so for example `start: { no_error: null }`
      is an empty stack instead of an error. Phase 4 later exercised `loop:` preparation
      end to end in `a_loop_document_prepares_as_authored_and_an_invalid_loop_flag_is_refused`;
      it verifies that the deferred block is kept as authored. Alternative: expose the pre-completion frontmatter from
      `schema_validation.rs`.
    - |-
      Configuration migration choices: (1) a repository can no longer turn messaging off,
      because `active_messenger: null` now inherits the user's route, so the config TUI's
      "(disabled)" repository option was removed; an off switch would need a new setting. (2)
      Keyed routes accept only snake_case keys (`discord_webhook`, `slack_webhook`); the
      converter and the migration message rewrite hyphenated spellings. (3) Route and action
      payload types are `closed`. (4) Null-as-unset reading also covers TTS
      `provider`/`prewarm` and `models` entries, which would otherwise fail to load after the
      schema accepted them. (5) Saving a config no longer writes `null` for a key the file
      never wrote. (6) Darkmatter's now-unreachable null messages in `config/load.rs` were
      deleted. If (1) is reversed, add a distinct "disabled" value to the repository schema
      and the TUI.
    - |-
      A shipped-corpus guard in `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`
      (`presence_only_sites`) refused any `unknown(required)` or `any(required)` in shipped
      schemas, on the old rule that a null-admitting type cancels `required`. I removed it:
      `required` now means "the key must be written", and the regenerated lifecycle schemas
      use `unknown(required)` on purpose for required operands that admit null. Alternative:
      keep the guard and rewrite those operands, which would lose required presence.
    - |-
      Phase 4 found that an `initialize` lifecycle effect still runs when the document's
      frontmatter or its raw JSON Schema is refused, because `initialize` runs before schema
      validation by documented design. The matrix's default says an invalid input "launches no
      action". I kept the documented behavior and wrote the new CLI cells
      (`darkmatter/cli/tests/l1/compose_loader_wiring.rs`) so their marker effect sits in
      `composed`, which does not run on a refusal. Alternative: validate before `initialize`,
      which changes the lifecycle order and its docs. If reversed: move schema validation ahead
      of `initialize` and add `initialize` marker cells to that file.
    - |-
      Overflowing YAML numbers such as `1e400` load as the text `"1e400"` (the YAML parser
      turns them into strings); a pinned test (`setter_values::spellings_yaml_reads_as_strings_are_never_rounded`)
      and Darkmatter's frontmatter matrix rely on that. But `SimplifiedSchema::try_from` reads
      raw JSON text through YAML, so `"default": 1e400` loads and fails only later as
      `InvalidDefault`, while the `.json` file loaders refuse it at load, and
      `simplified-schema/docs/topics/foundations.md` and `darkmatter/docs/topics/numbers.md`
      now explain this loader distinction after the Phase 5 documentation correction.
      I left the code unchanged (a fix was tried and reverted because it broke the pinned
      behavior). Alternatives: refuse non-finite YAML numbers at
      load everywhere (about 60 lines in the admitted-YAML loader, plus re-pinning those
      tests); the alternative of correcting the two docs was completed in Phase 5.
      Related: the content factory also accepts YAML flow
      syntax, such as a trailing comma, in raw JSON text that the `.json` loaders refuse.
    - |-
      `yaml(not-empty)` and `json(not-empty)` judge the text, not the parsed value, so `"{}"`
      and `"[]"` count as content, and soft validation turns a native `{}` into the text
      `"{}"` and accepts it. This follows the spec's wording ("empty or whitespace-only text")
      and is pinned in `not_empty_values.rs::yaml_and_json_text_must_still_parse`. Alternative:
      parse the text and refuse an empty parsed collection. If reversed: the `not-empty` check
      for these types and that test.
    - |-
      Darkmatter's md baseline leaves side-effect and expression-function operands untyped,
      but passive validation now shares the parser's flag and placement checks.
      `ensure_file: { file: a.md, no_error: 1 }`, `{ when: 'true' }` without a branch or
      action, and an authored `no_error: false` beside `then` are refused by both.
      `lifecycle_schema_parity.rs` now pins agreement. These repairs followed the first review;
      Phase 4 had only recorded the disagreements.
    - |-
      Phase 5 updated the pinned doc phrases in Darkmatter's doc-contract test
      `schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes`.
      The library and Darkmatter pages used to say an eager-only `null` is allowed "because missing
      and `null` both count as absence"; under the new rules a written `null` meets presence and is
      judged by the type, so the pages were rewritten and the test now pins the new sentences
      ("Key required, and the value is validated eagerly", "An optional or eager-only property
      accepts `null` at both phases"). The test's validator re-derivation is unchanged and passes.
      Alternative: restore the old sentences, which would document the old rule.
    - |-
      Documentation choices in Phase 5: the canonical missing/`null`/`required` table lives in
      `simplified-schema/docs/topics/validation.md` and the types, Darkmatter, and Claudine pages
      link to it; `types.md` keeps a short migration table for the removed `non-null`, and the
      Darkmatter caller skills mention that `non-null` is now a schema error (use `not-empty`).
      The anchors `#required-properties-with-a-default` and `#non-null-properties` were renamed or
      dropped (only pages in that area linked them). The library's repo skill `SKILL.md` was cut
      to under 200 lines by moving defaults and shipped-artifact/identity notes into new
      `defaults.md` and `shipped-and-identities.md`. The Darkmatter docs state the known
      schema/parser gap outright at that phase; the first review's repair updated the docs
      and now two authored `no_error` placements fail both validation and parsing.
      Claudine's configuration page frames the changes as "Earlier releases
      ..." before/after tables, and documents two inferred consequences no test pins: a
      repository cannot clear an event's actions (`[]` is an error, `null` inherits), and an
      empty event list in a legacy `config.json` fails conversion. Claudine's new `library.md`
      config snippet was written from signatures and is labeled as not compiled.
      Alternatives: put the table in `types.md`; keep migration notes out of caller skills;
      accept a longer `SKILL.md`; describe the parser gap only as "an error"; state only current
      Claudine behavior. If reversed, these are doc-only edits.
    - |-
      Phase 5 found that a property-level union whose arms declare different defaults is still
      reported as a conversion error (`SchemaError::Convert`), not `ConflictingDefaults`, which
      covers object arms and raw schemas. The docs describe both. Alternative: route that case
      through `ConflictingDefaults` too (a lowering change and artifact regeneration).
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
source_files_during_phase_6:
    - simplified-schema/lib/src/triggers/assemble.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
    - claudine/cli/src/commands/actions.rs
    - claudine/cli/src/commands/config_tui/tabs/messenger/input.rs
    - claudine/cli/src/commands/config_tui/tabs/messenger/modal.rs
    - claudine/cli/src/commands/config_tui/tabs/messenger/render.rs
    - claudine/cli/src/commands/config_tui/tabs/messenger/tests.rs
    - claudine/cli/src/commands/hooks/list.rs
    - claudine/cli/src/commands/hooks/variables.rs
    - claudine/cli/src/commands/init_wizard.rs
    - claudine/cli/src/commands/init/mod.rs
    - claudine/cli/tests/common/drain_interrupt.rs
    - claudine/cli/tests/common/webhook_listener.rs
    - claudine/cli/tests/l1/agent_cwd.rs
    - claudine/cli/tests/l1/command_routing.rs
    - claudine/cli/tests/l1/compose_presence_parity.rs
    - claudine/cli/tests/l1/config_json5_wiring.rs
    - claudine/cli/tests/l1/detached_audio.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/handle_blocking_output.rs
    - claudine/cli/tests/l1/handle_message_drain.rs
    - claudine/cli/tests/l1/handle_repo_config.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/pi_managed_rpc.rs
    - claudine/cli/tests/level2/level2_lifecycle_control.rs
    - claudine/lib/benches/runtime_hot_paths.rs
    - claudine/lib/generated/shipped.json
    - claudine/lib/src/composition/inline_prompt.rs
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
    - claudine/lib/src/composition/schema/shipped.rs
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
    - claudine/lib/src/config/persist_tests.rs
    - claudine/lib/src/config/persist.rs
    - claudine/lib/src/config/tts.rs
    - claudine/lib/src/config/wire_tests.rs
    - claudine/lib/src/config/wire.rs
    - claudine/lib/src/dispatch/loader/tests.rs
    - claudine/lib/src/steering/automatic.rs
    - claudine/lib/src/steering/automatic/tests.rs
    - claudine/lib/tests/config/repository-control.yaml
    - claudine/lib/tests/config/user-control.yaml
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/config_json_conversion.rs
    - claudine/lib/tests/l1/config_keyed_entries.rs
    - claudine/lib/tests/l1/config_schema_matrix.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine-config.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/tests/l1/compose_edit_action.rs
    - darkmatter/cli/tests/l1/compose_lifecycle.rs
    - darkmatter/cli/tests/l1/compose_loader_wiring.rs
    - darkmatter/cli/tests/l1/compose_schema.rs
    - darkmatter/cli/tests/l1/compose_setters.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
    - darkmatter/cli/tests/l1/schema_validate_core_types.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/lifecycle_host_tests.rs
    - darkmatter/dmls/tests/l1/edit_action_editor.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/required_defaults.rs
    - darkmatter/dmls/tests/l1/standalone_stdio.rs
    - darkmatter/dmls/tests/l1/value_hovers.rs
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
    - darkmatter/lib/src/lifecycle/tests/authored_flags.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/shipped.rs
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
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/composition_defaults.rs
    - darkmatter/lib/tests/l1/config_fields.rs
    - darkmatter/lib/tests/l1/config_load.rs
    - darkmatter/lib/tests/l1/content_policy_compose_contract.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs
    - darkmatter/lib/tests/l1/effective_default_preparation.rs
    - darkmatter/lib/tests/l1/frontmatter_extraction.rs
    - darkmatter/lib/tests/l1/function_call_binding.rs
    - darkmatter/lib/tests/l1/kind_semantic_checks.rs
    - darkmatter/lib/tests/l1/lifecycle_edit_action.rs
    - darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs
    - darkmatter/lib/tests/l1/lifecycle_generated_operands.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_parity.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/non_null_frontmatter.rs
    - darkmatter/lib/tests/l1/not_empty_frontmatter.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
    - darkmatter/lib/tests/l1/schemas_effective_presence.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_union_schema_matrix.rs
    - darkmatter/lib/tests/l1/shared_acyclic_selection.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/trigger_input_matrix.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/lib/tests/l1/yaml_document_selection.rs
    - darkmatter/lib/tests/l1/yaml_kind_registry.rs
    - darkmatter/schemas/darkmatter-config.yaml
    - darkmatter/schemas/partials/fm.yaml
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/md-lifecycle.yaml
    - darkmatter/schemas/partials/style.yaml
    - simplified-schema/lib/generated/shipped.json
    - simplified-schema/lib/src/claims.rs
    - simplified-schema/lib/src/coerce.rs
    - simplified-schema/lib/src/defaults.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/outcome.rs
    - simplified-schema/lib/src/phase.rs
    - simplified-schema/lib/src/prepared.rs
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
    - simplified-schema/lib/src/triggers/assemble.rs
    - simplified-schema/lib/src/triggers/envelope.rs
    - simplified-schema/lib/src/triggers/finding.rs
    - simplified-schema/lib/src/triggers/grammar.rs
    - simplified-schema/lib/src/triggers/lint.rs
    - simplified-schema/lib/src/triggers/matcher.rs
    - simplified-schema/lib/tests/l1/claimed_values.rs
    - simplified-schema/lib/tests/l1/declaration_grammar.rs
    - simplified-schema/lib/tests/l1/envelope_admission.rs
    - simplified-schema/lib/tests/l1/envelope_null_carriers.rs
    - simplified-schema/lib/tests/l1/input_robustness.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/non_null_properties.rs
    - simplified-schema/lib/tests/l1/not_empty_values.rs
    - simplified-schema/lib/tests/l1/required_defaults.rs
    - simplified-schema/lib/tests/l1/required_presence.rs
    - simplified-schema/lib/tests/l1/schema_identities.rs
    - simplified-schema/lib/tests/l1/schema_target_classification.rs
    - simplified-schema/lib/tests/l1/schema_target_inspection.rs
    - simplified-schema/lib/tests/l1/template_type_results.rs
    - simplified-schema/schemas/schema-trigger.yaml
    - simplified-schema/schemas/schema.yaml
    - simplified-schema/schemas/yaml-schema.yaml
documentation:
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/claudine/hook-actions.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/simplified-schema/declaration-narrowing.md
    - .claude/skills/simplified-schema/defaults.md
    - .claude/skills/simplified-schema/schema-documents.md
    - .claude/skills/simplified-schema/shipped-and-identities.md
    - .claude/skills/simplified-schema/SKILL.md
    - claudine/cli/README.md
    - claudine/docs/topics/automatic-steering.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/configuration.md
    - claudine/docs/topics/configuring-actions.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/messaging.md
    - claudine/docs/topics/schemas.md
    - claudine/docs/use-skills/use-claudine/composition.md
    - claudine/docs/use-skills/use-claudine/config.md
    - claudine/docs/use-skills/use-claudine/library.md
    - claudine/docs/use-skills/use-claudine/lifecycle.md
    - darkmatter/docs/cli/schema.md
    - darkmatter/docs/cli/set.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/configuration.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/frontmatter.md
    - darkmatter/docs/topics/lifecycle/error-handling.md
    - darkmatter/docs/topics/lifecycle/index.md
    - darkmatter/docs/topics/markdown-formatting.md
    - darkmatter/docs/topics/numbers.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/schemas/schema-targeting.md
    - darkmatter/docs/use-skills/use-darkmatter/authoring.md
    - darkmatter/docs/use-skills/use-darkmatter/cli.md
    - darkmatter/docs/use-skills/use-darkmatter/SKILL.md
    - darkmatter/docs/use-skills/use-dmls/schemas.md
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
completed_phase: 6
implemented: true
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

## Phase 2

### Wave 3 — Compile defaults (complete)

- `simplified-schema/lib/src/defaults.rs` rewritten around a public `DefaultPlan`:
  - `DefaultPlan::compile(Arc<Value>, &ValidatorCache) -> Result<Self, SchemaError>` reads the lowered (or raw) schema once into object *locations*. A location is the merge of every schema node that can describe the value there: the node itself, its local `$ref` target, and every `allOf` / `anyOf` / `oneOf` member, transitively. Properties, `additionalProperties` (when no `patternProperties`), `prefixItems`, and `items` each lead to a child location. Locations are memoized by their sorted node pointers, so recursive types compile to a finite graph.
  - Per property the plan records whether any node lists it in `required` and its one default. Every `default` keyword found in the property's declarations is collected without choosing an arm; equal values merge, different values are the new `SchemaError::ConflictingDefaults { pointer, property, declarations }` (location pointer and each `default` keyword's pointer). A `$ref` chain that returns to itself without describing a child is `RecursiveDefault { path: None, .. }` at compile time.
  - `DefaultPlan::for_identity(identity, schema, cache)` memoizes plans per `SchemaIdentity` for the process (capacity 64, cleared by `reset_process_caches`), for hosts that know their schema's identity.
  - `prepare_defaults(plan, instance, templates)` walks the value: a missing or `null` property takes its default (required or not); a `null` default fills only a missing key of a required property; a supplied non-null value always wins; a missing/`null` parent without a default is left alone; a used default is checked strictly (structural validator, root `$defs` in scope, memoized per declaration) against each property definition that declares it (`InvalidDefault`), then descended into so its children are completed; a default that keeps recreating its own parent is `RecursiveDefault { path: Some(..), reference: <default pointer> }`; array/tuple positions are never inserted; values at or below a nested template are skipped. Insertion replaces a missing or `null` value only.
  - Removed: `Settled`, `settle`, `prepare_unaccepted`, the per-arm union candidate logic, the root-union discriminant fallback, and `NestedTemplates::excuses_location` (its only caller was the removed per-arm judging).
- `SchemaError::AmbiguousDefaults` removed (`errors.rs`, `triggers/finding.rs`, Darkmatter `default_problem`); `ConflictingDefaults` added with display text and status-block handling. `InvalidDefault`'s message now says "property" (it applies to optional properties too).
- Lowering (`simplified/convert.rs::lower_document`) runs `defaults::check_declarations` on the lowered document, so `to_json_schema`, `to_json_schema_with_definitions`, phase lowering, and the facade's export refuse conflicting object-arm defaults. Raw JSON Schema hits the same check in `Prepared::new` (`DefaultPlan::compile`).

### Wave 4 — Integrate preparation

- `Prepared` (`prepared.rs`) holds a `DefaultPlan` (compiled in `compiled_through`, so `Prepared::new` fails on conflicting raw defaults) instead of the bare cache. `validate_strict` always prepares, then judges the completed candidate and returns the input borrowed with `defaults`; `validate_soft` prepares, returns at once when the completed candidate is valid, otherwise converts and prepares again. The two "accepted as written → no defaults" shortcuts are gone; `judge` keeps a cheap `is_valid` check before collecting problems.
- Darkmatter `EffectiveSchema` (`markdown/schemas/mod.rs`): new `default_plan: OnceLock<Arc<DefaultPlan>>` field (set in all three constructors: `build`, `for_yaml_document`, `phase_projection`); `default_plan()` uses `DefaultPlan::for_identity` when the schema has a layered/canonical digest (shared across documents) and compiles per schema otherwise. `complete` uses the plan; `converted_candidate` prepares first and only then asks whether conversion is needed (the old shortcut returned before any default). `required_defaults` renamed `applied_defaults` (call site and docs in `compose/schema_validation.rs`). `data_copy` now converts the default-completed candidate and withdraws each applied default (a replaced `null` restored, an inserted key removed), so `md set` (which writes from this copy) never writes a default, while conversion is no longer blocked by a missing required child. `ValidationReport::defaults`, `validate_instance`, and the compose module docs describe defaults for missing/`null` properties, required or not.
- `outcome.rs` (`ValidationSuccess::defaults`, `ValidationFailure::Unprepared`), `lib.rs` module list, `simplified/convert.rs` docs, and `ConflictingDefaults` docs updated.

- Darkmatter composition (`compose/schema_validation.rs::insert_defaults`) replaces a `null` as well as a missing value (see decisions_for_review).

### Behavior changes observed in dependents (tests updated)

- Optional defaults are now inserted into composed frontmatter, and a written `null` takes the default: `composition_defaults::{a_written_null_takes_the_default, an_optional_default_is_inserted}` (replacing the "never defaulted / never inserted" tests); `schema_validation::tests::defaulted_document_property_takes_its_default_rather_than_null`; `compose/tests/schema.rs::absent_caller_property_takes_the_document_owned_schema_file_default`; `dynamic_schema_slot_surfaces::…::an_inline_template_type_default_is_known_before_the_default_decision` (now `Some(3)`). The invalid-default message reads "default of property".
- Darkmatter config nulls take their scope default (see decisions_for_review): `config_fields::a_null_setting_takes_its_scope_default` (new), `config_load::a_key_with_no_value_takes_the_default` (renamed).
- `recursive_schema_merge.rs`'s raw baseline declared `note: {const: "kept", default: "#/$defs/item"}`, an invalid default that was never used before. Now that optional defaults apply, its `const` is `"#/$defs/item"`, so the fixture still checks that reference-shaped literal data is kept byte-for-byte.
- Root-union defaults apply from every arm (plan Rule 6). The old test `a_discriminant_selects_the_arm_whose_defaults_apply` became `defaults_of_every_arm_apply_and_the_discriminant_still_selects`: `{kind: page}` now completes to `{kind: page, title: untitled, slug: home}`.

### Requirement → test mapping (Phase 2)

All in `simplified-schema/lib/tests/l1/required_defaults.rs` unless noted. Boundary: the library's public API, in-process (F1). Darkmatter host: `darkmatter/lib/tests/l1/effective_default_preparation.rs` (F2, temp-dir document through `DarkmatterSchemas::effective_for`).

- Presence table, every row, missing vs written `null`, strict (borrowed + report) and soft (owned value) → `presence_table::*` (6 tests).
- Raw JSON Schema equivalents: optional raw default fills missing/null; raw null without default still judged by authored type (no wrapper); required raw default plus unchanged `required` array and property schema on export; raw parent default checked then children completed → `raw_schemas::*`.
- Unchanged input / borrowed vs owned → the `borrowed`/`owned` helpers in every cell; repeat stability → `strict_and_soft::a_soft_result_validates_again_unchanged`, `raw_schemas::a_raw_parent_default_is_checked_then_its_children_are_completed`.
- Null defaults (optional no-op, required null-admitting inserts only a missing key, required number fails `InvalidDefault`, written null not replaced by a null default) → `null_and_falsey::*`; falsey controls `0/false/""/[]/{}` → `null_and_falsey::falsey_values_are_never_replaced`.
- Optional invalid defaults lazily judged; parent default missing a required child not repaired → `optional_invalid_defaults::*`; existing `invalid_defaults::*` kept.
- Nested parents (missing/null parent untouched; existing parent's missing and null children filled; parent default's optional children completed, parents first; list items by index; local named type; positional items never inserted) → `parents::*`, `nesting::*`.
- Pending templates → `pending_templates::a_value_below_a_template_is_never_filled` (public `DefaultPlan`/`prepare_defaults`/`NestedTemplates`); pending `{{ }}` default slot → `slots::*`.
- Unions: equal defaults merge; conflicting root-arm defaults fail preparation and export even when unused; nested-arm conflict located at its property; raw-schema conflict; one-arm default on an already-valid input; spec's exclusive `timeout`/`retries` example (null exclusion still judges the completed value); closed arm rejects another arm's default (missing and written null); every arm's defaults apply with the discriminant still judging; root sibling default and sibling assertions → `unions::*`.
- Plain validator vs library on exported `required` → `a_plain_validator_still_rejects_an_absent_defaulted_required_property` (uses `jsonschema::validator_for`).
- Compile/apply internals (pure `$ref` loop refused at compile; recursive reference compiles once; default that recreates its parent is `RecursiveDefault`; tuple positions not inserted but their objects completed; `additionalProperties` values completed; no-default schema borrows; conflict check on nested unions) → `defaults.rs` unit tests (9).
- Host (Rule 9 shortcuts gone, parity, `data_copy`, raw conflict reported as a located problem) → `effective_default_preparation::*` (4 tests).

### Phase 2 verification

- `simplified-schema`: `just test` exit 0, with 2209 L1 tests (`work-counters,terminal`) and 1853 minimal tests passing, and "2209 test(s), none at or above 5.0s". `just lint` exit 0.
- `darkmatter`: `just lint` exit 0. `cargo nextest run -p darkmatter --no-fail-fast` (before the test updates above): 11656 run, 17 failed, 3 slow. 16 of those 17 are the behavior changes listed above; after the updates, those tests plus the new module pass (72 targeted tests, then 4 new ones). The remaining failure is `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower`, which hits the Claudine `messenger-route` conflict and is owned by Phase 3, Rule 17. The 3 slow tests were in code this phase did not touch, during a load spike (load average about 51 on this host): `installed_shells::injection_shaped_names_never_execute` 5.8 s, `expression_type_scope::…::scalar_unions` 6.5 s, and `base_schema_end_to_end::base_style_schema_agrees_with_the_runtime_style_parser` 7.6 s. All three are under the 10 s loaded-host allowance. I did not re-measure them alone.
- `darkmatter-lsp`, `darkmatter-cli`: `-E 'test(/default|set|frontmatter|schema/)'` gave 1671 run, all passed.
- `claudine` (`--features work-counters`, `-E 'test(/config|schema|default|steering|messag/)'`): 2001 run, 144 failed. Nearly all are config loading through the Claudine config schema, which no longer prepares because of `messenger-route`'s conflicting defaults (Phase 3, Rule 17). Three are Claudine composition default tests that pin the old semantics (see `message_to_agent`). `claudine-cli` was not run.
- Shipped artifacts: no schema text or payload changed in this phase, so no `PREPARATION_REVISION` was raised and `just shipped-schemas` was not run. It cannot regenerate Claudine's artifact until Rule 17 lands.
- Cross-OS check not run: no path, process, or `cfg` code changed.

## Phase 3

### Orchestration

- Wave 5's three tasks (envelopes, lifecycle inputs, configuration) ran as three parallel subagents with disjoint file ownership. None of them ran `just shipped-schemas`; the orchestrator regenerates all three artifacts once after Wave 5.
- The host was already loaded by another worktree's Claudine test run when the phase started (load average about 53 on 16 cores).

### Wave 5 — Admit envelopes (complete)

- Phase 1 had already landed most of Rule 12 (checked, unchanged): null `$schema`/`types`/metadata carriers read as absent in `RecognizedStandalone`, `categories: null` is absent, a tagged `$schema: null` beside `types` is types-only, `kind: yaml-schema` refuses a null export, a document with no usable carrier fails, `types: {}` is refused, `{ n: null }` inside a payload stays a declaration, and a trigger's `$schema: null` is matcher-only. The whole-YAML loader needed no change.
- `triggers/envelope.rs::parse_trigger_envelope` now refuses an empty `$schema` (`{}`, `[]`, blank text) with "`$schema`: must not be empty; omit it, or write `null`, for a matcher-only trigger". Before, discovery layered nothing for `$schema: {}` while whole-YAML loading against `schema-trigger.yaml` (`schema(not-empty)`) refused it. Module and function docs (which still said a null `$schema` was an error) corrected; `simplified/standalone.rs` module doc corrected (definitions forbid *empty* metadata, not `null`).
- Darkmatter's trigger robustness table followed: `trigger_input_matrix.rs` `payload_empty` now expects the located refusal, and its table cell changed from "preserve explicit empty schema" to `E`.
- New `simplified-schema/lib/tests/l1/envelope_null_carriers.rs` (registered in `main.rs`): 28 cells over tagged, `yaml-schema`, and informal envelopes, each run through the parser, envelope admission in normal and `PreparationMode::Uncached` mode, `SimplifiedSchema::try_from`, the `try_from_yaml` file factory, and whole-YAML loading, which must agree; controls for duplicate keys / inadmissible numbers and identical carrier spans in both modes; trigger payload null/empty/malformed cells and the `match:` prohibitions on both paths.
- Verification: `cargo nextest run -p simplified-schema --features work-counters,terminal --test l1 -E 'test(/envelope|standalone|yaml_document|schema_target|trigger|test_layout/)'` 86 passed; `--lib -E 'test(/standalone|envelope|trigger/)'` 155 passed; `--no-default-features` new module 8 passed; `just lint` passed. Darkmatter `trigger_input_matrix` 111 passed.

### Wave 6 — Preserve written data (in progress, done ahead of the lifecycle results where independent)

- Deleted `schema_validation.rs::materialize_optional_document_bindings` and its call (Rule 13). The undeclared-name warning already asks `EffectiveSchema::declares_top_level_property`, so nothing replaced it. Module docs and the `run` step list no longer describe the synthesis; `insert_defaults`' doc says it fills a missing or `null` value.
- Unit tests renamed and inverted to "never written": `absent_optional_document_properties_are_never_written_and_reruns_are_stable`, `a_referenced_schema_s_absent_optional_property_is_never_written`, `baseline_root_union_and_nested_properties_are_never_written`, `a_raw_json_schema_property_is_never_written`, `a_trigger_payload_property_is_never_written`, `normal_darkmatter_baseline_does_not_flood_frontmatter` (now two keys); `compose/tests/schema.rs::baseline_file_projection_layers_with_document_schema_and_skips_absent_optional` now asserts the key is absent rather than `null`.
- Expression typing (`compose/expression/scope/shape.rs::Shape::from_definition`) still assumed an optional property's default is never inserted and typed it nullable. It now drops `null` for a required property or one with a non-null literal default (a pending `{{ }}` default does not count). `composition_defaults::a_defaulted_property_is_typed_present_whether_or_not_it_is_required` (renamed) pins `spare: number(default(4))` as non-null and adds `bare: number` as the nullable control.
- `darkmatter/schemas/partials/style.yaml` cleanup comment and `fm.yaml` (`default(null)` is a no-op) commentary updated.
- Docs: `docs/inline/schema-validation.md` ("Properties with a default" rules and flowchart; "Optional parameter bindings" replaced by "Declared but unset properties"), `docs/topics/schemas/authoring-schemas.md` (anchor and null rule), `docs/topics/markdown-formatting.md` (composed `style.markdown` gains rule defaults; `md clean` writes nothing).
- Tests (F2, real composition of temp-dir documents through `Markdown::compose_with`, `darkmatter/lib/tests/l1/absent_property_contract.rs`): the specification's parent/child table — `an_unwritten_parent_property_is_missing_in_a_child_that_requires_it` (both `unknown(required)` and `string(required)` children are refused; the `unknown` row is the regression a synthetic `null` would hide), `a_written_null_meets_an_unknown_child_s_required_contract`, `a_written_null_fails_a_string_child_s_type`, `a_written_title_reaches_both_children`, `an_unwritten_parent_property_renders_empty_without_a_warning_or_a_key` (composed frontmatter has no `title`, no warning); `every_schema_form_declares_without_writing` (raw JSON Schema, referenced file, root union, trigger payload: empty render, no key, no warning, with an undeclared `ghost` control that does warn). A refused child surfaces as a `transclusion` warning naming `child.md`; the warning text does not carry the child's problems, so the missing-versus-null wording is not asserted there.
- `composition_defaults.rs`: `a_written_hash_mapping_gains_its_kind_default` (`hash: {}` → `{kind: simple}`, then the real stored-hash reader reports the missing `value`; absent/`null` `hash` gains nothing and reads as no stored hash) and `a_written_markdown_style_mapping_gains_the_cleanup_defaults` (written `style.markdown` gains every rule default, a written `null` rule takes its default, a written rule wins, the cleanup rule reader `rules_from_json` + `resolve` agrees; unwritten `style` and `style: {}` gain nothing below them); `a_nested_default_fills_a_present_parent_only` now asserts the absent parent has no key.
- Claudine composition (Wave 7 prep, `claudine/lib/src/composition/schema/`): `launch_defaults.rs::complete_launch_defaults` deleted with its three call sites. It existed because the launch projection dropped `required` from non-eager properties and the old engine filled only required ones; preparation now fills optional as well as required properties (consulting `required` for null-default insertion), so the launch verdict already lists those defaults and refuses a broken one, and the helper reported the refusal a second time. `classify.rs` reports `PropertyState::Defaulted` for a written `null` that takes its default, not only a missing key. Tests: `defaults_tests::{an_optional_default_is_inserted_without_collection, an_explicit_null_takes_the_default, an_explicit_null_without_a_default_is_judged_by_type}` replace the two old-semantics tests; `launch::a_refused_default_of_a_non_eager_property_blocks_at_launch` now passes with one report; `tests::invalid_optional_{,setter_}is_dropped_and_retried` expect the dropped key to stay absent (Wave 6). `cargo nextest run -p claudine --features work-counters --lib -E 'test(/composition::schema/)'`: 136 passed; `inline_launch_type_checks_a_present_required_non_eager_value` took 6.4 s at load average 75 (re-measure alone at the gate).

### Wave 7 — Prove host parity (in progress)

- DMLS (`darkmatter/dmls/tests/l1/required_defaults.rs`, real LSP session over a temp workspace): `a_defaulted_property_is_present_and_the_controls_are_still_diagnosed` (replaces the required-only test: required, optional, and written-`null` defaulted properties read as non-null; only the undefaulted optional `p` is possibly null; only `r` is missing; no diagnostic at `/w`), `a_refused_optional_default_is_reported_and_the_document_is_unchanged` (a now-used optional default its definition refuses is located at the declaration; the file on disk is byte-identical after the session). 5 passed.
- CLI (`darkmatter/cli/tests/l1/compose_schema.rs`, `CliProcessFixture`, spawned `md`): `defaults_fill_missing_and_null_values_without_editing_the_source` — `md schema validate --output json` lists the optional default and the default replacing a written `null`, `md compose --frontmatter` shows both and no top-level key for the declared but unwritten `note`, the body renders `[3][auto][]`, and the source is unchanged after both commands.
- `not-empty` YAML interpolation (`darkmatter/lib/tests/l1/not_empty_frontmatter.rs::an_interpolation_that_produced_no_yaml_is_refused`): `config: yaml(not-empty)` fed `"{{ source }}"` with `source: ''` is refused at `/config`; the same interpolation producing `a: 1` is accepted; unwritten `config` is not set.
- Phases (Claudine `composition/schema/defaults_tests.rs::launch`): `a_supplied_null_or_wrong_type_is_judged_by_type_at_launch` (`count: null` and `count: abc` on a non-eager `number(required)` are `SchemaValidation` at `/count` and `Invalid` in the status at launch), `a_missing_non_eager_required_value_is_deferred_at_launch_and_missing_when_composed` (deferred at launch, `MissingProperties [count]` for a chained document).

### Wave 5 — Align lifecycle inputs (complete)

- Rule 14: `darkmatter/lib/src/lifecycle/action_shape.rs` gained `literal_flag` (the shared literal Boolean flag binder over `simplified_schema::coercion`: `"yes"` → `true`, native numbers and `{{ }}` templates refused), `edit_file_value` (string coercion of `edit`'s file: `5`, `true`, `2.5` name files `"5"`, `"true"`, `"2.5"`) and `edit_target_problem`; `parse.rs` and `markdown/schemas/bound_operands.rs` share them. `no_error`/`create_new` default false; `gui` missing/null is `None`.
- Rule 15: `LifecycleParseOptions::authored_values: Option<&Value>` with an `AuthoredSource` cursor through stack/branch/task traversal; `authored_value: Option<&Value>` follows `raw` in `parse_single_action_with` and `parse_task_action_stack_with`. A non-null authored placement counts, `null` is omitted, two authored placements fail even when equal, filled defaults never count, create verbs, or give `set` a flag; `set` still refuses an authored `no_error` (including `null`). Wired through `lifecycle/{host_profile,md_host,mod}.rs`, DMLS (`overlay/event_views.rs`, mechanical), and Claudine `composition/{prepare,preflight,lifecycle/mod,sequence/task/mod}.rs`; Claudine's authored source is `prepare.rs::authored_lifecycle_values` (source frontmatter plus proxy overlay and layered overrides).
- Rule 11 / generator: `schema_projection.rs::long_arms` keeps optional-operand subsets and the `operands(...)` marker, marks included parameters required, adds one optional `no_error: boolean(default(false))`, and no longer emits the separate flag arms or the `min-keys` workaround. `just regen-lifecycle-schema` (darkmatter) rewrote the generated regions of `darkmatter/schemas/partials/lifecycle.yaml` (4 handwritten `min-keys` remain); `just regen-lifecycle-artifact` (claudine) ran clean and left `claudine/schemas/globals.lifecycle.json` unchanged. Handwritten `edit` definitions declare `create_new: boolean(default(false))`, `gui: boolean`, `no_error: boolean(default(false))`. Also migrated: `darkmatter/schemas/partials/md-lifecycle.yaml`, `claudine/schemas/{action,lifecycle}.yaml` (see decisions). 14 Darkmatter recursive-schema goldens re-blessed, all lifecycle-derived.
- Tests: new `darkmatter/lib/src/lifecycle/tests/authored_flags.rs` (9 tests: parsing a completed value with its authored source equals parsing the authored value — `{ edit: { file } }` with both flags filled, either authored placement, equal duplicates, null placements, `set`, flags beside `then`, filled-only stacks/items/bodies and bundle verb counts, loop concerns, string/native flags, both entry points); `lifecycle_edit_schema_parity.rs` (numeric/Boolean file names, flag conversion, null flags; completed-value parity for `{ edit: { file: notes.md } }`; `two_authored_flag_placements_pass_the_schema_and_fail_the_parser`); `lifecycle_schema_parity.rs::completed_flags_keep_the_schema_and_the_md_parser_in_agreement` (generated side effects and functions, bundles, conditionals, `cancelled`, `initialize`, `set` with `no_error: null`); `lifecycle_edit_action.rs`; executor tests with `FakeEditor` (targets `5`/`true`/`2.5` open, converted/null flags reach the editor, a filled `false` does not suppress a failure while authored `"yes"` does, cancellation still stops the stack); Claudine `no_error_must_be_a_boolean_in_both_homes`, `a_completed_loop_block_parses_with_the_document_as_its_authored_source`.
- Orchestrator additions (CLI, spawned `md`, `darkmatter/cli/tests/l1/compose_lifecycle.rs`): `a_numeric_no_error_fails_before_any_effect`, `two_authored_no_error_placements_fail_before_any_effect` (no event, no `ensure_file` marker), `converted_and_null_flags_run_and_an_unwritten_declared_key_stays_absent` (`no_error: "yes"` and `no_error: null` run; the lifecycle reads the declared, unwritten `title` as `null`; `--frontmatter` output has no `title`). 52 passed.
- Agent verification: Darkmatter l1 lifecycle/edit/bound-operand/action filter 460 passed; lib `/lifecycle/` 195 passed; Claudine lifecycle/sequence/preflight/prepare/schema filter 2520 passed apart from failures owned elsewhere (stale shipped artifacts; the envelope change, since resolved; `prepare::service::tests::an_invalid_optional_is_dropped_and_recorded_on_the_harness_route`, which expected Rule 13's synthesized `null` and now expects the key absent). `claudine_schemas::shared_payload_properties_have_identical_types` timed out at 30 s at load average about 125 and passed alone in 9.6 s; re-measured at the gate. The agent once ran a scoped nextest with `--no-fail-fast` by mistake.
- Darkmatter test updates found by the gate run (all follow the rules above, assertions moved to the new outcome): `content_policy_compose_contract::a_document_declared_policy_is_not_written_when_absent` (renamed), `dynamic_schema_slot_surfaces::…::an_inline_template_type_default_is_known_before_the_default_decision` (plain `number` → no key), `variable_type_projection::…::only_defaults_are_written` (renamed), `schema_enhancements_matrix/frontmatter.rs` four "absent" cells (no key), CLI `compose_schema::{compose_frontmatter_never_writes_unset_optional_parameters_and_is_stable_after_recompose, the_schema_stage_writes_a_default_but_never_an_unset_property, compose_frontmatter_keeps_a_caller_supplied_optional_parameter}` (replace the three synthetic-`null` tests), `directive_target_analysis` and DMLS `lsp_session::nullable_transclusion_diagnostic_matrix` (a defaulted optional target is non-null), `schema_embedded_trigger_bundles` envelope matrix (empty `$schema` → `Invalid`), CLI `compose_edit_action` and DMLS `edit_action_editor` malformed-`gui` rows now use `gui: 5` (`"yes"` is valid Boolean-like text under Rule 14; message "must be a boolean"), and `shipped_schema_corpus` lost its obsolete `presence_only_sites` guard (see decisions). Claudine: `claudine_schemas_matrix` `trigger_payload` Empty → `Error` (shared by the embedded matrix).

### Phase 3 verification

Run by the orchestrator after every subagent had finished, serially, one area at a time.

- Shipped artifacts: one root `just shipped-schemas` after Wave 5 regenerated `darkmatter/lib/generated/shipped.json` and `claudine/lib/generated/shipped.json` (the library's artifact was already current: no library schema YAML changed this phase). No preparation code changed, so no `PREPARATION_REVISION` was raised.
- `simplified-schema`: `just test` exit 0 (2218 L1 + 1861 minimal, "none at or above 5.0s"); `just lint` exit 0.
- `darkmatter` (`just test`, all three packages): first full run 12 failures (all listed above, fixed); then exit 0, 16642 passed, 6 slow (load 28 → 143 during the run). A second run (load up to 147) passed all 16642 with 58 slow, including about 10 at 10.1–15.6 s (`claudine_editor_parity::depth4_*`, `sequence_tests::diagnostics_then_point_requests_reuse_one_snapshot` 15.6 s, `expression_type_scope::…::scalar_unions` 14.7 s, `entry_point_parity::md_entry_points_agree_on_every_reference_part_00` 12.2 s, `compose_schema::compose_frontmatter_serializes_root_union_write_back` 11.3 s). Every one of the 58 measured alone (load 20–120): the slowest 4.5 s (`diagnostics_then_point_requests_reuse_one_snapshot`), `scalar_unions` 5.7 s then 2.3 s, `base_style_schema_agrees_with_the_runtime_style_parser` 3.2 s (it was 6.4 s and 5.1 s alone at load about 150). They are contention-sensitive tests, mostly in code this phase did not touch; the same tree's first run recorded only 6. `just lint` exit 0.
- `claudine` (`just test`: claudine-catalog-types, claudine, claudine-contract, claudine-cli, claudine-gen): first run 4 failures (empty trigger payload cells, fixed) and 86 slow at load about 150; all 86 measured alone at 3.0 s or less. Rerun exit 0: 12143 passed, 8 slow (5.5–7.0 s, load up to 98), each 1.0–3.0 s alone. `just lint` exit 0.
- `just check-tier-coverage` for `simplified-schema`, `darkmatter`, `claudine`: 0 stranded.
- `just publish-skills` (claudine) published `use-claudine`.
- Cross-OS check not run: no path, process, or `#[cfg]` code changed; CI covers the other systems.

### Requirement → test mapping (Phase 3)

- Rule 12 envelopes → `simplified-schema/lib/tests/l1/envelope_null_carriers.rs` (F1, public parse/admission/factory/whole-YAML APIs; selected and uncached modes).
- Rules 11, 14, 15 lifecycle → `darkmatter/lib/src/lifecycle/tests/authored_flags.rs`, `lifecycle_edit_schema_parity.rs`, `lifecycle_schema_parity.rs`, `lifecycle_edit_action.rs`, executor `FakeEditor` tests (F1/F2), CLI `compose_lifecycle.rs` flag cells (F3, spawned `md`, no effect on refusal), DMLS `edit_action_editor.rs`.
- Rules 16–20 configuration → `claudine/lib/src/config/wire_tests.rs`, `config_schema_matrix.rs`, `config_json_conversion.rs` (round trip, refusal before publication), steering/TTS/merge/persist tests, config TUI test (F1/F2), Darkmatter `config` tests.
- Rule 13 / Wave 6 written data → `absent_property_contract.rs` parent/child table and `every_schema_form_declares_without_writing` (F2), `composition_defaults.rs` hash/style reader tests, CLI `compose_schema::defaults_fill_missing_and_null_values_without_editing_the_source` (F3).
- Wave 7 host parity → DMLS `required_defaults.rs` (F3 LSP session), `composition_defaults::yaml_document_validation_defaults_missing_and_null_values`, `not_empty_frontmatter::an_interpolation_that_produced_no_yaml_is_refused`, Claudine `defaults_tests::launch::{a_supplied_null_or_wrong_type_is_judged_by_type_at_launch, a_missing_non_eager_required_value_is_deferred_at_launch_and_missing_when_composed}`.
- Pre-existing or skipped: none failing. The scope calculator was not used to skip gates; every gate ran.

## Phase 4

### Orchestration

- Wave 8 ran as three parallel subagents with no shared files (`simplified-schema/`, `darkmatter/`, `claudine/`). Each listed the matrix cells that already existed and added only the missing ones, then ran nextest scoped to what it touched. The orchestrator ran the gates afterward, alone on the host (load average about 8 on 16 cores).
- No subagent changed library, schema, or preparation source. Two fixes were tried and reverted: an overflowing-number refusal in the admitted-YAML loader, which contradicted a pinned test, and a `bound_operands.rs` flag check, which was unreachable for `unknown`-typed verbs. Both are recorded in decisions_for_review.

### Wave 8 — Complete input coverage (complete)

Matrix default: an invalid shape gives a located public error, runs nothing, and publishes nothing. Exceptions are as the plan lists them. Cells added by row (cells that already existed are named in each subagent's survey and in the Phase 1–3 sections):

- **Defaulted/required mapping properties** (`simplified-schema/lib/tests/l1/required_defaults.rs::input_shapes`):
  - A 19-cell table covers native wrong types, coercible and non-coercible text, empty and blank strings, a scalar given for a list, one and every invalid list item, a `[null]` item, a wrong-typed object child, and invalid object items whose item defaults are filled after soft conversion. Each cell checks strict and soft problem pointers, owned values and the `defaults` report, that the soft result validates again unchanged under both strict and soft, and that the input is not mutated.
  - Also added: `required_falsey_values_are_values`, and `a_failure_elsewhere_still_counts_the_default_toward_presence`.
- **`not-empty`** (`not_empty_values.rs`):
  - Added `a_whole_field_of_another_type_is_refused`, `soft_converts_a_scalar_to_text_with_content_and_strict_refuses_it`, `collections_locate_one_and_every_empty_or_invalid_item`, and `yaml_and_json_text_must_still_parse`.
  - `types_without_an_empty_value_refuse_not_empty` now also covers `literal`, `enum`, `date`, and `file`.
- **Envelope metadata and carriers** (`envelope_null_carriers.rs`):
  - 12 more cells, each run through every load path in both preparation modes: categories given as a scalar, a mapping, mixed lists, all-non-string lists, and lists holding `null` or a mapping member; `description: 5` and `[a]`; `types: Id`.
  - Also added `scalar_metadata_is_converted_to_text`.
- **Raw JSON Schema defaults** (`required_defaults.rs::raw_schema_inputs`): `an_optional_raw_null_default_is_a_no_op`, `authored_null_admission_and_required_stay_as_written`, `a_used_invalid_raw_default_is_unprepared_at_its_pointer`, `conflicting_arm_defaults_fail_prepared_new_before_any_document` (`anyOf` and `oneOf`), and `one_of_exclusivity_judges_the_completed_value`.
- **Lifecycle flags and operands** (Darkmatter):
  - `lifecycle_edit_schema_parity.rs` gains `{ edit: 2.5 }` accepted, and `gui: [true]`, a list-of-mapping `edit`, and a null `edit` in a bundle refused.
  - `lifecycle_schema_parity.rs` gains `every_no_error_placement_converts_text_and_refuses_a_number_in_both_checks`, `a_generated_long_form_no_error_follows_the_flag_rule_where_the_verb_is_typed`, and `an_authored_false_flag_beside_then_passes_the_schema_and_fails_the_parser` (pins the known gap).
- **Key counts on completed values:**
  - Darkmatter `key_counts_are_judged_on_the_completed_value_and_a_filled_flag_is_never_counted` (one-verb passes; two-verb still fails `max-keys`; a `when`-only conditional still fails `min-keys`; validating again is stable) and `no_default_creates_an_absent_event_or_action_payload`.
  - Claudine `config_keyed_entries::defaults_fill_only_inside_written_payloads`.
- **Claudine overrides and keyed payloads:**
  - New `claudine/lib/tests/l1/config_keyed_entries.rs` generates one `#[test]` per action (`sound_effect`, `speak`, `bash`, `call`, `report`, `message`) and per route (`discord`, `slack`, `signal`, `whatsapp`, `discord_webhook`, `slack_webhook`). Each runs through `load_claudine_config` on the real control fixture: the entry as written is the control, and one changed field is refused at that kind's payload in `config.yaml`.
  - The guard `every_schema_action_and_route_kind_has_its_own_test` compares the macro lists with the shipped schema's unions.
  - `config_schema_matrix.rs` gains `a_blank_or_null_root_is_refused_in_either_scope` and `a_repository_matcher_override_is_nonempty_and_replaces_per_event`. The shared-wrapper cells (absent/null, scalar, zero keys, two keys, unknown key) already existed once in `wire_tests.rs`.
- **Loaders:**
  - Admitted YAML (new `simplified-schema/lib/tests/l1/input_robustness.rs`): `an_unset_value_loads_as_null_in_every_spelling`, `trailing_content_is_a_located_load_error`, and `a_load_error_after_a_byte_order_mark_and_crlf_lines_points_into_the_original_text`. Duplicate keys, second documents, and `---`/`...` already had cells in `darkmatter/lib/tests/l1/yaml_document_loading.rs`.
  - Raw JSON Schema text (same file): `the_content_factory_admits_raw_json_numbers`, `json_files::both_json_loaders_admit_numbers_from_their_spelling`, and `json_files::loaded_raw_defaults_fill_missing_and_null_values` (through the factory and `Prepared`).
  - Markdown frontmatter (new `darkmatter/lib/tests/l1/frontmatter_extraction.rs`, public `Markdown::try_from_content` / `try_from(&Path)`): omitted versus null properties, body after the fence, and malformed YAML and a duplicate key as located load errors that leave the source unchanged.
  - Claudine JSON5 (`config_json_conversion.rs`): `a_nested_duplicate_member_is_located_at_its_second_spelling`. Every refusal cell now asserts the directory holds only `config.json`. The successful conversion asserts exactly `config.json.bak`, `config.yaml`, and the retained lock file.
  - Claudine YAML config wiring: `config_schema_matrix::a_duplicate_key_or_malformed_yaml_is_a_located_problem`.
- **CLI wiring:**
  - New `darkmatter/cli/tests/l1/compose_loader_wiring.rs`, six tests with one `md compose` spawn each: malformed frontmatter, a field-invalid frontmatter value, a raw JSON Schema control, a raw JSON Schema with a duplicate member, a field-invalid value under raw JSON Schema, and a malformed user config. Each failure asserts exit 1, the located error, no `composed` effect marker, no published body, and an unchanged source.
  - New `claudine/cli/tests/l1/config_json5_wiring.rs` (fake goose provider, portable): a malformed legacy file and a schema-refused field both stop before any launch, keep the source bytes, and write no YAML or backup. The valid control is the existing `config_yaml_wiring::a_legacy_json_file_is_converted_once_with_a_notice`.
  - `md schema validate` whole-YAML wiring already existed in `schema_validate_yaml.rs`.
- **`loop:` end to end:** `claudine/lib/src/composition/prepare/service/tests.rs::a_loop_document_prepares_as_authored_and_an_invalid_loop_flag_is_refused`. `loop` is a deferred lifecycle key: the schema stage judges it and fills nothing, the effective block equals the authored one, and the gate's `no_error` reads false from the parser. An authored `no_error: 5` fails preparation.
- **Code-smell grep** over the changed readers in all three areas: no silent-loss defect found. Dispositions:
  - `wire.rs` null-entry dropping drops only `null`; a malformed entry still fails.
  - `defaults.rs`' `required` names read with `filter_map(as_str)` reads schema structure, not data.
  - `with.unwrap_or_default()` in `action_shape.rs` is the documented empty overlay.
  - The `.ok()` calls are message-only, constant-text parses, or documented "nothing loaded" returns.
  - `#[serde(default)]` appears only on post-validation typed extraction of schema-filled fields or on intentional absent-equals-null options.
  - One documented non-application remains: a default under an empty nested key (`o.""`) is neither applied nor reported, because `AppliedDefault.path` cannot spell it.

### Wave 9 — Refresh shipped artifacts (complete)

- The plan's revision bumps (2→3, 3→4, 3→4) had already landed in Phase 1, and Phase 4 changed no shipped YAML or preparation code, so no further bump was needed. The root `just shipped-schemas` reported "every shipped artifact is current" with no diff. Cached and uncached verdict parity is covered by `envelope_null_carriers.rs` (both modes, every new cell).
- `source-inputs`: unchanged. No new test reads another package's source file. The new Claudine tests read their own package's `tests/config/user-control.yaml` through `include_str!`, the same way the existing `config_schema_matrix.rs` does. The guard reads the library's embedded schema constant. The new CLI test binary needs `test-fixtures`, which `claudine-cli`'s CI `features` already enable.
- No lowering goldens, identity pins, or generator parity expectations changed.

### Phase 4 verification

- `simplified-schema` `just test`: exit 0. L1 ran 2237 tests and the minimal build 1878, none at or above 5.0 s.
- `simplified-schema` `just lint`: exit 0.
- `just check-tier-coverage` for `simplified-schema`, `darkmatter`, and `claudine`: 0 stranded.
- Dependents, serial and scoped to the changed test files, because no library source changed:
  - `cargo nextest run -p darkmatter --test l1 -E 'test(/lifecycle_schema_parity|lifecycle_edit_schema_parity|frontmatter_extraction|test_layout/)'`: 26 passed.
  - `cargo nextest run -p darkmatter-cli --test l1 -E 'test(/compose_loader_wiring|test_placement|test_layout/)'`: 7 passed.
  - `cargo nextest run -p claudine --features work-counters --test l1 -E 'test(/config_keyed_entries|config_schema_matrix|config_json_conversion|test_layout/)'`: 47 passed.
  - `cargo nextest run -p claudine --features work-counters --lib -E 'test(/a_loop_document_prepares_as_authored/)'`: 1 passed.
  - `cargo nextest run -p claudine-cli --features test-fixtures --test l1 -E 'test(/config_json5_wiring|test_placement/)'`: 14 passed.
- The subagents' wider scoped runs also passed, alongside `just lint` in `darkmatter/` and `claudine/`: Darkmatter lifecycle/frontmatter 539 and CLI 88; Claudine config/prepare lib 557, l1 config and guards 55, CLI 110; simplified-schema scoped 143, and 128 under `--no-default-features`. Every new test ran in under 0.5 s.
- No failures, slow tests, or skipped gates. No cross-OS check: no path, process, or `#[cfg]` code changed, and the new tests build paths from temp dirs.

### Requirement → test mapping (Phase 4)

- Field validators: `required_defaults.rs::{input_shapes, raw_schema_inputs}`, `not_empty_values.rs`, `envelope_null_carriers.rs`, `lifecycle_{edit_,}schema_parity.rs`, `config_keyed_entries.rs`, and `config_schema_matrix.rs` (F1, public crate APIs).
- Loaders: `input_robustness.rs` (admitted YAML, raw JSON text), `frontmatter_extraction.rs`, `config_json_conversion.rs`, and `config_schema_matrix::a_duplicate_key_or_malformed_yaml_is_a_located_problem` (F1/F2, temp dirs).
- CLI wiring: `compose_loader_wiring.rs` and `config_json5_wiring.rs` (F3, spawned binaries, no launch or publication).
- Prepare path: `a_loop_document_prepares_as_authored_and_an_invalid_loop_flag_is_refused` (F2).

## Phase 5

### Orchestration

- Wave 10 ran as three parallel documentation writers, one per area (SimplifiedSchema, Darkmatter, Claudine), each owning only its area's `docs/`, repo skill, and canonical caller-skill sources. None ran a test suite. The orchestrator then published caller skills, ran the audit search, and ran the gates alone.

### Wave 10 — Document library semantics, Darkmatter behavior, Claudine migration (complete)

- **SimplifiedSchema**: `validation.md` is now the canonical page for the new rules: default application and validation as separate steps (diagrams redrawn), the missing/`null`/`required` table with the positional-argument/tuple rule, missing versus `null` parents, `null` and recursive defaults, compile-time conflicts, the public `DefaultPlan`/`prepare_defaults` API, and why exported JSON Schema applies no defaults. `types.md` replaces "Non-Null Properties" with "Empty Values (`not-empty`)" (per-type table, `yaml`/`json` judging text, a `non-null` migration table) and rewrites the phase table; `unions.md` adds defaults in unions and an optional-positional-argument example; `schema-documents.md` swaps `non-null` for `not-empty` and adds a `null`/empty carrier table; `foundations.md` and `conversion.md` now say `1e400` loads from YAML as the text `"1e400"` while the `.json` loaders refuse it; `diagnostics.md` lists `InvalidDefault`, `ConflictingDefaults`, `RecursiveDefault`. Eleven other stale pages were corrected (see `docs_updated_during_phase_5`). The repo skill `SKILL.md` went from 262 to 197 lines, with new `defaults.md` and `shipped-and-identities.md`.
- **Darkmatter**: `inline/schema-validation.md` carries the missing/`null` rule, the requested Mermaid flow (authored values → defaults → conversion → final validation → output, authored values retained), and a table of where defaults appear (composed copy, report, expression typing, never the source file or `md set`). Lifecycle pages replace the literal-only `edit` flag guidance with the real rules (boolean-like text converts; omitted or `null` takes the default; a number or template is an error), keep mandatory-target rules, separate duplicate authored placement from completed defaults, and say `initialize` runs before schema validation and keeps its effects. `numbers.md` and `frontmatter.md` drift fixed. Trigger pages now treat `unknown(required)` as a presence gate. Repo skill and the `use-darkmatter`/`use-dmls` sources updated.
- **Claudine**: configuration (before/after loaded values for TTS and steering, rejected empty overrides, no repository off switch for messaging, keyed route kinds, flat-format migration error text, JSON5 conversion, unchanged runtime enums), messaging, configuring-actions, schemas, lifecycle, looping, and composition pages; repo skill and `use-claudine/{config,lifecycle,composition,library}.md`. Stale `edit` rules (`edit: 5`, `gui: "yes"`, `create_new: null` called errors) were corrected to match `lifecycle_edit_schema_parity.rs`.
- Discrepancies resolved toward the code are listed in the writers' reports and summarized in `decisions_for_review` (property-level union default conflicts are `SchemaError::Convert`; `[code, null]` no longer makes a required named type optional; `categories: []` is rejected; `null | literal(a; required)` is `NullAdmitted`).

### Wave 11 — Publish caller skills (complete)

- `just publish-skills` in `darkmatter/` published `use-darkmatter` and `use-dmls`; in `claudine/` it published `use-claudine`. The published copies have no stray triple braces except one pre-existing, intentional fenced literal in `use-claudine/lifecycle.md`.
- Audit search (`NonNull`, `non-null`, `AmbiguousDefaults`, null-cancels-required, optional-defaults-as-metadata) over source, schemas, docs, and skills. Remaining legitimate categories:
  - the grammar's rejection of `non-null` (`grammar.rs::NON_NULL_REMOVED`, `simplified/mod.rs`) and tests pinning it (`declaration_grammar.rs`, `matcher.rs`, `not_empty_values.rs`, DMLS completion tests asserting it is not offered);
  - descriptive prose meaning "a value other than null" (Claudine trigger schemas and README, matcher and validator comments, protocol parsers, signal `exists`);
  - Darkmatter's expression-typing `FactType::NonNull` / `NON_NULL_FACT_TYPE`, a fact kind unrelated to the schema marker;
  - comments that a required property's type gets no `null` arm (`convert.rs`, Darkmatter schema tests), which still holds;
  - docs and caller skills that mention `non-null` only to say it was removed;
  - historical specs, completed features, frozen benchmark fixtures, and unrelated packages (schematic, tree-hugger, playa).
  - No `AmbiguousDefaults` remains anywhere active; no "metadata only" claim remains in the three areas.

### Phase 5 verification

- `git status … | xargs python3 scripts/ci/affected_scope.py`: `packages` = claudine, claudine-cli, darkmatter, darkmatter-cli, simplified-schema, test-toolkit, darkmatter-lsp; `change_class` = package (Phase 4's uncommitted tests are also in the tree). Gates were therefore run. With only the docs/skills, the resolved plan (`--resolved-plan`) still selects narrowed cells in darkmatter, darkmatter-lsp, claudine-cli, and test-toolkit.
- First dependent run found one failure: `darkmatter::l1 schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes` pinned three sentences the rewrite replaced (they stated the old "null counts as absence" rule). Updated the pins to the new sentences; the test's validator re-derivation is unchanged.
- `simplified-schema` `just test`: exit 0; L1 2237 and minimal 1878 tests, 0 at or above 5.0 s; `just slow-tests fresh` exit 0. `just lint`: exit 0.
- `darkmatter` `just lint`: exit 0 (after the test edit).
- Dependents, serial, from the resolved plan's filters plus doc/skill/guard/corpus tests in the binaries it selects whole:
  - `cargo nextest run -p darkmatter --test l1 -E 'test(/schema_phase_validation|current_root_documentation_contract|current_root_migration_guard|trigger_spelling_guard/)'`: 25 passed.
  - `cargo nextest run -p darkmatter --lib --test l1 -E '(binary_id(darkmatter) & test(/narrative_doc_function_table_matches_catalog|query_vocabulary_link_resolves_to_an_existing_doc_anchor/)) | (binary_id(darkmatter::l1) & test(/doc|skill|guard|corpus|readme/))'`: 463 passed.
  - `cargo nextest run -p darkmatter-lsp --lib --test l1 -E '(binary_id(darkmatter-lsp) & test(=overlay::expressions::tests::embedded_vocabulary_matches_the_topic_doc)) | (binary_id(darkmatter-lsp::l1) & test(/doc|skill|guard|lsp_session|anchor|link/))'`: 280 passed.
  - `cargo nextest run -p claudine-cli --features test-fixtures --test l1 -E 'test(/doc|skill|guard|loop_gate_ambient|readme/)'`: 255 passed.
  - `cargo nextest run -p test-toolkit -E 'test(=active_ci_authority_matches_the_retirement_contract)'`: 1 passed.
- No slow tests, no skipped gates, no pre-existing failures. No cross-OS check: documentation and one string-pin test edit only.

### Requirement → test mapping (Phase 5)

- Docs agree with the validator on the required/eager/phase matrix: `schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes` (F1; pins prose in `types.md`, `inline/schema-validation.md`, and the Darkmatter skill, then re-derives the matrix from the real validator).
- Doc anchors and vocabulary tables still resolve: Darkmatter catalog doc tests and DMLS `embedded_vocabulary_matches_the_topic_doc`.
- Behavior itself is covered by Phases 1–4; Phase 5 changed no behavior.

## Phase 6

### Orchestration

- No subagents. The tree started clean: Phases 1–5 were committed (`15fc97b83` … `c08c55754`), so Phase 6's only edits are one doctest import and the planning records. The gates ran one area at a time with nothing else from this session on the host (load average about 6–13 on 16 cores at the start).

### Wave 12 — Run scoped gates

- `simplified-schema` `just test`: exit 0. L1 (`work-counters,terminal`) 2237 passed, minimal (`--no-default-features`) 1878 passed, "2237 test(s), none at or above 5.0s". `just lint`: exit 0.
- `simplified-schema` `just doctest`: the first run failed one doctest, not caused by this fix: `triggers/assemble.rs::PayloadMemoTally` still imported `darkmatter::markdown::schemas::triggers::PayloadMemoTally`, left over from when the trigger code moved out of Darkmatter (commit `20f02a834` wrote it in Darkmatter). The import was changed to `simplified_schema::triggers::PayloadMemoTally` (a re-export from `triggers/mod.rs`). Rerun: 40 passed, 3 ignored, exit 0. This is a doc-comment change only.
- `darkmatter` `just test` (library, CLI, DMLS): exit 0, 16657 passed, 10 skipped by the recipe, "none at or above 5.0s". `just lint`: exit 0. `just doctest`: 214 passed, 8 ignored.
- `claudine` `just test`: exit 0, 12164 passed, 15 skipped by the recipe, "none at or above 5.0s". `just lint`: exit 0. `just doctest`: exit 0.
- After the doctest fix, `simplified-schema` `just lint` and `just test` were rerun on the final tree: both exit 0, same counts, none slow. `just slow-tests fresh`: exit 0. `just check-tier-coverage` for all three areas: 0 stranded. Root `just shipped-schemas`: "every shipped artifact is current".
- Dependents: the full Darkmatter and Claudine gates above ran on the final tree, so they cover every consumer of the library except Messenger's `research` feature. The one Phase 6 code change is a doctest import inside a `///` comment, which no dependent compiles, so Messenger was not run.
- No failures besides the pre-existing doctest, no slow tests, and no skipped gates. No cross-OS check: Phase 6 changed no path, process, or `#[cfg]` code.

### Wave 13 — Record completion evidence

- Wrote `implementation.md` beside the plan. It records what changed in each area, the `non-null` → `not-empty` migration table, completed values that changed, artifact and identity revisions (preparation 2→3 / 3→4 / 3→4, projection 1→2), departures from the plan, the input-matrix coverage, the spec's Verification items mapped to tests, and the final gate results. It covers the keyed config migration rationale (provider defaults that used to share one path) and the former `active_messenger: null` disable behavior explicitly.
- Checked against the code: every test named in `implementation.md` exists; `materialize_optional_document_bindings` is gone; `applied_defaults` replaced `required_defaults`; `RepoOverrideConfig::active_messenger` is `Option<String>`; the three `PREPARATION_REVISION` constants read 3/4/4.
- Plan tasks: all checked; `total_phases: 6` matches the plan's six phases. Diff scope for Phase 6: `simplified-schema/lib/src/triggers/assemble.rs` (doc comment) and this fix directory.

### Requirement → test mapping (Phase 6)

- Phase 6 added no behavior. Its one requirement, a doctest for a changed public API that compiles and runs, is met by `just doctest` in `simplified-schema/` (`triggers::assemble::PayloadMemoTally`, F1). The spec's Verification items map to tests in `implementation.md` → "Acceptance".

### Scope calculator (after every Phase 6 edit)

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`: `packages` = `simplified-schema`, `test-toolkit`, `claudine`; `change_class` = `package`. So the gates were required, and they ran (above). The `test-toolkit` cell is the archive-path guard, selected by the changed `assemble.rs`. `cd tools/test-toolkit && just archive-path-guard`: 2 passed, exit 0 (the slowest took 4.8 s; it is that guard's own build-and-scan test, and this fix did not change it).

Implementation complete, ready for review.

## Review record corrections — 2026-11-15

- The decision inventory still said the overflow-number documentation was
  left unchanged. Phase 5 had already corrected
  `simplified-schema/docs/topics/foundations.md` and
  `darkmatter/docs/topics/numbers.md`: YAML loads `1e400` as text, while the
  `.json` file loader rejects it as an out-of-range number. Verified by reading
  both current pages and rerunning `just test` in `simplified-schema/`, including
  `setter_values::spellings_yaml_reads_as_strings_are_never_rounded`.
- The decision inventory still said loop preparation had no end-to-end test.
  Phase 4 added
  `a_loop_document_prepares_as_authored_and_an_invalid_loop_flag_is_refused`.
  Verified its assertions and its passing execution (0.548 s) in the review's
  `claudine/` L1 run. This does not establish coverage of interpolated loop
  provenance, which remains a separate recorded limitation.
- Narrowed the statement that a null flag is omitted "everywhere": `set`
  still refuses an authored `no_error: null`. Verified through the public
  lifecycle parser and
  `lifecycle_schema_parity::completed_flags_keep_the_schema_and_the_md_parser_in_agreement`.

## Review 2 record corrections — 2026-11-15

The decision inventory still presented the first review's lifecycle gaps as
current behavior: authored flags beside `then`, numeric inner flags on md's
untyped operands, bare `when` items, and duplicate flag placements. Updated
those statements in place. The current shared passive checks refuse each of
them, as the parser does. Verified the implementations and the agreement tests
in Darkmatter's full `just test` run: 16,730 tests passed, none at or above
5 s. The earlier phase sections retain their historical test names and results.
Also corrected the claim that preparation ignores `required`: it still reads
that marker when deciding whether to insert a missing null default. Verified
against `Fill::fill` and the passing required/optional-null-default tests.
