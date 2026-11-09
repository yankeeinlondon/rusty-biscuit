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
  Phase 3 is done (only L1 test files; no source changed); read "## Phase 3" in this
  log. Facts for Phase 4 (DMLS): (1) with a computed file, `CallCheck::binding_error` is
  `None` for a refused options object; argument 2 is marked incompatible instead (same
  `dm.expression.incompatible_call`, same span), so DMLS ranges must come from the
  argument span, not from `binding_error`. (2) Literal `{format: "csv", header: false}`
  beside a literal `null` file with an object contract is rejected passively as
  `dm.data.row_shape_conflict` (a Phase 2 decision under review); keep DMLS consistent
  with the library. (3) Use `tuple([string, string])[]` for list rows; `string[][]` is
  not a type. (4) `just cross-check` mangles a quoted `-E 'test(...)'` filter; call
  `./scripts/cross-check.sh --os windows <pkg> ...` directly for a scoped run.
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
  - |-
    An entry computed for a caller whose lower-layer entry came from an older memo
    generation now carries that older generation forward, so no layer above it is
    memoized either. Before, the returned generation was the current one, which would
    have let a binding built from stale bytes fill the new maps once three layers chain.
    The plan reused `single_flight` "unchanged"; this one-line change was needed for its
    rule R8 to hold. There is no reasonable alternative; reverting it reintroduces the
    stale-fill bug, which a unit test now catches.
  - |-
    When the binder already refuses the options argument (an unknown key, a misspelled
    format name, a wrong field type), the passive check reports no option problem of its
    own, because a run reports the binder's error first. The plan did not say. The
    alternative was to also judge the literal members (for example a bad delimiter beside
    an unknown key) and let that message replace the binder's; that would make the
    editor and composition disagree about which problem a document has. Changing it
    touches only `options_bind` in `call_check.rs`.
  - |-
    The applicability and row-shape checks run passively whenever the format is provable,
    including beside a literal `null` file when `format` is a literal. At execution a
    `null` file returns `null` before options are read, so such a call would not fail at
    run time. The plan said literal option mistakes are rejected for a `null` file and
    defined "provable" without excluding a `null` file, so this follows its letter: a
    literal `{format: "json", delimiter: ";"}` is a mistake whatever the file is. The
    alternative is to skip steps 3 and 4 for a literal `null` file; that is a two-line
    change in `load_data_problem`.
  - |-
    `string[][]` is not a valid type (nested arrays are not in the grammar), but the
    loading-data doc listed it as a list-row contract and the new headerless error message
    suggested it. The message and the doc row now name only `tuple([…])[]`. The spec also
    names `string[][]`; it is left as written. Adding nested arrays to the grammar was out
    of scope (the spec rules out grammar changes).
  - |-
    The raw-layer entry's "memo answered" method is `answered_from_memo`, not
    `from_memo` like the other layers: clippy refuses a `from_*` method taking `self` on
    a crate-private type (the public ones are exempt). The plan's `SourceEntry::key()`
    accessor was not added because nothing reads it outside tests. Both are naming-only.
  - |-
    The plan asks the slot tests to use a "recording gate" and to call memo invalidation and
    the target collector directly. All three are private to the library, so the tests reach
    them through public paths instead: a recording approval handler for slot reads, a real
    `::shell` command to advance the memo generation, and the public slot pre-flight and URL
    discovery functions. The alternative was to make those internals public for tests, or to
    move these tests into the library's own unit tests. Either would widen the API or lose
    the public-path coverage. Choosing that instead means rewriting
    `load_data_options_slots.rs` against the newly exposed items.
  - |-
    `passive_and_execution_messages_are_identical` does not build the execution error by
    hand. It composes a real document where the same option value comes from frontmatter,
    so it is judged only when it runs, and compares that message with the editor-side one.
    Building the error directly would copy private message text and compare the code with
    itself. If the reviewer wants a direct comparison, the message builder would need to be
    public.
  - |-
    The plan says an explicit `format` grants no network access. With no `format` on an
    extensionless URL, one request still reaches the server, because the existing pre-fetch
    step downloads the URL before `load_data` rejects the format. The response is never
    used. The remote test therefore checks only the `dm.data.unsupported_format` failure and
    not that zero requests were made. Making pre-fetch skip calls whose format cannot be
    decided would be a new library rule; it was left for review rather than added in a
    tests-only phase.
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
  - darkmatter/lib/src/markdown/compose/expression/data/load.rs
  - darkmatter/lib/src/markdown/compose/expression/data/parse.rs
  - darkmatter/lib/src/markdown/compose/expression/data/locate.rs
  - darkmatter/lib/src/markdown/compose/expression/data/error.rs
  - darkmatter/lib/src/markdown/compose/expression/data/contract.rs
  - darkmatter/lib/src/markdown/compose/expression/data/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/data/option_execution_tests.rs
  - darkmatter/lib/src/markdown/compose/expression/call_check.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
docs_updated_during_phase_2:
  - darkmatter/docs/topics/loading-data.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/expressions.md
  - .claude/skills/darkmatter/requests.md
source_files_during_phase_3:
  - darkmatter/lib/tests/l1/load_data_options.rs
  - darkmatter/lib/tests/l1/load_data_options_passive.rs
  - darkmatter/lib/tests/l1/load_data_headerless.rs
  - darkmatter/lib/tests/l1/load_data_options_remote.rs
  - darkmatter/lib/tests/l1/load_data_options_slots.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
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

## Phase 2

### Work done

- **Memo layers** (R7–R12, `data/runtime.rs`): `DataMemo<R, S, B>` with one
  `Generation { raw, sources, bindings, contracts }`. `RawSource { text:
  Result<Arc<str>, usize>, len, hash }` is keyed by identity; `SourceKey
  { identity, format, delimiter }` keys the parse layer; the private
  `BindKey { source, contract, header }` keys the bind layer. `load_raw`
  counts `reads` (no hit counter), `load_source` counts `source_hits`, `bind`
  takes the effective header. `LoadedSource::hash` moved to `RawSource`.
  Module and stats docs rewritten.
- **Generation-chaining fix found while writing the tests.** `single_flight`
  returned the *current* generation when an entry was computed for a stale
  caller, so the next layer would have memoized a binding built from stale
  bytes into the new maps. It now passes the stale `expected` generation on.
  Before this phase only the bind layer chained, and its returned generation
  was discarded, so the old behavior was not observable;
  `invalidation_clears_all_three_layers_and_a_late_raw_load_never_populates_them`
  fails without the fix.
- **Parse and row view** (R13, R15): `parse.rs::decode_bytes`, `utf8_failure`,
  `decode_local` over them; `parse_text(text, format, delimiter, identity)`.
  `TabularSource { mode, header, data }` with `row_count()`/`row(i)`;
  `load.rs::table_rows` and `locate.rs::table_offset` read rows only through
  them.
- **Execution flow** (R9, R10, R14, R16, `data/load.rs`): the R14 order. The
  effective format is `options.format` or the extension; `check_applicability`;
  row mode plus `require_positional_rows` for `header: false`; fetch; raw →
  parse → bind. The read is observed once per call after the parse layer.
  Span fields `delimiter`, `header`, `raw_memo` added. The extension mapping
  is now `load.rs::data_format_of`, shared with the call check. A6/A7 held in
  Phase 1, so no remote or slot file changed (R27).
- **Passive check** (R17–R19, R25, R26, `call_check.rs`):
  `ArgumentType::PartialObject { shape, literals, computed }`, classified and
  displayed as `Typed(shape)`. `load_data_problem` follows the R18 order over
  a private `OptionsView` and `provable_format`; `data_problem` ranges
  `InvalidOptions` at argument 2; `load_data_type` takes two or three
  arguments. `is_tabular_path` became `format_of_path`.
- `data/mod.rs`: removed the `expect(dead_code)` on `options`.

### Requirement → test mapping (Phase 2)

All tests are L1, in-process through the crate's public function path
(`call_function`, `check_expression_calls`, `expression_type`) or the memo's
own API (F2).

| Behavior | Test |
| --- | --- |
| raw read once for two parse keys; raw failure shared | `data::runtime::tests::raw_source_is_read_once_for_two_parse_keys`, `a_raw_failure_is_shared_by_two_parse_keys` |
| header is its own binding over one parse | `runtime::tests::a_header_choice_is_its_own_binding_over_one_parse` |
| invalidation clears all layers; late raw load fills nothing | `runtime::tests::invalidation_clears_all_three_layers_and_a_late_raw_load_never_populates_them`, `a_parse_from_an_older_raw_generation_is_not_memoized` |
| single-flight across raw and parse | `runtime::tests::concurrent_callers_share_one_raw_load_and_one_parse` |
| one generation for all layers | `runtime::tests::memo_generation_advances_once_per_invalidation`, `option_execution_tests::invalidation_rereads_every_layer` |
| UTF-8 failure format-free until named | `parse::tests::invalid_utf8_offset_is_format_free_until_named`, `option_execution_tests::invalid_utf8_is_read_once_and_each_format_names_itself` |
| custom delimiter | `parse::tests::custom_delimiter_parses`, `option_execution_tests::two_delimiters_read_once_and_parse_twice` |
| logical headerless view; `/0/1` | `parse::tests::headerless_view_includes_the_first_record`, `locate::tests::headerless_root_and_rows_use_the_logical_view` |
| `{format: "csv", delimiter: ";"}` on `.txt`; explicit format overrides extension | `option_execution_tests::csv_format_with_a_semicolon_reads_a_txt_file`, `an_explicit_format_overrides_a_supported_extension` |
| headerless rows, parse once, bind twice | `option_execution_tests::header_false_reads_the_first_record_as_a_row_from_one_parse` |
| TSV tab only; CSV with tab; fails before reading | `option_execution_tests::tsv_accepts_only_a_tab_and_fails_before_reading` |
| tabular options on tree formats (alias names canonical format), nothing read, still recorded | `option_execution_tests::tabular_options_on_a_tree_format_fail_before_reading` |
| headerless object rows (literal and on-disk import) fail before reading | `option_execution_tests::headerless_object_rows_fail_before_reading` |
| null file ignores options | `option_execution_tests::a_null_file_ignores_its_options_and_reads_nothing` |
| null options/fields are defaults | `option_execution_tests::null_options_and_null_fields_are_the_defaults` |
| aliases share entries | `option_execution_tests::aliases_share_the_canonical_formats_entries` |
| observation re-recorded at every layer (N17) | `option_execution_tests::every_call_observes_the_read_whichever_layer_answers` |
| coerced option values are applied | `catalog::tests::load_data_options_bind_through_the_shared_binder` (adjusted, see below) |
| `PartialObject` keeps literals, classifies as its shape | `call_check::load_data_options_tests::a_partial_object_keeps_its_literal_members_and_classifies_as_its_shape` |
| R18 verdict order, argument 2 vs 1 | `load_data_options_tests::literal_option_values_are_judged_in_execution_order` |
| computed options/format defer; literal beside computed still judged | `load_data_options_tests::valid_or_unprovable_options_are_not_judged` |
| binder-refused options get no data problem | `load_data_options_tests::options_the_binder_refuses_are_the_binders_error_alone` |
| typing for both arities | `load_data_options_tests::both_arities_keep_the_contract_type` |
| passive message equals execution message | `load_data_options_tests::the_passive_message_is_the_execution_message` |
| two-argument behavior unchanged | existing `load_data_*`, `compose_read_set`, slot, call-check suites, unmodified |

The exhaustive L1 files (P, X, H, W, S) are Phase 3's; these in-file tests
cover what this phase changed and stay as unit-level guards.

### Test changes to existing tests

- `runtime.rs` in-file tests rewritten for the new generics (allowed by the
  plan); each old assertion is kept.
- `catalog::tests::load_data_options_bind_through_the_shared_binder` (added in
  Phase 1) asserted that `{delimiter: 5}` and `{header: "false"}` made the
  call *succeed*, which held only while the handler ignored options. It now
  asserts they bind and are applied: both reach the handler and fail as
  `dm.data.contract_mismatch` over `name,port\nweb,8080\n` (delimiter `5`
  leaves one cell per row; `header: false` reads the header as a row).

### Defect found and fixed

- `string[][]` is not a valid SimplifiedSchema type (nested arrays are not in
  the grammar; `(string[])[]` is refused too). `docs/topics/loading-data.md`
  listed it as a list-row contract, and Phase 1's `HeaderlessObjectRows`
  message suggested it. The message now suggests only `tuple([…])[]`, and the
  doc row lists only `"tuple([string, number])[]"`. The spec also names
  `string[][]` (lines 83, 145); the spec is left as written, per the snapshot
  rule.

### Gates

- Checkpoint 2 set (`test(load_data) | test(compose_read_set) |
  test(design_time_call_check) | test(flow_sensitive_narrowing) |
  test(variable_type_projection) | test(slot) | /expression::data::/ |
  test(catalog) | test(call_check)`): 1,094 passed before the new tests were
  added.
- R24 smell grep on `options.rs`: no matches.
- `just lint`: two clippy findings in new code (`byte_char_slices` in a test,
  `wrong_self_convention` on `RawEntry::from_memo`, renamed
  `answered_from_memo`), then clean.
- `just test` in `darkmatter/` (after every edit, no subagents running):
  **15,754 passed, 0 failed, 10 skipped** (3 slow).
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents (only Claudine consumes `load_data`, the call check, or
  `ArgumentType`), scoped and serial: `claudine --features work-counters -E
  'test(call_check) | test(load_data) | test(expression) | test(lifecycle)'`
  1,122 passed; `claudine-cli --features test-fixtures -E 'test(context) |
  test(load_data) | test(expression) | test(completion)'` 765 passed.
  `ArgumentType::PartialObject` is a new variant of a public enum; no crate
  outside `call_check.rs` matches it exhaustively.
- Cross-OS check skipped: this phase changes in-memory memo, parse, and
  passive-check logic only; no path, process, or OS-specific code. New test
  fixtures use `tempfile` and `DataSourceIdentity::local`.
- Gate-skip rule not applicable: the phase changes compiled source.

## Phase 3

### Work done

The five L1 stub files from Phase 1 now hold the plan's P, X, H, W and S
tests (88 tests). Five parallel subagents wrote one file each, and each ran
only its own nextest filter. No source file changed in this phase.

| File | Tests | Covers |
| --- | --- | --- |
| `lib/tests/l1/load_data_options_passive.rs` (P) | 22 | passive judgment through `check_expression_calls` / `expression_type`: row-shape conflict at argument 1 (literal format, computed file, provisional import not judged); every invalid delimiter, TSV non-tab, and tabular options on JSON (including `format: "json"` on `.csv`) as `dm.data.invalid_options` at argument 2; binder refusals (unknown key, unbindable field, bad format names `CSV`, `" csv"`, `""`, `text`, `YML`) as the binder's error with no data problem; computed and partial options deferred, literal members beside computed ones still judged; `{header: null}` on JSON accepted; `yml` and `ndjson` messages name YAML and JSONL; a literal mistake beside a `null` file; a dead ternary branch; result typing at both arities, null-file typing, `map` callback member typing; the passive message equals the execution message |
| `lib/tests/l1/load_data_options.rs` (X) | 37 | the R24 matrix (control row; 23 invalid cells, literal and computed, all `reads == 0` with an empty read set; null and absent cells sharing one read, parse and bind; coercion exceptions `delimiter: 5` → `"5"`, `header: "false"` → `false`; two wiring cells located at line 1 and the column of the options argument) and every X row of the acceptance table |
| `lib/tests/l1/load_data_headerless.rs` (H) | 18 | one test per row of the spec's headerless table; `/0/1`, a later row, a quoted multiline field, multibyte text, and CRLF bytes all asserted at line and column; BOM column (extra); `header_true_…` control; `header_false_then_true_share_one_parse` in both orders (`reads 1, parses 1, binds 2`) |
| `lib/tests/l1/load_data_options_remote.rs` (W) | 6 | wiremock on loopback: an extensionless URL with `{format: "json"}` keeps its `?id=7` query in the read set; without a format the call is `dm.data.unsupported_format`; a denied host is refused after an allowed run warmed the memo; a warmed allowed host makes `expect(1)` requests; the reference check defers and a later composition reads; a redirect is refused even with an explicit format |
| `lib/tests/l1/load_data_options_slots.rs` (S) | 5 | the counters show raw, parse and bind hits each re-recording the observation; a refusing slot handler stops the call before any warmed layer (data-layer counters unchanged); one generation advances for all layers after a `::shell` command; a three-argument target is collected by `compose_slot_approvals` without a read; `load_data`, `loadData` and `LOAD_DATA` three-argument URLs are discovered before any fetch |

### Requirement → test mapping (Phase 3)

Every row of the plan's "Acceptance criteria → tests" table marked P, X, H,
W, or S has a test with its exact name, except one: H
`headerless_later_row_multiline_multibyte_crlf_positions` is split into
`headerless_later_row_failure_is_located_at_its_own_record`,
`headerless_later_row_after_a_quoted_multiline_field_is_located`,
`headerless_multibyte_text_before_the_failing_cell_counts_characters`, and
`headerless_crlf_positions_match_lf_positions`, so each position shows its
own failure. Boundary: the crate's public compose and passive APIs,
in-process, at fidelity F2; filesystem through `tempfile`, network through a
loopback wiremock. All tests are L1, the largest takes 0.13 s, and
`just check-tier-coverage darkmatter` reports 0 stranded.

### Findings (behavior asserted as found)

- **Missing-file precedence holds only at execution.** With a computed
  delimiter, a missing file reports the file-reference error (the contract is
  prepared first, so `contract_preparations == 1`). With the literal
  `{format: "json", delimiter: ";"}` the passive check reports
  `dm.data.invalid_options` first, because it never looks for the file.
  `missing_file_keeps_precedence_over_applicability` asserts both.
- At execution a missing file is a bare `ExpressionError::FileReference`, not
  one wrapped in `DataError::FileReference`; the X helper accepts either.
- Literal binder refusals report `dm.expression.incompatible_call`. Computed
  ones report `dm.expression.evaluation_failure`, naming "options (argument 3)".
  With a computed file, `CallCheck::binding_error` is `None`, and argument 2
  is marked incompatible instead (same code, same span).
- A computed `null` file still runs the binder: computed `{quote: 1}` fails,
  but option mistakes that the binder accepts return `null`. With a literal
  `null` file and a literal tabular `format`, the row-shape rule applies
  passively. That follows the Phase 2 decision already in
  `decisions_for_review`, and P asserts it.
- A run without `format` on an extensionless URL still sends one request:
  pre-fetch discovery fetches argument 0 before `load_data` refuses the format.
  Nothing from that response is used. W does not assert "no request" there.
- The HTTP client never follows a redirect, so the redirect test shows that an
  explicit format does not get past that refusal (generic
  `dm.expression.evaluation_failure`; the test checks the message).
- In a slot, the contract is prepared before the slot gate (`load.rs`), so a
  refused slot call adds one `contract_preparations`. S asserts only the five
  data-layer counters.
- The slot gate trait, `invalidate_memo`, and the target collector are
  crate-private. S observes them through public paths instead: a recording
  `ShellApprovalHandler::approve_slot_reads`, a real `::shell` command, and
  `Markdown::compose_slot_approvals` / `discover_remote_urls_from_expressions`.

### Gates

- `just lint` in `darkmatter/`: clean (after all five files landed).
- `just test` in `darkmatter/`, no subagent running: **15,842 passed, 0
  failed, 10 skipped** (Phase 2: 15,754; +88).
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents: no source changed, only test files inside `darkmatter`'s own
  `l1` binary, so no dependent package can be affected; none were run.
- Cross-OS: the new tests compare read-set paths and write CRLF and
  multibyte bytes, so the five files were run on native Windows:
  `./scripts/cross-check.sh --os windows darkmatter --features
  effects-instrumentation,work-counters --test l1 -E 'test(/^load_data_options::|^load_data_options_passive::|^load_data_headerless::|^load_data_options_remote::|^load_data_options_slots::/)'`
  → **88 passed** (`windows pass`). `just cross-check …` with the same
  arguments fails on the quoted filter (`syntax error near unexpected token
  '('`), so the script was called directly.
- Darkmatter skill: no update needed; this phase changed no behavior.
