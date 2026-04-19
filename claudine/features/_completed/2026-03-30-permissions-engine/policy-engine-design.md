# Policy Engine Design

This document turns the permissions-engine spec into an implementation-ready design for a new `PolicyEngine` subsystem in Claudine.

Primary inputs:

- `claudine/features/2026-03-30-permissions-engine/spec.md`
- `claudine/features/2026-03-30-permissions-engine/opinion.md`
- provider permissions research in `claudine/docs/research/permissions/`
- current provider model in `claudine/lib/src/events/`
- current provider adapter and capability patterns in `claudine/lib/src/adapters/`
- current Protect implementation in `claudine/lib/src/services/protect.rs`

The core design decision is:

**`PolicyEngine` becomes Claudine's canonical provider-permissions layer.**

It owns:

1. loading provider-native permission state from disk
2. composing that state with CLI/runtime overrides
3. normalizing the result into a canonical cross-provider model
4. answering permission queries with explanation and provenance
5. planning and applying permission changes back to provider-native config or one-shot CLI args

It does **not** own runtime hook intervention, redaction, or live session blocking decisions. Those are explicitly out of scope for this design.

## Summary

`PolicyEngine` should be built as a new library module under `claudine/lib/src/permissions/`.

The design has five major parts:

1. a provider-agnostic canonical policy model
2. a backend trait for provider-specific config parsing and mutation planning
3. snapshot objects for configured and effective policies
4. a query API that returns decision plus explanation, not just a boolean
5. a mutation API that separates persistent config changes from ephemeral one-shot overrides

The result should let Claudine do all of the following consistently:

- ask "what is configured right now?"
- ask "what would be effective if these CLI args are used?"
- explain why a path, command, domain, MCP tool, or subagent action is allowed, asks, or denied
- generate an exact change plan before touching provider config
- produce one-time CLI flags where possible instead of mutating files

## Goals

1. Provide a single cross-provider permission query layer for all supported providers.
2. Distinguish clearly between configured policy and effective policy.
3. Preserve provider fidelity rather than flattening everything to the least common denominator.
4. Make explainability a first-class output.
5. Support both persistent and one-time mutation planning.
6. Keep the subsystem independent from Protect so it can stand on its own.
7. Make provider backends incremental so support can land provider by provider.

## Non-Goals

1. Rebuilding `ProtectService`.
2. Designing runtime hook outcomes like `StopSession` or `AskThenAllowOrStop`.
3. Inventing new provider permission models that do not map back to native provider behavior.
4. Hiding ambiguity when provider behavior is not fully knowable from config alone.
5. Supporting every provider at identical fidelity on day one.

## Design Principles

### 1. Canonical model first, but never at the cost of losing provenance

The engine must normalize policy into a common model, but every normalized rule must retain:

- which provider source produced it
- which native rule or field it came from
- whether the mapping is exact or degraded

Without that, the engine will be hard to trust.

### 2. Configured and effective are distinct products

The engine must never blur:

- filesystem-configured state
- CLI/runtime-effective state

Configured policy is stable and durable. Effective policy is ephemeral and invocation-specific.

### 3. Query results must be richer than `bool`

Simple booleans are not enough. A useful result must tell the caller:

- allow / ask / deny / unknown
- how confident the engine is
- whether the answer is exact or degraded
- what source rules produced the answer

### 4. Mutation planning is separate from mutation execution

The engine should always be able to show:

- what would change
- which file(s) would change
- whether the mapping is exact
- what one-shot CLI args could be used instead

before it writes anything.

### 5. Provider backends should be typed, not regex soup

Where possible, each backend should parse provider-native configs into typed intermediate structures rather than operating on raw strings.

## Recommended Module Layout

```txt
claudine/lib/src/permissions/
├── mod.rs
├── engine.rs
├── backend.rs
├── context.rs
├── canonical.rs
├── query.rs
├── explain.rs
├── change.rs
├── mutation.rs
├── native.rs
├── matchers.rs
└── providers/
   ├── mod.rs
   ├── claude.rs
   ├── codex.rs
   ├── gemini.rs
   ├── opencode.rs
   ├── qwen.rs
   ├── roo.rs
   ├── goose.rs
   └── kimi.rs
```

Recommended responsibilities:

- `engine.rs`
  - `PolicyEngine`
  - engine construction
  - backend registry
  - top-level snapshot APIs
- `backend.rs`
  - `ProviderPolicyBackend` trait
  - backend capability metadata
- `context.rs`
  - discovery context
  - config roots
  - env and CLI capture types
- `canonical.rs`
  - provider-agnostic policy model
  - provenance and fidelity metadata
- `query.rs`
  - query input types
  - query result types
  - snapshot query surfaces
- `explain.rs`
  - explanation rendering structs
  - human-readable summaries
- `change.rs`
  - `PolicyChange`
  - change operations and targeting
- `mutation.rs`
  - mutation plan types
  - patch/edit preview model
- `native.rs`
  - provider-native typed intermediate config structs
  - shared source-layer metadata
- `matchers.rs`
  - path, command, domain, tool, and glob matching helpers
- `providers/*.rs`
  - provider-specific discovery, parsing, canonicalization, and mutation planning

## Public API Shape

The engine should expose two ergonomic entrypoints:

1. a provider-agnostic snapshot API
2. a provider-selected convenience API

Recommended shape:

```rust
pub struct PolicyEngine {
    backends: HashMap<Provider, Box<dyn ProviderPolicyBackend>>,
}

impl PolicyEngine {
    pub fn new() -> Self;

    pub fn provider(&self, provider: Provider) -> ProviderPolicyHandle<'_>;

    pub fn configured(
        &self,
        provider: Provider,
        ctx: &PolicyContext,
    ) -> Result<ConfiguredPolicySnapshot>;

    pub fn effective(
        &self,
        provider: Provider,
        ctx: &PolicyContext,
        cli: CliPolicyInput<'_>,
    ) -> Result<EffectivePolicySnapshot>;
}

pub struct ProviderPolicyHandle<'a> {
    engine: &'a PolicyEngine,
    provider: Provider,
}

impl ProviderPolicyHandle<'_> {
    pub fn configured(&self, ctx: &PolicyContext) -> Result<ConfiguredPolicySnapshot>;

    pub fn effective(
        &self,
        ctx: &PolicyContext,
        cli: CliPolicyInput<'_>,
    ) -> Result<EffectivePolicySnapshot>;

    pub fn plan_change(
        &self,
        ctx: &PolicyContext,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan>;
}
```

This supports the spec's intent without making the query API itself provider-specific after CLI args are supplied.

## Core Context Types

### `PolicyContext`

The engine needs an explicit context object rather than reading ambient process state directly.

```rust
pub struct PolicyContext {
    pub cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub home_dir: Option<PathBuf>,
    pub system_root: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub trust: ProjectTrustContext,
}
```

Recommended behavior:

- `cwd` is always required
- `repo_root` is optional because some queries may happen outside a repo
- `home_dir` and `system_root` allow deterministic testing
- `env` is included because some providers use env vars to relocate config or influence policy
- `trust` captures whether project-local config is active for providers with trust gates like Codex and Gemini

### `ProjectTrustContext`

```rust
pub struct ProjectTrustContext {
    pub is_trusted: Option<bool>,
    pub source: TrustSource,
}

pub enum TrustSource {
    ProviderConfig,
    ExplicitInput,
    Unknown,
}
```

This prevents the engine from silently assuming project config is active when trust status is actually unknown.

## CLI Input Types

The engine should support two forms of effective-policy input:

```rust
pub enum CliPolicyInput<'a> {
    None,
    Argv(&'a [String]),
    Parsed(&'a ProviderCliOverrides),
}
```

`Argv` is the ergonomic caller entrypoint. `Parsed` exists for tests and future wrapper integration.

## Backend Trait

Each provider backend should implement one trait with five responsibilities:

1. discover relevant config sources
2. load typed native policy layers
3. compose layers into an effective native policy
4. canonicalize native policy into the shared model
5. plan persistent or one-shot changes

Recommended shape:

```rust
pub trait ProviderPolicyBackend: Send + Sync {
    fn provider(&self) -> Provider;

    fn discover_sources(&self, ctx: &PolicyContext) -> Result<Vec<PolicySource>>;

    fn load_native_layers(
        &self,
        ctx: &PolicyContext,
        sources: &[PolicySource],
    ) -> Result<Vec<NativePolicyLayer>>;

    fn parse_cli_overrides(
        &self,
        ctx: &PolicyContext,
        input: CliPolicyInput<'_>,
    ) -> Result<ProviderCliOverrides>;

    fn compose_native_policy(
        &self,
        ctx: &PolicyContext,
        layers: &[NativePolicyLayer],
        cli: Option<&ProviderCliOverrides>,
    ) -> Result<NativeEffectivePolicy>;

    fn canonicalize(
        &self,
        ctx: &PolicyContext,
        native: &NativeEffectivePolicy,
    ) -> Result<CanonicalPolicy>;

    fn plan_change(
        &self,
        ctx: &PolicyContext,
        current: &NativeEffectivePolicy,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan>;
}
```

## Why native typed intermediates matter

The canonical model should not be the only data model. Backends need native typed policy models because:

- mutation planning is easiest against native types
- exact provenance needs native rule references
- some provider behaviors are not naturally expressible directly in the canonical model

Examples:

- Codex sandbox and approval policy are separate native concepts
- Claude permissions, sandbox, and managed settings are layered differently
- Roo has modes, protected files, command allow/deny lists, and auto-approval categories

## Source Discovery Model

Every loaded layer should be represented explicitly.

```rust
pub struct PolicySource {
    pub id: String,
    pub kind: PolicySourceKind,
    pub path: Option<PathBuf>,
    pub precedence: u16,
    pub writable: bool,
}

pub enum PolicySourceKind {
    UserConfig,
    RepoConfig,
    LocalOverride,
    ManagedConfig,
    SystemConfig,
    ProfileConfig,
    RuleFile,
    CliOverride,
    EnvironmentOverride,
    Derived,
}
```

Recommended behavior:

- all configured snapshots include only filesystem-derived layers
- all effective snapshots append CLI and env layers as ephemeral sources
- every query result can cite these source IDs

## Canonical Policy Model

The canonical model should cover the recurring security axes across providers rather than trying to mimic one provider's config layout.

Recommended top-level shape:

```rust
pub struct CanonicalPolicy {
    pub provider: Provider,
    pub mode: PolicyMode,
    pub axes: CanonicalPolicyAxes,
    pub integrity: PolicyIntegrity,
    pub provenance: Vec<CanonicalRuleProvenance>,
    pub warnings: Vec<PolicyWarning>,
}

pub enum PolicyMode {
    Configured,
    Effective,
}

pub struct CanonicalPolicyAxes {
    pub filesystem: FilesystemPolicy,
    pub commands: CommandPolicy,
    pub network: NetworkPolicy,
    pub mcp: McpAccessPolicy,
    pub agents: AgentPolicy,
    pub runtime: RuntimePolicy,
}
```

### Filesystem axis

```rust
pub struct FilesystemPolicy {
    pub read_rules: Vec<PathAccessRule>,
    pub write_rules: Vec<PathAccessRule>,
    pub traversal_rules: Vec<PathAccessRule>,
    pub protected_config_paths: Vec<PathProtectionRule>,
}
```

### Command axis

```rust
pub struct CommandPolicy {
    pub shell_rules: Vec<CommandAccessRule>,
    pub execution_modes: Vec<ExecutionModeRule>,
}
```

### Network axis

```rust
pub struct NetworkPolicy {
    pub enabled: TernaryState,
    pub domain_rules: Vec<DomainAccessRule>,
    pub local_binding: TernaryState,
}
```

### MCP axis

```rust
pub struct McpAccessPolicy {
    pub server_rules: Vec<McpServerRule>,
    pub tool_rules: Vec<McpToolRule>,
}
```

### Agents axis

```rust
pub struct AgentPolicy {
    pub subagent_rules: Vec<SubagentRule>,
    pub mode_switch_rules: Vec<ModeSwitchRule>,
}
```

### Runtime axis

```rust
pub struct RuntimePolicy {
    pub approval_mode: Option<CanonicalApprovalMode>,
    pub sandbox_mode: Option<CanonicalSandboxMode>,
    pub can_bypass_permissions: TernaryState,
    pub project_trust_required: TernaryState,
}
```

## Decision Vocabulary

The canonical rule effects should use a common decision vocabulary:

```rust
pub enum PolicyEffect {
    Allow,
    Ask,
    Deny,
}
```

Query outputs should add uncertainty and fidelity instead of inventing more effect variants.

```rust
pub enum PolicyCertainty {
    Exact,
    BestEffort,
    Unknown,
}

pub enum MappingFidelity {
    Exact,
    Narrowed,
    Broadened,
    Approximate,
}

pub enum TernaryState {
    Yes,
    No,
    Unknown,
}
```

## Provenance Model

Every canonical rule should retain where it came from:

```rust
pub struct CanonicalRuleProvenance {
    pub source_id: String,
    pub native_reference: Option<String>,
    pub fidelity: MappingFidelity,
    pub note: Option<String>,
}
```

This is critical for both trust and debugging.

## Snapshot Objects

The engine should return queryable snapshots rather than requiring callers to pass all context into every query.

```rust
pub struct ConfiguredPolicySnapshot {
    pub provider: Provider,
    pub native: NativeEffectivePolicy,
    pub canonical: CanonicalPolicy,
}

pub struct EffectivePolicySnapshot {
    pub provider: Provider,
    pub native: NativeEffectivePolicy,
    pub canonical: CanonicalPolicy,
    pub cli: ProviderCliOverrides,
}
```

Each snapshot should expose convenience query methods:

```rust
impl ConfiguredPolicySnapshot {
    pub fn can_read<P: AsRef<Path>>(&self, path: P) -> QueryResult;
    pub fn can_write<P: AsRef<Path>>(&self, path: P) -> QueryResult;
    pub fn can_execute(&self, query: &CommandQuery) -> QueryResult;
    pub fn can_access_domain(&self, domain: &str) -> QueryResult;
    pub fn can_use_mcp_server(&self, server: &str) -> QueryResult;
    pub fn can_use_mcp_tool(&self, server: &str, tool: &str) -> QueryResult;
    pub fn can_spawn_subagent(&self, name: Option<&str>) -> QueryResult;
    pub fn can_modify_own_config(&self) -> QueryResult;
    pub fn query(&self, query: &PolicyQuery) -> QueryResult;
}
```

The same API should exist on `EffectivePolicySnapshot`.

## Query Model

The query layer should support both convenience helpers and one generic query entrypoint.

```rust
pub enum PolicyQuery {
    ReadPath(PathQuery),
    WritePath(PathQuery),
    TraversePath(PathQuery),
    ExecuteCommand(CommandQuery),
    AccessDomain(DomainQuery),
    UseMcpServer { server: String },
    UseMcpTool { server: String, tool: String },
    SpawnSubagent { name: Option<String> },
    SwitchMode { target: Option<String> },
    ModifyProviderConfig,
}
```

### Path query

```rust
pub struct PathQuery {
    pub path: PathBuf,
    pub path_kind: PathKindHint,
}

pub enum PathKindHint {
    File,
    Directory,
    Unknown,
}
```

The engine should canonicalize paths relative to `cwd` where possible and classify them into:

- workspace
- external to workspace
- home
- temp
- provider config path
- system path

### Command query

```rust
pub struct CommandQuery {
    pub raw: String,
    pub executable: Option<String>,
    pub argv: Vec<String>,
}
```

Recommended behavior:

- callers may provide just `raw`
- the engine attempts shell tokenization best-effort
- backends may match on raw string, executable, prefix, or argv depending on provider semantics

## Query Result Model

`QueryResult` is the core product callers should use.

```rust
pub struct QueryResult {
    pub effect: Option<PolicyEffect>,
    pub certainty: PolicyCertainty,
    pub stability: QueryStability,
    pub matched_rules: Vec<MatchedRule>,
    pub explanation: PolicyExplanation,
    pub warnings: Vec<PolicyWarning>,
}

pub enum QueryStability {
    Stable,
    MayChangeWithCli,
    MayChangeAtRuntime,
    Unknown,
}
```

### Why `Option<PolicyEffect>` is useful

Some providers or situations genuinely cannot produce a trustworthy allow/ask/deny answer for a given query. Examples:

- the provider has no explicit policy on that axis
- the config is conditional on trust state we do not know
- CLI input is missing and the answer depends on CLI flags
- the provider model is weaker than the canonical query being asked

Returning `None` with explanation is better than pretending certainty.

## Explanation Model

Explainability should exist as structured data, not just prose strings.

```rust
pub struct PolicyExplanation {
    pub summary: String,
    pub reasons: Vec<ExplanationReason>,
}

pub struct ExplanationReason {
    pub source_id: String,
    pub native_reference: Option<String>,
    pub message: String,
    pub fidelity: MappingFidelity,
}
```

Recommended behavior:

- `summary` is human-readable and short
- `reasons` are machine-friendly and can be rendered in CLI or UI later

## Rule Matching Semantics

The engine needs shared matching helpers, but final precedence remains backend-defined.

Shared matchers should support:

- path equality
- path prefix/subtree
- gitignore-style globs
- regex where the provider uses regex
- shell prefix match
- exact command match
- domain exact and wildcard match

Backends should own their provider's actual precedence rules. The common matcher library only provides primitives.

Examples:

- Claude: `deny > ask > allow`
- Codex: native precedence is a composition of sandbox policy, approval policy, rules, and trust
- Roo: mode availability, approval category, protected-file rules, and command allow/deny are layered differently

## Configured vs Effective Resolution Pipeline

Configured snapshot resolution:

1. discover provider filesystem sources
2. load typed native layers
3. compose native policy without CLI overrides
4. canonicalize to configured canonical policy
5. annotate warnings and provenance

Effective snapshot resolution:

1. perform configured snapshot steps
2. parse provider CLI overrides from argv or typed overrides
3. add ephemeral CLI/env layers
4. compose native policy with those overrides
5. canonicalize to effective canonical policy
6. annotate which answers are stable vs CLI-sensitive

## Mutation Design

The mutation layer should model proposed intent, not provider-native patch details.

### `PolicyChange`

```rust
pub struct PolicyChange {
    pub operations: Vec<PolicyChangeOp>,
    pub target: PolicyChangeTarget,
    pub persistence: PolicyPersistence,
}
```

```rust
pub enum PolicyChangeTarget {
    Auto,
    UserConfig,
    RepoConfig,
    LocalOverride,
}

pub enum PolicyPersistence {
    Persistent,
    OneShot,
}
```

### Operations

The change operations should cover the recurring cross-provider security actions.

```rust
pub enum PolicyChangeOp {
    GrantRead(PathBuf),
    GrantWrite(PathBuf),
    DenyRead(PathBuf),
    DenyWrite(PathBuf),
    RequireApprovalForCommand(CommandPattern),
    AllowCommand(CommandPattern),
    DenyCommand(CommandPattern),
    AllowDomain(String),
    DenyDomain(String),
    AllowMcpServer(String),
    DenyMcpServer(String),
    AllowMcpTool { server: String, tool: String },
    DenyMcpTool { server: String, tool: String },
    AllowSubagent(Option<String>),
    DenySubagent(Option<String>),
    SetApprovalMode(CanonicalApprovalMode),
    SetSandboxMode(CanonicalSandboxMode),
}
```

Not every provider will support every operation exactly. That is expected.

## Mutation plan output

Planning should produce a structured result with previews and fidelity metadata.

```rust
pub struct PolicyMutationPlan {
    pub provider: Provider,
    pub persistent_plan: Option<PersistentMutationPlan>,
    pub one_shot_plan: Option<OneShotMutationPlan>,
    pub warnings: Vec<PolicyWarning>,
    pub supported: bool,
}

pub struct PersistentMutationPlan {
    pub edits: Vec<ConfigEditPlan>,
    pub fidelity: MappingFidelity,
}

pub struct OneShotMutationPlan {
    pub argv: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub fidelity: MappingFidelity,
}

pub struct ConfigEditPlan {
    pub source_id: String,
    pub path: PathBuf,
    pub description: String,
    pub before_preview: Option<String>,
    pub after_preview: String,
}
```

Recommended behavior:

- if persistent change is unsupported, `persistent_plan` is `None` with warnings
- if one-shot CLI mapping is unsupported, `one_shot_plan` is `None`
- if a mapping broadens or narrows semantics, fidelity must reflect that

## Mutation execution

Execution should be explicit and separate:

```rust
impl PolicyMutationPlan {
    pub fn apply(&self) -> Result<AppliedMutationReport>;
}
```

The engine should only apply plans it generated itself. Callers should not be building file patch instructions manually.

## Provider-Specific Mapping Expectations

The design should acknowledge provider differences up front.

### Claude

Expected fidelity: high

Strengths:

- explicit permission arrays
- explicit sandbox config
- explicit managed vs local layering
- subagent-related permissions

Likely strong support for:

- read/write path queries
- command queries
- network/domain queries
- MCP queries
- approval mode and sandbox mode mutations

### Codex

Expected fidelity: medium-high

Complexities:

- permission truth spans sandbox mode, approval policy, execution rules, named permission profiles, trust state, and MCP controls
- execution rules apply only to shell commands
- MCP is not governed by the shell sandbox

Important design consequence:

Codex backend must preserve axis separation internally and expose warnings when a canonical answer crosses multiple native enforcement mechanisms.

### Gemini

Expected fidelity: high

Strengths:

- explicit policy engine
- explicit approval modes
- sandbox config
- rule-based matching

Likely strong support for both configured and effective snapshots.

### OpenCode

Expected fidelity: high

Strengths:

- explicit permission config
- clear ask/allow/deny model
- CLI/runtime overrides

Likely straightforward query and mutation support.

### Qwen

Expected fidelity: medium-high

Strengths:

- tool-based permissions similar to Gemini

Caveat:

- some schema and behavior may still be in motion

Backend should emit warnings for unstable or under-documented areas rather than pretending certainty.

### Roo

Expected fidelity: medium

Complexities:

- tool groups
- action categories
- mode restrictions
- protected files
- command allow/deny
- MCP layers

Backend should answer canonical queries, but query explanations must frequently mention that the answer is composed from multiple policy layers.

### Goose and Kimi

Expected fidelity: partial in early phases

These should be queryable where clear native policy exists, but the initial implementation can ship with lower confidence and more `Unknown` or `BestEffort` results rather than fake parity.

## Error Model

The subsystem should add explicit error variants rather than overloading generic config errors.

Recommended categories:

- provider backend missing
- source discovery failure
- native config parse failure
- CLI override parse failure
- unsupported query
- unsupported mutation
- mutation plan apply failure
- ambiguity due to missing trust or missing context

Recommended shape:

```rust
pub enum PolicyEngineError {
    BackendUnavailable(Provider),
    SourceDiscovery(String),
    NativeParse { source_id: String, message: String },
    CliParse { provider: Provider, message: String },
    UnsupportedQuery { provider: Provider, query: String },
    UnsupportedMutation { provider: Provider, op: String },
    ApplyFailed { path: PathBuf, message: String },
    AmbiguousContext(String),
}
```

## Caching

The first implementation does not need a complex cache, but the API should not preclude one.

Recommended v1 behavior:

- no cross-process cache
- in-memory engine instance may cache source discovery and parsed snapshots keyed by:
  - provider
  - relevant root paths
  - source mtimes or content hashes
  - CLI override hash for effective snapshots

`PolicyEngine::new()` should be cheap enough that callers can also instantiate it per operation when simplicity matters.

## CLI Integration Boundary

This design is library-first. It should not depend on a new CLI command existing.

That said, the model should support future commands cleanly:

- `claudine permissions show <provider>`
- `claudine permissions query <provider> --read <path>`
- `claudine permissions explain <provider> --bash "git push"`
- `claudine permissions change <provider> ...`

The existence of those future commands is a good design check for the library API.

## Testing Strategy

The test plan should be layered.

### 1. Backend fixture tests

For each provider:

- discover source order correctly
- parse representative config fixtures
- compose precedence correctly
- parse CLI overrides correctly

### 2. Canonicalization tests

Ensure native policy becomes canonical policy with correct:

- effects
- provenance
- fidelity
- warnings

### 3. Query contract tests

Given a snapshot, verify queries like:

- read path in workspace
- write external path
- dangerous shell command
- allowed domain
- denied MCP tool

### 4. Mutation planning tests

Verify:

- persistent edits target the expected file
- one-shot args match provider semantics
- unsupported changes degrade explicitly

### 5. Round-trip tests

Where feasible:

1. load native config
2. plan mutation
3. apply mutation to temp fixture
4. reload policy
5. verify the intended query outcome changed

### 6. Explanation snapshot tests

Query explanations should be snapshot-tested because they are part of the trust model.

## Recommended Rollout Plan

### Phase 1: Scaffolding

Create:

- module layout
- core types
- backend trait
- empty engine registry
- query and mutation plan structs

No provider support yet beyond a compile-ready skeleton.

### Phase 2: High-value providers

Implement:

- Claude
- Codex
- Gemini
- OpenCode

These cover the main distinct permission models and give the engine practical value early.

### Phase 3: Query completeness

Land:

- path queries
- command queries
- domain queries
- MCP queries
- subagent queries
- explainability

### Phase 4: Mutation planning

Implement persistent and one-shot planning for the same four providers.

Execution can come after planning if needed, but planning support should be complete first.

### Phase 5: Additional providers

Add:

- Qwen
- Roo
- Goose
- Kimi

with explicit fidelity and limitation reporting.

### Phase 6: Adoption

At this point other Claudine systems can start consuming snapshots and query results. That is intentionally outside this design, but the engine should be ready for it.

## Open Design Decisions

These do not block scaffolding, but they should be resolved before deeper provider work begins.

### 1. Config write targeting policy

For persistent changes, what should `Auto` mean when both user and repo config exist?

Recommended default:

- prefer repo config when the queried policy came from repo scope and the repo is trusted
- otherwise prefer user config

### 2. Human-readable explanation formatting

The structured explanation model should be stable first. Pretty CLI rendering can be decided later.

### 3. Trust-state discovery ownership

Some providers expose trust through their own config. Others infer it elsewhere. The backend should own provider-specific trust discovery, but `PolicyContext` should still allow explicit override for tests and wrappers.

## Final Recommendation

Implement `PolicyEngine` as a new `claudine::permissions` subsystem with:

- typed provider backends
- native and canonical policy layers
- configured and effective snapshots
- rich query results with explanation and provenance
- explicit mutation planning for persistent and one-shot changes

The most important design constraint is to preserve provider fidelity while still giving Claudine a uniform surface.

If that constraint is maintained, `PolicyEngine` will be a strong foundation for future safety and orchestration work without becoming another opaque policy box.
