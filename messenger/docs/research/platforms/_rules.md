# Research Contract Semantic Rules

The schemas beside this file ([`_schema.yaml`](./_schema.yaml),
[`_types.yaml`](./_types.yaml), [`_overrides.schema.yaml`](./_overrides.schema.yaml),
[`../../platforms.schema.yaml`](../../platforms.schema.yaml), and
[`../implementation/_schema.yaml`](../implementation/_schema.yaml)) prove
structure only. Darkmatter SimplifiedSchema cannot express references,
uniqueness by key, conditional requirements, or cross-document checks, so the
rules below belong to Messenger's deterministic Rust pass.

Each rule has a planned owner in the feature-gated `messenger::research`
module (Phase 3 of the
[research metadata pipeline](../../../features/2026-09-17-research-metadata-pipeline/plan.md)).
The fixture corpus at `messenger/lib/tests/fixtures/research/negative/semantic/`
holds at least one schema-valid document per rule; its file stem starts with
the rule code (`sr-unique--duplicate-fact-id.md`).

| Rule | Check | Owner |
|---|---|---|
| SR-VERSION | Reject any `schema_version`/`roster_version` other than 1 during typed load, independently of the schema literal | `research::load` |
| SR-STRICT-SCALARS | Numbers are YAML numbers; versions, IDs, and operands are YAML strings. Darkmatter coerces both ways (`"2000"` → 2000, `9.50` → `"9.5"`) | `research::load` |
| SR-ROSTER | Exactly the active roster's platforms and interfaces appear, one document per platform; a sending interface maps one or more adapters; a research-only interface maps none; each of the seven adapters maps exactly once across the fleet; paused/excluded subjects never enter the fleet | `research::validate::identity` |
| SR-CURATED | Curated sources per platform ≤ `curated_source_cap`; unique URLs; `interfaces` belong to that platform; `approved` sources record `approved_by` and `approved_on` | `research::validate::identity` |
| SR-UNIQUE | Unique interface, fact, source, gap, change, and fixture IDs within a document; no duplicate scoped fact `(platform, interface, operation, surface, kind)` without distinguishing conditions | `research::validate::identity` |
| SR-REF | Every ID reference resolves to a record of the right type: evidence → sources; interface fields → interfaces; profiles, bindings, constraints, errors, envelopes, fixtures, gaps, relationship targets, companion interfaces, `recommended_by`, coverage-cell gaps, interaction-fixture bindings | `research::validate::identity` |
| SR-EVIDENCE | A known fact cites one or more sources with a `retrieved` date; a source has exactly one of `url` or `location`; secondary-only facts stay visible but never executable | `research::validate::identity` |
| SR-STATE-VALUE | `known` numeric bounds carry `value`; `unknown`/`conflicting`/`not_applicable` carry none; `unknown` and `conflicting` name a gap; `conflicting` keeps two or more claims; `not_applicable` explains why; a release date or period only when `release_date_state: known` | `research::validate::constraints` |
| SR-CONDITION | Each condition kind has exactly one operand form (`equals`, `min`/`max`, or `one_of`) from its bounded vocabulary; an operand-less condition carries `gap` and makes its record non-executable | `research::validate::constraints` |
| SR-APPLICABILITY | Simultaneously applicable bounds stay visible; two values for one scoped bound whose conditions can both hold fail instead of "last entry wins" | `research::validate::constraints` |
| SR-AGGREGATE | `aggregate_max` needs `members` and `aggregation_scope`; member surfaces are unit-compatible (text units for text fields, `items` for collections) | `research::validate::constraints` |
| SR-KIND-UNIT | `recommended_max` names `recommended_by` and no `enforced_by`; known `hard_max`/`hard_min` name `enforced_by`; `bytes` only for payload kinds; `items` only for count kinds | `research::validate::constraints` |
| SR-ENFORCEABLE | Executable eligibility (a projection, not a rejection) requires `known` state, a resolved unit (not `unspecified_characters`/`unknown`), a known stage, resolved conditions, non-secondary evidence, and a supported consumer mapping | `research::validate::constraints` |
| SR-COVERAGE | One coverage matrix per interface; `researched` requires at least one record in that category for the interface; `not_applicable` explains why; `gap` names a gap. An empty array never means "no constraints" | `research::validate::coverage` |
| SR-GAP | An `investigated` gap has `searches`, `inspected_sources`, `unresolved_reason`, and `blocked_decision`; initial-baseline coverage rests only on investigated gaps; gap `facts` resolve | `research::validate::coverage` |
| SR-CHANGE | `requires_messenger_update: true` needs `reason` and a `requires_messenger_update` gap; change `facts` resolve | `research::validate::coverage` |
| SR-FORMAT | Profiles referenced by text bindings resolve; a `known` construct names its `support`; the grammar family never implies construct support; `mutually_exclusive` is symmetric; `fallback_for` targets a primary binding; a selector default is one of its values | `research::validate::bindings` |
| SR-IMAGE | Bindings listed in a role cell have that role; caption/alt-text bindings resolve to text bindings; provider-derived or provider-selected placement never projects as explicit caller control; `min_items ≤ max_items` and agree with shared constraints | `research::validate::bindings` |
| SR-ATTRIBUTION | Only `sender_identity`/`delegated_author`/`forwarded_origin` evidence can set `changes_sender: yes`; a caller-supplied label never implies authenticated identity; an account- or application-scoped avatar cannot bind a per-message image slot without evidence | `research::validate::bindings` |
| SR-LOCATION | A `shared_place` never has subject `author`; `live_location` uses `mode: live`; exactly one author-geolocation answer per interface | `research::validate::bindings` |
| SR-EXPRESSION | `app_only` never carries API `support`; reactions and emoji/sticker content are not message effects; an `exact` intent mapping needs evidence for the effect's meaning | `research::validate::bindings` |
| SR-INTERACTIVITY | Callback-only or receive-only companions cannot claim general text; `answer_type` matches `kind` (confirmation → boolean plus `confirmation_mapping`); `min_selections ≤ max_selections`; `link_button` has `answer_type: none`; application-managed interpretation is never `native`; aggregate-only results have no responder locator; grouped message controls never submit as `single_event`; companion interfaces are declared research-only interfaces | `research::validate::interaction` |
| SR-ENVELOPE | Error `origin` matches its envelope; locators suit `body_format` (`body_text` only for plain text, `sdk_variant:` only for SDK errors); match predicates read only locators the envelope defines | `research::validate::errors` |
| SR-MATCH | A known error has `match`; an unknown error may carry only `candidate_match`, never executable; `discriminator_equals` needs `discriminator_locator` | `research::validate::errors` |
| SR-MATCH-OVERLAP | Within interface, operation, version, origin, and phase, two signatures may overlap only when one strictly adds predicates (the more specific wins) | `research::validate::errors` |
| SR-ORIGIN-PHASE | `before_submission` implies `not_submitted`; a `warning` outcome is never `rejected`; a transport timeout never claims `rejected`, and `retry_candidate` never implies `replay_safety: safe` without evidence | `research::validate::errors` |
| SR-FIXTURES | Replay every executable signature against its positive, negative, and near-miss fixtures; unknown codes and changed envelopes classify as unknown | `research::validate::errors` |
| SR-SANITIZED | Fixtures contain no credentials, recipient identifiers, message bodies, raw transcripts, or terminal control sequences; provider text stays bounded | `research::validate::errors` (corpus scan in tests) |
| SR-OVERRIDE | An override's `fact` exists (not orphaned), `review_by` has not passed (not expired), and `target_hash`/`schema_hash` match the current fact and schema (not stale) | `research::validate::coverage` |
| SR-MAPPING | Mapping facts resolve in the named platform document; the adapter belongs to that platform; fingerprints are recomputed and any mismatch forces `unassessed` with `stale_reason`; only `review.status: accepted` is an implementation claim | `research::assess` |
