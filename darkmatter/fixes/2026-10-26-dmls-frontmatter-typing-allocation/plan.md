---
total_phases: 5
created: 2026-10-26
phase: 1
agent: "codex/gpt-6.1-sol"
yolo: "true"
human_review: false
---

# DMLS frontmatter typing allocation implementation plan

## Phase 1: Establish contracts and baseline

Reduce editor allocation by separating cursor lookup from document-wide typing and sharing schema storage across requests for the same analysis snapshot. Implement snapshot-local schema identities, an effective root assembled once, shared nested and synthesized definitions, and a lazy typed scalar inventory. Preserve editor results, source projections, schema precedence, discriminant selection, array crossing, and passive analysis.

Success requires cold point requests to type only requested paths without initializing an inventory; diagnostics to classify each scalar once per snapshot; repeated siblings and recursive depths to reuse schema constructions; every specified semantic-input change to invalidate reuse; and all existing response contracts to remain intact. The existing A3 benchmark must show each expression and verb hover below 1,000,000 allocated bytes at both sizes in both existing cold/warm cases. Diagnostics must grow linearly with entries for the fixed benchmark schema. Required area checks must pass, with no L1 test exceeding five seconds.

Wave numbers are unique across the plan. Tasks within a wave may use concurrent subagents only where their file ownership is disjoint; waves run in order. Within a task, bullets are ordered by dependency. All implementation remains in this worktree. This document plans implementation; it does not change code or execute implementation gates.

### Decisions

1. **Two access patterns**
   - **Decision:** Provide one scalar classifier with separate point lookup and lazy document-inventory APIs. Point requests may consult an initialized inventory but must never initialize it; expression hover and lifecycle fallback share the cursor classification.
   - **Alternatives considered:** Initialize everything on first hover; recompute typing for every provider invocation.
   - **Why:** The specification requires cold hover independence from unrelated entries and cross-request reuse for diagnostics.
   - **Downsides:** Both access paths need parity coverage and explicit ownership boundaries.

2. **Shared schema identities**
   - **Decision:** Use snapshot-local owner-retaining handles with stable node selectors/IDs into immutable schema owners. Assign identity lazily as nodes are reached, including canonical named-definition targets; identify properties by their containing node and property position. Cache views by source node identity, crossing mode, and selected arm or ordered admissible-arm identities. Instance paths locate authored discriminants only.
   - **Alternatives considered:** Remove indices from path keys; serialize or clone definitions into keys; eagerly index the entire schema graph; use unsafe lifetime extension.
   - **Why:** Index removal mixes different sibling selections, content keys repeat the expensive copying, and eager indexing makes first hover visit unrelated schema nodes. Shared owners avoid self-referential structures while retaining property metadata.
   - **Downsides:** Handle access adds indirection; IDs are valid only inside their snapshot. Recursive definition targets must canonicalize to the same identity at every depth.

3. **Construction boundaries**
   - **Decision:** Borrow or retain existing single-arm, inline-object, and recursive shapes. Construct only effective-root assembly, genuine multi-arm merges, and synthesized item definitions, once per relevant snapshot key. Retain existing merge order, deduplication, metadata, and baseline/extension/trigger/document precedence.
   - **Alternatives considered:** Optimize nested descent alone; clone the leaf after borrowing its ancestors; merge even a single admissible arm.
   - **Why:** The specification explicitly includes root assembly, item synthesis, and leaf ownership in the allocation fix.
   - **Downsides:** Internal `Cow` and owned-shape consumers need signature changes. Real synthesis still allocates.

4. **Snapshot lifetime**
   - **Decision:** Keep one current `Arc` analysis per document in `OverlayCache`, handed to every fresh `DocumentOverlay`. Its identity includes URI, buffer revision and exact text identity, current/last-good tree state and paired source text, effective schema and named definitions, effective configuration, prepared trigger registry, and captured resolution context. Reuse existing preparation/dependency invalidation; replace on any changed input and remove on close. Outstanding requests may finish with an old owner.
   - **Alternatives considered:** Cache by version alone; put a fresh lazy cell on each request wrapper; retain a process-global cache or snapshot history.
   - **Why:** Request wrappers are rebuilt, imported schemas change without edits, and malformed text cannot safely reuse another buffer's decoded projections.
   - **Downsides:** The current inventory remains resident until replacement/close; old requests can temporarily extend an old snapshot's lifetime. Revision identity must reach overlay construction where it is currently implicit.

5. **Lazy state and passivity**
   - **Decision:** Store owned entry indices, shared definition handles, decoded strings, and projections, never references into sibling fields. Use a thread-safe once-initialized inventory plus short-lived synchronization for shape caches; compute without recursively entering inventory initialization or keeping the outer overlay lock across provider work. Preserve current malformed-buffer response behavior, deriving any projection only from the text/tree pairing that behavior permits.
   - **Alternatives considered:** Self-referential cached `ExpressionValue` records; provider-local inventories; cache evaluated runtime values.
   - **Why:** This meets the specification's shared lifetime and passive editor contracts without new I/O, evaluation, mutation, or context discovery.
   - **Downsides:** Request adapters must reconstruct borrowed views from stored indices; lock ordering requires focused tests.

6. **Scope and evidence**
   - **Decision:** Include point definition lookup as well as hover/completion; retain whole-document work for diagnostics and document links. Use deterministic work counters and the unchanged A3 measurement protocol; add no performance spike. Preserve existing loader and recovery semantics rather than introducing configuration validation rules.
   - **Alternatives considered:** Leave definition using a full navigation scan; widen performance investigation across hosts; add a new parser robustness matrix.
   - **Why:** Definition currently filters `nav_targets` only after typing the document, which conflicts with the specification's point-query rule. Existing profiling settles the mechanism. This change adds no file/configuration loader or field validator; Markdown authoring and internal caches are outside the requested input robustness matrix.
   - **Downsides:** Navigation needs a scalar-local adapter. One-host allocation results establish this mechanism's improvement, not arbitrary-schema complexity or cross-platform latency bounds.

### Wave 1: Baseline and response evidence

- [x] **Capture baseline**
  - Before source changes, run `cargo bench -p dmls --bench pipeline_work_a3` on this host with the current frozen input, feature/profile settings, existing repetitions, and `a3x1`/`a3x10` cold/warm cases. This is the required baseline measurement, not a design spike.
  - Create `implementation-log.md` alongside this plan, recording the command, build settings, input identity, and raw output location plus expression-hover, verb-hover, and diagnostics bytes for every case. Record benchmark order: overlay → diagnostics → completion → verb hover → expression hover.
  - Capture enough build/environment information to repeat the measurement exactly; use Sniff for host discovery. Do not substitute the spec's historical byte counts for this baseline.

- [ ] **Pin response contracts**
  - Independently inspect/reuse existing assertions in `providers/frontmatter/sequence_tests.rs`, frontmatter provider/diagnostics unit tests, and `dmls/tests/l1/lifecycle_event_views.rs` and `lifecycle_host_profile.rs`. Add only missing focused characterization cases, in separate test files from benchmark/log work.
  - Preserve exact hover text/ranges, completion order and edits, and diagnostic codes/ranges/messages/severities. Include definition targets and document links so their consumer split remains observable.
  - Use existing test-toolkit fixtures and consolidated test registration; keep baseline behavior assertions passing before changing ownership.

Checkpoint: baseline data exists for every required benchmark cell, and response fixtures establish the contracts later phases must preserve. No spike or human decision remains pending.

## Phase 2: Build shared snapshot typing

Prerequisite: Phase 1 baseline is saved. This phase owns `overlay/` cache integration and the schema resolver portion of `providers/frontmatter.rs`; keep these tightly coupled edits sequential.

### Wave 2: Snapshot and resolver foundation

- [ ] **Own analysis snapshot**
  - Add a focused overlay analysis module holding immutable source/schema owners, root state, node identities, shared-view/item caches, and the lazy inventory interface. Avoid eagerly expanding or traversing all named definitions.
  - Install/reuse its shared handle in `OverlayCache` and attach it to every relevant `DocumentOverlay` construction path, including no schema, schema failure, standalone documents, and malformed-buffer fallback. Preserve existing eligibility on each path.
  - Thread buffer revision from the existing document lifecycle into overlay preparation where necessary. Match exact buffer identity and tree provenance as well as the prepared semantic inputs listed in Decisions; reuse existing schema dependency checks for imports, including failed-assembly recovery.
  - Evict on edit and `forget`, and replace after configuration, registry, context, effective-schema, or named-definition changes. An old request's retained handle must never become the current handle again.

- [ ] **Share path definitions**
  - Implement the owner-retaining schema/property handles and source-node identities; retain canonical named-definition identity across recursive re-entry and distinguish equally named properties with different metadata.
  - Replace instance-path shape ownership in `ShapeMemo`/`descend`. Perform authored discriminant selection at each instance using the existing shared selector; cache only the resulting shared view using node/mode/arm identity. Cache failed resolutions without conflating typed literal values.
  - Assemble `known_shape` once per snapshot. Share existing nested shapes; memoize genuine merged shapes and `array_item_def` synthesis by source identity and crossing mode. Update `def_at_path_ctx` to return a retained definition without cloning a nested leaf.
  - Migrate compilation-facing consumers to handle access, removing `into_owned`/shape clones where they only inspect a definition. Review affected docs/comments, especially `ShapeMemo`, `def_at_path_ctx`, and array-item semantics.

- [ ] **Count actual work**
  - Extend `work_counters.rs` with separately observable root assemblies, merged-shape constructions, item-definition syntheses, scalar classifications, full typing walks, and whole-document stack-expression walks. Keep descent attempts separate if retained.
  - Replace the misleading test-only `SHAPE_MATERIALIZATIONS` meaning and update its existing assertions. Increment at construction/classification sites, not lookup attempts; avoid serializing or cloning cache keys for instrumentation.
  - If extending `WorkCounts`, update its snapshot/delta methods and explicit benchmark struct initializers/formatters so the existing benchmark still compiles and retains its measurement protocol.

Checkpoint: focused resolver tests prove same-arm sibling and recursive reuse, different-arm isolation, stable merge ordering, both crossing modes, one root assembly, and bounded item synthesis. Existing consumers compile without copying existing nested shapes.

## Phase 3: Separate point and document consumers

Prerequisite: Phase 2 shared handles and classifier interface are available. Wave 3 tasks can run concurrently because their implementation ownership is disjoint. Coordinate changes to shared interfaces before the wave starts.

### Wave 3: Extraction and inventory

- [ ] **Extract scalar expressions**
  - In `overlay/event_views.rs` and the scalar projection helper surface, add lifecycle extraction for one indexed scalar. Reuse existing interpolation parsing, guard detection, event scope, dialect, skip rules, and source projection.
  - Make `stack_expressions` delegate to the same scalar extraction semantics while retaining document order and current behavior for diagnostics. This path may enumerate the document; point extraction may not call `frontmatter_interpolations` for the whole document.
  - Add focused parity tests for a typed expression being skipped, multiple spans in one scalar, unwrapped guards, interpolation-containing guards, and non-stack values. Preserve plain/quoted/block/tagged scalar behavior.

- [ ] **Cache scalar inventory**
  - In the new analysis module, implement scalar classification by entry index using shared schema state. Store expression, non-expression, and unresolved outcomes, including undecodable/no-projection results, so misses are not retried.
  - Point classification resolves only that entry's path. The document API walks scalar entries once on first demand, reuses prior point classifications, and publishes a complete document-ordered inventory with stored decoded text/projections and shared definition owners.
  - Later document consumers reuse the initialized inventory. Expose a noninitializing inventory lookup for point consumers; no accessor used by hover/completion may trigger its lazy full walk.
  - Add focused tests for once-only initialization and matching point/document results, including repeated negative classifications. Do not cache evaluated expressions or resolved navigation I/O.

### Wave 4: Provider integration

- [ ] **Route point requests**
  - In `providers/frontmatter.rs`, locate the cursor entry once and share its classification between `expression_hover` and `stack_expression_hover`. The fallback invokes only the scalar extractor from Wave 3, maintaining existing hover precedence and key-versus-value behavior.
  - Route completion and schema hover through shared root/definition handles. Additional properties actually referenced by a hovered expression may be resolved; unrelated scalars may not be typed.
  - Split `definition` from the full `nav_targets` scan: preserve named-type and `$schema` reference handling, then classify/resolve only the scalar containing the offset. Keep existing file-reference resolution authority and target/range behavior.

- [ ] **Route document consumers**
  - After point routing edits in the same file are complete, adapt `expression_values` to the shared inventory and wire `diagnostics/frontmatter.rs` to reuse it, preserving mixed-expression-union validation, diagnostic ordering, and range projection.
  - Retain the whole-document stack-expression collector for diagnostics; a hover after diagnostics must never invoke it. Other existing diagnostic walks are permitted and are not counted as full typing walks.
  - Let document links consume the same typed scalar inventory for file-type decisions while keeping target resolution outside the cache. Eliminate repeated membership scans where the stored per-entry classification already supplies the answer.
  - Complete the caller migration against the inventory below; rerun the text search to catch new or indirect consumers rather than relying solely on these names.

### Consumer inventory

| Surface | Current callers | Required access |
| --- | --- | --- |
| Expression hover | `expression_value_at`, `expression_hover`, `stack_expression_hover` | One cursor classification; local lifecycle spans; never initialize inventory |
| Completion | `top_level_key_completions`, `nested_key_completions`, `value_completions`, `suggestion_completions`, `union_item_completions`, `object_item_key_completions` | Shared root and requested path/candidates; retain transparent/explicit crossing rules |
| Schema hover | `schema_hover`, `union_item`, `schema_property_details` | Shared path definition; resolve only requested/referenced properties |
| DSL expressions | `providers/dsl.rs::schema_property_hover` and `KnownRoots::with_catalogs` | Retain shared root; remove owned root copying in site typing |
| Code actions | `providers/code_actions.rs::add_missing_required_keys` | Inspect shared root; requested schema-property enumeration remains allowed |
| Definition | `definition` currently calls `nav_targets` | Cursor-local navigation, no full typing or link collection |
| Document links | `document_links` → `nav_targets` | Document inventory permitted; share classifications and schema state |
| Diagnostics | `diagnostics/frontmatter.rs` → `expression_values`, `stack_expressions` | Initialize inventory once; reuse on later requests |
| Tests | `sequence_tests.rs`, severity tests, provider helpers calling these APIs | Migrate ownership assumptions; retain behavioral expectations |

Checkpoint: cold hover, completion, and definition perform no full typing or stack-expression walk. Repeated diagnostics reuse classifications across fresh request wrappers, and document links share schema constructions without changing outputs.

## Phase 4: Prove reuse and semantic equivalence

Prerequisite: Phase 3 integrated consumers. Wave 5 tasks may run concurrently in separate resolver/provider tests and overlay/session test files; neither edits production APIs during this wave.

### Wave 5: Regression coverage

- [ ] **Prove bounded work**
  - Build a compact 200-item lifecycle fixture using existing frozen/test schema inputs. Before diagnostics, test expression hover, verb hover, a cursor outside typed scalar values, completion, and definition. Assert zero full typing/stack walks and no classifications of unrelated scalars.
  - Bound shape construction by distinct reached node/arm/mode identities plus at most one root, independently of item count. Compare identical-item diagnostics at N and 2N: constructions and item syntheses must not scale with identical instances; required entry classifications may scale once per scalar.
  - On the same snapshot, assert diagnostics → hover performs no full typing/stack walk, diagnostics → diagnostics adds no classifications/full typing walk, and point requests before diagnostics agree with inventory-backed results afterward. Include non-expression and unresolved misses.
  - Cover siblings choosing different arms; absent, unknown, duplicate, and conflicting discriminants; strings versus numbers versus booleans versus null; recursive references; both array crossing modes; merged-arm completion; mixed expression unions; and plain, quoted, block, and tagged values. Reuse existing assertions wherever adequate.

- [ ] **Prove snapshot invalidation**
  - Exercise reuse through fresh `DocumentOverlay` wrappers, not just a retained provider context. For edit, imported-schema change with unchanged buffer version, effective configuration change, trigger-registry change, captured-context change, and close/reopen, prove the next request gets fresh classifications/root state and correct public results.
  - Include identical-text edits with a new revision, schema-assembly failure followed by repair, malformed YAML with a last-good tree, and repaired YAML. Verify decoded text, hover/diagnostic ranges, and completions belong to the buffer their projection describes.
  - Retain an old request snapshot while installing a replacement; show old ownership remains usable but cannot contaminate the current result. Verify close removes the cache's strong owner and no history accumulates, using deterministic ownership observations rather than RSS thresholds.
  - Use effects/work counters around prepared analysis to show it adds no reads, network/shell effects, document mutation, or context discovery. Distinguish existing preparation/dependency checks from newly introduced analysis work.

Checkpoint: deterministic assertions cover each specification acceptance case, all result contracts remain unchanged, and each added L1 test finishes within five seconds. Fix failures in the implementation before proceeding; do not defer any required case.

## Phase 5: Measure, document, and close implementation

Prerequisite: Phase 4 passes. In Wave 6, documentation and allocation measurement may run concurrently with disjoint file ownership; final gates follow both.

### Wave 6: Allocation evidence and documentation

- [ ] **Verify allocation targets**
  - Repeat the Phase 1 A3 command on the same host, settings, frozen inputs, sizes, cold/warm modes, and repetitions. Keep benchmark ordering intact; explicitly report that diagnostics precedes both hovers.
  - In `implementation-log.md`, tabulate before/after expression-hover, verb-hover, and diagnostics bytes for every case, and retain allocations/peak-live/timing output as supporting evidence. Require every measured hover to allocate fewer than 1,000,000 bytes.
  - Assess diagnostics growth against entry counts and explain residual required per-entry output allocation. Use work counters to establish shared constructions; two fixed-schema sizes do not prove arbitrary-schema scaling. Cite cold point-request tests as the proof that cheap hover does not rely on prior diagnostics.
  - If a target fails, correct the remaining allocations in the affected paths and rerun the relevant evidence. Do not weaken thresholds, reorder the benchmark to conceal work, or move required scope into a follow-up. Preserve the final numbers for comparison by `2026-10-25-shared-acyclic-definitions`.

- [ ] **Document cache behavior**
  - Update `dmls/docs/hover.md`, `diagnostics.md`, and `autocomplete.md` for cursor-only typing, lazy document inventory, reuse across requests, and invalidation after buffer or schema-input changes.
  - Include a compact two-item example with different discriminants: items share schema storage while retaining distinct selected results. Add a Mermaid lifecycle diagram where it clarifies lazy initialization and replacement.
  - Update `.claude/skills/darkmatter/dmls.md` with ownership, handle lifetime, point-versus-document APIs, passivity, and invalidation rules; finish affected Rust comment/doc review. Keep topic docs self-contained, with no spec links or lifecycle-directory references.
  - Review README scope: public responses and commands remain unchanged, so add only a short overview update if existing overview claims have become inaccurate. No dependency changes are planned; if implementation needs one, update the relevant dependency documentation in the same change.

### Wave 7: Final validation

- [ ] **Run area gates**
  - In `darkmatter/`, run `just test`, `just test-l2`, and `just lint`, using nextest-backed recipes and the shared test toolkit. Run the integrated area gates once after concurrent edits are merged; repeat only for new changes, failures, or unresolved concerns.
  - Ensure L2 harnesses never activate terminal/browser windows. Do not run `cargo fmt`, `cargo test`, or workspace-wide gates. Preserve portability across macOS, Linux, native Windows, and WSL2 without platform-specific cache logic; load the OS skill before any platform/path-comparison changes or environment-specific investigation.
  - Record results, timings relevant to the five-second L1 limit, and any repaired failures in the implementation log. A denied/unavailable required check is an unmet gate to report, never a passing result.

- [ ] **Audit completion evidence**
  - Map every acceptance item to a named test, benchmark row, documentation change, or gate result. Verify there are no remaining production full-inventory initializers on point paths, nested-shape clones, path-index-keyed shared views, or request-local-only caches masquerading as cross-request reuse.
  - Record any implementation departure and its decision/alternatives/why/downsides in the implementation log; keep the finalized specification unchanged. Leave no open design question or requested scope deferred.
  - Mark completed tasks in this plan and report implementation complete, ready for review. Leave the fix directory active; do not move it to `_completed`, run `just complete`, commit, or push.

Checkpoint: all allocation targets, deterministic work/behavior tests, invalidation cases, documentation, and required area gates are evidenced. The implementation is ready for review.
