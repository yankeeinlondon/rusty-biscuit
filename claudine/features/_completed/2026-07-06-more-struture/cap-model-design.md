# Provider-Neutral Cap Model — Design (ratified with Ken, 2026-07-08)

> Agreed during the `more-struture` classification checkpoint. This supersedes
> the audit's Cat 3 (window-taxonomy) recommendation: `UsageWindow` as a flat
> enum of vendor spellings is retired in favor of the provider-neutral model
> below. Still pre-code — captured here so the checkpoint decision is durable.

## Principles (Ken)

1. **Research discovers a provider's cap policies**, and the relationship is
   **1:M** — one provider imposes many caps (Claude Max alone: 5h-all, 7d-all,
   7d-opus, 7d-sonnet, …).
2. **The data structure is provider-neutral** — no vendor spellings baked into a
   type (`SevenDayOpus`/`Monthly` are out).
3. **A cap reduces to three attributes:**
   - **Model** — `All` (account-wide) or `Specific(model)`
   - **Timeframe** — window size before reset
   - **Remaining** — normalized to a **percent remaining** (the universal axis)
4. **Capture all cap events** — for reporting *and* lifecycle reaction.

## Two layers

### Layer A — cap-policy catalog (static, research-driven, 1:M) — **build now**

Per-provider metadata: the *set* of caps the provider imposes. Natural key
`(model, timeframe)`.

```
CapPolicy { model: CapScope, timeframe: Duration }
// Claude Max → [ (All, 5h), (All, 7d), (Opus, 7d), (Sonnet, 7d) ]
```

New provider-metadata surface, generated from research alongside
`expected_offerings`. Follows the one-change gen discipline (registry + emit +
regen + catalog.json + guards). Requires a research field for cap policies +
`_schema.yaml` sidecar.

### Layer B — cap event (runtime) — **build now**

A firing of one policy, carrying live remaining. Replaces the internals of
`UsageCapped` / `UsageCapApproaching` (the two `SignalKind`s stay — see below).

```
model:      CapScope,          // All | Specific(model_id)     — attr 1
timeframe:  Option<Duration>,  // 5h / 7d / ~monthly           — attr 2 (no enum)
remaining:  Option<Percent>,   // 0–100% remaining             — attr 3 (normalized)
resets_at:  Option<DateTime<Utc>>,  // when capacity returns (stored UTC)
message:    Option<String>,
```

- `CapScope` = `All | Specific(model_id)`. `model_id` is a provider-neutral token
  (reconcilable against the model catalog); kept flexible so a new vendor tier
  needs no code change (reliable-yet-flexible split).
- `timeframe: Duration` — every provider window maps to a duration; `(model,
  timeframe)` is the provider-neutral policy identity. No `UsageWindow` enum.
- **`remaining: Option<Percent>`** — the normalization. **`None` when the payload
  carries no numbers; `Some(%)` when known** (Ken's ruling). Convention: a fired
  `UsageCapped` ⇒ `Some(0)`; an `UsageCapApproaching` with no count ⇒ `None`
  (known-low, exact % unknown). Raw provider counters (tokens/requests) are NOT
  lost — they land in the Cat 1B supplementary payload map as context; the % is
  the typed axis every lifecycle rule branches on uniformly.

## Two signals, kept distinct (Ken's ruling)

- **`UsageCapApproaching`** — *purely informative*. Typically we just inform the
  caller. No forced handling.
- **`UsageCapped`** — *requires immediate handling*: a non-interactive session is
  **terminated immediately** if unhandled. This is the must-react terminal case.

Both carry the same Layer-B shape and are derived from the same `remaining%`;
they stay separate `SignalKind`s so lifecycle authors name them directly
(`on: usage_capped`) without a threshold expression.

## Overage / billing-state — out of the typed model

The three attributes deliberately exclude overage. It is billing state, not
cap-remaining. Overage spellings (`overage`, `seven_day_overage_included`) stay
in `message` text (or the supplementary map) — not a typed field — unless a
concrete need appears.

## Timezone handling for `resets_at` (Ken)

Rule: **research captures the source zone; storage is always UTC; user-facing
reports render in host-local tz.**

Existing infrastructure already covers most of this:

- **Research captures zone** — the signals `_schema.yaml` already *forces* a
  `zone: enum(utc,local,embedded_offset,unspecified)` on every extraction site,
  with `unspecified` flowing into `known_gaps`. ✅ Already in place; we just verify
  each `resets_at`/`lifts_at` site carries an accurate zone.
- **Storage in UTC** — `SignalEvent` reset fields are `DateTime<Utc>`, and
  `event_builder::convert` normalizes epoch/ISO inputs to UTC. ✅ Already in place.
- **The one real gap** — `parse_iso8601` currently interprets a *naive* timestamp
  declared `zone: local` **as UTC** (an admitted approximation, flagged via
  `known_gaps`). To honor the rule correctly, a `zone: local` naive timestamp
  must be converted **host-local → UTC** at ingest, not assumed UTC. This is the
  concrete correctness fix the tz requirement adds.
- **Reporting in host-local** — the reporting/render layer converts stored UTC →
  host-local for display. Verify the `logs`/report path does this (add if
  missing). Storage stays UTC regardless.

## Extraction mechanism — `ExtractStrategy` (ratified with Ken, 2026-07-08)

The decomposition of a provider's wire format into the three cap axes is
**provider-neutral and research-driven** — each provider's research declares how
to extract each axis. What differs per provider is only the *strategy*, not the
concept. To support that, `ExtractionSpec`'s source generalizes from a dot-path
to a strategy enum:

```rust
enum ExtractStrategy {
    Path(dotpath),                 // today's behavior
    Regex(re),                     // capture from a string
    StartStopTokens(start, stop),  // substring between markers
    Literal(value),                // pinned constant
}
ExtractionSpec { field, source: ExtractStrategy, unit, zone }
```

A provider's cap detection is a **set of records (match rules)**, each mapping
every needed property to a strategy:

- **Separate fields** — one record: `model: Path("model")`, `timeframe:
  Path("window")`, `remaining: Path("remaining_pct")`.
- **Combined token (claude)** — one record per token: match
  `rateLimitType == "seven_day_opus"` → `model: Literal("opus")`, `timeframe:
  Literal("7d")`, `remaining: Path(...)`. `Literal` pins the decomposition in
  research, not code.

This generalizes extraction for **all** records (not just caps) and closes two
Cat 5 grammar gaps for free (`Regex`, `StartStopTokens` become research-usable
everywhere).

**Sequencing (ratified):** land the `ExtractStrategy` generalization FIRST as its
own green step — all existing records regenerate `path:` → `Path(...)` with zero
behavior change — then the cap variant reshape + Layer A ride on top.

## Scope delta vs the original audit

- **Retires** Cat 3 (window-taxonomy widening) — replaced by `timeframe: Duration`.
- **Absorbs** Cat 1B's `provider`/`model` on cap signals into the typed
  `CapScope`; other-signal context still uses the payload map (open ruling).
- **Adds** Layer A (the 1:M cap-policy catalog) as new provider metadata.
- **Adds** the `zone: local` → UTC ingest fix + reporting-side host-local render.
- **Keeps** Cat 1A (TokensConsumed enrichment) and Cat 1C (message/top_up_url) as
  already approved.
