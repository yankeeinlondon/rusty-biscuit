---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-11-precompiled-shipped-schemas/spec.md
plan: darkmatter/features/2026-11-11-precompiled-shipped-schemas/plan.md
implemented_by: claude/opus
started_phase: 1
decisions_for_review:
    - |-
      The generator command is tested in-process, not by spawning it. The plan asked for spawning the `shipped_schemas` example for a valid `--check` and one malformed-input refusal. Examples are not built by `cargo nextest` or present in CI test archives, so a spawning test would have to run `cargo` itself (slow, and impossible in archive mode). Instead the L1 test compiles `examples/shipped_schemas.rs` into the test binary with `#[path]` and calls its `run` function with injected arguments, paths, generation callback, and output buffers; the example's `main` is a thin wrapper around `run`. Alternative: make the generator a `[[bin]]` so `CARGO_BIN_EXE_*` exists, at the cost of shipping a developer tool as a binary target. Redoing it means moving the example to a bin target and switching three tests to spawn it.
    - |-
      Added `serde_path_to_error` as a new direct dependency of `simplified-schema` so a refused artifact reports the JSON pointer of the member that failed typed decoding (the plan's matrix requires pointer plus line/column). The plan said the JSON/Serde mechanism needs no new serialization crate; this crate is not a format, it wraps the deserializer to track the path, and is already in the lockfile (used by `messenger`). Alternative: hand-written header decoding with manual pointers and no pointers for payload records. Reversing it means dropping the dependency and the pointer assertions in the matrix tests.
    - |-
      File-backed identities are unrepresentable on the wire rather than rejected by a separate validation walk. `DefNamespace` (`File`, `FileRootUnionArm`, `TriggerPayload`, `TriggerPayloadUnionArm`), `SchemaSourceId::File`, and a slot's origin path have hand-written Serde impls that refuse those variants on both serialization and deserialization, so generation fails loudly and a hand-edited artifact is malformed. Consequence: these language types cannot be serialized with file-backed values for any other purpose. Alternative: derive Serde normally and walk each decoded record for forbidden variants. Reversing it means replacing `shipped/wire_support.rs`'s impls with derives plus a validator.
    - |-
      Uncached generation bypasses identity-keyed state only (the shipped named-type memo, prepared tables, artifact registries) and still uses content-keyed process memos (the lowering's regex-pattern check memo and the structural validator caches). The plan's Rule 7 lists "validator memo" among things uncached calls never consult. A content-keyed memo returns the same answer the computation would, so it cannot hide stale preparation; threading the mode into every lowering and compile call would touch most of the resolver for no change in output. Reversing it means threading `PreparationMode` into `to_json_schema*` and the validator-cache entry points.
    - |-
      Simplified Schema's own artifact stores only each envelope definition's lowered JSON Schema, not its parsed AST or definition table, because admission needs only the JSON. The AST Serde support is exercised by the named-type table records and their round-trip tests instead. An artifact naming another owner is refused as `UnusableRecord` (the plan's reason list has no dedicated owner reason). Changing either means adding records or a `FallbackReason` variant and regenerating.
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
message_to_agent: |-
    Phase 1 landed `simplified_schema::shipped` (read `simplified-schema/docs/topics/shipped-artifacts.md` first). For Phase 2: (1) a host artifact depends on the library's: use `FreshLower::prepare()` once for generation (it gives both the fresh envelope definitions for `PreparationMode::Uncached(&fresh)` and `fresh.as_dependency()`), and `simplified_schema::shipped::registry::registry().as_dependency()` at run time; (2) the uncached entry points are the `*_with_mode` variants (`prepare_embedded_schema_with_mode`, `prepare_embedded_yaml_schema_with_mode`, `prepare_embedded_inline_schema_with_mode`, `prepare_embedded_catalog_declaration_with_mode`, `parse_standalone_schema_document_with_mode`, `parse_referenced_schema_document_with_mode`, `admit_envelope_text_with_mode`); (3) Darkmatter's `SHIPPED_SOURCES` should attach its tables with `ShippedSources::with_prepared(&DESCRIPTOR)`; build `PreparedNamedTypes::from_records` inside the `accept` validation callback (it needs the `Acceptance` token) and record tables with `NamedTypeTableRecord::prepare(doc, mode)`; (4) the new library work counters (`ArtifactDecode`, `ArtifactAccept`, `ArtifactGeneration`, `ShippedRuntimePreparation`, `ArtifactFallback*`) are NOT yet folded into `darkmatter/lib/src/work_counters.rs` (`library_kind`) — add Darkmatter kinds for them; (5) the root `just shipped-schemas` is a stub that only delegates to `simplified-schema/`; Phase 3 replaces it with the `scripts/shipped_schemas.py` coordinator, and Darkmatter's area recipe should follow the `simplified-schema/justfile` pattern; (6) the generator test pattern is in-process: `#[path = "../../examples/shipped_schemas.rs"] mod generator;` then call `generator::run(...)` (examples are not built by nextest/CI archives). The baseline timing fixtures and script live in `/tmp/pss-timing/` (stage.sh, run.sh) and baseline binaries in `/tmp/pss-baseline/bin/`; they may not survive a reboot — the log records every command and fixture to recreate them.
---

# Implementation Log for 2026-11-11-precompiled-shipped-schemas (5 phases)

## Phase 1

### Baseline capture (Wave 1, before any source change)

Host (from `sniff os` / `sniff hardware`): macOS 27.2 (kernel 27.2.0), Apple M4 Max, 16 logical/physical cores, arm64, hostname `shazam.home`. Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1`, `cargo-nextest 0.9.136`. Build settings: the workspace's existing `dev` (`debug = "line-tables-only"`, dependencies `debug = 0`) and `release` profiles; no overrides.

The host was **not quiet**: load average 33–41 during the timing sample (Blender at ~100% CPU plus another session's `kache`), and 60–107 while the L1 suites ran. The five-run spreads are tight (≤ 3%), so the medians are usable as a relative baseline; the after-sample in Phase 5 must be taken under recorded load too.

Builds (outside the measured interval): `cargo build -p darkmatter-cli --bin md`, `cargo build -p claudine-cli --bin claudine`, and the same with `--release`; binaries copied to `/tmp/pss-baseline/bin/{debug,release}/`.

Fixtures (`/tmp/pss-timing/stage.sh`, staged outside the checkout):

- `md/sample.md` containing `# Sample\n\nHello.\n`
- `cwd/` = an initialized, committed git repository holding a copy of `claudine/cli/tests/fixtures/frozen_prompts/prompts/` and `specs/demo-spec.md` containing `# Demo\n` (the staging of `compose_schema_cli::compose_frozen_clarify_reads_its_ordinary_doc_property`), plus `bin/goose`, a dry-run stub that prints `SHOULD NOT RUN` and exits 1 (it never ran)

Commands (`/tmp/pss-timing/run.sh <bin-root> <label>`; `hyperfine -N --warmup 1 --runs 5`):

- `env -i HOME=$F/home PATH=/usr/bin:/bin NO_COLOR=1 md compose sample.md` (cwd `$F/md`)
- `env -i HOME=$F/home PATH=$F/bin:/usr/bin:/bin NO_COLOR=1 PLAYA_DRY_RUN=1 PLAYA_SPOOL_DIR=$F/spool CLAUDINE_RENDEZVOUS_REPORT=false RENDEZVOUS_ENDPOINT=$F/rv/daemon.sock claudine compose --goose --dry-run prompts/clarify.md spec=specs/demo-spec.md` (cwd `$F/cwd`; the environment mirrors `CliProcessFixture::command`)

| Workload | Build | Median (5 runs) | Runs (ms) |
| --- | --- | --- | --- |
| `md compose` three-line document | debug | 291.2 ms | 289.3, 291.2, 289.9, 293.8, 293.0 |
| `md compose` three-line document | release | 56.9 ms | 60.3, 56.7, 56.9, 57.3, 56.9 |
| `claudine compose --goose --dry-run prompts/clarify.md` | debug | 1415.9 ms | 1415.9, 1417.3, 1415.0, 1424.8, 1415.5 |
| same | release | 207.5 ms | 207.8, 212.9, 206.1, 202.8, 207.5 |

All runs exited 0.

L1 baselines (canonical recipes, run once each, sequentially; JUnit preserved at `/tmp/pss-baseline/{darkmatter,claudine}-junit.xml`):

| Area | Tests | Wall clock | Summed JUnit durations | Tests ≥ 5 s |
| --- | --- | --- | --- | --- |
| `darkmatter` (`just test`) | 16348 passed, 10 skipped | 201.2 s | 2813.0 s | 0 |
| `claudine` (`just test`) | 11917 passed, 15 skipped | 349.1 s | 4879.0 s | 0 |

### Shared artifact mechanism (Wave 1, "Implement wire")

New module `simplified_schema::shipped` (`simplified-schema/lib/src/shipped/`):

- `mod.rs` — `WIRE_VERSION = 1`; `Digest` (xxHash-64 via `biscuit_hash::xx_hash_bytes`, wire form exactly 16 lowercase hex digits); `SourceFingerprint`, `DependencyFingerprint`, `ArtifactHeader` (all `deny_unknown_fields`, no `serde(default)`); `ArtifactDescriptor` (owner, revision, complete source table, `LowerArtifact` dependencies, configuration); `LowerArtifact::{Current, Refused}`; `PreparationMode::{Runtime, Uncached(&FreshLower)}`; `FreshLower` (freshly bootstrapped envelope definitions plus the library artifact they generate, and `as_dependency()` for hosts); `FallbackReason` (the eight reasons of Rule 17); `Refusal` (reason, JSON pointer, line, column, detail); `Acceptance` and `AcceptedArtifact<T>`, both constructible only inside `accept`; `generate`, `accept`, `load_or_prepare`/`Loaded`, `check_fresh`/`StaleArtifact`.
- `wire.rs` — deterministic encoding (`to_vec_pretty` + one LF) and strict decoding: a duplicate-key walk over the whole document (a `DeserializeSeed` visitor that records the pointer of the repeated key), `Deserializer::end()` for trailing content, explicit empty/UTF-8 checks, then a typed header decode with the payload held as a borrowed `RawValue`, so a wrong wire version is reported as such before the payload is read. Typed errors carry a `serde_path_to_error` pointer and an absolute line/column (payload offsets are rebased onto the whole artifact).
- `records.rs` — `NamedTypeTableRecord` (one shipped document's unexpanded named types, authored order) and `PreparedNamedTypes` (built only through `Acceptance`; serves a table only when the supplied `ShippedSources` documents fingerprint exactly to the inventory it was prepared from, memoized per table address) plus `PreparedSourcesDescriptor { owner, provider: fn() -> Option<&'static PreparedNamedTypes> }`.
- `registry.rs` — Simplified Schema's own artifact: `OWNER`, `PREPARATION_REVISION = 1`, `ARTIFACT_PATH`, `REGENERATE_COMMAND`, `ARTIFACT` (`include_bytes!("../../generated/shipped.json")`), `EnvelopePayload`/`EnvelopeDefinitionRecord`, `Registry::load(Option<&[u8]>)` (injectable bytes), the process `registry()`, `registry_is_loaded()`, `generate()`, `check_artifact()`.
- `wire_support.rs` — `present` (an `Option` field must be written; absent is a missing-field error, `null` is `None`), `no_host_path` (a slot's origin file is only representable as `null`), portable-only Serde for `DefNamespace` and `SchemaSourceId`, and `DefinitionTable` as ordered `[key, definition]` pairs with the `$defs`-key index rebuilt and duplicates refused.

Serde derives (`Serialize, Deserialize`, `deny_unknown_fields`) were added to the AST graph: `SchemaRoot`, `SchemaShape`, `PatternKeyDef`, `PatternKey`, `SchemaArm`, `PropertyDef`, `TypeExpr`, `TupleType`, `TupleElement`, `FunctionType`, `NarrowsKind`, `NarrowsClause`, `FunctionParameters`, `FunctionParameter`, `CallExample`, `ExampleOutcome`, `PropertyAtom`, `SimplifiedType`, `Constraint`, `Slot`, `DynamicKind`, `DynamicElement`, `DynamicList`, `TemplateConstraintKind`, `SuggestionCandidate`, `AlphanumericOption`, `SchemaSourcePathSegment`, `SchemaSourcePath`, `SchemaReferenceSite`, `DefKey`, `Definition`. Every `Option` field among them uses `present`. `AlphanumericOptions` (a bitset no AST node holds) gets none.

Envelope definitions now come from the registry (`envelope.rs`): `EnvelopeDefinitions` holds each definition's lowered JSON and compiles its validator lazily (`OnceLock`) through a private cache, as before. `EnvelopeDefinitions::prepare_uncached` is the old non-self-admitting bootstrap (`parse_standalone_unadmitted` + `lower_embedded_definition`), now lowered against an empty document table; `check_contracts` is the generation check that each definition is admitted by the definitions (previously only a test). `EnvelopeDefinitions::admit` judges text against a specific registry's definitions. New `admit_envelope_text_with_mode`.

Uncached route (Rule 7): `PreparationMode` is threaded through `parse_standalone_schema_document_with_mode`, `parse_referenced_schema_document_with_mode`, and `prepare_embedded_{schema,yaml_schema,inline_schema,catalog_declaration}_with_mode`; the resolver carries it inside `NamespaceSource::Embedded`, so embedded imports admit against the fresh definitions and a shipped partial (`Name@<namespace><stem>`) is parsed every time (no `SHIPPED_NAMED_TYPES` memo, no prepared table). Disk (`Files`) expansions always run in `Runtime`. The existing source-only functions delegate with `Runtime`; no public signature changed.

`ShippedSources` gained `with_prepared` (const) and `prepared()`; `ShippedSources::new` and `NONE` are unchanged and never consult a table. `resolve::shipped_named_types` now: uncached → parse; runtime → accepted prepared table when the documents match → otherwise the existing memo, counting a `ShippedRuntimePreparation`.

Work counters (`simplified-schema/lib/src/work_counters.rs`): `ArtifactDecode`, `ArtifactAccept`, `ArtifactGeneration`, `ShippedRuntimePreparation`, and one `ArtifactFallback*` per `FallbackReason`. Darkmatter's snapshot does not fold these yet (Phase 2 owns Darkmatter's counter changes).

Generation: `simplified-schema/lib/examples/shipped_schemas.rs` (no flag: fresh bytes to stdout; `--check [--artifact <path>]`; `--write`, which validates fresh bytes and replaces the file through a sibling temporary). Area recipe `just shipped-schemas [--check]`; a minimal root `just shipped-schemas` that delegates to it until Phase 3's coordinator replaces it. `simplified-schema/lib/generated/shipped.json` (5863 bytes) is generated and marked `linguist-generated` in `.gitattributes`; regeneration is byte-identical.

Content-keyed process memos (the regex-pattern check memo in lowering, the structural validator caches) are still consulted during uncached generation: their answer is a pure function of the text they are keyed by, so they cannot carry yesterday's preparation. Identity-keyed state (the shipped named-type memo, prepared tables, the registry) is bypassed.

Docs: new `simplified-schema/docs/topics/shipped-artifacts.md` (regeneration, freshness failure, revision bumps, header, load/fallback Mermaid flow, refusal reasons, fresh generation, prepared named types); updated `language/schema-documents.md` (the bootstrap paragraph had drifted: definitions now load from the artifact and their self-admission is a generation check, not test-only), `language-core.md` (`with_prepared`), `dependencies.md` (`serde_path_to_error`, `indexmap`'s `serde` feature), and the README overview. Skill: `.claude/skills/simplified-schema/SKILL.md` gained a "Shipped artifacts" section.

### Tests (requirement → test)

All in `simplified-schema/lib/tests/l1/shipped_artifacts.rs` (98 tests, registered in `tests/l1/main.rs`; L1, in-process at the public API; the generator test compiles `examples/shipped_schemas.rs` in through `#[path]`):

| Requirement | Tests |
| --- | --- |
| Freshness, with the command on failure | `shipped_artifact_is_fresh`, `a_mutated_source_fails_freshness_naming_the_regeneration_command` |
| Round trip incl. order | `the_checked_in_artifact_round_trips_to_uncached_preparation_in_order`, `a_named_type_table_round_trips_in_authored_order`, `a_definition_table_round_trips_as_ordered_pairs` |
| Accepted artifact is used (control) | `an_accepted_artifact_is_what_admission_judges_by`, `an_attached_prepared_table_serves_the_reserved_namespace` |
| Fallback is correct | `the_process_registry_accepts_the_artifact_and_admits_as_the_fallback_does`, every `refused!` row (admits through the fallback registry) |
| Zero runtime preparation when accepted; one when refused | `an_accepted_artifact_prepares_nothing_and_a_refused_one_prepares_once`, `a_served_table_parses_nothing_and_a_mismatched_one_parses_once` (`work-counters`) |
| Uncached generation bypasses artifact/registry/memo | `fresh_generation_ignores_the_artifact_and_the_registry`, `uncached_resolution_parses_even_with_a_prepared_table_attached` |
| Lower-dependency refusal propagates even when bytes match | `a_refused_lower_artifact_refuses_the_host_even_when_its_bytes_match_the_header`, `a_changed_lower_artifact_refuses_the_host`, `a_host_artifact_is_accepted_over_its_current_lower_artifact` |
| Custom same-name sources get ordinary preparation | `a_caller_table_under_the_host_s_paths_is_parsed_not_served` |
| Record invariants | `table_records_refuse_a_document_outside_the_inventory_or_twice`, payload rows below, `a_file_backed_identity_is_never_shipped`, `an_absent_optional_member_is_an_error_and_null_is_none` |
| Generation refuses bad descriptors | `a_descriptor_that_cannot_generate_is_refused_before_encoding` |
| Generator command wiring | `the_generator_checks_emits_and_writes`, `the_generator_reports_where_a_malformed_artifact_is_refused`, `a_failed_generation_leaves_the_artifact_in_place` |
| Error coordinates | `a_syntax_refusal_carries_the_parser_position`, `a_payload_record_refusal_is_located_in_the_whole_artifact` |

Input robustness matrix (one generated `#[test]` per row via `refused!`, each editing one member of a fresh artifact and asserting reason, pointer prefix, that the record callback never ran for a pre-record refusal, and that the fallback admits correctly):

- Document shapes (one loader): missing bytes → `MissingArtifact`; empty, whitespace-only, invalid UTF-8, truncated, trailing content, JSON5 comment, trailing comma, repeated top-level key, repeated key inside a payload schema, unknown top-level member, absent/null payload → `MalformedArtifact`. Control: trailing whitespace accepted.
- `wire_version`: absent/null/string/negative → malformed; `2` → `UnsupportedWireVersion`.
- `owner`: absent/null/number/empty → malformed; another owner → `UnusableRecord`.
- `preparation_revision`: absent/null/string/`0` → malformed; `2` → `PreparationRevisionMismatch`.
- `sources`: absent/null/object/one invalid element/all invalid/unknown member/repeated/unsorted/empty identity → malformed; `[]`, renamed, extra → `SourceMismatch`.
- Digest validator (shared): absent/null/number/empty/short/long/uppercase/non-hex → malformed; changed → `SourceMismatch`; dependency digest wired to the same validator.
- `dependencies`: absent/null/string/invalid element/zero revision → malformed; unexpected entry → `DependencyMismatch`; `[]` is the control.
- `configuration`: absent/null/object/malformed pair/all invalid → malformed; unexpected entry → `ConfigurationMismatch`; `[]` is the control.
- Payload: absent/null definitions, one or all invalid elements, unknown member, missing `json_schema` → malformed; empty list, misidentified, swapped order, null schema → `UnusableRecord`.

Smell sweep over `shipped/`: no `serde(default)`, `unwrap_or_default`, or `.ok()`; the one `filter_map` (`header_unchecked`) drops refused dependencies only after `check_header` has already refused on them; `PreparedNamedTypes::table` returning `None` means "parse the text", never "empty table".

### Gates

The scope calculator over this phase's working tree reports `packages: ['simplified-schema', 'test-toolkit', 'darkmatter', 'darkmatter-cli', 'repo-deps', 'sniff']`, `change_class: package`, so no gate was skipped.

- `simplified-schema`: `just test` — 1940 passed (`work-counters,terminal`) and 1663 passed (`--no-default-features`), 0 slow; `just lint` — clean in all three configurations (one `clippy::too_many_arguments` on `lower_embedded_schema` resolved with the same `allow` its sibling `expand_imports` carries).
- `darkmatter` (direct dependent; the session's required gates): `just test` — 16348 passed, 10 skipped, 0 slow, 155.9 s wall clock; `just lint` — clean.
- `claudine` (dependent, > 200 tests): the tests whose names touch the changed surface, `cargo nextest run -p claudine --features work-counters -p claudine-cli --features claudine-cli/test-fixtures -E 'test(/schema|bundle|trigger|global|shadow|envelope/)'` — 1478 passed, 0 slow.
- `just shipped-schemas` (root and area) regenerates `simplified-schema/lib/generated/shipped.json` byte-identically; `just shipped-schemas --check` in `simplified-schema/` passes.

No pre-existing failures were seen. No cross-OS check was run: the phase adds no path comparison or platform branch, and the artifact and its sources are LF text under the repository's `* text=auto eol=lf`, so the byte comparison holds on every OS; CI's ordinary coverage proves it on Linux and macOS.

