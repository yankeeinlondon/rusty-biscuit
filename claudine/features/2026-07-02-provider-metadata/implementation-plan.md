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

> **Checkpoint A rulings (2026-07-04, Ken):** (1) override files are **field-keyed**
> `{value, reason}` (spec's topic-sectioned mirror retired; spec.md amended);
> (2) the declarative `RegistryEntry` + named-`Coercion`-enum registry shape is
> ratified as the Phase B pattern; (3) inventory records carry a derived
> `dispatch_class: conditional | reference` (pattern-set v2) — Phase D's disposition
> table and Phase I's guard seed filter on it; (4) coercions must never drop input
> silently — skips are collected on `Generation` and printed by the generate/check
> reports (compound env-var sites were the motivating case), and the agent-models
> fleet prompt now mandates one env var per `model_selection` record for the next
> closeout refresh.

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

   > **Checkpoint B rulings (2026-07-04, Ken):** (1) **Roo Code is fully removed**
   > (enum variant, roster entry, research documents, facts file) rather than kept
   > dormant; the roster gains a `skip_research: true` entry flag for future
   > keep-identity-but-skip-fleets deprecations (fleet fan-out excludes the entry;
   > `claudine-gen` fails loudly if asked to generate a skipped provider).
   > (2) **Canonical slugs** ratified: `kimi`, `opencode`, `qwen` (roster `file:`
   > stem == slug fleet-wide; Rust `Provider` variants and serde forms unchanged).
   > (3) **Major-version policy**: ratified as specified — see spec.md
   > "Major-version changes are new providers" (new binary / wire protocol / config
   > surface ⇒ new roster entry with its own slug, never a mutation; Kimi v1→v2
   > precedent, `skip_research` or removal for the old entry).
   > (4) **Field-keyed matrix ratifications (round 2):** `billing_models` stays a
   > facts field, ratified even without a live consumer; `supports_skills` trusts
   > the recent skills research round and graduates research-fed at v1 via the
   > `support` enum→bool mapping (`first_class`/`partial` → `true`,
   > `convention_only`/`none`/`unknown` → `false`; goose/kimi `false` constants
   > ruled stale, adopted through v1's diff-reviewed regeneration); `config_paths`
   > naming ratified with the two-population decision (agent-cli key renamed
   > `config_files` → `config_paths` feeding the existing catalog field;
   > model-config key renamed `config_files` → `model_config_paths` as a distinct
   > population; table-A `config_files` retired as a separate field —
   > `ConfigFileSpec` becomes `config_paths`' future richer type); registry
   > vocabulary extensions approved as proposed (StringArray, Record, optionality
   > markers, multi-path research sources; `allowed_env_keys` stays facts rather
   > than multi-topic machinery). (5) Matrix Open question 7 (shared-constant
   > fields) ruled 2026-07-04, approved as recommended: `value_taking_flags` is
   > hoisted to shared code (per-provider field removed; union semantics stand),
   > and `structured_stream_flag` derives from `output_formats` (dropped from
   > table A). All Checkpoint B questions are now closed; generator v1 proceeds.
   > (6) **`ModelCatalogSource` reshape + rename (2026-07-05):** the enum is
   > mechanism-only — `None` / `Static` / `ShellCommand { program, args }`
   > (provider-specific `OpencodeCli`/`OpencodeCliQwenFiltered` variants
   > deleted); `ProviderInfo.dynamic_source` renamed `model_catalog_source`
   > everywhere (struct, describe key, override key, catalog.json). Qwen leaves
   > shell sourcing (research coercion → `static`; override deleted); opencode
   > pins the ShellCommand object; codex/kimi keep `static` pins (matrix Open
   > question 8). Generate-time unchained ruling recorded: model ground truth
   > joins arrive with Phase F's committed unchained-ai artifact — the registry
   > re-points `static_models` there; the runtime enum stays mechanism-only.
3. **Generator v1**: registry covers all current fields; `data.rs` generated
   byte-stable for all 8; drift test + CI `--check` land; `catalog.json` superset
   emitted; `--mapping` rendered by claudine-cli through renderable components.

Exit: no hand-written provider data remains; CI enforces regeneration equality.

## Phase C — pipeline workstream 0 + first component

1. Shared prep stages (design/pipeline-dry.md): extract OpenCode model resolution,
   shadow-home env, Codex output prep into `exec_prep/`; both pipelines consume them.
   Include an `OPENCODE_CONFIG_CONTENT` **merge contract**: system-prompt injection,
   MCP injection, and permission overlays all write this one env var (system-prompt
   summary) — the shared prep stage must merge, never overwrite.
2. `FinalMessage` renderable component (design/render-components.md migration 1)
   retires the ×3 Codex rendering; `lib/src/render/` module is born.
3. **AgentCapabilities retirement** (design/module-split.md): migrate
   `providers.rs` describe output to ProviderInfo + catalog.json (rendered through
   components); delete the `agents::Agent` trait, `agent_for`, the 80-field tree,
   every `legacy.rs`, and the agreement tests.

Exit: one source of provider truth; three duplicated prep sites gone; first
functional component live in both pipelines.

> **Phase C completion notes (2026-07-04):** shipped as specified — shared stages in
> `cli/src/commands/exec_prep/` (`resolve_model_and_validate`, `ensure_shadow_home`,
> `prepare_codex_structured_output`), `FinalMessage` in `lib/src/render/` (module
> born) with the CLI sink helper `output::emit_final_message`, and the full
> AgentCapabilities retirement (agents module, 7× `legacy.rs`, `agent_capabilities_fn`
> — generator boilerplate updated + all 7 `data.rs` regenerated in the same change;
> `catalog.json` unchanged since the field was serde-skipped). Conditional dispatch
> sites 28 → 23; shrink-only legacy guard now asserts an empty set. Two
> behavior-preserving judgment calls for review: (1) `ensure_shadow_home` was wired
> into the composition seam only — the direct wrapper's MCP seam keeps its
> read-only behavior in the degraded shadow-home case; (2) `WrapperProfile::supports_resume`
> lost its legacy-tree default (the typed catalog cannot represent kimi/qwen
> resume-without-entrypoint), so the default is now `false` with explicit `true`
> overrides paired with each `build_resume_args` impl (claude, codex, kimi, qwen).
>
> **Post-C ruling (2026-07-04, Ken) — resume parity:** provider-native resume
> support ⇒ Claudine resume support (spec resume row amended). The legacy
> `false` values for gemini/goose/opencode were stale — the 2026-07-03
> session-resumption research rates all 7 first-class — so those three profiles
> gained `build_resume_args` + `supports_resume` the same day (Goose pinned to
> explicit `--session-id`, OpenCode to explicit `--session`, per the research
> cautions; resume passthrough extended with `--print-logs`/`--log-level` so a
> resumed OpenCode structured run keeps its stderr bridge). `ResumeSpec`
> catalog graduation stays a Phase D item (summary-triage.md).

## Phase D — field expansion + WrapperProfile migration (rolling)

Consumes closeout-track topics as they land; repeats per topic:

1. Extend the mapping registry (topic → fields); graduate facts entries; regenerate.
2. Generate the **WrapperProfile disposition table** from the A2 inventory + the
   data/behavior litmus test; ► **CHECKPOINT D (Ken):** ratify the disposition table
   (57 overrides classified catalog-data vs behavior).
3. Migrate `catalog-data` overrides method-by-method to catalog-driven defaults;
   delete as they zero out. Behavior overrides stay.
4. **Behavior-gap triage** (per topic): `requires_claudine_update` flags and
   summary-surfaced gaps (provider-native capability with no Claudine last mile)
   become explicit backlog items, each with a disposition — implement now, schedule,
   or won't-do — reviewed at the same checkpoint. The seeded backlog lives in
   [summary-triage.md](summary-triage.md) (2026-07-03, one section per summary,
   disposition checkboxes). Two items are flagged **triage early** there: Kimi Wire
   1.9-pin vs 1.10-server (live breakage) and Codex's 10-event hook system (Claudine's
   notify-only-era Codex registration under-covers the canonical events). The
   cross-topic **Roo refresh sweep** consolidates Roo's missing/stale research across
   six topics into one item.

Exit: static-fact overrides at zero; profile is a genuine behavior shim; table A
fields all research-fed or facts-fed with a tracked graduation queue; every landed
topic's behavior gaps carry a disposition (no surfaced-only flags remain).

> **Phase D wave-1 status (2026-07-04):** steps 1–2 executed, awaiting Checkpoint D.
> (a) Goose `static_models` pinned to `[]` by override (Ken's ruling, this prompt) —
> the research list was aggregator-contaminated picker examples. (b) **Field
> expansion wave 1 landed**: 10 new `ProviderInfo` fields — research-fed
> `resume: ResumeSupport` (support level only, per the resume-parity ruling),
> `model_cli_flag` (new `CliFlagSitesToFlag` skip-loudly coercion),
> `non_interactive_conflicting_flags` (new `FlagListToStringSlice`); facts-fed
> `billing_models` (values recovered from the deleted legacy tree at `edd22f733^`),
> `allowed_env_keys`, `stdout_noise_prefixes`, `stderr_noise_prefixes`,
> `suppress_structured_stderr_on_success`, `supports_interactive_inline_closure`,
> `model_required_in_non_tty`. Registry now 41 entries (10 roster + 9 research +
> 22 facts); `ResumeSupport`/`BillingModel` enums live in catalog-types. Four
> `model_cli_flag` overrides pin `--model` (goose: aggregator `--provider`-first
> ordering; codex/gemini/opencode: compound `"--model, -m"` sites defeat the
> bare-token rule) — the agent-models fleet prompt should gain a one-bare-flag-
> per-site mandate at the next refresh (mirror of Checkpoint A ruling 4). NO
> consumer wired: all 66 profile overrides intact pending Checkpoint D. (c) The
> disposition table lives in [disposition-table.md](disposition-table.md) —
> live count is **66** (not 57/67): 17 clean catalog-data, 8 ruled candidates,
> 7 identity, 34 behavior. (d) Roo refresh sweep closed won't-do in
> summary-triage.md (superseded by Checkpoint B's Roo removal). Verified: full
> unit suite + lint + gen check clean; L2 123/126 with only the 3 known
> host-specific detail-pane failures.
>
> **Checkpoint D rulings, first batch (2026-07-04, Ken):** (1) **Kimi Wire →
> option D, full 1.10 adoption** (`[I]` in summary-triage.md: version window
> {1.9, 1.10} + model `StepRetry`/`mcp_status`/richer `Notification` + the
> `tasks/`/compaction surfaces; version-scoped facts per the kimi-cli 1.x vs
> Kimi Code 0.x product split; queued after step 3). (2) **`platform_kind`**:
> gemini/qwen/kimi ruled `vendor_platform`; field landed facts-fed for all 7
> the same day (registry 42). (3) **`sandbox` deferred** — no catalog field
> until the permissions six-axis work provides a consumer. (4) **PathTemplate
> grammar = `{snake_case}`** — audit found the committed catalog already
> conformant; no migration. (5) **`stream_protocol` → framing vocabulary at
> the NIS-graduation moment** (option b).
>
> **Checkpoint D rulings, second batch:** (2026-07-04) disposition-table asks
> 1–3 **approved** — the 17 clean catalog-data migrations, the C1
> `apply_structured_stream` split (derive for codex/kimi/gemini/qwen; claude
> via a `companion_flags` slot on the Stream `output_formats` record; opencode
> stays behavior), and C2 `apply_non_interactive_flags` stays behavior. Step-3
> migration unblocked. (2026-07-05) **Codex 10-event hooks ruled `[S]`** —
> scheduled after step 3 alongside the Kimi Wire option-D work; two-phase plan
> and caveats recorded in summary-triage.md.
>
> **Phase D step-3 completion (2026-07-05):** the ratified migrations shipped
> and verified (unit + lint + gen check clean; L2 = only the 3 known
> host-specific failures; inventory re-blessed 431/20). 22 overrides + 2
> helpers deleted; `platform_kind` landed facts-fed (registry 42);
> **static-fact overrides at zero** — remaining profile impls are identity or
> ratified behavior. C1 implementation detail: `companion_flags` added to
> `OutputFormatSupport` (claude Stream record `["--print", "--verbose"]`);
> the derived default keys on the `OutputFormat::Stream` record — `FlagValue`
> selectors push unconditionally (exact `push_stream_json_flags` semantics),
> `Flag`/`TransportFlag` keep the has-flag guard. **Judgment call for review
> (codex facts fix, override-as-ground-truth):** codex's Stream record was
> `schema-json`/`--output-schema`, which could not reproduce the override's
> `--json`; the Stream record is now `jsonl`/`--json` and the schema-json
> record was kept re-labeled `format: json` (argv detection and
> `apply_output_format(Json)` behavior preserved). Side effect:
> `claudine codex --output stream` now maps to `--json` instead of the
> previously nonsensical `--output-schema schema-json` (no test asserted the
> old argv). Rationale comments live in `facts/codex.yaml`. Step 4 (rolling
> topic rounds + remaining triage sections) and the two scheduled work items
> (Kimi Wire option D `[I]`, Codex hooks `[S]`) are the open Phase D tail;
> skill/docs drift refresh remains Phase I item 3.

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
   New declarative-record candidates from the NIS summary: Qwen exit-code
   classification (53 max-turns / 55 wall-clock / 130 interrupt — terminate with
   stderr only, bypassing `result`) and the Goose error-then-`complete` taint rule.
   Record-grammar extensions: the `source` enum needs an `acp` value (ACP
   `session/update` streams) and must distinguish promoted-structured stderr
   (OpenCode `--print-logs`) from diagnostic stderr.
5. Harvest v1 (unmatched error/warning events only, scrubbed, capped) ships last.

> **Phase E items 1–3 complete (2026-07-05).** Taxonomy: `SignalKind` (29, frozen;
> `human_input_requested`/`session_resumable` reserved per ruling) + `SignalEvent` +
> `SignalSource` (9, incl. `stderr_promoted`/`stderr_diagnostic` split, `acp`, `exit`)
> in catalog-types (dep floor + chrono, no-clock). Sidecar uses joined flat lists
> (`records[]` ↔ `extractions[].record`) — SimplifiedSchema rejects nested lists AND
> `eager` inside inline-object rows, so `evidence: file(required)` with existence
> deferred to `signals check`. Gen-side mirror test closes three-copy drift. Fleet ran
> 2026-07-05 (Ken-launched): 9/9 docs schema-valid, 79 records / 179 extractions;
> per-doc adversarial evaluation + surgical edits applied; evidence paths normalized
> to document-relative. **Engine-design findings for item-4/5 (E4):** (a) records
> match JSON scalars via stringified `eq`/`regex` — the compile step must pin a
> scalar→string coercion rule, and an `exists` operator would retire the `^[0-9]+$`
> idiom; (b) same-frame multi-signal payloads (claude `init`, kilo `step-finish`,
> qwen loop-result) starve under group-level first-match-wins — engine should
> evaluate first-match-wins **per signal kind** within the group, else co-resident
> signals are unreachable; (c) OpenCode's 8 records match Claudine's own
> `LogClassification` output ("classification-as-payload") — the five-branch
> vocabulary logic stays bespoke in errors.rs and the records are enum→signal
> mappings; needs ratification as the promoted-stderr convention (raw-text matching
> would need AND/negation the grammar lacks); (d) `Unit` needs `duration_millis`
> (claude `retry_after_ms`, pi `delayMs` are currently unit-lied/unit-less);
> (e) cross-fixture overlap assertions must tolerate declared exclusions (claude
> tokens-vs-billing, goose complete-vs-taint). **Parser-lag findings (code items):**
> kimi `KimiPromptResult` lacks `steps`, `KimiQuestionRequest` shape predates
> source's `questions[]`; qwen parser ignores upstream `system/subtype=init`;
> claude older status vocab `limited`/`blocked` documented.
>
> **Rulings executed (Ken, 2026-07-06):** (1) **Fixture provenance is
> first-class** — four classes (`capture` / `test_vector` / `source_shape` /
> `docs_example`) in `docs/research/signals/fixtures/provenance.yaml`,
> bijection-enforced by `gen/tests/fixtures_provenance.rs`; `source_shape` is
> legitimate only when verbatim-verified against pinned source AND labeled;
> the fleet prompt now carries the 4-rung evidence ladder + "never author
> payload bytes yourself." Goose's 8 fabricated fixtures were re-derived by
> round-tripping goose's actual serde types at commit 65eed515 (7 rewritten;
> the 8th — retries_exhausted — was PROVEN wire-invisible at that commit, so
> its record+fixture were deleted and the invisibility recorded as a gap).
> (2) **OpenCode classification-as-payload ratified** as the design's glue
> mode: for `stderr_promoted` sources the matched payload is defined as the
> serialized promoted-stderr classifier output (`LogClassification`); records
> stay declarative with the payload definition stated in the sidecar and
> per-record notes; the five-branch vocabulary logic stays bespoke in
> errors.rs. Known darkmatter quirk surfaced: `md schema validate` resolves
> frontmatter `file()` references against the CWD, not the document — signals
> docs only validate from their own directory.
>
> **E4 checkpoint rulings (2026-07-06, Ken):** (1) **Compiled tables live in a
> single generated lib module** — `lib/src/signals/generated.rs`, slug-keyed
> tables for all 9 roster providers; a hand-written `lib/src/signals/mod.rs`
> exposes `detection_table(slug)` / `for_provider(Provider)`. Table row types
> live in catalog-types (gen must not depend on the claudine lib). No
> ProviderInfo shape change, so the registry+emit+regen-all-7 coupling is not
> triggered and record churn stays out of data.rs. (2) **kilo/pi records are
> compiled-but-dormant** — generation-validated (malformed path/regex = error)
> and replayed by `signals check`, unreachable at wrapper runtime until
> Phase H adds their enum variants. (3) **`exists` MatchOp added now**
> (semantics: field present and non-null) — variant + sidecar enum + mirror
> test + rewrite of the 11 presence-proxy records (10× `^[0-9]+$`-style
> tokens/cost matches, 1× `^.+$` apiKeySource) in one change. (4) **CI:**
> claudine-tests.yml gains a claudine-gen matrix row (drift/mirror/provenance
> tests were previously local-only — hole closed) with `claudine signals
> check` wired alongside.
>
> **E4 complete (2026-07-06, four waves, uncommitted pending review):**
> T1 `Unit::DurationMillis` + `MatchOp::Exists` + `signal_table` row types in
> catalog-types; sidecar enums extended; 11 presence-proxy records → `exists`;
> claude `retry_after` / pi `delay_ms` unit fixes. T2 gen signals stage
> (`gen/src/signals.rs`): sidecar-validated load, path-grammar/regex/op-value/
> priority/duplicate gates, byte-stable emission of `lib/src/signals/generated.rs`
> (9 tables, 78 records / 172 extractions, kilo+pi dormant), catalog.json-style
> wiring + drift test. T3 engine (`lib/src/signals/`): restricted-JSONPath
> walker, per-KIND first-match-wins, pinned scalar→string coercion, inclusive
> since/until with union-until-narrowed version selection, canonical
> kind→SignalEvent builder (corpus field renames: `lifts_at`→`resets_at`,
> `model`→`resolved` fleet-wide, kimi `wait`→`retry_after`/`steps`→`limit`,
> pi `delay_ms`→`wait`/`message`→`prompt`), `SignalSink` dedup
> (CORRELATION_WINDOW 5s, occurrence fold), stream wiring in
> `run_child_stream_semantic` (one guarded serde_json parse per line;
> `ProcessResult.signals` carries `Vec<ObservedSignal>` for E5). T4
> `claudine signals check` (evidence existence, positive replay via the
> production engine incl. glue-mode `LogClassification::to_signal_payload()`
> — the E5 shim core; extraction resolution; negative overlap w/
> `_overlap-exclusions.yaml`, 17 entries; bespoke seam reporting 4 SKIPs) +
> `signals-check`/`test-gen` recipes + CI matrix row + README. Fleet: 78
> records, positives 74 / negatives 78 pass, 0 failures; suites 5291/5294
> (3 known host L2); dispatch inventory unchanged 435/20. **Review items for
> Ken:** (a) duplicate-gate identity includes the since/until window (else
> the ratified opencode 1.17.8 twins false-collide); (b) 36 extraction rows
> carry supplementary fields with no SignalEvent slot (cache_read/cost/
> provider/session_id/…) — kept as research evidence, builder resolves +
> debug-logs them; purge would be a new ruling; (c) 15 replay-surfaced
> overlap exclusions beyond the 2 ratified seeds (all structural cross-kind
> co-fires); (d) opencode 429 fixtures enriched with providerID/modelID tags
> real service=llm lines carry (provenance notes updated).
>
> **E5 complete (2026-07-06, three waves).** Wave 1 parser-lag: kimi
> `KimiPromptResult.steps`, `KimiQuestionRequest` current nested shape w/
> legacy tolerance (auto-response correctly keyed on the JSON-RPC envelope id,
> pinned in tests — the payload id/tool_call_id are extraction-only), qwen
> `system/subtype=init` accepted as session start. Wave 2 sink fan-in:
> `SignalHub` (Arc-shared engine+sink, one lock, poison-recovering, drain via
> mem::take) created per run and threaded to stdout loop + OpenCode stderr
> bridge (`with_signal_hub`; glue shim live at reasoning.rs classification
> points via `to_signal_payload`) + temporal guards
> (`EarlyTermination::to_signal_event()` exhaustive; emitted at the bridge
> fire point AND the wrapper's post-wait error_kind synthesis for all
> providers; EarlyTermination::RateLimit → UsageCapped w/ window Unknown);
> claude rate-limit projection `signals::project::rate_limit_info` with
> parser↔projection equivalence tests over the claude fixtures; CLI post-exit
> consumer (`IterationSummarySignals`) migrated field-wise projected-wins/
> parser-fills-gaps (output-invariant bridge; mid-stream consumers stay
> parser-fed, full retirement deferred until the engine path soaks). Wave 3
> bespoke chain (`signals/bespoke.rs`, slug-keyed — no new Provider dispatch,
> lib guard untripped): goose error-then-complete taint, claude
> result-success-with-prior-error taint, pi auto_retry_end exhaustion
> (replay-only, dormant), qwen native LoopType → RunawayRepetition (counts
> unknown → 0s, documented), kimi protocol-version negative match referencing
> `SUPPORTED_WIRE_PROTOCOL_VERSIONS` (runtime limited to stdout-fed payloads;
> the wire_io path keeps its own semantic guard), qwen exits 53/55/130 as an
> Exit-source chain detector; generic exit-source payload
> `{exit_code, stderr_tail}` (tail = 10 lines, `EXIT_STDERR_TAIL_LINES`)
> synthesized once per run on both spawn paths; qwen.md gaps updated
> (detection landed bespoke, evidence still awaits harvest). `signals check`:
> **positives 78/78, negatives 78/78, bespoke skipped 0, exclusions 17
> (none added), failures 0.** Suites 5258 pass / 3 known host L2; clippy/gen
> check/inventory (435/20, line-shifts only) clean.
>
> **E6 complete (2026-07-06) — Phase E closed.** Harvest v1:
> `lib/src/signals/harvest.rs` (minimal v1 error/warning predicate — top-level
> type/subtype/level/severity/status contains error|warn|fatal, `is_error`,
> top-level `error` key, Exit `exit_code != 0`; evaluated only on
> zero-record-fired payloads, opt-in only, disabled path is one Option check)
> + `lib/src/protect/scrub.rs` (capture-time redaction co-located with the
> protect catalog: 7 static rules — sk-/AKIA/gh*/xox/Bearer/JWT/email — plus
> key-named value redaction and home→`~` rewrite; `<redacted>` token per the
> messaging convention). Persistence `~/.claudine/harvest/<slug>/<date>.jsonl`
> via `harvest::flush_hub` at both spawn drain sites; retention by filename
> date then size oldest-first (30 days / 50 MiB / 100-entry run cap, all
> doc-commented consts; failures warn-only). Opt-in: user-scope
> `harvest_unmatched` (default false; repo scope rejects it) with
> `CLAUDINE_HARVEST` env override resolved in `policy.rs::harvest_enabled()`.
> Promotion process documented (fixtures README + module doc: human-reviewed →
> provenance class `capture`). Known gap carried: the kimi `wire_io` path never
> drains its hub (signals or harvest) — pre-existing, noted for the Kimi Wire
> [S] item. Follow-up spec parked at
> `features/2026-07-06-more-struture/spec.md` (unmapped-research graduation;
> implement only after this spec fully closes).

## Phase F — model-catalog boundary (parallel track, unchained-ai side)

Per design/model-catalog-boundary.md: identity parser as a lib target in
`unchained-ai/gen` (from the spike prototype) → committed
`artifacts/models-catalog.json` + JSON Schema + generated_at/schema_version →
claudine-gen consumption (expected-offering records with `catalog_id` joins;
plan-endpoint + local-runner offerings first-class in the mapping layer) →
`family_latest` resolution + staleness warnings → staged per-provider demotion of
dynamic listing to a drift channel emitting SignalEvents (couples to Phase E sink).
Per-provider listing sources for that staging (agent-models summary): programmatic —
Codex `debug models [--bundled]`, OpenCode `models --refresh`, Kilo `models` + gateway
REST, Kimi `/v1/models` + ACP `available_models`, Pi `--list-models` + RPC; none —
Claude/Gemini/Goose/Qwen. Correction from the non-interactive-sessions summary
(2026-07-03): for Claude/Gemini/Qwen the resolved model is observable from runtime
stream output (Gemini emits model metadata in `init`), but **Goose's stream has no init
event and never emits requested/resolved provider/model** — Goose resolution is
config/wrapper-side only, so its drift channel needs a different source.
► **CHECKPOINT F (Ken):** artifact schema review before claudine consumes it.

> **Phase F steps 1–2 DONE, Checkpoint F RULED (2026-07-06; uncommitted pending
> Ken's diff review).** Landed: lib target in `unchained-ai/gen` (`catalog`
> module — schema types, identity-key derivation, family-index builder,
> canonical emission), offline `emit-catalog` bin + `just artifact`, committed
> `unchained-ai/artifacts/models-catalog.json` + JSON Schema + README
> (662 offerings / 261 families / 131 duplicate groups / 1 gap:
> `zenmux/openai/chat-latest`), byte-equality drift tests + sanity floors in
> `gen/tests/catalog_drift.rs`. Rulings: (1) identity-key grammar ratified —
> `vendor/family[@version|@date_pin](+variant)*(+size)*(:tag)*`, sizes join,
> only `thinking` among serving tags is identity-bearing (delivery tiers
> `free`/`nitro`/… stay out); (2) `FamilyEntry.latest` is an **identity key**
> (names the release, not an offering — no canonical offering elected);
> (3) the identity parser stays in `unchained_ai::models::identity`, gen's lib
> target hosts only artifact schema + builder — design doc amended in place;
> (4) `generated_at` derives from the max `//! Generated:` header of the
> committed provider enum files (tracks data age; offline-deterministic).
> Root-caused during review: the Parsera LLM Specs API is sunsetted (silent
> graceful degradation) — direct-provider metadata coverage collapse spun off
> to `features/2026-07-06-model-metadata/spec.md` (Parsera → models.dev,
> separate track, does NOT block this spec: the Phase F identity path is
> metadata-independent, and duplicate-group joins recover roster-critical
> metadata meanwhile). Claudine-gen consumption is now unblocked.

> **Phase F consumption round 1 DONE (2026-07-06, uncommitted).** The artifact
> is now a **hard generation input** to claudine-gen: absence /
> schema_version mismatch fail generation loudly (`gen/src/artifact.rs`;
> expected version tracks the artifact — currently **2** after the parallel
> models.dev track's `release_date` bump); `generated_at` >30 days emits a
> staleness warning in `generate` and `check`. Two new generated ProviderInfo
> fields (one-change discipline held: registry + emit + regen all 7 +
> catalog.json): `expected_offerings: &[ExpectedOffering]` (agent-models
> `default_models[]` → id/alias/is_default/context_window + `class` +
> optional `catalog_id` identity-key join) and `offering_sources:
> &[OfferingSource]` (model-config `local_runners[]` → prefix/class/
> api_standard/integration; 6 runners × 7 providers). New shared vocab in
> catalog-types: `OfferingClass` (vendor_api/plan_endpoint/aggregator/
> local_runner), `LocalRunnerIntegration`, both row types. Rulings: the join
> ladder is **exact-only lookup** (exact `mapped_source/id`, then unique
> bare-id match; ambiguity → None) — claudine deliberately does NOT
> reimplement the identity grammar; classification is a curated table in gen
> source (kimi-for-coding/kimi-code → plan_endpoint; `opencode/` prefix →
> aggregator; default vendor_api); plan-endpoint ids skip the ladder
> (model-API-only artifact, F4). `static_models` untouched — the runtime
> validation-baseline flip stays staged per provider (next). Join coverage at
> generation: claude 6/8, codex 0/4 (real — Parsera-sunset-collapsed openai
> slice), gemini 10/11, goose 2/6, kimi 10/11 (11th is the plan endpoint),
> opencode 18/50, qwen 3/10; unjoined ids listed in the gen report. Fixture
> rule learned mid-round: gen tests must reference
> `artifact::EXPECTED_SCHEMA_VERSION`, never a literal version, or a parallel
> artifact bump breaks them (bit once, fixed).

> **Phase F family_latest DONE (2026-07-06; committed f9d58b02b/dc7758eb4/
> d4bd2a5d6 + two follow-up test files).** claudine-gen compiles a vendored
> family-index slice (signals-module pattern: `gen/src/families.rs` →
> `lib/src/model_catalog/families_generated.rs`, drift-checked in `check`) —
> 18 family keys derived from all providers' `catalog_id` joins via the
> shared `family_key()` prefix rule in catalog-types (the ONE claudine-side
> grammar touchpoint: identity key up to the first `@`/`+`/`:`; a derived
> key missing from the artifact families{} is a loud generation error).
> `ExpectedOffering` gained `resolves: Option<ResolvesVia>` — marked
> `family_latest` iff alias AND join (curated `RESOLVES_EXCEPTIONS` escape
> hatch, empty in v1; same-alias records deriving different families lose
> the mark and report as ambiguous). Marks landed: claude fable/opus/sonnet
> (haiku's record has no join), gemini pro + both flash + both flash-lite
> (`auto` unmarked — alias-only router). Runtime resolver
> `model_catalog::families`: `family_latest[_at]` (binary search) and
> `resolve_alias[_at]` returning `FamilyLatest { identity_key, family,
> staleness }`; `Staleness::Stale { age_days }` past
> `FAMILY_LATEST_MAX_AGE_DAYS = 30` vs the compiled-in
> `ARTIFACT_GENERATED_AT` — warn-not-fail, per ContentPolicy. Session-log
> stamping of resolved answers is deliberately deferred to the
> drift-channel increment (where ModelResolved already flows). Note for
> operators: any artifact re-emission drifts `families_generated.rs` by
> design — one `generate --yes` away. Remaining Phase F item: staged
> per-provider demotion of dynamic listing to a drift channel
> (expected_offerings becomes the validation baseline per provider;
> SignalEvents through the Phase E sink; Goose config-side source; Kimi
> wire-mode gap stands).

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
  Caveat (agent-logging summary): Kilo is **split-lineage** — the CLI is an OpenCode
  fork (XDG paths even on macOS, e.g. `~/.local/share/kilo/kilo.db`) while the IDE
  extensions are Roo forks with Roo-style task files. M-Kilo targets the CLI only;
  do not collapse the two product surfaces into one provider shape.
  Ladder inputs from the 2026-07-03 summaries: adapter contract is
  `kilo run --auto --format json --dir <cwd>`; `kilo run` denies questions and
  auto-rejects permissions by default; a structured `error` event outranks exit 0;
  Kilo ACP lacks `session/cancel` and needs a `session/request_permission` handler
  before the integration is usable.
  ► **CHECKPOINT H1 (Ken):** process retro — scaffold quality, generate UX, report
  accuracy; adjust before Pi.
- **M-Pi** — graduation #2, the sterner behavior test (bespoke models.json/API
  surface). Proves the data/behavior seam on a non-cousin.
  Ladder inputs from the 2026-07-03 summaries: Pi core has **no native MCP and no
  subagents** (both exist only via executable TypeScript extensions) and **no
  permission system** (external sandboxing required — feeds the permissions six-axis
  classification); headless determinism set is `pi --mode json` +
  `--no-approve --no-extensions --no-skills --no-prompt-templates --no-context-files`;
  ACP requires an external adapter with two divergent lines (registry `svkozak/pi-acp`
  vs the more capable `@victor-software-house/pi-acp`) — a version-drift surface.
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
