---
kind: log
feature: 2026-11-20-schema-typed-load-data
implementation_1: "2026-11-21T01:59:16+00:00"
---

# Review/Fix Log: Schema-Typed `load_data`

## Implementation of Review Findings #1

> **started at:** 2026-11-21T01:59:16+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-20-schema-typed-load-data/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review `implemented` confirmed `false` before repairs began
- no local OS testing rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); cross-OS proof is left to CI

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Unused branches prevent editor observation of concrete results | fixed | every child R1-F1.1–R1-F1.12 fixed or clean; regression `fm['key']` found and fixed (R1-F1.13); full `just test` 17443 passed |
| R1-F1.1 | Ternary with unavailable `ctx` in the unused arm (both orientations): observe the selected load, report bad age | fixed | lib `editor_observation::an_unselected_ctx_arm_is_not_bound` (+`_when_reversed`); DMLS `typed_load_data_editor::an_unselected_ctx_arm_does_not_withhold_the_verdict` (+`_when_reversed`): one `/direct/1/age` finding, equal to compose |
| R1-F1.2 | `load_data(...) \|\| ctx.runtime`: use the successful load | fixed | lib `editor_observation::an_unused_ctx_fallback_is_not_bound`; DMLS `typed_load_data_editor::an_unused_ctx_fallback_does_not_withhold_the_verdict` |
| R1-F1.3 | `try(load_data(...), ctx.runtime)`: use the successful load without evaluating recovery | fixed | lib `editor_observation::an_unused_recovery_argument_is_not_bound`; DMLS `typed_load_data_editor::an_unused_recovery_argument_does_not_withhold_the_verdict` |
| R1-F1.4 | Unsupported `find_files()` in an unused arm: observe the load, no listing | fixed | lib `editor_observation::an_unselected_listing_is_not_admitted` (no listing observation, `find_files` handler entries 0); DMLS `typed_load_data_editor::an_unselected_listing_does_not_withhold_the_verdict` |
| R1-F1.5 | Absent property in an unused arm: do not bind it; observe the load | fixed | lib `editor_observation::an_unselected_absent_property_is_not_bound`; DMLS `typed_load_data_editor::an_unselected_absent_property_does_not_withhold_the_verdict` |
| R1-F1.6 | `map([], i => ctx.runtime) \|\| load_data(...)`: evaluate without requiring `ctx` | fixed | lib `editor_observation::an_empty_map_callback_body_is_not_bound`; DMLS `typed_load_data_editor::an_unrun_callback_body_does_not_withhold_the_verdict` |
| R1-F1.7 | Control `true ? load_data(...) : []`: observed, one bad-age error in DMLS and compose | clean | lib `editor_observation::an_ordinary_unselected_arm_is_observed`; DMLS `typed_load_data_editor::an_ordinary_unselected_arm_is_judged` |
| R1-F1.8 | Control guarded local read `false ? load_data('gone.yaml') : 'other'`: observes `other`, no reads | clean | `recovery_turns_only_a_proven_miss_into_the_fallback` unchanged and passing: observes `other`, no reads |
| R1-F1.9 | DMLS verdict/member consumers withhold genuinely pending outcomes only | fixed | lib `editor_observation::a_selected_ctx_arm_is_pending`, `an_executed_listing_is_pending_before_it_lists`, `a_selected_absent_property_is_pending`, `a_pending_operand_is_not_recovered`, `a_callback_body_that_runs_is_pending`; DMLS `typed_load_data_editor::a_selected_ctx_arm_withholds_the_verdict`, `an_executed_listing_withholds_the_verdict`, `a_selected_absent_property_withholds_the_verdict`; `slot_jobs::observe` only reads outcomes (clean) |
| R1-F1.10 | Existing DMLS test `unsupported_runtime_only_and_cyclic_properties_are_pending` enshrines wrong behavior for unused `as_markdown` | fixed | `unsupported_runtime_only_and_cyclic_properties_are_pending` updated: reached `as_markdown` stays pending, unused `as_markdown` fallback judged with one finding; `ctx.cwd`/cycle cases still pending |
| R1-F1.11 | Executed unsupported operation stays pending before any effect; passive authored-call rejection still precedes execution | fixed | lib `editor_observation::an_executed_effectful_call_is_pending_before_it_runs` (handler entries 0); DMLS `typed_load_data_editor::an_invalid_call_in_an_unselected_arm_is_still_rejected_passively` (`dm.expression.incompatible_call`, same as compose) |
| R1-F1.12 | Comments and guides describing whole-expression admission | fixed | `docs/topics/schemas/schema-slots.md`, `dmls/docs/diagnostics.md`, `.claude/skills/darkmatter/dmls.md`, `observe.rs`/`editor_reads.rs` module docs; `use-skills` has no observation text |
| R1-F1.13 | (discovered) `fm['key']` regressed to pending under lazy binding: observe the one authored key | fixed | lib `editor_observation::a_literal_fm_index_reads_only_that_key`; `EvaluationLookup::resolve_index` defaults to `None`, so compose is unchanged |
| R1-F1.14 | (discovered) lookup-parity inventory guard did not list the new `PassLookup` | fixed | `lookup_parity::the_inventory_lists_every_lookup_implementation` passes; `PassLookup` classified `Observation` (never binds `ctx`; probed by the `editor_observation` pending rows) |
| R1-F2 | Headerless editor handling follows syntax instead of executed read options | fixed | every child R1-F2.1–R1-F2.11 fixed or clean; syntax predicate deleted; 14 DMLS tests fail when the executed facts are ignored |
| R1-F2.1 | Control literal `{header: false}`: no editor cell error | clean | DMLS `typed_load_data_editor::a_literal_headerless_read_has_no_editor_cell_verdict`: compose fails the bad cell, editor none |
| R1-F2.2 | `{header: 'false'}` binds to false: no editor cell verdict | fixed | DMLS `typed_load_data_editor::a_string_false_header_has_no_editor_cell_verdict` |
| R1-F2.3 | Variable `opts` with `{header: false}`: headerless deferral | fixed | DMLS `typed_load_data_editor::headerless_options_from_a_property_have_no_editor_cell_verdict` |
| R1-F2.4 | `{...{header: false}}`: headerless deferral | fixed | DMLS `typed_load_data_editor::spread_headerless_options_have_no_editor_cell_verdict` |
| R1-F2.5 | Selected headed call with unused headerless arm: judge the headed result | fixed | DMLS `typed_load_data_editor::a_selected_headed_arm_is_judged_beside_an_unused_headerless_arm` (editor equals compose, non-empty); reverse `a_selected_headerless_arm_has_no_editor_cell_verdict` |
| R1-F2.6 | Headed control: destination error reported | clean | DMLS `typed_load_data_editor::a_headed_read_is_judged` (editor equals compose) |
| R1-F2.7 | Per-call shape classifier `observed_call_shapes` uses executed facts | fixed | DMLS `typed_load_data_editor::{literal,string_false,spread}_headerless_cells_stay_provisional`, `headerless_cells_from_property_options_stay_provisional` (hover `unknown[]`); headed control `headed_call_rows_are_typed_from_the_observation` |
| R1-F2.8 | Passive `uncontracted_type` stays a safe fallback; observed facts used once bound | clean | lib `load_data_passive::typing::an_uncontracted_csv_read_narrows_only_from_provable_options`: literal false/`(null)`/literal spread → `(unknown[])[]`; `opts`/`'false'` → `unknown[] \| {}`; computed contract → `unknown` (supersets) |
| R1-F2.9 | Parenthesized/computed null contract and later spread overriding `header` | fixed | DMLS `typed_load_data_editor::a_parenthesized_null_contract…`, `a_null_contract_from_a_property…`, `a_computed_null_contract…`, `a_later_spread_header_false…` deferred; `a_later_header_true_is_judged` judged |
| R1-F2.10 | Callback completion/hover check proving headerless cells remain provisional | fixed | DMLS `typed_load_data_editor::headerless_cells_from_property_options_stay_provisional`: `map(..., r => r)` hover `unknown[]` |
| R1-F2.11 | (discovered) public observation API exposes the fact for executed reads only; a property reading a headerless property defers | fixed | lib `editor_observation::only_an_executed_headerless_uncontracted_read_is_marked`; DMLS `typed_load_data_editor::a_property_reading_a_headerless_property_has_no_editor_verdict` |

### Progress

- starting the work on 'R1-F1: Unused branches prevent editor observation of concrete results' at 01:59:50
        - delegated to a foreground rust-developer subagent (serial: R1-F2 edits the same observation module)
        - defect class confirmed: `admission::property_reads` walked every node (both `?:` arms, every `||` operand, `try` recovery, callback bodies) and `Pass::evaluate` returned pending on `reads.blocked` and eagerly bound every collected root before evaluation; `property_reads` was its only caller
        - design: removed `property_reads`/`PropertyRoot`/`PropertyReads`; calls are admitted when evaluation reaches them (`editor_reads::admit_call`, invoked from `evaluate_function` and `evaluate_callback_function` only when `ResolutionContext::editor` is set, before arguments run, so a refused call is `ObservationPending` before any effect); bindings resolve on demand through a new `PassLookup` (bare names, `fm.NAME`, whole `fm`, `ctx`/`current`/unbound `local` pending), with dependency properties evaluated the first time a reached read names them (pass state behind `RefCell`, `Cycle` when re-entered)
        - `ObservationPending` is non-recoverable, so `try`, `||`, and ternary conditions still propagate a reached pending operand
        - sibling sites: `slot_class_report` (schema-slot admission, `host.rs`) is a different contract and left intact (`schema-slots.md` "What a slot may contain": admission walks the whole expression and every slot is admitted before resolution); DMLS `slot_jobs::observe` and diagnostics consume outcomes only and have no own pre-scan; the passive call check is separate and unchanged
        - tests: 18 new public-API rows in `lib/tests/l1/editor_observation.rs` (observed rows, pending-before-effect rows with handler-entry checks, on-demand dependency/cycle test); 12 new DMLS protocol tests in `dmls/tests/l1/typed_load_data_editor.rs` (editor/compose parity for each row, pending negatives, passive call rejection in an unselected arm); `unsupported_runtime_only_and_cyclic_properties_are_pending` updated (effectful case now reached first; unused `as_markdown` fallback now judged)
        - docs: `schema-slots.md` (observed properties), `dmls/docs/diagnostics.md` pending row, repo skill `.claude/skills/darkmatter/dmls.md`, `observe.rs`/`editor_reads.rs` module docs
        - orchestrator follow-up: the subagent reported that `fm['key']` regressed from observed to pending (the evaluator now resolves it as a whole `fm` read); repaired with a defaulted `EvaluationLookup::resolve_index` hook (evaluator `Index` arm and caller-file-argument arm, forwarded by `ScopedLookup` unless a local shadows the root) that only the observation pass's `PassLookup` answers, so compose is unchanged
                - test `editor_observation::a_literal_fm_index_reads_only_that_key` (observed, template key → `Dependency`, absent key → `RuntimeOnly(fm.nowhere)`, unused whole-`fm` fallback, shadowing local): pass
- work completed for 'R1-F1: Unused branches prevent editor observation of concrete results' at 02:17:03
- starting the work on 'R1-F2: Headerless editor handling follows syntax instead of executed read options' at 02:17:03
        - delegated to a foreground rust-developer subagent
        - defect class confirmed: `observe::reads_headerless_uncontracted` scanned authored syntax (direct `header: false` literal field, unparenthesized literal `null` contract) across every branch; its only users were DMLS `slot_jobs::observe` (whole-property exclusion) and `observed_call_shapes` (call-shape exclusion); no lib, claudine, or test caller
        - design: the loader records the fact it already binds: `load_data_fn`, on success while `editor_reads::collecting_calls` runs, calls `editor_reads::note_read_mode(contract.is_none() && header == Some(HeaderMode::None))` (`header` is `Some` only for CSV/TSV, after option binding, so `"false"`, variables, spreads, and override order all follow the binder); `evaluate_expr` takes that mode for the same `load_data` node (always, so a failed call leaves none) and passes it to `note_call`
        - `CallValues`/`ObservedCall` gained `headerless_uncontracted`; a headerless execution's value is never kept in `values`; `PropertyObservation` gained `headerless_uncontracted` (own executed calls, or a bare read of a property that has it, tracked by a per-evaluation stack in `Pass`)
        - deleted `reads_headerless_uncontracted`; DMLS `observe` withholds verdict and observed shape only for `property.headerless_uncontracted`; `observed_call_shapes` unions observed headed values with `unknown[][]` for any headerless execution (never observed cells)
        - passive `call_check::uncontracted_type` left unchanged: it narrows only from `literal_value` (literal spreads fold correctly, `(null)` is literal), and unprovable options (`opts`, `{header: 'false'}`) keep the general container `unknown[] | {}`; a computed contract is `unknown`; all supersets of the bound result, so safe
        - tests: lib `editor_observation::only_an_executed_headerless_uncontracted_read_is_marked`, lib `load_data_passive::typing::an_uncontracted_csv_read_narrows_only_from_provable_options`; DMLS `typed_load_data_editor`: 10 deferral rows, 3 judged rows, 4 provisional-shape hover rows, one headed shape/completion control (replacing the old single headerless test); 14 of them fail with the executed facts ignored
        - docs: `schema-slots.md` (observed-property exceptions), `dmls/docs/{diagnostics,autocomplete,hover}.md`, repo skill `.claude/skills/darkmatter/dmls.md`, `observe.rs` module and field docs, `editor_reads.rs` docs
- work completed for 'R1-F2: Headerless editor handling follows syntax instead of executed read options' at 02:30:40
- orchestrator reconciliation at 02:32:00–02:39:00
        - inspected the combined diff (`load.rs`, `editor_reads.rs`, expression `mod.rs`, `binding.rs`, `observe.rs`, `admission.rs`, `slot_jobs.rs`, tests, docs); reverted a stray whitespace change (`let invalid =|message`) a subagent introduced in `evaluate_callback_function`
        - first full `just test` failed one guard, `lookup_parity::the_inventory_lists_every_lookup_implementation`: R1-F1's new `PassLookup` was not in the inventory; classified it under a new `Coverage::Observation` (it binds no `ctx`; the `editor_observation` pending rows probe it) and reran: green
        - Claudine (a consumer of the shared evaluator): `cargo check -p claudine --tests` passed; the narrowed Claudine `just test` could not link because the host disk is full (`Disk quota exceeded`, `/` at 100% of 200G). This is a host problem, not a test failure; the shared build cache was left untouched

### Completion Report

- **outcome: complete**: elapsed 2026-11-21T01:59:16+00:00 → 02:41 UTC (about 42 minutes)
- parent findings: **2 total = 2 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed**
        - instance rows: 25, all closed (20 fixed including the discovered rows R1-F1.13, R1-F1.14, and R1-F2.11; 5 clean: R1-F1.7, R1-F1.8, R1-F2.1, R1-F2.6, R1-F2.8)
- verified repairs
        - R1-F1: the editor observation admits each function and binds each name (`fm.X`, bare properties, `ctx`/`current`/`local`, dependent properties, with on-demand cycle detection) only when evaluation reaches it; an executed unsupported call becomes pending before its handler runs; the passive authored-call rejection is unchanged; `fm['key']` reads only that key through a defaulted `EvaluationLookup::resolve_index` hook that compose never answers
        - R1-F2: the loader records, from its bound arguments, whether each executed read was uncontracted and headerless; DMLS uses that per-property and per-call fact for destination verdicts and inferred shapes; the syntax predicate `reads_headerless_uncontracted` is deleted
- open findings: none
- decisions (ledger only; none needs the author)
        - headerless granularity: the whole property verdict is withheld when any executed read feeding it was headerless and uncontracted (the spec's minimum); a call that ran both ways types as the union with `unknown[][]`
        - call admission runs at the evaluator's call entry, before its arguments are evaluated, rather than after binding
- performance: no measurement was required; no test reached 5 s; `deferred_perf_measurement` not set
- OS rigs: none configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); cross-OS proof is left to CI
- changed files
        - library: `lib/src/markdown/compose/expression/{mod.rs,editor_reads.rs,binding.rs,data/load.rs}`, `lib/src/markdown/schemas/slots/{observe.rs,admission.rs}`, `lib/src/markdown/compose/tests/lookup_parity.rs` (R1-F1, R1-F2)
        - DMLS: `dmls/src/slot_jobs.rs` (R1-F2)
        - tests: `lib/tests/l1/editor_observation.rs`, `lib/tests/l1/load_data_passive.rs`, `dmls/tests/l1/typed_load_data_editor.rs`
        - docs: `docs/topics/schemas/schema-slots.md`, `dmls/docs/{diagnostics,autocomplete,hover}.md`, `.claude/skills/darkmatter/dmls.md`
- final verification (from `darkmatter/` unless noted)
        - `LANG=C.UTF-8 just test`: **17443 passed**, 10 skipped, 0 failed, none at or above 5.0 s (R1-F1, R1-F2)
        - `just lint`: passed, including the zed-dmls wasm32-wasip2 check
        - `just check-tier-coverage darkmatter` (repo root): 0 stranded
        - `git diff --check`: clean
        - `cargo check -p claudine --tests` (repo root): passed; the narrowed Claudine `just test` did not build because the disk is full (see reconciliation)
