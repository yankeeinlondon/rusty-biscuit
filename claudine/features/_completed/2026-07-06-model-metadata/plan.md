---
agent: open_code/zai-coding-plan/glm-5.2
phases: 5
created: 2026-07-06
start_phase: 1
yolo: "true"
spec: claudine/features/2026-07-06-model-metadata/spec.md
source_files_during_phase_1:
  - unchained-ai/gen/src/main.rs
  - unchained-ai/gen/src/metadata_generator.rs
  - unchained-ai/gen/src/models_dev.rs
  - unchained-ai/gen/src/provider_metadata/openrouter.rs
  - unchained-ai/lib/src/models/model_capability.rs
  - unchained-ai/lib/src/models/model_metadata.rs
  - unchained-ai/lib/src/rigging/providers/models/metadata_generated.rs
docs_updated_during_phase_1:
  - claudine/features/2026-07-06-model-metadata/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - unchained-ai/gen/Cargo.toml
  - unchained-ai/gen/scripts/regen_metadata.py
  - unchained-ai/gen/src/main.rs
  - unchained-ai/gen/src/metadata_generator.rs
  - unchained-ai/gen/src/models_dev.rs
  - unchained-ai/gen/src/parsera.rs
  - unchained-ai/gen/src/provider_metadata/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
docs_updated_during_phase_2:
  - claudine/features/2026-07-06-model-metadata/plan.md
  - unchained-ai/README.md
  - unchained-ai/lib/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/gen/src/artifact.rs
  - claudine/lib/src/model_catalog/service.rs
  - unchained-ai/gen/src/catalog.rs
  - unchained-ai/gen/tests/catalog_drift.rs
docs_updated_during_phase_3:
  - claudine/features/2026-07-06-model-metadata/plan.md
  - unchained-ai/artifacts/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - unchained-ai/lib/src/rigging/providers/models/anthropic.rs
  - unchained-ai/lib/src/rigging/providers/models/deepseek.rs
  - unchained-ai/lib/src/rigging/providers/models/gemini.rs
  - unchained-ai/lib/src/rigging/providers/models/groq.rs
  - unchained-ai/lib/src/rigging/providers/models/metadata_generated.rs
  - unchained-ai/lib/src/rigging/providers/models/mistral.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/openai.rs
  - unchained-ai/lib/src/rigging/providers/models/openrouter.rs
  - unchained-ai/lib/src/rigging/providers/models/xai.rs
  - unchained-ai/lib/src/rigging/providers/models/zai.rs
  - unchained-ai/lib/src/rigging/providers/models/zenmux.rs
docs_updated_during_phase_4:
  - claudine/features/2026-07-06-model-metadata/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/features/2026-07-06-model-metadata/plan.md
  - claudine/features/2026-07-06-model-metadata/spec.md
  - unchained-ai/docs/topics/models-and-metadata.md
  - unchained-ai/gen/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/unchained-ai/SKILL.md
  - .claude/skills/unchained-ai/model-generator.md
  - .claude/skills/unchained-ai/providers-and-models.md
source_code:
  - claudine/gen/src/artifact.rs
  - claudine/lib/src/model_catalog/service.rs
  - unchained-ai/gen/Cargo.toml
  - unchained-ai/gen/src/catalog.rs
  - unchained-ai/gen/src/main.rs
  - unchained-ai/gen/src/metadata_generator.rs
  - unchained-ai/gen/src/models_dev.rs
  - unchained-ai/gen/src/provider_metadata/mod.rs
  - unchained-ai/gen/src/provider_metadata/openrouter.rs
  - unchained-ai/gen/tests/catalog_drift.rs
  - unchained-ai/lib/src/models/model_capability.rs
  - unchained-ai/lib/src/models/model_metadata.rs
  - unchained-ai/lib/src/rigging/providers/models/anthropic.rs
  - unchained-ai/lib/src/rigging/providers/models/deepseek.rs
  - unchained-ai/lib/src/rigging/providers/models/gemini.rs
  - unchained-ai/lib/src/rigging/providers/models/groq.rs
  - unchained-ai/lib/src/rigging/providers/models/metadata_generated.rs
  - unchained-ai/lib/src/rigging/providers/models/mistral.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/openai.rs
  - unchained-ai/lib/src/rigging/providers/models/openrouter.rs
  - unchained-ai/lib/src/rigging/providers/models/xai.rs
  - unchained-ai/lib/src/rigging/providers/models/zai.rs
  - unchained-ai/lib/src/rigging/providers/models/zenmux.rs
documentation:
  - claudine/features/2026-07-06-model-metadata/plan.md
  - claudine/features/2026-07-06-model-metadata/spec.md
  - unchained-ai/README.md
  - unchained-ai/artifacts/README.md
  - unchained-ai/docs/topics/models-and-metadata.md
  - unchained-ai/gen/README.md
  - unchained-ai/lib/README.md
packages:
  - claudine
  - unchained-ai
  - unchained-ai-gen
---

# Execution Plan — Model Metadata Source Migration (Parsera → models.dev)

Derives from [`spec.md`](./spec.md). Implements in `unchained-ai/gen` +
`unchained-ai/lib` + the committed `artifacts/` boundary; the Claudine runtime
wrapper is explicitly out of scope (spec §Non-goals).

## Orienting facts (verified against the tree at plan time)

- `parsera.rs` (316 LOC) is consumed by `gen/src/main.rs:16,22,285–291,334,341`
  and `gen/src/metadata_generator.rs:12,280–318`. Doc/comment references to
  "Parsera" live in `lib/src/rigging/providers/models/mod.rs:260`,
  `lib/src/models/model_metadata.rs:4,98,103`,
  `gen/src/provider_metadata/{mod.rs:5,17, openrouter.rs:18}`,
  `lib/README.md:144`, `README.md:31`, `justfile:153`, and the
  `unchained-ai/model-generator.md` skill page.
- `ProviderModelMetadata` (`lib/src/models/model_metadata.rs:105`) has **no**
  `release_date` field today; `created` is the only chronology field.
- The artifact mirror is `OfferingMetadata` (`gen/src/catalog.rs:103`) +
  `metadata_from` (`catalog.rs:495`); `SCHEMA_VERSION = 1` (`catalog.rs:26`).
- The live metadata loop (`main.rs:333`) iterates a **flat** `all_model_ids`
  list with no provider context. models.dev is keyed **per provider**, so this
  loop must become provider-aware (see Phase 2 risk).
- The drift gate `gen/tests/catalog_drift.rs` asserts **byte-equality** of both
  `models-catalog.json` and `models-catalog.schema.json` against a rebuild, so
  any change to schema types **must** be followed by `just artifact` in the
  same phase or CI fails.
- **Claudine-side consumer assumption:** no Claudine code reads
  `models-catalog.json` today. The sole reference,
  `claudine/catalog-types/src/offering.rs:65`, carries `catalog_id` as an
  opaque join string with no version check; the boundary design
  (`model-catalog-boundary.md:40,89`) defers the runtime consumer until
  "earned". Spec Goal 7's "consumer/version-check updates" therefore resolve
  to **verify-and-document** on the Claudine side, not a code edit. This is
  flagged again in Phase 3; if a re-scan finds a reader, Phase 3 grows a real
  task.

## Cross-cutting rules

- **Never run `cargo fmt`** (repo convention; `main` is the formatting
  authority). Match surrounding style by hand.
- **Atomic writes only** via the existing `write_atomic` pattern
  (`main.rs:100`); no ad-hoc `std::fs::write` for generated output.
- Tests run through `just test` / `just lint` in the `unchained-ai` area
  (nextest under the hood). Do not introduce `cargo test` invocations.
- US English (en-US) for all symbols and docs.
- No `git commit` unless explicitly requested.

---

## Phase 1 — Source client, metadata types, mapping, fixtures (no behavior change)

**Goal:** Land the `models.dev` client, the `release_date`/canonical-capability
plumbing on the lib types, the field/provider mapping, and an offline fixture —
all purely additive. Generation behavior is unchanged (parsera still drives
metadata), the committed artifact stays byte-identical (`release_date` is added
to `ProviderModelMetadata` only, **not** yet to the serialized
`OfferingMetadata`), and `just test` stays green.

- [x] Add `release_date: Option<String>` to `ProviderModelMetadata`
  (`lib/src/models/model_metadata.rs:140`, after `created`). Keep `created`
  semantics unchanged. Update the struct doc block (drop the Parsera-only
  framing at lines 4,98,103 → source-neutral wording). Add a unit test
  asserting default `release_date == None`.
- [x] Introduce canonical capability tokens for the four models.dev booleans.
  Add a small typed surface in `lib/src/models/model_capability.rs` (or a new
  `model_capability` constant block) exposing canonical serialized strings:
  `function_calling`, `structured_output`, `reasoning`, `file_input`. Map per
  spec §Field mapping: `tool_call → function_calling`, `structured_output →
  structured_output`, `reasoning → reasoning`, `attachment → file_input`. Unit
  test each mapping.
- [x] Create `gen/src/models_dev.rs`: `ModelsDevResponse`, `ModelsDevProvider`,
  `ModelsDevModel` (with nested `Limit`, `Cost`, `Modalities`) deserializers
  consuming **only** the fields we use; `#[serde(default)]` on everything
  optional; unknown fields ignored.
- [x] In `models_dev.rs` implement `models_dev_provider_key(Provider) ->
  Option<&'static str>` per spec §Provider key mapping (`gemini → google`,
  `x-ai → xai`, `z-ai → zai`; `ollama`/`zenmux → None`). Unit-test all
  providers incl. the three renames and the two `None` cases.
- [x] In `models_dev.rs` implement `fn models_dev_to_metadata(&ModelsDevModel)
  -> ProviderModelMetadata` covering the full field map: `name→display_name`,
  `family`, `limit.context→context_window`, `limit.output→max_output_tokens`,
  `modalities`, `knowledge→knowledge_cutoff`, `release_date`, and capabilities
  via the Phase-1 canonical mapper. **Pricing unit conversion: cost fields are
  per-million USD; divide by `1e6` before storing** in
  `pricing.prompt_per_token` / `completion_per_token` /
  `input_cache_read_per_token` (`ModelPricing` stays per-token — spec is
  explicit this is non-negotiable). Drop `temperature`/`reasoning_options`/
  `last_updated`. Unit-test conversion incl. the ÷1e6 path with assertions at
  f64 tolerance.
- [x] Implement the fetch path `fetch_models_dev_with_retry() -> Result<...,
  ModelsDevError>` (single GET `https://models.dev/api.json`, 30s timeout, one
  retry after 2s — mirroring `parsera.rs:84,110` shape) returning a typed
  `BTreeMap<&'static str, BTreeMap<String, ModelsDevModel>>` keyed by
  models.dev provider → model id. **No guards wired yet** (Phase 2); this task
  is just the HTTP + deserialize path, unit-tested against the fixture via an
  injected `&str` parser (no network in tests).
- [x] Commit a trimmed fixture `gen/tests/fixtures/models-dev.json` covering
  roster providers only (anthropic, google, openai, openrouter, xai, zai,
  deepseek, groq, mistral, moonshotai) with 2–3 models each, including at
  least one dash/dot version pair and one date-pinned vs unpinned pair (e.g.
  `claude-opus-4-5-20251101` ↔ `claude-opus-4.5`) to drive Phase 2 matcher
  tests. Hand-trim to keep it small and offline-stable.
- [x] Add offline `models_dev.rs` unit tests against the fixture: provider-key
  mapping, field mapping, ÷1e6 pricing conversion, canonical capability
  mapping, modality parsing, and deserialize-with-unknown-fields-ignored.
- [x] Update `metadata_generator.rs::generate_entry`
  (lines 109–271) to also emit `release_date` when present (mirror the
  `created` block at 263–267). Carry `release_date` through
  `parsera_to_metadata`/`merge_native_with_parsera` as `None` for now so the
  committed file is unaffected until live regen. Add a unit test.

**Parallelizable:** the lib-type tasks (first two bullets) and the
`models_dev.rs` authoring can proceed concurrently once both start from this
plan; they only converge in Phase 2.

**Validation checkpoint (end of Phase 1):**
- `just test unchained-ai` and `just test unchained-ai-gen` green; `just lint`
  clean.
- `just artifact` produces **byte-identical** `models-catalog.json` and
  `models-catalog.schema.json` (`release_date` is not yet on `OfferingMetadata`,
  so no schema drift). `catalog_drift.rs` green.
- `metadata_generated.rs` is **not** regenerated here; confirm it is unchanged
  in `git diff`.

---

## Phase 2 — Matching ladder, source-neutral merge, parsera deletion, guards

**Goal:** Wire `models_dev.rs` as the live enrichment source, delete parsera
end-to-end, rework the merge to be source-neutral, land the identity-aware
matching ladder, and make metadata-source degradation fail loudly. After this
phase, `gen-models` (live) would produce a `metadata_generated.rs` sourced
from OpenRouter-native + models.dev — but the committed file is still stale
until Phase 4's live regen, so the catalog drift test continues to pass
against the existing committed data.

- [x] Implement the matching ladder in `gen/src/models_dev.rs` (or a new
  `gen/src/matching.rs` it re-exports): given a model id, its processing
  `Provider`, and the models.dev bucket for that provider's mapped key —
  (1) **exact** id-string match; (2) **identity-aware** match via
  `ModelIdentity::parse` on both sides, comparing the `identity_key` grammar
  (`vendor/family[@version|@date_pin](+variant)*(+size)*(:tag)*` from
  `gen/src/catalog.rs:232`); tie-break rules per spec: prefer exact date-pin
  agreement, then the unpinned row, else ambiguity → no match (one report
  line); **never match across providers**. Uses
  `unchained_ai::models::identity::ModelIdentity` (already a dep).
- [x] Unit-test the matcher against the Phase-1 fixture: exact hit; identity
  hit across dash/dot (`claude-opus-4-5-20251101` ↔ `claude-opus-4.5`);
  identity hit across date-pin vs unpinned with the documented preference;
  cross-provider refusal; ambiguity refusal; genuine no-match → `None`.
- [x] Rework `MetadataGenerator::merge_metadata`
  (`metadata_generator.rs:51`) to a source-neutral
  `(Option<ProviderModelMetadata>, Option<ProviderModelMetadata>) ->
  Option<ProviderModelMetadata>` (native wins per field; models.dev fills
  gaps; `created` stays native-only; `release_date` comes from models.dev).
  Delete `parsera_to_metadata` and `merge_native_with_parsera`
  (lines 280–318). Update/rewrite the merge unit tests
  (`metadata_generator.rs:461–573`) to the new signature.
- [x] **Restructure the live metadata loop to be provider-aware.** Today
  `main.rs:333` iterates flat `all_model_ids` with a global parsera index.
  Replace with iteration grouped by the processing `Provider`: for each
  provider, look up its models.dev bucket via `models_dev_provider_key`, run
  the matching ladder within-bucket, then merge native (OpenRouter raw) +
  models.dev via the new source-neutral merge. This is the spec's
  "within its mapped models.dev provider" rule and is the phase's main
  refactor risk — keep `result.provider_native_raw` semantics (OpenRouter
  only) intact and keyed by bare id as today.
- [x] Wire the **fetch sanity guards** (spec §Fetch sanity guards). After
  `fetch_models_dev_with_retry`, hard-fail `main()` (non-zero exit, no
  graceful-empty fallback) when: the fetch errors after retry; the response
  has fewer than `MODELS_DEV_MIN_PROVIDERS` (= 50) providers; or any
  roster-critical models.dev provider
  (`anthropic, google, moonshotai, openai, openrouter, xai, zai, deepseek,
  groq, mistral`) is absent or empty. Guards are skipped under `--dry-run`.
  Add unit tests for each failure mode against synthetic thin/missing
  responses.
- [x] Add the **no-degraded-mode** invariant test asserting that every
  metadata-source guard failure returns `Err` (no enum-only bypass path
  exists). This is the anti-sunset contract.
- [x] Extend the gen report (`GenerationSummary::print`, `main.rs:74–96`) to
  emit per-provider match coverage `matched/total` so erosion is visible at
  every regen. Cover both direct providers and the OpenRouter bucket.
- [x] **Delete `gen/src/parsera.rs`** and remove `mod parsera;` +
  the `use parsera::…` line (`main.rs:16,22`). Remove the parsera fetch block
  (`main.rs:285–292`) and the `find_parsera_metadata` call (`main.rs:334`).
- [x] Sweep **all** remaining Parsera prose references to source-neutral or
  models.dev wording: `lib/src/rigging/providers/models/mod.rs:260`,
  `gen/src/provider_metadata/mod.rs:5,17`,
  `gen/src/provider_metadata/openrouter.rs:18`, `lib/README.md:144`,
  `README.md:31`, `justfile:153`. Leave `features/_completed/…` historical
  docs untouched.
- [x] Drop the stale `gen/scripts/regen_metadata.py` if it references parsera
  (verify first; remove only if so).

**Parallelizable:** matcher implementation + tests (bullets 1–2) is
independent of the merge rework (bullet 3) and the report extension
(bullet 7); they converge in the `main.rs` restructure (bullet 6).

**Validation checkpoint (end of Phase 2):**
- `just test unchained-ai-gen` green incl. all new matcher, guard,
  no-degraded-mode, and merge tests; `just lint` clean.
- `rg -n parsera unchained-ai/{src,gen,lib,README.md,justfile}` returns no
  hits outside `features/_completed/`.
- `just artifact` still byte-identical (committed `metadata_generated.rs`
  unchanged); `catalog_drift.rs` green.
- `cargo run -p unchained-ai-gen -- --dry-run` runs offline (no keys required)
  and exits 0; simulate a thin models.dev response in a unit test and confirm
  it errors loudly.

---

## Phase 3 — Artifact schema v2 wiring (offline)

**Goal:** Bump the committed artifact to `schema_version: 2`, serialize
`release_date`, update the artifact schema/docs and drift gate, and resolve
the Claudine-side consumer question. All offline — no live regen yet (that is
Phase 4), so the only data change is the version bump.

- [x] Add `release_date: Option<String>` (with
  `#[serde(default, skip_serializing_if = "Option::is_none")]`) to
  `OfferingMetadata` (`gen/src/catalog.rs:103`) and to the `metadata_from`
  projection (`catalog.rs:495`). Because every committed entry has
  `release_date == None` until Phase 4, the data JSON stays byte-identical
  except for the version bump.
- [x] Bump `SCHEMA_VERSION` to `2` (`gen/src/catalog.rs:26`). Update the
  constant doc comment to note v2 adds serialized `release_date`.
- [x] Re-emit both committed files via `just artifact`; commit the new
  `models-catalog.json` (now `schema_version: 2`) and the regenerated
  `models-catalog.schema.json`. `catalog_drift.rs` must be green after.
- [x] Update `gen/tests/catalog_drift.rs` (and the `catalog_shape_sanity_floors`
  test at line 50) to assert the committed artifact carries
  `schema_version == 2`. Add a positive test that `build_catalog()` emits 2.
- [x] Update `unchained-ai/artifacts/README.md` §`schema_version compatibility`
  to declare v2 current and v1 unsupported (consumers must reject v1).
- [x] **Claudine consumer verification (assumption check).** Re-scan the
  Claudine tree for any reader of `models-catalog.json` / `ModelsCatalog` /
  artifact `schema_version`. Expected result: none exists today (only
  `claudine/catalog-types/src/offering.rs:65`'s opaque `catalog_id` join).
  - If confirmed: record the finding in `artifacts/README.md` ("no Claudine
    reader yet; the version-reject contract is enforced by the drift gate and
    takes effect for Claudine consumers when the runtime consumer is earned")
    and add a regression test placeholder asserting `SCHEMA_VERSION == 2` so a
    future reader inherits the contract.
  - If a reader is found: add a `schema_version` check there that accepts v2
    and rejects v1 with a loud error, plus a test for each branch. (Flagged
    contingency; not expected per the boundary design.)
  - Phase 3 finding: a reader exists in `claudine/gen/src/artifact.rs`; it now
    accepts v2 and rejects v1 with tests.
- [x] Add an artifact-schema test that builds a fixture offering with a
  populated `release_date` (`YYYY-MM-DD`), serializes, and asserts the field
  round-trips and is skipped when `None`.

**Parallelizable:** bullets 1–3 are sequential (type → constant → emit); the
README/report/tests bullets (4,5,6,7) can be drafted in parallel once the
emission lands.

**Validation checkpoint (end of Phase 3):**
- `just test unchained-ai-gen` green; `catalog_drift.rs` asserts v2.
- Committed `models-catalog.json` shows `"schema_version": 2`; all
  `release_date` fields absent (skipped) until Phase 4.
- Claudine assumption resolution is written down; no Claudine code change
  unless the re-scan surprises us.

---

## Phase 4 — Live regen (Ken's shell; needs provider API keys)

**Goal:** Run the migrated generator against live APIs to refresh
`metadata_generated.rs` and re-emit the committed artifact with real
models.dev data (including `release_date` and recovered direct-provider
pricing). This phase is operator-driven; it is not CI-verified.

- [x] From `unchained-ai/`, run `just generate-models` (needs provider API
  keys in env). Review the new per-provider `matched/total` coverage report;
  confirm roster-critical providers are non-zero and the guards did not trip.
- [x] Spot-check the regenerated `metadata_generated.rs` for pricing sanity
  (anthropic, openai) and that `release_date` is populated as `YYYY-MM-DD`
  where models.dev provides it; confirm `created` is unchanged from the prior
  native path.
- [x] **► CHECKPOINT (Ken):** eyeball the regen diff — first regen on a new
  metadata source rewrites most of `metadata_generated.rs`; verify direct
  anthropic models now carry per-token pricing before proceeding.
- [x] Run `just artifact` to re-emit `models-catalog.json` and
  `models-catalog.schema.json` with the live data; commit both plus
  `metadata_generated.rs`.
- [x] Add/confirm a **coverage floor** test (offline, against the committed
  `metadata_generated.rs`) asserting every direct anthropic model carries
  metadata with per-token pricing. This becomes the post-regen backstop.
- [x] Capture the per-provider coverage snapshot in the regen PR description
  (spec Done-when requirement).
- [x] Post-regen validation: `just test unchained-ai` +
  `just test unchained-ai-gen` green; `catalog_drift.rs` green against the
  freshly emitted artifact; sanity floors (offerings > 600, gaps < 5,
  duplicate groups > 100) hold; the committed artifact now shows populated
  `release_date` entries and `schema_version: 2`.

**Validation checkpoint (end of Phase 4):** all offline tests green against
the live-regenerated committed files; coverage floor test passes; PR
description carries the coverage report.

**Phase 4 coverage snapshot for PR description:**

- Anthropic: 10/10 matched
- Deepseek: 2/2 matched
- Gemini: 22/50 matched
- Groq: 15/17 matched
- HuggingFace: 0/0 matched
- Mistral: 25/72 matched
- MoonshotAi: 4/11 matched
- Ollama: 0/3 matched
- OpenAi: 7/9 matched
- OpenRouter: 343/343 matched
- Xai: 8/9 matched
- Zai: 8/8 matched
- ZenMux: 0/141 matched

---

## Phase 5 — Docs drift pass

**Goal:** Bring every doc/skill that mentions the metadata source or artifact
schema into alignment with the shipped v2 + models.dev reality. Doc-only;
no behavior or generated-file changes (repo §Comment Quality scope
discipline).

- [x] Rewrite `.opencode/skill/unchained-ai/model-generator.md` **and**
  `.claude/skill/unchained-ai/model-generator.md` (both copies exist):
  Parsera → models.dev throughout — purpose, source-file tree (drop
  `parsera.rs`, add `models_dev.rs`), pipeline step 1, the "Parsera
  Integration" §(lines 71–94) becomes a "models.dev Integration" section
  describing per-provider bucketing, the matching ladder, ÷1e6 pricing, and
  the guards.
- [x] Update `unchained-ai/artifacts/README.md` regeneration section if not
  already done in Phase 3 (ensure it references models.dev, not Parsera).
- [x] Update `docs/dependencies.md` (root) **only if** the dependency set
  changed (models.dev uses the existing `reqwest` — likely no change; verify
  and skip if clean).
- [x] Stamp this feature spec: set the `clarified:` frontmatter / status line
  at top of `spec.md` to reflect implementation landed; note the Claudine
  consumer finding from Phase 3.
- [x] Final repo sweep: `rg -ni parsera` should return only
  `features/_completed/…` historical entries and the `.vscode/settings.json`
  word-list entry (harmless; leave or remove at reviewer discretion).

**Validation checkpoint (end of Phase 5):**
- `just test` / `just lint` green across `unchained-ai` (doc changes must not
  touch compiled output).
- `rg -ni parsera unchained-ai claudine --glob '!features/**'` clean except
  the documented exceptions.

---

## Done when (mirrors spec §Done when)

- `parsera.rs` deleted; `models_dev.rs` is the sole non-native enrichment
  source with loud-fail guards (no enum-only bypass).
- `ProviderModelMetadata` + `OfferingMetadata` carry `release_date`;
  `created` unchanged; models.dev dates populate `release_date` after regen.
- Committed artifact emits `schema_version: 2`; drift gate, schema docs, and
  the (currently vacuous) Claudine consumer contract accept v2 and reject v1.
- Capabilities flow through canonical typed tokens mapping models.dev booleans
  to `function_calling` / `structured_output` / `reasoning` / `file_input`.
- Offline fixture tests cover mapping, ÷1e6 conversion, the matching ladder,
  and all guard failure modes; `just test`/`just lint` green in unchained-ai.
- Phase-4 live regen committed: every direct anthropic model has per-token
  pricing; coverage report in the PR; `metadata_generated.rs` + both artifact
  files drift-clean.
- Skills/READMEs updated; no stray Parsera references outside history.

## Risk register

1. **Provider-aware metadata loop (Phase 2)** — the flat `all_model_ids`
   refactor is the largest mechanical change; mitigated by keeping
   `provider_native_raw` (OpenRouter-only) keyed exactly as today and merging
   per-provider. If the refactor surfaces OpenRouter-prefixed id matching
   issues, fall back to identity-key matching for the OpenRouter bucket only.
2. **Claudine consumer assumption (Phase 3)** — plan assumes no Claudine reader
   of the artifact exists. Phase 3's re-scan is the gate; if wrong, Phase 3
   grows a real code+test task and the boundary design needs a note.
3. **÷1e6 precision** — store pricing as f64 from `cost / 1e6_f64`; assert in
   tests at 1e-12 relative tolerance to avoid float drift across regen.
4. **Scheduling** — per spec §Relationship to provider-metadata, this should
   land before the weekly regen cadence hardens; no interim regen should run
   expecting Parsera enrichment.
