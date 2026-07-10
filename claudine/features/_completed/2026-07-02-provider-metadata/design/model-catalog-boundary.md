# Supplemental Design: Model Catalog Boundary Contract (unchained-ai ↔ claudine)

> **Status:** ratified at Checkpoint F (2026-07-06). Refines spec.md "Model Ground
> Truth" (division of domains, integration shape, identity grammar, refinements 1–4)
> and hl-approach tracks B/C. Ratified input: F4 (plan endpoints get first-class
> offering records in Claudine's mapping layer). Checkpoint F rulings are amended
> inline below, marked *(Checkpoint F)*.

## Canonical identity (refinement 1 — the representation, not just the mechanism)

- **Identity key:** a normalized string `family@version[+variant…]` (e.g.
  `claude-sonnet@4.5`, `kimi-k2@2.7+code`), derived deterministically by the spike's
  parser. The full parsed struct (source, vendor, family, version, variants, date-pin,
  serving tags) travels alongside in the artifact; the string key is what mapping
  records reference.
- **Stability contract:** unchanged input id ⇒ unchanged key across regenerations.
  Curation-table changes that would move existing keys are breaking artifact changes
  (schema_version bump, see below).
- **Duplicate-offering groups** (130 found): the group shares the identity key; no
  "canonical offering" is elected — consumers that need one pick by their own policy
  (e.g. direct-API over aggregator). The artifact records group membership.
- **Unparseable ids** are identity-less offerings — allowed, carried through, flagged
  in the artifact's gap list (the spike's `openrouter/auto` case). Generation does not
  fail on them.

## The JSON artifact (the entire cross-crate boundary)

- **Producer:** `unchained-ai/gen`. Ruling on the bin-vs-lib tension, as amended
  *(Checkpoint F)*: the identity parser lives in `unchained_ai::models::identity`
  (it landed there pre-Phase-F and also serves runtime family fallback); the
  **lib target inside the gen crate** hosts the artifact schema types, family-index
  builder, and emission (lib+bin, no new workspace member). The artifact remains
  the only cross-area interface. Claudine never links unchained-ai code (rig-core
  weight stays out, as decided).
- **Artifact:** `unchained-ai/artifacts/models-catalog.json`, committed, with
  `schema_version`, `generated_at`, the offering list (parsed identity + metadata +
  duplicate groups + family index), and a published JSON Schema next to it.
  `claudine/gen` consumes it by workspace-relative path; absence or
  schema_version mismatch is a loud generation error on the claudine side.
- **Escalation to a shared crate** (spec's option 2) is "earned" when a *runtime*
  consumer needs the types — see model_catalog below; not before.

## Plan endpoints and local runners (refinement 4 — F4, ratified)

`zai-coding-plan/*`, `kimi-for-coding/*`, `ollama/*`, `omlx/*` get **first-class
offering records in Claudine's mapping layer**, sourced from the agent-models (and
model-config/local_runners) research, with an optional `catalog_id` join into the
unchained-ai artifact where the underlying model is known. The unchained catalog stays
model-API-only by design. Consequence: the ids Claudine's own research fleet runs on
become representable, and a fallback can be classified same-family-downgrade vs
cross-vendor-substitution even for plan offerings.

## `family_latest` semantics

- **Resolver:** Claudine, at selection/compose time, against the vendored artifact
  snapshot (the family index ships in the artifact; no cross-crate call).
- **`latest` names a release, not an offering** *(Checkpoint F)*: the family
  index's `latest` field is an **identity key** (`anthropic/claude-opus@4.8`);
  consumers pick a concrete offering from the duplicate group by their own policy,
  consistent with the no-canonical-offering rule above.
- **Rolling aliases** (`sonnet`, `kimi-latest`, `kimi-k2`) resolve via mapping records
  marked `resolves: family_latest` (as decided); the concrete answer is stamped into
  session logs so reporting sees which model an alias meant *that session*.
- **Staleness threshold:** if `generated_at` is older than a configurable max age
  (default 30 days), `family_latest` answers carry a warning; the spike's
  already-stale answers (sonnet 4.6 vs the real 4.8-era world) show why this is a
  correctness input, not hygiene.

## Runtime `model_catalog` migration (the service the spec forgot)

`claudine/lib/src/model_catalog/` stays a runtime service (cache, user overrides,
dynamic CLI listing). Changes, staged per provider:

1. The generated expected-offering records (from agent-models research + the artifact
   join) become the validation baseline **only once they exist for that provider**;
   until then, dynamic listing remains truth (current behavior — no flag-day flip of
   `fetch_provider_catalog` semantics).
2. Once flipped, dynamic listing demotes to a **drift channel**: set differences
   emit a `SignalEvent` (design/signal-detection.md) and surface in `logs` reports.
3. User overrides keep their current merge position (they win locally) but gain an
   optional `catalog_id` so an override can still join identity.
4. The hand-drifted `static_models` lists retire in favor of the generated records —
   they are the facts-file pattern's model-domain cousin and follow the same
   graduation rule (design/catalog-generation.md).

Runtime metadata needs (cost-aware selection, capability-aware fallback): served by
compiling **per-provider offering slices** (only providers on the roster, only fields
claudine consumes) into the generated data — not the 687-model catalog. If a future
consumer genuinely needs the full catalog at runtime, that is the "earned" trigger for
the shared crate.

## Regeneration policy (refinement 3, made concrete)

- Manual trigger (`gen-models` needs API keys + a human shell), goal cadence weekly.
- ContentPolicy, minimally defined here: `{ generated_at, max_age, on_stale: warn }` —
  stamped in the artifact, enforced by consumers (the staleness threshold above).
- CI verifies artifact/schema_version compatibility and the committed-artifact drift
  test on the parsing side (fixtures), NOT live regeneration.
- Parsera validation pass: scoped as a one-time audit task, not a pipeline stage.

## Curation tables

The three tables (variant vocab ~30 tokens, vendor aliases, serving tags) live in
`unchained-ai/gen` **source** — reviewable in PRs like code, no overrides layer of
their own in v1. New-token workflow: parser flags unrecognized candidate tokens in the
gen report; a human promotes them to the table. The spike's rough edges (`:thinking`
tag, o-series↔gpt family relation) are decided at first promotion, recorded in table
comments.
