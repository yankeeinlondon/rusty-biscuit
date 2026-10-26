---
spec: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/fixes/2026-10-26-sequence-quadratic-frontmatter/spec.md
plan: claudine/fixes/2026-10-26-sequence-quadratic-frontmatter/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - claudine/lib/src/work_counters.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/benches/pipeline_work_alloc.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine
    - darkmatter
    - dmls
human_review: false
message_to_agent: |-
    Phase 1 (attribution) is done; read the implementation log's Phase 1 for the site/counter table, the source-to-repair map and Design Decisions 2-4, which fix the repair interfaces: a Darkmatter EffectiveState with an Arc-shared base and an owned top-level overlay, a per-preparation seed (decoded frontmatter + caller setters + its derived_deferred_keys + base state) in the preflight Loader with only the `seq` overlay per step, and bucketed prompt-reference dedupe. New work-counters kinds (claudine: PreflightStepFrontmatterNodes, PreflightDeferredScanNodes, EarlyBindingStateNodes, PredictionWithStateNodes, ShellValueStateNodes, PromptReferenceCompare; darkmatter: DeferredRootScanParse) count JSON nodes copied/scanned, so a flat per-step value at 10 and 100 steps is the regression criterion. The bench gained `SQ` rows (`-- SQ`) for the with-hint and set-value paths. Note that `just test-library` cannot take a quoted `-E` expression; pass positional test-name filters instead.
---

# Implementation Log for 2026-10-26-sequence-quadratic-frontmatter (4 phases)

## Phase 1

Wave 1, attribution spike. Before any repair, this phase measures the sequence-size
multipliers in prompt and shell preflight, adds the counters that confirm
them, and maps each one to a repair. The only production-visible change is
counter instrumentation compiled under `work-counters`; `note_work!`
expands to nothing without that feature.

### Scenarios and commands

All runs are on this macOS host in release builds, one scenario at a time.

| What | Command (repository root unless noted) |
| ---- | --------------------------------------- |
| Prompt preflight (C7 at 10/100/1,000 tasks, 100 KB prompt frontmatter) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- C7` |
| Shell preflight (A6 at 10/100 steps, ten holes each) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- A6` |
| A4 sequence preflight (49 distinct prompt documents and one 200-leg group) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- A4` |
| New `with`-hint and `set`-value rows (10/100 steps) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- SQ` |
| Group, container, and A4 group-step rows (C1, C3, A4) | in `claudine/`: `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_c1 pipeline_spike_c3 pipeline_spike_a4` |
| CPU samples | `sample <pid> 15 -f <file>` on the bench binary, then `rustfilt -i <file> -o <file>.dm` |

**Measurement boundary.** The bench's `preflight` closure covers source
resolution, `resolve_sequence_plan`, and `build_preflight_graph`. Context
capture and fixture writes are setup and lie outside the boundary. Rows
report the warm (last) run.

**Profiling repeat.** One C7 or A6 invocation runs for under 2 s, which is too
short for a 15 s sample. For the sampling run only, I wrapped the bench
`main` in a `TMP_SPIKE_REPEAT` loop: 12 repetitions for C7 and 100 for A6,
same inputs and sizes. The loop has since been removed and is not part of
the change. Sampling the wrapper shell's pid by mistake gave an empty first
C7 sample; the C7 rows below come from the second sample, of the binary's
own pid.

### Baseline (reproduces the spec)

| Row | 10 | 100 | 1,000 |
| --- | -: | --: | ----: |
| C7 prompt preflight: bytes | 5.83 MB | 63.04 MB | 2,330.0 MB |
| C7: bytes per task | 583 KB | 630 KB | 2,330 KB |
| C7: allocations | 17,353 | 371,831 | 27,095,864 |
| A6 shell preflight: `dm.ExpressionParse` | 1,900 | 109,000 | — |
| A6: bytes | 3.97 MB | 92.12 MB | — |
| C1: one leg's live-frontmatter copy (payload 0 KB) | 7.65 KB | 48.2 KB | 455.0 KB |
| C1: group-step peak (payload 0 KB, cap 4) | 3.82 MB | 27.75 MB | 669.5 MB |
| A4 preflight (50 steps, 200 legs) | bytes 115.9 MB, peak 22.5 MB | | |
| A4 group step (200 legs) | bytes 728.2 MB, peak 190.0 MB, `LiveFrontmatterCopy` 200 | | |

The spec's 63 MB / 2.33 GB prompt rows, its `10·S² + 90·S` shell count, and
its 455 KB per leg copy and 669 MB peak all reproduce exactly.

### CPU attribution

**C7, 1,000 tasks.** In the sample, `build_preflight_graph_with_context` is
2,257 samples, all of them under `Loader::walk_plan`:

| Callee of `walk_plan` | Samples | Share |
| --------------------- | ------: | ----: |
| `reapproval::early_binding_state`: caller clone 244; `EffectiveStateBuilder::build` 770 (its `RawTable` clone 219, `deep_merge` clone 217, value drops 329) | 1,014 | 45% |
| `Loader::step_frontmatter`'s `HashMap::from_iter` through `stored_text` → `decode_literal_tokens` (a deep copy of the whole document frontmatter) | 693 | 31% |
| `Loader::load_task` (`providers::predict` 69, `resolve_reference` 55, prompt frontmatter copy 32) | 174 | 8% |
| dropping the previous step's `EffectiveState` | 161 | 7% |
| dropping the previous step's `Arc<HashMap>` step frontmatter | 153 | 7% |
| `derived_deferred_keys` (strings scanned, none parsed) | 49 | 2% |

**A6, 100 steps.** `walk_plan` is 10,900 samples:

| Callee of `walk_plan` | Samples | Share |
| --------------------- | ------: | ----: |
| `derived_deferred_keys` over the whole step frontmatter | 6,372 | 58% |
| `Loader::load_task` → `prepare_site` (per-hole `compose_approval_value` 516, `owner_schema` clone 348, `canonical` 302, `DeclaredTypes::for_schema` 248, `project_types` 176, `parse_authored` 168) | 2,200 | 20% |
| `early_binding_state` | 1,098 | 10% |
| `step_frontmatter` decode | 654 | 6% |
| drops of the previous step's state and frontmatter | 352 | 3% |
| `reject_removed_sequence_reads` (once per preparation) | 90 | 1% |

`prepare_site`'s callees are per hole and per site, and they show no
sequence-size term in the counters (see below). Their constant factor, the
three parses per hole, is D8's and is out of scope.

### Counters added (`work-counters` only)

The new claudine kinds count **JSON nodes copied or scanned** (`value_nodes`)
rather than calls. A call count is the same at every sequence size; a node
count grows when the work visits other steps.

| Counter | Site |
| ------- | ---- |
| `PreflightStepFrontmatterNodes` | `walk_plan` root and per-step frontmatter builds; `load_sequence`'s referenced-document frontmatter; `prepare_nested_sequence` / `prepare_nested_group` (`state.data().clone()`) |
| `PreflightDeferredScanNodes` | the `derived_deferred_keys` scans at those same three walk sites |
| `EarlyBindingStateNodes` | `reapproval::early_binding_state`, for every caller |
| `PredictionWithStateNodes` | `providers::predict_key`'s `with` branch (`layers.step_frontmatter.clone()`) |
| `ShellValueStateNodes` | `Loader::record_resolved_bytes` for a `set` value (`(*self.step_frontmatter).clone()`) |
| `PromptReferenceCompare` | `record_prompt_reference`'s linear `prompt_references.contains` |
| `dm.DeferredRootScanParse` (Darkmatter) | each expression parsed by `derived_deferred_keys`'s `value_roots`; it is also counted as an `ExpressionParse` |

### Site / counter table (per step = total ÷ builds or tasks)

Walk sites build their frontmatter S + 1 times: once for the root, then once per step.

| Site | Scenario | 10 | 100 | 1,000 | Per step | Grows with S |
| ---- | -------- | -: | --: | ----: | -------- | ------------ |
| step frontmatter build (`PreflightStepFrontmatterNodes`) | C7 | 559 | 32,689 | 3,026,989 | 51 → 324 → 3,024 | **yes** |
| ″ | A6 | 790 | 34,810 | — | 72 → 345 | **yes** |
| `derived_deferred_keys` scan (`PreflightDeferredScanNodes`) | C7 / A6 | = build | = build | = build | same | **yes** |
| ″ parses (`dm.DeferredRootScanParse`) | A6 | 1,100 | 101,000 | — | 110 → 1,010 (= 10·S(S+1) total) | **yes** |
| other `ExpressionParse` (total − scan) | A6 | 800 | 8,000 | — | 80 → 80 | no (D8 constant) |
| early-binding state (`EarlyBindingStateNodes`) | C7 / A6 | = build | = build | = build | same | **yes** |
| `with` hint state copy (`PredictionWithStateNodes`) | SQ-with | 728 | 52,388 | — | 73 → 524 per task | **yes** |
| `set` value state copy (`ShellValueStateNodes`) | SQ-set | 928 | 72,388 | — | 93 → 724 per site | **yes** |
| prompt reference dedupe (`PromptReferenceCompare`) | C7 (one document, two distinct references) | 17 | 197 | 1,997 | 2 → 2 → 2 | no, bounded by distinct references |
| ″ | A4 (49 distinct documents) | 1,176 = 49·48/2 | | | ∝ distinct references | **yes**, with distinct prompt references |
| provider prediction state (`PredictionStateBuild`, C7's 231 KB) | C7 | 20 | 200 | 2,000 | 2 → 2 → 2 builds, prompt-document sized | no (C7 constant) |
| lifecycle parse (`dm.LifecycleConfigParse`) | SQ-set | 21 | 201 | — | 2 → 2 | no |
| nested preparation copy (`prepare_nested_*`) | C3 nested-sequence-of-3 | 65 nodes per run | | | size of the live state, which carries `sequence:` | **yes** (code reading; the fixture has a small document) |

### Allocation attribution (prompt bytes)

The C7 rows fit `bytes = c + a·tasks + b·nodes` exactly, with `nodes` =
`PreflightStepFrontmatterNodes`: c = 1.38 MB per preparation, a = 409 KB per
task, b = 634 B per copied node. A fourth size not used in the fit
(300 tasks: 278,089 nodes) predicts 300.5 MB; it measures 308.9 MB, within
2.7%.

- At 1,000 tasks the node term is 3,026,989 × 634 B = **1.92 GB of 2.33 GB
  (82%)**: each step's whole-frontmatter decode plus `early_binding_state`'s
  three copies and their drops. The CPU split above matches it.
- The per-task constant (409 MB at 1,000) includes C7's 231 KB of prediction
  state per task (`PredictionStateBuild`) and the per-task loading of the
  prompt document. Neither grows with S, so both stay with Track 5.
- After the repair, the model predicts 422.8 KB per task at 100 tasks and
  410.7 KB at 1,000: a ratio of 0.97, inside the ±10% criterion. The
  per-preparation intercept is kept, not subtracted.

No contributor is left unexplained: the shell `S²` term is entirely the
scan's parses, and the prompt `N²` bytes are the node term.

### Runtime container candidates (for Wave 3's "Isolate group views")

These sites were found by reading code; they copy live state that carries the
`sequence:` list. Group legs are confirmed by `LiveFrontmatterCopy` and C1
above. Wave 3 confirms the others with counters before it repairs them.

- `task/group.rs:471`: `Mutex::new(seed.clone())` for each declared leg
  (confirmed)
- `task/group.rs:74`, `task/group.rs:797`, `task/templates.rs:42`: copies of
  `stack.frontmatter` / `state.data()`
- `task/sequence.rs:37,108,111,157,192,300`: copies of the nested sequence's
  event, gate, and step frontmatter
- `preflight/mod.rs` `prepare_nested_sequence` / `prepare_nested_group`:
  `Arc::new(state.data().clone())`
- `preflight/reapproval.rs` `ShellApproval::verify`: copies
  `basis.frontmatter` whenever there are runtime writes, which happens at
  each re-checked task
- `ApprovalRecordOwned.frontmatter` holds one `Arc` per step, each to that
  step's own full copy, so retained memory grows as S² across a graph's
  approvals

### Source-to-repair map

| Site | Repair (phase) |
| ---- | -------------- |
| `Loader::step_frontmatter` per-step decode (`walk_plan`, `load_sequence`) | shared preparation seed (decoded once) plus a per-step overlay (Phase 2 view, Phase 3 shell owner) |
| `derived_deferred_keys` per step | derived keys computed once per preparation, then each step scans only its overlay against those keys (Phase 3 shell) |
| `early_binding_state` per step (caller clone, builder clone, `deep_merge`) | one base `EffectiveState` per preparation; each step's state shares that base under its overlay (Phase 2 Darkmatter view, Phase 3) |
| providers `with` hint `step_frontmatter.clone()` | classify against the shared view plus one entry (Phase 3 prompt) |
| `record_resolved_bytes` `set` value clone | `ResolvedShellValue::resolve` takes the shared view (Phase 3 shell) |
| `ApprovalRecordOwned.frontmatter` and `verify` | the record holds seed plus overlay; `verify` layers runtime writes over it instead of copying (Phase 3 shell) |
| `prompt_references.contains` | bucket references by `(path, position, in_group)` (Phase 3 prompt) |
| group legs and other runtime container copies | private deltas over the group-start seed (Phase 3 group) |
| `prepare_site` constants, C7 prediction state, three parses per hole | unchanged (constant per step; Track 5) |

### Design Decisions

1. **Node-weighted counters**
   - **decision:** Each new claudine counter adds the JSON node count of the
     map it copies or scans (`work_counters::value_nodes`). Darkmatter gets
     one call-count kind, `DeferredRootScanParse`, inside `value_roots`.
   - **alternatives considered:** one count per call, which is the same at
     every S and cannot show a multiplier; byte probes in production code,
     which would need allocator access the library does not have.
   - **why:** A per-step node count that stays flat from 10 to 100 steps is
     exactly the spec's criterion, so Phase 3 regressions can assert it
     without exact-byte assertions. `DeferredRootScanParse` separates the
     scan's parses from D8's constant three per hole within `ExpressionParse`.
   - **downsides:** A node count says nothing about string lengths. The
     allocation bench still has to prove the bytes (Wave 5).
2. **The shared state lives in Darkmatter's `EffectiveState`**
   - **decision:** Phase 2 gives `EffectiveState` an immutable, `Arc`-shared
     base map with an owned top-level overlay. A preparation builds one base
     state (one copy of the decoded document frontmatter plus caller setters),
     and each step derives its state as base plus overlay (`seq`), without a
     deep copy. Overlay entries shadow whole top-level values, as today's
     `HashMap::extend` does, and an overlay that writes `ctx` or `replace`
     goes through the same merge rules `build` applies. Lookups (`get`,
     `get_checked`, expression scope) read the overlay first. Consumers of
     `data()` move to a layered view; a whole map is materialized only at an
     existing serialization boundary.
   - **alternatives considered:** a Claudine-only fix that keeps building a
     full `EffectiveState` per step (three of the four copies happen inside
     `EffectiveStateBuilder::build`, so it cannot remove the multiplier);
     dropping `sequence` from the per-step map (rejected by plan decision 2,
     because authored data must stay readable); sharing values inside
     `serde_json::Value` (no `Arc` variant exists).
   - **why:** 82% of prompt bytes and 45% of prompt CPU come from
     `early_binding_state`'s copies. The skill's rule ("one request's context
     and resolution authorities") is kept, because the base state is built
     once from the same request context.
   - **downsides:** It changes a Darkmatter public type: `data()`'s about 15
     consumers across both crates need the layered view. Phase 2 must widen
     validation to Darkmatter.
3. **Preparation seed and derived keys in the Loader**
   - **decision:** `Loader` holds one preparation seed: the decoded authored
     frontmatter plus caller setters, its `derived_deferred_keys`, and the
     base state. It is rebuilt only when a preparation starts (`walk_plan`, a
     referenced `load_sequence` document, a nested preparation). Each step
     adds its overlay and computes `derived_deferred_keys(overlay, roots,
     &seed_derived)`. If an overlay key shadows a seed key, the seed's scan
     is redone for that step with the shadowed key replaced, so transitive
     reads stay exact. That path is rare, because `seq` is reserved.
   - **alternatives considered:** process-wide memoization (plan decision 3
     rejects it); hiding `sequence` from the scan, which would change derived
     keys for a document value that reads a step definition.
   - **why:** The A6 parse count then becomes `10·S` (one scan) plus `80·S`,
     the same per step at 10 and 100 steps.
   - **downsides:** A seed with a large inline definition is still scanned
     once per preparation, and the shadowing fallback is a second code path
     that needs its own test.
4. **Prompt-reference dedupe**
   - **decision:** Index recorded references by `(path, position, in_group)`
     and compare `with` maps only within that bucket. The order of
     `graph.prompt_references` is preserved.
   - **alternatives considered:** a `HashSet<PromptReference>`, which is
     impossible because `with` holds `serde_json::Map` and is not `Hash`;
     leaving it, since it costs CPU only.
   - **why:** It grows with the number of distinct prompt references: A4 does
     1,176 comparisons for 49 documents. That is sequence-size-dependent work,
     so it is in scope.
   - **downsides:** One more map in the graph builder.
5. **Profiling repeat is temporary**
   - **decision:** The `TMP_SPIKE_REPEAT` loop used for sampling was removed
     after the samples were taken. The bench keeps only the new `SQ` rows.
   - **alternatives considered:** a permanent repeat flag in the bench.
   - **why:** Nothing else needs it, so keeping it would add surface (Rule 2).
     The sampling method is recorded above.
   - **downsides:** Repeating the CPU sample means re-adding the loop by hand.

### Notes for later phases

- The ignored `pipeline_spike_c1_group_leg_state` takes 6.2 s in release, over
  the five-second budget. Wave 5 must split it into bounded cases or move it
  into the bench, as the plan requires.
- C3's nested-sequence row now reports `PreflightStepFrontmatterNodes` (65)
  from `prepare_nested_sequence`, which makes it a ready regression probe
  for the nested-preparation repair.

### Gates and verification

- `just lint` in `darkmatter/` (run twice: before and after the DMLS test
  fixes) and in `claudine/`: pass.
- `cargo clippy -p claudine -p darkmatter --features claudine/work-counters,darkmatter/work-counters --lib --benches -- -D warnings`: clean, so the feature-on build compiles too.
- `just test` in `darkmatter/`: 12,575 tests passed.
- `just test` in `claudine/`: 11,648 tests passed.
- Release bench rows (C7, A6, A4, SQ) and the ignored crate-private rows
  (C1, C3, A4 group step) ran with the feature on; the results are recorded
  above.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages` = biscuit-hash, claudine, darkmatter, darkmatter-cli, dmls,
  test-toolkit; `change_class` = package. The worktree also holds other
  fixes' uncommitted edits. Gates were run, not skipped.
- Downstream: the Darkmatter change is one `note_work!`, which compiles to
  nothing without `work-counters`. Only Claudine and DMLS enable that
  feature, and both were tested in full.
- Cross-OS: no paths, processes, or OS-specific code changed, so no
  cross-check was run.

**Load-sensitive DMLS tests, which failed before this phase.** In the first two
Darkmatter `just test` runs,
`registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`
failed and passed alone. It still failed with this phase's Darkmatter edits
reverted, so this phase did not cause it. Cause: the test counted only
`request complete` lines, but a request whose elapsed time (which includes
the deliberately held preparation) crosses the 500 ms `SLOW_THRESHOLD` is
logged as `slow request` (WARN). The captured log showed exactly that for
request 502. After the fix,
`startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration`
failed the same way, for request 900 behind a held startup probe. Both tests
now accept either completion event for their id. The assertion's intent, one
completion event that records the wait or the cancellation, is unchanged.
