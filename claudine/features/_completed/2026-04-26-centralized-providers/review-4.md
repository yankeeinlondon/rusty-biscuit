---
agent: ""
model: ""
ready: false
---

# Feature Review: Centralized Providers

## Findings

### 1. `providers --describe --format json` still omits large parts of the catalog

**Severity:** High

The spec makes `ProviderInfo` the serializable data half of the central catalog and explicitly lists `capabilities` and `resource_support` as fields hanging off the registry. The current implementation instead stores these as skipped function pointers:

- `agent_capabilities_fn` and `resource_support_fn` are both `#[serde(skip)]`: `claudine/lib/src/provider/mod.rs:150`.
- `claudine providers --describe --format json` serializes `Vec<&ProviderInfo>` directly, so skipped fields are not present in the describe payload: `claudine/cli/src/commands/providers.rs:107`.

That means the new inspectable JSON surface does not round-trip "everything we know about a provider"; it omits the full legacy agent capability catalog and the resource portability catalog. This is a functional gap for the feature's primary promise, and the current tests only assert a small typed subset exists, not that the full catalog exists.

**Suggestion:** Add serializable data fields or a dedicated serializable describe DTO that includes the capability and resource-support data. If those legacy structs are not currently serializable, either derive/hand-roll serialization or expose a typed equivalent. Add a test that the JSON output contains capability/resource keys and representative nested data such as non-interactive capabilities and skill/command/agent/script support.

### 2. Payload detection is still hard-coded outside provider behavior traits

**Severity:** Medium

The design says runtime payload detection should live behind provider behavior, so a provider variant carries a typed handle to that dynamic facet. In practice, every provider behavior inherits the default `false` detection methods:

- `ProviderBehavior::detect_from_payload` defaults to `false`: `claudine/lib/src/provider/behavior.rs:42`.
- `AdapterBehavior::detect` also defaults to `false`: `claudine/lib/src/provider/behavior.rs:142`.
- Provider impls such as Claude only return `provider_adapter()` and do not override detection: `claudine/lib/src/provider/claude.rs:102`.
- `Provider::detect_from_payload` then falls back to direct provider-specific shape checks and explicit `Provider::Gemini`, `Provider::Claude`, `Provider::Codex`, `Provider::OpenCode`, and `Provider::KimiCode` returns: `claudine/lib/src/provider/methods.rs:141`.

So the behavior traits exist, but they are not the source of truth for this runtime operation. Adding a provider still requires editing `provider/methods.rs` for payload detection, which violates the "new provider file plus registry" goal and leaves tests too weak: they only prove the stubs do not panic, not that known payloads are recognized by their provider behavior.

**Suggestion:** Move the existing shape checks into per-provider behavior implementations. `Provider::detect_from_payload` should only iterate `all_providers()` and ask `info.behavior` or `info.adapter`. Add tests that known Claude/Gemini/Codex/OpenCode/Kimi payloads are detected by the relevant provider behavior directly, and add a source/invariant guard so `provider/methods.rs` cannot regain provider-specific fallback returns.

### 3. Public docs now contradict the implemented compatibility window

**Severity:** Low

Review 3 correctly restored the deprecated compatibility surfaces, but `building-an-agent-wrapper.md` still says Phase 8 removed them:

- The doc claims Phase 8 removed the deprecated `AgentId` alias and `crate::events::Provider` re-export: `claudine/docs/topics/building-an-agent-wrapper.md:190`.
- The implementation currently keeps `events::Provider` and related re-exports for one release cycle: `claudine/lib/src/events/provider.rs:9`.
- The implementation also keeps `agents::AgentId`: `claudine/lib/src/agents/mod.rs:15`.

This is not a runtime blocker, but it will mislead anyone adding providers or auditing the migration state.

**Suggestion:** Update the doc to say the compatibility exports remain until the post-Phase-8 cleanup release, matching the code comments and deprecation tests.

## Test Coverage Notes

Existing targeted tests are useful, but they miss the two most important gaps above:

- `provider_info_serializes_round_trip` checks selected typed keys but not the omitted capability/resource catalogs.
- `detect_from_payload_exercise_all_providers` and `adapter_detect_exercise_all_providers` only verify no panic on empty payloads; they do not require behavior-level detection to work.

## Verification

I ran:

```bash
sniff repo
cargo test -p claudine --lib provider::tests --quiet
cargo test -p claudine-cli providers --quiet
cargo test -p claudine-cli wrapper_registry_covers_every_provider_and_documents_exceptions --quiet
```

All targeted tests passed. The review findings are contract/coverage gaps, not failures in the current targeted test set.

## Production Readiness

Not ready. The implementation is close, but the serializable provider catalog is incomplete and one of the core dynamic operations still bypasses the centralized behavior model.
