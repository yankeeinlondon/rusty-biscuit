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
    - |-
        Phase 4: clearing the data memo after a command also clears the cache
        of prepared `as` contracts. The plan names only the data maps. The
        reason is that a command can regenerate an imported schema file, and a
        kept contract would then describe the old file. Alternative: keep
        contracts across commands, which saves re-resolving imports but can
        serve a stale imported type. Redo cost: one line in
        `DataRuntime::invalidate_memo` and one runtime test.
    - |-
        Phase 4: the typed contract-schema error carries the `as` text as
        written beside the schema error (`ContractSchema { written, source }`).
        The plan wrote `ContractSchema(Arc<SchemaError>)`. Projection then has
        the written text without a second lookup. `prepare_contract` returns a
        boxed error because clippy rejects the unboxed one as too large. A
        missing, unmatched, or unreadable import becomes a file-reference
        error that names the import; every other schema error becomes
        `ContractSchema`. Redo cost: small, confined to
        `compose/expression/data/{error,contract}.rs`.
    - |-
        Phase 4: when `file_exists` finds no file, the read set lists every
        candidate path it probed, each canonicalized through its existing
        parent folder. A missing path cannot be canonicalized whole, and
        without this macOS would report `/var/...` for a missing file but
        `/private/var/...` once the file exists. Alternative: record the raw
        absolute candidate text, which is simpler but spells the same file
        two ways. Redo cost: one helper in `functions/fs.rs`.
    - |-
        Phase 4: the read set is copied onto every root `ComposeReport`,
        including results from `compose_with`, not only through
        `compose_with_warnings`. `ShortcutLookup::new` became test-only,
        because no production code still calls it. A new public adapter,
        `conditions::evaluate_condition_in(expr, data, &ResolutionContext)`,
        lets compose-owned conditions share the request's runtime, and
        `evaluate_condition_against` delegates to it. Redo cost: low.
    - |-
        Phase 4 gap, not decided away: `validate_schema(...)` records the
        document it validates but not the schema files that validation reads.
        R8 asks for schema files read while running `validate_schema`. The
        schema resolver's `referenced_files` are not yet threaded back to the
        expression function. Phase 5 or 6 should add them, at the same place
        `load_data` records its contract imports.
    - |-
        Phase 4 scanner consumers: claudine's `whole_value_span` is kept, and
        the shared scanner is a fallback (`whole_value_expression`) used only
        when the old first-`}}` rule rejects the value. So a value the old rule
        accepted keeps its old reading. Claudine's `SpanScanner` treats each
        complete expression, literal, or token range that the shared scanner
        reports as opaque, so an escaped quote inside one cannot expose `::`
        or `->`. Openers the scanner does not report keep the old `{{`/`}}`
        counter. `mask_interpolations` and `protect_interpolation_expressions`
        take each expression's end from the scanner. Literal and token openers
        keep the first-`}}` rule they had before, which is why
        `protect_interpolation_expressions("k: {{{ a }}} b")` still gives
        `__DM_EXPR_0__} b`. Redo cost: confined to those functions and their
        tests.
    - |-
        Phase 5: a function's binding failure can now be re-read as a domain
        error. `FunctionBinding::refining_binding_errors` registers a hook
        that runs only after the call failed to bind. `load_data` uses it so
        that a computed scalar `as` (for example a frontmatter value
        `"string"`) fails with `dm.data.not_a_container`, as spec R2 requires,
        instead of the generic type error the `type-definition(container)`
        parameter would give. The plan did not say how the computed case gets
        its code. Alternatives: drop `container` from the catalog parameter
        (loses the editor and passive checks the option gives), or match the
        binder's error during diagnostic projection (fragile text-free
        matching in Phase 6). Redo cost: the hook in `functions/mod.rs` and
        `data::load::refine_binding_error`.
    - |-
        Phase 5: `"[string, number]"` written as the `as` string is rejected by
        the passive check as malformed syntax (`dm.expression.incompatible_call`),
        not as `dm.data.not_a_container` as spec R2's acceptance line says.
        The shared type parser does not accept a bracket list inside a string
        (only a YAML sequence spells that union), so the text never reaches the
        container rule. It is still rejected before anything runs. Getting the
        spec's code would need a grammar change in simplified-schema. Redo
        cost: that grammar change plus one test
        (`a_bracketed_scalar_union_is_rejected_as_malformed_syntax`).
    - |-
        Phase 5: a URL is fetched before the data memo is consulted, through
        the existing `fetch_remote_text`, which checks host policy and is
        single-flight and cached on its own. A fetch error stays a plain
        evaluation failure, exactly like `frontmatter()` today; the typed
        file-reference projection of a denied host for every read-side
        function is Phase 6's (rule 4). A missing remote runtime (the
        reference check) is the file-reference RemoteNotEnabled error and never
        reaches the memo. Redo cost: confined to `data/load.rs`.
    - |-
        Phase 5: number admission failures (an inexact integer such as
        9007199254740993, NaN or infinity in TOML/JSON5) are reported as
        `dm.data.malformed`, located where the parser proves it, because they
        are each format's numeric rules rather than a contract mismatch. JSON5
        uses simplified-schema's existing `admit_json5_text_numbers` (it reads
        each number's spelling) instead of walking parsed values, because a
        value walk cannot see an integer json-five already rounded. YAML
        custom tags are rejected anywhere in the tree, on this path only; the
        shared YAML loader is unchanged. Redo cost: `data/parse.rs`.
    - |-
        Phase 5: the CSV/TSV source memo stores the parsed table (records with
        field offsets), and rows are built per contract at bind time, because
        the row shape depends on `as` while the source memo is keyed by the
        file alone. So the object-mode duplicate/empty-header error is raised
        at bind time and memoized per contract. `ContractMismatch` now also
        carries the table and row mode (`TabularSource`) and always the text,
        local or remote, so Phase 6 can locate cells. `DataSourceIdentity::Remote`
        boxes its URL to keep `DataError` under clippy's large-error limit.
        Redo cost: `data/{runtime,error,load}.rs`.
    - |-
        Phase 5 passive check: an embedded `Name@darkmatter:<stem>` import is
        expanded passively only when every import in the contract is a shipped
        one; any file import leaves the whole contract provisional. An unknown
        shipped type name is accepted passively and fails at execution as an
        invalid contract (a static error would be about five lines). When both
        `file` and `as` are wrong, both findings are kept in argument order.
        For typing (R10), every named reference projects to `unknown` in
        place, so `{ host: Host@./t.yaml, port: number }[]` keeps `port`.
        Redo cost: `call_check.rs` and `contract::expand_embedded_contract`.
    - |-
        Phase 5 found that adding a catalog function must also update several
        hand-kept copies that the plan did not list: the two verb pattern
        lists in `schemas/partials/md-lifecycle.yaml`, and Claudine's mirrored
        expression category in `claudine/schemas/action.yaml` and
        `claudine/schemas/lifecycle.yaml` (a Claudine parity test compares it
        with Darkmatter's). These now include `load_data`, so `load_data` is
        also a lifecycle verb (`- load_data: [file, as]`), as every other
        value-accepting function is. The darkmatter skill now lists every such
        copy. Redo cost if load_data should not be a lifecycle verb: an
        exclusion in the lifecycle schema generator and those four YAML files.
packages:
    - simplified-schema
    - biscuit-file
    - biscuit-file-cli
    - darkmatter
    - darkmatter-lsp
    - claudine
    - darkmatter-cli
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
    Phases 4 and 5 are both uncommitted in the working tree (Phases 1-3 are
    committed). load_data runs end to end: `expression/data/load.rs` (the
    function and the binding refiner) and `data/parse.rs` (decode, BOM, UTF-8,
    per-format parsing and number admission). Every failure is a typed
    `DataError`; `DataError::code()` gives the `dm.data.*` code, and
    `ExpressionError::from(Arc<DataError>)` keeps the file family as
    `ExpressionError::FileReference`. Until Phase 6 projects them, data
    failures still publish as `dm.expression.evaluation_failure` (with the
    right message). What Phase 6's projection has to work with:
    `ContractMismatch { identity, format, written, failure, text, table }` -
    `text` is always present (local and remote), `table` is
    `Some(TabularSource { mode, data })` for CSV/TSV (field offsets are in
    `data`); `Malformed { location: DataSourceLocation }` already holds the
    parser-proven byte offset/line/column (invalid UTF-8: `byte_offset` into
    the original bytes, no line or column). The passive check already
    publishes `dm.data.not_a_container`, `dm.data.this_import_rejected`, and
    `dm.data.row_shape_conflict` through `CallCheck::data_problem` and
    `CallCheck::diagnostics`, so composition and DMLS show them at the `as`
    span; do not add a second path for those. A remote fetch error is still
    an `ExpressionError::Other` string, like `frontmatter()`; rule 4's typed
    denial projection is yours. Known gap still open from Phase 4:
    `validate_schema` does not add its schema files to the read set. Adding a
    catalog function touches several hand-kept copies; the darkmatter skill's
    library-surfaces.md lists them. Tests: `load_data_execution` and
    `load_data_passive` in `darkmatter/lib/tests/l1/`. The function registry
    and parity test modules compile only with `--features
    effects-instrumentation` (the `just test` recipe enables it).
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/compose/expression/data/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/data/contract.rs
    - darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/data/error.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/shell.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/compose_read_set.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/graph.rs
    - darkmatter/cli/src/commands/validate.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/collection_editor.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/tests.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/flow-control/sequences.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/requests.md
source_files_during_phase_5:
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/md-lifecycle.yaml
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/data.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/expression/data/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/data/contract.rs
    - darkmatter/lib/src/markdown/compose/expression/data/error.rs
    - darkmatter/lib/src/markdown/compose/expression/data/load.rs
    - darkmatter/lib/src/markdown/compose/expression/data/parse.rs
    - darkmatter/lib/src/markdown/compose/expression/data/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/load_data_execution.rs
    - darkmatter/lib/tests/l1/load_data_passive.rs
    - darkmatter/lib/tests/l1/function_binding_registry.rs
    - darkmatter/lib/tests/l1/function_catalog_parity.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
    - darkmatter/dmls/src/diagnostics/call_check.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
docs_updated_during_phase_5:
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/library-surfaces.md
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

## Phase 4

Phase 4 ("Request runtime and scanner consumers") ran as two parallel
subagent tasks with disjoint file ownership: the request runtime (darkmatter
lib and CLI plumbing) and the scanner consumers (two darkmatter masking
functions, three DMLS region sites, claudine's sequence grammar). The
orchestrator reviewed both diffs and then ran every gate once, with no
subagent running.

### Request runtime

- New `compose/expression/data/{mod,contract,runtime,error}.rs`.
  - `DataError` keeps the file-reference family as
    `DataError::FileReference(FileReferenceDiagnostic)` and adds typed
    variants for the six data codes plus `ContractSchema`.
  - `ExpressionError::Data(Arc<DataError>)` is authoring-fatal, including
    inside callbacks. Until Phase 6 projects it, it falls through to
    `dm.expression.evaluation_failure`.
- `prepare_contract` implements rule 13: it parses the `as` text, rejects
  `@this` at any depth, and expands imports through
  `SchemaRoots::for_document`. Its result holds the bare shape and the
  exact memo key: the serialized schema, the bare declaration, and the
  sorted import identities. `classify_rows` implements rule 14 and reports
  a row mode as `Known` or `Provisional`.
- `DataRuntime` has three memos: contracts, sources keyed by identity, and
  bound results keyed by identity and contract. Each entry is
  single-flight through an `Arc<OnceLock<…>>`, and the map lock is never
  held while loading. Each memo carries a generation, so
  `invalidate_memo()` stops an in-flight load from filling the new maps.
  A remote-not-enabled deferral is never memoized. The read set is
  deduplicated and comes back lexically sorted. `DataRuntimeStats` gives
  counts for tests.
- Plumbing follows rule 17, on the remote-fetch pattern.
  - `ComposeOptions::{with_shared_data_runtime, with_data_runtime,
    data_runtime_handle}` attach a runtime, and `ResolutionContext.data`
    receives it in both expression-context builders. A root pipeline with
    none attached makes its own.
  - `md compose` attaches one before its first `prepare_request`, so the
    reference check, pre-flight, lifecycle, and composition share it; `md
    graph` and `md validate refs` do the same.
  - `MdShellRunner::run_values` shares the runtime through a new crate-only
    `execute_set_shell_values_sharing`. The public function claudine calls is
    unchanged.
- `read_set: Vec<String>` is on `ComposeReport` (unioned by `merge`) and
  on `ComposeFailure`, and `ComposeRequest::read_set()` returns a
  snapshot. The envelope and the output schemas wait for Phase 6.
- Invalidation (rule 15) happens in two places:
  `shell_expansion::execute_prepared_outcome`'s run closure, which covers
  `::shell`, shell blocks, frontmatter `$(...)`, and lifecycle `set` shell
  values, and `MdShellRunner::execute`, after every attempt. A cached or
  skipped command does not invalidate.
- Reads are recorded (rule 19) at the operation boundaries.
  - `fs.rs::load_markdown` records after resolution and before the read.
  - `file_exists_fn` records the URL, the file found, or on a miss every
    probed candidate.
  - `prepare_contract` records contract imports.
- Rule 18: `conditions::evaluate_condition_in` is the context-taking
  adapter. `ShortcutLookup::new` is now test-only.

### Scanner consumers

- DMLS: `completion_partial` and the new `expressions::expression_body_start`
  use `expression_region_at`. `dsl.rs` and `frontmatter.rs` take the scope
  start from `expression_body_start` instead of `rfind("{{")`.
- `mask_interpolations` and `protect_interpolation_expressions` end each
  expression where `scan_plain` ends it. Literal, token, escaped, and
  unclosed openers keep their old rule.
- In claudine, a value the old `whole_value_span` rejects falls back to the
  shared scanner (`whole_value_expression`). `SpanScanner` treats each
  complete interpolation range reported by `scan_plain` as opaque.
  `claudine/docs/topics/flow-control/sequences.md` describes this.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| R8 read set: frontmatter + missing `file_exists` probe; failure after resolving is on `ComposeFailure`; malformed and null record nothing; dedup and lexical order | `compose_read_set::{a_frontmatter_read_and_a_missing_file_probe_are_both_recorded, a_read_that_fails_after_resolving_is_on_the_failure, a_malformed_reference_and_a_null_argument_record_nothing, repeated_reads_are_recorded_once_in_lexical_order}` |
| R8 one test per evaluation surface (body; frontmatter; `::block`; `when=`; transclusion targets; `::map`/`::filter`) | the first four above (body), `frontmatter_interpolation_records_its_reads`, `a_block_condition_records_its_reads`, `a_transclusion_condition_records_its_reads`, `a_transclusion_target_records_its_reads`, `map_and_filter_callbacks_record_their_reads` |
| R7/rule 17 separate library compositions share nothing; an attached runtime spans the passes of one command | `separate_compositions_share_no_runtime`, `an_attached_runtime_spans_every_pass_of_one_command` |
| rule 15 an executed command empties the memo and keeps the read set; cached or skipped commands do not | `an_executed_command_empties_the_memo_and_keeps_the_read_set`, `a_cached_or_skipped_command_does_not_empty_the_memo` |
| rule 15 single-flight, two contracts = one read / two binds, in-flight old generation not reused, failure memoized but deferral not | lib unit tests under `expression::data::runtime` (29 data-module tests in total) |
| rule 13/14 contract preparation and row classifier (every rule-14 case, `@this` at depth, scalar rejected, `ContractSchema`, import identity per folder, missing import keeps file-reference family) | lib unit tests under `expression::data::contract` |
| rule 3 fatality and file-family conversion | lib unit tests under `expression::data::error` |
| R11 DMLS completion after a nested type string in an open `{{`; `}}` in a string; prose apostrophes; literals/escaped openers; quoted `{{` | `overlay::expressions::tests::{completion_partial_follows_a_nested_type_string_in_an_open_interpolation, completion_partial_ignores_a_closer_inside_a_string, completion_partial_keeps_prose_apostrophes_out_of_later_regions, completion_partial_declines_literals_and_escaped_openers, expression_body_start_skips_a_quoted_opener}`, l1 `collection_editor::a_quoted_opener_does_not_move_the_completion_scope_into_a_string` |
| R11 `$(cmd {{ f("}}") }})` keeps the command's shape | `frontmatter_shell_expansion` tests `argument_interpolation_with_a_quoted_closer_keeps_the_command_shape` (through execution) and `mask_interpolations_ends_each_expression_where_the_scanner_does` |
| R11 YAML fallback | `frontmatter::tests::{test_parse_frontmatter_fallback_keeps_a_quoted_closer_inside_its_expression, test_protect_interpolation_expressions_keeps_prose_and_literal_spans}` |
| R11 claudine `sequence: "{{ load_data(f, \"{a: {b: string}}\") }}"` is an expression; escaped quotes keep separators inside | claudine `sequence::tests::{a_quoted_closer_keeps_a_whole_value_expression, escaped_quotes_keep_separators_inside_the_interpolation, an_operator_paren_inside_an_interpolation_is_a_typed_error}` |

Input robustness: this phase adds no file-format loader. Phase 5 owns the
parsers and the matrix.

### Gates (macOS, no subagents running)

- Scope calculator:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, darkmatter, darkmatter-cli, darkmatter-lsp,
  test-toolkit, claudine-cli]`, `change_class: package`. Gates required.
- darkmatter `just test`: 14,490 passed, 10 skipped (113 s), none at or
  above 5 s. `just lint`: exit 0, including the zed-dmls wasm check.
- `just check-tier-coverage darkmatter` and `claudine`: 0 stranded.
- Dependents:
  - claudine, filtered `/sequence|interpolat|expression|scan|template|composition/`:
    2,492 passed. claudine `just lint`: exit 0.
  - `cargo check --tests` on research, claudine-cli, claudine-gen,
    messenger, messenger-cli, sniff-detection-cli, playa-cli,
    biscuit-speaks-cli, biscuit-terminal-cli, and biscuit-icon-cli: clean.
- Cross-OS: the read set compares canonical paths, including candidates for
  missing files, so Windows was checked.
  `./scripts/cross-check.sh darkmatter --os windows compose_read_set expression::data conditions md_host file_exists`:
  177 passed on build-win-native, including all 13 `compose_read_set` and 29
  data-module tests. `just cross-check` cannot pass a quoted `-E '…'`
  filter (it re-splits it), so the script was called directly with
  positional filters.
- No failures, no load-sensitive reruns, and no pre-existing failures.
- The runtime subagent once ran its own filter (the 29 data-module tests)
  with `--no-fail-fast` by mistake. The run was limited to those tests and
  had no effect.
- Docs: the darkmatter `docs/` pages for the read set and the runtime are
  Phase 8's (R12). Nothing user-visible on the CLI changed in this phase,
  because the envelope field arrives in Phase 6.

## Phase 5

Phase 5 ("Data execution and passive typing") ran in three steps. The
orchestrator first wrote the shared foundations: the catalog entry, the
registration, the six code constants, the passive contract judgment, and the
remote-read list. Then two subagents worked in parallel with disjoint file
ownership, one on data execution and one on passive typing. Last, the
orchestrator ran every gate once, with no subagent running, and fixed the
hand-kept copies the gates exposed.

### Foundations (orchestrator)

- `load_data(file: file | null, as: type-definition(container)) -> any | null`
  is in `schemas/partials/functions.yaml` (Filesystem, fallible). Its
  example `load_data("fixture.yaml", "string[]")` runs in the catalog example
  test. The generated function table in `docs/topics/darkmatter-expressions.md`
  was regenerated with `just regen-expr-doc`.
- `functions/data.rs` registers it with the alias `loaddata`. A new registered
  rule, `FunctionBinding::refining_binding_errors`, lets a function turn a
  failed binding into its own domain error. It runs only after binding has
  failed, and the handler is still never entered.
- `remote.rs::REMOTE_READ_FUNCTIONS` now includes `load_data`.
- `data/error.rs` adds the six `dm.data.*` constants, `DATA_DIAGNOSTIC_CODES`,
  and `DataError::code()`. None of them is in `COMPOSE_DIAGNOSTIC_CODES`.
- `contract::judge_written_contract` is the one passive verdict on an `as`
  text. It reports a syntax failure, `ThisImportRejected`, or `NotAContainer`.
  `prepare_contract`, the binding refiner, and the passive call check all
  start from it.

### Data execution

- `data/load.rs::load_data_fn` follows the plan's order. A null file returns
  null first. Then it prepares the contract (and records its imports),
  resolves the file and records it, takes the format from the path or URL-path
  extension (case-insensitive, through `detect_from_extension`), and takes the
  CSV/TSV row mode. A URL is fetched next, so host policy is checked before
  the memo. The source memo reads, strips the BOM, decodes, parses, and
  admits numbers. The bind memo then binds the bare shape. No partial value
  escapes.
- `data/parse.rs` handles each format:
  - YAML goes through the shared `load_yaml_document`, plus a tag rejection
    on this path only.
  - JSON uses serde_json plus `admit_json_text_numbers`. JSONL uses
    `read_json_lines` plus per-record admission.
  - TOML walks `Toml::value()` with `admit_integer` and `admit_f64`. The
    wrapper's `toml_to_json` is unchanged.
  - JSON5 uses `Json5`, `reject_non_finite_json5`, and
    `admit_json5_text_numbers`.
  - CSV/TSV use `read_delimited`.
  - Parser errors become a byte offset first, then a line and character
    column through `biscuit_file::source_position`. Invalid local UTF-8
    reports a `byte_offset` into the original bytes, with no line or column.
- Runtime changes:
  - `LoadedSource` holds a `SourceContent::{Tree, Table}`, the text (for
    remote sources too), and the byte count. `LoadedData` holds the item
    count.
  - `SourceEntry::from_memo()` and the new `BindEntry::from_memo()` say
    whether each memo level answered.
  - A `load_data` debug span records the identity, format, bytes, items, and
    both memo answers.
  - The four Phase 4 `#[allow(dead_code)]` markers are gone.
- `ContractMismatch` carries the text, plus `TabularSource { mode, data }`
  for CSV/TSV, so Phase 6 can locate cells.
- `functions/fs.rs` exposes `file_reference_diagnostic`, which
  `file_reference_error` wraps. `resolve_arg`, `file_reference_error`, and
  `expression_other` are now `pub(crate)` so `load.rs` can share them.

### Passive typing

- `CallCheck` has a new public field `data_problem: Option<DataProblem>`
  (code, argument, message). It is set for two-argument `load_data` calls,
  matched case-insensitively and by alias, whose `as` is a literal string.
  - When it is set, the call is rejected, and the problem is ranged at the
    `as` span.
  - `CallCheck::diagnostics` publishes the `dm.data.*` code, and composition
    and DMLS both read it from there.
  - The message is the `DataError` Display, so passive and runtime wording
    agree.
  - Malformed contract syntax keeps `dm.expression.incompatible_call`.
- A contract whose imports are all shipped (`Name@darkmatter:<stem>`) is
  expanded passively through `contract::expand_embedded_contract`, from the
  embedded sources only. A contract with any file import stays provisional,
  and no file is opened.
- A literal file with a `.csv`/`.tsv` path (for a URL, its path) applies
  `classify_rows` before anything runs.
- Typing (rule 22): `expression_type` types a literal contract through
  `ScopeShape::of_type`, with every named reference replaced by `unknown`.
  The file argument's nullability is applied: a literal null file gives
  null, a possibly null file adds `| null`, and a proven non-null file gives
  the bare type. The element type reaches `map`/`filter` callbacks. A
  computed or invalid contract falls back to `any | null`.

### Hand-kept copies the gates exposed (orchestrator)

The first full `just test` failed in five places. Each one is a copy of the
function list kept somewhere else:

- `just regen-lifecycle-schema` adds `load_data` verb types to
  `schemas/partials/lifecycle.yaml`. The verb count in
  `lifecycle_schema_generation.rs` went from 129 to 130.
- The two verb pattern lists in `schemas/partials/md-lifecycle.yaml` gained
  `load_data`.
- The recursive-schema goldens were re-blessed. The diff is only those two
  pattern lists (`load_data|`) and their digests.
- DMLS `a3_completions.json` was re-blessed. The diff is one new completion
  item, `load_data(file, as)`.
- The Claudine parity test `claudine_action_types_are_darkmatters_neutral_definitions`
  needed `load_data` in `claudine/schemas/action.yaml` and
  `claudine/schemas/lifecycle.yaml`.

The darkmatter skill (`library-surfaces.md`) now lists every such copy, along
with the binding-refinement and `data_problem` patterns.

### Requirement-to-test mapping

All tests are L1, run in-process through `compose_with_warnings` or the
passive-check API, each in its own TempDir.

| Requirement | Tests (`load_data_execution::` unless noted) |
|---|---|
| R1 example, null, named import, computed scalar and `@this` | `the_os_example_renders_as_csv`, `a_null_file_returns_null_and_reads_nothing`, `a_named_file_import_executes`, `a_computed_scalar_contract_is_not_a_container`, `a_computed_this_import_is_rejected`; refiner unit tests `data::load::tests` |
| R1/R2 passive: every yes/no row, `@this` at depth, provisional file import, shipped imports, null file still checked, alias spellings, malformed syntax keeps incompatible_call | `load_data_passive::container_rule::*`, `load_data_passive::imports::*` |
| R2 `"[string, number]"` | `load_data_passive::container_rule::a_bracketed_scalar_union_is_rejected_as_malformed_syntax` (see decisions) |
| R3 every extension, upper case, unsupported | one `*_loads` per extension, `an_upper_case_extension_loads`, `an_unknown_extension_is_unsupported` |
| R3 document-loader matrix (JSON last-wins and trailing garbage; YAML duplicate, multi-document, tag, syntax, aliases; TOML duplicate, garbage, nan/inf; JSON5 comments, trailing commas, invalid, trailing, non-finite; JSONL blank lines, malformed line number, two values per line; CSV strict quoting, ragged, quoted delimiter and newline; TSV quoting; inexact integer in every numeric format) | the `loads!`/`rejects!` tables, e.g. `json_duplicate_keys_keep_the_last`, `json_reports_the_character_column_after_a_multibyte_character` |
| R3 BOM, CRLF, invalid UTF-8, empty input, per format family | `*_bom_is_stripped`, `*_crlf_loads`, `*_invalid_utf8_reports_its_byte`, `empty_*` |
| R3 loaded values stay data; `kind`/`$schema` read nothing else | `loaded_text_stays_data`, `a_kind_mapping_selects_no_schema_and_reads_nothing_else` |
| R4 object rows, tuple rows, header rules, header-only, cell-conversion table | `object_rows_convert_a_numeric_port`, `tuple_rows_skip_the_header`, `an_object_header_must_not_repeat_a_name`, `an_object_header_must_name_every_column`, `list_rows_allow_repeated_and_empty_headers`, `a_header_only_file_has_no_rows`, `number_cells_*`, `boolean_cells_*`, `string_cells_are_kept_verbatim`, `an_empty_boolean_cell_is_a_mismatch_with_its_table` |
| R4 passive row shape (conflicting arms, scalar/unknown/object elements, URL path, other formats skip, import-decided provisional) | `load_data_passive::tabular::*` |
| R5 field validators (absent/null/optional slots, conversion, mixed/all-invalid arrays, wrong whole field, tuple slot, dictionary, empty values, no partial value) | `a_required_member_*`, `an_optional_member_stays_absent_or_null`, `compatible_scalar_text_converts`, `a_mixed_array_fails_at_the_bad_element_only`, `an_all_invalid_array_fails_at_every_element`, `a_whole_field_of_the_wrong_type_fails`, `a_missing_tuple_slot_is_missing`, `a_dictionary_value_converts_or_fails_by_key`, `empty_values_remain_explicit`, `a_contract_mismatch_publishes_no_value` |
| R6 no remote runtime; allowed URL with query/fragment, one fetch | `a_url_without_a_remote_runtime_is_not_fetched`, `an_allowed_url_with_a_query_and_fragment_loads_as_json_once` (wiremock) |
| R7 reuse, modes, re-read after a command, repeated failure | `a_map_over_three_items_reads_its_data_once`, `a_map_callback_and_the_body_share_one_read`, `two_contracts_read_once_and_bind_twice`, `one_relative_import_in_two_folders_validates_separately`, `{read_write,read_only,refresh,off}_mode_reads_once`, `a_frontmatter_command_that_rewrites_the_data_is_read_again`, `a_repeated_failure_fails_at_each_site` |
| R8 read set with the contract import; failure after resolving | `the_read_set_holds_the_data_file_and_its_contract_import`, `a_load_that_fails_after_resolving_is_recorded` |
| R9 guard pattern, `\|\|` does not absorb | `the_guard_yields_an_empty_list_for_a_missing_file`, `the_guard_yields_the_data_of_a_present_file`, `the_guard_does_not_hide_a_malformed_file`, `or_does_not_absorb_a_load_failure` |
| Static rejection reads nothing and runs no earlier expression (a `DataRuntime` read-count hook) | `load_data_passive::composition::the_precheck_rejects_before_anything_runs` |
| R10 typing, nullability, references unknown, computed unknown, callback propagation, no member warning | `load_data_passive::typing::*` |
| Catalog parity and registry | `function_binding_registry` (new `load_data` row), `function_catalog_parity` (alias), `catalog::tests::every_executable_example_evaluates_to_its_declared_result`, `narrative_doc_function_table_matches_catalog` |
| DMLS shows the data code at `as` | darkmatter-lsp `diagnostics::call_check::tests::a_scalar_load_data_contract_shows_its_data_code_at_the_as_argument` |

Input robustness: the plan's document-loader matrix is covered once per
distinct loader. YAML/YML and JSONL/NDJSON share loaders and get only
extension wiring controls. CSV and TSV share one loader, plus one tab-specific
quoting case. The field validators are covered once through the shared
coercion engine. A smell check of the new parser code found no `.ok()` or
`unwrap_or_default()` on a load-bearing parse.

### Gates (macOS, no subagents running)

- darkmatter `just lint`: exit 0. `cargo check --all-targets --features
  effects-instrumentation`: no warnings.
- darkmatter `just test`: the final run passed 14,648 tests with 10 skipped
  (112 s). The earlier runs failed only on the hand-kept copies listed above.
  No new test takes 1 s or more; the slowest `load_data_*` test took 0.14 s.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Scope calculator:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, darkmatter, darkmatter-cli, darkmatter-lsp,
  test-toolkit, claudine-cli, simplified-schema]`, `change_class: package`.
  The gates were required, and they ran. This tree still includes the
  uncommitted Phase 4 work.
- Dependents:
  - claudine, filtered `/lifecycle|schema|catalog|function|expression|verb|action|golden/`:
    2,390 passed. The first run caught the parity failure described above.
  - simplified-schema `just test`: 1,547 and 1,359 passed. The calculator
    selects it.
  - `cargo check --tests` on claudine, claudine-cli, and research: clean.
  - darkmatter-cli `claudine` filter: 16 passed.
- Cross-OS: the new code reads files, compares canonical paths, and one test
  runs `git` from a frontmatter command.
  `./scripts/cross-check.sh darkmatter --os windows load_data function_binding_registry catalog::tests lifecycle_schema`:
  356 passed on build-win-native.
- Process notes:
  - The two subagents started in the background instead of the foreground.
    The orchestrator waited for both before running any gate.
  - The data-execution subagent ran its own filter once with
    `--no-fail-fast`. The run was limited to its own tests.
- No pre-existing failures, and no load-sensitive reruns.
- Docs: only the generated function table changed. The `load_data` topic
  documentation is Phase 8's (R12), and diagnostic projection is Phase 6's.
