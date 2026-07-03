# Provider Metadata: Multi-Phase Implementation Plan

> **Goal:** implement the provider-metadata spec end-to-end — catalog generation,
> module restructuring, pipeline DRY, signal catalog, model-catalog boundary, and
> functional rendering — validated by onboarding three real providers (Kilo Code, Pi,
> Antigravity).
>
> **Authority chain:** spec.md is the umbrella; the six `design/*.md` docs rule where
> they refine it; this plan sequences the work. The topics-closeout plan
> (`topics-closeout-plan.md`) is an **external dependency track** executed by a
> separate agent — this plan consumes its outputs and never runs research fleets
> except where explicitly stated (signals sequence, Antigravity sweep).

## Context the executing agent needs

- **Use the `claudine` agent skill** (load first). Read the six `design/*.md` docs
  before their phase — each opens with ratified rulings this plan assumes.
- **Testing:** `just test` / `just test-l2` / `just lint` per package area; nextest.
  Every phase ends green. Generated-code phases add their own drift tests.
- **Scope discipline:** code-motion commits (module split) contain zero behavior
  change; behavior changes never hide in generated-code commits. No `cargo fmt`.
- **Orchestration:** implementation work fans out to subagents per bounded unit
  (one crate, one provider, one component); the orchestrator holds the plan state and
  reviews diffs. Subagents never commit.
- **HITL checkpoints** are marked ► — stop and get Ken's sign-off before proceeding.

## Dependency picture

```
[Closeout track: research fleets]────────────────────────────┐ (external, parallel)
                                                              ▼
A1 walking skeleton ──► B module split + generator v1 ──► D field expansion
        │                        │                          + profile migration ──► I guards
A2 dispatch inventory ──────► C pipeline ws0 + FinalMessage ─┘        ▲
                                 │                                    │
F model-catalog boundary (parallel, unchained-ai side)────────────────┤
E signal catalog (needs A1's catalog-types + gen)─────────────────────┤
G rendering buildout (interleaves after C)────────────────────────────┤
                                                              H provider ladder
                                                              (Kilo → Pi → Antigravity)
```

## Phase A — de-risk (the gate phase)

**A1. Walking skeleton** (design/catalog-generation.md made real, minimally):

1. Create `claudine/catalog-types` (leaf: coerced enums, shared vocab enums,
   `DisplayPolicy`/`EventClass` shells) and `claudine/gen` (bin, deps: catalog-types +
   darkmatter + serde — no claudine lib).
2. Mapping registry with ~5 entries spanning every source kind: two roster identity
   fields, one facts-file field, one research-fed scalar and one research-fed enum
   (from agent-models / agent-permissions), one override.
3. One provider (claude) end-to-end: scrape its current constant values into
   `docs/providers/facts/claude.yaml` (one-time scraper), generate `data.rs`
   fragments, byte-compare against the hand-written original.
4. Prove the gates: enum-subset compatibility check fails on a doctored sidecar;
   a deliberate source collision fails loudly; drift test == `--check` mode.
5. `claudine providers generate --mapping` emits the registry as JSON (CLI rendering
   comes later).

Exit: skeleton generates byte-equivalent data for claude; all four gate behaviors
demonstrated by tests.
► **CHECKPOINT A (Ken):** review the skeleton — registry ergonomics, facts-file
shape, generate UX (diff/confirm/decline→override), before the pattern is multiplied
by 36 fields × 8 providers.

**A2. Mechanical dispatch inventory** (design/pipeline-dry.md; parallel with A1,
cheap): the scan script + committed inventory (path, line, pattern form, providers
named) covering the extended pattern set (`match`, `matches!`, `==`). Replaces the
stale topic-doc tables (topic doc gets a pointer). Output feeds C, D, and I sizing.

## Phase B — module split + generator v1

1. **Module split** (design/module-split.md): `provider/<slug>/{mod,data,behavior,legacy}.rs`
   for all 8 providers — pure code motion (ProviderInfo const → data.rs, four trait
   impls → behavior.rs, AgentCapabilities builders → legacy.rs), parsers/adapters stay
   put, lib allow-list updated, shrink-only guard on `legacy.rs`.
2. **Field source matrix**: every current ProviderInfo field + table A → declared
   source (roster | topic | facts). Facts files scraped for all 8 providers.
   ► **CHECKPOINT B (Ken):** review the matrix — it is the contract everything else
   consumes; wrong source declarations are expensive later.
3. **Generator v1**: registry covers all current fields; `data.rs` generated
   byte-stable for all 8; drift test + CI `--check` land; `catalog.json` superset
   emitted; `--mapping` rendered by claudine-cli through renderable components.

Exit: no hand-written provider data remains; CI enforces regeneration equality.

## Phase C — pipeline workstream 0 + first component

1. Shared prep stages (design/pipeline-dry.md): extract OpenCode model resolution,
   shadow-home env, Codex output prep into `exec_prep/`; both pipelines consume them.
2. `FinalMessage` renderable component (design/render-components.md migration 1)
   retires the ×3 Codex rendering; `lib/src/render/` module is born.
3. **AgentCapabilities retirement** (design/module-split.md): migrate
   `providers.rs` describe output to ProviderInfo + catalog.json (rendered through
   components); delete the `agents::Agent` trait, `agent_for`, the 80-field tree,
   every `legacy.rs`, and the agreement tests.

Exit: one source of provider truth; three duplicated prep sites gone; first
functional component live in both pipelines.

## Phase D — field expansion + WrapperProfile migration (rolling)

Consumes closeout-track topics as they land; repeats per topic:

1. Extend the mapping registry (topic → fields); graduate facts entries; regenerate.
2. Generate the **WrapperProfile disposition table** from the A2 inventory + the
   data/behavior litmus test; ► **CHECKPOINT D (Ken):** ratify the disposition table
   (57 overrides classified catalog-data vs behavior).
3. Migrate `catalog-data` overrides method-by-method to catalog-driven defaults;
   delete as they zero out. Behavior overrides stay.

Exit: static-fact overrides at zero; profile is a genuine behavior shim; table A
fields all research-fed or facts-fed with a tracked graduation queue.

## Phase E — signal catalog (spec Phase 2s; design/signal-detection.md)

1. Taxonomy + `SignalEvent` in catalog-types; detection-record schema sidecar
   (path grammar, four operators, priority).
2. **Author + run the `signals` research sequence** (source-code-first methodology —
   this fleet is owned by THIS plan, not closeout); seed the fixture corpus from
   existing parser test data.
3. Generate-time record compilation into `&'static` tables + the generic engine +
   sink dedup; `claudine signals check` (positive fixtures for every record,
   mechanical-overlap negative assertions), wired into CI.
4. Migration map executions: Claude rate-limit records; OpenCode 429 cascade as
   priority-ordered records (bespoke locator only if the path grammar can't reach);
   temporal guards named in taxonomy as permanent bespoke, emitting through the sink.
5. Harvest v1 (unmatched error/warning events only, scrubbed, capped) ships last.

## Phase F — model-catalog boundary (parallel track, unchained-ai side)

Per design/model-catalog-boundary.md: identity parser as a lib target in
`unchained-ai/gen` (from the spike prototype) → committed
`artifacts/models-catalog.json` + JSON Schema + generated_at/schema_version →
claudine-gen consumption (expected-offering records with `catalog_id` joins;
plan-endpoint + local-runner offerings first-class in the mapping layer) →
`family_latest` resolution + staleness warnings → staged per-provider demotion of
dynamic listing to a drift channel emitting SignalEvents (couples to Phase E sink).
► **CHECKPOINT F (Ken):** artifact schema review before claudine consumes it.

## Phase G — rendering buildout (interleaves after C)

Migrations 2–4 of design/render-components.md: `AgentPrompt`/`SystemPrompt` absorb
`prompt_reporting`; `EventRenderer` + exhaustive dispatch table replace the live
sink's scattered branches (per event class, incremental); `StreamRenderable` span
contract for `ThinkingToken`/`ToolUse`; `MetricsReport` with the mandatory browser
target as the dual-target proof; DisplayPolicy populated as a generated catalog
section (noise prefixes move here — single owner).

## Phase H — the provider ladder (validation milestones)

Prerequisites: generator v1 (B), enough topics landed for a meaningful graduation
report (closeout well underway).

- **M-Kilo** — graduation #1. OpenCode-fork cousin: smallest behavior delta, so the
  process is the test: variant wiring (3 hand edits, compiler-walked) →
  `generate kilo --scaffold` → behavior half → graduation report clean.
  ► **CHECKPOINT H1 (Ken):** process retro — scaffold quality, generate UX, report
  accuracy; adjust before Pi.
- **M-Pi** — graduation #2, the sterner behavior test (bespoke models.json/API
  surface). Proves the data/behavior seam on a non-cousin.
  ► **CHECKPOINT H2 (Ken):** second retro; confirm the process is provider-shape
  independent.
- **M-Antigravity** — true end-to-end. Roster entry lands only AFTER the closeout
  fleets finish (a mid-closeout 10th provider would fork fleet coverage). Then: a
  **single-provider all-topics research sweep** (temp-pilot-roster technique; this
  milestone is the standing argument for the `claudine sequence` per-item selector) →
  evaluation subagents → generate → behavior half → wired provider.
  ► **CHECKPOINT H3 (Ken):** end-to-end retro; this is the spec's Goal-1 acceptance
  test.

## Phase I — lock-in

1. CLI drift guard (design/pipeline-dry.md): extended patterns, blanket exemptions,
   grandfather-with-burn-down allow-list seeded from the (now shrunken) A2 inventory;
   back-port extended patterns to the lib guard.
   ► **CHECKPOINT I (Ken):** allow-list + burn-down tags review.
2. Close the spec: Open Questions 1–5 stamped with their design-doc rulings; DRAFT
   status lifted; `docs/topics/provider-metadata.md` refreshed against reality
   (its inventory is already superseded by A2's mechanical one).
3. Drift maintenance: claudine skill (architecture + module map sections), CLAUDE.md
   workspace notes (new crates catalog-types/gen), per-area docs/dependencies.md.

## Done when

Generated `data.rs` is the only source of provider data (drift-tested in CI); the
legacy tree is gone; static-fact WrapperProfile overrides are zero; signals flow
through compiled detection tables + one sink with `signals check` in CI; the model
artifact boundary is live with plan-endpoint identity; pipeline output flows through
functional render components with DisplayPolicy data; both drift guards hold the
line; and three providers — Kilo, Pi, Antigravity — entered production through the
new process, each with a completed retro.
