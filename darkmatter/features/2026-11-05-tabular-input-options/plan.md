---
kind: plan
total_phases: 5
created: 2026-11-08
phase: 1
agent: claude/sonnet
yolo: true
---

# Plan: Tabular Input Options for `load_data`

> **Outdated (2026-11-08): re-plan before executing.** This plan predates the
> author's 2026-11-08 decisions in the spec (`yml`/`ndjson` format aliases, the
> TSV delimiter rule, and sequencing after 2026-11-05-dynamic-schema-types).
> Its memo and loader design (R7 and the `load.rs` execution order) must be
> redone on top of the dynamic-schema-types changes to `runtime.rs` and
> `load.rs` once that feature lands. Do not execute this plan as written.

## Summary and Definition of Done

`load_data(file, as)` gains an optional third argument, a closed options
object `{format?, delimiter?, header?}`. The work is spread over five layers,
and the order below follows their dependencies:

1. **Contract surface** (catalog signature, option model, the new
   `dm.data.invalid_options` code, a headerless row-shape conflict).
2. **Library judgment and execution**: the passive check (shared by
   composition and DMLS), the execution flow in `load.rs`, headerless row
   views shared by binding and diagnostic lookup, and a three-layer memo
   (raw source → parsed source → bound result).
3. **DMLS**: nested option-key and enum-value completion from the declared
   object shape, hover, diagnostics.
4. **Catalog examples, generated pages, hand-written docs, and the Darkmatter
   skill.**
5. **CLI wiring tests and the final verification** (`just test`, `just lint`).

The feature is **done** when all of the following hold (each is also an
acceptance bullet of the spec):

- `load_data("rows.csv", "tuple([string, number])[]", {header: false})`
  returns every record (two-record CSV, TSV, and one-record file covered);
  an object-row contract with `header: false` fails with
  `dm.data.row_shape_conflict`, passively for literal values and at
  execution for computed values and on-disk contract imports.
- `{format: "csv", delimiter: ";"}` reads `rows.txt`; a wrong-but-supported
  extension is overridden; TSV keeps its tab default; `"|"` works; an
  extensionless URL loads with `{format: "json"}`.
- Every invalid option shape listed in the spec fails with the shared binder
  diagnostic or `dm.data.invalid_options`, located at the third argument
  passively, before any read.
- One source read twice with different delimiters gives `reads = 1`,
  `parses = 2`; equivalent default spellings share a parse and a bind;
  changing only `header` parses once and binds twice; call order never
  changes a result; every two-argument result, read set, diagnostic field,
  and work count is unchanged.
- Headerless diagnostics report `/0/1` and the field's raw position for the
  first record, and stay aligned for a later row, a quoted multiline field,
  multibyte text, and CRLF.
- DMLS completes option keys and enum values from the declared shape, hover
  shows the third parameter, callback typing and null-file typing are
  unchanged for both arities.
- Catalog examples pass the passive shipped-artifact corpus check; `md
  compose --output json` shows one headerless success and one option failure
  (exit 1, empty stderr, located diagnostic, envelope-conformant).
- `just test` and `just lint` pass in `darkmatter/`; docs and the skill
  describe the landed behavior and never link to the spec.

Out of scope (settled by the spec): other CSV dialect settings, non-UTF-8
encodings, content or `Content-Type` sniffing, a `columns` option, `bf`
changes, CLI flags, size limits, persistent caches, grammar changes, new
dependencies. **No spike, no timing benchmark, no L2/L3 test** is part of this
plan: reuse is proven by `DataRuntime::stats()` counts and the spec states no
performance question (ruling recorded as rule R19).

## Phase 1: Contract surface

Establishes every name and type the later phases build on. Nothing here changes
a two-argument result.

### Necessary Rules

Paths are relative to `darkmatter/` unless stated. "Data module" means
`lib/src/markdown/compose/expression/data/`.

- **R1 — One overload, optional third parameter.** The catalog entry
  `load_data` in `schemas/partials/functions.yaml` becomes
  `parameters(file: file | null, as: type-definition(container), options?: {format: enum(yaml, json, jsonl, csv, tsv, toml, json5), delimiter: string, header: boolean} | null)`.
  Evidence: `grammar.rs::parse_parameters` accepts `name?:` with a general
  union, an inline object is closed (`translate.rs::inline_object`, doc
  comment "closed unless it declares a `<string>` catch-all"), and "an
  optional property also admits `null`" (`translate.rs`, presence rules), so
  unknown keys, wrong field types, bad format names, `{}`, `null`, and
  `{header: null}` are all decided by the shared binder with its existing
  diagnostics, as the spec requires. A single overload (not two, as `pr_list`
  uses) keeps one bound path and renders as `load_data(file, as, [options])`.
  Callers: the catalog is read by `FunctionBinding::context("load_data", …)`
  in `lib/src/markdown/compose/expression/functions/data.rs`, which needs no
  change.
- **R2 — Description text.** The catalog description is extended with the
  third argument: keys, accepted values, defaults (`format` from the
  extension; `delimiter` comma for CSV and tab for TSV; `header` true), and
  one sentence that a delimiter or header is invalid for a non-tabular
  format. Hover reads this text (R17).
- **R3 — Option model lives in a new file** `<Data module>/options.rs`
  (declared `pub(crate) mod options;` and re-exported items listed in R4 in
  `data/mod.rs`). It contains:
  - `pub(crate) struct DataOptions { format: Option<DataFormat>, delimiter: Option<u8>, header: Option<bool> }`
    with `DataOptions::from_bound(value: Option<&Value>) -> Result<Self, DataError>`.
    Absent argument, `Value::Null`, and `{}` all give `DataOptions::default()`.
    For each key: absent or `Null` is `None`. `format` must be a
    `Value::String` equal (exact, case-sensitive, untrimmed) to one of the
    seven canonical names (`format_from_name(&str) -> Option<DataFormat>`;
    `yml` and `ndjson` are **not** names). `delimiter` must be a
    `Value::String` accepted by `delimiter_byte(&str) -> Result<u8, DataError>`.
    `header` must be a `Value::Bool`. Any other shape or key reaching this
    function (unreachable after binding) is
    `DataError::InvalidOptions`, never ignored, defaulted, or `.ok()`'d.
  - `delimiter_byte`: exactly one `char`, ASCII, not `"`, CR, or LF;
    otherwise `InvalidOptions { key: "delimiter", … }` whose reason reads
    "must be one ASCII character other than a double quote, CR, or LF
    (write a tab as \"\\t\")". Evidence: spec table row for `delimiter`, and
    `biscuit_file::read_delimited`'s own refusal set (`delimited.rs`,
    `InvalidDelimiter`).
  - `DataOptions::check_applicability(&self, format: DataFormat) -> Result<(), DataError>`:
    a non-`None` `delimiter` or `header` with a format other than `Csv`/`Tsv`
    is `InvalidOptions` for the first offending key in the order
    `delimiter`, `header`, reason "applies only to CSV and TSV data, but this
    call reads {format name}; remove it or select a tabular format".
    `header: true` counts as non-null.
  - `pub(crate) fn effective_delimiter(format: DataFormat, chosen: Option<u8>) -> Option<u8>`
    (`Some(chosen or b',' / b'\t')` for `Csv`/`Tsv`, `None` otherwise) and
    `pub(crate) fn effective_header(format: DataFormat, chosen: Option<bool>) -> Option<bool>`
    (`Some(chosen.unwrap_or(true))` for tabular formats, `None` otherwise).
    These feed the memo keys (R7), so omitted and explicit defaults share
    entries.
- **R4 — New error `DataError::InvalidOptions { key: String, reason: String }`**
  in `<Data module>/error.rs`. `code()` returns the new
  `pub const INVALID_OPTIONS_CODE: &str = "dm.data.invalid_options";` which is
  added to `DATA_DIAGNOSTIC_CODES` and re-exported from `data/mod.rs`.
  `written_contract()` returns `None` for it. Display:
  ``load_data(): option `{key}` is invalid: {reason}``. It is an error, not a
  governed warning (the doc comment on `DATA_DIAGNOSTIC_CODES` already says
  none is). `data_records` in `lib/src/markdown/compose/report_diagnostics.rs`
  needs no arm (its `_ => vec![record]` already projects code and message), and
  `nested.rs` / `ExpressionError::from` treat every non-file-reference
  `DataError` uniformly.
- **R5 — Headerless object conflict reuses `dm.data.row_shape_conflict`.** Add
  `RowShapeConflict::HeaderlessObjectRows` in `contract.rs` (unit variant,
  display: "`header: false` reads rows by position, but this contract
  describes rows as objects; use a `tuple([…])[]` or `string[][]` contract, or
  read an input that has a header"). A helper
  `pub fn require_positional_rows(class: RowClassification) -> Result<(), RowShapeConflict>`
  returns `Err(HeaderlessObjectRows)` for `Known(RowMode::Object)` and `Ok`
  for `Known(RowMode::List)` and `Provisional` (provisional stays open until
  execution, per the spec). It is exported from `data/mod.rs`. The error is
  still raised as `DataError::RowShapeConflict { written, conflict }`, so
  `code()` and every consumer are unchanged.
- **R6 — Where each error is located.** Passive findings are ranged by
  `DataProblem.argument`: `2` for `dm.data.invalid_options`, `1` (`as`) for the
  row-shape conflict. Execution diagnostics are projected at the expression's
  call-site span by `report_diagnostics.rs`; the evaluator carries no
  per-argument spans (`argument_spans` exist only on `CallCheck`), and adding
  them is out of scope. Therefore the execution location of
  `dm.data.invalid_options` and of the headerless conflict is the call
  expression, with the offending key named in the message. This is a
  deliberate, documented narrowing of the spec's "locate at the third
  argument"; Phase 5 records it in the implementation log
  (`implementation-log.md`, created by that phase) and the docs state
  the passive/execution difference plainly.
- **R7 — Memo layout (decided here, implemented in Phase 2).** The memo has
  three single-flight layers in `runtime.rs`:
  `raw: HashMap<DataSourceIdentity, Slot<R>>`,
  `sources: HashMap<SourceKey, Slot<S>>`,
  `bindings: HashMap<BindKey, Slot<B>>` where
  `SourceKey { identity: DataSourceIdentity, format: DataFormat, delimiter: Option<u8> }`
  and `BindKey = (SourceKey, String /*contract key*/, Option<bool> /*effective header*/)`.
  `DataMemo` becomes `DataMemo<R, S, B>`. All four maps (with `contracts`) are
  replaced together in `Generation::new`, so `invalidate_memo` clears every
  layer and still never touches the read set. `R` is
  `RawSource { content: RawContent, len: usize }` with
  `RawContent::Bytes(Arc<[u8]>)` (local file) and `RawContent::Text(Arc<str>)`
  (fetched, BOM already stripped by the existing `strip_bom` call), `len` the
  size reported in the `bytes` tracing field (local byte count; fetched body
  length before BOM stripping, as today). `DataFormat` already derives `Hash`
  (`biscuit-file/lib/src/format.rs`).
- **R8 — Counters.** `reads` increments inside the raw layer's loader only;
  `parses` increments via `note_parse()` after a successful parse (unchanged);
  `binds` increments in the bind layer (unchanged); `source_hits` counts
  parsed-source hits (unchanged definition, now keyed by `SourceKey`);
  `bind_hits` unchanged. There is **no** new counter and no `raw_hits`: raw
  reuse is read as `reads` staying flat, as the spec directs. Existing
  two-argument tests keep passing unmodified because a parse-layer hit never
  consults the raw layer.
- **R9 — UTF-8 decoding stays in the interpretation layer.** `RawContent::Bytes`
  holds undecoded bytes; `parse.rs::decode_local` takes `&[u8]` (callers:
  `parse_local_document`, which passes `&bytes`, and the new parse loader).
  A UTF-8 error therefore names the format of the call that triggered the
  parse and is cached under that call's `SourceKey`.
- **R10 — `parse_text` signature** becomes
  `parse_text(text: &str, format: DataFormat, delimiter: Option<u8>, identity: &DataSourceIdentity)`;
  `Csv`/`Tsv` call `parse_delimited(text, delimiter.unwrap_or(b',' or b'\t'))`;
  other formats ignore `delimiter`. `parse_local_document` passes `None`.
  `parse_delimited`'s `InvalidDelimiter` arm remains unreachable because
  `delimiter_byte` already refused the byte.
- **R11 — Headerless row view.** `TabularSource` (in `error.rs`) gains
  `header: bool` and two methods:
  `row_count(&self) -> usize` and `row(&self, index: usize) -> Option<&DelimitedRecord>`.
  With `header == true` they read `data.rows`; with `false` they read
  `data.headers` (the first nonblank record, which the reader stores there
  without interpreting) followed by `data.rows`. `load.rs::table_rows` (binding)
  and `locate.rs::table_offset` (diagnostic lookup) must both iterate through
  these methods and never touch `data.rows` directly; this is the "same
  logical row view" the spec requires. No text is rewritten and no synthetic
  header is prepended. Object mode is only reachable with `header == true`
  (R5), so header-name validation (`check_header`) is untouched.
- **R12 — Execution order in `load_data_fn`** (replaces the current order in
  `load.rs`): (1) null `file` → `Ok(Value::Null)` immediately (existing first
  line; the options are already bound by the shared binder, so literal mistakes
  were already rejected); (2) `DataOptions::from_bound(call.args().get(2))`;
  (3) read `as`, `prepare_contract`; (4) resolve the identity and `record` it
  in the read set; (5) effective format = `options.format` else
  `format_of(&identity)` (the `dm.data.unsupported_format` path); (6)
  `check_applicability`; (7) for tabular formats, `contract.row_mode()` and,
  when the effective header is `false`, `require_positional_rows`; (8) remote
  fetch / host policy (`ctx.fetch_remote_text`) — **every call, before any
  memo lookup**, exactly as today; (9) `load_source(SourceKey, …)` whose loader
  calls `load_raw(identity, …)` then decodes and parses; (10) `bind(…)` keyed
  per R7. Evidence for the ordering: the spec's "validate bound option values
  first, then prepare the contract, resolve and record the source, choose the
  effective format, check option applicability and row shape, enforce remote
  policy, read and parse, and bind".
- **R13 — Remote handling is otherwise unchanged.** Option differences never
  change a URL identity (query and fragment stay); an explicit `format`
  grants no network access; URL discovery (`remote.rs::REMOTE_READ_FUNCTIONS`
  looks only at argument 0) needs no change; the reference check's
  `RemoteNotEnabled` deferral is returned before the memo (step 8) so it is
  still never cached.
- **R14 — Passive judgment design (implemented in Phase 2).**
  `call_check.rs::ArgumentType` gains one variant
  `PartialObject { shape: ScopeShape, literals: serde_json::Map<String, Value>, computed: Vec<String> }`,
  produced by `ArgumentType::of` only for an object literal (through
  `Paren`) with at least one literal-valued and at least one non-literal
  member. Classification (`classify_argument_with_definitions`) treats it as
  `Typed(shape)`; `Display` delegates to `Typed`. All other argument types
  keep their meaning. Evidence: `ArgumentType` is matched only inside
  `call_check.rs` (17 sites; no other crate matches it), so the addition is
  contained, and without it a literal `delimiter` beside a computed member
  would be invisible, violating "check literal option values even when
  another argument is computed".
- **R15 — Passive verdict order** (one `DataProblem` per call, matching
  execution order): (a) literal option values (`delimiter` string via
  `delimiter_byte`; `format` string via `format_from_name` only as a source of
  the format, since the binder owns bad names); (b) the existing contract
  judgment (unchanged); (c) applicability of a literal non-null `delimiter` /
  `header` against the *provable* effective format; (d) the tabular row rule,
  now also `header: false` ⇒ `require_positional_rows`. Only `Value::String`
  delimiters and `Value::Bool(false)`/`Bool(true)` headers are judged
  passively; any other literal shape is left to the binder and execution.
  The effective format is provable when: a literal non-null `format` exists
  (even with a computed `file`); otherwise the options are *closed*
  (omitted, literal `null`, a fully literal object, or a `PartialObject`
  whose `computed` list lacks `format`) **and** `file` is a literal string
  whose extension is supported (existing `is_tabular_path` extension logic,
  generalized into `provable_format`). A variable, call, or `Unknown` options
  argument, or a computed `format`, makes the format unprovable and defers
  every format-dependent check (spec: a computed options object might
  override a literal extension). Dead-branch behavior and literal `null` file
  behavior are unchanged: option judgments never look at `file` except for
  its extension.
- **R16 — Typing.** `load_data_type` in `call_check.rs` accepts `args.len()`
  of 2 or 3 (`[file, contract]` or `[file, contract, _options]`) and ignores
  the third; the inferred type stays the literal `as` type with the same
  null-file handling, including for computed options.
- **R17 — Hover needs no new code.** `format_function_block` renders
  `typed_signature()`, which already prints the third parameter
  (`[options: {…} | null]`) and the description (R2). Hover gets a test only.
- **R18 — DMLS completion is a new module** `dmls/src/providers/option_argument.rs`
  (registered in `providers/mod.rs`, called from `providers/dsl.rs::completion`
  and `providers/frontmatter.rs` next to the `type_definition_argument`
  calls, for `Site::Body` and `Site::Frontmatter`). Behavior: when the cursor
  is inside an object literal that is argument *n* of a call, and the catalog
  overload's parameter *n* (after dropping `null`) is an inline object, it
  claims the cursor. At a key position it offers that object's declared
  property names not already present in the literal; at the value position
  of an `enum` property it offers the quoted members; at the value position
  of a `boolean` property it offers `true` and `false`; any other value
  position returns `None` so ordinary expression completion runs. Property
  names and members are read from `ExpressionFunctionDescriptor.function_type.parameters[n].def`
  (`PropertyDef` → `TypeExpr::InlineObject(shape).properties`), never from a
  list in the editor. Parameters typed plain `object` (`pr_list`,
  `cicd_list`) have no declared shape and are unaffected. The shared
  expression-region and string-escape helpers in `type_definition_argument.rs`
  (`Analyzed`, `expression_region_at` use, projection/encode) are made
  `pub(crate)` and reused instead of duplicated. The new module recovers
  unfinished input with the expression `Lexer` (the technique
  `string_argument_call` already uses): tokenize the expression text up to
  the cursor, keep a stack of open `(`/`{` frames recording call name,
  argument index, and for `{` the last completed key and whether a `:` was
  seen since the last `,`.
- **R19 — No spike, no performance work.** The spec says work counts answer
  the reuse question and requires no benchmark; nothing here needs a runtime
  measurement. No L2/L3 test, no terminal or browser window, so focus rules
  are moot.
- **R20 — Fixtures and example outputs** (Phase 4): add to
  `catalog/example_data/` and `EXAMPLE_DATA_FILES` (`catalog/mod.rs`)
  `fixture-headerless.csv` (`web,8080\ndb,5432\n`),
  `fixture-semicolon.txt` (`name;port\nweb;8080\ndb;5432\n`), and
  `fixture-export` (no extension; the same JSON text as `fixture.json`). The
  catalog gains exactly three examples:
  `load_data("fixture-headerless.csv", "tuple([string, number])[]", {header: false}) => [["web", 8080], ["db", 5432]]`,
  `load_data("fixture-semicolon.txt", "{name: string, port: number}[]", {format: "csv", delimiter: ";"}) => [{name: "web", port: 8080}, {name: "db", port: 5432}]`,
  `load_data("fixture-export", "{name: string, tags: string[]}", {format: "json"}) => {name: "Ada", tags: ["math", "engines"]}`.
  The generated page lists a data file when an example names it as a quoted
  string (`example_data_section`), so no generator code changes; files whose
  name has no recognized extension are fenced as `text`.
- **R21 — Generated artifacts.** After the catalog edit, run
  `just regen-expr-doc` (rewrites `docs/functions/load-data.md` and the table
  in `docs/topics/darkmatter-expressions.md`) and `just regen-lifecycle-schema`
  (rewrites the `load_data-short`/`load_data-long` types in
  `schemas/partials/lifecycle.yaml`; the generator, not a hand edit, decides
  whether the short form gains the optional element). Both parity checks run in
  `just test`. Hand-edit nothing those recipes own.
- **R22 — Test placement.** New L1 files are registered once, in Phase 1, as
  stubs so parallel test tasks never edit the same `mod` list:
  `lib/tests/l1/load_data_options.rs` (execution, matrix, memo reuse),
  `lib/tests/l1/load_data_options_passive.rs` (passive verdicts, typing),
  `lib/tests/l1/load_data_headerless.rs` (headerless table, positions),
  `lib/tests/l1/load_data_options_remote.rs` (wiremock). `dmls` tests extend
  `dmls/tests/l1/type_definition_arguments.rs` neighbors with the new file
  `dmls/tests/l1/option_arguments.rs` (registered in `dmls/tests/l1/main.rs`).
  CLI tests extend `cli/tests/l1/load_data.rs`. Fixtures use `tempfile` and
  portable path helpers (`entry`, `biscuit_file::to_portable_string`) so they
  pass on macOS, Linux, native Windows, and WSL2; CRLF content is written as
  explicit bytes. Existing helpers are re-declared locally (they are private
  per file today).
- **R23 — Input robustness (what the matrix covers).** The options object is
  validated by two shared validators — the **binder** (field types, enum
  names, closed keys) and `DataOptions` (R3) — and loaded by no new document
  loader; the CSV/TSV reader is unchanged (`biscuit-file::read_delimited`
  already takes any permitted delimiter). So the matrix is a field-shape
  matrix (Phase 2 task "Options matrix"), plus the headerless data table.
  Default outcome for every invalid shape: a located error, nothing read,
  nothing published. Exceptions: absent and explicit `null` both mean the
  default; `delimiter: 5` converts to `"5"` and is valid (shared
  coercion, spec: "normal conversions"); `header: "false"` converts to
  `false`. Smell grep before declaring done (Phase 5): no
  `#[serde(default)]`, `unwrap_or_default()`, `filter_map(.. as_str())`, or
  `.ok()` on the option parse in `options.rs`.

### Wave 1 (parallel; disjoint files)

- [ ] **Option model** — create `data/options.rs` per R3 with unit tests
  - Implement `DataOptions`, `format_from_name`, `delimiter_byte`,
    `check_applicability`, `effective_delimiter`, `effective_header`.
  - Unit tests in the file: control (`None`, `Null`, `{}` → default), every
    delimiter shape (`","`, `";"`, `"|"`, `"\t"` valid; `""`, `",,"`, `"é"`,
    `"\""`, `"\r"`, `"\n"` invalid, each message naming `delimiter`), format
    names (7 valid, `"CSV"`, `"yml"`, `"ndjson"`, `" csv"`, `""` invalid),
    applicability (`delimiter`/`header: true` on each of the five tree formats
    and both tabular formats), wrong shapes reaching `from_bound`
    (`format: 5`, `header: "x"`, unknown key `"quote"`) are `InvalidOptions`.
  - Prerequisite: none. Risk: none.
- [ ] **Error and conflict types** — R4 and R5
  - Add `InvalidOptions`, `INVALID_OPTIONS_CODE`, `DATA_DIAGNOSTIC_CODES`
    entry, `RowShapeConflict::HeaderlessObjectRows`, `require_positional_rows`,
    exports in `data/mod.rs`; unit tests in `error.rs` (`code()`, Display,
    `written_contract()`) and `contract.rs` (conflict display contains
    `header: false`; `Provisional` stays open).
  - Update the module list comment in `data/mod.rs`.
- [ ] **Catalog signature** — R1 and R2
  - Edit the `load_data` entry in `schemas/partials/functions.yaml` (signature
    and description only; **no new examples yet**, they need Phase 2).
  - Run `just regen-expr-doc` and `just regen-lifecycle-schema` (R21), check the
    diff touches only `load_data` text, the function page, and the lifecycle
    types. Update `load.rs::tests::failed_at`'s signature string to the new
    signature text.
  - Risk: the catalog loader must accept the inline-object parameter; the
    registry/catalog tests (`function_catalog_parity`,
    `function_binding_registry`, catalog `tests.rs`) run in the checkpoint and
    prove it.
- [ ] **Test stubs** — R22
  - Create the four lib stub files and `dmls/tests/l1/option_arguments.rs`,
    each with a module doc comment and no tests, and register them in
    `lib/tests/l1/main.rs` and `dmls/tests/l1/main.rs`.

### Checkpoint 1

- [ ] `cargo nextest run -p darkmatter -E 'test(options) | test(catalog) | test(function_binding_registry) | test(function_catalog_parity) | test(lifecycle)'` passes; a two-argument `load_data` call still evaluates (existing `load_data_*` suites pass unmodified).

## Phase 2: Library judgment and execution

Depends on Phase 1. Wave 1 tasks touch different files and run in parallel;
Wave 2 needs Wave 1; Wave 3 tests run in parallel.

### Wave 1 (parallel)

- [ ] **Passive check** — `call_check.rs` per R14, R15, R16
  - Add `ArgumentType::PartialObject` and its construction in
    `ArgumentType::of`; delegate classification and `Display` to `Typed`.
  - Replace `load_data_problem`'s two-element destructure with a 2- or
    3-element one; build an `OptionsView` (private enum: `Closed(literals)`,
    `Partial { literals, computed }`, `Unknown`) from the third
    `ArgumentType` (`Literal(Null)`/missing → closed and empty;
    `Literal(Object)` → closed; `PartialObject` → partial; everything else →
    unknown). Implement `provable_format` and the verdict order of R15,
    reusing `DataOptions` helpers (`delimiter_byte`, `format_from_name`,
    `check_applicability` semantics via a shared message builder so passive
    and execution text are byte-identical). The row rule calls
    `classify_rows` then `require_positional_rows` when a literal
    `header: false` and a tabular format are both provable.
  - `data_problem(error)` keeps mapping `DataError::code()`; add the
    argument mapping (`InvalidOptions` → 2, others → 1) there.
  - `load_data_type` accepts arity 2 or 3 (R16).
  - Update the doc comments on `load_data_problem`, `load_data_type`, and the
    table in `expression_type`'s doc (`load_data(file, '<contract>')` row) so
    they describe both arities (drift rule).
  - Risk: `PartialObject` must not change any other function's verdict; the
    existing `design_time_call_check`, `flow_sensitive_narrowing`, and
    `variable_type_projection` suites guard that.
- [ ] **Memo layers** — `runtime.rs` per R7 and R8
  - Introduce `RawSource`, `RawContent`, `SourceKey`, `BindKey`; make
    `DataMemo<R, S, B>`; add `load_raw(identity, loader)` (counts `reads`),
    change `load_source` to take `&SourceKey` (no longer counts `reads`; counts
    `source_hits`), change `bind` to take `(source, contract_key, header)`.
    Keep `single_flight`, deferral removal, generation checks, and
    `invalidate` semantics exactly; `Generation::new` creates all four maps.
  - Update `DataRuntime::{load_source, load_raw, bind}` wrappers, the module
    doc (three layers, key definitions), and the `DataRuntimeStats` field docs
    (`reads` = raw source loads; `source_hits` = parsed-source hits).
  - Rewrite the in-file unit tests for the new generics and keys; keep every
    existing assertion's intent (single-flight, invalidation generation,
    stale-source bind not memoized, failure memoized but deferral not, read
    set survives invalidation). Add: raw reuse across two `SourceKey`s
    (`reads == 1`, two parses), a raw failure shared by two keys, and a late
    old-generation raw load not populating the new maps.
- [ ] **Parse and row view** — `parse.rs`, `error.rs`, `locate.rs` per R9–R11
  - `decode_local(&[u8], …)`, `parse_text(…, delimiter, …)`, and
    `parse_local_document` (still `Vec<u8>` public signature) adapt; update
    their doc comments and the CSV/TSV sentences in the module docs.
  - `TabularSource { mode, header, data }` with `row_count` / `row`;
    `locate.rs::table_offset` and its unit tests use them (root path still
    resolves to the first record's offset; `/i/…` indexes the logical row
    view).
  - Add unit tests: headerless view of `"a,b\n1,2\n"` has two rows; locator
    resolves `/0/1` to line 1 column 3 for a headerless table and keeps the
    existing header-skipping behavior for `header: true`.

### Wave 2

- [ ] **Execution flow** — `load.rs` per R12 and R13
  - Rewrite `load_data_fn` in the order of R12 using `DataOptions`,
    `effective_*`, `SourceKey`, and the new memo calls; move the file-reading
    closure into the raw loader (`std::fs::read` → `RawContent::Bytes`;
    fetched body → `RawContent::Text(Arc::from(strip_bom(&body)))` with
    `len = body.len()`); the source loader decodes with `decode_local` /
    clones the text `Arc`, parses with the effective delimiter, calls
    `note_parse()`, and builds `LoadedSource`.
  - `bind_source` and `table_rows` take the effective header; `table_rows`
    builds `Value::Array` through `TabularSource::row_count/row`, a table
    with no records still yields `[]`, and `TabularSource` stored in
    `ContractMismatch` carries `header`.
  - Record `format` in the tracing span from the *effective* format; add the
    effective `header` and `delimiter` span fields (`Empty` placeholders in
    the `#[tracing::instrument]` field list).
  - Update the module and function docs (order, third argument, null file
    before options) and the `refine_binding_error` unit-test fixture signature text
    (change it to the new three-parameter signature).
  - Prerequisites: all Wave 1 tasks. Risk: keeping the two-argument counts
    unchanged — the existing `load_data_runtime`, `load_data_execution`,
    `load_data_passive`, and `compose_read_set` suites are the guard and run
    in the checkpoint before new tests are added.

### Checkpoint 2

- [ ] `cargo nextest run -p darkmatter -E 'test(load_data) | test(compose_read_set) | test(design_time_call_check) | test(flow_sensitive_narrowing) | test(variable_type_projection)'` passes with the new code and **no existing test modified except unit tests of `runtime.rs`**.

### Wave 3 (parallel test authoring; each file independent)

- [ ] **Passive tests** — `lib/tests/l1/load_data_options_passive.rs`
  - Cover, through `check_expression_calls` / `expression_call_diagnostics` /
    `expression_type` with `TypeScope::unguarded` (no filesystem):
    control (`{header: false}` with positional contract accepted); unknown
    key, wrong field type, bad format name (binder diagnostics, argument
    index 2); each invalid delimiter (`""`, `",,"`, `"é"`, `"\""`, `"\r"`,
    `"\n"`) → `dm.data.invalid_options` at argument 2 with the key named;
    `delimiter`/`header: true` with `{format: "json"}` and with a literal
    `x.json` file → invalid_options; `{header: null}` on json accepted; literal
    options with a **computed** `file` still rejected; computed options
    (variable, call, partial object with computed `format`) never produce a
    format-dependent verdict even beside a literal `x.json` and a literal
    `delimiter` (no false rejection); literal-`null` file with a literal
    mistake still rejected; `header: false` + object contract → row-shape
    conflict at argument 1 for a literal `.csv`/`.tsv` file, for a literal
    `format: "csv"` with a computed file, and not for a provisional imported
    contract; `{format: "json"}` on a `.csv` literal makes the row rule
    inapplicable; result type equals the two-argument type for both arities,
    for a null file, for computed options; callback member typing over a
    three-argument `map(load_data(…, {header: false}), r => r.…)` is
    unchanged; a mixed object literal (`{delimiter: ";", format: f}`) still
    judges its literal delimiter.
  - Assert passive and execution messages are identical by comparing against
    `DataError::InvalidOptions` Display built from the same inputs.
- [ ] **Options matrix and execution** — `lib/tests/l1/load_data_options.rs`
  (R23; public result is the composed text, typed `DataError` inside
  `ComposeFailure`, `DataRuntime::stats()`, and read set)
  - *Control row:* unedited fixture (`rows.csv` with header, `{}`) → rows.
  - *Field shapes through the shared validators* (each: absent, explicit
    null, wrong type, empty where the type allows) for `format`, `delimiter`,
    `header`, plus whole-object shapes (`null`, `{}`, `5`, `"csv"`, `[]`):
    default outcome = error and `reads == 0`; exceptions asserted
    individually: absent/null → defaults; `delimiter: 5` valid; `header:
    "false"` converts; `delimiter: ""` → `dm.data.invalid_options`; `format:
    ""`/`"CSV"` → binder error; `delimiter: [","]`, `header: [1]`, `format: 5`
    → binder error.
  - *Wiring cells* (one control + one invalid per validator, composed through
    `Markdown::compose_with_warnings`): binder error and
    `dm.data.invalid_options` each show their code and `reads == 0`.
  - *Applicability:* non-null `delimiter` and `header` on a `.json` file and
    with `format: "json"` on a `.csv` file fail before reading; resolution
    failure (missing file) keeps precedence over applicability for an
    explicit `format`.
  - *Behavior:* semicolon via `format: "csv"` on `.txt`; supported-but-wrong
    extension overridden (`.json` read as csv); TSV default tab; pipe; CSV
    with `"\t"` delimiter reports format CSV in a malformed diagnostic; `.tsv`
    with `format: "csv"` parses with comma.
  - *Null file:* valid options with `null` file → `null`, `reads == 0`,
    `contract_preparations == 0`, read set empty; invalid *computed* options
    with `null` file → `null` (no semantic check), literal mistakes still fail
    at composition via the passive check.
  - *Computed options* validated at execution: computed bad delimiter, computed
    `header: false` with object contract (→ `dm.data.row_shape_conflict` with
    the `header: false` message), a contract imported from a schema file
    resolving to object rows with `header: false` (execution-time conflict,
    `reads == 0`), computed `format` overriding a literal extension.
  - *Memo reuse* (`stats()`): two delimiters over one source → `reads 1`,
    `parses 2`; `{}`, `{format: "csv"}`, `{delimiter: ",", header: true}` →
    one parse, one bind; header-only change → `parses 1`, `binds 2`; different
    contracts → one parse, two binds; distinct explicit formats of one source
    never share parsed results or format-specific failures (invalid UTF-8
    named per format); every permutation of a 3-call set yields the same
    results; an object-mode header-name failure does not poison positional
    mode over the same parse; a memoized failure reused at a second call site
    projects with that site's own span; existing invalidation re-reads all
    layers (`reads` rises, read set unchanged) in every `CacheAccessMode`;
    concurrent identical calls (threads) read once.
- [ ] **Headerless table and positions** — `lib/tests/l1/load_data_headerless.rs`
  - Spec table, one test per row: empty file and blank-lines-only → `[]`
    judged against `as` (a `tuple([string])[]` accepts, a non-empty-required
    contract reports its own mismatch); one record (including empty and
    repeated cell values) → one row; leading/intervening blank lines skipped
    with original positions; ragged later record → `dm.data.malformed` at
    that record (first record sets width); quoted delimiter/newline/doubled
    quote parse strictly; invalid quoting → `dm.data.malformed`, no partial
    result; two-record CSV, TSV, and one-record file return all rows;
    `header: true` skips the first record for positional rows.
  - Positions: first-record failure reports pointer `/0/1` with the field's
    raw line and character column; also a later row, a quoted multiline
    field, multibyte text, and CRLF input, each asserting line and column so a
    one-row shift fails; missing cell keeps the nearest-parent rule.
  - Keep-immutability: the same file loaded `header: false` then
    `header: true` (and in reverse order) in one runtime gives correct,
    independent results from one parse.
- [ ] **Remote** — `lib/tests/l1/load_data_options_remote.rs` (wiremock, follow
  `load_data_runtime.rs`'s `RemoteReadConfig` usage; no live endpoint)
  - Extensionless `GET /export?id=7` returning JSON loads with
    `{format: "json"}` from an allowed host and fails
    `dm.data.unsupported_format` without it; the URL identity and read-set
    entry keep the query.
  - Denied host → file-reference denial (not cached, no body returned) even
    with a warmed memo from an allowed context; allowed host with a warmed
    memo returns without a second request; the reference check without
    remote permission defers (`RemoteNotEnabled` not cached) and a later
    composition with permission reads; an explicit `format` does not bypass
    redirect policy (redirect to a denied host fails).

### Checkpoint 3

- [ ] `just test` in `darkmatter/` is green for `lib` (new and existing); fix every failure at its cause (do not weaken an existing assertion).

## Phase 3: DMLS

Depends on Phase 2 (diagnostics) and Phase 1 (catalog). Wave 1 tasks are
independent.

### Wave 1 (parallel)

- [ ] **Option completion provider** — R18
  - Create `dmls/src/providers/option_argument.rs`; make the shared helpers in
    `type_definition_argument.rs` `pub(crate)` rather than copying; wire into
    `providers/dsl.rs::completion` (after the type-definition check, before
    `directives::name_partial`) and `providers/frontmatter.rs` (after the
    type-definition call), for both sites; a claimed cursor with no match
    returns an empty list only for key/enum/boolean positions, otherwise
    `None`.
  - Items: `CompletionItemKind::PROPERTY` for keys (insert `key: `), `ENUM_MEMBER`
    for enum values (quoted `"csv"`), `VALUE` for `true`/`false`; text edits
    replace only the typed partial and are encoded with the site's
    projection (`Analyzed::encode`) like the type-definition provider.
  - Document the module in `dmls/docs/autocomplete.md` (what completes, from
    the declared object shape).
  - Risk: unfinished input; unit-test through the lexer-recovery path with
    `{{ load_data("a.csv", "string[][]", { ` and
    `{ format: ` cursors, quoted/unquoted partials, a key already present
    (not offered again), a second argument position (no claim), a
    callback inside the call.
- [ ] **DMLS tests** — `dmls/tests/l1/option_arguments.rs`
  - Completion at body and frontmatter sites: keys `format`, `delimiter`,
    `header` at `{` and after `,`; `format:` offers the seven names;
    `header:` offers `true`/`false`; `delimiter:` offers nothing and falls
    through; `pr_list({` still shows no declared-key completion; completion
    list is derived from the catalog (assert equality with the descriptor's
    inline-object property names, not a literal copy).
  - Hover on `load_data` includes `options`, `format`, `delimiter`, `header`,
    and the defaults sentence.
  - Diagnostics through the existing editor path
    (`expression_call_diagnostics` style used by
    `dmls/tests/l1/expression_call_diagnostics.rs`): invalid delimiter shows
    `dm.data.invalid_options` ranged on the third argument, unknown key shows
    the binder code on the third argument, `header: false` + object contract
    shows `dm.data.row_shape_conflict` on `as`, computed options with a
    literal `.json` file and a literal delimiter produce no diagnostic, and
    `call_check_agreement.rs`-style parity holds: the editor diagnostics are
    exactly the composition rejections (extend that suite with one
    three-argument case).
  - Both arities keep inferred callback member completion and null-file
    typing: extend the `expressions.rs` tests near lines 2528–2535 with
    `map(load_data("hosts.yaml", "{ name: string }[]", {}), h => h.` and the
    `null`-file case.
- [ ] **Diagnostics table and docs** — `dmls/src/diagnostics/codes.rs`
  - Add `dm.data.invalid_options` (ERROR, "a literal `load_data` options
    object the passive call check proves wrong, ranged at the third
    argument") to the table in the module doc; update
    `dmls/docs/hover.md` and `dmls/docs/autocomplete.md` for the third
    argument. Keep prose written for a developer new to the repo.

### Checkpoint 4

- [ ] `just test` in `darkmatter/` (covers `darkmatter-lsp`) is green.

## Phase 4: Catalog examples, generated pages, docs, skill

Depends on Phases 2 and 3. Wave 1 tasks are independent; Wave 2 regenerates.

### Wave 1 (parallel)

- [ ] **Catalog examples and fixtures** — R20
  - Add the three fixtures, `EXAMPLE_DATA_FILES` entries, and three
    `example(...)` lines to the `load_data` entry. Extend the catalog test
    `load_data_function_page_shows_every_format_family` neighbor with a test
    that the page shows each new example and its input file, mentions the
    third parameter, and links nothing to a feature or fix.
- [ ] **Topic docs** — write for a developer with no repo experience: lead
  with what the reader can do, a compact example per rule, a Mermaid diagram
  for the flow
  - `docs/topics/loading-data.md`: new section "Options" (keys table,
    defaults, headerless rows with a positional contract, custom delimiter,
    extensionless URL with `format`, applicability errors, invalid-option
    examples), update "How a call runs" (new order, R12) and its diagram,
    "Formats" (explicit format replaces extension detection, CSV with a tab
    delimiter stays `csv`), "CSV and TSV" (header rule becomes conditional),
    "Reading a file once per command" (read versus parse versus bind: raw
    source, parsed source keyed by format and delimiter, bound value keyed by
    header), "When loading fails" (`dm.data.invalid_options` row; where it is
    located: third argument in the editor and passive check, the call
    expression at execution, key named in the message), "Remote URLs"
    (`format` grants no access).
  - `docs/topics/caching.md` "The `load_data` memo": three layers, the
    effective-key table, equivalent default spellings, `reads`/`parses`/
    `binds` semantics, invalidation clears every layer, read set unchanged.
  - `docs/cli/data-contracts.md`: the error-code tables (lines near 337 and
    544), a headerless example, the `md compose --output json` option-failure
    example described field by field (code, severity, span, no `data_location`
    because no data file was read), and the new code.
  - `docs/topics/darkmatter-expressions.md` hand-written parts (the
    `load_data(path, as)` rows near 1337–1343): third argument; leave the
    generated table to `just regen-expr-doc`.
  - `README.md` stays an overview: change only the one-line description of
    Loading Data (README line 93) to mention the optional options argument.
  - No doc names or links to a feature/fix or this plan.
- [ ] **Darkmatter skill** — `.claude/skills/darkmatter/`
  - `expressions.md` (`load_data` section): signature with `options`,
    `dm.data.invalid_options` in the codes list, headerless/`format`/`delimiter`
    rules, passive verdict order (R15), `PartialObject`.
  - `requests.md` / `compose.md` memo rules (near the `load_data_runtime`
    references): the three layers, keys, counters.
  - `dmls.md` (near line 534): option completion module and that it reads
    the declared object shape. `testing.md` (`load_data` tests): the new
    test files. Keep `SKILL.md` under its size rule and link rather than
    inline.

### Wave 2

- [ ] **Regenerate and verify** — run `just regen-expr-doc` and
  `just regen-lifecycle-schema` again (examples changed), confirm the diff
  only adds the three examples, their files, and the signature; the passive
  shipped-artifact corpus check and the catalog example runner pass.

### Checkpoint 5

- [ ] `cargo nextest run -p darkmatter -E 'test(catalog) | test(corpus) | test(function_catalog_parity) | test(docs)'` passes.

## Phase 5: CLI wiring, log, and final verification

Depends on Phase 4.

### Wave 1 (parallel)

- [ ] **CLI tests** — `cli/tests/l1/load_data.rs` (spawn `md` once per case
  via `CliProcessFixture`, `--output json`)
  - One headerless success: `{{ load_data("rows.csv", "tuple([string, number])[]", {header: false}) }}` composes, exit 0, empty stderr, the rows in the output, read set lists `rows.csv`.
  - One option failure: `{delimiter: ",,"}` exits 1 with empty stderr, one
    diagnostic with code `dm.data.invalid_options`, a located span, and the
    envelope validates against the existing envelope schema through the
    existing `validate_strictly` helper (spell the schema file in this test
    file because CI maps `include_str!` to the test that names it).
  - No terminal or browser windows are involved.
- [ ] **Implementation log** — create `implementation-log.md` in this feature
  directory recording: the execution-location narrowing of R6 (spec says
  third argument; execution reports at the call expression because the
  evaluator carries no per-argument spans; passive and DMLS locate at the
  argument), the `PartialObject` addition (R14), and anything else that
  departed from the spec text. Leave `spec.md` untouched.

### Wave 2 — final verification

- [ ] Run the smell grep over the changed files: `rg -n "serde\(default\)|unwrap_or_default\(\)|filter_map\(.*as_str|\.ok\(\)" darkmatter/lib/src/markdown/compose/expression/data/options.rs` returns nothing for the option parse; fix any hit.
- [ ] Re-read every changed `///` / `//!` / `//` comment for drift (CLAUDE.md "Authoring discipline"): `load.rs`, `runtime.rs`, `parse.rs`, `error.rs`, `locate.rs`, `contract.rs`, `call_check.rs`, `data/mod.rs`, DMLS modules.
- [ ] `just lint` in `darkmatter/` is clean (do **not** run `cargo fmt`).
- [ ] `just test` in `darkmatter/` is green end to end.
- [ ] Confirm the diff contains no `Co-authored`, no link from a doc to a feature/fix, no new dependency (`git diff --stat -- '*Cargo.toml'` empty), and no file under `_unscheduled`.
- [ ] Update the spec frontmatter? **No** — an agent's terminal state is "implementation complete, ready for review"; do not move the feature or run `just complete`.
