---
created: 2026-07-13
status: draft
reviewed: false
depends_on:
    - ../_completed/2026-05-12-lifecycle/spec.md
    - ../_completed/2026-06-26-positional-and-key-value/spec.md
---

# Transient Frontmatter for Proxy Handoffs (`with:`)

## Introduction

Claudine's lifecycle `proxy` action hands execution to another prompt document,
but it cannot currently attach route-specific frontmatter to that handoff. The
only frontmatter inputs guaranteed to survive proxy re-materialization are the
original caller-supplied compose overrides. Values computed by the routing
document are not carried into the target.

That limitation encourages an intuitive but invalid action shape:

```yaml
initialize:
    stack:
        - when: spec
          action:
              - proxy: prompts/_implement/implement-plan.md
                phase: phase
                total_phases: total_phases
                plan: plan
```

The lifecycle grammar correctly rejects this as an ambiguous multi-key action
object: positional form must contain exactly one verb key, while key/value form
must contain an explicit `action:` discriminator. Rewriting it as key/value form
does not solve the underlying problem because `proxy` currently accepts only
`target`; the other keys become unknown parameters.

Using `set_frontmatter` or `merge_frontmatter` before `proxy` is not an
appropriate substitute. Those are persistent Darkmatter side effects: they
modify the target Markdown file, can change its hash, create cross-run coupling,
and introduce races when multiple compositions use the same prompt.

This feature adds an optional `with:` mapping to key/value `proxy` actions. It
provides a typed, transient frontmatter overlay for the target prompt without
modifying either source file.

```yaml
initialize:
    stack:
        - when: spec
          action:
              - action: proxy
                target: prompts/_implement/implement-plan.md
                with:
                    phase: "{{ phase }}"
                    total_phases: "{{ total_phases }}"
                    plan: "{{ plan }}"
```

## Goals

- Let a proxying document pass route-specific data to the immediate target.
- Keep the handoff transient: no Markdown file writes and no hash changes.
- Evaluate `with:` through the existing lifecycle/Darkmatter late-binding path,
  including typed whole-value interpolation.
- Apply the overlay before the target's composition and schema validation so it
  can satisfy required target properties and participate in target expressions.
- Preserve the caller's explicit compose overrides as the highest-precedence
  input layer.
- Give `with:` a clear lifetime across retries, resumes, loops, and proxy chains.
- Carry the overlay through every supported proxy route, not only
  `initialize`.

## Non-goals

- Changing positional `proxy: target.md` syntax.
- Turning `set_frontmatter` or `merge_frontmatter` into in-memory operations.
- Persisting proxy inputs into the target document.
- Adding general nested-map parameters to every lifecycle action. `with:` is a
  typed configuration field owned specifically by `proxy`.
- Adding a new expression evaluator or interpolation grammar.
- Changing proxy target resolution, cycle detection, hop limits, or lifecycle
  placement rules.
- Making a source document's complete effective frontmatter implicitly inherit
  into every target. Handoff data remains explicit.

## Authoring Contract

### Syntax

`with:` is accepted only on key/value `proxy` actions:

```yaml
- action: proxy
  target: "{{ next_prompt }}"
  with:
      spec: "{{ spec }}"
      iteration: "{{ iteration }}"
      dry_run: "{{ false }}"
```

The canonical fields are:

| Field | Required | Type | Meaning |
|---|---:|---|---|
| `action` | yes | literal `proxy` | Selects key/value lifecycle action form |
| `target` | yes | lifecycle string | Existing proxy target reference |
| `with` | no | mapping | Transient top-level frontmatter overlay for the immediate target |
| `no_error` | no | boolean | Existing dispatch-error behavior; evaluation errors remain unsuppressed |

`with: {}` is valid and equivalent to omitting `with:`. A non-mapping value is
a typed frontmatter error. In v1, an entire mapping cannot be supplied as
`with: "{{ payload }}"`; authors write the mapping explicitly and may inject
typed object or array values at individual keys.

Positional form remains intentionally compact and unchanged:

```yaml
- proxy: prompts/next.md
```

An author who needs `with:` must opt into key/value form. A multi-key object
without `action: proxy` remains ambiguous and must continue to fail with the
existing positional-versus-key/value guidance.

### Value semantics

The lifecycle action rule remains authoritative:

> Values are literal by default. Use `{{ ... }}` to inject a variable or
> expression.

`with:` keys are static YAML strings and are never interpolated. Values are
resolved recursively at event time:

- a mixed string such as `"phase-{{ phase }}"` resolves to a string;
- a string consisting of exactly one interpolation span preserves the resolved
  type (`bool`, number, string, array, object, or null);
- YAML numbers, booleans, arrays, objects, and nulls retain their authored
  types;
- strings inside nested arrays and objects follow the same interpolation rule;
- arrays and objects are data values, not positional action arguments.

Example:

```yaml
- action: proxy
  target: prompts/next.md
  with:
      attempt: "{{ iteration }}"       # number when iteration is numeric
      ready: "{{ true }}"              # boolean
      label: "phase-{{ iteration }}"   # string
      files: "{{ changed_files }}"     # typed array
      metadata:
          source: router
          area: "{{ ctx.area }}"
```

Evaluation uses the same event-time Darkmatter subtree composition used by the
rest of the lifecycle surface. It must not introduce a bespoke Claudine
interpolator. The overlay resolves against the source document's live
frontmatter plus the globals valid for the event (`err`, `timing`, and
`current`, where applicable).

Resolution is just-in-time. A preceding action in the same stack that changes
the source document's live frontmatter is visible to `with:`. Unknown roots,
malformed expressions, unknown functions, or out-of-scope late-binding globals
fail closed as lifecycle evaluation errors.

### Atomic handoff

The target and the complete `with:` mapping are evaluated before proxy state is
changed. Target resolution, cycle/hop validation, and overlay evaluation must
all succeed before the active prompt is swapped.

If any step fails:

- no partial overlay is installed;
- the source remains the active document;
- the target is not initialized or composed;
- the failure follows the existing lifecycle evaluation/setup-error route;
- `no_error` does not suppress expression evaluation failures.

## Target Composition and Precedence

The resolved `with:` mapping is merged into the target's authored frontmatter
before Darkmatter composes the target. It therefore participates in:

- target frontmatter interpolation and computed properties;
- SimplifiedSchema validation, coercion, defaults, and eager file handling;
- target lifecycle parsing and target-side `initialize` conditions;
- shell-command discovery and approval for the fresh target run;
- the prompt body delivered to the provider.

Precedence, from lowest to highest, is:

1. target-authored frontmatter;
2. the immediate proxy's `with:` mapping;
3. caller-supplied compose overrides (`key=value` / `--set`) retained in
   `RematerializeInputs`.

The original caller remains authoritative. A router cannot silently replace an
explicit caller value with `with:`. Target computed properties and schema
normalization run after these input layers are assembled.

The overlay is shallow at the top level, matching Claudine's existing
`merge_frontmatter_overlay` behavior:

- a scalar or array replaces the target-authored value;
- an object replaces the target-authored object at that key rather than deep
  merging it;
- a null value removes that target-authored top-level property before
  composition.

Caller overrides may subsequently restore or replace any of those keys.

File-valued properties are data at handoff time. Their validation and
normalization use the target's normal Darkmatter composition context and the
carried file-reference fallback context; `with:` does not add a second path
resolver.

## Lifetime and Proxy Chains

The overlay is scoped to the immediate target document:

- it survives re-materialization of that target for retry, resume, and loop
  iterations;
- it remains available to all lifecycle events and body composition for that
  target;
- it is never written to disk;
- it is discarded when that target proxies to another document.

A downstream proxy receives only its own `with:` mapping plus the original
caller overrides. It does not implicitly inherit a previous hop's overlay. To
forward a value, the intermediate prompt does so explicitly from its current
effective frontmatter:

```yaml
- action: proxy
  target: prompts/final.md
  with:
      spec: "{{ spec }}"
```

Omitting `with:` on a downstream proxy therefore clears the active proxy
overlay for the new target. This prevents route-local values from leaking
through a long proxy chain while keeping caller inputs globally stable.

Cycle detection and `MAX_PROXY_HOPS` are based only on resolved document paths
and are unchanged. An overlay does not create a distinct identity for the same
target path.

## Runtime Model

The proxy payload must remain typed through the full provider-neutral control
pipeline. The existing target-only variants are insufficient because dropping
the overlay at any seam would make behavior depend on which lifecycle event
initiated the proxy.

The design should carry a single resolved handoff value (name illustrative):

```rust
pub struct ProxyHandoff {
    pub target: String,
    pub with: IndexMap<String, serde_json::Value>,
}
```

The parse-time lifecycle representation retains the unevaluated `with:`
template mapping. Event execution resolves it into a typed `ProxyHandoff`, and
the same value flows through `StackControl`, `ControlDispatch`, initialize-time
pre-harness routing, the harness recovery dispatcher, and the library loop
engine.

At the harness boundary, `HarnessPromptState` needs an active prompt-overlay
layer with document scope. Its behavior is:

- initialized empty for the source document;
- replaced atomically by the resolved handoff overlay after target resolution
  and cycle validation;
- retained for re-materializations of the active target;
- replaced (including by empty) on the next successful proxy handoff.

The existing `overlay` field may own this responsibility if its document scope
is made explicit and every assignment site follows the replacement rule. A
separate typed field is preferable if `overlay` also has or gains another
independent source; the implementation must not conflate proxy lifetime with a
longer-lived caller overlay.

All proxy entry points must carry the handoff:

- `initialize` proxying performed before `run_harness_loop`;
- a target document proxying again from its own `initialize`;
- provider-loop recovery proxying from `start`, `success`, `failure`, or
  `finalize` where runtime support exists;
- library loop-engine proxy routing;
- setup/blocked paths that already support or explicitly reject proxy recovery.

No path may silently retain the target while dropping `with:`. Runtime surfaces
that cannot execute proxy today keep their existing typed unsupported behavior,
but parsing and evaluation remain consistent.

## Security and Side Effects

`with:` itself is data-only and performs no filesystem mutation. It does not
invoke Darkmatter's effect engine, create a temporary Markdown file, or trigger
frontmatter hashing.

Because the overlay is installed before the target's fresh composition and
pre-flight, any executable target configuration it influences must pass the
same schema and shell-approval controls as target-authored frontmatter. The
pre-flight scanner and the executed target must consume the same resolved
overlay; approval of one materialization followed by execution of another is
not acceptable.

Normal status output may report that a proxy handoff includes an overlay, but
must not print overlay values. Tracing may record property names and counts;
values can contain secrets and must follow existing redaction policy.

## Errors and Diagnostics

Add typed, source-aware diagnostics for:

- `with:` being anything other than a mapping;
- an invalid/interpolation-failing value at a specific `with.<key>` path;
- a proxy-only `with:` parameter used on another action;
- target schema validation failure after the overlay is applied.

Errors rooted in authored frontmatter must use the existing
`FrontmatterExcerpt` rendering path and highlight the most specific locatable
line. Diagnostics should identify the lifecycle property (`initialize`,
`failure`, and so on), the `proxy` action, and the failing `with` key without
dumping unrelated overlay values.

The current ambiguous-action diagnostic remains correct for this invalid form:

```yaml
- proxy: prompts/next.md
  with:
      spec: "{{ spec }}"
```

Its actionable rewrite should point to:

```yaml
- action: proxy
  target: prompts/next.md
  with:
      spec: "{{ spec }}"
```

## Backward Compatibility

This is additive:

- positional `proxy: target.md` is unchanged;
- key/value `{ action: proxy, target: target.md }` is unchanged;
- caller overrides continue to survive all proxy handoffs;
- proxy resolution, control placement, cycle protection, and hop limits are
  unchanged;
- action parameters other than `proxy.with` continue to reject direct mapping
  values.

No migration is required for valid existing prompt documents. Documents using
the previously invalid multi-key positional shape gain a clear supported
rewrite.

## Documentation

Update the lifecycle topic, composition topic, and Claudine skill to cover:

- key/value `proxy.with` syntax;
- literal and typed interpolation behavior;
- precedence against target-authored values and caller overrides;
- immediate-target lifetime and explicit forwarding across proxy chains;
- the distinction between transient `with:` and persistent
  `set_frontmatter`/`merge_frontmatter` effects.

Examples must not imply that `phase: phase` evaluates a variable; expression
values use `phase: "{{ phase }}"`.

## Acceptance Criteria

1. Key/value `proxy` accepts an optional mapping-valued `with:` field; omitted
   and empty mappings preserve existing behavior.
2. Positional `proxy: target.md` remains valid and unchanged; positional proxy
   plus sibling `with:` remains an ambiguous-action error with an actionable
   key/value rewrite.
3. `with:` leaf strings resolve through lifecycle DM2 at event time against the
   source's live frontmatter and valid lifecycle globals.
4. Whole-value interpolation preserves typed bool, number, null, array, and
   object values; nested arrays/objects recursively resolve string leaves.
5. A malformed expression, unknown root/function, or illegal late-binding
   reference aborts the handoff atomically before the active prompt changes.
6. The target observes the overlay during its schema validation, computed
   frontmatter interpolation, `initialize` event, shell pre-flight, and body
   composition.
7. Precedence is target-authored frontmatter < `proxy.with` < caller compose
   overrides. Null removes a target-authored key at the proxy layer, and the
   overlay otherwise replaces top-level values shallowly.
8. A target schema's required property can be satisfied by `with:`; an invalid
   overlaid value produces the normal target schema error with target/source
   context rather than invoking the provider.
9. The overlay survives retry, resume, and loop re-materialization of the
   immediate target without any file mutation.
10. A subsequent proxy replaces the active overlay. Previous-hop values do not
    leak unless the intermediate prompt explicitly forwards them.
11. Initialize-time, target-initialize, recovery, and loop-engine proxy paths
    preserve the same typed handoff; no supported route drops `with:`.
12. Failed target resolution, cycle/hop rejection, or overlay evaluation leaves
    the current document and overlay unchanged.
13. Neither source nor target Markdown bytes or Darkmatter hashes change solely
    because `with:` is used.
14. Shell approval inspects the same overlaid target configuration that is later
    executed.
15. User-facing status and errors do not disclose unrelated overlay values.
16. Lifecycle/composition docs and the Claudine skill describe the new contract
    and clearly distinguish it from persistent frontmatter effects.

## Test Strategy

### L1 — library and parser

- Parse key/value proxy with omitted, empty, scalar-valued, and nested `with:`
  entries; reject non-mapping `with:` values.
- Confirm `with:` remains proxy-specific and other actions still reject unknown
  or direct-map parameters.
- Prove literal strings stay literal, mixed interpolation produces strings, and
  whole-value interpolation preserves every supported JSON type.
- Prove nested map/array leaf interpolation and a precise nested-path evaluation
  error.
- Verify a preceding same-stack live-frontmatter mutation is visible to
  `with:`.
- Carry the typed overlay through `LifecycleControlAction` → `StackControl` →
  `ControlDispatch` without loss.
- Verify `decide_control` remains event-agnostic and proxy cycle/hop decisions
  remain path-only.
- Test shallow replacement, null removal, and precedence helpers independently.
- Test document-scoped overlay replacement: retain for retry/resume/loop of one
  target, clear or replace on the next proxy.

Place parser/executor tests in the existing sibling lifecycle test modules;
keep inline tests only where the production/test size remains within the
repository's test-placement thresholds.

### L2 — real composition harness

- An `initialize` router passes `spec` through `with:` to a target whose schema
  requires it; the target composes and runs without modifying either file.
- A caller override for the same property wins over `with:`.
- A target computed property and body interpolation both observe the overlaid
  typed value.
- A target `initialize` condition observes `with:` and can perform its existing
  `skip`, `error`, or proxy behavior.
- A failure/finalize proxy uses `err.*` inside `with:` and the target receives
  the resolved typed data.
- A proxy target retries or loops and receives the same overlay on every
  re-materialization.
- A three-document chain proves hop A's overlay does not leak to C unless B
  forwards it explicitly.
- A cycle, missing target, invalid `with:` expression, and target schema failure
  each exit non-zero without invoking the target provider.
- A target shell command derived from `with:` is audited and the approved bytes
  are the executed bytes.
- Run the real-terminal cases through the existing lifecycle-control harness,
  using platform-neutral temporary paths and assertions suitable for macOS,
  Windows, and Linux.

### Regression

- Existing proxy parser, initialize handoff, recovery, target lifecycle,
  cycle/hop-limit, and caller-rematerialization tests remain green unchanged.
- `just test` and `just test-l2` pass in the `claudine` package area.
- `just lint` passes without adding new provider-dispatch exceptions.

## Out of Scope Follow-ups

- A compact positional payload syntax such as `proxy: [target, payload]`.
- Passing a complete mapping via `with: "{{ payload }}"`.
- Deep-merge controls or per-key merge strategies.
- Persisting or serializing proxy overlays for deferred execution.
- A general call/return value model between prompt documents.
