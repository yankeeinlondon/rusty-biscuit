---
kind: plan
total_phases: 6
created: 2026-11-08
phase: 1
agent: claude/sonnet
yolo: true
---

# Plan: Tabular Input Options for `load_data`

Source: the `2026-11-05-tabular-input-options` spec, finalized 2026-11-08,
including the author decisions of that date: `yml` and `ndjson` are accepted as
`format` aliases and normalized to `yaml` and `jsonl`; on `tsv` only `"\t"` is a
valid `delimiter`; this feature is implemented **after**
`2026-11-05-dynamic-schema-types` and builds on its changes to the `load_data`
runtime and loader. Every implementation choice is settled in **Phase 1 →
Necessary Rules** (`R1`…`R30`). Rules that depend on dynamic-schema-types cite
its rule IDs (`N17`, `N18`, `N20`, `N22`, `N28`, `N29`, `N35`); Phase 1 starts
by checking each such assumption against the landed code.

Execution conventions for every phase:

- Paths are relative to `darkmatter/` unless stated. "Data module" means
  `lib/src/markdown/compose/expression/data/`.
- Run package-area recipes only: `just test` and `just lint` inside
  `darkmatter/` (nextest under the recipe; it covers `darkmatter`,
  `darkmatter-lsp`, and `darkmatter-cli`). A focused run during a wave may use
  `cargo nextest run -p <package> -E '<filter>'`; never `cargo test`.
- Never run `cargo fmt`, never commit, never move the spec, never run
  `just complete`. The terminal state is "implementation complete, ready for
  review".
- Each task that changes a symbol's behavior also fixes that symbol's `///`,
  `//!`, and inline comments in the same change (repo drift rule).
- Docs describe current behavior and never name or link a feature, fix, spec,
  or this plan.
- No test opens a terminal or browser window; no L2/L3 test is added.
- Fixtures use `tempfile` and portable path helpers (`entry`,
  `biscuit_file::to_portable_string`) so they hold on macOS, Linux, native
  Windows, and WSL2; CRLF and invalid-UTF-8 content is written as explicit
  bytes. Load the `os` skill before any path-comparison code.

## Summary and Definition of Done

`load_data(file, as)` gains an optional third argument, a closed options object
`{format?, delimiter?, header?}`:

```md
{{ load_data("rows.csv", "tuple([string, number])[]", {header: false}) }}
{{ load_data("rows.txt", "{ name: string, port: number }[]", {format: "csv", delimiter: ";"}) }}
{{ load_data("https://example.com/export?id=7", "string[]", {format: "json"}) }}
```

The work lands in five layers, in dependency order:

1. **Contract surface**: the catalog signature, the option model with alias
   normalization and the delimiter rules, `dm.data.invalid_options`, and the
   headerless row-shape conflict.
2. **Library judgment and execution**: the passive check shared by composition
   and DMLS; a three-layer memo (raw source → parsed source → bound result) on
   top of the dynamic-schema-types observations, slot gate, and generation
   counter; the new `load.rs` execution order; one logical headerless row view
   shared by binding and diagnostic lookup.
3. **DMLS**: nested option-key and enum-value completion from the declared
   object shape, hover, diagnostics.
4. **Catalog examples, generated pages, hand-written docs, and the Darkmatter
   skill.**
5. **CLI wiring tests and final verification.**

### Definition of Done

- [ ] Phase 1's rebase check passed or its failures were resolved by revising
  the affected rules, recorded in `implementation-log.md`.
- [ ] Every acceptance criterion of the spec has the named passing test listed
  in "Acceptance criteria → tests" below.
- [ ] Every existing two-argument result, diagnostic field, read set, and
  `DataRuntime::stats()` count is unchanged: the pre-existing `load_data_*`,
  `compose_read_set`, and dynamic-schema slot suites pass with no assertion
  changed (only `runtime.rs` in-file unit tests are rewritten for the new
  generics, keeping each assertion's intent).
- [ ] The dynamic-schema guarantees hold over all three memo layers: a memo hit
  at any layer re-records the observation (N17); the slot gate is consulted
  before the first data-source memo layer (N28); one invalidation advances the
  generation for all layers (N35); a three-argument call's target is collected
  passively and found by pre-fetch discovery (N20, N29).
- [ ] Catalog examples pass the passive shipped-artifact corpus check; the
  generated function page and lifecycle types are regenerated, not hand-edited.
- [ ] `docs/topics/loading-data.md`, `docs/topics/caching.md`,
  `docs/cli/data-contracts.md`, DMLS docs, and the Darkmatter skill describe the
  landed behavior.
- [ ] `just test` and `just lint` pass in `darkmatter/`.

Out of scope (settled by the spec): other CSV dialect settings, non-UTF-8
encodings, content or `Content-Type` sniffing, a `columns` option, `bf`
changes, CLI flags, size limits, persistent caches, grammar changes, new
dependencies. No spike and no timing benchmark: reuse is proven by
`DataRuntime::stats()` counts (R30).

### Acceptance criteria → tests

File abbreviations: **P** `lib/tests/l1/load_data_options_passive.rs`, **X**
`lib/tests/l1/load_data_options.rs`, **H** `lib/tests/l1/load_data_headerless.rs`,
**W** `lib/tests/l1/load_data_options_remote.rs`, **S**
`lib/tests/l1/load_data_options_slots.rs`, **D** `dmls/tests/l1/option_arguments.rs`,
**C** `cli/tests/l1/load_data.rs`, **U** in-file unit tests.

| Spec criterion | Tests |
| --- | --- |
| Headerless two-record CSV, TSV, one record | H `headerless_csv_returns_both_records`, `headerless_tsv_returns_both_records`, `headerless_single_record_is_one_row` |
| Headerless object-row conflict: passive literal; execution for computed values and on-disk imports | P `header_false_with_object_contract_is_rejected_at_as`, `header_false_with_literal_csv_format_and_computed_file_is_rejected`, `header_false_with_provisional_import_is_not_judged`; X `computed_header_false_with_object_contract_fails_at_execution`, `imported_object_contract_with_header_false_fails_before_reading` |
| `{format: "csv", delimiter: ";"}` on `rows.txt`; wrong supported extension; TSV tab default; pipe | X `csv_format_with_semicolon_reads_txt`, `explicit_format_overrides_a_supported_extension`, `tsv_format_defaults_to_tab`, `pipe_delimiter_reads_rows` |
| `yml`/`ndjson` aliases behave as and share memo entries with `yaml`/`jsonl` | U `options.rs::aliases_normalize_to_canonical_formats`; X `yml_alias_shares_parse_and_bind_with_yaml`, `ndjson_alias_shares_parse_and_bind_with_jsonl`, `alias_failure_names_the_canonical_format` |
| `{format: "tsv", delimiter: ","}` fails before reading; `"\t"` accepted | P `tsv_with_non_tab_delimiter_is_invalid_options`; X `tsv_with_comma_delimiter_fails_before_reading`, `tsv_with_explicit_tab_delimiter_is_accepted`, `tsv_extension_with_semicolon_delimiter_fails_before_reading` |
| Invalid option objects (unknown keys, unbindable types, bad format names, each bad delimiter); JSON with `header`/`delimiter`; shared diagnostic and third-argument location; no I/O | P `unknown_option_key_is_a_binding_error_at_argument_2`, `unbindable_option_field_is_a_binding_error_at_argument_2`, `invalid_format_names_are_binding_errors`, `each_invalid_delimiter_is_invalid_options_at_argument_2`, `tabular_options_on_json_are_invalid_options`, `passive_and_execution_messages_are_identical`; X `option_matrix_invalid_cells_read_nothing`, `tabular_options_on_json_fail_before_reading`; D `invalid_delimiter_is_ranged_on_the_third_argument`, `unknown_option_key_is_ranged_on_the_third_argument`, `tabular_option_on_json_is_ranged_on_the_third_argument` |
| Computed options validated at execution; no false passive rejection; null options/fields; null-file calls | P `computed_options_beside_a_literal_json_file_are_not_judged`, `partial_object_with_computed_format_defers_format_checks`, `literal_delimiter_beside_computed_member_is_still_judged`; X `computed_invalid_delimiter_fails_at_execution`, `computed_format_overrides_a_literal_extension`, `null_options_and_null_fields_use_defaults`, `null_file_with_options_reads_and_imports_nothing`, `null_file_with_invalid_computed_options_returns_null` |
| Headerless table rows; `/0/1` position; later row, quoted multiline, multibyte, CRLF | H `headerless_empty_and_blank_only_inputs_are_empty`, `headerless_one_record_keeps_empty_and_repeated_cells`, `headerless_blank_lines_are_skipped_with_original_positions`, `headerless_ragged_record_is_malformed_at_that_record`, `headerless_quoted_delimiter_newline_and_doubled_quote_parse_strictly`, `headerless_invalid_quoting_is_malformed_without_a_result`, `headerless_first_record_failure_reports_pointer_0_1`, `headerless_later_row_multiline_multibyte_crlf_positions`, `headerless_missing_cell_uses_nearest_parent`; U `locate.rs::headerless_root_and_rows_use_the_logical_view` |
| Two delimiters → `reads = 1`, `parses = 2`; equivalent defaults share; header-only change; distinct formats; call ordering | U `runtime.rs::raw_source_is_read_once_for_two_parse_keys`; X `two_delimiters_read_once_and_parse_twice`, `equivalent_default_spellings_share_one_parse_and_bind`, `header_only_change_parses_once_and_binds_twice`, `distinct_formats_share_no_parse_or_failure`, `call_order_does_not_change_results` |
| Header-name failure does not poison positional mode; reused failure names each call; invalidation and concurrency cover all layers | X `object_header_failure_does_not_poison_positional_mode`, `reused_failure_names_each_call_site`, `invalidation_rereads_every_layer_in_every_cache_mode`, `concurrent_identical_option_calls_read_once`; U `runtime.rs::invalidation_clears_all_three_layers_and_a_late_raw_load_never_populates_them`, `concurrent_callers_share_one_raw_load_and_one_parse` |
| Extensionless URL with `{format: "json"}`; allowed/denied hosts with a warmed memo; reference-check deferral | W `extensionless_url_loads_with_json_format`, `extensionless_url_without_format_is_unsupported`, `denied_host_is_refused_even_with_a_warmed_memo`, `allowed_host_with_a_warmed_memo_makes_no_second_request`, `reference_check_defers_and_composition_reads_later`, `explicit_format_does_not_bypass_redirect_policy` |
| Both arities keep inferred callback types and nullability in DMLS; two-argument behavior unchanged | P `result_type_is_the_contract_type_for_both_arities`, `null_file_typing_is_unchanged_with_options`; D `both_arities_keep_callback_member_completion`, `both_arities_keep_null_file_typing`; existing suites unmodified (Checkpoint 2) |
| Catalog examples pass the corpus check; CLI headerless success and option failure | existing `function_catalog_parity`, `shipped_corpus_invocation`, catalog `tests.rs` example runner; C `compose_headerless_rows_json`, `compose_invalid_option_json_envelope` |
| Dynamic-schema integration (sequencing note) | S `memo_hits_at_every_layer_rerecord_the_observation`, `slot_gate_refuses_before_any_warmed_layer`, `generation_advances_once_for_all_layers`, `three_argument_target_is_collected_passively`, `three_argument_url_is_discovered_before_fetch` |
| `just test` / `just lint` | Phase 6 final verification |

## Phase 1 — Rebase check, baseline, and contract surface

Goal: prove the plan's dynamic-schema assumptions against the landed code,
record a baseline, and establish every name and type later phases build on.
Nothing here changes a two-argument result.

### Dynamic-schema assumptions (checked by the first task)

Each line names the dynamic-schema rule it depends on and the plan rules that
change if it fails.

- **A1 (N17)** `DataRuntime` has an observation log (`Observation` with a kind
  and an optional BLAKE3 hash), recorded where `load_data` reads bytes, stored
  with the memo entry (on `LoadedSource` or a field it owns), and re-recorded on
  a memo hit; `read_mark()`/`observations_since()` exist. Affects R9, R10, R14.
- **A2 (N17)** `load_data` records its not-found probe before returning the
  not-found error, and `DataRuntime::prepare_contract` records missing contract
  imports. Affects R14 step 4 only by position (it must stay before format
  choice).
- **A3 (N18)** The local read in `load.rs` goes through
  `simplified_schema::resolve::buffers::read(path)`, not `std::fs::read`.
  Affects R9 (the raw loader keeps that call).
- **A4 (N28)** `load_data` calls `ctx.authorize_slot_read(..)` for its data
  target after resolution and before both the remote fetch and the source memo
  lookup; with no gate it is a no-op. Affects R14 step 8 and R27.
- **A5 (N35)** `DataRuntime::memo_generation()` exists, advances once per
  `invalidate_memo`, and is derived from (or bumped under the same lock as) the
  `Generation` replacement in `DataMemo::invalidate`. Affects R8, R10.
- **A6 (N20)** `load_data`'s registration carries its slot class as a fact,
  `REMOTE_READ_FUNCTIONS` (`lib/src/markdown/compose/remote.rs:28`) is derived
  from facts, and pre-fetch discovery inspects only argument 0 without
  requiring an exact arity. Affects R27.
- **A7 (N29)** `slots/targets.rs::collect_targets` reads `load_data`'s target
  from argument 0 without requiring an exact arity. Affects R27.
- **A8 (memo shape)** `runtime.rs` still has `DataMemo<S, B>` with
  `single_flight`, `Generation { number, sources, bindings, contracts }`, and the
  `DataRuntimeStats` fields `reads, parses, binds, source_hits, bind_hits,
  contract_preparations, contract_hits, invalidations` (dynamic-schema may have
  added fields; they are kept). Affects R7–R10.
- **A9 (catalog and call check)** The `load_data` catalog entry
  (`schemas/partials/functions.yaml:792`) and `call_check.rs::load_data_problem`
  / `load_data_type` keep their pre-dynamic shape (N20 may have added a class
  fact to the entry; that is kept). Affects R1, R17–R19.
- **A10 (DMLS)** The N42 enum-completion change to
  `dmls/src/providers/frontmatter.rs` has landed and the
  `type_definition_argument` wiring at `dsl.rs:57,60` and `frontmatter.rs:70`
  still exists. Affects R21.

### Necessary Rules

**Rebase discipline**

- **R1. Rebase check gates the plan.** Before any other task, verify A1–A10
  against the code and against
  `darkmatter/features/2026-11-05-dynamic-schema-types/implementation-log.md`.
  The dynamic-schema feature must report its Phase 4 data-runtime work
  (Observations, gate hook) as implemented; if it is still in progress, stop
  and report to the author rather than build on uncommitted code. For each
  assumption record "holds" or the exact difference (symbol, `path:line`) in
  this feature's `implementation-log.md` under "Rebase check". When an
  assumption fails, revise the affected rules in this plan **then** (edit the
  rule text, log the revision with the reason) before any implementation task
  starts; never work around a mismatch in code. The line numbers below are from
  the pre-dynamic tree (2026-11-08) and are re-anchored by symbol name.

**Contract surface**

- **R2. One overload with an optional third parameter.** The catalog entry
  `load_data` (`schemas/partials/functions.yaml:792-805`, today
  `parameters(file: file | null, as: type-definition(container))`) becomes
  `parameters(file: file | null, as: type-definition(container), options?: {format: enum(yaml, yml, json, jsonl, ndjson, csv, tsv, toml, json5), delimiter: string, header: boolean} | null)`.
  Evidence: `simplified-schema/lib/src/simplified/grammar.rs:1331`
  `parse_parameters` accepts `name?:` followed by `parse_union_arms`, the full
  type grammar; inline-object properties are optional unless `required`, an
  optional property admits `null`, and an inline object is closed. So unknown
  keys, wrong field types, case-variant or untrimmed format names, `null`,
  `{}`, and `{header: null}` are decided by the shared binder with its existing
  diagnostics, as the spec requires ("do not introduce a second, stricter
  native-type rule"). The catalog has no inline-object or `enum` parameter
  today (only `fallback?: number` style, `functions.yaml:170`), so the catalog
  task proves the binder accepts it (R29 risk); if the binder or catalog
  loader cannot bind a closed inline object, stop and revise R2 with the
  author. A single overload keeps one bound path and renders as
  `load_data(file, as, [options])`. `FunctionBinding::context("load_data", …)`
  in `lib/src/markdown/compose/expression/functions/data.rs` needs no change.
- **R3. Description text.** The catalog description gains the third argument:
  the three keys, accepted values (aliases `yml` → `yaml`, `ndjson` → `jsonl`),
  defaults (`format` from the extension; `delimiter` comma for CSV and tab for
  TSV; `header` true), that TSV accepts only a tab delimiter, and that
  `delimiter` or `header` on a non-tabular format is an error. Hover reads this
  text (R20).
- **R4. Option model in a new file** `<Data module>/options.rs`, declared
  `pub(crate) mod options;` in `data/mod.rs`:
  - `pub(crate) struct DataOptions { format: Option<DataFormat>, delimiter: Option<u8>, header: Option<bool> }`
    and `DataOptions::from_bound(value: Option<&Value>) -> Result<Self, DataError>`.
    An absent argument, `Value::Null`, and `{}` give `DataOptions::default()`;
    each key absent or `Null` is `None`. `format` must be a `Value::String`
    accepted by `format_from_name`; `delimiter` a `Value::String` accepted by
    `delimiter_byte`; `header` a `Value::Bool`. Any other shape or key reaching
    this function (unreachable after binding) is `DataError::InvalidOptions`,
    never ignored, defaulted, or `.ok()`'d.
  - `pub(crate) fn format_from_name(name: &str) -> Option<DataFormat>`: exact,
    case-sensitive, untrimmed match of `yaml`, `json`, `jsonl`, `csv`, `tsv`,
    `toml`, `json5`, plus the aliases `yml` → `DataFormat::Yaml` and `ndjson` →
    `DataFormat::Jsonl`. The alias is gone after this call: diagnostics and memo
    keys see only the canonical `DataFormat` (spec: "`{format: "yml"}` and
    `{format: "yaml"}` share entries").
  - `pub(crate) fn delimiter_byte(text: &str) -> Result<u8, DataError>`: exactly
    one `char`, ASCII, not `"`, CR, or LF; otherwise
    `InvalidOptions { key: "delimiter", reason }` with reason "must be one ASCII
    character other than a double quote, CR, or LF (write a tab as \"\\t\")".
    Evidence: `biscuit_file::read_delimited`'s own refusal set
    (`biscuit-file/lib/src/delimited.rs`, `InvalidDelimiter`), surfaced today as
    the unreachable arm at `<Data module>/parse.rs:384`.
  - `DataOptions::check_applicability(&self, format: DataFormat) -> Result<(), DataError>`,
    in this order: (a) for a format other than `Csv`/`Tsv`, a non-`None`
    `delimiter`, then `header` (even `Some(true)`), is `InvalidOptions` with
    reason "applies only to CSV and TSV data, but this call reads {format name};
    remove it or select a tabular format"; (b) for `Tsv`, a `delimiter` other
    than `b'\t'` is `InvalidOptions { key: "delimiter" }` with reason "TSV data
    is tab-separated, so only \"\\t\" is accepted; read data separated by
    another character with format \"csv\"" (author decision 2026-11-08). The rule
    applies to the **effective** format, whether selected by `format` or by a
    `.tsv` extension. `{format: "csv", delimiter: "\t"}` is valid and stays
    `csv` in diagnostics.
  - `pub(crate) fn effective_delimiter(format, chosen: Option<u8>) -> Option<u8>`
    (`Some(chosen.unwrap_or(b','))` for `Csv`, `Some(b'\t')` for `Tsv`, `None`
    otherwise) and `pub(crate) fn effective_header(format, chosen: Option<bool>) -> Option<bool>`
    (`Some(chosen.unwrap_or(true))` for tabular formats, `None` otherwise). These
    feed the memo keys (R7), so omitted and explicit defaults share entries.
  - One `pub(crate) fn invalid_options_message(key, reason) -> String` builds
    the message used by both `DataError`'s `Display` and the passive check, so
    the text is byte-identical (spec: "the same code and message in passive
    checking and execution").
- **R5. New error `DataError::InvalidOptions { key: &'static str, reason: String }`**
  in `<Data module>/error.rs` (enum at `error.rs:67`). `code()`
  (`error.rs:191-199`) returns the new
  `pub const INVALID_OPTIONS_CODE: &str = "dm.data.invalid_options";`, added to
  `DATA_DIAGNOSTIC_CODES` (`error.rs:35-42`) and re-exported from
  `data/mod.rs:26-29`. `written_contract()` returns `None` for it. `Display`:
  ``load_data(): option `{key}` is invalid: {reason}``, naming accepted values.
  It is an error, never a governed warning, and carries no data location
  ("Neither error claims a position in an unread data file"). The code reaches
  every consumer of the catalog: `data_records` in
  `lib/src/markdown/compose/report_diagnostics.rs` projects code and message
  through its default arm; the DMLS code table (`dmls/src/diagnostics/codes.rs:24`)
  and docs gain a row (Phases 4–5). No CLI output schema enumerates data codes
  (`cli/schemas/output/render.yaml:25` names only `malformed` and
  `contract_mismatch` for `data_location`), so no schema changes.
- **R6. Headerless object conflict reuses `dm.data.row_shape_conflict`.** Add
  the unit variant `RowShapeConflict::HeaderlessObjectRows` to `contract.rs:93`
  with display "`header: false` reads rows by position, but this contract
  describes rows as objects; use a `tuple([…])[]` or `string[][]` contract, or
  read an input that has a header". Add
  `pub fn require_positional_rows(class: RowClassification) -> Result<(), RowShapeConflict>`:
  `Known(RowMode::Object)` → `Err(HeaderlessObjectRows)`; `Known(RowMode::List)`
  and `Provisional` → `Ok` (provisional stays open until execution). Exported
  from `data/mod.rs`. Raised as `DataError::RowShapeConflict { written, conflict }`,
  so `code()` and every consumer are unchanged. Header-name validation
  (`load.rs:240` `check_header`) stays specific to object rows.

**Memo (built on N17 / N35)**

- **R7. Three single-flight layers in one generation.** `runtime.rs`'s
  `Generation` (`runtime.rs:177-188`) holds
  `raw: HashMap<DataSourceIdentity, Slot<R>>`,
  `sources: HashMap<SourceKey, Slot<S>>`,
  `bindings: HashMap<BindKey, Slot<B>>`, and the existing `contracts`, where
  `pub(crate) struct SourceKey { identity: DataSourceIdentity, format: DataFormat, delimiter: Option<u8> }`
  and `pub(crate) struct BindKey { source: SourceKey, contract: String, header: Option<bool> }`
  (both `Clone + Hash + Eq`; `DataFormat` already derives `Hash`). `DataMemo`
  becomes `DataMemo<R, S, B>`. Evidence: the spec's effective-key table; the
  current maps are `sources` keyed by identity and `bindings` by
  `(identity, contract key)` (`runtime.rs:173-182`). All maps are replaced
  together by `Generation::new`, so `invalidate` clears every layer and never
  touches the read set (`runtime.rs:263-268`).
- **R8. Generation chaining.** `single_flight` (`runtime.rs:276-311`) is reused
  unchanged for all four maps. The raw layer runs with `expected = None`; the
  parse layer with `expected = Some(raw_entry.generation)`; the bind layer with
  `expected = Some(source_entry.generation)` (as `bind` already does at
  `runtime.rs:349`). An entry produced from an older generation is computed for
  its caller alone and never memoized, so an in-flight read started before an
  invalidation cannot fill the new maps (spec: "the generation check preventing
  an older in-flight read from filling the new maps"). `memo_generation()` (N35)
  keeps reporting the single generation number shared by all layers; if A5
  found a separate counter, it is bumped inside `DataMemo::invalidate` under
  the same lock.
- **R9. Raw-source entry carries the bytes' result and the observation.**
  `pub(crate) struct RawSource { text: Result<Arc<str>, usize>, len: usize, hash: [u8; 32] }`
  (revised by the rebase check: N17 stores the BLAKE3 hash of the bytes read on
  the memo entry, `LoadedSource::hash`, not an `Observation` value):
  `text` is the BOM-stripped decoded text, or `Err(offset)` of the first
  invalid UTF-8 byte in the **original** bytes (BOM counted, the existing rule
  at `parse.rs:48-62`); `len` is the size reported in the `bytes` span field
  (local byte count; fetched body length before BOM stripping, as `load.rs:101`
  does). Decoding once in the raw layer shares one `Arc<str>` across every
  interpretation (spec: "Share the retained bytes/text rather than copying it
  for each format"), while the raw result is format-free; the parse layer turns
  `Err(offset)` into `DataError::Malformed` naming **its** format, cached under
  its `SourceKey` (spec: "Keep format-dependent UTF-8 errors in the
  interpretation layer"). The raw loader reads local bytes through the same
  call A3 found (N18 `buffers::read`) and computes the observation exactly where
  N17 placed it; a fetched body becomes `Ok(Arc::from(strip_bom(&body)))` with
  its observation. A local read failure is the existing memoized
  `DataError::FileReference` (`load.rs:103-111`). The hash moves off
  `LoadedSource` (where N17 stored it) onto `RawSource`; `LoadedSource` keeps
  `content`, `format`, `text: Arc<str>` (an `Arc` clone of the raw text), and
  `bytes`.
- **R10. Observations are re-recorded from the raw layer on every call.**
  `load_data` consults the raw layer on every non-null call (a raw hit is a map
  lookup and bumps no counter), then the parse layer, then the bind layer, as
  three sequential calls; no loader calls another layer, so no `OnceLock` is
  ever waited on while another is held. `load_data`
  records the raw entry's hash through `ctx.data.observe_read(&identity, Some(hash))`
  (revised: N17's recording call), whichever layer later answers; a memoized
  raw failure records `observe_read(&identity, None)`. To keep N17's existing
  outcome, the call is made once per call after the parse layer answers: the
  raw hash when the parse layer succeeded (hit or miss), `None` when the raw
  read or the parse failed, exactly as `load.rs` observes today. So a
  hit at the parse or bind layer still re-records, which N17 requires and which
  a parse-only lookup would lose. This changes no two-argument count: `reads`
  rises only when the raw loader runs and `source_hits` still counts parse-layer
  hits.
- **R11. Counters.** `reads` increments inside the raw loader only (moved from
  `load_source`, `runtime.rs:325`); `parses` via `note_parse()` after a
  successful parse (unchanged); `binds` per binder run (unchanged);
  `source_hits` counts parse-layer hits; `bind_hits` unchanged. No new counter
  and no `raw_hits`: raw reuse reads as `reads` staying flat (spec). The
  `DataRuntimeStats` field docs (`runtime.rs:119-137`) are updated to say this.
- **R12. Memo API.** `DataMemo::load_raw(&identity, loader) -> Result<RawEntry<R>, Arc<DataError>>`
  (`RawEntry { generation, value: Arc<R>, from_memo }`);
  `load_source(&SourceKey, &RawEntry<R>, loader)`; `bind(&SourceEntry<S>, contract_key, header, binder)`.
  `SourceEntry` stores its `SourceKey` (keeping `identity()` and adding
  `key()`); `DataRuntime`'s wrappers (`runtime.rs:470-489`) follow. Deferral
  removal and failure memoization are unchanged: one computation per key for
  concurrent callers, including failures; the reference check's
  `RemoteNotEnabled` deferral is returned before any memo call (R14 step 9), so
  it is still never stored.

**Parsing and the row view**

- **R13. `parse_text` takes the delimiter.**
  `parse_text(text: &str, format: DataFormat, delimiter: Option<u8>, identity: &DataSourceIdentity)`;
  `Csv` and `Tsv` call `parse_delimited(text, delimiter.unwrap_or(b',' / b'\t'))`
  in place of the fixed bytes at `parse.rs:91-92`; other formats ignore it.
  `parse_local_document` (`parse.rs:156`, public) passes `None` and keeps its
  signature. Add `pub(crate) fn decode_bytes(bytes: &[u8]) -> Result<Arc<str>, usize>`
  (the raw layer's decoder) and `pub(crate) fn utf8_failure(identity, format, offset) -> DataError`;
  `decode_local` is reimplemented over them so its message and location do not
  change.
- **R14. Execution order in `load_data_fn`** (replaces `load.rs:52-131` and the
  module doc at `load.rs:1-9`):
  1. `file` is `null` → `Ok(Value::Null)` (the binder already ran, so literal
     and binding mistakes were reported; computed options are not checked
     further, and nothing is imported or read);
  2. `DataOptions::from_bound(call.args().get(2))` (semantic option checks);
  3. read `as` and `prepare_contract` (unchanged, `load.rs:56-60`);
  4. resolve the identity, including the N17 not-found probe; then the N28
     slot gate (`ctx.authorize_slot_read`, revised by the rebase check: it
     already sits here, before recording); then `record` it in the read set
     (unchanged, `load.rs:62-90`);
  5. effective format: `options.format`, else `format_of(&identity)`
     (`load.rs:146-164`, the `dm.data.unsupported_format` path, which an
     explicit format skips entirely);
  6. `options.check_applicability(format)`;
  7. for tabular formats, `contract.row_mode()` (`load.rs:79-84`) and, when the
     effective header is `Some(false)`, `require_positional_rows`;
  8. (the N28 slot gate ran in step 4, at the position A4 found; it is not
     moved);
  9. remote fetch and host policy through `ctx.fetch_remote_text`, on every
     call, before any memo lookup (unchanged, `load.rs:86-95`);
  10. `load_raw(&identity, …)`, then record the observation (R10);
  11. `load_source(&SourceKey { identity, format, delimiter: effective_delimiter(..) }, …)`,
      whose loader turns the raw text into `LoadedSource` (or the format's
      UTF-8 failure), parses with the effective delimiter, and calls
      `note_parse()`;
  12. `bind(…, &contract.key, effective_header(..), …)`.
  Evidence: the spec's "validate bound option values first, then prepare the
  contract, resolve and record the source, choose the effective format, check
  option applicability and row shape, enforce remote policy, read and parse,
  and bind". No parse or fetch follows an option or row-shape error, and
  resolution failures keep precedence over format-dependent checks. The
  `#[tracing::instrument]` fields (`load.rs:46-51`) record the effective format
  and gain `delimiter` and `header` (`Empty` placeholders), plus a `raw_memo`
  flag beside `source_memo`/`bind_memo`.
- **R15. One logical row view.** `TabularSource` (`error.rs:158-163`) gains
  `header: bool` and two methods: `row_count(&self) -> usize` and
  `row(&self, index: usize) -> Option<&DelimitedRecord>`. With `header == true`
  they read `data.rows`; with `false` they read `data.headers` (the first
  nonblank record, which `read_delimited` stores without interpreting) followed
  by `data.rows`. `load.rs::table_rows` (`load.rs:208-232`) and
  `locate.rs::table_offset` (`locate.rs:172-191`, which reads `data.rows`
  directly at `:176`) both iterate only through these methods. The root pointer
  still resolves to the first record's offset. The parsed `DelimitedData` is
  never mutated, no synthetic header is prepended, and no text is rewritten, so
  `/0/1` with no header names the second cell of the first input record at its
  raw offset. Object mode is reachable only with `header == true` (R6), so
  `check_header` runs only there. A table with no records still binds `[]`.
- **R16. `bind_source` takes the effective header** and builds the
  `TabularSource { mode, header, data }` it stores in `ContractMismatch`
  (`load.rs:174-201`).

**Passive judgment**

- **R17. `ArgumentType::PartialObject`.** `call_check.rs:108-121`'s
  `ArgumentType` gains
  `PartialObject { shape: ScopeShape, literals: serde_json::Map<String, Value>, computed: Vec<String> }`,
  produced by `ArgumentType::of` (`call_check.rs:128-136`) only for an object
  literal (through `Paren`) with at least one literal-valued and at least one
  non-literal member. Classification treats it as `Typed(shape)` and `Display`
  delegates to `Typed`, so every other function's verdict is unchanged.
  Evidence: `ArgumentType` is matched only inside `call_check.rs`; without it a
  literal `delimiter` beside a computed member is invisible, violating "check
  literal option values even when another argument is computed".
- **R18. Passive verdict order** in `load_data_problem` (`call_check.rs:1096`,
  today `let [file, contract] = arguments else { return None }`), accepting two
  or three arguments, one `DataProblem` per call in execution order:
  (a) literal option values: a `Value::String` `delimiter` through
  `delimiter_byte`; a `Value::String` `format` through `format_from_name` only
  as the source of the format (bad names are the binder's); (b) the existing
  contract judgment (`literal_contract_problem`, `call_check.rs:1114-1143`,
  unchanged); (c) applicability (R4 `check_applicability`, including the TSV
  delimiter rule) of literal non-null `delimiter`/`header` against the
  **provable** effective format; (d) the tabular row rule, plus
  `require_positional_rows` when a literal `header: false` and a provably
  tabular format are both present. The effective format is provable when a
  literal non-null `format` exists (even with a computed `file`); otherwise
  when the options are closed (omitted, literal `null`, a fully literal object,
  or a `PartialObject` whose `computed` lacks `format`) **and** `file` is a
  literal whose extension is supported (`is_tabular_path`,
  `call_check.rs:1155-1163`, generalized to `provable_format`). A variable,
  call, or `Unknown` options argument, or a computed `format`, makes the format
  unprovable and defers every format-dependent check (spec: "a computed options
  object might override a literal filename's extension"). Dead-branch behavior
  and literal-`null`-file behavior are unchanged; literal option mistakes are
  still rejected for a `null` file because (a) never looks at `file`.
  `data_problem` (`call_check.rs:1145-1151`, today `argument: 1`) maps
  `InvalidOptions` to argument `2` and everything else to `1`, so the editor
  ranges it via `argument_spans` (`call_check.rs:882`).
- **R19. Typing.** `load_data_type` (`call_check.rs:236-254`) accepts
  `[file, contract]` or `[file, contract, _options]` and ignores the third; the
  inferred type stays the literal `as` type with the same null-file handling,
  including for computed options. The `expression_type` doc table row
  (`call_check.rs:150`) and both functions' docs describe both arities.

**DMLS**

- **R20. Hover needs no new code.** `format_function_block` renders
  `typed_signature()` and the catalog description (R3); hover gets a test only.
- **R21. Option completion is a new module** `dmls/src/providers/option_argument.rs`
  (registered in `providers/mod.rs`, called from `providers/dsl.rs::completion`
  beside `type_definition_argument::completion` at `dsl.rs:57` and from
  `providers/frontmatter.rs` beside `frontmatter.rs:70`, for `Site::Body` and
  `Site::Frontmatter`). When the cursor is inside an object literal that is
  argument *n* of a call and the catalog overload's parameter *n* (after
  dropping `null`) is an inline object, it claims the cursor: at a key position
  it offers the declared property names not already present; at the value of an
  `enum` property, the quoted members (all nine for `format`, aliases included,
  because they are declared); at the value of a `boolean` property, `true` and
  `false`; at any other value position it returns `None` so ordinary expression
  completion runs. Names and members come from
  `ExpressionFunctionDescriptor.function_type.parameters[n].def`
  (`TypeExpr::InlineObject(shape).properties`), never from an editor list
  (spec). Plain `object` parameters (`pr_list`, `cicd_list`) are unaffected.
  The shared helpers in `type_definition_argument.rs` (`Analyzed`, region and
  projection/encode helpers) are made `pub(crate)` and reused. Unfinished input
  is recovered with the expression `Lexer` (the technique `string_argument_call`
  uses): tokenize up to the cursor, keep a stack of open `(`/`{` frames with
  call name, argument index, and for `{` the last completed key and whether a
  `:` was seen since the last `,`.

**Remote, slots, and robustness**

- **R22. Remote handling is otherwise unchanged.** Options never change a URL
  identity (query and fragment kept); an explicit `format` grants no network
  access and never bypasses redirect policy; host authorization (step 9)
  precedes every cached remote body and interpretation.
- **R23. Test placement.** New L1 files are registered once in Phase 1 as
  stubs so parallel test tasks never edit a shared `mod` list:
  `lib/tests/l1/{load_data_options, load_data_options_passive, load_data_headerless, load_data_options_remote, load_data_options_slots}.rs`
  in `lib/tests/l1/main.rs`, and `dmls/tests/l1/option_arguments.rs` in
  `dmls/tests/l1/main.rs`. CLI tests extend `cli/tests/l1/load_data.rs`. Helpers
  private to existing files (`write`, `entry`, `compose_file`,
  `load_data_runtime.rs:41-74`) are re-declared locally.
- **R24. Input robustness matrix** (one loader: the options object; validators:
  the shared binder and `DataOptions`). Default for every invalid shape: a
  located error, nothing read (`reads == 0`), nothing bound. Exceptions,
  asserted individually: absent key, explicit `null` key, `null` options, and
  `{}` mean the defaults; the shared coercion engine's normal conversions apply
  (expected: `delimiter: 5` binds to `"5"` and is valid, `header: "false"` binds
  to `false`; the test asserts the engine's actual outcome and, if it differs,
  the implementation log records it; no handler rule is added). Field shapes:
  for each of `format`, `delimiter`, `header`: absent, `null`, wrong type
  (number, list, object), empty string; for `format` additionally `"CSV"`,
  `" csv"`, `"yml"` (valid alias), `"ndjson"` (valid alias), `"text"`; for
  `delimiter` `""`, `",,"`, `"é"`, `"\""`, `"\r"`, `"\n"`, `","`, `";"`, `"|"`,
  `"\t"`; whole-object shapes `null`, `{}`, `5`, `"csv"`, `[]`, unknown key
  `quote`. Control row: unedited `rows.csv` with header and `{}`. Two wiring
  cells (one binder error, one `dm.data.invalid_options`) go through
  `Markdown::compose_with_warnings`; one CLI test proves the output path. Smell
  grep before closing Phase 2: `rg -n 'serde\(default\)|unwrap_or_default\(\)|filter_map\(.*as_str|\.ok\(\)' lib/src/markdown/compose/expression/data/options.rs`
  returns nothing on the option parse.
- **R25. Error locations.** Passive findings are ranged by `DataProblem.argument`:
  `2` for `dm.data.invalid_options`, `1` (`as`) for the row-shape conflict.
  At execution the evaluator carries no per-argument spans (`argument_spans`
  exist only on `CallCheck`, `call_check.rs:767`), so an execution-time
  `dm.data.invalid_options` (computed options only; a literal mistake is always
  caught passively first) is located at the call expression with the key named
  in the message. This narrows the spec's "locate it at the third argument" for
  computed options only; Phase 1's log task records it as a departure and the
  docs state it.
- **R26. Shared message text.** Passive and execution messages for
  `InvalidOptions` and `HeaderlessObjectRows` come from the same builders
  (R4, R6); a test compares them.
- **R27. Dynamic-schema surfaces accept the third argument.** The N29 target
  collector and N20 pre-fetch discovery read `load_data`'s target from
  argument 0 at any arity; the slot class of `load_data` stays `InputReading`;
  an options object of literals adds no read and no forbidden binding (N14). If
  A6 or A7 found an exact-arity match, Phase 2 Wave 2 relaxes it in that file
  (named in the log) and nothing else.
- **R28. Fixtures and catalog examples** (Phase 5): add to
  `lib/src/markdown/compose/expression/catalog/example_data/` and
  `EXAMPLE_DATA_FILES` (`catalog/mod.rs`) `fixture-headerless.csv`
  (`web,8080\ndb,5432\n`), `fixture-semicolon.txt`
  (`name;port\nweb;8080\ndb;5432\n`), and `fixture-export` (no extension; the
  same JSON as `fixture.json`). Exactly three examples are added to the entry:
  `load_data("fixture-headerless.csv", "tuple([string, number])[]", {header: false}) => [["web", 8080], ["db", 5432]]`,
  `load_data("fixture-semicolon.txt", "{name: string, port: number}[]", {format: "csv", delimiter: ";"}) => [{name: "web", port: 8080}, {name: "db", port: 5432}]`,
  `load_data("fixture-export", "{name: string, tags: string[]}", {format: "json"}) => {name: "Ada", tags: ["math", "engines"]}`.
  The generated page lists a data file when an example names it as a quoted
  string; an extensionless file is fenced as `text`.
- **R29. Generated artifacts.** After a catalog edit run `just regen-expr-doc`
  (rewrites `docs/functions/load-data.md` and the table in
  `docs/topics/darkmatter-expressions.md`) and `just regen-lifecycle-schema`
  (rewrites `load_data-short`/`load_data-long` in
  `schemas/partials/lifecycle.yaml:685-688`; the generator decides whether they
  gain the optional element). Both parity checks run in `just test`; hand-edit
  nothing those recipes own.
- **R30. No spike, no performance work.** The spec states work counts answer
  the reuse question; nothing here needs a measurement.

### Wave 1 (serial; gates the plan)

- [x] **Rebase check and baseline** (R1)
  - Read `darkmatter/features/2026-11-05-dynamic-schema-types/implementation-log.md`;
    confirm its data-runtime and gate work (its Phase 4 "Data runtime
    observations", "Gate hook", "Remote-read discovery") is implemented and
    not mid-edit (`git status` shows no uncommitted change under the data
    module, `functions/`, `compose/remote.rs`, `schemas/slots/`). If it is not,
    stop and report.
  - Create `implementation-log.md` in this feature folder with headings
    Rebase check, Baseline, Departures, Decisions applied. Under Rebase check,
    one entry per assumption A1–A10: "holds" with the verified symbol and
    `path:line`, or the difference. For each failure, revise the affected
    R-rules in this `plan.md` now and log the revision; do not proceed until
    every assumption is "holds" or "revised".
  - Seed Departures with R25 (execution location of computed-option failures).
  - Run `just test` and `just lint` in `darkmatter/`; record pass/fail counts
    and any pre-existing failure under Baseline.

### Wave 2 (parallel; disjoint files)

- [x] **Option model and error types** (R4, R5, R6) —
  `<Data module>/options.rs` (new), `error.rs`, `contract.rs`, `mod.rs`
  - Implement `DataOptions`, `format_from_name`, `delimiter_byte`,
    `check_applicability`, `effective_delimiter`, `effective_header`,
    `invalid_options_message`; `InvalidOptions`, `INVALID_OPTIONS_CODE`, the
    catalog entry, `HeaderlessObjectRows`, `require_positional_rows`; exports
    and the module list comment in `data/mod.rs`.
  - Unit tests in `options.rs`: `defaults_from_absent_null_and_empty`,
    `aliases_normalize_to_canonical_formats` (`yml` → Yaml, `ndjson` → Jsonl),
    `format_names_are_exact` (`CSV`, ` csv`, `""`, `text` → `None`),
    `valid_delimiters` (`,` `;` `|` `\t`), `invalid_delimiters_name_the_key`
    (`""`, `,,`, `é`, `"`, CR, LF), `tabular_options_on_tree_formats_are_invalid`
    (each of five tree formats × `delimiter`, `header: true`),
    `tsv_accepts_only_a_tab_delimiter`, `csv_with_tab_stays_csv`,
    `effective_values_share_defaults`, `unbound_shapes_are_invalid_options`.
  - Unit tests in `error.rs` (`code()`, `Display`, `written_contract() == None`)
    and `contract.rs` (`HeaderlessObjectRows` display mentions `header: false`;
    `Provisional` and `List` stay open).
- [x] **Catalog signature** (R2, R3, R29) — `schemas/partials/functions.yaml`
  plus regenerated `docs/functions/load-data.md`,
  `docs/topics/darkmatter-expressions.md` (generated table only),
  `schemas/partials/lifecycle.yaml`
  - Edit signature and description only (examples come in Phase 5). Run both
    regen recipes; check the diff touches only `load_data` text.
  - Add a catalog unit test in `lib/src/markdown/compose/expression/catalog/tests.rs`,
    `load_data_options_parameter_is_a_closed_inline_object`, asserting the
    descriptor's parameter 2 is optional, admits `null`, and is an inline object
    with exactly `format` (nine-member enum), `delimiter`, `header`.
  - Risk: the binder's handling of a closed inline-object parameter has no
    catalog precedent. Prove it in this task with a binder-level test (in the
    same catalog test file) that `{quote: ","}` and `{header: [1]}` fail binding
    at argument 2 and `{header: null}` binds; if the binder cannot do this,
    stop and revise R2 with the author (R1 discipline).
- [x] **Test stubs** (R23) — the five lib stub files, `dmls/tests/l1/option_arguments.rs`,
  `lib/tests/l1/main.rs`, `dmls/tests/l1/main.rs`
  - Each stub has a module doc comment and no tests.

### Checkpoint 1

- [x] `cargo nextest run -p darkmatter -E 'test(options) | test(catalog) | test(function_binding_registry) | test(function_catalog_parity) | test(lifecycle) | test(load_data)'`
  passes; existing `load_data_*` suites pass unmodified (a two-argument call
  still evaluates; a three-argument call binds and is ignored by the unchanged
  handler).

## Phase 2 — Library judgment and execution

Depends on Phase 1. Wave 1 tasks touch disjoint files.

### Wave 1 (parallel)

- [ ] **Memo layers** (R7–R12) — `<Data module>/runtime.rs`
  - Add `RawSource`, `RawEntry`, `SourceKey`, `BindKey`; `DataMemo<R, S, B>`;
    `load_raw` (counts `reads`), `load_source(&SourceKey, &RawEntry, …)` (counts
    `source_hits`), `bind(…, header, …)`; `Generation::new` creates all four
    maps; move the N17 observation from `LoadedSource` to `RawSource`; keep
    `memo_generation()` covering all layers (R8).
  - Update the module doc (`runtime.rs:1-19`: three layers, keys, generation
    chaining, observations live on the raw layer) and the stats field docs.
  - Rewrite in-file unit tests for the new generics, keeping each existing
    assertion's intent (`runtime.rs:541-712`: read once and bind twice,
    memo-answered flags, single-flight, invalidation, stale bind not memoized,
    failure memoized but deferral not, read set survives invalidation, clones
    share state, contract prepared once). Add
    `raw_source_is_read_once_for_two_parse_keys`,
    `a_raw_failure_is_shared_by_two_parse_keys`,
    `invalidation_clears_all_three_layers_and_a_late_raw_load_never_populates_them`,
    `a_parse_from_an_older_raw_generation_is_not_memoized`,
    `concurrent_callers_share_one_raw_load_and_one_parse`,
    `memo_generation_advances_once_per_invalidation`.
- [ ] **Parse and row view** (R13, R15) — `<Data module>/parse.rs`, `error.rs`
  (`TabularSource` only), `locate.rs`
  - `decode_bytes`, `utf8_failure`, `decode_local` over them; `parse_text` with
    the delimiter; module and function docs updated (CSV/TSV sentences).
  - `TabularSource { mode, header, data }` with `row_count`/`row`;
    `table_offset` uses them.
  - Unit tests: `invalid_utf8_offset_is_format_free_until_named` (same bytes,
    two formats, two messages, one offset), `custom_delimiter_parses`,
    `headerless_view_includes_the_first_record`, and in `locate.rs`
    `headerless_root_and_rows_use_the_logical_view` (`/0/1` → line 1, column 3
    for `"a,b\n1,2\n"`; header-skipping behavior unchanged for `header: true`).
- [ ] **Passive check and typing** (R17–R19, R25, R26) —
  `lib/src/markdown/compose/expression/call_check.rs`
  - `ArgumentType::PartialObject`; a private `OptionsView`
    (`Closed(literals)`, `Partial { literals, computed }`, `Unknown`) built from
    the third `ArgumentType`; `provable_format`; the R18 verdict order reusing
    `options.rs` helpers; `data_problem` argument mapping; `load_data_type`
    arity 2 or 3; doc comments for both arities.
  - Risk: `PartialObject` must not change another function's verdict; the
    existing `design_time_call_check`, `flow_sensitive_narrowing`, and
    `variable_type_projection` suites guard it in the checkpoint.

### Wave 2 (after Wave 1)

- [ ] **Execution flow** (R9, R10, R14, R16, R22, R27) — `<Data module>/load.rs`,
  plus only the file(s) the rebase check named under A6/A7, if any
  - Rewrite `load_data_fn` in the R14 order; the raw loader holds the read
    (`buffers::read` per A3) and the observation; the parse loader turns the
    raw text into `LoadedSource` or the format's UTF-8 failure; record the
    observation after every raw lookup (R10); `bind_source` and `table_rows`
    take the effective header and use `TabularSource::row_count/row`.
  - Update the module doc (`load.rs:1-9`), the function doc (`load.rs:33-45`,
    third argument, null file before options), span fields, and the
    `failed_at` test fixture signature string (`load.rs:307`) to the new
    signature text.
  - If A6/A7 found an exact-arity match, relax it to "argument 0 at any arity"
    and adjust its doc comment.
  - Risk: unchanged two-argument counts and read sets; the existing suites are
    the guard and run in the checkpoint before new tests are written.

### Checkpoint 2

- [ ] `cargo nextest run -p darkmatter -E 'test(load_data) | test(compose_read_set) | test(design_time_call_check) | test(flow_sensitive_narrowing) | test(variable_type_projection) | test(slot)'`
  passes with no existing test modified except `runtime.rs`'s in-file unit
  tests.
- [ ] Smell grep of R24 is clean.

## Phase 3 — Library tests

Depends on Phase 2. Each task owns one stub file from Phase 1; all run in
parallel.

### Wave 1 (parallel)

- [ ] **Passive tests** — `lib/tests/l1/load_data_options_passive.rs`
  - Through `check_expression_calls` / `expression_call_diagnostics` /
    `expression_type` with `TypeScope::unguarded` (no filesystem): the P tests
    in the acceptance table, plus: `{header: null}` on a JSON file accepted;
    `{format: "yml", delimiter: ","}` is invalid and names YAML;
    `{format: "json"}` on a literal `.csv` makes the row rule inapplicable;
    literal-`null` file with a literal mistake still rejected
    (`literal_mistake_with_null_file_is_rejected`); a dead ternary branch with a
    bad literal delimiter is still rejected; callback member typing over
    `map(load_data(…, {header: false}), r => r.…)` is unchanged.
  - `passive_and_execution_messages_are_identical` compares the passive message
    with `DataError::InvalidOptions`'s `Display` from the same inputs.
- [ ] **Options matrix, execution, and memo reuse** — `lib/tests/l1/load_data_options.rs`
  - R24 matrix (`option_matrix_control_row`, `option_matrix_invalid_cells_read_nothing`,
    `option_matrix_null_and_absent_cells_use_defaults`,
    `option_matrix_coercion_exceptions`, two wiring cells through
    `Markdown::compose_with_warnings`).
  - The X tests in the acceptance table. Assertions use the composed text,
    the typed `DataError` inside `ComposeFailure`, `DataRuntime::stats()`, and
    the read set. Specifics: `explicit_format_overrides_a_supported_extension`
    reads a `.json` file holding CSV text with `{format: "csv"}`;
    `csv_with_tab_delimiter_reports_csv_in_malformed` asserts the format field;
    `tsv_extension_with_csv_format_parses_with_comma`;
    `missing_file_keeps_precedence_over_applicability` (explicit
    `{format: "json", delimiter: ";"}` on a missing file reports the
    file-reference failure); `imported_object_contract_with_header_false_fails_before_reading`
    uses an on-disk contract import and asserts `reads == 0`;
    `distinct_formats_share_no_parse_or_failure` writes invalid UTF-8 and asserts
    the YAML and JSON failures each name their own format with
    `reads == 1`; `call_order_does_not_change_results` runs every permutation of
    a three-call set; `invalidation_rereads_every_layer_in_every_cache_mode`
    loops over every `CacheAccessMode` and asserts `reads` rises and the read
    set is unchanged; `concurrent_identical_option_calls_read_once` uses
    threads over one `DataRuntime`; `null_file_with_options_reads_and_imports_nothing`
    asserts `reads == 0`, `contract_preparations == 0`, empty read set.
- [ ] **Headerless table and positions** — `lib/tests/l1/load_data_headerless.rs`
  - One test per row of the spec's headerless table and the H tests in the
    acceptance table. Positions assert line and character column so a one-row
    shift fails: first record `/0/1`; a later row; a quoted multiline field
    before the failing cell; multibyte text (`é`, `日本`) before the failing
    cell; CRLF line endings written as bytes. `header_true_skips_the_first_record_for_positional_rows`
    is the control. `header_false_then_true_share_one_parse` loads one file both
    ways in both orders: correct independent results, `parses == 1`,
    `binds == 2`.
- [ ] **Remote** — `lib/tests/l1/load_data_options_remote.rs`
  - `wiremock` local mock following `load_data_runtime.rs`'s `RemoteReadConfig`
    usage; no live endpoint. The W tests in the acceptance table: `GET
    /export?id=7` returning JSON loads with `{format: "json"}` and the read-set
    entry keeps the query; without `format` it fails
    `dm.data.unsupported_format`; a denied host is refused even after an
    allowed context warmed the memo, with no body returned; an allowed host
    with a warmed memo makes one request in total (mock `expect(1)`); the
    reference check without remote permission defers (not cached) and a later
    composition with permission reads; a redirect to a denied host fails with
    an explicit `format`.
- [ ] **Dynamic-schema integration** — `lib/tests/l1/load_data_options_slots.rs`
  - Using the observation, gate, and collector APIs the rebase check recorded:
    `memo_hits_at_every_layer_rerecord_the_observation` (three calls answered by
    the raw, parse, and bind layers each add the observation to
    `observations_since(mark)`); `slot_gate_refuses_before_any_warmed_layer` (a
    document call warms all three layers; a slot call with the same options and
    a refusing recording gate is refused, the gate saw the target, `reads`
    unchanged); `generation_advances_once_for_all_layers`;
    `three_argument_target_is_collected_passively` (N29, a literal file with an
    options object); `three_argument_url_is_discovered_before_fetch` (N20,
    including a folded spelling such as `loadData`).

### Checkpoint 3

- [ ] `just test` in `darkmatter/` is green for `lib`; fix every failure at its
  cause and never weaken an existing assertion.

## Phase 4 — DMLS

Depends on Phases 1–2 (catalog, diagnostics).

### Wave 1 (parallel)

- [ ] **Option completion provider** (R21) — `dmls/src/providers/option_argument.rs`
  (new), `providers/mod.rs`, `providers/type_definition_argument.rs`
  (visibility only), `providers/dsl.rs`, `providers/frontmatter.rs` (one call
  beside the existing `type_definition_argument` call)
  - Wire after the type-definition check and before ordinary completion for
    both sites; items: `PROPERTY` for keys (insert `key: `), `ENUM_MEMBER` for
    quoted enum values, `VALUE` for `true`/`false`; text edits replace only the
    typed partial and are encoded with the site's projection.
  - In-file unit tests over the lexer recovery: cursors after `{`, after `,`,
    after `format: `, inside a partial quoted value, a key already present (not
    offered again), the second argument (no claim), a callback inside the call.
- [ ] **Diagnostics table and DMLS docs** — `dmls/src/diagnostics/codes.rs`
  (module doc table), `dmls/docs/autocomplete.md`, `dmls/docs/hover.md`,
  `dmls/docs/diagnostics.md`
  - Add `dm.data.invalid_options` (ERROR, a literal `load_data` options object
    the passive call check proves wrong, ranged at the third argument; extend
    the `row_shape_conflict` row with `header: false`). Document option
    completion from the declared object shape and the three-parameter hover.
    Write for a developer new to the repo.

### Wave 2 (after Wave 1)

- [ ] **DMLS tests** — `dmls/tests/l1/option_arguments.rs`, and one added case in
  `dmls/tests/l1/call_check_agreement.rs`
  - Completion at body and frontmatter sites: keys at `{` and after `,`;
    `format:` offers the nine declared names; `header:` offers `true`/`false`;
    `delimiter:` falls through; `pr_list({` shows no declared-key completion;
    the offered keys equal the descriptor's inline-object property names (no
    literal copy).
  - Hover on `load_data` shows `options`, the three keys, and the defaults.
  - The D tests in the acceptance table, plus
    `header_false_with_object_contract_is_ranged_on_as` and
    `computed_options_with_literal_json_file_produce_no_diagnostic`; one
    three-argument parity case in `call_check_agreement.rs` (editor diagnostics
    equal composition rejections).
  - `both_arities_keep_callback_member_completion`
    (`map(load_data("hosts.yaml", "{ name: string }[]", {}), h => h.`) and
    `both_arities_keep_null_file_typing`.

### Checkpoint 4

- [ ] `just test` in `darkmatter/` is green (includes `darkmatter-lsp`).

## Phase 5 — Catalog examples, docs, skill

Depends on Phases 2–4. Wave 1 tasks are disjoint; Wave 2 regenerates.

### Wave 1 (parallel)

- [ ] **Catalog examples and fixtures** (R28) — `schemas/partials/functions.yaml`,
  `catalog/example_data/` (three new files), `catalog/mod.rs`
  (`EXAMPLE_DATA_FILES`), `catalog/tests.rs`
  - Add the three fixtures and examples. Add
    `load_data_function_page_shows_option_examples` beside
    `load_data_function_page_shows_every_format_family`: the page shows each
    new example and its input file and mentions the third parameter.
- [ ] **Topic docs** — `docs/topics/loading-data.md`, `docs/topics/caching.md`,
  `docs/cli/data-contracts.md`, `docs/topics/darkmatter-expressions.md`
  (hand-written parts only), `docs/errors/composition-problems.md` (only if its
  `dm.data.*` list enumerates codes), `README.md` (only if its Loading Data line
  now misstates behavior)
  - Lead with what the reader can do, a compact example per rule, and a
    Mermaid diagram for the call flow (R14) and for raw → parse → bind reuse.
  - `loading-data.md`: an "Options" section (key table with aliases, defaults,
    headerless rows with a positional contract, custom delimiter, the TSV tab
    rule, extensionless URL with `format`, applicability errors, invalid-option
    examples); update "How a call runs", "Formats" (an explicit format replaces
    extension detection; CSV with a tab stays `csv`), "CSV and TSV" (the header
    rule is conditional), "Reading a file once per command" (read versus parse
    versus bind), "When loading fails" (`dm.data.invalid_options`; passive and
    editor location at the third argument, the call expression for computed
    options at execution; the key named in the message), "Remote URLs"
    (`format` grants no access).
  - `caching.md` "The `load_data` memo": three layers, the effective-key table,
    equivalent default spellings and aliases, `reads`/`parses`/`binds`/hits
    semantics, invalidation clears every layer and never the read set,
    observations recorded on every call.
  - `data-contracts.md`: the code tables (near lines 337 and 544), a headerless
    example, and the `md compose --output json` option-failure example
    described field by field (code, severity, span; no `data_location` because
    no data file was read).
  - No doc names or links a feature, fix, spec, or this plan.
- [ ] **Darkmatter skill** — `.claude/skills/darkmatter/expressions.md`
  (`load_data` section), `requests.md` ("Memo shape"), `dmls.md`, `testing.md`
  - Signature with `options`; aliases and TSV rule; `dm.data.invalid_options`;
    passive verdict order and `PartialObject`; the three memo layers, keys,
    counters, observation and gate placement; the option completion module; the
    new test files. Keep `SKILL.md` within its size rule (link, do not inline).

### Wave 2 (after Wave 1)

- [ ] **Regenerate and verify** — run `just regen-expr-doc` and
  `just regen-lifecycle-schema`; confirm the diff adds only the three examples,
  their files, and the signature; the passive shipped-artifact corpus check and
  catalog example runner pass.

### Checkpoint 5

- [ ] `cargo nextest run -p darkmatter -E 'test(catalog) | test(corpus) | test(function_catalog_parity) | test(docs)'` passes.

## Phase 6 — CLI wiring, log, and final verification

Depends on Phase 5.

### Wave 1 (parallel)

- [ ] **CLI tests** — `cli/tests/l1/load_data.rs` (spawn `md` through
  `CliProcessFixture`, `--output json`)
  - `compose_headerless_rows_json`:
    `{{ load_data("rows.csv", "tuple([string, number])[]", {header: false}) }}`
    exits 0, empty stderr, rows in the output, read set lists `rows.csv`.
  - `compose_invalid_option_json_envelope`: `{delimiter: ",,"}` exits 1, empty
    stderr, one `dm.data.invalid_options` diagnostic whose span covers the third
    argument, and the envelope validates strictly against the existing envelope
    schema (spell the schema path in this file per the `rust-testing` skill's
    read-form rule).
- [ ] **Implementation log** — this feature's `implementation-log.md`
  - Complete Departures (R25 and anything else that departed from the spec,
    including any R24 coercion difference) and Decisions applied. Leave
    `spec.md` untouched.

### Wave 2 — final verification

- [ ] Re-read every changed `///`/`//!`/`//` comment for drift (`load.rs`,
  `runtime.rs`, `parse.rs`, `error.rs`, `locate.rs`, `contract.rs`,
  `options.rs`, `call_check.rs`, `data/mod.rs`, the DMLS modules).
- [ ] `just lint` in `darkmatter/` is clean (no `cargo fmt`).
- [ ] `just test` in `darkmatter/` is green end to end.
- [ ] `git diff --stat -- '*Cargo.toml'` is empty; no doc links a feature or
  fix; nothing was created under `_unscheduled`.
- [ ] Tick the Definition of Done and state "implementation complete, ready for
  review". Do not move the feature or run `just complete`.

## Review decisions (2026-11-08)

The author reviewed this plan's reported spec ambiguities and accepted its
resolutions: a computed options argument's `dm.data.invalid_options` locates at
the whole call (a departure, logged), literal options still locate at the
third argument; the TSV tab-only rule applies to the effective format; DMLS
completion offers `yml` and `ndjson`; the raw-source layer is consulted on
every call so observations are re-recorded.
