---
spec: "/Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/spec.md"
plan: "darkmatter/features/2026-11-12-registration/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - "claudine/lib/Cargo.toml"
    - "claudine/lib/generated/shipped.json"
    - "claudine/lib/src/composition/schema/bundle.rs"
    - "claudine/lib/src/composition/schema/findings.rs"
    - "claudine/lib/src/composition/schema/shipped.rs"
    - "claudine/lib/src/invocation_context/document_tests.rs"
    - "claudine/lib/tests/l1/claudine_schemas_embedded.rs"
    - "claudine/lib/tests/l1/claudine_schemas_matrix.rs"
    - "claudine/lib/tests/l1/claudine_schemas.rs"
    - "claudine/lib/tests/l1/claudine_trigger_selection_parity.rs"
    - "claudine/lib/tests/l1/host_catalog.rs"
    - "claudine/lib/tests/l1/invalid_trigger_findings.rs"
    - "claudine/lib/tests/l1/shipped_artifacts.rs"
    - "darkmatter/cli/src/commands/clean.rs"
    - "darkmatter/cli/src/commands/schema/triggers.rs"
    - "darkmatter/cli/src/commands/schema/validate.rs"
    - "darkmatter/cli/src/commands/trigger_findings.rs"
    - "darkmatter/cli/src/render_style.rs"
    - "darkmatter/cli/tests/l1/schema_validate_yaml.rs"
    - "darkmatter/dmls/src/diagnostics/frontmatter.rs"
    - "darkmatter/dmls/src/overlay/mod.rs"
    - "darkmatter/dmls/src/overlay/registry.rs"
    - "darkmatter/dmls/src/overlay/schema.rs"
    - "darkmatter/dmls/src/overlay/yaml_contract.rs"
    - "darkmatter/dmls/src/overlay/yaml_document.rs"
    - "darkmatter/dmls/src/router.rs"
    - "darkmatter/dmls/src/slot_jobs.rs"
    - "darkmatter/dmls/tests/l1/claudine_schema_discovery.rs"
    - "darkmatter/dmls/tests/l1/schema_reference_parity.rs"
    - "darkmatter/dmls/tests/l1/schema_roots_parity.rs"
    - "darkmatter/dmls/tests/l1/yaml_document_editor.rs"
    - "darkmatter/lib/benches/schema_validation.rs"
    - "darkmatter/lib/Cargo.toml"
    - "darkmatter/lib/generated/shipped.json"
    - "darkmatter/lib/src/markdown/cleanup/document.rs"
    - "darkmatter/lib/src/markdown/cleanup/frontmatter.rs"
    - "darkmatter/lib/src/markdown/compose/context/report.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/contract.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/locate.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/parse.rs"
    - "darkmatter/lib/src/markdown/compose/schema_validation.rs"
    - "darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs"
    - "darkmatter/lib/src/markdown/output/command/clean.rs"
    - "darkmatter/lib/src/markdown/output/command/data.rs"
    - "darkmatter/lib/src/markdown/schemas/about.rs"
    - "darkmatter/lib/src/markdown/schemas/catalog.rs"
    - "darkmatter/lib/src/markdown/schemas/clean.rs"
    - "darkmatter/lib/src/markdown/schemas/embedded.rs"
    - "darkmatter/lib/src/markdown/schemas/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/roots.rs"
    - "darkmatter/lib/src/markdown/schemas/schema_files.rs"
    - "darkmatter/lib/src/markdown/schemas/shipped.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/codes.rs"
    - "darkmatter/lib/src/markdown/schemas/tests/reset_process_caches.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/assemble.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/discovery.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/envelope.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/finding.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/grammar.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/lint.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/matcher.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs"
    - "darkmatter/lib/src/markdown/schemas/triggers/prepared.rs"
    - "darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs"
    - "darkmatter/lib/src/markdown/schemas/yaml_documents/marks.rs"
    - "darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/yaml_documents/registry.rs"
    - "darkmatter/lib/src/markdown/schemas/yaml_documents/select.rs"
    - "darkmatter/lib/src/markdown/schemas/yaml_documents/semantic.rs"
    - "darkmatter/lib/src/work_counters.rs"
    - "darkmatter/lib/tests/l1/base_schema_end_to_end.rs"
    - "darkmatter/lib/tests/l1/deferred_positions.rs"
    - "darkmatter/lib/tests/l1/directive_target_analysis.rs"
    - "darkmatter/lib/tests/l1/kind_semantic_checks.rs"
    - "darkmatter/lib/tests/l1/meta_schema_phase4.rs"
    - "darkmatter/lib/tests/l1/prepared_trigger_bundles.rs"
    - "darkmatter/lib/tests/l1/recursive_schema_golden.rs"
    - "darkmatter/lib/tests/l1/recursive_schema_merge.rs"
    - "darkmatter/lib/tests/l1/schema_cache_validity.rs"
    - "darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs"
    - "darkmatter/lib/tests/l1/schema_language_ownership_guard.rs"
    - "darkmatter/lib/tests/l1/schema_phase_validation.rs"
    - "darkmatter/lib/tests/l1/schema_reference_parity.rs"
    - "darkmatter/lib/tests/l1/schema_roots.rs"
    - "darkmatter/lib/tests/l1/schemas_effective_presence.rs"
    - "darkmatter/lib/tests/l1/schemas_literal_expression.rs"
    - "darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs"
    - "darkmatter/lib/tests/l1/shared_acyclic_consumers.rs"
    - "darkmatter/lib/tests/l1/shipped_artifacts.rs"
    - "darkmatter/lib/tests/l1/shipped_schema_corpus.rs"
    - "darkmatter/lib/tests/l1/trigger_discovery_recovery.rs"
    - "darkmatter/lib/tests/l1/trigger_envelope_claim.rs"
    - "darkmatter/lib/tests/l1/trigger_input_matrix.rs"
    - "darkmatter/lib/tests/l1/trigger_match_grammar.rs"
    - "darkmatter/lib/tests/l1/trigger_payload_load_errors.rs"
    - "darkmatter/lib/tests/l1/trigger_selection_parity.rs"
    - "darkmatter/lib/tests/l1/trigger_spelling_guard.rs"
    - "darkmatter/lib/tests/l1/yaml_document_loading.rs"
    - "darkmatter/lib/tests/l1/yaml_document_selection.rs"
    - "darkmatter/lib/tests/l1/yaml_kind_registry.rs"
    - "darkmatter/schemas/schema-trigger.yaml"
    - "scripts/ci/test_affected_scope.py"
    - "simplified-schema/lib/generated/shipped.json"
    - "simplified-schema/lib/src/embedded.rs"
    - "simplified-schema/lib/src/envelope.rs"
    - "simplified-schema/lib/src/lib.rs"
    - "simplified-schema/lib/src/roots.rs"
    - "simplified-schema/lib/src/shipped/registry.rs"
    - "simplified-schema/lib/src/slots.rs"
    - "simplified-schema/lib/src/triggers/assemble.rs"
    - "simplified-schema/lib/src/triggers/discovery.rs"
    - "simplified-schema/lib/src/triggers/envelope.rs"
    - "simplified-schema/lib/src/triggers/finding.rs"
    - "simplified-schema/lib/src/triggers/grammar.rs"
    - "simplified-schema/lib/src/triggers/lint.rs"
    - "simplified-schema/lib/src/triggers/matcher.rs"
    - "simplified-schema/lib/src/triggers/mod.rs"
    - "simplified-schema/lib/src/triggers/payload_memo_tests.rs"
    - "simplified-schema/lib/src/triggers/prepared.rs"
    - "simplified-schema/lib/src/work_counters.rs"
    - "simplified-schema/lib/src/yaml_documents/load.rs"
    - "simplified-schema/lib/src/yaml_documents/marks.rs"
    - "simplified-schema/lib/src/yaml_documents/mod.rs"
    - "simplified-schema/lib/src/yaml_documents/registry.rs"
    - "simplified-schema/lib/src/yaml_documents/select.rs"
    - "simplified-schema/lib/src/yaml_documents/semantic.rs"
    - "simplified-schema/lib/tests/l1/envelope_admission.rs"
    - "simplified-schema/lib/tests/l1/shipped_artifacts.rs"
    - "simplified-schema/lib/tests/l1/shipped_corpus.rs"
    - "simplified-schema/schemas/schema-trigger.yaml"
docs_updated_during_phase_1:
    - "darkmatter/docs/topics/schemas/authoring-schemas.md"
    - "darkmatter/docs/topics/schemas/definition.md"
    - "darkmatter/docs/topics/schemas/schema-discovery.md"
    - "simplified-schema/README.md"
    - "simplified-schema/docs/topics/features.md"
    - "simplified-schema/docs/topics/language-core.md"
    - "simplified-schema/docs/topics/language/schema-documents.md"
    - "simplified-schema/docs/topics/shipped-artifacts.md"
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - ".claude/skills/darkmatter/SKILL.md"
    - ".claude/skills/darkmatter/cli.md"
    - ".claude/skills/darkmatter/dmls.md"
    - ".claude/skills/darkmatter/expressions.md"
    - ".claude/skills/darkmatter/library-surfaces.md"
    - ".claude/skills/darkmatter/schema.md"
    - ".claude/skills/simplified-schema/SKILL.md"
    - ".claude/skills/simplified-schema/build-and-guards.md"
    - ".claude/skills/simplified-schema/schema-documents.md"
packages:
    - "simplified-schema"
    - "darkmatter"
    - "darkmatter-cli"
    - "darkmatter-lsp"
    - "claudine"
decisions_for_review:
    - |-
      Darkmatter no longer re-exports the moved types (triggers, schema roots, EmbeddedSchemaBundle, KindRegistry, LoadedYamlDocument, RegistryScope) from `darkmatter::markdown::schemas`; every workspace import now names `simplified_schema::{triggers, roots, embedded, yaml_documents}` directly, as the plan's "update workspace imports directly" asks. Darkmatter keeps a small `markdown/schemas/triggers.rs` with only `scan`, `scan_with_embedded`, `scan_with_source`, and `prepare_trigger_bundle` wrappers that add its shipped sources, plus `kind_host()`, `shipped_kind_registry()`, and a new `shipped_sources()`. Alternative: keep re-exports for a softer migration (fewer import edits, but two paths to one type). Reversing means re-adding `pub use` lines in `darkmatter/lib/src/markdown/schemas/mod.rs`; no behavior depends on it.
    - |-
      Host sources are threaded by storing them on the result rather than passing them to every call: `TriggerRegistry` remembers the `ShippedSources` it was scanned with (`sources()`), and payload/embedded-trigger memo keys include the namespace. `KindRegistry` takes a `KindHost { sources, shipped }` struct. `ShippedSources` gained `PartialEq` (namespace, partials folder, document paths and texts) so `TriggerRegistry` keeps deriving `PartialEq`. Alternative: pass sources to each matching/evaluation call (more signature churn, no stored state). Reversing touches `simplified-schema/lib/src/triggers/{discovery,assemble}.rs` and Darkmatter's `effective_for` call site.
    - |-
      The trigger definition's prepared form stays in Darkmatter's shipped artifact (`kind_definitions`), not in Simplified Schema's, even though `schema-trigger.yaml` is now library-owned and listed in `ENVELOPE_DEFINITIONS`. The library artifact's payload still holds only the two lowered envelope definitions; its header now fingerprints all three. This keeps the artifact formats unchanged ("supplied shipped kind definitions" in the plan). Alternative: move all kind-definition records into the library artifact (cleaner ownership, but a new payload member and a larger regeneration change). Reversing means moving `KindDefinitionRecord` and `accept_kind_definitions` from `darkmatter/lib/src/markdown/schemas/shipped.rs` into `simplified-schema/lib/src/shipped/registry.rs`.
    - |-
      Without the `file` feature, Simplified Schema gates `triggers::{scan, scan_with_embedded, enumerate_root_files}`, the filesystem root source, and `KindRegistry::from_filesystem`, and a `$path` match condition never admits a document (a trigger with `$path` never matches), mirroring how `file_context` reports file checks as unavailable rather than passing them. `SchemaRoots::for_document`, `scan_with_source`, and `KindRegistry::build` remain available without `file` (they use only `std::fs` probes, no `sniff`/`gix`/`walkdir`). Alternative: also gate the schema roots and `scan_with_source`. Reversing is a matter of `#[cfg(feature = "file")]` attributes in `simplified-schema/lib/src/{roots.rs,triggers/discovery.rs}`.
message_to_agent: |-
    Phase 1 moved schema-kind infrastructure into simplified-schema: `simplified_schema::{triggers, roots, yaml_documents}` and `embedded::EmbeddedSchemaBundle`; `schema-trigger.yaml` now lives at `simplified-schema/schemas/schema-trigger.yaml` (third entry of `envelope::ENVELOPE_DEFINITIONS`). Library entry points that resolve payloads take a `ShippedSources`; from Darkmatter code use `darkmatter::markdown::schemas::{shipped_sources(), kind_host(), shipped_kind_registry()}` or the wrappers in `darkmatter::markdown::schemas::triggers` (`scan`, `scan_with_embedded`, `scan_with_source`, `prepare_trigger_bundle`). Darkmatter no longer re-exports the moved types. Preparation revisions are now Simplified Schema 2, Darkmatter 3, Claudine 3; any later change to what shipped sources prepare to must bump them again and run `just shipped-schemas` from the root. The Phase 2 `ValidationSuccess::is_converted` rename and default engine have not been started.
---

# Implementation Log for 2026-11-12-registration (9 phases)

## Phase 1

### Wave 1 — Move schema core

Moved, with history-preserving renames, into `simplified-schema/lib/src/`:

- `darkmatter/lib/src/markdown/schemas/triggers/` → `triggers/` (grammar, envelope, lint, matcher, discovery, assemble, prepared, finding, payload-memo unit tests). `SCHEMA_INVALID_TRIGGER_CODE` moved with it (Darkmatter re-exports the code).
- `darkmatter/lib/src/markdown/schemas/roots.rs` → `roots.rs`.
- `yaml_documents/{load,marks,registry,select,semantic}.rs` → `yaml_documents/`, plus a new library `yaml_documents/mod.rs` holding `LoadedYamlDocument` (now with `text()`, `path()`, `loaded()`, and `kind_checks(...)` accessors).
- `EmbeddedSchemaBundle` (and its two unit tests) → `simplified_schema::embedded`.
- `darkmatter/schemas/schema-trigger.yaml` → `simplified-schema/schemas/schema-trigger.yaml`, now the third entry of `envelope::ENVELOPE_DEFINITIONS` (`SCHEMA_TRIGGER_DEFINITION_PATH`). Only the two envelope definitions admit envelopes; the library artifact's payload is unchanged (still two lowered envelopes), its header now fingerprints three sources.
- `SLOT_NOT_ALLOWED_IN_KIND_CODE` and `slots_not_allowed_message` → `simplified_schema::slots` (Darkmatter re-exports).
- Work counters `TriggerDiscovery`, `PayloadMergeCheck`, `PayloadCompileCheck`, `YamlKindCheck` (with thread recording, `work_counters::threads`) are counted by the library and folded into `darkmatter::work_counters::{snapshot, threads}`.

Host inputs are explicit (Rule 2): every library entry point that resolves a payload takes `ShippedSources` (`scan`, `scan_with_embedded`, `scan_with_source`, `prepare_trigger_bundle`, `resolve_trigger_payload`, `TriggerRegistry::new`); `TriggerRegistry` stores the sources it was scanned with (`sources()`), and memo keys (file-payload memo, embedded-trigger memo) include the namespace. `KindRegistry::{build, from_filesystem}` take a `KindHost { sources, shipped: &[ShippedDefinition] }`; `ShippedDefinition::accepted`, `prepare_shipped`, and `prepare_shipped_definitions(sources)` are public so a host supplies its artifact's prepared definitions or text fallback. `ShippedSources` gained `namespace()` and `PartialEq` (namespace, partials folder, document paths and texts).

Darkmatter keeps only delegation/adaptation: `DarkmatterSchemas::{validate_yaml_document, prepare_yaml_contract}`, `YamlContract`, `YamlValidation`, `YamlOutcome`, `Route`; a new `markdown/schemas/triggers.rs` holding the four wrappers that add `SHIPPED_SOURCES`; and `kind_host()`, `shipped_kind_registry()`, `shipped_sources()`. `darkmatter::markdown::schemas` no longer re-exports the moved types; every workspace import (Darkmatter lib/CLI/DMLS sources, tests, and benches; Claudine lib/CLI) was rewritten to `simplified_schema::{triggers, roots, embedded, yaml_documents}`.

Feature boundary (Rule 3): `triggers::{scan, scan_with_embedded, enumerate_root_files}`, the filesystem root source, and `KindRegistry::from_filesystem` require `file`; without it a `$path` condition never admits a document. `cargo tree -p simplified-schema --no-default-features` contains no `sniff`, `gix`, or `walkdir`. Filesystem-backed unit tests are gated on `file` so `just test-minimal` compiles.

Artifacts: preparation revisions bumped (Simplified Schema 1→2, Darkmatter 2→3, Claudine 2→3) and `just shipped-schemas` regenerated all three. A structural JSON diff of old and new artifacts shows only header changes (revisions, dependency digests, the added source) and the trigger definition's identity/namespace path (`darkmatter/schemas/…` → `simplified-schema/schemas/…`); no schema payload meaning changed.

Freshness/source inputs: Darkmatter's `shipped_artifact_is_fresh` and manifest now spell `simplified-schema/schemas/schema-trigger.yaml` and `simplified-schema/lib/src/yaml_documents/registry.rs`; Claudine's spell the four moved `simplified-schema/lib/src/triggers/*.rs` files instead of the old Darkmatter paths; `scripts/ci/test_affected_scope.py` and `trigger_spelling_guard.rs` use the new trigger paths.

### Wave 2 — Ownership checks and documentation

- `schema_language_ownership_guard.rs` now pins the moved symbols in their library files (trigger grammar/matcher/envelope/discovery/assemble/prepared, `SchemaRoots`, `EmbeddedSchemaBundle`, `KindRegistry`/`ShippedDefinition`/`prepare_shipped`, `LoadedYaml`/`LoadedYamlDocument`, `Selection`, `kind_checks`), so Darkmatter, its CLI, and DMLS cannot grow a second implementation.
- Unit regression cases moved with their modules (discovery, assemble, payload memo, matcher, envelope, grammar, lint, marks, embedded bundle checks); Darkmatter's integration suites (`trigger_*`, `yaml_*`, `prepared_trigger_bundles`, `schema_embedded_trigger_bundles`, `shipped_schema_corpus`, `kind_semantic_checks`) were retained and rewired to the library paths, preserving exact `md schema triggers` origins and errors (CLI suites unchanged and passing).
- Corpus: Darkmatter's `shipped_schema_corpus` now scans `simplified-schema/schemas` and classifies all three shipped `yaml-schema` definitions as `YamlSchemaDefinition` (keeps the "refused as a `$schema` target" coverage through Darkmatter's resolver); its checkout-copy-replaces-built-in assertion moved to the `simplified-schema/schemas` folder. The library corpus now admits `schema-trigger.yaml` alongside the two envelope definitions. `yaml_kind_registry` fixtures point `SCHEMAS_DIR` at `simplified-schema/schemas`.
- `simplified-schema/lib/tests/l1/shipped_artifacts.rs`: the mutated-source pointer is looked up by identity (header sources are path-sorted and `schema-trigger.yaml` sorts first); `another_revision` uses `PREPARATION_REVISION + 1`; `one_invalid_source` points at index 3.
- Documentation (a Documenter subagent, docs/skills only): 8 docs and 9 skill files updated (see frontmatter); a stale comment it found in `simplified-schema/lib/src/shipped/registry.rs` was corrected in code.

### Requirement → test mapping

| Behavior | Tests |
| --- | --- |
| Ownership boundary (no second implementation) | `darkmatter::l1 schema_language_ownership_guard::*` |
| Trigger grammar, discovery order, shadowing, matcher, payload memo unchanged | library unit tests `triggers::*` (moved), `darkmatter::l1 trigger_*`, `schema_roots`, `trigger_selection_parity`, `claudine::l1 claudine_trigger_selection_parity` |
| Kind selection / registry / checkout-copy replacement at the new shipped path | `darkmatter::l1 yaml_kind_registry::*`, `kind_semantic_checks::*`, `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` |
| Trigger definition shipped from library and admitted | `simplified-schema::l1 envelope_admission::*`, `shipped_corpus::the_library_envelope_definitions_pass_the_compiled_definitions` |
| Prepared bundle meaning unchanged across the move | `darkmatter::l1 prepared_trigger_bundles::*`, `claudine::l1 shipped_artifacts::*` |
| Artifact chain fresh and regenerated together | `shipped_artifact_is_fresh` in all three packages; `scripts/ci/test_affected_scope.py RealWorkspaceTestInputTests.test_shipped_schema_inputs_select_each_downstream_freshness_test` |
| Minimal feature build without filesystem discovery | `just test-minimal` (simplified-schema, 1773 tests) and `cargo tree --no-default-features` (no sniff/gix/walkdir) |
| End-to-end through the CLI with the shipped artifact | `darkmatter-cli::l1 schema_validate_yaml::*`, `schema_validate*`, `md schema triggers` suites (in `just test`) |

This phase is a pure ownership move; no parser, schema, or persisted-value behavior changed, so no new input-robustness cells apply.

### Gates

- `simplified-schema`: `just test` — 2129 passed (default) + 1773 passed (minimal), none ≥ 5 s; `just lint` clean; `just check-tier-coverage simplified-schema` — 0 stranded.
- `darkmatter`: `just test` — 16282 passed, 10 skipped, none ≥ 5 s; `just lint` clean; `just check-tier-coverage darkmatter` — 0 stranded.
- Dependents: `claudine` filtered to `shipped_artifacts|claudine_schema|trigger|host_catalog|container_schema_recursion|composition::schema` — 1116 passed; `claudine-cli` filtered to `schema_completion|inline_compose|signals` — 241 passed. All targets of every touched package compile (`cargo check --all-targets`).
- Gates were not skipped (the phase changes compiled source in five packages).
- Cross-OS: not run. The move changes module ownership and import paths only; path handling (`roots.rs`, `shipped_identity`) is byte-identical apart from the source of the shipped-path list, so there is no new OS-specific risk; CI covers Linux/macOS and the nightly covers Windows/WSL2.
- No failing or pre-existing failing tests remain.
