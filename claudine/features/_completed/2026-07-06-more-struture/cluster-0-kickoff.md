# Kickoff — finish Cluster 0 (caps) of the `more-struture` epic

> Paste into a fresh session, or say: "read
> `claudine/features/2026-07-06-more-struture/cluster-0-kickoff.md` and begin."
> Written 2026-07-08 at a clean checkpoint; Clusters 1–3 + the Cluster-0
> foundation are committed and green.

## Load first

- Use the **`claudine`** agent skill before anything. Pull in `darkmatter`,
  `biscuit-file`, `sniff` as needed. You are in the **claudine** package area.
- Read, in order: `features/2026-07-06-more-struture/audit.md` (the ruled
  classification), `cap-model-design.md` (the ratified cap model + `ExtractStrategy`
  mechanism), `implementation-plan.md` (clusters + what's DONE).

## Where things stand (committed, do not redo)

The `more-struture` checkpoint is complete — Ken ruled all 5 categories. **Four
pieces are committed and green** (`just test`/`lint`/`test-gen`/`signals-check`):

- **Cluster 1** — `SignalEvent` widenings: `TokensConsumed`
  +cache_read/cache_write/cost/reasoning; `GenerationRetried`/`TurnLimitReached`
  +message; `NoFunds` +top_up_url.
- **Cluster 2** — `ObservedSignal.context: BTreeMap` holds every extraction field
  with no variant slot (was `debug!`-dropped); `build_event` →
  `(SignalEvent, SignalContext)`; hub threads it via `emit_with_context`.
- **Cluster 3** — qwen `QwenLoopType` graduated to a catalog-types enum;
  `bespoke.rs` consumes `QwenLoopType::iter()`; mirror test vs qwen.md. **goose was
  dropped** (its credits-exhausted is already detected via the stronger
  `creditsExhausted` notification — see the audit's Cat 2 correction).
- **Cluster 0 — FOUNDATION (`ExtractStrategy`)** — `ExtractionSpec.path: &str` →
  `source: ExtractStrategy` = `Path | Regex{path,pattern} | StartStopTokens{path,
  start,stop} | Literal(value)`. Engine `resolve_source` (in
  `lib/src/signals/event_builder.rs`) dispatches all four. `gen/src/signals.rs`
  reads `path:` (sugar) / `literal:` / `regex:` / `start:`+`stop:` and emits the
  variant; all 82 records regenerated `path:` → `ExtractStrategy::Path(...)`, gen
  byte-clean, **zero behavior change**. `Literal` is the key new capability — it
  pins a research-declared constant.

## Your job — the REST of Cluster 0 (the cap reshape)

The cap model was ratified with Ken (see `cap-model-design.md`). It is
**provider-neutral**; per-provider research declares how to extract the axes via
`ExtractStrategy` (now available). Three attributes: **model** (`All` |
`Specific(model)`), **timeframe** (reset-window duration), **remaining** (`Option`
percent — `None` when unknown, `Some(%)` known; capped ⇒ `Some(0)`). Overage is
NOT modeled (stays in `message`). Two signals stay distinct: `UsageCapApproaching`
(informative) vs `UsageCapped` (must-handle; unhandled ⇒ non-interactive session
terminates).

### 0a — cap event reshape + tz fix (do first)

1. **catalog-types** (`catalog-types/src/signal.rs`): add
   `enum CapScope { All, Specific(&'static str or String) }` (Serialize, matches
   the SignalEvent derive set). Reshape both variants:
   ```
   UsageCapApproaching { model: CapScope, timeframe: Option<Quantity /*DurationSecs*/>,
                         remaining: Option<Quantity /*Percent*/>, resets_at, message }
   UsageCapped         { model: CapScope, timeframe: Option<Quantity>,
                         remaining: Option<Quantity>, lifts_at, message }
   ```
   **Retire `UsageWindow`** (and its `SevenDayOpus` wart) — remove the enum, the
   re-export, `take_window`. Reuse `Quantity{Percent}` / `Quantity{DurationSecs}`
   rather than new Percent/Duration types (Rule 2) unless a 0–100 invariant needs
   enforcing.
2. **event_builder** (`lib/src/signals/event_builder.rs`): rewrite the two cap
   arms of `build_from_fields` to take `model`/`timeframe`/`remaining`. `model`
   comes from a `model` extraction (Literal in claude's records, else `All`); add a
   `take_cap_scope` helper. Delete `take_window`. Remaining convention above.
3. **tz fix**: `parse_iso8601` currently treats a naive `zone: local` timestamp as
   UTC (an admitted approximation). Honor `zone: local` → convert host-local → UTC
   at ingest. Storage stays `DateTime<Utc>`. Verify the `logs`/report path renders
   UTC → host-local for display (add if missing). Storage always UTC.
4. **records** (research → regen): replace claude's single `window <-
   rate_limit_info.rateLimitType` extraction with **one record per rateLimitType
   token**, each pinning `model: Literal(...)` + `timeframe: Literal(...)` and
   extracting `remaining` where a fixture carries it (**no fabrication** — only
   what fixtures/harvest evidence). `"usage"` → `model: All`, timeframe unknown.
   Add `window`/`remaining`/`timeframe`/`model` extraction to `UsageCapped` records
   only where evidenced. Update `docs/research/signals/_schema.yaml` extraction
   authoring form to allow `literal:`/`regex:`/`start:`+`stop:` (currently `path`
   only). Regenerate (`claudine-gen generate --yes`), keep gen byte-clean.
5. **consumers** — update every `UsageWindow` / cap-variant site (grep both):
   `lib/src/signals/project.rs` (the `RateLimitInfo` bridge), `stream/reporting.rs`
   / `reporting/mod.rs`, `render` `DisplayPolicy` consumers,
   `stream/logs/opencode/reasoning.rs` (constructs `UsageCapped`),
   `semantics_tests.rs`, `gen/src/signals.rs`. Add semantics tests proving a claude
   cap record now yields `{model: Opus, timeframe: 7d}`.

### 0b — Layer A cap-policy catalog (1:M provider metadata)

- New research surface: a `cap_policies:` list per provider (`{model, timeframe}`)
  — decide signals-topic vs facts; add the `_schema.yaml` sidecar entry.
- `ProviderInfo` gains `cap_policies: &[CapPolicy]`; `gen/src/emit.rs` emits it;
  **one-change gen discipline**: registry + emit + regen-all `data.rs` +
  `catalog.json` + BOTH field-list guards, together. Never hand-edit a `data.rs`.
- Populate Claude Max first (`(All,5h)`,`(All,7d)`,`(Opus,7d)`,`(Sonnet,7d)`);
  others as research supplies. Re-bless dispatch inventory if governed sites move.

## Conventions (repo rules)

- Never `cargo fmt`. Never commit unless told. Subagents never commit. US English.
- Tests from `claudine/`: `just test`, `just lint`, `just test-gen`,
  `just signals-check`. `cd` is zoxide-shimmed — if not in `claudine/`, use
  `just -f claudine/justfile -d claudine <recipe>`.
- `catalog-types`/`ProviderInfo` change → one-change gen discipline. Any signal
  reshape → catalog-types + event_builder + records + sidecar + regen +
  `signals check` green, one change per cluster.
- Re-bless after changes: dispatch inventory
  (`CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`),
  skill `hash:` (`md hash --save`) only if a hashed skill doc is edited.
- Known host flakes (ignore ONLY on exact match): 3 `level2_tmux_*_chooser_detail`;
  `argv_normalization` handle-leak; sniff `detect_area_errors_when_not_in_repo`;
  `loop_engine::…rate_limit_pause…` and `config_tui…build_modal_hotkey_line…`
  (timing flakes seen this session).

## Success

- `UsageWindow` gone; both cap variants carry `{model: CapScope, timeframe,
  remaining%}`; claude cap records decompose via `Literal`; tz stores UTC / renders
  local; Layer A cap-policy catalog generated and populated for Claude Max.
- `just test` / `lint` / `test-gen` / `signals-check` green (only known flakes
  excepted); gen byte-clean; artifacts re-blessed.
- Move the feature to `_completed` when the whole epic is green, and update the
  `claudine` skill's signals/cap sections + `docs/topics` signals doc.
