# Review: Compose Operations and Options Refactor

Reviewed against:

- `darkmatter/plans/2026-03-22. refactoring-compose-operations-and-options.md`
- `darkmatter/lib/src/markdown/compose/types.rs`
- `darkmatter/lib/src/markdown/compose/mod.rs`
- `darkmatter/lib/src/markdown/compose/transclusion/types.rs`
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs`
- `darkmatter/docs/darkmatter-compose-pipeline.md`
- `.claude/skills/darkmatter/SKILL.md`
- `.claude/skills/darkmatter/compose.md`

Validation run:

- `just test` in `darkmatter/`
- Observed library result: `1492 passed, 0 failed, 2 ignored`

## Findings and Recommendations

### P1: Concurrent cycle detection is currently too aggressive and will reject valid DAGs

Relevant code:

- `darkmatter/lib/src/markdown/compose/transclusion/types.rs:94-110`
- `darkmatter/lib/src/markdown/compose/transclusion/types.rs:126-167`
- `darkmatter/lib/src/markdown/compose/mod.rs:647-729`

The shared `active: Arc<Mutex<HashSet<String>>>` makes sibling work items treat "currently being rendered elsewhere" as a cycle. That is not equivalent to cycle detection.

Examples that should be valid but are likely to fail under parallel execution:

- a parent including the same child file twice
- a diamond graph where two siblings both include the same grandchild
- mixed frontmatter/directive reuse of the same target in the same compose run

Only ancestry repetition is a real cycle. Global "active anywhere" state turns legitimate shared dependencies into false-positive `CycleDetected` errors.

Recommendation:

- Remove global active-set membership from cycle errors and rely on per-branch ancestry (`stack`) for correctness.
- If cross-thread coordination is still needed, use shared state only for metrics or deduplication, not as proof of a cycle.
- Add explicit regression tests for duplicate sibling includes and diamond-shaped dependency graphs before changing the implementation.

### P1: The enum-defined pipeline order is not the actual runtime order

Relevant code:

- `darkmatter/lib/src/markdown/compose/types.rs:94-110`
- `darkmatter/lib/src/markdown/compose/mod.rs:223-291`

The refactor added `ComposeOperation::phase()` and `ComposeOperation::default_order()`, but `run_compose_pipeline_internal()` does not use them. The runtime order is still hard-coded.

That already caused drift:

- `default_order()` says transclusion order is `BlockTransclusion -> FrontmatterTransclusion -> CodeTransclusion -> TocLinking`
- the runner executes `TocLinking -> Block/Code -> Frontmatter`

This undercuts one of the main goals in the plan: a single authoritative enumeration of compose operations and their execution order.

Recommendation:

- Refactor the runner to iterate `ComposeOperation::default_order()` for non-transclusion phases and use a dedicated `run_transclusion_phase()` for transclusion operations.
- If transclusion needs a special executor, still derive the operation ordering from the enum rather than duplicating it in `mod.rs`.
- Add a test that asserts the runtime phase ordering matches `default_order()` so future additions cannot drift silently.

### P2: "Concurrent transclusion" is only partially implemented

Relevant code:

- `darkmatter/lib/src/markdown/compose/mod.rs:247-291`
- `darkmatter/lib/src/markdown/compose/mod.rs:512-770`
- `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs:4-8`
- `darkmatter/docs/darkmatter-compose-pipeline.md:7-12`
- `darkmatter/docs/darkmatter-compose-pipeline.md:90-97`

The plan called for a unified transclusion phase that would collect block, frontmatter, code, and TOC-linking work and resolve it concurrently. The implementation only parallelizes block/code directives inside `run_block_transclusion_stage()`.

Current state:

- `::file` and `::code` are parallelized
- `prologue` / `epilogue` are still resolved serially
- `::toc-linking` is still resolved serially
- `toc_linking/mod.rs` still documents itself as a Stage 1 step

This is not inherently wrong, but it is narrower than the plan and narrower than the docs currently claim.

Recommendation:

- Either finish the phase-level refactor so all transclusion operations flow through one concurrent collection/apply path, or
- explicitly narrow the documentation so it says "block/code transclusion is parallelized" instead of "transclusion is concurrent"

If kept partial, the code comments in `compose/mod.rs` and `toc_linking/mod.rs` should be updated together so the behavior is described consistently.

### P2: Shell approval/runtime state does not actually persist across concurrent child composes

Relevant code:

- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:335-345`
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:403-415`
- `darkmatter/lib/src/markdown/compose/mod.rs:678-695`

`ShellExpansionRuntime::clone_for_child()` copies allow-once and policy state into each child, but `PipelineRuntime::merge_child()` only merges transclusion depth stats back to the parent. Any state changes made inside concurrent child composes are dropped when the child finishes.

Practical consequences:

- sibling transclusions can prompt for the same approval more than once
- allow-once decisions made in one child do not benefit siblings
- policy loading work can be repeated unnecessarily

This is also at odds with the runtime comments that say approvals persist across recursive transclusion.

Recommendation:

- Decide whether shell approval state should be branch-local or compose-global.
- If compose-global is the intended behavior, share shell runtime behind synchronized shared state and merge approval counts deterministically.
- If branch-local is acceptable, update the comments and docs to stop promising cross-child persistence.

### P2: Documentation sync is incomplete

Relevant docs:

- `.claude/skills/darkmatter/SKILL.md:37-67`
- `.claude/skills/darkmatter/compose.md:7-27`
- `.claude/skills/darkmatter/compose.md:163-191`
- `darkmatter/docs/darkmatter-compose-pipeline.md:48-55`
- `darkmatter/lib/src/markdown/compose/mod.rs:6-20`
- `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs:4-8`

There is still substantial terminology and behavior drift:

- the skill file still describes a two-stage pipeline instead of the new three-phase model
- the skill docs still say transclusion is fully concurrent
- `compose.md` still describes `fail_fast: true` as "log and continue" even though interpolation and structural transclusion errors can return immediately
- the `ComposeReport` snippet in `compose.md` is stale and omits newer report fields
- the pipeline docs still advertise Link Validation as part of the pre-op pipeline, while the implementation does not ship it yet
- code comments disagree on whether TOC linking is Stage 1, transclusion, serial, or concurrent

Recommendation:

- Treat docs sync as unfinished, not complete.
- Make one source of truth for terminology: `Inline Pre`, `Transclusion`, `Inline Post`.
- Document Link Validation explicitly as deferred/unimplemented until the feature lands.
- Update `compose.md` error handling semantics to match the actual code paths.

### P3: The flattening goal is only partially complete at the public API boundary

Relevant code:

- `darkmatter/lib/src/markdown/compose/types.rs:410-447`
- `darkmatter/plans/2026-03-22. refactoring-compose-operations-and-options.md:606-609`

`ComposeOptions` is now flat, which is the right direction. But the old nested model is still partially exposed:

- `with_shell()` still accepts a `ShellExpansionOptions` struct
- `ShellExpansionOptions` remains part of the public shell-expansion API
- `TransclusionOptions` survives as an internal convenience type rather than being deleted as originally planned

This is not a correctness bug, but it means callers still have to know about the old nested mental model in some places.

Recommendation:

- Prefer direct builders on `ComposeOptions` for the common flat fields (`with_shell_timeout`, `with_shell_policy_root`, `with_allow_remote_transclusion`, etc.)
- Keep internal projection types if they help implementation, but avoid exposing them as the primary way to configure compose behavior

## Test Coverage Gaps

### 1. Order tests do not prove the order that matters

Relevant tests:

- `darkmatter/lib/src/markdown/compose/mod.rs:1233-1249`
- `darkmatter/lib/src/markdown/compose/mod.rs:2570-2587`

`test_compose_stages_run_in_order()` only verifies that some stages ran. It would not fail if the transclusion order changed again.

`page_block_coexists_with_interpolation()` also does not prove ordering. The current assertion passes whether interpolation runs before or after page blocks because the condition uses state, not interpolated body content.

Recommended additions:

- a test that proves `PageBlocks` runs before `ShellExpansion` by placing a shell directive inside a false block and asserting it never executes
- a test that proves cleanup happens after transclusion by asserting spacing normalization occurs on inserted child content
- a test that proves the transclusion runtime order matches `ComposeOperation::default_order()`, or alternatively a test that explicitly codifies the intended transclusion sub-order

### 2. No tests cover duplicate or shared-dependency transclusion graphs

Recommended additions:

- same child included twice from one parent
- two siblings both including the same grandchild
- same file referenced once by `prologue` and once by `::file`
- repeated `::code` includes of the same file under parallel execution

These are the minimum regression tests needed before keeping the current parallel transclusion strategy.

### 3. No tests cover concurrent shell-state propagation across child documents

Recommended additions:

- two included children both requiring approval for the same `::shell` command
- allow-once approval in one child suppressing a second prompt in a sibling child
- approval counts in child reports aggregating correctly at the parent

### 4. Enum and option metadata lack direct tests

Recommended additions:

- `ComposeOperation::default_order()` exact contents
- `ComposeOperation::phase()` mapping for every variant
- `ComposeOperation::all()` completeness
- consistency between `default_order()` and the actual executor behavior

### 5. Documentation-facing behavior needs coverage too

Recommended additions:

- a doc test or integration test that matches the published compose pipeline examples
- a test that codifies current `fail_fast` behavior for interpolation, transclusion structural errors, and TOC-linking errors
- a test or comment asserting that Link Validation is currently deferred so docs and code do not drift again

## Ergonomic and Performance Recommendations

### 1. Replace `HashSet<ComposeOperation>` with a fixed-size operation set

Relevant code:

- `darkmatter/lib/src/markdown/compose/types.rs:11-12`
- `darkmatter/lib/src/markdown/compose/types.rs:113-115`
- `darkmatter/lib/src/markdown/compose/types.rs:154`

`ComposeOperation` has a tiny fixed cardinality. Using `HashSet` adds allocation, hashing, and less predictable debug ordering for something that is really a bitset.

Recommendation:

- use `enumset`, `bitflags`, or a small fixed boolean array keyed by discriminant
- keep `disable()`, `only()`, and `is_enabled()` as the public API so callers do not pay the complexity cost

This should make `ComposeOptions::new()`, cloning, and hot-path membership checks cheaper and simpler.

### 2. Introduce per-compose memoization for repeated target work

Even after correctness is fixed, the current parallel model will re-read and re-compose the same target multiple times within one parent document.

Recommendation:

- add an internal per-compose cache keyed by target identity plus the transclusion options that materially affect output
- start with TOC extraction and local markdown transclusion, where repeated work is most obvious

This was explicitly out of scope for the original plan, so it should be treated as a follow-on optimization after correctness issues are resolved.

### 3. Unify transclusion preparation and application paths

Right now transclusion behavior is split across:

- TOC linking
- block/code transclusion
- frontmatter transclusion

Recommendation:

- normalize each transclusion source into a common "prepared operation" type
- resolve them with one executor
- apply them through one report-aggregation path

That will simplify correctness reasoning, make concurrency behavior consistent, and reduce the risk of one transclusion kind having different warning/fail-fast semantics from the others.

### 4. Clarify whether TOC linking operates on raw source or composed output

Relevant code:

- `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs:56-64`

`process_toc_linking()` reads the target file and computes a TOC from raw markdown. It does not run the target through compose first.

Recommendation:

- either document that TOC linking is intentionally based on raw source, or
- change it to compose the target document before extracting headings if the desired mental model is "TOC of the document after preparation/transclusion"

This matters more now that TOC linking has been repositioned conceptually into the transclusion phase.

## Suggested Next Steps

Recommended implementation order:

1. Fix the cycle-detection model and add graph-shape regression tests.
2. Make runtime order derive from `ComposeOperation` metadata or collapse the metadata back to a single authoritative place.
3. Decide whether transclusion is truly phase-level concurrent or only directive-level concurrent, then align code and docs.
4. Fix shell runtime propagation semantics for concurrent children.
5. Finish docs sync and flattening cleanup.
