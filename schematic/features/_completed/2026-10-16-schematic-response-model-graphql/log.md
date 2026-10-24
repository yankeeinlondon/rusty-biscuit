---
implementation_1: "2026-10-22T20:04:07-07:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-10-22T20:04:07-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-16-schematic-response-model-graphql/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it was confirmed `false` before repairs began
- the review has two high findings (R1-F1, R1-F2) and two observations; the observations are not repaired in this cycle (the follow-up pass carries them)

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | OpenAI stream events decode as the wrong variants | fixed | importer carries `discriminator` into `TypeAlias`; generator emits tag-dispatching `Deserialize`; all children below verified |
| R1-F1.1–F1.53 | Each of the 53 `BetaResponseStreamEvent` alternatives decodes as itself with payload retained | fixed (F1.1–F1.2 were clean controls) | `definitions/tests/openai_stream_events.rs`: 3/3 pass; the guard decodes all 53 as themselves, compares its list with the spec's `anyOf`, and reported 51 wrong when the old untagged derive was temporarily restored |
| R1-F1.54 | Anthropic message events and nested deltas: tagged, no overlap | clean | `MessageStreamEvent`/`ContentBlockDelta` are `#[serde(tag = "type")]`; hand-written, not imported |
| R1-F1.55 | LM Studio chat events: tagged | clean | `ChatStreamEvent` is `#[serde(tag = "type")]` |
| R1-F1.56 | Ollama generate item: error/chunk disjoint | clean | `Error(StreamErrorLine)` first, requires `error`; chunks require `model`/`created_at`/`done`; `just test-schema` 65/65 |
| R1-F1.57 | Ollama chat item: error/chunk disjoint | clean | same invariant as F1.56 |
| R1-F1.58 | Ollama pull/push item: error/progress disjoint | clean | error requires `error`; progress requires `status` |
| R1-F1.59 | Ollama create item: error/progress disjoint | clean | same invariant as F1.58 |
| R1-F1.60 | Ollama OpenAI-compatible chunks: concrete structs | clean | `OpenAIChatStreamChunk`/`OpenAICompletionStreamChunk` are single structs |
| R1-F1.61 | ElevenLabs timestamp chunks and raw audio | clean | `SpeechWithTimestampsResponse` is one struct; raw audio is bytes |
| R1-F1.62 | Generated `GetResponse` stream projection preserves the correct typed item | fixed | `schema/tests/streaming_providers.rs` `openai_get_response_sse_ends_cleanly_at_connection_close` passes (10/10 in binary) |
| R1-F1.63 | Generated-provider OpenAI test covers text, completed, and error events | fixed | same test now streams text delta, `response.completed` with payload, and `error`; stale first-match comment rewritten |
| R1-F1.64 | Sibling: all 52 top-level discriminated unions in the OpenAI import | fixed | 47 now dispatch by tag; `InputItem`/`BetaInputItem` stay untagged with an import warning (`type` optional on `EasyInputMessage`); 3 import as `Value` |
| R1-F1.65 | Sibling: unions without a `discriminator` keyword separable by a const property (9 top-level) | deferred (nonblocking) | no spec authority covers these request-side unions; inferring an undeclared tag is a policy choice; documented in `docs/io/import-openapi.md` and the skill's known gaps |
| R1-F2 | Referenced oneOf schemas bypass response-shape classification | fixed | `json_shapes` resolves the media schema via `RefResolver::resolve_schema` before inspecting `oneOf`; all children verified |
| R1-F2.1 | Direct response, inline discriminator `oneOf` → two tagged shapes (control) | clean | `define/tests/response_openapi.rs` `a_direct_response_separates_an_inline_union` / `a_referenced_response_separates_an_inline_union` (JSON and YAML) |
| R1-F2.2 | Direct response, referenced discriminator `oneOf` → two tagged shapes | fixed | `define/tests/response_openapi.rs` `a_direct_response_separates_a_referenced_union` / `a_referenced_response_separates_a_referenced_union` (JSON and YAML) |
| R1-F2.3 | Referenced response, inline discriminator `oneOf` → two tagged shapes (control) | clean | `define/tests/response_openapi.rs` `a_direct_response_separates_an_inline_union` / `a_referenced_response_separates_an_inline_union` (JSON and YAML) |
| R1-F2.4 | Referenced response, referenced discriminator `oneOf` → two tagged shapes | fixed | `define/tests/response_openapi.rs` `a_direct_response_separates_a_referenced_union` / `a_referenced_response_separates_a_referenced_union` (JSON and YAML) |
| R1-F2.5 | Direct response, inline object/array `oneOf` → object+array shapes (control) | clean | `define/tests/response_openapi.rs` `a_direct_response_separates_an_inline_union` / `a_referenced_response_separates_an_inline_union` (JSON and YAML) |
| R1-F2.6 | Direct response, referenced object/array `oneOf` → object+array shapes | fixed | `define/tests/response_openapi.rs` `a_direct_response_separates_a_referenced_union` / `a_referenced_response_separates_a_referenced_union` (JSON and YAML) |
| R1-F2.7 | Referenced response, inline object/array `oneOf` (control) | clean | `define/tests/response_openapi.rs` `a_direct_response_separates_an_inline_union` / `a_referenced_response_separates_an_inline_union` (JSON and YAML) |
| R1-F2.8 | Referenced response, referenced object/array `oneOf` → object+array shapes | fixed | `define/tests/response_openapi.rs` `a_direct_response_separates_a_referenced_union` / `a_referenced_response_separates_a_referenced_union` (JSON and YAML) |
| R1-F2.9 | Error response projection through the same JSON reader keeps one shape per status | clean | `an_error_status_keeps_one_shape_for_a_referenced_or_inline_separable_union`; referenced error body keeps `Json(Named)` as before |
| R1-F2.10 | Generator success-type decision reaches the multi-shape branch for referenced separable successes | fixed | gen `openapi_import_test::a_referenced_separable_union_generates_a_multi_shape_success` |
| R1-F2.11 | OpenAPI export preserves the imported alternatives and tags | fixed | `a_separated_referenced_union_exports_its_alternatives_and_tags_and_imports_again` |
| R1-F2.12 | Sibling: unmapped `oneOf` members tagged by component name despite a single-value `enum` | fixed | `an_unmapped_member_is_tagged_with_its_enum_value_inline_and_referenced`, `a_member_with_an_optional_tag_leaves_the_union_inseparable` |
| R1-F2.13 | Sibling: separated shapes carried no `variant`, so generation hit `GeneratedNameCollision` (inline too) | fixed | cell macro asserts variant names; gen multi-shape test passes |
| R1-F2.14 | Consumer: OpenAI overlay kept the first JSON shape, changing `GetConversationItem` success to `Message` | fixed | `keep_conversation_item_union` in `openai_pagination.rs`; `definition_baseline::openai` failed before, passes after; `just generate` artifacts byte-identical |
| R1-F2.15 | Sibling: chained references, self-referential cycle, inseparable referenced union | fixed | `a_union_behind_chained_references_separates`, `a_self_referential_reference_or_union_keeps_its_named_schema`, `an_inseparable_referenced_union_keeps_its_named_schema_with_a_located_warning` |
| R1-F2.16 | Pre-existing: an inline array member imports as `Value` tagged `Array`, so re-export loses the array schema (same inline and referenced) | deferred (nonblocking) | falls under the review's allowed deferral "broader extension/import fidelity"; not a regression from this finding |

### Progress

- starting the work on 'R1-F1 OpenAI stream events decode as the wrong variants' at 20:05:41
        - defect class: an imported union with a declared (or const-derivable) discriminator property is emitted as a first-match untagged enum, so an alternative is accepted without its discriminator value matching
        - source located: `gen/src/model_gen.rs` `generate_union_enum` emits `#[serde(untagged)]` for every `oneOf`/`anyOf`; the importer drops `discriminator`, and single-value `type` enums (`x-stainless-const`) become plain `String`
        - delegated to a foreground rust-developer subagent (owns logging under this bullet)
        - sibling sweep (vendored OpenAI spec, before): 1,173 `oneOf`/`anyOf` sites, 103 carry a `discriminator` (none with `mapping`); 52 are top-level component unions emitted as Rust enums (37 `oneOf`, 15 `anyOf`, all `$ref` alternatives), the other 51 are nested (property/items positions, 48 `$ref`-only and 3 with inline alternatives) and import as `serde_json::Value`, which cannot select a wrong variant; Unfolded Circle `UCR-core-openapi.yaml` has 2 nested discriminators with `mapping` but no imported definition (its definitions are hand-written); only OpenAI is imported (`just import-openai`), and the AsyncAPI importer maps every schema to `JsonValue`
        - edge cases found among the 52: alternatives whose tag lives under `allOf` (e.g. `FunctionToolCallResource`), nested unions as alternatives (`BetaResponsesServerEvent` -> `BetaResponseStreamEvent`), multi-valued tags (`WebSearchTool`: `web_search`, `web_search_2025_08_26`), duplicate tag values across alternatives (`Item`, `RealtimeConversationItem`), and optional tags (`EasyInputMessage`, `ItemReferenceParam`)
        - implemented: `TypeAlias` gains an optional `discriminator: Option<UnionDiscriminator>` (`property`, per-alternative accepted `tags`), omitted from serde when `None` so existing bundles round-trip; the importer (`define/src/openapi/import/mappings/discriminator.rs`) resolves each alternative's tags as `mapping` keys, else the property's string `enum` (through `$ref`, `allOf` members, and nested unions), else the implicit component name when the property is undeclared or an unrestricted string; an alternative that declares the property optional, a non-string property, or an inline alternative without a value leaves the union untagged with a located warning
        - implemented: `generate_union_enum` keeps `Serialize`/`JsonSchema` with `#[serde(untagged)]` and, for a discriminated union, emits a `Deserialize` impl that reads the property first, decodes only the alternatives that accept the value (shared values tried in order), and errors on a missing, non-string, or unlisted value naming the union and property
        - re-imported OpenAI (`just import-openai`): 47 unions now dispatch on `type` (only the 47 `Deserialize` derives changed in `types.rs`); 2 stay untagged with warnings (`InputItem`, `BetaInputItem`: `EasyInputMessage` makes `type` optional); 3 discriminated components (`Item`, `BetaItem`, `ChunkingStrategyRequestParam`) carry `type: object` beside `oneOf` and already import as `serde_json::Value`, which cannot pick a wrong variant
        - tests added: 6 `schematic-define` lib tests (`openapi::import::mappings::discriminator::tests::*`: mapping keys, enum through `allOf`/nested unions/`$ref` enum, implicit component name, optional/non-string/nullable tag -> untagged + located warning, inline alternative without a value, no discriminator); 2 `schematic-gen` lib tests (emitted dispatch impl; no/misaligned discriminator stays untagged); 6 `schematic-gen` L1 tests in `tests/l1/discriminated_unions.rs`, which compiles the checked-in generator output `tests/fixtures/discriminated_union/models.rs` (drift-checked against the generator) and decodes through it: tag beats first structural match, shared tags tried in order, serialization unchanged, missing/non-string/unknown tag errors, tagged alternative failure is reported rather than falling back
        - guard added: `definitions/tests/openai_stream_events.rs` lists the 53 alternatives once (an exhaustive `match` breaks the build if the enum gains one), compares the list to the vendored spec's `BetaResponseStreamEvent` `anyOf` (read with `manifest_dir!().join("../specs/openai/openapi.yaml")`), and decodes each alternative with its spec-documented `type` and distinct member values; proof of fidelity: with the enum's old derived untagged `Deserialize` temporarily restored the guard reported 51 alternatives decoding wrongly (the review's count), restored afterwards (`cmp` clean)
        - discovered: a `Default` discriminated union carries an empty tag, so a defaulted value no longer deserializes (untagged accepted it); `Default` exists only so containing structs can derive it, and wire data always carries a tag, so this is documented rather than changed; the guard fills the four members that hold a nested union (`part`, `item`) with documented alternatives
        - generated-client check: `schema/tests/streaming_providers.rs` `openai_get_response_sse_ends_cleanly_at_connection_close` now streams audio delta, audio done, the review's `response.output_text.delta`, a `response.completed` carrying an output message, and an `error` event, and asserts each decodes as its own variant with payload (text identity/indexes/logprobs, completed `response.id`/`status`/output text, error `code`/`message`/`param`); its stale "untagged, first-match" comment was rewritten; `just test-schema` 65/65 passed
        - undiscriminated sibling (decided, not applied): 65 `oneOf`/`anyOf` sites without `discriminator` have members separable by a common string-enum property, 9 of them top-level enums; 5 of those 9 are named-member unions with a required, disjoint `type` (`TextResponseFormatConfiguration`, `BetaTextResponseFormatConfiguration`, `ChatCompletionRequestUserMessageContentPart`, `Filters`, `BetaFilters` with multi-valued `type`), 3 are Assistants event unions of inline members (`MessageStreamEvent`, `RunStreamEvent`, `RunStepStreamEvent`, keyed on `event`, all variants `serde_json::Value`), and `RealtimeAudioFormats` has an optional `type`; not applied because choosing a dispatch property the spec does not declare is an inference policy beyond this finding's class (a declared discriminator), it would change request-side public decoding outside the `GetResponse` stream boundary, and a union could qualify on more than one property; e.g. `{"type":"json_object"}` still decodes as `ResponseFormatText` today; recorded in `docs/io/import-openapi.md` and the `schematic-define` skill's known gaps
        - clean rows re-verified by reading the hand-written definitions: Anthropic `MessageStreamEvent`/`ContentBlockDelta` are `#[serde(tag = "type")]` (R1-F1.54); LM Studio `ChatStreamEvent` is `#[serde(tag = "type")]` (R1-F1.55); Ollama `GenerateStreamItem`/`ChatStreamItem`/`PullModelStreamItem`/`CreateModelStreamItem` are untagged with `Error(StreamErrorLine)` first, whose only member `error: String` is required and never present on a chunk/progress line, while chunks require `model`/`created_at`/`done` and progress requires `status` (R1-F1.56-59); Ollama `OpenAIChatStreamChunk`/`OpenAICompletionStreamChunk` and ElevenLabs `SpeechWithTimestampsResponse` are single structs, and ElevenLabs raw audio is `Success::bytes()` (R1-F1.60-61); none is imported, so the importer change does not touch them, and their schema-crate stream tests are among the 65 passing
        - docs/skills updated: `docs/io/import-openapi.md` (mapping row plus a new "Unions and discriminators" section with example, resolution order, flow diagram, untagged fallback warning, and the empty-tag `Default`), `docs/generation/generated-clients.md` and `.claude/skills/schematic-define/schematic/openapi.md` (mapping row), `.claude/skills/schematic-define/SKILL.md` (known gaps)
        - artifacts: `just import-openai` rewrote only `definitions/src/openai/types.rs` (47 derive lines replaced by dispatch impls); `just generate` left `schema/src`, `openapi/`, and `postman/` byte-identical (the enums keep `Serialize`/`JsonSchema` with `#[serde(untagged)]`)
- work completed for 'R1-F1 OpenAI stream events decode as the wrong variants' at 20:45:06
        - orchestrator check: inspected the generated `Deserialize for BetaResponseStreamEvent` dispatch and the changed-file set; `just generate` left `schema/src`, `openapi/`, `postman/` byte-identical; only `definitions/src/openai/types.rs` regenerated
        - documented departure: serializing a `Default` discriminated enum and reading it back now fails (empty tag); `Default` exists only so containing structs can derive it
- starting the work on 'R1-F2 Referenced oneOf schemas bypass response-shape classification' at 20:45:06
        - defect class: response-shape classification inspects only inline schema nodes, so a resolvable reference to a separable `oneOf` is treated as one opaque named shape
        - delegated to a foreground rust-developer subagent (owns logging under this bullet)
        - sweep: the vendored OpenAI spec has one JSON success whose media schema is a `$ref` to a `oneOf` (`GetConversationItem` 200 -> `ConversationItem`, 28 members, `discriminator: type` with no `mapping`, each member's tag is a single-valued `enum`); the 6 other referenced unions are `anyOf` SSE items and do not reach JSON classification
        - discovered (R1-F2.12): `separate_one_of` tags an unmapped member with its component name even when the member restricts the property with an `enum`; applied to `ConversationItem` that would tag `Message` as `type: "Message"` instead of `"message"` and break a reply that decodes today, so the response tags now come from the same resolution the R1-F1 union `Deserialize` uses (`resolve_discriminator`: mapping key, else enum, else component name), and a member without exactly one value (optional or non-string property, multi-valued enum) makes the `oneOf` inseparable
        - implemented: `json_shapes` resolves the media schema (`RefResolver::resolve_schema`, which follows chained references and returns an error on a cycle) before looking for `oneOf`; separable -> tagged shapes; inseparable referenced -> the original `Json(Named)` plus a located warning (R28 "plus a diagnostic"); inseparable inline -> `serde_json::Value` plus the warning as before; an unresolvable or cyclic reference keeps the named shape
        - decided (R1-F2.9): an error status whose media schema is a `$ref` keeps `Json(Named)` with no diagnostic, exactly as before; separating a named union there would only replace a type that already decodes every member with `serde_json::Value`; an inline separable `oneOf` on an error status keeps its existing one-shape warning
        - discovered (R1-F2.13): separated JSON shapes carried no `variant`, so the generator derived `Json` for every member and rejected the endpoint (`GeneratedNameCollision`); this already applied to every imported separable inline `oneOf` and would have failed `just generate` for `GetConversationItem`; separated shapes now take the member's component name (`Directory`, `File`), or `Array`/`Object` for an unnamed array/object member
        - tests added: `define/tests/response_openapi.rs` cells R1-F2.1-8 (4 macro-generated tests, each covering the discriminator and array/object endpoint of the exporter fixture, through `OpenApiImport::build()` from JSON and YAML), chained reference, inseparable referenced union (named + located warning), cyclic reference and self-referential union, enum-derived tags inline and referenced, optional tag, error status (referenced and inline), export round trip; `gen/tests/l1/openapi_import_test.rs` `a_referenced_separable_union_generates_a_multi_shape_success`
        - discovered (R1-F2.14): after `just import-openai` the OpenAI overlay's scoping (keep the first JSON shape) turned `GetConversationItem`'s success from `ConversationItem` into `Message`, a narrower public type that fails on every other item kind; `definition_baseline::openai` caught it; the overlay now restores `Json(Named("ConversationItem"))` for that endpoint (`keep_conversation_item_union`), its module docs and `docs/io/import-openapi.md` overlay paragraph say so; with that, `just generate` leaves `schema/src`, `openapi/`, and `postman/` byte-identical, so no generated public type changes; the imported builder in `definitions/src/openai/mod.rs` now declares the 28 tagged shapes (tags `message`, `function_call`, ... from the members' `enum`s), which the overlay replaces
        - observation (not repaired, pre-existing, same for inline and referenced): an inline array member (the exporter's form for `Vec<File>`) imports as `serde_json::Value` tagged `Array`, so re-exporting loses the array schema and a second import no longer separates it
        - docs/skills updated: `docs/io/import-openapi.md` (mapping row, referenced-union section with example, tag and variant rules, inseparable/error-status/cycle behavior, warning list, variant row of the round-trip table, overlay paragraph); `.claude/skills/schematic-define/schematic/openapi.md` (Responses)
        - verification: `just lint` clean; `schematic-define` filtered run 340/340; `schematic-gen --lib --test l1` responses/import/drift/overlay/baseline/discriminated filter 109/109; `schematic-definitions` 603/603; `just test-schema` not run (no `schema/src` change)
- work completed for 'R1-F2 Referenced oneOf schemas bypass response-shape classification' at 21:10:06
        - orchestrator check: `just import-openai` rewrote only `definitions/src/openai/mod.rs` (`GetConversationItem` builder now declares 28 tagged shapes); the overlay keeps the public `ConversationItem` success; `just generate` artifacts byte-identical
- starting phase gates at 21:10:06
        - `just test` (schematic): exit 0; 2684 passed / 15 skipped, 9 slow passed, 1 runtime compile-fail passed
        - `just lint` and `just lint-schema`: exit 0
        - `just test-schema`: 65/65 passed
        - `just check-tier-coverage schematic`: 0 stranded
        - `cargo check -p model-citizen --all-targets` (reverse dependency of `schematic-definitions`, also compiles sniff): clean
        - cross-OS check skipped: the repairs are serde/codegen logic with no path, process, or OS-specific code
- phase gates completed at 21:17:04

### Completion Report

- **outcome:** complete
- **elapsed:** 2026-10-22T20:04:07-07:00 → 21:17:04 (about 1 h 15 min)
- **parent findings:** total 2 = fixed 2 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
- **instance rows:** R1-F1.1–F1.53 fixed (F1.1–F1.2 were clean controls), R1-F1.54–F1.61 clean, R1-F1.62–F1.64 fixed; R1-F2.1/.3/.5/.7/.9 clean, R1-F2.2/.4/.6/.8/.10–.15 fixed
- **nonblocking deferrals (not required by the findings' authority):**
        - R1-F1.65: unions without a `discriminator` keyword (9 top-level OpenAI unions) are still first-match untagged; inferring an undeclared tag is a policy choice; documented in `docs/io/import-openapi.md` and the skill's known gaps; next action: an author decision on inferring tags from a required const property
        - R1-F2.16: an inline array union member imports as `Value` tagged `Array` and loses its array schema on re-export; falls under the allowed "broader extension/import fidelity" deferral
- **verified repairs:**
        - R1-F1: the importer carries a union's declared discriminator (`mapping` → single `enum` value → component name) into `TypeAlias`, and `model_gen` emits a tag-dispatching `Deserialize`; 47 OpenAI unions now dispatch by tag, and 2 stay untagged with a warning
        - R1-F2: `json_shapes` resolves the media schema (including chains, with cycle fallback) before separating a `oneOf`; tags come from the shared discriminator resolver; separated shapes get variant names; the OpenAI overlay keeps `GetConversationItem`'s public success type
- **changed files:**
        - R1-F1: `define/src/models/{aliases.rs,mod.rs}`, `define/src/lib.rs`, `define/src/openapi/import/mappings/{discriminator.rs,mod.rs,schema.rs}`, `define/src/registration.rs`, `define/tests/{composition.rs,definition_bundles.rs}`, `gen/src/{model_gen.rs,asyncapi_import.rs}`, `gen/tests/l1/{discriminated_unions.rs,main.rs}`, `gen/tests/fixtures/discriminated_union/models.rs`, `definitions/src/openai/types.rs` (regenerated), `definitions/tests/openai_stream_events.rs`, `schema/tests/streaming_providers.rs`, `docs/generation/generated-clients.md`
        - R1-F2: `define/src/openapi/import/mappings/classify.rs`, `define/src/openapi/import/resolver.rs`, `define/tests/response_openapi.rs`, gen `openapi_import_test`, `definitions/src/openai_pagination.rs`, `definitions/src/openai/mod.rs` (regenerated)
        - both: `docs/io/import-openapi.md`, `.claude/skills/schematic-define/schematic/openapi.md`, `.claude/skills/schematic-define/SKILL.md`
- **final verification:** `just test`, `just lint`, `just lint-schema`, `just test-schema`, `just check-tier-coverage schematic`, `cargo check -p model-citizen --all-targets`, all passing (results above); R1-F1 is shown by `openai_stream_events` and `streaming_providers`, and R1-F2 by `response_openapi`, `openapi_import_test`, and `definition_baseline::openai`

## Implementation of Review Findings #2

> **started at:** 2026-10-22T21:21:45-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-16-schematic-response-model-graphql/review-2.md'
- this is iteration 2 of the review-to-implement cycle

## Follow-up Pass for Review #2

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-16-schematic-response-model-graphql/review-2.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-16-schematic-response-model-graphql/spec.md'
- the review's `findings` list is empty, so there is nothing to repair in this pass; its two `observations` are carried to the backlog unchanged
- no source, test, or doc changes were made, so no focused tests, `just test`, `just lint`, or cross-OS checks were run for this pass (the last gates are the Review #1 phase gates above, all passing)

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| R2-O1 | Unions without a declared discriminator still use first-match decoding | backlog (observation) | inferring a tag from an undeclared common property is an import policy decision outside a declared-discriminator fix; already documented in `docs/io/import-openapi.md` ("Unions and discriminators"); carried as R1-F1.65 in the Review #1 ledger |
| R2-O2 | Inline array alternatives lose their schema on re-export | backlog (observation) | pre-existing import/export fidelity limit, identical for inline and referenced unions; falls under the accepted "broader import fidelity" deferral; carried as R1-F2.16 in the Review #1 ledger |

### Completion Report

- **outcome:** complete; nothing fixed (no findings), nothing deferred, 2 observations recorded
- **backlog entry:** `schematic/fixes/_unscheduled/schematic-response-model-graphql-follow-ups/spec.md`
- **review frontmatter:** `implemented: true`, `implemented_by: claude/opus`
