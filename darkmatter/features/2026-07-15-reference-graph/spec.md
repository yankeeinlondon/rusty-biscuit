---
status: draft
reviewed: true
reviewed_by: claude/default
reviewed_on: 2026-07-15
review_iterations: 4
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
- Every graph carries private root-document, descendant-document, source,
  graph-mode, and options provenance.
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
2. Reject prebuilt graphs produced from different represented root or visited
   descendant document bytes, source identity, graph mode, or graph-affecting
   options.
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
- a graph whose root is unchanged but whose stored child references came from
  a descendant document that has since changed or disappeared;
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
4. A graph may be reused only when its root document, source, graph mode,
   options, and descendant-document identities match the validation request and
   current dependency state.
5. Cloning a graph preserves its contents and provenance exactly.
6. Provenance never changes graph presentation or serialization.
7. Stateful identity tracking does not keep stateful objects alive.
8. When Darkmatter cannot establish identity, reuse is rejected rather than
   guessed.
9. Identity is stable across `ComposeOptions::clone()`: a graph built from
   `options.clone()` still matches `options`, so the Finding 18 reuse path
   (which clones the same options for both the build and the validation call)
   is never spuriously rejected.
10. Provenance identifies every file-backed descendant document materialized as
    a graph node, not only the root document. Before validation consumes stored
    child reference records, every descendant source must still exist, be
    readable, and have the represented-state identity recorded during graph
    construction.
11. A changed, missing, or unreadable descendant is a provenance mismatch. The
    validator rejects the graph before flattening it; selectively re-reading
    child headings does not make stale stored reference records safe.

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
    pub fn iter(&self) -> impl Iterator<Item = &ReferenceGraphNode> + '_;
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
iterator. The return type is a plain `impl Iterator`, not
`ExactSizeIterator`: the natural implementation is
`std::iter::once(root).chain(nodes.iter())`, and `std::iter::Chain` does not
implement `ExactSizeIterator`. Callers that need a length already have
`node_count()`; do not hand-write an `ExactSizeIterator` adapter just to widen
the bound.

The internal constructor computes the root, source, mode, and options provenance
itself, then consumes the dependency manifest produced by the same graph-build
runtime:

```rust
pub(crate) fn from_build(
    document: &Markdown,
    options: &ReferenceGraphOptions,
    mode: ReferenceGraphMode,
    root: ReferenceGraphNode,
    nodes: Vec<ReferenceGraphNode>,
    dependencies: ReferenceDependencyManifest,
) -> ReferenceGraph;
```

`ReferenceDependencyManifest` is private build output assembled while child
documents are already loaded. It contains compact identities, not child
`Markdown` values. Only `reference/graph.rs` production construction may create
a non-empty manifest. The constructor does not accept separately supplied root
or options hashes, so those identities cannot be forgotten or mislabeled.

There is no public `new`, `from_parts`, `Default`, deserializer, or mutable
parts accessor.

## Graph Views

The current public `ReferenceGraphNodeView` has public fields
(`node`, `graph`, `follow`), allowing callers to pair a node with an unrelated
graph. Remove it from the public API and replace direct field construction with
a graph-produced view rooted at the graph's own root:

```rust
let json = serde_json::to_value(graph.view(follow))?;
```

`graph.view(follow)` is the public replacement: it always serializes from
`self.root` (the previous `node: &graph.root`) and threads `follow` down the
tree. `ReferenceGraphView` and any per-node view helper keep their fields
private, so a view can never be constructed against a foreign graph or node. A
crate-private recursive helper may serialize child nodes (the old
`ReferenceGraphNodeView` serialization logic becomes this private helper). The
emitted root JSON remains byte-for-byte shape-compatible:

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
    dependencies: ReferenceDependencyManifest,
}

enum ReferenceGraphMode {
    Full,
    TransclusionOnly,
}
```

### Root document identity

The root document identity covers the complete in-memory document state
observed by graph construction:

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

### Descendant dependency identity

Graph construction reads each followed local Markdown document, prepares it,
and stores its reference records in a graph node. Those records become stale if
the document changes after graph construction. Provenance therefore includes a
private manifest entry for every file-backed non-root document materialized as
a node:

```rust
struct ReferenceDependencyManifest {
    documents: Vec<ReferenceDocumentDependency>,
}

struct ReferenceDocumentDependency {
    source: ComposeSource,
    document: ReferenceDocumentIdentity,
}
```

Requirements:

1. Capture the identity from the same loaded `Markdown` value used to build the
   node; do not perform a second construction-time file read.
2. Hash once per unique resolved descendant source. Repeated insertions of the
   same document share one manifest entry.
3. Store entries in deterministic source order so graph clones and diagnostics
   are stable.
4. Before flattening the graph, validation reloads each local dependency and
   compares its represented-state identity. A missing or unreadable dependency
   is a mismatch, not an empty document.
5. The validation check uses content identity rather than modification time,
   size, inode, or another metadata-only shortcut. Metadata may be used only as
   an optimization that cannot produce a false match.
6. Dependency verification must not expose hashes or sources in graph JSON.
7. Remote reference targets that are not materialized as child graph nodes do
   not enter this manifest. Their reachability remains governed by
   `ReferenceValidationOptions::validate_remote` and the existing remote-read
   policy.

This manifest is not a persistent cache contract. It is the minimum state
needed to prevent public prebuilt validation from mixing stored references from
an old child document with live filesystem state from a new one.

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

1. Define one crate-private, exhaustive `ComposeOptions` field-classification
   authority in the `ComposeOptions` owning module. Derive
   `ReferenceGraphOptionsIdentity::capture` from it. The linked performance
   follow-up's compose-cache identity uses the same classification but derives a
   purpose-specific value fingerprint; graph comparison and persistent cache
   identity must not collapse into one undifferentiated product.
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
10. Identity must survive `ComposeOptions::clone()`. The Finding 18 reuse path
    clones the same options for the build and the validation call — see
    `FileTree::ensure_built`, which passes `self.graph_options.clone()` to
    `reference_graph(...)` and again into the validation options. A graph built
    from `options.clone()` must therefore satisfy the compatibility check
    against `options` (and any further clone). Stateful fields consequently
    share instances through `Arc` (or an equivalent identity-preserving handle)
    so a clone compares equal by instance identity; a clone that manufactured a
    fresh stateful instance would reject the legitimate reuse and silently
    erase Finding 18's win (caught only by the performance gate, not by the
    correctness tests). This is the load-bearing constraint that lets fail-closed
    identity coexist with the reuse optimization.
11. Only `ReferenceGraphOptions` (its wrapped `ComposeOptions`) participates in
    graph identity. The non-graph `ReferenceValidationOptions` fields
    (`validate_remote`, `validate_fragments`, `remote_timeout`, `fail_fast`)
    govern how the flattened set is validated, not how the graph is built, and
    are excluded from provenance.

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
dependency mismatch   -> error
all identities match  -> validate using the prebuilt graph
```

The request-side identity is drawn from the method's own inputs: `self`
supplies the live document and source identity, and the compared options are
`options.graph` — the `ReferenceGraphOptions` nested inside the passed
`ReferenceValidationOptions`, the same value the graph would have been built
from. The other `ReferenceValidationOptions` fields do not participate (see
Options identity requirement 11). Descendant identity is checked independently
by reloading the graph's private dependency manifest before graph flattening.

Use a structured mismatch reason rather than tests that depend only on an
arbitrary message substring. The preferred shape is a dedicated
`ReferenceGraphMismatch` reason carried by a `ReferenceError` variant. Its
terminal rendering should state which dimension differs and instruct the caller
to rebuild the graph from the same document and options. A dependency mismatch
also identifies the changed, missing, or unreadable child source without
exposing its content fingerprint.

The ordinary `validate_references` path remains unchanged conceptually: it
builds a full graph and immediately validates it, so compatibility is guaranteed
by construction.

A mismatch is a hard error, not a transparent rebuild. A self-healing fallback
(silently discard the supplied graph, build a fresh one, validate that) was
considered and rejected: it would preserve the "always returns a correct report"
property but hide the caller bug that produced the mismatched pairing, which is
exactly the class of defect this feature exists to surface (invariant 8, "reuse
is rejected rather than guessed"). Callers who genuinely want build-and-validate
in one step already have `validate_references`; `validate_references_with_graph`
is the explicit opt-in whose entire contract is "I hold a matching graph," so a
mismatch there is a programming error worth failing loudly on.

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

Provenance is finalized exactly once per graph, at the single outer
constructor. `build_graph_inner` (which recurses through `build_node`) already
assembles the final `root` and `all_nodes` before returning; its terminal
`Ok(ReferenceGraph { root, nodes })` becomes
`Ok(ReferenceGraph::from_build(md, options, mode, root, all_nodes,
dependencies))`.

`build_graph_inner` accepts `ReferenceGraphMode`, not a separate
`extract_references: bool`. It derives extraction behavior from the mode so the
stored provenance and graph contents have one source of truth:

```rust
let extract_references = matches!(mode, ReferenceGraphMode::Full);
```

The two builders pass `Full` and `TransclusionOnly`, respectively. While
`build_node` recursively loads child Markdown, the run-local analysis runtime
records one compact dependency identity per unique resolved child source. The
outer constructor consumes that finished manifest. Hash once per unique visited
document, not once per insertion or once per recursive node occurrence.

Migrate these groups:

1. `reference/graph.rs`: internal construction, flattening, graph renderers,
   and unit tests.
2. `reference/validate.rs`: descendant traversal and compatibility check.
3. `reference/file_tree/model.rs`: root/descendant indexing and tests.
4. `reference/types.rs`: node lookup, traversal, graph serialization tests.
5. `darkmatter/cli/src/commands/graph.rs`: JSON view construction.
6. `darkmatter/lib/tests/reference_integration.rs`: public accessor coverage.

Tests that currently create graph literals must route through the internal
constructor. There are two distinct cases, and the ban on fabricated identity
applies to only one of them:

- **Validation tests** exercise `validate_references_with_graph` and its
  compatibility check. These must build the graph from a real `Markdown`
  through the ordinary builders so provenance is genuine. Do not add a
  `for_test()` escape hatch that fabricates an unrelated identity to force a
  match — that would defeat the feature under test.
- **Flatten / model / serialization tests** (the many synthetic-shape graphs in
  `reference/graph.rs`, `reference/types.rs`, and `reference/file_tree/model.rs`
  — e.g. two prologues sharing line 0, or hand-authored `child_a`/`child_b`
  node IDs) assemble node shapes that no single real document produces, and
  they never call validation. They may hand the pre-built `root` and `nodes` to
  `from_build` with a placeholder document (`&Markdown::new("")`) and an empty
  dependency manifest. The recorded provenance is inert for these tests because
  nothing compares it; this is not a fabricated *match*, so it does not violate
  the rule above. `from_build` accepting caller-supplied `root`/`nodes` and the
  private manifest is exactly what makes this possible without a separate
  public escape hatch.

## Verification

### Invariant tests

- Matching document, source, mode, and options produce validation parity with
  `validate_references`.
- Different body bytes are rejected.
- Different represented frontmatter is rejected, including forms that produce
  the same reference shape.
- Identical content with a different file source is rejected.
- Identical content with a different URL source is rejected.
- Editing a visited child document after graph construction is rejected before
  flattening, even when fragment validation is disabled.
- Adding a broken reference to a visited child after graph construction cannot
  produce a successful report from the stale graph.
- Deleting a visited child or making it unreadable is reported as a dependency
  mismatch.
- Reaching one child through multiple insertions produces one dependency entry,
  and an unchanged child passes validation.
- Representative scalar, collection, context, schema, transclusion, remote,
  and shell option changes are rejected.
- A transclusion-only graph is rejected by full reference validation.
- A cloned full graph validates identically to its original.
- Recreated callback, preflight, and fetch instances are rejected even when
  their visible configuration matches.
- Building and dropping a graph does not increase the final strong count of
  callbacks, preflight graphs, or shared runtimes.
- A graph built from `options.clone()` passes the compatibility check against
  the original `options` and against a further clone (invariant 9). This is the
  guard that the Finding 18 reuse path — which clones the same options for the
  build and the validation call — is not silently rejected, turning a
  performance regression that only the benchmark would notice into a hard
  correctness assertion.
- A graph built with a progress/side-channel callback attached only to the
  build options is rejected when validation supplies a *different* instance of
  that callback, confirming instance-identity comparison (and documenting that
  attaching such a callback to only one of the two calls forfeits reuse — the
  accepted v1 conservatism).

### Accessor and presentation tests

- `root()`, `nodes()`, `iter()`, `node_by_id()`, and `node_count()` preserve
  current ordering and lookup semantics.
- `iter()` yields the root exactly once.
- Mermaid and DOT snapshots are unchanged.
- File-tree terminal output is unchanged.
- Followed and non-followed JSON graph views are unchanged.
- JSON contains no provenance, hashes, graph mode, or runtime identities.
- JSON contains no dependency manifest, child fingerprints, or freshness
  diagnostics.

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
- Descendant verification hashes each unique visited child at most once per
  validation call and does not parse or reconstruct the reference graph.
- A construction regression is unacceptable when it exceeds both 5% and
  100 microseconds at the median on a stable fixture.
- Record fixture identity, sample count, dispersion, host, and commit/worktree
  provenance.

### Required gates

- Focused reference unit and integration tests.
- Darkmatter `just test`.
- Darkmatter `just lint`.
- Darkmatter `just build`; this package-area recipe covers the affected
  `darkmatter`, `darkmatter-cli`, and `dmls` packages.
- `git diff --check`.
- GitNexus `detect_changes()` before commit.

Level 2 and Level 3 tests are not required: this feature changes no terminal
query, real-terminal rendering, browser interaction, or host input behavior.

## Documentation

Update together with implementation:

- `ReferenceGraph` rustdoc: builder-produced, immutable artifact contract.
- `Markdown::reference_graph` and `transclusion_graph` rustdoc: recorded mode.
- `Markdown::validate_references_with_graph` rustdoc: matching provenance,
  descendant freshness, and full-mode requirements plus structured mismatch
  errors.
- Darkmatter skill: use builders and read-only accessors; provenance is private
  and JSON-invisible.
- The 2026-07-12 performance review records: move the public graph correctness
  work to this linked feature rather than claiming the partial design landed.

## Acceptance Criteria

1. `ReferenceGraph` has no public or `pub(crate)` data fields.
2. Downstream code can inspect but cannot construct or mutate graph contents.
3. All graph construction routes through one provenance-computing internal
   constructor.
4. Full validation rejects mismatched root document state, descendant document
   state, source, graph mode, and options before flattening node contents.
5. `ComposeOptions` identity is canonical, compact, exhaustive, and does not
   rely on `Debug` output.
6. Graph ownership does not extend stateful callback/runtime/preflight
   lifetimes.
7. Each unique visited local child has one compact dependency identity, and
   changed, missing, or unreadable children reject reuse before flattening.
8. Graph mode is the sole input controlling reference extraction; no parallel
   Boolean can disagree with recorded provenance.
9. Identity is clone-stable: a graph built from `options.clone()` still
   validates against `options`, so the Finding 18 reuse path is preserved
   rather than silently rebuilding.
10. Direct field and graph-view literal callsites use the supported accessors.
11. Existing graph, file-tree, Mermaid, DOT, terminal, and JSON behavior is
   preserved.
12. Focused tests, Darkmatter L1, lint, build, whitespace, and GitNexus scope
    checks pass.
13. Performance evidence confirms that safe prebuilt validation retains its
    intended win without a material graph-construction regression.
