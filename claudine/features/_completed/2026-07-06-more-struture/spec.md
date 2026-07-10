# More Structure: Graduating Unmapped Research into the Typed System

> **Status: ACTIVE (2026-07-08).** The gate — *"the 2026-07-02 provider-metadata
> spec fully complete (Phases E–I closed, provider ladder validated)"* — is met:
> that epic is RATIFIED & IMPLEMENTED and Checkpoint H3 is approved. This spec is
> now unblocked. Method §"Method (when activated)" governs: **scripted audit →
> classification → ► CHECKPOINT with Ken → implement per cluster.** No code lands
> before the classification checkpoint.

## Why

The Phase E4 signal-engine work surfaced a recurring shape: the research fleets
capture more structured data than the typed system can hold. Today that surplus
is preserved research-side (frontmatter rows, `vocabulary:` lists, `gaps:`
entries, `notes:`) and — where the engine touches it — resolved at runtime but
dropped into `debug!` logs. None of it is queryable, projectable, or drift-
guarded the way mapped data is. This spec reviews the accumulated surplus and
decides, item by item, whether mapping it explicitly improves the fleet and
mapping regime — or whether research-side is exactly where it belongs.

## Inventory of known unmapped data

### 1. Supplementary extraction fields with no `SignalEvent` slot (the E4 headline)

**36 extraction rows fleet-wide** resolve at runtime but have no variant field
to land in. The builder (`lib/src/signals/event_builder.rs`) resolves them and
logs leftovers at `debug!`; `claudine signals check` asserts they resolve. Field
names observed: `cache_read`, `cache_write`, `cost`, `reasoning`, `provider`,
`session_id`, `protocol_version`, `top_up_url`, `timestamp`, `request_id`,
kilo `attempt` / `next`, and kin. They were deliberately **kept** at E4
(ratified: research evidence + future-variant fodder, not purged). Decision
needed per field-family:

- widen an existing variant (e.g. `TokensConsumed` gaining `cache_read` /
  `cache_write` / `cost` / `reasoning` — the largest cluster), or
- a generic supplementary payload map on `ObservedSignal` (typed key registry,
  no per-field variant churn), or
- leave research-side and stop resolving at runtime (delete the debug path).

### 2. Record fields that never compile

`vocabulary:`, `locator:`, `distinguish:`, `confidence:`, `notes:` stay
research-side by design. But some `vocabulary` lists are real closed sets that
bespoke code re-encodes as string needles:

- qwen native `LoopType` (10 members) — bespoke loop-detection record
- goose `ProviderError` display prefixes (12) — substring records
- opencode five-branch classifier needles (`errors.rs`)

Candidate: graduate closed vocabularies into catalog-types enums with sidecar
mirrors, so bespoke code and records share one source.

### 3. Observed vocabulary values with no variant

- claude `rateLimitType: "usage"` maps to `UsageWindow::Unknown` today (the
  taxonomy has FiveHour/SevenDay/SevenDayOpus/Monthly). Either the taxonomy
  gains a member or the mapping is documented as intentional.

### 4. `gaps:` entries that are really structured backlog

Fleet-wide `gaps:` prose encodes machine-checkable facts the regime could track
explicitly (a `gap` record class with a disposition, mirroring the
summary-triage pattern):

- qwen exit codes 53/55/130 — no fixtures (fabrication banned); harvest (E6)
  is the designated supplier
- goose `retries_exhausted` — proven wire-invisible at commit 65eed515;
  "re-check on future goose versions" is a standing task nothing tracks
- kimi `unsupported_protocol_version` — negative match, wrapper-bespoke
- codex app-server richer vocabulary (`UsageLimitExceeded`, `ServerOverloaded`,
  `Unauthorized`, …) — not fixture-backed
- gemini dynamic `stats.models.<model>` keys — unaddressable by the restricted
  path grammar
- pi `responseModel` — needs `exists` + field-A≠B comparison (grammar gap)
- kilo step-finish `model.modelID IS NOT NULL` — presence check pushed into a
  SQL locator string

### 5. Grammar/operator gaps implied by the corpus

Collected from items above rather than new research: negation (kimi negative
match), field-to-field comparison (pi A≠B), dynamic key segments (gemini).
Each is a `MatchOp`/path-grammar extension with the usual cost (variant +
sidecar + mirror test + engine + `signals check` in one change) — only worth
it when a second producer appears.

## Method (when activated)

1. **Scripted audit, not memory:** regenerate the 36-row list mechanically
   (extraction fields × `SignalEvent` slots), sweep `vocabulary:`/`gaps:`
   across all live topics (not just signals), and diff against this inventory —
   the corpus will have moved by then (E6 harvest promotions, D-1.5 rounds).
2. **Classify each item** into: new variant field | new variant/enum member |
   new vocab enum | supplementary payload map | grammar extension | stays
   research-side (with the *why* recorded).
3. **Checkpoint with Ken** on the classification before any code.
4. **Implement through the established regime** — catalog-types change +
   sidecar + mirror test + fleet doc edits + regen + `signals check` green in
   one change per cluster. No parallel mapping mechanism.

## Non-goals

- No new research fleets (this consumes existing accumulation only).
- No runtime behavior changes ahead of the classification checkpoint.
- Harvest promotion mechanics stay owned by E6/provenance rules.
