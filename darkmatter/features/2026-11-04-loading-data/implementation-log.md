---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-04-loading-data/spec.md
plan: darkmatter/features/2026-11-04-loading-data/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
decisions_for_review:
    - |-
        Phase 1 changed no product code, docs, or schemas. The plan asked it to
        "fix integration drift within the interfaces this plan extends". The
        audit found no behavioral drift: the envelope, the Diagnostic record,
        ReportSink, the 17 output schemas, and docs/cli/data-contracts.md agree
        with each other and with the plan's rules. The only drift is in the
        plan's evidence citations: rule 17 cites commands/compose.rs lines 394
        and 497 for the two requests built from one options value. After the
        dotted-paths work those are lines 355 (check_request) and 458
        (compose_request). The plan text was left as written, because the
        named symbols are still right and editing a rule's evidence mid-run
        risks confusing later phases; the corrected lines are recorded here
        and in message_to_agent. If the reviewer prefers the plan itself
        corrected, only that one citation needs editing.
    - |-
        Phase 2 split CSV/TSV records on a lone carriage return (CR not followed
        by LF), because that is what the `csv` crate does by default. The spec says
        "LF and CRLF terminate records" and that a lone CR is not a line break.
        Line numbers in reader errors count LF only, so they agree with the line
        convention the data_location positions will use. Alternatives: reject a
        lone CR as malformed (a new error case the spec does not list), or treat it
        as cell text (would need a custom terminator and breaks CRLF handling in
        the csv crate). Redo cost if reversed: a small change in
        biscuit-file/lib/src/delimited.rs plus its tests.
    - |-
        The JSONL reader treats only JSON whitespace (space, tab, CR) as a blank
        line. A line holding other Unicode whitespace is an invalid record, not a
        skipped line, matching what serde_json itself accepts. The spec says
        "whitespace-only line is skipped" without defining whitespace. Redo cost:
        one predicate in biscuit-file/lib/src/json_lines.rs.
    - |-
        CSV, TSV, and JSONL are classified as "structured" formats in
        biscuit-file's DataFormat/FileType (rows of records are a tree), and all
        three require UTF-8. The plan asked only for the classification to be
        updated. Redo cost: two match arms and their tests.
    - |-
        The delimited reader returns `headers: None` for zero-byte or blank-only
        text and `Some(header)` with no rows for a header-only file; darkmatter
        will map both to `[]` in a later phase. It also keeps a defensive
        `Desynchronized` error variant for the case where the csv-core offset pass
        and the csv decoding pass disagree (should never occur).
    - |-
        The shared failure flattening (`locate_failures`) returns typed records
        (`ExpectedType::{Type, Union, Absent}`, `ActualKind` including `Missing`)
        instead of rendered strings, so darkmatter does the wording. Recursive
        named references (`TypeExpr::Ref`) count as provisional containers, like
        unresolved imports, because the text-only check cannot see the definition
        table; the alternative (reject them) would refuse valid recursive container
        types. `container` is also accepted inside `schema(property(name,
        type-definition(container)))`, alongside `forms` and `declared-required`.
    - |-
        Two intended behavior changes in simplified-schema, beyond the rules'
        letter: (1) frontmatter annotation now descends into a union's one
        reachable arm (the new reachability rule) instead of reporting the union
        whole when it has several non-null arms; (2) `validate_soft` reports on the
        partial (diagnostic) copy whenever binding fails, even when write-back
        commits nothing, which removes false "not of type number" reports. No
        existing test expectation changed. Write-back still commits only what the
        complete schema accepts.
    - |-
        biscuit-file's `just test` and `just lint` recipes now enable the new
        `delimited` feature (`--features fetch,delimited`, plus a clippy run with
        `--features delimited`); without that, the new `l1-delimited` test binary
        would never run locally. The plan asked for CI test-feature metadata only.
    - |-
        Phase 3's editor region API (`expression_region_at`) adds one rule that
        the composition scanner does not have: a `{{` outside a string at brace
        depth 0 starts a new region. Without it, the cursor in
        `{{ don't }} it's fine {{ x` would be placed in the first expression,
        because the apostrophe in "don't" opens a string that closes at "it's".
        The region API also applies no parse gate, so for text that does not
        parse its complete region can end later than the scanner's span (that
        is the point: unfinished text). The remaining ambiguity is that a prose
        apostrophe after a broken expression can read as an open string, so
        completion may be offered in prose there. Alternatives: gate regions on
        the scanner's spans (breaks completion after `load_data(f,
        "{a: {b: string}}") ` with no closing braces, the case the spec
        requires), or no restart rule (misplaces the cursor in the prose case).
        Redo cost: `region_from` in simplified-schema's scan.rs plus
        `tests/l1/expression_regions.rs`.
    - |-
        The rule-25 corpus test allows one case the plan's wording does not:
        after a fixed span ends at its real closer, the scanner can reach an
        opener that the old rule used to swallow, and that opener then gets the
        old rule's span, which may not parse. This is SP1's "garbage stays
        garbage and gets shorter" result on the spec's own acceptance line. The
        test accepts a new span only if it parses or equals the old rule applied
        at its own start. Under that precise invariant, no span that parsed before
        changes, and every span the new rule produces parses. Redo cost: the
        assertion in `simplified-schema/lib/tests/l1/scanner_corpus.rs`.
    - |-
        The data locators make choices the plan left open. (1) The root pointer
        `""` locates the first token, in JSON, JSONL (the first record), and
        JSON5, and offset 0 in TOML. (2) `locate_json` follows serde_json's
        `RawValue` grammar, which does not check `\u` escapes, so a lone
        surrogate in a string is still located even though full parsing
        rejects it. Callers locate text they have already parsed, so a second,
        strict parse was judged unnecessary. (3) JSON5 member names written
        with identifier escapes (`\u0062`) are decoded, so they compare as they
        read in the value tree. The plan said to take `Name` tokens raw.
        `json5_parse_error_offset` reads json-five's `ParsingError.index`,
        which is already a byte offset. Redo cost for each: one function in
        biscuit-file/lib/src/data_locations.rs plus its tests.
    - |-
        `PositionMap::project` now projects list and scalar roots too, as the
        plan intended. Its non-frontmatter callers through `build_position_map`
        (darkmatter's whole-YAML-document loader, the trigger envelope, and
        simplified-schema's envelope) now get exact positions for such roots
        where they used to fall back to top-level keys. Frontmatter callers
        cannot reach it with a non-mapping root: such frontmatter fails to
        deserialize into a map first. All darkmatter tests pass unchanged.
packages:
    - simplified-schema
    - biscuit-file
    - biscuit-file-cli
    - darkmatter
    - darkmatter-lsp
source_files_during_phase_2:
    - simplified-schema/lib/src/coerce.rs
    - simplified-schema/lib/src/coercion/error.rs
    - simplified-schema/lib/src/coercion/mod.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/simplified/narrowing.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/container_contracts.rs
    - simplified-schema/lib/tests/l1/located_failures.rs
    - simplified-schema/lib/tests/l1/soft_validation_repair.rs
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/lib/src/detect.rs
    - biscuit-file/lib/src/format.rs
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/json_lines.rs
    - biscuit-file/lib/src/delimited.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/test_layout.rs
    - biscuit-file/lib/tests/l1/format_detection.rs
    - biscuit-file/lib/tests/l1/json_lines.rs
    - biscuit-file/lib/tests/l1-delimited/main.rs
    - biscuit-file/lib/tests/l1-delimited/delimited_reader.rs
    - biscuit-file/cli/src/main.rs
    - biscuit-file/cli/tests/cli_tests.rs
    - biscuit-file/justfile
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/schema_language_ownership_guard.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - Cargo.lock
docs_updated_during_phase_2:
    - biscuit-file/README.md
    - biscuit-file/lib/README.md
    - biscuit-file/docs/dependencies.md
    - docs/dependencies.md
    - simplified-schema/docs/topics/conversion.md
    - simplified-schema/docs/topics/custom-vocabulary.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/validation.md
    - simplified-schema/docs/topics/language/schema-types.md
    - simplified-schema/docs/topics/language/types.md
docs_created_during_phase_2:
    - biscuit-file/docs/topics/data-readers.md
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/architecture.md
    - .claude/skills/biscuit-file/references/detect.md
    - .claude/skills/simplified-schema/SKILL.md
    - .claude/skills/simplified-schema/declaration-narrowing.md
source_files_during_phase_3:
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/source_position.rs
    - biscuit-file/lib/src/data_locations.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/source_position.rs
    - biscuit-file/lib/tests/l1/data_locations.rs
    - biscuit-file/lib/tests/l1-delimited/main.rs
    - biscuit-file/lib/tests/l1-delimited/delimited_locations.rs
    - simplified-schema/lib/src/positions.rs
    - simplified-schema/lib/src/expression/scan.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/expression_scanner.rs
    - simplified-schema/lib/tests/l1/expression_regions.rs
    - simplified-schema/lib/tests/l1/scanner_corpus.rs
    - simplified-schema/lib/tests/fixtures/scanner/regressions.md
docs_updated_during_phase_3:
    - biscuit-file/docs/topics/data-readers.md
    - biscuit-file/docs/dependencies.md
    - biscuit-file/lib/README.md
    - docs/dependencies.md
    - simplified-schema/docs/topics/foundations.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/architecture.md
    - .claude/skills/simplified-schema/SKILL.md
message_to_agent: |-
    Phase 3 landed the locators and the scanner core. Phases 2 and 3 are both
    still uncommitted in the working tree.
    (1) biscuit_file::{source_position(text, byte_offset) -> Option<(line,
    char_column)>} (1-based; LF and CRLF break lines, a lone CR does not;
    None past the end or off a char boundary) and data_locations::{locate_json,
    locate_jsonl, locate_toml (feature toml), locate_json5 and
    json5_parse_error_offset (feature json5)}, each (text, pointer) ->
    Option<usize byte offset>, re-exported at the crate root. A missing child
    gives None, so darkmatter must locate the parent pointer itself (rule 5).
    CSV and TSV need no locator: DelimitedField.offset is the raw offset. YAML:
    PositionMap::project now accepts list and scalar roots; take
    PositionMap::byte_range(...).start and convert it with source_position.
    Never use ValidationProblem.column (bytes) or top_level (column-1 guesses).
    (2) simplified_schema::expression::scan now applies the balanced,
    parse-gated rule (R11) everywhere scan/scan_plain is called, so
    `{{ load_data(f, "{a: {b: string}}") }}` is one expression in composition
    already. The editor API is scan::expression_region_at(text, offset,
    excluded) -> Option<ExpressionRegion { start, end: Option<usize>,
    string_start: Option<usize>, brace_depth }>. Use it for DMLS
    completion_partial and the dsl.rs/frontmatter.rs rfind("{{") sites. Route
    mask_interpolations, protect_interpolation_expressions, and claudine's
    whole_value_span through scan_plain spans (Phase 4 "Scanner consumers").
    claudine's SpanScanner is optional (rule 24).
---

# Implementation Log for 2026-11-04-loading-data (9 phases)

## Phase 1

Phase 1 ("Contract baseline") is an audit and record-keeping phase. It
changed no source, documentation, schema, or skill file.

### Rules carried forward

The plan's 26 Necessary Rules are the binding design for every later phase
and are not restated here. Summary of the outcomes they fix, so a later phase
can check its work against them:

- Output: `data_location` (rule 5's `DataLocation`) is added to `Diagnostic`
  and to the closed `diagnostic` type of every output schema a `load_data`
  failure can reach (compose, render, graph, validate-refs at minimum; the
  remaining 13 are audited in Phase 6). `read_set: string[]` is always
  serialized on `DocumentEnvelope`, `ComposeReport`, and `ComposeFailure`.
  The contract page stays `docs/cli/data-contracts.md`.
- Codes: six errors, `dm.data.{not_a_container, row_shape_conflict,
  this_import_rejected, unsupported_format, malformed, contract_mismatch}`,
  capped at 20 contract violations plus one overflow record; never in
  `COMPOSE_DIAGNOSTIC_CODES`.
- Errors stay typed (`ExpressionError::Data(Arc<DataError>)`) until
  `report_diagnostics.rs` projects them; file failures stay
  `dm.expression.evaluation_failure` with `FileReferenceDiagnostic`, and the
  projection gains the four reference fields for every read-side function.
- Locators live in biscuit-file (JSON, JSONL, TOML, JSON5, delimited);
  YAML uses simplified-schema's `PositionMap::project`.
- `DataRuntime` memoizes per command in every cache mode, invalidated after
  each executed command attempt; shared like `RemoteFetchRuntime`.

### Existing workspace edits (not this feature's work)

At the start of Phase 1, `git status` showed 57 modified and 7 untracked
paths, all belonging to the 2026-11-06-dotted-paths review cycle (its
`review-1.md`–`review-4.md` and `log.md` are untracked) plus the empty
`implementation-log.md` of this feature. Touched areas: darkmatter lib
(`compose/{pipeline,setters,schema_validation}`, `compose/expression/scope/raw.rs`,
`schemas/{catalog,completion,mod,rewrite,triggers,yaml_documents}`), darkmatter
CLI (`commands/{compose,frontmatter,schema/validate}`, `args/path_completion`,
`main.rs`), simplified-schema (`coerce.rs`, `file_match.rs`, `resolve.rs`,
`resolve/buffers.rs`, `setter_value.rs`, `validate.rs`), claudine
(`composition/schema/{classify,mod}`), docs and skills. None was modified by
this phase.

### Prerequisite evidence

- 2026-11-04-composition-errors is closed (`darkmatter/fixes/_completed/`).
- 2026-11-06-dotted-paths is implemented through its Phase 10 (commit
  `32d1302c7`) and is in its review cycle; its code is in the tree.
- The `sandboxing.yaml` carve-out is present at
  `darkmatter/lib/tests/l1/shipped_schema_corpus.rs:1710`.
- Every path the plan names exists: `output/command/document.rs`
  (`DocumentEnvelope` at :86), `cli/src/output/report.rs` (`ReportSink` at
  :36), `compose/expression/diagnostics.rs` (`Diagnostic` at :119),
  `report_diagnostics.rs`, `policy.rs`, `functions/fs.rs` (`resolve_arg` :60,
  `file_exists_fn` :168, `load_markdown` :940), `context/options.rs`
  (`remote_fetch` :662, `remote_fetch_runtime` :2388,
  `expression_resolution_context_in` :1824,
  `local_expression_resolution_context_in` :1874), `pipeline/mod.rs`
  (remote-fetch reuse at :71–72), `conditions.rs`
  (`evaluate_condition_against` :355, `ShortcutLookup` :398),
  `preflight/collect.rs`, `call_check.rs`, `reference/graph.rs`
  (`inline_pre_options` :1166), `remote.rs` (`REMOTE_READ_FUNCTIONS` :28),
  `yaml_documents/load.rs`, `cache/{remote_cache,types}.rs`,
  `examples/expression_doc_generator.rs`; simplified-schema `coerce.rs`
  (`partial_copy` :599, `failing_paths` :657, `flatten_failures` :796).
- Rule 4's premise holds: `report_diagnostics.rs:399` fills
  `reference_details` only for `MarkdownError::Reference`.
- `detect_from_extension` is private (`biscuit-file/lib/src/detect.rs:155`);
  `narrowing::denotes_container` does not exist yet.

### Contract baseline audit

- `DocumentEnvelope` fields (`file`, `outcome`, `document`, `diagnostics`,
  `summary`, `perf`, `lifecycle_output`) match `cli/schemas/output/compose.yaml`
  and the field table in `docs/cli/data-contracts.md` "## `md compose` and
  `md render`".
- `Diagnostic` fields (`code`, `severity`, `message`, `source`,
  `expected_type`, `actual_type`, `span`, `iteration_path`, flattened
  `reference_details`) match the schema's closed `diagnostic` type and the
  doc's "## Diagnostics" and "### Reference details" tables.
- 17 output schemas under `cli/schemas/output/`; each declares its own
  `diagnostic` type (clean.yaml also has `clean-diagnostic`). None has
  `data_location` or `read_set`, as expected before Phase 6.
- Drift found: only the plan's line citations in rule 17 (see
  `decisions_for_review`). No behavioral drift; nothing fixed.

### Historical SP1 result

Recorded, not re-measured (rule 25): over 6,909 repository files holding
10,399 expressions, the balanced, parse-gated scanner changed 6 files, every
change turning a non-parsing span into a parsing one; no file under
`prompts/`, `tests/fixtures/`, or `docs/` changed. Later tests must not assert
that today's checkout has exactly six differences.

### YAML pointer-only limitation

For a YAML data file containing an alias or a multi-line flow collection,
`PositionMap::project` refuses the whole file, so a contract violation's
`data_location` carries `file` and `pointer` only, with no `line`/`column`.
The `PositionMap::top_level` fallback (top-level keys at column 1) is never
reported as a `data_location`. Parser errors in such files still keep their
authoritative parse-error positions.

### Gates

Scope calculator (run from the repository root after all Phase 1 edits):

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, claudine-cli, darkmatter, darkmatter-cli,
  simplified-schema, test-toolkit, darkmatter-lsp, sniff]`,
  `change_class: package`. The list comes from the dotted-paths edits
  already in the tree.
- The same command restricted to this feature's directory still reports
  `packages: [darkmatter, darkmatter-cli]`, `change_class: package`, so the
  gates were not skippable.

Gates, run once on macOS with no subagents running:

- `just test` (darkmatter): 14,438 run, 14,438 passed, 10 skipped, 103.9 s.
  No failures and no load-sensitive reruns.
- `just lint` (darkmatter): exit 0, including the zed-dmls wasm32-wasip2 check.

Not run, and why:

- Tests of dependent packages: Phase 1 changed no compiled code, so no
  dependent package's behavior can differ from baseline.
- `just cross-check`: no OS risk (no code changed), and plan Phase 9 forbids it
  because it makes a throwaway commit.

Requirement-to-test mapping: Phase 1 changes no behavior, so it adds no
tests. The baseline proves the interfaces later phases extend are green:
`output_envelope::*` and `output_commands` conformance in darkmatter-cli
L1, and `shipped_schema_corpus` in darkmatter L1, all inside the full run
above.

## Phase 2

Phase 2 ("Schema contracts and file readers") ran as two parallel subagent
tasks, one per crate, followed by one gate pass run by the orchestrator with
no subagent running.

### Schema contracts (simplified-schema)

- **Container predicate (rule 12):** `narrowing::{ContainerAdmission,
  container_admission, denotes_container}`. Null arms are dropped. Every
  non-null arm must be an array, tuple, `object`, inline object, or pattern
  dictionary, and at least one must exist. `Imported`/`Ref` arms are
  `Provisional`. `"[string, number]"` written as YAML is a union of scalars
  and is rejected.
- **`container` option:** `Constraint::Container` is wired through the
  grammar, types, lowering (`{"container": true}`), serialization, and
  format.rs `TypeDefinitionOptions`. For the payload key: absent or `false`
  turns the check off, `true` turns it on, and `null` or a wrong type is a
  preparation error. Darkmatter's authoring metadata (`schemas/about.rs`
  descriptors, match-safe list) and its `matches!` sites (call-binding
  translate, trigger grammar/matcher) accept it.
- **Shared flattening (rule 20):** `coercion::{locate_failures,
  LocatedFailure, ExpectedType, ActualKind, json_pointer}` and
  `ConversionError::located_failures()`. The private `coerce.rs::pointer` was
  moved here, so no third pointer encoder exists. `failing_paths` and
  `annotate_problems` both use the helper; the private `flatten_failures` is
  gone. The ownership guard now lists these names and the narrowing names.
- **validate_soft repair (rule 21):** `partial_copy` masks unexpected keys and
  tuple elements out of the retry input, binds the rest with the existing
  relaxed binder, and reinserts the masked entries in their original order.
  The production binder is unchanged. `Conversion::Failed { committed,
  diagnostic }` keeps write-back and diagnostics apart.

### File readers (biscuit-file)

- `DataFormat`/`FileType` gain `Jsonl`, `Csv`, and `Tsv`. `detect_from_extension`
  is now public and re-exported, and maps `.ndjson` to `Jsonl`.
- `bf` rejects the three new formats with
  "`<FORMAT>` input is not supported for conversion; supported inputs: …".
- `json_lines::read_json_lines` has no feature gate. `delimited::read_delimited`
  sits behind the new opt-in `delimited` feature (csv 1.3 + csv-core 0.1.13).
  That feature is not in `default`, is in `full`, and is enabled on
  darkmatter's dependency.
- The delimited reader runs one strict pass with csv-core first, which finds
  each field's raw start offset and enforces the quote and width rules. A
  decoding pass with `csv` then supplies the cell text.
- Tests added: the `l1-delimited` target (`required-features = ["delimited"]`),
  CI `features = ["fetch", "delimited"]`, and the layout gate. biscuit-file's
  `just test`/`just lint` recipes now include `delimited`. Before that change,
  the new binary did not run locally (found during the gate pass).
- `cargo tree`: csv/csv-core are absent with `--no-default-features` and with
  the default features, and present with `--features delimited`.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| R2 tables via predicate | `container_contracts::{every_container_row_denotes_a_container, scalars_unknown_and_null_alone_are_not_containers, a_bracket_list_of_scalars_is_a_union_not_a_tuple, an_unresolved_reference_is_provisional_never_proof}` |
| R2 via format check (validate_strict) | `container_contracts::{container_values_pass_the_format_check, non_container_values_fail_the_format_check, a_named_reference_passes_provisionally_without_io, container_combines_with_the_other_options, a_property_option_may_require_a_container_declaration}` |
| `container` grammar | `container_contracts::{container_parses_and_round_trips, container_takes_no_arguments, container_is_only_an_option_of_type_definition}` |
| payload absent/false/true/null/wrong type | `container_contracts::{container_lowers_to_a_true_flag, an_absent_or_false_flag_leaves_the_check_off, a_true_flag_turns_the_check_on, a_null_or_non_boolean_flag_is_a_preparation_error}` |
| R5 flattening, union reachability, SP3 table | `located_failures::*` (9 tests: plain array, nullable array never lists null, tuple missing/extra, object extra key `absent`, two reachable arms → one record at `""`, per-element union, one reachable arm descended, pointer escaping `/` `~` empty key, ambiguous union) |
| rule 21 validate_soft | `soft_validation_repair::*` (7 tests: extra row key only problem, extra tuple element, nested object siblings converted, failing value beside extra key, nullable container, unions no arm explains still fail, write-back never commits partial value) |
| R3 extension detection, enums | `format_detection::*` (7 tests) and the unit tests in detect.rs/format.rs |
| JSONL reader rules | `json_lines::*` (8 tests: order, blank lines counted, CRLF, number spelling range, zero-byte, invalid record line + byte offset with multibyte char, exact location, two values on one line) |
| R4 strict delimited rules, raw offsets | `l1-delimited delimited_reader::*` (16 tests, comma and tab: `a,"x""y,z",q` raw starts 0/2/11, quoted newline, CRLF, blank lines, header after blank lines, delimiter-only row, no trimming, header-only, empty, ragged wider/narrower, unterminated quote, quote in unquoted field, text after closing quote, invalid delimiter, comma plain in TSV) |
| bf rejection | `cli_tests::detected_data_formats_without_a_converter_are_rejected` |

Input robustness: no format or configuration loader is wired up to user
files yet, because `load_data` arrives in Phases 4 and 5. The readers' own
document shapes are covered above: an empty file, blank lines, CRLF,
trailing or invalid content (a JSONL second value; text after a closing
quote), and malformed quoting. The `container` payload field covers absent,
`false`, `true`, `null`, and wrong type.

### Gates (macOS, no subagents running)

- Scope calculator:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [biscuit-file, biscuit-file-cli, darkmatter, darkmatter-lsp,
  simplified-schema, test-toolkit, claudine, darkmatter-cli, sniff]`,
  `change_class: package`. The gates were therefore required.
- darkmatter `just test`: 14,438 run, 14,438 passed, 10 skipped (109 s).
  No test took 5 s or more.
- darkmatter `just lint`: exit 0, including the zed-dmls wasm check.
- simplified-schema `just test`: 1,518 passed (all features) and 1,330
  passed (no default features). `just lint`: exit 0.
- biscuit-file `just test` (now including `delimited`): 1,230 passed, plus 6
  minimal-tier tests. `just lint`, now including the delimited clippy run:
  exit 0.
- `just check-tier-coverage` for darkmatter, biscuit-file, and
  simplified-schema: 0 stranded.
- Dependents:
  - claudine, filtered `/schema|validat|coerc|frontmatter|format/`:
    1,405 passed.
  - `cargo check --all-targets` over the 17 other crates that depend on
    biscuit-file or simplified-schema (research, schematic-define/gen,
    worktree, model-citizen, renderable, claudine-cli/gen, playa-cli,
    tree-hugger(-cli), sniff(-detection-cli), messenger(-cli),
    content-policy, biscuit-terminal): clean.
- No failures, no load-sensitive reruns, and no pre-existing failures.
- Cross-OS check: not run. The new code has no path, process, or OS-specific
  behavior. Line endings are parsed from text and covered by CRLF tests on
  every host. CI will run Windows and WSL2 nightly.

## Phase 3

Phase 3 ("Source locators and scanner core") ran as two parallel subagent
tasks with disjoint file ownership: biscuit-file locators plus
`positions.rs`, and simplified-schema `expression/scan.rs`. The orchestrator
then reviewed the work, fixed dependency-doc drift, documented the scanner
rule, and ran every gate once with no subagent running.

### Source locators (biscuit-file, simplified-schema positions)

- `source_position(text, byte_offset)` (no feature gate) returns a 1-based
  line and a character column. LF and CRLF break lines; a lone CR is an
  ordinary character. An offset past the end or inside a character gives
  `None`.
- `data_locations::{locate_json, locate_jsonl}` (no gate), `locate_toml`
  (`toml`), and `locate_json5` plus `json5_parse_error_offset` (`json5`) take
  a pointer and return a byte offset (rule 7).
  - JSON walks containers on the path as `RawValue`, last duplicate wins. The
    crate now enables serde_json's `raw_value` itself.
  - JSONL indexes the non-blank records and adds each line's start offset.
  - TOML walks `DeTable::parse` spans.
  - JSON5 descends over `tokenize_str` spans, last duplicate wins.
  - `ParsingError.index` is already a byte offset in json-five 0.3.1
    (`parser.rs:235`).
- Malformed text or a malformed pointer gives `None`, and so does a missing
  child. Darkmatter will locate the parent itself.
- CSV/TSV need no new locator; the Phase 2 `DelimitedField.offset` is raw,
  and `delimited_locations` proves it with `source_position`.
- `PositionMap::project`: only the mapping-root check was removed. Frontmatter
  can never reach it with a non-mapping root: `parse_yaml_map_rejecting_duplicates`
  deserializes into a map first, and the other caller serializes a mapping it
  built itself. `top_level` and `LineIndex` are unchanged.

### Scanner core (simplified-schema)

- `scan_expression` computes the legacy span and the balanced span (quotes
  with `\` escapes, multi-line strings, single-brace depth). If the two end at
  the same place, nothing is parsed. Otherwise the legacy span is kept when it
  parses. Failing that, the balanced span is used only if it parses, and
  scanning resumes after it. In every other case the legacy span and its
  resume point stand. Literals, tokens, and escaped openers are unchanged.
- `expression_region_at(text, offset, excluded) -> Option<ExpressionRegion
  { start, end, string_start, brace_depth }>` reports editor cursor state
  with the same lexical rules and no parse gate. It adds the restart rule
  recorded in decisions_for_review.
- The module doc now states the invariant and the trade-off in place of
  "purely lexical".
- Performance: the balanced pass is one forward pass per opener. An
  unterminated quote in prose makes that pass run to the end of the text, so
  a document full of such openers scans in quadratic time. No corpus file
  comes close; SP1 recorded no measurable cost.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| rule 6 position helper (LF, CRLF, lone CR, multibyte, bounds) | biscuit-file `source_position::*` (6) |
| rule 7 JSON (root/nested, padding, tabs/minified, `~0` `~1` empty key, missing child → parent, duplicate last wins, surrogate escapes, multibyte, CRLF, malformed) | `data_locations::json::*` (11) |
| rule 7 JSONL (blank lines keep physical lines, root, CRLF, missing record → parent, malformed) | `data_locations::jsonl::*` (5) |
| rule 7 TOML (array of tables, dotted keys, inline tables/arrays, super-table after sub-table, escapes, multibyte, CRLF, missing child, malformed) | `data_locations::toml::*` (6) |
| rule 7 JSON5 (comments, unquoted/single-quoted keys, trailing commas, escapes, duplicate, line continuation, multibyte, CRLF, missing, malformed, parse-error byte offset after multibyte) | `data_locations::json5::*` (9) |
| delimited quotes/doubled quotes/embedded delimiter/quoted newline, multibyte, quoted tab TSV, CRLF | biscuit-file `l1-delimited delimited_locations::*` (5) |
| `project` relaxed root | `positions::tests::{a_list_root_is_projected, a_scalar_root_is_projected, an_alias_under_a_list_root_still_refuses_projection}`; existing positions tests unchanged |
| R11 scanner acceptance cases | `expression_scanner::*` (13) and unit tests `the_balanced_close_skips_strings_and_single_braces`, `the_gate_keeps_a_parsing_older_span` |
| R11 editor cursor state (incomplete region after a nested type string, quoted `{{`, depth, CRLF/multibyte, literals/tokens/escapes/excluded) | `expression_regions::*` (10) |
| rule 25 fixed-input corpus comparison | `scanner_corpus::scanner_corpus_changes_only_spans_that_did_not_parse` reads the frozen `tests/fixtures/scanner/regressions.md` and five shipped schemas (`darkmatter/schemas/partials/{lifecycle,functions}.yaml`, `darkmatter/schemas/darkmatter.yaml`, `simplified-schema/schemas/{schema,yaml-schema}.yaml`), each through `include_str!` inside the test |

Input robustness: the locators read text a loader has already parsed, and
they never decide whether a document is valid. The only shapes they decide
are pointers: valid, malformed, escaped, empty key, missing child, and
duplicate key. Each of those is covered per locator above. No loader is wired
up to user input yet; that work is in Phases 4 and 5.

### Gates (macOS, no subagents running)

- Scope calculator:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [biscuit-file, biscuit-file-cli, darkmatter, darkmatter-lsp,
  simplified-schema, test-toolkit, claudine, darkmatter-cli, sniff]`,
  `change_class: package`. That output covers the Phase 2 and Phase 3 edits,
  which are both uncommitted. The gates were therefore required.
- simplified-schema `just test`: 1,547 and 1,359 passed. `just lint`: exit 0.
- biscuit-file `just test`: 1,272 passed, plus 6 minimal-tier tests.
  `just lint`: exit 0.
- darkmatter `just test`: 14,438 passed, 10 skipped (108 s). `just lint`:
  exit 0, including the zed-dmls wasm check.
- `just check-tier-coverage` for darkmatter, biscuit-file, and
  simplified-schema: 0 stranded.
- Dependents:
  - claudine, filtered
    `/sequence|interpolat|expression|scan|template|frontmatter|compose|composition/`:
    2,529 passed. Claudine consumes the scanner.
  - `cargo check` of sniff, content-policy, renderable, tree-hugger,
    worktree, and messenger: clean.
- No failures, no load-sensitive reruns, and no pre-existing failures.
- Cross-OS check: not run. The changes are pure text processing with no
  paths, processes, or OS APIs. CRLF and lone-CR handling is tested on every
  host, and CI runs Windows and WSL2 nightly.
