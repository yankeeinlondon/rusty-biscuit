# Protect Service Implementation Checklist

This checklist turns the Protect design into scoped, issue-sized work items grouped by package/module.

## claudine/lib

### services/protect.rs

- [ ] **PROTECT-001: Rule Engine (Phase 1)**
  - Implement real rule matching for command patterns, protected paths, and secret patterns.
  - Replace current risk-driven placeholder logic with policy-driven evaluation.
  - Add deterministic precedence (`deny` > `ask` > `allow`) and explicit conflict handling.
- [ ] **PROTECT-002: Provider Override Resolution**
  - Implement deep merge of global `ProtectConfig` with `ProviderProtectOverride`.
  - Add explicit behavior for `enabled: false` override and posture inheritance.
- [ ] **PROTECT-003: Completion Retry State**
  - Track per-session completion retry counters.
  - Enforce `completion.max_retries` with loop-protection outcome.
- [ ] **PROTECT-004: MCP Redaction Utilities**
  - Add redaction helper API for MCP payload text/JSON.
  - Support configured redact patterns and instruction-payload stripping.
- [ ] **PROTECT-005: Audit Record Export**
  - Add helper methods to snapshot/export `ProtectState` records for logs/reports.

### events/config.rs

- [ ] **PROTECT-006: Config Validation**
  - Add semantic validation for `settings.protect` (e.g., non-zero retries, sane max history).
  - Return `ConfigValidation` errors for invalid protect settings.

### dispatch/loader.rs

- [ ] **PROTECT-007: Protect Config Merge Hardening**
  - Enforce repo override rules that cannot silently weaken strict user posture unless explicitly allowed.
  - Add tests for protect-specific merge precedence.

### dispatch/mod.rs

- [ ] **PROTECT-008: Protect Pre-Action Integration**
  - Evaluate `ProtectService` before running configured actions for relevant events.
  - Map `ProtectOutcome` to dispatch behavior (continue/skip/block/exit).
- [ ] **PROTECT-009: Protect Post-Action Integration**
  - Feed `AfterTool`/completion event context back into Protect for corrective decisions.

### adapters/*

- [ ] **PROTECT-010: Capability Handshake**
  - Expose adapter-level protect capability metadata (gate strength, mutability, subagent visibility).
  - Ensure profile mapping stays aligned with adapter behavior.
- [ ] **PROTECT-011: Outcome Mapping**
  - Map normalized protect outcomes to provider-native payloads and exit codes.
  - Add provider-specific downgrade reasons when outcome cannot be enforced.

### actions/*

- [ ] **PROTECT-012: Protect-Aware Action Execution**
  - Add optional protect context to `Call` action responses.
  - Enable protect-driven short-circuit before expensive/unsafe calls.

### error.rs

- [ ] **PROTECT-013: Error Surface Expansion**
  - Add protect-specific error variants for rule parsing, invalid policy, and enforcement mapping.

### tests (lib)

- [ ] **PROTECT-014: Unit Test Coverage**
  - Add exhaustive tests for risk/posture/mode matrix.
  - Add tests for capability downgrades and YOLO behavior.
- [ ] **PROTECT-015: Integration Fixture Coverage**
  - Add per-provider fixtures validating outcome mapping to adapter responses.
  - Include subagent and MCP-focused fixtures.

## claudine/cli

### commands/init/*

- [ ] **PROTECT-016: Init Wizard Protect Defaults**
  - Add optional prompts for enabling Protect and selecting posture (`advisory`, `balanced`, `strict`).
  - Seed provider-aware defaults into generated config.

### commands/hooks.rs / commands/about.rs

- [ ] **PROTECT-017: Visibility Commands**
  - Add command output describing active protect posture and provider capability downgrades.
  - Show whether runtime is in Normal or YOLO assumptions.

### commands/dry_run.rs / commands/handle.rs

- [ ] **PROTECT-018: Dry-Run Evaluation Output**
  - Include protect decision preview in dry-run output.
  - Add structured output mode for CI parsing.

### tests (cli)

- [ ] **PROTECT-019: CLI Integration Tests**
  - Add tests verifying protect defaults in `init --quick`.
  - Add tests for protect decision visibility in `dry-run`/`handle`.

## claudine/docs

### protect-service.md

- [ ] **PROTECT-020: Keep Design and Code in Sync**
  - Update design when enum/struct names or outcome semantics change.
  - Maintain provider detail notes when capability research updates.

### protect-service-implementation-checklist.md

- [ ] **PROTECT-021: Track Delivery Status**
  - Mark items complete as merged.
  - Link each checklist item to PRs/issues.

## Suggested Delivery Order

- [ ] **M1:** PROTECT-001, PROTECT-002, PROTECT-006, PROTECT-007, PROTECT-014
- [ ] **M2:** PROTECT-008, PROTECT-009, PROTECT-010, PROTECT-011, PROTECT-015
- [ ] **M3:** PROTECT-016, PROTECT-017, PROTECT-018, PROTECT-019
- [ ] **M4:** PROTECT-003, PROTECT-004, PROTECT-005, PROTECT-013, PROTECT-020, PROTECT-021
