# `more-struture` — Scripted Audit + Classification (2026-07-08)

> Produced per the spec's §"Method (when activated)" step 1–2. Mechanical, not
> from memory: the extraction × slot list was regenerated from
> `lib/src/signals/generated.rs` + `event_builder.rs`; `vocabulary:`/`gaps:` were
> swept across all live research topics. This is the **CHECKPOINT input** — no
> code lands until Ken rules on the classification below.

## How this was generated

- **Category 1** — `scratchpad/audit_extractions.py` brace-parses every
  `DetectionRecord` in `generated.rs`, reads its `ExtractionSpec` field names,
  and subtracts the fields `build_from_fields` consumes per `SignalKind`
  (transcribed from `event_builder.rs`). The remainder is the unmapped set.
- **Categories 2–5** — `vocabulary:`/`gaps:` grep sweep across all 22 live
  research topic dirs, plus a read of the bespoke re-encoders
  (`signals/bespoke.rs`, `stream/logs/opencode/errors.rs`).

## Corpus has moved since the spec was written

| Spec (2026-07-06) | Reality (2026-07-08) |
|---|---|
| "36 extraction rows fleet-wide" | **36 records** carry ≥1 unmapped field, but **71 field-instances / 30 distinct field names** (E6 harvest + ladder providers grew the tail). |
| Field names listed: cache_read, cache_write, cost, reasoning, provider, session_id, protocol_version, top_up_url, timestamp, request_id, kilo attempt/next | All still present; **new** since spec: `model`, `reasoning_output`, `uncached_input`, `cache_creation`, `input_other`, `context_tokens`, `max_context_tokens`, `message_id`, `tool_call_id`, `method`, `status`, `variant`, `occurred_at`, `created_at`, `cwd`, `max_attempts`, `error_type`. |

---

## Category 1 — 30 unmapped extraction fields (71 instances, 36 records)

Grouped into clusters by shape. Frequency in parentheses.

### 1A. TokensConsumed enrichment — the headline cluster (largest, cleanest)
`cache_read` (10) · `cache_write` (5) · `cost` (5) · `reasoning` (2) ·
`reasoning_output` (1) · `uncached_input` (1) · `cache_creation` (1) ·
`input_other` (1) · `context_tokens` (1) · `max_context_tokens` (1)

13 `TokensConsumed` records resolve token-accounting fields with nowhere to land.
`TokensConsumed` today = `{ input, output, total }`.

**Recommendation: widen `TokensConsumed`** with optional `cache_read`,
`cache_write`, `cost`, `reasoning` (the 4 the spec named; ≥5 producers each or
lifecycle-actionable cost). Leave the 5 long-tail singletons
(`uncached_input`/`cache_creation`/`input_other`/`context_tokens`/`max_context_tokens`/`reasoning_output`)
research-side — one provider each, no second producer.

### 1B. Cross-cutting diagnostic context
`provider` (9) · `model` (5) · `session_id` (4)

Appear on `stderr_promoted` limit records (rate_limited / provider_overloaded /
usage_capped / retries_exhausted) and `model_resolved`-init records. "Which
provider/model/session hit this" — not the signal's semantic core, but useful.

**Recommendation: this is the spec's `ObservedSignal` supplementary payload-map
candidate.** A single typed key registry (not 4× variant churn). Alternative:
stop resolving them (delete the debug path). **Ken ruling needed** — payload-map
vs stop-resolving.

### 1C. Genuine drops — variant has no slot for a field it resolves
`message` (3) on `GenerationRetried` + `TurnLimitReached` · `top_up_url` (1) on `NoFunds`

These variants have **no** `message`/`top_up_url` field, yet the record resolves
one → silently dropped. All three are lifecycle-actionable (a `failure.message`
or "add credits here" URL).

**Recommendation: widen** `GenerationRetried` + `TurnLimitReached` with optional
`message`, and `NoFunds` with optional `top_up_url`. Cheap, high-signal.

### 1D. Retry-context (duplicate home exists)
`attempt` (3) · `next` (2) · `max_attempts` (1) · `error_type` (1)

On `RateLimited`/`ProviderOverloaded` stream-retry records. **`GenerationRetried`
already carries `attempt`/`max_attempts`/`error_type`** — these are the same wire
retry seen through a limit lens.

**Recommendation: leave research-side.** `GenerationRetried` is the proper home;
widening the limit variants duplicates it. No action.

### 1E. Correlation IDs / timestamps / misc singletons
`request_id` (3) · `timestamp` (2) · `tool_call_id` (1) · `method` (1) ·
`message_id` (1) · `occurred_at` (1) · `created_at` (1) · `cwd` (1) ·
`protocol_version` (1) · `status` (1) · `variant` (1)

Correlation IDs (not lifecycle-actionable), event-local timestamps (the sink
stamps its own), and one-off secondary labels.

**Recommendation: leave research-side; drop the `debug!` resolution** for the
pure-noise ones (correlation IDs, timestamps) to shrink the leftover path. Keep
them as documented extraction evidence in the research docs.

---

## Category 2 — closed vocabularies re-encoded by bespoke code

| Vocabulary | Members | Where re-encoded | Shares with records? |
|---|---|---|---|
| qwen `LoopType` | 10 | `QWEN_LOOP_TYPES` array, `bespoke.rs` | record `stream-runaway_repetition-result-loop` (prose only) |
| goose `ProviderError` prefixes | 12 | `vocabulary:` in `goose.md` (3 records) | 3 declarative substring records share the list |
| opencode classifier needles | ~5 fuzzy | `classify_llm_failure`, `errors.rs` | dialect substrings (AuthenticationError, AI_RetryError, overload, cap needles, 429) |
| claude `error.type` | 10 | declarative `match_value` strings in `claude.md` | already declarative — no bespoke duplication |
| kimi wire protocol versions | set | `SUPPORTED_WIRE_PROTOCOL_VERSIONS`, `bespoke.rs` | negative match (no record) |

**Recommendation: graduate `qwen LoopType` and `goose ProviderError` to
catalog-types enums with sidecar mirrors** — both are clean closed sets with a
single canonical source that bespoke code + records could share. **Keep opencode
needles bespoke** (fuzzy provider dialects, not a clean closed set — enumifying
is false precision). **Keep claude `error.type` as-is** (already declarative
strings; no bespoke duplication to unify). **Ken ruling needed** on scope.

---

## Category 3 — observed vocab with no variant — **SUPERSEDED**

> Ken's cap redesign (see [`cap-model-design.md`](cap-model-design.md)) retires
> the flat `UsageWindow` enum entirely. Timeframe becomes a provider-neutral
> `Duration`, model becomes `CapScope`, remaining normalizes to `Option<Percent>`,
> and a new 1:M cap-policy catalog is added. The original Cat 3 text is kept below
> for provenance only.


- claude `rateLimitType`: observed `usage`, `seven_day`; SDK declares `five_hour`,
  `seven_day`, `seven_day_opus`, `seven_day_sonnet`, `seven_day_overage_included`,
  `overage`. `UsageWindow` = `{ FiveHour, SevenDay, SevenDayOpus, Monthly, Unknown }`.
  → `usage`, `seven_day_sonnet`, `seven_day_overage_included`, `overage` all
  collapse to `Unknown`. (Note: `Monthly` is in the taxonomy but **not** claude's
  SDK list — sourced from another provider.)

**Recommendation: document as intentional.** `Unknown` is the deliberate catch-all
(`take_window` already comments this); `usage` is not a window and the
sonnet/overage members are low-value. No taxonomy change — add a one-line note in
`signal.rs` enumerating the known-collapsed SDK spellings so it's not re-litigated.

---

## Category 4 — `gaps:` as structured backlog

Swept all topics (signals + acp/hooks/mcp/resume/plugins/non-interactive-sessions/
system-prompt/agent-permissions). Findings:

- The overwhelming majority of `gaps:` entries are **methodology notes**
  ("not exercised locally", "no schema published", "Windows path inferred") — not
  machine-actionable.
- A **small machine-checkable subset** exists: version-gated re-checks (goose
  `retries_exhausted` wire-invisible @65eed515 "re-check on future versions"; qwen
  `TodoCreated` absent in v0.15.6), MCP protocol-version dates (fleet-wide,
  uniform), and the signals-specific gaps (qwen exit 53/55/130 harvest-supplied,
  kimi `unsupported_protocol_version`, gemini dynamic `stats.models.<model>`, pi
  `responseModel`, kilo step-finish presence).
- **agent-permissions structural quirk:** its `gaps:` is nested under
  `policy_engine:` — a top-level parser misses all 10 files.

**Recommendation: do NOT build a typed compiled gap-record class.** It is a whole
new subsystem for data that is 90% prose. Instead, if tracking is wanted, add a
lightweight **`recheck_on_version:` optional field to the existing research
frontmatter schema** (`_schema.yaml` sidecars) for the handful of genuine
version-gated re-checks — checkable by the existing sidecar validator, no compiled
surface. **Ken ruling needed** — lightweight frontmatter field vs leave entirely.

---

## Category 5 — grammar/operator gaps

| Gap | Provider | Current handling |
|---|---|---|
| negation (negative match) | kimi `unsupported_protocol_version` | bespoke `KimiProtocolWindow` |
| field-to-field compare (A≠B) | pi `responseModel` | (gap, unimplemented) |
| dynamic key segments | gemini `stats.models.<model>` | (gap, unaddressable) |
| presence / `IS NOT NULL` | kilo step-finish `model.modelID` | **`exists` MatchOp already added in Phase H** |

**Recommendation: leave as bespoke; no grammar extensions now.** The spec's own
rule — "only worth it when a second producer appears" — holds: each is a
single-provider one-off. `exists` already landed (Phase H) because it had the
clearest case. Document the trigger (2nd producer) and stop. Overlaps B5
(antigravity app-log classifier), which is itself a bespoke one-off.

---

## Steer applied — Ken, 2026-07-08

Ken's three principles reshape the recommendations above:

1. **Unused research is a clear miss.** Research the fleet captured but the type
   system drops should be held *somewhere*, not debug-logged into the void.
2. **Add precision where research enables it.** Where a resolved value could
   sharpen a signal, graduate it.
3. **Subscription caps are critical + sensitive.** Cap signals get priority.

### Cap-detection precision gap (elevated to top priority)

A targeted re-audit of the `UsageCap*` records found the most cap-relevant miss:

- **All 8 `UsageCapped` records extract NO `window`** — when a cap actually hits,
  the window is always `Unknown`. Only the softer `UsageCapApproaching` (2
  records) extracts a window, and it collapses `usage` / `seven_day_sonnet` /
  `seven_day_overage_included` / `overage` to `Unknown` (Cat 3).
- **2 `stderr_promoted` `UsageCapped` records resolve `provider` + `model`** (which
  subscription/model capped) **and drop them** into the debug path (Cat 1B).

Both are "critical research, currently unused." → cap work is Cluster 0 below.

### Rulings recorded (Ken, 2026-07-08)

- **Cat 1 (1A + 1C): APPROVED** — widen `TokensConsumed` (+cache_read/cache_write/
  cost/reasoning); fix genuine drops (`GenerationRetried`/`TurnLimitReached`
  +message, `NoFunds` +top_up_url).
- **Cat 3 → caps redesign: RATIFIED** — see [`cap-model-design.md`](cap-model-design.md)
  (provider-neutral, both layers, two signals, tz rule).
- **Cat 1B: APPROVED option (A)** — an **open `context: BTreeMap` on
  `ObservedSignal`** (lib-side only; frozen `SignalEvent` untouched). `build_event`
  returns the leftover map; the `debug!`-drop path is removed. High-value keys
  (starting with `model`) read out by name; graduate to typed fields only when one
  earns it. No typed key registry.

### Cat 2 CORRECTION (2026-07-08, during implementation)

Implementation-time verification overturned the goose 2b premise:

- **goose `Credits exhausted → NoFunds` is ALREADY detected** — record
  `stream-no_funds-credits_exhausted` matches the dedicated `creditsExhausted`
  system notification (`ProviderError::CreditsExhausted`), fixture-backed and
  signals-check-green. The doc's `distinguish` note states this is deliberately
  *stronger* than matching the "Credits exhausted" provider-error prose. So the
  "missed signal" does not exist.
- **The other 8 unmapped ProviderError prefixes** (context-length, network,
  request-failed, execution, usage-data, unsupported-op, 404, provider-refused)
  have **no corresponding `SignalKind`** — mapping them needs new taxonomy
  members (out of scope). No 2b work remains.
- **goose 2a** (graduate the 12-member vocabulary to an enum) is now weaker: only
  3 of 12 are matched, and via full match strings, not a bespoke re-encoding of
  the closed set — the vocabulary is documentation, not duplicated code. Low value.
- **qwen 2a stands** — `QWEN_LOOP_TYPES` (10) in `bespoke.rs` IS a genuine bespoke
  re-encoding; graduating it to a catalog-types enum removes real duplication.

Net: Cluster 3 reduces to **qwen 2a**. Pending Ken's re-ruling on goose.

### Cat 2 ruling (Ken, 2026-07-08) — SUPERSEDED by the correction above

- **goose: 2a + 2b** — graduate `GooseProviderError` (12) to a catalog-types enum
  with sidecar mirror, AND add the signal-worthy unmapped members (`Credits
  exhausted → NoFunds`; triage the other 8 unmapped prefixes for signal fit).
- **qwen: 2a now** — graduate `QwenLoopType` (10) to an enum with sidecar mirror
  (drift-guard / single source). **No** loop-type preservation on
  `RunawayRepetition` (no differential action; would be speculative surface).

### Cat 4 + Cat 5 rulings (Ken, 2026-07-08)

- **Cat 4: (B) leave as prose.** `gaps:` document *absence*, not dropped usable
  data — the "unused research" principle doesn't apply. The actionable slice
  (version-gated re-checks) is tiny; the annotation field alone has no consumer,
  and the useful version-check report is a separate future feature. No action this
  pass.
- **Cat 5: defer (no action).** Grammar/operator gaps (negation, field≠field,
  dynamic keys) stay bespoke until a 2nd producer appears; `exists` already landed
  in Phase H. Documented trigger: revisit on a second producer.

### CHECKPOINT COMPLETE — all categories ruled (Ken, 2026-07-08)

Implementation may proceed per [`implementation-plan.md`](implementation-plan.md).

### Revised recommendations under the steer

- **Cat 1B → supplementary payload map (CONFIRM):** adopt the `ObservedSignal`
  typed key-registry map as the universal "hold everything resolved" surface. It
  holds `provider`/`model`/`session_id` **and subsumes the 1E singletons** — so
  *no resolved field is ever dropped again*. Directly serves principle 1. For cap
  signals, `provider`/`model` become queryable (which subscription capped).
- **Cat 3 → WIDEN `UsageWindow` (REVERSED, now cap-critical):** was "document as
  intentional"; under principles 2+3 it becomes **add the missing SDK window
  spellings** (`SevenDaySonnet`, `SevenDayOverageIncluded`, `Overage`) so cap
  windows are precisely identified. Plus close the research-side gap: **add
  `window` extraction to the `UsageCapped` records** wherever the payload carries
  it (harvest/fixture permitting — no fabrication). The claude `"usage"` spelling
  is a generic marker, not a window → keep it `Unknown` with a documented note, or
  add a `Subscription` member (sub-decision for Ken).
- **Cat 2 → graduate qwen `LoopType` + goose `ProviderError` (unchanged):**
  precision, one shared source. opencode needles stay bespoke (fuzzy dialects,
  enumifying = false precision); claude `error.type` already declarative.
- **Cat 4 → lightweight `recheck_on_version:` sidecar field (unchanged, low
  priority):** `gaps:` are *known limitations*, the opposite of usable data;
  principle 1 doesn't apply. Only the version-gated re-checks are actionable.
- **Cat 5 → defer (unchanged):** none are cap-critical; wait for a 2nd producer.

## Proposed implementation clusters (only if approved)

- **Cluster 0 (caps — critical, do first):** widen `UsageWindow`
  (+SevenDaySonnet/SevenDayOverageIncluded/Overage); add `window` extraction to
  `UsageCapped` records where payloads carry it; hold `provider`/`model` on cap
  signals (via the Cluster II payload map). Everything cap-touching in one pass.


Each is one atomic change through the established regime (catalog-types + sidecar
+ mirror test + fleet doc + regen + `signals check` green):

- **Cluster I** (1A + 1C): widen `TokensConsumed` (+cache_read/cache_write/cost/
  reasoning), `GenerationRetried`/`TurnLimitReached` (+message), `NoFunds`
  (+top_up_url). One catalog-types change + event_builder arms + record field
  wiring + regen.
- **Cluster II** (1B): `ObservedSignal` supplementary payload map **OR** stop
  resolving provider/model/session_id — pending ruling.
- **Cluster III** (Cat 2): qwen `LoopType` + goose `ProviderError` → catalog-types
  enums + sidecar mirrors.
- **Cluster IV** (Cat 3): doc-only note in `signal.rs`; (Cat 4) optional
  `recheck_on_version:` sidecar field — pending ruling.
- **No cluster** for 1D, 1E, Cat 5 — stays research-side (with the *why* recorded
  above).
