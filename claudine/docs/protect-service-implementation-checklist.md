# Protect Service Implementation Checklist

This checklist turns the Protect design into scoped, issue-sized work items grouped by package/module.

## claudine/lib

### services/protect.rs

- [x] **PROTECT-001: Rule Engine (Phase 1)**
  - Implement real rule matching for command patterns, protected paths, and secret patterns.
  - Replace current risk-driven placeholder logic with policy-driven evaluation.
  - Add deterministic precedence (`deny` > `ask` > `allow`) and explicit conflict handling.
  - Ref: `claudine/lib/src/services/protect.rs`
- [x] **PROTECT-002: Provider Override Resolution**
  - Implement deep merge of global `ProtectConfig` with `ProviderProtectOverride`.
  - Add explicit behavior for `enabled: false` override and posture inheritance.
  - Ref: `claudine/lib/src/services/protect.rs`
- [x] **PROTECT-003: Completion Retry State**
  - Track per-session completion retry counters.
  - Enforce `completion.max_retries` with loop-protection outcome.
  - Ref: `claudine/lib/src/services/protect.rs`
- [x] **PROTECT-004: MCP Redaction Utilities**
  - Add redaction helper API for MCP payload text/JSON.
  - Support configured redact patterns and instruction-payload stripping.
  - Ref: `claudine/lib/src/services/protect.rs`
- [x] **PROTECT-005: Audit Record Export**
  - Add helper methods to snapshot/export `ProtectState` records for logs/reports.
  - Ref: `claudine/lib/src/services/protect.rs`

### events/config.rs

- [x] **PROTECT-006: Config Validation**
  - Add semantic validation for `settings.protect` (e.g., non-zero retries, sane max history).
  - Return `ConfigValidation` errors for invalid protect settings.
  - Ref: `claudine/lib/src/events/config.rs`

### dispatch/loader.rs

- [x] **PROTECT-007: Protect Config Merge Hardening**
  - Enforce repo override rules that cannot silently weaken strict user posture unless explicitly allowed.
  - Add tests for protect-specific merge precedence.
  - Ref: `claudine/lib/src/dispatch/loader.rs`

### dispatch/mod.rs

- [x] **PROTECT-008: Protect Pre-Action Integration**
  - Evaluate `ProtectService` before running configured actions for relevant events.
  - Map `ProtectOutcome` to dispatch behavior (continue/skip/block/exit).
  - Ref: `claudine/lib/src/dispatch/mod.rs`
- [x] **PROTECT-009: Protect Post-Action Integration**
  - Feed `AfterTool`/completion event context back into Protect for corrective decisions.
  - Ref: `claudine/lib/src/dispatch/mod.rs`

### adapters/*

- [x] **PROTECT-010: Capability Handshake**
  - Expose adapter-level protect capability metadata (gate strength, mutability, subagent visibility).
  - Ensure profile mapping stays aligned with adapter behavior.
  - Ref: `claudine/lib/src/adapters/mod.rs`
- [x] **PROTECT-011: Outcome Mapping**
  - Map normalized protect outcomes to provider-native payloads and exit codes.
  - Add provider-specific downgrade reasons when outcome cannot be enforced.
  - Ref: `claudine/lib/src/adapters/mod.rs`
  - Ref: `claudine/lib/src/adapters/codex.rs`
  - Ref: `claudine/lib/src/adapters/goose.rs`
  - Ref: `claudine/lib/src/adapters/qwen.rs`
  - Ref: `claudine/lib/src/adapters/roo.rs`

### actions/*

- [x] **PROTECT-012: Protect-Aware Action Execution**
  - Add optional protect context to `Call` action responses.
  - Enable protect-driven short-circuit before expensive/unsafe calls.
  - Ref: `claudine/lib/src/actions/hook_response.rs`
  - Ref: `claudine/lib/src/dispatch/runner.rs`

### error.rs

- [x] **PROTECT-013: Error Surface Expansion**
  - Add protect-specific error variants for rule parsing, invalid policy, and enforcement mapping.
  - Ref: `claudine/lib/src/error.rs`

### tests (lib)

- [x] **PROTECT-014: Unit Test Coverage**
  - Add exhaustive tests for risk/posture/mode matrix.
  - Add tests for capability downgrades and YOLO behavior.
  - Ref: `claudine/lib/src/services/protect.rs`
- [x] **PROTECT-015: Integration Fixture Coverage**
  - Add per-provider fixtures validating outcome mapping to adapter responses.
  - Include subagent and MCP-focused fixtures.
  - Ref: `claudine/lib/src/adapters/mod.rs`

## claudine/cli

### commands/init/*

- [x] **PROTECT-016: Init Wizard Protect Defaults**
  - Add optional prompts for enabling Protect and selecting posture (`advisory`, `balanced`, `strict`).
  - Seed provider-aware defaults into generated config.
  - Ref: `claudine/cli/src/commands/init/prompts.rs`
  - Ref: `claudine/cli/src/commands/init/mod.rs`

### commands/hooks.rs / commands/about.rs

- [x] **PROTECT-017: Visibility Commands**
  - Add command output describing active protect posture and provider capability downgrades.
  - Show whether runtime is in Normal or YOLO assumptions.
  - Ref: `claudine/cli/src/commands/hooks.rs`
  - Ref: `claudine/cli/src/commands/about.rs`

### commands/dry_run.rs / commands/handle.rs

- [x] **PROTECT-018: Dry-Run Evaluation Output**
  - Include protect decision preview in dry-run output.
  - Add structured output mode for CI parsing.
  - Ref: `claudine/cli/src/commands/dry_run.rs`
  - Ref: `claudine/cli/src/commands/handle.rs`

### tests (cli)

- [x] **PROTECT-019: CLI Integration Tests**
  - Add tests verifying protect defaults in `init --quick`.
  - Add tests for protect decision visibility in `dry-run`/`handle`.
  - Ref: `claudine/cli/tests/protect_cli.rs`

## claudine/docs

### protect-service.md

- [x] **PROTECT-020: Keep Design and Code in Sync**
  - Update design when enum/struct names or outcome semantics change.
  - Maintain provider detail notes when capability research updates.
  - Ref: `claudine/docs/protect-service.md`

### protect-service-implementation-checklist.md

- [x] **PROTECT-021: Track Delivery Status**
  - Mark items complete as merged.
  - Link each checklist item to PRs/issues.
  - Ref: `claudine/docs/protect-service-implementation-checklist.md`

## Suggested Delivery Order

- [x] **M1:** PROTECT-001, PROTECT-002, PROTECT-006, PROTECT-007, PROTECT-014
- [x] **M2:** PROTECT-008, PROTECT-009, PROTECT-010, PROTECT-011, PROTECT-015
- [x] **M3:** PROTECT-016, PROTECT-017, PROTECT-018, PROTECT-019
- [x] **M4:** PROTECT-003, PROTECT-004, PROTECT-005, PROTECT-013, PROTECT-020, PROTECT-021
