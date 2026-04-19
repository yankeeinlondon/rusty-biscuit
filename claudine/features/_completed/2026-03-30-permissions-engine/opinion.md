# Opinion: Permissions Engine Direction

## Summary

The current `ProtectService` is not a good foundation for the spec as written if we treat it as the primary home of provider permissions. It is a **runtime hook decision engine**. The spec describes something different: a **provider policy model** that can:

- read configured permissions from provider config files
- combine those with CLI/runtime overrides
- answer structured questions like `can_read`, `can_write`, and `can_bash`
- express the result in a provider-agnostic canonical form
- propose and apply policy changes back to provider-native config or one-shot CLI args

Those are adjacent concerns, but they are not the same concern.

My recommendation is:

1. Build a new `PolicyEngine` as a separate subsystem.
2. Keep `ProtectService` for now.
3. Refactor `ProtectService` to consume `PolicyEngine` results once the engine is stable.
4. Only after that, decide whether Protect should be slimmed down heavily or rewritten on top of the new engine.

That gives us the architecture the spec wants without discarding the useful parts of Protect prematurely.

## What Protect Is Today

The current implementation in [protect.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/services/protect.rs) is centered on:

- normalized event-time inputs via `ProtectInput::from_event_meta`
- heuristic risk inference from event text and command strings
- Claudine-owned policy config via `ProtectConfig`
- provider hook capability profiles via `ProviderProtectProfiles`
- hook-time outcomes like `Allow`, `AskThenAllowOrStop`, `StopCurrent`, `StopSession`, and MCP redaction

That is valuable, but it is fundamentally about:

- "an event is happening right now"
- "given this provider's hook surface, how much can Claudine intervene"
- "should we allow, ask, stop, or redact"

It is **not** fundamentally about:

- parsing provider-native permission config
- reconstructing effective permissions from config plus CLI args
- answering path- and command-level authorization queries
- emitting provider-native mutations

In other words: Protect is an enforcement and advisory layer. The spec is asking for a policy representation and mutation layer.

## Why The Gap Matters

The mismatch shows up in several places:

- `ProtectConfig` is Claudine-defined policy, not the provider's real policy.
- `ProtectInput` derives state heuristically from `EventMeta`; it does not resolve durable permission state from provider config.
- `ProviderProtectProfiles` describe hook and blocking capability, not provider permission schema.
- current decisions are coarse runtime outcomes, not reusable canonical permissions.
- there is no mutation model for provider-native config beyond Claudine's own config.

This is why Protect feels opaque: it is making safety judgments from partial runtime evidence, but it is not yet anchored to a first-class model of what the underlying agent has actually been configured to allow.

## Options Considered

## Option 1: Evolve Protect In Place

Add provider config parsing, CLI override handling, canonical policy structs, and mutation APIs directly into `ProtectService` and `protect.rs`.

### Pros

- fastest path in terms of file count
- reuses existing service entry point
- Protect already knows about providers, runtime modes, subagents, MCP, and sandboxes

### Cons

- conflates two different responsibilities: runtime intervention and policy modeling
- makes `protect.rs` even larger and harder to reason about
- query APIs like `can_read("/path")` become awkward inside an event-oriented service
- mutation APIs become coupled to hook-time behavior and forensic state
- harder to test because static policy resolution and live runtime decisioning share one surface
- the spec explicitly says this is not a replacement for Protect, which argues against collapsing everything into Protect

### Opinion

I would not choose this unless the goal is short-term expedience over design quality. It solves the immediate problem in the least clean way.

## Option 2: Build `PolicyEngine` Beside Protect, Then Make Protect A Consumer

Introduce a new permissions subsystem with provider-specific backends and a canonical provider-agnostic model. Keep Protect focused on runtime intervention, but allow it to consult `PolicyEngine` for authoritative permission facts.

### Pros

- matches the spec directly
- preserves a clean separation between static/effective policy and runtime safety decisions
- lets Protect become less heuristic over time
- keeps mutation logic isolated from hook evaluation
- supports CLI use cases beyond Protect, such as inspection, reporting, explainability, and future `claudine permissions ...` commands
- easier to test in layers:
  - provider config parsing
  - canonical policy resolution
  - CLI override composition
  - mutation planning/apply
  - Protect runtime decisions
- safest migration path because existing Protect behavior can keep working while backends are added incrementally

### Cons

- more upfront design work
- temporary duplication while Protect still contains some heuristic logic
- requires a deliberate boundary so `PolicyEngine` does not turn into another monolith

### Opinion

This is the strongest option. It matches both the spec and the current reality of the codebase.

## Option 3: Full Rewrite Protect Around The New Engine

Stop evolving the current Protect service and replace it with a new permissions-first architecture immediately.

### Pros

- clean conceptual reset
- avoids dragging forward heuristics and structural compromises
- makes the new engine the obvious source of truth from day one

### Cons

- high migration risk
- easy to regress current blocking, redaction, and completion-loop behavior
- provider hook capability handling would still need to be rebuilt
- large rewrite scope before the underlying provider backends are proven
- the current code already has useful pieces worth keeping, especially:
  - outcome mapping
  - provider hook capability profiles
  - dispatch integration points
  - MCP redaction machinery

### Opinion

I would not start here. A full rewrite may become reasonable later, but only after the new engine exists and has replaced enough of Protect's inference logic to justify collapsing the old service.

## Recommended Approach

Option 2 makes the most sense.

The key architectural decision should be:

**`PolicyEngine` owns permission truth. Protect owns runtime judgment and enforcement strategy.**

That division maps well to the code and to the provider landscape in the research:

- Claude, Gemini, Qwen, and OpenCode have tool-oriented permission models.
- Codex has a mixed model: sandbox policy, approval policy, execution rules, MCP allowances, and project trust.
- Roo has mode/tool-group/action-category/path-layer controls rather than one simple permission map.

A canonical policy layer is exactly what Claudine should normalize. Protect can then consume that normalized result and decide what to do when a live event arrives.

## What `PolicyEngine` Should Own

At minimum:

- configured policy from filesystem only
- effective policy from filesystem plus CLI/runtime overrides
- canonical policy representation
- structured query surface
- mutation planning and application
- explainability about why a decision was reached

I would model it around something like:

```rust
pub struct PolicyEngine {
    backends: HashMap<Provider, Box<dyn ProviderPolicyBackend>>,
}

pub trait ProviderPolicyBackend {
    fn load_configured_policy(&self, ctx: &PolicyContext) -> Result<CanonicalPolicy>;
    fn resolve_effective_policy(
        &self,
        ctx: &PolicyContext,
        cli: Option<&ProviderCliParams>,
    ) -> Result<EffectivePolicy>;
    fn plan_change(
        &self,
        ctx: &PolicyContext,
        change: &PolicyChange,
    ) -> Result<PolicyMutationPlan>;
    fn apply_change(&self, plan: &PolicyMutationPlan) -> Result<()>;
    fn one_time_args(&self, change: &PolicyChange) -> Result<Vec<String>>;
}
```

Core output types should distinguish:

- `CanonicalPolicy`: provider-agnostic normalized rules
- `EffectivePolicy`: canonical policy plus derived facts and provenance
- `PolicyExplanation`: why a query resolved the way it did
- `PolicyMutationPlan`: file edits and/or CLI args needed for a change

## Queries The Engine Should Support

The spec mentions read, write, and bash. I would expand that set immediately so the abstraction does not become too narrow:

- file read on path
- file write on path
- directory traversal outside workspace
- shell command execution
- shell command execution with escalation
- outbound network access
- network access to specific domains
- MCP server access
- MCP tool access
- subagent spawning
- mode/approval bypass state
- whether the agent can mutate its own config

Those are the recurring concepts across the provider research. If the canonical model only covers read/write/bash, it will fit Claude and Gemini reasonably well but will distort Codex and Roo.

## How Protect Should Change

Once `PolicyEngine` exists, Protect should stop trying to infer durable permission state from loose event metadata whenever authoritative policy data is available.

Protect should become a consumer that does things like:

- ask `PolicyEngine` whether the effective provider policy already allows or denies the attempted action
- detect gaps between what the provider enforces and what Claudine wants
- apply extra runtime rails only where provider-native policy is insufficient
- generate better explanations because it can cite actual policy sources rather than heuristics alone

That would make Protect both less opaque and more trustworthy.

## What I Would Not Do

I would not make `PolicyEngine` responsible for live hook outcomes like `StopSession` or `AskThenAllowOrStop`.

Those are not permission facts. They are runtime control decisions that depend on:

- hook availability
- interactivity
- current phase
- provider blockability
- redaction opportunities
- user experience tradeoffs

That belongs in Protect or a sibling runtime-guard layer, not in the policy engine.

## Migration Plan

### Phase 1

Create a new `permissions` module in `claudine/lib` and implement the canonical types and engine boundary.

### Phase 2

Implement provider backends for the highest-value providers first:

- Codex
- Claude
- Gemini
- OpenCode

Those give the best coverage of distinct policy models.

### Phase 3

Expose simple query APIs and an explain API. Even before mutation exists, this will make the system debuggable.

### Phase 4

Implement `PolicyChange` planning and `one_time` CLI arg generation provider by provider.

### Phase 5

Refactor Protect to consult `PolicyEngine` during `ProtectInput` evaluation. Remove heuristic branches that are replaced by authoritative policy data.

### Phase 6

Reassess whether `ProtectService` should remain as a thinner runtime layer or be rewritten around the new primitives.

## Final Recommendation

Do not rewrite Protect first.

Build `PolicyEngine` first, as a separate subsystem, and then move Protect onto it.

That approach gives Claudine:

- a trustworthy canonical permission model
- query and mutation APIs that actually match the spec
- a clearer explanation story for users
- a cleaner separation between "what is configured" and "what should happen right now"

If that work goes well, a later rewrite of Protect may make sense. But the rewrite should be the result of a successful permissions engine, not the prerequisite for one.
