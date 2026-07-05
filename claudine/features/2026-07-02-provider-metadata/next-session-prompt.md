# Next-session kickoff prompt (Kimi Wire → D-1.5 → Phase E)

> Disposable working doc: paste the block below into a fresh session after
> `/clear`. Delete this file once the session is underway.

```
Work the provider-metadata plan
(claudine/features/2026-07-02-provider-metadata/implementation-plan.md) in this
order: (0) the Kimi Wire [I] fix, (1) Phase D round 1.5, (2) Phase E (signal
catalog, design/signal-detection.md).
Act as orchestrator; use the claudine agent skill; subagents never commit, no
cargo fmt; never commit unless I say so.

Context beyond the plan/design docs:

1. STATE: Phases A+B+C+D-round-1 are COMPLETE and committed. All rulings live
   as blockquotes in implementation-plan.md, RULED/RESOLVED stamps in
   field-source-matrix.md, and disposition-table.md (marked EXECUTED) — read
   those three before touching anything. summary-triage.md carries the
   disposition state ([I]/[S]/[W] marks are Ken's rulings).
2. CRITICAL COUPLING (the one that bites): all 7 lib/src/provider/<slug>/data.rs
   are GENERATED. Any shape change to ProviderInfo or its record types requires
   claudine/gen/src/{registry,emit}.rs updates + regenerating all 7 +
   docs/providers/catalog.json IN THE SAME CHANGE
   (cargo run -p claudine-gen -- generate --yes / -- check). Registry is 42
   entries (10 roster / 9 research / 23 facts); coerced enums live in
   claudine/catalog-types (ResumeSupport / BillingModel / PlatformKind pattern
   with lib re-export shims); OutputFormatSupport carries companion_flags and
   the derived apply_structured_stream default consumes the Stream record.
   Dispatch inventory: 431 sites / 20 conditional; bless after cli edits with
   CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test
   dispatch_inventory.

3. STEP 0 — Kimi Wire (ruled [I] option D, live breakage; full scope in
   summary-triage.md agent-logging section): Claudine pins
   WIRE_PROTOCOL_VERSION="1.9" (cli/src/commands/wrap/exec/wiring/builders.rs)
   with strict equality in validate_initialize_response; current Kimi Code
   servers speak 1.10 and fail the handshake. Deliver in two stages, both this
   session if feasible: (a) version window {1.9, 1.10} with a
   remediation-bearing failure on unknown versions + fixtures — this unbreaks
   users; (b) model the 1.10 event surface (StepRetry, StatusUpdate.mcp_status,
   richer Notification) in lib/src/stream/providers/kimi.rs; the unmodeled
   on-disk surfaces (tasks/, compaction snapshots) may be deferred with a note.
   Version-scope any facts you touch: legacy kimi-cli (Python, 1.x line,
   self-reports "kimi, version 1.47.0", ~/.kimi) vs Kimi Code (kimi binary,
   0.x line, ~/.kimi-code, KIMI_CODE_HOME); Wire 1.9/1.10 is a third,
   independent version axis.

4. D ROUND 1.5 (no fleet runs; ends in ONE HITL checkpoint — stop for Ken):
   a. Propose dispositions ([I]/[S]/[W] + one-line rationale) for every
      remaining untriaged summary-triage.md item, grouped by section. Do not
      mark the file until Ken ratifies; present the proposal table.
   b. Present the three parked rulings with options + recommendation:
      session_locations semantics (what does the field MEAN — claude's value
      is a directory, codex's mixes an app log and shell snapshots; no single
      agent-logging role filter reproduces them), memory_files extraction rule
      (system-prompt config_sources: mode==append AND format==markdown?), acp
      vocabulary reconciliation (constants' server_mode values like
      available_via_wire_proxy vs the sidecar's native/adapter/partial/none vs
      the summary's native/adapter/unassessed).
   c. After ratification: execute whichever graduations the rulings unblock
      (memory_files and acp are mechanical registry re-points + regen;
      session_locations depends on the ruling). Optional if Ken says go:
      execute the stream_protocol NIS graduation (ruled option b — framing
      vocabulary ndjson/jsonl/json-rpc; StreamProtocol variant rename is a
      shape change: enum + emit.rs + consumer audit + regen in one change).
   d. The permissions env_vars.effect / precedence.scope enum derivation
      (schema-v2-approvals.md items 1c/1d, if approved) is an analysis task
      that fits here: inventory observed values across the 9 landed docs,
      propose enums for Ken.
5. PHASE E (after the D-1.5 checkpoint clears): per plan §Phase E and
   design/signal-detection.md. Anchors: SignalEvent taxonomy in catalog-types;
   detection-record schema sidecar (path grammar, four operators, priority);
   the `signals` research sequence is OWNED BY THIS PLAN (the one exception to
   "never run fleets") — PAUSE for Ken's go before launching it (cost); seed
   fixtures from existing parser test data; generate-time compilation to
   &'static tables + generic engine + sink dedup; `claudine signals check`
   wired into CI. Migration-map inputs: Claude rate-limit records; the
   OpenCode 429 cascade is THREE separate conditions (AI SDK envelope vs
   per-provider responseBody) and OpenCode 1.17.8 changed its stream-error
   format (drift evaded usage-cap classification → unbounded retries →
   indefinite hang) — records must cover both formats; temporal guards
   (step_timeout, stalled-generation) stay permanent bespoke but emit through
   the sink; Qwen exit codes 53/55/130 bypass `result` (stderr-only); Goose
   error-then-`complete` taint rule; record grammar needs source: acp and the
   promoted-structured (OpenCode --print-logs) vs diagnostic stderr
   distinction.

6. PARALLEL TRACK (not this session): Ken is reviewing
   features/2026-07-02-provider-metadata/schema-v2-approvals.md (per-item
   decision sheet). If decisions arrive mid-session, drafting the approved
   sidecar/_fleet.md edits is in scope; RUNNING refresh fleets is not — the
   topics-closeout track owns those. Note the closeout log's 2026-07-05
   correction: the widened permissions fleet ALREADY ran 2026-07-03 — do not
   treat it as pending.
7. QUEUED, DO NOT START: Codex 10-event hooks [S] (two-phase plan in
   summary-triage.md); Phase F (unchained-ai side, separate track).
8. HOST QUIRKS: 3 detail-pane L2 tests fail on this host even on clean HEAD
   (level2_tmux_chooser_detail_right_in_wide_terminal + the two
   chooser_detail_above_in_tall_terminal variants); flaky-retry tests:
   lifecycle_executor::no_error_does_not_suppress_evaluation_raise,
   schema_reference_stays_document_relative_through_claudine_load,
   wrap_perf::compose_perf_emits_report_to_stderr. Run just test / just
   test-l2 / just lint from the claudine package area with absolute cd.
```
