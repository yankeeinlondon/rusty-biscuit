# PolicyEngine

`PolicyEngine` is Claudine's provider-agnostic permissions subsystem.

It answers questions like:

- Can this provider read or write a path?
- Will this command run automatically, ask first, or be denied?
- Is a domain allowed?
- Can an MCP server or MCP tool be used?
- Can a subagent be spawned?
- What approval and sandbox mode is in effect?
- What config change or one-shot CLI override would grant or deny a permission?

It is designed to normalize provider-native permission models into a single query and mutation surface without hiding important ambiguity.

## What It Does

`PolicyEngine` is responsible for:

- discovering provider-native permission sources on disk
- parsing those sources into typed native layers
- composing configured policy and CLI/runtime overrides
- converting the result into a canonical cross-provider model
- answering structured permission queries with explanations and warnings
- planning persistent config edits and one-shot CLI overrides

It is intentionally separate from `ProtectService`.

`ProtectService` is the runtime intervention layer.

`PolicyEngine` is the source of truth for "what is the provider configured to allow, ask, deny, or leave ambiguous?"

## Built-in Providers

`PolicyEngine::new()` registers built-in backends for:

- Claude
- Codex
- Gemini
- OpenCode
- Qwen
- Goose
- Kimi

You can check whether a backend is registered and what it supports:

```rust
use claudine::events::Provider;
use claudine::permissions::PolicyEngine;

let engine = PolicyEngine::new();

assert!(engine.has_backend(Provider::Claude));

let caps = engine.capabilities(Provider::Codex)?;
println!("{:?}", caps.fidelity);
println!("MCP queries: {}", caps.mcp_queries);
```

## Core Concepts

### Configured vs Effective

The engine distinguishes two kinds of policy snapshots:

- `ConfiguredPolicySnapshot`
  On-disk policy only
- `EffectivePolicySnapshot`
  On-disk policy plus CLI/runtime overrides

This distinction matters because many providers can change behavior for a single run through CLI flags.

Typical pattern:

```rust
use claudine::events::Provider;
use claudine::permissions::{CliPolicyInput, PolicyContext, PolicyEngine};

let engine = PolicyEngine::new();
let ctx = PolicyContext::new(std::env::current_dir()?);

let configured = engine.configured(Provider::Claude, &ctx)?;
let effective = engine.effective(
    Provider::Claude,
    &ctx,
    CliPolicyInput::Argv(&[
        "--permission-mode".to_owned(),
        "auto".to_owned(),
    ]),
)?;
```

### Native vs Canonical

Each provider backend keeps a native representation internally, because provider-specific mutation planning and provenance need provider-specific data.

The engine also produces a canonical model so callers can ask the same kinds of questions across providers.

The canonical policy is organized into six axes:

- filesystem
- commands
- network
- MCP
- agents
- runtime

### Explainability

Queries do not return a bare boolean. They return a `QueryResult`:

```rust
pub struct QueryResult {
    pub effect: Option<PolicyEffect>,
    pub certainty: PolicyCertainty,
    pub stability: QueryStability,
    pub matched_rules: Vec<MatchedRule>,
    pub explanation: PolicyExplanation,
    pub warnings: Vec<PolicyWarning>,
}
```

This lets callers distinguish:

- a definite allow
- an ask-before-allow
- a definite deny
- an unknown answer because trust, context, or provider semantics are ambiguous

## Creating a Context

Every query and mutation flow starts with `PolicyContext`.

```rust
use claudine::permissions::{PolicyContext, ProjectTrustContext, TrustSource};

let ctx = PolicyContext::new(std::env::current_dir()?)
    .with_repo_root(repo_root.clone())
    .with_home_dir(home_dir.clone())
    .with_trust(ProjectTrustContext {
        is_trusted: Some(true),
        source: TrustSource::ExplicitInput,
    });
```

Important fields:

- `cwd`
  Used for relative-path normalization during queries
- `repo_root`
  Used for repo-scoped config discovery and workspace classification
- `home_dir`
  Used for user config discovery
- `system_root`
  Useful for deterministic tests
- `env`
  Captured environment used by provider discovery
- `trust`
  Important for trust-gated providers such as Codex and Gemini

If trust is unknown for a provider that gates repo policy on trust, queries may intentionally return unknown results with warnings.

## Querying Policy

There are two main entrypoints:

- `PolicyEngine::configured(provider, ctx)`
- `PolicyEngine::effective(provider, ctx, cli)`

You can also create a provider-scoped handle:

```rust
use claudine::events::Provider;
use claudine::permissions::PolicyEngine;

let engine = PolicyEngine::new();
let claude = engine.provider(Provider::Claude);

let snapshot = claude.configured(&ctx)?;
```

### Convenience Query Methods

Both snapshot types expose the same convenience methods:

- `can_read(path)`
- `can_write(path)`
- `can_traverse(path)`
- `can_execute(&CommandQuery)`
- `can_access_domain(domain)`
- `can_use_mcp_server(server)`
- `can_use_mcp_tool(server, tool)`
- `can_spawn_subagent(name)`
- `can_switch_mode(target)`
- `can_modify_own_config()`
- `query(&PolicyQuery)`

Example:

```rust
use claudine::permissions::CommandQuery;

let result = snapshot.can_execute(&CommandQuery::from_raw("git status"));

if result.is_allowed() {
    println!("command is allowed");
}
```

### Generic Query API

For code that wants one generic entrypoint, use `PolicyQuery`:

```rust
use claudine::permissions::{PathQuery, PolicyQuery};

let result = snapshot.query(&PolicyQuery::ReadPath(PathQuery::unknown("src/main.rs")));
```

Supported query kinds:

- `ReadPath`
- `WritePath`
- `TraversePath`
- `ExecuteCommand`
- `AccessDomain`
- `UseMcpServer`
- `UseMcpTool`
- `SpawnSubagent`
- `SwitchMode`
- `ModifyProviderConfig`

## Query Semantics

### Path Queries

Path queries are normalized relative to `PolicyContext.cwd`.

That means these two calls are treated as the same target when possible:

- `snapshot.can_read("src/main.rs")`
- `snapshot.can_read("/repo/src/main.rs")`

The engine also classifies the normalized path and includes that in explanations:

- workspace
- provider-config
- home
- temp
- system
- external

### Command Queries

Use `CommandQuery::from_raw(...)` for the common case.

It performs best-effort shell-style tokenization and extracts an executable name:

```rust
use claudine::permissions::CommandQuery;

let query = CommandQuery::from_raw("FOO=1 cargo test --lib");
assert_eq!(query.executable.as_deref(), Some("cargo"));
```

### MCP Queries

The engine supports both:

- server-level MCP queries
- tool-level MCP queries

Tool queries inherit server-level policy when appropriate. For example, if a server is denied, its tools are denied too.

### Config Modification Query

`can_modify_own_config()` does not mean "is the file writable on the host filesystem?"

It means "does the canonical policy consider the provider's own config paths protected?"

This is useful when integrating with runtime safety logic.

## Understanding `QueryResult`

### `effect`

The canonical decision vocabulary is:

- `Allow`
- `Ask`
- `Deny`

`effect` is `Option<PolicyEffect>`, not just `PolicyEffect`.

When the engine cannot answer confidently, it returns `None`.

### `certainty`

`certainty` tells you how authoritative the answer is:

- `Exact`
- `BestEffort`
- `Unknown`

### `stability`

`stability` tells you whether the answer could change under different conditions:

- `Stable`
- `MayChangeWithCli`
- `MayChangeAtRuntime`
- `Unknown`

Configured snapshots often return `MayChangeWithCli` for axes where provider CLI flags can change behavior.

### `matched_rules`

`matched_rules` shows which canonical rules produced the answer, in precedence order.

Each matched rule carries provenance back to the provider-native source.

### `explanation`

`explanation.summary` is a short human-readable answer.

`explanation.reasons` contains structured reasons with:

- source id
- native reference
- message
- mapping fidelity

### `warnings`

Warnings surface important ambiguity or degraded modeling.

Examples:

- trust is unknown for a trust-gated provider
- a backend had to approximate native provider behavior
- a mutation plan is broadened or unsupported

## Canonical Policy Model

If you need the entire normalized policy rather than a single query result, use `snapshot.canonical`.

The top-level shape is:

```rust
pub struct CanonicalPolicy {
    pub provider: Provider,
    pub mode: PolicyMode,
    pub axes: CanonicalPolicyAxes,
    pub provenance: Vec<CanonicalRuleProvenance>,
    pub warnings: Vec<PolicyWarning>,
}
```

This is useful when:

- building higher-level UI
- inspecting all configured rules
- exporting policy summaries
- integrating another service with the engine

## Mutation Planning

`PolicyEngine` supports mutation planning through `PolicyChange`.

The important distinction is:

- mutation planning
  "What would need to change?"
- mutation application
  "Write the planned persistent edits"

### Create a Change Request

```rust
use std::path::PathBuf;

use claudine::permissions::{
    CommandPattern, PolicyChange, PolicyChangeOp,
};

let change = PolicyChange::persistent(vec![
    PolicyChangeOp::GrantWrite(PathBuf::from("/tmp/build")),
    PolicyChangeOp::AllowCommand(CommandPattern::new("cargo test")),
]);
```

Common operations include:

- `GrantRead`
- `GrantWrite`
- `DenyRead`
- `DenyWrite`
- `RequireApprovalForCommand`
- `AllowCommand`
- `DenyCommand`
- `AllowDomain`
- `DenyDomain`
- `AllowMcpServer`
- `DenyMcpServer`
- `AllowMcpTool`
- `DenyMcpTool`
- `AllowSubagent`
- `DenySubagent`
- `SetApprovalMode`
- `SetSandboxMode`

### Choose Persistence

Two persistence modes are supported:

- `PolicyPersistence::Persistent`
  Plan config-file changes
- `PolicyPersistence::OneShot`
  Plan CLI/env overrides only

Convenience constructors:

- `PolicyChange::persistent(...)`
- `PolicyChange::one_shot(...)`

### Choose a Target

Persistent changes can target:

- `Auto`
- `UserConfig`
- `RepoConfig`
- `LocalOverride`

Important note:

- `LocalOverride` is provider-specific
- Claude supports a real local override file
- providers without a local override concept may reject this target explicitly

### Plan a Change

```rust
let handle = engine.provider(Provider::Claude);
let plan = handle.plan_change(&ctx, &change)?;
```

The result is a `PolicyMutationPlan`:

```rust
pub struct PolicyMutationPlan {
    pub provider: Provider,
    pub persistent_plan: Option<PersistentMutationPlan>,
    pub one_shot_plan: Option<OneShotMutationPlan>,
    pub warnings: Vec<PolicyWarning>,
    pub supported: bool,
}
```

### Inspect the Plan

Persistent plans contain file edit previews:

```rust
if let Some(persistent) = &plan.persistent_plan {
    for edit in &persistent.edits {
        println!("{}", edit.path.display());
        println!("{}", edit.description);
        println!("{}", edit.after_preview);
    }
}
```

One-shot plans contain launch-time overrides:

```rust
if let Some(one_shot) = &plan.one_shot_plan {
    println!("argv: {:?}", one_shot.argv);
    println!("env: {:?}", one_shot.env);
}
```

### Apply the Plan

```rust
let report = plan.apply()?;
println!("applied {} edit(s)", report.edits_applied);
```

`apply()` only executes persistent file edits.

One-shot plans are descriptive and must be used by the caller when launching the provider.

## Common Example

This example shows the full flow:

```rust
use std::path::PathBuf;

use claudine::events::Provider;
use claudine::permissions::{
    CliPolicyInput, CommandPattern, CommandQuery, PolicyChange, PolicyChangeOp,
    PolicyContext, PolicyEngine,
};

let engine = PolicyEngine::new();
let ctx = PolicyContext::new(std::env::current_dir()?);

// Inspect current configured policy
let snapshot = engine.configured(Provider::Codex, &ctx)?;
let write_result = snapshot.can_write("target/output.txt");
println!("{}", write_result.explanation.summary);

// Inspect effective policy with CLI overrides
let effective = engine.effective(
    Provider::Codex,
    &ctx,
    CliPolicyInput::Argv(&["--full-auto".to_owned()]),
)?;
let exec_result = effective.can_execute(&CommandQuery::from_raw("git status"));
println!("{:?}", exec_result.effect);

// Plan a one-shot change
let change = PolicyChange::one_shot(vec![
    PolicyChangeOp::AllowCommand(CommandPattern::new("cargo test")),
    PolicyChangeOp::GrantWrite(PathBuf::from("/tmp/cache")),
]);
let plan = engine.provider(Provider::Codex).plan_change(&ctx, &change)?;

if let Some(one_shot) = &plan.one_shot_plan {
    println!("argv: {:?}", one_shot.argv);
}
```

## Provider Capability Checks

Backends do not all support every axis or mutation shape equally.

Use capability metadata before assuming support:

```rust
let caps = engine.capabilities(Provider::Goose)?;

if caps.mcp_queries {
    let result = snapshot.can_use_mcp_server("filesystem");
    println!("{:?}", result.effect);
}
```

Capability fields:

- `fidelity`
- `filesystem_queries`
- `command_queries`
- `network_queries`
- `mcp_queries`
- `agent_queries`
- `persistent_mutations`
- `one_shot_mutations`

## Common Ambiguity Cases

### Trust-gated repo policy

For providers that require trust before repo config is active, unknown trust can produce unknown query answers with warnings.

### CLI-sensitive configured results

Configured snapshots may say `MayChangeWithCli` because a launch flag could change the answer for that axis.

### Provider-specific limitations

Some native provider models do not map perfectly into the canonical model. When that happens:

- `certainty` may degrade to `BestEffort`
- provenance fidelity may be `Approximate`
- warnings may explain the limitation

## When to Use `PolicyEngine`

Use `PolicyEngine` when you need:

- a provider-independent permission query surface
- explanations for why a decision was reached
- a distinction between configured and effective policy
- a safe preview of permission changes before writing config
- one-shot CLI/env permission plans

Do not use it as a runtime blocker by itself.

If you need to decide whether to stop or intervene in a live session, use a runtime policy layer such as `ProtectService` and consume `PolicyEngine` from there.
