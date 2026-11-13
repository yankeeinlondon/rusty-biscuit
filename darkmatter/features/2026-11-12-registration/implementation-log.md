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
    - "sniff"
    - "messenger"
source_files_during_phase_2:
    - "claudine/lib/src/composition/schema/classify.rs"
    - "claudine/lib/src/composition/schema/defaults_tests.rs"
    - "claudine/lib/src/composition/schema/launch_defaults.rs"
    - "claudine/lib/src/composition/schema/mod.rs"
    - "claudine/lib/src/composition/schema/status_render.rs"
    - "claudine/lib/tests/l1/claudine_schemas.rs"
    - "darkmatter/cli/schemas/output/schema-validate.yaml"
    - "darkmatter/cli/src/commands/schema/validate.rs"
    - "darkmatter/cli/tests/l1/compose_schema.rs"
    - "darkmatter/cli/tests/l1/output_commands.rs"
    - "darkmatter/dmls/src/diagnostics/frontmatter.rs"
    - "darkmatter/dmls/src/providers/frontmatter.rs"
    - "darkmatter/dmls/tests/l1/main.rs"
    - "darkmatter/dmls/tests/l1/required_defaults.rs"
    - "darkmatter/lib/Cargo.toml"
    - "darkmatter/lib/src/atomic_replace.rs"
    - "darkmatter/lib/src/config/migration_receipt.rs"
    - "darkmatter/lib/src/config/mod.rs"
    - "darkmatter/lib/src/config/write.rs"
    - "darkmatter/lib/src/effects/fs_write.rs"
    - "darkmatter/lib/src/lib.rs"
    - "darkmatter/lib/src/markdown/compose/expression/host/artifact.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/shape.rs"
    - "darkmatter/lib/src/markdown/compose/pipeline/mod.rs"
    - "darkmatter/lib/src/markdown/compose/schema_validation.rs"
    - "darkmatter/lib/src/markdown/hash/mod.rs"
    - "darkmatter/lib/src/markdown/hash/node_edit.rs"
    - "darkmatter/lib/src/markdown/hash/path_edit.rs"
    - "darkmatter/lib/src/markdown/schemas/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs"
    - "darkmatter/lib/src/strict_json.rs"
    - "darkmatter/lib/src/yaml_edit.rs"
    - "darkmatter/lib/tests/l1/compose_setters.rs"
    - "darkmatter/lib/tests/l1/composition_defaults.rs"
    - "darkmatter/lib/tests/l1/config_migration_receipt.rs"
    - "darkmatter/lib/tests/l1/config_write.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/lib/tests/l1/schema_phase_validation.rs"
    - "darkmatter/lib/tests/l1/validation_contract_parity.rs"
    - "darkmatter/lib/tests/l1/yaml_edit.rs"
    - "messenger/lib/src/research/load.rs"
    - "messenger/lib/tests/research_corpus.rs"
    - "simplified-schema/lib/examples/validate_person.rs"
    - "simplified-schema/lib/src/coerce.rs"
    - "simplified-schema/lib/src/defaults.rs"
    - "simplified-schema/lib/src/errors.rs"
    - "simplified-schema/lib/src/lib.rs"
    - "simplified-schema/lib/src/outcome.rs"
    - "simplified-schema/lib/src/prepared.rs"
    - "simplified-schema/lib/src/triggers/finding.rs"
    - "simplified-schema/lib/tests/l1/declaration_narrowing.rs"
    - "simplified-schema/lib/tests/l1/envelope_admission.rs"
    - "simplified-schema/lib/tests/l1/main.rs"
    - "simplified-schema/lib/tests/l1/required_defaults.rs"
    - "simplified-schema/lib/tests/l1/root_union_assertions.rs"
    - "simplified-schema/lib/tests/l1/validation_contract.rs"
    - "sniff/lib/src/error.rs"
    - "sniff/lib/src/filesystem/git/committed_tree.rs"
    - "sniff/lib/src/filesystem/git/mod.rs"
    - "sniff/lib/src/filesystem/mod.rs"
    - "sniff/lib/tests/l1/committed_tree.rs"
    - "sniff/lib/tests/l1/main.rs"
docs_updated_during_phase_2:
    - "claudine/docs/topics/composition.md"
    - "claudine/docs/topics/frontmatter-properties.md"
    - "darkmatter/dmls/docs/diagnostics.md"
    - "darkmatter/dmls/docs/hover.md"
    - "darkmatter/docs/cli/schema.md"
    - "darkmatter/docs/inline/schema-validation.md"
    - "darkmatter/docs/topics/darkmatter-expressions.md"
    - "darkmatter/docs/topics/frontmatter.md"
    - "darkmatter/docs/topics/schemas/authoring-schemas.md"
    - "darkmatter/docs/topics/schemas/definition.md"
    - "darkmatter/README.md"
    - "simplified-schema/docs/topics/global-type-constraints.md"
    - "simplified-schema/docs/topics/language/types.md"
    - "simplified-schema/docs/topics/simplified-schema-struct.md"
    - "simplified-schema/docs/topics/validation.md"
    - "sniff/docs/sniff-library-architecture.md"
docs_created_during_phase_2:
    - "darkmatter/docs/topics/config-writes.md"
    - "darkmatter/docs/topics/yaml-editing.md"
skills_files_updated_during_phase_2:
    - ".claude/skills/claudine/cli-commands.md"
    - ".claude/skills/darkmatter/compose.md"
    - ".claude/skills/darkmatter/dmls.md"
    - ".claude/skills/darkmatter/frontmatter.md"
    - ".claude/skills/darkmatter/library-surfaces.md"
    - ".claude/skills/darkmatter/schema.md"
    - ".claude/skills/darkmatter/SKILL.md"
    - ".claude/skills/simplified-schema/SKILL.md"
    - ".claude/skills/sniff/remote-and-repository.md"
    - ".claude/skills/sniff/SKILL.md"
decisions_for_review:
    - |-
      Darkmatter no longer re-exports the moved types (triggers, schema roots, EmbeddedSchemaBundle, KindRegistry, LoadedYamlDocument, RegistryScope) from `darkmatter::markdown::schemas`; every workspace import now names `simplified_schema::{triggers, roots, embedded, yaml_documents}` directly, as the plan's "update workspace imports directly" asks. Darkmatter keeps a small `markdown/schemas/triggers.rs` with only `scan`, `scan_with_embedded`, `scan_with_source`, and `prepare_trigger_bundle` wrappers that add its shipped sources, plus `kind_host()`, `shipped_kind_registry()`, and a new `shipped_sources()`. Alternative: keep re-exports for a softer migration (fewer import edits, but two paths to one type). Reversing means re-adding `pub use` lines in `darkmatter/lib/src/markdown/schemas/mod.rs`; no behavior depends on it.
    - |-
      Host sources are threaded by storing them on the result rather than passing them to every call: `TriggerRegistry` remembers the `ShippedSources` it was scanned with (`sources()`), and payload/embedded-trigger memo keys include the namespace. `KindRegistry` takes a `KindHost { sources, shipped }` struct. `ShippedSources` gained `PartialEq` (namespace, partials folder, document paths and texts) so `TriggerRegistry` keeps deriving `PartialEq`. Alternative: pass sources to each matching/evaluation call (more signature churn, no stored state). Reversing touches `simplified-schema/lib/src/triggers/{discovery,assemble}.rs` and Darkmatter's `effective_for` call site.
    - |-
      The trigger definition's prepared form stays in Darkmatter's shipped artifact (`kind_definitions`), not in Simplified Schema's, even though `schema-trigger.yaml` is now library-owned and listed in `ENVELOPE_DEFINITIONS`. The library artifact's payload still holds only the two lowered envelope definitions; its header now fingerprints all three. This keeps the artifact formats unchanged ("supplied shipped kind definitions" in the plan). Alternative: move all kind-definition records into the library artifact (cleaner ownership, but a new payload member and a larger regeneration change). Reversing means moving `KindDefinitionRecord` and `accept_kind_definitions` from `darkmatter/lib/src/markdown/schemas/shipped.rs` into `simplified-schema/lib/src/shipped/registry.rs`.
    - |-
      Without the `file` feature, Simplified Schema gates `triggers::{scan, scan_with_embedded, enumerate_root_files}`, the filesystem root source, and `KindRegistry::from_filesystem`, and a `$path` match condition never admits a document (a trigger with `$path` never matches), mirroring how `file_context` reports file checks as unavailable rather than passing them. `SchemaRoots::for_document`, `scan_with_source`, and `KindRegistry::build` remain available without `file` (they use only `std::fs` probes, no `sniff`/`gix`/`walkdir`). Alternative: also gate the schema roots and `scan_with_source`. Reversing is a matter of `#[cfg(feature = "file")]` attributes in `simplified-schema/lib/src/{roots.rs,triggers/discovery.rs}`.
    - |-
      Union defaults: when some arm of a union accepts a value exactly as written, no defaults are applied, even if another arm would also succeed once its defaults were filled. This keeps every existing verdict unchanged and avoids calling an already-valid value ambiguous. The plan only said to judge each arm's complete candidate and reject conflicting ones. Alternative: always build every arm's defaulted candidate and report ambiguity whenever two differ, which is stricter but can turn currently valid documents into errors. Reversing touches `Walk::union`/`root_union` in `simplified-schema/lib/src/defaults.rs` and its union tests.
    - |-
      Defaults are checked only when a value actually needs them (an absent required property), not when a schema is prepared. A schema with a broken default therefore loads fine and fails only for documents that rely on that default. Alternative: validate every default at preparation time, which finds broken defaults earlier but could make existing shipped or user schemas fail to load. Reversing is moderate work in `Prepared::compiled_through` and Darkmatter's effective-schema preparation.
    - |-
      A refused default (wrong type, breaks a constraint, ambiguous across union arms, or recursive) is reported as `ValidationFailure::Unprepared` by the library and as one `ConstraintViolation` problem (`dm.schema.constraint`) pointing at the default's JSON Pointer in Darkmatter. No new problem code or failure variant was added. Alternative: a dedicated `InvalidDefault` problem code, which is clearer for tools but means new match arms in simplified-schema, Darkmatter, DMLS, and Claudine. Small, contained change if wanted.
    - |-
      Composition inserts defaults in every schema pass, before type conversion, and marks inserted values as data so a default's own text is never scanned for `{{ }}` or `$( )`. Frontmatter interpolation pass 1 runs before the schema stage, so a pass-1 template sees the property as absent; pass 2, shell expansion, and the body see the default (spec 3.3 lists those). Alternatives: insert only from the final verdict (breaks conversion when a required child is missing) or allow templates inside defaults (would make defaults executable). Reversing the data marking is one loop in `compose/pipeline/mod.rs`.
    - |-
      `md schema validate --output json` adds a `defaults: [{ path, value }]` array to a file record only when it is non-empty, so existing outputs do not change. The terminal (pretty) output does not show defaults. Alternative: always emit the key (more regular for scripts) and show defaults in the terminal report. Cheap to change in `darkmatter/cli/src/commands/schema/validate.rs` and `cli/schemas/output/schema-validate.yaml`.
    - |-
      Plain-YAML edits refuse any path that reaches an alias, anchor, or tag up front, including removals and edits inside an anchored node that nothing aliases, which the Markdown frontmatter editor allows. A list append is a `Set` whose last index equals the list length. Newly written nested lists are not indented under their key (`key:\n- x`), matching the existing Markdown serializer. Alternatives: relax the anchor refusal to match Markdown, or indent new lists (which would also change Markdown output). The refusal is a few lines in `darkmatter/lib/src/yaml_edit.rs` (`refuse_unsupported_nodes`).
    - |-
      Locked configuration writes refuse a target whose parent directory does not exist rather than creating it; Windows read-only targets are refused instead of having their attribute cleared; new files get the temp file's default mode (0600 on Unix); a multi-file save validates and writes receipts for every file before any rename but is not atomic across files (a later rename failure lists what already landed). `SourceExpectation::Current` lets cooperating writers re-read under the lock instead of aborting each other. Test seams reuse the existing `effects-instrumentation` feature. Lock files (`<name>.darkmatter-lock`) stay beside repository configs and show as untracked files until a later phase adds an ignore entry. Alternatives: create parent directories (Claudine init would otherwise do it), clear read-only, or a new test-seam feature. Each is local to `darkmatter/lib/src/config/write.rs`.
    - |-
      The migration receipt is written inside `ConfigTransaction::commit` (through `TargetChange::with_migration`), after validation and before the target is replaced, rather than through a separate public write call. This makes "receipt before publication" and "receipt hash equals the published bytes" structural; a stale receipt from a failed save is refused at cleanup as `Unpublished`. Backup paths must also appear in the caller's known-source list. Alternative: a separate `write_receipt` API, more flexible for Phase 3's converter but easier to misuse. Reversing touches `config/write.rs` and `config/migration_receipt.rs`.
    - |-
      Sniff's committed-tree snapshot takes `/`-separated repository-relative paths only and rejects `\\` on every OS (Windows included), returns ids as hex strings, errors on non-UTF-8 names in `paths()` instead of returning lossy names, and lists only files and symlinks (not directories or submodules). Alternatives: accept `\\` on Windows, use byte paths, or include submodules for the CI checker. Each is local to `sniff/lib/src/filesystem/git/committed_tree.rs`.
    - |-
      Claudine shows an absent required property with a default as a new status state `Defaulted`. At launch it runs one extra validation without phase rules, only when some non-eager required property is absent and has a default, so a refused default blocks before the agent runs instead of surfacing at the completion check. A refused default nested under a present non-eager parent is still found only at completion. Alternatives: reuse `Valid`/`Missing` for the status, or always/never run the extra validation. Reversing is local to `claudine/lib/src/composition/schema/{classify,mod,status_render}.rs`.
message_to_agent: |-
    Phase 2 landed the foundations Phase 3 builds on. Defaults: `simplified_schema::defaults::{prepare_defaults, AppliedDefault}`; `ValidationSuccess::defaults` and `is_modified` (renamed from `is_converted`); Darkmatter `ValidationReport::defaults`; composition inserts absent required defaults (never edits the source file). YAML edits: `darkmatter::yaml_edit::{YamlEdit, edit_yaml_paths, YamlEditError}` (no whole-file serialization fallback; append = Set at index == len). Locked writes: `darkmatter::config::write::{ConfigTransaction, TargetChange, SourceExpectation, ConfigWriteError}` (lock `<name>.darkmatter-lock`, 2 s deadline, validator callback, missing parent dir refused) and `darkmatter::config::migration_receipt::{MigrationSource, complete_cleanup, read_receipt, MigrationError}` (receipt `<name>.darkmatter-migration.json`, written inside `commit` via `TargetChange::with_migration`; cleanup takes the caller's exact known legacy paths). `darkmatter::config` currently holds only `write` and `migration_receipt`; Phase 3 adds schema/load/etc. The crate-private `crate::strict_json::parse` is the one strict JSON reader. Sniff: `GitRepo::committed_tree() -> CommittedTreeSnapshot` (`commit_id()`, `tree_id()`, `read_file(&str) -> CommittedEntry::{Absent, Blob, Symlink, Directory, Submodule}`, `paths()`), not yet retained in Darkmatter's `RepositoryObservation`/`ContextCapture` (Rule 12's wiring is still to do when configuration loading needs it). Shipped artifacts did not change in Phase 2 (preparation revisions still Simplified Schema 2, Darkmatter 3, Claudine 3). Unrelated local edits under `prompts/` appeared during Phase 2 and are not part of this work.
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

## Phase 2

Phase 2 ran as two waves of parallel subagents (Wave 3: default engine, YAML edits, committed trees; Wave 4: composition defaults with locked writes, then default consumers), each verifying only its own slice; the coordinator ran the area gates once afterwards.

### Wave 3 — Independent foundations

- **Default engine (Rules 4–6).** New `simplified-schema/lib/src/defaults.rs`: `AppliedDefault { path, value }`, `DefaultPreparation { candidate, defaults }`, and the single entry point `prepare_defaults(json_schema, instance, templates, validators, cache)`. It walks the lowered schema (`$ref`/`$defs`, `allOf`, property and root unions, array items), fills only absent required properties whose default is resolved, parents before children, declaration order, ascending indexes, first occurrence wins. Each default is judged strictly (structural validators, no I/O, no coercion) against its complete property contract. Refusals are new `SchemaError::{InvalidDefault, AmbiguousDefaults, RecursiveDefault}` located by JSON Pointer into the lowered schema (no source spans exist there) and surface from `Prepared` as `ValidationFailure::Unprepared`. `Prepared::validate_strict` judges the completed candidate but returns the input borrowed plus `defaults`; `validate_soft` inserts, converts, then inserts again (conversion of a parent commits only once its required children are present). `ValidationSuccess` gained `defaults`; `is_converted` was renamed `is_modified` at every caller (`examples/validate_person.rs`, the four simplified-schema L1 files, `darkmatter/lib/tests/l1/validation_contract_parity.rs`; the other files the plan listed only contain the word in test names). Exported JSON Schema is unchanged (`required` kept; `default` stays an annotation), which the docs now state as a limit for external validators.
- **Plain-YAML edits (Rule 8).** The node-range engine moved from `markdown/hash/path_edit.rs` into crate-private `markdown/hash/node_edit.rs` behind an `Envelope` trait; `path_edit.rs` is now the Markdown adapter with unchanged behavior (including its Markdown-only serialization fallback). New public `darkmatter::yaml_edit::{YamlEdit, edit_yaml_paths, YamlEditError, YamlEditErrorKind, YamlLocation}` uses the whole-document span, never serializes the whole file, keeps BOM/CRLF/missing final newline/`---` marker, creates missing or null parents, appends with `Set` at index == length, and refuses alias/anchor/tag/flow mutations and malformed YAML with a located error (byte offset, 1-based line/column).
- **Committed trees (Rule 12, Sniff part).** New `sniff/lib/src/filesystem/git/committed_tree.rs`: `GitRepo::committed_tree() -> CommittedTreeSnapshot` (`Send + Sync`, asserted at compile time) pins commit and tree ids from the existing gix handle (not `head_id()`), reads through a `ThreadSafeRepository`, and returns `CommittedEntry::{Absent, Blob { bytes, executable }, Symlink { target }, Directory, Submodule { commit_id }}` with no filters, helpers, hooks, or subprocesses. Unborn HEAD is an empty view; a missing object is `SniffError::Git`; invalid paths are the new `SniffError::InvalidRepositoryPath`. Wiring it into Darkmatter's `RepositoryObservation`/`ContextCapture` is left for the phase that loads configuration (the Wave 3 task owned only Sniff).

### Wave 4 — Foundation integration

- **Composition defaults (Rules 6–7).** `ValidationReport::defaults` added and propagated through every constructor (no-schema, phase, YAML document, raw paths). `EffectiveSchema::validate_instance` judges the `prepare_defaults` candidate; validation APIs stay read-only (`data_copy` deliberately excludes defaults so `md set` never writes one). A refused default becomes one `ConstraintViolation` problem whose `schema_path` is the default's pointer and which has no line/column. `compose/schema_validation.rs::run_with_registry` inserts `EffectiveSchema::required_defaults` before conversion on every pass, re-matches triggers on the completed frontmatter and loops while newly matched triggers add defaults, and returns inserted paths, which the pipeline marks as data. Frontmatter interpolation pass 1 runs before the schema stage, so only pass 2, shell expansion, and the body see defaults (documented). Expression presence typing needed no code change: `PropertyDef::is_required` already treats required (including defaulted) properties as non-null; tests now pin it. `md schema validate` JSON output gained a `defaults` array (only when non-empty) and an `applied-default` output-schema type. The shipped implement-plan fixture's `phase: number(required;default(1))` is no longer reported missing at completion (`schema_phase_validation.rs` updated).
- **Locked writes and receipts (Rules 9, 34).** New `darkmatter::config` (only `write` and `migration_receipt` so far), the shared crate-private `atomic_replace.rs` (extracted from `effects/fs_write.rs`, which now calls it with identical behavior), and crate-private `strict_json.rs` (extracted from `expression/host/artifact.rs`; `parse_strict` remains as a wrapper with the same wording; the lifecycle host profile still reaches it through that wrapper). `ConfigTransaction` resolves real targets (symlinks followed; missing target = canonical nearest ancestor + remaining components), takes retained sibling `<name>.darkmatter-lock` locks via `File::try_lock` (2 s deadline, 20 ms backoff) in portable-path order, re-reads, edits through `edit_yaml_paths`, runs the caller's validator, compares expected bytes/hash, and replaces through a synced temp file preserving permissions. Receipts (`<name>.darkmatter-migration.json`, strict closed JSON) are written inside `commit` before publication; `complete_cleanup` requires the caller's exact known legacy paths and matching target/source hashes, and is idempotent.
- **Default consumers.** Claudine: unphased and completion reports no longer list defaulted properties as missing, so prompts already skip them; new `PropertyState::Defaulted` in status reports; at launch, `complete_launch_defaults` (new `claudine/lib/src/composition/schema/launch_defaults.rs`) runs one unphased validation only when a non-eager required absent property declares a default, so a refused default blocks before the agent runs. DMLS: the `$schema` declaration hover shows the default; a refused default is placed on the `$schema` declaration value (or the reference) rather than the whole frontmatter; narrowing already treats required defaults as present.

### Coordinator fixes and findings

- `claudine-cli::l1 test_placement::repository_test_placement` failed in the full Claudine run: `classify.rs` grew to 836 production lines (limit 800). Fixed by moving `complete_launch_defaults` into new `launch_defaults.rs`; the guard and the moved tests pass.
- `cargo check --workspace --all-targets` found a Phase 1 leftover: `messenger` (research feature) still imported `LoadedYamlDocument` from `darkmatter::markdown::schemas`, which Phase 1 stopped re-exporting. Fixed in `messenger/lib/src/research/load.rs` and `messenger/lib/tests/research_corpus.rs` to import `simplified_schema::yaml_documents::LoadedYamlDocument`. The workspace now compiles; `messenger` research tests (40) and `just lint` pass.
- Shipped artifacts: no regeneration needed. The default engine works on existing lowered output, so no preparation revision changed (still Simplified Schema 2, Darkmatter 3, Claudine 3); every package's `shipped_artifact_is_fresh` passes in the gate runs below.
- Unrelated local edits under `prompts/` (11 files, modified at 10:40 during this phase by someone else) are not part of this work and are excluded from the frontmatter lists.

### Requirement → test mapping

| Behavior | Tests |
| --- | --- |
| Strict borrows + reports defaults; soft inserts; borrowed when unedited | `simplified-schema::l1 required_defaults::strict_and_soft::*` |
| Explicit null, nullable, optional default, missing without default | `required_defaults::{an_explicit_null_is_present_and_judged_by_the_declared_type, a_null_admitting_property_stays_optional_and_is_not_filled, an_optional_default_is_never_inserted, a_missing_required_property_without_a_default_is_still_missing}` |
| Nested parents, arrays, recursion, escaped paths | `required_defaults::nesting::*`, unit `defaults::tests::*` |
| Invalid defaults (type, constraint, nested required child) | `required_defaults::invalid_defaults::*` |
| Pending vs resolved slot defaults | `required_defaults::slots::*` |
| Unions: ambiguity, equal candidates, discriminants, sibling assertions | `required_defaults::unions::*` |
| Export keeps `required` | `required_defaults::exported_json_schema_keeps_the_required_assertion` |
| Library/Darkmatter verdict parity | `darkmatter::l1 validation_contract_parity::*` |
| Composition: body, pass-2, source untouched, null, optional, nested, triggers, invalid default unlocated, passive report, YAML report, presence typing | `darkmatter::l1 composition_defaults::*` (15 tests) |
| End-to-end `md compose` with a real file | `darkmatter-cli::l1 compose_schema::compose_fills_a_required_default_without_editing_the_source` |
| CLI output schema conformance | `darkmatter-cli::l1 output_commands::conforms_schema_validate_reports_the_defaults_it_relied_on` |
| YAML edits (empty roots, parents, lists, empty collections, bytes kept, BOM, CRLF, no final newline, refusals, round trip) | `darkmatter::l1 yaml_edit::*` (37 tests); Markdown regressions `frontmatter_path_edit::*`, `darkmatter-cli::l1 get_set_rm::*`, `rm::*` |
| Committed tree parity, unborn HEAD, worktree vs index vs HEAD, ref movement, linked worktree, missing objects, path validation, threads | `sniff::l1 committed_tree::*` (8 tests; parity against `git ls-tree -r -z` / `git rev-parse`) |
| Locked transactions (aliases, deadline, order, changed input, unsupported edit, validation, injected replacement failure, modes, lock retained, create, symlink) | `darkmatter::l1 config_write::*` (18 tests) |
| Receipts (interruption, unpublished, wrong paths, 18 invalid shapes, hash mismatches, idempotence) | `darkmatter::l1 config_migration_receipt::*` (18 tests) |
| Strict JSON reader | unit `strict_json::tests::*`, `host::artifact::tests::the_strict_parser_rejects_duplicate_keys_at_any_depth_and_trailing_content` |
| Claudine prompts and launch defaults | `claudine composition::schema::defaults_tests::*` (9 tests) |
| DMLS narrowing, hover, refused-default placement | `darkmatter-lsp::l1 required_defaults::*` (4 tests) |

Input robustness: the receipt reader is the one new configuration-format loader; its document shapes (duplicate key, trailing content, not JSON) and field shapes (absent, null, wrong type, wrong-type element, empty `sources`, unknown key, wrong `format_version`, `backup_path` with the wrong action) are covered by `config_migration_receipt::invalid_receipt_shapes_are_located_errors_that_keep_the_source`, each asserting the JSON pointer and that the source is kept. The YAML editor and the transaction writer are writers, not loaders. Smell grep over the new loader: no `#[serde(default)]`, `unwrap_or_default`, or `.ok()` on a load-bearing field (it parses through `strict_json` and reads fields explicitly).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` → `change_class: package`, `packages: [claudine, darkmatter, darkmatter-cli, darkmatter-lsp, messenger, simplified-schema, sniff, test-toolkit]`; gates were not skipped.
- `simplified-schema`: `just test` — 2161 passed (default) + 1805 passed (minimal), none ≥ 5 s; `just lint` clean; `just check-tier-coverage simplified-schema` — 0 stranded.
- `sniff`: `just test` — 3570 passed, 33 skipped, none ≥ 5 s; `just lint` clean; tier coverage 0 stranded.
- `darkmatter`: `just test` — 16377 passed, 10 skipped, none ≥ 5 s; `just lint` clean; `just check-tier-coverage darkmatter` — 0 stranded; `just slow-tests fresh` — fresh.
- `claudine`: `just test` — 11955 passed, 15 skipped, none ≥ 5 s (after the placement fix above); `just lint` clean.
- Dependents: `cargo check --workspace --all-targets` clean (after the `messenger` fix); `messenger` `--features research` check, research tests (40 passed), and `just lint` clean.
