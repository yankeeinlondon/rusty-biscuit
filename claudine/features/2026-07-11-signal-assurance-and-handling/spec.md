---
created: 2026-07-11
reviewed: true
status: ready for planning and implementation
---

# Signal Assurance and Configurable Handling

## Motivating Incident

A non-interactive `claudine codex <prompt>` run failed with Codex's
server-side error:

> Selected model is at capacity. Please try a different model.

This is a **transient provider-capacity condition** — OpenAI's servers were
busy. It has nothing to do with the user's subscription caps. Two independent
gaps turned a 15-second blip into a dead run:

1. **Detection gap.** Codex has no `ProviderOverloaded` detection record, so
   the error classified as a generic agent error. The taxonomy member exists
   (`SignalKind::ProviderOverloaded`) and four providers already emit it
   (Claude†, OpenCode, Pi, Kilo) — Codex is a documented gap in
   `docs/research/signals/codex.md` ("`ServerOverloaded`… flattened to
   message text; needs committed exec JSONL fixtures").

2. **Handling gap.** Even where the signal *is* detected, Claudine does
   nothing with it. Signals are observational today: they land in the JSONL
   summary row and (via the synthesized `session_end`) the lifecycle `err`
   global. Recovery exists only when a prompt author writes a
   `failure`/`finalize` lifecycle stack with a `retry` control. There is no
   default: a transient overload kills every run whose prompt didn't opt in.

† Detection-record coverage per `lib/src/signals/generated.rs`; some
providers additionally classify overload vocabulary in their stream parsers
(`SemanticErrorKind`), but that layer feeds rendering, not signals.

This spec addresses both gaps as two workstreams that share one contract:
**every critical signal is either detected, researched and attested absent,
or loudly flagged — and every detected critical signal has a configurable
handling strategy with a sensible default.**

In this document, **signal** always means a diagnostic `SignalEvent`, not an
operating-system process signal such as `SIGINT`. Process-signal behavior is
an input to the handling engine's cancellation contract but is otherwise
unchanged.

---

## Part 1 — Signal Assurance (coverage as a contract)

### 1.1 The critical-signal tier

Today all 30 `SignalKind` members are equal citizens: a provider's signal
table contains whatever records the research fleet happened to find. Nothing
distinguishes "nice to have" (`ProviderVersion`) from "a wrapped run is
flying blind without it" (`ProviderOverloaded`).

We introduce a **criticality tier** on the taxonomy. A signal is *critical*
when **both** hold:

- **(a) Importance** — its absence materially degrades wrapped-run behavior
  (wrong terminal/retryable classification, silent money/cap burn, undetected
  auth failure).
- **(b) Near-certain existence** — we believe the condition almost surely
  *does* occur on the provider's platform and is observable on at least one
  of its output surfaces. (Every hosted LLM has capacity incidents; every
  authenticated CLI can hit an expired credential.)

Criterion (b) is what makes a coverage gap an *action item* rather than
noise: if the condition surely exists and we can't detect it, either the
research missed it or the provider genuinely doesn't surface it — and both
outcomes must be recorded, not silently tolerated.

#### Conditional criticality

Some signals are only near-certain for providers with a matching platform
shape. Criticality is therefore evaluated against generated provider
metadata rather than being a global boolean:

| Signal | Critical when… | Metadata gate |
|---|---|---|
| `UsageCapped` | provider bills by subscription/plan window | `BillingModel::Subscription` |
| `NoFunds` | provider bills by prepaid credits | `BillingModel::PrepaidCredits` |
| `SessionResumable` | provider can resume non-interactively | `ResumeSupport::FirstClass \| NonInteractiveOnly \| Partial` |

The gate reuses the existing `catalog-types` vocab (`BillingModel`,
`ResumeSupport`) that codegen already stamps into each provider's `data.rs`.

#### Tier assignments (RATIFIED 2026-07-11, all 30 kinds)

Four tiers: **critical** (gap blocks codegen unless attested-absent;
attested absence warns at wrap time), **expected** (gap reported in the
coverage matrix, non-blocking), **standard** (detected where found, no
obligation), **exempt** (Claudine-synthesized at the wrapper layer — never
a per-provider research target; excluded from the coverage matrix
entirely).

| Signal | Tier | Rationale |
|---|---|---|
| `ProviderOverloaded` | **critical** | The motivating incident; every hosted LLM has capacity events. Transient — misclassifying wastes runs. |
| `RateLimited` | **critical** | Universal on hosted APIs; carries `retry_after`/`reset_at` that `wait_until_reset` needs. |
| `UsageCapped` | **critical** *(gate: `BillingModel::Subscription`)* | Cap-vs-overload confusion is the costliest misread (opposite reactions). |
| `NoFunds` | **critical** *(gate: `BillingModel::PrepaidCredits`)* | Terminal for that provider; prime change-provider trigger. |
| `AuthInvalid` | **critical** | Every authenticated CLI can hit expired/revoked credentials; unhandled it masquerades as generic failure. |
| `TokensConsumed` | expected | Cost/usage observability we really want to uncover — for *logging* purposes more than handling (ruling: demoted from critical, 2026-07-11). Already 8/10. |
| `UsageCapApproaching` | expected | Valuable early warning; providers genuinely may not surface it (only Claude does). |
| `Interrupted` | expected | Useful in logs; the wrapper already infers Ctrl+C itself, so a provider-native record is a bonus. |
| `ModelResolved` | expected | Which model actually served the run — feeds catalog validation; some providers never state it. |
| `ModelFallback` | expected | Silent model substitution by the provider is worth knowing; several providers do it internally without exposing it. |
| `SessionResumable` | expected *(gate: `ResumeSupport` ≠ None/Unknown)* | Enables wait-and-*resume*; pointless where resume isn't supported. |
| `RetriesExhausted` | expected | The provider's own retry loop giving up — good context, not run-degrading if absent. |
| `GenerationRetried` | expected | Provider-native transient-retry visibility (Kimi `StepRetry`, Pi `auto_retry_*`); explains latency. |
| `TurnLimitReached` | expected | Provider-imposed session budget. **Must be config-handleable** (ruling note, 2026-07-11 — see §2.2a). |
| `SessionTimeLimitReached` | expected | Same as `TurnLimitReached`: expected tier + config-handleable (§2.2a). |
| `AuthKindDetected` | standard | Which auth method is active — diagnostic color only. |
| `PermissionDeniedRead` / `PermissionDeniedWrite` | standard | Real but provider-surfacing is rare (codex research found no exec-stream fixture); promoting would generate mostly attestations. |
| `ProviderVersion` | standard | Discoverable via CLI metadata outside the stream anyway. |
| `RepeatedStreamError` | standard | Bespoke cross-record counter, provider-specific value. |
| `SessionTainted` | standard | Bespoke cross-event rule (Goose error-then-complete); inherently a per-provider quirk. |
| `UnsupportedProtocolVersion` | standard | Wire-format drift guard; rare, provider-specific. |
| `HumanInputRequested` | standard | Reserved kind (no emitter until durable-HITL work); tier it when it's real. |
| `StalledGeneration` | **exempt** | Claudine-synthesized (the wrapper watches the stream and concludes "live but dead" — the provider never announces it). Currently OpenCode-scoped only because OpenCode's retry churn defeats the byte-silence `step_timeout`; see the generalization note below. |
| `ModelCatalogDrift` | **exempt** | Wrapper-computed catalog comparison. |
| `Timeout` / `StepTimeout` | **exempt** | Claudine's own timeout guards. |
| `ExitExpression` / `RunawayRepetition` / `RunawayVolume` | **exempt** | Claudine's content guards. (Qwen's native loop-detection record remains a standard-tier bonus where a provider offers one; the guard itself is Claudine's.) |

> **Follow-up (from the 2026-07-11 StalledGeneration review):** avoid
> provider-specific detection where the concept is provider-neutral. The
> stalled-generation guard should be generalized into a **progress clock**
> keyed on semantic progress events (`OutputText`/`Reasoning`/`ToolCall`/
> `ToolResult`/`FileChange`/…) that every provider's parser already emits —
> catching any "heartbeat-but-no-progress" pattern fleet-wide instead of
> only OpenCode's `llm_call_start` churn. Handling then keys on the signal
> kind (`Transient` family), never on the provider. Tracked as its own
> follow-up work item, not a blocker for this spec.

Tier membership lives in `catalog-types` next to `SignalKind` (e.g.
`SignalKind::criticality()` + conditional-gate metadata) so both the
research tooling and the runtime read one authority.

### 1.2 Current coverage baseline (2026-07-11)

Detection-record coverage across the 10 compiled providers, from
`lib/src/signals/generated.rs` (declarative + bespoke records; wrapper-level
bespoke guards like timeouts are not per-provider and excluded):

| Signal | Coverage | Missing providers |
|---|---|---|
| `RateLimited` | 8/10 | antigravity, gemini |
| `TokensConsumed` | 8/10 | antigravity, opencode |
| `AuthInvalid` | 7/10 | claude, codex, gemini |
| `ModelResolved` | 6/10 | antigravity, codex, goose, kimi |
| `ProviderOverloaded` | 4/10 | **codex**, antigravity, claude, gemini, kimi, qwen |
| `UsageCapped` | 4/10 | antigravity, claude, gemini, goose, kimi, qwen |
| `NoFunds` | 4/10 | antigravity, codex, gemini, kimi, opencode, qwen |
| `UsageCapApproaching` | 1/10 | all but claude |

(Full 30-row matrix to be emitted mechanically — see §1.4. Note the
surprises this table already surfaces: *Claude* lacks `UsageCapped` and
`AuthInvalid` detection records despite having cap-approaching ones, and
several "missing" cells have vocabulary sitting in stream parsers that never
graduated into detection records.)

### 1.3 Pipeline changes (research → attestation → codegen → gate)

The provider-metadata pipeline (fleet research with `_schema.yaml` sidecars
→ `claudine-gen` → generated tables → `signals check`) gains a coverage
obligation at each stage:

**(a) Research prompts name the critical set explicitly.** The
`docs/research/signals/` fleet prompt is amended: for every signal in the
critical/expected tiers the researcher must return *one of*:

- a detection record (as today), or
- a **`not_found` attestation** — a structured frontmatter entry stating
  what was searched (source code paths, docs, live probing), why the signal
  could not be grounded, and a confidence level. The existing free-text
  `gaps:` list is the prototype; this promotes the critical subset of it to
  schema-validated data the tooling can consume.

```yaml
# proposed sidecar addition (signals/_schema.yaml)
not_found:
  - signal: provider_overloaded          # must be a critical/expected kind
    searched: [source_code, docs, live_probe]
    reason: "exec --json flattens ServerOverloaded to message text; no
             stable discriminator found at rust-v0.142.5"
    confidence: source_code              # same ladder as records
    verified_against: "rust-v0.142.5"
```

**(b) Codegen enforces the contract.** `claudine-gen generate` (and the
CI drift check) computes, per provider: `critical ∖ (records ∪ not_found)`.
A non-empty set is a **generate-time error** — the pipeline refuses to
produce provider metadata when a provider neither detects nor attests a
critical signal. Expected-tier gaps produce warnings in the generate
report. `not_found` attestations are stamped into generated provider
metadata (not into a second runtime detection table), so the runtime knows
the difference between "absent, attested" and
"absent, unresearched" — relevant for §1.5 alerting).

**(c) A coverage report command.** `claudine signals coverage` renders the
matrix from §1.2 live from the generated tables: per-signal tier, per-
provider status (`detected` / `attested-absent` / `GAP`), with the same
styling conventions as the other reports. This is the human surface for the
same data the generate-time gate enforces.

**(d) `signals check` extension.** The existing evidence-replay gate
(83/83/0 today) additionally fails when a critical-tier record exists but
has no committed evidence fixture — a critical signal with an unproven
matcher is close to no signal at all.

### 1.4 Lifecycle-driven gap follow-up during fleet research

The user raised: we don't currently use lifecycle hooks during fleet
research to detect research gaps and resume the session to ask for missing
pieces. Assessment:

The research documents are schema-validated (`$schema: ./_schema.yaml`), so
a *structural* gap (missing frontmatter key) already fails the run. What we
don't do is *semantic* gap detection — "the document validates but attests
nothing about `provider_overloaded`."

This fits the existing lifecycle machinery well: the research sequence's
`success` stack can run a coverage probe (a small `claudine signals
coverage --provider <slug> --json` invocation against the freshly-written
doc) and use the `resume` control — which already exists and re-enters the
*same session* with a follow-up message — to ask the researcher agent
specifically for the missing signals before the sequence step is allowed to
succeed. Concretely:

```yaml
success:
  stack:
    - when: "{{ shell_ok('claudine signals coverage --research-doc ' + doc.path + ' --critical-only --quiet') == false }}"
      resume:
        message: "The research doc is missing critical-signal coverage for: … Investigate these specifically; if truly absent, add not_found attestations."
        max_attempts: 2
```

This is worth doing (it converts a human review loop into an automated one)
but is **phase-2 material**: the sidecar `not_found` schema and the coverage
command must exist first, and the pattern needs one manual pilot before
being baked into the fleet prompts.

### 1.5 User alerting for unresolvable gaps

When the full ladder has run — research prompt, resume-driven follow-up,
human review — and a critical signal still can't be grounded, the terminal
state is an **attested absence**, and the user must see it:

- `claudine providers` / `claudine signals coverage` render attested-absent
  critical cells with a warning glyph and the attestation reason.
- At **wrap time**, the first run of a provider with an attested-absent
  critical signal emits a one-line stderr notice (once per session, not per
  event): *"⚠ codex cannot detect provider-overload conditions; transient
  capacity errors will surface as generic failures."* Suppressed by
  `--silent`; never repeated mid-run.
- An *unattested* gap (should be impossible once the generate gate lands)
  renders as an error-severity cell — it means someone bypassed the
  pipeline.

Attestations are versioned evidence, not permanent waivers. Each
`not_found` entry must carry `verified_against`, using the provider version
already captured by the research document. When the installed provider's
major version exceeds that value, coverage reports render the attestation
as **stale / re-verify**. Staleness remains non-blocking so a provider update
cannot break unrelated CI, but it is included in the next fleet research
worklist. Versions are compared through the generated provider-version
parser; an installed or attested version that cannot produce a numeric
major is conservatively stale rather than silently current.

---

## Part 2 — Configurable Handling

### 2.1 Where handling sits (three layers)

```
1. Prompt lifecycle stacks   (most specific — author-owned, per-document)
2. Claudine configuration    (this spec — user/repo config, per signal-group)
3. Built-in defaults         (ship with Claudine; sensible, conservative)
```

Layer 1 already exists (the seven flow-control verbs, `retry`/`resume`/
`proxy`…). Layer 2 is new: **declarative, reusable handling policy** in
`.claudine/config.json`, so the 90% cases don't require every prompt file to
carry a recovery stack. Layer 3 is layer 2 with Claudine-shipped values.

> **RULING (2026-07-11): lifecycle-first, `finalize` last.** The error moves
> through the prompt's lifecycle events first — `failure` (or `blocked`)
> fires with its recovery-control dispatch, unchanged. Config handling is
> consulted only after no lifecycle control recovered the attempt. But
> **`finalize` is deferred until all config-based handling is done**: every
> other lifecycle event precedes config handling; `finalize` becomes the
> run's true epilogue, firing exactly once after the whole handling saga
> (lifecycle recovery attempts + config strategy attempts) has concluded —
> whether it concluded in eventual success or terminal failure.

The resulting event order for one run:

```
attempt N:  start → … → success | failure(+stack recovery dispatch)
                              │      ▲ failure fires ONCE — see ruling below
              no lifecycle recovery?
                              ▼
            config handling (retry / wait / change-provider)
              ├─ re-entry → attempt N+1 (failure NOT re-fired on config
              │             attempts; success fires if one succeeds)
              └─ exhausted / not applicable
                              ▼
            finalize  ← fires ONCE, with the `handler` global (§2.1a)
```

> **RULING (2026-07-11): `failure` fires once.** The `failure` event fires
> when the first unrecovered failure hands off to config handling — and is
> **not** re-fired by config-driven attempts that also fail. (Lifecycle-
> driven retries from a `failure` stack are unchanged: those attempts are
> the prompt's own doing.) The per-attempt story during config handling is
> carried by Claudine's own styled stderr recovery notices (*"↻ retry 2/4
> in 10s"*) — no author wiring needed, no notification storms possible. A
> config attempt that *succeeds* fires `success` normally (at most once by
> construction); `finalize` then reports the full saga via `handler`.

Mechanics:

- Today, `finalize` runs per iteration right after the terminal event. Under
  this ruling it is *hoisted out of the attempt loop*: the post-`failure`
  fallthrough consults config handling directly, and `finalize` runs after
  the loop exits for any reason.
- Notification noise is resolved structurally: `failure` fires once at the
  hand-off, `finalize` fires once at the end with full knowledge of what
  handling occurred (via `handler`).
- The shared attempt counter and the wall-clock `timeout` budget span both
  layers; lifecycle `retry` budgets and config strategy budgets are tracked
  separately but draw from the same clock.
- `start` re-fires on every config-driven attempt (per-attempt heartbeat,
  no error semantics — see Review Decision #3).

> **RULING (2026-07-11): `finalize` becomes purely observational.** With the
> hoisting, `finalize` **loses its recovery dispatch**: its stack may no
> longer carry `retry`/`resume`/`proxy` controls (a parse-time placement
> error, like `skip` outside `initialize` today). Recovery lives in two
> places only — `failure`/`blocked` stacks (layer 1) and config handling
> (layer 2). This removes the existing "last-chance recovery in
> `finalize`" capability deliberately: an epilogue that can re-enter the
> run isn't an epilogue, and post-handling recovery would either ping-pong
> with config handling or need a once-per-run guard. Existing prompts using
> `finalize`-stack recovery must move those controls to a `failure` stack
> (migration note for the implementation plan).

### 2.1a The `handler` global (deferred/late-binding)

A new late-binding global — alongside `err`, `timing`, `current` in
`LATE_BINDING_ROOTS` — reporting whether an error occurred and, if so,
whether anything handled it. Shape (pseudocode, from the 2026-07-11 ruling):

```rust
pub enum Handler {
    /// The run had no error to handle.
    NoError,
    /// An error occurred; neither prompt stacks nor config handling
    /// recovered it (terminal failure).
    NotHandled,
    /// An error occurred and was recovered.
    Handled(HandlingMeta),
}

pub struct HandlingMeta {
    /// Which layer recovered: "prompt" (lifecycle stack) | "configuration".
    handler_source: HandlerSource,
    /// The original error that triggered handling (the same classified
    /// shape the `err` global carries).
    underlying_error: Err,
    /// Human-readable account of what the handler did.
    msg: String,
    // Ratified additions (2026-07-11):
    /// Which strategy ran: "incremental_retry" | "wait_for_cap" | …
    strategy: &'static str,
    /// Total attempts consumed by handling.
    attempts: u32,
    /// Cumulative delay/wait time across the handling saga.
    waited: Option<Duration>,
    /// Provider-change chain, when any (renderable as
    /// "codex/gpt-5.2 → claude/sonnet").
    hops: Vec<(Agent, Model)>,
    /// Wait-and-resume actually resumed the session (vs. re-ran).
    resumed: bool,
    /// Audit trail for named-map substitution decisions:
    /// which config version supplied the map.
    map_source: Option<MapSource>, // "repo-committed" | "repo" | "user"
}
```

Expression surface: `handler` is truthy shorthand for "an error was
handled" (mirroring how a bare `err` works), with `handler.source`,
`handler.msg`, `handler.err.*` projections; `finalize` is its primary
consumer:

```yaml
finalize:
  stack:
    - when: "{{ handler && handler.source == 'configuration' }}"
      info: "recovered after {{ handler.msg }}"
    - when: "{{ err && !handler }}"
      message: "❌ run failed: {{ err.msg }}"
```

In events that fire *before* config handling (`failure`, `success` on a
retried attempt), `handler` reflects handling completed *so far* — e.g. a
`success` stack after a prompt-stack retry sees
`handler.source == 'prompt'`.

`Handler::NoError` and `Handler::NotHandled` are falsey; only
`Handler::Handled(_)` is truthy. Projections from either falsey state yield
`null`, following the existing late-binding null-propagation rules. The
serialized expression names are stable snake_case values (`no_error`,
`not_handled`, `handled`; sources `prompt`, `configuration`).

### 2.2 The grouping axis: dispositions, not signals

The user's instinct — map handlers to *groups* of signals, not per-signal —
is already half-built. Claudine's diagnostics facet layer defines:

```rust
/// What class of response could resolve the error.
pub enum Disposition {
    Transient,      // same action may succeed if retried now
    Throttled,      // will succeed later, at a known/estimable time
    Correctable,    // needs a different action; won't self-resolve
    NeedsInput,     // needs a human decision
    Unrecoverable,  // stop and surface
}
```

This is exactly the handler-grouping abstraction ("retry `Transient`,
wait-and-retry `Throttled`, surface the rest" — its own doc comment). The
missing piece is a ratified **signal-kind → disposition mapping**:

| Disposition | Signal kinds (handling-relevant subset) |
|---|---|
| `Transient` | `ProviderOverloaded`, `RepeatedStreamError`, `StalledGeneration` |
| `Throttled` | `RateLimited` (has `retry_after`/`reset_at`), `UsageCapped` (has `lifts_at`)* |
| `Correctable` | `UsageCapped`*, `NoFunds`, `AuthInvalid`†, `ModelCatalogDrift`, `TurnLimitReached`/`SessionTimeLimitReached` (session continuation — §2.2a) |
| `NeedsInput` | `HumanInputRequested`, `AuthInvalid`† (interactive re-auth) |
| `Unrecoverable` | `RunawayRepetition`, `RunawayVolume`, `ExitExpression`, `SessionTainted` |

\* `UsageCapped` is genuinely multi-natured, and all of its natures are
legitimate user choices the config must support (ruling, 2026-07-11):

  1. **Change provider** (short-term most common): move the non-interactive
     session to a different, non-capped agent/model — requires a
     `ProviderMap` (§2.4) so the user has expressed where a given
     agent/model failure should move.
  2. **Wait-and-resume**: when the cap's reset is close enough (the
     signal's `lifts_at` vs. a configured threshold), hold the session in a
     wait state and continue when the cap lifts.
     > **RULING (2026-07-11):** *resume* the interrupted session where the
     > provider supports non-interactive resume and a session id exists (a
     > cap can land mid-session with real work already done); **degrade to
     > retry** (re-run the prompt from scratch) where resumption is not an
     > option. Availability keys off the generated `ResumeSupport` metadata
     > plus the presence of a live session id.
  3. **Defer** (future): once the **Rendezvous** daemon matures, queue the
     prompt for scheduled re-execution (e.g. at `lifts_at`). This is the
     config-layer counterpart of the lifecycle `defer` verb, which already
     parses but returns `LifecycleDeferNotImplemented` pending the same
     backend.

The cap-handling choice is expressed as a typed option set rather than free
strategy composition (ruling sketch, 2026-07-11 — names non-final):

```rust
pub enum UsageCapHandlingOptions {
    /// No handling; fail fast. THE DEFAULT when nothing is configured
    /// (ruling, 2026-07-11). The failure message does the teaching: it
    /// includes the cap's reset time from the signal's `lifts_at`
    /// ("resets at 14:30") and a hint that `handling.usage_capped` can
    /// wait or change provider.
    None,
    /// Handle the cap by changing the provider using a ProviderMap.
    ///
    /// - Unnamed → the _default_ ProviderMap, looked up in both user and
    ///   repo scoped configurations.
    /// - Named → repo configuration ONLY (user scope ignored), with the
    ///   committed-revision constraint of §2.4b.
    /// - When the referenced map (default when unnamed) cannot be found —
    ///   or matches no entry for the failed agent/model — **warn + fail
    ///   fast** (ruling, 2026-07-11). Only `ChangeProviderElseWait`
    ///   degrades to waiting.
    ChangeProvider(Option<String>),
    /// Wait for the cap to lift, with an optional maximum wait in minutes.
    /// `None` = no time limit — wait until the cap-restore window.
    WaitForCap(Option<u32>),
    /// Change provider when a usable map entry exists; otherwise wait
    /// (this variant is the sanctioned fallback path a bare
    /// `ChangeProvider` deliberately does not have).
    ChangeProviderElseWait(Option<String>, Option<u32>),
}
```

`WaitForCap` requires a future, valid `lifts_at`. A missing, malformed, or
already-past reset time warns and fails fast; it never guesses a sleep
duration. The wait is clamped to the strategy's time limit and the run's
remaining wall-clock timeout. `ChangeProviderElseWait` follows the same rule
when it reaches its wait branch.

> **RULING (2026-07-11): sibling enums.** `NoFunds` and `AuthInvalid` get
> their own typed enums, both `None | ChangeProvider(Option<String>)` in
> v1 — no wait variants (credits don't replenish on a schedule; a broken
> credential won't self-heal). `AuthInvalid` reserves a `Reauthenticate`
> variant (pause for interactive re-auth, then continue) for the future —
> declared like the reserved `defer` strategy, surfacing not-implemented
> if selected, since it drags in TTY detection and provider-specific
> re-auth flows.

> **RULING (2026-07-11): config handling is non-interactive-only.** In an
> interactive wrapper session the human is the handler: the provider's own
> UI surfaces the error, and a wrapper that silently re-launched or
> switched providers under a live session would be deeply surprising.
> Signals are still *detected and logged* during interactive sessions —
> they simply never trigger strategies. Config handling applies to the
> non-interactive paths only (compose / inline-compose / sequence and
> non-interactive direct wraps).

† `AuthInvalid` is correctable via a provider change non-interactively, or
needs-input when a TTY is present.

> **RULING (2026-07-11): two config philosophies, split by family.**
> **Decision-heavy families** (caps, auth, funds — the `Correctable` cluster)
> are configured via **typed per-family option enums** in the
> `UsageCapHandlingOptions` mold: closed sets, self-documenting variants,
> combinators like `ChangeProviderElseWait` built in. **Mechanical
> families** (`Transient`, `Throttled` — where the options really are just
> retry/wait parameters) keep the **generic disposition-level strategy
> table** (§2.3). The generic `escalate` strategy is dropped from v1 — the
> typed combinators cover the chains that matter.

```jsonc
"handling": {
  // mechanical families — generic strategy entries:
  "transient":  { "strategy": "incremental_retry", "max_attempts": 4 },
  "throttled":  { "strategy": "wait_until_reset", "max_wait": "30m" },
  // decision-heavy families — typed per-family option enums:
  "usage_capped": { "option": "change_provider_else_wait",
                    "map": "org-approved", "time_limit": 30 },
  "auth_invalid": { "option": "change_provider" },   // unnamed → default map
  "no_funds":     { "option": "none" }
}
```

### 2.2a Session-limit handling (`TurnLimitReached` / `SessionTimeLimitReached`)

Ruling note (2026-07-11): these two are expected-tier for *detection*, but
configuration **must** be able to handle them. Both mean "the provider cut
the session off by budget, mid-work."

The right recovery hinges on one judgment (recorded with the ruling):
**did the incomplete session's partial work make the problem easier?** If
yes, a from-scratch *retry* faces an easier problem — and benefits from a
compact, fresh context window. If no, a retry will likely hit the same
wall (though it *may* complete anyway — this work is non-deterministic),
and *resuming* the session preserves what was done. Because the answer is
task-dependent, the enum offers both paths explicitly (RATIFIED
2026-07-11):

```rust
pub enum SessionLimitHandlingOptions {
    /// No handling; fail fast (the default).
    None,
    /// Resume the session with a continuation prompt, up to an optional
    /// maximum number of continuations (None = 1). Providers without
    /// resume capability fall back to FAIL FAST.
    Resume(Option<u32>),
    /// Same as `Resume`, except providers without resume capability fall
    /// back to RETRY (re-run from scratch).
    ResumeFallback(Option<u32>),
    /// Re-run the prompt from scratch, up to an optional maximum number
    /// of retries (None = 1). Useful when the first failed pass is
    /// expected to have completed part of the work, so the retry's more
    /// compact context window is a benefit rather than a loss.
    Retry(Option<u32>),
}
```

(Naming note from the ruling: `ResumeContinue` was rejected as awkward —
plain `Resume` reads better even though a sibling variant carries the
retry fallback.)

Distinct from `ChangeProvider` on purpose: a budget wall is not a provider
*failure*, and silently moving to a different agent/model mid-task for a
budget reason would violate the same conscious-decision principle as
Ruling #5.

### 2.2b Recovery instruction messages (ruling, 2026-07-11)

Every retry- or resume-shaped recovery benefits from telling the agent it
is *recovering from a previous failure* — otherwise a resumed session may
not realize it was cut off, and a retried prompt may redo (or trip over)
work the failed attempt already did. Mechanism:

- **Each signal kind carries a default recovery message** (catalog authored
  in the implementation plan; a kind whose recovery needs no message may
  default to empty).
- **Delivery differs by recovery shape:**
  - **Retry** — the prompt must be re-submitted anyway, so the message is
    **appended to the end of the re-submitted prompt**.
  - **Resume** — the original prompt is already in the session's context
    window, so the message **becomes the new prompt** that kicks off the
    resumed session.

Any handling entry may set an optional `message` override. `message: ""`
explicitly suppresses the catalog default. In v1 only the session-limit
family has non-empty defaults; overload and throttle errors commonly happen
before generation, so claiming that partial work exists would be misleading.

Ratified defaults for the session-limit family:

- Retry:
  > `\n> **IMPORTANT:** a previous attempt at completing this work did not
  > finish, but you should expect that some portions of the work were
  > already done.`
- Resume:
  > `We are resuming this work because it was prematurely terminated due to
  > session limits being hit before the task was able to adequately
  > complete the work.`

### 2.3 Mechanical handling strategies

Per the two-philosophies ruling (§2.2), this generic table now covers the
**mechanical families only** (`Transient`, `Throttled`); the decision-heavy
families (caps/auth/funds) use their typed option enums instead, and
`change_provider` as a *generic* strategy plus the `escalate` combinator
are **dropped from v1** (provider changes happen exclusively through the
typed enums' `ChangeProvider*` variants):

| Strategy | Behavior | Natural fit | Key parameters |
|---|---|---|---|
| `fail_fast` | No config-layer recovery. Surface immediately to lifecycle/terminal error. | any (it's the escape hatch) | — |
| `delayed_retry` | Wait a fixed delay, retry **once**, then fail. | `Transient` | `delay` (default `15s`) |
| `incremental_retry` | Retry with growing delay up to a cap. | `Transient` | `initial_delay` (`5s`), `multiplier` (`2.0`), `max_attempts` (`4`), `max_delay` (`2m`), `jitter` (bool) |
| `wait_until_reset` | Sleep until the signal's own `reset_at`/`retry_after`, then **resume** the session where possible, degrading to retry (§2.2 ruling). It permits one post-wait launch. If the payload has no usable time, wait `fallback_delay` and launch once; a repeated throttle then fails. | `Throttled` | `max_wait` (`30m` — beyond it, fail), `fallback_delay` (`15s`) |
| `defer` *(reserved)* | Queue the prompt with the Rendezvous daemon for scheduled re-execution. Config-layer counterpart of the lifecycle `defer` verb; surfaces not-implemented until the backend lands. | `Throttled` with a distant reset | `at` / `after` |

> **RULING (2026-07-11): transient retries stay on the same model.**
> `ProviderOverloaded`-class glitches are almost always very temporary;
> same-model retry with backoff restores working state. Switching models is
> switching *capability levels* and requires a conscious user decision — it
> is never an automatic escalation from a transient retry. (Codex's "try a
> different model" copy notwithstanding.) Provider changes are exclusively
> a typed-enum decision on the decision-heavy families — the mechanical
> table has no `change_provider` at all.

Built-in defaults (deliberately conservative):

- `Transient` → `incremental_retry` (initial `5s`, ×2, 4 attempts),
  same agent, same model — *this alone fixes the motivating incident for
  every provider once the detection records exist.*
- `Throttled` → `wait_until_reset` with `max_wait: 10m` — long waits should
  be a deliberate user choice, not a default surprise.
- Decision-heavy families → each typed enum's `None` variant (fail fast
  with a teaching message; ruling recorded on `UsageCapHandlingOptions` —
  changing provider changes which model bills, never silently by default).
- `NeedsInput` → `fail_fast` non-interactively; TTY prompting is future
  work (rendezvous/HITL).
- `Unrecoverable` → `fail_fast` always (not configurable — a runaway guard
  trip must never auto-retry; this preserves the existing
  `ProcessTermination::Aborted → AgentFailure` fail-fast ruling).

Guard interactions that must hold regardless of strategy:

- Every waited delay respects Ctrl+C (route through the existing
  signal-aware wait substrate, never a bare `thread::sleep`).
- The wall-clock `timeout` budget, when configured, spans retries — a
  retry strategy can't turn a 10-minute cap into 40 minutes.
- Retries reuse the existing harness attempt loop (`run_harness_loop`'s
  attempt counter, `reset_for_next_iteration`, budget arithmetic from
  `ControlBudgets`) rather than growing a parallel loop.

### 2.4 `ProviderMap`

For cap/funds/auth conditions, retrying the *same* provider is pointless —
the correct move is *a different agent and/or model*. A `ProviderMap`
(previously drafted as `FailoverMap`; renamed per the 2026-07-11 ruling
sketch) expresses that ordered preference:

```jsonc
// .claudine/config.json
"provider_maps": {
  "default": [
    { "when": { "agent": "codex" },                    "try": [ { "agent": "codex", "model": "gpt-5.2-codex" }, { "agent": "claude" } ] },
    { "when": { "agent": "claude", "model": "opus*" }, "try": [ { "agent": "claude", "model": "sonnet" }, { "agent": "opencode" } ] }
  ],
  "overnight": [
    { "when": { "agent": "*" }, "try": [ { "agent": "opencode", "model": "minimax-m2" } ] }
  ]
}
```

Match/walk semantics:

- **Entry match**: first `when` clause matching the *failed* `(agent,
  model)` wins. `agent` is either an exact canonical provider name or `*`;
  `model` is an anchored glob where `*` matches zero or more characters
  and `?` matches one character. Matching is ASCII case-insensitive, as
  model identifiers are. No fuzzy/contains matching is permitted in policy
  because a typo must not silently select a different provider. A map with
  no matching entry means no change.
- **Candidate walk**: candidates are tried in order. A candidate that fails
  with a condition in the same disposition group advances to the next; a
  candidate that fails otherwise surfaces that failure. Candidate omitting
  `model` uses the target agent's default resolution.
- **Re-entry point**: a provider change re-runs from prompt materialization
  (the same re-entry the lifecycle `retry` verb uses), so provider-specific
  frontmatter (`agent:`/`model:` keys) is re-resolved against the new
  target. The composed prompt *content* is identical.
- **Loop safety**: a chain never revisits an `(agent, model)` pair already
  attempted in this run (mirrors `proxy_handoff_allowed`'s cycle guard),
  and has a hop cap.
- **Transparency**: every hop emits a styled stderr notice (*"↻ changing
  provider: codex/gpt-5.2 → claude (usage_capped)"*), and the summary row
  records the full hop chain so `claudine logs` can report change-provider
  frequency per provider.

### 2.4a Default vs. named maps — scoping

- **Default map** (`ChangeProvider(None)`): resolved from **both** user and
  repo scoped configurations. Entry lists are concatenated repo-first and
  first-match wins. Entries are atomic; there is no entry-level deep merge.
- **Named map** (`ChangeProvider(Some("overnight"))`): resolved from the
  **repo configuration only**; user scope is ignored entirely.

### 2.4b Named-map committed-revision constraint

Named maps exist so a repository can govern agent/model substitution. A
named map is therefore read from the `.claudine/config.json` blob committed
at the current `HEAD`, never from the working-tree copy. If the file is
untracked or absent at `HEAD`, no named map matches and Claudine warns. If
the working-tree file differs, Claudine warns that local changes were
ignored. The default map remains a personal/local convenience surface and
uses normal merged configuration.

Repository discovery uses `sniff`; committed-blob access uses an in-process,
cross-platform git library and never shells out to `git`.

> **Reader's note — review revision:** the draft required a live fetch from
> the remote default branch when local state was dirty or unpushed. That
> design put network latency, credentials, remote naming, and offline
> failure into the recovery path and still could not prove organizational
> approval. Reading the current committed blob is deterministic, offline,
> cross-platform, and gives the runtime an honest contract: "committed in
> the revision being executed." Branch protection and review remain the
> repository host's responsibility. A future cryptographically attested
> policy bundle can strengthen this guarantee without changing map
> semantics.

### 2.5 Runtime wiring (the missing signal-to-handler bridge)

Today's signal flow ends in bookkeeping:

```
child output → detection engine → SignalHub → drained at summary → JSONL row
                                                    ↘ err global on session_end
```

Handling needs the *classification* available at the post-`failure` seam
(§2.1) where the harness decides a failed attempt got no lifecycle
recovery. Proposal: extend the existing `classify_failure` seam
(`harness/runtime.rs`) — which today maps `ProcessTermination` →
`FailureEvent` — into a disposition-aware classification that consults the
run's observed signals, evaluated when the `failure` event's control
dispatch falls through (with `finalize` deferred until handling concludes):

```
AttemptOutcome + drained ObservedSignals   (failure fired, no lifecycle
    → select the handling-relevant          recovery; finalize deferred)
      signal by explicit precedence
    → its Disposition
    → configured HandlingStrategy
    → Retry-with-delay | Wait-until | Change-provider hop | Surface
                              ▼
    finalize fires once, with the `handler` global populated
```

Notes:

- Signals are already available before summary emission (the hub is drained
  in the wrapper attempt path); the change is threading them into the
  attempt-outcome decision rather than past it.
- Detection-record `priority` is scoped to records of the same signal kind;
  it is not a cross-kind severity order. Handling uses this explicit
  precedence, from strongest safety constraint to weakest:
  `Interrupted` → runaway/exit/taint guards → `AuthInvalid` → `NoFunds` →
  `UsageCapped` → session limits → `RateLimited` → transient kinds. User
  interruption suppresses config handling entirely. If two kinds at the
  same level fire, the most recently observed event wins and all events
  remain in the summary for diagnosis.
- Selection is **attempt-local**. At attempt completion the hub produces an
  immutable snapshot for classification and separately appends those events
  to the run-level audit summary. The next attempt starts with an empty
  handling snapshot, so a prior overload cannot cause a later unrelated
  failure to retry.
- Config handling is eligible only when the attempt failed and its selected
  signal is causally terminal. Informational signals observed during an
  otherwise successful attempt never trigger handling. A provider's
  non-zero exit plus a matched terminal diagnostic is sufficient causality;
  an earlier diagnostic followed by a provider success is not.
- The stream parsers' `SemanticErrorKind` layer stays what it is
  (rendering); handling keys off signals only. Where a provider's overload
  vocabulary exists in a parser but not as a detection record, Part 1's
  audit graduates it into a record — one authority, no dual-source drift
  (the lesson from the code-block color-mode bug).

### 2.6 Configuration schema sketch

```jsonc
// ~/.claudine/config.json (user)  or  <repo>/.claudine/config.json (repo)
{
  "handling": {
    // mechanical families — generic strategy entries (§2.3):
    "transient":   { "strategy": "incremental_retry",
                     "initial_delay": "5s", "multiplier": 2.0,
                     "max_attempts": 4, "max_delay": "2m" },
    "throttled":   { "strategy": "wait_until_reset", "max_wait": "10m" },
    // decision-heavy families — typed option enums (§2.2):
    "usage_capped": { "option": "change_provider_else_wait",
                      "map": "org-approved",       // named → repo-only, committed at HEAD
                      "time_limit": 30 },          // minutes
    "no_funds":     { "option": "none" },
    "auth_invalid": { "option": "change_provider" }  // unnamed → default map
  },
  "provider_maps": { /* §2.4 */ }
}
```

- Repo config merges over user config per the existing loader semantics;
  `handling` follows the same key-wise merge as other config sections.
- Durations are the human strings the timeout system already parses
  (`15s`, `2m`, `30m`).
- `unrecoverable` is intentionally not a key — see §2.3.
- Frontmatter surface (per-document override) is deliberately minimal in
  v1: `handling: { map: <name> }` and `handling: false` (= `fail_fast`
  everything, "my lifecycle stacks own recovery").
- `max_attempts` means total provider launches in that strategy, including
  the failed launch that selected it. Consequently the built-in transient
  default of `4` permits at most three additional launches. Zero is invalid.
  The session-limit enum's continuation/retry counts remain *additional*
  attempts because those fields are explicitly named maximum continuations
  or retries.
- Retry jitter uses bounded full jitter in `[0, computed_delay]`; tests use
  an injectable clock and deterministic RNG. Waiting and backoff are async,
  cancellation-aware operations on every OS and must not block a runtime
  worker thread.
- Config handling is bypassed for user interruption, lifecycle `stop` or
  `skip`, policy/protect denial, schema/preparation failure, and the
  `claudine-contract` adapter. The adapter retains hard fail-fast behavior;
  its callers own latency and retry policy. Signal detection and logging
  remain enabled in all bypass cases.

---

## Phasing

| Phase | Contents | Unblocks |
|---|---|---|
| **A** | Criticality tier in `catalog-types`; sidecar `not_found` schema; `claudine signals coverage` command; coverage matrix baseline committed | visibility |
| **B** | Research fleet re-run for critical-signal gaps (Codex `ProviderOverloaded` first — the motivating fix); generate-time coverage gate; `signals check` evidence rule | detection |
| **C** | Disposition mapping ratified; mechanical-family `handling` config + strategy engine (`fail_fast`/`delayed_retry`/`incremental_retry`/`wait_until_reset`) wired into the harness attempt loop; `failure`-once + `finalize` hoisting + `handler` global; built-in defaults live | the incident never recurs |
| **D** | `ProviderMap` (including named-map committed-revision reads); typed option enums for the decision-heavy families (`UsageCapHandlingOptions` + siblings); logs reporting of hops | cap/auth resilience |
| **E** | Lifecycle-driven research gap follow-up (resume-based); wrap-time attested-absence notices | pipeline self-healing |

Phase C is deliberately shippable *before* provider maps: retry strategies
alone resolve the motivating incident class.

---

## Rulings so far (2026-07-11)

1. **Lifecycle-first precedence, `finalize` last** — the prompt's lifecycle
   events fire first and config handling is consulted only after no
   lifecycle control recovered; `finalize` is hoisted out of the attempt
   loop and fires exactly once, after all config handling concludes (§2.1).
2. **The `handler` late-binding global** — `NoError | NotHandled |
   Handled(HandlingMeta)` reports whether/how an error was handled
   (prompt vs. configuration), available to lifecycle expressions,
   primarily in `finalize` (§2.1a).
3. **`UsageCapped` is multi-strategy** — change provider to a non-capped
   agent/model (via `ProviderMap`) is the expected short-term common case;
   wait-and-resume is right when the reset is close; Rendezvous-backed
   `defer` is the future third option. Expressed as the typed
   `UsageCapHandlingOptions` enum (§2.2).
4. **Transient retries stay on the same model** — `ProviderOverloaded` is
   almost always a very temporary glitch; switching models is a capability
   change requiring a conscious decision, never an automatic escalation
   (§2.3).
5. **Wait-and-resume degrades to retry** — resume the interrupted session
   where the provider supports non-interactive resume; re-run from scratch
   where it does not (§2.2).
6. **Named `ProviderMap`s are revision-governed** — named maps resolve from
   the repo config committed at the current `HEAD`; working-tree changes are
   ignored with a warning. The default map is exempt (§2.4a/§2.4b).
7. **`finalize` is purely observational** — post-hoisting, `finalize`
   loses its recovery dispatch (parse-time placement error); recovery
   lives in `failure`/`blocked` stacks and config handling only (§2.1).
8. **Map-not-found → warn + fail fast** — a `ChangeProvider` whose map is
   missing or matches no entry fails fast; only `ChangeProviderElseWait`
   degrades to waiting (§2.2).
9. **`HandlingMeta` fields ratified** — `strategy`, `attempts`, `waited`,
   `hops`, `resumed`, `map_source` join the sketch's
   `handler_source`/`underlying_error`/`msg` (§2.1a).
10. **`failure` fires once** — at the hand-off to config handling; config-
    driven attempts never re-fire it. Per-attempt visibility is Claudine's
    styled stderr recovery notices (§2.1).
11. **`UsageCapped` default is `None`** — fail fast with a teaching
    message ("resets at HH:MM" + config hint) when nothing is configured
    (§2.2).
12. **Typed enums for decision-heavy families, generic table for
    mechanical ones** — caps/auth/funds get `UsageCapHandlingOptions`-mold
    enums; `Transient`/`Throttled` keep the generic strategy table;
    generic `change_provider` and `escalate` are dropped from v1 (§2.2,
    §2.3).
13. **Sibling enums ratified** — `NoFunds` and `AuthInvalid` are both
    `None | ChangeProvider` in v1 (no wait variants); `AuthInvalid`
    reserves `Reauthenticate` for future interactive re-auth (§2.2).
14. **Config handling is non-interactive-only** — interactive sessions
    detect and log signals but never trigger strategies; the human is the
    handler (§2.2).
15. **Tier table ratified, all 30 kinds** — with amendments:
    `TokensConsumed` demoted to expected (logging value, not handling);
    `StalledGeneration` reclassified exempt (Claudine-synthesized) with a
    follow-up to generalize the guard provider-neutrally via a semantic
    progress clock; `TurnLimitReached`/`SessionTimeLimitReached` expected
    **and** config-handleable via `SessionLimitHandlingOptions`
    (§1.1, §2.2a).
16. **`SessionLimitHandlingOptions` ratified** — `None | Resume |
    ResumeFallback | Retry` (each `Option<u32>`, default 1); the
    resume-vs-retry choice hinges on whether the incomplete session's
    partial work made the problem easier; `ResumeContinue` naming rejected
    (§2.2a).
17. **Recovery instruction messages** — every signal kind carries a
    default recovery message; retry appends it to the re-submitted prompt,
    resume sends it as the new kick-off prompt; session-limit defaults
    ratified (§2.2b).

## Review Decisions

1. **Recovery-message override surface (§2.2b).** An optional `message`
   key on any handling entry overrides the signal kind's default recovery
   message; `message: ""` explicitly suppresses it. Default-message
   catalog for v1: the **session-limit family** carries the §2.2b ratified
   texts; **all other kinds default to empty**. Rationale: a
   `ProviderOverloaded`/`RateLimited` refusal usually precedes generation
   (no partial work to warn about), and a wrong "some work was already
   done" hint is worse than none. When live experience shows a kind
   reliably fails mid-work, its default graduates from empty to authored —
   a one-line catalog change.

2. **Disposition-mapping overrides.** Hard-code the signal→disposition
   mapping in v1; no config override. The mapping is a *semantic claim*
   about what an error means, not a preference — letting config rebrand
   `RateLimited` as `Transient` would let a mis-edit turn every throttle
   into a hot retry loop against a provider asking us to slow down. The
   per-signal handling entries already give users the *effective* control
   they need (they can set `throttled.strategy` to whatever they want)
   without corrupting the taxonomy.

3. **`start` cadence on config re-entries.** `start` re-fires on every
   config-driven attempt. It is the per-attempt heartbeat with no error
   semantics, and suppressing it would break any `start`-anchored
   bookkeeping (e.g. a `set_frontmatter` marking "attempt began").
   Consistent with lifecycle-driven retries today. Only `failure` (once,
   Ruling #10) and `finalize` (once, Rulings #1/#7) are special-cased.

4. **Named-map revision mechanics (§2.4b).** Read only the
   `.claudine/config.json` blob at the current `HEAD`. Do not fetch or invoke
   credential helpers in the run path. Ignore and warn on working-tree
   differences; an absent committed blob means no named-map match. This is
   revision governance, not a claim that the revision was remotely reviewed.

5. **Default-map merge precedence.** Entry-list concatenation, **repo
   entries first**, first-matching-`when` wins. This keeps the org theme
   consistent (repo preferences take precedence) while letting a user's
   personal default map fill gaps the repo doesn't cover. No entry-level
   deep-merging — an entry is atomic.

6. **Attestation staleness.** A `not_found` attestation gains a required
   `verified_against: <provider version>` field (the version the research
   fleet ran against — already captured in research frontmatter). The
   coverage surfaces (`signals coverage`, generate report) mark an
   attestation **stale** when the installed provider's major version
   exceeds `verified_against`'s major. Stale ≠ gap: it stays non-blocking
   (codegen proceeds) but renders as a distinct "re-verify" state and is
   the trigger list for the next research fleet run. Automatic re-opening
   (hard gate) was considered and rejected: a major provider bump would
   otherwise break CI for every consumer overnight.

7. **Contract adapter.** `claudine-contract` **ignores config handling
   entirely** — hard `fail_fast` semantics, not merely defaulted but not
   consulted. A deterministic consumer (Reaper, Darkmatter) owns its own
   latency budget and retry policy at the `InferenceAdapter` call site;
   Claudine silently sleeping 10 minutes inside a "one text-inference
   operation" contract call would violate the contract's spirit. Signals
   are still detected and logged on contract runs (observability is free).
   If contract consumers later want transient-retry, it becomes an
   explicit knob on the adapter construction, not inherited config.

8. **Attested-absence notice placement.** The **trailer metadata section**
   — alongside the session badges, which already carry exactly this class
   of "operator should know" information, rendered by the same
   `Section::TrailerMetadata` machinery. A pre-execution stderr preamble
   was rejected: it would violate the CLI section-order contract (nothing
   before the execution header), and the notice is advisory, not
   actionable-before-run. `--silent` suppresses it; it renders at most
   once per session.

9. **Handling arbitration.** Matcher priority remains an intra-kind
   detection detail. Cross-kind handling uses the explicit safety-first
   precedence in §2.5, and only attempt-local terminal signals participate.

10. **Attempt counting and cancellation.** Mechanical `max_attempts`
    includes the triggering launch; waits use async cancellation-aware
    primitives with injectable time/randomness for deterministic tests.

11. **Embedding boundary.** `claudine-contract`, user interruptions,
    policy/protect denials, and preparation failures bypass config handling.
    This prevents hidden latency and prevents safety decisions from being
    reinterpreted as recoverable provider failures.

## Follow-up work items (out of scope for this spec)

- Generalize the stalled-generation guard into a provider-neutral semantic
  progress clock (§1.1 follow-up note).
- Rendezvous-backed `defer` (config strategy + lifecycle verb share the
  backend once it lands).
- `Reauthenticate` variant for `AuthInvalid` (interactive re-auth flows).
- Lifecycle-driven research-gap follow-up automation (§1.4, phase E) —
  pilot manually first.
