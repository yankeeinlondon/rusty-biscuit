# Protect Service

The Protect service is a policy-based security guard for agentic CLI systems. It evaluates commands, prompts, tool calls, and MCP responses against configurable rules to prevent destructive actions, credential leakage, and prompt injection attacks.

## Core Architecture

### ProtectService

The central actor in `claudine/lib/src/services/protect.rs`:

```rust
pub struct ProtectService {
    config: ProtectConfig,
    profiles: ProviderProtectProfiles,
    state: ProtectState,
}
```

**Responsibilities:**

- Evaluate event contexts against policy rules
- Return normalized protection outcomes
- Emit structured audit records
- Provide degradation reasons when hard enforcement is unavailable

### Evaluation Flow

1. Input is created from event metadata (`ProtectInput`)
2. Service resolves provider-specific policy overrides
3. Policy evaluation determines desired outcome
4. Capability-aware degradation adjusts for provider limitations
5. Decision is recorded in state for auditing
6. Result is returned to caller

## Configuration Model

### ProtectConfig

```rust
pub struct ProtectConfig {
    pub enabled: bool,
    pub posture: ProtectPosture,           // advisory | balanced | strict
    pub allow_repo_posture_downgrade: bool,
    pub yolo: YoloPolicy,
    pub rules: ProtectRules,
    pub completion: CompletionPolicy,
    pub mcp: McpPolicy,
    pub subagents: SubagentPolicy,
    pub privilege: PrivilegePolicy,
    pub providers: HashMap<Provider, ProviderProtectOverride>,
    pub max_recent_decisions: u16,
}
```

### Posture Levels

| Posture | Behavior |
|---------|----------|
| `advisory` | Never block in-line. Only collect findings and recommend next actions. |
| `balanced` | Ask for risky actions, block critical actions. (default) |
| `strict` | Prefer hard stops for any high-confidence dangerous behavior. |

### Runtime Modes

| Mode | Behavior |
|------|----------|
| `normal` | Full preventive policy, ask-before-risk for ambiguous actions, completion gates enabled |
| `yolo` | Reduced prevention due to provider bypass posture, most medium-risk becomes advisory, forensic trail preserved |

## Policy Domains

### 1. Command & Path Protection

The service matches tool arguments and resolved filesystem/network intent against user-configurable regex patterns:

#### Blocked Command Patterns
Commands that are always stopped (configurable via `blocked_command_patterns`).

#### Ask Command Patterns
Commands requiring user confirmation before execution (configurable via `ask_command_patterns`).

#### Protected Paths
Filesystem paths that require confirmation when accessed (configurable via `protected_paths`).

#### Built-in Risk Detection

The service automatically classifies commands by risk level:

**Critical Risk** (triggers stop):

- `rm -rf` - Destructive recursive delete
- `drop database` - Database destruction

**High Risk:**

- `chmod` - Permission changes
- `curl` - Network requests

**Medium Risk:**

- `write` - File writes
- `delete` - File deletions

### 2. Privilege & Runtime Guardrails

Detects elevated runtime posture and automatically escalates policy:

#### Root Detection

- Detects `uid=0` or `is_root` flag in event metadata
- When running as root without sandbox: denies destructive operations

#### Network Write Detection
Triggers "ask" confirmation for these patterns:

- `curl -x` (proxy)
- `curl -d` (POST data)
- `wget --post`
- `scp`
- `rsync`
- `git push`

#### Broad Filesystem Write Detection
Triggers "ask" confirmation for:

- `rm -rf /` - Root deletion
- `chmod -r` - Recursive permission changes
- `chown -r` - Recursive ownership changes
- `find / -delete` - System-wide deletion

### 3. MCP Response Protection

#### Server Trust Policy

- `allowlist` - Approved MCP servers (all others require ask)
- `denylist` - Blocked MCP servers

#### Response Content Controls

- **If modifiable**: Redact secrets and strip instruction-like payloads
- **If stoppable**: Block high-confidence malicious payloads
- **Otherwise**: Mark advisory and attach audit finding

#### Instruction Payload Blocking
Detects prompt injection patterns in MCP responses:

- `ignore previous instructions`
- `system prompt`
- `developer instructions`
- `do not reveal`
- `tool instructions`

### 4. Completion Gates

- Runs configured verifications on completion events
- Includes loop guard tokens (`max_retries`, idempotency marker) to prevent infinite stop loops
- Default max retries: 3

### 5. Subagent Security

- Treats subagent creation as first-class security event
- Applies stricter default profile (`read-mostly`, reduced shell/network/MCP)
- If provider lacks subagent hook parity, marks session risk tier higher

### 6. Secret Detection

The `secret_patterns` configuration identifies sensitive data for redaction:

- API keys
- Passwords
- Tokens
- Private keys

Redacted content is replaced with `[REDACTED]` in MCP responses.

## Normalized Outcomes

Protect evaluation returns one of:

| Outcome | Description |
|---------|-------------|
| `Allow` | Action permitted |
| `AskThenAllowOrStop` | User confirmation required; can be denied |
| `StopCurrent` | Block this call, continue session |
| `StopSession` | Terminate entire run |
| `AllowWithRedaction` | Sanitize content before model consumption |
| `AdvisoryOnly` | No native blocking path; emit finding + recommendation |

### Outcome Precedence

When multiple outcomes apply, the highest-priority outcome wins:

1. StopSession (6)
2. StopCurrent (5)
3. AskThenAllowOrStop (4)
4. AllowWithRedaction (3)
5. AdvisoryOnly (2)
6. Allow (1)

## Provider Capabilities

Protect is capability-aware: it computes a normalized decision first, then downgrades when a provider cannot enforce that decision natively.

### Capability Profile

```rust
pub struct ProviderProtectCapabilities {
    pub pre_tool_gate: GateCapability,        // None | Influence | Guarantee
    pub user_prompt_gate: GateCapability,
    pub mcp_response_gate: GateCapability,
    pub completion_gate: GateCapability,
    pub subagent_visibility: VisibilityLevel, // None | Partial | Full
    pub subagent_policy_control: bool,
    pub sandbox_available: bool,
    pub bypass_mode_available: bool,
}
```

### Provider Capability Matrix

| Provider | Pre-Tool | Prompt | MCP | Completion |
|----------|----------|--------|-----|-----------|
| Claude | Guarantee | Guarantee | Guarantee | Guarantee |
| Gemini | Guarantee | Guarantee | Guarantee | Guarantee |
| OpenCode | Guarantee | Influence | Influence | Influence |
| Roo | Guarantee | None | None | Guarantee |
| KimiCode | Guarantee | Influence | None | None |
| QwenCode | Influence | None | None | None |
| Codex | None | None | None | None |
| Goose | None | None | None | None |

### Degradation Behavior

When provider capabilities cannot fulfill the desired outcome:

- `StopCurrent` → `AdvisoryOnly` (if no stop_current capability)
- `StopSession` → `AdvisoryOnly` (if no stop_session capability)
- `AskThenAllowOrStop` → `AdvisoryOnly` (if no ask_user capability)
- `AllowWithRedaction` → `AdvisoryOnly` (if no modify capability)

## Configuration Examples

### Basic Configuration

```json
{
  "protect": {
    "enabled": true,
    "posture": "balanced",
    "rules": {
      "blocked_command_patterns": ["rm -rf"],
      "ask_command_patterns": ["chmod", "chown"],
      "protected_paths": ["/etc", "/var", "~/.ssh"],
      "secret_patterns": ["(?i)api[_-]?key", "sk-[0-9a-zA-Z]+"]
    }
  }
}
```

### Strict Configuration

```json
{
  "protect": {
    "enabled": true,
    "posture": "strict",
    "rules": {
      "blocked_command_patterns": ["rm -rf", "drop database", "chmod 777"],
      "ask_command_patterns": ["curl", "wget", "scp", "rsync"],
      "protected_paths": ["/", "/home", "/var", "/etc", "~/.ssh", "~/.aws"]
    },
    "privilege": {
      "deny_when_root_without_sandbox": true,
      "require_ask_for_network_writes": true,
      "require_ask_for_broad_fs_writes": true
    }
  }
}
```

### MCP-Only Configuration

```json
{
  "protect": {
    "enabled": true,
    "posture": "balanced",
    "mcp": {
      "allowlist": ["filesystem", "git"],
      "denylist": ["unsafe-plugin"],
      "redact_patterns": ["(?i)password", "sk-[0-9a-zA-Z]+"],
      "block_instruction_payloads": true
    }
  }
}
```

## State & Auditing

### ProtectState

In-memory rolling state retained for forensic inspection:

```rust
pub struct ProtectState {
    pub decision_count: u64,
    pub recent: VecDeque<ProtectDecisionRecord>,
    pub completion_retries_by_session: HashMap<String, u8>,
}
```

### Decision Records

```rust
pub struct ProtectDecisionRecord {
    pub provider: Provider,
    pub phase: ProtectPhase,
    pub mode: ProtectRuntimeMode,
    pub risk: RiskLevel,
    pub outcome: ProtectOutcome,
    pub degraded: bool,
    pub degraded_from: Option<ProtectOutcome>,
    pub reason: String,
    pub session_id: Option<String>,
    pub completion_retry_count: Option<u8>,
}
```

### Export APIs

- `snapshot_records()` - Returns vector of recent decision records
- `export_state()` - Returns full state with metadata
- `export_records_jsonl()` - Returns records as JSON Lines for log sinks

## Evaluation Phases

Protect evaluates at multiple points in the agent lifecycle:

| Phase | Description |
|-------|-------------|
| `BeforePrompt` | User prompt injection detection |
| `BeforeTool` | Pre-execution tool call evaluation |
| `AfterTool` | Post-execution result evaluation |
| `McpResponse` | MCP server response sanitization |
| `Completion` | Turn completion verification |
| `SubagentStart` | Subagent creation security check |
| `SubagentStop` | Subagent termination review |
| `Runtime` | General runtime evaluation |

## Integration Points

- **Events**: Protect evaluates normalized event payloads
- **Adapters**: Map provider-native payloads to `ProtectInput` and `ProtectOutcome`
- **Dispatch**: Invoke Protect before action execution
- **Actions**: Protect outcomes may trigger existing action types

## YOLO Mode Behavior

When provider is in bypass/YOLO mode:

- Medium-risk actions force to `AdvisoryOnly` (unless `allow_critical_blocking` is true)
- Critical blocking can be disabled
- Forensic trail collection is enhanced
- Most medium-risk policies switch to advisory + telemetry
