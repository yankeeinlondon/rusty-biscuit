---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-tabular-input-options/spec.md"
plan: "darkmatter/features/2026-11-05-tabular-input-options/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/expression/data/options.rs
  - darkmatter/lib/src/markdown/compose/expression/data/error.rs
  - darkmatter/lib/src/markdown/compose/expression/data/contract.rs
  - darkmatter/lib/src/markdown/compose/expression/data/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
  - darkmatter/schemas/partials/functions.yaml
  - darkmatter/schemas/partials/lifecycle.yaml
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/l1/load_data_options.rs
  - darkmatter/lib/tests/l1/load_data_options_passive.rs
  - darkmatter/lib/tests/l1/load_data_headerless.rs
  - darkmatter/lib/tests/l1/load_data_options_remote.rs
  - darkmatter/lib/tests/l1/load_data_options_slots.rs
  - darkmatter/dmls/tests/l1/main.rs
  - darkmatter/dmls/tests/l1/option_arguments.rs
  - darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
  - claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap
docs_updated_during_phase_1:
  - darkmatter/docs/functions/load-data.md
  - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - darkmatter
  - darkmatter-lsp
  - claudine-cli
message_to_agent: |-
  Phase 1 is done; read "Rebase check" in this log first. Three things matter for Phase 2.
  (1) R9/R10/R14 were revised: a memo entry stores `hash: [u8; 32]`, not an Observation;
  re-record with `ctx.data.observe_read(&identity, hash)` once per call after the parse
  layer (`None` when the read or the parse failed), which is what `load.rs` does today.
  The slot gate (`ctx.authorize_slot_read`) already runs right after resolution, before
  `ctx.data.record`; keep it there. (2) `data/mod.rs` declares `options` with
  `#[cfg_attr(not(test), expect(dead_code, …))]`. Delete that attribute once
  `load_data`/`call_check` use the items; otherwise `just lint` fails on an unfulfilled
  expectation. (3) The catalog already has the three-argument signature, so a
  three-argument call binds today and the handler ignores argument 2. The `///` on
  `load_data_fn` and the `//!` in `load.rs` still describe two arguments; R14 rewrites
  them. Every `dm.data.invalid_options` message must come from
  `options::invalid_options_message` (reached through `DataError`'s Display) so passive
  and execution text are byte-identical. `format_name` in `error.rs` is now
  `pub(super)` for the options messages.
decisions_for_review:
  - |-
    The rebase check found two shape differences from what the plan assumed, and the plan
    rules were edited to match the landed code. (1) A memo entry stores the BLAKE3 hash of
    the bytes it read, not a whole observation record, so the new raw-source entry (R9)
    carries `hash` and the per-call re-recording (R10) uses the existing `observe_read`
    call. To keep today's outcome, that call is made once per call after the parse layer,
    with `None` when the read or the parse failed. (2) The slot-read gate already runs
    right after the file is resolved, before the read set is recorded and before the
    format is chosen; R14 now lists it there instead of after the option checks. The
    alternative was to move the gate later. That would let an option error be reported
    for a read the gate refuses, and it would change how dynamic-schema slots are
    ordered. Redoing this means editing R9/R10/R14 and the Phase 2 load order.
  - |-
    The new `options.rs` model is not used by `load_data` until Phase 2. To keep the
    lint gate green (warnings are errors), its module declaration carries
    `#[cfg_attr(not(test), expect(dead_code, …))]`. Because it is `expect`, not
    `allow`, the attribute becomes a warning as soon as Phase 2 uses the items, which
    forces its removal. The alternative was to make the items `pub`, which would widen
    the public API for no reason. Nothing needs redoing beyond deleting the attribute.
  - |-
    `DataOptions::from_bound` reports a non-object options value or an unknown key as
    `InvalidOptions { key: "options", … }`. The plan names only the three option keys,
    but such a value cannot reach this function after the shared binder ran (the binder
    rejects it first, at argument 2, which the catalog test proves). The alternatives
    were to panic, which is unacceptable on a load-bearing path, or to add a separate
    variant, which would be dead weight. Changing it touches only `options.rs`.
  - |-
    The catalog description drops "(chosen by its extension)" from its first sentence,
    because an explicit `format` can now override the extension. It describes the three
    option keys, the aliases, the defaults, the TSV tab-only rule, and the non-tabular
    error, as R3 requires. Rewording it later means rerunning `just regen-expr-doc` and
    `just regen-lifecycle-schema`.
---

# Implementation Log for 2026-11-05-tabular-input-options (6 phases)

## Phase 1

### Rebase check

`2026-11-05-dynamic-schema-types` is closed and committed (`cc54ccea0`, moved
to completed); `git status` shows no uncommitted change under the data module,
`functions/`, `compose/remote.rs`, or `schemas/slots/`. (The working tree
holds unrelated uncommitted edits from another session under `claudine/`,
`dmls/docs/editors/README.md`, `docs/topics/schemas/dmls-schema-support.md`,
and `features/2026-11-05-yaml-locator-hardening/`; this phase does not touch
them.)

- **A1 (N17): holds, with a shape difference (revised R9, R10).** The log is
  `DataRuntime::observations` (`Observation { entry, kind, hash }`,
  `data/runtime.rs:112`), `read_mark()` / `observations_since()`
  (`runtime.rs:587,592`). What the memo entry stores is not an `Observation`
  but `LoadedSource::hash: [u8; 32]`; `load.rs` re-records it on every call
  with `ctx.data.observe_read(&identity, hash)` after `load_source`
  (`hash: None` for a memoized failure), and observes `None` through the
  `unread` closure for a failure before the read (unsupported format, row
  shape, fetch refusal). R9/R10 now say "the hash" and `observe_read`.
- **A2 (N17): holds.** `record_probed_candidates(&file_ref, ctx)` runs before
  the not-found error (`load.rs:71`); `DataRuntime::prepare_contract`
  observes missing imports through `contract_observations` (`runtime.rs:649`).
- **A3 (N18): holds.** `simplified_schema::resolve::buffers::read(path)` in
  the source loader (`load.rs:115`).
- **A4 (N28): holds, earlier than the plan assumed (revised R14).**
  `ctx.authorize_slot_read(FUNCTION, ReadKind::Content, …)` runs right after
  resolution and **before** `ctx.data.record(&identity)`, format choice, the
  fetch, and the memo (`load.rs:77-83`). It precedes steps 9–12 as required;
  R14 now lists it between resolution and recording.
- **A5 (N35): holds.** `DataRuntime::memo_generation()` returns
  `DataMemo::generation()`, the `Generation::number` replaced by
  `DataMemo::invalidate` under the one lock (`runtime.rs:315-325,604`).
- **A6 (N20): holds.** `REMOTE_READ_FUNCTIONS` no longer exists; discovery
  uses `remote::fetches_url(name)` derived from the registration's
  `read_fact` (`remote.rs:436`), `load_data` registers
  `.reads_or_fetches(ReadKind::Content)` which sets
  `SlotClass::InputReading` (`functions/data.rs:5`, `functions/mod.rs:230`),
  and `collect_expression_urls` reads `args.first()` at any arity
  (`remote.rs:410`).
- **A7 (N29): holds.** The collector is
  `schemas/slots/grants/targets.rs::collect_targets` (path differs from the
  plan's `slots/targets.rs`); `reads_of` reads `args.first()` at any arity
  (`targets.rs:85`).
- **A8 (memo shape): holds.** `DataMemo<S, B>`, `single_flight`,
  `Generation { number, sources, bindings, contracts }`, and
  `DataRuntimeStats { reads, parses, binds, source_hits, bind_hits,
  contract_preparations, contract_hits, invalidations }` (`runtime.rs:173-311`).
- **A9 (catalog and call check): holds.** `schemas/partials/functions.yaml:792`
  keeps `parameters(file: file | null, as: type-definition(container))`;
  `call_check.rs::load_data_problem` (`:1096`), `load_data_type` (`:236`),
  `data_problem` (`:1145`), `is_tabular_path` (`:1155`) keep their shapes.
- **A10 (DMLS): holds.** N42 landed (`dmls/src/providers/frontmatter/enum_item_tests.rs`);
  `type_definition_argument` wiring is at `dsl.rs:57,60` and
  `frontmatter.rs:75`.

Plan revisions made for this check: R9 and R10 (hash instead of an
`Observation` value), R14 (gate position). See "Decisions applied".

### Baseline

- `just test` (started before any edit of this phase, 2026-11-08): 15,559 passed,
  1 failed, 10 skipped, 150 not run (fail-fast). The failure,
  `darkmatter-lsp::l1 validation_timing_log::the_editor_setting_overrides_the_file_threshold`,
  passed when rerun alone while the host's load average was about 25–30, so it is
  recorded as **load-sensitive** and pre-existing.
- `just lint` was not run separately before the edits; the phase-end lint (below)
  is clean.

### Work done

- **Option model** (R4): `data/options.rs` holds `DataOptions::from_bound`,
  `check_applicability`, `format_from_name` (exact names plus the `yml` → YAML and
  `ndjson` → JSON Lines aliases), `delimiter_byte`, `effective_delimiter`,
  `effective_header`, and `invalid_options_message`, which is the one message
  builder for both the error's `Display` and the later passive check. Its items are
  not called from `load_data` yet (Phase 2).
- **Error** (R5): `DataError::InvalidOptions { key, reason }`, `INVALID_OPTIONS_CODE`
  (`dm.data.invalid_options`) added to `DATA_DIAGNOSTIC_CODES` and re-exported;
  `written_contract()` is `None` and it is not a deferral.
- **Row shape** (R6): `RowShapeConflict::HeaderlessObjectRows` and
  `require_positional_rows` (object rows refused; list and provisional rows stay
  open), exported from `data/mod.rs`.
- **Catalog** (R2, R3, R29): one overload,
  `options?: {format: enum(…nine…), delimiter: string, header: boolean} | null`, plus
  the description text. `just regen-expr-doc` and `just regen-lifecycle-schema`
  rewrote only the `load_data` text in `docs/functions/load-data.md`, the generated
  table row in `docs/topics/darkmatter-expressions.md`, and
  `load_data-short`/`load_data-long` in `schemas/partials/lifecycle.yaml` (the
  generator added the optional tuple element and two long forms).
- **The R2 risk is retired**: the shared binder binds a closed inline-object
  parameter. `{quote: ","}`, `{header: [1]}`, `{format: "CSV"}`, `{format: " csv"}`,
  `{format: "text"}`, `{delimiter: [","]}`, `5`, `"csv"`, and `[]` all fail binding
  with every argument error at index 2; `null`, `{}`, `{header: null}`, and
  `{format: null, delimiter: null}` bind. The R24 coercion outcomes are as
  expected: `{delimiter: 5}` and `{header: "false"}` both bind (the handler still
  ignores options in this phase).
- **Stubs** (R23): five lib L1 files and `dmls/tests/l1/option_arguments.rs`,
  declared in their `main.rs`; each has only a module doc.
- **Intended golden updates from the new signature**:
  `dmls/tests/fixtures/response_contracts/a3_completions.json` (the completion
  item's label and detail, regenerated with `DMLS_UPDATE_GOLDEN=1`) and Claudine's
  `l1__context_command__context_expressions_function_order_snapshot.snap`
  (`load_data(file, as)` → `load_data(file, as, [options])`, `INSTA_UPDATE=always`).
  Both diffs are exactly that one signature.

### Requirement → test mapping (Phase 1)

| Behavior | Test |
| --- | --- |
| absent / `null` / `{}` / null fields are defaults | `data::options::tests::defaults_from_absent_null_and_empty` |
| aliases normalize; failure names canonical format | `aliases_normalize_to_canonical_formats` |
| exact, case-sensitive, untrimmed names | `format_names_are_exact` |
| valid and invalid delimiters, shared message | `valid_delimiters`, `invalid_delimiters_name_the_key` |
| tabular options on tree formats; delimiter before header | `tabular_options_on_tree_formats_are_invalid` |
| TSV tab-only; CSV with tab stays CSV | `tsv_accepts_only_a_tab_delimiter`, `csv_with_tab_stays_csv` |
| omitted and explicit defaults share effective keys | `effective_values_share_defaults` |
| unbound shapes never ignored | `unbound_shapes_are_invalid_options` |
| error code, display, no contract, not a deferral | `data::error::tests::invalid_options_has_its_own_code_and_names_no_contract` |
| headerless object rows refused, list/provisional open | `data::contract::tests::headerless_rows_refuse_known_object_rows_only` |
| catalog parameter shape (optional, nullable, closed, nine members) | `catalog::tests::load_data_options_parameter_is_a_closed_inline_object` |
| binder decides object shape at argument 2 (F2, public `call_function`) | `catalog::tests::load_data_options_bind_through_the_shared_binder` |
| two-argument behavior unchanged | the existing `load_data_*` suites, unmodified, inside Checkpoint 1 and `just test` |

### Gates

- Checkpoint 1 (`cargo nextest run -p darkmatter --features effects-instrumentation -E
  'test(options) | test(catalog) | test(function_binding_registry) |
  test(function_catalog_parity) | test(lifecycle) | test(load_data)'`): 1,177 passed.
- `just test` in `darkmatter/`: first run failed only on the intended
  `a3_completions_are_unchanged` golden; after updating it, **15,724 passed, 0
  failed, 10 skipped**.
- `just lint` in `darkmatter/`: one clippy `byte_char_slices` finding in a new test,
  fixed; then clean.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents: only Claudine names `load_data` or the catalog signature. Scoped runs
  `-E 'test(context) | test(completion) | test(load_data) | test(lifecycle) |
  test(catalog) | test(expression)'`: `claudine-cli --features test-fixtures` 1,055
  passed; `claudine --features work-counters` 1,464 passed. (A first run without
  `test-fixtures` failed `stalled_deliveries_share_one_drain_budget…`, because its
  drain-budget seam compiles only with that feature. It is unrelated to this change
  and passes with the feature that `just test` enables.)
- Cross-OS check skipped: this phase adds no path, process, or OS-specific code.

### Departures

- **R25, execution location of computed-option failures.** The spec asks for
  `dm.data.invalid_options` to be located at the third argument. Passive
  findings are (`DataProblem.argument = 2`), but at execution the evaluator
  carries no per-argument spans, so a failure of a *computed* options object
  is located at the call expression, with the key named in the message. A
  literal mistake is always caught passively first. The docs will say so.

### Decisions applied

- R9/R10/R14 revised by the rebase check (above).
- R2 applied as written (one overload, closed inline object); the binder proof
  passed, so no revision was needed.
