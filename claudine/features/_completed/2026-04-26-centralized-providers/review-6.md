---
agent: ""
model: ""
ready: true
---

# Review 6 — Centralized Provider System

This review evaluates the final implementation of the centralized provider system against the [`spec.md`](./spec.md) and [`design.md`](./design.md) documents. Prior review suggestions have been successfully incorporated.

## Summary

The centralized provider system is now fully implemented and successfully serves as the single source of truth for all provider-specific data and behavior. The migration has successfully collapsed `AgentId` into `Provider`, centralized dispatch through `ProviderInfo` and behavior traits, and replaced stringly-typed metadata with strong types.

### Key Accomplishments

- **Centralized Registry:** `claudine::provider::registry::provider_info(Provider)` is the authoritative source for all provider facts. The implementation uses an O(1) `OnceLock`-backed array, ensuring both performance and exhaustiveness.
- **Strong Typing:** Descriptive `Vec<&'static str>` fields have been replaced with typed models like `PathTemplate`, `OutputFormatSupport`, `EntrypointSpec`, `SystemPromptSpec`, `YoloSupport`, and `ReasoningSupport`.
- **Trait-Based Behavior:** Genuinely dynamic operations (stream parsing, MCP injection, payload detection) are now behind focused traits (`ProviderBehavior`, `McpBehavior`, `AdapterBehavior`, `ConfiguratorBehavior`), eliminating giant `match` blocks.
- **Identifier Unification:** `AgentId` has been successfully unified with `Provider`. Compatibility is maintained via a `#[deprecated]` alias in `claudine::agents`.
- **CLI Wrapper Thinning:** `WrapperProfile` now leverages the central catalog for default implementations, significantly reducing per-provider boilerplate in the CLI crate.
- **Exceptional Testing:** The test suite includes 50+ exhaustiveness and invariant tests, including a source-code scanner that prevents unauthorized `match` expressions or provider-specific branches from drifting back into the library.

## Implementation Details & Gaps

### Functionality Gaps
- **None identified.** All phases (0–8) of the migration plan appear complete.
- **ACP Support:** Phase 7 (First-class ACP support) is implemented at the metadata level, allowing providers to declare ACP capture for events (e.g., Goose `request_permission`, Kimi `ApprovalRequest`). This matches the spec's intent to treat ACP as a typed capability.

### Correctness & Performance
- **Zero Heap Allocation:** `ProviderInfo` and its associated data live entirely in the binary's read-only segment.
- **Efficient Dispatch:** The hot path for identity lookups is O(1). Trait object indirection is reserved for setup-time or config-time operations, posing zero impact on event processing performance.
- **Backwards Compatibility:** Legacy facades (`agents::agent_for`, `linking::capabilities_for`) and CLI output (`claudine providers --describe`, `claudine hooks --support`) have been preserved and are verified by tests.

### Test Coverage
- **Exhaustiveness Invariants:** Registry coverage and self-consistency are pinned by tests that walk `PROVIDERS_DISPLAY_ORDER`.
- **Drift Guard:** The `no_unauthorized_match_provider_in_lib` and `detect_from_payload_has_no_provider_specific_branches` tests provide an innovative and robust defense against future regression.
- **Serialization:** `provider_info_serializes_round_trip` ensures the JSON describe surface is authoritative and complete.

## Ergonomics & Improvements

- **Registration Targets:** The `EventMappingTable::registration_targets()` helper provides a clean way for configurators to iterate only over events that require hook registration, replacing several ad-hoc lists.
- **PromptArgConventions:** Centralizing this in the catalog allowed for a single, provider-blind prompt extraction algorithm in the CLI wrapper, removing a significant source of "wrapper magic."

## Assessment

The implementation is **idiomatic, complete, and highly robust**. It effectively eliminates the technical debt of fragmented provider knowledge and establishes a scalable foundation for adding new providers.

**Status:** Ready for Production.
