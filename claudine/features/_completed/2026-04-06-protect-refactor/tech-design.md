# Protect Refactor Technical Design

## Overview

This design replaces Claudine's current Protect runtime with a much smaller deny-catalog service. The new Protect has one job: inspect a small set of high-risk runtime surfaces and block actions that match curated dangerous patterns. It does not ask, warn, downgrade, or consult `PolicyEngine`.

The target outcome is:

- a flat config model
- a compile-time rule catalog
- deterministic `allow` or `block` results
- no posture system
- no severity matrix
- no capability downgrade logic
- no rolling protect state
- no `PolicyEngine` dependency

## Goals

- Block genuinely dangerous shell commands in tool calls.
- Block writes or edits to sensitive filesystem locations.
- Block prompt-injection patterns in MCP server responses.
- Keep YOLO and non-YOLO behavior identical.
- Stay non-interactive so long-running sessions continue without human intervention.
- Make configuration obvious enough that a user can audit it quickly.

## Non-Goals

- Replacing provider-native permission systems.
- Introducing a general allow/ask/deny policy layer.
- Modeling arbitrary intent through `PolicyQuery`.
- Auditing cross-provider consistency.
- Providing a rich forensic event store.

## Current-State Constraints

Today's implementation is tightly coupled to concepts the spec explicitly removes:

- `ProtectService` depends on `PolicyEngine`.
- Evaluations flow through an 8-step pipeline in `services/protect/evaluate.rs`.
- Decisions rely on `ProtectPosture`, `ProtectSeverity`, YOLO softening, and provider capability downgrade.
- `ProtectIntent` mirrors `PolicyQuery`.
- `ProtectState` stores rolling decision history.
- CLI init and config merging assume posture-based Protect.

The refactor is therefore structural, not incremental. It should not try to preserve the current internal model.

## Proposed Architecture

### Runtime Model

Protect becomes a standalone matcher service with three scan surfaces:

1. Bash tool commands
2. Write/Edit tool target paths
3. MCP server responses

Each evaluation returns either:

- `Allow`
- `Block`

When blocked, the result includes:

- rule group
- rule id or custom pattern name
- regex pattern
- matched text
- surface
- optional normalized target path
- config key to disable the group

### Service Boundary

Preferred API:

```rust
pub struct ProtectService {
    catalog: CompiledCatalog,
    config: ProtectConfig,
}

impl ProtectService {
    pub fn new(config: ProtectConfig, platform: ProtectPlatform) -> Result<Self>;
    pub fn evaluate(&self, request: &ProtectRequest) -> ProtectDecision;
}
```

`ProtectService` no longer receives `PolicyEngine`, provider capability profiles, or mutable state.

### Module Layout

Recommended new module structure:

- `catalog.rs`
  - built-in rule definitions
  - platform filtering
  - compile-time group metadata
- `config.rs`
  - new flat config schema
  - legacy-compat deserialization shim if retained
- `matcher.rs`
  - `RegexSet` compilation
  - second-pass individual regex confirmation
- `path.rs`
  - path normalization
  - sensitive prefix checks
  - `allow_paths` resolution
- `observe.rs`
  - simplified extraction of command/path/MCP payloads from event metadata
- `decision.rs`
  - `ProtectDecision`, `ProtectOutcome`, `ProtectMatch`
- `service.rs`
  - orchestration only
- `mod.rs`
  - public re-exports

Files expected to be removed entirely:

- `evaluate.rs`
- `downgrade.rs`
- `intent.rs`
- `redact.rs`
- `state.rs`

`explain.rs` should either be removed or reduced to a tiny formatter for blocked messages.

## Configuration Design

### User-Facing Shape

Shorthand:

```json
{
  "protect": true
}
```

Expanded form:

```json
{
  "protect": {
    "rules": {
      "filesystem_destruction": {
        "enabled": true,
        "allow_paths": ["node_modules", "target", "dist", "build", ".cache"]
      },
      "git_destructive": false,
      "sensitive_paths": true
    },
    "custom_patterns": [
      { "name": "no_prod_deploy", "pattern": "deploy.*production" }
    ]
  }
}
```

Recommended Rust shape:

```rust
pub struct ProtectConfig {
    pub enabled: bool,
    pub rules: ProtectRuleToggles,
    pub custom_patterns: Vec<CustomPattern>,
}
```

Where `ProtectRuleToggles` is a typed struct keyed by the built-in groups, not a free-form `HashMap<String, ...>`. That keeps JSON validation strict and completion-friendly.

### Rule Group Config

Each built-in group is either:

- `false`
- `true`
- `{ "enabled": bool, "allow_paths": [...] }`

Only groups that operate on filesystem targets should accept `allow_paths`. For groups where it has no meaning, config validation should reject it.

### Legacy Compatibility

Preferred approach: support old Protect config for one transition window, but deserialize it into the new model with explicit warnings or errors for removed fields.

Legacy fields to reject or translate:

- `posture`
- `allow_repo_posture_downgrade`
- `yolo`
- `completion`
- `mcp.redact_patterns`
- `providers`
- `rules.blocked_command_patterns`
- `rules.ask_command_patterns`
- `rules.protected_paths`

Alternative: hard cut with no compatibility shim.

Reasons to choose the alternative:

- the team wants the smallest possible implementation
- existing Protect users are few
- preserving old shapes risks keeping dead concepts alive in code and docs

## Catalog Design

### Built-In Groups

Use the spec's consolidated groups as the source of truth:

- `filesystem_destruction`
- `disk_manipulation`
- `remote_execution`
- `git_destructive`
- `system_sabotage`
- `network_sabotage`
- `container_cloud`
- `database_nukes`
- `obfuscated_execution`
- `prompt_injection`
- `credential_exfiltration`
- `sensitive_paths`

Each rule definition should include:

- `group`
- `rule_id`
- `surface`
- `pattern`
- `platforms`
- `supports_allow_paths`

### Compile-Time Asset

Preferred approach: define the catalog as Rust constants in `catalog.rs`.

Reasons:

- simplest startup path
- no runtime file loading
- compile failures catch invalid regexes early
- straightforward to review in code review

Alternative: maintain the catalog in JSON or TOML and embed it with `include_str!`.

Reasons to choose the alternative:

- the catalog grows substantially
- non-Rust contributors need to edit patterns often
- the team wants tooling to generate docs and tests from one data source

### Regex Compilation

Compile one `RegexSet` per rule group plus a parallel vector of concrete `Regex` values for final confirmation.

Evaluation flow inside a group:

1. `RegexSet::matches(input)` finds candidate rule indexes.
2. Matching candidate regexes run individually to capture the exact match text.
3. The first confirmed match blocks the request.

This preserves the performance win from `RegexSet` while still reporting the exact rule and matched substring.

### Platform Filtering

Platform selection happens once during service construction.

- macOS excludes Linux-only rules
- Linux excludes macOS-only rules
- cross-platform rules load everywhere

The platform should come from the wrapper's runtime environment, not from provider identity.

## Scan Surface Design

### Bash Commands

Protect inspects Bash command strings before execution.

Normalization:

- trim outer whitespace
- preserve original command for reporting
- optionally create a whitespace-normalized shadow string for matching

The canonical input to regexes should be the original command string. Pattern authors should use `\s+` and optional `sudo` prefixes as noted in the spec.

### Write/Edit Paths

Protect only scans paths for write-like tools, not reads.

Matching algorithm:

1. extract the tool path from event metadata
2. expand `~`
3. resolve relative paths against the event working directory
4. canonicalize existing ancestors when possible
5. lexically normalize `.` and `..`
6. compare against normalized sensitive prefixes

Preferred sensitive-path matcher: normalized prefix comparison, not regex.

Reasons:

- the prefixes are small and stable
- it avoids unnecessary regex complexity
- path-specific behavior such as `~/.ssh/` expansion is easier to reason about

Alternative: glob-based matching.

Reasons to choose the alternative:

- requirements expand beyond prefixes into broader path classes
- users want wildcard exceptions such as `**/generated/etc/**`

### MCP Responses

Protect scans only MCP-originated response payloads, using the `prompt_injection` group.

Preferred behavior:

- scan string payloads directly
- JSON payloads are serialized into stable text before scanning
- on match, block propagation to the model

Alternative: recursive JSON string-field scanning.

Reasons to choose the alternative:

- false positives from structural JSON serialization become a problem
- the team wants exact field locations in reports

## `allow_paths` Design

`allow_paths` exists to suppress false positives for destructive commands aimed at well-known disposable directories such as `node_modules` and `target`.

Preferred implementation:

- support `allow_paths` only for `filesystem_destruction`
- tokenize shell commands into words
- extract candidate path operands for a small set of supported commands (`rm`, `find`, `shred`, `chmod`, `chown`, `chattr`)
- compare normalized operands against the configured allowlist

Important behavior:

- relative entries such as `target` match relative path operands and descendants
- absolute allowlist entries match normalized absolute targets
- if no target path can be extracted, the allowlist does not apply

Alternative: generic substring matching against the raw command.

Reasons to choose the alternative:

- implementation time must stay minimal
- the project accepts a slightly weaker false-positive story initially

This alternative should be avoided if possible because it is easier to bypass and harder to explain.

## Custom Pattern Design

`custom_patterns` are compiled into a dedicated `custom` group that behaves like built-ins: first match blocks.

Recommended fields:

```rust
pub struct CustomPattern {
    pub name: String,
    pub pattern: String,
}
```

Custom patterns should apply to Bash command scanning by default. If future requirements need MCP or path-specific custom rules, extend the schema with an optional `surface` field rather than overloading the regex itself.

Alternative: allow per-pattern `surface` immediately.

Reasons to choose the alternative:

- teams already know they need custom MCP injection or path rules
- avoiding a follow-up schema change matters more than keeping v1 simple

## Decision and Reporting

### Decision Shape

Preferred outcome model:

```rust
pub enum ProtectOutcome {
    Allow,
    Block,
}
```

with:

```rust
pub struct ProtectDecision {
    pub outcome: ProtectOutcome,
    pub blocked: Option<ProtectMatch>,
}
```

This matches the spec more directly than reusing `StopCurrent`.

Alternative: keep `StopCurrent` as the blocking outcome internally.

Reasons to choose the alternative:

- it reduces dispatch integration churn
- existing provider-response mapping already understands it

### User Output

Blocked output should include:

- group
- rule id or custom pattern name
- regex pattern
- matched text
- config key to disable the group

Example:

```text
[protect] BLOCKED
  Group: filesystem_destruction
  Rule: rm_root_glob
  Pattern: (sudo\s+)?rm\s+-rf\s+/\*?
  Match: rm -rf /var/*

  Disable group:
    protect.rules.filesystem_destruction = false
```

For `credential_exfiltration`, consider masking the matched text in human-visible output if it appears to contain secret material.

Alternative: always show raw matched text.

Reasons to choose the alternative:

- operators need maximum debugging fidelity
- output is already confined to trusted local terminals

## Integration Plan

### Library

Update `claudine/lib/src/services/protect/*` to the new model and reduce re-exports in:

- `claudine/lib/src/services/protect/mod.rs`
- `claudine/lib/src/services/mod.rs`

Remove public types that no longer exist:

- `ProtectPosture`
- `ProtectSeverity`
- `ProtectIntent`
- `ProtectPolicyMode`
- capability profile types
- redaction plan types
- state export types

### Dispatch

`dispatch` should continue to evaluate Protect before forwarding a blocking response to providers, but the mapping logic becomes much smaller:

- no policy snapshot resolution
- no downgrade
- no completion retry logic
- no pre/post advisory distinction beyond whether a phase produced a decision

### Adapters and Observation

Provider adapters should only extract the data required for the three supported scan surfaces. Everything else currently emitted for Protect can be removed.

### CLI Init

Replace posture-driven init prompts with a binary enable/disable prompt.

Quick mode should write:

```json
{
  "settings": {
    "protect": true
  }
}
```

If the implementation wants slightly more explicit defaults, it can serialize the expanded object, but the shorthand should remain accepted.

### Hooks and Status Views

`claudine hooks` and any related status rendering should stop talking about posture, downgrade assumptions, or provider capability matrices. Protect should be described as enabled/disabled plus any non-default per-group toggles.

## Testing Strategy

### Unit Tests

- config parsing for `true`, `false`, shorthand objects, and invalid `allow_paths`
- regex compilation and platform filtering
- command-group matching for representative patterns from each group
- path normalization across relative, absolute, and `~`-prefixed paths
- `allow_paths` suppression for `rm -rf node_modules` and `rm -rf target`
- MCP prompt-injection detection on text and JSON payloads
- custom pattern compilation and blocking

### Integration Tests

- `claudine init --quick` writes the new Protect config
- a blocked Bash command appears in `protect_pre`
- a write to `~/.ssh/config` is blocked
- an allowed write inside the repo is not blocked
- an MCP payload containing injection text is blocked before model propagation

### Regression Tests

Add explicit tests proving these removed behaviors no longer exist:

- no posture merge
- no YOLO softening
- no provider capability downgrade
- no PolicyEngine dependency

## Migration Plan

1. Replace config schema and parsing.
2. Land the standalone catalog and matcher.
3. Simplify observation extraction to supported surfaces only.
4. Rewire dispatch to `allow` or `block`.
5. Remove legacy types, tests, docs, and CLI posture prompts.
6. Update `claudine/docs/topics/protect-service.md` to match the new model.

## Risks and Alternatives

### Provider Enforcement Gaps

The spec removes capability downgrade, but some providers still lack equivalent blocking hooks on all surfaces.

Preferred approach:

- keep Protect itself provider-agnostic
- only invoke blocking Protect in execution paths where Claudine truly has an enforcement point

Alternative:

- keep a thin outer provider-support matrix outside Protect and return "Protect unavailable on this surface" telemetry

Reasons to choose the alternative:

- users need explicit visibility into where Protect is effective today
- unsupported providers would otherwise appear silently less protected

### Repeated Block Loops

The current completion retry logic is removed. A model may repeatedly attempt the same blocked action.

Preferred approach for v1: accept this and rely on the provider's normal blocked-tool feedback loop.

Alternative: add a minimal session-local duplicate block counter later.

Reasons to choose the alternative:

- real-world runs show retry thrashing
- blocked actions create excessive noise in long sessions

### Catalog Maintenance

A regex catalog will drift if it is not reviewed as an explicit asset.

Recommended process:

- keep the runtime catalog in code
- keep `regexp.md` as human-facing design documentation
- add tests that bind representative sample commands to the expected group

## Recommended Implementation Choices

These choices best match the spec while minimizing long-term complexity:

- standalone `ProtectService` with no `PolicyEngine`
- typed flat config with per-group toggles
- Rust-constant rule catalog compiled into `RegexSet`s
- normalized prefix matching for sensitive paths
- command-aware `allow_paths` extraction for filesystem-destruction rules
- `Allow` or `Block` outcomes only
- provider enforcement handled outside Protect, not via internal downgrade logic
