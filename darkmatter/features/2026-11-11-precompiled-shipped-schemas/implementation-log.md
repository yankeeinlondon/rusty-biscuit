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
    - |-
      The shared loader now accepts a large payload section by section (`accept_deferred`): at load it checks the layout, the header, and the list of payload members, and each member is checked, decoded, and validated the first time a consumer reads it. A member found damaged then is refused on its own and only that item is prepared from its text; the other members keep serving. The plan (Rule 5) says one accepted artifact is all-or-nothing. The reason: with whole-artifact decoding, Darkmatter's artifact cost about as much to load as the preparation it replaced (178 ms debug, 30 ms release), and `md compose` improved only from 285 to 276 ms debug. With sections it loads in about 5 ms release and a plain compose decodes only the catalog and baseline (now 112 ms debug, 29 ms release). Results are the same either way, because a section can only be used after the header proved the artifact describes the compiled text, and a section that fails falls back to the same preparation a refused artifact uses. Alternatives: keep whole-artifact acceptance and accept the slower load (misses the spec's timing targets by a wide margin), or switch to a binary format (the spec allows it, but it is a much larger change and still decodes everything). Reverting means making Darkmatter's registry decode all sections at load (`Registry::check_all` already does exactly that) and dropping `accept_deferred`/`Section` from the library.
    - |-
      Artifacts are no longer fully pretty-printed (Rule 1 said `serde_json::to_vec_pretty`). The header stays pretty-printed and each payload member is written as compact JSON on its own line. Pretty-printing made Darkmatter's artifact 7.1 MB instead of 1.8 MB and roughly quadrupled the time to scan it; the one-member-per-line layout also lets the loader find each section without parsing the others, and a line diff still shows which header field or section changed. The cost: a section's diff is one very long line, and a hand-reformatted artifact is refused as malformed (it falls back; the freshness test then fails with the regeneration command). Alternatives: fully compact (smallest, but a diff is one line and the loader must scan everything), or fully pretty (as planned, slow). Reverting means changing `wire::encode` and the layout reader and regenerating both artifacts.
    - |-
      Prepared trigger bundles do not store the trigger envelope (its `match:` conditions); at run time the envelope is reparsed from the compiled text, and only the resolved payload comes from the artifact. Reparsing is a small YAML parse, and it keeps every path pattern bound to the registry's own context (Rule 10). Two bundles with the same name and files compare equal whether or not one carries a prepared descriptor, since the descriptor changes only how results are obtained. Alternatives: store the parsed conditions too (saves a small parse; needs serializable matcher types), or compare descriptors by pointer. Either is a local change in `triggers/prepared.rs` / `embedded.rs`.
    - |-
      CI scheduling of Darkmatter's freshness test covers the core Simplified Schema preparation files (the resolver, envelope admission, embedded sources, the shipped-artifact mechanism, and the lowering files already declared), not every file that could conceivably change lowering output. A Simplified Schema change outside that set still runs Simplified Schema's own tests and is caught by Darkmatter's freshness test on the next run that schedules Darkmatter. Alternative: declare a much longer `source-inputs` list (every `simplified-schema/lib/src` file), at the cost of a narrowed Darkmatter cell on almost every Simplified Schema change. Changing it means editing the list in `darkmatter/lib/Cargo.toml` and the matching `include_str!` lines in `shipped_artifact_is_fresh`.
    - |-
      Two small public seams exist only for tests: `Registry::load_over(bytes, lower)` (inject artifact bytes and the lower artifact's state) and `DarkmatterSchemas::with_darkmatter_baseline_in(&Registry)` (`#[doc(hidden)]`, attaches the baseline from an injected registry). A refused injected registry falls back to the same process-wide lazy preparation as the real one. Alternatives: `test-fixtures`-gated APIs, or a per-registry fallback cache. Either is a contained change in `shipped.rs` and the Darkmatter artifact tests.
    - |-
      Darkmatter's artifact header lists the library's two envelope YAML files as its own sources too, because Darkmatter's kind-definition step reads them, even though the library's artifact (a dependency) already covers them. Removing them means editing `shipped::sources()` and regenerating.
    - |-
      The bundle section of Claudine's artifact is one payload member, decoded whole on first read: 3.4 MB, almost all of it the `claudine.yaml` and `sequence.yaml` payloads (each about 1.8 MB of lowered JSON, shape, and cyclic definitions). Decoding it costs about 25 ms release and 180 ms debug, against about 100 / 630 ms to prepare it from text, and `claudine compose --goose --dry-run prompts/clarify.md` went from 1416 to 857 ms debug (207 to 121 ms release; load about 35). The plan's 650 ms debug target is Phase 5's acceptance sample. The alternative is to decode each trigger document lazily, so a document matching one trigger never decodes the other; that changes Darkmatter's `PreparedTriggerBundle` (entries become per-document sections) and discovery's lookup, and is recorded as the next step in the handoff rather than done here. Redoing it means splitting `TriggerBundleRecord` into per-document sections and regenerating Claudine's artifact.
    - |-
      Claudine's CI scope follows the Phase 2 choice for Darkmatter: its freshness test spells, and its manifest declares as `source-inputs`, a curated set of lower preparation files (Darkmatter's `schemas/{catalog,embedded,resolve,shipped}.rs` and `triggers/{assemble,envelope,prepared}.rs`, plus the same Simplified Schema files Darkmatter declares), the two lower artifacts, and every source either lower artifact fingerprints. The plan's Wave 5 asked for every non-test `.rs` file under `simplified-schema/lib/src/` (Darkmatter, Claudine) and `darkmatter/lib/src/` (Claudine), generated from the trees. That would be about 600 `include_str!` lines and a narrowed Claudine L1 cell, which compiles Claudine's whole test archive, on nearly every Darkmatter change. A change outside the curated set that alters Claudine's output without changing either lower artifact is still caught the next time Claudine is scheduled. Changing it means extending the list in `claudine/lib/Cargo.toml` (and `darkmatter/lib/Cargo.toml`) and the matching lines in each `shipped_artifact_is_fresh`.
    - |-
      The root coordinator (`scripts/shipped_schemas.py`) checks each generated output in memory only structurally (JSON, one final line feed, its owner, and the lower owners named as dependencies), because Python has no xxHash to recompute digests; after replacing the files it runs each example's `--check`, which rebuilds over the new files and performs the full acceptance and freshness check. The plan said to validate all outputs in memory before replacing any. A digest-level in-memory check would need a Python xxHash dependency or a new `--check-bytes` mode in each example. Its contract tests live in `test-toolkit` (as the other Python scripts' contracts do) and run the script with a stand-in generator (`--generator`), so no test builds a crate. Changing it means adding that mode to the three examples and a call in the script.
    - |-
      `prepare_payload` in Claudine returns `String` errors like the other two owners' generation APIs, so two `retained` entries were added to `claudine/cli/tests/l1/error_guards/transport-allow.toml` for the typed-error-collapse guard. The alternative is a typed generation error in Claudine, unlike the other owners.
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
message_to_agent: |-
    Phase 3 landed Claudine's artifact (`claudine::composition::schema::shipped`, `claudine/lib/generated/shipped.json`), Darkmatter's source-selection tests, and the root coordinator `scripts/shipped_schemas.py` (root `just shipped-schemas` regenerates all three artifacts all-or-nothing; run it after any lower change, since Claudine's header fingerprints Darkmatter's artifact bytes). For Phase 4: (1) any change to Darkmatter's or the library's preparation output (for example new digest/classification fields in records, Rule 13) needs the owning `PREPARATION_REVISION` bumped and the root `just shipped-schemas`, which regenerates Claudine's too; Claudine's prepared trigger payloads are `triggers::prepared::PayloadRecord`, so a new field there changes Claudine's artifact. (2) Timing (load about 35, not quiet): `claudine compose --goose --dry-run prompts/clarify.md` 857 ms debug / 121 ms release (baseline 1416 / 207; target 650 debug). Claudine's bundle section is 3.4 MB and costs about 180 ms debug / 25 ms release to decode on first read, nearly all of it the `claudine.yaml` and `sequence.yaml` payloads; decoding each trigger document lazily (per-document sections in `PreparedTriggerBundle`) is the next saving if Phase 4's hashing work does not reach the target. (3) Claudine's freshness test spells its lower inputs inline; if you add a preparation file Claudine reads, add it there and to `claudine/lib/Cargo.toml` `source-inputs` (the planner test `RealWorkspaceTestInputTests.test_every_declared_source_input_is_read_by_its_declarer` enforces the pairing).
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


## Phase 2

### Orchestration

Before delegating, the coordinator made the shared edits the three parallel tasks depend on:

- `simplified_schema::shipped::present` is now public (hosts mark every `Option` field of their records with it, so an absent member is an error and `null` is `None`).
- `simplified_schema::work_counters::Work::PositionMapConstruction` (a `PositionMap` built from text) was added.
- Darkmatter's `work_counters::Work` gained `ArtifactDecode`, `ArtifactAccept`, `ArtifactGeneration`, `ShippedRuntimePreparation`, the eight `ArtifactFallback*` kinds, and `SourcePositionMapConstruction`, all folded from the library's counters in `library_kind` (Phase 1's handoff item 4), plus Darkmatter-owned `BaselineCheck`, `PayloadMergeCheck`, and `PayloadCompileCheck` for the checks that move to generation.

Three tasks then ran in parallel, each scoped to disjoint files and verifying only its own slice: deferred position maps (Rule 11), the Darkmatter registry, artifact, generator, and non-trigger consumer routing (Wave 2 "Prepare host records" and Wave 3 routing), and prepared trigger bundles with discovery routing (the prepared-bundle helper Claudine uses in Phase 3).

### Deferred position maps (Wave 2, Rule 11)

`simplified_schema::PositionMap` now has two states: `Built` (every existing constructor: `new`, `Default`, `project`, `top_level`, `build_position_map`) and `Deferred`, an `Arc` holding the exact text and a `OnceLock` of the built source (`PositionMap::deferred(Arc<str>)`). `is_empty`, `byte_range`, and `locate_all` with at least one problem build it once; `locate_all(&mut [])` and `Debug` never do; clones and threads share one build. `Work::PositionMapConstruction` is counted in the single text-to-map builder (Darkmatter folds it as `SourcePositionMapConstruction`). Envelope admission (`EnvelopeAdmission::positions`) and whole-YAML loading (`LoadedYaml`, over the text after a BOM with its `offset` added back) use deferred maps. Trigger errors: `locate_trigger_error` previously built a map for every error kind and discarded it; it now locates only a `TriggerMatch` without a span, and discovery shares one `TriggerSourcePositions` holder (a deferred map over the text after a BOM) between the error's span and the finding's line, so a reported trigger file builds at most one map. `project` and `top_level` (Darkmatter's Markdown path, already deferred by `ProblemPositions`) are not counted. `ProblemPositions` is unchanged.

Found while testing, unchanged and pre-existing: a standalone schema envelope starting with a BOM is refused at recognition ("more than one document"), before positions are involved.

### Darkmatter's artifact and registry (Wave 2 "Prepare host records", Wave 3 routing)

New public module `darkmatter::markdown::schemas::shipped`: `OWNER = "darkmatter"`, `PREPARATION_REVISION = 1`, `ARTIFACT` (`include_bytes!` of `darkmatter/lib/generated/shipped.json`), `sources()` (the Rule 9 inventory: `embedded::SHIPPED` including Content Policy's editor schema, the kind definitions including `schema-trigger.yaml` and the library's two envelope YAMLs, `mutations.yaml`, and the four function-example files; `functions.yaml` once, with a generation check that its two compiled copies are identical), `descriptor_over`, `Registry::{load, load_over, refusal, check_all, as_dependency}`, `registry()`, `NAMED_TYPES` (the descriptor attached to `SHIPPED_SOURCES`), `FreshDarkmatter::{prepare, artifact, as_dependency}`, `generate()`, `check_artifact()`.

Payload members: `named_types` (inventory plus one `NamedTypeTableRecord` per partial), `catalog` (`GlobalCatalogRecord`: identity, description, each global's name, definition, and definition table, in requirement order), `baseline` (the lowered JSON; its typed form is rebuilt from the catalog's `fm`), `inline_baseline`, `kind_definitions` (resolved `$schema` mirror plus claimed kinds; generation fails if an embedded resolution reports any host path list or advisory), `functions` and `effects` (`FunctionCatalog`, now Serde, checked by `check_record()` after decode). Every `Option` field uses `present`; no Serde defaults.

Uncached preparation threads `PreparationMode` through `GlobalCatalog` embedded loading, the inline baseline, kind-definition preparation, and `load_catalog`. Consumers routed through the registry: `shipped_baseline`/`darkmatter_base_schema_ref` (`BASELINE`, `INLINE_BASELINE`), `GlobalCatalog::darkmatter()`, `yaml_documents::registry::shipped_definitions()` and `KindRegistry::shipped()` (built once and stored on the registry), `expression_function_catalog()`, `effect_function_catalog()`; effects' `REGISTRATIONS` stays a runtime binding cache. A refused artifact (or section, below) never prepares everything: each view falls back to that consumer's existing lazy preparation, counting `ShippedRuntimePreparation`. `with_darkmatter_baseline_json_schema` runs `validate_baseline_schema` (counting `BaselineCheck`) only when the baseline did not come from the accepted artifact. `reset_process_caches_for_bench` keeps the compiled-in registries loaded (doc updated).

Generator `darkmatter/lib/examples/shipped_schemas.rs` (same CLI as the library's; `--check`/`--write` first require the compiled-in Simplified Schema artifact to equal fresh lower generation and otherwise name `just shipped-schemas`), area recipe `just shipped-schemas [--check]`, root `just shipped-schemas` now runs both areas in order (Phase 3 replaces it with the coordinator), `.gitattributes` marks the artifact `linguist-generated`.

### Load cost: layout and deferred sections (coordinator)

The first generated Darkmatter artifact was 7.1 MB pretty-printed (1.76 MB compact): named-type tables 666 KB (lifecycle partial 365 KB), inline baseline 555 KB, catalog 212 KB, functions 202 KB. Loading it took 178 ms debug / 30 ms release, close to the preparation it replaced, and `md compose` moved only from 285 to 276 ms debug and 55 to 45 ms release. Release breakdown (one-off timing example, since removed): the whole-document duplicate-key walk plus header scan 13.7 ms, typed payload decode a further ~6 ms; views free once decoded.

Two changes in the shared mechanism:

1. **Layout.** `wire::encode` writes the header pretty-printed and each payload member compact on its own line (compact JSON never holds a raw line feed). Darkmatter's artifact is now 1.8 MB, and a line diff names the header field or payload section that changed. Simplified Schema's artifact was regenerated in the same layout (no revision bump: the payload is identical, the bytes are not, and its freshness test compares bytes).
2. **Deferred sections.** `shipped::accept_deferred` checks the layout (`wire::decode_laid_out`), the header, and the payload member list only, without scanning any member's JSON; `Sections::take` hands each member to its owner as a `Section<T>`, and `Section::get` walks that member for repeated keys, decodes it, and validates its records on first read (once per process). A section refused on first read refuses only itself; its consumer prepares that one item from text, and its refusal (reason, pointer, artifact line and column) is counted. A deferred artifact must be in the exact layout (`MalformedArtifact` otherwise). Darkmatter's registry uses it: `md compose` on a plain document decodes only the catalog and baseline sections. The baseline section depends on the catalog section and is refused with it. `Registry::check_all` forces every section (the generator's `--check` and the tests use it).

Release load after both: Darkmatter registry 4.9 ms (catalog section 3.9 ms, baseline 1.1 ms on first read). Debug: 2.7 ms load once warm, catalog 12.6 ms, baseline 5.6 ms.

`md compose` on the three-line fixture (same commands as the Phase 1 baseline, `/tmp/pss-timing/md.sh`, five-run medians, load average 25 during the run, so not a quiet-host sample):

| Build | Phase 1 baseline | After Phase 2 |
| --- | --- | --- |
| debug | 291.2 ms | 111.7 ms |
| release | 56.9 ms | 29.3 ms |

The spec targets (90 / 20 ms) are Phase 5's acceptance sample; Phase 4's layer-digest cache keys remove the remaining per-run canonical hashing of the merged baseline.

### Prepared trigger bundles (Wave 3 discovery routing; Claudine's helper)

New `darkmatter::markdown::schemas::triggers::prepared`: `TriggerBundleRecord { bundle, documents }` (bundle file order), `TriggerDocumentRecord { name, outcome }`, `TriggerOutcomeRecord::{NotTrigger, MatcherOnly, Payload(Box<PayloadRecord>)}`, `PayloadRecord { json_schema, shape, definitions, advisories }`; `prepare_trigger_bundle(bundle, mode)` runs every check (envelope, payload resolution through `mode` with a private validator cache when uncached, merge and compile checks) and fails on any invalid document or a template payload; `PreparedTriggerBundle::from_record(&Acceptance, pointer, record)` validates names, order, completeness against the header's `{bundle}/{file}` sources, and object payloads; `PreparedBundleDescriptor { owner, provider }`; `EmbeddedSchemaBundle::{with_prepared, prepared, documents}` (`new` stays source-only; equality ignores the descriptor). Discovery keeps filename selection first; a surviving document of a bundle whose prepared bundle matches (name and every file's fingerprint, memoized per file table) gets its envelope reparsed from the compiled text and its payload from the record, skipping resolution and the merge/compile checks; otherwise the ordinary memoized path runs (counting `ShippedRuntimePreparation` when a descriptor was attached). `enforce_merge_compatible`/`enforce_compiles` count `PayloadMergeCheck`/`PayloadCompileCheck`. Envelopes are not stored: their conditions and `pattern_cwd` stay bound to the registry's context.

### CI test-input scope

`shipped_artifact_is_fresh` spells Simplified Schema's artifact, Content Policy's editor schema, and the library files that parse, admit, resolve, lower, and encode shipped schemas; Darkmatter's `[package.metadata.ci.tests] source-inputs` gained `simplified-schema/lib/src/{embedded.rs, envelope.rs, resolve.rs, resolve/graph.rs, shipped/mod.rs, shipped/records.rs, shipped/wire.rs, shipped/wire_support.rs}` (the lowering files `simplified/{convert,definitions,grammar,mod,standalone,types}.rs` were already declared). `python3 scripts/ci/affected_scope.py simplified-schema/lib/src/shipped/wire.rs` now plans a narrowed `{darkmatter, ubuntu-latest, L1}` cell; `python3 -m unittest test_affected_scope.RealWorkspaceTestInputTests` passes (8 tests).

### Tests (requirement → test)

Simplified Schema:

| Requirement | Tests |
| --- | --- |
| Deferred maps build nothing until queried; one build per map, shared by clones and threads | `deferred_positions::{a_valid_envelope_builds_no_map_until_a_coordinate_is_requested, an_eager_map_counts_once_at_construction, clones_share_one_construction, concurrent_lookups_build_once}` |
| Same coordinates as eager maps (LF, CRLF, BOM, BOM+CRLF, multi-byte columns, aliases, missing/unknown keys) | `deferred_and_eager_maps_agree_on_authored_bytes`, `byte_columns_and_crlf_lines_address_the_original_text`, `an_admitted_envelope_locates_like_an_eager_map`, `a_rejected_envelope_is_located_like_an_eager_map` |
| Layout: one member per line; header pretty | `shipped_sections::each_payload_member_is_written_on_its_own_line` |
| Sections decode lazily, keep authored order | `a_section_is_decoded_only_when_first_read_and_keeps_authored_order` |
| A malformed / repeated-key / record-rejected section refuses only itself, located at its artifact line and column, counted once | `a_malformed_section_refuses_only_itself_at_its_line_and_column`, `a_repeated_key_inside_a_section_refuses_that_section`, `a_record_a_section_validator_rejects_is_unusable`, `a_refused_section_is_counted_once` |
| Whole-artifact refusals of the deferred loader (one generated test per row) | pretty-printed payload, missing comma, trailing comma, content after the document, missing final LF, unquoted key, repeated member (`/payload/limit`), unknown member (`UnusableRecord` `/payload/extra`), missing member, header digest mismatch, malformed header member |

The Phase 1 rows `a_repeated_top_level_key_is_malformed` and `a_repeated_key_inside_a_payload_schema_is_malformed` edited the old pretty text; their text edits were updated to the new layout (same assertions).

Darkmatter (`tests/l1/`):

| Requirement | Tests |
| --- | --- |
| Freshness, with the command | `shipped_artifacts::shipped_artifact_is_fresh` |
| Mutated in-memory source fails freshness and acceptance naming the command | `a_mutated_source_fails_freshness_and_acceptance_naming_the_command` |
| Round trip in order; deterministic fresh generation | `the_checked_in_artifact_round_trips_to_uncached_preparation_in_order`, `fresh_generation_is_deterministic_and_ignores_checked_in_artifacts` |
| Cold accepted lookups: zero runtime preparation, zero baseline check | `cold_accepted_lookups_prepare_nothing_and_skip_the_baseline_check` |
| Fallback with injected bytes; views equal the accepted ones; validation equal; baseline check runs when the baseline falls back | `missing_bytes_fall_back`, `malformed_json_falls_back`, `a_source_mismatch_falls_back`, `another_preparation_revision_falls_back`, `a_malformed_record_falls_back`, `an_unusable_record_falls_back`, `a_refused_catalog_section_refuses_the_baseline_typed_by_it` |
| Stale lower artifact whose bytes match the upper header | `a_stale_lower_artifact_refuses_darkmatter_even_when_its_bytes_match_the_header` |
| Reading the baseline decodes no unrelated section | `reading_the_baseline_decodes_no_unrelated_section` |
| Custom same-name `ShippedSources` parsed, not served | `a_custom_shipped_sources_under_darkmatter_s_paths_is_parsed_not_served` |
| Generator wiring | `the_generator_checks_emits_and_writes`, `a_stale_lower_artifact_stops_check_and_write_naming_the_root_command`, `the_generator_reports_where_a_malformed_artifact_is_refused`, `a_failed_generation_leaves_the_artifact_in_place` |
| Record validators (control + one invalid shape each) | `the_re_encoded_checked_in_artifact_is_the_accepted_control`, `a_null_catalog_description_is_none_and_an_absent_one_is_malformed`, and 18 `refused!` rows (payload members, named-type tables and inventory, catalog identity and globals, kind definitions, function categories/positions/overloads, an absent `Option` in an example) |
| Prepared bundle = source-only results with zero merge/compile checks | `prepared_trigger_bundles::a_prepared_bundle_discovers_what_its_sources_discover_without_the_payload_checks` |
| Same-name bundle with other text takes ordinary preparation | `a_changed_bundle_of_the_same_name_takes_ordinary_preparation` |
| Disk filename suppression first; siblings prepared; imports stay in the bundle | `a_disk_file_suppresses_its_prepared_document_while_its_siblings_stay_prepared` |
| Bundle record validators; generation runs the checks and fails on an invalid trigger or template | `record_acceptance_refuses_each_invalid_shape`, `generation_fails_on_an_invalid_trigger_after_running_the_checks`, `generation_records_each_document_in_file_order`, `generation_refuses_a_template_payload` |
| Whole-YAML and trigger entry points build no map for valid sources; invalid ones locate as before | `deferred_positions::{a_valid_yaml_document_builds_no_position_map, an_invalid_yaml_document_builds_one_map_and_locates_the_original_text, a_valid_trigger_builds_no_position_map, an_invalid_trigger_builds_one_map_and_locates_the_original_text}` |

Smell sweep over the new loaders (`shipped.rs`, `triggers/prepared.rs`, `shipped/{mod,wire}.rs`): no `#[serde(default)]`; every record `Option` uses `present`; no `unwrap_or_default`/`.ok()`/`filter_map(.. as_str())` on a load-bearing parse. Retained: `Section`-returning views use `.ok()` to turn a section refusal into the documented fallback (the refusal is counted and logged in `Section::get`), not to accept data.

### Gates

The scope calculator over the working tree reports `packages: ['darkmatter', 'simplified-schema', 'test-toolkit', 'claudine', 'darkmatter-cli', 'darkmatter-lsp', 'repo-deps']`, `change_class: package`; no gate was skipped.

- `simplified-schema`: `just test` — 1965 passed (`work-counters,terminal`) and 1683 passed (`--no-default-features`), none slow; `just lint` — clean (one `type_complexity` fixed with a `SectionText` alias, one dead test struct removed); `just check-tier-coverage simplified-schema` — nothing stranded.
- `darkmatter`: `just test` — 16398 passed, 10 skipped, 86.5 s, "none at or above 5.0s" (load average about 6); `just lint` — clean (including the zed-dmls wasm check); `just check-tier-coverage darkmatter` — nothing stranded.
- Dependents: `cargo nextest run -p claudine --features work-counters -p claudine-cli --features claudine-cli/test-fixtures -E 'test(/schema|bundle|trigger|global|shadow|envelope|catalog|baseline|kind/)'` — 1795 passed; `cargo nextest run -p messenger -E 'test(/compose|schema|frontmatter|baseline/)'` — 4 passed; `cargo check --all-targets` of every other direct dependent (`biscuit-icon-cli`, `biscuit-speaks-cli`, `biscuit-terminal-cli`, `messenger`, `messenger-cli`, `claudine-gen`, `playa-cli`, `sniff-detection-cli`, `research`, `claudine`, `claudine-cli`) — clean.
- `just shipped-schemas` regenerates both artifacts; `cargo run -p darkmatter --example shipped_schemas -- --check` exits 0.

No pre-existing failures were seen. No Windows cross-check was run: the artifacts are LF under the repository's `* text=auto eol=lf`, so `include_bytes!` sees the same bytes on every OS, and a damaged layout only falls back; the position work preserves the existing offsets and is covered by the existing CRLF/BOM fixtures, which CI runs on every environment.

## Phase 3

### Orchestration

Two tasks ran in parallel, on disjoint files: Claudine's artifact and registry (Wave 4 "Integrate Claudine registry"), and Darkmatter's source-selection tests (Wave 4 "Prove source selection", test files only). The coordinator then did Wave 5: the root regeneration coordinator, Claudine's CI scope, the planner assertions, docs, and skills, followed by the gates.

### Claudine's artifact (Wave 4)

New public module `claudine::composition::schema::shipped`: `OWNER = "claudine"`, `PREPARATION_REVISION = 1`, `ARTIFACT_PATH`, `REGENERATE_COMMAND`, `ARTIFACT`, `sources()` (the eight bundle files at `claudine/{file}` plus the host catalog at `claudine/schemas/globals.yaml`, a nested path so `PreparedTriggerBundle::from_record` never takes it for a bundle document), `descriptor_over`, `ClaudinePayload { bundle: TriggerBundleRecord, globals: GlobalCatalogRecord }`, `Registry::{load, load_over(bytes, [simplified-schema, darkmatter]), refusal, check_all, bundle, globals}`, `registry()`, `BUNDLE` (the `PreparedBundleDescriptor`), `prepare_payload`, `FreshClaudine`, `generate()`, `check_artifact()`. Loading uses `accept_deferred`, one section per member.

- `claudine_schema_bundle()` stays `const fn` and returns the bundle with `shipped::BUNDLE` attached; generation uses the source-only `source_bundle()`. `is_claudine_owned`, the `CLAUDINE_*` constants, and origin labels are unchanged.
- `claudine_globals()` is a registry view; the text loading moved to `globals_from_sources()`, which counts `ShippedRuntimePreparation`. `host_global` is unchanged.
- Generator `claudine/lib/examples/shipped_schemas.rs` (same CLI as Darkmatter's; `--check`/`--write` require both compiled-in lower artifacts to equal fresh generation and otherwise name the root command), area recipe `just shipped-schemas`, `.gitattributes` entry. The artifact is 3.4 MB (see "Load cost" below).
- `claudine/docs/topics/schemas.md` has a new section, "How the built-in copy is prepared". It covers regeneration, freshness failures, fallback per section, and the fact that disk/bundle selection and DMLS host-global handling are unchanged.

### Source selection (Wave 4, Darkmatter tests)

`darkmatter/lib/tests/l1/prepared_trigger_bundles.rs` now serves two in-memory prepared bundles (the existing one and a `$path`-conditioned `path-host`) through a small `Host` helper. The existing seven tests are unedited, and so are Claudine's existing disk `claudine.yaml`/`document.yaml` tests. No defect was found and no `src/` changed. Behavior pinned along the way, unchanged and pre-existing: disk filenames claim embedded names by exact match, so a disk `HOST.yaml` does not suppress an embedded `host.yaml` (both install).

### Regeneration and CI scope (Wave 5)

- `scripts/shipped_schemas.py` (root `just shipped-schemas`) runs the three examples in emit mode in dependency order and checks each output (JSON, one final LF, its owner, the lower owners as dependencies). It stages sibling `.tmp` files, then renames them into place, then runs each example's `--check`. Any failure before the rename changes nothing. `--root`/`--generator` exist for its contract tests (`tools/test-toolkit/tests/shipped_schemas_coordinator_contracts.rs`, which runs the real script with a stand-in generator; declared in test-toolkit's `source-inputs`). The real coordinator was run twice on the finished tree, and all three artifacts were byte-identical before, between, and after (`every shipped artifact is current`, 18.5 s).
- Claudine's `shipped_artifact_is_fresh` spells both lower artifacts, every source either lower artifact fingerprints, and the lower preparation code. `claudine/lib/Cargo.toml` `source-inputs` gained those `.rs` files: Darkmatter's `schemas/{catalog,embedded,resolve,shipped}.rs` and `triggers/{assemble,envelope,prepared}.rs`, plus the 14 Simplified Schema files Darkmatter declares. Darkmatter's freshness test now also spells `simplified-schema/schemas/{schema,yaml-schema}.yaml`, which its header fingerprints.
- `scripts/ci/test_affected_scope.py::RealWorkspaceTestInputTests` gained two tests. `test_shipped_schema_inputs_select_each_downstream_freshness_test` covers the envelope YAML, `resolve.rs`, and the artifact of Simplified Schema; Content Policy's editor schema; and the function-example YAML, `catalog.rs`, and artifact of Darkmatter. Each yields a narrowed `{pkg, ubuntu-latest, L1}` cell with `test(=shipped_artifacts::shipped_artifact_is_fresh)` for every downstream owner, and an artifact-only change still selects its owner's full suite. `test_the_shipped_schema_coordinator_selects_its_contracts` covers the coordinator. The whole class passes, including `test_every_declared_source_input_is_read_by_its_declarer`; the first run caught a bug in my own assertion (`packages` holds records), fixed and rerun.
- `simplified-schema/docs/topics/shipped-artifacts.md` gained "What the root command does" (with a flow diagram) and "When CI runs the freshness tests". The Darkmatter and Claudine skills name the coordinator and the source-input pairing.

### Load cost

A one-off timing example (deleted after use) measured:

| | release | debug |
| --- | --- | --- |
| Claudine registry load (header and member list, plus both lower headers) | 15 ms | 34 ms |
| bundle section, first read | 25 ms | 179 ms |
| globals section | 0.3 ms | 1.4 ms |
| uncached bundle preparation (what a refused artifact costs) | 98 ms | 629 ms |

Section sizes: `claudine.yaml` payload 1.83 MB, `sequence.yaml` 1.82 MB, `inline-compose.yaml` 2.6 KB, globals 5.8 KB.

`claudine compose --goose --dry-run prompts/clarify.md spec=specs/demo-spec.md` (Phase 1 staging, `hyperfine -N --warmup 1 --runs 5`, load average about 35, so not a quiet sample): debug 856.7 ms (Phase 1 baseline 1415.9), release 120.7 ms (baseline 207.5). All runs exited 0.

### Tests (requirement → test)

Claudine (`claudine/lib/tests/l1/shipped_artifacts.rs`, 23 tests; 21 without `work-counters`):

| Requirement | Tests |
| --- | --- |
| Freshness over the full chain, naming the command | `shipped_artifact_is_fresh` |
| Round trip in order; deterministic generation | `the_checked_in_artifact_round_trips_to_uncached_preparation_in_order`, `fresh_generation_is_deterministic` |
| A normal shipped bundle: zero runtime preparation and zero moved checks | `cold_accepted_lookups_prepare_nothing` (discovery in a temp dir plus `claudine_globals()`: runtime preparation, `PayloadMergeCheck`, and `PayloadCompileCheck` all 0) |
| Generation bypasses both lower owners and Claudine's registry | `generation_consults_no_artifact` (`ArtifactDecode`/`ArtifactAccept` 0) |
| Fallbacks: missing, malformed, source, revision, record, and dependency mismatches | `missing_bytes_fall_back`, `malformed_json_falls_back`, `a_source_mismatch_falls_back`, `another_preparation_revision_falls_back`, `a_malformed_record_falls_back` (only the globals section is refused; the bundle is still served), `a_refused_darkmatter_artifact_falls_back`, `a_changed_darkmatter_artifact_falls_back` |
| A custom same-name bundle is prepared, not served | `a_custom_same_name_bundle_is_prepared_not_served` |
| Generator wiring (check, emit, write, stale lower, malformed `--artifact` with pointer and line, failed generation, usage) | `the_generator_checks_emits_and_writes`, `a_stale_lower_artifact_stops_check_and_write_naming_the_root_command`, `the_generator_reports_where_a_malformed_artifact_is_refused`, `a_failed_generation_leaves_the_artifact_in_place` |
| Record validators: a control plus one invalid shape each | `the_re_encoded_checked_in_artifact_is_the_accepted_control`, `a_record_for_another_bundle_is_refused`, `a_bundle_document_outside_the_sources_is_refused`, `a_host_catalog_at_another_path_is_refused`, `a_global_out_of_order_is_refused`, `a_null_catalog_description_is_none_and_an_absent_one_is_malformed` |

Darkmatter (`prepared_trigger_bundles.rs`, 8 new tests):

| Requirement | Test |
| --- | --- |
| Mixed: a disk trigger is lowered while its siblings are served | `a_mixed_registry_names_each_winning_trigger_and_checks_only_the_disk_payload`: merge, compile, and runtime counters `[1, 1, 0]`; layers `[File(notes), Embedded host.yaml]`; per-property origins; the problems; equal to the source-only bundle |
| An ignored or invalid same-name file still suppresses the embedded trigger | `an_ignored_or_invalid_same_name_disk_file_suppresses_its_prepared_trigger` |
| Embedded imports ignore a same-name disk library | `a_prepared_trigger_imports_the_bundle_library_never_a_same_name_disk_file` |
| A changed imported library invalidates the dependent entries | `a_bundle_whose_imported_library_changed_is_not_served` |
| A same-name bundle with another file set | `a_same_name_bundle_with_an_added_file_is_not_served`, `a_same_name_bundle_with_a_removed_file_is_not_served` (two tests, because the process-wide `EMBEDDED_LOADED` memo correctly skips the checks for a second identical bundle in one process) |
| Path predicates stay bound to each registry's context | `a_prepared_path_condition_matches_in_each_registry_s_own_context` |
| Bundle order and case-fold collisions are unchanged by a descriptor | `bundle_order_and_case_folding_are_unchanged_by_a_prepared_descriptor` |

Coordinator (`test-toolkit`, 4 tests): `a_successful_run_replaces_every_artifact_and_a_repeat_is_a_no_op`, `a_failed_generation_leaves_all_three_artifacts_in_place`, `output_that_is_not_the_owner_s_artifact_replaces_nothing` (not JSON, wrong owner, missing dependency, two final LFs, empty), `a_failed_check_after_writing_is_reported`.

Smell sweep over Claudine's new loader (`shipped.rs`): no `#[serde(default)]`; the record `Option`s are Darkmatter's (`GlobalCatalogRecord`, `PayloadRecord`), which use `present`; `.ok()` appears only where a refused section turns into the documented fallback (the refusal is counted in `Section::get`).

### Gates

Scope calculator over the working tree (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = claudine, darkmatter, repo-deps, simplified-schema, test-toolkit, claudine-cli, darkmatter-cli, darkmatter-lsp; `change_class: package`. No gate was skipped.

- `darkmatter`: `just test`: 16406 passed, 10 skipped, "none at or above 5.0s" (load average 40 at start). `just lint`: clean, including the zed-dmls wasm check. `just check-tier-coverage darkmatter`: nothing stranded.
- `claudine` (changed directly; its recipe covers `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and `claudine-gen`, which includes every reverse dependent of `claudine`): `just test`: 11940 passed, 15 skipped, none at or above 5.0 s. `just lint`: clean. `just check-tier-coverage claudine`: nothing stranded.
- `simplified-schema` (no source change this phase): `cargo nextest run -p simplified-schema --features work-counters,terminal -E 'test(/shipped/)'`: 122 passed.
- `test-toolkit`: `shipped_schemas_coordinator_contracts`: 4 passed.
- Planner: `python3 -m unittest test_affected_scope.RealWorkspaceTestInputTests`: all pass after the assertion fix above.
- `just slow-tests fresh`: fresh.
- The subagents' scoped runs: Claudine schema, bundle, trigger, global, and catalog filters 1326 passed, `claudine-cli` 320 passed, the Claudine guards 176 + 159 passed; Darkmatter trigger and bundle filter 508 passed.

No pre-existing failures were seen. No Windows cross-check was run. The new code adds no path handling: artifacts are compiled in with `include_bytes!` under the repository's LF rule, and the coordinator uses `pathlib` and `os.replace`. Its contract tests resolve `python3` or `python` and pass a `.py` stand-in through `sys.executable`, so they need no shebang execution on Windows. CI runs the coordinator tests (test-toolkit) and Claudine on every environment its schedule selects.
