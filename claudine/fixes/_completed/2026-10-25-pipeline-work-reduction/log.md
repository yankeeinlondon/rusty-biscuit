---
kind: implementation-log
spec: 2026-10-25-pipeline-work-reduction
implementation_1: "2026-10-27T15:20:33-07:00"
---

# Implementation log: pipeline work reduction

## Implementation of Review Findings #1

> **started at:** 2026-10-27T15:20:33-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-25-pipeline-work-reduction/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it stays `false` until the cycle closes

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Lifecycle and body loop bindings copy the whole previous result | fixed | all children fixed or clean; decision: new Darkmatter `RuntimeBinding::Members { member, whole }` (alternatives: whole-root `Lazy`, which still copies on the first scalar read, rejected by the review; passing the full path to the resolver, a wider contract); `LoopAmbient.last_output` is `Arc<Value>` |
| R1-F1.1 | Lifecycle bindings (incl. task/container events via `StackExecutionContext::bindings`): scalar `loop.*` read → zero `LoopAmbientMaterialize` | fixed | `lifecycle_loop_binding_construction_builds_no_loop_object` 1→0; `lifecycle_loop_scalar_reads_build_no_loop_object_beside_a_large_result` 0 with correct values; `executor::tests::binding_contract::loop_member_reads::event_loop_scalar_reads_build_no_loop_object` via `execute_event` → `StackExecutionContext::bindings`, 0 |
| R1-F1.2 | Composition host globals (body/frontmatter): scalar `loop.*` read → zero `LoopAmbientMaterialize` | fixed | `composition_loop_global_construction_builds_no_loop_object` 1→0; `iteration_bindings::work_counts::scalar_loop_reads_in_body_and_frontmatter_build_no_loop_object` (real `prepare_direct`) 1→0; Darkmatter `host_globals_compose` transcluded child builds nothing |
| R1-F1.3 | `LoopExpressionLookup::get` condition path → zero (control) | clean | `loop_leaf_reads` 4/4 unchanged (control) |
| R1-F1.4 | `EventMetaExpressionLookup::get` hook path → zero (control) | clean | `hook_leaf_reads` 6/6 unchanged (control) |
| R1-F1.5 | Bare `loop` / `hook` whole-root read → one materialization, same value | fixed | new coverage: bare `loop` builds exactly 1 object equal to `to_value()`; bracket access 1 per session; `loop.missing` null; nested `loop.last_output.*` equals whole-root projection (`lifecycle_loop_member_reads_equal_the_whole_object_projection`); Darkmatter `binding_contract::member_aware` 6 tests |
| R1-F2   | Property-union rewriting loses the caller's file-match origin | fixed | all children fixed or clean; decision: `CallerOrigins` gains a whole-instance form, property arms compile with their owning top-level property's origin and are retained by arm address and origin (alternative: hand property arms the full property-keyed map, rejected because it matches keyword locations by name, the mismatch the old code avoided) |
| R1-F2.1 | Root arm, direct eager file, caller origin → `launch/a.md` | clean | `caller_origin_direct_root_arm_file` → `launch/a.md` before and after |
| R1-F2.2 | Reference to recursive union, caller origin → `launch/a.md` | fixed | `caller_origin_reference_to_recursive_union`: absolute path → `launch/a.md` |
| R1-F2.3 | Additional property union `node \| number`, caller origin → `launch/a.md` | fixed | `caller_origin_property_union_with_reference`: absolute → `launch/a.md` |
| R1-F2.4 | Array-item traversal `node[]`, caller origin → `["launch/a.md"]` | fixed | `caller_origin_list_of_recursive_union`: `[absolute]` → `["launch/a.md"]` |
| R1-F2.5 | Inline-object traversal `{ path: node }`, caller origin → `{ "path": "launch/a.md" }` | fixed | `caller_origin_inline_object_of_recursive_union`: `{path: absolute}` → `{"path": "launch/a.md"}` |
| R1-F2.6 | Root arm, document origin → `a.md` | clean | `document_origin_*` control → `a.md`, passes before and after |
| R1-F2.7 | Reference to recursive union, document origin → `a.md` | clean | `document_origin_*` control → `a.md` |
| R1-F2.8 | Additional property union, document origin → `a.md` | clean | `document_origin_*` control → `a.md` |
| R1-F2.9 | Array-item traversal, document origin → `["a.md"]` | clean | `document_origin_*` control → `["a.md"]` |
| R1-F2.10 | Inline-object traversal, document origin → `{ "path": "a.md" }` | clean | `document_origin_*` control → `{"path": "a.md"}` |
| R1-F2.11 | Retained-validator key includes origin identity (two properties, different origins, same arm) | fixed | `caller_origin_and_document_origin_properties_keep_their_own_arm_validators`: `spec` absolute → `launch/a.md`, `other` → `a.md`; `RewriteArmCompile` = 4 cold (2 arms × 2 origins, independent of 20 items), 0 warm |
| R1-F1.6 | Sibling: `LoopFrame::ambient` copied `last_output` on every call (uncounted) | fixed | frame stores `Arc<Value>`; `every_ambient_shares_the_frame_result_without_copying_it` asserts pointer equality |
| R1-F1.7 | Sibling: every Darkmatter `RuntimeBinding` match (`read_global`, `kind`/`Debug`, `HostGlobals::identity`, `check_agreement`, `subtree::undeclared_view`) | fixed | each handles `Members`; no other workspace crate matches the enum; identity uses the cache pointer like `Lazy` (possible same-run compose-cache miss, never a wrong hit) |
| R1-F2.12 | Sibling: one arm address reached under two top-level properties with different origins | fixed | unit `rewrite::tests::a_shared_arm_is_judged_from_each_owning_property_origin`; reverting the key to address-only failed it |
| R1-F2.13 | Sibling: root-union arms (`root_arm`) | clean | already compiled with the effective schema's caller origins; R1-F2.1 control |
| R1-F2.14 | Sibling: free `rewrite_eager_file_values` helper | clean | takes no caller origins; its documented contract is document context, so every property is document-origin |
| R1-F2.15 | Sibling: `EffectiveSchema::validate` | clean | the verdict the rewrite now matches; every row asserts it is valid |
| R1-F2.16 | Sibling: compose `select_file_mode_on_path`, `root_schema_arm_applies`, coercion validators | clean | the first strips `match` and judges in the record's own origin; `root_schema_arm_applies` passes `caller_origins(records)`; coercion validators judge no origin by design |

### Progress

- starting the work on 'R1-F1 lifecycle and body loop bindings' at 15:21:19
        - defect class: a namespace root bound eagerly (or lazily as a whole) at a host-global boundary copies unrelated large members before a scalar consumer selects its member
        - Darkmatter's `RuntimeBinding` offers only whole-root `Eager`/`Lazy`; the repair needs member-aware resolution at that boundary
- wrote the R1-F1 regressions first; before the repair they failed as the review predicted
        - `lifecycle_loop_binding_construction_builds_no_loop_object` and `composition_loop_global_construction_builds_no_loop_object` observed 1 `LoopAmbientMaterialize` each (expected 0)
        - `scalar_loop_reads_in_body_and_frontmatter_build_no_loop_object` (real `prepare_direct` path) observed 1 (expected 0)
- Darkmatter: added `RuntimeBinding::Members { member, whole }` (`RuntimeBinding::members`, `MemberProvider`)
        - a dotted `root.name.rest` read calls `member(name)` and selects `rest` with the same projection an eager binding uses; a bare read, or a read the evaluator does not spell as a dotted path (bracket access), runs `whole` once per session through the shared lazy cache; after that, member reads select from the cached value
        - sibling sweep of every `RuntimeBinding` match: `read_global`, `kind`/`Debug`, `HostGlobals::identity` (cache-pointer identity, as for `Lazy`, so no canonical JSON of the whole value is computed), `check_agreement`, `subtree::undeclared_view`; no other workspace crate matches the enum (grep: claudine and darkmatter only; dmls names it only in a text guard)
        - `with_document_fallbacks` and transclusion inherit unchanged: both go through `HostGlobals::resolve` → `read_global`; a transcluded child's member read builds nothing and a bare read builds once per composition (shared cache)
- Claudine: `EnclosingLoop::value` replaced by `EnclosingLoop::binding`; `runtime_bindings_enclosed` and `composition_host_globals` bind `loop` with `RuntimeBinding::members(member_named, to_value)` over an owned ambient clone; `Pending` stays eager `null`, `Outside` stays unavailable
        - to make the `'static` clone free, `LoopAmbient.last_output` is now `Arc<Value>`; sites updated: `LoopAmbient::new`, `member`, `loop_control.rs` pass-result write, two tests
        - sibling R1-F1.6: `LoopFrame::ambient` copied `last_output` on every call (uncounted); the frame now stores `Arc<Value>` and shares it with every ambient (`every_ambient_shares_the_frame_result_without_copying_it`)
        - remaining `LoopAmbient::to_value` production callers: `LoopExpressionLookup::get` for bare `loop` and the new binding's `whole` provider, both whole-root requests by contract
- docs: `claudine/docs/topics/flow-control/looping.md` (dotted reads are cheap everywhere, not only in the condition), `darkmatter/docs/topics/darkmatter-expressions.md` (Host Bindings: the member-aware variant), skills `claudine/SKILL.md` and `darkmatter/expressions.md`
- verification (scoped): new and control tests 25/25 claudine + 7/7 darkmatter; claudine lifecycle/looping/sequence::task 1134/1134; claudine lifecycle/looping/prepare/preflight/hook 1300/1300; darkmatter binding/host_globals/subtree/expression 947/947; claudine+darkmatter guards 297/297; claudine-cli loop/agent_text/composition_outputs 534/534; clippy `-D warnings` clean for darkmatter, claudine (work-counters), claudine-cli; `cargo check` default features for darkmatter, darkmatter-cli, dmls, claudine, claudine-cli, claudine-contract
- work completed for 'R1-F1 lifecycle and body loop bindings' at 15:38:40
- starting the work on 'R1-F2 property-union caller origin' at 15:39:05
        - defect class: origin information that validation used is dropped between validation and a later decision over the same value, so the later decision judges the value from a different base
- wrote the R1-F2 regressions first (`darkmatter/lib/tests/l1/schemas_rewrite_arm_validators.rs`, one `#[test]` per row via `origin_rows!`, resolved through `DarkmatterSchemas::effective_for`, then `EffectiveSchema::validate` and `normalize_frontmatter`)
        - before the repair: R1-F2.2/.3/.4/.5 and .11 failed, each validating but normalizing to the absolute `…/launch/a.md` instead of `launch/a.md`; R1-F2.1 and .6–.10 passed (clean controls)
- Darkmatter: `CallerOrigins` gained a whole-instance form (`CallerOrigins::whole_instance(ValueOrigin)`, judged by `origin_at` whatever property a keyword's location names; distinct cache-key bytes)
        - `rewrite.rs::ArmValidators` drops the origin-free `property_cache`; `rewrite_object` calls `enter_property(name)` before walking each top-level property, and `property_arm` compiles the arm with that property's whole-instance origin and retains it by arm address *and* origin
        - a document-origin property uses the empty map, so its validators keep the previous cache key and the D1 reuse
        - R1-F2.12 (unit, `rewrite::tests::a_shared_arm_is_judged_from_each_owning_property_origin`): one arm address reached under a document property and a caller property; a mutation that retained by address alone failed it (`spec` stayed absolute), restored afterwards
- sibling sweep: root-union arms (`root_arm`, already the effective schema's caller origins) clean; the free `rewrite_eager_file_values` takes no caller origins and keeps its document-context contract, clean; `EffectiveSchema::validate` clean (it is the verdict the rewrite now matches); compose's `select_file_mode_on_path` strips `match` and judges in the record's own origin, clean; `root_schema_arm_applies` passes `caller_origins(records)`, clean; coercion's structural validators judge no origin by design
- docs: `darkmatter/docs/topics/schemas/definition.md` (union-arm validators section: property-arm origin and per-origin retention, with example), `.claude/skills/darkmatter/schema.md`, `rewrite.rs`/`file_match.rs`/`validate.rs` doc comments
- verification (scoped): `schemas_rewrite_arm_validators` 17/17; `schemas_rewrite | recursive_schema_caller | caller_origin | normalize | rewrite:: | file_match | schema_cache_validity` 224/224; `schema | caller | match` 2533/2533 (work-counters, effects-instrumentation); `cargo check -p darkmatter -p darkmatter-cli -p claudine` clean; `just lint` in darkmatter clean
- work completed for 'R1-F2 property-union caller origin' at 15:49:44
- orchestrator reconciliation at 16:21:41
        - read both diffs (Darkmatter `binding.rs` `Members` variant and `read_global`; `rewrite.rs`/`validate.rs`/`file_match.rs` origin threading) and re-ran every gate myself
        - discovered: the exported `CDPATH` lists `/Volumes/coding/wt/rusty-biscuit/feat-pipeline` first, so a relative `cd claudine` landed in another worktree and one lint run checked the wrong tree; all gates below were rerun with `CDPATH=` and absolute paths
        - fixed: `just lint` (default features) failed with dead code `large_text` in `lifecycle/bindings/tests/loop_member_reads.rs`; moved the helper into its `work-counters`-gated module

### Completion Report

- **outcome: complete**; elapsed about 61 minutes (15:20:33 → 16:21:41)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1 fixed: lifecycle (`runtime_bindings_enclosed`, through `StackExecutionContext::bindings`) and composition host-global (`composition_host_globals`) `loop` bindings now resolve `loop.<member>` without building the whole object; binding construction 1 → 0 `LoopAmbientMaterialize`, scalar reads 0, bare/bracket reads 1 with the unchanged value; sibling `LoopFrame::ambient` copy removed (R1-F1.6)
        - R1-F2 fixed: property-level union arms compile with their owning top-level property's origin and are retained per (arm, origin); caller-origin rows R1-F2.2–.5 and .11 now normalize to `launch/a.md`, document controls unchanged, D1 reuse kept (`RewriteArmCompile` independent of item count, 0 warm)
- instance rows: 7 fixed + 2 clean under R1-F1 (incl. siblings R1-F1.6/.7); 6 fixed + 10 clean under R1-F2 (incl. siblings R1-F2.12–.16); none open
- open findings: none
- implementer decisions recorded in the review's `human_review_items` (`RuntimeBinding::Members` and its compose-cache identity; `CallerOrigins` whole-instance form)
- no performance measurement was required by these findings; `deferred_perf_measurement` not set

#### Changed files

- R1-F1: `darkmatter/lib/src/markdown/compose/expression/{binding.rs,mod.rs}`, `darkmatter/lib/src/markdown/compose/subtree.rs`, `darkmatter/lib/tests/l1/{binding_contract.rs,host_globals_compose.rs}`, `claudine/lib/src/composition/lifecycle/bindings.rs`, `claudine/lib/src/composition/looping/{expression.rs,frame.rs}`, `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`, tests `claudine/lib/src/composition/lifecycle/bindings/tests.rs`, `.../bindings/tests/loop_member_reads.rs` (new), `.../lifecycle/executor/tests/binding_contract.rs`, `.../looping/engine/tests/{iteration_bindings.rs,iteration_actions.rs}`, `.../looping/frame/tests.rs`; docs `claudine/docs/topics/flow-control/looping.md`, `darkmatter/docs/topics/darkmatter-expressions.md`, `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/expressions.md`
- R1-F2: `darkmatter/lib/src/markdown/schemas/{validate.rs,file_match.rs,rewrite.rs}`, `darkmatter/lib/tests/l1/schemas_rewrite_arm_validators.rs`; docs `darkmatter/docs/topics/schemas/definition.md`, `.claude/skills/darkmatter/schema.md`

#### Final verification (macOS unless noted, `CDPATH=`)

- `darkmatter: just test` → 12,973 passed, 10 skipped (includes dmls); new rows: `just test caller_origin` 8/8, `just test member_aware` 7/7 (R1-F1, R1-F2)
- `claudine: just test` → 11,759 passed, 15 skipped; `just test loop_member_reads` 7/7 (R1-F1)
- `claudine: just lint` → pass (after the dead-code fix); `darkmatter: just lint` → exit 0 (incl. zed-dmls wasm check)
- `just check-tier-coverage claudine` and `darkmatter` → 0 stranded
- `cargo build -p darkmatter -p darkmatter-cli -p claudine -p claudine-cli` (default features) → ok; only the pre-existing `__eh_frame` linker warning
- `just cross-check darkmatter --os windows` (build-win-native) → 9,955 passed, 66 skipped (R1-F2 path/origin risk)
