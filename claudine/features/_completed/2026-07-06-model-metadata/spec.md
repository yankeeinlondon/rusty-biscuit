---
clarified: "implemented/codex/gpt-5"
review_iterations: 2
---

# Model Metadata Source Migration: Retire Parsera, Adopt models.dev

> **Status:** IMPLEMENTED — phases 1–5 landed.
> Implementation spans `unchained-ai/gen`, required `unchained-ai/lib`
> metadata types, generated metadata, model-catalog artifact schema v2
> emission, docs, and any Claudine-side artifact consumer/version checks needed
> to accept v2. Claudine runtime wrapper behavior remains out of scope. The
> spec lives here because it serves the 2026-07-02 provider-metadata artifact
> boundary (design/model-catalog-boundary.md). It does not block, and is not
> blocked by, that spec's Phase F — see
> [Relationship to provider-metadata](#relationship-to-the-provider-metadata-spec).

## Why

The Parsera LLM Specs API — one of the two metadata sources feeding
`metadata_generated.rs` — **has been sunsetted upstream**. The endpoint
(`api.parsera.org/v1/llm-specs`) still returns HTTP 200 but now serves a
residual ~125-spec dataset covering only gemini/deepseek/openai. No Anthropic,
Mistral, Moonshot, xAI, or Z.ai entries exist at all. RubyLLM, whose maintainer
co-created the API with Parsera, has migrated to [models.dev](https://models.dev).

The degradation was **silent**: `fetch_parsera_specs_with_retry` warns and
returns an empty/thin map on failure, so generation proceeded and produced a
catalog whose direct-provider metadata coverage collapsed without any test or
report noticing. Measured against the Phase F artifact (662 offerings):
425 offerings carry metadata, ~403 of which trace to the OpenRouter native
path; direct-provider coverage is anthropic 0/10, mistral 0/62, moonshotai
0/11, x-ai 0/9, z-ai 0/8.

Two lessons, both in scope:

1. Replace the dead source with its successor.
2. Make metadata-source failure **loud** so an upstream sunset can never again
   pass silently through generation, commit, and CI.

## Goals

1. **Retire Parsera entirely** — delete `gen/src/parsera.rs` and all references;
   no residual tertiary source.
2. **Adopt models.dev** (`https://models.dev/api.json` — open-source SST
   project, free, no API key) as the enrichment source alongside the existing
   OpenRouter native path.
3. **Direct-provider metadata with direct-provider pricing.** models.dev keys
   models in direct bare form (`claude-opus-4-8`) and carries per-provider
   `cost` and `limit` blocks. Measured floor by exact id match alone:
   anthropic 10/10, z-ai 8/8, x-ai 8/9, groq 15/17, deepseek 2/2,
   openrouter 307/340, mistral 18/62, google 22/50, moonshotai 4/11 —
   recovering 76 of the 237 currently metadata-less offerings before any
   normalization.
4. **Identity-aware matching** as the fallback matcher (see
   [Matching ladder](#matching-ladder)), lifting coverage further across
   dash/dot, date-pin, and fused-stem spelling drift.
5. **Fetch sanity guards**: generation fails loudly when a metadata source is
   unavailable, implausibly thin, or missing roster-critical providers.
6. Add a first-class serialized `release_date` field for source release dates;
   leave `created` unchanged and do not alias models.dev `release_date` into it.
7. Bump the committed model-catalog artifact `schema_version` to `2`, update
   schemas/docs/consumer checks for v2, and make artifact readers reject v1
   after the migration lands.
8. Refresh `metadata_generated.rs` (live regen) and re-emit the committed
   model-catalog artifact from validated source data; all existing drift tests
   stay green.

## Non-goals

- **Per-provider native API parsers** (Gemini's models endpoint returns token
  limits; Mistral returns context/capabilities). That is the only path to the
  direct-only tail models.dev lacks (Mistral embeddings/OCR, Gemini
  TTS/previews) — worth doing roster-first, but a separate follow-up.
- **Broad artifact redesign beyond schema v2.** This migration bumps the
  artifact schema only for the serialized metadata addition (`release_date`).
  The intrinsic-vs-offering metadata split (identity-level `models` section)
  remains deferred.
- **Local runners.** models.dev has no usable ollama catalog; local-model
  metadata remains model-citizen territory.
- **Claudine runtime wrapper behavior.** Claudine-side work is limited to
  artifact consumer/version-check updates needed to accept schema v2.

## Design

### Source client

New `gen/src/models_dev.rs` replacing `parsera.rs`:

- Single GET of `https://models.dev/api.json` (~150 providers), reqwest with
  the existing timeout/retry-once pattern — but degraded results are **errors**,
  not warnings (see guards).
- Response shape: `{ "<provider>": { "models": { "<model-id>": { … } } } }`.
- Deserialize only the fields we consume; unknown fields ignored.

### Provider key mapping

Ours → models.dev: `anthropic`, `deepseek`, `groq`, `mistral`, `moonshotai`,
`openai`, `openrouter` map 1:1; `gemini → google`, `x-ai → xai`, `z-ai → zai`.
`ollama` and `zenmux` have no models.dev key: ollama is out of scope; zenmux
rows continue to ride the OpenRouter native path via shared vendor-form id
strings (96/141 today) plus the identity join in the artifact.

### Field mapping (models.dev → `ProviderModelMetadata`)

| models.dev | ours | note |
|---|---|---|
| `name` | `display_name` | |
| `family` | `family` | parser-derived fallback stays (metadata_generator.rs) |
| `limit.context` | `context_window` | |
| `limit.output` | `max_output_tokens` | |
| `modalities.input/output` | `modalities` | string forms already align |
| `cost.input` / `cost.output` | `pricing.prompt_per_token` / `completion_per_token` | **per-million → per-token: divide by 1e6.** `ModelPricing` stays per-token USD — it is a shared, serialized type; changing its unit is a breaking artifact change for no gain |
| `cost.cache_read` | `pricing.input_cache_read_per_token` | ÷ 1e6 |
| `knowledge` | `knowledge_cutoff` | |
| `release_date` | `release_date` | serialized artifact/schema addition; preserve as source date string (`YYYY-MM-DD`) and leave `created` unchanged |
| `tool_call` / `structured_output` / `reasoning` / `attachment` | `capabilities` entries | map via canonical serialized capability strings backed internally by typed constants or an enum: `tool_call → function_calling`, `structured_output → structured_output`, `reasoning → reasoning`, `attachment → file_input` |
| `temperature`, `reasoning_options`, `last_updated` | dropped in v1 | revisit if a consumer wants them |

`created` remains whatever the existing OpenRouter-native path provides today.
Consumers that need release chronology can prefer `release_date` when present
and fall back to `created` only as an older-source timestamp.

### Artifact schema v2

Adding serialized metadata is an artifact schema change. The model-catalog
artifact emitted by `unchained-ai/gen` must set `schema_version: 2` when
`release_date` is present. The implementation must update:

- the artifact schema/docs that define valid model-catalog fields,
- generated artifact emission and drift checks,
- Claudine-side artifact consumer/version checks so v2 is accepted and v1 is
  rejected, and
- tests that fail if v2 artifacts are emitted but docs or consumers still
  recognize only v1 or continue accepting v1.

Phase 3 found an actual Claudine-side artifact reader in `claudine-gen`
(`claudine/gen/src/artifact.rs`), rather than the originally expected vacuous
consumer boundary. The reader now accepts schema v2 and rejects v1 with tests;
the artifact README records this contract.

### Matching ladder

For each generated model id, within its mapped models.dev provider:

1. **Exact** model-id string match (covers the direct providers wholesale).
2. **Identity-aware** match: parse both our id and each models.dev candidate id
   with `ModelIdentity::parse` and compare **identity keys** (the Phase F
   `vendor/family[@version|@date_pin](+variant)*(+size)*(:tag)*` grammar).
   This is matching on *what the id denotes* instead of how it is spelled —
   the same normalization the spike validated at 99.9%: `claude-opus-4-5-20251101`
   (dash version, date pin) matches a source row keyed `claude-opus-4.5`
   (dot version, no pin) because both parse to `anthropic/claude-opus@4.5`.
   Rules: never match across providers; if multiple candidates share the key
   (e.g. a date-pinned and an unpinned row), prefer exact date-pin agreement,
   then the unpinned row; ambiguity beyond that → no match, one report line.
3. No match → no metadata from this source (report line, feeds the
   unmatched-tail section of the gen report).

Merge priority is unchanged in spirit: **OpenRouter native > models.dev** per
field (native is fetched live at the moment of generation and also serves the
zenmux id space). `MetadataGenerator::merge_metadata`'s Parsera-typed signature
is reworked to a source-neutral `ProviderModelMetadata`-vs-`ProviderModelMetadata`
merge so future sources slot in without another rework.

### Fetch sanity guards (the anti-sunset contract)

Generation **fails** (no graceful-empty fallback for metadata sources) when:

- the models.dev fetch errors after one retry, or
- the response contains fewer than a floor count of providers (proposal: 50),
  or
- any **roster-critical provider** (`anthropic`, `google`, `moonshotai`,
  `openai`, `openrouter`, `xai`, `zai`, `deepseek`, `groq`, `mistral`) is
  absent or has zero models.

The gen report additionally prints per-provider match coverage
(`matched/total`) so gradual erosion is visible at every regen, not just
total collapse.

There is no degraded-mode escape hatch in v1. Any degraded metadata-source
condition fails generation loudly; enum-only generation is not allowed to bypass
metadata-source failures in this spec.

### Testing

- **Committed fixture**: a trimmed snapshot of `api.json` (roster providers
  only) under `gen/tests/fixtures/`, driving offline tests of the field
  mapping, unit conversion, provider-key mapping, canonical capability mapping,
  and the matching ladder (exact hit, identity hit across dash/dot and date-pin
  drift, cross-provider refusal, ambiguity refusal).
- **Guard tests**: thin/missing-provider responses must error.
- **No-degraded-mode tests**: degraded metadata-source conditions must fail
  generation loudly; no enum-only bypass is accepted.
- **Artifact schema tests**: emitted model-catalog artifacts use
  `schema_version: 2`, include serialized `release_date` as `YYYY-MM-DD`, and
  schema/docs/consumer version checks accept v2 and reject v1.
- Existing Phase F artifact drift tests are the backstop for the re-emit step:
  they fail until `emit-catalog` is re-run, and the sanity floors
  (offerings > 600, gaps < 5, duplicate groups > 100) hold post-migration.
- Coverage floor test post-regen (uses the committed `metadata_generated.rs`,
  offline): every direct anthropic model carries metadata with pricing.

## Sequence

1. **Client + metadata types + mapping + fixtures** — `models_dev.rs`,
   `unchained-ai/lib` metadata type updates for `release_date` and canonical
   capabilities, field/provider mapping, offline tests. No behavior change to
   generation yet.
2. **Matching ladder** — identity-aware matcher in the gen crate (uses
   `unchained_ai::models::identity`), source-neutral merge rework,
   `parsera.rs` deleted, guards wired, gen report extended.
3. **Artifact schema v2 wiring** — emit `schema_version: 2`, serialize
   `release_date`, update schemas/docs, drift checks, and Claudine-side artifact
   consumer/version checks to accept v2 and reject v1.
4. **Live regen** (Ken's shell — needs provider API keys):
   `just generate-models`, review the coverage report, then `just artifact`
   to re-emit the committed schema-v2 model-catalog artifact.
   ► **CHECKPOINT (Ken):** review the regen diff — first regen on a new
   metadata source rewrites most of `metadata_generated.rs`; eyeball pricing
   spot-checks (anthropic, openai) and the new `release_date` field before
   commit.
5. **Docs drift pass** — `.claude/skills/unchained-ai/model-generator.md`
   (Parsera → models.dev), artifact schema/version docs, `artifacts/README.md`
   regeneration section, root `docs/dependencies.md` if dependencies changed,
   this spec stamped.

## Relationship to the provider-metadata spec

Phase F's artifact boundary consumes **identity**, which derives from wire ids
and the parser — it is complete (661/662) regardless of this migration.
Metadata coverage affects only metadata-dependent consumers (cost-aware
selection, capability-aware fallback), which are downstream of Checkpoint F,
and the artifact's duplicate-group join already recovers metadata for the
roster-critical direct models meanwhile. The one scheduling constraint: this
migration should land **before the weekly regen cadence becomes routine**, so
that regens improve rather than fossilize the coverage gap — and no regen
should be run expecting Parsera enrichment in the interim.

## Done when

- `parsera.rs` is gone; `models_dev.rs` is the sole non-native enrichment
  source, with guards that fail generation on source degradation.
- The serialized metadata/artifact schema includes `release_date`; models.dev
  dates populate it, and `created` remains unchanged.
- The model-catalog artifact emits `schema_version: 2`; schema docs, drift
  checks, and Claudine-side artifact consumers/version checks accept v2 and
  reject v1.
- Capabilities are emitted through canonical typed constants or an enum, with
  models.dev booleans mapped to `function_calling`, `structured_output`,
  `reasoning`, and `file_input`.
- Degraded metadata-source conditions fail generation loudly; no
  enum-only bypass exists in v1.
- Offline fixture tests cover mapping, conversion, matching ladder, and
  guards, including canonical capability mapping and artifact schema v2;
  `just test` / `just lint` green in the unchained-ai area.
- A live regen has landed: every direct anthropic model has metadata with
  per-token pricing; per-provider coverage report committed in the regen PR
  description; `metadata_generated.rs` and both artifact files re-emitted and
  drift-clean.
- Docs updated (model-generator skill page, artifacts README).
