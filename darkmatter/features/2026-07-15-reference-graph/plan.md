---
agent: "claude/"
total_phases: 5
created: 2026-07-15
phase: 1
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/remote_fetch.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/lib/src/markdown/compose/cache/runtime.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/reference/provenance.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - darkmatter
---

# Execution Plan — Opaque Reference Graph

Turns `ReferenceGraph` into an opaque, immutable, builder-produced artifact
carrying private build provenance, and makes
`Markdown::validate_references_with_graph` reject any prebuilt graph whose root
document, source, graph mode, options, or descendant-document state does not
match the validation request — before the graph is flattened.

## Source Map (verified against HEAD)

| Concern | Location |
|---------|----------|
| `ReferenceGraph`, `ReferenceGraphNode`, `ReferenceGraphNodeView`, `ReferenceGraphOptions` | `darkmatter/lib/src/markdown/reference/types.rs` |
| Graph builders, `build_graph_inner(extract_references: bool)`, `build_node`, `flatten_graph`, `ReferenceGraph { root, nodes }` literal (`graph.rs:96`), `runtime.load_markdown` (child `Markdown`) | `darkmatter/lib/src/markdown/reference/graph.rs` |
| `validate` / `validate_with_graph`, `ReferenceValidationOptions` (`graph`, `validate_remote`, `validate_fragments`, `remote_timeout`, `fail_fast`) | `darkmatter/lib/src/markdown/reference/validate.rs` |
| Production descendant-field read (`for node in &graph.nodes`) that must migrate during opacity cutover | `darkmatter/lib/src/markdown/reference/validate.rs:663` |
| `ReferenceError` enum + `BlockError::status_block` | `darkmatter/lib/src/markdown/reference/errors.rs` |
| `Markdown::reference_graph` / `transclusion_graph` / `validate_references_with_graph` | `darkmatter/lib/src/markdown/reference/mod.rs` (293–540); `pub use types::*` at 17 |
| `FileTree::ensure_built` (clones `graph_options` into build **and** validation — Finding 18 path) | `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:225` |
| `build_file_tree_model` (`graph.root`, `graph.nodes`) | `darkmatter/lib/src/markdown/reference/file_tree/model.rs:183` |
| CLI JSON `ReferenceGraphNodeView { node, graph, follow }` | `darkmatter/cli/src/commands/graph.rs:29` |
| Public-field access in integration tests (`graph.root`, `graph.nodes`, `graph.root.child_insertions`) | `darkmatter/lib/tests/reference_integration.rs` (60, 62, 66, 141–142, 216, 245, 661–662, 835, 838, 1125) |
| `ComposeOptions` (40+ fields, all `pub(crate)`/private) + `ComposeSource` | `darkmatter/lib/src/markdown/compose/context/options.rs` |
| Stateful `Arc`-backed fields (clone-shared instance identity): `shell_approval_handler: Option<Arc<dyn ShellApprovalHandler>>`, `preflight_graph: Option<Arc<PreflightGraphNode>>`, `remote_fetch: Option<RemoteFetchRuntime>` (`inner: Arc<RemoteFetchInner>`) | same file / `compose/remote_fetch.rs:95` |
| Existing partial, `Debug`-based compose-cache fingerprint that the shared classification must replace/delegate | `darkmatter/lib/src/markdown/compose/cache/hashing.rs:131` |
| Compose-cache key assembly and persistent-cache context/eligibility callsite | `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:1330`; `compose/cache/runtime.rs:239` |
| Run-local Markdown cache that descendant freshness checks must bypass | `darkmatter/lib/src/markdown/compose/cache/runtime.rs:178` |
| `Markdown::frontmatter()` / `content()` (body) / `source()`; `Frontmatter::raw_source() -> Option<&str>` | `markdown/mod.rs:165/175/185`; `markdown/frontmatter.rs:67` |
| Hashing authority (`biscuit-hash` already a dep at `lib/Cargo.toml:87`): `xx_hash`, `xx_hash_variant`, `HashVariant` | `biscuit_hash` (used in `markdown/hash`, `markdown/toc`) |
| Existing Criterion benches and dependency | `darkmatter/lib/benches/`; `darkmatter/lib/Cargo.toml:121` |

### Confirmed constraints from the read-through

- `RemoteFetchRuntime`, `shell_approval_handler`, and `preflight_graph` all wrap
  an `Arc`, so a `ComposeOptions::clone()` **shares** the same allocation. Store
  only clone-stable `Weak` handles and compare weak allocation identity; never
  retain a strong `Arc` or a raw pointer address. This is the load-bearing fact
  that lets fail-closed identity coexist with the Finding 18 reuse path
  (`FileTree::ensure_built` clones `self.graph_options`) without extending
  stateful-object lifetimes or accepting allocator-address reuse.
- `reference/mod.rs` does `pub use types::*;`, so demoting
  `ReferenceGraphNodeView` to `pub(crate)`/private removes it from the public
  surface automatically.
- Criterion is already configured and `lib/benches/` contains existing benches;
  no reference-graph bench exists yet, so the performance gate needs only a new
  registered bench target.
- The compile break is intentional and atomic: once `ReferenceGraph` fields go
  private, `graph.rs`, `types.rs` (mermaid/dot use `self.root`/`self.nodes`
  inside `impl`, so those stay legal), `file_tree/model.rs`, `cli/.../graph.rs`,
  and the integration tests must all migrate in the same phase to restore green.

---

## Preflight — impact and scope (before Phase 1 edits)

- [ ] Run GitNexus upstream impact analysis before editing the three indexed
  symbols. At the plan-review index: `ReferenceGraph` is MEDIUM risk with six
  direct dependents; `validate_with_graph` is LOW risk with three direct
  dependents; `build_graph_inner` is MEDIUM risk with two direct and 41
  transitive dependents. Record the fresh results and warn before proceeding if
  any result has risen to HIGH or CRITICAL.
- [ ] Confirm the working-tree baseline and preserve unrelated changes. The
  expected implementation scope includes `reference`, `file_tree`, CLI graph,
  the `ComposeOptions` owning module, compose-cache hashing/runtime and the
  transclusion cache callsite, tests, docs, and the new benchmark.

---

## Phase 1 — Identity, provenance, and cache coordination (stays green)

Introduce every new type and its capture logic before wiring it into
`ReferenceGraph`, then move compose caching onto the shared classification. The
crate keeps compiling and every new unit is tested directly. Rendered output and
public behavior do not change; cache reuse becomes more conservative where
identity cannot be proven.

- [x] Add `ReferenceGraphMode { Full, TransclusionOnly }` (Debug, Clone, Copy, PartialEq, Eq) in the reference subsystem (new `reference/provenance.rs`), re-exported crate-internally.
- [x] Add `ReferenceDocumentIdentity` with `pub(crate) fn capture(md: &Markdown) -> Self`. Retain only compact values: (a) Darkmatter's current Markdown-aware frontmatter-map hash with `strict = true`, (b) Darkmatter's body hash with `strict = true`, and (c) a mandatory whole-represented-state fingerprint. The strict component policy preserves map order and body bytes for fail-closed reuse. The whole-state input includes a versioned domain, representation discriminants, retained authored `Frontmatter::raw_source()` bytes when present, an order-preserving canonical encoding of the **current** frontmatter map, and the current body bytes. Including both raw source and the current map is mandatory because `Frontmatter::as_map_mut()` can change the in-memory map without invalidating the retained raw snapshot. Raw strings/canonical serialization are transient hashing inputs and are not stored. Rustdoc must state this is an in-process correctness guard, **not** cryptographically unforgeable.
- [x] Add `ReferenceDocumentDependency { source: ComposeSource, document: ReferenceDocumentIdentity }` and `ReferenceDependencyManifest { documents: Vec<ReferenceDocumentDependency> }`, with a private builder API that (a) inserts at most one entry per exact resolved `ComposeSource` already used to construct the node, without a second/ad hoc canonicalization, and (b) keeps entries in deterministic source order. `Default`/empty manifest supported for synthetic tests.
- [x] Define one crate-private, exhaustive `ComposeOptions` field-classification authority inside `compose/context/options.rs`, the owning module required to see private fields. Destructure `ComposeOptions` **exhaustively with no `..`** so a future field is a compile error until its graph and cache treatment is chosen. Classify each collection by semantics: preserve the order of ordered vectors such as `magic_paths` and `env_path_whitelist`; sort only genuinely unordered maps/sets such as `exclude_keys`, `pre_approved_commands`, and canonical map/context entries. Encode canonical values with field names, type boundaries, and a versioned domain marker; never use `Debug` output.
- [x] Derive two purpose-specific products from that single classification: (a) conservative, fail-closed `ReferenceGraphOptionsIdentity::capture`, covering every field, and (b) the compose-cache value fingerprint plus a persistent-cache eligibility decision. Replace or delegate the existing `cache::hashing::options_hash`; do not create a third parallel field inventory. Preserve the existing source/effective-state/context/directive-overlay/pass-scope cache-key dimensions. Process-local stateful identity may participate in run-local reuse only; if equivalence requires instance identity, disable persistent-cache reads and writes for that key.
- [x] For stateful callback/runtime/preflight fields, retain only `Weak` allocation handles and compare weak allocation identity. Never store raw pointer addresses or strong `Arc`s. Add a narrow crate-private weak-identity accessor on `RemoteFetchRuntime` rather than reaching through its private `inner` field. A dropped or recreated instance is non-equivalent even when visible configuration matches.
- [x] Add `ReferenceGraphProvenance { document: ReferenceDocumentIdentity, source: Option<ComposeSource>, mode: ReferenceGraphMode, options: ReferenceGraphOptionsIdentity, dependencies: ReferenceDependencyManifest }` (Debug, Clone). Add a `pub(crate)` compatibility method (e.g. `check(&self, request_document, request_source, request_mode, request_options) -> Result<(), ReferenceGraphMismatch>`) that compares in the order document → source → mode → options (descendant verification is done separately by the validator in Phase 3). Define the `ReferenceGraphMismatch` reason enum (dimension differs; carries the changed/missing/unreadable child source **without** its content fingerprint) here for reuse.

**Dependency order:** land `ReferenceGraphMode` and the shared exhaustive options
classification first. `ReferenceDocumentIdentity`, the two derived options
products, and `ReferenceDependencyManifest` can then proceed independently;
`ReferenceGraphProvenance` and its `check` depend on the graph products. The
compose-cache callsite migration depends on the cache product and its persistent
eligibility result.

### Phase 1 unit tests (direct, no graph wiring yet)
- [x] Options identity is equal across differing insertion orders of genuinely unordered maps/sets such as `exclude_keys` and `pre_approved_commands`, unequal when ordered `magic_paths` or `env_path_whitelist` entries are reordered where order can affect behavior, and unequal across representative option families (scalar, collection, context, schema, transclusion, remote, shell).
- [x] Options identity is **clone-stable**: `capture(&opts) == capture(&opts.clone())` including when a stateful `Arc` field is set (shared instance) — and **unequal** when a fresh instance of that stateful field is substituted.
- [x] A dropped stateful instance followed by allocator churn and construction of a fresh instance never compares equal; identity capture and graph ownership leave strong counts unchanged.
- [x] Cache identity derives from the same classification as graph identity; value-equivalent options reuse the correct cache, while a key requiring process-local identity never reads or writes persistent cache state.
- [x] `ReferenceDocumentIdentity`: same bytes/current map → equal; different body → unequal; different represented frontmatter that yields the same reference shape → unequal; mutating a parsed document's frontmatter map after capture → unequal even though `raw_source()` remains present.
- [x] `ReferenceDependencyManifest`: repeated insertion of the same resolved source yields exactly one entry; entries are in deterministic source order.

### Checkpoint 1
`just test` (darkmatter) green; `just lint` clean. No public API or behavior
change yet (new types are crate-internal; allow `dead_code` where unavoidable
until Phase 2 wires them). Remove every temporary allowance as its item is wired.

---

## Phase 2 — Opaque `ReferenceGraph` cutover (atomic compile break → restore green)

Flip `ReferenceGraph` to private fields + provenance and migrate **every**
in-crate, CLI, and test callsite in the same phase so the workspace compiles
again with byte-identical behavior. No new validation semantics yet.

- [ ] In `types.rs`, change `ReferenceGraph` to `{ root: ReferenceGraphNode, nodes: Vec<ReferenceGraphNode>, provenance: ReferenceGraphProvenance }` with **private** fields. Add public read-only accessors: `root(&self) -> &ReferenceGraphNode`, `nodes(&self) -> &[ReferenceGraphNode]`, `iter(&self) -> impl Iterator<Item=&ReferenceGraphNode> + '_` implemented as `std::iter::once(&self.root).chain(self.nodes.iter())` (plain `impl Iterator`, **not** `ExactSizeIterator`), `node_by_id`, `node_count`, `to_mermaid`, `to_dot` (existing bodies keep using `self.root`/`self.nodes` — legal inside `impl`). Keep `Clone`; implement `Debug` manually so its observable presentation contains only the existing root/nodes and never private provenance. No public `new`/`from_parts`/`Default`/deserializer/mutable accessor.
- [ ] Add `pub(crate) fn from_build(document: &Markdown, options: &ReferenceGraphOptions, mode: ReferenceGraphMode, root: ReferenceGraphNode, nodes: Vec<ReferenceGraphNode>, dependencies: ReferenceDependencyManifest) -> ReferenceGraph`. It computes root/source/mode/options provenance **itself** (from `document`, `document.source()`, `mode`, and `ReferenceGraphOptionsIdentity::capture(&options.compose)`) and stores the passed dependency manifest. No separately-supplied root/options hashes — identities cannot be mislabeled.
- [ ] In `graph.rs`, replace `build_graph_inner(md, options, extract_references: bool)` with `build_graph_inner(md, options, mode: ReferenceGraphMode)`, deriving `let extract_references = matches!(mode, ReferenceGraphMode::Full);` (single source of truth). `build_transclusion_graph` passes `TransclusionOnly`; `build_reference_graph` passes `Full`.
- [ ] Thread a `ReferenceDependencyManifest` accumulator through the analysis runtime / `build_node` recursion. Each time a child `Markdown` is loaded (`runtime.load_markdown`), record one manifest entry from **that already-loaded value** (no second construction-time read), deduped per unique resolved child `ComposeSource`. Covers `::file`, `::toc-linking`, and frontmatter prologue/epilogue child loads. Remote/non-materialized targets do **not** enter the manifest.
- [ ] Replace the terminal `Ok(ReferenceGraph { root, nodes })` (`graph.rs:96`) with `Ok(ReferenceGraph::from_build(md, options, mode, root, all_nodes, dependencies))`. Update `flatten_graph`/`flatten_node` to use `graph.root()` / `graph.node_by_id(...)` accessors.
- [ ] Migrate `validate.rs::collect_graph_heading_slugs` from `&graph.nodes` to `graph.nodes()`; this production callsite is separate from `flatten_graph`.
- [ ] Migrate `file_tree/model.rs` `build_file_tree_model`: `graph.root` → `graph.root()`, `graph.nodes` → `graph.nodes()` (node-map build loop at 192–197 and root at 199).
- [ ] Remove `ReferenceGraphNodeView` from the public API (demote to `pub(crate)` or make its serialization a private recursive helper). Add public `ReferenceGraphView<'_>` with **private** fields plus `ReferenceGraph::view(&self, follow: bool) -> ReferenceGraphView<'_>` that always serializes from `self.root` and threads `follow`. Emitted root JSON stays byte-for-byte shape-compatible (`file`, `source`, `references`, `transclusions`, nested `node` only when `follow`); provenance/hashes/mode/manifest never appear.
- [ ] Migrate `cli/src/commands/graph.rs`: replace the `ReferenceGraphNodeView { node: &graph.root, graph, follow }` literal with `serde_json::to_value(graph.view(follow))?`, preserving the `["validation"]` injection and pretty-printing.
- [ ] Migrate in-crate synthetic-shape tests (the `graph.rs`, `types.rs`, `file_tree/model.rs` hand-authored `ReferenceGraph { root, nodes }` literals and `ReferenceGraphNodeView { .. }` literals) to `from_build(&Markdown::new(""), &ReferenceGraphOptions::default(), <mode>, root, nodes, ReferenceDependencyManifest::default())` and `graph.view(follow)`. These never call validation, so the inert placeholder provenance is legitimate (**not** a fabricated match).
- [ ] Migrate `lib/tests/reference_integration.rs` public-field reads (`graph.root` → `graph.root()`, `graph.nodes` → `graph.nodes()`, `graph.root.child_insertions` → `graph.root().child_insertions`) at the lines listed in the Source Map.
- [ ] Immediately before making the fields private, run a fresh repository-wide `rg` audit for direct `ReferenceGraph` field access and struct/view literals. Treat the Source Map as a starting inventory, not a complete frozen list.
- [ ] Remove all temporary Phase 1 `dead_code` allowances once the primitives are wired.

**Parallelizable (after `types.rs` + `from_build` land):** `graph.rs`
migration, `file_tree/model.rs` migration, the view replacement + CLI migration,
and the integration-test migration are independent edit sets that can proceed
concurrently, converging at the compile checkpoint.

### Checkpoint 2
Workspace compiles. `just test` + `just lint` (darkmatter) green. Mermaid, DOT,
file-tree, and followed/non-followed JSON output are **unchanged** vs `main`
(existing snapshots pass untouched). `ReferenceGraph` has no public/`pub(crate)`
data fields; `ReferenceGraphNodeView` is no longer publicly reachable.

---

## Phase 3 — Validation compatibility contract (the new guard behavior)

Wire provenance verification into the public prebuilt-graph validation path.
This is where a mismatched pairing becomes a **hard error** before flattening.

- [ ] Add a structured `ReferenceGraphMismatch` variant to `ReferenceError` (carrying the `ReferenceGraphMismatch` reason from Phase 1). Implement its `BlockError::status_block` arm: state which dimension differs and instruct the caller to rebuild the graph from the same document and options; a dependency mismatch names the changed/missing/unreadable child source **without** exposing any content fingerprint.
- [ ] In `validate.rs::validate_with_graph`, **before** `flatten_graph`: build the request-side identity from the method's own inputs — `md` supplies live document + source identity, and the compared options are `options.graph` (the `ReferenceGraphOptions` nested in the passed `ReferenceValidationOptions`; the other `ReferenceValidationOptions` fields do **not** participate). Call `graph`-provenance `check(...)` for document → source → mode (`Full` required) → options. Any mismatch returns `Err(ReferenceError::ReferenceGraphMismatch(..))`.
- [ ] Add descendant verification (independent of the four-dimension check): for every entry in the graph's private dependency manifest, read the exact stored local `ComposeSource::File` directly from the authoritative filesystem and compare its represented-state identity against the recorded one. This path must bypass the graph-building runtime, run-local Markdown cache, and persistent cache so stale cached content cannot produce a false match. A changed, missing, or unreadable descendant is a mismatch reported **before** flattening. Use content identity (not mtime/size/inode) as the source of truth; metadata may only short-circuit in ways that cannot produce a false match. Hash each unique visited child at most once per validation call; do not rebuild or re-parse the graph.
- [ ] Confirm the ordinary `validate` path (build-then-validate) is compatibility-guaranteed by construction and remains behaviorally unchanged (it passes the freshly built graph whose provenance matches `md`/`options.graph` by definition).
- [ ] Confirm `FileTree::ensure_built` still succeeds: it clones `self.graph_options` into both `reference_graph(...)` and the validation options, so the clone-stable options identity (Phase 1) must make the reuse pass rather than reject — this is the Finding 18 guard.

**Sequential:** the error variant lands first; the four-dimension check and the
descendant verification build on it and share the mismatch reason.

### Checkpoint 3
`just test` + `just lint` green. The Finding 18 parity test
(`validate_with_graph_matches_validate_with_fragments`) still passes.
`FileTree` graph+validate reuse (`md graph --validate`) works end-to-end with no
spurious mismatch.

---

## Phase 4 — Verification suite (invariants, presentation, guard, performance)

Prove the invariants and lock the presentation/perf contracts. Add these to the
reference unit tests and `lib/tests/reference_integration.rs`.

### Invariant tests (build graphs through the **real** builders — no fabricated-identity escape hatch)
- [ ] Matching document/source/mode/options ⇒ `validate_references_with_graph` parity with `validate_references`.
- [ ] Different body bytes rejected; different represented frontmatter rejected (incl. forms producing the same reference shape).
- [ ] Identical content at a different **file** source rejected; identical content at a different **URL** source rejected.
- [ ] Editing a visited child after graph construction rejected before flattening, even with fragment validation disabled; adding a broken reference to a visited child cannot yield a successful report from the stale graph.
- [ ] A run-local/persistent cache populated with the old child cannot mask a subsequent on-disk edit.
- [ ] Deleting / making a visited child unreadable reported as a dependency mismatch. Use deterministic cross-platform cases rather than depending only on Unix permission bits (for example, missing path and file-replaced-by-directory read failure).
- [ ] One child reached through multiple insertions ⇒ exactly one dependency entry; an unchanged child passes.
- [ ] Representative scalar / collection / context / schema / transclusion / remote / shell option changes rejected.
- [ ] `TransclusionOnly` graph rejected by full reference validation even when every other identity matches.
- [ ] A cloned full graph validates identically to its original.
- [ ] Recreated callback / preflight / fetch instances rejected even when visible configuration matches, including after the original is dropped and allocator churn occurs; a build-only side-channel callback rejected when validation supplies a **different** instance (documents the accepted v1 conservatism).
- [ ] Building and dropping a graph does not increase the final strong count of callbacks / preflight graphs / shared runtimes (Arc strong-count assertion — no lifetime extension).
- [ ] A graph built from `options.clone()` passes the check against the original `options` **and** a further clone (the Finding 18 clone-stability guard; turns a benchmark-only regression into a hard assertion).

### Accessor & presentation tests
- [ ] `root()`, `nodes()`, `iter()`, `node_by_id()`, `node_count()` preserve ordering/lookup; `iter()` yields the root exactly once.
- [ ] Mermaid and DOT snapshots unchanged; file-tree terminal output unchanged; followed and non-followed JSON views unchanged.
- [ ] JSON contains no provenance, hashes, graph mode, runtime identities, dependency manifest, child fingerprints, or freshness diagnostics.
- [ ] `Debug` output preserves the previous root/nodes presentation and contains no provenance, hashes, graph mode, dependency manifest, or runtime identity handles.

### Compile-time maintenance guard
- [ ] Confirm the no-`..` `ComposeOptions` destructure in the shared classification authority is the guard for new fields (a focused comment + the existing exhaustive match). Add canonical-equality tests across unordered map/set insertion orders, order-sensitive tests for ordered vectors, and inequality across representative option families (may reuse Phase 1 tests; ensure coverage lives with the guard).

### Performance check
- [ ] Add and register `lib/benches/reference_graph.rs` using the already-configured Criterion dependency. On small, large, and multi-transclusion fixtures, compare `validate_references` (build + validate) with `validate_references_with_graph` where graph construction occurs outside the timed loop. Separately benchmark graph construction on baseline and candidate commits so provenance construction cost is measured directly. Prebuilt validation must remain materially faster than rebuilding; provenance must not retain large objects or add superlinear work; descendant verification hashes each unique child at most once and does not reconstruct the graph.
- [ ] Record raw Criterion output plus fixture hashes, sample count, dispersion, host, and baseline/candidate commit or worktree provenance in `darkmatter/features/2026-07-15-reference-graph/results.md`. A construction regression is unacceptable only when it exceeds **both** 5% and 100 microseconds at the median on a stable fixture.

**Parallelizable:** invariant tests, accessor/presentation tests, and the bench
are independent work streams once Phase 3 lands.

### Checkpoint 4
All new tests pass under `just test`; `just lint` clean. Bench run recorded and
within the regression budget; Level 2/Level 3 tiers are **not** required (no
terminal-query, real-terminal, browser, or host-input behavior changes).

---

## Phase 5 — Documentation, drift, and final gates

- [ ] `ReferenceGraph` rustdoc: builder-produced, immutable-artifact contract (private root/descendants/provenance; read-only accessors).
- [ ] `Markdown::reference_graph` / `transclusion_graph` rustdoc: records graph mode.
- [ ] `Markdown::validate_references_with_graph` rustdoc: matching provenance, descendant freshness, `Full`-mode requirement, and structured mismatch errors (a mismatch is a hard error, not a transparent rebuild — direct build-and-validate callers to `validate_references`).
- [ ] Update the darkmatter skill (`.claude/skills/darkmatter/`): use builders + read-only accessors; provenance is private and JSON-invisible. Regenerate the skill `hash:` with `md hash <file>` after editing.
- [ ] Add only a dated correction/supersession notice and cross-links to the 2026-07-12 performance review record; do not rewrite its historical body, checkboxes, or original measurements.
- [ ] Do not change dependency documentation for Criterion: it is already present. Update dependency docs only if implementation introduces another dependency (none is expected).
- [ ] Run required gates: focused reference unit + integration tests, darkmatter `just test`, darkmatter `just lint`, workspace build (or the documented narrow fallback if an unrelated generated artifact blocks a member), and `git diff --check`. Obtain Linux and Windows CI evidence for descendant file/source handling and compilation; the local macOS run alone is not final cross-platform sign-off.
- [ ] Run GitNexus `detect_changes()` (compare against `main`) before any commit. Confirm changes remain within the preflight scope, including the coordinated cache hashing/runtime/transclusion edits required by the shared options classification, and report affected symbols and execution flows.

### Final acceptance (maps to spec Acceptance Criteria 1–13)
- [ ] No public/`pub(crate)` data fields on `ReferenceGraph`; downstream can inspect but not construct/mutate.
- [ ] All construction routes through the single provenance-computing `from_build`.
- [ ] Full validation rejects mismatched root/descendant/source/mode/options **before** flattening.
- [ ] One exhaustive `ComposeOptions` classification produces distinct canonical, compact, `Debug`-free graph and cache identities; ordered collections preserve order and unordered collections are canonicalized.
- [ ] Stateful/process-local identity never enters a persistent-cache key; such keys are limited to run-local reuse.
- [ ] Graph ownership does not extend stateful callback/runtime/preflight lifetimes.
- [ ] One compact dependency identity per unique visited local child; changed/missing/unreadable children reject reuse before flattening.
- [ ] Graph mode is the sole input controlling reference extraction.
- [ ] Identity is clone-stable (Finding 18 reuse preserved, not silently rebuilt).
- [ ] Direct-field and view-literal callsites use the supported accessors.
- [ ] Existing graph / file-tree / Mermaid / DOT / terminal / JSON behavior preserved.
- [ ] Focused tests, darkmatter L1, lint, build, whitespace, and GitNexus scope checks pass.
- [ ] Performance evidence confirms prebuilt validation keeps its win without a material construction regression.
