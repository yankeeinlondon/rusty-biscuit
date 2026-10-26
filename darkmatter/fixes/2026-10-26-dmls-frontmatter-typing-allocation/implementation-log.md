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
    - darkmatter
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
source_files_during_phase_4:
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/frontmatter/snapshot_tests.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/tests/l1/recursive_schema_completion.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/src/providers/frontmatter/snapshot_tests.rs
docs_updated_during_phase_5:
    - darkmatter/dmls/docs/hover.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/lifecycle/host-artifact.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
source_code:
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
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/providers/frontmatter/snapshot_tests.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/tests/l1/recursive_schema_completion.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
documentation:
    - darkmatter/dmls/docs/hover.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/lifecycle/host-artifact.md
    - .claude/skills/darkmatter/dmls.md
completed_phase: 5
implemented: true
decisions_for_author: []
pipeline_refreshed:
    - 2026-10-25-fix-now
    - 2026-10-25-shared-acyclic-definitions
    - 2026-10-25-pipeline-work-reduction
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

## Phase 4

Built on `6a23ecbe7` (Phases 1–3 committed). Done in this session,
sequentially, without subagents: both Wave 5 tasks were test-only and the
existing helpers lived in the files being extended. One production defect
the new invalidation test exposed was fixed (below).

### What changed

- **`providers/frontmatter/sequence_tests.rs` (extended).**
  - `lifecycle_document(n)`: `n` failure handlers (a guard, a `{{ … }}`
    span, `ensure_file`, `warn`) beside `n` `file[]` references and one
    undeclared key, using Claudine's shipped lifecycle schema.
  - Five cold point-request tests, one per request through
    `ProviderRegistry::with_substrate()` (the bench's path), generated by
    `cold_point_request_tests!` so nextest spreads them: expression hover,
    verb hover, key hover (a cursor outside every typed scalar value),
    completion, definition. Each runs on a fresh snapshot of a 200-item
    document and asserts no typing or stack walk, at most one
    classification, at most one root assembly, and constructions bounded
    by the five schema positions on the deepest cursor path. It also
    asserts the same counts on a 20-item document. Observed: at most
    1 merged shape plus 1 synthesis (verb hover: 2 merged + 1), and
    0–1 classifications.
  - `diagnostics_constructions_do_not_scale_with_identical_items`: full
    registry diagnostics at 100 and 200 items. Same diagnostics, one typing
    walk and one stack walk each, identical root, merged, and synthesis
    counts, and classifications differing by exactly 100 × 5 scalars.
  - `diagnostics_then_point_requests_reuse_one_snapshot` (200 items):
    point-types a guard, a plain verb value (non-expression), and an
    undeclared key (unresolved) before diagnostics. Then diagnostics (one
    walk); repeated diagnostics (identical result, zero classifications,
    walks, or constructions); and hover, verb hover, key hover, completion,
    and definition after diagnostics, with zero walks or classifications and
    answers identical to the same requests on a cold snapshot. The point
    typings taken before the inventory existed equal what the inventory
    holds.
  - `every_discriminant_shape_selects_per_item_within_one_snapshot`: four
    arms with two shared discriminants (`kind`, `mode`) and twelve sibling
    items on one snapshot. Selected: string, number, agreeing pair, boolean
    `true`, and a key that resolves past a duplicate literal. Unselected:
    string `'true'`, string `'2'`, `null`, absent, unknown, duplicate
    literal, and conflicting keys. Each selected item shares its arm's
    shape; every unselected item shares one merged view (built once) in
    arm-then-property order; completion on the same snapshot offers a
    boolean-selected item only its arm's keys and an unselected item every
    arm's keys.
  - `every_scalar_representation_types_alike_through_point_and_inventory`:
    plain, single-quoted, double-quoted, block, and tagged expression values
    plus a mixed `[number, expression]` union. Point typing (before any
    inventory) and the inventory decode the same text. The inventory
    classifies only the scalars no point request did. Exact values project
    back to their own authored bytes, and block or tagged values stay
    inside their value span.
- **`providers/frontmatter/snapshot_tests.rs` (new; declared in
  `providers/frontmatter.rs`).**
  `every_input_change_starts_a_snapshot_that_answers_from_its_own_inputs`
  drives one long-lived `OverlayState` the way the router does
  (`prepare_for_document`, then a fresh `DocumentOverlay` from
  `for_document_timed` with a revision per request). Each request publishes
  diagnostics, then hovers on `guard`, then completes a key. Its answers are
  compared with an oracle: a fresh `OverlayState` that saw only the inputs
  that step needs (the last-good text plus the malformed one, for malformed
  steps). The cases are:
  - reuse by a fresh overlay wrapper (zero work)
  - same text at a new revision
  - an edit
  - an imported-schema change, then restoring it (a new snapshot, never the
    old one)
  - a configuration change
  - a trigger-registry change (writing a trigger plus
    `invalidate_trigger_inputs`, which brings in the trigger's required
    `model` in diagnostics and completion)
  - a context change
  - schema-assembly failure, then repair
  - malformed YAML over the last-good tree, then a second malformed buffer
    with shifted offsets, then the repaired buffer
  - a request finishing on a retained replaced overlay (it answers for its
    own buffer, and the current snapshot is reused unchanged afterward)
  - no replaced snapshot kept by the cache (`Arc::strong_count == 1` for
    each)
  - close dropping the cache's owner, and reopening starting over

  Passivity: around every request it asserts zero DMLS reads, hashes,
  parses, enumerations, or registry scans, no context build, no Darkmatter
  `TriggerDiscovery` or `EffectivePreparation`, and (under
  `effects-instrumentation`, which `just test` enables) no effect engine
  and no network attempt. The document on disk is unchanged at the end.
  Preparation (`prepare_for_document` / `for_document_timed`) is measured
  outside the requests, since it is the existing dependency-check stage.
- **Fix: `overlay/mod.rs::error_dependencies`** now records the file of
  `SchemaError::SchemaDocument` (a malformed imported schema file) and
  `SchemaError::Io`. Before, a document whose `$schema` file failed to parse
  cached the failure with no dependency, so repairing that file without
  editing the buffer left the failure (and its snapshot) cached until the
  text, configuration, registry, or context changed. Regression test
  `overlay::tests::repairing_a_malformed_imported_schema_reassembles_an_unchanged_buffer`
  (and the snapshot test's failure-then-repair step) fails without the
  fix, which was checked by reverting it, and passes with it. The function's
  doc comment now lists the new cases.
- **Skill:** `.claude/skills/darkmatter/dmls.md` names where reuse and
  invalidation are proven and the rule that a path-bearing `SchemaError`
  variant must be listed in `error_dependencies`.

### Design Decisions

- **Decision:** prove invalidation with one end-to-end sequence over a
  long-lived `OverlayState`, comparing each step's public answers with a
  fresh state's answers, instead of one test per input.
  - **Alternatives considered:** one test per input change; extending the
    overlay-level `frontmatter_analysis_is_shared_until_an_input_changes`,
    which checks only `Arc` identity.
  - **Why:** the contamination risk is history, meaning a snapshot built
    from earlier inputs answering for later ones, which only a sequence
    exposes. Comparing with a fresh state is an exact oracle for every
    answer (ranges, codes, messages, hover text, completion order) without
    hand-written expectations. The whole sequence runs in 0.8 s.
  - **Downsides:** a failure names a step, not a test. Each step's label is
    in its assertion message.
- **Decision:** cold point-request tests go through
  `ProviderRegistry::with_substrate()`, one generated test per request.
  - **Alternatives considered:** the frontmatter provider alone; one test
    looping over all five requests (2.7 s).
  - **Why:** the A3 bench and the editor go through the registry, so a
    second provider walking the document would be caught. Splitting keeps
    each test under 1 s and lets nextest spread them across cores.
  - **Downsides:** five tests share one helper. The macro list is the only
    list of requests, so no guard test is needed.
- **Decision:** the "conflicting" and "duplicate" discriminant cases use
  the shared selector's own definitions: two discriminant keys selecting
  different arms, and one literal declared by two arms.
  - **Alternatives considered:** a YAML duplicate key in one item mapping.
  - **Why:** a duplicate YAML key makes the frontmatter unparseable (the
    malformed-buffer path, covered by the snapshot test), so it never reaches
    selection. `discriminant.rs` names these exact cases.
  - **Downsides:** none.
- **Decision:** fix the failed-assembly dependency gap in this phase rather
  than weakening the failure-then-repair case.
  - **Alternatives considered:** test only failures that wrap
    `ReferencedSchema` (already tracked); file a follow-up.
  - **Why:** the plan requires a failure followed by repair, and says to
    fix failures in the implementation before proceeding. A malformed import
    is the common mid-edit failure. The fix is one match arm on the existing
    dependency mechanism.
  - **Downsides:** a missing import still fails as `NoMatch`, which names
    no path, so creating the missing file does not refresh an unchanged
    buffer until a watched or rescanned change. That is the existing
    behavior and is out of this phase's scope.

### Requirement → test mapping (this phase)

| Acceptance item | Test |
| --- | --- |
| 1a: cold hover on one entry of a 200-item lifecycle document (expression, verb, outside typed values) builds at most the path's positions plus one root, independent of item count, with no walk; completion and definition likewise | `sequence_tests::cold_expression_hover_types_only_the_cursor`, `cold_verb_hover_types_only_the_cursor`, `cold_key_hover_types_no_scalar_walk`, `cold_completion_types_only_the_cursor`, `cold_definition_types_only_the_cursor` |
| 1b: diagnostics builds each position or merged view once; doubling identical items doubles no construction; classification per scalar | `sequence_tests::diagnostics_constructions_do_not_scale_with_identical_items` (plus the existing `identical_items_do_not_multiply_shared_constructions` and `recursive_depths_and_same_arm_items_share_one_view`) |
| 1c: hover after diagnostics walks nothing; repeated diagnostics reuses the inventory; point results before diagnostics equal the inventory's (expression, non-expression, unresolved) | `sequence_tests::diagnostics_then_point_requests_reuse_one_snapshot`, `every_scalar_representation_types_alike_through_point_and_inventory` |
| 1d: edit, identical-text revision, imported-schema change, configuration, trigger-registry, and context changes, close/reopen, schema failure → repair, malformed YAML → repair; old handle usable, never current again; no history | `snapshot_tests::every_input_change_starts_a_snapshot_that_answers_from_its_own_inputs`, `overlay::tests::repairing_a_malformed_imported_schema_reassembles_an_unchanged_buffer`, existing `overlay::tests::frontmatter_analysis_is_shared_until_an_input_changes` |
| Passivity (no reads, network or shell effects, mutation, or context discovery from analysis) | `snapshot_tests::…` (asserted around every request) |
| 3: sibling arms; absent, unknown, duplicate, conflicting discriminants; strings, numbers, booleans, null; merged-arm completion | `sequence_tests::every_discriminant_shape_selects_per_item_within_one_snapshot`, existing `sibling_items_share_their_selected_arm_and_nothing_else`, and the `sibling_completion_*` unit tests |
| 3: recursive references; both crossing modes | existing `recursive_depths_and_same_arm_items_share_one_view`, `transparent_crossing_serves_key_only_authoring_ancestry`, `union_arms_are_chosen_by_array_crossing` |
| 3: mixed expression unions; plain, quoted, block, and tagged values | `sequence_tests::every_scalar_representation_types_alike_through_point_and_inventory`, existing `diagnostics::frontmatter` mixed-union tests and `event_views::one_scalar_yields_exactly_its_share_of_the_document_walk` |
| 3: responses unchanged | Phase 1's `frontmatter_response_contracts` goldens, untouched and passing |

Fidelity F1, L1, in-process at the provider-registry and overlay APIs that
own the behavior (the boundary the server calls). The Input Robustness Matrix
does not apply: no file-format or configuration reader changed (the fix
widens which existing error paths are recorded as dependencies).

### Notes

- The shared discriminant selector treats `kind: 2.0` as not matching
  `literal(2)` (`serde_json` number identity). That is existing library
  behavior outside this fix, so the case was left out of the new test
  rather than pinned.
- An inline `$schema` union written as the quoted flow string
  `"[number, expression]"` does not assemble; the YAML-list form does. The
  test uses the list form.
- Slowest new tests: `diagnostics_then_point_requests_reuse_one_snapshot`
  about 2.0 s, `diagnostics_constructions_do_not_scale_with_identical_items`
  about 1.5 s, each cold point test 0.7–0.9 s, the snapshot sequence 0.8 s.
  All are well under 5 s.

### Gates

- `just lint` (darkmatter/): pass. `cargo clippy -p dmls --all-targets
  --features effects-instrumentation -- -D warnings`: pass.
- `just test` (darkmatter/): **12,567 passed, 10 skipped, 0 failed** in
  79.1 s.
- `just check-tier-coverage darkmatter`: 0 stranded. No new test name
  carries a tier marker.
- Dependent packages: none (no workspace package depends on `dmls`).
- Cross-OS check: not run. The new tests build paths with
  `canonicalize_simplified` and `Url::from_file_path` under temp dirs, like
  their neighbors. The fix is one match arm, with no path comparison or OS
  code. CI's Windows and WSL2 cells cover it.
- Scope check, run from the repository root after all edits:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [dmls, test-toolkit, darkmatter]`, `change_class: package`,
  so both gates were required and both ran.

## Phase 5

Built on the Phase 4 working tree (uncommitted Phase 4 changes present).
Done in this session without subagents: the allocation work was a chain of
measure → fix → re-measure on the same files, and the documentation depends
on the final design.

### Where the hover bytes went

Phase 4 left expression hover at 3.79 MB and verb hover at 9.5 MB warm /
17.9 MB cold. A temporary allocation probe (a `bytes_allocated` reading at
each stage boundary of `FrontmatterProvider::hover`, removed before the final
run) attributed them:

| Stage | Hover | Bytes per hover |
| ----- | ----- | --------------: |
| `named_references::hover`: parses the `$schema` declaration twice (`unusable_target_hover`, `reference_definition`) | both | 671,029 |
| `meta_schema_hover`: parses it a third time | both | 335,515 |
| `LifecycleHost::for_document` → `host_lifecycle_profiles`: reads and **YAML-parses every schema-root file** (`is_global_catalog`, `declares_fm`) before its cache lookup | expression | 2,777,017 |
| `union_item` → `alternative_accepts_in` × 6: each alternative's reachable definitions cloned, lowered to JSON Schema, serialized to hash for the validator cache (cold: also compiled) | verb | ≈ 6.8 MB warm / 15.1 MB cold |
| `union_item` → `ShapeMemo::def_at(item)`: the array-item definition synthesized for the new snapshot (every edit makes one) | verb | 1,671,753 |

None of it was frontmatter typing: after Phase 3 both hovers did zero
classifications and walks. The bench's "expression hover" lands on a
top-level `{{ … }}` scalar that is not in a lifecycle stack, so after the
lifecycle-host lookup it falls through to the schema hover of `phase`.

### What changed

- **`AuthoredDeclaration::frontmatter_at`** (`overlay/named_types.rs`): a
  request parses the frontmatter `$schema` declaration only when its offset
  lies inside the value (end inclusive). `named_references::{hover,
  definition}` and `meta_schema_hover` use it. No declaration span lies
  outside the value, so no answer changes.
- **`host_lifecycle_profiles`** (`darkmatter/lib/.../catalog.rs`): the cache
  key (path, content hash, artifact) is computed first, and a file that is
  not a host catalog is cached as `None`, so an unchanged schema root is read
  and hashed but never YAML-parsed again. A file is still re-classified when
  its bytes change.
- **`AlternativeMatcher` and `alternative_fingerprint`**
  (`darkmatter/lib/.../completion.rs`, `simplified/query.rs`): the lowering
  and validator lookup of `alternative_accepts_in` is a preparable value
  (`alternative_accepts_in` is now `prepare(..).accepts(..)`), and an
  alternative has a content fingerprint: its derived `Debug` text and that of
  every definition it reaches, in the order the lowering allocates `$defs`
  keys, streamed into a hasher without building the text.
- **`CrossSnapshotSchema`** (`overlay/analysis.rs`): one per `OverlayState`,
  shared by every snapshot through `FrontmatterAnalysis::shared()`. It holds
  prepared alternative matchers keyed by `alternative_fingerprint`, and
  synthesized array-item definitions keyed by a fingerprint of the item atoms
  they are copied from. Each kind is bounded at 256 entries (cleared when
  full). A snapshot keeps an `Arc` to each item definition it took, so the
  node-address identities of Phase 2 still hold for its life.
  `union_item` judges through it; `ShapeMemo::def_at` takes item definitions
  through it (`array_item_def` split into `array_item_atoms` + synthesis).

### Allocation result

Same command, host, build settings, frozen inputs, sizes, cold/warm cases,
and repetitions as the Phase 1 baseline (see Phase 1). Raw TSV:
`phase5-a3.tsv` beside this log. The benchmark's order is unchanged:
**overlay → diagnostics → completion → verb hover → expression hover**, so
diagnostics runs before both hovers.

| Case | Expression hover before → after | Verb hover before → after | Diagnostics before → after |
| ---- | ------------------------------: | ------------------------: | -------------------------: |
| a3x1-warm  | 144,793,776 → **105,025** | 154,998,355 → **58,763** |   525,453,337 → 437,251,200 |
| a3x1-cold  | 144,793,776 → **105,025** | 163,339,067 → **58,763** |   928,335,132 → 840,132,995 |
| a3x10-warm | 966,693,644 → **105,025** | 976,898,223 → **58,763** |   970,158,217 → 472,521,047 |
| a3x10-cold | 966,693,644 → **105,025** | 985,238,935 → **58,763** | 1,373,040,012 → 875,402,842 |

Every measured hover allocates under 1,000,000 bytes at both sizes, warm
and cold, and both are independent of document size. Supporting values:
verb hover 946 allocations, 14,016 peak live bytes, median 1.6 ms; expression
hover 184 allocations, 44,518 peak live bytes, median 0.25–0.35 ms (a3x10
hover peak live was 417,936,523 bytes at baseline).

**What the byte target does and does not depend on.**

- It does not depend on diagnostics having typed the document: neither hover
  classifies or walks anything (`dmls_work` all zero in the rows), and the
  cold point-request tests (`sequence_tests::cold_*`, unchanged and passing)
  prove a hover before any diagnostics types only its own scalar with no
  walk.
- Cold equals warm for the hovers because the matchers and item definitions
  are server state (`OverlayState`), which the bench keeps across loops as a
  running server does; its cold mode empties the library's process caches
  only. The first hover a server ever serves over alternatives it has not
  seen still prepares them: the probe measured that at about 16.8 MB for the
  A3 verb hover (6 matcher lowerings and compilations plus one item
  synthesis), paid once per distinct alternative content, not per edit.
- The expression hover's remaining 105 KB is mostly reading and hashing the
  schema-root files for the lifecycle host (≈104 KB), which is existing
  per-request file reading, not new work.

**Diagnostics growth.** For the fixed schema, typing work in diagnostics is
linear in entries and constant in schema constructions: scalar
classifications 66 → 453, one typing walk, one stack walk, one root
assembly, 8 merged shapes and 0 item syntheses at both sizes. Bytes grow
437.3 MB → 472.5 MB (warm) for ten times the lifecycle stacks: ≈ 35 MB for
387 more classified scalars, ≈ 91 KB each. That residual is required
per-entry output outside frontmatter typing: expression parses (587 →
4,412) and the lifecycle call checks, whose operand-site comparisons grow
faster than linearly (1,107 → 97,506) and are the subject of
`2026-10-26-sequence-quadratic-frontmatter`. Two sizes of one schema do not
establish scaling for arbitrary schemas; the work-count tests are the
evidence for the typing mechanism.

For `2026-10-25-shared-acyclic-definitions`: compare its re-run of this
bench against the "after" columns above.

### Design Decisions

1. **Close the hover target outside frontmatter typing.**
   - **Decision:** The remaining hover bytes were in the `$schema`
     declaration parse, lifecycle-host discovery, union-item matching, and
     item synthesis, so this phase fixed those paths, including two library
     functions in `darkmatter` (`host_lifecycle_profiles`,
     `alternative_accepts_in`).
   - **Alternatives considered:** Report the target unmet as outside this
     fix's typing scope; reorder the bench so diagnostics warms the hover.
   - **Why:** The plan requires correcting remaining allocations in the
     affected paths and forbids weakening the threshold or reordering the
     benchmark. Each fix is behavior-preserving and local.
   - **Downsides:** The change now touches the `darkmatter` library as well
     as `dmls`.

2. **Keep union-item matchers and item definitions across snapshots, keyed
   by content.**
   - **Decision:** `CrossSnapshotSchema`, one per `OverlayState`, holds
     prepared `AlternativeMatcher`s keyed by `alternative_fingerprint` and
     synthesized item definitions keyed by a fingerprint of their atoms, each
     bounded at 256 entries (cleared when full).
   - **Alternatives considered:** Per-snapshot caches only (every edit is a
     new snapshot, so each post-edit hover still paid ≈ 8.4 MB, and cold
     ≈ 16.8 MB); a process-global cache in the library (the spec rules out a
     process-global schema-shape cache, and the bench's cold mode would
     rightly reset it, leaving cold hover at ≈ 8 MB of validator compilation);
     an exact-match prefilter by JSON kind and required keys (duplicates
     validator semantics, still leaves several alternatives to compile).
   - **Why:** An edit rarely changes the schema, so content is the right
     key; the overlay state is where DMLS already keeps cross-request schema
     caches, and it is bounded and owned, not process-global.
   - **Downsides:** The first hover a server serves over unseen alternatives
     still pays their preparation (≈ 16.8 MB on A3), once per distinct
     content. The bench's cold mode no longer shows that cost on hover,
     because the cache is server state; this log reports it instead. Up to
     256 compiled validators and item definitions stay resident per server.

3. **Fingerprint by streaming derived `Debug` text into `DefaultHasher`.**
   - **Decision:** Fingerprints hash the derived `Debug` output of the schema
     values without building it, using `std`'s `DefaultHasher`.
   - **Alternatives considered:** Hash the lowered JSON (that is the
     allocation being removed); derive `Hash` across the schema types (a wide
     API change to a shared library type family, including `f64`-bearing
     values); `PartialEq`-verified keys (would retain a clone of every
     alternative).
   - **Why:** The types already derive `Debug` deterministically (`IndexMap`
     properties, no hash-ordered fields), the hash is only an in-process cache
     key (like the existing 64-bit validator-cache key), and `biscuit-hash`
     has no streaming hasher; `DefaultHasher` is already used for in-process
     keys in the library (`body_origin.rs`, `render/link.rs`).
   - **Downsides:** A future non-deterministic `Debug` field would turn
     reuse into silent misses (answers stay correct; bytes rise). Recorded
     for `2026-10-25-shared-acyclic-definitions` in its plan.

4. **An item definition's key covers only the atoms it is copied from.**
   - **Decision:** The item-definition fingerprint hashes the array arms'
     item atoms (from the one `resolved_alternatives` call synthesis already
     makes), not every definition they reach.
   - **Alternatives considered:** Reuse `alternative_fingerprint` per atom
     (first attempt); it walks every reachable definition and broke
     `recursive_schema_editor::completion_and_hover_open_only_the_definitions_on_the_cursor_path`
     by opening `meta` and `tag`.
   - **Why:** Synthesis copies only those atoms; references inside them are
     resolved later against the requesting snapshot's own definitions, so
     equal atoms give an equal definition regardless of what they reach.
   - **Downsides:** None found.

5. **Cache non-catalog classification in `host_lifecycle_profiles`, latest
   revision per path.**
   - **Decision:** Compute the existing cache key before classifying, cache a
     non-catalog file as `None`, and drop older entries for that path when
     one is inserted.
   - **Alternatives considered:** Cache the `LifecycleHost` per snapshot
     (a regenerated `.lifecycle.json` would be missed until the next edit,
     a behavior change); key by modification time (rejected by
     `2026-10-25-fix-now` for content identity).
   - **Why:** Classification depends only on the bytes the key already
     hashes, so freshness is unchanged: every request still reads the roots.
   - **Downsides:** Requests still read and hash each root file (≈ 104 KB on
     the A3 expression hover). `2026-10-25-fix-now` rewrites this cache as
     latest-per-path for catalogs too; its spec and plan now keep this.

6. **Gate the `$schema` declaration parse by offset.**
   - **Decision:** `AuthoredDeclaration::frontmatter_at` parses only when the
     offset lies in the `$schema` value, end inclusive.
   - **Alternatives considered:** Parse once per request and share between
     the three callers (still 335 KB per hover anywhere in the frontmatter).
   - **Why:** Every span those callers match lies inside the value.
   - **Downsides:** None; the inclusive end admits one offset at the next
     line's start, where the parse finds nothing.

### Requirement → test mapping (this phase)

| Requirement | Evidence |
| ----------- | -------- |
| Each measured hover < 1,000,000 bytes at a3x1/a3x10, warm and cold | A3 bench, table above; raw `phase5-a3.tsv` |
| Hover bytes do not depend on prior diagnostics typing | `sequence_tests::cold_*` (unchanged, passing: zero walks, ≤ 1 classification); hover rows show zero `dmls_work` |
| Diagnostics linear for a fixed schema | A3 rows: classifications 66 → 453, constructions constant; explanation above |
| Prepared matcher answers exactly as `alternative_accepts_in`; fingerprint is content identity (equal across assemblies, differs for another arm and for a change reached only by recursion; `None` when a definition is missing) | `darkmatter` `recursive_schema_completion::a_prepared_matcher_and_its_fingerprint_follow_the_alternative_s_content` |
| Cross-snapshot reuse: a post-edit hover synthesizes and prepares nothing yet equals a fresh state's; a changed alternative is rebuilt (and only it), and the hover follows the new schema | `dmls` `snapshot_tests::item_definitions_and_matchers_are_shared_only_between_equal_schemas` |
| Item-definition key opens no definition off the cursor path | `recursive_schema_editor::completion_and_hover_open_only_the_definitions_on_the_cursor_path` (existing; failed on the first attempt, passes now) |
| Lifecycle discovery follows each root file's bytes (non-catalog → catalog → non-catalog), and an unchanged catalog is a cache hit | `darkmatter` `catalog::tests::lifecycle_profiles_follow_each_root_file_s_current_content` |
| `$schema` declaration parsed only inside its value; ends inclusive; flow value through its bracket | `dmls` `named_types::tests::a_request_parses_the_declaration_only_inside_the_schema_value` |
| Hover and every response unchanged | Phase 1 `frontmatter_response_contracts` goldens (12, untouched), `union_item_editor`, `named_type_references`, `lifecycle_host_profile`, and the full suites |

Fidelity F1/L1, in-process at the public library API and the provider and
overlay APIs DMLS's server calls. The Input Robustness Matrix does not apply:
no file-format or configuration reader's accepted shapes changed (the
catalog change reorders a cache lookup ahead of classification; the
classifier is unchanged).

### Notes

- The bench's "expression hover" offset (`file_exists(` + 2) is in the
  top-level `phase` value, not an Expression-typed value or a lifecycle
  stack; it answers with the schema hover. The protocol is unchanged, so the
  numbers are reported as measured. A hover on a real Expression-typed value
  also calls `dsl::host_catalogs`, which loads every host catalog uncached on
  each call; `2026-10-25-fix-now` owns that site.
- `darkmatter-cli` `compose_shell_template::every_suggested_rule_approves_the_command`
  took 4.9 s inside the full suite (1123% CPU) and 3.0 s alone; it spawns
  `md` processes and is load-sensitive. No L1 test reached 5 s.

### Completion audit

- Point paths: `expression_values`, `stack_expressions`, and `nav_targets`
  are called only from diagnostics (`diagnostics/frontmatter.rs`) and
  document links; hover and definition use `scalar_typing`,
  `cursor_scalar`, and `scalar_stack_expressions` only.
- No nested-shape clone remains on the walk (`selected.clone()` and
  leaf-`PropertyDef` clones are gone; the remaining `into_owned` calls are
  link-path strings). Item definitions are shared `Arc`s.
- No path-index-keyed shared view: `FrontmatterAnalysis` keys by node
  address and admission; `CrossSnapshotSchema` by content fingerprint.
- No request-local cache posing as reuse: the analysis lives in
  `OverlayCache` per URI, and `CrossSnapshotSchema` in `OverlayCache`
  itself, both reached from every fresh `DocumentOverlay`.
- Departures from the spec, recorded above as decisions: library changes in
  `darkmatter` (Decision 1), and content-keyed reuse across snapshots for
  matchers and item definitions (Decision 2), which goes beyond the spec's
  per-snapshot item cache; the spec is left as decided.

### Gates

Run once after all edits, with no subagents running.

- Scope check from the repository root after every edit (code, docs, plan,
  log, spec, frontmatter):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [darkmatter, dmls, test-toolkit, darkmatter-cli]`,
  `change_class: package`, so every gate ran.
- `just lint` (darkmatter/): pass. `cargo clippy -p dmls -p darkmatter
  --all-targets -- -D warnings`: pass.
- `just test` (darkmatter/): **12,571 passed, 10 skipped, 0 failed** in
  80.8 s (final run, after the last code change; an earlier full run before
  the catalog's latest-revision pruning also passed). Slowest 3.9 s
  (`darkmatter-cli` `compose_shell_template::every_suggested_rule_approves_the_command`,
  existing, process-spawning); none at 5 s.
- `just test-l2` (darkmatter/): passed for `darkmatter`, `darkmatter-cli`,
  `dmls`; no window activation (existing harnesses unchanged).
- `just check-tier-coverage darkmatter`: 0 stranded. No new test name
  carries a tier marker; the new tests sit in modules and binaries the L1
  recipes already compile (`catalog::tests`, `tests/l1/recursive_schema_completion.rs`,
  `overlay::named_types::tests`, `providers::frontmatter::snapshot_tests`).
- Dependent packages: no workspace package depends on `dmls`. For the
  `darkmatter` library changes, the only consumer outside the area that
  reaches them is Claudine (shell-template typing through
  `alternative_accepts_in`; host catalogs and lifecycle artifacts):
  `cargo nextest run -p claudine -E 'test(host_catalog) | test(shell_template) | test(lifecycle_artifact) | test(template)'`
  → 302 passed.
- Cross-OS check: not run. The changes add no path comparison, process, or
  OS-specific code: in-memory caches, a fingerprint hasher, and a test that
  builds its root with `canonicalize_simplified` under a temp dir. CI's
  Windows and WSL2 cells cover it.

## Pipeline Refresh

- `2026-10-25-fix-now` (depends on this fix): **changed.** Spec section 1
  and a new design decision keep the non-catalog classification cache under
  its latest-per-path rewrite of `host_lifecycle_profiles`; plan gains
  Decision 12 (predecessor delivered, interfaces and bench figures to
  preserve) and P2.1 names the existing entries. Plan not started.
- `2026-10-25-shared-acyclic-definitions` (re-runs this fix's bench):
  **changed.** Spec decision and plan Decision 10 record the after-numbers to
  compare against and require deterministic `Debug` text on the definition
  types the content fingerprints read. Plan not started.
- `2026-10-25-pipeline-work-reduction` (starts from this fix): **changed.**
  X1, cached alternative matchers for editor hover, moved from "still
  deferred" to delivered by this fix, with a design decision. No plan yet.
- `2026-10-26-sequence-quadratic-frontmatter` (starts from this fix):
  checked and unchanged; it concerns Claudine's sequence runtime and
  preflight, which this fix does not touch.

## Review 1

### Design Decisions

- **Decision:** Apply the allocation target to the existing benchmark's measured
  edit loops, preserving its warm-up, server lifetime, phase order, cold-cache
  reset, and repetitions. Report first-use preparation separately.
  - **Alternatives considered:** Require every first-ever hover to allocate
    under 1 MB; reset the new server-owned store between benchmark loops.
  - **Why:** Acceptance item 2 says "each measured hover" and explicitly
    requires the existing benchmark cases. Its original warm-up and persistent
    server state are unchanged. The cold point-request tests independently
    prove that cursor typing does not require prior diagnostics.
  - **Downsides:** Passing this target does not establish a sub-1-MB first
    hover over previously unseen schema alternatives; the measured preparation
    cost remains approximately 16.8 MB.
- **Decision:** Continue the full review without an author decision gate.
  - **Alternatives considered:** Request confirmation of the cross-snapshot
    cache or the additional library changes.
  - **Why:** Two independent decision audits found no deferred requested scope
    or decision meeting the human-review bar. The implementation records its
    alternatives, reasons, and costs; these internal choices are settled.
    The missing-import limitation is explicitly preexisting, while the spec
    requires following existing dependency invalidation and preserving behavior.
  - **Downsides:** The review establishes the specified behavior and measured
    allocation result, rather than extending this fix into missing-file
    discovery or a stronger universal first-request allocation guarantee.
