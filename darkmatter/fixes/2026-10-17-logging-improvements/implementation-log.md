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
    Phase 4 (R5/R6 file list and lazy parsing) is complete. Facts Phase 5 depends on:
    - Startup no longer parses Markdown. `router::spawn_file_listing` walks the workspace on a background thread and sends `Internal::FileList` to the dispatch thread (the router loop is a `crossbeam_channel::select!` over the client and internal channels; `handle_internal` installs results). Add Phase 5's background results (registry ready, full index built) as new `Internal` variants so every index mutation stays on the dispatch thread. The walk currently starts right after the handshake; the plan wants it scheduled after registry preparation, so spawn it after your new startup registry step.
    - Interim full index: `ServerState::ensure_full_index` is synchronous on the dispatch thread and is called by `workspace_symbol`, `references`, `rename`, and `will_rename_files` (not `prepare_rename`). Replace its body with the R7 state machine. Keep the entry points and the `WorkspaceIndex` interface (`unparsed_paths`, `adopt_full`, `is_full`; `open_buffer`/`close_buffer`; `disk_changed`/`disk_deleted`; `install_listing`). Read failures are skipped today (`startup::read_and_index`); R7 needs them to fail the build. `adopt_full` already skips open buffers and unlisted paths.
    - Log lines already emitted (Phase 5 owns their tests and the `workspace` field, ruling 5): `workspace files listed markdown yaml duplicates elapsed_ms` (in `Router::install_internal`) and `workspace rescan complete` with the same fields (in `ServerState::relist`, which also runs on config and ignore-file changes). `startup index complete` is gone. Nothing logs `schema registry ready` or `full index built` yet.
    - Graph model: every listed Markdown file is a `WorkspaceGraph` document. Unparsed ones have `DocumentRecord::parsed == false`, `content_hash` 0, and a root node only, so the graph's document count is the listed count, not the parsed count. Use `WorkspaceIndex::len()` (parsed) or `listed().len()` for R8 counts. `WorkspaceGraph::listing()` is `Pending` until the list lands.
    - Tests: `LspFixture::initialize` waits for `StartupEvent::FileListInstalled` (through `RunOptions::with_startup_hooks`); `start_with_file_list_gate` with `initialize_unlisted` holds the walk. `flush_server` (`workspace/symbol`) now builds the full index, so do not use it as a barrier in a parse-counting test; `workspace_file_list.rs` uses a `documentSymbol` round trip (`latest`). The R7 build-once tests should count `work_counters` parses.
    - Progress: the startup token `dmls/startup-index` now covers only the walk, and its `end` is sent after the list is installed. The full-index build reports no progress yet (R7 asks for it).
source_files_during_phase_2:
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/logging_tests.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/clock.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/main.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/logging_tests.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - darkmatter/dmls/src/workspace/discover.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/src/workspace/mod.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/graph/mod.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/capabilities.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/completion.rs
    - darkmatter/dmls/src/providers/wiki.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/semantic_tokens.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
docs_updated_during_phase_4:
    - darkmatter/docs/lsp/architecture.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/dmls.md
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

## Phase 2

### What changed

- **New module `dmls/src/request_timing.rs`.**
  - `SLOW_THRESHOLD` (500 ms, a constant).
  - `RequestTimings` records the elapsed time of each `Stage` (`OverlayBuild`,
    `SchemaAssembly`, `TriggerRegistry`, `ProviderChain`) and the
    `Wait::{FileList, FullIndex}` flags. It reads an injected `Clock`.
  - `CompletionEvent::emit` writes the one completion event:
    `slow request` / `request complete`, or `slow notification` /
    `notification complete`.
  - Event fields: `method`, `uri` (omitted when the message names no
    document), `id` (requests only), `outcome` (`ok` / `error` / `panic` /
    `cancelled`), `cancelled=true` (cancelled only), `elapsed_ms`,
    `<stage>_ms` (omitted for a stage that did not run),
    `waited_for_file_list`, and `waited_for_full_index`.
  - The module docs record the nesting: overlay build ⊇ schema assembly ⊇
    trigger registry; the provider chain is a sibling. A stage that runs more
    than once in a message reports the sum of its runs.
- **Router (`dmls/src/router.rs`).**
  - `run()` starts a fresh `RequestTimings` on `ServerState` at every request
    or notification dequeue. `shutdown` and `exit` are not timed: the loop
    returns on them.
  - `handle_request` emits the completion event after the response is sent,
    for every path: a cancel from the ledger, an `Ok` response (an error
    response is `outcome="error"`, and a `RequestCanceled` code is
    `cancelled`), or a caught panic.
  - `handle_notification` emits after its handler returns, or after a panic
    is caught.
  - `with_document` times the overlay build and the provider call (`f`).
    `workspace/symbol`, `prepareRename`, `rename`, and `willRenameFiles`
    time their provider calls directly.
  - A `#[cfg(test)]` method, `logging_tests::PANIC_METHOD`, panics so the
    panic boundary can be tested.
- **Overlay.** `OverlayState::for_document_timed` takes the record.
  `for_document` keeps its signature and passes an `untimed()` record, so
  about 50 test call sites are unchanged. `schema_for` and
  `standalone_named_types` are timed as schema assembly, and each
  `trigger_registry` call is timed as the trigger registry.
- **Clock injection.** `RunOptions` gains `clock: Arc<dyn Clock>` (default
  `SystemClock`) and `with_clock`. `Clock` gains a `Debug` supertrait so
  `RunOptions` keeps `#[derive(Debug)]`. `main.rs` builds its options with
  `..RunOptions::new(snapshot)`.
- **Design note (ruling 2).** The record is a per-message value that lives on
  `ServerState` and is replaced at each dequeue. It is passed explicitly into
  the overlay build (`for_document_timed`), not through a global span. Keeping
  it on `ServerState` avoids adding a parameter to every handler and to
  `refresh_all_diagnostics`, which notification handlers reach through
  `reload_config` and `rescan_workspace`.
- **No waits yet.** Nothing in DMLS waits for the file list or full index
  until Phases 4 and 5, so both flags are always `false` today. Those phases
  must call `timings.record_wait(Wait::FileList | Wait::FullIndex)` where the
  request blocks.

### Finding: `lsp_server` debug trace logs full payloads (pre-existing)

At `--log-level debug`, the `lsp_server` crate's own transport trace
(`lsp_server::msg`, `lsp_server::stdio`, through the `log` bridge) prints
every protocol message in full, including `didOpen` text. This behavior was
already there; it is not DMLS's code. The spec says log filtering stays
unchanged, so I did not filter it. DMLS's and Darkmatter's own records
contain no source text, which the subprocess test asserts by scoping to
`dmls*`/`darkmatter*` records. If this should change (for example, a default
`lsp_server=info` directive), that is a separate decision.

### Requirement-to-test mapping (R9)

| Requirement | Test |
| --- | --- |
| below 500 ms → one `debug` `request complete` | `dmls request_timing::tests::below_the_threshold_is_one_debug_request_complete` (499 ms, `ManualClock`) |
| exactly 500 ms → `warn` `slow request` | `…::exactly_at_the_threshold_is_one_warn_slow_request` |
| above → `warn` with method/uri/id/outcome/elapsed/wait flags; absent stages omitted | `…::above_the_threshold_is_one_warn_slow_request_with_every_field` |
| URI-less requests omit `uri` | `…::a_request_without_a_document_omits_uri`; dispatch: `router::logging_tests::a_workspace_symbol_request_omits_uri_and_overlay_stages` |
| stage figures nested, not summed; wait flag | `…::stages_are_reported_separately_and_nested_not_summed` |
| repeated stage sums | `…::a_stage_that_runs_twice_reports_its_sum` |
| cancelled → elapsed + `cancelled=true` | `…::a_cancelled_request_logs_elapsed_and_cancelled_true`; dispatch: `router::logging_tests::a_cancelled_request_emits_one_cancelled_completion` |
| notifications: own text, no id, both levels | `…::notifications_use_their_own_text_and_carry_no_request_id`; dispatch: `router::logging_tests::notifications_log_their_own_completion_without_response_timing` |
| wiring: fast hover → one debug event with all four stages | `router::logging_tests::a_fast_hover_logs_one_debug_request_complete_with_its_stages` |
| wiring: slow hover → one warn event, every stage moved | `router::logging_tests::a_slow_hover_logs_one_warn_slow_request` |
| provider error (MethodNotFound, InvalidParams) → one `error` event | `router::logging_tests::error_responses_emit_one_error_completion` |
| caught panic → one `panic` event, dispatch continues | `router::logging_tests::a_caught_panic_emits_one_panic_completion_and_dispatch_continues` |
| no source text or frontmatter values (every level, in-process) | `router::logging_tests::no_source_text_or_frontmatter_value_is_logged` |
| logs never on stdout; strict framing; stderr routing | `dmls l1 stdio_subprocess::request_logs_go_to_stderr_and_never_into_the_stdout_protocol_stream` |
| `--log-file` routing unchanged | `dmls l1 stdio_subprocess::a_log_file_receives_request_logs_instead_of_stderr` |
| `textDocument.uri` extraction | `request_timing::tests::document_uri_reads_only_the_text_document_uri` |

Dispatch-level tests run `run_server` over `Connection::memory()` with a
clock that moves a fixed step on every read. The server thread runs under
`tracing::dispatcher::with_default`, so no global subscriber is needed.
`initialized` never reaches dispatch, because `lsp_server`'s handshake
consumes it, so the no-URI notification case uses `$/cancelRequest`.

The input robustness matrix does not apply: this phase reads no file
format or configuration.

### Gates and downstream tests

- Targeted: `cargo nextest run -p dmls -E 'test(/request_timing|logging_tests/)'`
  gives 17 passed; `-E 'test(/stdio_subprocess/)'` gives 6 passed.
- `just lint` (darkmatter): clean.
- `just test` (darkmatter): 10172 passed, 10 skipped, 0 failed (33 s).
- `just check-tier-coverage darkmatter`: 0 stranded.
- Downstream: no package outside `darkmatter/` depends on `dmls`. The
  changed public items (`RunOptions.clock`, the `Clock: Debug` supertrait,
  `OverlayState::for_document_timed`, the `request_timing` module) are used
  only inside the area, which `just test` covers.
- Cross-OS check not run: the change has no path or OS-specific logic. The
  subprocess tests build URIs with `url::Url::from_{file,directory}_path`
  and read both streams to EOF.
- `claudine/cli/tests/l1/tts_prewarm.rs` was already modified in the working
  tree before this phase started. It is not part of this phase.

## Phase 3

### What changed

- **Library seam (`lib/src/markdown/schemas/triggers/discovery.rs`).**
  - `scan_with_source(context, bundles, &mut dyn SchemaRootSource)` is the
    scan body. `scan_with_embedded` (and so `scan`) calls it with a
    filesystem source, so `md` behaves exactly as before.
  - `SchemaRootSource` supplies each root's file list and each root file's
    text. Roots, shadowing, envelope loading, and payload resolution are
    unchanged and stay in the library.
  - `enumerate_root_files` exposes discovery's own listing, which keeps the
    symlink, filename-byte ordering, and case-fold collision rules.
  - `TriggerRegistry::dependencies()` lists each file trigger's envelope
    and each payload file with its imports and examples.
- **New `dmls/src/overlay/registry.rs` (ruling 1).**
  - *Root contents* are shared across documents and keyed by normalized
    path: each root folder's listing, and each root file's text, xxHash, and
    `RootFileClass` (named / trigger / failed trigger / unclassified).
  - When a folder is listed, every file in it is classified, including
    shadowed files and files after a failing trigger, so the summary
    accounts for every enumerated file.
  - *Document registries* are keyed by document path. Each is
    `scan_with_source` over the root contents in the document's own
    `for_source` context. It therefore equals `triggers::scan` by
    construction, and the tests assert `==` on the whole `TriggerRegistry`
    (payloads, claimed names, and context included).
  - A lookup is fresh when the scope is not stale and its stored context
    `==` the new one. A fresh lookup does no I/O.
  - Each preparation records:
    - the searched and absent root folders, with their roles;
    - every root file's hash;
    - each dependency outside the roots, read and hashed once;
    - the folders that hold dependencies.
  - `invalidate_paths` re-reads a changed known file and compares its hash.
    Creations and deletions in recorded folders invalidate without a hash.
    A failed scope is invalidated by any change, so creating a missing
    payload recovers it.
  - `invalidate_all` (rescan, config, manifest) clears the listings and
    marks every root file *unverified*: the next read compares the hash, and
    an unchanged file keeps its classification.
  - Installation is one map insert after the scan finishes. Every install
    gets a new `version`.
  - `summary()` returns the R8 counts (`roots`, `tree_schemas`,
    `scoped_schemas`, `trigger_schemas`, `failed_triggers`,
    `unclassified`, `failed_scopes`). It is exposed as
    `OverlayState::trigger_registry_summary`, but **Phase 5 must log it**:
    nothing logs it yet.
- **R4 (one source of truth).** `record_failure` keeps one record per
  failure scope (workspace boundary, document folder), as before. The same
  `is_new` comparison decides both the diagnostic transition and the log
  level:
  - `warn` for a new failure, `debug` for a repeat;
  - `info` `trigger registry recovered` on success after a recorded failure;
  - `trigger registry unavailable` when there is no last-good registry,
    including root I/O failures, which never serve last-good data.
  - Fields: `scope` (the document folder), `trigger_file`, `error`, and
    `has_last_good` on the retaining line.
- **Overlay (`overlay/mod.rs`).**
  - `trigger_registry` is now a lookup. The `Stage::TriggerRegistry`
    wrappers are kept.
  - The schema cache key uses the registry `version`. It used to use an
    xxHash of the registry's whole `Debug` text on every request.
  - The registry's envelopes and payload files are removed from
    `CachedSchema.deps`, because the registry tracks them. Before, every
    request re-read and re-hashed them. The document's own `$schema`
    imports, named types, and extensions keep their read-on-lookup freshness
    (Review Decisions).
  - New `invalidate_trigger_inputs` and `invalidate_all_trigger_registries`.
- **Router.**
  - `didChangeWatchedFiles` passes its paths to `invalidate_trigger_inputs`,
    or calls `invalidate_all` when the change is a context input (manifest
    or `.git`).
  - A `didSave` of any `.yaml`/`.yml` invalidates that path, whether the
    buffer is open or not, and refreshes diagnostics for `ClientWatched`
    clients. `ServerRescan` clients already refresh after their rescan.
  - `rescan_workspace` and `reload_config` call `invalidate_all`.
  - Doc comments were updated to match.
- **Work counters.** `enumerations` and `registry_scans` were added.
  Payload resolution happens only inside a scan, so `registry_scans == 0`
  shows that no payload was resolved.
- **Late contexts.** A document in a folder that has not been prepared
  yet, including an untitled buffer's synthetic path, misses the cache and
  is prepared before its overlay is built. The workspace-boundary gate
  (`trigger_boundary`) is unchanged: a document outside every workspace
  folder still has no registry and never gets the workspace's roots.

### Departures and decisions

- **Watch globs.** `**/schemas/*.yaml|yml` were already registered for
  `ClientWatched` clients, so schema-root edits keep reaching the cache.
  The wider YAML globs and dependency-location watches (ruling 4) are
  Phase 4 work, as planned. Until then, an external (non-editor) edit to a
  payload *outside* any `schemas/` folder reaches a `ClientWatched` client
  only through a save or a rescan.
- **Context generation is not part of the registry key.**
  `RepositoryContexts::invalidate` drops the repository's context for *any*
  watched path inside it. Keying on the generation would rescan on every
  edit and discard last-good registries. Freshness is decided by context
  equality instead, and manifest changes invalidate everything.
- **Last-good scope.** Last-good is kept per document registry across
  context changes, as before (the old key was boundary plus folder). Root
  I/O failures keep the installed registry but never serve it.
- **Existing tests.** Three overlay tests edited schema files on disk
  without an event. Under the cache those edits are invisible, so the
  tests would have passed without testing anything. Each now calls
  `invalidate_trigger_inputs`, which is the new contract. The
  `strict_mode_recovery_spike` test showed that a deleted root folder broke
  path matching (`/var` vs `/private/var`). `normalized` now canonicalizes
  the nearest existing ancestor.

### Requirement-to-test mapping (R3, R4)

| Requirement | Test |
| --- | --- |
| registry equals `triggers::scan` for package, package-area, tree-root documents with `SCHEMAS_DIR` and a fixture home | `dmls overlay::registry::tests::per_document_registries_equal_direct_discovery` |
| unclassified file shadows a later trigger | same test (package's ordinary `tree.trigger.yaml`) |
| repeated ready lookups do no listing/read/hash/scan | `…::repeated_ready_lookups_do_no_io` (all five counters zero) |
| through the overlay: no registry work; other schema assembly keeps freshness | `dmls overlay::tests::repeated_overlay_builds_skip_the_registry_but_keep_schema_freshness` (3 builds → reads 3 = the `$schema` file only; unevented edit still re-assembles) |
| root contents shared across documents | `…::root_contents_are_shared_across_document_registries` |
| watched event: envelope, payload outside every root, new root file | `…::watched_changes_to_an_envelope_an_external_payload_and_a_new_root_file_update_the_registry` |
| unchanged xxHash does not reclassify/rescan | `…::an_event_for_an_unchanged_file_does_not_rescan` |
| deletion; missing payload creation recovers; cold failure is unavailable (`Ok(None)`) | `…::deleting_a_trigger_removes_it_and_creating_a_missing_payload_recovers` |
| external home and `SCHEMAS_DIR` roots fail and recover | `…::external_home_and_environment_roots_recover` |
| non-YAML (`.json`) example dependency | `…::a_non_yaml_example_change_reloads_its_trigger` |
| last-good only where unshadowed | `…::last_good_is_retained_only_where_the_malformed_file_is_unshadowed` |
| root I/O fails closed (non-UTF-8 root file: cross-OS, no chmod), healthy scope still loads, recovery | `…::a_root_io_error_fails_closed_and_recovers` |
| scoped contexts keep distinct payload and `$path` resolution | `…::scoped_contexts_keep_distinct_payload_and_pattern_resolution` |
| R4: one `warn`, a different error a second, repeats at `debug`, one `info` recovery; no re-published diagnostic | `…::repeated_identical_failures_warn_once_and_recovery_logs_info` |
| R4: cold failure → `trigger registry unavailable` once at `warn`, then `debug` | `…::a_cold_failure_logs_unavailable_once` |
| classification and counting rules | `…::root_files_are_classified_and_counted_once_per_folder` |
| saving an open YAML buffer invalidates; unsaved edits do not (protocol, `ClientWatched`) | `dmls::l1 lsp_session::saving_an_open_trigger_buffer_updates_consumers_and_unsaved_edits_do_not` (mutation-checked: fails with the `didSave` hook removed) |
| recovery clears old diagnostics, refreshes affected documents | existing `lsp_session::{client_watcher,server_rescan}_publishes_and_clears_unopened_trigger_envelope_diagnostic`, `repairing_trigger_payload_clears_open_envelope_immediately`, `strict_mode_recovery_spike::…` (all pass on the cache) |
| incident reproduction (checkpoint 3): an unrelated mid-edit file (the real incident fixture) logs nothing; a broken claimed envelope warns once across 5 hovers | `dmls router::logging_tests::a_mid_edit_unrelated_schema_file_logs_nothing_and_a_broken_trigger_warns_once` |

The input robustness matrix does not apply. This phase adds no parser; it
caches the existing classifier, whose matrix Phase 1 covered.

### Gates and downstream tests

- Targeted runs: `cargo nextest run -p dmls -E 'test(/overlay::registry|repeated_overlay_builds|trigger/)'`
  gave 36 passed. The new LSP and logging tests pass on their own.
- `just lint` (darkmatter): clean.
- `just test` (darkmatter): 10189 passed, 10 skipped, 0 failed (35 s).
- `just check-tier-coverage darkmatter`: 0 stranded.
- Downstream (consumers of `scan_with_embedded`, now a wrapper):
  - `cargo nextest run -p claudine -E 'test(/schema|trigger|embedded/)'`:
    334 passed.
  - `-E 'test(/compos/)'`: 2103 passed.
  - messenger `-E 'binary(/research/) | test(/research/)'`: 15 passed.
- Cross-OS check: `just cross-check dmls --os windows`. The registry
  normalizes paths through `canonicalize_simplified`, so this phase carries
  OS risk.
  - Attempt 1: 937/938.
    `overlay::registry::tests::repeated_identical_failures_warn_once_and_recovery_logs_info`
    compared a logged path with `repo.join("schemas/tree.trigger.yaml")`
    as text, and that path keeps its literal `/` on Windows. This was a
    test-only bug, already recorded in the `os` skill
    (`windows-paths.md` item 8). The fix joins the path one component at a
    time.
  - Attempt 2: 938/938 passed.
  - `darkmatter` (lib) was not cross-checked: its change is a
    source-abstraction refactor with no path logic, and the `dmls` run
    compiles and exercises it.
- `just lint` was re-run after the fix: clean.

## Phase 4

### What changed

- **File list (`dmls/src/workspace/discover.rs`, R5).**
  - `discover_workspace` lists Markdown (`include`/`exclude`) and YAML
    (`*.yaml`/`*.yml`, `exclude` only) per effective root. It reads no
    contents.
  - The walk sets `require_git(false)`, keeps `hidden(false)`, skips every
    `.git` directory with `filter_entry`, and still does not follow symlinks.
    Duplicate physical files are still reported, and each is logged at `warn`.
  - A walk error is recorded in `DiscoveryReport::errors`. It makes the list
    incomplete and logs `workspace discovery incomplete`.
  - `listed_kind` answers "would the walk list this path?" for one watched
    path. It uses the same filter and checks the same ignore rules one
    directory level at a time.
- **Startup (`router.rs`).**
  - The startup indexer is gone. A background thread lists the workspace and
    hands the result to the dispatch thread as an `Internal::FileList` event.
    The router loop is now a `crossbeam_channel::select!` over the client and
    internal channels, so every index mutation stays on the dispatch thread.
  - On install, it logs `workspace files listed markdown yaml duplicates
    elapsed_ms`, refreshes open documents' diagnostics, and then ends the
    `dmls/startup-index` progress. Ending the progress after the install means
    `wait_for_startup_complete` still means "diagnostics reflect the list".
  - `startup index complete documents=…` is removed (R8).
  - `RunOptions::with_startup_hooks(StartupHooks { file_list_gate, observer })`
    is the test seam. The observer receives `StartupEvent::FileListInstalled`.
- **Lazy parsing (`graph/invalidate.rs`, `graph/arena.rs`, R6).**
  - `WorkspaceIndex` now holds:
    - the Markdown list and the YAML list;
    - the `Listing` state (`Pending`, or `Ready { complete }`);
    - the open-buffer set and the stale set;
    - the `full` flag.
  - Every listed Markdown file is a graph document. An unparsed one has a
    root node only, `DocumentRecord::parsed == false`, and `content_hash` 0.
    So link, wiki-link, and path-completion consumers ask "is it in the file
    list?" with no other change (Wave 9, first task). A fragment link or
    heading wiki link to an unparsed target lands on the document root, and
    its anchor is left unchecked.
  - `prepare_targets(path)` parses, for an open document:
    - the targets of its fragment links and heading wiki links;
    - its `::file`/`::code` closure, followed outward with a visited set.
  - `ensure_parsed`:
    - reads one path and reuses the parse when the xxHash is unchanged;
    - drops the parse on a failed read, so old headings are never served;
    - always lets open buffers win.
  - A watched change marks a parsed, closed document stale. The full index,
    once built, re-reads it at once. A deletion removes it.
  - `close_buffer` restores the disk view. A listed file becomes unparsed,
    or is re-read when the index is full; an unsaved or unlisted document
    leaves the index.
  - YAML never becomes a graph node. Before this phase, a watched
    `schemas/*.yaml` event was indexed as Markdown.
- **Where parsing happens (router).**
  - `refresh_diagnostics` prepares the open document first.
  - Hover, definition, and completion prepare the target under the cursor:
    the link, wiki link, or transclusion node at the offset
    (`providers::reference_target_at`), or the anchor and wiki-heading
    completion target (`providers::completion_heading_target`). The second
    one uses the extracted `completion::heading_target` and
    `wiki::heading_target`, which share their target resolution with the
    completion providers.
  - All parsing runs on the dispatch thread under the index lock. So
    "concurrent requests share one parse" and "a stale worker result cannot
    replace a newer buffer" hold by construction.
- **Gated diagnostics (`router::listing_gates`).**
  - While the list is `Pending`, these are withheld: `dm.links.broken_path`,
    `dm.links.missing_anchor`, `dm.transclusion.broken_path`,
    `dm.transclusion.cycle`, and the three wiki resolution codes.
  - An incomplete list keeps withholding the wiki codes. Wiki resolution is
    decided by the list alone; the other codes are decided by disk.
- **Full index (interim bridge to Phase 5).**
  - `ServerState::ensure_full_index` builds synchronously on the dispatch
    thread: it lists first if the startup list has not arrived, parses every
    unparsed or stale listed file on the worker pool with the index lock
    released, and then `adopt_full`.
  - It runs before references, rename, `willRenameFiles`, and workspace
    symbols. `prepare_rename` does not call it.
  - It records `Wait::FullIndex` and, when it lists, `Wait::FileList`.
  - Once full, the index stays current file by file: watched changes and
    newly listed files are parsed at once.
- **Rescan and config.**
  - `rescan_workspace` (the save fallback) re-lists the workspace and logs
    `workspace rescan complete` with the R5 fields. It then re-reads only the
    parsed, closed documents (`recheck_parsed`) and re-parses those whose
    bytes changed. Unparsed files stay unparsed.
  - A `[workspace]` config change re-lists the workspace without reading
    files, and re-registers the workspace watchers.
  - A watched `.dmls.toml` now reloads the configuration
    (`ConfigState::reload`, through the shared `update_config`). A watched
    `.gitignore` or `.ignore` re-lists the workspace.
- **Watches (`watch.rs`, ruling 4).**
  - The workspace registration now covers `**/*.yaml`, `**/*.yml`,
    `**/.gitignore`, `**/.ignore`, and `**/.dmls.toml`, replacing
    `**/schemas/*.yaml|yml`. It stays non-empty when `include` is empty.
  - New `schema_watch_registration`: for each folder in
    `OverlayState::trigger_watch_folders()`, one watch is rooted at the
    folder's parent with `<folder>/*`. The folders are searched and absent
    roots plus dependency folders, so a root that does not exist yet is still
    watched. The router re-syncs this after every message.
  - New `ClientProfile::supports_relative_watch_patterns`. When it is false,
    or when the client rejects the schema registration,
    `schema_watch_fallback` makes every save mark the registries stale.
  - A rejected workspace registration switches the server to `ServerRescan`.
    Registration responses are tracked in a `WatchLedger`, and
    re-registration unregisters the old id first.
  - No polling was added.
- **Docs and comments.**
  - `lsp/architecture.md`: the resolve table, the paragraph below it, and
    the watch paragraph.
  - `lsp/features.md`, `dmls/docs/file-references.md`, and the
    `missing_anchor` row in `dmls/docs/diagnostics.md` now say "workspace
    document" or "file list" instead of "indexed".
  - Module docs fixed: `workspace/mod.rs`, `workspace/startup.rs`, and the
    `request_timing.rs` mention of the startup index.
  - Skill: `.claude/skills/darkmatter/dmls.md`.
  - The full sweep and the new `indexing.md` page remain Phase 6.

### Departures and decisions

- **The registry is not prepared before the walk.** The plan says the walk
  is "scheduled after registry preparation". No startup registry
  preparation exists yet (Phase 3 left it lazy), so the walk starts right
  after the handshake. Phase 5 adds the preparation step and should spawn
  the walk after it.
- **Interim full index.** References, rename, and workspace symbols must
  stay complete once startup no longer parses everything. Phase 4 therefore
  adds a synchronous `ensure_full_index`. Phase 5 replaces its body with
  the state machine; the entry points and the `WorkspaceIndex`
  `unparsed_paths`/`adopt_full` interface stay. Its read failures are
  skipped, as the old startup indexer skipped them; R7's
  fail-the-build rule is Phase 5 work.
- **Missing `$schema` targets are not gated.** The overlay decides them by
  reading disk through `FileReference` (`dm.schema.prepare`). Absence from
  the list cannot cause a false "missing", and the generic code also
  carries unrelated schema errors, which must publish immediately.
- **The anchor target is parsed once, on demand.** It is not cached as a
  separate structure. The lazily parsed document is stored in the same
  `WorkspaceIndex.documents` map, keyed by path with its xxHash, so a single
  passive parser (`index_document`) serves lazy and full builds, as R7
  requires.
- **Targets outside the workspace are not parsed.** They resolve through
  `FileReference` as before (`EdgeTarget::File`), and their anchors stay
  unchecked. Existing context invalidation still covers them.
- **The `.git/info/exclude` file is not watched.** It sits inside `.git`.
  A change to it takes effect on the next rescan, config change, or
  restart. This is noted in `watch::DISCOVERY_INPUT_NAMES`.
- **The test fixture waits for the list.** `LspFixture::initialize` now
  waits for `FileListInstalled`; `initialize_unlisted` does not. Every
  existing session test passed unchanged under the new behavior. One
  consequence: `flush_server` (`workspace/symbol`) now builds the full
  index.

### Requirement-to-test mapping (R5, R6)

| Requirement | Test |
| --- | --- |
| fixture with Markdown, YAML, gitignored `node_modules/`, a hidden folder, a symlink, and `.git/` holding `.md` gives the expected list and counts; no reads | `dmls workspace::discover::tests::test_lists_markdown_and_yaml_skipping_ignored_git_and_symlinks` |
| YAML uses `exclude` only; empty `include` lists no Markdown | `…::test_exclude_applies_to_yaml_and_include_does_not` |
| duplicates reported (overlapping roots) | `…::test_overlapping_roots_report_each_duplicate` |
| a non-repository root honors `.gitignore` | `…::test_gitignore_applies_outside_a_repository` |
| a linked worktree (`.git` as a file) honors `.gitignore` | `…::test_gitignore_applies_in_a_linked_worktree` |
| a listing error marks the list incomplete | `…::test_an_unreadable_root_marks_the_list_incomplete` |
| a watched path is listed by the same rules (ignored, `.git`, outside, absent) | `…::test_listed_kind_agrees_with_the_walk`; `dmls::l1 workspace_file_list::an_ignore_file_change_rebuilds_the_list_without_parsing` (second half) |
| startup with a large fixture (300 files) parses only the open document; listing reads nothing | `dmls::l1 workspace_file_list::startup_parses_only_the_open_documents` |
| broken-link (and wiki) diagnostic not published before the list; syntax findings are; refreshed when ready | `…::path_diagnostics_wait_for_the_file_list` (fails with the gate removed) |
| missing-anchor check parses exactly the target; a second check reuses it; unchanged bytes re-read without re-parse; a changed target is re-read; deletion never serves old headings | `…::a_missing_anchor_check_parses_exactly_its_target_and_reuses_it` |
| heading completion (link and wiki) parses its target on demand | `…::heading_completion_and_hover_parse_their_target_on_demand` (fails with the completion preparation removed) |
| transclusion cycle detection follows unopened targets outward, not unrelated documents | `…::transclusion_cycles_are_found_through_unopened_documents`; unit `graph::invalidate::tests::test_prepare_targets_parses_fragment_targets_and_the_transclusion_closure_only` |
| open buffers override disk (no reads); close restores disk; an unsaved-only entry leaves | `…::an_open_buffer_overrides_disk_and_closing_it_restores_the_disk_view`; unit `…::test_close_restores_the_disk_view_or_drops_an_unsaved_buffer` |
| fallback save lists without parsing, re-checks only parsed targets, reuses unchanged bytes; a config change re-lists without parsing | `…::the_save_fallback_lists_without_parsing_and_rechecks_only_parsed_targets` |
| an ignore-file change rebuilds the list without parsing | `…::an_ignore_file_change_rebuilds_the_list_without_parsing` |
| rejected watch registration recovers via the save fallback; YAML and ignore globs registered | `…::a_rejected_watcher_registration_falls_back_to_rescanning_on_save` (fails with the fallback removed) |
| YAML listed and watched, never a graph document | `…::yaml_is_listed_and_watched_but_never_a_document` |
| schema directories and external dependency folders are watched (relative patterns) | `…::schema_inputs_outside_the_workspace_are_watched_by_folder`; unit `workspace::watch::tests::test_schema_watch_registration_roots_each_folder_at_its_parent` |
| without folder watches, a save re-checks external schema inputs | `…::without_folder_watches_a_save_rechecks_external_schema_inputs` (fails with the fallback removed) |
| `**/*.yaml`/`**/*.yml` and discovery inputs watched even with empty `include` | `workspace::watch::tests::test_registration_covers_include_globs`, `…::test_empty_include_keeps_yaml_and_schema_watches` |
| unparsed targets leave anchors unchecked until parsed | `graph::invalidate::tests::test_unparsed_targets_leave_anchors_unchecked` |
| listing installs without reads, adopts and drops, and keeps open buffers | `…::test_listed_documents_are_unparsed_graph_documents`, `…::test_install_listing_adopts_created_drops_vanished_and_keeps_open_buffers` |
| xxHash reuse, failed read, deletion (unit) | `…::test_ensure_parsed_reuses_an_unchanged_parse_and_never_serves_a_failed_read` |
| full index kept current file by file | `…::test_full_index_stays_current_file_by_file` |
| references, rename, and workspace symbols still complete (interim full index) | existing `lsp_session::heading_rename_rewrites_references`, `will_rename_files_updates_and_refuses`, `level1_graph_index` (all pass); R7 tests are Phase 5 |
| `dmls --bench-index` still measures the full index | existing `bench::tests` (unchanged; bench uses `discover_workspace` and `collect_indices_from_files`) |

The input robustness matrix does not apply. This phase reads no new file
format or configuration key; the config file reload uses the existing
`ConfigState::reload` loader.

### Same-host sample (checkpoint 4, macOS, this repository, release build)

`./target/release/dmls --bench-index $PWD` on 2026-10-17, uncommitted tree
on top of `d010cd48a`:

- **Startup listing only** (`discover`, now with `.git` skipped and YAML
  included): 172.9 ms. The spec measured 211 ms for the old Markdown-only
  walk.
- **Full index** (unchanged benchmark): 5682 Markdown files, 4856.9 ms wall.
  Stages: read 352.2 ms, parse 64527.8 ms of CPU summed across workers,
  build 509.5 ms. Peak RSS 665.3 MiB.

Startup now pays only the listing, plus the open documents and their
referenced targets. This is not a numeric gate.

### Gates and downstream tests

- Targeted runs:
  - `cargo nextest run -p dmls -E 'test(/graph::invalidate|workspace::discover|workspace::watch|workspace::startup|graph::arena/)'`
  - `-E 'test(/workspace_file_list/)'`: 12 passed.
- Mutation checks: each named test failed with its behavior removed (the
  gating filter, the completion preparation, the rejected-registration
  fallback, the schema save fallback), and passed once it was restored.
- `cargo nextest run -p dmls`: 943 passed before the new LSP tests.
- `just lint` (darkmatter): clean after two clippy fixes (`while let`, and
  `slice::from_ref` in tests).
- `just test` (darkmatter): 10214 passed, 10 skipped, 0 failed (43 s).
- `just check-tier-coverage darkmatter`: 0 stranded.
- Downstream: no workspace package depends on `dmls`, and the darkmatter
  library was not changed in this phase.
- The scope calculator also scheduled a `test-toolkit` lint cell (a
  changed suite input). `just lint` in `tools/` is clean.
- Scope calculator (all of this phase's edits saved): `packages:
  [darkmatter, dmls, test-toolkit]`, `change_class: package`. Gates ran.
- Cross-OS check: `just cross-check dmls --os windows` passed on the first
  attempt, 963 passed and 9 skipped. The rig ran the patched tree, so no
  observation was published. The risks checked were path walking, ignore
  rules, watch URIs, and absolute YAML paths in the tests.
- Pre-existing, unchanged: `dmls --bench-index .` with a relative path logs
  a context-build error (`request directory . is not an absolute path`).
  Pass an absolute path.
