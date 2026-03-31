# Protect Service Technical Design

This document turns the permissions-engine spec and policy-engine design into an implementation-ready design for a rewritten `ProtectService`.

Primary inputs:

- `claudine/features/2026-03-30-permissions-engine/spec.md`
- `claudine/features/2026-03-30-permissions-engine/policy-engine-design.md`
- `claudine/features/2026-03-30-permissions-engine/opinion.md`
- current Protect implementation in `claudine/lib/src/services/protect.rs`
- current dispatch integration in `claudine/lib/src/dispatch/mod.rs`
- current adapter protect hooks in `claudine/lib/src/adapters/`
- wrapper launch/env behavior in `claudine/cli/src/commands/wrap/`

The core design decision is:

**`ProtectService` will be fully rebuilt as a runtime decision layer on top of `PolicyEngine`, with `PolicyEngine` as the only source of truth for provider permissions.**

That means:

1. `ProtectService` no longer owns command allowlists, protected-path lists, MCP access lists, or inferred provider policy.
2. `ProtectService` always evaluates against a configured or effective `PolicyEngine` snapshot.
3. `ProtectService` still owns runtime judgment that `PolicyEngine` explicitly does not own:
   - event normalization
   - provider enforcement capability downgrade
   - MCP payload redaction
   - completion loop protection
   - audit trail and telemetry

## Summary

The current `ProtectService` mixes three different concerns:

1. Claudine-local safety policy
2. heuristic inference about provider permissions
3. hook-time enforcement mapping

That structure is no longer correct once `PolicyEngine` exists.

The replacement service should instead be built as a thin but opinionated layer with this pipeline:

1. normalize a live hook event into one or more concrete attempted actions
2. resolve the provider's effective policy through `PolicyEngine`
3. query the policy snapshot for each attempted action
4. combine those answers with Claudine runtime posture and provider enforcement capabilities
5. return a single runtime outcome plus structured evidence, optional redaction output, and audit metadata

The rewrite should preserve the useful runtime behaviors from the current service:

- `Allow`, `AskThenAllowOrStop`, `StopCurrent`, `StopSession`, `AllowWithRedaction`, and `AdvisoryOnly`
- capability-aware downgrade when a provider cannot enforce the ideal outcome
- completion retry/loop protection
- MCP text/JSON redaction
- provider-aware short-circuit integration in dispatch

What changes is where the facts come from:

- provider permission truth comes from `PolicyEngine`
- runtime control posture comes from Claudine `ProtectConfig`
- provider blocking/modification limits come from the adapter capability surface

## Goals

1. Make `PolicyEngine` the sole permission authority used by Protect.
2. Remove duplicated permission truth from `ProtectConfig`.
3. Replace heuristic event-to-policy inference with provider-aware action extraction.
4. Preserve hook-time runtime control outcomes and capability downgrade behavior.
5. Surface structured evidence for every Protect decision.
6. Support pre-action and post-action evaluation from the same service.
7. Integrate redaction into the actual dispatch path rather than exposing unused helper APIs.
8. Keep the service testable in isolation from provider config parsing.

## Non-Goals

1. Moving permission parsing or mutation logic back into `ProtectService`.
2. Auto-writing provider config during hook evaluation.
3. Replacing `PolicyEngine` query or mutation APIs.
4. Designing a user-facing `claudine protect ...` CLI in this phase.
5. Solving every provider's action-extraction edge case in one pass.

## Why A Full Rewrite Is Warranted

The existing `ProtectService` is not a narrow refactor away from the desired architecture.

The current implementation is built around:

- `ProtectInput::from_event_meta(...)`, which uses shared heuristics instead of provider-native extraction
- `ProtectRules`, which duplicate policy decisions that should come from provider config
- `McpPolicy.allowlist` / `denylist`, which duplicate provider MCP permissions
- risk inference from loose strings like `"chmod"` or `"rm -rf"`
- a local `ProviderProtectProfiles` registry that partly duplicates adapter knowledge

Once `PolicyEngine` lands, keeping that structure would leave Claudine with two separate policy systems:

1. provider-native truth in `PolicyEngine`
2. Claudine-owned shadow truth in `ProtectService`

That would be harder to explain, harder to trust, and harder to test than a rewrite.

## Responsibility Split

### `PolicyEngine` owns

- provider config discovery
- configured policy resolution
- effective policy resolution with CLI/runtime overrides
- canonical cross-provider query answers
- policy provenance, fidelity, and certainty
- mutation planning and application

### `ProtectService` owns

- converting live events into concrete attempted actions
- deciding what Claudine should do right now for this event
- applying Claudine posture to uncertain or dangerous runtime situations
- mapping desired outcomes through provider enforcement capabilities
- MCP payload redaction
- loop protection and audit state

### Adapters own

- provider-native event parsing and response formatting
- provider enforcement capability metadata
- provider-specific extraction of actionable protection intents from `EventMeta`

## Design Principles

### 1. Protect never answers a permission question without a policy snapshot

If Protect needs to know whether a path write, command execution, MCP tool call, or subagent spawn is allowed, it must ask `PolicyEngine`.

It may still add runtime guardrails on top of that answer, but it does not invent its own permission truth.

### 2. Event extraction is provider-aware

The generic `ProtectInput::from_event_meta(...)` model is too weak. Different providers expose different shapes for:

- tool names
- shell commands
- path arguments
- MCP server and tool identifiers
- subagent operations
- approval/sandbox state

The new service should consume provider-aware observations produced by adapters.

### 3. Claudine posture governs runtime action, not permission truth

`ProtectConfig` still matters, but its purpose changes. It should answer questions like:

- when should an uncertain result become `Ask` vs `Stop`
- when should root-without-sandbox escalate
- when should completion retries stop a session
- when should MCP payloads be redacted or blocked

It should not answer:

- which command is allowed
- which path is protected
- which MCP server is allowed

### 4. Protect decisions must be explainable

Every returned decision should carry:

- the queried policy evidence
- the runtime guards that were applied
- whether the final outcome was degraded by provider capability
- whether the policy basis came from configured or effective policy

### 5. Dispatch integration should stay linear

The runtime integration should remain simple:

1. pre-action protect evaluation
2. optional short-circuit
3. actions execute
4. post-action protect evaluation
5. optional short-circuit or redaction

The rewrite should not create a second orchestration layer outside dispatch.

## Recommended Module Layout

The current single-file service should be split into a dedicated module tree:

```txt
claudine/lib/src/services/protect/
├── mod.rs
├── service.rs
├── config.rs
├── request.rs
├── observe.rs
├── intent.rs
├── evaluate.rs
├── decision.rs
├── downgrade.rs
├── redact.rs
├── state.rs
└── explain.rs
```

Recommended responsibilities:

- `service.rs`
  - `ProtectService`
  - snapshot caching
  - top-level evaluate APIs
- `config.rs`
  - runtime-only protect config
  - provider overrides
  - validation and merge logic
- `request.rs`
  - `ProtectRequest`
  - runtime/session context
- `observe.rs`
  - `ProtectObservation`
  - adapter-facing normalized observation shape
- `intent.rs`
  - `ProtectIntent`
  - attempted actions extracted from observations
- `evaluate.rs`
  - policy querying
  - intent scoring
  - final decision synthesis
- `decision.rs`
  - `ProtectDecision`
  - `ProtectOutcome`
  - structured evidence and findings
- `downgrade.rs`
  - provider capability downgrade rules
- `redact.rs`
  - MCP text and JSON redaction
  - payload modification plan
- `state.rs`
  - recent records
  - completion retry tracking
- `explain.rs`
  - human-readable and machine-readable explanations

## Public API Shape

The replacement service should be constructed with an engine dependency instead of acting as a self-contained policy actor.

Recommended shape:

```rust
pub struct ProtectService {
    engine: Arc<PolicyEngine>,
    config: ProtectConfig,
    state: ProtectState,
}

impl ProtectService {
    pub fn new(engine: Arc<PolicyEngine>, config: ProtectConfig) -> Self;

    pub fn config(&self) -> &ProtectConfig;

    pub fn evaluate(
        &mut self,
        request: &ProtectRequest,
    ) -> Result<ProtectEvaluation>;

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

`ProtectEvaluation` should replace the current `ProtectDecision`-only return surface.

```rust
pub struct ProtectEvaluation {
    pub decision: ProtectDecision,
    pub policy_mode: ProtectPolicyMode,
    pub findings: Vec<ProtectFinding>,
    pub redaction: Option<ProtectRedactionPlan>,
    pub warnings: Vec<PolicyWarning>,
}

pub enum ProtectPolicyMode {
    Effective,
    ConfiguredFallback,
}
```

This addresses a current gap: Protect can signal `AllowWithRedaction`, but it does not return a concrete modification plan that dispatch can apply.

## Core Request Types

### `ProtectSessionContext`

Protect needs more than a raw event. It must know how the provider session was launched.

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

Important behavior:

- wrapper-launched sessions should use the actual provider argv from `AGENT_PARAMS`
- non-wrapper hook invocations may fall back to `ProtectCliContext::None`
- when CLI data is absent, Protect should request a configured snapshot and mark the result as `ConfiguredFallback`

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

This replaces the current wide `ProtectInput` bag.

## Provider-Aware Observation Layer

The service should stop trying to interpret every provider payload generically. Instead, adapters should supply a normalized observation.

Recommended new adapter hook:

```rust
fn observe_protect(
    &self,
    event: &AgenticEvent,
    meta: &EventMeta,
) -> Option<ProtectObservation>;
```

Recommended shape:

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
```

`ProtectIntent` is the key bridge between live events and `PolicyEngine` queries.

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
    ModifyProviderConfig,
    CompletionOutputScan,
}
```

Why this matters:

- `PolicyEngine` already defines canonical query types
- Protect should reuse them directly wherever possible
- adapter extraction is then about populating those query types correctly for each provider

## Policy Snapshot Resolution

For each request, Protect should resolve one snapshot only once.

Resolution order:

1. if `ProtectSessionContext.cli` is available, request `EffectivePolicySnapshot`
2. otherwise request `ConfiguredPolicySnapshot`
3. cache that snapshot for the rest of the evaluation
4. reuse it for all intent queries in that request

Within a single service instance, snapshot caching should be keyed by:

- provider
- `cwd`
- `repo_root`
- trust context
- CLI fingerprint

This is enough to avoid reloading policy twice during pre-action and post-action evaluation in the same hook process.

## Runtime Config Redesign

`ProtectConfig` should be narrowed so it controls runtime decisioning only.

### Remove from `ProtectConfig`

These fields duplicate `PolicyEngine` truth and should be removed:

- `rules.blocked_command_patterns`
- `rules.ask_command_patterns`
- `rules.protected_paths`
- `mcp.allowlist`
- `mcp.denylist`

### Keep in `ProtectConfig`

These remain valid because they are runtime rails, not provider permission truth:

- `enabled`
- `posture`
- `allow_repo_posture_downgrade`
- `yolo`
- `completion`
- MCP redaction settings
- root/sandbox hardening
- provider-specific posture overrides
- audit retention

Recommended revised shape:

```rust
pub struct ProtectConfig {
    pub enabled: bool,
    pub posture: ProtectPosture,
    pub allow_repo_posture_downgrade: bool,
    pub yolo: YoloPolicy,
    pub completion: CompletionPolicy,
    pub mcp: McpRedactionPolicy,
    pub runtime: RuntimeGuardPolicy,
    pub providers: HashMap<Provider, ProviderProtectOverride>,
    pub max_recent_decisions: u16,
}
```

Where:

```rust
pub struct RuntimeGuardPolicy {
    pub deny_when_root_without_sandbox: bool,
    pub ask_on_unknown_write: bool,
    pub ask_on_unknown_command: bool,
    pub ask_on_provider_config_mutation: bool,
}

pub struct McpRedactionPolicy {
    pub redact_patterns: Vec<String>,
    pub block_instruction_payloads: bool,
}
```

The `ask_on_unknown_*` fields are optional, but some equivalent uncertainty policy needs to exist. `PolicyEngine` can return `Unknown`; Protect must decide what that means operationally.

## Decision Pipeline

The new runtime decision pipeline should be deterministic and layered.

### Step 1. Resolve provider runtime posture

Protect still derives runtime posture from:

- Protect config
- provider-specific config override
- session `interactive`
- session `yolo`
- adapter enforcement capabilities

### Step 2. Resolve policy snapshot

Use `PolicyEngine` to get configured or effective policy as described above.

### Step 3. Query each extracted intent

For each `ProtectIntent`, Protect calls the matching query on the snapshot.

Examples:

- `WritePath` -> `snapshot.can_write(path)`
- `ExecuteCommand` -> `snapshot.can_execute(command)`
- `UseMcpTool` -> `snapshot.can_use_mcp_tool(server, tool)`
- `SpawnSubagent` -> `snapshot.can_spawn_subagent(name)`
- `ModifyProviderConfig` -> `snapshot.can_modify_own_config()`

### Step 4. Build findings

Each query becomes a `ProtectFinding`.

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
```

### Step 5. Apply runtime guards

After policy findings are collected, Protect applies Claudine-owned guards that are not permission queries:

- root without sandbox
- YOLO downgrade rules
- completion retry loop rules
- MCP payload redaction or instruction blocking
- optional provider-config mutation escalation

### Step 6. Select desired outcome

The service then chooses one desired outcome before considering provider limitations.

Recommended priority order:

1. `StopSession`
2. `StopCurrent`
3. `AskThenAllowOrStop`
4. `AllowWithRedaction`
5. `AdvisoryOnly`
6. `Allow`

### Step 7. Downgrade through provider capabilities

If the desired outcome cannot be enforced by the current provider event surface, downgrade it using adapter capabilities.

This logic should stay conceptually the same as the current service, but the capability data should come directly from the adapter for the current provider rather than a separate registry.

## Decision Semantics

Protect needs a clear matrix for interpreting `QueryResult`.

### Exact policy results

- `effect = Allow` -> baseline `Allow`
- `effect = Ask` -> baseline `AskThenAllowOrStop`
- `effect = Deny` -> baseline `StopCurrent`

### Inexact or uncertain results

- `certainty = BestEffort` with `effect = Allow`
  - `Advisory` posture: `AdvisoryOnly`
  - `Balanced` posture: `Allow`
  - `Strict` posture: `AskThenAllowOrStop`
- `effect = None` or `certainty = Unknown`
  - `Advisory` posture: `AdvisoryOnly`
  - `Balanced` posture: `AskThenAllowOrStop` for writes, command execution, MCP tool calls, and config mutation
  - `Strict` posture: `StopCurrent` for the same categories

The exact matrix may be tuned during implementation, but v1 must define one centrally instead of reintroducing ad hoc heuristics.

### Provider-config mutation

`ModifyProviderConfig` should be treated as high sensitivity even if the provider allows it.

Recommended default:

- `Balanced`: `AskThenAllowOrStop`
- `Strict`: `StopCurrent`
- `Yolo`: degrade to `AdvisoryOnly` only if blocking is explicitly disabled by config

## MCP Redaction Integration

The new design should make redaction a first-class dispatch output.

### Current problem

The current service exposes:

- `redact_mcp_text(...)`
- `redact_mcp_json(...)`

but dispatch does not actually consume a structured redaction plan from `evaluate(...)`.

### New design

`ProtectEvaluation.redaction` should carry the actual payload mutation plan:

```rust
pub enum ProtectRedactionPlan {
    ReplaceText(McpTextRedaction),
    ReplaceJson(McpJsonRedaction),
    BlockPayload { reason: String },
}
```

Dispatch should:

1. run post-action Protect evaluation
2. if a redaction plan exists, apply it to the pending `HookResponse`
3. then pass the modified response to `adapter.format_response(...)`

This keeps payload mutation inside the normal response pipeline instead of as a disconnected helper API.

## Capability Model Changes

`GateCapability`, `VisibilityLevel`, and the concept of enforcement downgrade remain useful.

What should change is the source of truth:

- keep `ProviderAdapter::protect_capabilities()`
- remove the need for `ProtectService` to maintain its own default capability map

Recommended change:

- `ProviderProtectProfiles` becomes a test fixture or disappears entirely
- dispatch passes the adapter capability surface directly into evaluation
- integration tests assert adapter capability fixtures, not service-local defaults

## Dispatch Integration

The dispatch path should change in three places.

### 1. Session context construction

Before pre-action Protect evaluation, dispatch should build `ProtectSessionContext` from:

- `provider`
- `cwd`
- repo root if available
- current env
- wrapper flags such as `YOLO` and `INTERACTIVE`
- `AGENT_PARAMS` for original provider argv
- session ID

This becomes the bridge from wrapper launch state to `PolicyEngine::effective(...)`.

### 2. Adapter observation

Replace:

```rust
ProtectInput::from_event_meta(...)
```

with:

```rust
adapter.observe_protect(...)
```

and then wrap that in `ProtectRequest`.

### 3. Redaction application

After post-action evaluation, dispatch should apply any `ProtectRedactionPlan` before formatting the response.

## Wrapper Integration

The wrapper already injects `AGENT_PARAMS`, `YOLO`, and `INTERACTIVE`. That is enough to get v1 moving.

Recommended behavior:

- `ProtectSessionContext.cli` should parse `AGENT_PARAMS`
- `ProtectSessionContext.yolo` should come from wrapper flags, not from fragile event payload pattern matching
- `ProtectSessionContext.interactive` should come from wrapper flags when present

Future improvement:

- add an explicit serialized policy-context env payload if trust resolution or shadow-home paths need to be made more deterministic for hooks

## Mutation Planning Usage

`PolicyEngine` can plan persistent and one-shot policy changes. `ProtectService` should not apply them during hook evaluation, but it should be able to attach remediation hints.

Recommended optional field:

```rust
pub struct ProtectRemediation {
    pub one_time_args: Option<Vec<String>>,
    pub summary: String,
}
```

Use cases:

- a denied subagent spawn can suggest narrower one-time args
- a denied MCP tool call can explain which provider policy source blocked it
- a denied write can tell the user whether this is a CLI-only override or a persistent config issue

This is especially valuable for future CLI explain/fix flows, but hook-time application remains out of scope.

## State And Telemetry

The current bounded forensic state is still useful and should be preserved.

Recommended record shape additions:

- policy mode used: effective vs configured fallback
- query count
- top finding source
- certainty summary
- source IDs involved in the decision
- whether redaction was applied

`ProtectState` should continue to track:

- total decision count
- bounded recent decisions
- completion retry counters

## Migration Plan

The rewrite should land in bounded phases.

### Phase 1. Introduce the new module skeleton

- create `services/protect/`
- move existing outcome, config, and state types into the new module tree
- keep the public re-exports stable from `services/mod.rs`

### Phase 2. Add `ProtectSessionContext` and engine dependency

- update service construction to require `Arc<PolicyEngine>`
- build policy context and CLI context in dispatch
- support configured fallback when effective context is missing

### Phase 3. Replace `ProtectInput::from_event_meta(...)`

- add `ProviderAdapter::observe_protect(...)`
- implement shared default extraction plus provider-specific overrides where needed
- migrate dispatch to use observations and requests

### Phase 4. Replace duplicated permission rules

- remove command/path/MCP allow/deny lists from `ProtectConfig`
- update config validation and merge logic
- migrate tests to PolicyEngine-backed findings

### Phase 5. Integrate redaction into dispatch

- return `ProtectRedactionPlan` from evaluation
- mutate `HookResponse` before adapter formatting

### Phase 6. Add remediation hints and richer telemetry

- attach optional `ProtectRemediation`
- expand exported state and JSONL records

## Testing Strategy

The rewrite should be tested in layers.

### Unit tests

- config validation and merge rules
- intent-to-query mapping
- decision matrix for `Allow` / `Ask` / `Deny` / `Unknown`
- capability downgrade behavior
- redaction planning
- completion loop protection

### Service tests with mocked policy snapshots

- exact allow/write path query yields `Allow`
- exact deny/query yields `StopCurrent`
- uncertain query under `Balanced` yields `AskThenAllowOrStop`
- same uncertain query under `Strict` yields `StopCurrent`
- root-without-sandbox escalates even when policy allows
- provider-config mutation is elevated

### Dispatch integration tests

- pre-action deny short-circuits correctly
- post-action deny short-circuits correctly
- post-action MCP response is redacted before adapter formatting
- effective policy uses wrapper-provided `AGENT_PARAMS`
- configured fallback is marked when CLI context is unavailable

### Adapter tests

- `observe_protect(...)` extracts expected intents per provider fixture
- capability metadata remains aligned with enforcement mapping behavior

## Risks

### 1. Action extraction quality will determine trustworthiness

If adapters extract incomplete intents, Protect will have authoritative policy answers for the wrong question. This is the biggest correctness risk in the rewrite.

Mitigation:

- make intent extraction explicit and test it per provider
- preserve explanation output so misses are diagnosable

### 2. Effective policy may be unavailable in some hook contexts

Not every invocation will have the original provider argv.

Mitigation:

- support configured fallback explicitly
- surface fallback in telemetry and explanations
- use `AGENT_PARAMS` whenever the session came through Claudine wrappers

### 3. Narrowing `ProtectConfig` is a breaking config change

Existing user settings may include command/path allowlists or MCP access lists.

Mitigation:

- add config migration notes
- reject deprecated fields with a targeted validation message instead of silently ignoring them

## Open Questions

1. Should uncertain `Allow` results from `PolicyEngine` be configurable per axis, or is posture-wide behavior enough for v1?
2. Should adapter observation live on `ProviderAdapter`, or should it be a separate provider-specific protect extractor trait to keep adapters smaller?
3. Do we want post-action completion secret scanning to remain inside Protect, or should that become a separate reporting/validation subsystem later?

## Final Recommendation

Rebuild `ProtectService` as a runtime evaluation layer that depends on `PolicyEngine` for all permission facts.

The implementation should keep the useful operational parts of the current service:

- outcome vocabulary
- capability downgrade
- redaction
- loop protection
- audit records

but delete the parts that will become wrong once `PolicyEngine` exists:

- heuristic permission truth
- local command/path/MCP allow-deny rules
- generic event-to-policy inference
- duplicated provider capability registries

That gives Claudine a clean architecture:

- `PolicyEngine` explains what the provider can do
- `ProtectService` decides what Claudine should do right now
- adapters explain what the provider can enforce and what the event actually means
