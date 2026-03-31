# ProtectService Rewrite: Implementation Plan

Implements the design in `protect-final-design.md`. The current `ProtectService` is a single 2,278-line file (`services/protect.rs`) with its own rule-based permission matching. The rewrite splits it into 12 focused modules and delegates all permission truth to `PolicyEngine`.

## Baseline State

| Item | Location | Lines |
|------|----------|-------|
| Current ProtectService | `services/protect.rs` | 2,278 |
| Current re-exports | `services/mod.rs` | ~30 |
| PolicyEngine | `permissions/` (20 files) | ~15,000 |
| Provider adapters | `adapters/` (9 files) | ~3,000 |
| Dispatch integration | `dispatch/mod.rs` | ~400 |

All file paths below are relative to `claudine/lib/src/`.

---

## Phase 1: Module Skeleton and Config Split

**Goal:** Restructure the single `protect.rs` file into a module directory. Narrow `ProtectConfig` by removing fields that duplicate PolicyEngine truth. No functional changes to the evaluation pipeline yet.

### 1.1 Create module directory

Create `services/protect/` with these files (initially containing moved code + placeholder stubs):

| File | Source (from `protect.rs`) | Content |
|------|---------------------------|---------|
| `mod.rs` | New | Module declarations + re-exports |
| `service.rs` | Lines 17-99 | `ProtectService` struct and top-level methods |
| `config.rs` | Lines 697-1070 | `ProtectConfig`, validation, merge logic, posture types |
| `request.rs` | New (stub) | `ProtectRequest`, `ProtectSessionContext`, `ProtectCliContext` (Phase 2 populates) |
| `observe.rs` | New (stub) | `ProtectObservation`, `RuntimeFacts`, `ProtectPayload` (Phase 3 populates) |
| `intent.rs` | New (stub) | `ProtectIntent` enum (Phase 3 populates) |
| `evaluate.rs` | Lines 100-460 | Evaluation pipeline (`evaluate_enabled`, `desired_outcome`, policy evaluators) |
| `decision.rs` | Lines 1334-1430 | `ProtectOutcome`, `ProtectDecision`, `GateCapability`, `VisibilityLevel`, `ProviderProtectCapabilities` |
| `downgrade.rs` | Lines 631-696 | Capability downgrade logic |
| `redact.rs` | Lines 461-630 | MCP redaction: `McpTextRedaction`, `McpJsonRedaction`, helpers |
| `state.rs` | Lines 1850-1942 | `ProtectState`, `ProtectDecisionRecord`, `ProtectStateExport` |
| `explain.rs` | New (stub) | Explanation rendering (Phase 6 populates) |

Delete `services/protect.rs` after the split is complete.

### 1.2 Move types file by file

For each file, extract the relevant types and functions from `protect.rs` into the new module file. Preserve all existing tests by placing them in the module that owns the tested type.

**Move order** (least dependencies first):

1. `decision.rs` -- outcome types, gate capability, visibility, provider capabilities, provider profiles
2. `state.rs` -- `ProtectState`, `ProtectDecisionRecord`, `ProtectStateExport`
3. `redact.rs` -- `McpTextRedaction`, `McpJsonRedaction`, redaction helpers
4. `downgrade.rs` -- `downgrade_for_capability()` + tests
5. `config.rs` -- `ProtectConfig`, all policy structs, override structs, validation, merge, defaults
6. `evaluate.rs` -- evaluation pipeline functions (depends on config, decision, redact, downgrade)
7. `service.rs` -- `ProtectService` struct (depends on everything above)
8. `mod.rs` -- wire modules, re-export all public types

**Verification:** `cargo test -p claudine --lib` and `cargo check -p claudine` must pass after each file move.

### 1.3 Narrow ProtectConfig

Remove fields that duplicate PolicyEngine truth. Add new runtime-only fields.

**Fields to remove:**

| Removed field | Type | Reason |
|--------------|------|--------|
| `rules.blocked_command_patterns` | `Vec<String>` | PolicyEngine `CommandPolicy` |
| `rules.ask_command_patterns` | `Vec<String>` | PolicyEngine `CommandPolicy` |
| `rules.protected_paths` | `Vec<String>` | PolicyEngine `FilesystemPolicy` |
| `rules.secret_patterns` | `Vec<String>` | Moves to `McpRedactionPolicy.secret_patterns` |
| `mcp.allowlist` | `Vec<String>` | PolicyEngine `McpAccessPolicy` |
| `mcp.denylist` | `Vec<String>` | PolicyEngine `McpAccessPolicy` |
| `subagents` | `SubagentPolicy` | PolicyEngine `AgentPolicy` |
| `privilege.require_ask_for_network_writes` | `bool` | PolicyEngine `NetworkPolicy` |
| `privilege.require_ask_for_broad_fs_writes` | `bool` | PolicyEngine `FilesystemPolicy` |

After removal, delete `ProtectRules`, `ProtectRulesOverride`, `SubagentPolicy`, `SubagentPolicyOverride`, `SubagentProfile`.

**Fields to keep (unchanged):**

- `enabled`
- `posture` (`ProtectPosture`)
- `allow_repo_posture_downgrade`
- `yolo` (`YoloPolicy`)
- `completion` (`CompletionPolicy`)
- `max_recent_decisions`
- `providers` (`HashMap<Provider, ProviderProtectOverride>`)

**Fields to restructure:**

Replace `mcp: McpPolicy` and `privilege: PrivilegePolicy` with:

```rust
pub struct McpRedactionPolicy {
    pub redact_patterns: Vec<String>,
    pub block_instruction_payloads: bool,
    pub secret_patterns: Vec<String>,  // moved from rules.secret_patterns
}

pub struct RuntimeGuardPolicy {
    pub deny_when_root_without_sandbox: bool,  // kept from privilege
    pub ask_on_unknown_write: bool,
    pub ask_on_unknown_command: bool,
    pub ask_on_provider_config_mutation: bool,
}
```

New `ProtectConfig`:

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
```

**Update `ProviderProtectOverride`** to match (remove `rules`, `mcp`, `subagents`, `privilege` options; add `mcp_redaction`, `runtime_guards` options).

### 1.4 Config migration validation

Add validation in `ProtectConfig::validate()` (or a separate `ProtectConfig::from_value()` method) that detects removed fields in raw config values and returns targeted error messages.

Detection approach: before deserializing into the new `ProtectConfig`, check the raw `Value` for keys like `"rules"`, `"mcp.allowlist"`, `"mcp.denylist"`, `"subagents"`, etc. If present, return an error like:

> `protect.rules.blocked_command_patterns` has been removed. Command permissions are now managed through your provider's native permission config. See: `claudine hooks --describe` for provider config locations.

This requires a custom deserialization wrapper or a pre-deserialization check in the config loader.

### 1.5 Update services/mod.rs

Replace the single `mod protect;` with `mod protect;` pointing to the directory module. Update all re-exports to match the new public API. Ensure no downstream `use` paths break.

### Phase 1 deliverables

- [ ] Module directory `services/protect/` with 12 files
- [ ] All existing types moved without functional change
- [ ] `ProtectConfig` narrowed, old fields removed
- [ ] Config migration validation for removed fields
- [ ] `services/mod.rs` re-exports updated
- [ ] All existing tests pass
- [ ] `cargo check -p claudine` clean

---

## Phase 2: Engine Dependency and Session Context

**Goal:** Wire `PolicyEngine` into `ProtectService`. Introduce session context types. Update dispatch to construct contexts. The evaluation pipeline still uses the old logic (rule-based) but the engine is available.

### 2.1 Update ProtectService struct

```rust
pub struct ProtectService {
    engine: Arc<PolicyEngine>,
    config: ProtectConfig,
    state: ProtectState,
}
```

Remove `profiles: ProviderProtectProfiles` field. The service no longer maintains its own capability registry -- adapter capabilities are passed in per-evaluation.

**Update constructors:**

```rust
impl ProtectService {
    pub fn new(engine: Arc<PolicyEngine>, config: ProtectConfig) -> Self;
    pub fn config(&self) -> &ProtectConfig;
    pub fn state(&self) -> &ProtectState;
    pub fn export_state(&self) -> ProtectStateExport;
}
```

Remove `with_profiles()` and `profiles()`.

### 2.2 Define session context types in `request.rs`

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

pub struct ProtectRequest {
    pub provider: Provider,
    pub event: AgenticEvent,
    pub phase: ProtectPhase,
    pub session: ProtectSessionContext,
    pub observation: ProtectObservation,
}
```

Import `PolicyContext` from `crate::permissions::PolicyContext` and `ProviderCliOverrides` from `crate::permissions::ProviderCliOverrides`.

### 2.3 Define ProtectPolicyMode and ProtectEvaluation in `decision.rs`

```rust
pub enum ProtectPolicyMode {
    Effective,
    ConfiguredFallback,
}

pub struct ProtectEvaluation {
    pub decision: ProtectDecision,
    pub policy_mode: ProtectPolicyMode,
    pub findings: Vec<ProtectFinding>,
    pub redaction: Option<ProtectRedactionPlan>,
    pub warnings: Vec<PolicyWarning>,
}
```

`ProtectFinding` is a stub in this phase (populated in Phase 4). `ProtectRedactionPlan` is a stub (populated in Phase 5).

### 2.4 Add new public API methods

In `service.rs`, add the new entry points alongside the existing ones:

```rust
impl ProtectService {
    /// Full structured evaluation from a pre-built request.
    pub fn evaluate(&mut self, request: &ProtectRequest) -> Result<ProtectEvaluation>;

    /// Convenience entry point for dispatch.
    pub fn evaluate_event(
        &mut self,
        provider: Provider,
        event: AgenticEvent,
        meta: &EventMeta,
        ctx: &ProtectSessionContext,
        adapter: &dyn ProviderAdapter,
    ) -> Result<Option<ProtectEvaluation>>;
}
```

Rename the existing `evaluate(&mut self, input: &ProtectInput) -> ProtectDecision` to `evaluate_legacy(...)` temporarily. The new `evaluate()` delegates to `evaluate_legacy()` internally in this phase, wrapping the result in a `ProtectEvaluation` with empty findings and no redaction plan.

### 2.5 Snapshot resolution in `evaluate.rs`

Add snapshot resolution logic:

```rust
fn resolve_snapshot(
    engine: &PolicyEngine,
    session: &ProtectSessionContext,
) -> Result<(Box<dyn SnapshotQueryable>, ProtectPolicyMode)>
```

Where `SnapshotQueryable` is a local trait that abstracts over `ConfiguredPolicySnapshot` and `EffectivePolicySnapshot` (both have identical query surfaces):

```rust
trait SnapshotQueryable {
    fn query(&self, query: &PolicyQuery) -> QueryResult;
}

impl SnapshotQueryable for ConfiguredPolicySnapshot { ... }
impl SnapshotQueryable for EffectivePolicySnapshot { ... }
```

Resolution logic:
1. If `cli` is `Argv(args)` -> `engine.effective(provider, &ctx, CliPolicyInput::Argv(args))` -> `Effective`
2. If `cli` is `Parsed(overrides)` -> `engine.effective(provider, &ctx, CliPolicyInput::Parsed(overrides))` -> `Effective`
3. If `cli` is `None` -> `engine.configured(provider, &ctx)` -> `ConfiguredFallback`

This function is called but its results are not yet used for decisions (Phase 4 wires it in).

### 2.6 Update dispatch to build session context

In `dispatch/mod.rs`, construct `ProtectSessionContext` from available data:

```rust
let policy_ctx = PolicyContext::new(
    meta.env.cwd().unwrap_or_default().into(),
)
.with_repo_root(meta.env.repo_root().map(PathBuf::from))
.with_home_dir(dirs::home_dir());

let cli_ctx = std::env::var("AGENT_PARAMS")
    .ok()
    .map(|params| {
        let argv: Vec<String> = shell_words::split(&params).unwrap_or_default();
        ProtectCliContext::Argv(argv)
    })
    .unwrap_or(ProtectCliContext::None);

let session_ctx = ProtectSessionContext {
    provider,
    policy_context: policy_ctx,
    cli: cli_ctx,
    interactive: std::env::var("INTERACTIVE").ok().map_or(false, |v| v == "1" || v == "true"),
    yolo: std::env::var("YOLO").ok().map_or(false, |v| v == "1" || v == "true")
        || std::env::var("CLAUDINE_YOLO").ok().map_or(false, |v| v == "1" || v == "true"),
    session_id: std::env::var("CLAUDINE_SESSION_ID").ok(),
};
```

For this phase, dispatch continues to call the legacy `evaluate_legacy()` path. The new session context is constructed but only used for snapshot resolution (which is called but results discarded).

### 2.7 Update ProtectService construction in dispatch

```rust
let engine = Arc::new(PolicyEngine::new());
let mut protect_service = config.settings().protect.clone().map(|protect| {
    ProtectService::new(engine.clone(), protect)
});
```

Remove `ProviderProtectProfiles` construction from dispatch.

### Phase 2 deliverables

- [ ] `ProtectService` takes `Arc<PolicyEngine>`
- [ ] `ProtectSessionContext`, `ProtectCliContext` types defined
- [ ] `ProtectEvaluation`, `ProtectPolicyMode` types defined
- [ ] New `evaluate()` and `evaluate_event()` public API (delegates to legacy internally)
- [ ] Snapshot resolution function implemented
- [ ] Dispatch builds `ProtectSessionContext` from env vars
- [ ] Dispatch constructs `PolicyEngine` and passes it to service
- [ ] All existing tests pass
- [ ] `cargo check -p claudine` clean

---

## Phase 3: Provider-Aware Observation

**Goal:** Replace generic `ProtectInput::from_event_meta(...)` with provider-aware intent extraction via adapters.

### 3.1 Define observation types in `observe.rs`

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

### 3.2 Define intent types in `intent.rs`

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

Import `PathQuery`, `CommandQuery`, `DomainQuery` from `crate::permissions::query`.

Add `ProtectIntent::to_policy_query() -> Option<PolicyQuery>` conversion:
- All variants map directly except `CompletionOutputScan` which returns `None`.

### 3.3 Add `observe_protect()` to `ProviderAdapter` trait

In `adapters/mod.rs`, add a default method:

```rust
fn observe_protect(
    &self,
    event: &AgenticEvent,
    meta: &EventMeta,
) -> Option<ProtectObservation> {
    default_observe_protect(event, meta)
}
```

The `default_observe_protect()` function mirrors the current `ProtectInput::from_event_meta()` logic but produces `ProtectObservation` with `ProtectIntent` variants instead of flat fields:

- `meta.tool_name` + write indicators -> `WritePath`/`ReadPath` intents
- `command` field -> `ExecuteCommand` intent
- `paths` -> `ReadPath`/`WritePath`/`TraversePath` intents based on event type
- `mcp_server_id` -> `UseMcpServer` + optionally `UseMcpTool` intents
- `agent_type` on SubagentStart -> `SpawnSubagent` intent
- Completion event -> `CompletionOutputScan` intent
- Runtime facts extracted from meta.extra (root, sandbox, bypass)

### 3.4 Implement Claude-specific `observe_protect()`

Override in `ClaudeAdapter` to properly extract intents from Claude's tool call structures:

- `write_file` / `edit_file` -> `WritePath` with the target path
- `read_file` / `read_directory` -> `ReadPath`
- `bash` / `execute_command` -> `ExecuteCommand` with the command string
- `mcp__*` tool names -> `UseMcpServer` + `UseMcpTool` (parse server/tool from tool name)
- Permission mode changes -> `ModifyProviderConfig`

Other adapters can keep the default for now (the design says "start with default, improve per provider").

### 3.5 Migrate dispatch to use observations

Replace the current dispatch flow:

```rust
// Before:
ProtectInput::from_event_meta(provider, event, &meta)
    .map(|input| service.evaluate_legacy(&input))

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

At this point `evaluate()` still delegates to `evaluate_legacy()` internally, but it translates the `ProtectObservation` back into a `ProtectInput` for the legacy path. This ensures behavioral continuity.

### 3.6 Build post-action observation

Replace `build_post_action_input()` with `build_post_action_observation()` that constructs a `ProtectObservation` from the action response. This is simpler since post-action observations primarily need `CompletionOutputScan` and MCP payload intents.

### 3.7 Deprecate ProtectInput

Mark `ProtectInput` and `ProtectInput::from_event_meta()` as `#[deprecated]`. They will be removed after Phase 4 completes.

### Phase 3 deliverables

- [ ] `ProtectObservation`, `RuntimeFacts`, `ProtectPayload` types defined
- [ ] `ProtectIntent` enum with `to_policy_query()` conversion
- [ ] `observe_protect()` added to `ProviderAdapter` trait with default implementation
- [ ] Claude-specific `observe_protect()` override
- [ ] Dispatch migrated to use observations
- [ ] Post-action observation builder
- [ ] `ProtectInput` deprecated
- [ ] All existing tests pass
- [ ] `cargo check -p claudine` clean

---

## Phase 4: Policy-Backed Evaluation

**Goal:** Replace `ProtectRules`-based permission decisions with `PolicyEngine` snapshot queries. Implement the full decision matrix. This is the core behavioral change.

### 4.1 Define ProtectFinding in `decision.rs`

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

### 4.2 Implement severity classification

In `evaluate.rs`, add:

```rust
fn classify_severity(
    intent: &ProtectIntent,
    result: &QueryResult,
    posture: ProtectPosture,
) -> ProtectSeverity
```

Rules:
- `Deny` effect + destructive operation (config mutation, broad write) -> `Critical`
- `Deny` effect + other -> `High`
- `Ask` effect -> `Medium`
- `Allow` effect -> `Info`
- `Unknown`/`BestEffort` under Strict -> elevated by one level
- `CompletionOutputScan` results follow their own severity (secret found -> `High`, loop detected -> `Critical`)

### 4.3 Implement the new evaluation pipeline in `evaluate.rs`

Replace the body of `evaluate()` (which currently delegates to `evaluate_legacy()`) with the full pipeline:

**Step 1: Resolve provider runtime posture**

```rust
fn resolve_posture(
    config: &ProtectConfig,
    session: &ProtectSessionContext,
) -> ProtectPosture
```

Merge base posture with provider-specific override.

**Step 2: Resolve snapshot**

Call `resolve_snapshot()` (implemented in Phase 2).

**Step 3: Query each intent**

For each `ProtectIntent` in the observation:
- If `intent.to_policy_query()` returns `Some(pq)`, call `snapshot.query(&pq)`
- If `CompletionOutputScan`, run Protect-owned logic (secret scan, loop detection)
- Build a `ProtectFinding` for each

**Step 4: Apply runtime guards**

After policy findings, evaluate `RuntimeGuardPolicy`:

```rust
fn apply_runtime_guards(
    config: &ProtectConfig,
    runtime: &RuntimeFacts,
    posture: ProtectPosture,
    findings: &mut Vec<ProtectFinding>,
)
```

- Root without sandbox -> `ProtectFinding` with `RuntimeGuard` source
- `ask_on_unknown_write` / `ask_on_unknown_command` for `Unknown` results
- `ask_on_provider_config_mutation` for `ModifyProviderConfig` intent

**Step 5: Select desired outcome**

```rust
fn select_desired_outcome(
    findings: &[ProtectFinding],
    posture: ProtectPosture,
    session: &ProtectSessionContext,
) -> ProtectOutcome
```

Implement the decision semantics matrix from the design:

| Effect | Exact | BestEffort (Advisory) | BestEffort (Balanced) | BestEffort (Strict) |
|--------|-------|-----------------------|-----------------------|---------------------|
| Allow | Allow | AdvisoryOnly | Allow | AskThenAllowOrStop |
| Ask | AskThenAllowOrStop | AdvisoryOnly | AskThenAllowOrStop | StopCurrent |
| Deny | StopCurrent | AdvisoryOnly | StopCurrent | StopCurrent |

For `None`/`Unknown` under Balanced:
- Writes, commands, MCP tools, config mutation -> `AskThenAllowOrStop`
- Reads, domain access -> `AdvisoryOnly`

Aggregate across all findings using precedence: `StopSession` > `StopCurrent` > `AskThenAllowOrStop` > `AllowWithRedaction` > `AdvisoryOnly` > `Allow`

**Step 6: Apply YOLO mode**

If session is in yolo mode, apply `YoloPolicy` rules:
- `force_advisory_for_medium_risk` -> downgrade medium-risk Ask to AdvisoryOnly
- `allow_critical_blocking` -> allow critical blocking if true

**Step 7: Downgrade through provider capabilities**

Call `downgrade_for_capability()` using adapter-provided capabilities (passed into evaluation, not from internal registry).

**Step 8: Build ProtectEvaluation**

Assemble the final `ProtectEvaluation` with decision, findings, policy mode, warnings.

### 4.4 Update ProtectDecision

```rust
pub struct ProtectDecision {
    pub outcome: ProtectOutcome,
    pub desired_outcome: ProtectOutcome,
    pub degraded: bool,
    pub reason: String,
    pub capability: Option<GateCapability>,
}
```

Note: `desired_outcome` replaces `degraded_from: Option<ProtectOutcome>`. The `degraded` flag is now derived: `desired_outcome != outcome`.

### 4.5 Remove legacy evaluation path

Delete:
- `evaluate_legacy()` method
- `evaluate_enabled()` method
- `desired_outcome()` method
- `evaluate_privilege_policy()` / `evaluate_rule_policy()` / `evaluate_mcp_policy()` / `fallback_risk_outcome()`
- `ProtectInput` struct and `from_event_meta()`
- All helper functions in the old pipeline (`command_blob`, `text_blob`, `path_matches`, `match_patterns`, etc.)
- `ProtectRuntimeMode`, `RiskLevel` enums (severity is now `ProtectSeverity`, risk is now `ProtectFinding.severity`)

### 4.6 Update completion retry logic

Preserve the per-session retry counter. The trigger is now: any `ProtectFinding` with `source: CompletionLoop` and the retry count exceeding `completion.max_retries` upgrades the outcome to `StopSession`.

### 4.7 Write new unit tests

Test the decision matrix exhaustively:

```
test_exact_allow_returns_allow
test_exact_deny_returns_stop_current
test_exact_ask_returns_ask
test_best_effort_allow_advisory_posture_returns_advisory
test_best_effort_allow_balanced_posture_returns_allow
test_best_effort_allow_strict_posture_returns_ask
test_best_effort_deny_balanced_returns_stop_current
test_unknown_write_balanced_returns_ask
test_unknown_read_balanced_returns_advisory
test_unknown_write_strict_returns_stop_current
test_root_without_sandbox_escalates
test_provider_config_mutation_balanced_returns_ask
test_provider_config_mutation_strict_returns_stop
test_yolo_degrades_medium_ask_to_advisory
test_completion_retry_exceeded_returns_stop_session
test_multiple_intents_highest_severity_wins
test_mcp_secret_pattern_returns_allow_with_redaction
```

Use mock `PolicyEngine` with `StubBackend` for controlled snapshot results.

### Phase 4 deliverables

- [ ] `ProtectFinding`, `ProtectFindingSource`, `ProtectSeverity` types
- [ ] Severity classification logic
- [ ] Full evaluation pipeline (steps 1-8)
- [ ] Decision semantics matrix implemented
- [ ] Runtime guard evaluation
- [ ] YOLO mode handling
- [ ] `ProtectDecision` updated with `desired_outcome`
- [ ] Legacy evaluation path removed
- [ ] `ProtectInput` removed
- [ ] Completion retry logic updated
- [ ] Comprehensive unit tests for decision matrix
- [ ] All tests pass
- [ ] `cargo check -p claudine` clean

---

## Phase 5: Redaction Integration

**Goal:** Return `ProtectRedactionPlan` from evaluation. Apply redaction in dispatch before adapter formatting. Remove standalone redaction methods.

### 5.1 Define ProtectRedactionPlan in `redact.rs`

```rust
pub enum ProtectRedactionPlan {
    ReplaceText(McpTextRedaction),
    ReplaceJson(McpJsonRedaction),
    BlockPayload { reason: String },
}
```

### 5.2 Generate redaction plan during evaluation

In the evaluation pipeline (Step 4 or as a post-step), when an observation has `ProtectPayload::McpText(...)` or `ProtectPayload::McpJson(...)`:

1. Check `mcp_redaction.block_instruction_payloads` -- if triggered, return `BlockPayload`.
2. Apply `mcp_redaction.redact_patterns` + `mcp_redaction.secret_patterns` -- if any match, return `ReplaceText`/`ReplaceJson`.
3. Otherwise `None`.

The existing `redact_mcp_text()` and `redact_mcp_json()` internal logic is reused but the result is packaged as `ProtectRedactionPlan` on `ProtectEvaluation.redaction`.

### 5.3 Apply redaction in dispatch

After post-action evaluation, before formatting:

```rust
if let Some(eval) = &protect_post {
    if let Some(plan) = &eval.redaction {
        action_response = apply_redaction(action_response, plan);
    }
}
```

Implement `apply_redaction(response: HookResponse, plan: &ProtectRedactionPlan) -> HookResponse`:
- `ReplaceText` -> replace `response.additional_context` or relevant text field
- `ReplaceJson` -> replace `response.updated_input` or relevant JSON field
- `BlockPayload` -> clear payload and set reason

### 5.4 Remove standalone redaction methods

Remove `ProtectService::redact_mcp_text()` and `ProtectService::redact_mcp_json()` as public methods. Redaction is now part of evaluation output.

### 5.5 Update `AllowWithRedaction` outcome

The `AllowWithRedaction` outcome is now only set when `ProtectEvaluation.redaction` is `Some(...)`. The finding source is `McpRedaction`.

### Phase 5 deliverables

- [ ] `ProtectRedactionPlan` enum defined
- [ ] Redaction plan generated during evaluation
- [ ] Dispatch applies redaction before adapter formatting
- [ ] Standalone `redact_mcp_text` / `redact_mcp_json` removed
- [ ] Tests for redaction plan generation and application
- [ ] All tests pass

---

## Phase 6: Observability and Remediation

**Goal:** Expand decision records with new fields. Add optional remediation hints. Update export schema.

### 6.1 Expand ProtectDecisionRecord in `state.rs`

```rust
pub struct ProtectDecisionRecord {
    pub provider: Provider,
    pub phase: ProtectPhase,
    pub outcome: ProtectOutcome,
    pub desired_outcome: ProtectOutcome,
    pub degraded: bool,
    pub reason: String,
    pub session_id: Option<String>,
    // New fields:
    pub policy_mode: ProtectPolicyMode,
    pub intent_count: usize,
    pub finding_sources: Vec<ProtectFindingSource>,
    pub certainty_summary: PolicyCertainty,
    pub source_ids: Vec<String>,
    pub redaction_applied: bool,
    pub provider_warnings: usize,
    pub completion_retry_count: Option<u8>,
}
```

`certainty_summary` is the worst (least confident) certainty across all findings.

### 6.2 Update record creation

In `service.rs`, when recording a decision, populate the new fields from `ProtectEvaluation`:

```rust
fn record_evaluation(&mut self, eval: &ProtectEvaluation) {
    let record = ProtectDecisionRecord {
        policy_mode: eval.policy_mode.clone(),
        intent_count: eval.findings.len(),
        finding_sources: eval.findings.iter()
            .map(|f| f.source.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect(),
        certainty_summary: eval.findings.iter()
            .map(|f| f.result.certainty)
            .min()  // worst certainty
            .unwrap_or(PolicyCertainty::Unknown),
        source_ids: eval.findings.iter()
            .flat_map(|f| f.result.matched_rules.iter().map(|r| r.provenance.source_id.clone()))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect(),
        redaction_applied: eval.redaction.is_some(),
        provider_warnings: eval.warnings.len(),
        // ... existing fields from decision
    };
    self.state.record(record);
}
```

### 6.3 Implement explain in `explain.rs`

```rust
pub struct ProtectExplanation {
    pub summary: String,
    pub findings: Vec<FindingExplanation>,
    pub policy_mode: ProtectPolicyMode,
    pub remediation: Option<ProtectRemediation>,
}

pub struct FindingExplanation {
    pub intent_description: String,
    pub effect: Option<PolicyEffect>,
    pub certainty: PolicyCertainty,
    pub matched_rules: Vec<String>,  // human-readable rule descriptions
    pub severity: ProtectSeverity,
}

pub struct ProtectRemediation {
    pub one_time_args: Option<Vec<String>>,
    pub summary: String,
}
```

Add `explain_last(&self) -> Option<ProtectExplanation>` to `ProtectService`. This renders the most recent evaluation's findings into human-readable form.

### 6.4 Update JSONL export

`export_records_jsonl()` already serializes `ProtectDecisionRecord`. The new fields will be included automatically via serde. Verify the output schema is correct.

### 6.5 Remove unused types from re-exports

Clean up `services/mod.rs` re-exports. Remove types that are no longer public:
- `ProtectInput`
- `ProtectRuntimeMode`
- `RiskLevel`
- `ProtectRules`, `ProtectRulesOverride`
- `SubagentPolicy`, `SubagentPolicyOverride`, `SubagentProfile`
- `PrivilegePolicy`, `PrivilegePolicyOverride`
- `McpPolicy`, `McpPolicyOverride`
- `ProviderProtectProfiles`

Add new types:
- `ProtectEvaluation`
- `ProtectFinding`, `ProtectFindingSource`, `ProtectSeverity`
- `ProtectPolicyMode`
- `ProtectSessionContext`, `ProtectCliContext`
- `ProtectRequest`
- `ProtectObservation`, `RuntimeFacts`, `ProtectPayload`
- `ProtectIntent`
- `ProtectRedactionPlan`
- `McpRedactionPolicy`
- `RuntimeGuardPolicy`
- `ProtectExplanation`, `ProtectRemediation`

### Phase 6 deliverables

- [ ] `ProtectDecisionRecord` expanded with new fields
- [ ] Record creation populated from `ProtectEvaluation`
- [ ] `explain_last()` method implemented
- [ ] `ProtectExplanation`, `ProtectRemediation` types
- [ ] JSONL export updated
- [ ] Re-exports cleaned up
- [ ] All tests pass
- [ ] `cargo check -p claudine` clean

---

## Cross-Cutting Concerns

### Snapshot caching

Within a single `ProtectService` instance, cache snapshots keyed by:
- `(provider, cwd, repo_root, trust_context_hash, cli_fingerprint)`

Use a simple `HashMap` with these keys. Clear on service reconstruction. This avoids re-parsing provider config across pre-action and post-action evaluations in the same hook process.

Implement in Phase 2 (alongside snapshot resolution) but only activate in Phase 4 when snapshots are actually consumed for decisions.

### Testing strategy summary

| Phase | Test type | What to test |
|-------|-----------|-------------|
| 1 | Unit | Config validation, deprecated field rejection, merge rules |
| 2 | Unit | Session context construction, snapshot resolution |
| 3 | Unit | Intent extraction per provider, observation shape |
| 4 | Unit + Service | Decision matrix (posture x certainty x effect), capability downgrade, YOLO, completion loops |
| 5 | Unit + Integration | Redaction plan generation, dispatch redaction application |
| 6 | Unit | Record population, explanation rendering, JSONL export |

### Dispatch integration tests

Add or update dispatch tests for:
- Pre-action deny short-circuits correctly
- Post-action MCP response is redacted before adapter formatting
- Effective policy uses wrapper-provided `AGENT_PARAMS`
- Configured fallback is marked when CLI context is unavailable
- `ProtectEvaluation.warnings` includes policy warnings

### Breaking changes

| Change | Impact | Mitigation |
|--------|--------|-----------|
| `ProtectConfig` field removal | User configs with removed fields | Targeted validation errors at load time |
| `ProtectService::new()` signature | Dispatch, tests | Update all call sites |
| `ProtectDecision` field changes | Consumers of decision records | Update dispatch, JSONL consumers |
| Removed `redact_mcp_*` public methods | Any external callers | None expected; these were internal |
| `ProviderProtectProfiles` removal | Dispatch construction | Already replaced by adapter-provided capabilities |

---

## Dependency Graph

```
Phase 1 (skeleton + config)
    |
    v
Phase 2 (engine + context) ----+
    |                           |
    v                           |
Phase 3 (observation) ----+    |
    |                      |    |
    v                      v    v
Phase 4 (evaluation) <----+----+
    |
    +--------+
    |        |
    v        v
Phase 5    Phase 6
(redact)   (observability)
```

Phases 5 and 6 are independent of each other and can be done in parallel after Phase 4.

---

## Risk Checklist

- [ ] **Intent extraction quality:** Start with default `observe_protect` that mirrors current behavior. Test with fixtures. Only improve after baseline works.
- [ ] **PolicyEngine query quality:** Decision matrix handles `Unknown` and `BestEffort` from day one. Don't assume stability/warnings are populated.
- [ ] **Breaking config:** Reject deprecated fields with targeted messages naming the removed field and pointing to provider config.
- [ ] **Effective policy unavailable:** Support `ConfiguredFallback` explicitly. Surface in telemetry.
- [ ] **Behavioral regression:** Phase 3 translates observations back to legacy path to verify. Phase 4 replaces with new path. Compare outputs on fixture events before removing legacy.

---

## Open Decisions (to resolve before or during implementation)

1. **`observe_protect` location:** On `ProviderAdapter` trait or separate `ProtectObserver` trait? Design recommends adapter (keeps provider surface unified). Plan follows this recommendation.

2. **Per-axis certainty control:** Posture-wide for v1. No per-axis config.

3. **MCP secret scanning:** Stays in Protect. Not extracted to separate service.

4. **`explain_last()` scope:** Convenience method on `ProtectService`. Full CLI report consumes `PolicyEngine` directly (not part of this plan).
