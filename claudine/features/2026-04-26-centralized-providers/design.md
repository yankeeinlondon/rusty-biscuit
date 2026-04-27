# Centralized Provider System — Technical Design

Companion to: [`spec.md`](./spec.md)

This document describes implementation mechanics, module boundaries, error handling, testing patterns, and rollout logistics that are intentionally absent from the functional specification. Where the spec states *what* and *why*, this document states *how*.

---

## 1. Module Dependency Graph

```text
┌─────────────────────────────────────────────────────────────────────┐
│                            lib crate                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   provider/  │  │    events/   │  │   stream/    │              │
│  │  (canonical) │  │  (re-export) │  │              │              │
│  │              │  │              │  │              │              │
│  │ • Provider   │  │ • Provider   │  │ • Protocol   │              │
│  │   enum       │  │   (dep.)     │  │   types      │              │
│  │ • ProviderInfo│  │ • Matrix tests│ │ • Semantic   │              │
│  │ • provider_info│ │              │  │   parsers    │              │
│  └──────┬───────┘  └──────────────┘  └──────┬───────┘              │
│         │                                      │                     │
│  ┌──────▼───────┐  ┌──────────────┐  ┌────────▼───────┐            │
│  │   linking/   │  │    mcp/      │  │    agents/     │            │
│  │              │  │              │  │  (deprecated)  │            │
│  │ • Capability │  │ • Import     │  │                │            │
│  │   queries    │  │ • State      │  │ • Registry     │            │
│  │              │  │ • Inject     │  │   (deprecated) │            │
│  └──────────────┘  └──────────────┘  └────────────────┘            │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ uses (one-way)
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                           cli crate                                  │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    wrap/profile.rs                             │   │
│  │  • wrapper_for(Provider) → &'static dyn WrapperProfile        │   │
│  │  • Consumes provider_info(p).capabilities.*                   │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

**Critical invariant:** The lib crate must never depend on the cli crate. `WrapperProfile` types, `tempfile`, and `std::process::Command` are CLI-only. All cross-crate communication flows through `&'static ProviderInfo` (data) or `&'static dyn WrapperProfile` (CLI-side only).

---

## 2. Registry Implementation

### 2.1 Central registry: `provider_info(Provider) -> &'static ProviderInfo`

```rust
// lib/src/provider/registry.rs
use std::sync::OnceLock;

static REGISTRY: OnceLock<[&'static ProviderInfo; 8]> = OnceLock::new();

pub fn provider_info(p: Provider) -> &'static ProviderInfo {
    let registry = REGISTRY.get_or_init(|| [
        &CLAUDE_INFO,
        &CODEX_INFO,
        &GEMINI_INFO,
        &GOOSE_INFO,
        &KIMI_INFO,
        &OPENCODE_INFO,
        &QWEN_INFO,
        &ROO_INFO,
    ]);
    registry[p as usize]
}
```

**Why an array:** O(1) lookup, no branching, compile-time size check (array length must match variant count). The `Provider` enum derives `#[repr(usize)]` starting at 0.

**Why `OnceLock`:** The array elements are `&'static` references to const data; `OnceLock` initializes exactly once on first access. No lazy computation, no `LazyLock` overhead.

**Why not a `match`:** A `match` over `Provider` is exactly what this spec aims to eliminate as the *only* permitted match in the lib crate. The array indirection achieves the same exhaustiveness (the array length must match the enum variant count) without a match expression.

### 2.2 Exhaustiveness enforcement

The compiler enforces exhaustiveness via the array length:

```rust
// In provider/tests.rs or a compile_fail test
const _PROVIDER_COUNT: usize = 8; // must match Provider variant count

#[test]
fn registry_exhaustiveness() {
    let registry = REGISTRY.get().expect("registry initialized");
    assert_eq!(registry.len(), _PROVIDER_COUNT);
    for (i, info) in registry.iter().enumerate() {
        assert_eq!(*info as *const ProviderInfo, provider_info(Provider::from(i)) as *const ProviderInfo);
    }
}
```

If a new `Provider` variant is added without updating the array, the test fails at runtime. If `#[repr(usize)]` is used, a compile-time `static_assert` can be added via the `static_assertions` crate or a const-eval trick.

### 2.3 CLI-side wrapper registry

```rust
// cli/src/commands/wrap/profile.rs
static WRAPPER_REGISTRY: OnceLock<[&'static dyn WrapperProfile; 8]> = OnceLock::new();

pub fn wrapper_for(p: Provider) -> &'static dyn WrapperProfile {
    let registry = WRAPPER_REGISTRY.get_or_init(|| [
        &ClaudeWrapper,
        &CodexWrapper,
        // ... etc
    ]);
    registry[p as usize]
}
```

**Same pattern, separate crate.** The lib and CLI registries are initialized independently; the exhaustiveness test ensures both have the same variant count.

---

## 3. Error Type Hierarchy

### 3.1 Domain-specific error types

```rust
// lib/src/provider/mcp.rs
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("MCP not supported for this provider")]
    NotSupported,
    
    #[error("Invalid MCP configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

// lib/src/provider/config.rs
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Hook configuration not supported for this provider")]
    NotSupported,
    
    #[error("Unsupported config format: {0:?}")]
    UnsupportedFormat(ConfigFormat),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

**Design choice:** Each domain trait has its own error type rather than a unified `ProviderError`. This keeps error handling precise at call sites and avoids a monolithic error enum that every domain would have to contribute to.

### 3.2 Error propagation

Callers handle errors per-domain:

```rust
// Example: MCP injection
match provider_info(p).mcp.inject(&mut ctx) {
    Ok(()) => { /* injected */ }
    Err(McpError::NotSupported) => { /* skip — provider doesn't do MCP */ }
    Err(e) => return Err(e.into()), // real error
}
```

The `NotSupported` variant is not a failure; it's a typed way of saying "this provider doesn't participate in this domain."

---

## 4. Testing Strategy

### 4.1 Exhaustiveness tests (4 invariants)

These tests must run in CI for every PR. See spec §Exhaustiveness Tests for the four invariants.

**Additional recommendation:** Use a procedural or macro-generated test that fails at compile time if the `Provider` variant count changes:

```rust
// provider/tests.rs
#[test]
fn provider_variant_count_matches_registry() {
    // This will fail to compile if Provider gains/loses variants
    // without updating _PROVIDER_COUNT
    const _COUNT_CHECK: [(); 8] = [(); Provider::VARIANT_COUNT];
}
```

If `strum` is available, `Provider::VARIANT_COUNT` can be generated via `strum::EnumCount`.

### 4.2 Snapshot tests

All CLI output commands get snapshot tests:

```bash
# Phase 1 snapshot baseline
claudine providers --describe --format json > tests/snapshots/providers.json
claudine hooks --support > tests/snapshots/hooks_support.txt
claudine hooks --mapping > tests/snapshots/hooks_mapping.txt
claudine hooks --describe > tests/snapshots/hooks_describe.txt
```

**Across phases 1–6:** Snapshots must be bit-for-bit identical.
**Phase 7:** Snapshots update to include `Acp` capture method (intentional).
**Phase 8:** Snapshots update to remove deprecated fields (if any output changes).

### 4.3 Property tests (post-Phase 5)

```rust
// provider/tests/property.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn event_mapping_consistency(p in provider_strategy(), event in agentic_event_strategy()) {
        let info = provider_info(p);
        let mapping = info.event_mapping.mappings.iter()
            .find(|m| m.event == event)
            .expect("every event has a mapping");
        
        // Invariant: supported implies native_name is non-empty
        if !matches!(mapping.support, EventSupportLevel::NotSupported) {
            assert!(!mapping.native_name.is_empty());
        }
        
        // Invariant: Hook implies configurator is registered
        if matches!(mapping.support, EventSupportLevel::Hook { .. }) {
            assert!(info.configurator.supported_formats().len() > 0);
        }
        
        // Invariant: StreamParse implies protocol is Some
        if matches!(mapping.support, EventSupportLevel::StreamParse { .. }) {
            assert!(info.stream_protocol.is_some());
        }
        
        // Invariant: Acp implies AcpSupport is non-NotSupported
        if matches!(mapping.support, EventSupportLevel::Acp { .. }) {
            assert!(!matches!(info.capabilities.runtime.acp.server_mode, AcpServerMode::NotSupported));
        }
    }
}
```

### 4.4 Smoke tests

Manual smoke tests run before each phase ships:

```bash
for provider in claude codex gemini goose kimi opencode qwen; do
    echo "Testing $provider..."
    claudine "$provider" <<< "Say hello and exit"
done
```

---

## 5. Performance Considerations

### 5.1 Memory layout

`ProviderInfo` is a struct of `&'static` pointers and small values. On 64-bit systems:

- `&'static str`: 16 bytes (ptr + len)
- `&'static [T]`: 16 bytes (ptr + len)
- `&'static dyn Trait`: 16 bytes (data ptr + vtable ptr)
- `Option<StreamProtocol>`: 1–2 bytes (discriminant + payload, assuming `StreamProtocol` is a small enum)

Estimated `ProviderInfo` size: ~300–400 bytes per provider × 8 providers = ~3KB total. This lives in the binary's read-only data segment; no heap allocation.

### 5.2 Access patterns

```rust
// Hot path: CLI argument parsing
let info = provider_info(provider_arg);  // O(1) array index
info.capabilities.non_interactive.entrypoints  // O(1) field access
```

No dynamic dispatch on the hot path unless calling behavior traits (`info.behavior.create_semantic_parser(...)`), which happens during stream setup, not per-event.

### 5.3 Trait object overhead

The `ProviderInfo` struct carries 4 trait object fields (`behavior`, `mcp`, `adapter`, `configurator`). Each is a `&'static dyn Trait` (16 bytes). Vtable dispatch is used only when:

- Parsing inbound payloads (`AdapterBehavior::parse_payload`)
- Setting up stream parsers (`ProviderBehavior::create_semantic_parser`)
- Registering hooks (`ConfiguratorBehavior::register_hooks`)
- MCP operations (`McpBehavior::*`)

All of these are **config-time or setup-time** operations, not per-event hot paths. The spec's claim of < 0.01% wall-clock impact is defensible.

---

## 6. Deprecation Mechanics

### 6.1 Timeline

| Phase | Deprecated item | Replacement | Removal |
|-------|----------------|-------------|---------|
| 0 | `AgentId` | `Provider` | Phase 8 |
| 1 | `events::Provider` (re-export) | `provider::Provider` | Phase 8 |
| 5 | `agents::*` (re-export) | `provider::*` | Phase 8+1 release |
| 5 | `linking::capabilities::*` (re-export) | `provider::*` | Phase 8+1 release |

### 6.2 `#[deprecated]` attributes

```rust
// events/provider.rs (Phase 1)
#[deprecated(since = "0.9.0", note = "Use claudine::provider::Provider instead")]
pub use crate::provider::Provider;

// agents/mod.rs (Phase 5)
#[deprecated(since = "0.10.0", note = "Use claudine::provider::ProviderInfo instead")]
pub use crate::provider::{ProviderCapabilities, ProviderInfo};
```

### 6.3 External consumer survey

Before Phase 8, audit the rusty-biscuit monorepo and any published crates for:
- Imports of `claudine::events::Provider`
- Imports of `claudine::agents::*`
- Imports of `claudine::linking::capabilities::*`

Create migration issues/PRs for each affected consumer. Phase 8 does not ship until all in-repo consumers are migrated.

---

## 7. Open Implementation Questions

These are not blockers but should be decided during the first phase of implementation:

1. **Should `Provider` derive `strum::EnumCount`?** This would give compile-time `VARIANT_COUNT` for the registry array length check. If `strum` is not already a dependency, is it worth adding?

2. **Should `provider_info()` panic on unknown `Provider` variants?** With `#[repr(usize)]`, an out-of-bounds variant would panic on array access. Alternatively, use a `match` in debug builds for a better error message and the array in release builds.

3. **Should behavior traits use `&self` or `&'static self`?** The current design uses `&self` because the implementors are zero-sized types (`ClaudeBehavior`, `CodexBehavior`, etc.) and the references are `&'static`. The `&self` lifetime is implicitly `&'static self` in this context.

4. **How are provider-specific constants named?** The spec uses `CLAUDE_INFO`, `ClaudeBehavior`. Should these be `Claude::INFO`, `Claude::BEHAVIOR` (associated constants on a unit struct) or free constants in `provider/claude.rs`? Associated constants are more namespaced but slightly more verbose.

---

## 8. File Checklist for New Provider

When adding a ninth provider, the following files must be edited:

### Lib crate
- [ ] `lib/src/provider/mod.rs` — add variant to `Provider` enum
- [ ] `lib/src/provider/<name>.rs` — create provider definition (data + behavior impls)
- [ ] `lib/src/provider/registry.rs` — add `&<NAME>_INFO` to array
- [ ] `lib/src/provider/tests.rs` — no change needed (exhaustiveness tests auto-detect)
- [ ] `lib/src/stream/protocol/<name>.rs` — stream protocol types (if applicable)
- [ ] `lib/src/stream/<name>_semantic.rs` — semantic parser (if applicable)

### CLI crate
- [ ] `cli/src/commands/wrap/profile.rs` — add `WrapperProfile` implementor and registry entry

### Tests
- [ ] `tests/snapshots/*` — update if output changes (only if new provider affects default CLI output)

**Files that should NOT be edited:**
- `events/provider.rs` (thin re-export)
- `linking/capabilities.rs` (queries `provider_info`)
- `mcp/import.rs`, `mcp/state.rs`, `mcp/inject.rs` (dispatch via `McpBehavior` trait)
- `adapters/` (dispatch via `AdapterBehavior` trait)
- `config/` (dispatch via `ConfiguratorBehavior` trait)

This checklist validates the spec's success criterion: *"Adding a hypothetical ninth provider requires editing exactly one new lib file plus extending the central lib registries and the CLI-side wrapper_for registry."*
