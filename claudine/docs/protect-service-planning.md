## Provider Details

### Claude Code
Source: [claude-code.md](./protect/claude-code.md)

- Strongest native enforcement surface in the set: blocking pre-tool hook, blocking completion hooks, blocking permission hook, and MCP response interception (`PostToolUse`) with modify/stop support.
- Hooks fire in subagents, so policy parity between root and subagent execution is realistic.
- Supports sandboxing, but also has bypass mode; Protect must detect bypass posture and harden policy behavior accordingly.
- Design implication: Claude can run full Protect policy (preventive + corrective + completion gate) with minimal degradation.

### Codex CLI
Source: [codex.md](./protect/codex.md)

- No blocking pre-tool hook in current CLI hook model; available signals are mostly observational (`notify`, JSONL stream, OTEL).
- Completion signals exist but are non-blocking; no MCP response interception event.
- Sandbox exists and subagent permissions are configurable, but enforcement is mostly via static config and permission model, not runtime hook veto.
- Design implication: Protect must run in "advisory/monitor" mode by default for Codex hooks, with optional external orchestrator kill behavior for high-risk detections.

### Gemini CLI
Source: [gemini-cli.md](./protect/gemini-cli.md)

- Strong blocking hooks (`BeforeTool`, `BeforeAgent`) plus post-tool interception (`AfterTool`) that can sanitize or deny output.
- Completion gating is possible (`AfterAgent`) with loop-protection expectations.
- Subagent behavior coverage is less explicit; Protect should assume partial visibility and enforce conservative defaults for delegated work.
- Sandbox and bypass/YOLO modes are both available.
- Design implication: near full Protect enforcement, with explicit safeguards around subagent uncertainty.

### Goose
Source: [goose.md](./protect/goose.md)

- Hook surface is largely non-blocking (`GOOSE_STATUS_HOOK`, stream-json events).
- MCP supported but no response interception gate.
- Completion events are observable but not blockable.
- Design implication: Protect is primarily detective/alerting in first phase; preventive controls must come from permission profiles, extension policy, and optional wrapper behavior.

### Kimi Code
Source: [kimi-code.md](./protect/kimi-code.md)

- Blocking control exists mainly through wire/approval pathways; not all tool activity is uniformly interceptable.
- User prompt event exists but is non-blocking and non-mutating.
- Subagent events are visible; hook parity across subagent execution is not fully guaranteed.
- No sandbox; bypass mode exists.
- Design implication: Protect must distinguish approval-gated paths (enforceable) from non-gated paths (monitor/escalate), and treat root/no-sandbox posture as high-risk.

### OpenCode
Source: [opencode.md](./protect/opencode.md)

- Blocking pre-tool control exists (`stop`, `ask-stop`), and prompt mutation is supported (`chat.message`).
- MCP response can be modified (`tool.execute.after`) but not hard-stopped at that stage.
- Completion events are present but non-blocking.
- No built-in sandbox and no global bypass mode.
- Design implication: strong preventive controls at tool boundary, corrective controls post-tool, but completion enforcement must be advisory.

### Qwen CLI
Source: [qwen-cli.md](./protect/qwen-cli.md)

- Pre-tool blocking exists via SDK callback pattern, not as a standard CLI hook surface.
- No user prompt event and no completion event hook in the CLI model.
- No MCP response interception event.
- Sandbox and bypass/YOLO mode available.
- Design implication: first implementation should support two levels: CLI mode (monitor-heavy) and SDK-integrated mode (stronger tool-call enforcement).

### Roo Code
Source: [roo-code.md](./protect/roo-code.md)

- Strong pre-tool gating and completion blocking are available; subagent events and subagent hook coverage are comparatively strong.
- MCP is supported but not interceptable pre-consumption.
- No sandbox; bypass mode exists.
- Design implication: robust policy enforcement is possible for tool/complete/subagent workflows, but MCP and privilege boundaries need compensating controls.

## Protect Service

### Design Goals

- Provide one policy model that feels consistent across providers, while transparently degrading where provider control surfaces are weaker.
- Keep engagement trust-empowered: block only high-confidence harmful actions; use ask/advise for ambiguous cases.
- Centralize policy definition and enforcement in a single library service (`services::ProtectService`).
- Support two runtime modes:
  - Normal mode: balanced prevention + guidance.
  - YOLO mode: reduced prevention due to provider bypass posture, increased telemetry and post-hoc checks.

### Rust Module Shape

- New module path: `claudine/lib/src/services/protect.rs` (exported through `services/mod.rs`).
- Central actor:

```rust
pub struct ProtectService {
    config: ProtectConfig,
    profiles: ProviderProtectProfiles,
    state: ProtectState,
}
```

- `ProtectService` responsibilities:
  - evaluate event/tool/prompt/completion contexts against policy,
  - return normalized protection outcomes,
  - emit structured audit records,
  - provide degradation reasons when hard enforcement is unavailable.

### Configuration Model

Add `protect` to both user and repo configuration entry points.

```rust
pub struct ProtectConfig {
    pub enabled: bool,
    pub posture: ProtectPosture,          // advisory | balanced | strict
    pub allow_repo_posture_downgrade: bool,
    pub yolo: YoloPolicy,                 // behavior overrides when provider is in bypass mode
    pub rules: ProtectRules,
    pub completion: CompletionPolicy,
    pub mcp: McpPolicy,
    pub subagents: SubagentPolicy,
    pub privilege: PrivilegePolicy,
    pub providers: HashMap<Provider, ProviderProtectOverride>,
    pub max_recent_decisions: u16,
}
```

- Merge behavior:
  - user config = baseline,
  - repo config overlays user config,
  - repo may tighten but should not silently weaken strict user settings unless explicitly allowed via `allow_repo_posture_downgrade`.
  - provider overrides are deep-merged through partial override structs (`*Override`) rather than full replacement.

### Capability-Driven Enforcement (Core Idea)

Protect should not branch directly on provider name in business logic. Instead, adapters expose a capability profile used by policy evaluation.

```rust
pub struct ProviderProtectCapabilities {
    pub pre_tool_gate: GateCapability,
    pub user_prompt_gate: GateCapability,
    pub mcp_response_gate: GateCapability,
    pub completion_gate: GateCapability,
    pub subagent_visibility: VisibilityLevel,
    pub subagent_policy_control: bool,
    pub sandbox_available: bool,
    pub bypass_mode_available: bool,
}
```

This allows:
- one policy engine,
- per-provider degradation messages,
- future wrapper mode to upgrade capability flags without redesigning policy logic.

### Normalized Outcomes

Protect evaluation returns one of:

- `Allow`.
- `AskThenAllowOrStop`.
- `StopCurrent` (block call, continue session).
- `StopSession` (terminate run).
- `AllowWithRedaction` (sanitize content before model consumption).
- `AdvisoryOnly` (no native blocking path; emit finding + recommendation).

Provider adapters map these outcomes to native hook responses (or best-effort observability when no response channel exists).

### Current Implementation Notes (2026-02-21)

- `ProtectService` now includes:
  - deep provider override resolution,
  - per-session completion retry loop protection,
  - MCP text/JSON redaction helpers,
  - audit snapshot/JSONL export APIs.
- Dispatch now evaluates Protect both pre-action and post-action.
- Adapters expose a protect capability handshake and protect-outcome mapping path.
- CLI supports protect-aware defaults in `init --quick`, interactive protect posture prompts, and structured `--json` outputs in `dry-run`/`handle`.

### Policy Domains

#### 1. Tool-Call Protection

- Match tool name + arguments + resolved filesystem/network intent.
- Enforce command/path policies (destructive shell patterns, protected paths, credential files, risky network destinations).
- Prefer `ask` for medium-confidence risky operations; `stop` for high-confidence destructive patterns.

#### 2. Prompt Protection

- Scan user prompts for direct secret disclosure requests, exfiltration attempts, or instruction patterns that attempt policy bypass.
- If mutation is supported, inject constrained safety context. If only blocking is supported, fail with explicit reason.

#### 3. MCP Protection

- Server trust policy: allowlist/denylist by server id + transport class.
- Response controls:
  - if modifiable: redact secrets and strip instruction-like payloads,
  - if stoppable: block on high-confidence malicious payload signatures,
  - otherwise: mark advisory and attach audit finding.

#### 4. Completion Gates

- Run configured verifications (tests/lints/secret scans) on completion events where block is supported.
- Include loop guard tokens/state (`max_retries`, idempotency marker) to prevent infinite stop loops.
- Where blocking is unavailable, run checks asynchronously and emit remediation findings.

#### 5. Subagent Security

- Treat subagent creation as a first-class security event.
- Apply stricter default profile to subagents (`read-mostly`, reduced shell/network/MCP).
- If provider lacks subagent hook parity, mark session risk tier higher and tighten root-agent actions.

#### 6. Privilege & Runtime Guardrails

- Detect elevated runtime posture (`uid=0`, known bypass/YOLO flags, missing sandbox).
- Escalate policy automatically in high-risk posture:
  - disallow destructive shell/file operations by default,
  - require explicit approval for network writes and broad file writes,
  - add high-visibility alerts to audit log.

### Normal vs YOLO Behavior

- `Normal`:
  - preventive policy where supported,
  - ask-before-risk for ambiguous actions,
  - completion gates enabled.
- `YOLO`:
  - assume reduced provider guarantees,
  - block only explicit critical signatures when still technically possible,
  - switch most medium-risk policies to advisory + telemetry,
  - preserve forensic trail (event stream + decisions + reasons).

### Integration with Existing Claudine Architecture

- `events`: Protect evaluates normalized event payloads.
- `adapters`: map provider-native payloads to `ProtectInput` and `ProtectOutcome`.
- `dispatch`: invoke Protect before action execution for relevant events.
- `actions`: Protect outcomes may trigger existing action types (block/ask/report/shell).
- `config`: add `protect` node to user/repo schemas and merge pipeline.

### Phase 1 vs Future Wrapper

Phase 1 (this design target):
- Hook-native enforcement only.
- Capability-aware graceful degradation.
- No process wrapping requirement.

Future wrapper-compatible seams included now:
- `ProtectRuntimeContext` abstraction for env/runtime/sandbox details.
- External preflight input channel for sanitized env and launch policy.
- Adapter capability upgrades when wrapper supplies stronger interception (pre-exec command filtering, env scrubbing, MCP I/O mediation).

### Recommended Rollout

1. Implement data model + capability profiles + dry-run evaluator.
2. Enable hard enforcement for providers with strong gates (Claude, Gemini, OpenCode, Roo; Kimi/Qwen where runtime mode supports it).
3. Enable advisory/monitor mode for low-control providers (Codex, Goose, Qwen CLI-only paths).
4. Add completion gate execution with loop protection.
5. Add policy packs (`balanced`, `strict`) and provider-specific defaults.

### Test Strategy

- Unit tests:
  - rule evaluation (tool/prompt/MCP/completion/subagent/privilege),
  - degradation mapping from outcomes to capabilities,
  - merge semantics of user+repo `protect` config.
- Integration tests:
  - adapter fixtures per provider for representative events,
  - verify produced decisions match capability constraints,
  - verify YOLO posture changes policy behavior.
- Safety regression suite:
  - known destructive commands,
  - secret leakage patterns,
  - malicious MCP payload samples,
  - completion-loop guard behavior.

Implementation checklist: [protect-service-implementation-checklist.md](./protect-service-implementation-checklist.md)
