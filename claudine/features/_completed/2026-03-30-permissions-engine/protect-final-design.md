# ProtectService Final Design

This is the implementation-ready design for the rewritten `ProtectService`, finalized after the `PolicyEngine` implementation landed.

It supersedes the two prior design documents:

- `protect-functional-design.md` (functional responsibilities)
- `protect-technical-design.md` (technical structure)

and incorporates findings from the implementation review in `review.md`.

## What Changed Since the Prior Designs

The prior designs were written before `PolicyEngine` existed. Now that the engine is implemented (~15,000 lines across 20 files, 8 provider backends, full query/mutation/explanation surface), several assumptions can be tightened or corrected.

### 1. PolicyEngine API is richer than assumed

The implemented engine exposes typed snapshots (`ConfiguredPolicySnapshot`, `EffectivePolicySnapshot`) with 11 convenience query methods each (`can_read`, `can_write`, `can_traverse`, `can_execute`, `can_access_domain`, `can_use_mcp_server`, `can_use_mcp_tool`, `can_spawn_subagent`, `can_switch_mode`, `can_modify_own_config`, plus generic `query`). Every query returns `QueryResult` with `effect`, `certainty`, `stability`, `matched_rules`, `explanation`, and `warnings`. Protect can consume these directly instead of building its own matching.

### 2. All eight providers have backends

The prior designs anticipated phased provider rollout. All eight providers (Claude, Codex, Gemini, OpenCode, Qwen, Roo, Kimi, Goose) now have backends with varying fidelity. Protect does not need provider-specific permission fallbacks.

### 3. Native payload types are opaque

`NativePolicyLayer`, `NativeEffectivePolicy`, and `ProviderCliOverrides` use `Box<dyn Any + Send + Sync>` internally. Protect has no reason or ability to inspect native payloads. This confirms the clean boundary: Protect asks canonical questions, not native ones.

### 4. Review findings affect Protect design

The implementation review identified:

- Trust-gated repo config may load when trust is unknown (P1)
- Relative paths are not canonicalized against `cwd` (P2)
- `QueryStability` and warnings are largely unimplemented (P2)

These mean Protect must be defensive about `QueryResult` quality in v1. Specifically:

- Protect must check `certainty` and `stability` and not treat `Unknown` or `BestEffort` answers as authoritative
- Protect must normalize paths before querying when possible
- Protect should propagate policy warnings into decision records

### 5. Provider observation replaces generic event parsing

The prior technical design proposed `ProviderAdapter::observe_protect(...)`. This remains the right call because `ProtectInput::from_event_meta(...)` uses shared heuristics that do not understand provider-specific tool call shapes, MCP payload formats, or subagent launch patterns.

## Design Summary

The rewritten `ProtectService` is a runtime decision layer with this pipeline:

1. Receive a live hook event with provider-aware observations.
2. Resolve an effective (or configured fallback) `PolicyEngine` snapshot.
3. Derive one or more typed intents from the observation.
4. Query the snapshot for each intent.
5. Combine policy answers with Claudine runtime posture, risk classification, and provider enforcement capabilities.
6. Return a single runtime outcome with structured evidence and optional redaction plan.

Protect owns runtime judgment. PolicyEngine owns permission truth.

## Goals

1. Use `PolicyEngine` as the sole permission authority.
2. Remove duplicated permission truth from `ProtectConfig` (command patterns, path lists, MCP allow/deny lists).
3. Replace generic event-to-policy inference with provider-aware intent extraction.
4. Preserve hook-time runtime control outcomes and capability downgrade behavior.
5. Surface structured evidence and policy provenance in every decision.
6. Support pre-action and post-action evaluation.
7. Integrate redaction into dispatch as a concrete plan, not disconnected helper methods.
8. Keep the service testable against mocked snapshots without provider config parsing.

## Non-Goals

1. Moving permission parsing back into `ProtectService`.
2. Auto-writing provider config during hook evaluation.
3. Replacing or wrapping `PolicyEngine` query/mutation APIs.
4. Designing a user-facing `claudine protect ...` CLI in this phase.
5. Solving every provider's intent-extraction edge case in one pass.

## Module Layout

```txt
claudine/lib/src/services/protect/
  mod.rs
  service.rs
  config.rs
  request.rs
  observe.rs
  intent.rs
  evaluate.rs
  decision.rs
  downgrade.rs
  redact.rs
  state.rs
  explain.rs
```

### Responsibilities

- `mod.rs` -- re-exports, module wiring
- `service.rs` -- `ProtectService` struct, top-level `evaluate` and `evaluate_event` APIs, snapshot caching
- `config.rs` -- runtime-only `ProtectConfig`, provider overrides, validation, merge logic
- `request.rs` -- `ProtectRequest`, `ProtectSessionContext`, `ProtectCliContext`
- `observe.rs` -- `ProtectObservation`, `RuntimeFacts`, `ProtectPayload`
- `intent.rs` -- `ProtectIntent` enum (maps 1:1 to `PolicyQuery` variants)
- `evaluate.rs` -- policy querying, intent scoring, decision synthesis
- `decision.rs` -- `ProtectDecision`, `ProtectOutcome`, `ProtectEvaluation`, `ProtectFinding`
- `downgrade.rs` -- provider capability downgrade rules
- `redact.rs` -- MCP text/JSON redaction, `ProtectRedactionPlan`
- `state.rs` -- recent records, completion retry tracking, export
- `explain.rs` -- human-readable and machine-readable explanation rendering

## Public API

```rust
pub struct ProtectService {
    engine: Arc<PolicyEngine>,
    config: ProtectConfig,
    state: ProtectState,
}

impl ProtectService {
    pub fn new(engine: Arc<PolicyEngine>, config: ProtectConfig) -> Self;

    pub fn config(&self) -> &ProtectConfig;

    /// Full structured evaluation from a pre-built request.
    pub fn evaluate(
        &mut self,
        request: &ProtectRequest,
    ) -> Result<ProtectEvaluation>;

    /// Convenience entry point for dispatch. Builds observation from adapter,
    /// constructs request, and evaluates.
    pub fn evaluate_event(
        &mut self,
        provider: Provider,
        event: AgenticEvent,
        meta: &EventMeta,
        ctx: &ProtectSessionContext,
        adapter: &dyn ProviderAdapter,
    ) -> Result<Option<ProtectEvaluation>>;

    pub fn state(&self) -> &ProtectState;

    pub fn export_state(&self) -> ProtectStateExport;
}
```

### Why `ProtectEvaluation` replaces `ProtectDecision` as the top-level return

The current `ProtectDecision` carries only an outcome, a reason string, and degradation info. That is too thin. The new top-level return includes structured findings, policy mode, redaction plan, and warnings:

```rust
pub struct ProtectEvaluation {
    pub decision: ProtectDecision,
    pub policy_mode: ProtectPolicyMode,
    pub findings: Vec<ProtectFinding>,
    pub redaction: Option<ProtectRedactionPlan>,
    pub warnings: Vec<PolicyWarning>,
}

pub enum ProtectPolicyMode {
    /// Used effective policy (configured + CLI overrides).
    Effective,
    /// CLI context was unavailable; used configured-only policy.
    ConfiguredFallback,
}
```

`ProtectDecision` itself remains compact for dispatch short-circuit checks:

```rust
pub struct ProtectDecision {
    pub outcome: ProtectOutcome,
    pub desired_outcome: ProtectOutcome,
    pub degraded: bool,
    pub reason: String,
    pub capability: Option<GateCapability>,
}
```

## Context Types

### `ProtectSessionContext`

Carries the bridge between wrapper/dispatch state and `PolicyEngine`:

```rust
pub struct ProtectSessionContext {
    pub provider: Provider,
    pub policy_context: PolicyContext,
    pub cli: ProtectCliContext,
    pub interactive: bool,
    pub yolo: bool,
    pub session_id: Option<String>,
}

pub enum ProtectCliContext {
    None,
    Argv(Vec<String>),
    Parsed(ProviderCliOverrides),
}
```

Construction rules:

- Wrapper-launched sessions use `AGENT_PARAMS` for `Argv`.
- Non-wrapper hook invocations fall back to `ProtectCliContext::None`.
- When CLI data is absent, Protect requests a `ConfiguredPolicySnapshot` and marks the result as `ConfiguredFallback`.
- `PolicyContext` is built from `EventMeta.env` (cwd, repo root), trust context, and home dir.

### `ProtectRequest`

```rust
pub struct ProtectRequest {
    pub provider: Provider,
    pub event: AgenticEvent,
    pub phase: ProtectPhase,
    pub session: ProtectSessionContext,
    pub observation: ProtectObservation,
}
```

## Provider-Aware Observation

The service should stop trying to interpret every provider payload generically. Instead, each adapter supplies a normalized observation.

### New adapter method

```rust
fn observe_protect(
    &self,
    event: &AgenticEvent,
    meta: &EventMeta,
) -> Option<ProtectObservation>;
```

This replaces `ProtectInput::from_event_meta(...)`.

### Observation shape

```rust
pub struct ProtectObservation {
    pub summary: Option<String>,
    pub intents: Vec<ProtectIntent>,
    pub runtime: RuntimeFacts,
    pub payload: Option<ProtectPayload>,
}

pub struct RuntimeFacts {
    pub is_root: bool,
    pub has_sandbox: Option<bool>,
    pub bypass_mode: bool,
}

pub enum ProtectPayload {
    McpText(String),
    McpJson(Value),
}
```

### Intent types

`ProtectIntent` maps directly to `PolicyQuery` variants. This is intentional: Protect should reuse PolicyEngine's query vocabulary rather than inventing a parallel one.

```rust
pub enum ProtectIntent {
    ReadPath(PathQuery),
    WritePath(PathQuery),
    TraversePath(PathQuery),
    ExecuteCommand(CommandQuery),
    AccessDomain(DomainQuery),
    UseMcpServer { server: String },
    UseMcpTool { server: String, tool: String },
    SpawnSubagent { name: Option<String> },
    SwitchMode { target: Option<String> },
    ModifyProviderConfig,
    CompletionOutputScan,
}
```

Notes:

- `PathQuery`, `CommandQuery`, `DomainQuery` are imported from `crate::permissions::query`.
- `CompletionOutputScan` has no PolicyEngine equivalent. It triggers Protect's own secret-scan and loop-detection logic.
- One event may yield multiple intents. For example, an MCP tool call might produce `UseMcpServer`, `UseMcpTool`, and `AccessDomain`.

### Conversion to PolicyQuery

Each `ProtectIntent` converts to a `PolicyQuery` for snapshot querying:

| ProtectIntent | PolicyQuery | Notes |
|---|---|---|
| `ReadPath(pq)` | `PolicyQuery::ReadPath(pq)` | Direct |
| `WritePath(pq)` | `PolicyQuery::WritePath(pq)` | Direct |
| `TraversePath(pq)` | `PolicyQuery::TraversePath(pq)` | Direct |
| `ExecuteCommand(cq)` | `PolicyQuery::ExecuteCommand(cq)` | Direct |
| `AccessDomain(dq)` | `PolicyQuery::AccessDomain(dq)` | Direct |
| `UseMcpServer { server }` | `PolicyQuery::UseMcpServer { server }` | Direct |
| `UseMcpTool { server, tool }` | `PolicyQuery::UseMcpTool { server, tool }` | Direct |
| `SpawnSubagent { name }` | `PolicyQuery::SpawnSubagent { name }` | Direct |
| `SwitchMode { target }` | `PolicyQuery::SwitchMode { target }` | Direct |
| `ModifyProviderConfig` | `PolicyQuery::ModifyProviderConfig` | Direct |
| `CompletionOutputScan` | N/A | Handled by Protect-owned logic |

## Snapshot Resolution

For each request, Protect resolves one snapshot per evaluation:

1. If `ProtectCliContext` has argv or parsed overrides, request `EffectivePolicySnapshot` via `engine.effective(provider, &policy_ctx, cli_input)`.
2. Otherwise request `ConfiguredPolicySnapshot` via `engine.configured(provider, &policy_ctx)`.
3. Record which mode was used in `ProtectPolicyMode`.
4. Cache the snapshot for the rest of that evaluation (all intents query the same snapshot).

Within a single service instance, snapshot caching should be keyed by:

- provider
- cwd
- repo_root
- trust context hash
- CLI fingerprint (hash of argv if present)

This avoids reloading policy across pre-action and post-action evaluations in the same hook process.

## Runtime Config Redesign

### Fields to remove from `ProtectConfig`

These duplicate PolicyEngine truth:

- `rules.blocked_command_patterns`
- `rules.ask_command_patterns`
- `rules.protected_paths`
- `mcp.allowlist`
- `mcp.denylist`
- `subagents.enabled` / `subagents.tighten_permissions` / `subagents.default_profile`

### Fields to keep

These are runtime rails, not permission truth:

- `enabled`
- `posture` (Advisory, Balanced, Strict)
- `allow_repo_posture_downgrade`
- `yolo` (YoloPolicy)
- `completion` (CompletionPolicy)
- `max_recent_decisions`
- Provider-specific posture overrides

### New fields

```rust
pub struct ProtectConfig {
    pub enabled: bool,
    pub posture: ProtectPosture,
    pub allow_repo_posture_downgrade: bool,
    pub yolo: YoloPolicy,
    pub completion: CompletionPolicy,
    pub mcp_redaction: McpRedactionPolicy,
    pub runtime_guards: RuntimeGuardPolicy,
    pub providers: HashMap<Provider, ProviderProtectOverride>,
    pub max_recent_decisions: u16,
}

pub struct RuntimeGuardPolicy {
    pub deny_when_root_without_sandbox: bool,
    pub ask_on_unknown_write: bool,
    pub ask_on_unknown_command: bool,
    pub ask_on_provider_config_mutation: bool,
}

pub struct McpRedactionPolicy {
    pub redact_patterns: Vec<String>,
    pub block_instruction_payloads: bool,
    pub secret_patterns: Vec<String>,
}
```

### Migration

Existing user configs with removed fields must produce a targeted validation error at load time, not silently ignore the fields. The error should explain that provider-native permission config is now the source of truth and point to the relevant provider config location.

## Decision Pipeline

### Step 1. Resolve provider runtime posture

From:

- `ProtectConfig.posture`
- provider-specific override (if any)
- session `interactive` and `yolo` flags
- adapter enforcement capabilities

### Step 2. Resolve policy snapshot

As described in Snapshot Resolution above.

### Step 3. Query each extracted intent

For each `ProtectIntent`, call the matching query on the snapshot. For intents that map to `PolicyQuery`, this is a direct call:

```rust
let result = snapshot.query(&intent.to_policy_query());
```

For `CompletionOutputScan`, use Protect-owned logic (secret scanning, loop detection).

### Step 4. Build findings

Each query result becomes a `ProtectFinding`:

```rust
pub struct ProtectFinding {
    pub intent: ProtectIntent,
    pub result: QueryResult,
    pub severity: ProtectSeverity,
    pub source: ProtectFindingSource,
}

pub enum ProtectFindingSource {
    PolicyQuery,
    RuntimeGuard,
    McpRedaction,
    CompletionLoop,
}

pub enum ProtectSeverity {
    Info,
    Medium,
    High,
    Critical,
}
```

Severity derives from:

- the policy effect (`Deny` -> High/Critical, `Ask` -> Medium, `Allow` -> Info)
- the operation type (config mutation, destructive commands -> elevated)
- certainty/fidelity (low-confidence answers may be elevated under strict posture)

### Step 5. Apply runtime guards

After policy findings, Protect applies Claudine-owned guards that are not permission queries:

- Root without sandbox (from `RuntimeFacts.is_root` + `RuntimeFacts.has_sandbox`)
- YOLO downgrade rules
- Completion retry loop protection
- MCP payload redaction and instruction blocking
- Provider-config mutation escalation (`runtime_guards.ask_on_provider_config_mutation`)

### Step 6. Select desired outcome

Combine all findings into one desired outcome using precedence:

1. `StopSession`
2. `StopCurrent`
3. `AskThenAllowOrStop`
4. `AllowWithRedaction`
5. `AdvisoryOnly`
6. `Allow`

Aggregation rules:

- Any finding with `Deny` effect on a critical operation -> `StopSession`
- Any finding with `Deny` effect on a pre-action operation -> `StopCurrent`
- Any finding with `Ask` effect -> `AskThenAllowOrStop` (unless a stronger stop applies)
- MCP payload redaction needed -> `AllowWithRedaction`
- Advisory-only when Claudine wants to intervene but provider cannot enforce
- All allowed -> `Allow`

### Step 7. Downgrade through provider capabilities

If the desired outcome cannot be enforced by the current provider/event surface, downgrade using adapter capabilities (same logic as current service but sourced from adapter, not internal registry).

Record:

- desired outcome
- actual enforced outcome
- capability that caused degradation

## Decision Semantics Matrix

### Exact policy results

| `QueryResult.effect` | Baseline outcome |
|---|---|
| `Some(Allow)` | `Allow` |
| `Some(Ask)` | `AskThenAllowOrStop` |
| `Some(Deny)` | `StopCurrent` |

### Inexact or uncertain results

| Certainty/Effect | Advisory posture | Balanced posture | Strict posture |
|---|---|---|---|
| `BestEffort` + `Allow` | `AdvisoryOnly` | `Allow` | `AskThenAllowOrStop` |
| `BestEffort` + `Ask` | `AdvisoryOnly` | `AskThenAllowOrStop` | `StopCurrent` |
| `BestEffort` + `Deny` | `AdvisoryOnly` | `StopCurrent` | `StopCurrent` |
| `None` or `Unknown` | `AdvisoryOnly` | depends on operation | `StopCurrent` |

For `None`/`Unknown` under Balanced posture:

- writes, command execution, MCP tool calls, config mutation -> `AskThenAllowOrStop`
- reads, domain access -> `AdvisoryOnly`

### Provider-config mutation

`ModifyProviderConfig` is always elevated:

| Posture | Outcome |
|---|---|
| Advisory | `AdvisoryOnly` |
| Balanced | `AskThenAllowOrStop` |
| Strict | `StopCurrent` |

Even when the provider policy says `Allow`, the `runtime_guards.ask_on_provider_config_mutation` flag can escalate. This is a Claudine guardrail, not a provider permission.

## MCP Redaction Integration

### Current gap

The current service exposes `redact_mcp_text(...)` and `redact_mcp_json(...)` as standalone helpers, but dispatch does not consume a structured redaction plan from `evaluate(...)`.

### New design

`ProtectEvaluation.redaction` carries a concrete payload mutation plan:

```rust
pub enum ProtectRedactionPlan {
    ReplaceText(McpTextRedaction),
    ReplaceJson(McpJsonRedaction),
    BlockPayload { reason: String },
}
```

Dispatch should:

1. Run post-action protect evaluation.
2. If `evaluation.redaction` is `Some(...)`, apply it to the pending `HookResponse`.
3. Then pass the modified response to `adapter.format_response(...)`.

Redaction rules come from `McpRedactionPolicy` (redact patterns, secret patterns, instruction blocking), not from provider permission config.

## Capability Model

`GateCapability`, `VisibilityLevel`, and enforcement downgrade remain useful. What changes is the source:

- Keep `ProviderAdapter::protect_capabilities()`.
- Remove `ProviderProtectProfiles` as a service-internal registry. Dispatch passes adapter capabilities directly into evaluation.
- `ProtectService` does not maintain its own default capability map.

## Dispatch Integration

Three changes to the dispatch path.

### 1. Session context construction

Before pre-action evaluation, dispatch builds `ProtectSessionContext` from:

- `provider`
- `EventMeta.env` for cwd, repo root
- `EventMeta.extra` for trust context
- Wrapper flags: `YOLO`, `INTERACTIVE`
- `AGENT_PARAMS` for provider CLI argv
- `CLAUDINE_SESSION_ID` for session ID

### 2. Adapter observation replaces `ProtectInput::from_event_meta(...)`

```rust
// Before (current):
ProtectInput::from_event_meta(provider, event, &meta)
    .map(|input| service.evaluate(&input))

// After:
adapter.observe_protect(&event, &meta)
    .map(|obs| {
        let request = ProtectRequest {
            provider,
            event,
            phase: ProtectPhase::from(event),
            session: session_ctx.clone(),
            observation: obs,
        };
        service.evaluate(&request)
    })
```

### 3. Redaction application

After post-action evaluation, dispatch applies any `ProtectRedactionPlan` before formatting:

```rust
if let Some(eval) = &protect_post {
    if let Some(plan) = &eval.redaction {
        action_response = apply_redaction(action_response, plan);
    }
}
```

## Wrapper Integration

The wrapper already injects `AGENT_PARAMS`, `YOLO`, `INTERACTIVE`, and `CLAUDINE_SESSION_ID`. That is sufficient for v1.

- `ProtectSessionContext.cli` -> parse `AGENT_PARAMS` into `ProtectCliContext::Argv`
- `ProtectSessionContext.yolo` -> from `YOLO`/`CLAUDINE_YOLO` env vars
- `ProtectSessionContext.interactive` -> from `INTERACTIVE` env var

When `AGENT_PARAMS` is absent (non-wrapper hook invocation), `ProtectCliContext::None` triggers configured-only fallback.

## State and Telemetry

### Decision records

Each evaluation produces a record for the bounded deque. New fields beyond the current implementation:

- `policy_mode`: effective vs configured fallback
- `intent_count`: number of intents evaluated
- `finding_sources`: which finding sources contributed (PolicyQuery, RuntimeGuard, etc.)
- `certainty_summary`: worst certainty across findings
- `source_ids`: policy source IDs involved
- `redaction_applied`: whether redaction plan was generated
- `provider_warnings`: count of propagated policy warnings

### Completion retry tracking

Preserved from the current service. Per-session retry counter; exceeding `completion.max_retries` upgrades any non-stop outcome to `StopSession`.

### Export

`export_state()` and `export_records_jsonl()` remain. Record schema gains the new fields above.

## Remediation Hints (Optional)

Protect should not apply policy mutations during hook evaluation, but it can attach remediation hints for future CLI explain/fix flows:

```rust
pub struct ProtectRemediation {
    pub one_time_args: Option<Vec<String>>,
    pub summary: String,
}
```

Use cases:

- A denied subagent spawn can suggest narrower one-shot args.
- A denied MCP tool call can explain which policy source blocked it.
- A denied write can tell the user whether this is a CLI-only override or a persistent config issue.

This is optional in v1. The data is available from `PolicyEngine` mutation planning, but wiring it into every protect evaluation may not be worth the cost until a CLI consume it.

## Testing Strategy

### Unit tests

- Config validation and merge rules (especially deprecated-field rejection)
- Intent-to-query mapping for each `ProtectIntent` variant
- Decision matrix for all posture x certainty x effect combinations
- Capability downgrade behavior
- Redaction planning
- Completion loop protection
- Severity classification

### Service tests with mocked policy snapshots

The engine dependency is `Arc<PolicyEngine>`. For service tests, construct a `PolicyEngine` with test backends that return controlled snapshots.

Test scenarios:

- Exact `Allow` path query -> `Allow`
- Exact `Deny` path query -> `StopCurrent`
- `Ask` command query -> `AskThenAllowOrStop`
- `Unknown` write under Balanced -> `AskThenAllowOrStop`
- Same `Unknown` write under Strict -> `StopCurrent`
- Root without sandbox escalates even when policy allows
- Provider-config mutation elevated under Balanced
- YOLO mode degrades medium-risk Ask to AdvisoryOnly
- Completion retry threshold exceeded -> StopSession
- MCP payload with secret pattern -> AllowWithRedaction

### Dispatch integration tests

- Pre-action deny short-circuits correctly
- Post-action deny overrides action response
- Post-action MCP response is redacted before adapter formatting
- Effective policy uses wrapper-provided `AGENT_PARAMS`
- Configured fallback is marked when CLI context is unavailable
- `ProtectEvaluation.warnings` includes policy warnings

### Adapter observation tests

- `observe_protect(...)` extracts expected intents per provider fixture
- Claude: tool call with file write -> WritePath intent
- Claude: shell command -> ExecuteCommand intent
- Codex: tool call -> appropriate intents (limited by fire-and-forget model)
- MCP tool call -> UseMcpServer + UseMcpTool + optional AccessDomain
- Capability metadata aligns with enforcement mapping behavior

## Migration Plan

### Phase 1. Module skeleton and config split

- Create `services/protect/` module tree.
- Move existing outcome types, state types, and redaction types into the new tree.
- Narrow `ProtectConfig` (remove duplicated permission fields, add `runtime_guards` and `mcp_redaction`).
- Add config migration: reject deprecated fields with targeted error messages.
- Keep public re-exports stable from `services/mod.rs`.

### Phase 2. Engine dependency and session context

- Update `ProtectService::new` to require `Arc<PolicyEngine>`.
- Introduce `ProtectSessionContext` and `ProtectCliContext`.
- Build policy context and CLI context in dispatch from `EventMeta` and env vars.
- Support configured fallback when CLI context is missing.

### Phase 3. Provider-aware observation

- Add `ProviderAdapter::observe_protect(...)` with a default implementation that mirrors current `from_event_meta` logic (preserves behavior during migration).
- Implement provider-specific overrides where the default extraction is known to be weak.
- Migrate dispatch to use observations and `ProtectRequest`.

### Phase 4. Policy-backed evaluation

- Replace `ProtectRules`-based permission decisions with `PolicyEngine` snapshot queries.
- Implement the decision matrix (posture x certainty x effect).
- Implement `ProtectFinding` and `ProtectEvaluation`.
- Migrate tests to PolicyEngine-backed findings.

### Phase 5. Redaction integration

- Return `ProtectRedactionPlan` from evaluation.
- Apply redaction in dispatch before adapter formatting.
- Remove standalone `redact_mcp_text` / `redact_mcp_json` public methods (redaction is now part of evaluation output).

### Phase 6. Observability and remediation

- Expand decision records with new fields (policy mode, certainty summary, source IDs, etc.).
- Attach optional `ProtectRemediation` when useful.
- Update JSONL export schema.

## Risks and Mitigations

### 1. Intent extraction quality determines trustworthiness

If adapters extract incomplete intents, Protect will have authoritative policy answers for the wrong question.

Mitigation:

- Make intent extraction explicit and test it per provider with fixture events.
- Preserve explanation output so misses are diagnosable.
- Start with a default `observe_protect` that mirrors current behavior, then improve per provider.

### 2. Effective policy may be unavailable in some hook contexts

Not every invocation has the original provider argv.

Mitigation:

- Support configured fallback explicitly.
- Surface fallback in telemetry and explanations via `ProtectPolicyMode::ConfiguredFallback`.
- Use `AGENT_PARAMS` whenever the session came through Claudine wrappers.

### 3. PolicyEngine query quality is still maturing

The review found that stability, warnings, and path canonicalization are incomplete. Protect will be building on answers that may improve.

Mitigation:

- Protect's decision matrix must handle `Unknown` and `BestEffort` certainty correctly from day one.
- Do not assume warnings or stability are populated; treat absence as unknown.
- As PolicyEngine improves (path canonicalization, trust gating), Protect automatically benefits.

### 4. Narrowing ProtectConfig is a breaking config change

Existing user settings may include command/path allowlists or MCP access lists.

Mitigation:

- Reject deprecated fields with a targeted validation message at config load time.
- Message should name the removed field, explain why it was removed, and point to the provider-native config location.

## Open Questions

1. Should adapter observation live on `ProviderAdapter`, or on a separate `ProtectObserver` trait to keep adapters smaller? The adapter already has `protect_capabilities()` and `map_protect_outcome()`, so adding `observe_protect()` keeps the provider surface unified, but it further grows the trait.

2. Should uncertain `Allow` results be configurable per security axis, or is posture-wide behavior enough for v1? Per-axis control adds complexity but lets users say "I trust the engine on file reads but want strict on commands." Recommendation: posture-wide for v1, per-axis later if needed.

3. Should MCP secret scanning remain inside Protect, or should it become a separate service? Recommendation: keep it in Protect for now since it directly feeds `ProtectRedactionPlan`. Extract later if it grows complex enough to warrant independent evolution.

4. Should `ProtectService` expose a public `inspect` or `explain` method for future CLI reporting, or should that be a separate consumer of the same `PolicyEngine` + `ProtectConfig`? Recommendation: add `explain_last()` -> `Option<ProtectExplanation>` as a convenience, but the full CLI report command should consume PolicyEngine directly.

## Responsibility Split (Final)

| Concern | Owner |
|---|---|
| Provider config discovery and parsing | PolicyEngine |
| Configured policy resolution | PolicyEngine |
| Effective policy resolution with CLI overrides | PolicyEngine |
| Canonical cross-provider query answers | PolicyEngine |
| Policy provenance, fidelity, certainty | PolicyEngine |
| Mutation planning and application | PolicyEngine |
| Converting live events into typed intents | ProtectService + Adapters |
| Runtime posture-based decisioning | ProtectService |
| Provider enforcement capability downgrade | ProtectService |
| MCP payload redaction | ProtectService |
| Completion loop protection | ProtectService |
| Decision audit trail and telemetry | ProtectService |
| Provider-native event parsing | Adapters |
| Provider-native response formatting | Adapters |
| Provider enforcement capability metadata | Adapters |
| Provider-specific intent extraction from events | Adapters (via `observe_protect`) |
