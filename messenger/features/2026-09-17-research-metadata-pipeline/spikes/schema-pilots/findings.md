---
title: Schema Pilots — SimplifiedSchema fit for the research metadata contract
date: 2026-09-17
status: completed
---

# Schema Pilots: Findings

This spike covers the Phase 1 Wave 1 **Schema Pilots** task. It drafts a throwaway pilot schema (`schema_version: 0`) and four minimal platform records, then uses the real `md` CLI to see what Darkmatter SimplifiedSchema can enforce. Anything the schema cannot enforce is listed as a rule for the Rust semantic pass.

None of these files is research. Values come from the existing prose research, the Discord truncation fix's SDK analysis, and the three documentation spot-checks in the spec. Claims that the evidence does not establish carry `confidence: low` or `state: unknown` with a gap record. No network fetches were made.

> **Superseded by the [architecture record](../../architecture.md):** the pilot's dotted interface IDs (`discord.webhook`) and SHA-256 fingerprints are pilot conventions only. Production uses the architecture record's snake-case interface IDs and xxh64 (`biscuit-hash`) fingerprints, as the spec requires xxHash for non-Markdown content.

## Artifacts

| Path | Purpose |
|---|---|
| `_schema.yaml` | Document shape: root properties, required arrays, closed root |
| `_types.yaml` | Named reusable types imported with `Name@./_types.yaml` |
| `discord.md` | Individual, SDK, and aggregate bounds; a conflicting fact; a send-only webhook; a bot with research-only gateway and interactions companions; a question binding and a form binding; three error shapes |
| `telegram.md` | `parsed_text` caption versus `utf16_code_units` entities; a parse-mode selector that excludes `entities`; album; hosting-mode conditions; JSON error envelope and fixtures |
| `slack.md` | Blocks versus the top-level `text` fallback; `recommended_max` 4000 versus truncation at 40000; unknown errors with non-executable `candidate_match` (HTTP 200 `ok:false`, webhook plain text) |
| `signal.md` | signal-cli JSON-RPC; operand-less `bridge_version` conditions; unknown facts with two `investigated` gaps |
| `_mappings.schema.yaml`, `_mappings.types.yaml`, `mappings.pilot.yaml` | Implementation-assessment pilot |
| `negative/controls/` | Base documents that must pass |
| `negative/schema-rejects/` | Samples that `md` must reject |
| `negative/schema-accepts-rust-must-reject/` | Invalid samples that the schema accepts; each belongs to a Rust semantic rule |
| `negative/schema-limitations/` | Wrong values that `md` accepts because of a tooling behavior |
| `probes/` | Minimal grammar probes behind every limitation below |

## Validation commands and results

Binary: `target/debug/md` (`md 0.1.0`), built at 17:19 on 2026-09-17 from this worktree after the last Darkmatter commit (`49c55747b`). The installed `~/.cargo/bin/md` (built 2026-09-09) returns the same exit codes for the pilots and every sample in `schema-rejects/`. All commands ran from `messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/` with `NO_COLOR=1`. Messages below are from `--format json`.

    MD=../../../../../target/debug/md
    $MD schema validate discord.md telegram.md slack.md signal.md mappings.pilot.yaml   # exit 0
    $MD schema validate --format json negative/*/*                                     # per-file below

| File | Exit | Result |
|---|---|---|
| `discord.md`, `telegram.md`, `slack.md`, `signal.md`, `mappings.pilot.yaml` | 0 | valid |
| `controls/minimal-valid.md` | 0 | valid |
| `controls/composition-allowlist.md` (`prompt`, `hash` at the root) | 0 | valid |
| `schema-rejects/unknown-top-level-key.md` | 1 | `Additional properties are not allowed ('notes' was unexpected)` |
| `schema-rejects/unknown-nested-key.md` | 1 | `/constraints/0 Additional properties are not allowed ('limit' was unexpected)` |
| `schema-rejects/unknown-enum.md` | 1 | `/constraints/0/unit "characters" is not one of "utf8_bytes", "unicode_scalars" or 6 other candidates` |
| `schema-rejects/missing-required-array.md` | 1 | `"gaps" is a required property` |
| `schema-rejects/missing-knowledge.md` | 1 | `/constraints/0 "knowledge" is a required property` |
| `schema-rejects/wrong-type-value.md` | 1 | `/constraints/0/value "two-thousand" is not of type "integer"` |
| `schema-rejects/wrong-type-adapters-scalar.md` | 1 | `/interfaces/0/adapters "discord" is not of type "array"` |
| `schema-rejects/negative-value.md` | 1 | `/constraints/0/value -1 is less than the minimum of 0` |
| `schema-rejects/bad-locator.md` | 1 | `/envelopes/0/code_locator "error_code" does not match "^(/[^ ]*\|header:…)$"` |
| `schema-rejects/incomplete-role-matrix.md` | 1 | `/role_coverage/0/roles "link_preview" is a required property` |
| `schema-rejects/unsupported-schema-version.md` | 1 | `/schema_version 0 was expected` |
| `schema-rejects/mapping-missing-fingerprints.yaml` | 1 | `/assessments/0/fingerprints [] has less than 1 item` |
| `schema-accepts-rust-must-reject/*` (9 files) | 0 | valid by design; see Rust semantic rules |
| `schema-limitations/numeric-string-coerced.md` (`value: "2000"`) | 0 | valid: coerced to a number (L9) |
| `schema-limitations/version-float-coerced.md` (`latest_stable: 9.50`) | 0 | valid: coerced to `"9.5"`, dropping the trailing zero (L9) |
| `schema-limitations/mapping-unfenced.yaml` | 0 | `doesn't have a schema definition so is valid by default` (L10) |

## What SimplifiedSchema can express

| Requirement | Verdict | Evidence |
|---|---|---|
| Closed nested records | **Yes.** Every inline object compiles with `additionalProperties: false`, including named object types imported with `@`. | `unknown-nested-key.md`; `probes/p3_doc_bad.md` |
| Named reusable types | **Yes, from a separate file.** `Name@./_types.yaml`, `Name[](…)@file`, and `Name(required)@this` inside the types file all work. In the same file they also become document properties (L2). | the pilots; `probes/p3`, `probes/p9` |
| Required arrays | **Yes.** `Type[](required)@file` and `(min(1); required)`. An empty array still satisfies `required`, so category coverage is a Rust rule. | `missing-required-array.md` |
| Enums | **Yes.** Unknown members are rejected with path and line. | `unknown-enum.md` |
| Top-level composition allowlist | **Yes, via a pattern key.** `"<pattern::^(prompt\|hash)$>": any` at the root closes the root; `$schema` is stripped before validation. The docs' Limitations list says roots cannot be closed, so this relies on under-documented behavior (L1). | `unknown-top-level-key.md`, `controls/composition-allowlist.md`, `probes/p1`, `probes/p2` |
| Exact literals | **Yes.** `literal(0; required)` rejects other versions. | `unsupported-schema-version.md` |
| Complete coverage matrices | **Yes, for fixed key sets.** Making every canonical role (or formatting construct) a required key forces an explicit cell, and a cell may be `state: unknown`. | `incomplete-role-matrix.md` |
| Locator syntax | **Yes.** A single-quoted regex in `pattern('…')`. | `bad-locator.md`, `probes/p12` |
| Value present with `state: unknown` | **Only through a nested discriminated union, and the pilot does not adopt it.** See L3–L5. | `probes/p4*`, `probes/p5` |

## Limitations found (with probes)

**L1. Roots are open unless a pattern key is present.** Without a pattern key an unknown root key passes (`probes/p1_root_open.md`, exit 0). Adding a pattern key rejects it: `root Additional properties are not allowed ('stray' was unexpected)` (`probes/p2_root_pattern.md`, exit 1). This contradicts the documented limitation "No `additionalProperties: false` opt-in at the root". Ask the Darkmatter owners to document this as the supported root-closure mechanism, or to add `strict`.

**L2. A standalone schema file cannot separate named types from document properties.** Types defined in the `$schema:` envelope are also root properties, so `Item: { id: leaked }` validates in a document (`probes/p9.md`, exit 0). A tagged envelope cannot add `$schema:`: `tagged schema documents support only kind and types; found unsupported keys: $schema` (`probes/p10.md`, exit 2). The pilot therefore keeps types in `_types.yaml` and the root in `_schema.yaml`.

**L3. There are no arrays of unions.** `Cond[]@./p3_types.yaml`, where `Cond` is a discriminated union, fails to load: `cannot apply [] / constraints to the union-typed named type Cond@./p3_types.yaml` (`probes/p3_union_array.md`, exit 2). Condition records, fact records keyed by `state`, and matching signatures therefore stay flat (`kind` plus optional operands), and operand validity per kind is a Rust rule.

**L4. A union-typed property cannot be required.** `Knowledge(required)@this` on a union fails with the same error. `(required)` on an arm is silently ignored for a nested property, so a missing `knowledge` passes (`probes/p5_missing.md`, exit 0; also `probes/p4_bad_missing_knowledge.md`).

**L5. Discriminant narrowing applies only at the top level.** A top-level union with `state: unknown` plus `value` reports the real cause, `Additional properties are not allowed ('value' was unexpected)` (`probes/p7.md`). Inside an array item the same union reports only the other arms' literals, `"known" was expected; "not_applicable" was expected`, and never names `value` (`probes/p4_bad_value_with_unknown.md`). Together with L3 and L4, this is why the pilot uses a flat `Knowledge` block and leaves state/value consistency to Rust.

**L6. A multi-line quoted inline object fails in a standalone schema file.** It loads from inline frontmatter (`probes/p14_multiline_inline.md`, exit 0), but both named imports and whole-file references fail: `invalid SimplifiedSchema for property <source>: could not project SimplifiedSchema expression spans through YAML source` (`probes/p14_multiline_import.md`, `p14_multiline_whole.md`, exit 2). A single-quoted multi-line form also fails (`p15_single_quoted.md`), while one line works (`p15_one_line.md`). The installed binary reproduces the error. Every type in `_types.yaml` is therefore one long line, which is hard to review. This looks like a Darkmatter bug in the source-span projection.

**L7. Regex groups need quoting.** `pattern(^(/a|b)$)` fails with `expected , or ) in argument list, found (` (`probes/p12_pattern_parens.md`, exit 2). `pattern('^(…)$')` works.

**L8. Descriptions inside `{ … }` stop at the first comma.** Field notes therefore live in YAML comments, which do not appear in diagnostics.

**L9. Type coercion is always on.** `value: "2000"` becomes a number and passes. `latest_stable: 9.50` becomes the string `"9.5"`, silently dropping the trailing zero from a version (`negative/schema-limitations/`). There is no strict-types flag, so Rust must reject non-string version scalars and string-typed numbers.

**L10. `md schema validate` reads only frontmatter.** A bare YAML data file with `$schema:` is reported `valid by default` (exit 0) even when its data is invalid (`schema-limitations/mapping-unfenced.yaml`, `probes/p11_bare.yaml`). With `--schema`, the file's data is ignored and only the baseline's requirements are reported (`probes/p11_bare_valid_data.yaml`: `a` is present but reported missing). Wrapping the file in `---` fences works (`probes/p11_fenced.yaml`, exit 1 as expected). `mappings.pilot.yaml` is fenced for this reason.

**L11. Pretty diagnostics drop nested property names.** Pretty output prints `role_coverage/0/roles is a required property`, while JSON names the missing property (`link_preview`). Tooling should consume `--format json`.

**L12. Uniqueness and references are out of reach.** `unique` compares whole items only. The schema has no key uniqueness, no ID references, no cross-document checks, and no conditional requirements such as "if `state` is known, `value` is required".

**L13. `date` cannot hold a partial date.** Telegram's "March 2026" cannot be stored, so it becomes `release_date_state: unknown` plus an explanation.

Authoring note: PyYAML rejects a plain scalar containing `?` inside a flow mapping, while `md` (serde) accepts it. The pilots quote free-text `question` values so both parsers read them.

## Rust semantic rules

These rules belong in the deterministic Rust pass. Samples in `negative/schema-accepts-rust-must-reject/` pass `md` today, and each one names its rule.

| Rule | Check | Why the schema cannot | Sample |
|---|---|---|---|
| SR-VERSION | Reject unsupported `schema_version` during typed load, not only through the literal | Rust types must refuse the version independently | — |
| SR-STRICT-SCALARS | Numbers must be YAML numbers; versions and IDs must be YAML strings | Coercion is always on (L9) | `schema-limitations/*coerced.md` |
| SR-ROSTER | Exact active-roster platforms and interfaces; `sending_adapter` has one or more adapters; `research_only` has none; each adapter maps once across the fleet | Cross-document and conditional | `research-only-with-adapter.md` |
| SR-UNIQUE | Unique `interface_id`, fact IDs, source IDs, and gap IDs; no duplicate scoped facts `(platform, interface, operation, constraint)` | `unique` compares whole items only | `duplicate-fact-id.md` |
| SR-REF | Every `Id` reference resolves to the right record type: evidence to sources, interface, profiles, bindings, constraints, errors, envelopes, fixtures, gaps, relationship targets, companion interfaces, `recommended_by` | No reference type | `dangling-evidence.md` |
| SR-STATE-VALUE | `known` numeric bounds require `value`; `unknown`, `conflicting`, and `not_applicable` forbid it; `conflicting` needs at least two `claims`; `unknown` needs a gap; `not_applicable` needs an explanation; `release_date` if and only if `release_date_state: known` | No conditional requirements; unions are unusable (L3–L5) | `value-with-unknown-state.md`, `known-without-value.md` |
| SR-EVIDENCE | A known fact needs one or more resolved sources with a `retrieved` date; secondary-only facts are never executable; a source needs exactly one of `url` or `location` | Cross-record | `signal.md` `src.signal_cli.jsonrpc_man` has no date |
| SR-CONDITION | Each condition `kind` has exactly the right operands (`equals`, `min`/`max`, or `one_of`) and a vocabulary of values. An operand-less condition is unresolved: it must carry a gap and makes the record non-executable | Flat conditions (L3) | `condition-missing-operand.md`; `signal.md` |
| SR-APPLICABILITY | Simultaneously applicable bounds for one scope stay visible; overlapping applicability without distinguishing conditions fails | Cross-record | — |
| SR-AGGREGATE | `aggregate_max` requires `members` and `aggregation_scope`; members' surfaces must be text-compatible with the unit; `items` only for count kinds; `bytes` only for payload kinds | Cross-field | `aggregate-bad-members.md` |
| SR-KIND-UNIT | `recommended_max` carries `recommended_by` and no `enforced_by`; `hard_max` needs `enforced_by` when known | Cross-field | — |
| SR-ENFORCEABLE | Enforcement eligibility requires known state, a resolved unit (not `unspecified_characters` or `unknown`), a known stage, resolved conditions, non-secondary evidence, and a supported consumer mapping | Derived projection | `c.discord.bot.embed_count` (secondary only) |
| SR-COVERAGE | Required surface and category coverage per interface; an empty array counts as coverage only with a known-absence record or gap; one role matrix per interface | Empty arrays satisfy `required` | — |
| SR-FORMAT | Text-binding profiles resolve; mutual relationships are symmetric (`mutually_exclusive` both ways); `fallback_for` targets a primary binding; the selector default is in `values` or explicitly absent | Cross-record | `telegram.md` relationships |
| SR-IMAGE | Bindings listed in a role cell have that role; `caption_binding` and `alt_text_binding` resolve to text bindings; `provider_selected` placement never projects as caller control; `max_items` agrees with shared constraints | Cross-record | `discord.md` `alt_text_binding` points at a binding that does not exist yet |
| SR-ENVELOPE | Error `origin` matches the envelope origin; locators are valid for `body_format` (`body_text` only for `plain_text`, `sdk_variant:` only for `sdk_error`); match predicates use only locators the envelope defines | Cross-record | — |
| SR-MATCH | A known error needs `match`; an unknown error may carry only `candidate_match`, which is never executable; each signature has one or more predicates; `discriminator_equals` needs `discriminator_locator` | Conditional | `slack.md` |
| SR-MATCH-OVERLAP | Within interface, operation, version, origin, and phase, two signatures must be equal only if one strictly adds predicates; otherwise fail | Needs set logic | `overlapping-signatures.md` |
| SR-ORIGIN-PHASE | `before_submission` implies `delivery_certainty: not_submitted`; a `warning` outcome cannot be `rejected`; `retry_candidate` never implies `replay_safety: safe` without evidence | Cross-field | `sdk-phase-certainty-mismatch.md` |
| SR-FIXTURES | Replay each executable signature against its positive and negative fixtures; unknown codes classify as unknown | Execution | `discord.md`, `telegram.md` fixtures |
| SR-INTERACTIVITY | A callback-only interface cannot claim inbound text; `answer_type` matches `kind` (confirmation to boolean plus `confirmation_mapping`; multiple choice to an option-ID array); `min_selections` ≤ `max_selections`; companion interfaces are `research_only` with a matching relationship; grouped controls never imply `single_event` form submission | Cross-record | `discord.md` |
| SR-GAP | An `investigated` gap requires searches, inspected sources, unresolved reason, blocked decision, and next investigation; gap `facts` resolve; every referenced gap exists | Conditional | `signal.md` |
| SR-CHANGE | `requires_messenger_update: true` requires `reason`; change `facts` resolve | Conditional | — |
| SR-MAPPING | Mapping `facts` resolve in `platform_doc`; fingerprints are recomputed; any mismatch forces `unassessed` with `stale_reason`; only `review.status: accepted` can be reported as an implementation claim | Cross-file and computed | `mappings.pilot.yaml` `map.telegram.text_binding` |

## Implementation-assessment placement

**Recommendation: keep reviewed mappings in a separate file, not in the platform-document frontmatter.** The pilot's `mappings.pilot.yaml` follows this layout.

- **Different owners and review cadence.** Research agents write the platform documents from external evidence. Mappings describe Messenger code and need human acceptance. The spec keeps these reviews separate, and a mapping edit should never touch an evidence document or its change history.
- **Different invalidation.** Mappings change when code changes. Keeping them in frontmatter would make every adapter refactor look like a research-document change to the delta phase and the automatic-renewal check.
- **Fingerprints must cover both sides.** Each assessment hashes the code files it inspected (`sha256-file`) and the fact records it assessed (`sha256-canonical-json-record`: SHA-256 of the record's key-sorted compact JSON). A change to either side makes the assessment `unassessed`. The pilot shows this with `map.telegram.text_binding`, whose recorded `telegram.rs` digest comes from `e178f0263` and does not match the checkout.
- **Granularity.** File-level digests are conservative: an unrelated edit in the same file invalidates the assessment, while an unrelated commit elsewhere does not. Symbol-scoped digests, for example from tree-hugger, would narrow this but would need a portable canonical extraction.
- **Tooling.** Because of L10, the mappings file needs `---` fences to be validated by `md`. It could also be a `.md` file.

## Vocabulary

The pilot uses the spec's vocabularies as written. It needed the following additions, which are gaps for review:

| Area | Gap found | Pilot handling |
|---|---|---|
| `unit` | File and request sizes are not `utf8_bytes`. An unknown bound also has no known unit. | Added `bytes` and `unknown` |
| constraint `kind` | Lower bounds: Telegram text is "1–4096"; media groups have a minimum. The spec asks for lower bounds but defines no kind. | Recorded in `explanation`; proposed `hard_min` / `item_count_min` |
| `surface` | Needs rich-object names that are not specific to Discord embeds (`rich_title`, `rich_description`, `rich_field_*`, `rich_footer_text`, `rich_author_name`), collection surfaces (`rich_objects`, `rich_fields`, `blocks`, `media_group`, `attachments`), and interactive surfaces (`callback_data`, `option_label`, `option_value`) | Added |
| condition `kind` | No kind for permission grants or intents (Discord `MESSAGE_CONTENT`, Telegram privacy mode) or for release age (signal-cli "older than about three months breaks") | Stored as `prerequisites` strings or gaps; proposed `permission_grant`, `release_age` |
| Knowledge granularity | A fact can have a known value and a conflicting or unknown dimension (Discord embed overflow: prose says silent failure, twilight rejects). One `state` per record cannot say this. | Split into a separate `conflicting` record; proposed per-dimension states or dimension-tagged `claims` |
| Image submission | One slot accepts several mechanisms (Telegram `photo`: multipart upload, URL, or `file_id`) | `submission` became the array `submissions` |
| Format family | Telegram HTML is a subset of HTML | Used `html` with a dialect; consider `html_subset` |
| Errors | Unknown errors need a place for a drafted, non-executable signature. Replay safety is separate from recovery. Fixtures can come from repository evidence without being captured. | Added `candidate_match`, `replay_safety`, and `provenance: observed_in_repo` |
| Text bindings | The same native field can serve two roles (Slack `text` as primary content or as a notification fallback) | Added `content_role` |
| Interfaces | Send-only, receive-only, and callback-only interfaces need an explicit direction | Added `direction` |
| Inbound | Whether the send identity can be reused | Added `reuses_send_identity` |
| Remediation | No `unknown` value | The field is omitted when unknown; consider adding `unknown` |
| Chronology | Month-only release dates | `unknown` plus an explanation; consider a precision field |
| Constraint scope | Discord bot and webhook share service bounds, and per-interface records duplicate them | Duplicated for the webhook `content` bound only; open question |
| Attribution, location, expression | Not piloted as dedicated binding records | Only a generic `capabilities` record (`category`, `capability`, `support`) was used |

## Coverage checklist

| Area | Covered by |
|---|---|
| Aggregate limits | `discord.md` `c.discord.bot.embeds_total`: `aggregate_max` 6000 with six repeated members and `aggregation_scope: message`. `SR-AGGREGATE` with `aggregate-bad-members.md`. |
| Individual vs SDK bounds and units | `discord.md` `content.service_max` (`unspecified_characters`) vs `content.sdk_max` (`unicode_scalars`, `enforced_by: sdk`, `applies_when` twilight 0.17) |
| Measurement stages | `telegram.md` caption `parsed_text`; entity binding `entity_offset_unit: utf16_code_units`; text `measurement_stage: unknown` |
| Recommendation vs threshold | `slack.md` `recommended_max` 4000 (`recommended_by`) vs `hard_max` 40000 (`overflow_behavior: truncate`) |
| Byte and count limits | `telegram.md` `payload_bytes_max` (unknown, hosting-mode conditioned); `item_count_max` for media groups and embeds |
| Explicit uncertainty | `unknown` with gaps throughout; `conflicting` with claims in `discord.md`; two investigated gaps in `signal.md` plus one in `discord.md`; `release_date_state: unknown` in all four documents; `versioning: unresearched` in `slack.md` and `signal.md` |
| Structured surfaces | `slack.md` `tb.slack.blocks` (`structured_blocks`) with `tb.slack.text.fallback` (`fallback_for`, `notification_fallback`) |
| Parse modes, formats, and text bindings | `telegram.md` `parse_mode` selector, `mutually_exclusive` with `entities`, three profiles; required construct coverage via `Constructs` |
| Images | `discord.md` and `telegram.md` bindings (multipart, album, provider-derived preview); four complete role matrices; `incomplete-role-matrix.md` |
| Executable error signatures | Structured service error: `telegram.md` 429 and 403 (`http_status` + `native_code_number`, `/parameters/retry_after`). Webhook response: `discord.md` 400 field-error body (observed) and `slack.md` plain-text `exact_text_token` (candidate only). Local SDK failure: `discord.md` `sdk_error_variant`, `before_submission`, `not_submitted`. Fixtures include a near miss and an unknown code. |
| Version applicability | `api_versions` and `chronology` in all four documents; SDK chronology (twilight 0.17) kept separate from provider API v10; `sdk_version` and `bridge_version` conditions |
| Interactivity | Send-only: `discord.webhook` and `slack.incoming_webhook` with `mechanism: none`. Companion receivers: `discord.gateway`, `discord.interactions`, `slack.events_api` with relationships. Same-interface receive: Telegram long-poll or webhook. Question bindings: Discord buttons (confirmation), Telegram inline keyboard (single choice) and poll (approximate). Form bindings: Discord modal (`single_event`) vs Telegram sequential prompts. |
| Implementation fingerprints | `mappings.pilot.yaml`: `partial`, `missing`, and `unassessed` (stale fingerprint), with code and test references, revision, and code and fact digests. `mapping-missing-fingerprints.yaml` is rejected. |
| Composition allowlist | `controls/composition-allowlist.md` passes; `unknown-top-level-key.md` is rejected |

## Open questions

1. Is root closure through a pattern key (L1) a supported contract? If not, Darkmatter needs a `strict` root option before the contract freezes.
2. Should Darkmatter fix multi-line inline objects in standalone files (L6) before `_types.yaml` becomes the reviewed schema? One-line types are hard to review.
3. Keep two schema files (`_schema.yaml` and `_types.yaml`), or accept one file whose named types leak into the root namespace and have Rust reject them? The spec names only `_schema.yaml`.
4. Should knowledge states be per dimension (value, unit, stage, overflow, applicability), or should partially known facts be split into separate records as the pilot does?
5. Should a constraint scope a list of interfaces when bot and webhook share a service bound, or should the duplication stay explicit?
6. Should `hard_min` and `item_count_min` join the constraint kinds, and should `bytes` and `unknown` join the units?
7. Should attribution, location, and expression get dedicated binding records before the vocabulary freezes? Only a generic `capabilities` shape was piloted.
8. Should mapping fingerprints use file-level or symbol-level scope, and should the canonical-record digest be defined by the Rust loader rather than this pilot's Python?
