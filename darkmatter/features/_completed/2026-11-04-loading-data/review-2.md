---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_review_items: []
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-07T11:54:16-08:00"
spec: 2026-11-04-loading-data/spec.md
implemented: false
description: "A **feature** review of `2026-11-04-loading-data/spec.md`"
feature: 2026-11-04-loading-data/review-2.md
previous: 2026-11-04-loading-data/review-1.md
---

# Loading Data — Review 2

The feature is **production ready** under the specification's contract. All six previous findings are verified fixed, including all 72 child instances in the repair log. No new findings remain, and no human decision is needed to continue.

I reviewed the specification, the three supporting spike conclusions, review 1, and the repair log's Finding Ledger before examining the repairs and surrounding implementation. Verification covered parsing and conversion, contract checks, command ordering and approval, memo invalidation, dependency reporting, diagnostics, CLI output, editor behavior, scanner consumers, documentation, and test selection. The working tree already contained the repair changes; this review changes only the review documents and the requested spec metadata.

## Previous Finding Verification

The required-instance column names the child IDs from the repair's Finding Ledger in [log.md](log.md). The tables below record the checks behind each outcome; completion flags were not treated as evidence. Review 1 contains no Human rulings section and no blocked finding. Its informational implementation choices and the spec's authorized follow-ups remain unchanged.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Lifecycle file writes leave later steps reading cached data | R1-F1.1–R1-F1.8: file append, JSONL append, shell attempts, body/frontmatter execution, skipped steps, failed effects, shared effect dispatch, Claudine host | **verified fixed** | Shared dispatch invalidates immediately after the engine attempt; event-end clearing is removed. Passing runtime tests prove fresh reads and preservation of reuse for skipped steps. A direct CLI reproduction emits `before 1`, `after 2`. | Nothing |
| R1-F2 — Collection and shell expressions are frozen before earlier commands run | R1-F2.1–R1-F2.13: map/filter body reads, later shell arguments, existing command-template controls, shell blocks, nested maps, new files, directive targets, structural and transclusion conditions | **verified fixed** | Pending body reads retain source/scope information and resolve between command executions. CLI reproductions return `1`, then `2` for map and filter; both a later directive and a later shell block write result value `2`. Approval tests reject refreshed bytes without authorization. | Nothing |
| R1-F3 — Failed schema expansion drops dependencies already read | R1-F3.1–R1-F3.11: successful, failed, missing, transitive, repeated and cached imports through both functions, plus other consumers | **verified fixed** | Both contract preparation and `validate_schema` collect actual resolver reads on success and failure. Runtime tests assert complete dependency sets, including earlier successful imports, transitive imports, warm-cache failures, and exclusion of unread missing files. | Nothing |
| R1-F4 — Several malformed YAML paths omit required source positions | R1-F4.1–R1-F4.13: second document, non-string keys, tagged values, parser/numeric controls, every other loader, local decoding, JSON5 token refusals | **verified fixed** | YAML event marks locate post-parse refusals independently of the tree locator. Public composition tests check coordinates with aliases, BOM and CRLF. The biscuit-file JSON5 unreadable-token regression passes. Direct CLI second-document reproduction reports line 2, column 1. | Nothing |
| R1-F5 — Allowed-host transport failures omit structured reference details | R1-F5.1–R1-F5.13: all seven remote-reading functions, controls, unavailable runtime, local failures, schema reads, transclusion boundary | **verified fixed** | Typed transport/schema failures survive to the shared diagnostic projection. Public-result tests check reference fields, original messages and codes, absent data locations, recovery behavior, and schema/import failures. | Nothing |
| R1-F6 — Quoted bracket unions bypass the required container diagnostic | R1-F6.1–R1-F6.14: literal/computed carriers, controls, valid containers, callback typing, named imports, editor/trigger consumers, malformed syntax | **verified fixed** | Definition values share the new bracket-list parser; schema declaration strings retain their grammar. Passing passive, execution, simplified-schema, trigger and editor tests cover admission and rejection. CLI scalar-union reproduction reports `dm.data.not_a_container` at column 24 with an empty read set. | Nothing |

### Instance evidence

Grouped rows below cover every ledger child ID, including the sites marked clean by the implementer. Test names refer to their owning package and module, not to private parser results alone.

| Ledger child IDs | Site and verification |
| --- | --- |
| R1-F1.1, R1-F1.2 | Darkmatter `load_data_runtime::a_later_step_reads_the_line_an_earlier_append_line_wrote` and `a_later_step_reads_the_record_an_earlier_append_jsonl_wrote` assert before/after counts and two reads in initialize, composed, finalize and failed events. |
| R1-F1.3, R1-F1.4 | Lifecycle shell and shared shell-execution boundaries invalidate before examining the attempt's outcome. Passing frontmatter rewrite, per-item shell and failed-writing-command tests retain their original expected results. |
| R1-F1.5 | `steps_that_did_not_run_keep_reuse` asserts one read and zero invalidations across guarded-off and informational steps. |
| R1-F1.6 | Both failed-side-effect tests assert a second read, including when the failure stops the stack and the next read occurs in another event. |
| R1-F1.7 | `every_dispatched_side_effect_ends_reuse` exercises file/directory creation, frontmatter mutation and failed HTTP dispatch. Inspection confirms the hook is after the single shared dispatch for all effect verbs, before error propagation. |
| R1-F1.8 | Claudine's expression host constructs fresh options without a reused data runtime. The shared executor hook's default does not introduce caching in that host. |
| R1-F2.1 | Passing map/filter body regressions and independent CLI reproductions produce `value: 1`, then `value: 2`. |
| R1-F2.2 | `a_later_shell_argument_sees_an_earlier_commands_write` and independent CLI reproduction write `r.v = 2`. `a_refreshed_shell_argument_must_still_be_approved` covers rejection, exact-rule authorization and authorized template shape. |
| R1-F2.3–R1-F2.5 | Existing per-item command-template, frontmatter rewrite and failed-writing-command controls pass; the accumulation cases finish with value 31. |
| R1-F2.6, R1-F2.7 | Shell-block/map and nested-map regressions pass, observing `1, 2` and `1, 2, 3, 4` respectively. |
| R1-F2.8, R1-F2.9 | Top-level read/new-file regression observes `1, 2, 5`; the mixed shell-block/directive regression observes 3 after the block. The established directive-before-block stage order remains explicit. |
| R1-F2.10 | Whole-value and partial transclusion targets both select the updated `two.md` in `a_transclusion_target_after_a_command_sees_its_write`. |
| R1-F2.11 | Inspected block-stage pending-read resolution and refreshed approval selection. An additional direct CLI reproduction writes 2 from a shell-block argument after a preceding directive changes the data from 1 to 2. |
| R1-F2.12, R1-F2.13 | Structural conditions, sources and callbacks still determine the body before body commands execute. Transclusion conditions run after shell stages. These are the specified stage boundaries, rather than stale reads from a completed command. |
| R1-F3.1, R1-F3.4, R1-F3.6 | Successful contract imports, successful validation, transitive imports and embedded-schema controls pass. Embedded definitions contribute no filesystem entry. |
| R1-F3.2, R1-F3.3, R1-F3.7 | Failed-contract tests assert all files read before a later bad export, missing import or transitive failure; the data source is not read when contract preparation fails. |
| R1-F3.5, R1-F3.8 | Failed `validate_schema` cases retain the subject plus all schema files actually read, including the earlier good import and transitive parent. |
| R1-F3.9, R1-F3.10 | Repeated-failure tests cover the same runtime, a fresh runtime with resolver caches warm, and repeated validation. Dependency sets remain complete. |
| R1-F3.11 | Inspected consumers of preparation summaries and dependency recording. The two repaired expression functions own this read-set projection; ordinary schema preparation, triggers and CLI schema reporting do not become expression reads. No recovery variant exists for either function. |
| R1-F4.1–R1-F4.3 | Passing public diagnostic tests locate the second document, root/nested/sequence/aliased non-string keys, and tagged values after aliases. YAML mark-pass unit tests support the public-result assertions. |
| R1-F4.4–R1-F4.6 | Root/flow tag controls, syntax errors, duplicate keys and inadmissible numbers retain positions. The aliases restriction continues to apply only to locating contract violations in a valid tree. |
| R1-F4.7, R1-F4.8 | JSON syntax and JSONL record errors retain source offsets converted to character coordinates; JSONL blank lines preserve physical numbering. Public execution and diagnostic cases pass. |
| R1-F4.9, R1-F4.13 | JSON5 syntax and parsed-but-unreadable decimal, signed, hexadecimal and escaped tokens now have positions. Biscuit-file's 32 location tests and Darkmatter's malformed-data regressions pass. |
| R1-F4.10–R1-F4.12 | TOML parser/numeric failures, CSV/TSV strict quote/width failures and local UTF-8 rejection pass. Invalid delimiter is unreachable from fixed CSV/TSV dispatch; invalid UTF-8 intentionally reports only the original byte offset. |
| R1-F5.1–R1-F5.6 | All six non-recovering remote readers use `remote_text_error`; HTTP and decoding failures share its typed branch. Wiremock public-result tests exercise both response classes at the shared adapter and at least one failure per caller. Four reference fields are present; `data_location` is absent. |
| R1-F5.7, R1-F5.8 | `try_frontmatter` still recovers both failures without diagnostics. Denied-host and missing-file controls retain their existing reference classifications. |
| R1-F5.9–R1-F5.11 | Unavailable-runtime `file_exists` uses the same typed refusal as its siblings. Local Markdown read failures retain I/O reference details; local data UTF-8 failures remain malformed-data diagnostics by contract. |
| R1-F5.12 | Missing schema target and missing named import tests assert structured fields and preserved message/code; readable-invalid and unsupported remote-schema controls do not falsely claim an attempted fetch. |
| R1-F5.13 | Transclusion keeps its existing reference-error projection; it is not routed through the expression transport adapter. |
| R1-F6.1–R1-F6.4 | Literal and computed scalar/bracket/textual unions reject with the data-specific code. Passive cases assert the `as` span and no reads; computed cases assert no data read. |
| R1-F6.5, R1-F6.6 | Shared container tests distinguish declaration grammar from definition-value parsing; existing accepted-container and null-file controls pass. |
| R1-F6.7–R1-F6.9 | A union of containers loads from literal strings, computed strings, YAML lists and expression lists; scalar list carriers reject consistently. Nullable empty-file behavior and static result typing pass. |
| R1-F6.10, R1-F6.11 | Callback literal alternatives get the data code when all are invalid; mixed alternatives defer to execution. Named list-defined types are judged after resolution and valid container imports load. |
| R1-F6.12, R1-F6.13 | Editor data-code/span and bracket-list hover tests pass. Trigger matching uses the same value parser and its quoted-list regression passes in the full Darkmatter run. |
| R1-F6.14 | Malformed and empty bracket spellings retain generic grammar/call errors, rather than being misclassified as valid scalar unions. |

The additional sibling instances were found and repaired during the previous repair pass; they are not newly discovered defects in this review. None of the original findings is only partially repaired.

## Unblocked Findings

None.

## Blocked Findings

None. No prerequisite changed and no author ruling is required.

## Spec coverage and verification level

This feature reads data and produces values, files, diagnostics and editor-protocol responses. Level 1 is the appropriate boundary for these requirements. It adds no keyboard behavior or terminal-emulator rendering requirement that would require Level 2 or Level 3 evidence.

| Requirement | Strongest relevant verification checked |
| --- | --- |
| R1 — Function registration, null handling, named types, editor authoring | L1 public composition tests, catalog parity/generated-page checks, editor hover/completion tests; file imports remain passive until execution. |
| R2 — Container-only contracts | L1 literal/computed rejection and type/span tests, shared simplified-schema tests, direct CLI rejection control. |
| R3 — Supported formats and malformed input | L1 per-format public execution/diagnostic cases, numeric admission, BOM/CRLF, UTF-8 and inert loaded-text cases; CLI success/error wiring. |
| R4 — Strict CSV/TSV and row selection | L1 object/list rows, unions and row conflicts, strict quote/width/header rules, empty values and the pinned conversion vocabulary. |
| R5 — Contract enforcement and structured errors | L1 public diagnostics and CLI envelope conformance: per-format coordinates, missing/extra values, union selection, authored order, 20-record cap and overflow, reference fields, no partial publication. |
| R6 — Remote reads | L1 loopback HTTP tests for policy denial, allowed reads, URL extension/query handling, failure details and one transport fetch per run. |
| R7 — Memo lifetime and execution ordering | L1 work counters, concurrent-load/generation tests, every cache access mode, shared passes and child documents, failed-command invalidation; direct CLI lifecycle/map/filter/directive/block reproductions. |
| R8 — Complete read set | L1 per-evaluation-surface and per-function tests, false existence probes, URL normalization, failures and imports; CLI output tests assert the envelope's stable field shape. |
| R9 — Optional files | L1 absent/present/malformed guard cases, non-recovering `||`, and reference-check/regeneration ordering. |
| R10 — Typed literal result | L1 expression typing and headless editor-protocol completion: literal object members, computed/imported unknown results, nullable and definitely null files. |
| R11 — Balanced scanner with compatibility check | L1 bounded scanner corpus and prose counterexamples, nested object/string cases, frontmatter masking, editor cursor and Claudine consumer coverage. The completed broad spike is historical evidence, not a working-tree test. |
| R12 — Documentation and shared validation repair | Generated function-page/catalog checks, current topic/data-contract/type-definition docs, authorized follow-up files, and shared-validation tests that avoid false positives on convertible text. |

The changed tests are declared by their respective `tests/l1/main.rs` targets. Their names do not move them into an inactive tier. `just check-tier-coverage darkmatter` reports zero stranded tests. The simplified-schema no-default-features run also exercises the shared changes without the default feature set.

## Input robustness matrix

Default outcome: invalid syntax or data that fails its contract produces a located error and no composed document. Exceptions are the ones the spec defines: optional members can be absent or null; compatible text converts; explicit empty values stay empty; JSON duplicate keys are last-wins; empty documents follow their format's rule. YAML aliases and multi-line flow collections retain pointer-only contract locations, while malformed YAML still has parser/event coordinates.

The field-shape matrix belongs to shared validators, since the fields in a data file are supplied by its contract. Public `load_data_execution` tests exercise required/optional property absence and null separately, wrong whole-field types, mixed and all-invalid array elements, null elements, missing/null/wrong tuple slots, dictionary values, explicit empty string/list/object values, and refusal to publish partial conversions. The default is not inferred from a nullable Rust field or from agreement between two parsers.

| Distinct loader | Duplicate and trailing-content behavior | Format-specific controls checked |
| --- | --- | --- |
| YAML/YML | Duplicates, invalid trailing syntax, non-string keys and additional documents reject with positions. | Empty/null, anchors/aliases, custom tags, numeric admission, BOM and CRLF; repaired semantic-refusal coordinates. |
| JSON | Duplicate keys use the last value; trailing garbage rejects with position. | Empty rejects, native null remains data, multibyte error columns count characters, exact numeric admission. |
| JSONL/NDJSON | Duplicate keys follow JSON per record; invalid record/trailing text rejects at its physical position. | Blank JSON-whitespace lines skip; empty gives `[]`; null records stay data; original numeric spelling is admitted before conversion. |
| JSON5 | Permitted duplicate behavior remains unchanged; invalid trailing syntax rejects. | Comments/trailing commas accepted, empty rejects, nonfinite/inexact/unreadable tokens reject with positions. |
| TOML | Duplicate keys and trailing invalid statements reject with positions. | Empty gives `{}`, datetime values become text, exact integer/nonfinite checks, array-of-table locations. |
| CSV/TSV | Object mode rejects empty/duplicate headers; list mode skips headers. Ragged rows and quote violations reject with positions. | Empty/header-only files, blank lines, quoted delimiter/newline/doubled quote, raw cell offsets, no trimming; TSV shares the delimiter-parameterized reader. |

The code-smell search found no new load-bearing `serde(default)` or `filter_map(as_str)` path that silently drops invalid input. `DelimitedData::headers: Option<_>` expresses an empty input, not a nullable configuration field. The `first_tag` key fallback is behind the loader's non-string-key rejection. Contract-carrier `.ok()` calls leave invalid carriers to the binder; parser-location defaults were traced to the parser-specific error and admission paths covered above. No additional permissive-default defect was found.

## Verification executed for this review

| Command | Result |
| --- | --- |
| `just test load_data` in darkmatter | 330 passed; includes repair regressions and CLI wiring. |
| `just test` in darkmatter | 14,868 passed, 10 skipped; 105.310 seconds; no test at or above five seconds. Includes the editor, schema, scanner, lifecycle and output-conformance suites. |
| `just lint` in darkmatter | Passed, including the Zed WASM check. |
| `just check-tier-coverage darkmatter` | Zero stranded tests. |
| `just test container_contracts` in simplified-schema | 19 selected default-feature tests passed; the recipe also ran its complete no-default-features suite, 1,362 passed. |
| `just test data_locations` in biscuit-file | 32 selected location tests passed; the recipe's reference-syntax-only run also passed, 6 tests. |
| Direct `md compose --output json` reproductions in temporary directories | Lifecycle `before 1`/`after 2`; map and filter `1`/`2`; later directive and shell-block arguments write 2; second YAML document reports `(2, 1)`; bracket scalar union reports the data code at the `as` argument with no reads. All asserted exit status and an empty stderr. |

The repair log's broader biscuit-file, simplified-schema and Claudine full-suite/lint results were inspected as recorded evidence; only the commands above are claimed as rerun here. Both review files also pass `md schema validate`, and the review-directory diff passes `git diff --check`. Cross-OS results remain CI's responsibility and are not a readiness finding. The macOS linker emitted its large unwind-section warning during the focused build; tests passed. No formatter, commit, focus-changing terminal/browser test or production-code edit was performed by this review.

## Recurrence

No finding recurs. The six earlier defect classes are repaired, and this review found no unchanged instance, repair regression or missed sibling to carry forward. The earlier nonblocking locator-cost observation remains an optimization opportunity, not a stronger performance requirement.
