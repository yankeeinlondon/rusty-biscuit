---
kind: plan
total-phases: 6
created: 2026-11-22
status: planned
phase: 6
agent: codex/gpt-6.2-sol
yolo: true
packages:
- simplified-schema
- darkmatter
- darkmatter-cli
- darkmatter-lsp
- claudine
- claudine-cli
- test-toolkit
source_files_during_phase_1:
- simplified-schema/schemas/namespace-schema.yaml
- simplified-schema/schemas/schema-trigger.yaml
- simplified-schema/schemas/schema.yaml
- simplified-schema/lib/generated/shipped.json
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/bundle.rs
- simplified-schema/lib/src/resolve/buffers.rs
- simplified-schema/lib/src/resolve/select.rs
- simplified-schema/lib/src/schema/export.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/shipped/registry.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/standalone.rs
- simplified-schema/lib/src/triggers/assemble.rs
- simplified-schema/lib/src/triggers/discovery.rs
- simplified-schema/lib/src/triggers/envelope.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/src/triggers/mod.rs
- simplified-schema/lib/src/triggers/prepared.rs
- simplified-schema/lib/src/yaml_documents/mod.rs
- simplified-schema/lib/src/yaml_documents/registry.rs
- simplified-schema/lib/src/yaml_documents/select.rs
- simplified-schema/lib/tests/l1/bundle_resolution.rs
- simplified-schema/lib/tests/l1/kind_envelopes.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/shipped_artifacts.rs
- simplified-schema/lib/tests/l1/yaml_kind_selection.rs
- darkmatter/schemas/darkmatter-host.yaml
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/src/config/registration.rs
- darkmatter/lib/src/config/registration/bundle.rs
- darkmatter/lib/src/config/registration/manifest.rs
- darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
- darkmatter/lib/src/markdown/output/command/data.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/tests/l1/host_registration_snapshot.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
- darkmatter/lib/tests/l1/yaml_kind_registry.rs
- darkmatter/cli/schemas/output/schema-triggers.yaml
- darkmatter/cli/src/commands/schema/triggers.rs
- darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
- darkmatter/dmls/src/providers/frontmatter/schema_value_hover.rs
- claudine/lib/generated/shipped.json
- claudine/lib/src/composition/schema/shipped.rs
- claudine/lib/tests/l1/invalid_trigger_findings.rs
- claudine/cli/tests/l1/invalid_trigger_warnings.rs
docs_updated_during_phase_1:
- simplified-schema/docs/topics/language/schema-documents.md
- simplified-schema/docs/topics/language-core.md
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/selecting-schema-files.md
- darkmatter/docs/topics/host-registration.md
- darkmatter/docs/topics/schemas/schema-triggers.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
- darkmatter/docs/cli/clean.md
- darkmatter/dmls/docs/diagnostics.md
- claudine/docs/topics/schemas.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/SKILL.md
- darkmatter/docs/use-skills/use-dmls/schemas.md
source_files_during_phase_2:
- claudine/lib/src/composition/lifecycle/mod.rs
- claudine/schemas/globals.lifecycle.json
- claudine/lib/tests/l1/host_catalog.rs
- claudine/lib/tests/l1/host_publication.rs
- simplified-schema/lib/src/resolve.rs
- darkmatter/lib/src/config/load.rs
- darkmatter/lib/src/config/registration.rs
- darkmatter/lib/src/config/registration/manifest.rs
- darkmatter/lib/src/config/registration/namespaces.rs
- darkmatter/lib/src/config/registration/selection.rs
- darkmatter/lib/src/lifecycle/host_profile.rs
- darkmatter/lib/src/lifecycle/host_profile/tests.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/expression/binding.rs
- darkmatter/lib/src/markdown/schemas/catalog.rs
- darkmatter/lib/src/markdown/schemas/catalog/discovery_tests.rs
- darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/benches/pipeline_work_support/mod.rs
- darkmatter/lib/tests/host_fixtures/mod.rs
- darkmatter/lib/tests/l1/host_functions.rs
- darkmatter/lib/tests/l1/host_namespaces.rs
- darkmatter/lib/tests/l1/host_registration.rs
- darkmatter/lib/tests/l1/host_selection.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/cli/tests/l1/host_registration.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/dmls/benches/pipeline_work_a3.rs
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/src/lib.rs
- darkmatter/dmls/src/overlay/event_views.rs
- darkmatter/dmls/src/overlay/expressions.rs
- darkmatter/dmls/src/overlay/hosts.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/src/providers/lifecycle_host_tests.rs
- darkmatter/dmls/src/providers/mod.rs
- darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
- darkmatter/dmls/tests/l1/claudine_host.rs
- darkmatter/dmls/tests/l1/edit_action_editor.rs
- darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
- darkmatter/dmls/tests/l1/host_function_diagnostics.rs
- darkmatter/dmls/tests/l1/host_globals.rs
- darkmatter/dmls/tests/l1/host_registration_watches.rs
- darkmatter/dmls/tests/l1/lifecycle_event_views.rs
- darkmatter/dmls/tests/l1/lifecycle_host_profile.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/registered_host_editor_agreement.rs
- darkmatter/dmls/tests/l1/registered_host_views.rs
docs_updated_during_phase_2:
- darkmatter/docs/topics/host-registration.md
- darkmatter/docs/topics/lifecycle/host-artifact.md
- darkmatter/docs/topics/host-functions.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/docs/lsp/features.md
- darkmatter/dmls/docs/autocomplete.md
- darkmatter/dmls/docs/diagnostics.md
- darkmatter/dmls/docs/features.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/expressions.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/darkmatter/lifecycle.md
- .claude/skills/darkmatter/schema.md
- darkmatter/docs/use-skills/use-dmls/schemas.md
source_files_during_phase_3:
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/triggers/assemble.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/src/triggers/prepared.rs
- darkmatter/lib/src/config/registration.rs
- darkmatter/lib/src/config/registration/merge.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/tests/l1/host_trigger_merge.rs
- darkmatter/lib/tests/l1/registered_yaml_kinds.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/cli/schemas/output/schema-validate.yaml
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/tests/l1/host_registration.rs
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/yaml_contract.rs
- darkmatter/dmls/src/overlay/yaml_document.rs
- darkmatter/dmls/tests/l1/lsp_session.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/registered_trigger_conflict.rs
- darkmatter/dmls/tests/l1/registered_yaml_kinds.rs
docs_updated_during_phase_3:
- darkmatter/docs/topics/host-registration.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/SKILL.md
- darkmatter/docs/use-skills/use-dmls/schemas.md
source_files_during_phase_4:
- claudine/registration/action.yaml
- claudine/registration/claudine-config.yaml
- claudine/registration/composition.yaml
- claudine/registration/content-policy.yaml
- claudine/registration/document.yaml
- claudine/registration/formal-sequence-file.yaml
- claudine/registration/group.yaml
- claudine/registration/inline-composition.yaml
- claudine/registration/lifecycle-events.yaml
- claudine/registration/linking.yaml
- claudine/registration/loop.yaml
- claudine/registration/seq.yaml
- claudine/registration/sequence-document.yaml
- claudine/registration/sequence.yaml
- claudine/registration/shared.yaml
- claudine/schemas/action.yaml
- claudine/schemas/claudine-config.yaml
- claudine/schemas/claudine-document.yaml
- claudine/schemas/claudine.yaml
- claudine/schemas/document.yaml
- claudine/schemas/globals.lifecycle.json
- claudine/schemas/globals.yaml
- claudine/schemas/inline-compose.yaml
- claudine/schemas/lifecycle.yaml
- claudine/schemas/linking.yaml
- claudine/schemas/sequence-types.yaml
- claudine/schemas/sequence.yaml
- claudine/lib/build.rs
- claudine/lib/Cargo.toml
- claudine/lib/generated/globals.lifecycle.json
- claudine/lib/generated/shipped.json
- claudine/lib/examples/lifecycle_artifact.rs
- claudine/lib/examples/shipped_schemas.rs
- claudine/lib/benches/pipeline_work.rs
- claudine/lib/benches/pipeline_work_alloc.rs
- claudine/lib/src/composition/mod.rs
- claudine/lib/src/composition/inventory.rs
- claudine/lib/src/composition/inventory/walk.rs
- claudine/lib/src/composition/inventory/tests.rs
- claudine/lib/src/composition/lifecycle/mod.rs
- claudine/lib/src/composition/lifecycle/tests/nested_span.rs
- claudine/lib/src/composition/registration.rs
- claudine/lib/src/composition/registration/legacy.rs
- claudine/lib/src/composition/registration/published.rs
- claudine/lib/src/composition/schema/bundle.rs
- claudine/lib/src/composition/schema/globals.rs
- claudine/lib/src/composition/schema/mod.rs
- claudine/lib/src/composition/schema/shipped.rs
- claudine/lib/src/composition/sequence/mod.rs
- claudine/lib/src/composition/sequence/declarations/tests.rs
- claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
- claudine/lib/src/config/atomic.rs
- claudine/lib/src/config/schema.rs
- claudine/lib/src/config/setup.rs
- claudine/lib/tests/l1/claudine_identification.rs
- claudine/lib/tests/l1/claudine_schema_parity.rs
- claudine/lib/tests/l1/claudine_schema_shipped_corpus.rs
- claudine/lib/tests/l1/claudine_schemas.rs
- claudine/lib/tests/l1/claudine_schemas_embedded.rs
- claudine/lib/tests/l1/claudine_schemas_matrix.rs
- claudine/lib/tests/l1/claudine_trigger_selection_parity.rs
- claudine/lib/tests/l1/container_schema_recursion.rs
- claudine/lib/tests/l1/context_construction_guard.rs
- claudine/lib/tests/l1/embedded_inventory_preflight.rs
- claudine/lib/tests/l1/host_catalog.rs
- claudine/lib/tests/l1/host_publication.rs
- claudine/lib/tests/l1/installed_registration.rs
- claudine/lib/tests/l1/installed_registration_agreement.rs
- claudine/lib/tests/l1/main.rs
- claudine/lib/tests/l1/registration_inventory.rs
- claudine/lib/tests/l1/sequence_schema.rs
- claudine/lib/tests/l1/shipped_artifacts.rs
- claudine/cli/src/args.rs
- claudine/cli/src/main.rs
- claudine/cli/src/telemetry.rs
- claudine/cli/src/commands/mod.rs
- claudine/cli/src/commands/registrations.rs
- claudine/cli/src/commands/init_wizard.rs
- claudine/cli/src/completion/schema_completion/tests.rs
- claudine/cli/tests/l1/error_guards/transport-allow.toml
- claudine/cli/tests/l1/host_registration_refresh.rs
- claudine/cli/tests/l1/launch_trigger_schemas.rs
- claudine/cli/tests/l1/main.rs
- claudine/cli/tests/l1/registrations_command.rs
- claudine/cli/tests/l1/shipped_prompts.rs
- claudine/cli/src/commands/wrap/runaway_guard.rs
- claudine/cli/tests/l1/implement_plan_commit_message.rs
- claudine/cli/tests/l1/lifecycle_refusal_reports.rs
- claudine/cli/tests/l1/lifecycle_edit_action.rs
- claudine/justfile
- darkmatter/lib/src/config/init.rs
- darkmatter/lib/src/config/registration.rs
- darkmatter/lib/src/config/registration/bundle.rs
- darkmatter/lib/src/config/registration/manifest.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/benches/pipeline_work_support/mod.rs
- darkmatter/lib/tests/l1/base_schema_end_to_end.rs
- darkmatter/lib/tests/l1/config_init.rs
- darkmatter/lib/tests/l1/config_widening.rs
- darkmatter/lib/tests/l1/context_construction_guard.rs
- darkmatter/lib/tests/l1/embedded_registration.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/schema_phase_validation.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/trigger_spelling_guard.rs
- darkmatter/cli/tests/l1/claudine_schema_bundle.rs
- darkmatter/cli/tests/l1/init_command.rs
- darkmatter/cli/tests/l1/schema_validate.rs
- darkmatter/dmls/src/config_contracts.rs
- darkmatter/dmls/src/context.rs
- darkmatter/dmls/src/diagnostics/nested_span.rs
- darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
- darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
- darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
- darkmatter/dmls/tests/l1/config_contracts.rs
- darkmatter/dmls/tests/l1/edit_action_editor.rs
- simplified-schema/lib/src/prepared.rs
- simplified-schema/lib/src/shipped/wire_support.rs
- simplified-schema/lib/tests/l1/shipped_corpus.rs
- simplified-schema/lib/src/phase.rs
- simplified-schema/lib/tests/l1/phase_projection.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/schema_identities.rs
- tools/test-toolkit/tests/shipped_schemas_coordinator_contracts.rs
- scripts/shipped_schemas.py
- justfile
- Cargo.lock
- .github/ci/permission-inputs.json
docs_updated_during_phase_4:
- claudine/schemas/README.md
- claudine/docs/cli/index.md
- claudine/docs/dependencies.md
- claudine/docs/topics/configuration.md
- claudine/docs/topics/schemas.md
- darkmatter/docs/topics/host-registration.md
- darkmatter/docs/topics/lifecycle/host-artifact.md
docs_created_during_phase_4:
- claudine/docs/cli/registrations.md
skills_files_updated_during_phase_4:
- .claude/skills/claudine/SKILL.md
- .claude/skills/claudine/module-map.md
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/lifecycle.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/simplified-schema/defaults.md
- claudine/docs/use-skills/use-claudine/config.md
source_files_during_phase_5:
- darkmatter/dmls/src/overlay/hosts.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/router.rs
- darkmatter/dmls/src/work_counters.rs
- darkmatter/dmls/src/workspace/watch.rs
- darkmatter/dmls/tests/common/mod.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/registration_reload_scheduling.rs
- darkmatter/dmls/tests/l1/host_registration_watches.rs
- darkmatter/dmls/tests/l1/registered_yaml_kinds.rs
docs_updated_during_phase_5:
- darkmatter/docs/topics/host-registration.md
- darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/darkmatter/dmls.md
---

# Host registration v2 implementation plan

## Summary and Definition of Done

Ship one portable Claudine inventory and one passive Darkmatter reader for it. Claudine publishes digest-addressed bytes at fixed paths during setup, install, and explicit publication. Darkmatter validates the entire snapshot, selects Markdown hosts from ordinary trigger matches, builds a checked trigger layer, and selects registered YAML kinds after built-in and local lookup. DMLS keeps the last loaded view through a burst of publication events and installs the latest completed view for both Markdown and YAML.

Done means the reader, CLI, editor, and Claudine's in-process validation agree on isolated fixtures. An interrupted or overlapping publish yields a complete release or a located refusal, retry repairs the installation, and a byte-identical publish leaves timestamps alone. Setup preserves unrelated registration and permissions state. The embedded inventory, generated artifacts, and published bytes agree. The old manifest format, legacy schema-root host discovery, and normal-command refresh are removed. The documentation and caller skills describe the shipped behavior, and the scoped acceptance suites pass.

Start on the tree containing the completed `2026-11-22-object-composition` prerequisite. Its spec is finalized in the planning tree, and the current resolver lacks the planned composition support. Use that prerequisite's resolved object model for `$extends`; this plan's first checkpoint proves the dependency through a real schema. Tasks outside waves run in order. Tasks inside a wave can run concurrently, with the file ownership stated below. Every delegated brief must state that the entire session tree is non-interactive. Keep writes in the session worktree. Run the phases through final verification without a human gate. Update current documentation for each behavior change in the phase that lands it, then complete the cross-package examples and skill publication in Phase 6. Leave the feature active for review.

## Phase 1 — Schema and snapshot foundations

### Necessary Rules

1. **Use one closed member contract.** Extend `simplified-schema/schemas/{schema-trigger,schema}.yaml` and add `namespace-schema.yaml` to `simplified-schema/lib/src/envelope.rs`'s shipped definitions. Trigger `description` accepts a string, trigger and namespace `types` accept a mapping including an empty table, and optional fields reject explicit null. Namespace `$schema` accepts one object, including an empty object, with local types. A registered library requires `kind: schema` and a nonempty `types` table and forbids `$schema`. An ordinary `kind: schema` envelope keeps its schema-exporting forms and permits `name` only in the types-only form. Use `^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$` for names, exact comparison, and case-sensitive namespaces. Reuse `expression/binding.rs::check_root` for identifier syntax and reserved binding roots, and refuse `null` via the same protection used by `protected_kind`. Reject collisions with `DARKMATTER_GLOBALS`. Preserve `seq`, `loop`, and `group`, which the draft namespace files declare and Darkmatter's reserved root catalog leaves to hosts.

2. **Keep labels and type scope at the declaration.** Parse labels into `TriggerEnvelope` and its prepared record. Scope discovery uniqueness to each physical schema root and each embedded source root, before shadowing, including eligible types-only files. A duplicate marks both participants and excludes their definitions from that root's usable set. Registration refuses the entire profile and points to every duplicate declaration. Keep path-based identity, file suppression, and lookups. Carry `name` into `md schema triggers`'s row and output schema. For a known label show `name (path)` in human diagnostics and preserve the path and span in machine diagnostics. `triggers/discovery.rs` already owns source identity and findings, and `triggers/prepared.rs` reconstructs envelopes for embedded consumers. Trigger-private named types resolve inside their own payload and reachable declarations; external imports of a trigger remain refused in the resolver. Apply the same private scope to namespace files, which are dedicated catalog inputs and cannot be selected by ordinary root `$schema`.

3. **Use a closed, pre-read bundle.** In `darkmatter/lib/src/config/registration/{manifest.rs,bundle.rs}`, require all eight v2 fields, integer version 2, nonempty host and version strings, explicit arrays for `files` and `functions`, a closed `{catalog, facts}` object per function, and a digest table whose keys match the authored member entries exactly. Allow `files: []` and `functions: []`; the lifecycle member remains mandatory. Resolve listed paths using `FileReference` anchored to the manifest, then normalize identity to slash-separated bundle-relative paths. Require a leading `./` or `../`, forward slashes, and portable ASCII filename segments. Check for Windows reserved device names, trailing periods, and ASCII case-insensitive component collisions before publishing. Accept in-root dot traversal after resolution and reject escaping paths, path aliases with the same resolved identity, case-fold collisions, and symlink or junction components inside the bundle. Embedded built-ins use the existing `darkmatter:` mappings in `markdown/schemas/embedded.rs`. This preserves copied-install identity and keeps manifests authored for one platform usable on the others.

4. **Resolve against verified bytes throughout.** Introduce a bundle-specific resolver scope in SimplifiedSchema's `resolve/select.rs` and `resolve/buffers.rs`, independent of the existing template grant scope in `slots/reads.rs`. Apply it before fallback or open in import, payload, object composition, and example resolution, including post-template preparation. Disk resolution captures local identities through `FileReference`; embedded preparation uses virtual identities and the same relative path and member rules. Both read the verified byte table exclusively. Track attempted and missing local paths for diagnostics and watches without reading unlisted targets. Apply the normal permission check independently for explicitly evaluated templates. File-backed examples get the same boundary and their normal envelope check. A standalone `kind: example` is refused as a registration member, so v2 ships no standalone file-backed examples. The four-kind list in spec §1 and `example.rs::validate_example_bytes` require both checks; cover the unlisted and disallowed-kind cases. Built-in dependencies are trusted shipped inputs and stay outside the per-bundle digest map.

5. **Hash content once per read.** Read raw bytes of the manifest and every addressed member, verify via `biscuit_hash::blake3_hash_bytes` in lowercase hex, then parse the verified snapshot. On missing member or digest mismatch only, read the manifest a second time. Retry once using the second manifest exactly when its bytes differ. Report the final failure with manifest field and member path, publish no profile, and keep dependencies and candidates from both attempts for watch recovery. Keep all distinct failures generated by duplicate-name checks. Use `blake3_hash_json` for revision over host, version, format, sorted normalized file paths, normalized lifecycle and function pairs, and a normalized member-digest map. Sort function pairs by catalog then facts path since the current reader combines named catalogs without using manifest order for precedence. Keep raw manifest hashes separately for watch input comparison. This replaces the root-sensitive dependency bookkeeping in `registration.rs::revision` and preserves `agreement`'s equal-claim collapse and conflicting-claim reporting. Detect format 1 before strict v2 envelope validation and report a located `dm.host.registration` warning directing users to host setup or update. Apply the same warning to repository entries, which Claudine setup leaves in place.

6. **Share namespace and lifecycle preparation.** Put snapshot-to-catalog preparation under `config/registration` and retain the existing `GlobalCatalog` and definition/source records in `markdown/schemas/catalog.rs`. Prepare each namespace as its own object with its real file and span, then combine the declarations with their original definition tables. Give `lifecycle/host_profile.rs::lifecycle_artifact` and `HostLifecycleProfile::from_artifact_at` the prepared namespace catalog hash in place of the old catalog text. Format 2 hashes a canonical JSON path-to-exact-byte-digest map for the union of namespace files and all bundled dependencies used to prepare them. An empty catalog hashes the empty map. Keep the event payload and Darkmatter core binding rules. Check host, artifact version, catalog hash, and declared host namespace bindings before acceptance. Keep the function catalog and facts validation in `markdown/compose/expression/host/artifact.rs` as the implementation for existing pairs. Remove only the root-discovery branches of the global/function readers. Explicit catalogs and the built-in catalog retain their supported APIs.

7. **Select once from authored Markdown.** Change `selection.rs::RegisteredHosts::identify` into the ordinary trigger selection entry point and carry the matching trigger indices with the selected profile in `Selected`. Build the Markdown match input consistently in the schema and editor entry points from authored frontmatter and path, before defaults, coercion, or overrides. Strip the `$schema` control key, as `DarkmatterSchemas::resolve_trigger_layers` does now. Use `triggers::matcher` for literal and path semantics and `admit_identifying_trigger` during snapshot preparation for static admission against `md_owned_names`. Matcher-only registered triggers can identify a host and contribute zero payloads. Compare claims by their matched profiles and retain the existing ambiguity policy. Remove `Legacy`, `LegacyHost::discover`, legacy function/catalog root probes, and `HostView::legacy_inputs`. Keep native Darkmatter selection, scoped host globals, unknown-host-event reporting, and caller-supplied Claudine runtime bindings.

8. **Merge registered matches before applying overrides.** Put the checked layer builder in `config/registration/merge.rs` and call it from `DarkmatterSchemas::resolve_trigger_layers`, pending-slot discovery, and the listing/trace path in `cli/src/commands/schema/triggers.rs`. Use the already-resolved `PropertyDef` and definition tables from SimplifiedSchema and call `is_definition_subtype_with_definitions` bidirectionally. Compare requiredness, effective null admission, constraints, defaults, descriptions, and resolved examples separately. Walk corresponding nested declarations for equivalent structure and match equivalent union arms by type and rules, independent of storage IDs; keep cycle detection for recursive definition pairs. Compare every pair per property before choosing the unique narrowest equivalence class, then the first normalized path as representative of that class. Carry the winning property origin and re-home reached definitions using existing resolved-layer machinery. A single matched payload retains its root rules. Two or more contributing payloads allow a single root union, require default root rules, and refuse writes from other payloads to keys absent or unequal in any arm. Merge ordinary declarations arm by arm, preserving order. Produce a structured `SchemaError` for `dm.host.trigger_conflict` with the document path, host, both trigger paths, and key or root rule. Do the check before discarding properties overridden by higher layers. Keep disk-trigger suppression and the existing baseline → host → local trigger → document schema precedence. Memoize static successes and conflicts per profile revision and sorted matched set in the profile; bypass that memo for all pending-derived results. Existing local-trigger slot pruning remains in place for its own layers.

9. **Add a YAML lookup tier at the unclaimed branch.** Extract the optional low-priority lookup seam at `simplified-schema/lib/src/yaml_documents/select.rs::select` after root failures, local claims, and incomplete definitions are considered. Reuse the existing definition claim extraction in `yaml_documents/registry.rs` for pre-resolved registered `yaml-schema` definitions. Index them under the agreed profiles in `RegisteredHosts`; an unresolved same-host claim with competing definitions remains a selection failure. Collapse copied equal profiles, and report distinct claims sorted by host and path using the existing `KindSelection`/duplicate cause. Carry host and definition identity in the prepared `YamlContract`. Call this seam in `DarkmatterSchemas::prepare_yaml_contract`, which both `md schema validate` and `dmls/src/overlay/yaml_contract.rs` use. Markdown trigger disabling stays on its Markdown route, and whole YAML receives no Markdown layer or lifecycle evaluation.

10. **Migrate Claudine's proven definitions.** Keep `claudine/registration/` as the schema-only source and publish each source-relative name under `schemas/`. Move `action.yaml`, `document.yaml`, and `linking.yaml` from `claudine/schemas/` into it. Replace the draft `registration/lifecycle-events.yaml` with the authoritative types from `schemas/lifecycle.yaml` as a types-only library, preserving `cancelled-stack`, generated verb references, and existing argument rules. Remove the redundant old `schemas/lifecycle-events.yaml`. Copy `content-policy/schemas/content-policy.yaml` byte-for-byte to `registration/content-policy.yaml`; its `@last_updated` strings are suggestions/pattern data and it has no named file import. Keep `review.yaml` in repository discovery. Move the canonical types-only `claudine-config.yaml` to `registration/claudine-config.yaml` because the old publisher already ships it. Point Claudine config validation and repository references to the moved definition. Keep the three namespace drafts and port missing descriptions from `schemas/globals.yaml`. Delete the old `claudine-document.yaml`, `globals.yaml`, and `globals.lifecycle.json` after replacing consumers. The handwritten list in `composition/schema/bundle.rs` already references the deleted `sequence-types.yaml`; restore the current types from `schemas/sequence.yaml`, not that old file or the narrower draft step grammar.

11. **Give each Claudine trigger distinct property ownership.** Put the common properties from `schemas/claudine.yaml` in `registration/shared.yaml::common-attributes`, excluding `prompt` and `sections`. Use the old provider, model, event, control, and linking declarations. Include the managed hash, `last_updated`, and the established `content_policy` property from Darkmatter's baseline, referencing the bundled policy type with the baseline array and minimum. Both `composition.yaml` and the arms of `inline-composition.yaml` extend that common type. Inline selection is required literal `kind: inline-compose`, any non-null `sections`, or any non-null `prompt` in the absence of a non-null `sequence` and `kind: sequence`. Use a `group` of required string, number, boolean, object, and array predicates for each presence test. Composition selection is the inverse of the full inline condition combined with the old host-specific keys in `claudine-document.yaml` or literal `kind: sequence`. Preserve the old optional prompt type in this non-inline payload for normal composition and sequence input. Inline arms require one non-null key and give the other an optional null-only type. Move the old `schemas/sequence.yaml` trigger to `registration/sequence-document.yaml` and move its named types to `registration/sequence.yaml`, preserving its literal/presence alternatives and sequence-only payload. Type `formal-sequence-file.yaml` with a required inline `step[]`, preserving empty lists and the parser's scalar shorthand; root `$schema`, `$state`, and `template` retain their authored semantics. The old inline matcher excludes sequence, `sequence/mod.rs` routes structural lists through the formal parser, and the runtime calls section execution upcoming in `error/render/sequence_loop.rs`. This feature adds sections schema validation and leaves sections execution for that runtime feature.

12. **Prepare once for embedded and installed consumers.** Generate a sorted `(relative_path, source_bytes)` include table in `claudine/lib/build.rs`, with a directory and per-file Cargo rerun directive. Reject unexpected files, nonportable names, symlinks/junctions, case-colliding components, non-UTF-8 sources, and ill-formed member envelopes at build time. Share the pure inventory walk and member-envelope checks with generation through a small source module compiled by both targets; the build target can use SimplifiedSchema and YAML parsing from existing workspace dependencies. Use the Darkmatter bundle preparer for the full generator and installed semantic checks. Give members a bundle-relative identity plus source provenance so embedded consumers never call the existing disk-only `RegisteredTrigger::path()` accessor. Route duplicate suppression using the actual disk identity or the embedded identity and payload digest, as in `same_registered_trigger`. Replace `bundle.rs`'s lists, and change `shipped.rs::{sources,prepare_payload,accept_sections}` and `schema/globals.rs` to derive the checked triggers, kind definitions, and globals from that inventory. Keep prepared provenance through the shipped record and bump affected preparation revisions. In `composition/schema/mod.rs::discover_schema_owner` supply the checked embedded Claudine profile before ordinary root triggers, using embedded Claudine identity in the in-process commands. In `sequence/mod.rs` validate explicitly declared formal `kind: sequence` YAML against the prepared embedded kind using the already parsed document before normalizing it. Keep the runtime grammar, shorthand, offset-selected data and lifecycle execution paths. Existing `prepare_direct_with_schema`, `prepare_inline_with_schema`, and sequence preflight remain the launch entry points. Store the generated lifecycle artifact at `claudine/lib/generated/globals.lifecycle.json`, outside the schema-only source tree. Generate it from the same preparation and encoder through the root `just shipped-schemas` coordinator. The Claudine generator emits the lifecycle candidate with an explicit output argument supplied by `scripts/shipped_schemas.py`; stage and validate it before any generated-file replacement. Embed the artifact as bytes for listing and publication. Generation must be able to run against the prior artifact and may never depend on the prior bytes being current.

13. **Publish with a fixed-path transaction.** Replace `HostRevision`'s revision-folder API in `claudine/lib/src/composition/registration.rs` with `RegistrationInventory` and `publish(home, inventory)`. Use one stable OS file lock beside the output root at `~/.claudine/registrations/.darkmatter.lock`, using the standard file lock and bounded contention style of `darkmatter/lib/src/config/write.rs`. In `claudine/lib/src/config/atomic.rs` expose a registration-only temp-name prefix through the existing sibling-temp writer and reuse its Windows replacement retry. Preflight the output root, path components, member destinations, and authorized removals with no-follow metadata and Windows reparse-point checks before mutation. Under the lock, replace only changed members, remove retired entries taken from the previous valid v2 manifest after the same boundary check, and remove only regular leftover temps with the reserved prefix in publisher directories. Preserve unknown files for doctor to report. Replace the manifest last and only for different bytes. The unchanged case performs no member/manifest write and preserves timestamps. A failed operation leaves the prior manifest available for digest verification. A subsequent publish converges on the embedded bytes. Use the OS lock across cooperating processes, with no in-memory-only substitute.

14. **Keep publication separate from configuration.** Add a hidden `Commands::Registrations` in `claudine/cli/src/args.rs` and dispatch it in `main.rs` before normal Darkmatter config loading and initialization. Allow no argument, an exact slash-separated inventory pathname, or the reserved positional token `publish`; that token cannot collide with a member pathname. List and detail use the embedded inventory and need no home or installed files. Publish needs the captured home and writes to stdout only after success. Output pretty JSON plus newline with sorted paths, optional `name` omitted when absent, exact UTF-8 contents, and lowercase 16-digit seed-0 XXH64 using `biscuit_hash::xx_hash_bytes`. Remove `refresh_host_registration` and its main dispatch branch. The existing first-run initialization can still explicitly invoke setup. Use Claudine’s existing `build --release` output to run `registrations publish` after `_install` in `claudine/justfile`. Resolve the release executable using Cargo metadata’s target directory and the native executable suffix, so custom install roots and an older PATH binary cannot redirect publication. The install recipe builds the inventory and binary from this worktree before installing, and install flags change neither source membership nor registration content. Propagate publication failure. Doctor and update are follow-on features; the public inventory and publish/setup functions are their interface.

15. **Migrate settings in place.** Extend `darkmatter/lib/src/config/init.rs::register_manifest` with `replace_manifest_registration(context, scope, reference, old_path)` and retain the existing append entry point for other callers. Resolve the new path through `manifest_identity` and use a captured-context path candidate for an absent old file. Match only the known old default path, canonical equivalents, and the new entry's equivalents. Keep the canonical new spelling at the first affected index, perform duplicate removals in descending index order and append only when absent, through `ConfigTransaction`'s source-byte expectation and YAML edits. Preserve neighboring sequence comments and unrelated entries. In `setup_claudine`, publish, run the existing permissions handoff, then migrate the user registration even when the handshake reports a destination conflict. A Claudine config write failure keeps the current setup failure behavior. After registration succeeds, remove only the exact old manifest and revision tree under `~/.claudine`. Preflight the full legacy tree for symlinks/junctions and report an obstruction without following it. Leave repository configuration untouched. This preserves the current ordering in `config/setup.rs::setup_claudine` and extends the already-locked config writer; no separate doctor config editor is required.

16. **Debounce only registration work.** Extend `dmls/src/overlay/hosts.rs::HostRegistrations` with a deadline and generation per existing configuration key, retaining the last `InstalledHosts` while dirty. Drive deadlines from the router loop and send work through its existing registry worker. Watch events reset the deadline across batches, and save fallback queues the same input refresh. Stamp `HostPreparation` and `PreparedHosts` with the generation; ignore stale completions and start the latest job after its deadline. Keep watch inputs for failures and refresh watch coverage after every accepted completion. Use effective revision and diagnostic comparison separately so a watch/fingerprint-only change needs no document analysis; a broken or repaired view updates diagnostics and invalidates affected Markdown and YAML contracts. Include the installed host version in `YamlContractStore::key` and pass the same loaded view to off-thread YAML preparation. Keep the ordinary trigger, document, and config invalidation paths on their current schedules. Existing `HostRegistrations::{begin,install,invalidate_paths}`, `RegistryQueue` in `router.rs`, and `workspace/watch.rs::schema_watch_registration` provide these seams. The spec's approximate file count gives no duration measurement. Run the one Phase 5 sample on Linux; no broader performance spike is part of this feature.

- [x] **Fixture bootstrap**
  - Restore compile-time fixture inputs before the Phase 2 and 3 checkpoints. Point the existing includes of the deleted `claudine/schemas/sequence-types.yaml` in SimplifiedSchema's `shipped_corpus.rs`, Darkmatter CLI's `claudine_schema_bundle.rs`, and DMLS's `claudine_schema_discovery.rs` at the checked-in `darkmatter/lib/benches/fixtures/claudine-schemas/sequence-types.yaml` for those intermediate runs. Phase 4 moves these live test consumers to the v2 inventory. Claudine's own deleted-file imports are fixed by its Phase 4 embed cutover and stay outside the earlier test targets.

### Wave 1

- [x] **Kind envelopes**
  - Own SimplifiedSchema kind files, `envelope.rs`, and the trigger envelope/discovery/prepared label path. Add per-root duplicate diagnostics, trigger-local type tests, and namespace exclusion from ordinary schema-target selection. Exercise the prerequisite by resolving a library import through `$extends` and asserting inherited property provenance.
- [x] **Manifest envelope**
  - Own `darkmatter/schemas/darkmatter-host.yaml` and `config/registration/manifest.rs`. Add the v2 reader, normalized member specification, raw manifest locations, and the upgrade diagnostic. Implement initial manifest shape tests using the public loader once the snapshot task lands.

### Wave 2

- [x] **Bundle resolver**
  - Own SimplifiedSchema resolution and read scopes. Implement the disk and embedded bundle inputs from Rules 3–4. Exercise static imports, nested object composition, payload indirection, examples, and permission-checked template results through the registered loader in Phase 2.
- [x] **YAML claims**
  - Own SimplifiedSchema `yaml_documents/{registry,select}.rs`. Add the pre-resolved definition constructor and unclaimed-lookup seam while preserving the existing public selection entry point. Prove built-in and local error precedence through that entry point.

- [x] **Verified snapshot**
  - Wire Phase 1 outputs in `config/registration/{bundle.rs,manifest.rs}` and `registration.rs`. Implement normalized paths, byte table, digest verification, one retry, and revision identity. Add deterministic read controls in the fixture loader to prove changed-manifest retry, unchanged-manifest refusal, and two releases never being combined. Keep refusal paths for missing inputs. Use focused nextest verification of the new source and loader tests at the Phase 2 checkpoint.

## Phase 2 — Passive host profiles

- [x] **Catalog preparation**
  - Implement Rule 6, shared by the verified disk loader and the embedded Claudine generator. Return the prepared catalog and exact dependency digest map from the same preparation pass. Verify multi-file and transitive hash coverage, path-independent identity, declaration and property spans, binding checks, and version 1 and stale artifact refusal.
  - Build v2 fixture constructors in `darkmatter/lib/tests/host_fixtures/mod.rs` and the DMLS L1 helpers. Give them explicit functions to re-address a fixture after legitimate member edits and to corrupt a single boundary without repairing it. Keep representative function catalog and facts pairs. Retain format 1 fixtures only for upgrade diagnostics and legacy-discovery absence tests.

- [x] **Host selection**
  - Classify each verified files member, prepare every trigger and YAML definition, and build `RegisteredHosts` with matched trigger indices and registered-kind claims. Use the shared resolver, namespace builder, and YAML claim constructor.
  - Migrate `DarkmatterSchemas::registered_selection_for` and `registered_matches`, composition host capture and schema validation, and DMLS `HostView` and `select_host` to the shared Markdown selection result. Remove legacy imports and probes from `config/registration/selection.rs`, `markdown/schemas/catalog.rs`, DMLS `overlay/{hosts,mod}.rs`, and provider test helpers.
  - Convert the previous legacy editor tests for global hover, navigation, and lifecycle typing to explicitly registered fixtures, including `host_globals.rs`, `lifecycle_host_profile.rs`, `lifecycle_event_views.rs`, and `edit_action_editor.rs`. Convert `pipeline_work_a3.rs` and `pipeline_work_support` fixtures to v2 so benchmarks still measure registered host analysis. Change negative legacy tests to assert that a root-local globals file supplies no host. Keep the existing function facts freshness and host-scope tests.

- [x] **Reader checkpoint**
  - Regenerate the SimplifiedSchema and Darkmatter shipped artifacts in dependency order using their area `just shipped-schemas` recipes. The root coordinator is used after the Claudine cutover in Phase 4. Run targeted nextest for trigger discovery, shipped envelopes, and v2 registration and selection against the new artifacts.
  - Prove refused registration keeps watches on the manifest, listed paths and missing parents. Cover reordered lists, moved copies, same-host disagreements, multiple hosts, literal mismatch, path qualifiers, malformed presence operands, shared-event-only documents, and grant-independent passive analysis.

### Input robustness matrix

The default for an invalid shape is a located error or warning at the public boundary, zero host activation from the refused input, zero execution, and zero publication. A build refusal emits no valid embed table. Reuse controls from the real Claudine v2 inventory once Phase 4 lands. Keep the small fixture hosts for two-host and conflicting-definition cases. Apply one edit per cell.

| Validator or loader | Public assertion and exceptions |
| --- | --- |
| Manifest scalars and member paths | Through `load_entries`, walk missing, null, wrong whole type, and empty for string and path validators, plus unsupported versions, string and floating-point versions, bad path syntax, escape, and case collisions. The format 1 exception is the dedicated upgrade warning. |
| Member lists and function pairs | Walk missing, null, wrong container, mixed and entirely invalid elements, and explicit empty. `files: []` and `functions: []` are legal empty inventories for those categories. In each function object prove catalog and facts are required, and extra keys fail. Cover repeated paths across categories as well as within `files`. |
| Digest table and values | Walk missing/null/wrong map, empty map, mixed and entirely invalid values, absent and excess members, wrong type, empty, length, uppercase, and nonhex values. The mandatory lifecycle member always requires a digest. Exercise wrong content separately from malformed digest text. |
| Member envelopes | Cover missing/null/wrong/empty mapping payload and types shapes once per shared validator, then a positive control and one invalid use for each wired field. Empty namespace payload and empty local types are valid. Missing optional type table, labels, and descriptions are valid. Explicit null fails. A missing trigger payload is a matcher-only trigger. Validate label pattern and both duplicate spans, namespace identifier and reserved/default global collisions, missing/wrong member kind, library `$schema`, unknown keys, and broad trigger branches. |
| Lifecycle JSON | Start from the generated artifact and edit version, host, hash, event/binding containers, and bound namespace names. Walk shape cases once through the strict shared required value, object, string, and collection validators, including omitted vs null for optional event metadata under the existing artifact rules. Preserve and test existing valid empty collections. Verify stale transitive hashes through a content-addressed manifest rebuilt over the edited bytes. |
| YAML readers | Test duplicate keys including nested tables, trailing garbage, second document, and bare YAML null. Run this once per distinct production loader used by the manifest, member envelope, and kind evaluation; cover normal comments and quoted values. Reuse v2 member preparation tests for any loader shared with generation. |
| Strict JSON readers | Exercise duplicate keys, trailing garbage and ordinary JSON whitespace through lifecycle and function facts. Reuse one loader matrix where `artifact.rs::parse_strict` is shared. Keep a per-artifact valid control and one bad integration case. |
| Config mutation | Through setup and config APIs, prove an omitted or empty register list receives the new entry. Preserve the existing reader's explicit-null-as-no-entries behavior for user config and assert it separately. Wrong-type configuration and invalid items leave configuration bytes alone. Distinguish a valid permissions conflict, which retains the inline/out-of-scope block and still migrates the registration after a successful publish. |

At the final checkpoint audit new parse paths for load-bearing `#[serde(default)]`, `Option<T>` conflating null and absence, lossy `filter_map`, `unwrap_or_default()`, and `.ok()`. Exercise the CLI through `CliProcessFixture` using one positive and one located refusal per distinct user-facing loader. Assert zero launches on refusals. Review findings on any shape must name the other states in that validator's row and column.

## Phase 3 — Shared validation

### Wave 3

- [x] **Trigger layer**
  - Implement Rule 8 in `config/registration/merge.rs` and `markdown/schemas/mod.rs`. Carry winning provenance through `EffectiveSchema` and keep the static cache separate from pending slot resolution. Wire compose, clean, validate, detailed trigger reporting, and DMLS schema diagnostics through the same layer. For a failed layer retain syntax diagnostics and omit dependent schema features.
  - Add public effective-schema regressions for equivalent declarations, enum/string narrowing, required vs optional, effective nullability, descriptions/examples/constraints/defaults, recursive aliases, nested rules, and a narrower third type that must still expose an incompatible pair. Cover unmatched conflicts, same-file suppression, arm order, inherited keys, a competing alternative key, second root union, nondefault root constraints, proven local override, passive unresolved templates, and conflict survival under an authored override. Assert the conflict code and source relationships in CLI and DMLS.
- [x] **YAML tier**
  - Own `markdown/schemas/yaml_documents/`, the YAML branch of `cli/src/commands/schema/validate.rs`, and DMLS YAML preparation/key changes in `overlay/yaml_contract.rs`. Implement Rule 9 using the selected definition's host identity. Use the loaded host view for DMLS contract preparation even when no Markdown trigger work is outstanding. Leave debounce and router invalidation to Phase 5.
  - Test identical copies, distinct definitions within and across hosts, built-in and local winners, unreadable roots, an incomplete local claim, registered precedence over a root schema, no kind claim falling through to a root schema, and registered YAML navigation/diagnostics from the source path. Prove only whole-document validation takes place.

- [x] **Consumer checkpoint**
  - Run Darkmatter CLI, library, and DMLS L1 filters for host registration, schema triggers, YAML validation, and registered-view function/lifecycle behavior. Keep the v2 fixtures isolated from workspace and user roots. Exercise the CLI error in `commands/schema/validate.rs::FileOutcome::schema_error` and DMLS related locations in `diagnostics/frontmatter.rs::schema_prepare_diagnostic`; reuse `diagnostics/yaml_document.rs::selection_diagnostic` for registered kind collisions.

## Phase 4 — Claudine bundle and publication

- [x] **Source migration**
  - Apply Rules 10–11. Preserve authoritative lifecycle, provider, linking, and sequence types. Check exact content-policy copy and require every resulting import to resolve from the inventory or supported built-ins. Update live references in root and package schema sources, including `simplified-schema/lib/tests/l1/shipped_corpus.rs`, `darkmatter/lib/tests/l1/{shipped_schema_corpus,base_schema_end_to_end}.rs`, `darkmatter/cli/tests/l1/{claudine_schema_bundle,schema_validate}.rs`, `darkmatter/dmls/tests/l1/claudine_schema_discovery.rs`, `dmls/src/providers/frontmatter/sequence_tests.rs`, Claudine’s `config/schema.rs::CONFIG_SCHEMA_SOURCE` and `composition/lifecycle/mod.rs::lifecycle_artifact`, and Claudine’s `sequence_schema`, `claudine_identification`, `host_catalog`, `shipped_artifacts`, and `cli/src/completion/schema_completion/tests.rs` fixtures. Frozen historical inputs retain their original paths and bytes.
  - Add real inventory tests for plain shared events, prompt, sections, allowed null counterparts, both values, an explicit inline kind with neither value, formal YAML, sequence presence, wrong sequence types, and literal mismatch. Assert a matching sequence layer cannot weaken the inline union. Assert setup/config still consumes the canonical config definition.
- [x] **Embedded inventory**
  - Implement Rule 12. Seed the moved lifecycle artifact at its new generated path with the previous bytes so the first build can compile the generator. Generate replacement bytes only from fresh inputs. Keep build-time regeneration independent of installed configuration. Verify that rebuild input tracking covers additions, deletions, moves, edits, and nested files and that staged artifact generation refuses a stale derived member before replacing any output.
  - Run root `just shipped-schemas`, then verify the generated outputs with the existing artifact checks. Repoint or retire `claudine/lib/examples/lifecycle_artifact.rs` and the old recipe by making them delegate to the shared generator and root recipe. Check inline, direct, and sequence preflight against the embedded inventory with an empty fake home and working directory outside the repository.

- [x] **Fixed publication**
  - Own `composition/registration.rs`, `config/atomic.rs`, and `host_publication.rs`. Apply Rule 13. Export the manifest path, inventory members, published byte views, and publisher outcome for the CLI and future repair. Verify interruption with barriers at a member replacement and before manifest commit, two cooperating publishers, reader outcomes, cleanup, the Windows persist retry, and a timestamp-identical no-op. Test symlinks to out-of-tree sentinels and native Windows reparse classification without requiring privileged symlink creation for the Windows unit test.
### Wave 4

- [x] **Registration command**
  - Own the Claudine CLI routes and install recipe. Use Rule 14 and the library API specified in Rule 13. Keep machine output free of warnings/log lines on stdout. Replace `host_registration_refresh.rs` with hermetic listing, detail, publish, and startup-passivity tests. Prove no bundle access during ordinary commands using a valid Claudine config plus an obstructed installation path and observable read/write instrumentation, alongside checks for missing and stale member bytes. Exercise the install command with an isolated Cargo root and home.
- [x] **Config migration**
  - Own Darkmatter `config/init.rs`, Claudine `config/setup.rs`, and setup CLI reporting. Use Rule 15 and the shared publisher. Cover setup on empty and preexisting homes, equivalent legacy paths with a missing old file, duplicates, conflicts, interrupted config writes, malformed settings, preserved comments and unrelated references, obsolete repository entries, and refused legacy cleanup. Keep one CLI test for `md init` preserving a permissions reference and the registration list.

- [x] **Publisher checkpoint**
  - Run Claudine and Darkmatter nextest filters for the embedded corpus, shipped artifacts, registration publication, setup, and installed CLI agreement. Check the JSON inventory against the exact published bytes, including the manifest and generated lifecycle artifact. Compare passive Darkmatter/DMLS results from the installed bundle with in-process Claudine validation on representative documents, using temp roots and existing command fixtures. Record the intentional sections validation/runtime boundary and the fixture migrations in the implementation log.

## Phase 5 — DMLS reload scheduling

- [x] **Measure publication**
  - Run one Linux sample with the Phase 4 binary in a temporary home. Seed a valid publication of the same inventory with edited member bytes and a corresponding valid earlier manifest, then time the real changed publish with a monotonic clock, from immediately before the first member write through successful manifest replacement. Use five warmed changed publishes with the home reseeded outside the timed interval for each. Verify changed publication and full v2 load on every trial. Save sample values and the resulting window in the implementation log.
  - Let the measured duration be the slowest valid sample in milliseconds. At or below 150 ms use 300 ms. Above 150 ms set the window to the next multiple of 50 ms at or above twice the measured duration. An incomplete or invalid sample retains the initial 300 ms and records the actual failure; the implementation continues through deterministic verification. Run the sample once, with no cross-OS benchmark or later timing recheck.

- [x] **Queue host reloads**
  - Implement Rule 16 using the measured constant and existing configuration keys. Coalesce changed, created, and deleted registration inputs across batches; reset the specific key's deadline on each relevant event, including directory and missing-target events. Keep the old installed view while dirty, and publish diagnostics on the latest completed view. Preserve independent indexing/overlay updates and the save fallback. Queue from the watch index on the router thread. Read and hash files on the worker.
  - Invalidate only views and open documents tied to the changed configuration, and update both YAML and Markdown requests. Preserve explicit configuration/schema reload behavior for non-registration changes. When validation fails, drop the refused host and republish the located diagnostic. Recovery clears that diagnostic, refreshes watches, and restores analysis without reopening the document.

- [x] **Reload checkpoint**
  - Use a controlled clock and worker completion queue in `overlay/hosts.rs` and the L1 registration-watch fixtures. Assert two batches yield one load after quiet, a newer event supersedes an in-flight result, an unchanged view changes no analysis generation, watch coverage follows a moved or missing target, and repair clears stale diagnostics. Include Markdown and YAML fixtures open across the same lifecycle, an unrelated config and a normal edit that remain immediate, and a client using save fallback. Run the DMLS L1 watch, YAML, and registered-host suites.

## Phase 6 — Documentation and end-to-end proof

### Wave 5

- [ ] **Darkmatter documentation**
  - Update `darkmatter/docs/topics/host-registration.md`, `docs/topics/lifecycle/host-artifact.md`, `dmls/docs/{diagnostics.md,editors/README.md}`, `darkmatter/README.md`, the Darkmatter repo skill and the canonical `use-darkmatter` and `use-dmls` skills. Show the v2 envelope, YAML selection, checked merges, passive capabilities, and editor reload flow. Include the v1 repair instruction and a compact example of a located conflict. Run Darkmatter's `just publish-skills`.
- [ ] **Claudine documentation**
  - Update `claudine/docs/topics/{schemas,configuration}.md`, `claudine/schemas/README.md`, the `claudine/docs/cli/` maintainer reference and index, `claudine/README.md`, and the Claudine repo and `use-claudine` skills. Explain the authored and published layouts, inventory JSON, setup order, recovery, supported overrides, and the scope of inline sections validation. Run Claudine's `just publish-skills`.
- [ ] **Schema documentation**
  - Update `simplified-schema/docs/topics/language/schema-documents.md`, its README and repo skill, and the monorepo kind catalog. Show namespace payload and private types, per-root label uniqueness, and path-based imports. Document any build dependency additions in the relevant dependency inventories.

- [ ] **Final verification**
  - Run the full scoped L1 suites with area recipes for SimplifiedSchema and Darkmatter plus Claudine's library and CLI recipes. Run the areas' lint checks and scoped examples/bench compilation for migrated fixtures. Use `just shipped-schemas` and verify a second deterministic generation leaves the artifacts byte-identical. Verify Markdown and YAML analysis against the published binary bundle from a temporary home with no repo or user schema roots.
  - Use the OS skill's configured non-interactive build hosts and `just cross-check` for affected package evidence on native Windows and WSL2, plus macOS evidence on the available macOS validation lane. Exercise the native Windows replacement/reparse and copied-root paths and keep platform cases in L1. Record the actual platform results and any unavailable lane in the implementation log without creating a new CI gate. Use nextest, the Test Toolkit, and existing CLI fixtures. No real terminal, browser, or focus changes are required.
  - Search live sources, documentation, benchmark fixtures, and generated provenance for deleted Claudine paths, `LegacyHost`, old lifecycle APIs, and startup refresh. Retain deliberate migration/refusal and frozen fixture references. Validate the shipped source-input lists in the affected Cargo manifests and the artifact coordinator’s subsystem tests. Check every acceptance item from the spec and the matrix above and record command results, the timing sample, any implementation departures, and the documented sections boundary for the review after this phase. Leave source changes uncommitted in the starting worktree.