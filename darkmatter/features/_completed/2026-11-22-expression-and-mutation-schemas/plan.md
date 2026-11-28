---
kind: plan
total-phases: 5
created: 2026-11-22
status: planned
phase: 6
agent: codex/gpt-6.2-sol
yolo: true
source_files_during_phase_1:
- simplified-schema/schemas/expression-schema.yaml
- simplified-schema/schemas/mutation-schema.yaml
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/schema/export.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/shipped/registry.rs
- simplified-schema/lib/src/simplified/standalone.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/src/yaml_documents/load.rs
- simplified-schema/lib/src/yaml_documents/marks.rs
- simplified-schema/lib/src/yaml_documents/mod.rs
- simplified-schema/lib/src/yaml_documents/semantic.rs
- simplified-schema/lib/generated/shipped.json
- simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/shipped_artifacts.rs
- darkmatter/lib/src/markdown/schemas/schema_files.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/yaml_kind_registry.rs
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/manifest.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/globals.lifecycle.json
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/
- claudine/lib/generated/shipped.json
docs_updated_during_phase_1:
- simplified-schema/docs/topics/language/schema-documents.md
- simplified-schema/docs/topics/language/unions.md
- simplified-schema/docs/topics/language/index.md
- simplified-schema/docs/topics/diagnostics.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/simplified-schema/schema-documents.md
- .claude/skills/darkmatter/SKILL.md
packages:
- simplified-schema
- darkmatter
- darkmatter-cli
- claudine
- darkmatter-lsp
- claudine-cli
source_files_during_phase_2:
- darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/checks.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/expression/host/mod.rs
- darkmatter/lib/src/effects/types.rs
- darkmatter/lib/src/lifecycle/signatures.rs
- darkmatter/lib/src/markdown/schemas/catalog.rs
- darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
- darkmatter/lib/src/markdown/schemas/embedded.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/shipped.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
- darkmatter/schemas/partials/functions.yaml
- darkmatter/schemas/partials/mutations.yaml
- darkmatter/schemas/partials/context-variables.yaml
- darkmatter/lib/generated/shipped.json
- claudine/lib/generated/shipped.json
- darkmatter/lib/tests/fixtures/host_functions/project.yaml
- darkmatter/lib/tests/fixtures/host_functions/project.facts.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/darkmatter/schemas/partials/context-variables.yaml
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-context-variables.json
- darkmatter/lib/tests/host_fixtures/mod.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/callable_catalog_validation.rs
- darkmatter/lib/tests/l1/base_schema_end_to_end.rs
- darkmatter/lib/tests/l1/collection_analysis.rs
- darkmatter/lib/tests/l1/global_catalog.rs
- darkmatter/lib/tests/l1/host_functions.rs
- darkmatter/lib/tests/l1/member_check.rs
- darkmatter/lib/tests/l1/recent_commits_shared_type.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/schemas/output/schema-validate.yaml
- darkmatter/cli/tests/l1/schema_validate_yaml.rs
- darkmatter/dmls/src/overlay/yaml_contract.rs
- darkmatter/dmls/src/overlay/yaml_document.rs
- darkmatter/dmls/src/overlay/schema.rs
- darkmatter/dmls/src/overlay/suggestions.rs
- darkmatter/dmls/src/overlay/registry.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/diagnostics/yaml_document.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/yaml_document_catalog_editor.rs
- darkmatter/dmls/tests/l1/yaml_document_editor.rs
- darkmatter/dmls/tests/l1/yaml_document_session.rs
- darkmatter/dmls/tests/l1/host_function_diagnostics.rs
- simplified-schema/lib/src/yaml_documents/load.rs
- simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs
- simplified-schema/lib/tests/l1/shipped_corpus.rs
docs_updated_during_phase_2:
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/docs/topics/host-functions.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/dmls/docs/diagnostics.md
- simplified-schema/docs/topics/language/objects-and-named-types.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/compose.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
- darkmatter/schemas/darkmatter-host.yaml
- darkmatter/lib/src/config/registration.rs
- darkmatter/lib/src/config/registration/bundle.rs
- darkmatter/lib/src/config/registration/locate.rs
- darkmatter/lib/src/config/registration/manifest.rs
- darkmatter/lib/src/config/registration/selection.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
- darkmatter/lib/src/markdown/compose/expression/host/mod.rs
- darkmatter/lib/src/markdown/compose/pipeline/phases.rs
- darkmatter/lib/src/markdown/schemas/catalog.rs
- darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/tests/host_fixtures/mod.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/host_functions.rs
- darkmatter/lib/tests/l1/host_namespaces.rs
- darkmatter/lib/tests/l1/host_registration.rs
- darkmatter/lib/tests/l1/host_registration_companions.rs
- darkmatter/lib/tests/l1/host_registration_locations.rs
- darkmatter/lib/tests/l1/host_registration_snapshot.rs
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/claudine_schema_bundle.rs
- darkmatter/cli/tests/l1/compose_foreign_documents.rs
- darkmatter/cli/tests/l1/compose_host_scope.rs
- darkmatter/cli/tests/l1/host_registration.rs
- darkmatter/cli/tests/l1/host_runtime_scope.rs
- darkmatter/dmls/benches/pipeline_work_a3.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/lib.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/src/providers/lifecycle_host_tests.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/claudine_host.rs
- darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
- darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
- darkmatter/dmls/tests/l1/host_catalog_reload.rs
- darkmatter/dmls/tests/l1/host_function_diagnostics.rs
- darkmatter/dmls/tests/l1/lifecycle_host_profile.rs
- darkmatter/dmls/tests/l1/registered_host_editor_agreement.rs
- simplified-schema/lib/src/member_envelope.rs
- simplified-schema/lib/tests/l1/member_envelope.rs
- claudine/lib/src/composition/registration/published.rs
- claudine/lib/generated/shipped.json
- claudine/lib/tests/l1/host_catalog.rs
- claudine/lib/tests/l1/host_publication.rs
- claudine/lib/tests/l1/installed_registration.rs
- claudine/lib/tests/l1/registration_inventory.rs
docs_updated_during_phase_3:
- darkmatter/docs/topics/host-registration.md
- darkmatter/docs/topics/host-functions.md
- claudine/docs/topics/schemas.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/compose.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/expressions.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
- claudine/lib/src/composition/registration.rs
- claudine/lib/src/composition/registration/inspect.rs
- claudine/lib/src/composition/registration/published.rs
- claudine/lib/src/doctor/registration.rs
- claudine/lib/src/doctor/registration_tests.rs
- claudine/lib/tests/l1/host_catalog.rs
- claudine/lib/tests/l1/host_publication.rs
- claudine/lib/tests/l1/installed_registration.rs
- claudine/cli/src/commands/registrations.rs
- claudine/cli/tests/l1/doctor.rs
docs_updated_during_phase_4:
- claudine/docs/cli/doctor.md
- claudine/docs/cli/registrations.md
- claudine/docs/topics/configuration.md
- claudine/docs/topics/schemas.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/claudine/SKILL.md
source_files_during_phase_5:
- claudine/lib/src/composition/registration/published.rs
- claudine/lib/src/composition/schema/shipped.rs
- schemas/kind.yaml
docs_updated_during_phase_5:
- claudine/README.md
- claudine/docs/topics/schemas.md
- claudine/docs/use-skills/use-claudine/SKILL.md
- claudine/docs/use-skills/use-claudine/cli.md
- claudine/docs/use-skills/use-claudine/config.md
- claudine/docs/use-skills/use-claudine/library.md
- darkmatter/README.md
- darkmatter/docs/topics/expression-engine.md
- darkmatter/docs/topics/host-functions.md
- darkmatter/docs/topics/host-registration.md
- darkmatter/docs/topics/lifecycle/host-artifact.md
- darkmatter/docs/use-skills/use-darkmatter/authoring.md
- darkmatter/docs/use-skills/use-darkmatter/cli.md
- darkmatter/docs/use-skills/use-dmls/SKILL.md
- darkmatter/docs/use-skills/use-dmls/schemas.md
- docs/kind-documents.md
- simplified-schema/README.md
- simplified-schema/docs/topics/language/schema-documents.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/claudine/SKILL.md
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/simplified-schema/SKILL.md
- claudine/docs/use-skills/use-claudine/SKILL.md
- claudine/docs/use-skills/use-claudine/cli.md
- claudine/docs/use-skills/use-claudine/config.md
- claudine/docs/use-skills/use-claudine/library.md
- darkmatter/docs/use-skills/use-darkmatter/authoring.md
- darkmatter/docs/use-skills/use-darkmatter/cli.md
- darkmatter/docs/use-skills/use-dmls/SKILL.md
- darkmatter/docs/use-skills/use-dmls/schemas.md
source-code:
- claudine/cli/src/commands/registrations.rs
- claudine/cli/tests/l1/doctor.rs
- claudine/lib/generated/shipped.json
- claudine/lib/src/composition/registration.rs
- claudine/lib/src/composition/registration/inspect.rs
- claudine/lib/src/composition/registration/published.rs
- claudine/lib/src/composition/schema/shipped.rs
- claudine/lib/src/doctor/registration.rs
- claudine/lib/src/doctor/registration_tests.rs
- claudine/lib/tests/l1/host_catalog.rs
- claudine/lib/tests/l1/host_publication.rs
- claudine/lib/tests/l1/installed_registration.rs
- claudine/lib/tests/l1/registration_inventory.rs
- darkmatter/cli/schemas/output/schema-validate.yaml
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/tests/l1/claudine_schema_bundle.rs
- darkmatter/cli/tests/l1/compose_foreign_documents.rs
- darkmatter/cli/tests/l1/compose_host_scope.rs
- darkmatter/cli/tests/l1/host_registration.rs
- darkmatter/cli/tests/l1/host_runtime_scope.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/schema_validate_yaml.rs
- darkmatter/dmls/benches/pipeline_work_a3.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/yaml_document.rs
- darkmatter/dmls/src/lib.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/registry.rs
- darkmatter/dmls/src/overlay/schema.rs
- darkmatter/dmls/src/overlay/suggestions.rs
- darkmatter/dmls/src/overlay/yaml_contract.rs
- darkmatter/dmls/src/overlay/yaml_document.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/src/providers/lifecycle_host_tests.rs
- darkmatter/dmls/tests/l1/claudine_host.rs
- darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
- darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
- darkmatter/dmls/tests/l1/host_catalog_reload.rs
- darkmatter/dmls/tests/l1/host_function_diagnostics.rs
- darkmatter/dmls/tests/l1/lifecycle_host_profile.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/registered_host_editor_agreement.rs
- darkmatter/dmls/tests/l1/yaml_document_catalog_editor.rs
- darkmatter/dmls/tests/l1/yaml_document_editor.rs
- darkmatter/dmls/tests/l1/yaml_document_session.rs
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/src/config/registration.rs
- darkmatter/lib/src/config/registration/bundle.rs
- darkmatter/lib/src/config/registration/locate.rs
- darkmatter/lib/src/config/registration/manifest.rs
- darkmatter/lib/src/config/registration/selection.rs
- darkmatter/lib/src/effects/types.rs
- darkmatter/lib/src/lifecycle/signatures.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/checks.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
- darkmatter/lib/src/markdown/compose/expression/host/mod.rs
- darkmatter/lib/src/markdown/compose/pipeline/phases.rs
- darkmatter/lib/src/markdown/schemas/catalog.rs
- darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
- darkmatter/lib/src/markdown/schemas/embedded.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/schema_files.rs
- darkmatter/lib/src/markdown/schemas/shipped.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
- darkmatter/lib/tests/fixtures/host_functions/project.facts.json
- darkmatter/lib/tests/fixtures/host_functions/project.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/globals.lifecycle.json
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/manifest.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/action.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/claudine-config.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/composition.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/content-policy.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/document.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/formal-sequence-file.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/group.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/inline-composition.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/lifecycle-events.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/linking.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/loop.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/seq.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/sequence-document.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/sequence.yaml
- darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter/schemas/shared.yaml
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-context-variables.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/darkmatter/schemas/partials/context-variables.yaml
- darkmatter/lib/tests/host_fixtures/mod.rs
- darkmatter/lib/tests/l1/base_schema_end_to_end.rs
- darkmatter/lib/tests/l1/callable_catalog_validation.rs
- darkmatter/lib/tests/l1/collection_analysis.rs
- darkmatter/lib/tests/l1/global_catalog.rs
- darkmatter/lib/tests/l1/host_functions.rs
- darkmatter/lib/tests/l1/host_namespaces.rs
- darkmatter/lib/tests/l1/host_registration.rs
- darkmatter/lib/tests/l1/host_registration_companions.rs
- darkmatter/lib/tests/l1/host_registration_locations.rs
- darkmatter/lib/tests/l1/host_registration_snapshot.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/member_check.rs
- darkmatter/lib/tests/l1/recent_commits_shared_type.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- darkmatter/lib/tests/l1/yaml_kind_registry.rs
- darkmatter/schemas/darkmatter-host.yaml
- darkmatter/schemas/partials/context-variables.yaml
- darkmatter/schemas/partials/functions.yaml
- darkmatter/schemas/partials/mutations.yaml
- schemas/kind.yaml
- simplified-schema/lib/generated/shipped.json
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/member_envelope.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/schema/export.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/shipped/registry.rs
- simplified-schema/lib/src/simplified/standalone.rs
- simplified-schema/lib/src/triggers/finding.rs
- simplified-schema/lib/src/yaml_documents/load.rs
- simplified-schema/lib/src/yaml_documents/marks.rs
- simplified-schema/lib/src/yaml_documents/mod.rs
- simplified-schema/lib/src/yaml_documents/semantic.rs
- simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/member_envelope.rs
- simplified-schema/lib/tests/l1/shipped_artifacts.rs
- simplified-schema/lib/tests/l1/shipped_corpus.rs
- simplified-schema/schemas/expression-schema.yaml
- simplified-schema/schemas/mutation-schema.yaml
documentation:
- claudine/README.md
- claudine/docs/cli/doctor.md
- claudine/docs/cli/registrations.md
- claudine/docs/topics/configuration.md
- claudine/docs/topics/schemas.md
- claudine/docs/use-skills/use-claudine/SKILL.md
- claudine/docs/use-skills/use-claudine/cli.md
- claudine/docs/use-skills/use-claudine/config.md
- claudine/docs/use-skills/use-claudine/library.md
- darkmatter/README.md
- darkmatter/dmls/docs/diagnostics.md
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/docs/topics/expression-engine.md
- darkmatter/docs/topics/host-functions.md
- darkmatter/docs/topics/host-registration.md
- darkmatter/docs/topics/lifecycle/host-artifact.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- darkmatter/docs/use-skills/use-darkmatter/authoring.md
- darkmatter/docs/use-skills/use-darkmatter/cli.md
- darkmatter/docs/use-skills/use-dmls/SKILL.md
- darkmatter/docs/use-skills/use-dmls/schemas.md
- docs/kind-documents.md
- simplified-schema/README.md
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/language/index.md
- simplified-schema/docs/topics/language/objects-and-named-types.md
- simplified-schema/docs/topics/language/schema-documents.md
- simplified-schema/docs/topics/language/unions.md
completed-phase: 5
implemented: true
---

# Expression and mutation schemas implementation plan

## Summary and Definition of Done

Ship two callable catalog kinds and use the expression kind to discover registered host expressions from manifest v3. Keep the built-in expression and mutation catalogs as sources for signatures, documentation, and lifecycle operand generation. A single passive reader checks both kinds in Darkmatter, including when called by the CLI, DMLS, and registration. Registered expressions get their binding facts from derived sibling files tied to the exact catalog bytes. Runtime handlers remain owned by the host process.

Done means the same authored catalog produces the same located findings through the CLI and editor. A manifest inventory accounts for every catalog, derived companion, and lifecycle artifact. A bad member or conflicting name refuses the whole host and leaves enough watch information for repair to restore it. A good registration supplies analysis only to documents selected for that host. Setup upgrades the fixed installation in place, and Claudine listing, publication, and doctor agree on the files and their exact bytes. Built-in signatures, runtime behavior, and the generated lifecycle data contract pass their existing checks. Generated artifacts, docs, and published caller skills describe the implementation.

Start from the completed host registration v2 and Claudine doctor implementations. The planning checkout still has the v1 reader in `darkmatter/lib/src/config/registration.rs` and the revision-directory publisher in `claudine/lib/src/composition/registration.rs`. The prerequisite plans establish the fixed bundle, verified-byte reader, editor reload worker, inventory, and repair entry points used below. Run phases in order. Tasks within a numbered wave may run concurrently and have separate file ownership. Every delegated brief must state that the entire session tree is non-interactive. Keep work in the session worktree and finish all phases before review.

## Phase 1 — Define and admit callable catalogs

### Necessary Rules

1. **Keep the artifact boundaries from the prerequisites.** Require exactly seven manifest v3 fields (`kind`, `host`, `version`, `format_version`, `files`, `lifecycle`, and `digests`). Remove `functions` from the schema, decoder, encoder, normalized meaning, and sample manifests. Keep `files: []` as a valid empty collection and the lifecycle artifact required. Keep the v2 fixed path `~/.claudine/registrations/darkmatter/manifest.yaml`, writer lock, manifest-last protocol, and lifecycle artifact format and namespace hash. The v2 spec's manifest and publication sections and the doctor plan's inventory and repair rules establish those boundaries. Check integer version before the closed v3 shape and report `unsupported host registration format version <n>; update Darkmatter and the host tool, then republish the registration` at `format_version`. Apply it to v1, v2, and future integer versions, including repository entries. Never decode a legacy manifest as a loadable profile. Retain the prior v2 manifest decoder exclusively for its guarded managed-path set during publication cleanup. Entries deduced from a malformed older manifest confer no cleanup authority. Setup and doctor still publish through the same v3 writer.

2. **Use Simplified Schema's standalone admission for document shape.** Add `StandaloneSchemaEnvelope::{ExpressionSchema, MutationSchema}` in `simplified-schema/lib/src/simplified/standalone.rs`. Recognize both kinds, expose their labels and allowed keys, and parse their `types` through the existing source-map path. Add `simplified-schema/schemas/{expression-schema,mutation-schema}.yaml` and include both in `envelope.rs::ENVELOPE_DEFINITIONS`, `EnvelopeDefinitions`, and `shipped/registry.rs`'s envelope records. Both kinds have closed roots. Use strict authored shape admission for these two kinds, with a nonempty mapping of named types and nonempty list of category strings. Allow an omitted description and an authored empty string. Reject explicit null and scalar coercion for metadata. Preserve the category parser's case-sensitive, trimmed labels and optional nonblank descriptions after `->`; use the same rule for both kinds. A bare null in `types` is an error. A quoted `"null"` is valid helper type syntax. This follows the specification's closed mapping and the distinction already made by `catalog/mod.rs::null_type_entry`. Keep the existing soft admission policy for other envelopes. Extend Simplified Schema's `yaml_documents/semantic.rs` with syntax-only checks for these kinds, using the already selected built-in definitions without general named-type expansion.

3. **Keep catalogs private to callable tooling.** Set `exports_schema = false` by construction. In `simplified-schema/lib/src/resolve.rs::parse_referenced_schema_document_with_mode`, use the recognition guard that currently backs `claims_yaml_schema` to reject both callable kinds even when their declarations are malformed. Give them a distinct `NotSchemaReason::CallableCatalog` with wording that the file describes callable signatures and cannot be used as a document schema or named-type import. Cover the disk and embedded paths that call the reference reader, including trust-boundary reads in the v2 resolver. Guard `check_standalone_named_types` and Darkmatter's `markdown/schemas/schema_files.rs` against routing catalogs into generic import or template preparation. Keep normal function-typed properties, named helper types and categories in ordinary `kind: schema` files. `StandaloneSchemaDocument::exports_schema` distinguishes a types table from an export, but the resolver also accepts types-only named-type libraries, so the kind gate is required. A root-discovered catalog is authorable YAML and never an implicit registration, trigger, kind definition, or import library.

4. **Centralize callable semantics and preserve catalog scope.** Keep the reader in `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs`. Use `load_catalog(source: &str, path: &Path, expected: StandaloneSchemaEnvelope, example_files: &[(&str, &str)]) -> Result<FunctionCatalog, CatalogError>` and the same arguments followed by `PreparationMode` for `load_catalog_with_mode`. Only the two callable envelope variants are valid expected kinds. Keep `FunctionCatalog`, `HostFunctionCatalog::load`, and `HostFunctions::builder` as the existing types and host-facing constructor names. Use the admitted YAML value for null detection and authored example spellings. Preserve descriptor order and overload spelling, and require at least one callable. At least one overload of each callable must name a category. Other overloads inherit it and every explicit category must agree. This preserves `load_catalog_with_mode`'s category reduction. Require a direct function or a union of direct function atoms for a callable. A union mixing a direct function with another type is invalid. Ordinary helper definitions stay private and may be used in parameter and return types. Reuse `FunctionCatalog::type_resolver` (`@this` and existing unqualified local spelling), definition traversal, overload checks, and declaration narrowing compilation. Resolve local names in the catalog and reject external references and unresolved template slots without resolving files or evaluating them. Retain the existing callable translation limits. Validate helper references in authored declaration order, including unused helpers, without expanding imports. Allow grammar-valid local recursion in helpers; retain the existing call-binding translator's recursion limits for referenced operands and returns. Compile host declarations through the existing eager registration builder for passive checks, using only local types and signature-backed facts. Built-in callback and lazy functions continue through the existing built-in registry configurations. For mutations reject `narrows(...)` on their callable signatures and nested function signatures. Keep inline examples as parsed data and admit file-form callable examples only from the pre-supplied built-in example table. The current `project_examples`, `compile_declared_fact`, `functions/mod.rs::build_registry`, and host builder distinguish these contexts.

5. **Keep authored names and explicit conflict locations.** Use ASCII lowercase for callable names and aliases, as `HostFunctions::lookup` and `host/mod.rs::validate_names` do. Reject duplicate folded callable entries within each callable catalog, including variants sharing a spelling after folding. A standalone host expression catalog also rejects canonical names matching any built-in canonical name or alias. The built-in expression reader admits its own names and projects built-in aliases. The mutation reader admits its own names without projecting expression aliases. Keep authored canonical spelling as the catalog and facts key; match runtime builder names using the existing exact canonical spelling. Retain alias checks in the runtime builder and facts rebuild. In registration aggregate canonical names and aliases over all expression members of one profile using a normalized-path-ordered pass. Reject all colliding owners with a related location for the earlier declaration. Two profiles may declare the same spelling. For YAML keys/categories use the existing marked YAML loader and source map. For identical duplicate YAML keys add a marked-key diagnostic pass that records the first authored key and the repeated key, at mapping scope, and runs on the duplicate-error path only. For overload and semantic conflicts retain `LocatedRelated` from the parser or attach the competing declaration key span. In a facts alias collision locate the alias entry and its prior alias or catalog key. Built-in collisions refer to the embedded declaration identity and span, with the alias spelling in the message. Existing `HostCatalogError::span/related` and `raw_type_entries` show where the reader currently loses this information.

6. **Project one judgment through CLI and editor.** Add `DarkmatterSchemas::validate_callable_catalog(&self, document: &LoadedYamlDocument) -> Result<(), SchemaError>` in `markdown/schemas/yaml_documents/mod.rs` and call it from `prepare_yaml_contract` for a loaded YAML callable catalog. Keep the kind contract for a valid document. Built-in file-form examples come from the existing embedded tables. Direct built-in preparation supplies those tables explicitly. For standalone validation, allow them for the built-in source identities `CATALOG_PATH` and `MUTATIONS_PATH`, including their exact repo-relative paths under a captured Rusty Biscuit checkout root. Use the captured context and Biscuit File path identity for that match. A basename alone never selects an embedded table. `HostFunctionCatalog::load` always supplies an empty table, including during registration. Carry catalog failures through a located `SchemaError::CallableCatalog { path, message, span, related }` with related file and byte ranges. Project them in `commands/schema/validate.rs` (human and JSON) and `dmls/src/diagnostics/yaml_document.rs` with code `dm.schema.callable_catalog`, primary range from the same byte span, and secondary locations from the shared error. Preserve the common YAML load error path for malformed YAML, including duplicate diagnostics and both key locations. In registration project a failing catalog into `RegistrationDiagnostic` with code `dm.host.registration`, the originating member and location, and related file locations. For CLI YAML validation return directly after the catalog result, skipping `resolve_schema_file` and its slot approvals. In DMLS `overlay/yaml_contract.rs::declaration_part` key catalog preparation on full text, run it on the existing registry worker, and install the newest result. Teach `overlay/{schema,suggestions,registry,mod}.rs` the two kinds for local type authoring, excluding them from ordinary imports and root-schema reference facts. Suppress the standalone overlay's redundant error when the shared YAML result or loader already reports it. The current YAML worker keys generic kind contracts on `kind` and root `$schema` and the CLI follows successful validation with effectful resolution. Both need explicit routing.

7. **Derive companions within the verified bundle.** Keep `host/artifact.rs::facts_artifact_path` as the only naming rule. Strip the last extension from an admitted `.yaml` or `.yml` member and append `.facts.json`, preserving all earlier dots and the directory. Use v2 `manifest.rs` and `bundle.rs` to resolve and verify the YAML files and lifecycle, classify the verified YAML, then derive and verify expression companions before building a profile. Form the exact digest-key set from authored `files`, `lifecycle`, and the derived sibling spellings, preserving their authored relative prefix. Use the existing normalized member identity and case/no-follow checks across all three sources. A same-stem YAML/YML pair, duplicated resolved member, or companion listed in `files` refuses the bundle. Accept only the four v2 member kinds and `expression-schema`. Reject `mutation-schema` on its `kind` with the message `host-defined actions are not supported`. Register no partial profile. On missing companion, locate the manifest's `files[i]` entry and name the expected sibling. On present bad companion, point into its contents or to its root when only a file-level cause is known. Capture its attempted identity and missing parent in the watch inputs before a failing read. Reuse v2's one retry on missing member or digest mismatch when the manifest bytes changed, and retain watch candidates from both attempts. Include derived paths and digests in the normalized, order-independent and root-independent revision. Keep the existing cache owners `MdRequest::registered_hosts` and DMLS `HostRegistrations`; document selection reuses their installed profiles. The v2 snapshot proof, `manifest_member_candidates`, and `RegisteredHostProfile::functions` already separate watching, validated data, and per-document selection.

8. **Keep the facts wire and runtime contract.** Use `HostFunctions::analysis_only_at` and its strict parser/rebuilder when reading a verified companion from the bundle. Refuse the profile for every incomplete entry as v1 `load_members` already does. Keep the facts format version, required properties, byte hash, and authored narrowing as authority. Check every typed registration and preserve the current eager execution, pre-conversion source exclusions, and return checks in `HostFunctionsBuilder::build` and `functions/mod.rs::call_host`. Keep batch validation across registered members separate from runtime registration of one catalog. Store analysis sets with their catalog and facts paths in `RegisteredHostProfile`. Enter only the selected profile's sets in DMLS `overlay/hosts.rs` and `providers/dsl.rs` and in `cli/src/commands/compose.rs::run_compose`, including its passive reference and call checks. Enter the scope for the selected top-level composition request, keeping the current nested expansion context inherited within that request. The guards drop on return and are never installed globally by registration. `transclusion/engine.rs::directive_child_request` derives nested input from its parent, and `MdRequest` has a separate selection for each top-level document. Preserve the existing `md schema validate` data-only behavior for Markdown and its shared catalog-document check for YAML. Change the selected analysis-only runtime failure to `host function <name>() requires the host runtime`. Keep existing host expression evaluation in action operands through the expression dispatcher. Lifecycle action keys continue through the existing `lifecycle/signatures.rs`, registry and generated event artifact. The current dispatcher has distinct expression, built-in action, and host service paths. No catalog grants permissions, adds action keys, or changes event availability. `when` remains deferred across all three catalog kinds.

9. **Keep Claudine's real inventory and test the companion path with a fixture.** The current `claudine/registration/` contains no callable catalog, and `claudine/lib/tests/l1/host_catalog.rs::claudine_ships_no_host_function_catalog_and_no_facts_artifact` pins that behavior. Keep it and point it at the v3 inventory. Extend the inventory's typed kinds with `expression-schema`, `mutation-schema`, and `expression-facts` and keep v2's `manifest` and `lifecycle` listings. Recognize both YAML catalog kinds during source classification, then reject mutation publication through the same member rule as installed loading. Give the private inventory constructor an explicit table of generated companion bytes keyed by derived published path; production Claudine passes the empty table. Keep `claudine/registration/` YAML-only, and place generated companion bytes in that companion input alongside the existing generated lifecycle input. The constructor validates the embedded source and companion set through Darkmatter's shared verified bundle preparation, including exact companion enumeration, catalog hash, names, and facts. Use one test fixture with a bound Rust handler and generated facts to exercise these paths and the same publisher, listing rows, and doctor inspection and repair. The v2 inventory already provides the byte source for these consumers. Keep the production inventory limited to Claudine's current schemas and lifecycle artifact.

10. **Regenerate from source and measure no spike.** Bump the affected preparation revisions relative to the prerequisite tree and regenerate in dependency order. Keep the two built-in YAML files in place and use the current recipes. The exact generated `lifecycle.yaml` bytes and the descriptors' signatures, aliases, examples, and binding behavior are regression oracles. The spec asks for no latency threshold or new runtime measurement. Read-only traces exposed the open choices and the tests can verify the load-once invariant directly.

- [x] **Prerequisite baseline**
  - Own the baseline evidence entry in `implementation-log.md`. Use existing v2 and doctor fixtures to verify the fixed-path publisher, manifest loader, doctor repair, and DMLS reload pass focused L1. Record the starting Git tree and capture pre-change bytes of `darkmatter/schemas/partials/lifecycle.yaml`, the generated expression table, and `darkmatter/docs/functions/`. Record their paths and byte hashes in the implementation log so later sessions can compare against the baseline. Use the existing expression and registry parity tests as the descriptor and dispatch baseline. Read this evidence before regenerating anything.
  - Use version 2 as the migration fixture. Freeze the prerequisite publisher's emitted manifest and exact installed member bytes as the v2 test fixture. Use it only for version refusal, fixed-path migration, and preservation of unrelated config. The v2 completion is an entry dependency for the feature, that must already be present at the start of Phase 1.

- [x] **Envelope definitions**
  - Own Simplified Schema's `schemas/`, `lib/src/{envelope.rs,simplified/standalone.rs,shipped/registry.rs,yaml_documents/semantic.rs,yaml_documents/load.rs,yaml_documents/marks.rs,resolve.rs,errors.rs}` and associated tests. Implement Rules 2, 3, and the shared diagnostic representation from Rules 5 and 6. Add the new variants through all exhaustive matches and generated record admission. Keep the bootstrap independent of old shipped bytes.
  - Walk the YAML loader and common authored-shape matrix below through the public admission/validation result. Prove a normal types library still supports imports, and well-formed and malformed callable envelopes cannot be selected or imported. Update Simplified Schema's language index and unions topic to explain private callable type scope.

### Input Robustness Matrix

For the new and changed validators, every invalid shape yields a located error or registration refusal. It runs no callable, resolves no template or external example, and publishes no profile or bytes. Start each family from a successful real-tool fixture and change one field per cell. Assert the crate's public admission, catalog/YAML result, loaded registration, inventory result, or doctor rows. Exercise absent, explicit null, wrong whole-field type, and empty shape for each validator family; for typed arrays/maps also exercise one bad element and all bad elements. For another field using the same validator, cover a good control and one invalid shape.

- **Catalog envelope.** Description may be omitted or an empty string. `categories` and `types` are required and nonempty. A types table made entirely of helpers fails the Darkmatter callable check. Quoted null helper and a signature's optional explicit-null argument are valid, while `name:` as a types entry fails. Valid type carrier spellings include signature strings, sequence and pipe unions, and normal helper mappings; cover mixed and wholly invalid union arms independently from mixed callable/helper semantics. Inline examples are passive. Unknown fields, including `$schema` and `when`, fail. Test the category string validator once, including trim, missing label and description, duplicate, and wrong and empty entries. Prove missing category across all overloads, inherited category, undeclared category, and disagreement.
- **V3 manifest.** Only `files: []` represents an explicit empty member list, with the lifecycle's digest still required. Exercise `files` through the array validator and the per-path boundary checks. Check that absent `functions` succeeds and even an explicit empty `functions` fails as unknown. Exercise the hash map validator through `digests` with missing, extra, wrong-typed, null, malformed or uppercase digests and a valid digest for different bytes. Run a good control and one invalid shape per scalar field sharing a validator, and separate wrong-type and unsupported integer cases for `format_version`. Distinguish a missing catalog digest, missing companion digest, and a missing file with the expected digest still in the manifest. Cover collisions with a listed facts file, a same-stem catalog, and escaping or linked candidates using the prerequisite boundary fixtures.
- **Facts companion.** Preserve the existing `required`, `nullable`, `string`, `object`, `array`, and integer paths in `host/artifact.rs`. The required nullable `conversion_recovery` and `malformed_reference` fields accept explicit null for no rule, and absence fails. Empty `aliases`, `facts`, and `source_exclusions` mean explicit empty lists; an empty per-exclusion `kinds` list fails in `artifact.rs::rebuild`. Exercise one representative array and per-element validator fully, and wire-test the other uses. Empty `functions` fails for a nonempty declaration; empty function and rule maps fail required fields. Cover stray or missing function entries, stale catalog hash with a valid manifest digest, and a narrowing or source exclusion contradictory to the declaration. Keep tests for unsupported facts version and strict byte freshness.
- **Document loaders.** Cover duplicate keys, extra YAML documents, trailing garbage, malformed input and accepted YAML comments/BOM/CRLF/null spelling once per distinct loader or reuse its existing public tests and add wiring coverage. The shared Simplified Schema YAML admission handles catalogs. Use the v2 manifest loader's own tests for any distinct decode path. Facts uses `artifact.rs::parse_strict`; prove nested duplicate keys and trailing JSON fail via registered-host loading. JSON5 comments and trailing commas fail in facts JSON. Include an authored non-ASCII character before an error to verify source coordinates across projections.
- **CLI wiring.** Run a successful fixture and a malformed and semantic failure through `md schema validate`; check the location, related position where applicable, exit status, and zero side effects. Use one invalid manifest/companion case through `md compose` and exercise doctor inspection and repair from a valid tool-published installation. Keep the remaining cells in-process. Before closing, search changed readers for `#[serde(default)]`, ambiguous `Option<T>`, `filter_map` with `as_str()`, `unwrap_or_default()`, and `.ok()` on load-bearing fields. For any review finding give the other outcomes in its validator or loader row and column.

## Phase 2 — Share Darkmatter catalog validation

- [x] **Catalog reader**
  - Implement Rules 4–6 in `markdown/compose/expression/catalog/mod.rs`, `expression/host/mod.rs`, and `markdown/schemas/yaml_documents/mod.rs`. Reuse the parsed document and source-map ranges through descriptor projection and semantic checks. Move canonical host name validation to `HostFunctionCatalog::load` and keep the alias and completeness defenses at set construction. Resolve reserved spellings against `functions/mod.rs`'s existing binding rows, with the same canonical and alias comparisons as `is_builtin_name`. Reuse `registered_aliases` for built-in expression descriptor projection. Keep this name-only lookup independent of the joined registry, which calls back into the catalog loader.
  - Carry field and authored overload ranges out of parser and binding errors. For binding errors reported with overload indices, use the corresponding source-map arm and attach the first matching arm for duplicate/disagreeing declarations. For a missing field use the enclosing mapping. For file-form examples, reject unlisted references directly from the supplied table. No filesystem fallback.
  - Retag `darkmatter/schemas/partials/{functions,mutations}.yaml` and only fixtures used as callable catalogs. Update `expression_function_catalog_from_sources`, `effects/types.rs::effect_function_catalog_from_sources`, and `markdown/schemas/shipped.rs::{prepare,accept_function_catalog}` and the two call sites of `load_catalog_with_mode`. Maintain acceptance of valid current records and source fallback with Rust binding and lifecycle registry parity. Update `lifecycle/signatures.rs`'s loader fixture and leave the lifecycle generator's data-shape logic intact.
  - Own focused `catalog/tests.rs`, library `tests/l1/host_functions.rs` and `tests/l1/kind_semantic_checks.rs` cases for both catalog kinds and the catalog matrix. Update `darkmatter/lib/tests/fixtures/host_functions/project.yaml` and regenerate its companion from the existing test binding. Use that YAML as the shared CLI/DMLS control with literal `include_str!` paths and one authored edit per failing fixture. Update tests that currently expect a built-in collision only after construction to assert the earlier host catalog refusal. Keep ordinary function-language fixtures under `kind: schema`. Verify host-only declaration checks never enter a handler, and built-in callback and lazy signatures still compile using their registered evaluation modes.
  - Regenerate Simplified Schema then Darkmatter shipped files with each area's `just shipped-schemas` and run focused envelope, catalog, effects, and shipped tests. This checkpoint precedes parallel CLI and DMLS integration.

### Wave 1

- [x] **CLI catalog route**
  - Own `darkmatter/cli/src/commands/schema/validate.rs` and CLI YAML validation tests. Preserve the load error with original span and related marks. Render the shared error in human and machine output, using existing terminal renderables and located record conventions. Ensure `validate_yaml` cannot reach `resolve_schema_file` for the two callable kinds. Use the CLI wiring cases from the matrix plus an existing valid ordinary schema with a template to prove its route is preserved.
  - Verify each invalid control leaves example or template sentinels untouched. Include a successful CLI check of the shipped expression and mutation sources through the captured checkout context.

- [x] **Editor catalog route**
  - Own `darkmatter/dmls/src/overlay/{yaml_contract,yaml_document,schema,suggestions,registry,mod}.rs`, `diagnostics/yaml_document.rs`, and the YAML authoring/diagnostic L1 tests. Support completion and hover for the new kind fields and local type declarations through the existing standalone tooling. Keep the generic authoring worker from resolving catalog imports as ordinary schema data. On an invalid catalog keep its whole YAML claim, suppress root `$schema` fallback, and publish one diagnostic per failure.
  - Prove valid → invalid → valid text with the same `kind` refreshes findings by buffer version. Check overload ranges, duplicate YAML keys and folded collisions against the same source fixture as the CLI. Check that repeated reads of the same installed buffer revision neither rebuild the catalog nor enter a handler.

### Checkpoint

- [x] **Catalog parity**
  - After Wave 1 run `just test -E 'test(catalog) | test(schema_validate_yaml) | test(yaml_document) | test(schema_reference)'` from `darkmatter/`. The public API cases also live in this filter. Keep LSP session tests headless.

## Phase 3 — Load v3 host registrations

- [x] **Verified companions**
  - Own `darkmatter/schemas/darkmatter-host.yaml` and `darkmatter/lib/src/config/registration.rs` plus prerequisite `registration/{manifest,bundle}.rs`. Build the closed v3 manifest and extend verified preparation with Rule 7. Use `FileReference` and the v2 bundle probe and identity path for companions. Use `biscuit_hash::blake3_hash_bytes` on exact member bytes and keep the existing canonical hash projection for normalized revision.
  - Use one classification/preparation path for embedded and installed inputs. In the disk reader perform the first digest validation for explicit members before classifying YAML; then resolve derived companions, record attempted companion paths, check the exact digest-key set and verify remaining bytes. Apply the same classifier, derivation and exact member/digest reconciliation to an embedded inventory. Unlisted or stray bytes never feed resolution. Extend `manifest_member_candidates` and the returned `LoadedRegistrations` watch records to keep both the manifest field and companion candidate during failure, including an unchanged manifest with missing companion bytes.
  - Verify v2 fixed-path input refuses with its version and the upgrade instruction. Retain the v2 fixture's boundary, retry, digest, copied/reordered identity, duplicate claim, and verified-byte tests with v3 normal fixtures. Add cases for differing YAML extensions, multiple dots, shared derived paths, missing digest versus missing bytes, valid digest with stale inner hash, and forbidden mutation with its exact kind location. Preserve expected missing paths for DMLS in each failure case.

- [x] **Binding facts**
  - Own `expression/host/{mod,artifact}.rs` and the host catalog helpers remaining in `markdown/schemas/catalog.rs`. Keep `analysis_only_at`'s direct-call behavior. Add strict registration projection over its result, carrying typed fact-field and alias provenance using `biscuit_file::data_locations::locate_json` for JSON pointers and the declaration map for YAML keys. Do not infer locations by matching rendered error text. Use the strict JSON decoder's location for syntax and duplicate failures. Keep the approved artifact format and encoder unchanged.
  - Validate cross-catalog names and aliases during profile assembly by walking members in normalized order. Keep alias-versus-alias, canonical-versus-alias and built-in alias collision cases, including two same-host catalogs with case variants. Prove a different host may publish the same canonical name and alias without affecting the first host. Preserve a private alias table per set and leave mount order irrelevant after collision admission.
  - Delete any callable schema-root probe retained by a prerequisite, including `is_host_function_catalog`'s old shape heuristic and that legacy path's cache tests. Preserve explicit `HostFunctionCatalog::load` and `HostFunctions` APIs, and preserve the registration loader's refusal on incomplete analysis. Replace tests that depend on root discovery with a root-discovery isolation case and registered-profile tests. Apply the facts and manifest matrix through public `load_entries` and published fixture bytes.

### Wave 2

- [x] **Compose host scope**
  - Own CLI `commands/compose.rs`, its host registration tests, and the analysis-only failure in `expression/functions/mod.rs::call_host`. Keep the existing selected-profile guard alive during passive reference checking, static call checking, and compose. Prove a bad call receives binding diagnostics while a valid call that reaches execution receives the host-runtime error. Use a probe with pre-conversion source exclusions in the companion and compare against an ordinary Markdown document and a document belonging to a different host. Do not change Markdown `md schema validate` into an expression linter.
  - Exercise an expression in a lifecycle action operand, verify the same spelling does not become an action key, and prove real host execution uses the bound handler with eager binding and return checks. Verify one built-in expression also accepted as an action still works in both positions, an effect-only mutation is not callable in an ordinary expression, and the current `load_data` read policy remains in force. Use the existing host runtime and lifecycle L1 fixtures.

- [x] **Editor host reload**
  - Own `darkmatter/dmls/src/overlay/hosts.rs`, host diagnostics projection and `registered_host_editor_agreement.rs`, `registered_host_views.rs`, and `host_registration_watches.rs`. Reuse the v2 installed revision and worker. Verify missing companion on initial load, removal of a valid companion, stale and restored bytes, coalesced publication, and refresh without reopening the document. Assert one load per settled revision for several documents, no reload for body edits, and no callable exposure from refused or unselected registrations.
  - Check disappearing completions and binding diagnostics after refusal and their recovery after repair. Assert case-insensitive canonical/alias lookup and the shared related-location information for conflicts. Include a catalog physically present in a schema root and absent from the manifest. Clear any legacy tests for implicit discovery.

### Checkpoint

- [x] **Registration parity**
  - After Wave 2 regenerate Darkmatter's shipped artifact with the area's `just shipped-schemas`. Run `just test -E 'test(host_registration) | test(registered_host) | test(host_functions) | test(host_selection) | test(host_function_diagnostics)'` from `darkmatter/`. Update the host registration and host functions topics with v3 and the scope/runtime boundary.

## Phase 4 — Publish and repair one inventory

- [x] **Inventory encoder**
  - Own `claudine/lib/src/composition/registration.rs`, the v2 embedded include-table generator under `claudine/lib/build.rs`, and Claudine's related registration and shipped tests. Implement Rule 9 and encode members, companions and lifecycle from one verified source set. The manifest names only YAML members in `files`, references lifecycle separately, and hashes them and each derived facts companion. Keep the generated lifecycle artifact at its v2 location. Use exact source bytes for private inventory construction and listing.
  - Keep the v2 writer's lock, preflight and atomicity in `composition/registration.rs`, and preserve the same-path setup handoff in `config/setup.rs`. Expose the inventory's kinds through the existing hidden `Commands::Registrations` adapter, with the v2 list/detail/publish semantics and 16-digit XXH64 byte fingerprints. Use the same typed classification for CLI tests; leave selection and execution untouched.
  - Add a fixture to the inventory tests with `schemas/project.yaml`, its generated facts, a valid trigger and the lifecycle artifact from the existing fixture. Bind an eager `project_double` handler and an alias, with a string source exclusion on its numeric input, then generate the companion via `HostFunctions::facts_artifact`. Test missing, stray and stale embedded companions and mutation source rejection before any destination is written. Check manifest fields, exact listing and detail bytes, published bytes, and stable timestamps for repeat publication.

- [x] **Doctor upgrade**
  - Own the prerequisite registration check and repair callback under `claudine/lib/src/doctor/`, its L1 tests and CLI repair fixture. Route inspection through the v3 inventory including `expression-facts`; keep the doctor severity and exit policy. Exercise fixture companion missing as an error, modified present bytes as a warning with the unchanged manifest, and an unsupported installed v2 manifest as an error with an update and republish remedy. Keep the existing repair safety, path and post-verification rules.
  - Load the real tool-owned v2 fixture at the v2 fixed path in an isolated home, retaining an equivalent user register reference, unrelated registration entries and config comments. Demonstrate setup converts it in place to v3, and run a separate v2 → v3 upgrade with `claudine doctor --fix`. Verify an installed v3 profile is accepted by Darkmatter after both upgrades. Assert the manifest is published last and a second setup/doctor repair with current bytes makes no change. For the companion-bearing fixture inject the inventory through the doctor's existing library test probe and use the same repair callback as production; prove reads and listing agree after repair.
  - Run `just test -E 'test(registration) | test(host_publication) | test(host_catalog) | test(setup) | test(doctor)'` from `claudine/`. Update `claudine/docs/topics/schemas.md`, `claudine/docs/cli/doctor.md` and coverage in the configuration topic for the companion inventory and upgrade behavior.

## Phase 5 — Verify generated outputs and publish guidance

### Wave 3

- [x] **Language guidance**
  - Own `simplified-schema/README.md` and relevant language topics, the root `docs/kind-documents.md` and `schemas/kind.yaml`, and `.claude/skills/simplified-schema/SKILL.md`. Explain catalog envelope validation versus Darkmatter callable checks. Include a compact example of a private helper type, category and overload and a link from the language index. Add the two literals beside the existing schema kinds in the root kind catalog. Keep canonical schema links in docs.

- [x] **Host guidance**
  - Own `darkmatter/README.md`, `darkmatter/docs/topics/{host-functions,host-registration,expression-engine}.md`, lifecycle host-artifact guidance, and `darkmatter/dmls/docs/diagnostics.md`. Show both catalog kinds, the v3 files/digests example with its sibling, and a Rust host binding generating the facts. Include the scope from declaration → selected-document analysis → host-only evaluation, and a short publish/repair workflow showing an older installed version and the in-place upgrade. Update the Darkmatter repo skill's schema and expression guidance and canonical `use-darkmatter` and `use-dmls` authoring/CLI/schema fragments. Document that the two catalogs disallow `when` and that lifecycle scope remains in the existing artifact.
  - Leave `darkmatter/docs/topics/darkmatter-expressions.md`'s generated table and `darkmatter/docs/functions/` to the generation task so there is no concurrent editing of generated sections.

- [x] **Publisher guidance**
  - Own `claudine/README.md`, finish the Claudine publication and doctor pages from Phase 4, and update `.claude/skills/claudine/SKILL.md` and the canonical `claudine/docs/use-skills/use-claudine/{cli,config,library}.md` fragments. Explain install/setup and doctor upgrade behavior, what listing shows, and companion missing/modified actions. Transclude the canonical docs in caller skills where appropriate. Link current docs to current docs.

- [x] **Generated parity**
  - Run `just regen-expr-doc` and `just regen-lifecycle-schema` from `darkmatter/`, followed by `just shipped-schemas` from the repository root. Validate Simplified Schema, Darkmatter and Claudine artifact provenance via their existing shipped checks. Run the root generator a second time and compare exact generated bytes, including the prerequisite Claudine lifecycle artifact.
  - Compare `darkmatter/schemas/partials/lifecycle.yaml` with the Phase 1 baseline bytes and assert equality. Run the existing expression descriptor/binding, lifecycle short/long, overload dispatch, deferred operand, `no_error`, event gating, and shipped/runtime fallback suites. Compare generated expression descriptors and docs with the baseline and retain the same signatures, alias lists, examples and order. Retagging alone requires no lifecycle import-name or data-contract change.

### Checkpoint

- [x] **Final validation**
  - After the wave, publish the updated caller skills with `just publish-skills` in `darkmatter/` and `claudine/`. Inspect the published examples and their template literals. Keep the repo's canonical sources as the only skill edits.
  - Run full scoped L1 and lint with `just test` and `just lint` in `simplified-schema/`, `darkmatter/`, and `claudine/`, plus Simplified Schema's `just test-minimal`. Keep L1 gates and runtime fixture paths from the Test Toolkit. Use existing CI environment coverage and portable tests for macOS, Linux, native Windows, and WSL2. These checks require no real terminal, browser, or network service. Do not add CI cells, run `cargo fmt`, create a branch, commit, or move the feature.
  - Record commands, results, any departures, generator comparisons, and representative CLI/DMLS findings and upgrade/repair output in `implementation-log.md`. Check off every task when its deliverables pass. Leave the implementation ready for review in the active feature directory.