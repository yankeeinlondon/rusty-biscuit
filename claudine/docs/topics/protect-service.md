# Protect Service

The Protect service is Claudine's runtime decision layer that evaluates LLM agent actions against security policies. It sits between provider adapters and the PolicyEngine, translating raw agentic events into structured intents, evaluating them against policy snapshots, and producing capability-aware decisions that respect each provider's enforcement surface.

## Architecture

```
Provider Adapter ──observe──> ProtectObservation
                                    │
                              ProtectRequest
                                    │
                            ProtectService.evaluate_structured()
                                    │
                    ┌───────────────┼───────────────────┐
                    ▼               ▼                   ▼
             PolicyEngine    Runtime Guards      Redaction Engine
                    │               │                   │
                    └───────┬───────┘                   │
                            ▼                           │
                    Decision Matrix                     │
                            │                           │
                    YOLO Mode Override                  │
                            │                           │
                    Capability Downgrade                │
                            │                           │
                    Completion Loop Check               │
                            │                           │
                            └───────────┬───────────────┘
                                        ▼
                                ProtectEvaluation
```

### Module Layout

| Module | Purpose |
|---|---|
| `service.rs` | `ProtectService` — central actor, owns engine + config + state |
| `config.rs` | `ProtectConfig` and all nested policy structs |
| `evaluate.rs` | 8-step evaluation pipeline |
| `decision.rs` | Outcome types, capability profiles, finding structs |
| `intent.rs` | `ProtectIntent` enum — maps 1:1 to `PolicyQuery` |
| `observe.rs` | `ProtectObservation` extraction from events |
| `request.rs` | `ProtectRequest` and session context |
| `redact.rs` | MCP payload redaction (text + JSON) |
| `downgrade.rs` | Capability-aware outcome degradation |
| `explain.rs` | Human-readable explanation rendering |
| `state.rs` | Rolling decision records and audit export |

## Evaluation Pipeline

`ProtectService::evaluate_structured()` delegates to `evaluate_request()` which runs an 8-step pipeline:

### Step 1: Resolve Posture

Merges the base `ProtectPosture` with any provider-specific override. The three postures control baseline aggressiveness:

| Posture | Behavior |
|---|---|
| **Advisory** | Never block. Collect findings and recommend actions only. |
| **Balanced** (default) | Ask for risky actions, block critical ones. |
| **Strict** | Hard-stop for any high-confidence dangerous behavior. |

### Step 2: Resolve Policy Snapshot

Queries the `PolicyEngine` for either an effective snapshot (when CLI args are available) or a configured fallback. The snapshot type is tracked as `ProtectPolicyMode`:

- **Effective** — full policy resolved with CLI argv or parsed overrides
- **ConfiguredFallback** — static configured policy (no CLI context)

Resolution order: `ProtectCliContext::Argv` or `::Parsed` tries `engine.effective()` first, falls back to `engine.configured()`. `ProtectCliContext::None` goes straight to configured fallback.

### Step 3: Query Each Intent

Each `ProtectIntent` in the observation is converted to a `PolicyQuery` via `to_policy_query()` and evaluated against the snapshot. The result is a `QueryResult` containing:

- `effect`: `Allow`, `Ask`, or `Deny`
- `certainty`: `Exact`, `BestEffort`, or `Unknown`
- `matched_rules`: which policy rules matched
- `warnings`: any policy warnings

The special `CompletionOutputScan` intent has no corresponding `PolicyQuery` and is handled by the completion scanner instead (see Step 7b below).

Severity is classified per finding:

| Effect | Base Severity | Destructive Intent |
|---|---|---|
| Deny | High | Critical |
| Ask | Medium | Medium |
| Allow | Info | Info |
| None/Unknown | Medium | Medium |

Under **Strict** posture, uncertain results (`BestEffort` or `Unknown` certainty) are elevated one severity level.

### Step 4: Apply Runtime Guards

Synthetic findings are injected for privilege escalation scenarios:

| Guard | Condition | Severity |
|---|---|---|
| Root without sandbox | `is_root && has_sandbox == false` | Critical (Strict) / High (other) |
| Network write | Command is `curl`, `wget`, `ssh`, etc. and policy allowed | Medium |
| Broad FS write | Write targets `/etc/`, `/usr/`, `/var/`, `/opt/`, `/System/`, dotfiles | Medium |

Runtime guards only fire when the corresponding `PrivilegePolicy` flag is enabled (all default to `true`).

### Step 5: Select Desired Outcome (Decision Matrix)

Each finding is mapped through the decision semantics matrix. The outcome with the highest precedence wins.

**Decision semantics by certainty and posture:**

| Effect | Exact | BestEffort + Advisory | BestEffort + Balanced | BestEffort + Strict |
|---|---|---|---|---|
| Allow | Allow | AdvisoryOnly | Allow | AskThenAllowOrStop |
| Ask | AskThenAllowOrStop | AdvisoryOnly | AskThenAllowOrStop | StopCurrent |
| Deny | StopCurrent | AdvisoryOnly | StopCurrent | StopCurrent |

For `Unknown` certainty under **Balanced** posture, write/command/MCP-tool/config-mutation intents produce `AskThenAllowOrStop`; read-like intents produce `AdvisoryOnly`.

**Outcome precedence** (highest priority wins):

| Priority | Outcome |
|---|---|
| 6 | `StopSession` |
| 5 | `StopCurrent` |
| 4 | `AskThenAllowOrStop` |
| 3 | `AllowWithRedaction` |
| 2 | `AdvisoryOnly` |
| 1 | `Allow` |

### Step 6: Apply YOLO Mode

When `session.yolo == true`, outcomes may be softened:

- `force_advisory_for_medium_risk` (default: `true`): medium-or-below `AskThenAllowOrStop` becomes `AdvisoryOnly`
- `allow_critical_blocking` (default: `true`): when `false`, blocking outcomes (`StopCurrent`/`StopSession`) become `AdvisoryOnly`

### Step 7: Build Redaction Plan

If the observation carries an MCP payload (`McpText` or `McpJson`), the redaction engine checks for:

1. **Instruction injection** — hardcoded phrases like "ignore previous instructions", "system prompt", etc. When `block_instruction_payloads` is `true` (default), the entire payload is blocked.
2. **Secret patterns** — regex patterns from `rules.secret_patterns` and `mcp.redact_patterns` replace matches with `[REDACTED]`.

If redaction is applied and the outcome was `Allow` or `AdvisoryOnly`, it upgrades to `AllowWithRedaction`.

Redaction plans:

| Plan | When |
|---|---|
| `BlockPayload` | Instruction injection detected |
| `ReplaceText` | Secret patterns matched in text payload |
| `ReplaceJson` | Secret patterns matched in JSON payload (recursive visitor) |

### Step 8: Assemble Evaluation

The final `ProtectEvaluation` bundles:
- `decision`: outcome + desired outcome + degraded flag + reason + capability
- `policy_mode`: Effective or ConfiguredFallback
- `findings`: all findings from policy queries, runtime guards, and completion scans
- `redaction`: optional redaction plan
- `warnings`: policy warnings from the engine

## Capability-Aware Downgrade

After the evaluation pipeline, `ProtectService` applies provider capability downgrade. If the provider cannot enforce the desired outcome, it degrades to `AdvisoryOnly`:

| Desired Outcome | Required Capability | Degraded To |
|---|---|---|
| `StopCurrent` | `can_stop_current()` | `AdvisoryOnly` |
| `StopSession` | `can_stop_session()` | `AdvisoryOnly` |
| `AskThenAllowOrStop` | `can_ask_user()` | `AdvisoryOnly` |
| `AllowWithRedaction` | `can_modify()` | `AdvisoryOnly` |

Capabilities are resolved per-phase:

| Phase | Capability Field |
|---|---|
| `BeforeTool` | `pre_tool_gate` |
| `BeforePrompt` | `user_prompt_gate` |
| `AfterTool` | `post_tool_gate` |
| `McpResponse` | `mcp_response_gate` |
| `Completion` | `completion_gate` |
| `SubagentStart`/`SubagentStop` | `subagent_visibility` (mapped to gate) |
| `Runtime` | `Influence` (hardcoded) |

### Gate Capability Levels

| Level | can_ask_user | can_modify | can_stop_current | can_stop_session |
|---|---|---|---|---|
| `None` | no | no | no | no |
| `Influence` | yes | yes | yes | no |
| `Guarantee` | yes | yes | yes | yes |

### Default Provider Profiles

| Provider | pre_tool | post_tool | prompt | mcp_response | completion | sandbox | bypass |
|---|---|---|---|---|---|---|---|
| Claude | Guarantee | Guarantee | Guarantee | Guarantee | Guarantee | yes | yes |
| Codex | None | None | None | None | None | yes | yes |
| Gemini | Guarantee | Guarantee | Guarantee | Guarantee | Guarantee | yes | yes |
| Goose | None | None | None | None | None | yes | yes |
| Kimi Code | Guarantee | None | Influence | None | None | no | yes |
| OpenCode | Guarantee | None | Influence | Influence | Influence | no | no |
| Qwen Code | Influence | None | None | None | None | yes | yes |
| Roo Code | Guarantee | None | None | None | Guarantee | no | yes |

## Completion Loop Protection

After capability downgrade, the service applies completion retry policy. For `Completion` phase evaluations that produce a non-Allow outcome:

1. A per-session retry counter increments
2. When `retry_count > max_retries` (default: 3), the outcome escalates to `StopSession`
3. If the evaluation produces `Allow` or `AllowWithRedaction`, the counter resets

## Intents

`ProtectIntent` maps 1:1 to `PolicyQuery` variants (except `CompletionOutputScan`):

| Intent | Description |
|---|---|
| `ReadPath(PathQuery)` | File/directory read |
| `WritePath(PathQuery)` | File/directory write |
| `TraversePath(PathQuery)` | Directory traversal |
| `ExecuteCommand(CommandQuery)` | Shell command execution |
| `AccessDomain(DomainQuery)` | Network domain access |
| `UseMcpServer { server }` | MCP server usage |
| `UseMcpTool { server, tool }` | MCP tool invocation |
| `SpawnSubagent { name }` | Subagent creation |
| `SwitchMode { target }` | Runtime mode change |
| `ModifyProviderConfig` | Provider configuration mutation |
| `CompletionOutputScan` | Completion output validation (Protect-internal, no PolicyQuery) |

## Observation Extraction

`ProtectObservation` is extracted from events by provider adapters via `observe_protect()`. The default extractor (`default_observe_protect`) handles:

- **Command intent**: from `tool_input.command` or `meta.prompt`
- **Path intents**: from `tool_input.{path,file,target}`; classified as write when tool name contains "write", "edit", "create", or "delete"
- **MCP server intent**: from `meta.extra.mcp_server_id`
- **Subagent intent**: on `SubagentStart` events, using `meta.agent_type`
- **Completion scan**: automatically added on `TurnComplete` events
- **Runtime facts**: `is_root` from uid/flag, `has_sandbox` from sandbox_enabled/mode, `bypass_mode` from permission/approval/sandbox/execution mode fields
- **Payload**: from `meta.tool_response` (string becomes `McpText`, object becomes `McpJson`)

## Configuration

`ProtectConfig` is the top-level configuration, loaded from `settings.protect` in `~/.claudine/config.json`.

### Active Configuration Fields

```json
{
  "enabled": true,
  "posture": "balanced",
  "allow_repo_posture_downgrade": false,
  "yolo": {
    "allow_critical_blocking": true,
    "force_advisory_for_medium_risk": true,
    "collect_forensic_trail": true
  },
  "rules": {
    "secret_patterns": ["sk-[a-z0-9]+"]
  },
  "completion": {
    "enabled": true,
    "max_retries": 3,
    "check_commands": ["rm\\s+-rf"],
    "secret_scan": true
  },
  "mcp": {
    "redact_patterns": ["token=[a-z0-9]+"],
    "block_instruction_payloads": true
  },
  "subagents": {
    "enabled": true,
    "tighten_permissions": true,
    "default_profile": "read_mostly"
  },
  "privilege": {
    "deny_when_root_without_sandbox": true,
    "require_ask_for_network_writes": true,
    "require_ask_for_broad_fs_writes": true
  },
  "providers": {},
  "max_recent_decisions": 256
}
```

### Deprecated Fields (Hard Errors on Use)

These fields have migrated to PolicyEngine and produce validation errors:

- `rules.blocked_command_patterns`
- `rules.ask_command_patterns`
- `rules.protected_paths`
- `mcp.allowlist`
- `mcp.denylist`

### Provider Overrides

Each provider can have partial overrides layered on top of the base config:

```json
{
  "providers": {
    "codex": {
      "enabled": true,
      "posture": "advisory",
      "completion": { "enabled": false }
    }
  }
}
```

`provider_aware_defaults()` automatically softens low-control providers (Codex, Goose, Qwen Code) to Advisory posture with completion disabled.

### Validation

`ProtectConfig::validate()` checks:
- Deprecated fields are not populated
- `max_recent_decisions` is 1..=10000
- `max_retries > 0` when completion is enabled
- All regex patterns in `secret_patterns` and `redact_patterns` compile

## State and Auditing

`ProtectState` maintains an in-memory rolling log of decisions:

- `decision_count`: total evaluations
- `recent`: bounded `VecDeque<ProtectDecisionRecord>` (capped at `max_recent_decisions`)
- `completion_retries_by_session`: per-session retry counters

Each `ProtectDecisionRecord` captures:
- Provider, phase, outcome, desired outcome, degraded flag
- Policy mode, intent count, finding sources, certainty summary
- Matched rule source IDs, redaction status, warning count
- Session ID and completion retry count

Export formats:
- `export_state()` — full `ProtectStateExport` struct
- `export_records_jsonl()` — one JSON object per line
- `snapshot_records()` — cloned record list

## Explanations

`explain_last()` renders the most recent evaluation into a `ProtectExplanation`:

- `summary`: outcome, desired outcome, and degraded status
- `findings`: per-finding detail with intent description, effect, certainty, matched rules, severity
- `policy_mode`: which snapshot type was used
- `remediation`: suggested actions when decisions are degraded or running in ConfiguredFallback mode

## Phases

| Phase | Agentic Events | Description |
|---|---|---|
| `BeforePrompt` | `BeforePrompt` | Before model processes user input |
| `BeforeTool` | `BeforeTool`, `PermissionRequest` | Before tool execution (most restrictive) |
| `AfterTool` | `AfterTool`, `ToolError` | After tool execution |
| `McpResponse` | `AfterModel` | MCP server response handling |
| `Completion` | `TurnComplete` | Output completion validation |
| `SubagentStart` | `SubagentStart` | Subagent spawning |
| `SubagentStop` | `SubagentStop` | Subagent termination |
| `Runtime` | all others | General runtime events |

## Entry Points

| Method | Use Case |
|---|---|
| `evaluate_structured(&ProtectRequest)` | Full evaluation from a pre-built request |
| `evaluate_event_structured(provider, event, meta, ctx, adapter)` | Convenience: observe + evaluate in one call |
| `redact_mcp_text()` / `redact_mcp_json()` | Legacy standalone redaction (deprecated) |
| `explain_last()` | Human-readable explanation of most recent evaluation |
| `snapshot_records()` / `export_state()` / `export_records_jsonl()` | Audit and telemetry |

## Construction

| Constructor | Use Case |
|---|---|
| `ProtectService::new(engine, config)` | Default provider profiles |
| `ProtectService::with_profiles(engine, config, profiles)` | Custom capability map |
| `ProtectService::with_capabilities(engine, config, provider, caps)` | Single-provider override |
