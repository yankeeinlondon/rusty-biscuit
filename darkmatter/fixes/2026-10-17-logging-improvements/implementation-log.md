---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/fixes/2026-10-17-logging-improvements/spec.md"
plan: "darkmatter/fixes/2026-10-17-logging-improvements/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/dmls/src/clock.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/src/log_capture.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/graph/substrate.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/lib/tests/l1/trigger_envelope_claim.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/fixtures/triggers/feature-review-incident.yaml
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
human_review: false
message_to_agent: |-
    Phase 1 is complete. Facts later phases depend on:
    - Test seams live in DMLS: `dmls::work_counters::{work_counts, WorkCounts::since}` (reads, content hashes, Markdown document parses; every production DMLS file read/hash goes through `work_counters::read`/`read_to_string`/`hash_bytes`, and `index_document_timed` calls `record_parse`). Route the Phase 3 registry's reads and hashes through these helpers so the no-I/O test can assert a zero delta. In-memory cache-key hashes (e.g. `xx_hash_bytes(format!("{registry:?}"))` in `overlay/mod.rs`) are intentionally not counted.
    - `dmls::clock::{Clock, SystemClock, ManualClock}` for R9 timing.
    - `dmls::log_capture::CapturedLogs`: `scoped(|| ..)` captures only the calling thread; use `CapturedLogs::global()` (once per process; nextest is process-per-test) when events come from dispatch or worker threads. `tracing_test::traced_test` cannot see `dmls` events from the `l1` integration binary.
    - The R1 claim rule only accepts a plain `kind` key at column 0. A quoted key (`"kind": trigger-schema`) or any JSON-style flow mapping that fails to parse is unclaimed, per the spec. Claudine's robustness matrix writes quoted keys, so its duplicate/trailing trigger cells now expect "unclassified" (`unclaimed_trigger` in `claudine/lib/tests/l1/claudine_schemas_matrix.rs`).
    - `SchemaError::TriggerEnvelopeYaml { message, line, column }`: column counts characters of the source with a leading BOM removed. DMLS converts it in `diagnostics/frontmatter.rs::trigger_yaml_span`.
    - Docs (definition.md claim rule and R2 error) were left for Phase 6 as planned.
---

# Implementation Log for 2026-10-17-logging-improvements (6 phases)

## Phase 1

### What changed

- **R1 claim fallback** (`lib/src/markdown/schemas/triggers/envelope.rs`).
  `parse_trigger_envelope_from_str` strips an initial BOM and turns CRLF into
  LF, then parses. Parseable source takes exactly today's path. Unparseable
  source runs `claims_envelope`, a line-oriented scanner (ruling 3) that
  tracks multi-line single/double-quoted scalars (with `\"` and `''`), block
  scalars (`|`, `>-`, `|2+`, including a root `--- |`), and open flow
  collections. A column-0 `kind[ ]*: trigger-schema` line (plain, single- or
  double-quoted, optional `  # comment`) claims only when it is provably
  top-level. Otherwise the result is `Ok(None)`.
- **Departure (normalization before parsing, not only before the claim
  check).** The spec says to normalize CRLF/BOM "before checking". Probing
  showed libyaml rejects a valid envelope written with a BOM *and* CRLF
  ("more than one document"), so the matrix row "CRLF + BOM gives the same
  result as LF without BOM" could only hold by normalizing before the parse.
  This changes only sources that failed to parse before. Reported columns on
  line 1 then exclude the BOM, and DMLS accounts for that.
- **R2 variant** (`lib/src/markdown/schemas/errors.rs`):
  `SchemaError::TriggerEnvelopeYaml { message, line, column }`. The message
  has the parser's ` at line L column C` suffix removed. The column counts
  characters (verified: serde_yaml_ng's `index` is bytes, its `column` is
  chars). `TriggerLoad` and `EmbeddedTriggerLoad` now display
  "trigger-schema file `…` is not valid YAML at line 17, column 93: …" for
  this source, and "is malformed: …" for anything else.
  - Their status blocks switch header and hint to a YAML hint ("…must be
    quoted…").
  - `TriggerMatch` is unchanged.
- **CLI help.** The CLI renders library status blocks. Compose wraps the scan
  error in `MarkdownError::SchemaValidationFailed`, whose generic hint ("Check
  that the document's $schema…") hid the YAML help. Added
  `SchemaError::preparation_hint` (crate-private), and
  `schema_validation_failed_block` takes it, so `md compose` shows the YAML
  hint. Every CLI error exits 1; the CLI has no per-variant exit codes, so
  none were added.
- **DMLS authoring position** (`dmls/src/diagnostics/frontmatter.rs`,
  `trigger_yaml_span`). The trigger diagnostic spans from the reported
  line/char column to the end of that line, through the source map, so it
  lands in the negotiated encoding. With no location it keeps today's
  whole-file range. The `overlay/mod.rs` "ordinary schema-root YAML remains
  silent" comment is accurate again and was left in place.
- **Test seams (DMLS only, no behavior change).**
  - `dmls::work_counters` (reads / content hashes / document parses), wired
    into every production DMLS file read and content hash: overlay
    `hash_files`/`deps_unchanged`, startup `read_and_index`, watch
    `scan_manifests`, the named-type reads in `overlay/named_types.rs` and
    `diagnostics/named_types.rs`, and `index_document_timed`, which counts the
    parse and its hash. In-memory cache-key hashes are deliberately not
    counted.
  - `dmls::clock::{Clock, SystemClock, ManualClock}`.
  - `dmls::log_capture::CapturedLogs` (`scoped` = calling thread only,
    `global` = process-wide default, once per process).
  - The test toolkit has no capture or clock helper, only `init_test_tracing`.
    DMLS unit tests already use `tracing_test::traced_test`, which filters to
    the crate under test and so cannot see `dmls` events from the `l1`
    integration binary.
- **Existing test updated with the rule.** In
  `schema_embedded_trigger_bundles::the_trigger_envelope_matrix_holds_for_json_syntax`,
  six JSON-syntax cells (duplicate or trailing `kind`/`match`/`$schema`)
  expected `Error`. These documents are unparseable and have no column-0
  claim line, so under the spec's stated limitation ("malformed flow
  mappings … remain unclaimed") they are now `Unclassified`. The
  block-YAML twin of that matrix is unchanged; its claim line is present.

### Requirement-to-test mapping

| Requirement / matrix row | Test |
| --- | --- |
| R1 table: claimed (plain, quoted, comment, `kind :`, tab) | `lib l1 trigger_envelope_claim::claim_lines_in_unparseable_source_are_yaml_errors` |
| R1 table: unclaimed (no kind, `kind: schema`, indented, nested, suffix, comment-out, no space, multi-line quoted, escaped quote, block scalar, open flow, root block scalar, malformed flow mapping) | `…::unclaimed_unparseable_source_is_not_a_trigger` |
| claim after a closed scalar still claims | `…::a_claim_after_a_closed_scalar_still_claims` |
| CRLF and BOM same as LF without BOM | `…::crlf_and_a_bom_classify_like_lf_without_a_bom` |
| duplicate `kind` (parser rejects; claim rule decides) | `…::a_duplicate_kind_key_is_decided_by_the_claim_rule` |
| trailing invalid content | `…::trailing_invalid_content_is_decided_by_the_claim_rule` |
| second YAML document | `…::a_second_document_is_decided_by_the_claim_rule` |
| quoted-key and JSON envelopes unchanged | `…::parseable_quoted_key_and_json_envelopes_keep_their_behavior` |
| `kind` field shapes (absent, null, `kind:`, 123, `""`, list) | `…::parseable_kind_shapes_other_than_the_claim_are_unclaimed` |
| control row | `…::the_control_envelope_loads` |
| incident fixture is not a trigger (error really is at 17:93) | `…::the_incident_file_is_not_a_trigger` |
| R2 message and help name YAML, `TriggerLoad`/`Embedded` text | `…::the_yaml_error_names_yaml_not_the_match_grammar` |
| grammar errors keep grammar help | `…::a_match_grammar_failure_keeps_its_grammar_hint` |
| column is in characters | `…::the_reported_column_counts_characters` |
| discovery: valid trigger beside unparseable unclaimed file loads | `lib l1 schema_roots::an_unparseable_unclaimed_schema_root_file_does_not_fail_discovery` |
| discovery: claimed unparseable is `TriggerLoad` + location | `lib l1 schema_roots::an_unparseable_claimed_schema_root_file_fails_discovery_with_its_yaml_location` |
| embedded wrappers | `lib l1 schema_embedded_trigger_bundles::an_unparseable_embedded_document_fails_only_when_it_claims_the_envelope` (+ updated JSON matrix) |
| corpus: every shipped schema classifies without error | `lib l1 shipped_schema_corpus::shipped_schema_artifacts_parse_passively` (new per-artifact assertion) |
| CLI e2e: incident file, `md compose` ok, `md schema triggers` exit 0, trigger still applies | `cli l1 schema_triggers::an_unparseable_unclaimed_schema_root_file_leaves_compose_and_triggers_working` |
| CLI e2e: claimed broken file, non-zero, YAML message and help | `cli l1 schema_triggers::an_unparseable_claimed_schema_root_file_reports_its_yaml_error` |
| DMLS: diagnostic at reported position, UTF-16 vs UTF-8 (emoji before col 93) | `dmls l1 lsp_session::a_claimed_unparseable_trigger_envelope_is_diagnosed_at_its_yaml_position` |
| DMLS: unclaimed unparseable gets no trigger diagnostic | `dmls l1 lsp_session::an_unclaimed_unparseable_schema_root_file_gets_no_trigger_diagnostic` |
| seams | unit tests in `dmls/src/{clock,work_counters,log_capture}.rs` |

The incident fixture is `darkmatter/lib/tests/fixtures/triggers/feature-review-incident.yaml`:
today's `schemas/feature-review.yaml` with the line-17 quotes removed. The
CLI fixture writes `.git` directly (the existing `initialize_repository`)
instead of spawning `git init`, per the anti-pattern list.

### Smell grep

`rg 'serde\(default|unwrap_or_default\(\)|\.ok\(\)|filter_map\(.*as_str' lib/src/markdown/schemas/triggers/envelope.rs`
gives no hits. The one deliberate `Err(_)` arm (`Err(_) if !claims_envelope(..)`) is the
R1 rule itself, not a parse failure silently read as absent.

### Gates and downstream tests

- `just lint` (darkmatter): clean.
- `just test` (darkmatter): 10153 passed, 10 skipped, 0 failed (75 s).
- `just check-tier-coverage darkmatter`: 0 stranded.
- Claudine (consumes the trigger classifier and `EmbeddedTriggerLoad`):
  `cargo nextest run -p claudine -E 'test(/schema|trigger|embedded|compos/)'`
  gives 2281 passed.
  - Before the matrix update this selection failed 10 + 6 matrix cells.
    Claudine's matrix writer quotes every key (`"kind":`), so its duplicate
    and trailing trigger-file cells are now unparseable and unclaimed per the
    R1 rule (nonconventional key spelling), and drop the trigger instead of
    erroring.
  - Updated with one documented exception, `unclaimed_trigger`. The shipped
    `claudine.yaml` uses a plain `kind:` line, so a real mid-edit break of
    it is still claimed and still fails visibly.
  - `cargo clippy -p claudine --tests -- -D warnings`: clean.
- claudine-cli `-E 'test(/schema|trigger|compos|signal/)'`: 1000 passed, 1
  failed. `codex_app_server::composition_yolo_frontmatter_disables_the_codex_sandbox`
  panics at fixture setup copying the `claudine-fake-codex` binary (`NotFound`),
  which a `-p claudine-cli` nextest build does not produce. This is
  environmental, unrelated to this change, and was not investigated further.
- messenger (uses `DarkmatterSchemas` for research validation):
  `-E 'binary(/research/) | test(/research/)'` gives 15 passed.
- Cross-OS check not run: the change has no path or OS-specific logic, and
  CRLF/BOM handling is covered in-process.
