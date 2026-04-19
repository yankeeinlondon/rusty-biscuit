# Feature: PipelineBuilder for Testing

## Problem Statement

Testing complex pipelines in Claudine requires verbose, repetitive setup code that obscures test intent. Three areas are particularly affected:

### 1. EventMeta Construction

`EventMeta` has 14 fields. Tests routinely construct it with only 2-3 fields set, forcing verbose patterns like:

```rust
let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::BeforeTool);
meta.session_id = Some("test-sess".into());
meta.tool_name = Some("Bash".into());
meta.tool_input = Some(json!({"command": "rm -rf /"}));
meta.env = EnvironmentContext { /* complex nested struct */ };
```

### 2. Dispatch Config Construction

The `canonical_dispatch.rs` tests use a `make_config_with_action()` helper, but it doesn't scale to multi-binding scenarios, protect configurations, or harness plans:

```rust
fn make_config_with_action(event: AgenticEvent, action: HookAction) -> CanonicalRuntimeConfig {
    let mut actions = HashMap::new();
    actions.insert(event, vec![action]);
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions = actions;
    compile_canonical_runtime(config, None).unwrap()
}
```

### 3. HarnessPlan Construction

`HarnessPlan` is a complex nested struct (timeout, pre_checks, post_checks, handlers) that requires significant boilerplate even for simple test cases.

## Goals

Provide test-specific builder types that:

1. Reduce boilerplate in test code
2. Make test intent clearer by focusing on relevant fields
3. Provide sensible defaults so tests don't need to specify every field
4. Live behind `#[cfg(test)]` gates so they add zero production code
5. Are located in `claudine-lib/tests/` as a test-only utility module

## Design

### Module Location

```
claudine/lib/tests/
├── builders/
│   ├── mod.rs
│   ├── event_meta.rs      # EventMetaBuilder
│   ├── dispatch_config.rs # DispatchConfigBuilder / CanonicalRuntimeBuilder
│   └── harness_plan.rs   # HarnessScenarioBuilder
```

### EventMetaBuilder

```rust
#[cfg(test)]
pub struct EventMetaBuilder {
    provider: Provider,
    event: AgenticEvent,
    session_id: Option<String>,
    tool_name: Option<String>,
    tool_input: Option<Value>,
    tool_response: Option<Value>,
    error: Option<String>,
    prompt: Option<String>,
    extra: HashMap<String, Value>,
    env: EnvironmentContext,
}

#[cfg(test)]
impl EventMetaBuilder {
    pub fn new(provider: Provider, event: AgenticEvent) -> Self;

    // Fluent setters — all return Self for chaining
    pub fn session_id(mut self, id: impl Into<String>) -> Self;
    pub fn with_tool(mut self, name: impl Into<String>, input: Value) -> Self;
    pub fn with_mcp_tool(mut self, server: &str, tool: &str, response: Value) -> Self;
    pub fn with_error(mut self, msg: impl Into<String>) -> Self;
    pub fn extra(mut self, key: impl Into<String>, value: Value) -> Self;
    pub fn cwd(mut self, path: impl Into<String>) -> Self;
    pub fn env(mut self, env: EnvironmentContext) -> Self;

    // Terminal build method
    pub fn build(self) -> EventMeta;
}
```

**Defaults:**
- `timestamp: Utc::now()`
- `env: EnvironmentContext::default()`
- `extra: HashMap::new()`

**Example usage:**
```rust
// Before
let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::BeforeTool);
meta.tool_name = Some("Bash".into());
meta.tool_input = Some(json!({"command": "rm -rf /"}));
meta.env = EnvironmentContext::default();

// After
let meta = EventMetaBuilder::new(Provider::Claude, AgenticEvent::BeforeTool)
    .with_tool("Bash", json!({"command": "rm -rf /"}))
    .build();

// MCP tool example
let meta = EventMetaBuilder::new(Provider::Claude, AgenticEvent::AfterTool)
    .with_mcp_tool("evil", "read", json!("prompt injection"))
    .session_id("test-123")
    .build();
```

### CanonicalRuntimeBuilder

```rust
#[cfg(test)]
pub struct CanonicalRuntimeBuilder {
    config: ClaudineConfig,
    repo_root: Option<PathBuf>,
}

#[cfg(test)]
impl CanonicalRuntimeBuilder {
    pub fn new() -> Self;

    // Binding management
    pub fn bind(mut self, event: AgenticEvent, actions: Vec<HookAction>) -> Self;
    pub fn bind_one(mut self, event: AgenticEvent, action: HookAction) -> Self;

    // Protect configuration
    pub fn protect_enabled(mut self, enabled: bool) -> Self;
    pub fn protect_with_rules(mut self, rules: ProtectConfig) -> Self;

    // Logging
    pub fn logging(mut self, enabled: bool) -> Self;

    // Sound defaults
    pub fn default_sounds(mut self, sounds: DefaultSounds) -> Self;

    // Repository root
    pub fn repo_root(mut self, path: impl Into<PathBuf>) -> Self;

    pub fn build(self) -> CanonicalRuntimeConfig;
}
```

**Defaults:**
- `protect.enabled: false`
- `logging: true`
- `default_sounds: DefaultSounds::default()`

**Example usage:**
```rust
// Before
let mut actions = HashMap::new();
actions.insert(AgenticEvent::SessionStart, vec![HookAction::Report { handler: None }]);
let mut config = ClaudineConfig::default();
config.protect.enabled = false;
config.default_sounds = DefaultSounds::default();
config.actions = actions;
let runtime = compile_canonical_runtime(config, None).unwrap();

// After
let runtime = CanonicalRuntimeBuilder::new()
    .bind_one(AgenticEvent::SessionStart, HookAction::Report { handler: None })
    .protect_enabled(false)
    .build();

// Multi-binding with protect
let runtime = CanonicalRuntimeBuilder::new()
    .bind(AgenticEvent::BeforeTool, vec![
        HookAction::SoundEffect { effect: "alert".into(), volume: 1.0, speed: 1.0 }
    ])
    .bind(AgenticEvent::AfterTool, vec![
        HookAction::Report { handler: None }
    ])
    .protect_enabled(true)
    .build();
```

### HarnessScenarioBuilder

```rust
#[cfg(test)]
pub struct HarnessScenarioBuilder {
    source_path: PathBuf,
    timeout: Option<Duration>,
    pre_checks: Vec<ValidationRule>,
    post_checks: Vec<ValidationRule>,
    handlers: Vec<(FailureEvent, HandlerAction)>,
}

#[cfg(test)]
impl HarnessScenarioBuilder {
    pub fn new(source_path: impl Into<PathBuf>) -> Self;

    // Timeout
    pub fn timeout(mut self, duration: Duration) -> Self;

    // Pre-check validations
    pub fn pre_check(mut self, rule: ValidationRule) -> Self;
    pub fn pre_check_file_exists(mut self, path: PathBuf) -> Self;
    pub fn pre_check_shell(mut self, cmd: String) -> Self;

    // Post-check validations
    pub fn post_check(mut self, rule: ValidationRule) -> Self;
    pub fn post_check_response_includes(mut self, needle: String) -> Self;

    // Handler for failures
    pub fn on_failure(mut self, event: FailureEvent, action: HandlerAction) -> Self;

    pub fn build(self) -> HarnessPlan;
}
```

## Implementation Notes

### Builder Pattern Conventions

1. **Immutable builds** — Each builder method consumes `self` and returns `Self`, enabling fluent chaining without interior mutability.

2. **No panic in build** — Builders should only populate fields that have defaults. `build()` should always succeed for valid test input.

3. **Type coherence** — Use the actual domain types (`AgenticEvent`, `HookAction`, `Provider`) rather than raw strings/integers where possible.

4. **Sparse by default** — Only require what is necessary to construct a valid object. Use `Default::default()` for all optional fields.

### Module Structure

```rust
// claudine/lib/tests/builders/mod.rs
pub mod event_meta;
pub mod dispatch_config;
pub mod harness_plan;

pub use event_meta::EventMetaBuilder;
pub use dispatch_config::CanonicalRuntimeBuilder;
pub use harness_plan::HarnessScenarioBuilder;
```

### Integration with Existing Tests

Existing tests should be **gradually migrated**, not rewritten. The builders are additive:

```rust
// New test — use builder
#[tokio::test]
async fn dispatch_with_tool_block() {
    let runtime = CanonicalRuntimeBuilder::new()
        .protect_enabled(true)
        .build();

    let meta = EventMetaBuilder::new(Provider::Claude, AgenticEvent::BeforeTool)
        .with_tool("Bash", json!({"command": "rm -rf /"}))
        .build();

    let outcome = dispatch_canonical_with_runtime(Provider::Claude, AgenticEvent::BeforeTool, meta, &runtime).await.unwrap();
    assert!(outcome.protect_pre.as_ref().is_some_and(|d| d.is_blocked()));
}
```

### Relationship to Developer's CLI PipelineBuilder

The CLI-focused `PipelineBuilder` in `wrap/profile.rs` addresses production code duplication. The testing builders described here address test code verbosity. They are **orthogonal**:

| PipelineBuilder (CLI) | Builders (Testing) |
|---------------------|-------------------|
| Production code | Test code only |
| Reduces wrapper arg mutations | Reduces setup boilerplate |
| Lives in `cli/src/commands/wrap/` | Lives in `lib/tests/builders/` |
| `#[cfg(test)]` not needed | Behind `#[cfg(test)]` gates |

## Acceptance Criteria

- [ ] `EventMetaBuilder` exists with all fluent setters and `build()` method
- [ ] `CanonicalRuntimeBuilder` exists supporting multi-binding, protect config, and logging toggle
- [ ] `HarnessScenarioBuilder` exists supporting pre/post checks and failure handlers
- [ ] All builders are behind `#[cfg(test)]` gates
- [ ] Module is accessible from `claudine-lib` integration tests via `use claudine::tests::builders::*`
- [ ] At least 3 existing tests are migrated to use the new builders
- [ ] No production code paths are affected
- [ ] All existing dispatch tests pass with the new builders
- [ ] Documentation: each builder has doc comments with usage examples

## Out of Scope

- Builder for `CompositionExecutionRequest` (38-field struct, low test utility)
- Runtime `PipelineBuilder` for CLI arg deduplication (separate effort)
- Changing existing test structure or patterns outside of migration
- Any production-facing builder types
