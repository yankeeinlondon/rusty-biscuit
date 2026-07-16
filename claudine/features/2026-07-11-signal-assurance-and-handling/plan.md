---
created: 2026-07-11
spec: ./spec.md
phases: 8
start_phase: 1
---

# Signal Assurance and Configurable Handling — Execution Plan

This plan implements the reviewed spec in [`spec.md`](./spec.md) (17 rulings +
11 review decisions). Spec phases A–E map onto numbered plan phases as:
A → 1, B → 2+3, C → 4+5, D → 6, E → 7; phase 8 is the closing validation and
drift sweep. The motivating incident (a transient Codex "at capacity" error
killing a non-interactive run) is fully resolved once phases 2 and 5 have both
landed.

## Dependency Overview

- **Phase 1** is the foundation for everything in Part 1: the criticality
  tier, `not_found` attestation schema, and the coverage command are consumed
  by phases 2, 3, and 7.
- **Phase 2** (research gap-fill) must land **before** phase 3 flips the
  generate-time hard gate, or CI goes red on known gaps. Phase 2 requires
  live provider access for fixture harvesting; cells that cannot be grounded
  resolve as `not_found` attestations instead — the phase cannot dead-end.
- **Phase 4** (lifecycle re-ordering) is the highest-blast-radius change
  (existing L1/L2 lifecycle tests, prompt-file migration). It is independent
  of phases 1–3 and may proceed in parallel with them.
- **Phase 5** (handling engine) depends on phase 4 (the post-`failure`
  fallthrough seam and `handler` global) and benefits from phase 2 (Codex
  overload record) for its end-to-end proof.
- **Phase 6** depends on phase 5 (engine + config plumbing).
- **Phase 7** depends on phases 1–3 (attestations must exist to notice).
- Primary package areas: `claudine/catalog-types`, `claudine/gen`,
  `claudine/lib` (`signals`, `composition`, `harness`, new `handling`),
  `claudine/cli` (`commands/signals.rs`, `commands/wrap/harness_orch/`),
  `claudine/docs/research/signals/`.
- Repo-wide guards that must stay green throughout: the dispatch-inventory
  drift test (`claudine-cli/tests/dispatch_inventory.rs` — no new
  decentralized `match Provider`), `claudine providers generate` drift check,
  and `claudine signals check` (currently 83/83/0; totals grow with new
  records).

---

## Phase 1 — Criticality Tier, Attestation Schema, Coverage Command

*Goal: the tier taxonomy exists in code, research docs can attest absence,
and `claudine signals coverage` renders the matrix — all visibility, no
gating yet.*

- [ ] Add `SignalCriticality { Critical, Expected, Standard, Exempt }` to
  `claudine/catalog-types/src/signal.rs` (serde + strum snake_case, matching
  the existing vocab enums' derive set).
- [ ] Add `SignalKind::criticality(billing: BillingModel, resume: ResumeSupport)
  -> SignalCriticality` encoding the ratified tier table (spec §1.1),
  including the three conditional gates:
  - [ ] `UsageCapped` critical iff `BillingModel::Subscription`, else expected;
  - [ ] `NoFunds` critical iff `BillingModel::PrepaidCredits`, else expected;
  - [ ] `SessionResumable` expected iff resume support ≠ `None`/`Unknown`,
    else standard.
- [ ] Frozen-table unit test mirroring spec §1.1 row-for-row (the same
  guard style as `signal_kind_member_list_is_frozen`), so a tier change is a
  conscious spec edit.
- [ ] Add `NotFoundAttestation` to `claudine/catalog-types/src/signal_table.rs`
  (`signal: SignalKind`, `searched: &[SearchSurface]`, `reason`,
  `confidence`, `verified_against`) and an `attestations:
  &'static [NotFoundAttestation]` field on `ProviderSignalTable` — stamped
  into the existing generated provider metadata, **not** a second runtime
  detection table (spec §1.3(b)).
- [ ] Extend `claudine/docs/research/signals/_schema.yaml` with the
  `not_found` array (spec §1.3(a) shape incl. required `verified_against`).
  Respect the SimplifiedSchema single-nesting constraint (flat entry objects).
- [ ] Teach `claudine/gen` (`gen/src/signals.rs` + `gen/src/inputs.rs`) to
  parse `not_found` entries, validate them (known critical/expected
  `SignalKind`, non-empty `verified_against`, no entry that *also* has a
  detection record for the same kind), and emit them into
  `lib/src/signals/generated.rs` via `gen/src/emit.rs`.
- [ ] Add `Coverage` subcommand to `SignalsCommand`
  (`claudine/cli/src/commands/signals.rs`): per-signal × per-provider matrix
  from the generated tables + each provider's generated
  `BillingModel`/`ResumeSupport` metadata. Cell states: `detected`,
  `attested-absent`, `GAP`; exempt-tier rows excluded entirely.
  - [ ] `--provider <fuzzy>` filter and `--json` output (stdout pipeable,
    status to stderr, per the output-system conventions).
  - [ ] Table styling via the shared table conventions (fill-to-margin,
    inverse inline code) used by `claudine context`.
- [ ] Generate and commit the baseline snapshot as
  `claudine/docs/signals-coverage.md` (marked as generated; regeneration
  command in a comment).
- [ ] Unit tests: gate arithmetic (`critical ∖ (records ∪ not_found)`)
  as a pure function shared with phase 3's generate-time gate; coverage cell
  classification; JSON shape.

**Validation checkpoint:**

- [ ] `just test` green in `claudine/` (workspace: `just test claudine` at
  root) plus `cargo nextest run -p claudine-catalog-types -p claudine-gen`.
- [ ] `claudine signals coverage` renders all non-exempt kinds × 10
  providers; known-good spot checks: Codex `ProviderOverloaded` = GAP,
  OpenCode `ProviderOverloaded` = detected, no `Timeout` row.
- [ ] `claudine providers generate` round-trips with zero drift (attestation
  field emits empty for all providers at this point).

## Phase 2 — Research Gap-Fill for Critical Cells (Codex Overload First)

*Goal: every critical-tier cell is either a detection record with committed
evidence or a `not_found` attestation — starting with the motivating Codex
`ProviderOverloaded` gap.*

- [ ] Update the signals fleet prompt
  (`claudine/docs/research/signals/_fleet.md`) per spec §1.3(a): for every
  critical/expected-tier signal the researcher must return a record or a
  `not_found` attestation with `verified_against`; forbid silent omission.
- [ ] **Codex `ProviderOverloaded`** (the motivating fix):
  - [ ] Source-confirm the current Codex `ServerOverloaded` → exec-stream
    message projection (the research doc pins `rust-v0.142.5`; re-verify at
    the installed version) and the observed "Selected model is at capacity"
    copy.
  - [ ] Add declarative record(s) on `type=error` / `type=turn.failed`
    message text (`substring_ci` on "at capacity", plus any source-confirmed
    overload copy), priority-ordered **after** the existing usage-cap records
    (intra-kind priority only — cross-kind arbitration is phase 5).
  - [ ] Commit scrubbed evidence fixture(s) under
    `claudine/docs/research/signals/fixtures/codex/`.
- [ ] Sweep the remaining critical-tier GAP cells from the phase 1 baseline,
  each resolving to record+fixture or attestation:
  - [ ] `ProviderOverloaded`: antigravity, claude, gemini, kimi, qwen —
    graduate existing stream-parser overload vocabulary
    (`stream/providers/{claude,pi,antigravity}.rs` `lower.contains("overloaded")`
    etc.) into detection records where the payload shape supports it
    (spec §2.5: one authority, no dual-source drift).
  - [ ] `AuthInvalid`: claude, codex, gemini.
  - [ ] `RateLimited`: antigravity, gemini.
  - [ ] `UsageCapped` / `NoFunds`: only providers whose generated
    `BillingModel` gate makes them critical; others stay expected-tier.
- [ ] Regenerate (`claudine providers generate`) and reconcile
  `signals check` totals (every new declarative record needs its replayable
  fixture).
- [ ] Note in each research doc's `changes:` list what was added and at which
  provider version (`verified_against` sourcing).

**Validation checkpoint:**

- [ ] `claudine signals coverage` shows **zero** critical-tier `GAP` cells
  (every cell `detected` or `attested-absent`).
- [ ] `claudine signals check` passes with `positives_passed == records`
  at the new, higher record count.
- [ ] Replaying the original incident line (`Selected model is at capacity.
  Please try a different model.`) through the Codex parser + signal engine
  yields a `provider_overloaded` observation (new unit test pinning this).

## Phase 3 — Generate-Time Gate, Evidence Rule, Staleness

*Goal: the coverage contract becomes enforced, not just visible.*

- [ ] Flip the hard gate in `claudine-gen`: `critical ∖ (records ∪ not_found)`
  non-empty for any provider → generate/check **error** naming the provider,
  signal, and remediation options; expected-tier gaps → warnings in the
  generate report (spec §1.3(b)). Wire into the existing CI drift-check code
  path.
- [ ] Extend `signals check`: a critical-tier detection record without a
  committed evidence fixture is a failure (spec §1.3(d)).
- [ ] Staleness: compare each attestation's `verified_against` major version
  against the installed provider's version (reuse the generated
  provider-version parsing); render **stale / re-verify** in
  `signals coverage`; unparseable versions are conservatively stale
  (spec §1.5). Staleness is non-blocking everywhere.
- [ ] Negative tests: a synthetic roster entry with a missing critical cell
  fails generate; a stale attestation renders stale but exits 0.

**Validation checkpoint:**

- [ ] `claudine providers generate --check` (drift mode) green on the real
  tree; red on the synthetic-gap fixture test.
- [ ] `just test` + `just lint` green in `claudine/` and `gen`.

## Phase 4 — Lifecycle Re-Ordering: `failure` Once, `finalize` Hoisted, `handler` Global

*Goal: the event-order contract of spec §2.1/§2.1a holds — before any
handling engine exists. Highest-blast-radius phase; land it alone.*

- [ ] **Parse-time placement**: `retry`/`resume`/`proxy` become invalid in a
  `finalize` stack (`LifecycleControlAction::is_valid_for` /
  `LifecycleActionPlacement` in `lib/src/composition/`), with a typed error
  and did-you-mean pointing at `failure` (Ruling #7). `stop`/`error` remain
  valid; `defer` remains parse-valid-everywhere (existing contract).
- [ ] **Hoist `finalize`** out of the attempt loop in
  `cli/src/commands/wrap/harness_orch/loop_control.rs`:
  - [ ] `run_finalize_with_recovery` → observational `run_finalize` (no
    `dispatch_terminal_control` on the finalize outcome; evaluation-error
    halt semantics unchanged).
  - [ ] `finalize` fires exactly once after the loop exits (success,
    exhausted failure, or abort), including the setup-failure routing
    helpers (`emit_blocked_finalize_with_err`, `emit_failure_finalize_with_err`)
    which must defer their finalize leg to the hoisted site.
  - [ ] Sequence path parity (`commands/wrap/sequence/`): same ordering per
    step.
- [ ] **`failure` fires once per handling saga** (Ruling #10): guard flag on
  `LifecycleRunGuard` — set at the first unrecovered failure; lifecycle-stack
  retries (prompt's own doing) reset it, config-driven re-entries (phase 5)
  will not.
- [ ] **`handler` late-binding global** (spec §2.1a):
  - [ ] `Handler` / `HandlingMeta` / `HandlerSource` / `MapSource` types in
    `lib/src/composition/lifecycle_context.rs` beside `LifecycleErrorInfo`,
    with the ratified fields (`strategy`, `attempts`, `waited`, `hops`,
    `resumed`, `map_source`) and stable snake_case serialization
    (`no_error` / `not_handled` / `handled`; sources `prompt` /
    `configuration`).
  - [ ] Falsey semantics: `NoError`/`NotHandled` are falsey, projections from
    them yield `null` (existing null-propagation rules).
  - [ ] Register `handler` in `LATE_BINDING_ROOTS`
    (`lib/src/composition/lifecycle/mod.rs`) and thread it through the DM2
    `SubtreeCompose` lookup like `err`/`timing`/`current`.
  - [ ] Populate `Handled { handler_source: prompt, .. }` when a lifecycle
    `retry`/`resume` control recovers (in `dispatch_terminal_control`'s
    `Retry`/`Resume` arms).
- [ ] Migration sweep: repo prompt files, fixtures, and tests using
  `finalize`-stack recovery move those controls to `failure` stacks; the
  spec's own examples already comply.
- [ ] Docs-with-code (drift rule): update
  `.claude/skills/claudine/{composition.md,validations-and-handlers.md}` and
  the lifecycle topic docs for the new ordering, `failure`-once, observational
  `finalize`, and the `handler` global; re-stamp skill `hash:` frontmatter
  via `md hash`.

**Validation checkpoint:**

- [ ] New L1 tests: event-order (start → failure-once → finalize-last),
  finalize-recovery parse error, `handler` truthiness/falsiness + null
  projection + snake_case names, prompt-retry populates
  `handler.source == 'prompt'`.
- [ ] Full `just test` and `just test-l2` in `claudine/` green (known blast
  radius: lifecycle L2 suites — fix, don't skip).

## Phase 5 — Handling Engine: Mechanical Strategies, Arbitration, Recovery Messages

*Goal: the incident class never recurs — a transient overload on any provider
with a detection record retries automatically under conservative defaults.*

- [ ] **Config surface** (`lib/src/config/claudine_config.rs` +
  `dispatch/loader.rs` merge): `handling` section with mechanical-family
  entries (`transient`, `throttled`) as tagged strategy objects
  (`fail_fast` / `delayed_retry` / `incremental_retry` / `wait_until_reset`;
  reserved `defer` parses but surfaces not-implemented), optional `message`
  override per entry (`""` suppresses), and the frontmatter escape
  `handling: false` (fail-fast everything). Durations reuse the timeout
  parser's human strings. `deny_unknown_fields` stays.
- [ ] **Disposition mapping**: hard-coded `SignalKind -> Disposition` fn
  (Review Decision #2) colocated with `Disposition` in
  `lib/src/diagnostics/facets.rs`, mirroring spec §2.2's table; frozen test.
- [ ] **New module `lib/src/handling/`**:
  - [ ] Attempt-local signal snapshot seam on `SignalHub` (immutable
    per-attempt drain for classification; run-level audit accumulation
    unchanged — Review Decision #9).
  - [ ] Cross-kind arbitration ladder exactly as spec §2.5:
    `Interrupted` → runaway/exit/taint guards → `AuthInvalid` → `NoFunds` →
    `UsageCapped` → session limits → `RateLimited` → transient; same-level
    tie → most recent; causal-terminality rule (failed attempt + matched
    terminal diagnostic; informational signals never trigger).
  - [ ] Strategy executor: attempt accounting where `max_attempts` includes
    the triggering launch (zero invalid); bounded full jitter in
    `[0, computed_delay]`; injectable clock + deterministic RNG for tests;
    waits routed through the existing signal-aware, Ctrl+C-respecting wait
    substrate (never bare `thread::sleep`); every wait clamped to the run's
    remaining wall-clock `timeout`.
  - [ ] `wait_until_reset`: single post-wait launch; no usable time →
    `fallback_delay` (default `15s`) then one launch; repeated throttle
    fails.
  - [ ] Built-in defaults: `Transient` → `incremental_retry` (5s, ×2, 4
    attempts, same agent+model); `Throttled` → `wait_until_reset`
    (`max_wait 10m`); `Unrecoverable`/`NeedsInput` → fail-fast
    (not configurable).
- [ ] **Bypass boundary** (Review Decision #11): engine not consulted for
  `claudine-contract`, user interruption, lifecycle `stop`/`skip`,
  policy/protect denials, schema/preparation failures, or any interactive
  session (Ruling #14). Detection/logging stay on in all bypass cases.
- [ ] **Loop integration** (`loop_control.rs`): post-`failure` fallthrough
  consults the engine (spec §2.1 diagram); re-entry resets per-iteration
  guard state, re-fires `start` (Review Decision #3), does **not** re-fire
  `failure`; `finalize` waits for saga end and receives populated
  `Handler::Handled { handler_source: configuration, .. }` /
  `NotHandled`.
- [ ] **Recovery messages** (spec §2.2b mechanism): message catalog keyed by
  `SignalKind` (all-empty in this phase — session-limit texts arrive with
  phase 6's typed enums); retry delivery appends to the re-submitted prompt;
  resume delivery is deferred to phase 6 (no resume-shaped mechanical
  strategy except `wait_until_reset`, which must thread it when resuming).
- [ ] **Stderr recovery notices**: styled `↻ retry 2/4 in 10s` lines via the
  existing recovery-report helper; suppressed by `--silent`; per-attempt
  story lives here, not in lifecycle events (Ruling #10).
- [ ] JSONL: summary row records handling outcome (strategy, attempts,
  waited) alongside the existing signals array.
- [ ] Docs-with-code: `.claude/skills/claudine/` (SKILL.md wrapper notes +
  composition/timeouts docs) gain the handling-layer contract; cli-reference
  for config keys; `md hash` re-stamp.

**Validation checkpoint:**

- [ ] Deterministic unit tests (injected clock/RNG): backoff schedule, jitter
  bounds, attempt accounting incl. triggering launch, wall-clock clamp,
  arbitration ladder ordering, causal-terminality, attempt-local snapshot
  isolation ("prior overload cannot retry a later unrelated failure").
- [ ] End-to-end L2: stub provider emitting the Codex capacity line then
  succeeding on re-launch → run succeeds with one `failure` event, one
  `finalize`, `handler.source == 'configuration'`, retry notice on stderr.
- [ ] Ctrl+C during a strategy wait exits promptly (signal-aware wait test).
- [ ] `claudine-contract` behavior byte-identical before/after (its tests
  untouched and green).

## Phase 6 — `ProviderMap` and Decision-Heavy Typed Enums

*Goal: cap/funds/auth/session-limit conditions get their typed, governed
recovery options.*

- [ ] **`ProviderMap` config** (`provider_maps` in both scopes): entry
  parse/validation; anchored-glob matcher (`*`/`?`, ASCII case-insensitive,
  **no fuzzy/contains** — a typo must not select a different provider);
  `agent` exact-canonical-or-`*`; default-map merge = repo-first entry
  concatenation, first-match wins, entries atomic (Review Decision #5);
  named maps repo-scope only.
- [ ] **Committed-revision reads** (Ruling #6 / Review Decision #4): named
  maps read the `.claudine/config.json` blob at current `HEAD` via `gix`
  (in-process, never shells out; repo discovery via `sniff`); untracked or
  absent at `HEAD` → no named-map match + warn; working-tree differs → warn
  that local changes were ignored. Add `gix` to `claudine` deps and update
  `claudine/docs/dependencies.md` + repo `docs/dependencies.md`.
- [ ] **Change-provider execution**: re-entry from prompt materialization
  against the new `(agent, model)`; never revisit an attempted pair + hop
  cap (mirroring `proxy_handoff_allowed`); styled hop notice
  (`↻ changing provider: codex/gpt-5.2 → claude (usage_capped)`); hop chain
  into `HandlingMeta.hops` + summary row; `map_source` audit
  (`repo-committed` / `repo` / `user`).
- [ ] **Typed enums** (config `option` entries per spec §2.2/§2.2a):
  - [ ] `UsageCapHandlingOptions`: `None` (default; teaching fail-fast
    message with `lifts_at`) / `ChangeProvider` (map-not-found → warn +
    fail fast) / `WaitForCap` (requires future valid `lifts_at`, else warn +
    fail fast; clamped to time limit + wall clock; resume-preferred with
    degrade-to-retry per Ruling #5) / `ChangeProviderElseWait`.
  - [ ] `NoFunds` / `AuthInvalid`: `None | ChangeProvider`; `AuthInvalid`
    reserves `Reauthenticate` (parses, surfaces not-implemented).
  - [ ] `SessionLimitHandlingOptions`: `None | Resume | ResumeFallback |
    Retry` (each `Option<u32>`, default 1, counts are *additional*
    continuations/retries per the review's counting note); `Resume` gates on
    provider resume support + live session id, falling back per variant.
  - [ ] Session-limit recovery-message defaults (the two §2.2b ratified
    texts): retry-append + resume-kick-off delivery.
- [ ] Frontmatter surface: `handling: { map: <name> }` document override.
- [ ] `claudine logs`: report change-provider hop frequency from the summary
  rows.
- [ ] Docs-with-code: config TUI left untouched (config-file-only for v1 —
  mirror the `harvest_unmatched` precedent); skill + cli-reference updates;
  `md hash` re-stamp.

**Validation checkpoint:**

- [ ] Matcher property tests (anchored, ASCII-CI, no substring leakage);
  merge-precedence tests (repo-first, atomic entries).
- [ ] Committed-blob tests against fixture repos (tracked+clean,
  tracked+dirty→warn+ignore, untracked→no match, absent at HEAD→no match);
  no network, no `git` subprocess (assert via test harness).
- [ ] Cycle-guard test: A→B→A map chain aborts with the typed error.
- [ ] End-to-end L2: stub provider emitting a usage-cap signal +
  `change_provider` map → hop notice, second provider runs,
  `handler.hops` populated, `finalize` fires once.

## Phase 7 — Attested-Absence Notices and Research Follow-Up Automation

*Goal: unresolvable gaps are visible at wrap time, and the research fleet
self-checks its critical coverage.*

- [ ] Wrap-time attested-absence notice (spec §1.5 / Review Decision #8): at
  most once per session, rendered into `Section::TrailerMetadata` alongside
  the badges; `--silent` suppresses; text names the signal and consequence
  ("codex cannot detect provider-overload conditions; …").
- [ ] `signals coverage --research-doc <path> --critical-only --quiet`
  probe mode (exit-code contract) for use inside research sequences.
- [ ] Pilot the lifecycle-driven gap follow-up on **one** topic run
  (spec §1.4): a `success`-stack `resume` control that re-asks the
  researcher for missing critical signals, budgeted `max_attempts: 2`;
  record findings in the feature directory before generalizing.
- [ ] On a successful pilot, fold the pattern into
  `claudine/docs/research/signals/_fleet.md`.

**Validation checkpoint:**

- [ ] L2: wrapped run against a provider with an attested-absent critical
  signal renders the trailer notice exactly once; `--silent` run renders
  none.
- [ ] Probe mode exit codes verified against a complete and an incomplete
  research doc fixture.

## Phase 8 — Full Validation and Drift Sweep

*Goal: everything green, every doc current, feature ready for `_completed`.*

- [ ] Root `just test claudine` + package-area `just test`, `just test-l2`,
  `just lint` across `claudine`, `claudine-cli`, `claudine-catalog-types`,
  `claudine-gen`, `claudine-contract`.
- [ ] `claudine signals check` and `claudine providers generate --check`
  green at final record/attestation counts; dispatch-inventory guard green.
- [ ] `claudine signals coverage`: zero critical GAPs; regenerate and commit
  the `claudine/docs/signals-coverage.md` snapshot.
- [ ] Drift-maintenance sweep (CLAUDE.md rule): `claudine/README.md`,
  `.claude/skills/claudine/` (SKILL.md, architecture.md, cli-reference.md,
  timeline.md entry, composition/timeouts topic docs), `docs/dependencies.md`
  (gix), all skill hashes re-stamped via `md hash`.
- [ ] Manual smoke of the original incident path: `claudine codex` compose
  run through a stub reproducing the capacity error → automatic retry →
  success; and (live, opportunistic) a real Codex run.
- [ ] Move the feature directory to `claudine/features/_completed/` (only
  after Ken's sign-off).

**Validation checkpoint:**

- [ ] All of the above checked off; no red anywhere in the touched crates.
