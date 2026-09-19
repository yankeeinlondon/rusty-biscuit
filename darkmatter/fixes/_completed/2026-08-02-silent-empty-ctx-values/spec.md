---
status: draft
created: 2026-08-02
area: darkmatter
packages:
  - darkmatter
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-16
implemented: true
implemented_by: claude/default
review_iterations: 2
---

# A missing runtime value must not render as nothing

## Summary

Darkmatter documents can interpolate request-time facts through the `ctx`
namespace, for example `{{ ctx.today }}`, `{{ ctx.branch }}`, and
`{{ ctx.repo_root }}`. Runtime context is captured selectively because several
capture groups perform comparatively expensive host or repository discovery.

The defect is that a known `ctx.*` variable can currently reach composition
without its owning group in the request snapshot. The lookup then returns no
value, normal unresolved-value behavior converts that result to an empty
string, and composition succeeds without explaining that content was lost.

This specification makes the boundary explicit:

- a known variable from an uncaptured group is a hard composition error;
- a captured variable whose semantic value is `null`, `""`, or an empty
  collection remains a valid value;
- an unknown `ctx.*` name remains owned by the existing unknown-context-variable
  diagnostic;
- a requested capture that could not obtain all of its evidence remains owned
  by the existing `PartialRuntimeCapture` diagnostic and typed empty/null
  projection; and
- every document in a transclusion graph is subject to the same rule before it
  evaluates its first expression.

The hard-error decision is intentional. An uncaptured group is an internal or
caller contract violation, not a fact about the machine, and emitting a
plausible-looking partial document is the least safe outcome.

## Background

### Runtime values and capture groups

A runtime value is a fact about the composition request rather than authored
document data. `ctx.branch`, for example, describes the captured repository
state; it is not frontmatter.

A capture group is a set of values populated from the same source. The current
authority is
[`capture/groups.rs`](../../lib/src/markdown/compose/context/capture/groups.rs),
not a count copied into this specification. At review time it defines eleven
groups: Invocation, DateTime, Git, Repo, FileChanges, Languages, Documents, OS,
Hardware, GPU, and Agent.

Capture cost varies substantially. Clock and invocation values are cheap,
while repository topology, document inventory, and working-tree status can
require broad discovery. Darkmatter therefore derives `ContextRequirements`
from document content and captures only the requested groups. A document that
uses no discovery-backed `ctx.*` value must continue to avoid that discovery.

### The request snapshot

Composition materializes a `ComposeContext` into `EffectiveState`. That state
is a request snapshot: downstream stages must not recapture the current
directory, environment, repository, clock, or host state independently.
`ComposeContext::capture_requirements()` now records which groups the snapshot
represents, so an absent group no longer needs to be inferred from missing map
keys.

An ambient `ComposeOptions::new()` starts with zero-discovery context and is
upgraded when the document is available. A caller-supplied context is
authoritative and is not silently augmented from ambient process state. The
distinction matters:

- Darkmatter-owned ambient context may be expanded from the request's retained
  evidence before a newly discovered source is evaluated.
- Caller-supplied context must either cover the source's requirements or fail
  with the error specified here. Falling back to ambient discovery would break
  determinism and the `ComposeOptions` authority boundary.

### Four different absence states

The implementation must not collapse these states:

| State | Meaning | Required outcome |
|---|---|---|
| Known key, owning group not captured | The request snapshot cannot answer the lookup | Hard composition error |
| Known key, group captured, key has `null`, `""`, `[]`, or `{}` | The observation was made and absence/emptiness is the value | Render with existing scalar/collection semantics; no missing-capture diagnostic |
| Known key, group captured, projection omitted the key | Darkmatter's catalog and capture projection disagree | Hard internal-invariant composition error |
| Unknown key | The name is not in `context_variable_descriptors()` | Existing unknown-context-variable diagnostic, including suggestion behavior |

`PartialRuntimeCapture` is not a fifth spelling of “uncaptured.” A supplied
evidence request can explicitly observe that evidence is unavailable. In that
case the group is requested and represented by its defined empty/null
projection, and the existing partial-capture warning explains the degradation.
This specification does not promote that established warning to an error.

## The defect

### Snapshot composition

`EffectiveStateBuilder::build` copies context values into a fixed `ctx` map.
If a known key's group was never captured, the key is absent. The generic
expression lookup reports `None`, and interpolation can stringify that as an
empty value. The map alone cannot distinguish a legitimate captured absence
from a missing group; the snapshot's `ContextRequirements` can.

### Standalone condition evaluation

`CtxLookup` in
[`expression/ctx.rs`](../../lib/src/markdown/compose/expression/ctx.rs) is used
by `evaluate_condition_against`. It owns an ambient, work-directory-anchored
request and captures a known group before looking up its first value. That
demand-driven implementation is valid because the API itself owns the context.

The two paths do not need identical mechanics. They do need one classification
contract: a cataloged key has one owning group; an authoritative snapshot must
prove that group was captured before lookup; and a capture projection must
contain every cataloged key it owns. Standalone evaluation may satisfy the
precondition by capturing the group, while document composition may use only
the request-scoped snapshot supplied by `ComposeOptions`.

### The 2026-08-02 regression

While removing an unrelated performance problem,
`ComposeOptions::new()` stopped capturing every group up front. The change
assumed needed groups would be captured on demand later. That was true for
standalone `CtxLookup`, but not for the fixed snapshot used by composition.

The observable result was:

```text
composing "repo_root={{ ctx.repo_root }}|os={{ ctx.os }}|today={{ ctx.today }}"

before:  repo_root=C:\Users\ken\rusty-biscuit|os=Windows|today=2026-08-02
after:   repo_root=|os=|today=2026-08-02
```

DateTime was still present; Repo and OS were not. All 7,510 tests passed because
most interpolation tests used a fully populated fixed test context and no test
exercised the affected constructor through rendered output.

The immediate repair upgraded ambient options for the root document and added
catalog-driven output comparison in `lib/tests/ambient_ctx_capture.rs`. That
closes the observed root-document regression, but it does not establish the
runtime invariant and does not cover a transcluded child that introduces the
first reference to a group.

## Requirements

### 1. Classify a `ctx.*` lookup from existing authorities

The authoritative variable catalog is `context_variable_descriptors()` and the
authoritative key-to-group mapping is `ContextGroup::for_key`. Do not add a
second list to `EffectiveState`, the evaluator, or tests.

For every known lookup, compare the owning group with
`ComposeContext::capture_requirements()` before reading the projected value.
The checked lookup must preserve a typed distinction between:

- value present, including typed null and empty values;
- owning group not captured;
- owning group captured but cataloged key missing; and
- unknown context key.

The generic `Option<Value>` result is not sufficient for this boundary. The
composition evaluator must use the checked result; retaining an unchecked
convenience accessor for compatibility must not leave any compose stage on the
silent path.

### 2. Missing capture is fatal

A known variable whose owning group is absent from the authoritative request
snapshot fails composition before replacement text is committed. The error
must include:

- the full variable path, such as `ctx.repo_root`;
- the owning group, such as `Repo`;
- the source document identity and authored span or line when available; and
- guidance that a programmatic caller should capture the document's
  `ContextRequirements` or construct options from an appropriately captured
  `ComposeContext`.

This error is not a `ComposeWarning`, is not suppressed by non-fail-fast mode,
and must not return a partially composed document. CLI composition exits
nonzero through the existing typed compose-error rendering path. Library
callers receive a typed cause rather than a string-only `Transform` error.

A captured group that omits one of its own cataloged keys is also fatal, but
its message identifies a Darkmatter capture-projection invariant failure rather
than blaming the caller's requirements.

### 3. Unknown names and captured absence retain their established behavior

This fix must not reinterpret all empty results as capture failures.

- `ctx.branch` outside a Git repository can legitimately be null/empty after a
  successful Git capture.
- `ctx.gpu` can legitimately report no device after a successful GPU capture.
- `ctx.oss` is an unknown name and continues through the existing
  unknown-context-variable diagnostic and suggestion path.
- user-authored `ctx` merge/override behavior remains governed by the existing
  merge policy; user values must not falsify the runtime snapshot's captured
  group evidence.

No truthiness, string length, or map membership heuristic may stand in for the
captured-group marker.

### 4. Enforce the invariant on every expression surface

The checked behavior applies anywhere composition can evaluate a `ctx.*`
reference, including:

- both frontmatter interpolation passes;
- schema-adjacent composed values;
- page-block and transclusion conditions;
- body interpolation;
- `$()` ternary conditions and branches; and
- recursively composed local or remote transclusions.

Only an expression that is actually evaluated can raise the runtime error.
Unchosen ternary branches, short-circuited operands, interpolation literals,
and content removed before its expression stage must not fail merely because a
text scanner saw `ctx.`. `ContextRequirements` may conservatively overcapture
for performance safety, but the fatal diagnostic is driven by checked runtime
lookup.

DMLS validation remains passive and must not capture host state. It may keep
diagnosing unknown catalog names, but it cannot claim that a request-scoped
runtime group will be missing.

### 5. Cover the full transclusion graph and preserve one request epoch

The root-only ambient upgrade is insufficient. A root can contain no host
context reference while a local or remote child contains `{{ ctx.os }}`.

Darkmatter-owned ambient composition must ensure each newly reachable source's
requirements against the same request-scoped context before that source's
first expression stage. Missing groups may be added only through the retained
request evidence and same-epoch extension mechanism; a child must never call a
fresh ambient capture. Once a group is present, all later reads in the graph
reuse that captured projection.

Caller-supplied contexts remain frozen authorities. If a child introduces a
group the caller did not supply, composition fails with the missing-capture
error rather than silently consulting the host.

Conditional and dynamic transclusions do not require eager evaluation of every
possible child. It is valid to extend ambient context when a child becomes
reachable, provided the extension happens before that child's expressions and
before any cache identity that can reuse the child's output is finalized.

### 6. Cache identity must follow context growth

The original draft claimed that cache staleness was impossible because output
and `context_hash` came from the same snapshot. That statement is correct for
one document whose requirements are known before hashing, but incomplete for a
transclusion graph: a child may introduce a group after the parent phase's
context identity was hoisted.

No compose-cache lookup or write may use a context hash captured before all
groups affecting that cached result are present. Implementations may satisfy
this by discovering the reachable requirement closure before hashing, or by
invalidating and recomputing the relevant phase/cache identity whenever the
request snapshot grows. In either design:

- the cached result's context hash includes every non-excluded context value
  that can affect that result or a dependency folded into it;
- a cache hit cannot bypass the missing-group check; and
- cache lookup and cache write use the same finalized identity.

Capturing groups mentioned in an unexecuted branch may continue to
over-invalidate. Narrowing from mentioned groups to consumed groups is a
separate performance feature.

> **Reader note:** This replaces the earlier blanket “cache cannot disagree”
> conclusion. The same-snapshot argument remains sound only after the
> transclusion requirement boundary and cache-key timing are included.

### 7. Diagnostics are stable and non-duplicative

An expression must not emit both “unknown context variable” and “group not
captured.” Unknown keys have no owning group and therefore cannot produce the
missing-group error.

Repeated evaluation/rescan must not duplicate a diagnostic for the same
authored expression. A transcluded source retains its own source identity, so
the same bad reference authored independently in two files remains two issues.
Normal fatal evaluation means composition will usually stop at the first
missing-capture error; no aggregate-error facility is required by this fix.

## Design decision: error rather than warning

Three policies were considered:

1. **Warn and render empty.** This preserves output but keeps a plausible,
   incomplete artifact and lets callers ignore the contract violation.
2. **Capture ambient state at the missing lookup.** This repairs output but
   violates caller-supplied authority, can mix observation epochs, and can make
   cache identity diverge from rendered content.
3. **Fail the authoritative snapshot; extend only Darkmatter-owned ambient
   context from retained request evidence.** This preserves deterministic
   caller injection, request consistency, and selective capture.

Option 3 is selected. It separates a program defect (missing capture) from a
legitimate environmental absence (captured null/empty) without making every
compose pay for every group.

## Alternatives rejected

### Always capture every group

This removes the missing-group state by removing selectivity. It restores the
broad discovery cost that demand-driven capture was introduced to avoid and
causes unrelated host/repository changes to invalidate documents that never
read those values.

### Make every snapshot lookup independently lazy

Uncoordinated laziness allows stages or children to observe different clock,
environment, repository, or working-tree epochs and makes cache-key timing
easy to get wrong. Same-request monotonic extension from retained evidence is
acceptable; fresh point-of-use ambient discovery is not.

### Rely on constructor convention and output-comparison tests

`capture_for_document` and catalog-driven output tests are useful defenses, but
they do not make violation impossible. The 2026-08-02 failure was itself a
constructor convention violation, and comparing against non-empty output
cannot distinguish every legitimate empty value.

## Verification

The fix is complete when automated tests prove all of the following:

1. A known `ctx.*` reference using a caller-supplied context without its owning
   group returns a typed fatal error naming the variable, group, and source.
2. The same error is observed through frontmatter interpolation, body
   interpolation, a condition, and `$()` branching; shared evaluator tests may
   cover additional surfaces that use the same checked path.
3. A captured nullable/empty value renders with existing semantics and emits no
   missing-capture diagnostic.
4. A requested group with unavailable supplied evidence retains
   `PartialRuntimeCapture` behavior and does not become “uncaptured.”
5. An unknown key produces only the existing unknown-context-variable
   diagnostic.
6. A captured group with an intentionally malformed projection that omits a
   cataloged owned key fails as an internal invariant violation.
7. Ambient composition resolves a group first referenced by a local
   transcluded child and by a nested child while preserving one captured value
   for every later read.
8. A caller-supplied minimal context fails when that same child first requests
   a missing group and performs no ambient fallback capture.
9. Cache regression tests change a child-only context value and prove the
   parent cannot reuse stale composed output. Cache hits also preserve the
   missing-group failure contract.
10. Removing the root ambient upgrade or the transclusion requirement handoff
    causes a named missing-group failure rather than an empty-output mismatch.
11. A document graph with no discovery-backed context references still captures
    no discovery-backed group, and the full Darkmatter L1 suite has no test
    above the established five-second slow-test threshold.
12. The standalone `evaluate_condition_against` path still captures only the
    known groups actually reached by evaluation and obeys the same
    unknown/projection-invariant classification.

Use the Darkmatter package recipes: `just test` and `just lint`. This change is
library behavior and does not require real-terminal or browser testing unless
implementation work independently changes one of those surfaces.

## Documentation updates

Implementation must update the compose/context documentation to state:

- `ComposeOptions` is the context authority;
- ambient context can grow only from retained evidence within one request;
- caller-supplied contexts never fall back to ambient discovery;
- captured null/empty differs from an uncaptured group; and
- cache identity is finalized after the requirements for its result are known.

Any comments claiming that `ComposeOptions::new()` values are captured “on
demand during expression evaluation” must be revised to describe the actual
root/transclusion handoff and fixed-snapshot behavior.

## Out of scope

- Reducing the cost of an individual capture group.
- Narrowing requirements from mentioned groups to groups consumed on the
  executed control-flow path.
- Changing the semantic null/empty projection of an existing context variable.
- Changing the severity or suggestion policy for unknown `ctx.*` names.
- Making DMLS perform runtime capture or other composition effects.
- General aggregation of multiple fatal compose errors.
