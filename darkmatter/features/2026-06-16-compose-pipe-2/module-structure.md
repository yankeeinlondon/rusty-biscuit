# Compose Pipeline v2 — Module Structure

The structural counterpart to `tech-design.md`: how to restructure the compose modules **while** implementing the pre-flight redesign. Reviewed and agreed. It deliberately scopes to the areas v2 touches and flags god-file decomposition we can do opportunistically along the way. See `tech-design.md` § *Module Structure Alignment* for the behavioral constraints this structure must satisfy.

## Why now

The pre-flight redesign introduces a genuinely new concept — an **approval set** distinct from an **execution set** — plus per-compose command caching. Bolting those onto the current layout would deepen two existing god files instead of paying down the debt. Since we have to open these files anyway, this is the cheapest moment to carve cleaner seams.

## Current state — the god files

Line counts for the compose subtree (top offenders):

| Lines | File | What's tangled inside |
|------:|------|-----------------------|
| 6919 | `compose/mod.rs` | module decls + public `compose*` API + pipeline driver + per-stage dispatch + 5 stage impls + the **entire transclusion engine** + ~4000 lines of tests |
| 3371 | `compose/frontmatter_shell_expansion.rs` | frontmatter `$(...)` parse + interpolate + approve + execute + ternary branch handling |
| 3233 | `compose/types.rs` | `ComposeOptions`, `ComposeOperation`, `ComposeReport`, `ComposeSource`, warnings, ordering table |
| 2256 | `shell_expansion/mod.rs` | `prepare_directive` (policy gate) + `execute_prepared_directive` (runner) + helpers, all in one |
| 1362 | `shell_expansion/types.rs` | directive types, runtime, entry types |
| 1265 | `shell_expansion/discovery.rs` | `collect_shell_commands` graph walk |
| 1175 | `shell_expansion/executor.rs` | process execution, timeout, error handling |
| 1145 | `compose/conditions.rs` | shared `when=` evaluator (page blocks + transclusion) |

The single worst offender is `compose/mod.rs`. Everything hangs off `impl Markdown` (line 425 onward): the pipeline driver (`run_compose_pipeline_internal`), the four phase runners, five stage implementations (`run_replacement_stage`, `run_interpolation_stage`, `run_normalization_stage`, `run_shell_expansion_stage`, `run_page_blocks_stage`), **and** ~1200 lines of transclusion preparation/resolution/rendering (`prepare_*_transclusions`, `resolve_prepared_transclusion`, `render_markdown_transclusion`, `render_code_transclusion`, …). The orchestrator, the stages, and the transclusion engine are one type's method bag.

## What v2 actually touches

Mapping the tech-design's work to today's files:

1. **Pipeline orchestration** — insert pre-flight as a real stage; keep execution in-stage but membership-checked. → `compose/mod.rs` (driver + dispatch loop).
2. **Command discovery (condition-blind)** — collect the approval set across frontmatter, body, graph. → `shell_expansion/discovery.rs`, `frontmatter_shell_expansion.rs`.
3. **Approval** — batch validate + prompt once; expose the set across the orchestrator boundary. → `shell_expansion/mod.rs` (`prepare_directive`), `shell_expansion/policy.rs`, `shell_expansion/store.rs`, `claudine/.../preflight.rs`.
4. **Execution (condition-aware) + caching** — run reachable commands, membership-checked, memoized per compose. → `shell_expansion/executor.rs`, `shell_expansion/mod.rs`, possibly `cache/`.
5. **Condition handling** — blind for approval, aware for execution. → `conditions.rs`, `page_blocks/`, `transclusion/` condition sites.

So v2's blast radius is the orchestrator in `mod.rs` plus the entire `shell_expansion/` subtree plus `frontmatter_shell_expansion.rs` — three of the four biggest god files.

## Structural principles (POV)

1. **Name modules after the spec's verbs.** The redesign's vocabulary is *collect → validate → approve → execute → cache*. Modules should read back those stages so the code matches the design doc.
2. **Separate "could run" from "will run."** The approval-set and execution-set distinction should be a module boundary, not a flag threaded through one function.
3. **Stages are functions over content/state, not methods on `Markdown`.** Moving `run_*_stage` off `impl Markdown` decouples stages from the document type and makes each independently testable.
4. **The orchestrator is thin.** `mod.rs` should drive phases and own the public API; it should not *contain* stage logic or the transclusion engine.
5. **Touch-don't-rewrite.** Decompose only what v2 forces us into, plus the single highest-value extraction (transclusion out of `mod.rs`). Leave `expression/`, `context/`, `cache/` internals alone.

## Adopted top-level frame

A capability-first mind-map is the agreed top level (what Darkmatter *does*, not how files sit):

```
Darkmatter Library
├── delta            # document comparison
├── hash             # Markdown-aware hashing
├── render           # → components, render_tree, highlighting
└── compose          # → schema, preflight, inline, transclusion
    ├── schema
    ├── preflight
    ├── inline        # interpolation, page_blocks, shell_expansion, file_links, toc_links, normalize
    └── transclusion  # block_, frontmatter_, code_; (summarization, consolidation — not ready)
```

Two rules govern this frame:

- **Organize by nature, not by phase.** `file_links`/`toc_links` group under `inline` because of *what they are* (in-document link rewrites), even though they currently run in the Transclusion phase. Execution order lives in the `default_order`/phase table, never implied by the tree.
- **Capability map ≠ implementation map.** Cross-cutting infrastructure (the shared expression/conditions core) and the orchestrator spine are *not* nodes on this map; they are captured below as the Pipeline Domain Pattern.

The implementation tree below represents frontmatter transclusion as
`transclusion/frontmatter.rs`.

## Pipeline Domain Pattern

Darkmatter has **two pipeline domains** — `compose` and `render` — and both share one internal shape. The orchestrator is not a stage hanging off a domain; it *is* the domain's trunk. Each pipeline domain decomposes into three roles:

1. **Driver** — sequences the stages (the "orchestrator").
2. **Context** — the shared state threaded through every stage.
3. **Stages** — the operations the driver runs over the context.

Today only `render` names any of this; `compose`'s driver/context are buried in the `mod.rs` god file. The target makes both symmetric:

```
compose/                            render/
  pipeline/   driver  ← sequencer     render_tree/pipeline.rs       driver ← entrypoints
  context/    state   ← Effective-    render_tree/build_context.rs  state
              State / options /
              report / runtime
  stages/             operations      render_tree/fold.rs +         operations
    schema, preflight,                  *_extension, code_renderer,
    inline, transclusion                structural_gate, highlighting
```

Why this matters:

- **Altitude discipline.** A "driver" leaf next to `inline` would mix the coordinator with the coordinated. The driver is the trunk; stages are the branches.
- **Learn-once symmetry.** Once a reader understands how `compose` sequences stages over a shared context, `render` reads the same way. Two domains, one mental model.
- **Naming precedent already exists** — `render_tree/pipeline.rs` (114 lines) and `render_tree/build_context.rs` (1243). Compose should mirror with `compose/pipeline` + `compose/context` (its `EffectiveState`/options/report), extracted from `mod.rs`.

This pattern is the implementation-level answer to "where does the orchestrator go": inside each domain as `pipeline` + `context`, never as a sibling of the stages.

## Proposed target structure

Laid out to the Pipeline Domain Pattern — `compose` = **driver** + **support** + **stages** — and to the by-nature frame. Three groups inside `compose`:

```
compose/
├── mod.rs                       # THIN facade: compose/compose_with/compose_mut + re-exports
│
│   ── DRIVER (the orchestrator spine, lifted from mod.rs) ──
├── pipeline/
│   ├── mod.rs                   # run_compose_pipeline_internal: phase sequencing
│   ├── phases.rs                # InlinePre / Transclusion / InlinePost / Finalization dispatch (no Stage trait)
│   └── operations.rs            # ComposeOperation + ComposePhase + descriptors + OperationSet +
│                                #   default_order — the registry/ordering (from types.rs 27–491)
│
│   ── SUPPORT (shared within compose; used ACROSS stages, not a stage) ──
├── context/                     # shared state threaded through stages (from state.rs + types.rs)
│   ├── effective_state.rs       # EffectiveState + Builder + ResolvingLookup + merge helpers (state.rs)
│   ├── options.rs               # ComposeOptions + ComposeSource + TransclusionOptions
│   ├── runtime.rs               # ComposeContext (the ctx namespace)
│   └── report.rs                # ComposeReport + SourceRange + ComposeWarning
├── shell/                       # shell PRIMITIVES (renamed from shell_expansion/):
│   ├── directive.rs             #   parse + directive types (from parser.rs, types.rs)
│   ├── tokenize.rs              #   used by BOTH preflight (collect) AND inline (execute)
│   ├── alias.rs
│   ├── policy.rs                #   blacklist/whitelist matching
│   └── store.rs                 #   whitelist/blacklist files
├── conditions.rs                # when= evaluator: preflight SKIPS it; stages HONOR it
├── expression/                  # expr engine (unchanged internals) — interpolation/conditions/ternary
└── cache/                       # run-local + persistent caches (unchanged internals)
│
│   ── STAGES (the four families from the frame; impls moved off `impl Markdown`) ──
├── schema/                      # validation + coercion (from schema_validation.rs)
├── preflight/                   # NEW — approval-set lifecycle
│   ├── mod.rs                   #   collect → validate → approve
│   ├── collect.rs               #   condition-BLIND walk → approval_set (uses shell:: primitives)
│   └── approval.rs              #   batch whitelist/blacklist + single prompt; boundary export
├── inline/                      # in-document operations (by nature)
│   ├── interpolation.rs         #   (run_interpolation_stage)
│   ├── replacement.rs           #   (run_replacement_stage + existing replacement.rs)
│   ├── page_blocks.rs           #   (run_page_blocks_stage; thin over page_blocks/)
│   ├── file_links.rs
│   ├── toc_links.rs
│   ├── normalize.rs             #   (run_normalization_stage; Inline Post in phase terms)
│   ├── link_resolve.rs
│   ├── shell_expansion.rs       #   EXECUTION: condition-aware runner + membership gate
│   └── shell_cache.rs           #   NEW — per-compose command-keyed memoization (+ --no-cache)
└── transclusion/                # graph composition
    ├── engine.rs                #   NEW — TransclusionEngine struct (prepare/resolve/render from mod.rs)
    ├── block.rs / frontmatter.rs / code.rs
    └── …
```

Three notes that fall out of the frame:

- **Shell is cross-cutting, not "just an inline op."** Its *primitives* (tokenize/parse/policy/store) feed both `preflight/collect` (discovery, condition-blind) and `inline/shell_expansion` (execution, condition-aware). So `shell/` becomes shared **support**, and only the *execution* half lives under `inline/` as a stage. This is the concrete payoff of separating the approval set from the execution set — they share primitives but are different stages.
- **`context/` is the named home for the state** the driver threads — the `EffectiveState`/`ComposeOptions`/`ComposeReport` currently spread through `mod.rs`/`state.rs`/`types.rs`. This is the compose half of the driver+context pair the render side already has in `build_context.rs`.
- **`conditions.rs` stays one shared evaluator** — the blind/aware split is in the *callers* (preflight skips it; inline/transclusion honor it), not in two copies.

## Proposed decisions (was: open questions)

1. **Pre-flight ownership → inside `compose` as a producer, not the authority.** `preflight/collect` lives in compose (it needs interpolation + graph traversal only compose can do) and emits `approval_set`. Authorization stays at the boundary: the caller (Claudine) merges harness commands and approves, then hands the merged set back as the execution membership source. Keeps the "Darkmatter discovers, the caller authorizes" boundary intact.
2. **`shell_expansion/` → `shell/` (primitives) — do the rename, no shim.** The module's *role* changes from "a monolithic stage" to "shared primitives consumed by two stages," so the rename communicates something true. Execution moves to `inline/shell_expansion.rs`. **Claudine adapts to the new paths directly** (`claudine/.../preflight.rs` and composition imports) — no re-export shim; one clean sweep, honest history.
3. **Stages = free functions, no `Stage` trait.** `ComposeOperation` already is the registry/ordering. Stage impls become free functions taking `(&mut Markdown, &EffectiveState, &ComposeOptions, …)`. Dropped the `pipeline/stage.rs` trait from the earlier draft.
4. **Caching → self-contained `inline/shell_cache.rs` first.** Plain per-compose `HashMap<NormalizedCommand, CachedOutput>`. `cache::RunLocalCache` is content-hash single-flight for transclusion artifacts — adjacent but differently shaped; revisit convergence later, don't force it now.
5. **Transclusion → a `TransclusionEngine` struct.** Hold `runtime`/`report`/`options` as fields instead of threading them through a dozen `&mut Markdown` methods — that threading is most of why those methods are long. Biggest single reduction to `mod.rs`.
6. **`frontmatter_shell_expansion.rs` → minimal surgery for v2.** Extract the *collection* half into `preflight/collect`; leave frontmatter execution in place initially. Full split into `inline`/`shell` deferred to a follow-up so v2 isn't blocked on it.

## Migration approach

Incremental, interleaved with v2 — never a big-bang refactor. Ordered by value-to-risk:

1. **Extract the transclusion engine** from `mod.rs` into `transclusion/engine.rs` as `TransclusionEngine` (behavior-preserving). Biggest god-file reduction, de-risks everything after.
2. **Lift the driver** into `compose/pipeline/` and the state into `compose/context/` (behavior-preserving). Names the spine.
3. **Introduce `preflight/`**; route `collect_shell_commands` through `preflight/collect` with condition-blind walking. First v2 behavior change.
4. **Rename `shell_expansion/` → `shell/` primitives**; move execution to `inline/shell_expansion.rs`; add `inline/shell_cache.rs`.
5. **Lift remaining `run_*_stage` methods** into `inline/` / `schema/` as free functions.
6. **Slim `mod.rs`** to the public API + re-exports once its tenants have moved out.

Each step is independently shippable and testable; v2 behavior lands as the modules take shape rather than after a monolithic rewrite.

### Tech-design alignment

This structure is the *where*; [`tech-design.md`](./tech-design.md) is the *what/why*. The join — every migration step paired with the functional requirements and acceptance tests it delivers, plus a behavior-preserving/changing flag — lives in [`plan.md`](./plan.md). Quick map:

| Step | Tech-design payload | Behavior |
|------|---------------------|----------|
| 1 — `TransclusionEngine` | § Reusing the collection walk (graph metadata reuse) | preserving |
| 2 — driver/context extraction | (none — pure refactor) | preserving |
| 3 — `preflight/` collect | condition-blind approval, `DynamicCommandShape`, `compose_preflight` API, orchestrator boundary, doc reconciliation | changing |
| 4 — `shell/` rename + execution + cache | membership-gated execution, cache-by-default + `--no-cache`/`::no-cache`/`no_cache=true`, Claudine import sweep | changing |
| 5 — lift remaining stages | (none — pure refactor) | preserving |
| 6 — slim `mod.rs` | (none — pure refactor) | preserving |

The § *Module Structure Alignment* constraints in `tech-design.md` are binding on this structure; `plan.md` carries the full requirement/test traceability matrix.

## Explicitly out of scope

- `expression/` (10.5k lines), `remote*`, `toc_linking/` / `file_links/` internals — large but their *internals* are untouched by v2 (they move homes, not guts). Leave the guts alone.
- `compose/types.rs` (3233) — fully decomposed in step 2: `ComposeOptions`/`ComposeReport`/`ComposeContext` → `context/`; the `ComposeOperation`/`ComposePhase` registry → `pipeline/operations.rs`; perf types → existing `perf.rs`.

## Resolved follow-ups

- **Rename churn** — `shell_expansion/ → shell/` proceeds **without a shim**; Claudine adapts to the new paths in the same change.
- **`context/` granularity** — **four files**, split at the natural clusters in today's `state.rs` + `types.rs`: `effective_state.rs`, `options.rs`, `runtime.rs`, `report.rs`. A single `context/mod.rs` would recreate a ~2900-line god file; the clusters have distinct churn rates. The `ComposeOperation`/`ComposePhase`/`default_order` registry is *not* context — it moves to `pipeline/operations.rs`.
- **Render symmetry** — **compose first, then render**. Prove the driver/context/stages shape in `compose`, then apply the same naming to `render_tree/` (`pipeline.rs`/`build_context.rs` already half-exist there) as a follow-up.
