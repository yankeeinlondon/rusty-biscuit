# Spike: Model Identity Grammar (2026-07-02)

> Question under test: is the family of a model reliably inferable from its wire id, and
> can an intra-family ordering answer "the latest `sonnet` is …?" — empirically, against
> unchained-ai's real generated catalog.

## Method

- Extracted **687 wire ids** from the `/// Model:` doc comments of unchained-ai's
  generated provider enums (12 providers; corpus in [`model-ids.txt`](model-ids.txt)).
- Wrote a ~300-line dependency-free Rust prototype ([`parser.rs`](parser.rs)) implementing
  the spec's identity grammar: `[source /] vendor / model-id`, with model-id decomposed
  into family + version + variants + size + date-pin + serving tag.
- Full output in [`parse-report.txt`](parse-report.txt).

## Results — the hypothesis holds

| Metric | Result |
|--------|--------|
| Family inferred | **686/687 (99.9%)** — sole exception: `openrouter/openrouter/auto` (a meta-router, correctly identity-less) |
| Version or date-pin parsed | ~80% |
| Genuinely versionless products | ~15% (embeddings, `groq/compound`, `gpt-oss-120b`, …) — legit, not parse failures; "latest" = single member |
| Distinct vendor/family keys | 273 |
| Cross-source identity groups (same model from >1 source) | **130** (~19% of the corpus is duplicate offerings) |

### Latest-of-family demo (correct against this catalog)

| Query | Answer |
|-------|--------|
| latest `claude-sonnet` | 4.6 |
| latest `claude-opus` | 4.7 |
| latest `kimi-k` | k2.6 |
| latest `gpt` | gpt-5.5-pro |
| latest `glm` | glm-5.1 |

**Staleness caveat:** the catalog was generated 2026-05-07 — the real world has k2.7,
glm-5.2, opus-4.8. All answers are correct *against the corpus*; regeneration cadence is
therefore a **correctness input** to "latest", not a hygiene concern (ContentPolicy
applies to the catalog).

### Normalization wins (things the grammar unified without special-casing)

- **Dot vs dash versions**: `anthropic/claude-sonnet-4-5-20250929` ≡
  `openrouter/anthropic/claude-sonnet-4.5` — grouped as one model.
- **Era-dependent token order**: `claude-3.5-haiku` (version-first, 3.x era) ≡
  `claude-haiku-4-5` (tier-first, 4.x era) — same `claude-haiku` family, because the
  tokenizer treats version position independently.
- **Vendor spelling drift** required a 5-entry curated alias map (`z-ai`→`zai`,
  `x-ai`→`xai`, `mistralai`→`mistral`, `meta-llama`→`meta`, source `gemini`→vendor
  `google`) — consolidating 308→273 family keys and raising cross-source matches 103→130.

### Curation surface is small and enumerable

The grammar needs three curated tables, all short:

1. **Variant vocabulary** (~30 tokens): fast/highspeed/turbo, instruct/chat/it/base,
   preview/exp/beta, vl/vision/audio/tts, distill, guard, etc.
2. **Vendor alias map** (5 entries so far).
3. **Serving tags** (`:free`, `:extended`, `:nitro`, `:thinking`, local `:27b` size tags).

### Rough edges (backlog for the production version)

1. `:thinking`-style serving tags should join the identity key (currently
   `claude-3.7-sonnet:thinking` groups with plain `3.7-sonnet`).
2. Serial-number versions (`gemini-embedding-001`) parse into the family; treat trailing
   zero-padded digit runs as version serials.
3. Date-pin forms found in the wild: `YYYYMMDD`, `YYMM` (Mistral's `2512`),
   `MM-YYYY` pair, `YYYY-MM-DD` triple — all handled, but the YYMM heuristic is
   year-window-bounded (2023–2027) and needs widening over time.
4. OpenAI's fused ids (`4o`, `o3`, `o4-mini`) are family-literals by fiat — acceptable,
   but the o-series/gpt-series family relation is a curation decision, not inferable.

## Recommendation

The identity grammar is viable as **deterministic code plus three small curated tables**
— no LLM in the loop, no per-model curation. Production home: `unchained-ai/gen`, which
should (a) populate `family`/version/variant fields in the generated metadata from this
parser, (b) expose a `latest_in_family` query over the ordering (version compare,
date-pin then `created` as tiebreakers), and (c) include parsed identity in the JSON
artifact Claudine consumes. Claudine's mapping records then target parsed identities, and
rolling aliases (`kimi-latest`, `sonnet`) resolve through the family index.
