---
implementation_1: "2026-10-30T19:40:31-07:00"
---

# Log: Reference Navigation and Hover

## Implementation of Review Findings #1

> **started at:** 2026-10-30T19:40:31-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/fixes/2026-10-30-reference-navigation-and-hover/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed as `false` before repairs began

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Informal schema documents reject the permitted `categories` property; informal documents with valid `categories` must be usable schemas | fixed | every child below fixed or clean; shared key-set predicate `StandaloneSchemaEnvelope::admits_top_level_key`; fail-before proof failed 11/12 new tests; full `just test` in both areas green |
| R1-F1.1 | standalone parser: informal + `categories` recognized like tagged control | fixed | `schema_target_classification::an_informal_document_with_categories_is_recognized_and_exposes_them` (envelope `Pure`, categories `[Core]`; tagged control `Tagged`) |
| R1-F1.2 | document-schema resolver: direct file reference resolves | fixed | `schema_target_classification::an_informal_document_with_categories_is_a_schema_target` |
| R1-F1.3 | target inspection: accepted with preview | fixed | same test: `inspect_schema_target` Ok with lines `["title: string"]` |
| R1-F1.4 | named-type loading `Id@./subject.yaml` imports `Id` | fixed | `schema_target_classification::an_informal_document_with_categories_serves_named_imports` |
| R1-F1.5 | whole-file delegation `$schema: ./subject.yaml` resolves | fixed | `schema_target_classification::an_informal_document_with_categories_is_a_delegation_and_root_union_arm` |
| R1-F1.6 | root-union file arm resolves | fixed | same test: `[./subject.yaml, {count: number}]` resolves |
| R1-F1.7 | darkmatter-cli normal validation exits 0, `valid: true` | fixed | `darkmatter/cli` `schema_target_recognition::an_informal_schema_file_with_categories_applies_its_schema` (one spawn, exit 0, `valid: true`) |
| R1-F1.8 | DMLS `$schema` value hover shows definition preview | fixed | `value_hovers::informal_and_tagged_targets_with_categories_preview_their_definition` (exact `Schema Definition:` preview, informal and tagged) |
| R1-F1.9 | trigger payload assembly applies the payload | fixed | `trigger_discovery_recovery::a_payload_file_with_categories_applies_informal_or_tagged` (trigger installed, payload `level` enforced) |
| R1-F1.10 | DMLS schema-root classification yields `NamedSchema` | fixed | `overlay::registry::tests::informal_and_tagged_schemas_with_categories_classify_as_named_schemas` (`NamedSchema` both; `Unclassified` without `$schema`) |
| R1-F1.11 | file selection / navigation projection (content-independent) | clean | `select_schema_file` and the projection's `Found` branch never read contents; `schema_reference` 34 and `referenced_targets` 48 pass |
| R1-F1.12 | JSON recognition and example artifacts keep their own contracts | clean | JSON goes through `classify_json_schema_text` (dialect + meta-schema); examples use their own envelope; neither calls the informal key set |
| R1-F1.13 | informal `categories` invalid shapes reach the shared category validator with located errors; `[]` accepted | fixed | `schema_target_classification::informal_categories_field_shapes`: absent/`[]`/`[Core]` accepted; null, `Core`, `{}` rejected "`categories` must be a list"; `[Core, 3]` names `categories[1]`; `[3]` names `categories[0]`; each names the file and is not "unsupported top-level"; `foo: 1` still rejected; `categories_without_schema_is_not_an_informal_document` keeps `NoSchemaProperty` |
| R1-F1.14 | informal-envelope documentation lists `categories` | fixed | `schema-documents.md`, `schema-object.md`, `definition.md`, `authoring-schemas.md`, darkmatter and simplified-schema skills, and drifted rustdoc in `standalone.rs`/`resolve.rs`/DMLS `overlay/schema.rs` updated |
| R1-F1.15 | (discovered sibling) DMLS lexical envelope claim `standalone_envelope_claim` repeated the informal allowlist without `categories` | fixed | switched to the shared predicate; informal+categories case added to `envelope_claim_recognizes_block_and_flow_presentation` (`envelope_claim` 21 passed) |
| R1-F1.16 | (discovered sibling) `to_simplified_yaml` export dropped informal `categories`, losing the `category(...)` restriction on round trip | fixed | `schema/export.rs::write_document` writes `categories` for both envelopes; `schema_object` `informal_categories` test (`schema_object` 37 passed) |
| R1-F1.17 | (checked sibling) global-catalog loader key list in `darkmatter/lib/src/markdown/schemas/catalog.rs` | clean | separate strict `GlobalCatalog` contract the spec leaves unchanged; requires `kind: schema`, so not the informal envelope |

### Progress

- starting the work on 'R1-F1 informal schema documents reject categories' at 19:40:54
        - defect class: two envelope allowlists that must describe the same informal schema shape disagree with the format definition, so a permitted metadata property is rejected before the shared field validator can run
        - orchestrator pre-read: `parse_standalone_schema_document` parses `categories` from the mapping regardless of envelope, so admitting the key in the informal allowlists (`recognize_standalone` in `simplified/standalone.rs` and `classify_yaml_schema_text` in `resolve.rs`) wires informal documents to the existing `parse_categories` validator
        - dispatched one foreground subagent (rust-developer) for repair, sibling sweep, tests, and docs
        - added `StandaloneSchemaEnvelope::admits_top_level_key` in `simplified/standalone.rs` as the one key-set definition; `recognize_standalone` (both envelopes) and `classify_yaml_schema_text` now read it, so informal documents admit `categories`
        - sibling found: DMLS lexical claim `overlay/schema.rs::standalone_envelope_claim` repeated the informal allowlist without `categories`; switched it to the shared predicate and added an informal+categories case to `envelope_claim_recognizes_block_and_flow_presentation`
        - sibling checked, left: `darkmatter/lib/src/markdown/schemas/catalog.rs` global-catalog loader allowlist (`kind|description|$schema`) is the separate authored-catalog contract (requires `kind: schema`)
        - sibling found: `simplified-schema/lib/src/schema/export.rs::write_document` wrote `categories` only for the tagged envelope, so `to_simplified_yaml` of an informal document with categories dropped the list (and with it the `category(...)` restriction); now written for both envelopes, covered by `schema_object::...::informal_categories`
        - simplified-schema tests added in `tests/l1/schema_target_classification.rs` (informal+categories parser, resolver, inspection, import, delegation, root union, field-shape matrix, `$schema`-less control); `INFORMAL` control now carries `categories` too; all pass
        - darkmatter-side tests added: CLI `schema_target_recognition::an_informal_schema_file_with_categories_applies_its_schema`, DMLS `value_hovers::informal_and_tagged_targets_with_categories_preview_their_definition`, DMLS `overlay::registry::tests::informal_and_tagged_schemas_with_categories_classify_as_named_schemas`, lib `trigger_discovery_recovery::a_payload_file_with_categories_applies_informal_or_tagged`; all pass
        - fail-before proof: temporarily restricting `categories` to the tagged envelope in `admits_top_level_key` failed 11 of the 12 new/extended tests (the `$schema`-less control passes either way, as intended); source restored
        - docs updated for informal `categories`: `simplified-schema/docs/topics/language/schema-documents.md` (classification table, Pure Envelope, Categories), `simplified-schema/docs/topics/schema-object.md`, `darkmatter/docs/topics/schemas/definition.md` (envelope table), `darkmatter/docs/topics/schemas/authoring-schemas.md` (pure/tagged list); skills `.claude/skills/darkmatter/{SKILL.md,schema.md}` and `.claude/skills/simplified-schema/SKILL.md`; drifted comments in `standalone.rs` (module docs, `categories` field, `SchemaCategory`, `Pure` variant, `parse_standalone_schema_document` docs), `resolve.rs::classify_yaml_schema_text` docs, and DMLS `standalone_envelope_claim` docs fixed
        - verification: simplified-schema `just test categor` 8 + 1119 (minimal leg) passed; `schema_target_classification` 25, `schema_object` 37, `schema_target_inspection` 8, `shipped_corpus` 3 passed; darkmatter `just test value_hovers` 24, `schema_reference` 34, `referenced_targets` 48, `schema_target_recognition` 3, `trigger_discovery_recovery` 32, `envelope_claim` 21, `overlay::registry` 30, `schemas_named_import_errors` 9, `schema_enhancements_matrix` 10, `shipped_schema_corpus` 10 passed; `just lint` clean in both areas; `check-tier-coverage` reports no stranded tests for either
- work completed for 'R1-F1 informal schema documents reject categories' at 19:52:39
        - orchestrator reconciliation: read the combined diff; the key-set predicate is used at all three allowlist sites; the `errors.rs` and `source.rs` hunks in the working tree predate this cycle
        - decisions recorded for the author (added to the review's `human_review_items`): the shared key set is a public `StandaloneSchemaEnvelope::admits_top_level_key` method (alternatives: a crate-private constant plus a guarded DMLS copy, or a public constant slice) so DMLS cannot drift; informal `to_simplified_yaml` output now keeps `categories` (alternative: keep dropping it, which loses the restriction)

### Completion Report

- **outcome:** complete
- **elapsed:** about 13 minutes (19:40:31 to 19:53)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **child rows:** 17 (14 fixed, 3 clean); none open
- **repairs verified:**
        - R1-F1: one definition of each envelope's top-level key set (`StandaloneSchemaEnvelope::admits_top_level_key`) now drives the standalone recognizer, `$schema` target classification, and the DMLS lexical claim; informal documents with `categories` reach the existing `parse_categories` validator
        - discovered sibling R1-F1.16: informal export now preserves `categories`
- **open findings:** none
- **changed files (this cycle):**
        - code: `simplified-schema/lib/src/simplified/standalone.rs`, `simplified-schema/lib/src/resolve.rs`, `simplified-schema/lib/src/schema/export.rs`, `darkmatter/dmls/src/overlay/schema.rs`, `darkmatter/dmls/src/overlay/registry.rs` (test module)
        - tests: `simplified-schema/lib/tests/l1/schema_target_classification.rs`, `simplified-schema/lib/tests/l1/schema_object.rs`, `darkmatter/cli/tests/l1/schema_target_recognition.rs`, `darkmatter/dmls/tests/l1/value_hovers.rs`, `darkmatter/lib/tests/l1/trigger_discovery_recovery.rs`
        - docs and skills: `simplified-schema/docs/topics/language/schema-documents.md`, `simplified-schema/docs/topics/schema-object.md`, `darkmatter/docs/topics/schemas/definition.md`, `darkmatter/docs/topics/schemas/authoring-schemas.md`, `.claude/skills/darkmatter/SKILL.md`, `.claude/skills/darkmatter/schema.md`, `.claude/skills/simplified-schema/SKILL.md`
- **final verification (orchestrator, after all changes, no subagent running):**
        - `cd simplified-schema && just test`: 1119 passed (all legs); verifies R1-F1.1–F1.6, F1.13, F1.16
        - `cd simplified-schema && just lint`: clean (both feature sets)
        - `cd darkmatter && just test`: 12874 passed, 10 skipped, 0 failed; verifies R1-F1.7–F1.12, F1.15
        - `cd darkmatter && just lint`: clean, including the zed-dmls wasm32-wasip2 check
        - `just check-tier-coverage darkmatter` and `just check-tier-coverage simplified-schema`: no stranded tests
        - cross-OS check skipped: the change is a key-set predicate with no path, process, or OS-specific code
- **performance:** no measurement required; `deferred_perf_measurement` not set
