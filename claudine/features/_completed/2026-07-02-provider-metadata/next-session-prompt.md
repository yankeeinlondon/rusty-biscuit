# Next-session kickoff prompt (Phase E items 4–6 — engine + migrations + harvest)

> Disposable working doc: paste the block below into a fresh session after
> `/clear`. Delete this file once the session is underway.

```
Work Phase E items 4-6 (signal engine, migration map, harvest) of the
provider-metadata plan
(claudine/features/2026-07-02-provider-metadata/implementation-plan.md §Phase E),
governed by design/signal-detection.md — read that design doc FIRST, then the
TWO Phase E blockquotes in the plan (wave-1 completion + rulings-executed
2026-07-06): they carry the engine-design findings and ratified rulings this
prompt assumes.
Act as orchestrator; use the claudine agent skill; subagents never commit, no
cargo fmt; never commit unless I say so. HITL checkpoints: stop for me.

Context beyond the plan/design docs:

1. STATE (all committed, git clean): Phase E items 1-3 are DONE — taxonomy
   (catalog-types/src/signal.rs: SignalKind 29 frozen incl. reserved
   human_input_requested/session_resumable; SignalEvent tagged on `kind` with
   exhaustive kind() accessor; SignalSource 9 incl. stderr_promoted vs
   stderr_diagnostic, acp, exit; MatchOp eq/in/substring_ci/regex;
   DetectionMode; Quantity; UsageWindow; catalog-types dep floor gained chrono
   no-clock), the sidecar (docs/research/signals/_schema.yaml — joined flat
   lists records[] <-> extractions[].record; evidence is file(required)
   because SimplifiedSchema rejects `eager` in inline-object rows), the fleet
   doc, the fixture corpus (~90 files), AND the fleet itself: 9/9 provider
   docs schema-valid with 78 records / ~175 extractions, adversarially
   evaluated per-doc, surgical edits applied. Drift guards in place:
   gen/tests/signals_sidecar_mirror.rs (sidecar enums == VARIANTS x7) and
   gen/tests/fixtures_provenance.rs (provenance.yaml <-> files bijection).

2. RATIFIED RULINGS (2026-07-06, Ken — recorded in the plan blockquote):
   a. Fixture provenance is first-class: 4 classes (capture / test_vector /
      source_shape / docs_example) in fixtures/provenance.yaml; source_shape
      legitimate only when verbatim-verified against pinned source AND
      labeled; fleet prompt carries the evidence ladder + "never author
      payload bytes". Goose corpus was re-derived by serde round-trip at goose
      commit 65eed515; goose retries_exhausted record+fixture DELETED (proven
      wire-invisible at that commit).
   b. OpenCode "classification-as-payload" ratified as the design doc's glue
      mode: for stderr_promoted sources the matched payload IS the serialized
      output of the promoted-stderr classifier (LogClassification,
      lib/src/stream/logs/opencode/events.rs); the five-branch vocabulary/
      conjunction logic stays bespoke in errors.rs; the 8 opencode records
      stay declarative (payload definition stated in the sidecar comment +
      per-record notes). E4 implements this as a bespoke shim that serializes
      LogClassification to JSON and feeds the generic engine.

3. E4 SCOPE (plan §E item 3) with the engine findings as REQUIREMENTS:
   a. Generate-time compilation: claudine-gen consumes the research records ->
      parsed/validated at generate time (malformed path/regex = generation
      error) -> &'static detection tables. OPEN DESIGN DECISION to present at
      a checkpoint: where compiled tables live for the 7 compiled providers
      (data.rs vs a sibling generated signals module) and what happens to
      kilo/pi records (roster-only, no data.rs — likely compiled-but-dormant
      or deferred to Phase H). Remember the coupling: any ProviderInfo shape
      change needs registry+emit.rs+regen all 7+catalog.json in ONE change;
      regen via cargo run -p claudine-gen -- generate --yes / -- check.
   b. Matching semantics (from the evaluation findings — non-negotiable):
      per-SIGNAL-KIND first-match-wins within a provider x source group, NOT
      group-level single-winner (group-level starves same-frame multi-signal
      payloads: claude init carries model+apiKeySource; kilo step-finish
      carries tokens+routed-model; qwen loop-result carries usage). Pin a
      scalar->string coercion rule at compile time (records fleet-wide
      eq/regex against JSON numbers and booleans, e.g. ^429$, "true").
      Consider adding an `exists` operator to retire the ^[0-9]+$ idiom — if
      added, it is a MatchOp variant + sidecar enum + mirror test in the same
      change.
   c. Generic engine in lib walking JSON values against the provider's
      tables; version-range record selection: unknown version = union of
      version-scoped records w/ priority tiebreak; narrows once
      provider_version observed in-session. Sink: dedup on (session, signal
      kind, correlation window), first source wins, duplicates increment an
      occurrence counter. Insertion point: run_child_stream_semantic
      (cli/src/commands/wrap/exec/spawn.rs:~705) alongside SemanticEventSink;
      bespoke emitters join the SAME sink.
   d. Unit enum needs `duration_millis` (claude retry_after_ms currently
      unit-lied as unix_millis w/ gaps entry; pi delayMs left unit-less) —
      catalog-types Unit + sidecar + mirror test + fix those extraction rows
      in the same change.
   e. `claudine signals check`: new CLI subcommand (register in
      cli/src/args.rs + main.rs match, module cli/src/commands/signals.rs;
      nested-subcommand templates: mcp/logs). Replays every record's evidence
      fixture through the PRODUCTION engine (positive: match fires + every
      extraction resolves to declared unit/type; negative: mechanical-overlap
      assertions per provider x source group, WITH declared-exclusion support
      — known benign overlaps: claude tokens_consumed fires on the
      billing-error fixture's result line; goose complete matches line 2 of
      error-then-complete). Dev/CI-facing, requires a checkout. Wire into CI.
      Bless dispatch-inventory after cli edits: CLAUDINE_UPDATE_INVENTORY=1
      cargo nextest run -p claudine-cli --test dispatch_inventory (was 431/20
      pre-E; verify).
   f. Known darkmatter quirk: `md schema validate` resolves frontmatter
      file() refs against CWD, not the document — validate signals docs FROM
      docs/research/signals/. (Candidate darkmatter fix, out of scope.)

4. E5 MIGRATION MAP (plan §E item 4; execute after/with E4):
   - Claude rate-limit: records exist (incl. resolved_reset_at fallback as
     priority-ordered pairs); route through the engine; RateLimitInfo
     consumers become projections per design §Sink.
   - OpenCode: glue shim per ruling 2b — serialize LogClassification, engine
     maps to taxonomy kinds; LogClassification/ProviderLimitKind stay,
     consumers swap input type not logic.
   - Temporal guards: permanent bespoke — map EarlyTermination variants
     (lib/src/stream/logs/opencode/reasoning.rs:100-211; error_kind strings
     mirror the guard SignalKinds by design) to SignalEvent emissions through
     the sink.
   - Qwen exit codes 53/55/130: GREENFIELD (nothing exists today —
     exit_code() returns None in the adapter; codes fall through to generic
     classify_exit in cli/src/output/error_report.rs). Implement via the
     `exit` source: wrapper synthesizes {exit_code, stderr_tail} and feeds
     the engine. NO fixtures exist (fabrication banned; qwen doc records this
     in gaps) — capture live or land the detection bespoke-with-gaps until
     harvest supplies evidence.
   - Goose error-then-complete taint: GREENFIELD bespoke (adapters/goose.rs
     maps events independently today); emits SignalEvent::SessionTainted;
     evidence fixture stream-error-then-complete.jsonl is serializer-faithful
     source_shape.
   - Kimi: StepRetry -> GenerationRetried (parser already extracts the fields
     at providers/kimi.rs:371-404); unsupported_protocol_version is wrapper-
     synthesized (negative match — bespoke by nature, recorded in kimi gaps).
   - Parser-lag code items surfaced by research (fix alongside): kimi
     KimiPromptResult lacks `steps` (wire sends it) and KimiQuestionRequest
     has the flat legacy shape vs source's id/tool_call_id/questions[]; qwen
     parser ignores upstream system/subtype=init (protocol/qwen.rs:66-80)
     which the qwen priority-10 record needs.

5. E6 (harvest v1) ships LAST: opt-in capture of error/warning-class events
   matching no record -> ~/.claudine/harvest/, capture-time scrub rules
   co-located with the protect catalog, size+age retention caps. Harvest
   promotions become class `capture` in provenance.yaml (human-reviewed).

6. QUEUED, DO NOT START: the D-1.5 [S] bundles (MCP round 2, evidence
   adapters, permissions posture, system-prompt truth-up, wrapper-metadata
   capture, hooks round, resume round 2, ACP adoption, linker v2,
   local-runner bridge), Codex 10-event hooks [S], Phase F (unchained-ai
   track), memory-topic fleet run. Schema-v2 approvals items
   1a/2a-2f/3a/3b/4a remain Ken's parallel review track (1c/1d done).

7. HOST QUIRKS: 3 detail-pane L2 tests fail on this host even on clean HEAD
   (level2_tmux_chooser_detail_right_in_wide_terminal + the two
   chooser_detail_above_in_tall_terminal variants); flaky-retry tests:
   lifecycle_executor::no_error_does_not_suppress_evaluation_raise,
   schema_reference_stays_document_relative_through_claudine_load,
   wrap_perf::compose_perf_emits_report_to_stderr. Run just test /
   just test-l2 / just lint from the claudine package area with absolute cd.
   FORCE_COLOR no longer pins width to 80 — L2 capture assertions must be
   wrap-tolerant.
```
