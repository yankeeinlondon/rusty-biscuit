---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-04-loading-data/spec.md
plan: darkmatter/features/2026-11-04-loading-data/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
message_to_agent: |-
    Phase 1 was a baseline audit only; no code changed. Facts for Phase 2 onward:
    (1) The working tree already holds uncommitted edits from the
    2026-11-06-dotted-paths review cycle (simplified-schema coerce.rs,
    validate.rs, resolve.rs, setter_value.rs; darkmatter schemas/, compose/,
    cli commands; claudine schema classify). They are not this feature's work:
    do not revert them, and expect `affected_scope.py` over `git status` to
    report packages even when your phase touched none. simplified-schema
    `coerce.rs` (partial_copy at ~599, failing_paths ~657, flatten_failures
    ~796) is among those edited files, so read its current state before
    Phase 2's rule 20/21 work.
    (2) Rule 17's line citations are stale: commands/compose.rs builds
    `check_request` at line ~355 and `compose_request` at ~458 from one
    `prepare_request` closure (~332).
    (3) `detect_from_extension` is still private in biscuit-file/lib/src/detect.rs:155.
    (4) The 17 output schemas each declare their own closed `diagnostic`
    type; clean.yaml additionally has `clean-diagnostic`. No `data_location`
    or `read_set` exists anywhere yet.
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
