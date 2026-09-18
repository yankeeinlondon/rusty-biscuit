# Research Contract Semantic Rules

The schemas beside this file ([`_schema.yaml`](./_schema.yaml),
[`_types.yaml`](./_types.yaml), [`_overrides.schema.yaml`](./_overrides.schema.yaml),
[`../../platforms.schema.yaml`](../../platforms.schema.yaml), and
[`../implementation/_schema.yaml`](../implementation/_schema.yaml)) prove
structure only. Darkmatter SimplifiedSchema cannot express references,
uniqueness by key, conditional requirements, or cross-document checks, so the
rules below belong to Messenger's deterministic Rust pass.

Each rule is implemented by its owner in the feature-gated
`messenger::research` module (`research` Cargo feature; see the
[research metadata pipeline](../../../features/2026-09-17-research-metadata-pipeline/plan.md)).
Findings carry the rule code, a repository-relative path, a JSON Pointer, and
the stable ID concerned. `SCHEMA` marks a SimplifiedSchema problem reported
through Darkmatter's library.

Documents are validated in one of two scopes. **Fragment** (fixtures, pilots,
candidates) applies every rule to the records present. **Accepted** (accepted
research and initial baselines) also requires every active roster interface,
coverage and unknown facts resting only on investigated gaps, an image-role
matrix for researched images, and an author-geolocation answer for
researched location; for the roster it requires all five platforms and all
seven adapters.
The fixture corpus at `messenger/lib/tests/fixtures/research/negative/semantic/`
holds at least one schema-valid document per rule; its file stem starts with
the rule code (`sr-unique--duplicate-fact-id.md`).

| Rule | Check | Owner |
|---|---|---|
| SR-SCHEMA-BINDING | A file's `$schema` resolves (through `FileReference`) to the shipped schema for its kind; otherwise schema validation is skipped, because another schema proves nothing | `research::load` |
| SR-TOP-LEVEL | Unknown top-level keys are rejected outside the composition allowlist (`prompt`, `hash`; `$schema` is consumed by binding), independently of the schema's root closure | `research::load` |
| SR-VERSION | Reject any `schema_version`/`roster_version` other than 1 during typed load, independently of the schema literal | `research::load` |
| SR-STRICT-SCALARS | Numbers are YAML numbers; versions, IDs, and operands are YAML strings. Darkmatter coerces both ways (`"2000"` → 2000, `9.50` → `"9.5"`) | `research::load` |
| SR-ROSTER | Exactly the active roster's platforms and interfaces appear, one document per platform; a sending interface maps one or more adapters; a research-only interface maps none; each of the seven adapters maps exactly once across the fleet; paused/excluded subjects never enter the fleet | `research::validate::identity` |
| SR-CURATED | Curated sources per platform ≤ `curated_source_cap`; unique URLs; `interfaces` belong to that platform; `approved` sources record `approved_by` and `approved_on` | `research::validate::identity` |
| SR-UNIQUE | Unique interface, fact, source, gap, change, and fixture IDs within a document; no duplicate scoped fact `(platform, interface, operation, surface, kind, unit, measurement_stage)` with identical conditions and knowledge state (different units or stages are simultaneous bounds, not duplicates) | `research::validate::identity` |
| SR-REF | Every ID reference resolves to a record of the right type: evidence → sources; interface fields → interfaces; profiles, bindings, constraints, errors, envelopes, fixtures, gaps, relationship targets, companion interfaces, `recommended_by`, coverage-cell gaps, interaction-fixture bindings | `research::validate::identity` |
| SR-EVIDENCE | A known fact cites one or more sources with a `retrieved` date; a source has exactly one of `url` or `location`; secondary-only facts stay visible and valid but never executable (an eligibility reason, not a rejection) | `research::validate::identity` |
| SR-STATE-VALUE | `known` numeric bounds carry `value`; `unknown`/`conflicting`/`not_applicable` carry none; `unknown` and `conflicting` name a gap; `conflicting` keeps two or more claims; `not_applicable` explains why; a release date or period only when `release_date_state: known` | `research::validate::constraints` |
| SR-CONDITION | Each condition kind has exactly one operand form (`equals`, `min`/`max`, or `one_of`) from its bounded vocabulary; an operand-less condition carries `gap` and makes its record non-executable | `research::validate::constraints` |
| SR-APPLICABILITY | Simultaneously applicable bounds stay visible; two values for one scoped bound whose conditions can both hold fail instead of "last entry wins" | `research::validate::constraints` |
| SR-AGGREGATE | `aggregate_max` needs `members` and `aggregation_scope`; member surfaces are unit-compatible (text units for text fields, `items` for collections) | `research::validate::constraints` |
| SR-KIND-UNIT | `recommended_max` names `recommended_by` and no `enforced_by`; known `hard_max`/`hard_min` name `enforced_by`; `bytes` only for payload kinds; `items` only for count kinds | `research::validate::constraints` |
| SR-ENFORCEABLE | Executable eligibility (a projection, not a rejection) requires `known` state, a resolved unit (not `unspecified_characters`/`unknown`), a known stage, resolved conditions, non-secondary evidence, and a supported consumer mapping (a `recommended_max` has none: it is advisory) | `research::validate::constraints` |
| SR-COVERAGE | One coverage matrix per interface; `researched` requires at least one record in that category for the interface (interactivity also counts records of related companion interfaces; formatting counts the document's profiles); `not_applicable` explains why; `gap` names a gap. An empty array never means "no constraints" | `research::validate::coverage` |
| SR-GAP | An `investigated` gap has `searches`, `inspected_sources`, `unresolved_reason`, and `blocked_decision`; in Accepted scope, coverage cells, role cells, and unknown or conflicting facts rest only on investigated gaps; gap `facts` resolve | `research::validate::coverage` |
| SR-CHANGE | `requires_messenger_update: true` needs `reason` and a `requires_messenger_update` gap; change `facts` resolve to facts or gaps, except that a `removed` change names an ID that is no longer present | `research::validate::coverage` |
| SR-FORMAT | Profiles referenced by text bindings resolve; a `known` construct names its `support`; the grammar family never implies construct support (a construct that is not `known` carries no support); `mutually_exclusive` is symmetric; `fallback_for` targets a primary binding; a selector default is one of its values | `research::validate::bindings` |
| SR-IMAGE | Bindings listed in a role cell have that role; caption/alt-text bindings resolve to text bindings; provider-derived or provider-selected placement never projects as explicit caller control; `min_items ≤ max_items` and agree with shared constraints | `research::validate::bindings` |
| SR-ATTRIBUTION | Only `sender_identity`/`delegated_author`/`forwarded_origin` evidence can set `changes_sender: yes`; a caller-supplied label never implies authenticated identity; an account- or application-scoped avatar cannot bind a per-message image slot without evidence | `research::validate::bindings` |
| SR-LOCATION | A `shared_place` never has subject `author`; `live_location` uses `mode: live`; exactly one author-geolocation answer per interface | `research::validate::bindings` |
| SR-EXPRESSION | `app_only` never carries API `support`; reactions and emoji/sticker content are not message effects; an `exact` intent mapping needs evidence for the effect's meaning | `research::validate::bindings` |
| SR-INTERACTIVITY | Callback-only or receive-only companions cannot claim general text; `answer_type` matches `kind` (confirmation → boolean plus `confirmation_mapping`); `min_selections ≤ max_selections`; `link_button` has `answer_type: none`; application-managed interpretation is never `native`; aggregate-only results have no responder locator; grouped message controls never submit as `single_event`; companion interfaces are declared research-only interfaces | `research::validate::interaction` |
| SR-ENVELOPE | Error `origin` matches its envelope; locators suit `body_format` (`body_text` only for plain text, `sdk_variant:` only for SDK errors); match predicates read only locators the envelope defines (or JSON Pointers beneath one; `/` is authored as the root) | `research::validate::errors` |
| SR-MATCH | A known error has `match`; an unknown error may carry only `candidate_match`, never executable; `discriminator_equals` needs `discriminator_locator` | `research::validate::errors` |
| SR-MATCH-OVERLAP | Within interface, operation, version, origin, and phase, two signatures may overlap only when one strictly adds predicates (the more specific wins) | `research::validate::errors` |
| SR-ORIGIN-PHASE | `before_submission` implies `not_submitted`; a `warning` outcome is never `rejected`; a transport timeout never claims `rejected`, and `retry_candidate` never implies `replay_safety: safe` without evidence | `research::validate::errors` |
| SR-FIXTURES | Replay every executable signature against its positive, negative, and near-miss fixtures; unknown codes and changed envelopes classify as unknown | `research::validate::errors` |
| SR-SANITIZED | Fixtures contain no credentials, recipient identifiers, message bodies, raw transcripts, or terminal control sequences; provider text stays bounded | `research::validate::errors` (corpus scan in tests) |
| SR-OVERRIDE | An override's `fact` exists (not orphaned), `review_by` has not passed (not expired), and `target_hash`/`schema_hash` match the current fact and schema (not stale) | `research::validate::coverage` |
| SR-MAPPING | Mapping facts resolve in the named platform document; the adapter belongs to that platform; only `review.status: accepted` is an implementation claim (anything else has `status: unassessed` with a `stale_reason`); every code and test reference has a matching fingerprint and a portable repository-relative path. Fingerprints are recomputed by `research::assess::evaluate`: any mismatch makes the assessment `unassessed` and raises a `requires_messenger_update` gap (an outcome, not a finding) | `research::assess` |
| SR-REVIEW | A published review record under `docs/research/reviews/` parses as `messenger-research-review/1`, is named `{approval date}-{platform}-{run_id}.json`, and records an active roster platform; the CHANGELOG is regenerated from these records only | `research::generate` |
