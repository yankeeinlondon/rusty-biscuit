---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/spec.md"
plan: "darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_diagnostics.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_fault_interpolation_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_fault_item_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_fault_shape_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_faults.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_key_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_value_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_loop_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_navigation.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_plain_and_silent_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_property_hovers.json
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - dmls
human_review: false
source_files_during_phase_2:
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/benches/pipeline_work_a3.rs
    - darkmatter/dmls/tests/l1/recursive_schema_editor.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_3:
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
message_to_agent: |-
    Phase 3 split point and document typing. FrontmatterAnalysis now stores
    a ScalarTyping per scalar entry index (file arm; ExpressionTyping
    No/Undecodable/Decoded{text, projection}); providers/frontmatter.rs has
    the one classifier (classify_scalar), the point lookup
    scalar_typing(ctx, index) (never initializes the inventory), the
    document lookup scalar_inventory(ctx) (one typing_walks per snapshot),
    and cursor_scalar(ast, offset). hover and definition are cursor-local;
    event_views::scalar_stack_expressions extracts one scalar's lifecycle
    expressions and stack_expressions (diagnostics only) delegates to it.
    expression_values(ctx) and nav_targets(ctx) lost their ast parameter.
    Counters: scalar_classifications is counted only when a record is built,
    typing_walks only when the inventory is built, stack_walks only by
    stack_expressions. Phase 3 tests already cover cold hover/definition
    zero-walk and reuse on one fixture (sequence_tests
    point_typing_never_starts_the_inventory_and_the_inventory_reuses_it,
    definition_types_only_the_cursor_scalar, and event_views parity); Phase 4
    should extend rather than duplicate them. Interim A3
    (phase3-a3.tsv): hovers are now size-independent with zero walks, but
    expression hover is 3.79 MB and verb hover 9.5 MB warm / 17.9 MB cold,
    still above the 1 MB target; the remaining cost is outside typing
    (LifecycleHost::for_document, dsl::with_site_scope, dsl::host_catalogs,
    hover markdown) and Phase 5 must remove it. Phase 2 is still
    uncommitted in this worktree.
---

# Implementation Log for 2026-10-26-dmls-frontmatter-typing-allocation (5 phases)

## Phase 1

### Baseline A3 measurement

Taken before any source change, at commit `8aac9b7e64898cd7802405c35a4848d29905e2fa`
(branch `feat/pipeline`, working tree clean apart from fix-directory Markdown).

- **Command** (repository root): `cargo bench -p dmls --bench pipeline_work_a3 --color=never`
- **Build settings:** the default `bench` profile (inherits `release`; the root
  `Cargo.toml` overrides only `dev`, `test`, and `profiling`); no `RUSTFLAGS`
  or `CARGO_*` environment overrides; no feature flags; no
  `A3_PROVIDER_BREAKDOWN`. Toolchain `rustc 1.98.1 (48a229cea 2026-09-01)`,
  `cargo 1.98.1 (797e8a9bc 2026-08-05)`.
- **Host** (via `sniff os` / `sniff hardware`): `shazam.home`, macOS 27.2
  (Darwin 27.2.0), Apple M4 Max, 16 cores, arm64, 128 GB.
- **Input identity** (SHA-256, frozen bundle last changed in `79403b467`):
  - `darkmatter/lib/benches/fixtures/pipeline-work/implement-plan.md`
    `37ed7aa0785d27d507bb147c2972e61389f00c7e8cb72b6b3ecf9d9c3e4495de`
  - `darkmatter/lib/benches/fixtures/pipeline-work/globals.lifecycle.json`
    `3e42c2c9d66e25496880051a7a07c175feab9f4a078880c3f18174135ff395b1`
  - `darkmatter/lib/benches/fixtures/claudine-schemas/`: action `2a250d9d…`,
    claudine `9817beb7…`, globals `4a5df458…`, inline-compose `ff7d9165…`,
    lifecycle `3f3a4b61…`, linking `414a73e6…`, review `e2c5c417…`,
    sequence-types `b21f920e…`, sequence `606ca7a7…`
- **Protocol:** unchanged bench — `a3x1` and `a3x10`, each warm and cold, one
  priming loop, one counted loop (the bytes below), then `TIMED_RUNS = 5`
  timed loops for the medians. Phase order within a loop:
  **overlay → diagnostics → completion → verb hover → expression hover**, so
  diagnostics runs before both hovers.
- **Raw output:** `baseline-a3.tsv` beside this log (the bench's full TSV:
  work counters, allocations, bytes, peak live bytes, median ns). Wall time
  of the whole run: 14.8 s.

| Case | Expression hover bytes | Verb hover bytes | Diagnostics bytes |
| ---- | ---------------------: | ---------------: | ----------------: |
| a3x1-warm  | 144,793,776 | 154,998,355 |   525,453,337 |
| a3x1-cold  | 144,793,776 | 163,339,067 |   928,335,132 |
| a3x10-warm | 966,693,644 | 976,898,223 |   970,158,217 |
| a3x10-cold | 966,693,644 | 985,238,935 | 1,373,040,012 |

Supporting values from the same run: a3x10 hover peak live bytes 417,936,523;
completion 17.9 MB at every size (already size-independent); the a3x1-warm
expression hover allocated 749,516 times with a 24.3 ms median.

### Response contracts

Existing assertions reused as-is (no change needed; they already pin behavior
later phases must keep):

- `dmls/src/providers/frontmatter/sequence_tests.rs`: typed descent through
  array indices, crossing modes (`union_arms_are_chosen_by_array_crossing`,
  `transparent_crossing_serves_key_only_authoring_ancestry`), discriminated
  arm selection inside items, shipped Claudine predicate typing, list-item
  hover/completion/navigation boundaries, and list-item expression
  diagnostics. `expression_pass_materializes_each_distinct_ancestor_shape_once`
  asserts the old `SHAPE_MATERIALIZATIONS` meaning that Phase 2 replaces.
- `dmls/tests/l1/lifecycle_event_views.rs` (per-event binding views in
  diagnostics, hover, completion) and `lifecycle_host_profile.rs` (host
  profile call diagnostics by line).
- `recursive_schema_editor.rs` and `union_item_editor.rs` (recursive
  definitions and union items through a real LSP session).

What was missing: an exact pin of every response on the large lifecycle
document the allocation fix targets, and of definition/document-link
output on it (the consumer split Phase 3 makes). Added
`dmls/tests/l1/frontmatter_response_contracts.rs` (declared in
`tests/l1/main.rs`), which drives the provider registry in-process exactly
as the A3 bench does (frozen document and frozen Claudine bundle, read
through `include_str!` inside each test), and compares portable JSON
against goldens in `dmls/tests/fixtures/response_contracts/`
(regenerate with `DMLS_UPDATE_GOLDEN=1` only for an intended change):

| Test | Pins |
| ---- | ---- |
| `a3_lifecycle_value_hovers_are_unchanged` | the bench's verb (`ensure_file`) and expression (`file_exists(`) hover positions, and a guard's expression value |
| `a3_lifecycle_key_hovers_are_unchanged` | a guard's `when` key and its `then` key (merged-arm hover) |
| `a3_loop_hovers_are_unchanged` | the `loop` block key, its `until` expression, and a `break` item's `reason` key |
| `a3_property_hovers_are_unchanged` | a schema-typed `file` key and its expression value |
| `a3_plain_and_silent_hovers_are_unchanged` | plain and empty-string values, and three positions that must stay silent (item marker, comment, body) |
| `a3_completions_are_unchanged` | 5 positions; every item's label, kind, detail, sort/filter text, insert text and edits, in order (documentation prose excluded) |
| `a3_diagnostics_are_unchanged` | every diagnostic's code, range, message, severity, source, and data; a second publish on the same context is identical |
| `a3_lifecycle_fault_diagnostics_and_completions_are_unchanged` | injected lifecycle faults' diagnostics: malformed guard, unknown verb (`anyOf`), undeclared read in a verb interpolation, `err` unavailable in `initialize`, guard without `then`, blank stack item, mistyped boolean; plus completions at a blank key inside a guard item (merged-arm keys), a blank stack item (verbs), and a blank top-level line |
| `a3_fault_item_hovers_are_unchanged` | hovers on the unknown verb, the undeclared read, and the `err` read |
| `a3_fault_shape_hovers_are_unchanged` | hovers on the guard without `then` and the mistyped boolean |
| `a3_fault_interpolation_hovers_are_unchanged` | hovers inside verb and message interpolations (the lifecycle fallback path) |
| `a3_navigation_is_unchanged` | definition at 7 positions (two file values with targets on disk, file-typed key and expression, verb value, schema type, body) and the full document-link list |

Hovers are split into tests of at most three positions (five for the cheap
silent ones). The first Windows cross-check showed a 15-hover test at 7.3 s
and a 7-hover test at 7.3 s, over the L1 budget, because each pre-fix hover
types the whole document. After the split, each golden's content was checked
to be identical to the one it came from.

Requirement → test mapping for this phase: baseline bytes → `baseline-a3.tsv`
plus the table above; exact hover text/ranges → the eight `*_hovers_are_unchanged` tests; completion order and edits →
`a3_completions_are_unchanged`,
`a3_lifecycle_fault_diagnostics_and_completions_are_unchanged`;
diagnostic codes/ranges/messages/severities → `a3_diagnostics_are_unchanged`,
`a3_lifecycle_fault_diagnostics_and_completions_are_unchanged`; definition targets and document links →
`a3_navigation_is_unchanged`. The Input Robustness Matrix does not apply: no
file-format or configuration reader changed.

### Design Decisions

- **Decision:** pin responses with JSON goldens over the frozen A3 inputs,
  driven through the public `ProviderRegistry` in-process.
  - **Alternatives considered:** per-field `assert_eq!` on a handful of
    positions (misses unlisted fields and ordering); an LSP-session
    subprocess test (much slower, adds nothing for an in-process ownership
    change); goldens over the live `claudine/schemas/` (would drift with
    unrelated Claudine schema edits).
  - **Why:** the spec demands *exact* hover text, completion order/edits,
    and diagnostic fields; a golden catches any field drift, and the frozen
    bench inputs are the same document whose allocation is being fixed.
  - **Downsides:** an intended response change elsewhere (for example new
    hover prose) requires regenerating the goldens; completion
    documentation prose is deliberately left out to limit that churn.
- **Decision:** keep the Claudine bundle as a module-level `const` of
  `include_str!`s, but put the A3 document and each golden's
  `include_str!` inside its `#[test]`.
  - **Alternatives considered:** repeat the nine-file bundle in every test.
  - **Why:** the bundle is frozen and rarely edited, so the wider CI cell a
    module-level literal implies is cheap; the document and goldens stay
    narrowed to their tests.
  - **Downsides:** an edit to the frozen bundle schedules the whole `dmls::l1`
    binary instead of these twelve tests.

### Notes

- During this phase another process on the host committed this worktree's
  in-progress files (`a7dcd6bed` with the plan, log, and baseline; `6594bcae5`
  with the first version of the test file). This session made no commits.
  The remaining edits (test registration, goldens, the added fault test,
  this log) are uncommitted.
- One command of this session accidentally ran in the main checkout
  (`/Volumes/coding/personal/rusty-biscuit`) and created empty golden
  placeholders plus a `mod` line there; both were reverted immediately and
  that checkout's `git status` is clean again.

### Gates

- Scope check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [dmls, test-toolkit, darkmatter]`, `change_class: package`
  (the same on the final tree), so both gates ran.
- `just lint` (darkmatter/): pass on the final tree; `cargo clippy -p dmls --all-targets -- -D warnings`: pass.
- `just test` (darkmatter/), final tree after the hover split: **12,545
  passed, 10 skipped, 0 failed** in 79 s, with no test at 5 s or longer
  (the earlier run, before the split, was 12,538 passed).
- `just check-tier-coverage darkmatter`: 0 stranded.
- `just cross-check dmls --os windows` (build-win-native), attempt 1:
  1,445 passed, but two contract tests took 7.3 s (over the 5 s budget); the
  hover tests were split (see above). Attempt 2: **1,452 passed, 9 skipped**,
  every `frontmatter_response_contracts` test at 2.2–3.8 s, so the golden
  path/URI normalization holds on native Windows.
- Dependent packages: this phase changed only `dmls` test files and test
  fixtures. No dependent package compiles or reads them, so there were no
  dependent tests to run.
- No skills or docs changed: Phase 1 changes no behavior. The darkmatter
  skill is updated in Phase 5 with the new cache ownership.
- I mistakenly started `just lint` once from the repository root, which lints
  the whole monorepo. It was stopped after it began checking other packages,
  and the darkmatter-area lint was run instead.

## Phase 2

Built on `2fd4d3c25` (Phase 1 is committed in `ccdfc0287`/`6594bcae5`). All
work is uncommitted in this worktree.

### What changed

- **`dmls/src/overlay/analysis.rs` (new): `FrontmatterAnalysis`.** One
  snapshot's shared schema state: the effective root (`OnceLock`, assembled on
  first use), merged views of several admissible union arms, each union's arm
  discriminant JSON, and synthesized array-item definitions. Built values live
  in a small append-only arena (`SharedStore` over `OnceLock` chunks), so a
  borrow of one lasts as long as the borrow of the snapshot, with no `unsafe`
  and no new dependency. Keys are schema-node addresses plus `ArmAdmission`
  (`Any` for transparent crossing, `Items`/`Values` for explicit). A merged
  view's copy of a property that exactly one arm declares records its source's
  identity, so recursion below a merged view converges on the same cache entry
  at every depth. A document with no bundle shares the baseline root, a
  `LazyLock` constant built from compiled-in inputs.
- **`OverlayCache` owns the current snapshot per URI** (`analysis` map) and
  hands the same `Arc` to every `DocumentOverlay` (`overlay.analysis`, a new
  non-optional field). `AnalysisIdentity` holds the revision, the hash of the
  current text, stale-tree state, the hash of the tree's own text, the
  schema-cache key (text + config + registry version + context), and the
  bundle and named-type `Arc`s compared by identity. The cache already returns
  a new bundle `Arc` on any import-file change, so that change is covered.
  When the snapshot is reused, the overlay is handed the snapshot's own tree,
  so entry indices that later phases store address the tree consumers read.
  `forget` removes the snapshot. The standalone and trigger-schema paths get
  an uncached `FrontmatterAnalysis::detached()`.
- **Revision threading:** `OverlayState::for_document_timed` takes
  `revision: Option<i32>`. The router passes `Some(open.version())`;
  `for_document` (tests, benches) passes `None`.
- **Resolver (`providers/frontmatter.rs`):** `ShapeMemo<'c, 'p>` is
  constructed from the context (`ShapeMemo::new(ctx, crossing)`) and takes
  its root, definitions, frontmatter JSON, and snapshot from it, so a foreign
  root can no longer be passed in. It remembers only which `&'c SchemaShape`
  each instance path reached. `descend` borrows a discriminant-selected arm or
  a single admissible arm. It asks the snapshot for a merged view only when
  two or more arms are admissible. `def_at_path_ctx` is replaced by
  `ShapeMemo::def_at`, which returns `&'c PropertyDef` (no `Cow`, no leaf
  clone), with item definitions synthesized once per snapshot. `known_shape`
  returns `&'c SchemaShape`. Consumers migrated:
  `KnownRoots.shape` now borrows it (no owned root copy in DSL site typing),
  and `UnionItem.alternatives` is now `&'c [PropertyAtom]` (no `into_owned`).
  Code actions and the DSL schema hover no longer copy the root.
- **Work counters (`work_counters.rs`):** `WorkCounts` gains
  `root_assemblies`, `merged_shapes`, `item_syntheses`,
  `scalar_classifications`, `typing_walks`, and `stack_walks`, plus a
  `plus` method. Counts are taken at the construction site, never at a
  cache lookup. Typing walks and classifications are counted in
  `expression_values` and in `nav_targets`' file-typing loop, and stack walks
  in `event_views::stack_expressions`. Phase 3 restructures those consumers.
  The test-only `SHAPE_MATERIALIZATIONS` counter (which counted descent
  attempts) is gone. The A3 bench folds with `plus` and prints the new
  counts in its `dmls_work` column, with the protocol unchanged.

### Design Decisions

- **Decision:** identify schema nodes by their address inside an owner the
  snapshot retains, and store built values in a safe append-only `OnceLock`
  arena so walks return `&'c` borrows.
  - **Alternatives considered:** `Arc<SchemaShape>` handles plus selector
    paths re-navigated on each access; serialized/content keys; an arena
    crate (`elsa`, `typed-arena`); `unsafe` lifetime extension.
  - **Why:** every reachable node already lives in the root, the bundle's
    definitions, or a value this snapshot built, none of which moves or is
    mutated while the snapshot lives. Address identity is assigned lazily with
    no indexing pass, and the `OnceLock` arena gives borrowed returns without
    a dependency or `unsafe`, as the plan requires.
  - **Downsides:** identities are meaningless outside their snapshot, and a
    caller must never key the caches with a node from a different owner. The
    memo enforces this by taking its root from the context.
- **Decision:** a merged view's copy of a property declared by exactly one arm
  is cached under its source's identity.
  - **Alternatives considered:** key merged views by the copy's own address;
    key them by the ordered addresses of the admitted arm shapes.
  - **Why:** both alternatives give every recursive depth below a merged view
    a new identity, because each merge copies its children, so
    `then[0].then[0]…` built a new view per depth. The copy is a deep clone
    with identical content, so caching it under the source is exact, and
    recursion converges (`recursive_depths_and_same_arm_items_share_one_view`).
  - **Downsides:** a small source map, locked per descent step.
- **Decision:** a single admissible arm is borrowed rather than copied into a
  one-arm merged view.
  - **Alternatives considered:** keep building and caching the one-arm copy.
  - **Why:** the plan names single-arm shapes as borrowed. The old copy kept
    only `properties`. Every consumer of a memo shape reads only
    `properties` (checked: nothing reads `pattern_keys` or `constraints` on a
    memo shape), so the borrow is observably identical. The Phase 1 goldens
    confirm this.
  - **Downsides:** if a future consumer reads `pattern_keys` from a nested
    memo shape, it will see the arm's real pattern keys, where the old copy
    showed none. That is arguably more correct, but it would be a change.
- **Decision:** also cache each union's arm discriminant JSON per node and
  admission. Selection itself still runs for every instance against that
  instance's typed authored mapping.
  - **Alternatives considered:** rebuild the JSON on every descent.
  - **Why:** lifecycle stack unions have many arms, and rebuilding their JSON
    for every item is per-item allocation proportional to schema size, which
    is what this fix removes. The selector and its inputs are unchanged.
  - **Downsides:** one more store per snapshot.
- **Decision:** the snapshot is keyed by the bundle and named-type `Arc`
  identities, the schema-cache key, the revision, the text and tree-text
  hashes, and the stale flag. The snapshot owns its tree, and the overlay
  serves that tree.
  - **Alternatives considered:** a separate hash of config, registry, and
    context; serving the freshly parsed tree.
  - **Why:** overlay preparation already returns the same bundle `Arc` only
    while text, config, registry, context, and import files are unchanged,
    and it computes that key. Reusing it avoids a second invalidation scheme,
    as the spec asks. Serving the snapshot's tree keeps later stored entry
    indices aligned with the tree every consumer reads.
  - **Downsides:** per-request hashing of the current (and, when stale, the
    last-good) text. The text was already hashed for the schema key.
- **Decision:** `for_document_timed` takes `revision: Option<i32>`. `None`
  (only `for_document`, used by tests, benches, and corpus checks) keys on
  text alone.
  - **Alternatives considered:** make the revision mandatory on
    `for_document` too.
  - **Why:** only the server has client versions. The other callers build a
    fresh state per check, and requiring a version would churn about 15 test
    call sites without adding protection.
  - **Downsides:** a revision-less caller reuses across identical texts. That
    is correct for them, since there is no client version to distinguish.
- **Decision:** the standalone and trigger-schema overlay paths get a
  detached, uncached analysis. A context with no overlay resolves only
  top-level properties through `ShapeMemo` (no merged views or item
  definitions, because there is no snapshot to own them), and its root is the
  baseline constant.
  - **Alternatives considered:** a process-global analysis for the
    no-overlay case; `Option` on the overlay field.
  - **Why:** the spec forbids a process-global shape cache. A context without
    an overlay has no frontmatter, so nothing descends below a top-level key
    there, and every top-level lookup is unchanged. A non-optional field keeps
    consumers simple, and "existing eligibility" is preserved because those
    paths have no tree.
  - **Downsides:** nested lookups on an overlay-less context fail closed
    rather than merging. No current caller makes such a lookup.
- **Decision:** the lazy scalar-inventory slot lands in Phase 3 with its
  contents (classification records), not as an empty placeholder now.
  - **Alternatives considered:** an empty `OnceLock<()>` placeholder in this
    phase.
  - **Why:** Phase 3's "Cache scalar inventory" task defines what the
    inventory stores. A placeholder would be dead code and would have to be
    replaced. The snapshot already provides the ownership, invalidation, and
    sharing the inventory needs.
  - **Downsides:** Phase 3 adds a field to `FrontmatterAnalysis`.
- **Decision:** relax the lower bound in
  `tests/l1/recursive_schema_editor.rs::completion_and_hover_open_only_the_definitions_on_the_cursor_path`.
  The bound required each deeper hover to open 1–2 more definitions than the
  level above; it now requires 0–2 more. The `item`-only check and the
  completion bounds are unchanged.
  - **Alternatives considered:** leave the item-definition synthesis
    uncached so the old count is reproduced.
  - **Why:** the item definition of `children` is the same recursive node at
    every depth. The spec requires it to be synthesized once per snapshot, so
    a deeper hover legitimately reopens nothing. The test still proves that
    only the cursor path's definitions are opened, and that the growth is
    bounded.
  - **Downsides:** the test no longer proves that each level opens at least
    one definition. That property was a cost, not a contract.

### Requirement → test mapping (this phase)

| Requirement | Test |
| --- | --- |
| Same-arm siblings share one shape; different arms isolated; typed literals distinct (`2` vs `'2'`); stable merge order; unselected/missing/invalid discriminants share one merged view | `sequence_tests::sibling_items_share_their_selected_arm_and_nothing_else` |
| Recursive depths reuse one view; reuse across passes on one snapshot | `sequence_tests::recursive_depths_and_same_arm_items_share_one_view` |
| Constructions independent of identical-item count (10 vs 20); classification per entry | `sequence_tests::identical_items_do_not_multiply_shared_constructions` |
| Whole pass builds fewer constructions than distinct ancestor paths; second pass builds nothing; one root assembly | `sequence_tests::expression_pass_builds_each_shared_construction_once_per_snapshot` (replaces the `SHAPE_MATERIALIZATIONS` test) |
| Item definition synthesized once for 50 items, as the item rather than the array | `sequence_tests::item_definitions_are_synthesized_once_for_every_item` |
| One root assembly shared by `known_shape`, memo, and hover | `sequence_tests::the_root_is_assembled_once_and_shared` |
| Both crossing modes | existing `transparent_crossing_serves_key_only_authoring_ancestry`, `union_arms_are_chosen_by_array_crossing` |
| Snapshot shared by fresh overlays; replaced by revision, edit, imported-schema change, config change, context change, malformed buffer (over last-good tree), repair; old handle keeps its own inputs; `forget` drops the cache's owner | `overlay::tests::frontmatter_analysis_is_shared_until_an_input_changes` |
| Arena slots stable across chunks, built once | `overlay::analysis::tests::append_only_slots_keep_their_values_across_chunks` |
| New counters count and sum | `work_counters::tests::each_helper_counts_its_own_work` |
| Responses unchanged | Phase 1's twelve `frontmatter_response_contracts` tests, passing with goldens untouched |

The Input Robustness Matrix does not apply: no file-format or configuration
reader changed. The fidelity is F1/L1 in-process at the crate's provider and
overlay APIs, which is the boundary that owns this behavior.

### Interim A3 measurement (supporting evidence only)

Same command, host, and settings as the Phase 1 baseline; the raw TSV is
`phase2-a3.tsv`. Hover still runs the whole-document typing walk until
Phase 3, and the formal before/after table belongs to Phase 5.

| Case | Expression hover | Verb hover | Completion | Diagnostics |
| ---- | ---------------: | ---------: | ---------: | ----------: |
| a3x1-warm  | 4,461,499 (was 144,793,776) | 12,957,314 (was 154,998,355) | 2,142,462 | 442,592,132 (was 525,453,337) |
| a3x1-cold  | 4,461,499 | 21,298,026 | 2,142,462 | 845,473,927 |
| a3x10-warm | 7,281,965 (was 966,693,644) | 15,777,780 (was 976,898,223) | 2,146,494 | 477,757,311 (was 970,158,217) |
| a3x10-cold | 7,281,965 | 24,118,492 | 2,146,494 | 880,639,106 |

### Notes

- A `sed` aimed at `dmls/src/overlay/mod.rs` ran once in the main checkout
  (`/Volumes/coding/personal/rusty-biscuit`), because the shell's working
  directory had been reset. That one-line change was reverted with
  `git checkout -- darkmatter/dmls/src/overlay/mod.rs` there, and that
  checkout's status is clean again. Every later command used absolute
  worktree paths.

### Gates

- `just lint` (darkmatter/): pass, after removing thirteen `needless_borrow`
  sites left by the `Cow` → `&` change. `cargo clippy -p dmls --all-targets
  -- -D warnings`: pass.
- `just test` (darkmatter/): **12,552 passed, 10 skipped, 0 failed** in
  79.8 s. The slowest test took 4.47 s (an existing
  `claudine_editor_parity` test); none reached 5 s.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependent packages: none (no `Cargo.toml` in the workspace depends on
  `dmls`).
- Cross-OS check: not run. The change adds no path, process, or
  OS-specific code (`Mutex`, `OnceLock`, and pointer identity only). The new
  overlay test builds paths with `canonicalize_simplified` and
  `Url::from_file_path`, like its neighbors. CI's Windows and WSL2 cells cover
  it.
- Scope check, run after every edit in this phase (code, plan, log, skill):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [dmls, test-toolkit, darkmatter]`, `change_class: package`.
  The gates were therefore required, and all of them ran (above). No code
  changed after they ran.

## Phase 3

Built on the uncommitted Phase 2 tree (Phase 2 is not yet committed in this
worktree, so the working tree holds both phases' changes). All work was done
in this session, sequentially, without subagents: the point extractor, the
inventory, and the provider routing share one interface and were small enough
to change together.

### What changed

- **One-scalar extraction (`overlay/expressions.rs`, `overlay/event_views.rs`).**
  `expressions::scalar_interpolations(document, entry, index)` is the
  per-scalar step of `frontmatter_interpolations`, which now loops over it.
  `event_views::scalar_stack_expressions(host, document, ast, index)` yields
  the lifecycle expressions of one scalar: its `{{ … }}` spans, or a `when`
  guard holding no span. `stack_expressions` (still the whole-document walk,
  still counted as one `stack_walks`) now loops over entries, skipping as
  before, and calls the per-scalar function, then sorts by reach as before.
  The guard rules, dialects, projections, and skip semantics are unchanged.
- **Scalar inventory (`overlay/analysis.rs`).** `FrontmatterAnalysis` gains a
  `scalars: SharedStore<usize, ScalarTyping>` keyed by entry index and an
  `inventory: OnceLock<Box<[(index, slot)]>>`. `ScalarTyping` records whether
  the definition has a `file` arm and an `ExpressionTyping` (`No`,
  `Undecodable`, or `Decoded { text, projection }`); an entry no definition
  reaches is stored as `ScalarTyping::UNRESOLVED`. `scalar(index, classify)`
  is the point API and never initializes the inventory; `inventory(classify)`
  walks the tree's scalar entries once per snapshot (one `typing_walks`),
  classifying only entries no point request already stored, and every later
  call iterates the stored slots. `SharedStore` gained `slot_or_insert_with` /
  `value(slot)` so the inventory reads values without a lock per entry.
  `record_scalar_classification` moved to these two build sites and
  `record_typing_walk` to the inventory's initializer, so a lookup never
  counts.
- **Classifier and routing (`providers/frontmatter.rs`).** `classify_scalar`
  is the one classifier (explicit crossing through `ShapeMemo::def_at`,
  decoding with `ScalarProjection::for_value` against the request text, as
  `expression_values` did). `scalar_typing(ctx, index)` is the point lookup,
  `scalar_inventory(ctx)` the document one, and `cursor_scalar(ast, offset)`
  locates the scalar under a cursor.
  - `hover` locates the cursor scalar once and types it once: an
    Expression-typed value goes to `expression_hover` (which now takes the
    value), any other scalar to `stack_expression_hover`, which extracts only
    that scalar's expressions. The O(n²) `typed.contains` scan is gone.
  - `definition` handles named types and `$schema` as before, then types and
    resolves only the scalar containing the cursor (`schema_target`,
    `file_target`, `navigation_path` are the pieces `nav_targets` used
    inline).
  - `expression_values(ctx)` and `nav_targets(ctx)` (document links) read the
    inventory; both lost their `ast` parameter, since the tree must be the
    snapshot's own for stored indices to be valid. `ExpressionValue` now
    borrows its decoded text and projection from the snapshot.
  - `diagnostics/frontmatter.rs` keeps its one `expression_values` per
    publication and its one `stack_expressions` walk; a repeated publication
    on the same snapshot now classifies nothing.
- **Skill:** `.claude/skills/darkmatter/dmls.md` gains a "Point versus
  document typing" bullet with the API split and the rule that a cursor
  request never calls `expression_values`, `nav_targets`, or
  `stack_expressions`.

### Design Decisions

- **Decision:** the classifier lives in `providers/frontmatter.rs`; the
  analysis module stores outcomes and takes the classifier as a closure.
  - **Alternatives considered:** move `ShapeMemo` and the atom predicates
    into `overlay/analysis.rs`; have the analysis call into the provider
    module directly.
  - **Why:** the classifier needs `ShapeMemo` and `DocumentContext`, both
    provider-level. Passing a closure keeps the overlay layer free of a
    provider dependency, and one function still serves both access paths.
  - **Downsides:** a caller could in principle pass a different classifier;
    only `scalar_typing` and `scalar_inventory` call these methods.
- **Decision:** `ScalarTyping` stores the decoded text, projection, and two
  facts (file arm, expression arm), not a handle to the definition.
  - **Alternatives considered:** store a `&PropertyDef` (self-referential in
    the snapshot); store a node address and re-derive the definition.
  - **Why:** the plan forbids references into sibling fields, and no consumer
    of the inventory needs the definition: `expression_diagnostics` already
    re-resolves the path through `ShapeMemo`, whose shapes are shared through
    the same snapshot, so that is a lookup, not a construction.
  - **Downsides:** a future consumer needing the definition resolves it again
    (cheaply, through the shared shapes).
- **Decision:** an unresolved entry is stored as a non-file, non-expression
  record rather than with a separate `resolved` flag.
  - **Alternatives considered:** a `resolved: bool` field or an `Unresolved`
    variant.
  - **Why:** no consumer distinguishes "no definition" from "a definition with
    neither arm"; what the plan requires is that the miss is stored and not
    retried, which the store guarantees for every outcome. A field nothing
    reads failed the dead-code lint.
  - **Downsides:** a later consumer needing the distinction adds it then.
- **Decision:** `expression_values` and `nav_targets` take only the context
  and read the overlay's tree.
  - **Alternatives considered:** keep the `ast` parameter and fall back to an
    uncached walk when it is not the snapshot's tree.
  - **Why:** every caller already passed the overlay's tree; stored entry
    indices are valid only for the snapshot's tree, which is always the
    overlay's (detached overlays have no tree). Removing the parameter makes
    a mismatched tree unrepresentable.
  - **Downsides:** nine test call sites changed.
- **Decision:** `inventory` takes `&mut dyn FnMut` and returns an iterator
  with precise capture (`use<'s>`).
  - **Alternatives considered:** a generic `impl FnMut` (the 2024-edition
    return type then captures the closure, tying the iterator to the request
    context); returning a `Vec`.
  - **Why:** the returned iterator must live as long as the snapshot borrow
    only, and iterating stored slots needs no allocation.
  - **Downsides:** one dynamic call per classification on the first walk.
- **Decision:** document links keep resolving targets per request; only the
  typing is shared.
  - **Alternatives considered:** cache resolved targets in the inventory.
  - **Why:** resolution reads the filesystem, which the snapshot identity
    does not cover, and the plan excludes resolved navigation I/O from the
    cache.
  - **Downsides:** links still stat each file-typed value per request.

### Requirement → test mapping (this phase)

| Requirement | Test |
| --- | --- |
| Point extraction equals the document walk per scalar; typed expression skipped; two spans in one scalar; unwrapped guard; span-holding guard; non-stack value; container; out-of-range index; plain, single-quoted, double-quoted, block, tagged scalars | `overlay::event_views::tests::one_scalar_yields_exactly_its_share_of_the_document_walk` |
| Only the document walk counts as a stack walk | `overlay::event_views::tests::only_the_document_walk_is_counted_as_one` |
| Expression hover and stack (`{{ … }}`) hover type only the cursor scalar, with zero typing and stack walks; unresolved and plain negative outcomes stored once; the inventory classifies only what points did not, once; later document and point requests classify nothing; point and inventory results agree on fresh snapshots | `providers::frontmatter::sequence_tests::point_typing_never_starts_the_inventory_and_the_inventory_reuses_it` |
| Definition types one scalar, no walk; same target as document links; string-typed value not navigable | `providers::frontmatter::sequence_tests::definition_types_only_the_cursor_scalar` |
| A second document pass builds and classifies nothing | `sequence_tests::expression_pass_builds_each_shared_construction_once_per_snapshot` (extended) |
| Responses unchanged (hover text and ranges, completion, diagnostics, navigation) | Phase 1's twelve `frontmatter_response_contracts` goldens, untouched and passing; existing `lifecycle_event_views`, diagnostics, and navigation tests |

The fidelity is F1/L1, in-process at the provider and overlay APIs that own
the behavior. The Input Robustness Matrix does not apply: no file-format or
configuration reader changed.

### Interim A3 measurement (supporting evidence only)

Same command, host, and settings as the baseline; raw TSV `phase3-a3.tsv`.
The benchmark runs diagnostics before both hovers, so hover's zero
classifications below show inventory reuse; the cold-hover proof is the
`point_typing…` test above.

| Case | Expression hover | Verb hover | Diagnostics |
| ---- | ---------------: | ---------: | ----------: |
| a3x1-warm  | 3,790,759 (Phase 2: 4,461,499) | 9,509,494 (Phase 2: 12,957,314) | 442,597,222 |
| a3x1-cold  | 3,790,759 | 17,850,206 | 845,479,017 |
| a3x10-warm | 3,790,711 (Phase 2: 7,281,965) | 9,509,446 (Phase 2: 15,777,780) | 477,867,069 |
| a3x10-cold | 3,790,711 | 17,850,158 | 880,748,864 |

Both hovers are now independent of document size (a3x1 and a3x10 agree to
within 48 bytes) and do no typing or stack walk, but both remain above the
1,000,000-byte target. The remaining cost is outside frontmatter typing, in
the work every hover does regardless of size (`LifecycleHost::for_document`,
`dsl::with_site_scope`, `dsl::host_catalogs`, the expression hover markdown),
and the cold verb hover adds about 8.3 MB over warm. Phase 5 owns the target.

### Gates

- Scope check, run from the repository root after every edit (code, plan,
  log, skill, frontmatter):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [dmls, test-toolkit, darkmatter]`, `change_class: package`,
  so both gates were required, and both ran (below).
- `just lint` (darkmatter/): pass. `cargo clippy -p dmls --all-targets -- -D
  warnings`: pass.
- `just test` (darkmatter/): **12,556 passed, 10 skipped, 0 failed** in
  80.6 s. Slowest test 3.9 s (`claudine_editor_parity`, existing); none at
  5 s.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependent packages: none (no workspace package depends on `dmls`).
- Cross-OS check: not run. The change adds no path, process, or OS-specific
  code (index-keyed stores and in-memory typing only); CI's Windows and WSL2
  cells cover it.
