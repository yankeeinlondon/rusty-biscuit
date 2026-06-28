---
agent: open_code/zai-coding-plan/glm-5.2
phases: 3
created: 2026-06-24
start_phase: 1
yolo: "true"
packages:
    - claudine
source_files_during_phase_1:
    - claudine/lib/src/stream/logs/opencode/errors.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
    - claudine/fixes/2026-06-21-opencode-log-fix/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
    - claudine/lib/src/stream/logs/opencode/errors.rs
    - claudine/lib/src/stream/logs/opencode/reasoning.rs
documentation:
    - claudine/fixes/2026-06-21-opencode-log-fix/plan.md
    - claudine/fixes/2026-06-21-opencode-log-fix/spec.md
---

# Execution Plan — OpenCode 1.17.8 `stream error` Usage-Cap Detection Drift

Source spec: [`spec.md`](spec.md). All tasks trace to a spec section
(Design §1–§3, Goals 1–4, Tests 1–5, Acceptance Criteria 1–6) and to the
named files under `claudine/lib/src/stream/logs/opencode/`. File paths are
absolute-relative to the repo root.

> **Status note:** The fix described below is already implemented and
> committed in this worktree. At plan time the 143 `stream::logs::opencode`
> nextest cases pass (`cargo nextest run -p claudine stream::logs::opencode`).
> Treat each task as **verify-and-confirm** (check the box once the cited code
> matches the spec and the cited test is green), not implement-from-scratch.
> Any item that does *not* match becomes real work.

## Dependency graph at a glance

```
Phase 1 (errors.rs: §1 routing + §2 payload) ──┬──> Phase 3 (bridge terminal + closure)
                                               │
Phase 2 (reasoning.rs: §3 backstop) ───────────┘
```

- **Phase 2 is parallelizable with Phase 1** — the backstop lives in
  `reasoning.rs` and is independent of the `errors.rs` classification change.
  The backstop's only coupling is that a *recognized* cap (Phase 1) terminates
  on the first error, so the backstop only fires for *unrecognized* vocabularies
  (the residual case).
- **Phase 3 depends on Phase 1** — the semantic bridge's terminal abort (Goal 2,
  Test 4) is reachable only once `message="stream error"` classifies as a
  terminal `UsageCap`.

---

## Phase 1 — errors.rs: classify the new stream-error shape as a terminal UsageCap (Design §1, §2; Goals 1, 3; Tests 1–3)

Goal: a `message="stream error"` record with no `service=` tag and a flat
`error.error="…"` payload classifies as
`LogClassification::ProviderLimit { kind: UsageCap, .. }` with correct
`reset_at`, `provider_id`, `model_id`, and `provider_error`. Two coupled
changes in one file: (a) reach the failure classifier at all, and (b) read the
relocated error payload once there.

- [x] **1.1 (§1) Route `message="stream error"` into `classify_llm_failure`.**
  In `errors.rs` `classify(...)` (~`:31`), the LLM-failure path must not be
  gated solely on the literal `service` tag. Confirm an `effective_service`
  fallback reuses `infer_service_from_message` when `service` is absent
  (~`:36`–`:51`), and that `classify_llm_failure` is consulted when the
  effective service is `llm`/`provider`.
- [x] **1.2 (§1) Add the `"stream error"` arm to `infer_service_from_message`.**
  In `errors.rs` (~`:523`), confirm a `"stream error"` arm returns `"llm"` when
  both `providerID` and `modelID` are present, mirroring the existing `"stream"`
  (call-start) arm (~`:538`). This is what lets the *failure* record reach the
  failure classifier; the happy-path call start must stay `LlmCall`.
- [x] **1.3 (§2) Accept the flat `error.error` payload in `error_context`.**
  In `errors.rs` `error_context(...)` (~`:308`–`:321`), confirm `error.error`
  is a recognized source alongside `error`/`err`, and that surrounding quotes
  (present on the new flat-string form) are stripped so callers see the bare
  value. This is the single chokepoint `classify_llm_failure` uses for
  `has_error_context` and `provider_error`.
- [x] **1.4 (§2) Confirm the cap needle and reset-at extraction still match.**
  The `has_cap` scan in `classify_llm_failure` (~`:342`–`:346`) already runs
  over `record.raw`, so `Usage limit reached` matches the new
  `… for 5 hour …` dialect (the `for N hour` qualifier does not defeat the
  substring). Confirm `extract_reset_at(haystack)` (~`:711`) resolves
  `2026-06-22 13:59:38` from the new line, and that the cap-with-context branch
  (~`:355`–`:371`) populates `provider_error` from `error_context` (falling
  back to raw) plus `provider_id`/`model_id` from the `providerID`/`modelID`
  tags. Resolution order must stay: cap-with-context wins over
  retries-exhausted (~`:351`).
- [x] **1.5 (Impl. Notes) Note the OpenCode 1.17.8 drift source.** Confirm a
  code comment ties the routing change to OpenCode 1.17.8 and this fix
  directory (e.g. `errors.rs:37`–`:40` and the `infer_service_from_message`
  arm), so future drift is traceable.
- [x] **1.6 (Validation checkpoint — classification unit tests, Tests 1–3).**
  In the `errors.rs` test module, confirm: (1) the captured new-shape line
  classifies as `ProviderLimit { kind: UsageCap }` with `reset_at ==
  2026-06-22 13:59:38Z`, `provider_id == "zai-coding-plan"`,
  `model_id == "glm-5.2"`, and `provider_error` containing
  `Usage limit reached`; (2) the matching `message="stream"` call-start line
  still classifies as `LlmCall` (no regression); (3) the legacy
  `service=llm error={JSON … 1308 …}` fixtures still classify as `UsageCap`
  (backward compatibility). Run `cargo nextest run -p claudine stream::logs::opencode::errors`.

---

## Phase 2 — reasoning.rs: bounded abort for repeated unrecognized stream errors (Design §3; Goal 4; Test 5)

Goal: a future OpenCode format drift that the classifier does *not* recognize
as terminal must degrade to a bounded failure, not an indefinite hang. This is
defense-in-depth: with Phase 1 in place a known cap terminates on the first
error, so this backstop only catches the residual unclassified case.

> **Parallelizable:** 2.1–2.3 can proceed in parallel with Phase 1. The
> backstop is logic on the bridge and does not depend on the classification
> result; only its *test for unrecognized vocabulary* is independent.

- [x] **2.1 (§3) Track consecutive stream errors with a step-advance reset.**
  In `reasoning.rs`, confirm the bridge holds a `consecutive_stream_errors`
  counter (~`:218`–`:223`) that increments on each `message="stream error"`
  record with no intervening step advance, and resets to `0` on a genuine step
  transition in `on_step_loop` (~`:810`–`:812`). The reset is what stops a
  healthy retry storm from tripping the guard.
- [x] **2.2 (§3) Add the threshold constant and trigger.** Confirm
  `MAX_CONSECUTIVE_STREAM_ERRORS` (~`:226`–`:230`, proposed `5`) is a named
  constant with a comment tying it to this fix, and that `handle_structured`
  (~`:317`–`:324`) trips `on_repeated_stream_error` once the counter crosses the
  threshold and early termination has not already fired.
- [x] **2.3 (§3) Emit a terminal error + early-termination on the backstop.**
  Confirm `on_repeated_stream_error` (~`:953`–`:978`) emits a terminal
  `SemanticEvent::Error { terminal: true, kind: ApiRemote }` and fires
  `EarlyTermination::RepeatedStreamError { count }`, and that `is_stream_error`
  (~`:1000`–`:1003`) recognizes the keyword in both the trailing `message`
  field and the `message` tag.
- [x] **2.4 (Validation checkpoint — backstop unit test, Test 5).** In the
  `reasoning.rs` test module, confirm a synthetic *unrecognized* `stream error`
  vocabulary (no cap/429/auth needles) emits a terminal error and an
  `EarlyTermination::RepeatedStreamError` once the threshold is crossed, that
  fewer than the threshold does not, and that a genuine step advance between
  errors resets the counter so a fresh sub-threshold run stays quiet. Run
  `cargo nextest run -p claudine stream::logs::opencode::reasoning`.

---

## Phase 3 — Semantic-bridge terminal verification & closure (Goal 2; Test 4; Acceptance Criteria 1–6)

Goal: prove the classification drives the *existing* terminal early-termination
path so the wrapper aborts on the first cap error instead of spinning on
`Awaiting subagent`, then close out against every acceptance criterion.

- [x] **3.1 (Goal 2) Verify the bridge aborts on the first cap error.** Confirm
  `on_provider_limit` marks `UsageCap` (and `RetriesExhausted`) terminal
  (`reasoning.rs:435`–`:438`) and emits `SemanticEvent::Error { terminal: true,
  kind: ApiRemote }` plus `fire_early_termination(EarlyTermination::RateLimit)`
  (`reasoning.rs:481`–`487`), idempotent via `early_terminate_fired`. The terminal abort
  must fire even when stdout has already been seen (so the wrapper does not
  continue streaming after the cap).
- [x] **3.2 (Validation checkpoint — semantic bridge unit test, Test 4).** In
  the `reasoning.rs` test module, confirm ingesting the new `stream error` line
  emits a terminal `SemanticEvent::Error { terminal: true }` requesting early
  termination — mirroring the existing early-termination-for-`UsageCap` test
  shape, including the stdout-already-seen case. Run
  `cargo nextest run -p claudine stream::logs::opencode::reasoning`.
- [x] **3.3 (Manual validation) Replay the captured log line.** Record one
  manual check against the captured session line (or a replay fixture derived
  from it) confirming the wrapper aborts on the first cap error instead of
  spinning on `Awaiting subagent` for ~42 minutes. Source line is in
  `~/.local/share/opencode/log/opencode.log`, session `ses_1127ec2f`.
- [x] **3.4 (Acceptance Criteria 1–5) Walk the criteria.** Confirm: (1) the
  captured line classifies as `ProviderLimit { kind: UsageCap }` with correct
  `reset_at`/`provider_id`/`model_id`; (2) that drives a terminal
  `SemanticEvent::Error` and first-error termination; (3) the `message="stream"`
  call-start path is unchanged; (4) legacy `service=llm error={JSON}` fixtures
  still classify as `UsageCap`; (5) repeated unrecognized terminal stream errors
  trip the bounded backstop rather than hanging.
- [x] **3.5 (Acceptance Criterion 6) Full test + lint sweep.** Run
  `just test` in the `claudine/` package area (lib + contract + CLI) and
  `just lint`. Confirm `cargo fmt --check` (read-only) reports no drift
  introduced by this change. Per repo policy, never run write-mode `cargo fmt`.

> Depends on: Phase 1 (classification must produce the terminal `UsageCap`).

---

## Notes for the implementer

- **Never run `cargo fmt` write-mode** — match surrounding style by hand
  (repo policy in `AGENTS.md`). `cargo fmt --check` (read-only) is fine for
  diagnosis.
- **Reuse, don't duplicate:** prefer the existing `infer_service_from_message`,
  `error_context`, `extract_reset_at`, and the raw-haystack cap-needle scan
  over adding parallel matchers.
- **Comment quality:** any edit that changes a symbol's behavior must include a
  pass over its `///`/`//!` docs and inline `//` comments; fix or delete drifted
  ones in the same change. When drift is detected, assume the code is correct
  and the comment is wrong. Note the OpenCode 1.17.8 version where the drift
  originated.
- **Non-interactive session:** export `GIT_TERMINAL_PROMPT=0` before any git
  command; never run credential prompts, `gpg`, `ssh-add`, `sudo`, or
  background `&` shells. If a shell command does not complete within ~60s,
  abandon it.
- **Testing:** use `just test` (unit) inside the `claudine/` package area;
  `just test claudine` from the repo root. Nextest is the runner.
- **Non-Goals (do not do):** do not change the happy-path `message="stream"`
  call-start handling; do not redesign the `LogClassification` taxonomy or the
  semantic-event bridge; do not alter `step_timeout`/`timeout` precedence or
  env-var semantics beyond the targeted backstop; do not add provider-specific
  cap vocabulary beyond the observed ZAI `Usage limit reached for N hour`
  dialect (the existing substring needle already covers the core phrase).
