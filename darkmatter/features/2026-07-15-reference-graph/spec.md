---
status: draft
reviewed: false
created: 2026-07-15
inputs:
  - ../../reviews/2026-07-12-perf/spec.md
  - ../../reviews/2026-07-12-perf/review-3.md
  - ../../lib/src/markdown/reference/types.rs
  - ../../lib/src/markdown/reference/graph.rs
  - ../../lib/src/markdown/reference/validate.rs
  - ../../lib/src/markdown/reference/file_tree/model.rs
  - ../../lib/src/markdown/reference/file_tree/mod.rs
  - ../../cli/src/commands/graph.rs
related:
  - ../2026-07-15-performance-followup
---

# Opaque Reference Graph

## Status

Draft. This specification records the approved compatibility ruling and the
complete invariant before implementation. It supersedes the partial
provenance-only approach preserved on `rescue/review3-terminated-agent`.

## Summary

`ReferenceGraph` is the result of reading a root Markdown document, extracting
its references, following configured transclusions, and recording the resulting
document tree. Building it can perform enough parsing, composition, file I/O,
and recursive traversal that callers need to reuse one graph for rendering and
validation.

The current public type is an unconstrained data structure:

```rust
pub struct ReferenceGraph {
    pub root: ReferenceGraphNode,
    pub nodes: Vec<ReferenceGraphNode>,
}
```

At the same time,
`Markdown::validate_references_with_graph(&ReferenceGraph, ...)` assumes that
the supplied graph was built from the same document, source, graph mode, and
options. The type cannot enforce that assumption. A caller can construct a
graph from unrelated parts, pair a graph with another document, supply a
transclusion-only graph where full references are required, or mutate a valid
graph after construction.

This feature turns `ReferenceGraph` into an opaque, immutable,
builder-produced artifact. Its root, descendants, and provenance become
private. Public access is read-only. Validation checks compact build provenance
before consuming a prebuilt graph.

## Decision

Darkmatter will keep public graph construction and prebuilt-graph validation,
but only builder-produced graphs are valid:

- `Markdown::reference_graph` and `Markdown::transclusion_graph` remain the
  public constructors.
- `Markdown::validate_references_with_graph` remains public.
- `ReferenceGraph` fields become private.
- Public read-only accessors replace direct field access.
- No public constructor, mutable accessor, parts conversion, `DerefMut`, or
  mutation method will bypass the invariant.
- Every graph carries private document, source, graph-mode, and options
  provenance.
- Provenance is compact and does not retain a full `Markdown`,
  `ComposeOptions`, context, cache, callback, preflight graph, or remote-fetch
  runtime.
- Graph-view JSON retains its existing shape and never exposes provenance.

This is an intentional Rust source compatibility break. There are no active
external users, so this is the lowest-cost point at which to establish the
correct public abstraction.

## Goals

1. Make it impossible for downstream crates to construct or mutate a
   `ReferenceGraph` outside Darkmatter's builders.
2. Reject prebuilt graphs produced from different represented document bytes,
   source identity, graph mode, or graph-affecting options.
3. Preserve the graph-reuse performance improvement from Finding 18.
4. Preserve public builder and validation method signatures.
5. Preserve `ReferenceGraph: Clone`; a clone carries the same immutable data
   and provenance.
6. Preserve graph lookup, count, Mermaid, DOT, file-tree, terminal, and JSON
   behavior.
7. Keep provenance compact and avoid extending the lifetime of stateful compose
   resources.
8. Make future additions to `ComposeOptions` fail closed until their identity
   behavior is considered.

## Non-Goals

- Making `ReferenceGraph` a persistent or cross-process cache format.
- Treating provenance hashes as a cryptographic or hostile-input security
  boundary.
- Serializing or deserializing provenance.
- Providing a public graph editor or a public constructor from nodes.
- Making `ReferenceGraphNode` itself impossible to construct. Nodes may remain
  ordinary public data values because callers receive only immutable node
  references from a graph and cannot insert replacement nodes.
- Changing reference extraction, transclusion ordering, cycle handling,
  validation rules, or graph JSON fields.
- Folding residual performance-review work into this feature.

## Current Behavior and Failure Modes

### Graph construction

`Markdown::reference_graph(options)` builds a full graph containing
transclusions and each visited document's links, images, imports, and other
reference records. `Markdown::transclusion_graph(options)` builds a
transclusion-only graph. Both currently return the same public type without a
field identifying which construction mode was used.

### Graph reuse

`FileTree::ensure_built` constructs a graph for tree rendering, then passes the
same graph to `validate_references_with_graph` to avoid rebuilding it. This is
the valid and performance-critical reuse path.

### Unchecked pairings

The public validation entry point currently accepts all of these invalid
pairings:

- a graph built from a different Markdown document;
- the same content with a different file or URL source, changing relative
  resolution;
- the same document with different compose/reference options;
- a transclusion-only graph passed to full reference validation;
- a graph whose public `root` or `nodes` were changed after construction;
- a manually assembled `ReferenceGraph { root, nodes }`.

The result can be a plausible successful report that did not validate the
caller's actual document.

## Invariants

The implementation must maintain all of the following:

1. Every `ReferenceGraph` was constructed by Darkmatter from exactly one root
   `Markdown`, one `ReferenceGraphOptions` value, and one graph mode.
2. A graph's root and descendant collection cannot change after construction.
3. `Full` graphs contain the reference records required by reference
   validation; `TransclusionOnly` graphs are never accepted by the full
   validation entry point.
4. A graph may be reused only when its document, source, and options identities
   match the validation request.
5. Cloning a graph preserves its contents and provenance exactly.
6. Provenance never changes graph presentation or serialization.
7. Stateful identity tracking does not keep stateful objects alive.
8. When Darkmatter cannot establish identity, reuse is rejected rather than
   guessed.

## Target Public API

The exact internal layout remains private. The supported public surface is:

```rust
#[derive(Debug, Clone)]
pub struct ReferenceGraph {
    root: ReferenceGraphNode,
    nodes: Vec<ReferenceGraphNode>,
    provenance: ReferenceGraphProvenance,
}

impl ReferenceGraph {
    pub fn root(&self) -> &ReferenceGraphNode;
    pub fn nodes(&self) -> &[ReferenceGraphNode];
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ReferenceGraphNode>;
    pub fn node_by_id(&self, id: &str) -> Option<&ReferenceGraphNode>;
    pub fn node_count(&self) -> usize;
    pub fn to_mermaid(&self) -> String;
    pub fn to_dot(&self) -> String;
    pub fn view(&self, follow: bool) -> ReferenceGraphView<'_>;
}
```

`iter()` yields the root first, followed by `nodes()` in their existing stored
order. It exists because graph flattening, renderers, file-tree construction,
and tests repeatedly need this exact traversal. It must not expose a mutable
iterator.

The internal constructor computes provenance itself so no caller can forget or
mislabel it:

```rust
pub(crate) fn from_build(
    document: &Markdown,
    options: &ReferenceGraphOptions,
    mode: ReferenceGraphMode,
    root: ReferenceGraphNode,
    nodes: Vec<ReferenceGraphNode>,
) -> ReferenceGraph;
```

There is no public `new`, `from_parts`, `Default`, deserializer, or mutable
parts accessor.

## Graph Views

The current `ReferenceGraphNodeView` has public fields, allowing callers to pair
a node with an unrelated graph. Replace direct field construction with a
graph-produced view:

```rust
let json = serde_json::to_value(graph.view(follow))?;
```

`ReferenceGraphView` and any node-view helper keep their fields private. A
crate-private recursive helper may serialize child nodes. The emitted root JSON
remains byte-for-byte shape-compatible:

- `file`
- `source`
- `references`
- `transclusions`
- nested `node` values only when `follow` is enabled

Provenance, hashes, graph mode, and stateful identities never appear in JSON.

## Provenance Model

Provenance is private and mandatory:

```rust
struct ReferenceGraphProvenance {
    document: ReferenceDocumentIdentity,
    source: Option<ComposeSource>,
    mode: ReferenceGraphMode,
    options: ReferenceGraphOptionsIdentity,
}

enum ReferenceGraphMode {
    Full,
    TransclusionOnly,
}
```

### Document identity

Document identity covers the complete document state observed by graph
construction:

- raw frontmatter text when Darkmatter retains it;
- a canonical frontmatter representation otherwise;
- the body bytes;
- explicit domain and version markers between components.

Store both Darkmatter's Markdown-aware frontmatter/body identities and a
whole-represented-state fingerprint. The second fingerprint is mandatory: two
documents that produce the same reference shape, or the same semantic
frontmatter value through different represented bytes, must not become
interchangeable merely because their graphs look alike. Compute it with
`biscuit-hash` xxHash and explicit domain separators; do not introduce SHA or
an ad hoc hasher.

The identity is a practical in-process correctness guard, not an authentication
token. Documentation must not call xxHash provenance cryptographically
unforgeable.

### Source identity

Store the exact `Markdown::source()` value separately. Identical Markdown at
`docs/a.md` and `other/a.md` is not interchangeable because relative links,
transclusions, repository-root lookup, and diagnostics may differ.

Do not canonicalize source identity during comparison. A conservative false
negative merely rebuilds or rejects reuse; a false positive can validate the
wrong graph.

### Graph mode

The constructor records `Full` or `TransclusionOnly`. Reference validation
requires `Full` even when every other identity component matches. This check is
mandatory; options provenance alone cannot distinguish the two builders.

### Options identity

`ReferenceGraphOptions` currently wraps `ComposeOptions`, whose values affect
conditional directives, interpolation, replacements, schema coercion, source
resolution, transclusion, shell behavior, remote reads, and runtime context.

The initial identity is deliberately conservative: it covers every
`ComposeOptions` field, including fields not currently read by graph
construction. This prevents a newly wired option from silently becoming an
untracked graph input.

Requirements:

1. Define one crate-private `ReferenceGraphOptionsIdentity::capture` authority
   in the `ComposeOptions` owning module.
2. Destructure `ComposeOptions` exhaustively without `..` so adding a field
   causes a compile error until its identity treatment is chosen.
3. Encode ordinary values canonically with field names, type boundaries, and a
   versioned domain marker. Do not use `Debug` output as the canonical encoding.
4. Sort unordered sets and maps before hashing.
5. Include captured context values and environment values that can affect
   interpolation or conditions.
6. Use `biscuit-hash` xxHash for the compact value fingerprint.
7. For stateful callbacks or runtimes whose semantics cannot be represented by
   values, retain only weak or minimal identity handles and compare instance
   identity.
8. Do not retain full options, caches, contexts, callback `Arc`s, preflight
   graphs, or remote-fetch runtimes.
9. A dropped or recreated stateful instance is not assumed equivalent. Reject
   reuse unless identity is proven.

Performance-only fields may make identity stricter than necessary in v1. That
is acceptable. Narrowing the identity later requires a focused audit proving
the omitted field cannot affect any graph record, node, insertion, source, or
validation preparation.

## Validation Contract

`Markdown::validate_references_with_graph` retains its signature. Before
flattening or preparing fragments, it asks the graph to verify compatibility:

```text
document mismatch     -> error
source mismatch       -> error
graph mode mismatch   -> error
options mismatch      -> error
all identities match  -> validate using the prebuilt graph
```

Use a structured mismatch reason rather than tests that depend only on an
arbitrary message substring. The preferred shape is a dedicated
`ReferenceGraphMismatch` reason carried by a `ReferenceError` variant. Its
terminal rendering should state which dimension differs and instruct the caller
to rebuild the graph from the same document and options.

The ordinary `validate_references` path remains unchanged conceptually: it
builds a full graph and immediately validates it, so compatibility is guaranteed
by construction.

## Compatibility Ruling

The following source forms intentionally stop compiling:

```rust
ReferenceGraph { root, nodes }
graph.root
graph.nodes
ReferenceGraphNodeView { node, graph, follow }
```

They migrate to builder and accessor APIs:

```rust
let graph = markdown.reference_graph(options)?;
let root = graph.root();
let descendants = graph.nodes();
let json = serde_json::to_value(graph.view(follow))?;
```

Unchanged contracts:

- `Markdown::reference_graph` signature;
- `Markdown::transclusion_graph` signature;
- `Markdown::validate_references_with_graph` signature;
- `ReferenceGraph: Debug + Clone`;
- node order and lookup behavior;
- Mermaid and DOT output;
- file-tree and CLI behavior;
- serialized graph-view JSON.

## Callsite Migration

GitNexus reports MEDIUM upstream impact: 26 indexed symbols, six direct
dependents, two affected modules (`reference` and `file_tree`), and no indexed
execution-flow boundary outside them. Text search adds the CLI JSON adapter and
integration tests that access public fields directly.

Migrate these groups:

1. `reference/graph.rs`: internal construction, flattening, graph renderers,
   and unit tests.
2. `reference/validate.rs`: descendant traversal and compatibility check.
3. `reference/file_tree/model.rs`: root/descendant indexing and tests.
4. `reference/types.rs`: node lookup, traversal, graph serialization tests.
5. `darkmatter/cli/src/commands/graph.rs`: JSON view construction.
6. `darkmatter/lib/tests/reference_integration.rs`: public accessor coverage.

Tests that currently create graph literals must use a real Markdown builder or
a crate-private builder helper that still computes valid provenance. Do not add
a `for_test()` provenance escape hatch that fabricates an unrelated identity.

## Verification

### Invariant tests

- Matching document, source, mode, and options produce validation parity with
  `validate_references`.
- Different body bytes are rejected.
- Different represented frontmatter is rejected, including forms that produce
  the same reference shape.
- Identical content with a different file source is rejected.
- Identical content with a different URL source is rejected.
- Representative scalar, collection, context, schema, transclusion, remote,
  and shell option changes are rejected.
- A transclusion-only graph is rejected by full reference validation.
- A cloned full graph validates identically to its original.
- Recreated callback, preflight, and fetch instances are rejected even when
  their visible configuration matches.
- Building and dropping a graph does not increase the final strong count of
  callbacks, preflight graphs, or shared runtimes.

### Accessor and presentation tests

- `root()`, `nodes()`, `iter()`, `node_by_id()`, and `node_count()` preserve
  current ordering and lookup semantics.
- `iter()` yields the root exactly once.
- Mermaid and DOT snapshots are unchanged.
- File-tree terminal output is unchanged.
- Followed and non-followed JSON graph views are unchanged.
- JSON contains no provenance, hashes, graph mode, or runtime identities.

### Compile-time maintenance guard

The exhaustive no-`..` `ComposeOptions` destructure is the maintenance guard
for newly added fields. Add a focused test for canonical equality across
unordered map/set insertion orders and inequality across representative option
families.

### Performance check

Preserve a same-source Criterion comparison for full graph construction and
prebuilt validation on small, large, and multi-transclusion fixtures.

- Prebuilt validation must remain materially faster than rebuilding the graph.
- Provenance must not retain large objects or introduce superlinear work.
- A construction regression is unacceptable when it exceeds both 5% and
  100 microseconds at the median on a stable fixture.
- Record fixture identity, sample count, dispersion, host, and commit/worktree
  provenance.

### Required gates

- Focused reference unit and integration tests.
- Darkmatter `just test`.
- Darkmatter `just lint`.
- Workspace build, or the repository's documented narrow fallback when an
  unrelated generated artifact blocks one workspace member.
- `git diff --check`.
- GitNexus `detect_changes()` before commit.

Level 2 and Level 3 tests are not required: this feature changes no terminal
query, real-terminal rendering, browser interaction, or host input behavior.

## Documentation

Update together with implementation:

- `ReferenceGraph` rustdoc: builder-produced, immutable artifact contract.
- `Markdown::reference_graph` and `transclusion_graph` rustdoc: recorded mode.
- `Markdown::validate_references_with_graph` rustdoc: matching provenance and
  full-mode requirements plus structured mismatch errors.
- Darkmatter skill: use builders and read-only accessors; provenance is private
  and JSON-invisible.
- The 2026-07-12 performance review records: move the public graph correctness
  work to this linked feature rather than claiming the partial design landed.

## Acceptance Criteria

1. `ReferenceGraph` has no public or `pub(crate)` data fields.
2. Downstream code can inspect but cannot construct or mutate graph contents.
3. All graph construction routes through one provenance-computing internal
   constructor.
4. Full validation rejects mismatched document state, source, graph mode, and
   options before reading graph contents.
5. `ComposeOptions` identity is canonical, compact, exhaustive, and does not
   rely on `Debug` output.
6. Graph ownership does not extend stateful callback/runtime/preflight
   lifetimes.
7. Direct field and graph-view literal callsites use the supported accessors.
8. Existing graph, file-tree, Mermaid, DOT, terminal, and JSON behavior is
   preserved.
9. Focused tests, Darkmatter L1, lint, build, whitespace, and GitNexus scope
   checks pass.
10. Performance evidence confirms that safe prebuilt validation retains its
    intended win without a material graph-construction regression.
