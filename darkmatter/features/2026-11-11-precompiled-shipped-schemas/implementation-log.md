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
    - |-
      A schema made from a single layer (the baseline alone, or one matched trigger alone) is cached under a one-layer layered identity, not under that layer's own canonical digest. So when the same schema is compiled by a caller through the canonical API, the two get separate cache entries, and the schema is compiled twice in a process that does both. The alternative was to reuse the canonical digest for a single layer, which saves that one compilation but removes the role from the key. That is harmless today, since baseline and trigger merges behave the same, but it would not stay safe if the roles ever merged differently. Changing this touches only the two single-layer arms in `site_schema_with_override`.
    - |-
      The cross-process stability test re-runs the current test binary with `std::process::Command` in a helper mode (the pattern `transcluded_shell_failure.rs` already uses), instead of going "through the test toolkit" as the plan said. The toolkit has no helper for spawning a process. The encoding is also pinned to fixed values, which already proves stability across processes and platforms. Moving to a toolkit helper later would change only `a_layered_identity_is_stable_across_processes`.
    - |-
      The Claudine debug timing target (650 ms) is not met: the final paired sample is 839 ms (Phase 1 binary 1450 ms in the same session, under the same load). The `md` targets (90 / 20 ms) are met at 65 / 19.5 ms. The plan treats the targets as acceptance criteria. A profile shows every shipped-preparation cost the spec listed is gone; what remains is mostly per-document work the spec keeps at run time: compiling the effective validator (about 150 ms debug), resolving the prompt's own schema (about 65 ms), merging layers (about 50 ms), and decoding the prepared bundle section (about 125 ms, both 1.8 MB payloads even though one matches). I tried decoding each trigger document lazily and reverted it, because trigger discovery loads every document's payload up front, so it saved nothing without also making the registry's payload list lazy (a change across discovery, assembly, tracing, and `effective_for`, worth about 60 ms at most). Options: accept the miss as the limit of this design; make the trigger registry's payloads lazy for prepared entries (about 60 ms); or cut validator compile time (cache or simplify the merged schema's validator), which is the largest remaining cost and outside this feature.
    - |-
      Two small speedups landed in Phase 5 after profiling, beyond the plan's task list. (1) The expression type layer (`DeclaredTypes`) now projects the merged JSON Schema to its declarations only when an expression first reads one. Before, every composition projected the baseline, about a quarter of a release `md compose`, even for a document with no expressions. Results are unchanged; a new counter and three tests pin it. The alternative was to precompute the baseline's projection into Darkmatter's artifact, which saves the cost for documents that do read declarations too, but adds a record type, a revision bump, and a regeneration. (2) Artifact sections are decoded with plain `serde_json`, and the slower path-tracking decoder runs only to locate a refusal, so a refusal still names the same JSON pointer. Reverting either is a local change: `projection.rs` for (1), `wire::decode_typed` for (2).
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
source_files_during_phase_4:
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/src/coerce.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/phase.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/shipped/mod.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/work_counters.rs
    - simplified-schema/lib/tests/l1/schema_identities.rs
    - simplified-schema/lib/tests/l1/main.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/shipped.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/prepared.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/generated/shipped.json
    - darkmatter/lib/tests/l1/layered_validator_identities.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shipped_artifacts.rs
    - claudine/lib/src/composition/schema/shipped.rs
    - claudine/lib/generated/shipped.json
    - claudine/lib/tests/l1/shipped_artifacts.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/definition.md
    - simplified-schema/docs/topics/shipped-artifacts.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/simplified-schema/SKILL.md
source_files_during_phase_5:
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/compose_prompt_tiers.rs
    - claudine/cli/tests/l1/entry_point_parity.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - claudine/cli/tests/l1/sequence_overlay_pty.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/lib/src/composition/schema/shipped.rs
    - simplified-schema/lib/src/shipped/mod.rs
    - simplified-schema/lib/src/shipped/wire.rs
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/src/lib.rs
    - darkmatter/lib/src/markdown/schemas/shipped.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/effects/types.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/registry.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/tests/l1/raw_declaration_projection.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_5:
    - simplified-schema/docs/topics/shipped-artifacts.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/schema-discovery.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
    - darkmatter/docs/topics/schemas/schema-validation.md
    - claudine/docs/topics/schemas.md
    - claudine/schemas/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/simplified-schema/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/rust-testing/test-speed.md
source_code:
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
    - simplified-schema/lib/src/positions.rs
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
    - scripts/shipped_schemas.py
    - scripts/ci/test_affected_scope.py
    - tools/test-toolkit/tests/shipped_schemas_coordinator_contracts.rs
    - tools/test-toolkit/Cargo.toml
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/src/coerce.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/phase.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/tests/l1/schema_identities.rs
    - darkmatter/lib/tests/l1/layered_validator_identities.rs
    - claudine/cli/tests/common/mod.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/compose_prompt_tiers.rs
    - claudine/cli/tests/l1/entry_point_parity.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/sequence_magic_reference.rs
    - claudine/cli/tests/l1/sequence_overlay_pty.rs
    - claudine/cli/tests/l1/sequence_schema.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/tests/l1/raw_declaration_projection.rs
documentation:
    - simplified-schema/README.md
    - simplified-schema/docs/dependencies.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/language/schema-documents.md
    - simplified-schema/docs/topics/shipped-artifacts.md
    - simplified-schema/docs/topics/diagnostics.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/topics/schemas/schema-discovery.md
    - darkmatter/docs/topics/schemas/schema-triggers.md
    - darkmatter/docs/topics/schemas/schema-validation.md
    - claudine/schemas/README.md
completed_phase: 5
implemented: true
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

## Phase 4

### Orchestration

I did Waves 6 and 7 directly, without subagents: Wave 7's merge work depended on Wave 6's types, and the "cache isolation" tests were small enough to write alongside it. The gates ran once, serially, after all edits: Darkmatter, then Simplified Schema, then Claudine.

### Typed identities (Wave 6, Rules 12 and 14)

All in `simplified-schema/lib/src/validate.rs`:

- `SchemaIdentity::{Canonical { revision, digest }, Layered { revision, digest }}` and a public `SchemaDigest { identity, judges_files }`, with `SchemaDigest::canonical(&Value)`. That is the one place a schema is serialized and hashed, and it counts `Work::CanonicalSchemaSerialization`, a new counter folded into Darkmatter's snapshot. `judges_files(canonical_text)` is the old string classification, now a shared public helper.
- `LayeredIdentity` builds a layered identity in order: `layer(LayerRole::{Baseline, Trigger, Document}, digest)`, `derived_from(digest)`, and `phase_projection(phase, Option<projected document digest>)`, then `finish()`. The bytes it hashes are a length-prefixed domain, `LAYERED_IDENTITY_REVISION`, `resolve::MERGE_ALGORITHM_REVISION`, the step count, and then each step: a fixed tag, followed by the identity's variant tag, its revision as a little-endian u32, and its digest as a little-endian u64. A phase step adds `phase::PROJECTION_REVISION` and a phase tag. `judges_files` is the OR of every input.
- Cache keys are now `CacheKey { identity: SchemaIdentity, anchors: u64 }`. `canonical_hash` became `anchor_hash`, which hashes the base directory, fallback, caller origins, and repository anchors as before. The structural slot uses the same key type. Lookups still compare the full `JudgedIn`, and eviction is unchanged.
- New entry points: `validator_for_shared_with_digest`, `structural_validator_identified`, `canonical_digest_shared` (memoized per allocation, as before), `CompiledValidators::build_with_digest` (a root union still wraps its arms and finds each one canonically), and `ConversionPlan::for_identity` (the plan cache is keyed by `SchemaIdentity`, so a plan finds the structural validator compiled under the same identity). Canonical APIs are unchanged. `shared_schema_hash` and `for_hashed` stay canonical.
- `shipped::CanonicalDigestRecord { revision, digest, judges_files }` is the wire form. A record computed under another `CANONICAL_IDENTITY_REVISION` is refused.

### Merge provenance (Wave 7, Rule 13)

- Generated records: Darkmatter's payload member `baseline` is now `BaselineRecord { json_schema, digest }`, and `PayloadRecord` gained `digest`. Both are computed at generation. `ShippedBaseline.digest` comes from the record; the from-sources fallback computes it. `ResolvedPayload.digest` is `Some` only for a prepared payload.
- `schemas/mod.rs`: `BaselineSchema.digest` is `Some` for the shipped baseline. A caller's baseline gets its digest from `canonical_digest_shared`, once per allocation. `TriggerLayer.digest` is the prepared digest, or the per-allocation canonical one. A template payload is hashed after it resolves. `site_schema_with_override` identifies baseline-only and lone-trigger schemas as one-layer layered identities. `merged_trigger_schema` returns `(Arc<Value>, SchemaDigest)`. Its memo stores the identity, so a repeated document is not hashed again; on a miss the document layer is hashed once, over its fully resolved JSON, imports and template results included. `EffectiveSchema.digest: Option<SchemaDigest>` feeds both `build_with_digest` and the conversion plan.
- `phase_json_schema(effective, effective_digest, projected_document, phase)` reuses the effective identity when the projection leaves the schema unchanged. When it changes the schema, it derives `derived_from(effective).phase_projection(phase, canonical(projected document))`. When the effective schema has no tracked identity, the result is `None` and takes the canonical route. The memo also matches on the digest.
- These stay on the canonical route: `EffectiveSchema::for_yaml_document`, `rewrite.rs` (eager-file arms), wrapped root-union arms, `Prepared`, and the standalone APIs.
- Revisions: Darkmatter `PREPARATION_REVISION` 1 → 2 (record shape). Claudine `PREPARATION_REVISION` 1 → 2: its bundle record embeds `PayloadRecord`, so its record shape changed too. Simplified Schema's artifact is unchanged. I ran root `just shipped-schemas` once; it regenerated Darkmatter's and Claudine's artifacts, and every freshness test passes.

### Drift found and fixed

`darkmatter/docs/topics/schemas/definition.md` ("Validator Cache") said the built-in baseline declares an eager `file` property, so every document compiled the merged baseline per context. That was stale before this phase. The baseline types `style.page.stylesheet` with the lazy reference format, and the generated record says `judges_files: false`. I rewrote the section around the two identity forms, with a table, a flow diagram, and an example. I also rewrote the stale paragraph to describe what does cost one compile per context, a document's own `file(eager)`.

### Tests (requirement → test)

Simplified Schema (`lib/tests/l1/schema_identities.rs`, 12 tests; 10 without `work-counters`; registered in `tests/l1/main.rs`):

| Requirement | Test |
| --- | --- |
| Stable ordered encoding (pinned values) | `a_layered_identity_is_pinned_for_fixed_inputs` (`83df5d9770344f89`, projection `e89539b57da19d99`) |
| Cross-process stability, wiring once | `a_layered_identity_is_stable_across_processes` re-runs the test binary twice in `identity_helper_entrypoint` mode |
| Role, order, repetition, phase, and projected-document changes | `layer_order_role_and_projection_each_change_the_identity` (pairwise distinct) |
| Canonical object-key invariance, array-order significance | `a_canonical_digest_ignores_object_key_order_and_keeps_array_order` |
| File-reference syntax vs eager file / `match()`; layered OR | `only_the_eager_file_format_and_match_judge_files` |
| Canonical/layered domain separation (same 64-bit digest, different verdicts) | `canonical_and_layered_identities_never_share_a_validator` |
| Unchanged schema across differing data shares one validator | `documents_with_one_layered_identity_share_its_validator` |
| Base-directory and caller-origin separation; a file-free schema shared | `a_file_judging_layered_identity_is_keyed_by_document_folder_and_caller_origins` (verdicts under `feature = "file"`) |
| The canonical route still shares equal schemas across allocations; `shared_schema_hash` stays canonical | `the_canonical_route_shares_equal_schemas_across_allocations` |
| A known digest serializes nothing; the canonical route serializes once per allocation | `counted::a_known_digest_never_serializes_the_schema` |
| A conversion plan found by identity reuses that validator | `counted::a_conversion_plan_by_identity_reuses_the_validator_compiled_under_it` |

Darkmatter (`lib/tests/l1/layered_validator_identities.rs`, 13 tests; registered in `tests/l1/main.rs`). Every isolation case runs in one process on the shared cache and asserts verdicts that a wrong validator would flip:

| Requirement | Test |
| --- | --- |
| Reordered conflicting layers | `reordered_conflicting_triggers_never_share_a_validator` |
| Baseline/document role change | `baseline_and_document_roles_are_part_of_the_identity` |
| Changed disk import, unchanged importer (and restored) | `a_changed_import_changes_the_identity` |
| Changed resolved template schema | `a_changed_template_result_changes_the_identity` (granted `load_data` slot, data rewritten) |
| Launch vs completion required/generated | `launch_and_completion_are_distinct_identities` |
| Passive eager-file projection | `the_passive_eager_file_projection_is_its_own_identity` |
| Negative controls: document folder, repository context | `a_file_judging_identity_stays_keyed_by_folder_and_repository` |
| Negative control: fallback directory | `counted::the_fallback_folder_keeps_file_judging_validators_apart` (the fallback never changes a document verdict, so this one asserts compile counts) |
| Positive sharing across data documents | `counted::documents_with_identical_layers_share_a_validator` |
| Fresh shipped baseline-only validation: generated digest, zero serialization | `counted::the_shipped_baseline_is_never_serialized_for_its_key` |
| A dynamic layer is serialized once; a repeated allocation reuses it | `counted::only_the_document_layer_is_serialized_and_only_once` |
| Canonical fallback exercised | `counted::a_root_union_document_takes_the_canonical_route` |
| Reset → cold → warm; constants stay loaded | `counted::a_reset_empties_dynamic_caches_and_keeps_shipped_data` |

Load-bearing check (mutations, reverted byte-for-byte afterwards):
- Replacing the trigger and document digests in `merged_trigger_schema` with a constant failed 6 of the 13 tests, among them the reorder, import, template, folder/repository, and fallback tests.
- Reusing the unphased identity for a changed phase result failed the launch/completion and passive-projection tests.

Record matrix additions:
- Darkmatter `shipped_artifacts.rs` cells: baseline JSON not an object (now under `/payload/baseline/json_schema`), a baseline record of the wrong type, a digest of another revision (`UnusableRecord` at `/payload/baseline/digest/revision`), and an absent, null, short, or wrong-typed digest or classification (`MalformedArtifact`). The round-trip test asserts that the recorded baseline digest equals `CanonicalDigestRecord::of` of the lowered schema.
- Claudine `shipped_artifacts.rs`: `a_payload_digest_of_another_revision_is_refused` and `an_absent_payload_digest_is_malformed`.

Smell sweep over the new decoders (`BaselineRecord`, `PayloadRecord.digest`, `CanonicalDigestRecord`): no `#[serde(default)]` and no `Option` fields; every field is required, and `deny_unknown_fields` applies.

### Gates

- `darkmatter`: `just test` 16425 passed, 10 skipped; "none at or above 5.0s" (load average about 20). `just lint` clean, including the zed-dmls wasm check. `just check-tier-coverage darkmatter`: nothing stranded.
- `simplified-schema`: `just test` 1977 passed with `work-counters,terminal`, then 1693 passed without default features; none at or above 5.0 s. `just lint` clean after one fix: an unused variable in the no-`file` build of the new test.
- `claudine`, changed directly through its revision, artifact, and tests: `just test` 11942 passed, 15 skipped, none at or above 5.0 s. Its recipe covers every reverse dependent of `claudine`. `just lint` clean.
- Shipped-artifact freshness tests pass in all three areas after the single root `just shipped-schemas`.

No pre-existing failures were seen. The change adds no path handling and no `#[cfg]` branch, so no Windows cross-check was run; CI covers the other operating systems. The layered encoding is fixed-width little-endian, and its pinned values are platform-independent by construction.
- Scope calculator over the working tree (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `change_class: package`, `packages` = claudine, darkmatter, simplified-schema, test-toolkit, claudine-cli, darkmatter-cli. No gate was skipped.
- `just slow-tests fresh` reports the record as stale only because the plan and this log changed after the runs. It says neither file is inside a directory those tests cover; every recorded run had zero tests at or above 5.0 s.

## Phase 5

### Orchestration

Two foreground subagents ran in parallel for Wave 8: one removed the speed-only schema fixture from Claudine's tests, the other finished the docs and skills (Markdown only). Each verified only its own slice. The coordinator then fixed the drifted doc comments the docs agent listed, regenerated the artifacts, and ran every gate once per area, with nothing else running. The host was not quiet at any point. Other sessions' processes (node, Blender, `kache`) held the load average between 8 and 70, and every timing below records the load it was taken under.

### Restore shipped fixtures (Wave 8, Rule 15)

`CliProcessFixture::seed_inline_metadata_schema(schema, document_library)` wrote the shipped `inline-compose.yaml` as the fixture home's `schemas/claudine.yaml`, with `document.yaml` beside it. That shadowed Claudine's embedded trigger with a smaller schema, purely for speed. Commits `27845edca` and `5ceb22a89` introduced it, and the closed audit `audit-final-cli-speed.md` describes it; that audit was left untouched.

- **Roster: seven calling files**, all in `claudine/cli/tests/l1/`. This corrects the spec's historical example of eight.
  - `agent_text_is_data.rs`: 4 calls (two in tests, one in `assert_value_refused`, one in `plain_text_with_a_colon_is_still_repaired`)
  - `compose_prompt_tiers.rs`: 2 calls (`assert_local_root_wins`, `assert_form_resolves`)
  - `entry_point_parity.rs`: 2 calls in `fixtures()`. `fixtures`, `run_matrix_row`, and `run_candidate_part` lost their `host_schema`/`document_library` parameters, and the row macros no longer `include_str!` the two schemas. The Darkmatter `entry_point_parity/mod.rs` input edge stays.
  - `inline_completion_lifecycle.rs`, `sequence_magic_reference.rs` (in `staged()`), `sequence_overlay_pty.rs`, `sequence_schema.rs`: 1 call each.
- The helper is deleted from `claudine/cli/tests/common/mod.rs`.
- **Equivalent spelling found and removed:** `claudine/lib/src/invocation_context/document_tests.rs` had `Fixture::with_inline_metadata_schema`, the same workaround on the library side. It was added in `85068cc4e`, the same minute as `27845edca`. It and its 4 callers' use of it are gone, and the file is now byte-identical to before that commit. Its tests check how environment maps are routed, not schema behavior.
- **Retained, because shadowing or a custom schema is what each test checks:**
  - `claudine/cli/tests/l1/launch_trigger_schemas.rs`: a malformed or loosening disk `claudine.yaml` shadows the shipped one.
  - `invalid_trigger_warnings.rs`: custom invalid triggers.
  - `claudine/lib/tests/l1/host_catalog.rs`: writes the full shipped bundle plus `globals.yaml`.
  - `claudine_trigger_selection_parity.rs`: an empty `schemas/` with the embedded bundle.
  - `claudine_schemas.rs`: writes the whole shipped bundle under its real names.
  - The in-memory `include_str!` test subjects: `claudine_schema_parity.rs`, `container_schema_recursion.rs`, `claudine_schemas_embedded.rs`, `claudine_schema_shipped_corpus.rs`, `lifecycle/tests/nested_span.rs`.
- No assertion changed. All 174 tests in the seven modules passed, plus the 6 `document_tests`; the slowest took 1.97 s.

### Documentation (Wave 8)

- `simplified-schema/docs/topics/shipped-artifacts.md` covers:
  - a regeneration example, and what to do when the freshness test fails;
  - the stale-lower error;
  - a revision-bump table covering the preparation revisions, `MERGE_ALGORITHM_REVISION`, `PROJECTION_REVISION`, and `CANONICAL_IDENTITY_REVISION`;
  - a Mermaid load/fallback diagram across the three owners;
  - what a fallback costs.

  Claudine's artifact is no longer marked **planned**.
- Darkmatter `definition.md`, `schema-discovery.md`, `schema-triggers.md`, and `schema-validation.md`, and `claudine/docs/topics/schemas.md` and `claudine/schemas/README.md`: gap-fills only. No doc advised shadowing for speed. `dmls-schema-support.md` and `claudine/docs/topics/composition.md` had nothing stale.
- Skills: `simplified-schema`, `darkmatter` (`SKILL.md`, `schema.md`, `expressions.md`), `claudine` (`SKILL.md`, `cli-commands.md`), and `rust-testing/test-speed.md` (new cause 8, with figures only from measurements in this log).
- Dependency docs: no change was needed. The only new direct dependency, `serde_path_to_error` in Simplified Schema (Phase 1), is already listed.

### Drift found and fixed (comment-only)

These comments still described whole-artifact refusal, a "memo-free" generation, a cache "keyed by canonicalised schema hash", or a baseline record without its digest. The code is correct; the comments were brought in line with it:

- `simplified-schema/lib/src/shipped/mod.rs`: module docs mention per-section refusal under `accept_deferred`; "identity-keyed memo" in the fresh-generation docs and in `PreparationMode::Uncached`
- `simplified-schema/lib/src/shipped/wire.rs`: "All four refuse"
- `simplified-schema/lib/src/validate.rs`: `ValidatorCache` keyed by `SchemaIdentity`
- `simplified-schema/lib/src/lib.rs`: `reset_process_caches` keeps the shipped registry
- `darkmatter/lib/src/markdown/schemas/shipped.rs`: `baseline` carries its digest
- the static catalog fallbacks ("artifact or its section was refused"): `catalog.rs`, `compose/expression/catalog/mod.rs`, `effects/types.rs`, `yaml_documents/registry.rs`
- `claudine/lib/src/composition/schema/shipped.rs`: `prepare_payload`

### Final gates (Wave 9)

Run sequentially and alone, with canonical recipes:

| Area | `just test` | Slow (≥ 5 s) | `just lint` |
| --- | --- | --- | --- |
| `simplified-schema` | 1977 + 1693 passed | 0 | clean (incl. `--no-default-features`) |
| `darkmatter` (lib, cli, dmls, zed) | 16428 passed, 10 skipped | 0 | clean (incl. wasm32-wasip2) |
| `claudine` | 11942 passed, 15 skipped | 14 during a load spike (load average 31 → 52); each 1.0–2.2 s alone at load 14 | clean |

- **Claudine's slow tests.** The 14 are the six `shipped_artifacts` generation tests, `claudine_schemas::shared_payload_properties_have_identical_types`, three `sequence_schema` tests, two `container_schema_recursion` tests, and `error_guards::production_sources_pass_every_scan_backed_guard`. They took 5.0–7.8 s in the run, all under the 10 s loaded allowance, and were cleared by measuring each alone. They are recorded as load-sensitive. An earlier full Claudine run of this phase (load about 60) had one, the same `shared_payload_properties_have_identical_types` (5.23 s in the run, 2.15 s alone).
- **Summed JUnit durations.** JUnit files are preserved at `/tmp/pss-final/{darkmatter,claudine}-junit.xml`, with Phase 1's at `/tmp/pss-baseline/`.

  | Area | Phase 1 | Phase 5 |
  | --- | --- | --- |
  | Darkmatter | 2813.0 s (16349 tests, max 4.65 s) | 1570.0 s (16429, max 3.44 s) |
  | Claudine | 4879.0 s (11917, max 4.98 s) | 4257.6 s (11942, max 7.76 s under the load spike) |

  Both are under load, so they show direction only.
- **Other checks.** All passed or were clean:
  - `just shipped-schemas` regenerated all three artifacts, and they were already current, both before and after this phase's code changes.
  - `scripts/ci/test_affected_scope.py::RealWorkspaceTestInputTests`: 10 passed.
  - `cargo tree -p simplified-schema --no-default-features -e normal` contains no Darkmatter, Claudine, Content Policy, or CLI crate; the only repository crates are `biscuit-file` and `biscuit-hash`.
  - `cargo check -p simplified-schema -p darkmatter -p claudine --examples`: the three generator examples compile.
- **Dependents.** `cargo check --all-targets` passed for `biscuit-icon-cli`, `biscuit-speaks-cli`, `biscuit-terminal-cli`, `messenger`, `messenger-cli`, `claudine-contract`, `claudine-gen`, `playa-cli`, `sniff-detection-cli`, and `research`. The 73 tests in `messenger` and `research` that match schema, compose, frontmatter, or darkmatter passed.
- **Scope calculator.** `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gave `change_class: package` with `packages` = claudine, claudine-cli, darkmatter, simplified-schema, test-toolkit, darkmatter-cli, so no gate was skipped.
- **Environments.** Only macOS was tested locally. The phase adds no path handling and no `#[cfg]` branch, so no Windows cross-check was run. Existing CI proves byte freshness on Linux, macOS, Windows, and WSL2.

### Performance evidence (Wave 9)

The fixture and commands are exactly Phase 1's: `/tmp/pss-timing/run.sh`, `hyperfine -N --warmup 1 --runs 5`, the same `env -i` environments, and the same staged fixture. Debug and release binaries of `md` and `claudine` were built before measuring (`cargo build [--release] -p darkmatter-cli --bin md`, `-p claudine-cli --bin claudine`) and copied to `/tmp/pss-final/bin3/`. The host never went quiet, so the Phase 1 binaries (`/tmp/pss-baseline/bin/`) were re-run back to back with the final ones. The paired comparison is the evidence; the Phase 1 numbers stay as recorded.

**First sample (end of Wave 8; load about 30):** `md` 111 / 30 ms and Claudine 879 / 131 ms (debug / release). This missed all three targets, so both binaries were profiled with the Time Profiler (`xctrace record --all-processes` over loops of the release and debug commands, with samples aggregated per symbol):

- **`RawDeclarations::from_json_schema`: about 26% of a release `md compose`.** The call precheck built `DeclaredTypes::for_site` for every document. That eagerly projected the merged JSON Schema (the baseline alone is large), even for a document with no expressions.
  - Fix: `DeclaredTypes` now keeps the `EffectiveSchema` (a `RawSource` wrapper) and calls `raw_declarations()` on the first declaration read.
  - New counter `Work::RawDeclarationProjection`.
  - Release `md compose` went from 30.2 to 20.4 ms in a back-to-back `hyperfine` run.
- **`serde_path_to_error`: about 10 points of Claudine's roughly 15% bundle-section decode.**
  - Fix: `wire::decode_typed` decodes with plain `serde_json` and repeats the decode under `serde_path_to_error` only to locate a refusal. The decode is deterministic, so the repeat fails at the same place, and refusal pointers are unchanged.
- **Per-document lazy trigger decode: tried and reverted.** It kept the wire format: a deferred record held each document's outcome as a `RawValue` and decoded it on first `load`. But `TriggerRegistry::append_embedded_bundle` loads every bundle document's payload into the registry's parallel `payloads` vector up front, so laziness at the record level decoded everything anyway. A real gain needs that vector made lazy for prepared entries, which touches discovery, assembly, tracing, and `effective_for`. The profile put the most it could save at about half the bundle decode, roughly 60 ms debug. The change was reverted completely; `prepared.rs`, `triggers/mod.rs`, and Claudine's `shipped.rs` are back to their Phase 4 code.

**Final sample (pair 2; load average 28–34, comparable to Phase 1's 33–41; all runs exited 0):**

| Workload | Build | Phase 1 binary, same session | Final | Target | Met |
| --- | --- | --- | --- | --- | --- |
| `md compose` three-line document | debug | 290.5 ms | 65.2 ms | ≤ 90 ms | yes |
| same | release | 58.3 ms | 19.5 ms | ≤ 20 ms | yes, narrowly (pair 1: 20.3 ms at load 36) |
| `claudine compose --goose --dry-run prompts/clarify.md` | debug | 1449.6 ms | 839.0 ms | ≤ 650 ms | **no** |
| same | release | 210.5 ms | 119.0 ms | (recorded only) | — |

Phase 1 medians were 291.2 / 56.9 / 1415.9 / 207.5 ms. Final runs: md debug 65.2, 65.0, 69.6, 65.3, 64.5; md release 19.5, 19.9, 19.1, 19.1, 20.6; Claudine debug 836.7, 840.2, 842.2, 838.4, 839.0; Claudine release 119.0, 118.5, 115.7, 119.9, 119.4.

**Why Claudine misses its debug target.** A debug Time Profiler sample of the Claudine run (about 860 ms per run) breaks down as follows:

| Work | Share of the run | Approximate time |
| --- | --- | --- |
| Compiling the effective validator (`CompiledValidators::build_with_digest` → `jsonschema`) | ~17% | ~150 ms |
| Decoding the prepared bundle section (both 1.8 MB payloads, though one matches) | ~15% | ~125 ms |
| Resolving and parsing the prompt's own `$schema` | ~7% | ~65 ms |
| Merging trigger layers | ~6% | ~50 ms |
| Markdown parsing and composition stages | rest | — |

- Every shipped preparation cost the spec listed is gone, and the deterministic counters prove it (see below).
- The remaining cost is per-document work the spec keeps at run time, apart from the bundle decode.
- The measured saving (1450 → 839 ms) is larger than the spec's estimate of about 520 ms of shipped work.
- The 650 ms figure assumed the spec's quieter 1.2 s starting point. On this loaded host the Phase 1 binary runs at 1.45 s.

This miss is recorded in `decisions_for_review`.

**Deterministic evidence (counters, independent of timing):**

- `layered_validator_identities::counted::the_shipped_baseline_is_never_serialized_for_its_key`: 0 canonical serializations.
- The Phase 2/3 zero-preparation counters (`ShippedRuntimePreparation`, `BaselineCheck`, `PayloadMergeCheck`, `PayloadCompileCheck` at 0 for accepted artifacts). These sit beside the source, dependency, configuration, revision, and record refusal tests, which are all still green.
- New: `raw_declaration_projection::a_document_without_expressions_never_projects_the_baseline` (0 projections).

### Tests (requirement → test)

| Requirement | Test |
| --- | --- |
| The removed fixture changes no verdict | the seven modules (174 tests) and `invocation_context::document_tests` (6) unchanged and passing on the shipped schemas |
| Shadowing is still honored where it is the behavior | `launch_trigger_schemas.rs` (retained, passing) |
| A document with no expressions never projects the merged schema | `raw_declaration_projection::a_document_without_expressions_never_projects_the_baseline`: output rendered, 0 `RawDeclarationProjection` |
| A declaration read still projects (once) and decides the verdict | `raw_declaration_projection::a_call_on_a_baseline_property_reads_its_projected_type`: `upper(tags)` warns `dm.expression.uncertain_call` with actual type `string[] \| null` (the baseline's `tags`), exactly 1 projection |
| Undeclared-property detection is unchanged | `raw_declaration_projection::a_misspelled_property_is_still_reported_undeclared`, plus the existing `unknown_identifier_warning`, `design_time_call_check`, and `compose_member_warnings` suites |
| Plain decode with path-tracked refusal keeps every refusal's pointer and reason | the existing section and record refusal tests: Claudine `shipped_artifacts::{an_absent_payload_digest_is_malformed, a_payload_digest_of_another_revision_is_refused, a_null_catalog_description_is_none_and_an_absent_one_is_malformed, …}`, and the Simplified Schema and Darkmatter `shipped_artifacts` malformed-section tests; all assert pointers that only the path-tracked repeat produces |
| End to end through the normal invocation path | `md compose` and `claudine compose` of the timing fixture, both exit 0. The CLI on `[{{ descripton }}]` and `[{{ upper(tags) }}]` prints the same warnings from the Phase 4 binary and the final one |

New test file: `darkmatter/lib/tests/l1/raw_declaration_projection.rs`, declared in `tests/l1/main.rs` under `#[cfg(feature = "work-counters")]`, a feature Darkmatter's CI metadata enables. Each test anchors its request in its own temp directory and runs in 0.06–0.09 s.
