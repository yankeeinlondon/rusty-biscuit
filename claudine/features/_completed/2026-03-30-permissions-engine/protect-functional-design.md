# ProtectService Functional Design

This document defines the functional design for a rewritten `ProtectService` that uses `PolicyEngine` as its single source of truth for provider permission state.

Primary inputs:

- `claudine/features/2026-03-30-permissions-engine/spec.md`
- `claudine/features/2026-03-30-permissions-engine/policy-engine-design.md`
- `claudine/features/2026-03-30-permissions-engine/opinion.md`
- current dispatch flow in `claudine/lib/src/dispatch/`
- current protect implementation in `claudine/lib/src/services/protect.rs`

## Summary

The new `ProtectService` is a runtime guard and enforcement-orchestration service.

It should not parse provider-native permission config, infer durable permissions from scratch, or own the canonical permission model. `PolicyEngine` does that.

The new split is:

- `PolicyEngine` answers "what is configured?", "what is effective right now?", "why?", and "what would change?"
- `ProtectService` answers "a live action is about to happen or just happened; what should Claudine do with that fact?"

In practice, `ProtectService` should:

1. normalize runtime activity into protection intents
2. query `PolicyEngine` for authoritative policy answers
3. compare attempted activity against effective provider policy
4. choose a runtime response using Claudine posture and provider hook capabilities
5. redact sensitive payloads when blocking is impossible or incomplete
6. emit consistent decisions, explanations, and audit records back into dispatch

That makes Protect smaller in scope than today, but much more trustworthy.

## Functional Goals

1. Use `PolicyEngine` as the only authority for provider permission facts.
2. Make runtime decisions from explicit attempted operations rather than mostly heuristic text matching.
3. Preserve current dispatch ergonomics: pre-action decisioning, post-action decisioning, short-circuiting, provider-native response mapping, and audit export.
4. Keep Claudine-specific posture and risk policy in Protect, not in `PolicyEngine`.
5. Support provider capability degradation explicitly when Claudine wants a stronger runtime outcome than the provider can enforce.
6. Improve explainability by citing policy provenance and runtime evidence together.

## Non-Goals

1. Parsing provider-native config files in `ProtectService`.
2. Re-implementing canonical permission matching logic already owned by `PolicyEngine`.
3. Persistently mutating provider config from hook-time decisions.
4. Replacing provider adapters as the place where provider-native hook responses are formatted.
5. Inventing a new permission model separate from the canonical policy model.

## Core Service Boundary

### What ProtectService owns

- runtime intent extraction from live dispatch events
- posture-aware intervention policy
- risk escalation beyond provider-native allow or deny
- provider capability downgrade handling
- MCP payload redaction
- short-circuit decisions for pre-action and post-action dispatch phases
- forensic and telemetry state
- operator-facing explanation strings for live decisions

### What ProtectService does not own

- provider config discovery
- provider config parsing
- CLI override parsing for effective permissions
- canonical policy normalization
- exact path, command, domain, tool, MCP, network, and subagent permission evaluation rules
- mutation planning and persistent config edits

## Architectural Position

`ProtectService` should sit between dispatch and the provider adapter layer.

Functional flow:

1. Dispatch parses a provider event into normalized `EventMeta`.
2. Protect builds a runtime evaluation request from that event.
3. Protect asks `PolicyEngine` for the effective policy snapshot for the provider and current runtime context.
4. Protect evaluates the attempted operation against that snapshot.
5. Protect combines the policy answer with:
   - Claudine protect posture
   - risk classification
   - event phase
   - interactive or yolo mode
   - provider hook capabilities
6. Protect returns a `ProtectDecision`.
7. Dispatch either short-circuits or continues execution.
8. Adapter maps any blocking decision into provider-native response format.

## Recommended Responsibilities

The rewritten service should be organized around five responsibilities.

### 1. Runtime intent resolution

Protect must turn event metadata and action context into explicit attempted operations.

Examples:

- filesystem read of `/repo/.env`
- filesystem write of `/repo/src/main.rs`
- shell execution of `rm -rf build`
- outbound network call to `api.openai.com`
- MCP server access to `github`
- MCP tool invocation `github.create_pull_request`
- subagent spawn
- approval-bypassing execution mode

This is the most important rewrite compared with the current service. The service should prefer typed operation extraction over regex-first risk guessing.

### 2. Policy-assisted authorization

For each attempted operation, Protect should query the effective provider policy through `PolicyEngine`.

Protect needs answers such as:

- is this path read allowed, ask, denied, or unknown?
- is this path write allowed, ask, denied, or unknown?
- is this shell command permitted?
- is network enabled at all?
- is this domain permitted?
- is this MCP server or tool allowed?
- are subagents allowed?
- is the provider running in a mode that already bypasses approval or sandbox restrictions?

Protect should consume both:

- the decision
- the explanation payload, including certainty, fidelity, warnings, and provenance

### 3. Runtime control decisioning

Protect must convert policy answers plus Claudine runtime settings into action outcomes.

The service should keep a Claudine-owned outcome vocabulary similar to the existing one:

- `Allow`
- `AllowWithRedaction`
- `AskThenAllowOrStop`
- `StopCurrent`
- `StopSession`
- `AdvisoryOnly`

These are runtime intervention outcomes, not permission facts.

### 4. Redaction and damage containment

When the provider cannot block, or when the action has already completed, Protect should still mitigate exposure.

Primary responsibilities:

- redact sensitive MCP payloads before they leave Claudine-controlled surfaces
- downgrade to advisory or ask flows when hard blocking is unavailable
- preserve why the stronger desired outcome could not be enforced

### 5. Audit and observability

Protect should continue to retain and export runtime decision records, but the records should now include `PolicyEngine` facts.

Each decision record should capture:

- provider
- event
- phase
- attempted operation
- effective policy answer
- runtime risk classification
- protect posture
- desired outcome
- actual enforced outcome
- whether degradation occurred
- policy provenance references
- redaction summary if applicable

## Runtime Operations ProtectService Must Perform

### 1. Build evaluation context

For every protect evaluation, the service must assemble:

- provider
- normalized event
- event phase
- session ID if present
- cwd and repo context
- trust context
- environment hints relevant to provider runtime
- interactivity and yolo flags
- provider CLI invocation details when available
- post-action output context when evaluating after an action

This context is the bridge between dispatch and `PolicyEngine`.

### 2. Resolve effective policy snapshot

Protect must request the effective policy, not the configured-only policy, for live runtime decisions.

The lookup must use:

- provider
- `PolicyContext`
- effective CLI/runtime overrides if known

The service should cache effective snapshots within a dispatch/session scope when inputs are identical, because repeated events in one session are likely to ask the same questions.

### 3. Derive attempted operations

Protect should transform one runtime input into one or more `ProtectOperation`s.

Recommended operation classes:

- `ReadPath`
- `WritePath`
- `TraversePath`
- `ExecuteCommand`
- `ExecuteCommandWithEscalation`
- `AccessDomain`
- `AccessNetwork`
- `AccessMcpServer`
- `InvokeMcpTool`
- `SpawnSubagent`
- `SwitchMode`
- `BypassApproval`
- `MutateProviderConfig`

One event may yield multiple operations. For example, an MCP tool call might imply:

- MCP server access
- specific MCP tool access
- optional outbound domain access

Protect should evaluate each operation independently, then compute an aggregate runtime outcome.

### 4. Query permission decisions

For every attempted operation, Protect must call the appropriate query on the effective policy snapshot and capture:

- `PolicyEffect`
- certainty
- mapping fidelity
- explanation
- source provenance
- warnings

This is the pivot that removes provider-specific permission reasoning from Protect.

### 5. Classify runtime risk

Risk is still useful, but it should become a secondary modifier instead of the primary basis for policy truth.

Risk classification should consider:

- operation type
- target sensitivity
- whether policy answer is `Deny`, `Ask`, `Allow`, or `Unknown`
- whether action touches protected config, secrets, repo metadata, or external network
- whether the provider is already in a high-bypass runtime mode
- whether the action has already occurred

Recommended risk sources:

- policy result
- path sensitivity heuristics
- command sensitivity heuristics
- payload sensitivity heuristics
- session repetition or loop behavior

### 6. Choose desired runtime outcome

Protect should combine the operation results into a desired intervention outcome before capability downgrade.

Recommended decision rules:

- explicit policy `Deny` for a pre-action operation should usually produce `StopCurrent` or `StopSession`
- explicit policy `Ask` should usually produce `AskThenAllowOrStop` when the provider supports it
- `Unknown` policy should escalate according to posture
- explicit policy `Allow` may still escalate if Claudine detects a higher-order risk, but that must be called out as a Claudine guardrail rather than a provider denial
- post-action violations should favor `AllowWithRedaction`, `AdvisoryOnly`, or `StopSession` depending on severity and phase

### 7. Degrade to provider-enforceable outcome

Protect must translate desired outcomes into outcomes the current provider and event can actually enforce.

This remains a Protect responsibility because it depends on hook surfaces rather than permission truth.

Examples:

- a non-blocking provider event may only allow advisory reporting
- a provider may support deny but not interactive ask
- MCP output after tool completion may only allow redaction, not prevention

Every degradation must be explicit in the decision:

- original desired outcome
- actual enforced outcome
- capability that caused degradation

### 8. Redact payloads

Protect should preserve the existing redaction role, but the inputs should now be policy-assisted.

Redaction operations include:

- redact MCP text output
- redact MCP JSON payloads
- redact sensitive policy explanations when exposing them to low-trust surfaces

Redaction rules should consider both:

- Claudine-configured secret patterns
- policy-aware sensitivity signals, such as protected config paths or denied secret-bearing tools

### 9. Maintain runtime state

Protect should continue to retain bounded session state for:

- recent decisions
- completion retries and loop protection
- repeated denial patterns
- post-action escalation history

This state is necessary because runtime protection is not only about static policy. It is also about session dynamics.

### 10. Export decision records

Protect must provide stable reporting surfaces for:

- structured telemetry
- JSONL export
- audit snapshots
- future CLI reporting

The record format should be updated to include authoritative policy metadata from `PolicyEngine`.

## Functional Interaction Points

### Dispatch

Dispatch is the primary caller.

Pre-action interactions:

- dispatch builds the pre-action runtime input
- dispatch asks Protect for a decision
- dispatch short-circuits if Protect returns a blocking outcome

Post-action interactions:

- dispatch builds post-action runtime input from action output
- dispatch asks Protect for a second decision
- dispatch may replace a normal action response with a protect-driven response

Required Protect interfaces for dispatch:

- evaluate pre-action
- evaluate post-action
- inspect whether an outcome short-circuits
- export protect context for attaching to action responses

### PolicyEngine

`PolicyEngine` is the core dependency.

Protect interactions with the engine:

- build effective policy snapshot
- query authorization for one attempted operation
- request explanation and provenance
- read runtime mode facts such as approval mode, sandbox mode, trust requirements, or bypass capability

Protect must not reach around the engine to parse provider config directly.

### Provider adapters

Adapters remain responsible for provider-native response formatting and capability declaration.

Protect interactions:

- read provider hook capability profile
- rely on adapter mapping from protect outcome to provider hook response

Protect should not know provider response JSON details.

### Runtime config loader

Protect still consumes Claudine-owned config for posture and local guardrails.

That config should focus on:

- enable or disable protect
- posture
- risk thresholds
- completion loop protection
- redaction settings
- provider overrides for runtime behavior

It should no longer attempt to duplicate provider permission configuration.

### Action runner

The action runner needs Protect output in two places:

- short-circuiting before a call action executes
- attaching protect context to returned `HookResponse`

Protect should expose a compact response context suitable for action-level reporting.

### Reporting and telemetry

Protect remains the source for live runtime protection reports.

Consumers:

- JSONL logs
- SQLite reporting sync if extended later
- debugging output
- future `claudine protect` or `claudine permissions audit` style commands

### User configuration

The new design implies a sharper split in user-facing config:

- provider-native permissions live where the provider expects them
- `PolicyEngine` reads and explains them
- `settings.protect` only configures Claudine runtime intervention policy

This removes the current ambiguity where Claudine-owned protect config looks like a substitute for real provider permissions.

## Recommended Protect Input Model

The current `ProtectInput` should be replaced or heavily revised around explicit operations.

Recommended shape:

```rust
pub struct ProtectEvaluationRequest {
    pub provider: Provider,
    pub phase: ProtectPhase,
    pub event: AgenticEvent,
    pub session_id: Option<String>,
    pub runtime: ProtectRuntimeContext,
    pub operations: Vec<ProtectOperation>,
    pub evidence: ProtectEvidence,
}
```

Supporting types:

```rust
pub struct ProtectRuntimeContext {
    pub cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub trust: ProjectTrustContext,
    pub interactive: Option<bool>,
    pub yolo: Option<bool>,
    pub cli_args: Vec<String>,
}

pub struct ProtectEvidence {
    pub summary: Option<String>,
    pub raw_command: Option<Vec<String>>,
    pub touched_paths: Vec<PathBuf>,
    pub domains: Vec<String>,
    pub mcp_server: Option<String>,
    pub mcp_tool: Option<String>,
}

pub enum ProtectOperation {
    ReadPath(PathBuf),
    WritePath(PathBuf),
    TraversePath(PathBuf),
    ExecuteCommand { argv: Vec<String>, escalated: bool },
    AccessNetwork,
    AccessDomain(String),
    AccessMcpServer(String),
    InvokeMcpTool { server: String, tool: String },
    SpawnSubagent,
    BypassApproval,
    SwitchMode(String),
    MutateProviderConfig(PathBuf),
}
```

This lets Protect reason from typed facts rather than weak textual clues.

## Recommended Protect Output Model

Protect should continue returning a single top-level decision for dispatch, but it should also expose per-operation detail.

Recommended shape:

```rust
pub struct ProtectDecision {
    pub outcome: ProtectOutcome,
    pub desired_outcome: ProtectOutcome,
    pub degraded: bool,
    pub reason: String,
    pub operation_results: Vec<ProtectOperationResult>,
    pub capability: Option<GateCapability>,
    pub audit: ProtectAuditContext,
}
```

Per-operation detail:

```rust
pub struct ProtectOperationResult {
    pub operation: ProtectOperation,
    pub policy_effect: PolicyEffect,
    pub certainty: PolicyCertainty,
    pub fidelity: MappingFidelity,
    pub risk: RiskLevel,
    pub explanation_summary: String,
    pub provenance: Vec<String>,
}
```

This preserves the dispatch simplicity of a single outcome while making the service inspectable and debuggable.

## Decision Model

The top-level runtime outcome should follow this precedence order:

1. `StopSession`
2. `StopCurrent`
3. `AskThenAllowOrStop`
4. `AllowWithRedaction`
5. `AdvisoryOnly`
6. `Allow`

Aggregation rules:

- any critical denied operation can force `StopSession`
- any denied pre-action operation should at least force `StopCurrent`
- any ask-required operation should force `AskThenAllowOrStop` unless a stronger stop already applies
- redaction should apply whenever output already contains sensitive or policy-disallowed data and hard blocking is unavailable
- advisory should be used when Claudine wants to intervene but the provider cannot enforce the stronger outcome

## Functional Scenarios

### Scenario 1: denied write before tool execution

1. A provider emits `BeforeTool`.
2. Protect derives `WritePath(/repo/.env)`.
3. `PolicyEngine` says `Deny`, with exact provenance from provider config.
4. Protect posture is `Strict`.
5. Desired outcome becomes `StopCurrent`.
6. Provider supports blocking deny.
7. Dispatch short-circuits and adapter formats a deny response.

### Scenario 2: ask-required command on an interactive provider

1. Protect derives `ExecuteCommand(["git", "push"], escalated=false)`.
2. `PolicyEngine` says `Ask`.
3. Provider and event support ask semantics.
4. Protect returns `AskThenAllowOrStop`.

### Scenario 3: policy unknown in advisory posture

1. Protect cannot prove whether domain access is allowed.
2. `PolicyEngine` returns `Unknown` with degraded fidelity warning.
3. Posture is `Advisory`.
4. Protect returns `AdvisoryOnly`, not a hard stop.

### Scenario 4: MCP tool result already returned sensitive data

1. Tool completed and produced JSON containing secrets.
2. Protect evaluates post-action.
3. Redaction rules match the payload.
4. Even if the original action already occurred, Protect returns `AllowWithRedaction` and records a post-action incident.

### Scenario 5: completion loop

1. Protect repeatedly issues non-allow outcomes on completion events in the same session.
2. Retry threshold is exceeded.
3. Protect upgrades to `StopSession`.

## Success Metrics

Success should be measured functionally, not only by code structure.

### Correctness

1. For supported providers, Protect decisions about path, command, network, MCP, and subagent activity must be derived from `PolicyEngine` results rather than duplicated local permission logic.
2. The same runtime input must yield the same decision when `PolicyEngine` inputs are unchanged.
3. Every non-allow decision must cite at least one concrete cause:
   - policy denial
   - policy ask
   - policy uncertainty under posture
   - redaction rule
   - runtime loop protection
   - provider capability degradation

### Explainability

1. Every decision should expose a machine-readable explanation path.
2. Auditable decisions must retain provenance back to `PolicyEngine` source IDs where applicable.
3. Degraded decisions must always indicate:
   - desired outcome
   - actual outcome
   - why degradation happened

### Integration quality

1. Dispatch must not need provider-specific permission logic.
2. Adapters must not need to understand `PolicyEngine`.
3. Protect config must not attempt to replicate provider-native permission configuration.

### Operational quality

1. Protect evaluations should be cheap enough for hook-time use.
2. Repeated identical queries in a session should benefit from snapshot reuse or caching.
3. Decision history must stay bounded in memory.

### Coverage

1. Protect must handle pre-action and post-action flows.
2. Protect must support filesystem, shell, network, MCP, and subagent decisioning where `PolicyEngine` exposes those axes.
3. Unsupported or ambiguous provider surfaces must degrade explicitly, not silently.

## Recommended Acceptance Criteria

1. A denied provider-native path permission causes a blocking pre-action protect decision when the provider can block.
2. An ask-style provider-native permission causes an ask outcome when the provider supports ask behavior.
3. An unknown policy result escalates differently under `Strict`, `Balanced`, and `Advisory` posture.
4. Post-action MCP payloads can be redacted without mutating `PolicyEngine`.
5. Protect decision records include policy provenance metadata.
6. Provider capability downgrade is observable in tests and exported audit records.
7. No provider-native config parsing remains in `ProtectService`.

## Migration Implications

The rewrite should proceed after or alongside `PolicyEngine`, not before it.

Recommended migration order:

1. Build `PolicyEngine` snapshots and query APIs.
2. Introduce new typed `ProtectOperation` extraction.
3. Replace Protect's internal permission heuristics with `PolicyEngine` queries.
4. Preserve current dispatch and adapter response behavior behind the new service.
5. Shrink or remove obsolete Claudine-owned permission-like config from `ProtectConfig`.
6. Expand reporting to include policy provenance.

## Open Questions

1. Which operations can be extracted reliably from each normalized event today, and which will still require partial heuristics?
2. How much provider CLI context is available inside hook dispatch for constructing exact effective policy snapshots?
3. Should redaction policy remain fully in Protect config, or should some redaction sensitivity be derivable from canonical policy metadata?
4. Do we want a separate public `ProtectInspector` or `ProtectAuditRecord` surface for CLI reporting?
5. Should session-local snapshot caching live inside Protect, inside `PolicyEngine`, or in a shared wrapper context?

## Final Design Decision

The rewritten `ProtectService` should be a runtime guard that consumes `PolicyEngine` answers, not a second permissions engine.

If `PolicyEngine` is the source of truth for what the provider allows, Protect becomes the source of truth for what Claudine does with that fact during a live session.
