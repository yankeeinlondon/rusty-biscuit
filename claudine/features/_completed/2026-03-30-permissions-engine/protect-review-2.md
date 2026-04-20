# Protect Re-Review

Checks run:

- `cargo test -p claudine protect -- --nocapture`
- `cargo test -p claudine redaction -- --nocapture`

## Findings

### P1: Post-action redaction can override a stronger protect stop outcome — RESOLVED

Reordered post-action handling in `dispatch/mod.rs` to check
`should_short_circuit_on_protect()` **before** applying redaction.
Blocking outcomes now always take priority over redaction plans.

### P1: `AfterTool` capability downgrade is still over-permissive for providers with no post-action enforcement — RESOLVED

Added `post_tool_gate` field to `ProviderProtectCapabilities`. Each
provider profile now declares its actual AfterTool enforcement
capability (Guarantee for Claude/Gemini, None for all others).
`capability_for_phase` uses this field instead of hardcoding
`GateCapability::Influence`.

### P3: Deprecated Protect config fields are still accepted instead of being migration-blocked — RESOLVED

Replaced `deprecation_warnings()` with `validate_deprecated_fields()`
called from `validate()`. Deprecated fields (`blocked_command_patterns`,
`ask_command_patterns`, `protected_paths`, MCP `allowlist`/`denylist`)
now produce hard `ProtectInvalidPolicy` errors at load time. Five new
tests verify each field is rejected. The old regex validation for
removed fields was cleaned up.
