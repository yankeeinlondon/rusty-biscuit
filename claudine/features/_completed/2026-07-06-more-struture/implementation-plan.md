# `more-struture` — Implementation Plan

> All five audit categories ruled by Ken (2026-07-08) — see
> [`audit.md`](audit.md) and [`cap-model-design.md`](cap-model-design.md). This
> plan sequences the approved work into atomic clusters, each landing green
> (`just test` / `lint` / `test-gen` / `signals-check`, known host flakes
> excepted) before the next starts. No parallel mapping mechanism; every
> catalog-types/gen change follows the one-change discipline.

## Ruled outcomes (recap)

| Cat | Ruling |
|---|---|
| 1A + 1C | Widen `TokensConsumed` (+cache_read/cache_write/cost/reasoning); `GenerationRetried`/`TurnLimitReached` (+message); `NoFunds` (+top_up_url). |
| 1B | Open `context: BTreeMap` on `ObservedSignal` (lib-side); remove the `debug!`-drop. |
| 2 | goose `GooseProviderError` enum + sidecar + 2b signal mappings (`Credits exhausted → NoFunds` + triage); qwen `QwenLoopType` enum + sidecar (2a only). |
| 3 → caps | Provider-neutral cap redesign, both layers, two signals, tz rule. |
| 4 | Leave as prose (no action). |
| 5 | Defer (no action). |

## Progress

- **Cluster 1 — DONE (2026-07-08).** `TokensConsumed` +cache_read/cache_write/
  cost/reasoning; `GenerationRetried`/`TurnLimitReached` +message; `NoFunds`
  +top_up_url. All construction sites updated; new semantics test proves
  cache_read+cost now land. Full suite (3447) + signals-check + gen byte-identity
  + clippy green.
- **Cluster 2 — DONE (2026-07-08).** `build_event` returns `(SignalEvent,
  SignalContext)`; `ObservedSignal` gained `context: BTreeMap` (serialized into
  JSONL logs, `skip_if_empty`); `debug!`-drop removed; `emit_with_context` +
  `observe_with_context` thread it through the hub; first-event-wins. New test
  proves provider/model ride as context. 3448 tests + clippy + signals-check
  green.

- **Cluster 3 — DONE (2026-07-08), scope corrected.** Implementation-time
  verification found goose's `Credits exhausted → NoFunds` is **already** detected
  via the stronger dedicated `creditsExhausted` notification, so goose 2b did not
  exist; the other 8 prefixes have no `SignalKind` (out of scope); goose 2a was
  documentation-only. **Ken re-ruled: drop goose, do qwen 2a only.** `QwenLoopType`
  (10) graduated to a catalog-types enum; `bespoke.rs` consumes it via
  `QwenLoopType::iter()` (hard-coded array removed); mirror test ties the enum to
  the qwen.md `vocabulary:`. 3450 tests + clippy + gen check green.

- **Cluster 0 — Foundation (`ExtractStrategy`) DONE (2026-07-08).** `ExtractionSpec`
  generalized from `path: &str` to `source: ExtractStrategy` (Path | Regex |
  StartStopTokens | Literal) in catalog-types. Engine `resolve_source` dispatches
  all four (Path=walk, Literal=constant, Regex=capture-group-1 over path string,
  StartStopTokens=substring). `gen` reads `path:` (sugar) / `literal:` / `regex:` /
  `start:`+`stop:` authoring and emits the variant; all 82 records regenerated
  `path:` → `ExtractStrategy::Path(...)`, gen byte-clean, zero behavior change. CLI
  `signals check` diagnostic updated. 4 new strategy unit tests. Closes two Cat 5
  grammar gaps (Regex/StartStopTokens now research-usable).

- **Cluster 0 — 0a (cap event reshape + tz fix) DONE (2026-07-09).**
  `UsageWindow` (and its `SevenDayOpus` wart) retired. Both cap variants reshaped
  to `{ model: CapScope, timeframe: Option<Quantity/*DurationSecs*/>, remaining:
  Option<Quantity/*Percent*/>, resets_at|lifts_at, message }`. New `CapScope { All,
  Specific(Cow<'static,str>) }` (serializes to a bare token; `Cow` unifies the
  runtime owned token with static catalog `&'static str`). `event_builder` grew
  `take_cap_scope`, dropped `take_window`; capped ⇒ `remaining Some(0%)` default,
  approaching ⇒ `None`. **Claude cap records:** the two status-matched approaching
  records replaced by one record per `rateLimitType` SDK token (usage, seven_day,
  five_hour, seven_day_opus, seven_day_sonnet), each decomposing the combined token
  into `model`/`timeframe` via `ExtractStrategy::Literal` — the ratified claude
  mechanism (cap-model-design.md). 3 synthetic fixtures (five_hour/opus/sonnet,
  `docs_example` provenance). **tz fix:** `parse_iso8601` now honors `zone: local`
  (host-local → UTC at ingest); storage stays UTC; no report renders cap reset
  times so no display-side conversion was missing. Schema sidecar extended for
  `literal:`/`regex:`/`start:`+`stop:` authoring. Consumers (project bridge,
  opencode reasoning UsageCapped, semantics tests, gen validation tests) updated.
  gen byte-clean; signals check 85/85/0.

- **Cluster 0 — 0b (Layer A cap-policy catalog) DONE (2026-07-09).** New
  `CapPolicy { model: CapScope, timeframe: Quantity }` in catalog-types.
  `ProviderInfo` gained `cap_policies: &'static [CapPolicy]` (a **Facts** field,
  mirroring `billing_models`; sourced from `docs/providers/facts/<slug>.yaml`
  `cap_policies: [{model, timeframe_secs}]`). Full one-change gen discipline:
  registry entry + `Coercion::CapPolicyRecords` + `emit::cap_policies` +
  regen-all `data.rs` + `catalog.json` + both field-list guards (42→43 fields) +
  dispatch-inventory re-bless (pure line shifts, no new dispatch). Claude Max
  populated: `(All,5h) (All,7d) (Opus,7d) (Sonnet,7d)`; other 9 providers `&[]`
  pending research. `test`/`lint`/`test-gen`/`signals-check` green.

## Clusters and sequencing

Order is chosen so shared machinery (`event_builder`, `ObservedSignal`) is
touched in increasing-blast-radius order, wins land early, and the largest
change (caps) goes last with the regime already exercised.

### Cluster 1 — Cat 1 variant widenings *(smallest, self-contained; first)*

Purely additive optional fields on frozen variants; each field is **already
resolved** by existing records, so this just adds landing slots.

- `catalog-types/src/signal.rs`: `TokensConsumed` += `cache_read`, `cache_write`,
  `cost`, `reasoning` (all `Option<Quantity>`); `GenerationRetried` += `message:
  Option<String>`; `TurnLimitReached` += `message: Option<String>`; `NoFunds` +=
  `top_up_url: Option<String>`.
- `lib/src/signals/event_builder.rs`: add the `take_quantity`/`take_string` calls
  in the matching `build_from_fields` arms.
- Regen (`data.rs`/records unaffected — extraction fields already exist), run
  `signals check`; add/extend semantics tests proving the fields now populate.
- Regime: catalog-types + event_builder + mirror/semantics test + regen +
  `signals check` green.

### Cluster 2 — Cat 1B context map *(lib-only; second)*

- `lib/src/signals/event_builder.rs`: `build_event` returns `(Option<SignalEvent>,
  BTreeMap<&'static str, ResolvedValue|String>)` (leftover), instead of dropping
  to `debug!`.
- `lib/src/signals/sink.rs`: `ObservedSignal` += `context: BTreeMap<…>`; `emit`
  carries it through (fold policy: first-event-wins keeps the first context, or
  merge — decide during impl; first-wins matches existing `occurrences` policy).
- Remove the `debug!("…no SignalEvent slot…")` drop; keep a `warn!` only for the
  genuinely-unmapped-required case (already separate).
- Reporting (`reporting`/`logs`): optionally surface `context` (at least `model`).
  Read-out by name; no typed key registry.
- Regime: lib change + sink/projection tests + `just test` green. No catalog-types
  change.

### Cluster 3 — Cat 2 vocabulary enums *(independent; third)*

- **qwen (2a):** new `catalog-types` `QwenLoopType` (10 members) + sidecar YAML in
  the signals research schema + mirror test (enum ⟺ sidecar). Rewire
  `bespoke.rs::QWEN_LOOP_TYPES` to iterate the enum. No `RunawayRepetition` field
  change.
- **goose (2a + 2b):** new `catalog-types` `GooseProviderError` (12) + sidecar +
  mirror test. Rewire the 3 existing declarative substring records to the enum
  source. **2b:** add declarative record(s) for the signal-worthy unmapped members
  — `Credits exhausted → NoFunds` first; triage the other 8 (`Context length
  exceeded`, `Network error`, `Request failed`, `Execution error`, `Usage data
  error`, `Unsupported operation`, `Endpoint not found (404)`, `Provider refused
  request`) for signal fit, mapping the ones that fit, recording the rest as
  intentional non-signals.
- Regime (per vocabulary): catalog-types enum + sidecar + mirror test + fleet doc
  edits + regen + `signals check` green, one change per vocabulary.

### Cluster 0 — Caps *(largest, critical; last so the regime is warm)*

Split into two sub-clusters (Layer B event reshape can land before Layer A
catalog; both are "now").

**0a — Layer B cap event reshape + tz fix**
- `catalog-types/src/signal.rs`: new `CapScope { All, Specific(model_id: String) }`.
  Reshape `UsageCapped` and `UsageCapApproaching` to:
  `{ model: CapScope, timeframe: Option<Quantity /*DurationSecs*/>, remaining:
  Option<Quantity /*Percent*/>, resets_at/lifts_at: Option<DateTime<Utc>>, message }`.
  Retire `UsageWindow` (and its `SevenDayOpus` wart). *(Quantity{Percent/DurationSecs}
  reused rather than new Percent/Duration types — Rule 2; revisit only if a
  0–100 invariant needs enforcing.)*
- `event_builder.rs`: `take` the new fields; `remaining` convention capped ⇒
  `Some(0%)`, approaching-with-no-count ⇒ `None`. `model` from the payload's model
  field where present, else `All`.
- **tz fix:** `parse_iso8601` must honor `zone: local` (convert host-local → UTC),
  not assume UTC. Storage stays `DateTime<Utc>`. Verify `logs`/report path renders
  UTC → host-local for display; add if missing.
- Records: add `timeframe`/`remaining`/`model` extraction paths to cap records
  **only where fixtures/harvest actually carry them** (no fabrication); the rest
  stay `None` pending harvest.
- `projection`/`RateLimitInfo` bridge, `reporting`, render `DisplayPolicy` consumers
  updated for the reshaped variants.
- Regime: catalog-types + event_builder + records + sidecar + mirror + projection
  + regen + `signals check` + `just test` green.

**0b — Layer A cap-policy catalog (1:M provider metadata)**
- New research surface: a `cap_policies:` list per provider (signals topic or a new
  facts field) — `{ model, timeframe }` entries — + `_schema.yaml` sidecar.
- `ProviderInfo` gains `cap_policies: &[CapPolicy]`; `gen/src/emit.rs` emits it;
  regen all `data.rs`; update `catalog.json`; both field-list guards.
- Populate from research where evidenced (Claude Max policies first — best
  documented); others as research supplies them.
- Regime: full one-change gen discipline (registry + emit + regen-all +
  catalog.json + both guards) + `test-gen` + dispatch inventory re-bless.

## Cross-cluster closeout

- Re-bless: dispatch inventory (`CLAUDINE_UPDATE_INVENTORY=1 …`) if governed sites
  move; skill `hash:` only if a hashed skill doc is edited.
- Drift: update the `claudine` skill (signals/cap sections), `docs/topics` signals
  doc, and this feature's docs; move the feature to `_completed` when green.
- Non-goals held: no new fleets; harvest promotion stays E6-owned; grammar
  extensions (Cat 5) untouched.

## Open implementation choices (decide in-flight, low-stakes)

1. `Quantity{Percent}` reuse vs a dedicated `Percent` newtype (lean: reuse).
2. `context` fold policy on `ObservedSignal` (lean: first-event-wins, matching
   `occurrences`).
3. Whether `CapScope::Specific` holds a raw model-id string vs a catalog-reconciled
   id (lean: raw token, flexible; reconcile at report time).
