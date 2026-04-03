# Protect Refactor: Two Solutions

## Design Goals (shared by both solutions)

1. **Simple to understand** — a user should be able to look at their Protect config and immediately know what it does
2. **Encourage YOLO mode** — Protect behaves identically in YOLO and non-YOLO; the safety net it provides should make users _more_ comfortable enabling YOLO
3. **No postures** — no Advisory/Balanced/Strict; just a flat list of protections that are on or off
4. **PolicyEngine as sync tool** — Protect uses PolicyEngine to audit cross-provider consistency, not as a runtime gatekeeper
5. **Recommend, don't block** — Protect's default stance is to inform and recommend, not to prevent work
6. **Value without friction** — every protection should have an obvious "this saved me from something bad" moment

---

## Solution A: "Guard Rails"

### Concept

Protect is a flat list of named **guards**. Each guard is a self-contained protection with a clear name, a one-line description, and an on/off toggle. Guards fall into two categories:

1. **Scan Guards** — pattern-based checks that run during agent execution (e.g., secret detection in output, dangerous command patterns)
2. **Sync Guards** — checks that run at startup or on-demand to audit cross-provider policy consistency

There is no evaluation pipeline, no severity matrix, no posture system. A guard is either enabled or disabled. When a scan guard matches, it emits a warning to the user (never blocks silently). The user sees exactly what triggered and why.

### Configuration

```json
{
  "protect": {
    "guards": {
      "secret_scan": true,
      "credential_patterns": true,
      "destructive_commands": true,
      "broad_fs_writes": true,
      "policy_sync": true,
      "skill_path_access": true,
      "file_ref_access": true
    }
  }
}
```

Or simply:

```json
{
  "protect": true
}
```

When `true`, all default guards are enabled. When `false`, Protect is disabled entirely. When an object, each guard is individually toggled.

### Guard Catalog

#### Scan Guards (runtime)

| Guard | Default | What it does |
|---|---|---|
| `secret_scan` | on | Scans agent output for secret patterns (API keys, tokens, passwords). Blocks the action and redacts the matched content. |
| `credential_patterns` | on | Detects credential-like strings in tool arguments (e.g., `Authorization: Bearer ...` being passed to curl). Blocks execution. |
| `destructive_commands` | on | Matches dangerous shell patterns (`rm -rf /`, `DROP TABLE`, `git push --force`). Blocks execution. |
| `broad_fs_writes` | off | Blocks when an agent writes outside the current project directory (e.g., to `/etc/`, `~/.ssh/`, dotfiles). |
| `instruction_injection` | on | Detects prompt injection patterns in MCP server responses. Blocks the response from reaching the agent. |

Each scan guard has a built-in set of patterns. Users who want custom patterns can extend them:

```json
{
  "protect": {
    "guards": {
      "secret_scan": {
        "enabled": true,
        "extra_patterns": ["CUSTOM_SECRET_[A-Z0-9]+"]
      }
    }
  }
}
```

#### Sync Guards (startup / on-demand)

| Guard | Default | What it does |
|---|---|---|
| `policy_sync` | on | At startup, queries PolicyEngine for each installed provider's configured policy. Reports inconsistencies (e.g., Claude allows `/etc` writes but Codex denies them). Suggests fixes. |
| `skill_path_access` | on | Checks that all installed agents have read access to `~/.claude/skills/`, `~/.claudine/skills/`, and any other agent skill directories. Recommends adding read permissions if missing. |
| `file_ref_access` | on | During `compose` / `inline-compose`, scans the document for `@`-prefixed file references and checks whether the active provider has read permission for each path. Prompts the user before launch if any are missing. |

### Runtime Behavior

When a scan guard fires:

1. The action is blocked before it can execute or propagate
2. The guard name and matched pattern are displayed to the user
3. For output-side guards (`secret_scan`), the matched content is redacted before display
4. Example: `[protect:secret_scan] Blocked — API key detected in agent output (redacted)`
5. Example: `[protect:destructive_commands] Blocked — matched "rm -rf /" in shell command`

When a sync guard finds issues at startup:

1. A summary section is printed after initialization
2. Each inconsistency is listed with a recommended fix
3. Example:
   ```
   [protect:policy_sync] Cross-provider policy inconsistencies found:
     - Claude allows writes to /etc/* but Codex denies them
       Fix: Add deny rule to Claude's .claude/settings.json
     - Gemini has no MCP server restrictions configured
       Fix: Consider adding an MCP allowlist
   ```

### Architecture

```
ProtectService
├── scan_guards: Vec<ScanGuard>       // runtime pattern matchers
├── sync_guards: Vec<SyncGuard>       // startup/on-demand auditors
├── engine: Arc<PolicyEngine>         // used by sync guards only
└── config: ProtectConfig             // flat guard toggles

ScanGuard
├── name: &'static str
├── patterns: Vec<Regex>              // built-in + user extras
└── fn check(context: &ScanContext) -> Option<Block>

SyncGuard
├── name: &'static str
└── fn audit(engine: &PolicyEngine, ctx: &PolicyContext) -> Vec<Finding>
```

Key simplification: `ProtectService` no longer has an evaluation pipeline. It is a collection of guards that block matched actions. No postures, no severity matrix, no capability downgrade.

### Strengths

- **Radically simple mental model** — guards are named pattern matchers with on/off switches; if it matches, it blocks
- **Zero-friction default** — `"protect": true` gives sensible defaults that only block genuinely dangerous things
- **Easy to explain** — "Protect blocks secrets from leaking, dangerous commands from running, and audits cross-provider consistency"
- **Each guard is independently testable** — no interaction between guards, no matrix to reason about
- **Encourages YOLO** — guards block the same things in YOLO and non-YOLO; users know the sharp edges are covered regardless of mode
- **PolicyEngine stays clean** — PolicyEngine is only consulted for sync/audit, not woven into runtime decisions

### Limitations

- **Binary block/allow** — no middle ground like "ask the user." A guard either blocks or doesn't. Users who want interactive confirmation for edge cases would need to rely on the provider's own permission system.
- **False positives block work** — since guards block rather than warn, a false positive regex match stops the agent. Users need to be able to quickly disable the offending guard or add an exception. The `extra_patterns` mechanism helps but a per-guard allowlist/exception list may also be needed.
- **Sync guards are startup-only by default** — if a user changes provider config mid-session, the sync report is stale until next startup (though an on-demand `claudine protect sync` command could address this).

---

## Solution B: "Sync & Scan"

### Concept

Protect is split into two distinct, independently useful services:

1. **Policy Sync** — a cross-provider consistency engine that runs at startup (and on-demand). It uses PolicyEngine to compare rules across all installed providers, surfaces inconsistencies, and can optionally _apply_ recommended fixes with user confirmation.
2. **Output Scanner** — a lightweight runtime scanner that checks agent output and tool arguments against configurable patterns. When it finds something concerning, it annotates the output with a warning. For a small set of critical patterns (configurable), it can optionally **redact** content before it reaches the user or downstream tools.

The key difference from Solution A: the sync service is a first-class, interactive feature (not just a startup report), and the scanner supports redaction as a distinct action alongside warning.

### Configuration

```json
{
  "protect": {
    "sync": {
      "on_startup": true,
      "auto_fix": false,
      "recommend_skill_paths": true,
      "scan_file_refs": true
    },
    "scanner": {
      "secret_patterns": true,
      "credential_patterns": true,
      "destructive_commands": true,
      "injection_detection": true,
      "redact_secrets_in_output": false,
      "custom_patterns": []
    }
  }
}
```

Or simply:

```json
{
  "protect": true
}
```

When `true`, both sync and scanner are enabled with defaults. The `sync` and `scanner` sections are independently configurable.

### Policy Sync Service

The sync service answers: "Are my agents configured consistently?"

#### What it checks

| Check | Description |
|---|---|
| **Path rule consistency** | Do all providers agree on which paths are readable/writable? Surfaces cases where one provider allows a write that another denies. |
| **Command rule consistency** | Do all providers agree on which commands are allowed? Flags divergence. |
| **MCP server consistency** | Are the same MCP servers trusted/blocked across providers? |
| **Skill path readability** | Are agent skill directories (`~/.claude/skills/`, etc.) readable by all providers? |
| **File reference access** | When composing a document, are all `@`-referenced files readable by the target provider? |

#### How it works

```
Startup (or `claudine protect sync`):
  1. Enumerate installed providers
  2. For each provider, get ConfiguredPolicySnapshot from PolicyEngine
  3. Compare canonical rules across providers
  4. Produce SyncReport with findings and recommendations
  5. Display findings to user
  6. If auto_fix is enabled: generate PolicyMutationPlans and apply with confirmation
```

#### Interactive sync during composition

When `scan_file_refs` is enabled and the user runs `compose` or `inline-compose`:

1. Parse the document for `@`-prefixed file references
2. For the target provider, check read access for each referenced path
3. If any paths lack permission, prompt the user:
   ```
   The following files are referenced but your agent may not have read access:
     - @claudine/docs/topics/protect-service.md
     - @~/.claude/skills/rust/SKILL.md

   Grant read access for this session? [Y/n]
   ```
4. If accepted, apply a session-scoped policy override via PolicyEngine

### Output Scanner

The scanner is a simple pattern-matching layer. It has three possible actions per match:

| Action | Behavior |
|---|---|
| `warn` | Print a visible warning annotation in the output |
| `redact` | Replace the matched content with `[REDACTED]` |
| `ignore` | Pattern is recognized but no action taken (useful for suppressing false positives) |

Default actions per pattern category:

| Category | Default Action | What it matches |
|---|---|---|
| `secret_patterns` | warn | API keys, tokens, password-like strings |
| `credential_patterns` | warn | Auth headers, connection strings with credentials |
| `destructive_commands` | warn | `rm -rf`, `DROP TABLE`, `--force` pushes |
| `injection_detection` | warn | Prompt injection phrases in MCP responses |
| `redact_secrets_in_output` | redact (when enabled) | Same as `secret_patterns` but replaces content |

### Architecture

```
ProtectService
├── sync: PolicySyncService
│   ├── engine: Arc<PolicyEngine>
│   └── config: SyncConfig
├── scanner: OutputScanner
│   ├── patterns: Vec<ScanPattern>
│   └── config: ScannerConfig
└── config: ProtectConfig

PolicySyncService
├── fn sync_all(ctx: &PolicyContext) -> SyncReport
├── fn sync_provider(provider, ctx) -> ProviderSyncReport
├── fn check_file_refs(doc: &Document, provider, ctx) -> Vec<AccessFinding>
└── fn recommend_fixes(report: &SyncReport) -> Vec<PolicyMutationPlan>

OutputScanner
├── fn scan_output(text: &str) -> Vec<ScanMatch>
├── fn scan_tool_args(args: &Value) -> Vec<ScanMatch>
└── fn apply_redactions(text: &str, matches: &[ScanMatch]) -> String

SyncReport
├── provider_reports: Vec<ProviderSyncReport>
├── inconsistencies: Vec<Inconsistency>
├── recommendations: Vec<Recommendation>
└── fn summary() -> String
```

### Runtime Behavior

**At startup** (when `sync.on_startup` is true):
```
Claudine initialized.
[protect:sync] Checking cross-provider policy consistency...
  Claude, Codex, Gemini: 2 inconsistencies found
    1. Write access to ~/.ssh/ — Claude: deny, Codex: no rule (defaults to allow in sandbox)
       Recommendation: Add explicit deny in Codex config
    2. MCP server "filesystem" — Claude: allow, Gemini: no rule
       Recommendation: Add explicit allow in Gemini config
  Run `claudine protect sync --fix` to apply recommendations.
```

**During composition** (when `sync.scan_file_refs` is true):
```
Composing document with Claude...
[protect:file-refs] 3 file references found, checking access:
  ✓ @src/main.rs — allowed
  ✓ @claudine/docs/topics/protect-service.md — allowed
  ✗ @~/.claude/skills/rust/SKILL.md — no read rule

Grant read access for @~/.claude/skills/rust/SKILL.md? [Y/n]
```

**During agent execution** (scanner):
```
[protect:scanner] Possible API key in output: sk-...xxxx (secret_patterns)
```

### Strengths

- **Two clear concepts** — "sync checks consistency, scanner watches output" is easy to explain
- **Interactive sync is high-value** — the file reference scanning during composition directly prevents a common pain point (agent can't read files it was told about)
- **Redaction support** — unlike Solution A, users who want secrets scrubbed from output can enable it
- **PolicyEngine integration is focused** — PolicyEngine is used for what it's good at (cross-provider policy comparison and mutation), not shoe-horned into runtime decisions
- **Encourages YOLO** — scanner only warns by default; sync runs at startup and during composition, not during every tool call. Neither interferes with agent execution flow.
- **Extensible without complexity** — adding a new scan pattern is adding a regex; adding a new sync check is adding a comparison function. No matrices or pipelines.
- **`claudine protect sync` as a CLI command** — gives users an on-demand tool they can run anytime, which is tangible and understandable

### Limitations

- **Two concepts instead of one** — while simpler than the current 8-step pipeline, it's still two things (sync + scanner) rather than one flat list. Users need to understand the difference.
- **Sync requires PolicyEngine maturity** — the cross-provider comparison is only as good as the backends. Providers with `Partial` fidelity (like Goose) will have gaps in what can be compared.
- **Session-scoped overrides are ephemeral** — the file-ref access grants during composition only last for the session. If the user composes the same document again, they'll be prompted again (unless they permanently update their provider config).
- **Redaction changes content** — even though it's opt-in, redacting output means the user doesn't see the original. This could mask useful information in rare cases.

---

## Comparison

| Dimension | Solution A: Guard Rails | Solution B: Sync & Scan |
|---|---|---|
| **Mental model** | One concept: guards | Two concepts: sync + scanner |
| **Config complexity** | Single flat map of toggles | Two sections, but still simple |
| **Runtime overhead** | Minimal (pattern matching only) | Minimal (pattern matching only) |
| **PolicyEngine usage** | Startup audit only | Startup audit + composition-time access checks + on-demand CLI |
| **Blocking/redaction** | Block + redact on match | Warn + optional redact |
| **File ref scanning** | Startup check | Interactive prompt during composition |
| **Cross-provider sync** | Report at startup | Report + optional auto-fix + CLI command |
| **YOLO compatibility** | Identical behavior | Identical behavior |
| **Implementation size** | Smaller — guards are independent units | Medium — sync service needs cross-provider comparison logic |
| **Biggest strength** | Radical simplicity | High-value interactive features (file refs, sync --fix) |
| **Biggest risk** | False positives block work (needs easy escape hatch) | Two-concept model is slightly harder to explain |
