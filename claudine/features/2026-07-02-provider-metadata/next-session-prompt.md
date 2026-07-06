# Next-session kickoff prompt (Phase E — signal catalog)

> Disposable working doc: paste the block below into a fresh session after
> `/clear`. Delete this file once the session is underway.

```
Work Phase E (signal catalog) of the provider-metadata plan
(claudine/features/2026-07-02-provider-metadata/implementation-plan.md §Phase E),
governed by design/signal-detection.md — read that design doc FIRST, it opens
with ratified rulings the plan assumes.
Act as orchestrator; use the claudine agent skill; subagents never commit, no
cargo fmt; never commit unless I say so. HITL checkpoints: stop for me.

Context beyond the plan/design docs:

1. STATE: Phases A+B+C+D are COMPLETE including the D-1.5 checkpoint
   (2026-07-05, all ratified + executed + committed). Read
   d15-checkpoint.md for the D-1.5 rulings; summary-triage.md is fully
   marked (16 [I] / 52 [S] / 3 [W]) — the [S] bundles are scheduled, DO NOT
   start them. Also shipped 2026-07-05: Kimi Wire option D (version window
   {1.9,1.10}, request advertises 1.10, 1.10 events StepRetry /
   mcp_status / rich Notification parsed; fixture wire-protocol-110.jsonl),
   the hooks-report overhaul (glyph vocabulary + legend from one const,
   chunked degradation, unmapped_native_events surfaced), and three catalog
   changes: session_locations RETIRED, acp.server_mode research-fed
   (AcpServerMode: native/adapter/partial/none/unknown; claude/codex
   adapter, rest native), unmapped_native_events facts-fed (gemini
   BeforeToolSelection, opencode tool.definition).

2. CRITICAL COUPLING (the one that bites): all 7 lib/src/provider/<slug>/
   data.rs are GENERATED. Any shape change to ProviderInfo or its record
   types requires claudine/gen/src/{registry,emit}.rs updates + regenerating
   all 7 + docs/providers/catalog.json IN THE SAME CHANGE
   (cargo run -p claudine-gen -- generate --yes / -- check). Registry is 42
   entries (10 roster / 10 research / 22 facts); coerced enums live in
   claudine/catalog-types with lib re-export shims (ResumeSupport /
   BillingModel / PlatformKind / AcpServerMode pattern). Dispatch inventory:
   435 sites / 20 conditional; bless after cli edits with
   CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test
   dispatch_inventory.

3. PHASE E SCOPE (plan §E items 1-5):
   a. SignalEvent taxonomy in catalog-types + detection-record schema
      sidecar (path grammar, four operators, priority). RESERVE the
      human_input_requested and session_resumable signal kinds now (ruled
      2026-07-05 with the resume-round-2 [S] items).
   b. The `signals` research sequence is OWNED BY THIS PLAN (the one
      exception to "never run fleets") — author it, then PAUSE for my
      explicit go before launching (cost). Source-code-first methodology.
   c. Seed the fixture corpus from existing parser test data
      (lib/src/stream/protocol/fixtures/ + inline provider-parser tests).
   d. Generate-time record compilation to &'static tables + generic engine
      + sink dedup; `claudine signals check` wired into CI (positive
      fixture per record; mechanical-overlap negative assertions).
   e. Migration map: Claude rate-limit records; the OpenCode 429 cascade is
      THREE separate conditions (AI SDK envelope vs per-provider
      responseBody) and records must cover BOTH stream-error formats
      (OpenCode 1.17.8 changed it — that drift evaded usage-cap
      classification → unbounded retries → indefinite hang); temporal
      guards (timeout, step_timeout, stalled-generation) stay permanent
      bespoke but emit through the sink; Qwen exit codes 53/55/130 bypass
      `result` (stderr-only); Goose error-then-`complete` taint rule.
      Record grammar needs `source: acp` (ruled [I] → Phase E) and the
      promoted-structured (OpenCode --print-logs) vs diagnostic stderr
      distinction (the [S] "OpenCode promoted stderr is CONTRACT" triage
      item lands HERE). New candidates from this week's Kimi work: Wire
      StepRetry (typed retry observability: error_type/status_code/
      attempts) and error_kind unsupported_protocol_version.

4. PARALLEL TRACK (not this session): I am still reviewing the remaining
   schema-v2-approvals.md items (1a, 2a-2f, 3a/3b, 4a — 1c/1d are DONE,
   executed 2026-07-05). If decisions arrive, drafting sidecar/_fleet.md
   edits is in scope; RUNNING refresh fleets is not (closeout track owns
   those). The `memory` topic (docs/research/memory/) is authored but its
   fleet run is deferred — do not run it.

5. QUEUED, DO NOT START: Codex 10-event hooks [S]; the D-1.5 [S] bundles
   (MCP round 2, evidence adapters, permissions posture, system-prompt
   truth-up, wrapper-metadata capture, hooks round, resume round 2, ACP
   adoption, linker v2, local-runner bridge); Phase F (unchained-ai track).

6. HOST QUIRKS: 3 detail-pane L2 tests fail on this host even on clean HEAD
   (level2_tmux_chooser_detail_right_in_wide_terminal + the two
   chooser_detail_above_in_tall_terminal variants); flaky-retry tests:
   lifecycle_executor::no_error_does_not_suppress_evaluation_raise,
   schema_reference_stays_document_relative_through_claudine_load,
   wrap_perf::compose_perf_emits_report_to_stderr. Run just test /
   just test-l2 / just lint from the claudine package area with absolute
   cd. NOTE: FORCE_COLOR no longer pins width to 80 (2026-07-05 fix) — L2
   capture assertions must be wrap-tolerant (join newlines before substring
   match; precedent in level2_interrupt_feedback_capture.rs).
```
