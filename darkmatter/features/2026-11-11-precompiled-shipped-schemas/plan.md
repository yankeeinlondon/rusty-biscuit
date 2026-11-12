---
kind: plan
created: 2026-11-11
total_phases: 5
phase: 4
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
- simplified-schema/lib/src/shipped/mod.rs
- simplified-schema/lib/src/shipped/wire.rs
- simplified-schema/lib/src/shipped/wire_support.rs
- simplified-schema/lib/src/shipped/records.rs
- simplified-schema/lib/src/shipped/registry.rs
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/embedded.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/work_counters.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/definitions.rs
- simplified-schema/lib/src/simplified/alphanumeric.rs
- simplified-schema/lib/src/simplified/standalone.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/examples/shipped_schemas.rs
- simplified-schema/lib/generated/shipped.json
- simplified-schema/lib/tests/l1/shipped_artifacts.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/Cargo.toml
- simplified-schema/justfile
- justfile
- .gitattributes
- Cargo.lock
docs_updated_during_phase_1:
- simplified-schema/README.md
- simplified-schema/docs/dependencies.md
- simplified-schema/docs/topics/language-core.md
- simplified-schema/docs/topics/language/schema-documents.md
docs_created_during_phase_1:
- simplified-schema/docs/topics/shipped-artifacts.md
skills_files_updated_during_phase_1:
- .claude/skills/simplified-schema/SKILL.md
packages:
- simplified-schema
- darkmatter
- claudine
- claudine-cli
- test-toolkit
source_files_during_phase_2:
- simplified-schema/lib/src/positions.rs
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/shipped/mod.rs
- simplified-schema/lib/src/shipped/wire.rs
- simplified-schema/lib/src/shipped/wire_support.rs
- simplified-schema/lib/src/work_counters.rs
- simplified-schema/lib/generated/shipped.json
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/shipped_artifacts.rs
- simplified-schema/lib/tests/l1/shipped_sections.rs
- simplified-schema/lib/tests/l1/deferred_positions.rs
- darkmatter/lib/Cargo.toml
- darkmatter/lib/src/work_counters.rs
- darkmatter/lib/src/effects/mod.rs
- darkmatter/lib/src/effects/types.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
- darkmatter/lib/src/markdown/schemas/shipped.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/src/markdown/schemas/catalog.rs
- darkmatter/lib/src/markdown/schemas/embedded.rs
- darkmatter/lib/src/markdown/schemas/resolve.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs
- darkmatter/lib/src/markdown/schemas/yaml_documents/registry.rs
- darkmatter/lib/src/markdown/schemas/triggers/mod.rs
- darkmatter/lib/src/markdown/schemas/triggers/prepared.rs
- darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
- darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
- darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
- darkmatter/lib/src/markdown/schemas/triggers/finding.rs
- darkmatter/lib/examples/shipped_schemas.rs
- darkmatter/lib/generated/shipped.json
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/shipped_artifacts.rs
- darkmatter/lib/tests/l1/prepared_trigger_bundles.rs
- darkmatter/lib/tests/l1/deferred_positions.rs
- darkmatter/justfile
- justfile
- .gitattributes
docs_updated_during_phase_2:
- simplified-schema/docs/topics/shipped-artifacts.md
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/language/schema-documents.md
- darkmatter/docs/topics/schemas/definition.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/schema.md
- .claude/skills/simplified-schema/SKILL.md
source_files_during_phase_3:
- claudine/lib/src/composition/schema/shipped.rs
- claudine/lib/src/composition/schema/bundle.rs
- claudine/lib/src/composition/schema/globals.rs
- claudine/lib/src/composition/schema/mod.rs
- claudine/lib/examples/shipped_schemas.rs
- claudine/lib/generated/shipped.json
- claudine/lib/tests/l1/shipped_artifacts.rs
- claudine/lib/tests/l1/main.rs
- claudine/lib/Cargo.toml
- claudine/justfile
- claudine/cli/tests/l1/error_guards/transport-allow.toml
- darkmatter/lib/tests/l1/prepared_trigger_bundles.rs
- darkmatter/lib/tests/l1/shipped_artifacts.rs
- scripts/shipped_schemas.py
- scripts/ci/test_affected_scope.py
- tools/test-toolkit/tests/shipped_schemas_coordinator_contracts.rs
- tools/test-toolkit/Cargo.toml
- justfile
- .gitattributes
docs_updated_during_phase_3:
- claudine/docs/topics/schemas.md
- simplified-schema/docs/topics/shipped-artifacts.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/claudine/SKILL.md
- .claude/skills/darkmatter/SKILL.md
---

# Precompiled shipped schemas implementation plan

## Summary and Definition of Done

Move immutable schema preparation out of process startup into three checked-in JSON artifacts, owned by Simplified Schema, Darkmatter, and Claudine. Share the generator and acceptance mechanism in Simplified Schema; preserve ordinary preparation as the complete fallback. Route existing baseline, named-type, catalog, trigger, and YAML-kind consumers through their owner's immutable registry. Then defer unused source-position work, carry ordered layer identities into validator caching, and remove test fixtures that substitute smaller schemas solely for speed.

Completion means:

- All three artifacts regenerate deterministically, compare byte-for-byte with fresh uncached preparation, and round-trip every observable ordering and source identity. Freshness covers transitive sources and preparation code.
- Fresh processes perform zero runtime lowering of accepted shipped entries and zero relocated baseline/payload admission or compile checks. Stale, malformed, incompatible, custom, disk, and template inputs retain ordinary preparation and its diagnostics.
- Disk filename suppression, bundle-local imports, YAML-kind checkout replacement, unsaved-buffer invalidation, context-dependent file checks, and all existing diagnostic coordinates remain correct.
- Successful admission and YAML validation build no position map until coordinates are requested. Clones share one computation over the original source snapshot.
- Typed cache identities distinguish canonical schemas from ordered layer provenance. Every post-merge transformation is represented or uses the canonical final-schema route. Schema-equivalent data documents still share structural validators.
- `just test` and `just lint` pass in all three areas, including Simplified Schema's minimal-feature build and Darkmatter's DMLS tests. Relevant source guards and CI test-input planner checks pass. No new CI job, OS matrix, runtime disk cache, `build.rs`, or test-profile optimization is introduced.
- One quiet-host before/after sample records debug/release medians of five executions: three-line `md compose` at most 90/20 ms and frozen-prompt Claudine debug composition at most 650 ms. Darkmatter and Claudine L1 have no test over five seconds on the quiet host; report summed JUnit durations before and after. Deterministic counters establish the removed work independently of timings.
- Topic docs, API comments, skills, and the adjacent implementation log describe the final behavior. The feature remains in its active directory, implementation complete and ready for review.

All execution is non-interactive. Waves below have globally unique numbers. Parallel permission is explicit in each task; tasks with stated prerequisites run sequentially even within a wave. Later waves wait for earlier waves. Repeat the non-interactive constraint in every agent brief. One coordinator owns shared manifests, test roots, artifact regeneration, and final gates. No commit, push, branch creation, or lifecycle move is part of this plan.

## Phase 1: Shared artifact mechanism

### Necessary Rules

1. **Use JSON throughout this implementation.** Encode UTF-8 with `serde_json::to_vec_pretty` and exactly one final LF; use wire version `1` and owning-crate preparation revision `1` initially. Increment the owning revision for subsequent preparation semantics changes, including changes made in later phases of this implementation, then regenerate dependents. Do not add a binary-format experiment. Evidence: the spec selects JSON and already provides sufficient performance justification; `simplified-schema/lib/Cargo.toml` already has Serde, ordered JSON, and `indexmap/serde`. There is no evidence that JSON decoding misses a target. Timing verification is acceptance work, not a design spike.

2. **Put the common mechanism in `simplified-schema/lib/src/shipped/`.** `mod.rs` exports `ArtifactHeader`, `SourceFingerprint`, `DependencyFingerprint`, `ArtifactDescriptor`, `AcceptedArtifact<T>`, `FallbackReason`, and `PreparationMode`; `wire.rs` owns strict decoding and deterministic encoding; `records.rs` owns shared prepared records. Host payload records live in each host's `shipped.rs`. Expose `generate`, `load_or_prepare`, and `check_fresh` over supplied source tables, dependency descriptors, and preparation closures. Use callbacks, without a registration trait or global host registration. Evidence: `embedded::ShippedSources` explicitly carries host sources and `resolve::prepare_embedded_schema_in` already accepts supplied documents; the spec requires a shared lower-level mechanism without reverse dependencies.

3. **The wire header has mandatory `wire_version: u32`, `owner: String`, `preparation_revision: u32`, `sources: Vec<SourceFingerprint>`, `dependencies: Vec<DependencyFingerprint>`, and `configuration: Vec<(String, String)>`.** A source fingerprint contains its portable virtual identity and `xx_hash_bytes(text.as_bytes())` as a fixed-width 16-digit lowercase hex string. A dependency fingerprint contains owner, preparation revision, and the hex digest of its freshly generated artifact bytes. Sort source/configuration/dependency metadata by identity and reject duplicates. Payload ordering is authored ordering, never alphabetically sorted. Empty dependency/configuration lists are explicit values. Evidence: spec Header/Portability; `biscuit-hash` exposes `xx_hash_bytes`; `DefinitionTable` allocates `$defs` in insertion order; JSON has no platform-sized integer encoding requirement.

4. **Use owned Serde records and rebuild only indexes.** Add Serde support to the pure AST graph rooted at `SchemaRoot`, `PropertyDef`, constraints, spans, and source identities in `simplified/{types,source,definitions}.rs` and their contained language types. Store `DefinitionTable` as ordered `(DefKey, Definition)` pairs and reconstruct `by_json_key`; do not serialize a map with structured JSON keys. Use fixed-width wire integers with checked conversion for runtime indexes/spans. Reject file-backed namespaces in shipped records. Use host mirrors for `ResolvedSchema`, catalogs, admitted envelopes, kind definitions, and payloads containing `Arc`, static source labels, or derived caches. Reconstruct `Arc`s once and borrow views thereafter. Never serialize `Prepared`, `CompiledValidators`, conversion plans, regex caches, request contexts, or locks; never leak owned strings. Evidence: `simplified::definitions::DefinitionTable` has both `entries` and a derived index; `Prepared` contains compiled validators; `GlobalCatalog` owns its strings, and `ExpressionFunctionDescriptor` already uses owned `Box<str>` fields.

5. **One accepted artifact is an all-or-nothing unit.** Decode strictly, compare complete expected source inventory/configuration/revisions, recursively validate dependencies against current compiled text and revisions, and validate record identities, variants, reference targets, indexes, and required schema shapes before exposing entries. `AcceptedArtifact<T>` has a private constructor, so only this acceptance path can authorize skipping checks. A descriptor cannot supply a Boolean claiming checks passed. Any refusal discards the whole owning payload and invokes ordinary preparation; a lower refusal invalidates the owning artifact even when the checked-in lower artifact bytes match its dependency header. Preserve ordinary preparation errors and existing accessor panic behavior for genuinely broken compiled sources. Artifact refusal alone produces no warning. Evidence: spec Loader/Checks and `envelope::definitions`/`globals::claudine_globals` currently retain their preparation error contracts.

6. **Add explicit prepared descriptors without changing source-only behavior.** Keep `ShippedSources::new`, `ShippedSources::NONE`, and `EmbeddedSchemaBundle::new` as source-only constructors. Add const `with_prepared` builders taking a static descriptor with a function-pointer provider; provider calls occur only on prepared lookup. Its return is an owned/shared accepted handle, not leaked strings. Darkmatter's `SHIPPED_SOURCES` supplies its named-type view; `claudine_schema_bundle()` supplies its bundle view. Match owner/bundle identity, document identity, complete supplied inventory, and dependency fingerprints before use. A custom same-name bundle with different text takes ordinary preparation. Evidence: `embedded.rs` in both crates stores copyable static source tables, Claudine's bundle constructor is const, and `resolve::shipped_named_types` currently memoizes arbitrary host-supplied sources.

7. **Fresh generation bypasses every artifact and preparation memo transitively.** Thread explicit `PreparationMode::Uncached` through envelope admission, embedded resolution, catalog loading, shipped named-type loading, trigger envelope/payload preparation, and host generation callbacks; `Runtime` is the ordinary path. Uncached calls receive freshly prepared lower data explicitly. They never consult a runtime registry, named-type memo, payload memo, validator memo, or cached admission result. Use private validator caches for generation checks. Bootstrap the two envelope definitions with `parse_standalone_unadmitted` and `lower_embedded_definition`, then validate their contracts after both exist. Evidence: `envelope::prepare_definition` already has this non-self-admitting bootstrap; `resolve::shipped_named_types`, `GlobalCatalog::darkmatter`, and `load_embedded_trigger` otherwise conceal process memo hits. A thread-local toggle or environment switch is unnecessary and would hide dependencies.

8. **Keep one immutable registry per owner and keep compiled validators separate.** Put Simplified Schema's registry in `shipped/mod.rs`, Darkmatter's in `markdown/schemas/shipped.rs`, and Claudine's in `composition/schema/shipped.rs`. Existing static-returning accessors borrow registry fields. The envelope definition view may lazily compile validators in a separate validation holder. Effects' runtime `REGISTRATIONS` remains a binding cache. `KindRegistry::shipped()` becomes an `Arc` view stored with the shipped seed; per-workspace registries remain separate. Preserve `reset_process_caches_for_bench`: immutable compiled-in data stays loaded, runtime/custom/disk/validator/merge/phase/slot/matcher caches clear. Evidence: that function's existing contract expressly keeps compiled-in baseline/catalog constants; `schema_validation.rs::check_cold_and_warm` uses disk fixtures and expects payload and validator misses after reset. Do not invent registry unloading to satisfy an absent contract.

9. **The inventory includes every byte that preparation embeds.** Simplified Schema owns `ENVELOPE_DEFINITIONS`. Darkmatter owns `embedded::SHIPPED`, `schema-trigger.yaml`, `effects/types.rs::MUTATIONS_SOURCE`, and the four YAML texts in expression catalog `EXAMPLE_FILES`; Content Policy's `EDITOR_SCHEMA` remains owned by Content Policy and fingerprinted in Darkmatter. Do not create a fourth artifact. Claudine owns `CLAUDINE_SCHEMA_FILES` plus `HOST_CATALOG_TEXT`. Do not add `EXAMPLE_DATA_FILES` to preparation: they feed example execution/documentation, not `load_catalog`. Evidence: expression `load_catalog` takes `EXAMPLE_FILES`, whose decoded example text becomes descriptors; `EXAMPLE_DATA_FILES` is a distinct runner input. Dependency chain is Simplified Schema → Darkmatter → Claudine; DMLS gets no artifact.

10. **Preserve selection before lookup.** `TriggerRegistry::append_embedded_bundle`/discovery first applies existing root enumeration, claimed filenames, invalid findings, case-fold collisions, and bundle order, then obtains prepared surviving entries. Imports in a surviving embedded entry stay within its supplied bundle plus `darkmatter:`; disk suppression never substitutes disk import contents. Rebind `LoadedTrigger::pattern_cwd` and matcher context per registry. `yaml_documents::registry::{shipped_identity,shipped_claims}` retains checkout replacement by repository-relative definition path and existing duplicate claims; workspace/editor state is never serialized. Evidence: discovery's `claimed_names` includes ignored/invalid files, `assemble::resolve_embedded_payload` confines imports, and the kind registry has a different replacement contract.

11. **Defer coordinates within `PositionMap`, preserving public APIs.** Add `PositionMap::deferred(Arc<str>)`, backed by an `Arc` holding exact text and `OnceLock<PositionSource>`. Keep `build_position_map`, `project`, and `top_level` as eager compatibility constructors; the lazy constructor invokes the existing eager builder once. `is_empty`, `byte_range`, and actual problem lookup force initialization; `locate_all(&mut [])` does not. Cloning shares text and computation. Envelope admission uses deferred maps; whole-YAML loading keeps its BOM offset and defers its current body-map snapshot; trigger error positioning uses one deferred holder per reported snapshot. Do not change `ProblemPositions` or normalize authored bytes. Evidence: `EnvelopeAdmission::positions` is public; `positions.rs::build_position_map` currently reparses YAML; `yaml_documents/load.rs::LoadedYaml::byte_range` adds the skipped BOM offset; `triggers/envelope.rs::trigger_error_span` adds its source offset. Preserve those offsets and existing line/byte-column semantics exactly.

12. **Use typed cache domains, with canonical fallback for untracked transformations.** Introduce `SchemaIdentity::{Canonical { revision: u32, digest: u64 }, Layered { revision: u32, digest: u64 }}` and `SchemaDigest { identity, judges_files }` in `validate.rs`. Internal cache keys retain that enum alongside the context/anchor hash rather than collapsing the enum into another `u64`. Keep existing public canonical APIs; add `validator_for_shared_with_digest` and `CompiledValidators::build_with_digest`. Keep `ValidatorCache::content_hash` canonical for callers that expect canonical content. Evidence: `CacheInner::entries` is currently keyed by `u64`, `shared_schema_digest` memoizes by allocation, and `lookup_or_compile` deliberately strips context for `judges_files == false`.

13. **Layer identities describe the schema actually compiled.** Encode a byte domain, key revision, merge algorithm revision, layer count, and each ordered `(role, digest)` using fixed numeric tags, little-endian fixed-width integers, and length-prefixed variable bytes before `biscuit_hash::xx_hash_bytes`. Roles are baseline, ordered trigger, document, and phase projection. Prepared records carry canonical lowered-schema digests/classifications computed during generation. Runtime layers hash their fully resolved JSON, including disk imports and resolved template results, once per allocation. `site_schema_with_override`, `merged_trigger_schema`, `BaselineSchema`, `TriggerLayer`, `EffectiveSchema`, and `Merge` carry the resulting identity alongside JSON. `phase_json_schema` adds the phase plus canonical digest of the projected document and a passive-transform revision. A structurally unchanged phase result reuses the original identity; a changed result gets a derived identity. Root-arm wrappers and all other transforms without tracked provenance use canonical final-schema hashing. Frontmatter data does not enter the identity except through a resolved schema/template or schema projection. Evidence: those functions in `schemas/mod.rs` merge baseline → triggers → document and then project/make passive; `prepared::CompiledValidators::build` wraps root-union arms. This closes the post-merge gap without redesigning unrelated APIs.

14. **Keep file-dependence classification conservative.** Factor the existing `schema_digest` string classification into a shared helper; generation records it, runtime layers calculate it, and layered/derived identities OR their input classifications. Keep the exact current full-context/caller-origin equality checks and base/fallback anchors. An OR false positive is acceptable. Canonical fallback reclassifies the final schema. Evidence: `validate.rs::schema_digest` treats `darkmatter-file` and the match keyword as context dependent while the lazy file-reference format is structural; `JudgedIn` compares complete request and caller origins.

15. **Remove all current uses of the reduced metadata helper.** Remove `seed_inline_metadata_schema` calls in `agent_text_is_data.rs`, `compose_prompt_tiers.rs`, `entry_point_parity.rs`, `sequence_schema.rs`, `sequence_magic_reference.rs`, `inline_completion_lifecycle.rs`, and `sequence_overlay_pty.rs`, then delete the helper and schema-only plumbing/imports. Retain all assertions and dedicated custom/shadowing fixtures elsewhere. Current search finds seven calling files, rather than the spec's historical eight. Evidence: introduction commits `27845edca` and `5ceb22a89` and `audit-final-cli-speed.md` describe speed reduction; current calls pass the unmodified shipped `inline-compose.yaml` and `document.yaml`. Their assertions concern data preservation, prompt/file lookup, overlay input, and launch/completion behavior, not custom-schema selection. No current call qualifies for retention. Record this outcome in the implementation log.

16. **Regeneration is a developer command, not startup I/O.** Add a `shipped_schemas` example in each library, accepting mutually exclusive `--check` and `--write`, plus `--artifact <path>` for checking an explicitly supplied artifact in command tests, with no stdin. With neither mode flag, emit freshly generated bytes to stdout without consulting checked-in lower artifacts; `--check` and `--write` first require fresh checked-in lower artifacts. The root coordinator lives at `scripts/shipped_schemas.py` and uses the default emit mode. Area `just shipped-schemas` runs its example after checking lower artifacts and prints the root regeneration command on lower staleness. Root `just shipped-schemas` invokes a Python coordinator: obtain generated bytes from all three examples in dependency order, validate all outputs in memory, stage sibling temporary files, then replace outputs. Commands expose generation bytes on stdout with errors on stderr; Python never parses a formatted report. Generation uses current sources, not the possibly corrupt embedded artifact. Land valid initial outputs with each `include_str!`; absent-artifact fallback is tested through an optional injected byte slice, not by deleting embedded build inputs. Evidence: existing Claudine `lifecycle_artifact` example establishes the example-based generator pattern; spec requires buildable generators and validation before any output replacement. The runtime loader does no filesystem access.

17. **Use optional work counters for observable guarantees.** Add work variants for artifact decode/accept, embedded runtime preparation, generation-only baseline/merge/compile checks, position-map construction, canonical schema serialization, and fallback reasons `MissingArtifact`, `MalformedArtifact`, `UnsupportedWireVersion`, `PreparationRevisionMismatch`, `SourceMismatch`, `DependencyMismatch`, `ConfigurationMismatch`, and `UnusableRecord`. Fold library counters into Darkmatter's existing snapshot. A source-only custom descriptor path is ordinary preparation, not a warning or a production-global mutation. Evidence: `simplified-schema::work_counters::Work` is feature-gated, atomic, and already composed into Darkmatter counters. Count actual call sites, not inferred timings.

18. **No performance spike or wider workload is needed.** Capture the specified before sample once before Phase 1 changes, and the matching after sample once after Phase 5 changes. Use macOS on this host, existing build profiles, and isolated fixtures; build outside measured intervals. Use the frozen clarify fixture with a failing dry-run Goose provider stub, initialized temporary repository, and `specs/demo-spec.md` containing `# Demo\n`, staged by `compose_schema_cli::compose_frozen_clarify_reads_its_ordinary_doc_property`, with `--goose` and `spec=specs/demo-spec.md`. Use `# Sample\n\nHello.\n` for the three-line Markdown input. Record release Claudine timing for the requested debug/release comparison but impose only its specified debug threshold. Repair regressions in the artifact/layer/position path and rerun only affected checks; do not weaken assertions, timeout rules, or replace the workload. Evidence: the spec's measurements already justify precompilation and explicitly rule out a new performance spike, multi-OS timing gate, or broader benchmark workload.

### Wave 1

- [x] **Capture baseline**
  - Before behavior changes, build `md` and `claudine` in debug/release, stage the two inputs from Rule 18, and record five-run medians, exact commands/environment, host details via `sniff`, toolchain/build settings, and fixture contents in `implementation-log.md`. No provider actually launches; use the existing dry-run fixture/configuration.
  - Run the current Darkmatter and Claudine L1 recipes once on a quiet host; preserve per-package JUnit XML outside overwritten report paths and record summed durations and tests above five seconds. Keep these results separate from final correctness gates.
  - Preserve existing uncommitted spec/log content. Planning found `spec.md` modified and an existing untracked `implementation-log.md`; implementation appends its evidence without replacing either author's content.

- [x] **Implement wire**
  - Depends on baseline capture; implement Rules 1–7 in `simplified-schema`. Add module export, owned records, ordered Serde support, descriptors, uncached routes, complete validation, and injected-source/byte APIs.
  - Keep common runtime code valid with `--no-default-features`; put generator-only file writes in examples/coordinator. Do not add host dependencies.
  - Introduce `simplified-schema/lib/generated/shipped.json` with the code embedding it. Move `envelope::DEFINITIONS` prepared data behind the registry, retaining lazy validator compilation and the non-recursive bootstrap.
  - Add `simplified-schema` area's recipe and generated-file attributes. Add the first freshness/round-trip/fallback tests under `lib/tests/l1/shipped_artifacts.rs` and register the module in `tests/l1/main.rs`.

### Input Robustness Matrix

The new format reader is the shared artifact JSON decoder, not the existing YAML language parser. Its public result is `load_or_prepare` returning correctly prepared data plus instrumented refusal reason. Default for every invalid artifact shape: reject the entire artifact, report the precise JSON pointer and parser line/column in the internal refusal detail, expose no artifact entries, run the supplied ordinary preparation, and write nothing. Production publishes no new diagnostic; generator/check commands render the refusal location and fail without replacing outputs. Preparation failures retain their original public error. This is the deliberate exception to a loader that simply stops on invalid input.

- Test every required header scalar validator with absent, explicit null, wrong whole type, and an empty string where the scalar is a string. Zero revisions, wrong owner, non-hex/short/long digests, and unknown variants are invalid. Reuse shared identity/digest/revision validators: one full matrix per validator, then a positive and one invalid wiring case per consuming field.
- Test source/dependency/configuration lists and prepared ordered-pair collections with absent, null, wrong whole type, one invalid element among valid elements, all invalid elements, and empty lists. Empty configuration and lower-dependency lists are valid when the expected inventory is empty; source/entry lists must match the nonempty inventory. Empty schema objects, empty named-type tables, and matcher-only payloads are valid where the existing preparation produces them. Empty lists never mean missing data.
- Represent optional record metadata as explicit tagged `None`/`Some` variants, so absence/null is never silently conflated. JSON Schema `Value` fields retain literal null as schema data, with their enclosing required-field validation separate from schema semantics. Do not use `#[serde(default)]` to accept missing load-bearing records.
- Test record invariants through the same loader result: duplicate source/document/definition/JSON keys, dangling references, mismatched source identities, invalid enum tags, out-of-range wire indexes, malformed ordered pairs, missing payloads, and forbidden native file namespaces reject the owning artifact. Valid recursive references remain valid.
- One document-shape matrix covers the decoder: recursive duplicate JSON object keys are rejected before conversion to `Value`; trailing content, truncated JSON, JSON5 comments/trailing commas, invalid UTF-8, and empty bytes are rejected. Ordinary trailing JSON whitespace is accepted. Implement one duplicate-detecting recursive Serde visitor; `serde_json::Value` alone would lose duplicates. Both syntax and typed record errors retain paths/coordinates. No second format loader is introduced.
- Mutate one field at a time in freshly generated in-memory artifacts. Keep an unedited control proving the accepted artifact changes the public validation/preparation result. Prove malformed inputs never invoke the accepted-entry callback. Exercise runtime fallback plus a preparation-error control. Do not mutate repository files or production registries.
- Spawn each generation example only for valid `--check` wiring and one malformed-input refusal through a temporary test input; field/record matrices run in-process. Keep missing-byte injection in-process. Assert output files remain unchanged on rejection. Before closing, search touched loaders for Serde defaults, `Option<T>`, `filter_map` with `as_str`, `unwrap_or_default`, and `.ok()` on load-bearing parsing; remove silent acceptance and account for each retained occurrence.

### Validation checkpoint

Run targeted nextest tests for envelope bootstrap, admitted-envelope parity, artifact matrices, ordering, regeneration errors, and fallback. Run `just test` and `just lint` in `simplified-schema/` at this phase boundary; this includes its no-default-features suite. Fresh generation must still work with injected stale/corrupt current bytes and must not initialize its runtime registry. Update Simplified Schema's API docs and new `docs/topics/shipped-artifacts.md` with a regeneration example and load/fallback flow diagram.

## Phase 2: Darkmatter registry and lazy positions

### Wave 2

- [x] **Prepare host records**
  - Depends on Phase 1. Own `darkmatter/lib/src/markdown/schemas/shipped.rs`, `embedded.rs`, `catalog.rs`, and host generation records. Implement uncached Darkmatter preparation over Rule 9's inventory and fresh Simplified Schema data.
  - Store shared/inline `fm` forms, lowered JSON/digests, named-type tables, globals, admitted kind seeds, trigger definition, and function/effects records. Include import/source provenance, categories, overloads, examples, and definition order. Match existing catalog fields exactly; derived name indexes rebuild after decode.
  - Provide Darkmatter's public prepared-bundle generation/loading helper for Claudine with an explicit supplied document table and global requirements. It returns a host payload, not a `TriggerRegistry` with captured context.
  - Coordinator adds the Darkmatter example, recipe, generated artifact, and attributes. Validate payloads with the current draft/formats/keywords and call the existing baseline/simple-object/merge/compile checks during uncached generation.

- [x] **Defer position maps**
  - Depends on Phase 1; may run concurrently with host records. Own `simplified-schema/lib/src/positions.rs`, envelope position integration, `darkmatter/.../yaml_documents/load.rs`, and `triggers/envelope.rs`; do not edit Darkmatter registry/catalog files.
  - Implement Rule 11 and the position-construction counter. Keep one exact source snapshot per map, with existing BOM/range/base-line handling. Bind shipped source text from its source identity after loading rather than embedding position trees in JSON.
  - Test public envelope admission, trigger-envelope errors, and whole-YAML validation with valid no-map controls and error locations. Cover explicit map queries after successful admission, clone sharing, concurrent lookups, CRLF, BOM, Unicode byte columns, aliases, and missing/unknown keys. Reuse existing coordinate fixtures unchanged.

### Wave 3

- [x] **Route shipped consumers**
  - Depends on Wave 2. Replace `BASELINE`/`INLINE_BASELINE` in schema `mod.rs`, `GlobalCatalog::darkmatter`'s cache, `shipped_kind_definitions` storage, registry `SHIPPED`/`PREPARED`, expression catalog `CATALOG`, and effects `CATALOG` with registry views. Keep ordinary source/disk/custom accessors working.
  - Route `resolve::shipped_named_types` through the explicit descriptor before its source-only memo. Consolidate shipped entries in the owner registry; retain unregistered/file named-type memos and effects binding registrations.
  - In trigger discovery, consult an accepted prepared-bundle descriptor after filename selection and before ordinary envelope/payload preparation. Prepared data carries reusable conditions and payload, while path predicates stay bound to the registry's context. Keep `EMBEDDED_LOADED` only for ordinary custom bundles.
  - Skip `enforce_compiles`, `validate_simple_object_schema`, `enforce_merge_compatible`, and baseline checks only through an accepted handle. Ordinary fallback and disk/custom paths execute original checks. Keep runtime effective validator compilation.
  - Update reset docs/counter folding; regenerate artifacts after the position/preparation revision changes. Update `definition.md`, schema discovery/trigger/validation topic pages, and DMLS schema-support documentation alongside the behavior.

### Validation checkpoint

Add Darkmatter's `shipped_artifact_is_fresh` and semantic round-trip test. Assert cold accepted baseline/catalog/namespace/kind lookups cause zero embedded preparation, zero moved checks, and no position construction until queried. Exercise a stale dependency whose stale bytes still match the upper header, a mutated supplied source, custom same-name bundle, malformed records, and fallback failure. Run targeted nextest tests for source guards, `effective_schema_layering`, kind checkout replacement, unsaved buffers, catalogs/effects, and positions. Run the affected DMLS schema/editor suites through Darkmatter's canonical recipe filters. No browser or real terminal is required.

## Phase 3: Claudine artifact and shadowing

### Wave 4

- [x] **Integrate Claudine registry**
  - Depends on Phase 2. Add `composition/schema/shipped.rs`, its host payload/inventory, and generation example; use Darkmatter's preparation helper with `CLAUDINE_SCHEMA_FILES`, `HOST_CATALOG_TEXT`, and `CLAUDINE_GLOBALS`.
  - Extend `claudine_schema_bundle()` with the prepared descriptor. Replace globals `CATALOG` with a registry view without changing `claudine_globals`, `host_global`, origin labels, or `is_claudine_owned` semantics.
  - Add `claudine/lib/generated/shipped.json`, recipe, attributes, `shipped_artifact_is_fresh`, and round-trip/counter tests. Verify complete preparation bypass across both lower owners during generation.
  - Update `claudine/docs/topics/schemas.md` with regeneration, runtime fallback, and the unchanged disk/bundle selection rules. DMLS continues to consume host globals with its existing workspace invalidation; no stored binding availability is added.

- [x] **Prove source selection**
  - May run concurrently with Claudine integration by owning Darkmatter trigger tests only. Use explicit test descriptors/in-memory data; do not depend on the new Claudine registry while it is being written.
  - Preserve existing disk `claudine.yaml`/`document.yaml` tests without edits. Add one mixed test where a disk trigger is lowered and surviving siblings use accepted entries; assert per-source counters and validation/origin results, not only counts.
  - Add controls for an ignored or invalid same-name file still suppressing an embedded trigger, embedded imports ignoring disk `lifecycle.yaml`, changed custom bundle imports invalidating dependent entries, custom same-name bundles, and differing request path contexts. Retain current bundle-order/case-fold behavior.

### Wave 5

- [x] **Wire regeneration scope**
  - Depends on Wave 4; coordinator owns root/area recipes, manifests, and test roots. Add the root coordinator from Rule 16 and test generation failure preserves all three files. Successful root regeneration runs twice with byte-identical output.
  - Make each owner's freshness test regenerate its full lower chain in memory. In each freshness test body, explicitly read lower shipped YAML/artifacts and lower preparation source files using `manifest_dir!`/root-bound literals recognized by `docs/cicd/test-inputs.md`. Do not hide reads in a shared helper that broadens the filter to the whole binary.
  - Darkmatter's manifest `source-inputs` lists exact current `.rs` paths under `simplified-schema/lib/src/`; Claudine's lists exact paths under both `simplified-schema/lib/src/` and `darkmatter/lib/src/`. Generate these literal manifest inventories from those two trees, excluding test-only files, and retain existing entries. Freshness test bodies spell the corresponding lower source-tree reads; the planner then maps each exact declared path to that single test. This deliberately conservative code inventory covers all language/admission/lowering behavior without selecting a dependent's full suite. YAML/artifact dependencies are exact literal file reads from the inventories in Rule 9, not broad repository scans.
  - Add focused assertions in `scripts/ci/test_affected_scope.py::RealWorkspaceTestInputTests` for changes to Simplified Schema's envelope YAML, `resolve.rs`, and artifact; Darkmatter's functions example YAML, `catalog.rs`, and artifact; and Content Policy's editor schema. Assert the appropriate downstream `shipped_artifact_is_fresh` narrowed L1 filters. Artifact-only changes still select their owner normally because the artifact is embedded in product code.
  - Use `python3 scripts/ci/affected_scope.py --resolved-plan <repository-relative-path>` for representative paths and inspect the canonical output. Do not add a new job, full downstream L1 scope, or OS matrix. Run the planner's focused unittest class.

### Validation checkpoint

Run the three freshness tests and targeted nextest schema/bundle/global/launch/completion tests. Prove a normal Claudine shipped bundle incurs zero runtime embedded preparation and moved checks, with stale-source/dependency/configuration/revision/record variants falling back correctly. Run root and area `shipped-schemas` command controls, including a temporary malformed/stale descriptor fixture. Verify generated outputs contain only virtual identities, authored spans, fixed-width wire values, and deterministic metadata.

## Phase 4: Layer-aware validator identities

### Wave 6

- [ ] **Add typed identities**
  - Depends on Phase 3. Own `simplified-schema/lib/src/validate.rs`, `prepared.rs`, and shared identity helpers/records. Implement Rules 12–14; retain the canonical APIs and `content_hash` behavior.
  - Keep the identity enum in map keys through final lookup, with current eviction and full context equality. Route structural/context validators through the same typed key representation.
  - Add public-result tests for canonical/layered domain separation, stable ordered encoding, role/order changes, unchanged schema across differing data, canonical object-key invariance, array-order significance, file-reference syntax versus eager file/match checks, and caller-origin/context separation. Use existing compile counters and validator `Arc` reuse controls.
  - Add a tiny L1 test executable mode that prints the same fixed layered identity; invoke it twice through the test toolkit to prove cross-process stability. This tests wiring once, not every matrix cell in a subprocess.

### Wave 7

- [ ] **Carry merge provenance**
  - Depends on Wave 6. Own Darkmatter `schemas/mod.rs` merge/phase/effective structures and their construction sites. Carry identity/digest from baseline/payload/fully resolved document into merged JSON and `CompiledValidators::build_with_digest`.
  - Derive post-merge phase identities as Rule 13 specifies. Keep provenance optional for arbitrary public JSON input; untracked schema mutation, root-union arm wrapping, and standalone APIs take canonical final-schema hashing.
  - Include overridden/filtered trigger payloads by hashing the actual filtered payload or canonical fallback, not the original full payload's digest. Template resolution hashes the resulting schema. Do not use file mtime, path, frontmatter data, or source-file digest as a substitute for resolved schema content.
  - Regenerate artifacts after changes to generation records/revisions; retain byte freshness and ordering. Update validation/cache API comments and relevant validation topic docs.

- [ ] **Exercise cache isolation**
  - May run concurrently with merge integration by owning new tests only. Cover public validation and compile-counter behavior for reordered conflicting layers, baseline/document role changes, changed disk imports with unchanged importer, changed resolved template schemas, launch versus completion required/generated behavior, and passive eager-file projection.
  - Include positive sharing controls for two frontmatter documents using identical schema layers, and negative controls for document base directory, fallback directory, repository context, and caller origins. Assert verdicts that would differ with a wrong validator, not just unequal numeric keys.
  - Assert a fresh shipped baseline-only validation uses its generated digest and performs no canonical serialization of that baseline. Dynamic layers may serialize once; a repeated shared allocation must reuse its digest. Exercise canonical fallback explicitly.

### Validation checkpoint

Run focused nextest validator, phase, template, effective-layering, file-context, and cross-process identity tests. Test reset → cold → warm behavior, ensuring constants stay loaded and dynamic validator/payload/merge caches miss after reset. The canonical counter must prove the startup path no longer serializes the large merged shipped schema solely for a cache key. Root regeneration and all freshness tests pass after the final identity format settles.

## Phase 5: Fixture removal, documentation, and final evidence

### Wave 8

- [ ] **Restore shipped fixtures**
  - Depends on Phase 4. Remove the exact helper calls and obsolete schema arguments/includes listed in Rule 15; delete `CliProcessFixture::seed_inline_metadata_schema` after its callers are gone. Keep authored schemas, provider stubs, phase assertions, persistence assertions, and intentional disk/custom fixtures.
  - Search all Claudine test setup for copies of `inline-compose.yaml` into `schemas/claudine.yaml` to remove the same speed workaround spelled without the helper. Retain tests whose asserted behavior is disk shadowing/custom definitions. The named helper's complete removal is already settled; this search checks equivalent spellings rather than reopening it.
  - Record the seven-file roster, introduction history, removals, and intentional retained shadowing fixtures in the implementation log. Do not edit closed historical audit reports.

- [ ] **Finish documentation**
  - May run concurrently with fixture removal by owning docs/skills only. Finish `simplified-schema/docs/topics/shipped-artifacts.md`; update Darkmatter schema baseline/discovery/trigger/validation and DMLS schema-support pages and `claudine/docs/topics/schemas.md`. Give regeneration, freshness failure, fallback, and revision-bump examples, plus a diagram of the dependency/load flow. Docs do not name this feature directory.
  - Update `.claude/skills/{simplified-schema,darkmatter,claudine}/SKILL.md` and the Darkmatter schema topic with `just shipped-schemas`, owner/revision rules, and shipped YAML regeneration requirements. Update `rust-testing/test-speed.md` only with counter/timing-backed claims; describe remaining effective-validator compilation and stale fallback honestly.
  - Update area dependency docs for changed direct dependencies; the chosen JSON/Serde mechanism requires no new serialization crate. Keep READMEs as concise overviews linking to topic docs.
  - Review changed symbols' API and inline comments for drift, including the envelope bootstrap, baseline accessor, static catalogs, lazy maps, cache keys, and benchmark reset contract.

### Wave 9

- [ ] **Run final gates**
  - Depends on Wave 8. Coordinator runs `just shipped-schemas`, then `just test` and `just lint` separately in `simplified-schema/`, `darkmatter/`, and `claudine/`. Use canonical nextest recipes; Darkmatter's recipe covers `darkmatter-lsp` and the Zed CLI. Do not run `cargo fmt` or a workspace-wide test suite.
  - Preserve final per-package JUnit files immediately after each run. Run the focused CI planner tests and representative canonical plans from Wave 5. Source-scan, layout, dependency-direction, no-new-I/O, path/context, and language-ownership guards must pass.
  - Inspect the no-default-features dependency graph to confirm no Darkmatter/Claudine/Content Policy/CLI dependency was introduced into Simplified Schema. Compile the generator examples with ordinary checks; no extra CI matrix is added for them beyond the repository's existing target-kind policy.
  - Ordinary existing CI proves byte freshness on macOS/Linux/native Windows/WSL2; report exactly the environments actually tested locally. No new path comparison or platform branch is required by this design.

- [ ] **Record performance evidence**
  - Depends on Run final gates; runs alone on the quiet host. Rebuild debug/release outside measured intervals and repeat exactly Rule 18's fixture commands, five executions each. Record medians and differences against Wave 1, summed JUnit time changes, and all five-second outliers. Keep build and execution time separate.
  - Retain source and dependency refusal tests alongside zero-preparation counters; timings alone cannot establish artifact consumption. Resolve failing correctness gates and performance regressions in the changed paths, then repeat only affected validation and the affected five-run sample. Targets remain acceptance criteria, not warnings to leave for another session.
  - Append final implementation decisions, spec departures, commands, environment/build settings, counters, and results to `implementation-log.md`. Note JSON remains the chosen wire format and the seven-file fixture count corrects the historical spec example.

### Validation checkpoint

Re-read the finished changes against the definition of done and Input Robustness Matrix. Run `git diff --check` and inspect the final diff for unrelated changes and accidental host paths/generated nondeterminism. Confirm regeneration leaves artifacts unchanged, required test modules are registered, runtime fallback never publishes partial entries, no new I/O enters passive operations, and no phase depends on a human response. Leave the active feature in place with concrete test/performance evidence and an implementation-complete report for review.