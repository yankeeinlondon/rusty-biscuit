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
    - "biscuit-file"
    - "claudine-cli"
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
    - |-
      Phase 3 extended the SimplifiedSchema grammar: a `default(...)` argument starting with `[` or `{` (and not a `{{` slot) is now read as strict JSON, so schemas can declare list and mapping defaults (`default(["a"])`, `default({"k": 1})`). Before, `default([a])` was the text "[a]" and a comma split the argument, which a test pinned. The plan requires every config default (including the file-read lists, `[]`, and the `permissions` mapping) to live in the schema, which was impossible without this. `default([a])` (not valid JSON) is now a schema error; the text is still writable quoted (`default('[a]')`). Alternative: a separate list/mapping default syntax, or keep those defaults in code (forbidden by the plan). Reverting means undoing `simplified-schema/lib/src/simplified/{grammar,serialize}.rs`, the pinned test in `schemas_union_postfix.rs`, the language docs, and every config schema default.
    - |-
      Each scope's `permissions` default in `darkmatter-config.yaml` spells out all of its children's values, because the default engine judges a parent default as a whole and does not fill its children's defaults inside it (so `default({})` was refused). A unit test keeps the parent default equal to the children's defaults. Alternative: teach the engine to fill nested defaults inside a parent default (shared engine change, closer to the spec's "an empty mapping whose omitted children receive their scope's defaults"). Reverting means an engine change in `simplified-schema/lib/src/defaults.rs` and simplifying the two schema defaults.
    - |-
      Configuration-loading choices the plan left open: `@x` and `vault:x` patterns are rejected in `file-read`; at repository scope a relative pattern may not climb above its starting folder with `..` (judged lexically, no disk); unknown root keys come from the schema's closed root rather than a hard-coded list; `registrations()` covers the user file plus the repository working copy (the HEAD copy's entries are available through `repository()`); a committed `.darkmatter.yaml` that is a symlink, directory, or submodule is an error. Two small biscuit-file accessors were added so the loader does not parse references itself (`FileReference::env_variables`, `FileResolutionContext::portable_variables`). Each is local to `darkmatter/lib/src/config/load/entries.rs` or `load.rs`.
    - |-
      Claudine supplies its configuration snapshot to Darkmatter requests through a weak handle attached to each context it captures (a new `ConfigSnapshotSource` trait, following the existing context-extension pattern); `ComposeRequest::with_context` uses an explicit snapshot first, then that handle, then a passive load or the typed `ConfigSnapshotRequired` error. Threading the snapshot explicitly through every Claudine `file_resolution_context` path would have touched about 20 call sites. Claudine's local-only expression contexts (no network/shell/slots) build no request and read no config (`ComposeRequest::local_expression_context`). Reverting means explicit plumbing in `claudine/lib/src/invocation_context.rs` and its callers.
    - |-
      `md` and Claudine CLI test fixtures now write a `{}` `~/.darkmatter.yaml` into their fixture home by default (opt-out: `without_user_config()` / `without_darkmatter_config()`), so the new `dm.config.missing` warning does not change hundreds of existing stderr assertions. Claudine prints that warning for every command a person reads but not for `handle`/`steer` (machine-facing streams); completion reads no config. Alternative: update every affected assertion, or suppress the warning under a test-only switch. Reverting is local to `darkmatter/cli/tests/common/fixture.rs`, `claudine/cli/tests/common/{mod,wrap}.rs`, and `claudine/cli/src/main.rs`.
    - |-
      Claudine configuration loading choices: event and provider maps in `claudine-config.yaml` list their keys explicitly (the default engine skips values under pattern-keyed maps, so pattern keys left action/model defaults unfilled); `tts.prewarm` and `steering.automatic.enabled` default to on but are never inserted (so saving never writes them); nested serde defaults remain (only top-level ones were removed; the schema always fills values first); custom Protect patterns now require `name` and `pattern` (previously `{}` silently matched everything); duplicate keys are now errors (two old matrix cells expected last-wins); writing both `preferred_agent` and `favorite_agent` is an error; the schema is not in Claudine's shipped artifact (about 38 ms debug-build preparation once per process). Alternatives and redo cost: walking pattern keys in `simplified_schema::defaults` (moderate, shared), adding the schema to the shipped artifact (preparation revision bump), or removing nested serde defaults (about 35 tests).
    - |-
      Claudine's JSON-to-YAML conversion triggers by location: any path shaped `<dir>/.claudine/config.yaml` converts a sibling `config.json`; any other explicit path is read as YAML exactly where it is. A receipt whose cleanup fails stops the load and tells the user to remove it, rather than warning and continuing. Alternatives: trigger only for the two captured scope paths, or warn on a stuck receipt. Local to `claudine/lib/src/config/json_conversion.rs`.
    - |-
      Claudine's writers use `SourceExpectation::Current` (take whatever the file holds under the lock) rather than failing when the file changed since it was opened; this is what lets two sessions changing different keys both succeed, as the plan requires. The config TUI no longer deletes an empty repository file (that would destroy `permissions` and comments); removing the last override leaves `{}`. A cleared key that the file never wrote (its value was a default) is written as `null`. Setup manages "values that differ from the defaults", so re-running it overwrites a managed value the user changed since. An edited key loses its original quoting. Darkmatter's `yaml_edit` now turns a `{}` root into a block on `Set` and leaves `{}` after removing the last top-level key (previously refused / left an empty file). Alternatives: exact-byte expectation with retry, deleting empty repository files, refusing flow roots on the Claudine side. Local to `claudine/lib/src/config/persist.rs`, `claudine/cli/src/commands/config_tui/`, and `darkmatter/lib/src/yaml_edit.rs`.
    - |-
      With no usable home, Claudine's long-standing fallback configuration path is the relative `~/.claudine/config.yaml`; the locked writer only accepts absolute targets, so Claudine's persist module now makes a relative target absolute against the working directory (`std::path::absolute`) before writing, keeping the existing pinned behavior (`home_lookup_round_trip::a_relative_home_is_no_home_and_does_not_reach_the_real_profile`). Alternative: refuse to save without a home (a behavior change). Local to `absolute_target` in `claudine/lib/src/config/persist.rs`.
    - |-
      Permission references (Phase 4): a broken `permissions` reference (missing target, no `permissions` mapping in it, a second reference, invalid YAML, a repository target outside the repository) fails only `ConfigSnapshot::policy()` with `dm.config.reference`, not the whole configuration load, so registrations and other settings still load. A referenced block that omits a list takes the defaults of the scope that refers to it. Alternatives: fail the whole load (stricter, blocks unrelated commands), or apply the target file's own scope defaults. Redo is local to `darkmatter/lib/src/config/load.rs` and `load/reference.rs`.
    - |-
      Saved file approvals ("always allow" for a schema read) are written only as an escaped `~/…` exact path; a file outside the home is never saved (it is allowed for the run and reported `dm.config.not_saved`), because repository-relative or `{{VAR}}` spellings would need extra rules for which scope owns them. Saves follow the user `permissions` reference as captured at startup and require the block file's bytes to be unchanged since they were read (a concurrent edit gives not-saved rather than a retry). Alternatives: save repository-relative entries, or retry under the lock. Local to `darkmatter/lib/src/config/permissions/persist.rs`.
    - |-
      `network: allow` turns on read-side expression URL reads (`load_data(url)`) by itself, but `::file`/`::code` URL transclusion still also requires the library caller's `with_allow_remote_transclusion(true)`; `md compose` sets it, Claudine does not, so Claudine documents get URL reads in expressions but not URL transclusion. Redirects remain unfollowed under `allow`. Lifecycle HTTP writes (`EffectEngine::http_post`) still use only per-run exact hosts. Alternatives: make `allow` imply remote transclusion everywhere, or extend it to lifecycle writes. Redo is in `ComposeOptions::remote_reads_enabled` and Claudine's request preparation.
    - |-
      Legacy-policy migration choices: an invalid legacy line, an unreadable legacy file, or a malformed/unknown receipt stops the load with a located `dm.config.migration` error (nothing written); a failed save, or a source edited after its receipt was written, is a notice and the sources are kept (the second repeats every run until the file is removed by hand, rather than blocking `md`, which Rule 34's wording could read as an error). A `permissions` reference is never migrated into (notice instead). A whitelist with only comments still writes `shell-commands: []`. The passive hint says "the next `md` or Claudine run migrates it". Repository legacy files migrate when `md` projects its configuration onto a source's repository (`ConfigSnapshot::with_repository_projection`), and the migrated repository entries only take effect once committed (commit rule), which the notice says. Alternatives: errors for every refusal, migrate into referenced targets, migrate repository files only at startup. Local to `darkmatter/lib/src/config/migration.rs` and `load.rs`.
    - |-
      Shell decisions: a command preflight pre-approved is not re-checked against exclusions at run time (approve-once of an excluded command must keep working); a filled template value that an exclusion matches needs its own approval unless this run approved that shape explicitly, which is conservative when preflight approved a different, non-excluded shape by prompt. A rule saved by "always allow" never overrides an exclusion; the approved command still runs this run through its approve-once key. `ApprovalKey` lost its policy-root component, and the shell working directory no longer falls back to the old policy root (explicit working directory, else the document's folder, else the request directory). The compose-cache fingerprint now includes the shell rules. Alternatives: re-check exclusions at run time (breaks approve-once), record explicit approvals in `ShellPreApprovals` (small API change). Local to `darkmatter/lib/src/markdown/compose/shell_expansion/`.
    - |-
      Schema-slot decisions: the retired grant kinds (`GrantTarget::Dir`, `UrlAnyQuery`, `Site`) stay in the type only because the temporary migration parses legacy lines into it; the gate ignores them. An "always allow" chosen during pre-flight is saved when the slot session receives it, with its exclusion check judged from the request's document rather than the schema file (reads themselves are always judged from the reading document). A URL read offers only "allow once" and "deny for this run". Alternatives: a separate legacy target type (small), or carrying the reading context through `ComposePreflightApprovals` (medium). Local to `darkmatter/lib/src/markdown/schemas/slots/`.
    - |-
      Claudine permissions: `CapturedPermissions::Absent` exists for library callers that capture no configuration (pre-approves and saves nothing); every CLI route supplies a policy or its error. Claudine's shell approval cache is keyed by mode only, so a decision in one document answers for the rest of the invocation (it was per policy root). DMLS's passive shell classification judges exact and prefix rules but not `template` rules, so a `{{ }}` command shows "unknown" where `md` pre-approves it by shape (the parity table records this). Alternatives: require a captured policy (type change plus test churn), per-document caching, template matching in DMLS (small). Local to `claudine/lib/src/harness/shell*` and `darkmatter/dmls/src/overlay/shell.rs`.
    - |-
      Setup handshake: there is no production `claudine init` command any more (replaced by the configuration editor earlier), so the handshake runs in Claudine's first-run setup, which any Claudine command triggers when no Claudine configuration exists (help included). Pointing a `.darkmatter.yaml` with no block of its own at Claudine's file is silent there; moving an authored inline block is reported; a conflict, an invalid `.darkmatter.yaml`, or a failed write is a warning and the run continues with the original block in force. Claudine's block as it was before its own first write decides "equal or absent" (a fresh Claudine file always holds the default block). `md init` adds only missing top-level settings (a partial `permissions` block is not filled child by child), `--inline` copies the target's block exactly as written, an invalid existing file is reported and left alone, and `md init` does not report `dm.config.missing` for the file it creates. Setup errors use a new `InitError` type (`dm.config.not_saved` except an invalid file's own code). Alternatives: run the handshake only from an explicit command, fill partial blocks, fail Claudine on a handshake conflict. Local to `darkmatter/lib/src/config/init.rs`, `claudine/lib/src/config/setup.rs`, and `claudine/cli/src/commands/init_wizard.rs`.
    - |-
      Host identity (Phase 5): the identifying-trigger proof refuses an expression it cannot prove instead of attempting satisfiability, as the spec asks. Two readings were mine: (1) `none:` conditions keep ordinary matching inside identification, and a condition under an even number of `none:` counts as positive again; (2) a guard (no `required`) keeps ordinary matching, so a wrongly typed guard value defeats identification while a wrongly typed required host key still identifies. The library's refusal text says the key is owned by "the caller" rather than naming Darkmatter. Alternative: treat every property under identification as presence-only. Reversing is local to `simplified-schema/lib/src/triggers/identify.rs` and its 15 tests.
    - |-
      Same-host legacy suppression (Phase 5, plan Rule 26): a registered host whose identifying trigger did not match is removed from the legacy root-catalog fallback. A root catalog is excluded when its lifecycle artifact names a registered host or when it is a registered profile's own catalog file, and function catalogs in the same folder as an excluded catalog are excluded too, because function catalogs carry no host name. A root catalog without a usable lifecycle artifact cannot be tied to a host and is kept. This reads the spec's broad "compatibility fallback" wording narrowly so that registered globals and events apply only to identified documents; it is tested outside and inside a package area that holds the host's catalog. Alternative: keep every root catalog as fallback (spec wording read literally), which makes shared-event-only documents in `claudine/` foreign again. Building the legacy view now reads every lifecycle artifact in the roots instead of stopping at the first usable one. Reversing touches `config/registration/selection.rs` and two `host_selection` tests.
    - |-
      Registration scope switches (Phase 5): `--disable schema-triggers` also turns off the registered layer (registration is a trigger source), and registration applies outside a repository too, whereas root trigger discovery does not. Alternatives: a separate switch for registration, or no registration outside a repository. Reversing is a condition in the md CLI's compose/validate/clean wiring and `schema_validation.rs`.
    - |-
      Registered-layer cache identity (Phase 5): each registered trigger layer's identity is its payload digest combined with the manifest path, content hash, and host revision, built from existing Simplified Schema identity APIs; no new `LayerRole` was added. The trigger merge memo used to match by `Arc` identity alone and returned a stale merge when the payload memo shared one `Arc` across host revisions; it now also compares layer digests (merge output unchanged, so no revision bump). A root trigger counts as identical to a registered one when its canonical path, envelope, and payload JSON are equal (parsed values, not bytes). Alternative: a distinct layer role and byte-level identity; more library churn for the same verdicts.
    - |-
      Passive repository configuration for inspection commands (Phase 5): loading registrations for `md schema validate`, `md clean`, and `md schema triggers` loaded the repository configuration in migrating mode, and a CLI test run inside the checkout migrated the real repository root (restored immediately). These commands now read the repository projection passively (`ConfigSnapshot::with_passive_repository_projection`); `md compose` keeps its existing migrating load. Alternative: let every md command migrate as compose does. A regression test guards it.
    - |-
      Foreign-document compose (Phase 5, plan Rule 27): only host event keys (every event in the host's lifecycle registry) are withheld from compose-time interpolation and execution; host data keys such as `sequence` are still interpolated, because the manifest and lifecycle artifact carry no list of host-owned data keys. A Claudine document whose `sequence` or body reads `{{ seq.… }}` therefore fails normally under `md compose`, as the spec allows for unavailable host values. `composed`/`failed` are not withheld, so the host schema rejects them; because problems under withheld keys are dropped before reporting, a wrongly typed host event (`start: 5`) is not reported by `md compose` (DMLS and `md schema validate` still judge it). Alternatives: a "deferred keys" field in the manifest or lifecycle artifact (format change, moderate work), or keeping withheld-key problems in the report.
    - |-
      Foreign-document notice (Phase 5): `dm.host.foreign_document` depends on the root document only: a foreign child adds no second notice, and an md root that transcludes a foreign child gets none. Host globals are declared to analysis as unavailable with reason `dm.host.runtime_only`, so a shared stack reading `seq.name` fails with md's ordinary unavailable-global error. Host function sets are installed for typing on the composing thread only; a child evaluated on another thread reports an unknown function instead of a missing implementation. Info-severity run notices now print as info rows rather than warning rows on stderr, and `CommandRun`/`ReportSink` also collect notices raised during a run (previously only at its start). Alternatives: a notice per foreign document in the request; propagating host functions to worker threads.
message_to_agent: |-
    Phase 5 landed registration in the library and md CLI; DMLS is untouched (Phase 6). Read `darkmatter/docs/topics/host-registration.md` first. API: `darkmatter::config::registration::{load_registrations, load_entries, md_owned_names, agreement, RegisteredHosts, select, HostSelection { Md, Registered, Legacy, Ambiguous }, Selected, LegacyHost, RegisteredHostProfile}`; `LegacyHost` is the ONE central legacy view (root `host_catalogs`/`host_function_catalogs`/lifecycle profiles with registered hosts suppressed) and should replace DMLS's `LifecycleHost::first_usable` and the independent `providers/dsl.rs::host_catalogs` rescans. `DarkmatterSchemas::{with_registered_hosts, host_selection_for, registered_selection_for (reads nothing), registered_trace}`; `ComposeOptions::{with_registered_hosts, host_selection_for, with_selected_host}`; `RegisteredHostProfile::{dependencies, revision, analysis_globals}` give the files to watch and the revision for `AnalysisIdentity`. `ConfigSnapshot::with_passive_repository_projection` reads the repository config without migrating: DMLS must use passive loads only. Test fixtures: `darkmatter/lib/tests/host_fixtures/{mod.rs,recorder.rs}` (acme host, Claudine-like `relay` host with renames, in-process lifecycle recorder); the CLI suite includes them and lists them in `source-inputs`. Caution: never run md in a test with the real checkout as its repository; compose migrates legacy repository policy files. No mid-phase commits appeared during Phase 5; the host load average was 8-60 from unrelated work.
source_files_during_phase_3:
    - "biscuit-file/lib/src/file_reference/context.rs"
    - "biscuit-file/lib/src/file_reference/mod.rs"
    - "biscuit-file/lib/src/file_reference/portable/mod.rs"
    - "claudine/cli/src/commands/actions.rs"
    - "claudine/cli/src/commands/compose/prep.rs"
    - "claudine/cli/src/commands/config_tui/app.rs"
    - "claudine/cli/src/commands/config_tui/mod.rs"
    - "claudine/cli/src/commands/config_tui/persistence.rs"
    - "claudine/cli/src/commands/config_tui/persistence_tests.rs"
    - "claudine/cli/src/commands/config_tui/tabs/tts.rs"
    - "claudine/cli/src/commands/hooks/list.rs"
    - "claudine/cli/src/commands/init/mod.rs"
    - "claudine/cli/src/commands/init/prompts.rs"
    - "claudine/cli/src/commands/init_wizard.rs"
    - "claudine/cli/src/commands/link_display.rs"
    - "claudine/cli/src/commands/sequence.rs"
    - "claudine/cli/src/commands/uninstall.rs"
    - "claudine/cli/src/commands/wrap/composition/tests.rs"
    - "claudine/cli/src/commands/wrap/env/mod.rs"
    - "claudine/cli/src/commands/wrap/runaway_guard.rs"
    - "claudine/cli/src/completion/engine/mod.rs"
    - "claudine/cli/src/completion/engine/tests.rs"
    - "claudine/cli/src/main.rs"
    - "claudine/cli/src/request.rs"
    - "claudine/cli/tests/common/mod.rs"
    - "claudine/cli/tests/common/pty.rs"
    - "claudine/cli/tests/common/review_screen.rs"
    - "claudine/cli/tests/common/webhook_listener.rs"
    - "claudine/cli/tests/common/wrap.rs"
    - "claudine/cli/tests/l1/agent_cwd.rs"
    - "claudine/cli/tests/l1/argv_normalization.rs"
    - "claudine/cli/tests/l1/command_routing.rs"
    - "claudine/cli/tests/l1/completion_ownership.rs"
    - "claudine/cli/tests/l1/completion_perf.rs"
    - "claudine/cli/tests/l1/config_yaml_wiring.rs"
    - "claudine/cli/tests/l1/contamination_probes.rs"
    - "claudine/cli/tests/l1/darkmatter_config_gate.rs"
    - "claudine/cli/tests/l1/detached_audio.rs"
    - "claudine/cli/tests/l1/handle_blocking_output.rs"
    - "claudine/cli/tests/l1/handle_repo_config.rs"
    - "claudine/cli/tests/l1/home_lookup_round_trip.rs"
    - "claudine/cli/tests/l1/level1_compose_autocomplete_failure_pty.rs"
    - "claudine/cli/tests/l1/level1_dry_run_pty.rs"
    - "claudine/cli/tests/l1/level1_inline_compose_mismatch_pty.rs"
    - "claudine/cli/tests/l1/level1_lifecycle_shell_terminal_pty.rs"
    - "claudine/cli/tests/l1/main.rs"
    - "claudine/cli/tests/l1/pi_managed_rpc.rs"
    - "claudine/cli/tests/l1/protect_cli.rs"
    - "claudine/cli/tests/l1/skills_integration.rs"
    - "claudine/cli/tests/l1/tts_prewarm.rs"
    - "claudine/cli/tests/l1/wrap_compose_agent.rs"
    - "claudine/cli/tests/l1/wrap_compose_preflight.rs"
    - "claudine/cli/tests/l1/wrap_ctrl_c_windows.rs"
    - "claudine/cli/tests/level2/level2_auto_complete_chooser.rs"
    - "claudine/cli/tests/level2/level2_auto_complete_operation_file.rs"
    - "claudine/cli/tests/level2/level2_dry_run_approval_capture.rs"
    - "claudine/cli/tests/level2/level2_dry_run_metadata_capture.rs"
    - "claudine/cli/tests/level2/level2_explicit_operation_file_miss.rs"
    - "claudine/cli/tests/level2/level2_file_resolution_capture.rs"
    - "claudine/cli/tests/level2/level2_inline_compose_mismatch_capture.rs"
    - "claudine/cli/tests/level2/level2_invalid_file_reference_capture.rs"
    - "claudine/cli/tests/level2/level2_lifecycle_control.rs"
    - "claudine/cli/tests/level2/level2_malformed_frontmatter_capture.rs"
    - "claudine/cli/tests/level2/level2_perf_capture.rs"
    - "claudine/cli/tests/level2/level2_prompt_reporting_capture.rs"
    - "claudine/cli/tests/level2/level2_removed_validation_key_capture.rs"
    - "claudine/cli/tests/level2/level2_schema_parse_capture.rs"
    - "claudine/cli/tests/level2/level2_sequence_container_ctrl_c.rs"
    - "claudine/cli/tests/level2/level2_sequence_task_stream_capture.rs"
    - "claudine/cli/tests/level2/level2_typed_error_render_capture.rs"
    - "claudine/docs/providers/dispatch-inventory.json"
    - "claudine/lib/benches/runtime_hot_paths.rs"
    - "claudine/lib/generated/shipped.json"
    - "claudine/lib/src/composition/error/tests.rs"
    - "claudine/lib/src/composition/lifecycle/executor.rs"
    - "claudine/lib/src/composition/lifecycle/prewarm/worker.rs"
    - "claudine/lib/src/composition/lifecycle/tests/tts_settings.rs"
    - "claudine/lib/src/composition/mod.rs"
    - "claudine/lib/src/composition/prepare.rs"
    - "claudine/lib/src/composition/sequence/task/mod.rs"
    - "claudine/lib/src/config/claudine_config.rs"
    - "claudine/lib/src/config/claudine_config/tests.rs"
    - "claudine/lib/src/config/json_conversion.rs"
    - "claudine/lib/src/config/load.rs"
    - "claudine/lib/src/config/messaging_block/tests.rs"
    - "claudine/lib/src/config/mod.rs"
    - "claudine/lib/src/config/persist.rs"
    - "claudine/lib/src/config/persist_tests.rs"
    - "claudine/lib/src/config/schema.rs"
    - "claudine/lib/src/config/schema_tests.rs"
    - "claudine/lib/src/config/tts.rs"
    - "claudine/lib/src/dispatch/deps.rs"
    - "claudine/lib/src/dispatch/loader.rs"
    - "claudine/lib/src/dispatch/loader/tests.rs"
    - "claudine/lib/src/dispatch/runner/speak.rs"
    - "claudine/lib/src/dispatch/runner/tests.rs"
    - "claudine/lib/src/dispatch/tests.rs"
    - "claudine/lib/src/error.rs"
    - "claudine/lib/src/events/config.rs"
    - "claudine/lib/src/invocation_context.rs"
    - "claudine/lib/src/messaging/config.rs"
    - "claudine/lib/src/signals/harvest.rs"
    - "claudine/lib/src/steering/automatic.rs"
    - "claudine/lib/src/steering/automatic/tests.rs"
    - "claudine/lib/src/system_prompt/prepare.rs"
    - "claudine/lib/tests/config/repository-control.yaml"
    - "claudine/lib/tests/config/user-control.json5"
    - "claudine/lib/tests/config/user-control.yaml"
    - "claudine/lib/tests/l1/claudine_schemas_embedded.rs"
    - "claudine/lib/tests/l1/config_json_conversion.rs"
    - "claudine/lib/tests/l1/config_schema_matrix.rs"
    - "claudine/lib/tests/l1/main.rs"
    - "claudine/lib/tests/l1/path_lookup_guard.rs"
    - "claudine/schemas/claudine-config.yaml"
    - "darkmatter/cli/src/commands/compose.rs"
    - "darkmatter/cli/src/commands/graph.rs"
    - "darkmatter/cli/src/commands/validate.rs"
    - "darkmatter/cli/src/main.rs"
    - "darkmatter/cli/src/output/command.rs"
    - "darkmatter/cli/src/output/mod.rs"
    - "darkmatter/cli/src/output/report.rs"
    - "darkmatter/cli/src/request.rs"
    - "darkmatter/cli/tests/common/fixture.rs"
    - "darkmatter/cli/tests/l1/config_notices.rs"
    - "darkmatter/cli/tests/l1/main.rs"
    - "darkmatter/lib/generated/shipped.json"
    - "darkmatter/lib/src/config/load.rs"
    - "darkmatter/lib/src/config/load/entries.rs"
    - "darkmatter/lib/src/config/load/error.rs"
    - "darkmatter/lib/src/config/mod.rs"
    - "darkmatter/lib/src/config/schema.rs"
    - "darkmatter/lib/src/lifecycle/executor.rs"
    - "darkmatter/lib/src/lifecycle/executor/shell.rs"
    - "darkmatter/lib/src/lifecycle/executor/tests.rs"
    - "darkmatter/lib/src/lifecycle/md_host.rs"
    - "darkmatter/lib/src/markdown/compose/context/capture/mod.rs"
    - "darkmatter/lib/src/markdown/compose/context/capture/observations.rs"
    - "darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs"
    - "darkmatter/lib/src/markdown/compose/context/options.rs"
    - "darkmatter/lib/src/markdown/compose/context/repository_scope.rs"
    - "darkmatter/lib/src/markdown/compose/context/request.rs"
    - "darkmatter/lib/src/markdown/compose/context/runtime.rs"
    - "darkmatter/lib/src/markdown/compose/mod.rs"
    - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/store.rs"
    - "darkmatter/lib/src/markdown/schemas/embedded.rs"
    - "darkmatter/lib/src/markdown/schemas/shipped.rs"
    - "darkmatter/lib/src/yaml_edit.rs"
    - "darkmatter/lib/tests/l1/config_fields.rs"
    - "darkmatter/lib/tests/l1/config_load.rs"
    - "darkmatter/lib/tests/l1/context_construction_guard.rs"
    - "darkmatter/lib/tests/l1/empty_package_area.rs"
    - "darkmatter/lib/tests/l1/expression_recovery.rs"
    - "darkmatter/lib/tests/l1/file_tree_roots.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/lib/tests/l1/path_function_contracts.rs"
    - "darkmatter/lib/tests/l1/reference_integration.rs"
    - "darkmatter/lib/tests/l1/request_config.rs"
    - "darkmatter/lib/tests/l1/request_context_builder.rs"
    - "darkmatter/lib/tests/l1/schemas_union_postfix.rs"
    - "darkmatter/lib/tests/l1/unknown_identifier_warning.rs"
    - "darkmatter/lib/tests/l1/yaml_edit.rs"
    - "darkmatter/lib/tests/request_support/mod.rs"
    - "darkmatter/schemas/darkmatter-config.yaml"
    - "simplified-schema/lib/src/embedded.rs"
    - "simplified-schema/lib/src/simplified/grammar.rs"
    - "simplified-schema/lib/src/simplified/serialize.rs"
    - "simplified-schema/lib/src/triggers/assemble.rs"
    - "simplified-schema/lib/src/triggers/discovery.rs"
    - "sniff/lib/src/filesystem/mod.rs"
docs_updated_during_phase_3:
    - "biscuit-file/docs/topics/file-references.md"
    - "claudine/README.md"
    - "claudine/docs/pipeline.md"
    - "claudine/docs/topics/automatic-steering.md"
    - "claudine/docs/topics/completions/shell-completions.md"
    - "claudine/docs/topics/composition.md"
    - "claudine/docs/topics/configuring-actions.md"
    - "claudine/docs/topics/execution-flow.md"
    - "claudine/docs/topics/file-referencing.md"
    - "claudine/docs/topics/flow-control/lifecycle.md"
    - "claudine/docs/topics/protect-service.md"
    - "claudine/schemas/README.md"
    - "darkmatter/README.md"
    - "darkmatter/docs/cli/data-contracts.md"
    - "darkmatter/docs/cli/output-and-width.md"
    - "darkmatter/docs/topics/compose-requests.md"
    - "darkmatter/docs/topics/yaml-editing.md"
    - "simplified-schema/docs/topics/language-core.md"
    - "simplified-schema/docs/topics/language/objects-and-named-types.md"
    - "simplified-schema/docs/topics/language/types.md"
    - "simplified-schema/docs/topics/language/unions.md"
docs_created_during_phase_3:
    - "claudine/docs/topics/configuration.md"
    - "darkmatter/docs/topics/configuration.md"
skills_files_updated_during_phase_3:
    - ".claude/skills/claudine/SKILL.md"
    - ".claude/skills/claudine/architecture.md"
    - ".claude/skills/claudine/cli-reference.md"
    - ".claude/skills/claudine/hook-actions.md"
    - ".claude/skills/claudine/unified-hooks.md"
    - ".claude/skills/darkmatter/SKILL.md"
    - ".claude/skills/darkmatter/frontmatter.md"
    - ".claude/skills/darkmatter/library-surfaces.md"
    - ".claude/skills/darkmatter/requests.md"
    - ".claude/skills/simplified-schema/SKILL.md"
source_files_during_phase_4:
    - "biscuit-file/lib/src/file_reference/fetch.rs"
    - "biscuit-file/lib/tests/l1-fetch/fetch_integration.rs"
    - "claudine/cli/src/commands/compose/prep.rs"
    - "claudine/cli/src/commands/init_wizard.rs"
    - "claudine/cli/src/commands/init/mod.rs"
    - "claudine/cli/src/commands/sequence.rs"
    - "claudine/cli/src/commands/wrap/composition/pipeline.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/mod.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/shell_options.rs"
    - "claudine/cli/src/commands/wrap/harness_orch/types.rs"
    - "claudine/cli/src/commands/wrap/mod.rs"
    - "claudine/cli/src/commands/wrap/sequence/jit.rs"
    - "claudine/cli/src/commands/wrap/sequence/jit/tests.rs"
    - "claudine/cli/src/commands/wrap/sequence/mod.rs"
    - "claudine/cli/src/commands/wrap/sequence/task_run.rs"
    - "claudine/cli/src/commands/wrap/wrapper_stages.rs"
    - "claudine/cli/src/completion/schema_completion/mod.rs"
    - "claudine/cli/src/completion/schema_completion/tests.rs"
    - "claudine/cli/src/output/error_walker/tests.rs"
    - "claudine/cli/tests/common/mod.rs"
    - "claudine/cli/tests/l1/completion_committed_prompt_schema.rs"
    - "claudine/cli/tests/l1/compose_initialize_staged_boot.rs"
    - "claudine/cli/tests/l1/compose_schema_cli.rs"
    - "claudine/cli/tests/l1/contextual_errors.rs"
    - "claudine/cli/tests/l1/ctx_launch_anchor.rs"
    - "claudine/cli/tests/l1/ctx_per_run.rs"
    - "claudine/cli/tests/l1/darkmatter_config_gate.rs"
    - "claudine/cli/tests/l1/error_guards/transport-allow.toml"
    - "claudine/cli/tests/l1/guarded_shell_values.rs"
    - "claudine/cli/tests/l1/implement_plan_commit_message.rs"
    - "claudine/cli/tests/l1/inline_compose_indented_frontmatter.rs"
    - "claudine/cli/tests/l1/level1_approval_order_pty.rs"
    - "claudine/cli/tests/l1/level1_dry_run_pty.rs"
    - "claudine/cli/tests/l1/lifecycle_set_shell_values.rs"
    - "claudine/cli/tests/l1/lifecycle_shell_templates.rs"
    - "claudine/cli/tests/l1/lifecycle_traces.rs"
    - "claudine/cli/tests/l1/preflight_execution_parity.rs"
    - "claudine/cli/tests/l1/sequence_cli.rs"
    - "claudine/cli/tests/l1/sequence_jit.rs"
    - "claudine/cli/tests/l1/sequence_referenced_state.rs"
    - "claudine/cli/tests/l1/sequence_schema.rs"
    - "claudine/cli/tests/l1/sequence_shell_templates.rs"
    - "claudine/cli/tests/l1/shell_approval_prompt.rs"
    - "claudine/cli/tests/l1/shell_missing_values.rs"
    - "claudine/cli/tests/l1/wrap_compose_preflight.rs"
    - "claudine/cli/tests/level2/level2_dry_run_approval_capture.rs"
    - "claudine/cli/tests/level2/level2_lifecycle_control.rs"
    - "claudine/cli/tests/prompts/pr_flow_rehearsal.rs"
    - "claudine/lib/Cargo.toml"
    - "claudine/lib/src/composition/coordinator/tests.rs"
    - "claudine/lib/src/composition/lifecycle/executor.rs"
    - "claudine/lib/src/composition/lifecycle/executor/tests/guarded_shell_sites.rs"
    - "claudine/lib/src/composition/lifecycle/executor/tests/scope_site_conformance.rs"
    - "claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs"
    - "claudine/lib/src/composition/lifecycle/executor/tests/shell_templates.rs"
    - "claudine/lib/src/composition/preflight.rs"
    - "claudine/lib/src/composition/preflight/tests.rs"
    - "claudine/lib/src/composition/prepare.rs"
    - "claudine/lib/src/composition/prepare/bootstrap/tests.rs"
    - "claudine/lib/src/composition/schema/classify.rs"
    - "claudine/lib/src/composition/schema/mod.rs"
    - "claudine/lib/src/composition/schema/tests.rs"
    - "claudine/lib/src/composition/schema/translate.rs"
    - "claudine/lib/src/composition/sequence/preflight/mod.rs"
    - "claudine/lib/src/composition/sequence/preflight/tests.rs"
    - "claudine/lib/src/composition/sequence/task/sequence.rs"
    - "claudine/lib/src/composition/sequence/task/templates.rs"
    - "claudine/lib/src/composition/sequence/task/tests.rs"
    - "claudine/lib/src/config/mod.rs"
    - "claudine/lib/src/config/setup.rs"
    - "claudine/lib/src/harness/audit.rs"
    - "claudine/lib/src/harness/error.rs"
    - "claudine/lib/src/harness/mod.rs"
    - "claudine/lib/src/harness/shell.rs"
    - "claudine/lib/src/harness/shell/policy.rs"
    - "claudine/lib/src/harness/shell/tests.rs"
    - "claudine/lib/src/invocation_context/document_tests.rs"
    - "claudine/lib/src/system_prompt/prepare.rs"
    - "claudine/lib/src/system_prompt/prepare/tests.rs"
    - "claudine/lib/tests/l1/darkmatter_setup.rs"
    - "claudine/lib/tests/l1/main.rs"
    - "claudine/lib/tests/l1/shell_permission_parity.rs"
    - "darkmatter/cli/schemas/output/clean.yaml"
    - "darkmatter/cli/schemas/output/code-block.yaml"
    - "darkmatter/cli/schemas/output/compose.yaml"
    - "darkmatter/cli/schemas/output/delta.yaml"
    - "darkmatter/cli/schemas/output/edit.yaml"
    - "darkmatter/cli/schemas/output/get.yaml"
    - "darkmatter/cli/schemas/output/graph.yaml"
    - "darkmatter/cli/schemas/output/hash.yaml"
    - "darkmatter/cli/schemas/output/init.yaml"
    - "darkmatter/cli/schemas/output/list-themes.yaml"
    - "darkmatter/cli/schemas/output/render.yaml"
    - "darkmatter/cli/schemas/output/rm.yaml"
    - "darkmatter/cli/schemas/output/schema-about.yaml"
    - "darkmatter/cli/schemas/output/schema-detect.yaml"
    - "darkmatter/cli/schemas/output/schema-triggers.yaml"
    - "darkmatter/cli/schemas/output/schema-validate.yaml"
    - "darkmatter/cli/schemas/output/set.yaml"
    - "darkmatter/cli/schemas/output/toc.yaml"
    - "darkmatter/cli/schemas/output/validate-refs.yaml"
    - "darkmatter/cli/src/approval.rs"
    - "darkmatter/cli/src/args/cli.rs"
    - "darkmatter/cli/src/args/command.rs"
    - "darkmatter/cli/src/args/output.rs"
    - "darkmatter/cli/src/commands/clean.rs"
    - "darkmatter/cli/src/commands/compose.rs"
    - "darkmatter/cli/src/commands/compose/lifecycle.rs"
    - "darkmatter/cli/src/commands/init.rs"
    - "darkmatter/cli/src/commands/mod.rs"
    - "darkmatter/cli/src/commands/schema/validate.rs"
    - "darkmatter/cli/src/main.rs"
    - "darkmatter/cli/src/request.rs"
    - "darkmatter/cli/tests/common/fixture.rs"
    - "darkmatter/cli/tests/l1/approval_prompt.rs"
    - "darkmatter/cli/tests/l1/collection_blocks.rs"
    - "darkmatter/cli/tests/l1/compose_basic.rs"
    - "darkmatter/cli/tests/l1/compose_data_output.rs"
    - "darkmatter/cli/tests/l1/compose_lifecycle_approval.rs"
    - "darkmatter/cli/tests/l1/compose_lifecycle.rs"
    - "darkmatter/cli/tests/l1/compose_nested_markdown.rs"
    - "darkmatter/cli/tests/l1/compose_schema_enhancements.rs"
    - "darkmatter/cli/tests/l1/compose_shell_template.rs"
    - "darkmatter/cli/tests/l1/compose_shell.rs"
    - "darkmatter/cli/tests/l1/compose_transclusion.rs"
    - "darkmatter/cli/tests/l1/dynamic_schema_slot_template_types.rs"
    - "darkmatter/cli/tests/l1/dynamic_schema_slots.rs"
    - "darkmatter/cli/tests/l1/expression_recovery.rs"
    - "darkmatter/cli/tests/l1/init_command.rs"
    - "darkmatter/cli/tests/l1/invalid_trigger_warnings.rs"
    - "darkmatter/cli/tests/l1/lifecycle_passive_surfaces.rs"
    - "darkmatter/cli/tests/l1/load_data.rs"
    - "darkmatter/cli/tests/l1/main.rs"
    - "darkmatter/cli/tests/l1/md_process_fixture.rs"
    - "darkmatter/cli/tests/l1/output_commands.rs"
    - "darkmatter/cli/tests/l1/output_envelope.rs"
    - "darkmatter/cli/tests/l1/path_completion.rs"
    - "darkmatter/cli/tests/l1/policy_verbs.rs"
    - "darkmatter/cli/tests/l1/schema_recursive.rs"
    - "darkmatter/cli/tests/l1/schema_triggers.rs"
    - "darkmatter/cli/tests/l1/standalone_baseline.rs"
    - "darkmatter/dmls/src/config/mod.rs"
    - "darkmatter/dmls/src/diagnostics/slots.rs"
    - "darkmatter/dmls/src/overlay/shell.rs"
    - "darkmatter/dmls/src/providers/code_actions.rs"
    - "darkmatter/dmls/src/providers/dsl.rs"
    - "darkmatter/dmls/src/router.rs"
    - "darkmatter/dmls/src/slot_jobs.rs"
    - "darkmatter/dmls/tests/l1/main.rs"
    - "darkmatter/dmls/tests/l1/no_side_effects.rs"
    - "darkmatter/dmls/tests/l1/schema_slots_editor.rs"
    - "darkmatter/dmls/tests/l1/shell_permission_classification.rs"
    - "darkmatter/dmls/tests/l1/undeclared_member.rs"
    - "darkmatter/dmls/tests/l1/zed_extension_contract.rs"
    - "darkmatter/dmls/zed-dmls/extension.toml"
    - "darkmatter/lib/src/config/init.rs"
    - "darkmatter/lib/src/config/load.rs"
    - "darkmatter/lib/src/config/load/error.rs"
    - "darkmatter/lib/src/config/load/reference.rs"
    - "darkmatter/lib/src/config/migration.rs"
    - "darkmatter/lib/src/config/mod.rs"
    - "darkmatter/lib/src/config/network.rs"
    - "darkmatter/lib/src/config/permissions.rs"
    - "darkmatter/lib/src/config/permissions/persist.rs"
    - "darkmatter/lib/src/config/write.rs"
    - "darkmatter/lib/src/lifecycle/executor.rs"
    - "darkmatter/lib/src/lifecycle/executor/host.rs"
    - "darkmatter/lib/src/lifecycle/executor/shell.rs"
    - "darkmatter/lib/src/lifecycle/executor/tests.rs"
    - "darkmatter/lib/src/lifecycle/md_host.rs"
    - "darkmatter/lib/src/lifecycle/md_host/tests.rs"
    - "darkmatter/lib/src/lifecycle/preflight/candidates.rs"
    - "darkmatter/lib/src/lifecycle/shell_template.rs"
    - "darkmatter/lib/src/markdown/compose/context/options.rs"
    - "darkmatter/lib/src/markdown/compose/context/request.rs"
    - "darkmatter/lib/src/markdown/compose/expression/diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/fs.rs"
    - "darkmatter/lib/src/markdown/compose/expression/read_gate_tests.rs"
    - "darkmatter/lib/src/markdown/compose/expression/read_gate.rs"
    - "darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs"
    - "darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs"
    - "darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs"
    - "darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs"
    - "darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs"
    - "darkmatter/lib/src/markdown/compose/pipeline/mod.rs"
    - "darkmatter/lib/src/markdown/compose/policy.rs"
    - "darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs"
    - "darkmatter/lib/src/markdown/compose/preflight/mod.rs"
    - "darkmatter/lib/src/markdown/compose/remote_fetch.rs"
    - "darkmatter/lib/src/markdown/compose/remote_fetch/error.rs"
    - "darkmatter/lib/src/markdown/compose/remote.rs"
    - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs"
    - "darkmatter/lib/src/markdown/compose/shell_blocks/types.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/policy.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/site.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/store.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/template/approval.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs"
    - "darkmatter/lib/src/markdown/compose/shell_expansion/types.rs"
    - "darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs"
    - "darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs"
    - "darkmatter/lib/src/markdown/compose/tests/rendering.rs"
    - "darkmatter/lib/src/markdown/compose/tests/shell.rs"
    - "darkmatter/lib/src/markdown/compose/type_tests.rs"
    - "darkmatter/lib/src/markdown/errors/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/approval_tests.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/cache.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/codes.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/dependencies.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/evaluator.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/auto.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/legacy.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/matching.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/request.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/store.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/tests.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/host.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/session_tests.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/session.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/tests.rs"
    - "darkmatter/lib/tests/dynamic_slots_fixtures/mod.rs"
    - "darkmatter/lib/tests/fixtures/permission_parity/cases.json"
    - "darkmatter/lib/tests/l1/alias_lookup_once.rs"
    - "darkmatter/lib/tests/l1/collection_blocks/pipeline.rs"
    - "darkmatter/lib/tests/l1/collection_blocks/provenance.rs"
    - "darkmatter/lib/tests/l1/collection_discovery.rs"
    - "darkmatter/lib/tests/l1/collection_execution.rs"
    - "darkmatter/lib/tests/l1/compose_expression_failure_contract.rs"
    - "darkmatter/lib/tests/l1/compose_member_warnings.rs"
    - "darkmatter/lib/tests/l1/compose_read_set.rs"
    - "darkmatter/lib/tests/l1/composition_defaults.rs"
    - "darkmatter/lib/tests/l1/config_fields.rs"
    - "darkmatter/lib/tests/l1/config_init.rs"
    - "darkmatter/lib/tests/l1/config_load.rs"
    - "darkmatter/lib/tests/l1/config_migration.rs"
    - "darkmatter/lib/tests/l1/config_permission_persist.rs"
    - "darkmatter/lib/tests/l1/config_permissions.rs"
    - "darkmatter/lib/tests/l1/data_origin.rs"
    - "darkmatter/lib/tests/l1/diagnostic_source_provenance.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_captures.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_positions.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_precedence.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_read_order.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_schema_files.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_surfaces.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_template_triggers.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slot_template_types.rs"
    - "darkmatter/lib/tests/l1/dynamic_schema_slots.rs"
    - "darkmatter/lib/tests/l1/dynamic_slots_fixture_repos.rs"
    - "darkmatter/lib/tests/l1/error_snapshots/shell_expansion.rs"
    - "darkmatter/lib/tests/l1/expression_recovery_data.rs"
    - "darkmatter/lib/tests/l1/expression_recovery_slots.rs"
    - "darkmatter/lib/tests/l1/expression_recovery.rs"
    - "darkmatter/lib/tests/l1/fm_namespace_contract.rs"
    - "darkmatter/lib/tests/l1/frontmatter_surface_projection.rs"
    - "darkmatter/lib/tests/l1/layered_validator_identities.rs"
    - "darkmatter/lib/tests/l1/lifecycle_event_binding.rs"
    - "darkmatter/lib/tests/l1/lifecycle_passive_compose.rs"
    - "darkmatter/lib/tests/l1/literal_token.rs"
    - "darkmatter/lib/tests/l1/load_data_execution.rs"
    - "darkmatter/lib/tests/l1/load_data_options_slots.rs"
    - "darkmatter/lib/tests/l1/load_data_options.rs"
    - "darkmatter/lib/tests/l1/load_data_runtime.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/lib/tests/l1/missing_ctx_capture.rs"
    - "darkmatter/lib/tests/l1/network_permission.rs"
    - "darkmatter/lib/tests/l1/permission_parity.rs"
    - "darkmatter/lib/tests/l1/preflight_child_state_parity.rs"
    - "darkmatter/lib/tests/l1/preflight_repository_sigils.rs"
    - "darkmatter/lib/tests/l1/recursive_schema_typed_consumers.rs"
    - "darkmatter/lib/tests/l1/reference_provenance.rs"
    - "darkmatter/lib/tests/l1/required_context.rs"
    - "darkmatter/lib/tests/l1/runtime_blocking.rs"
    - "darkmatter/lib/tests/l1/schemas_runtime_parity.rs"
    - "darkmatter/lib/tests/l1/scope_site_conformance.rs"
    - "darkmatter/lib/tests/l1/semantic_results_never_persist.rs"
    - "darkmatter/lib/tests/l1/set_overlay_integration.rs"
    - "darkmatter/lib/tests/l1/shell_block_integration.rs"
    - "darkmatter/lib/tests/l1/shell_child_environment.rs"
    - "darkmatter/lib/tests/l1/shell_expansion_coordinates.rs"
    - "darkmatter/lib/tests/l1/shell_permissions.rs"
    - "darkmatter/lib/tests/l1/shell_result_values.rs"
    - "darkmatter/lib/tests/l1/shell_template_body_guards.rs"
    - "darkmatter/lib/tests/l1/shell_template_discovery.rs"
    - "darkmatter/lib/tests/l1/shell_template_execution.rs"
    - "darkmatter/lib/tests/l1/shell_template_policy.rs"
    - "darkmatter/lib/tests/l1/slot_permissions.rs"
    - "darkmatter/lib/tests/l1/slot_support.rs"
    - "darkmatter/lib/tests/l1/transcluded_shell_failure.rs"
    - "darkmatter/lib/tests/l1/unknown_identifier_warning.rs"
    - "darkmatter/lib/tests/request_support/mod.rs"
docs_updated_during_phase_4:
    - "biscuit-file/docs/topics/file-references.md"
    - "biscuit-file/README.md"
    - "claudine/docs/getting-started/index.md"
    - "claudine/docs/topics/completions/shell-completions.md"
    - "claudine/docs/topics/composition.md"
    - "claudine/docs/topics/configuration.md"
    - "claudine/docs/topics/execution-flow.md"
    - "claudine/docs/topics/flow-control/lifecycle.md"
    - "claudine/docs/topics/flow-control/sequences.md"
    - "claudine/docs/topics/pre-flight-checks.md"
    - "claudine/docs/topics/system-prompt.md"
    - "darkmatter/dmls/docs/features.md"
    - "darkmatter/dmls/zed-dmls/README.md"
    - "darkmatter/docs/cli/compose.md"
    - "darkmatter/docs/cli/data-contracts.md"
    - "darkmatter/docs/cli/graph.md"
    - "darkmatter/docs/cli/index.md"
    - "darkmatter/docs/cli/output-and-width.md"
    - "darkmatter/docs/cli/policy.md"
    - "darkmatter/docs/cli/schema.md"
    - "darkmatter/docs/cli/validate.md"
    - "darkmatter/docs/inline/filter.md"
    - "darkmatter/docs/inline/fm-shell-expansion.md"
    - "darkmatter/docs/inline/map.md"
    - "darkmatter/docs/inline/preflight-checks.md"
    - "darkmatter/docs/inline/shell-blocks.md"
    - "darkmatter/docs/inline/shell-expansion.md"
    - "darkmatter/docs/lsp/architecture.md"
    - "darkmatter/docs/lsp/configuration.md"
    - "darkmatter/docs/lsp/features.md"
    - "darkmatter/docs/topics/caching.md"
    - "darkmatter/docs/topics/config-writes.md"
    - "darkmatter/docs/topics/configuration.md"
    - "darkmatter/docs/topics/darkmatter-expressions.md"
    - "darkmatter/docs/topics/lifecycle/index.md"
    - "darkmatter/docs/topics/remote-url-references.md"
    - "darkmatter/docs/topics/schemas/authoring-schemas.md"
    - "darkmatter/docs/topics/schemas/dmls-schema-support.md"
    - "darkmatter/docs/topics/schemas/index.md"
    - "darkmatter/docs/topics/schemas/schema-slots.md"
    - "darkmatter/docs/topics/schemas/schema-triggers.md"
    - "darkmatter/docs/topics/schemas/schema-validation.md"
    - "darkmatter/README.md"
docs_created_during_phase_4:
    - "biscuit-file/docs/topics/fetch.md"
    - "darkmatter/docs/cli/init.md"
skills_files_updated_during_phase_4:
    - ".claude/skills/biscuit-file/SKILL.md"
    - ".claude/skills/claudine/cli-commands.md"
    - ".claude/skills/claudine/cli-reference.md"
    - ".claude/skills/claudine/SKILL.md"
    - ".claude/skills/darkmatter/cli.md"
    - ".claude/skills/darkmatter/compose.md"
    - ".claude/skills/darkmatter/dmls.md"
    - ".claude/skills/darkmatter/lifecycle.md"
    - ".claude/skills/darkmatter/remote-cache.md"
    - ".claude/skills/darkmatter/schema.md"
    - ".claude/skills/darkmatter/SKILL.md"
    - ".claude/skills/darkmatter/testing.md"
    - ".claude/skills/sniff/remote-and-repository.md"
source_files_during_phase_5:
    - "claudine/lib/generated/shipped.json"
    - "darkmatter/cli/Cargo.toml"
    - "darkmatter/cli/schemas/output/schema-triggers.yaml"
    - "darkmatter/cli/src/commands/clean.rs"
    - "darkmatter/cli/src/commands/compose.rs"
    - "darkmatter/cli/src/commands/compose/lifecycle.rs"
    - "darkmatter/cli/src/commands/schema/triggers.rs"
    - "darkmatter/cli/src/commands/schema/validate.rs"
    - "darkmatter/cli/src/output/command.rs"
    - "darkmatter/cli/src/output/mod.rs"
    - "darkmatter/cli/src/output/report.rs"
    - "darkmatter/cli/src/request.rs"
    - "darkmatter/cli/tests/l1/compose_foreign_documents.rs"
    - "darkmatter/cli/tests/l1/host_registration.rs"
    - "darkmatter/cli/tests/l1/main.rs"
    - "darkmatter/lib/generated/shipped.json"
    - "darkmatter/lib/src/config/load.rs"
    - "darkmatter/lib/src/config/mod.rs"
    - "darkmatter/lib/src/config/registration.rs"
    - "darkmatter/lib/src/config/registration/manifest.rs"
    - "darkmatter/lib/src/config/registration/selection.rs"
    - "darkmatter/lib/src/lifecycle/host_profile.rs"
    - "darkmatter/lib/src/lifecycle/md_host.rs"
    - "darkmatter/lib/src/markdown/compose/context/options.rs"
    - "darkmatter/lib/src/markdown/compose/expression/host/artifact.rs"
    - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/schema_validation.rs"
    - "darkmatter/lib/src/markdown/output/command/data.rs"
    - "darkmatter/lib/src/markdown/output/command/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/catalog.rs"
    - "darkmatter/lib/src/markdown/schemas/clean.rs"
    - "darkmatter/lib/src/markdown/schemas/embedded.rs"
    - "darkmatter/lib/src/markdown/schemas/mod.rs"
    - "darkmatter/lib/tests/host_fixtures/mod.rs"
    - "darkmatter/lib/tests/host_fixtures/recorder.rs"
    - "darkmatter/lib/tests/l1/foreign_document_compose.rs"
    - "darkmatter/lib/tests/l1/host_registration.rs"
    - "darkmatter/lib/tests/l1/host_selection.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/schemas/darkmatter-host.yaml"
    - "simplified-schema/lib/src/triggers/discovery.rs"
    - "simplified-schema/lib/src/triggers/identify.rs"
    - "simplified-schema/lib/src/triggers/matcher.rs"
    - "simplified-schema/lib/src/triggers/mod.rs"
    - "simplified-schema/lib/tests/l1/identifying_triggers.rs"
    - "simplified-schema/lib/tests/l1/main.rs"
docs_updated_during_phase_5:
    - "darkmatter/README.md"
    - "darkmatter/docs/cli/compose.md"
    - "darkmatter/docs/cli/data-contracts.md"
    - "darkmatter/docs/cli/schema.md"
    - "darkmatter/docs/topics/configuration.md"
    - "darkmatter/docs/topics/lifecycle/index.md"
    - "darkmatter/docs/topics/schemas/schema-triggers.md"
    - "simplified-schema/docs/topics/language-core.md"
docs_created_during_phase_5:
    - "darkmatter/docs/topics/host-registration.md"
skills_files_updated_during_phase_5:
    - ".claude/skills/darkmatter/SKILL.md"
    - ".claude/skills/darkmatter/lifecycle.md"
    - ".claude/skills/darkmatter/schema.md"
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
- Cross-OS (the phase adds file locking, atomic replacement, symlink and permission handling, and Git path handling): `just cross-check sniff --os windows` — 2508 passed, none ≥ 5 s. `just cross-check darkmatter --os windows` — all 11889 passed, but the archive-mode run recorded 112 tests ≥ 5 s (mostly `expression_*` and `schemas_literal_expression` cases). Diagnosis on `build-win` against the identical tree: two of them alone take 0.03 s; the four heaviest suites run together (242 tests) have none ≥ 5 s; and a plain `cargo nextest run -p darkmatter` of the whole package has exactly one slow test, `current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects` (> 10 s). That source-scan guard has been slow on Windows since at least 2026-11-08 (9.3–27.8 s in the host's slow-test history, and it was the only slow record on 2026-11-10), so it predates this phase; the other 111 were specific to that archive run's host conditions, not to this phase's code. (The plain run's one failure, `fs_tests::fn_filesystem::has_command_found_on_path_returns_true`, depends on the SSH shell's `PATH` and passes under the cross-check recipe.) No Windows test fails, so `os_failing` is not set; the pre-existing Windows-slow guard is left for a dedicated fix.
- The `prompts/` edits, Linux, and WSL2 were not exercised here; CI covers Linux/macOS on the pull request and the nightly covers Windows/WSL2.

## Phase 3

Phase 3 ran as foreground subagents: Wave 5's Darkmatter loader first (Claudine's schema imports its types), then the Claudine schema/loader/JSON5 converter and the request binding in parallel, then the per-key Claudine writers alone. Each subagent verified only its slice with nextest filters; the coordinator fixed the cross-agent leftovers and ran the area gates once afterwards.

### Wave 5 — Configuration contracts

- **Darkmatter configuration (Rules 10, 11).** New shipped `darkmatter/schemas/darkmatter-config.yaml` with `user-config`, `repository-config`, `permissions`, `user-permissions`, `repository-permissions` (closed roots; all lists required-with-default and non-null; scope file-read defaults live only in the schema). `ShippedSources::with_aliases` (simplified-schema `embedded.rs`; aliases count in `PartialEq` and preparation identity) maps `darkmatter:config` to that document, so `permissions@darkmatter:config` resolves. New `darkmatter::config::{schema, load}` (`load/entries.rs`, `load/error.rs`): `ConfigSnapshot::load(context, Option<&CommittedTreeSnapshot>, ConfigLoadMode)` reads the user file (home from the context), the repository working copy, and the HEAD copy from the committed tree; strict admitted-YAML loading, schema-only validation, defaults through the shared engine with authored presence kept (`Setting<T> { value, authored }`), dependency records (path, revision, BLAKE3 or absent), located `ConfigError`/`ConfigProblem` (file, revision, pointer, line, column), per-setting outcomes (`permissions()` is a `Result`; invalid `register` entries become findings and only they are omitted), and code constants `dm.config.{invalid,reference,not_saved,migration,missing}`. File-read entries are validated as `GlobReference`s against the portable-variable catalog (new `FileReference::env_variables()` and `FileResolutionContext::portable_variables()` in biscuit-file); shell entries through the single-entry parser extracted from `shell_expansion/store.rs` (`parse_rule_entry`, same behavior). `ConfigLoadMode::MigrateLegacy` currently behaves exactly like `Passive`; the legacy-policy migration is Phase 4's.
- **SimplifiedSchema grammar.** `default(...)` could not spell a list or mapping, which the config schemas need. An argument that starts with `[` or `{` (not a `{{` slot) is now strict JSON; the serializer quotes a text default starting with a bracket so it round-trips. The pinned case in `schemas_union_postfix.rs` changed accordingly; no shipped schema used the old spelling.
- **Claudine configuration (Rule 21).** New `claudine/schemas/claudine-config.yaml` (closed `user-config`/`repository-config` covering every existing field and nested variant; `permissions` imported from Darkmatter, never redeclared), compiled in by `claudine/lib/src/config/schema.rs`, prepared through the new `darkmatter::config::schema::prepare_host_config_type`. New `config/load.rs`: strict YAML, schema validation without coercion, defaults from the schema, then serde over the projection; new `ClaudineError::{ConfigInvalid, ConfigConversion}` with file/line/column/pointer. Top-level serde defaults were removed; repository inheritance fields stay optional (absent `active_messenger` inherits, `null` disables; `steering: null` is an error). `ClaudineConfig::validate` still runs after the merge.

### Wave 6 — Reader and writer integration

- **JSON5 conversion (temporary).** New `claudine/lib/src/config/json_conversion.rs`: `json5_scan` then `Json5`, schema validation, publish through `ConfigTransaction` with a migration receipt, re-read, rename to `config.json.bak`, one `dm.config.migration` notice. Both files → YAML wins with a warning; an identical existing backup allows removing the unchanged source, a differing one is an error keeping both; an interrupted conversion is completed by the next load from the receipt. Only `<dir>/.claudine/config.yaml` paths convert a sibling `config.json`; other explicit paths are read as YAML. Completion never converts and lost the `config.json5` candidate. Every reader (loader, runaway guard, init, init wizard, config TUI, link display, uninstall, main's init check, completion) moved to `config.yaml`.
- **Request binding (Rules 12, 13, 30).** The HEAD committed tree is pinned during the request's one repository discovery and retained in `RepositoryObservation`/`ContextCaptureEvidence`. `ComposeOptions::with_config_snapshot`; `ComposeRequest::config_snapshot()`; `prepare` loads passively when none was supplied; `with_context` on a repository context without a snapshot is `ContextBuildError::ConfigSnapshotRequired`; derived/child requests share the `Arc`. `MdRequest` loads with `MigrateLegacy` and caches per source repository from the same user bytes. Claudine attaches its invocation snapshot to captured contexts through a `ConfigSnapshotSource` handle. A missing `~/.darkmatter.yaml` gives one `dm.config.missing` per `md` invocation (stderr, or data-output `diagnostics` with stderr empty; none for help/usage) and one Claudine warning naming `claudine init` (not for `handle`/`steer`); a broken file stops `md compose`/`graph`/`validate refs` and every Claudine dispatch with a located error. CLI fixtures now seed `~/.darkmatter.yaml` (`{}`) with an opt-out.
- **Per-key writers (Rule 22).** New `claudine::config::persist` (`edit_config`, `save_changes`, `initialize_config`, `validate_config_text`) applies only explicit edits through `ConfigTransaction` with schema + semantic validation under the lock; the TUI keeps an opening baseline per scope and saves the recursive leaf diff; init and the init wizard supply only their managed paths; `save_claudine_config`/`save_repo_override_config` were removed. Fixed pre-existing bugs: `init --repo` wrote a user-shaped file to the repository path, and init read the repository file as user scope. Darkmatter's `yaml_edit` now turns a `{}` root into a block on `Set` and leaves `{}` when the last top-level key is removed.

### Coordinator fixes

- Two Claudine lib guards failed between agents: `context_construction_guard` (a `FileResolutionContext::from_snapshot` in `claudine/lib/src/config/schema.rs`; the Construction gate takes no Claudine allowance) and `path_lookup_guard` (an allowlist entry named the renamed `load_claudine_config`). Moved the schema preparation into Darkmatter's `config::schema::prepare_host_config_type` (one allowlisted construction with a reason, alongside `ConfigSnapshot::load_user`'s) and renamed the allowlist entry. Both guards pass in darkmatter and claudine.
- A subagent also edited the global (outside-repo) `~/.claude/skills/use-claudine/` skill to say `config.yaml`; it is not part of this repository's change set.
- During the gate run, `claudine-cli::l1 dispatch_inventory::dispatch_inventory_matches_committed_file` reported intentional drift (1869 → 1875 dispatch sites from the new config call sites); regenerated `claudine/docs/providers/dispatch-inventory.json` with `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`.
- `claudine-cli::l1 home_lookup_round_trip::a_relative_home_is_no_home_and_does_not_reach_the_real_profile` failed: with no usable home, Claudine's long-standing config path is the relative `~/.claudine/config.yaml`, and `ConfigTransaction` refuses relative targets (`the path is not absolute`). Fixed in `claudine/lib/src/config/persist.rs` (`absolute_target`), keeping the pinned behavior; the test and the 47 `persist` unit tests pass.
- Coordinator frontmatter repair: my first insertion of the Phase 3 keys clipped four characters from the preceding key in the plan and log and left a duplicated `---`; fixed and verified with `md get` and a diff showing only additions.

### Requirement → test mapping

| Behavior | Tests |
| --- | --- |
| Shipped config schema; `darkmatter:config` alias resolves; parent defaults equal children's | unit `config::schema::tests::*`; simplified-schema `embedded` unit tests; `shipped_artifact_is_fresh` in all three packages |
| Darkmatter loader: control (real generated default config), missing user file, dependency hashes, `{}` valid, empty file invalid | `darkmatter::l1 config_load::*` (14) |
| Document loader cells: duplicate keys at three depths, trailing garbage, multiple documents, non-string keys, inadmissible number, malformed YAML, invalid UTF-8, unknown root key, BOM/CRLF/comments/markers, `k:` null | `config_load::*` |
| HEAD vs working-copy projections, unborn HEAD, invalid committed file, repository context without snapshot | `config_load::*`, `darkmatter::l1 request_config::*` (8) |
| Field shapes: `file-read` full walk (absent → scope default, null, wrong whole/one/all elements, `[]`, exclusions only, blanks, portability, repo containment); `shell-commands`, `network`, `permissions` (`{}`, reference, blank/remote/null/wrong/unknown/leaving), `register` (default, invalid entries omitted, independence from `permissions`) | `darkmatter::l1 config_fields::*` (17) |
| List/mapping defaults in the grammar; serializer round trip | simplified-schema grammar/serialize unit tests; `darkmatter::l1 schemas_union_postfix::*` |
| Request capture: one discovery, snapshot shared by derived/child requests (pointer-equal), source in another repository gets its own projection from the same user bytes, snapshot source handle | `request_config::*`, unit test in `context/request.rs` |
| `md` diagnostics: one `dm.config.missing` (formatted stderr and data `diagnostics` with empty stderr; composing and non-composing commands), none for help/usage or a repo without config, located `dm.config.invalid` with exit 1 | `darkmatter-cli::l1 config_notices::*` (6) |
| Claudine stops before dispatch on a broken Darkmatter config; one missing-config warning | `claudine-cli::l1 darkmatter_config_gate::*` (2) |
| Claudine config schema: every top-level field and nested variant (control + located invalid wiring), shared validator shape walks, active_messenger absent/null, `steering: null` invalid, user-vs-repo key sets, semantic errors | `claudine::l1 config_schema_matrix::*` (16), `claudine` unit `config::schema::tests`, `config::load` tests |
| Fixtures describe the same values after the syntax change | `config_schema_matrix` (JSON5 control vs YAML control: `claudine/lib/tests/config/user-control.{json5,yaml}`) |
| JSON5 conversion: duplicate decoded keys, trailing garbage, non-finite numbers, wrong root, comments/trailing commas, both-files warning, identical/differing backup, publish-before-rename interruption, idempotence, changed source retained | `claudine::l1 config_json_conversion::*` (12) |
| CLI wiring (control, field-shape failure, syntax failure, nothing launched) | `claudine-cli::l1 config_yaml_wiring::*` (4) |
| Per-key writes: byte preservation, read/write/read twice, concurrency with a Darkmatter `permissions` write, sequence replaced whole, removal, no-op writes nothing, invalid refused untouched, repository scope, setup creation/idempotence | `claudine` unit `config::persist::tests::*` (19+), `claudine-cli` `config_tui::persistence::tests::*` (7), `init_wizard` (2) and `init` (3) tests |
| `{}` root edits and last-key removal | `darkmatter::l1 yaml_edit::*` |
| No-home fallback path still saves | `claudine-cli::l1 home_lookup_round_trip::*` |

Input-robustness smell audit (by the loader agents): the Darkmatter loader reads fields explicitly from the schema-judged value (no `#[serde(default)]`, `unwrap_or_default`, or `.ok()` on load-bearing fields). In Claudine, top-level serde defaults were removed; nested serde defaults remain only behind the schema projection (the schema fills those values before serde) and are recorded as a review decision; `Option<Option<String>>` remains for `active_messenger` (absent vs null).

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` → `change_class: package`, `packages: [biscuit-file, claudine, claudine-cli, darkmatter, darkmatter-cli, simplified-schema, sniff, test-toolkit]`; gates were not skipped.
- `darkmatter`: `just test` — 16430 passed, 10 skipped; one slow, `glob_implementation_guard::file_references_have_one_glob_implementation` at 5.23 s with load average 77 on 16 cores, 1.37 s alone (cleared under the loaded-host allowance). `just lint` clean; `just check-tier-coverage darkmatter` — 0 stranded.
- `simplified-schema`: `just test` — 2164 + 1808 (minimal) passed, none ≥ 5 s; `just lint` clean.
- `claudine`: the first two `just test` runs stopped at the two failures above (fixed). The host was heavily loaded by unrelated applications (load averages 150–427 on 16 cores: ImageMagick, Metal compiler services, Chrome, Lightroom), and the third run (all 12024 passed) recorded 186 tests ≥ 5 s; all 186 were re-measured alone (`-j 1`, load ~7): none ≥ 5 s, the slowest 3.4 s. A fourth `just test` at lower load: 12024 passed, 2 slow (5.35 s and 5.86 s, peak load 133), 2.17 s and 2.30 s alone, cleared. `just lint` clean.
- `sniff`: `just test` — 3570 passed, 4 slow (5.7–6.9 s: three macOS hardware/audio detection tests and one `sniff-detection-cli` test), 0.97–3.03 s alone; `just lint` clean. `biscuit-file`: `just test` — 1286 + 6 passed, none slow; `just lint` clean.
- Dependents: `cargo check --workspace --all-targets` clean; `messenger --features research` tests (40) pass. DMLS is inside the darkmatter area gate.
- Cross-OS (config paths under home and repository, committed-tree capture per request, locked writes): `just cross-check darkmatter --os windows` — 11936 passed; 7 ≥ 5 s: six `expression_recovery_data` tests (5.1–7.6 s) that run under 5 s in a narrow rerun (max 4.4 s), and `current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects` (11.7 s; 7.4 s in the narrow rerun), the Windows-only slow source-scan guard Phase 2 already recorded (9.3–27.8 s since 2026-11-08), untouched by this phase. `just cross-check claudine --os windows` — 6587 passed, none slow; `just cross-check claudine-cli --os windows` — 3968 passed, none slow. No Windows failure, so `os_failing` is not set. Linux and WSL2 are left to CI.

## Phase 4

Phase 4 ran as foreground subagents in three waves, each verifying only its slice with nextest filters; the coordinator integrated between waves and ran the area gates once at the end. Wave 7: the permission evaluator, network consent, and legacy migration in parallel. Wave 8: shell and schema-slot enforcement in parallel, then the Claudine/DMLS port (which needed the new shell API). Wave 9: setup (`md init`, Claudine handshake) in parallel with parity tests and cross-agent leftovers.

### Wave 7 — Permission mechanisms

- **Evaluator (Rules 14, 15).** New `darkmatter::config::permissions` (`PermissionPolicy`, `PermissionVerdict`, `PolicyLayer`, `ShellPermissions`) built once per snapshot and exposed as `ConfigSnapshot::policy()`; `permissions()` keeps returning the raw per-file settings (setup repair reads them). New `config/load/reference.rs` follows a `permissions` reference one hop: user scope on disk, repository working copy on disk with canonical containment, repository HEAD only through the pinned committed tree (`FileReference::candidate_plan`, committed symlinks followed with cycle detection and a 40-link limit). Missing/invalid targets, a target without a `permissions` mapping, and chains are `dm.config.reference` errors naming both files and suggesting `md init --inline`. Followed targets join `dependencies()` (`ConfigDependency::referenced_from`). Verdict rule: pre-approved iff (user ∨ (HEAD ∧ worktree)) positive and no `!` in any layer; network is the AND of every applicable layer.
- **Persistence (Rule 19).** New `config/permissions/persist.rs::save_user_approval` saves an exact shell rule or an escaped `~/…` file entry to the user block (following its reference), materializing an omitted list's schema defaults first, deduplicating, refusing when an exclusion matches, and validating before the locked replace; `NotSaved` carries `dm.config.not_saved`.
- **Network consent (Rule 17).** `biscuit_file::FetchPolicy::allow_all()` (explicit flag, not a wildcard; default stays deny-all) and new `darkmatter::config::network::NetworkPermission::from_snapshot`, the one projection both composition (`RemoteFetchRuntime::with_store_and_network`) and slot reads use; an invalid block denies and reports why. Coordinator: switched the projection from raw inline settings to `policy().network()` (so a referenced block's `allow` counts) and added `network_permission::a_referenced_user_block_that_allows_composes_a_remote_file`.
- **Legacy migration (Rules 20, 34).** New `config/migration.rs`: in `ConfigLoadMode::MigrateLegacy` the user scope's `~/.darkmatter-shell-{whitelist,blacklist}` (one shell unit, blacklist rules become `!` entries) and `~/.config/darkmatter/schema-read-policy` (under-home `allow file/dir` → escaped `~/` entries, `deny` → `!`, url/site/outside-home lines dropped with notices) migrate into missing properties only, through the locked writer with a receipt, then delete only sources whose bytes still match. Passive loads report a hint and never write. Coordinator: repository-scope migration runs in `ConfigSnapshot::with_repository_projection` (the path `md` and Claudine use per source repository), and `md` reports `dm.config.migration` notices (user scope at startup; repository scope as each projection happens, on stderr or in data-output `diagnostics`) via new `MdRequest::report_config_notices`.

### Wave 8 — Enforcement adapters

- **Shell (Rule 18).** Removed `policy_root`, `ShellPolicyPaths`, `resolve_policy_paths`, the policy-file loaders/appenders, user-blacklist refusal, and `BlacklistPersist`/"always deny". Every site (inline, `::shell`, shell blocks, frontmatter, templates, preflight, lifecycle, md host) decides: built-in blacklist refuses first (before yolo, caches, approvals) → any segment matching an exclusion requires approval → pre-approved when every segment is allowed → else approval. An invalid block fails commands with `ShellExpansionError::Config` (`dm.config.invalid`). "Always allow" saves and also applies for the rest of the run; a failed save warns `dm.config.not_saved` (added once to `COMPOSE_DIAGNOSTIC_CODES`). The compose-cache identities include the shell rules.
- **Schema slots (Rules 16, 19).** Removed the automatic grant (`grants/auto.rs`), the grant store, folder/site/any-query grants, and persistent denies (`grants/legacy.rs` keeps only what the migration parses). The gate decides each read before the file or any memo is touched: invalid permissions refuse; this run's deny/allow; `policy.file_read(target, reading)`; else the handler (allow once / always allow / deny for this run; URLs offer only per-run choices). `ReadGate::admits_listed` filters listings (`find_files`, `find_*_index`) to files admitted without a prompt; no directory prompt. Not-saved approvals surface through `SlotSession::take_not_saved_warnings`.
- **Claudine and DMLS.** Claudine's harness takes `CapturedPermissions` from the invocation's snapshot with md's decision order; its policy-root/document-directory fallback, "always deny", and `ShellPolicyWriteFailed` are gone. DMLS's shell overlay classifies passively from a loaded snapshot (Denied for the built-in blacklist, Approved only when all segments are pre-approved, Unknown otherwise or for an invalid block) and warns once when `.dmls.toml` still sets the retired `shell.policy_path`. `markdown::schemas::slots::session::load_passively` became public as the shared passive loader.

### Wave 9 — Setup and parity

- **Setup (Rule 23).** New `darkmatter::config::init` and `md init [--repo] [--inline]` (`cli/src/commands/init.rs`, output schema `cli/schemas/output/init.yaml`, `COMMAND_LEAVES` row, conformance tests, `docs/cli/init.md`): writes every property at its default, preserves authored text and references, adds only missing top-level settings, `--repo` fails before writing outside a repository, `--inline` replaces a reference with its target's block (or reported defaults when the target is broken). New `claudine::config::setup::setup_with_darkmatter` implements the 2.9 handshake (reference, transfer with Claudine written first and the reference published last via new `TargetChange::published_last`, conflict stop, third-party reference left alone). There is no production `claudine init` command any more, so the handshake runs from Claudine's first-run setup (see decisions). Without a home, loads now warn once that no home is available.
- **Parity and leftovers.** One shared table `darkmatter/lib/tests/fixtures/permission_parity/cases.json` (commands, templates, files, URLs, exclusions) drives `darkmatter::l1 permission_parity`, Claudine's `shell_permission_parity::shared_fixture`, and DMLS's `shell_permission_classification` parity test. Fixed: two context-construction guard hits (slot session field, DMLS save path, which now judges exclusions from the document's own context); DMLS slot watches now include `.git/index`/`HEAD` for any in-repository document (a refused-only document previously got no watch); retired `GrantStoreError`/`ShellExpansionError::SlotGrantStore`; `md schema validate`/`md clean` report not-saved slot warnings; a real Claudine defect — launch-schema checks had no slot session, so any templated schema failed as "unresolved slot" — fixed in `composition/prepare.rs`.
- **Audit.** Remaining references to `.darkmatter-shell-whitelist/blacklist`, `schema-read-policy`, `policy_root`, `shell.policy_path`, and "always deny" are only the temporary migration code/tests/docs and statements or tests asserting retirement.

### Coordinator fixes and findings

- Three signed commits (`ea84a2b02`, `31516c4bc`, `3e4cf53f3`, 18:25–18:32) appeared on the branch during Wave 9 that the coordinator did not make (every subagent was told not to commit). They are left in place; this phase's change set is those commits plus the working tree, and the gate scope was computed from `f00f3efa3` (the Phase 3 close). One of them changed `darkmatter/dmls/zed-dmls/extension.toml` to serve YAML too; `zed_extension_contract::zed_extension_manifest_and_crate_shape_are_loadable` was updated to that manifest.
- `md init` built a raw user-only context (`MdRequest::user_context`), which the md CLI context guard refuses; it now uses the launch context (the user scope reads only the home). The same in Claudine's `init_wizard.rs::write_with_darkmatter_setup` (now `capture_file_resolution_context`).
- `argv_normalization::wrapper_help_renders_wrapper_screen` failed: the first-run handshake printed "now points at" on `claude --help`; pointing an unblocked `.darkmatter.yaml` at Claudine's file is now silent (a transfer is still reported).

### Requirement → test mapping

| Behavior | Tests |
| --- | --- |
| Scope defaults, omitted vs `[]`, HEAD∧worktree per command, exclusions in each layer, uncommitted additions/removals, network AND, Git capture error | `darkmatter::l1 config_permissions::*` |
| References: user/worktree/HEAD following, seven bad-target cases located at both files, `..`/symlink escape, inline↔reference conversion, redirection, target edit/deletion per revision, committed symlink chain/cycle/escape/limit, unborn HEAD | `config_permissions::*` |
| File-read matching: `~/`, portable `{{VAR}}`, escaped globs, `..`, missing probes, symlink escape | `config_permissions::*` |
| Approval persistence: byte-preserving save through a reference, idempotence, default materialization, creation, exclusion refusal, outside-home/dir/no-home not saved, refused edits leave files untouched, repo file never edited | `darkmatter::l1 config_permission_persist::*` |
| Network consent: allow/deny/default, repo vs user, uncommitted widening, exact host, referenced block, invalid block, redirects, cached body, slot/composition parity | `darkmatter::l1 network_permission::*`, biscuit-file `fetch` unit + `fetch_integration` |
| Migration: mixed shell unit, authored `[]`, grant-store conversion, dropped lines, invalid lines, passive, repeat, crash window, changed source, failed write, document-dir files, references, no home, repository scope | `darkmatter::l1 config_migration::*` (16) |
| Migration notice in `md` data output with empty stderr | `darkmatter-cli::l1 init_command::*` |
| Shell: built-in refusal over allows/yolo/caches/templates/lifecycle; exclusions per layer; noninteractive approval-required; chains; save + run reuse; not-saved; invalid block | `darkmatter::l1 shell_permissions::*` (15), `darkmatter-cli::l1 compose_shell::*`, `approval_prompt::*` |
| Slots: reads decided before memo (content, probe, import, `validate_schema`), listings filtered without prompts, run override of exclusions, not-saved for excluded files, portable save round trip, repository default under the commit rule, URLs, invalid permissions | `darkmatter::l1 slot_permissions::*`, ported `dynamic_schema_slot*`, CLI `dynamic_schema_slots::*` |
| Claudine host decisions equal md's | `claudine::l1 shell_permission_parity::*`, harness/preflight unit tests |
| DMLS passive classification, no writes, `shell.policy_path` retirement warning | `darkmatter-lsp::l1 shell_permission_classification::*` |
| Shared parity table through md / Claudine / DMLS | `darkmatter::l1 permission_parity::*`, `shell_permission_parity::shared_fixture::*`, DMLS parity test |
| `md init` / setup: creation, idempotence, preservation, references, `--inline` (readable/broken), invalid file, `--repo` outside a repository, no home, publication order | `darkmatter::l1 config_init::*` (13), `darkmatter-cli::l1 init_command::*`, `output_commands` init rows |
| Claudine handshake: both orders, repeat, conflict, equal blocks, third-party reference, broken-reference repair, interrupted transfer, repository scope, no home | `claudine::l1 darkmatter_setup::*` (12) |

Input-robustness smell audit: the new loaders read references and migrated lines explicitly; no `#[serde(default)]`, `unwrap_or_default()`, or `.ok()` on a load-bearing parse was added (the agents grepped their files).

### Gates

- Scope: `git status`-based `affected_scope.py` no longer covers the phase (three mid-phase commits), so it was run over `git diff --name-only f00f3efa3` plus untracked files: `change_class: package`, `packages: [biscuit-file, claudine, claudine-cli, darkmatter, darkmatter-cli, darkmatter-lsp, test-toolkit]`; gates were not skipped.
- `darkmatter`: `just test` — first run 16451 passed, 1 failed (`zed_extension_contract`, above); after the fix 16483 passed, 10 skipped, none ≥ 5 s. `just lint` clean. `just check-tier-coverage darkmatter` — 0 stranded.
- `claudine`: first run stopped at `argv_normalization::wrapper_help_renders_wrapper_screen` (fixed); a load-sensitive `wrap_perf::composition_setup_and_provider_handoff_order_matches_phase_1_baseline` failure passed alone; then `context_construction_guard` (fixed). The third run (12048 passed) was taken at load 120–250 caused by another worktree's test suite and ImageMagick: 99 tests ≥ 5 s, one at 13.3 s; re-measured alone, the library ones were ≤ 4.6 s (load ~137) and the CLI ones 0.8–1.4 s (load ~35). A fourth `just test` at load ~30–55: 12048 passed, none ≥ 5 s. `just lint` clean (only the existing `__eh_frame` linker warning).
- `biscuit-file`: `just test` — 1292 + 6 passed, none slow; `just lint` clean.
- Dependents: `cargo check --workspace --all-targets` clean; `messenger --features research` tests (40) pass. Sniff uses only `FetchPolicy::deny_all().allow_host` (unchanged).
- Cross-OS (permission targets under home and repository, committed symlinks, locked writes, setup transfers): `just cross-check darkmatter --os windows` — 11969 passed; 4 ≥ 5 s, all pre-existing Windows-only slow tests in code this phase did not touch: `current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects` (10.7 s; recorded at 9.3–27.8 s since 2026-11-08), `expression_recovery_data::allowed_transport_failure_recovers::connection_refused` (7.1 s) and two `expression_regression::regression_division_by_zero_*` (5.3–5.6 s), the same group Phase 3 recorded. `just cross-check claudine --os windows` — 6612 passed, none slow; `just cross-check claudine-cli --os windows` — 3967 passed, none slow. No Windows failure, so `os_failing` is not set. Linux and WSL2 are left to CI.
- At 20:30, after this log's Phase 4 frontmatter was written but before the gates finished, three more signed commits appeared that the coordinator did not make (`3f1c173f5`, `ad6e24d34`, `5c8d29e71` "record Phase 4 close"); they hold this phase's remaining working-tree changes. The edits made after them (this Gates section and the last two plan checkboxes) are uncommitted.

## Phase 5

Phase 5 ran as four foreground subagents. The coordinator first wrote the Rule 25 proof (`simplified_schema::triggers::identify`) so that the two Wave 10 tasks could proceed in parallel: identity tests, and the manifest schema plus profile loader. Wave 11 then ran sequentially, because both of its tasks edit compose preparation: first layering and selection, then the foreign-document runtime. The coordinator ran the area gates once, after all subagents had finished. No mid-phase commits appeared.

### Wave 10 — Registration core

- **Identity proof (Rule 25).** New `simplified-schema/lib/src/triggers/identify.rs`: `admit_identifying_trigger(&MatchConditions, &BTreeSet<String>) -> Result<IdentifyingTrigger, IdentifyTooBroad>` (`dm.host.identify_too_broad`, located field such as `match[0].group[2]`, plus a repair hint) and `identifies`. In `identifies`, a positive required atom tests non-null presence; guards, `none:` children, and `$path` keep their ordinary meaning. Ordinary `matches` is unchanged; only `property_matches` and `PathSubject::admits` became `pub(crate)`. The subagent fixed field naming (an implicit single-child `all` no longer adds a segment), library-neutral wording, and a doc that disagreed with the code under double `none:`.
- **Manifest and profiles (Rule 24).** The new shipped schema `darkmatter/schemas/darkmatter-host.yaml` is the closed v1 envelope, with every field required and alias `darkmatter-host@darkmatter:host`. `just shipped-schemas` regenerated the Darkmatter and Claudine artifacts.
  - New module `darkmatter::config::registration` (`registration.rs`, `registration/manifest.rs`) provides `load_registrations`/`load_entries` → `LoadedRegistrations { profiles, diagnostics }`, all-or-nothing `RegisteredHostProfile`s, `md_owned_names()`, `agreement()` (same-host claims collapse or conflict), typed `RegistrationDiagnostic`s, and `report_diagnostics::registration_diagnostic`.
  - The manifest is validated before any member is read. Every member and transitive import is read once in one retained read set. That read set becomes `dependencies()`, and `revision()` is a BLAKE3 digest over manifest meaning plus dependency digests, so comments do not change it.
  - New explicit companion-path loaders: `HostLifecycleProfile::from_artifact_at`, `HostFunctions::analysis_only_at`, and `catalog::host_function_set_at`. Discovery by stem is unchanged.
  - `LoadedTrigger::from_file` was added in Simplified Schema because the trigger's fields are crate-private.
  - Registered trigger payloads resolve from their own file without schema roots, and their `pattern_cwd` is the document's tree root.

### Wave 11 — Registration consumers

- **Selection and layering (Rule 26).** New `config/registration/selection.rs` adds `HostSelection { Md, Registered, Legacy, Ambiguous }`, `Selected`, `RegisteredHosts`, `select`, and the one central `LegacyHost` view.
  - `DarkmatterSchemas::with_registered_hosts` layers, from lowest to highest: baseline, the selected host's matching triggers in manifest order, root triggers, then `$schema`. A same-basename root file does not shadow a registered trigger. An identical registered and root trigger applies once.
  - Origins carry the manifest identity and revision. Cache identities include them, and the merge memo now compares layer digests (see decisions).
  - Selection is wired into compose, `md schema validate`, `md clean`, and `md schema triggers` (new `registered` field in `cli/schemas/output/schema-triggers.yaml`).
  - The md CLI loads registrations once per repository (`MdRequest::registered_hosts`) and reports each distinct record once (`report_host_diagnostics`).
  - A caller that supplies its own host globals (Claudine) gets `Md` and no registered layer.
  - Departure found while testing: the inspection commands had started loading the repository configuration in migrating mode, and one test migrated the real checkout's root. The checkout was restored, and these commands now use the new `ConfigSnapshot::with_passive_repository_projection`, with a regression test.
- **Foreign documents (Rule 27).**
  - Library: `ComposeOptions::with_selected_host(profile)` declares the host's globals for analysis as unavailable (`dm.host.runtime_only`), via `RegisteredHostProfile::analysis_globals()`.
  - `md_host::{shared_event_keys, execution_projection, foreign_deferred_keys}` derive everything from the host's event registry.
  - md CLI: the host is identified before the lifecycle is parsed. Only `initialize`/`cancelled`/`finalize` are parsed and run, from the execution projection, with md bindings. Host event keys are withheld from compose-time interpolation. Host functions are known for typing but have no handlers.
  - `dm.host.foreign_document` is reported once per root request: an info row on stderr for formatted output, or one `diagnostics` entry with empty stderr for data output.
  - New in-process recorder `darkmatter/lib/tests/host_fixtures/recorder.rs` drives `md_host::drive` through the real `evaluate` and records events and effects.
  - The Claudine-like `relay` fixture host has renames, host-only events, a `seq` global, and a trigger that types `composed`/`failed` as `null`.
  - Departure: the motivating document's `say: "{{ seq.name }}"` alone cannot fail, because md skips `say` without evaluating it. The test adds an `info:` reading `seq.name` to show the normal runtime failure.

### Validation checkpoint — coordinator

- Added `host_selection::loading_selecting_and_composing_with_a_registration_writes_nothing`. It snapshots every file under the fixture's home, work tree, and host install, then loads, selects, builds the effective schema, and composes an identified document. No file is written, created, or removed; no approval handler, network host, or executable host is configured. `a_remote_member_is_refused_without_fetching` covers the network side.

### Requirement → test mapping

| Behavior | Tests |
| --- | --- |
| Witness proof: all/group/none/min-match bounds, nested, shared-only, optional-only, path-only, empty, nullable/unknown atoms | `simplified-schema::l1 identifying_triggers::identify_*` (15) |
| Presence-only identification for wrong-typed host keys vs ordinary matching; guard/`none:` semantics; `$path` still constrains; no I/O | `identifying_triggers::identify_tests_non_null_presence_where_ordinary_matching_tests_type`, `identify_keeps_guard_and_none_semantics`, `identify_path_*` |
| Claudine's nine keys identify at every non-null YAML type; shared events alone do not | `identifying_triggers::identify_admits_and_recognizes_a_claudine_style_host` |
| Manifest input matrix (control, absent, null, wrong type, element types, empty, closed keys, `format_version` 2/"1"/1.0, wrong kind, duplicate key, trailing document), remote member, missing member, stale lifecycle artifact, host mismatch, identify too broad, duplicate manifests, agreement/conflict; no partial profile | `darkmatter::l1 host_registration::*` (32) + unit tests in `config/registration*` |
| md-owned names exclude Claudine's identifying keys | `host_registration` unit/L1 owned-name test |
| Layering order, same-basename non-shadowing, identical dedup, authored override, import anchors, legacy fallback, suppression inside/outside a package area, conflicts, ambiguity, isolation, cache identity, registered trace, compose application, Claudine opt-out, passivity | `darkmatter::l1 host_selection::*` (15) |
| CLI: validate/compose/clean/triggers apply the registered schema; one warning in formatted and data outputs; no repository migration | `darkmatter-cli::l1 host_registration::*` (8) |
| Foreign compose: shared stacks only with md bindings, recorder (no speech/notify/effects), `seq` fails normally, `composed`/`failed` rejected by host schema, host functions not invoked, projection and deferred keys | `darkmatter::l1 foreign_document_compose::*` (10), `darkmatter-cli::l1 compose_foreign_documents::*` (8) |
| Notice once per root request (formatted/data, with a foreign child); md-only document gets none | `compose_foreign_documents::*` |
| Shipped artifact chain current | `darkmatter` / `claudine` / `simplified-schema` shipped-artifact tests |

Input-robustness smell audit: the manifest loader validates the whole document against the closed shipped schema before reading any field. The subagents grepped their new files for `#[serde(default)]`, `unwrap_or_default()`, and `.ok()` on load-bearing parses, and none remain. The pre-existing `from_host_text` probe is permissive on purpose; it only lists declared names, and `load` re-validates them.

### Gates

- `darkmatter`: `just test` — 16558 passed, 10 skipped, none ≥ 5 s (load 8–63). `just lint` clean (including the zed-dmls wasm check). `just check-tier-coverage darkmatter` — 0 stranded.
- `simplified-schema`: `just test` — 2179 + 1822 passed, none ≥ 5 s; `just lint` clean; `cargo check -p simplified-schema --no-default-features` clean.
- Dependents: `cargo check --workspace --all-targets` clean.
  - Claudine filtered by the changed surfaces (`shipped | schema | trigger | host | lifecycle | compose | guard | registry`): `claudine` + `claudine-cli` 2971 passed.
  - In that run, `claudine-cli lifecycle_message_drain::stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown` failed because it was run without `--features test-fixtures`: its 2 s budget override is compiled only with that feature. Rerun with the feature: claudine-cli 1358 passed.
  - Two `claudine_schemas` tests were at 5.0–5.2 s in the loaded run (load ≈ 60) and passed alone in 1.7 s and 2.1 s; they are load-sensitive, not slow.
- Repository root checked after every CLI slice: no `.darkmatter.yaml`, lock file, or deleted whitelist.
- Cross-OS: registration resolves manifest and member paths, canonical manifest identities, and `$path` subjects, so a Windows check was run. `just cross-check darkmatter --os windows` passed all 12028 tests with no failures. Four tests took 5 s or more, all on Windows only and all in code this phase did not change:
  - `current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects` (12.6 s) and `expression_recovery_data::allowed_transport_failure_recovers::connection_refused` (6.0 s), both already recorded in Phase 4;
  - `shipped_schema_corpus::shipped_markdown_named_type_references_resolve` (9.0 s) and `shipped_markdown_inline_schemas_parse_passively` (9.3 s). These two scan the repository's documentation, skill, and feature trees, take 0.8 s each on macOS, and reflect Windows file I/O under full-suite load; this phase added only one documentation page to what they scan.
  - With no Windows failure, `os_failing` is not set. Linux and WSL2 are left to CI.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gives `change_class: package` and `packages: [claudine, darkmatter, darkmatter-cli, simplified-schema, test-toolkit, claudine-cli]`. `test-toolkit` comes from the unrelated, already-present `prompts/plan.md` edit. The gates were not skipped.
