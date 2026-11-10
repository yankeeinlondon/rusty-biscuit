---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-08T20:35:39-08:00
spec: 2026-11-05-tabular-input-options/spec.md
implemented: false
description: "A **feature** review of `2026-11-05-tabular-input-options/spec.md`"
feature: 2026-11-05-tabular-input-options/review-2.md
previous: 2026-11-05-tabular-input-options/review-1.md
---

# Review of Tabular Input Options — iteration 2

The feature is **production ready** within the specification's scope. Both
previous findings are verified fixed, including every child row in the
repair's Finding Ledger. No new findings or remaining defects were identified.

Reviewed the specification, plan and its accepted November 8 decisions,
previous review, original implementation log, repair log, implementation,
documentation, and tests. The review preserves the accepted whole-call
location for computed-option errors, effective-format TSV rule, completion
aliases, and per-call read observations. Review 1 contains no pending Human
rulings section. Its informational human-review items do not block readiness
and are not being submitted for confirmation again.

## Previous Finding Verification

The table includes the parent findings and every required child instance,
including the additional instances the repair recorded. Evidence below comes
from reading the implementation and rerunning tests, rather than accepting
the ledger's completion claims.

Test names in the table refer to Darkmatter's
[passive option tests](../../lib/tests/l1/load_data_options_passive.rs),
[public composition tests](../../lib/tests/l1/load_data_options.rs), and
[editor protocol tests](../../dmls/tests/l1/option_arguments.rs).
The relevant implementation is Darkmatter's
[shared call checker](../../lib/src/markdown/compose/expression/call_check.rs),
which binds literal option members before checking their meaning and retains
independent option checks when the contract is unknown.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Passive option checks ignore shared coercion results | R1-F1.1–R1-F1.11, individually checked below | verified fixed | `OptionsView::of` uses the registered parameter binder; delimiter, applicability, TSV, and row decisions consume the resulting values. | Nothing |
| R1-F1.1 — Coerced headerless object rows | CSV object contract, `header: "false"` | verified fixed | `coerced_values::string_header_false_with_csv_object_contract_is_rejected_at_as`; public direct/dead-branch composition checks assert the contract argument's location and no reads. | Nothing |
| R1-F1.2 — Coerced delimiter length | CSV tuple contract, `delimiter: 10` | verified fixed | `coerced_values::numeric_delimiter_on_csv_binds_to_text_and_is_invalid_options`; direct/dead-branch checks reject the bound `"10"` at options. | Nothing |
| R1-F1.3 — Coerced non-tabular header | JSON, `header: "true"` | verified fixed | `coerced_values::string_header_on_json_is_invalid_options`; public composition rejects before reading. | Nothing |
| R1-F1.4 — Coerced TSV delimiter | TSV, `delimiter: 5` | verified fixed | `coerced_values::numeric_delimiter_on_tsv_is_invalid_options`; the bound `"5"` fails TSV's tab-only rule. | Nothing |
| R1-F1.5 — Partial object coercion | Literal `header: "false"` beside computed delimiter | verified fixed | `coerced_values::string_header_false_beside_a_computed_member_is_rejected_at_as`; public composition tests cover direct/dead branches with `sep` from frontmatter. | Nothing |
| R1-F1.6 — Editor projection | All four fully literal coercion failures | verified fixed | `coerced_literal_options_are_ranged_like_composition` checks all four diagnostic codes and exact argument ranges through an editor session. | Nothing |
| R1-F1.7 — Runtime control | All five computed coercion failures | verified fixed | `coerced_computed_options_fail_at_execution_with_the_same_codes` passes with the expected data errors and no source reads. | Nothing |
| R1-F1.8 — Native-value controls | Native false header on CSV objects; native true header on JSON | verified fixed | `coerced_values::native_controls_are_still_rejected` retains both errors. | Nothing |
| R1-F1.9 — Valid coercion control | CSV tuples, `header: "true"` | verified fixed | Passive acceptance and `coerced_header_true_on_a_tuple_contract_reads_rows` verify literal and computed success. | Nothing |
| R1-F1.10 — Format decisions | Enum, aliases, defaults | verified fixed | `coerced_values::literal_format_is_read_from_the_bound_options`; alias/default reuse tests pass. The catalog enum binds accepted names unchanged and rejects other names. | Nothing |
| R1-F1.11 — Documentation correction | Loading Data paragraph and both editor diagnostic rows | verified fixed | Read the updated conversion examples in `docs/topics/loading-data.md` and `dmls/docs/diagnostics.md`; they describe converted values rather than rejecting convertible native types. | Nothing |
| R1-F2 — Computed contracts suppress provable option applicability errors | R1-F2.1–R1-F2.13, individually checked below | verified fixed | `load_data_problem` keeps unknown contract verdicts optional; delimiter validity and format applicability run independently, while row classification waits for a known contract. | Nothing |
| R1-F2.1 — JSON header beside computed contract | Extension-selected JSON, `header: true` | verified fixed | `computed_contract::header_on_an_extension_json_file_is_invalid_options`; public direct/dead-branch composition asserts options location and no reads. | Nothing |
| R1-F2.2 — JSON delimiter beside computed contract | Extension-selected JSON, delimiter `;` | verified fixed | `computed_contract::delimiter_on_an_extension_json_file_is_invalid_options`; same public composition coverage. | Nothing |
| R1-F2.3 — TSV delimiter beside computed contract | Extension-selected TSV, delimiter `;` | verified fixed | `computed_contract::non_tab_delimiter_on_an_extension_tsv_file_is_invalid_options`; same public composition coverage. | Nothing |
| R1-F2.4 — Explicit override beside computed contract | CSV filename overridden to JSON, `header: true` | verified fixed | `computed_contract::explicit_json_format_on_a_csv_file_with_header_is_invalid_options`; direct/dead-branch composition checks the override. | Nothing |
| R1-F2.5 — Every non-tabular name | Explicit YAML/YML, JSON, JSONL/NDJSON, TOML, JSON5 with header | verified fixed | All seven `header_with_explicit_*_is_invalid_options` tests pass. Every name reaches the shared canonical-format applicability check. | Nothing |
| R1-F2.6 — Editor projection | JSON header/delimiter and TSV delimiter beside computed contract | verified fixed | `computed_contract_keeps_option_applicability_diagnostics` verifies three errors with exact options ranges and two clean controls. | Nothing |
| R1-F2.7 — Runtime control | Computed contract and computed options, extension and override | verified fixed | `computed_contract_and_options_fail_applicability_at_execution` verifies the original runtime failures. | Nothing |
| R1-F2.8 — Independent delimiter check | Computed contract, delimiter `;;`, literal or computed file | verified fixed | `computed_contract::unusable_delimiter_is_still_invalid_options` and public composition retain rejection. | Nothing |
| R1-F2.9 — Applicable/default controls | Null fields; CSV/TSV header true | verified fixed | `computed_contract::applicable_or_null_options_are_accepted` and `computed_contract_with_applicable_options_reads_rows` verify acceptance and actual loaded values. | Nothing |
| R1-F2.10 — Genuine unknown format | Whole computed options or computed format | verified fixed | `computed_contract::computed_options_or_format_still_defer`; computed override execution tests pass without false extension-based rejection. | Nothing |
| R1-F2.11 — Binder and contract precedence | Malformed literal contract and provably invalid literal container contract | verified fixed | `contract_precedence::malformed_contract_keeps_the_binder_error` and `provably_invalid_literal_contract_outranks_applicability`; inspected `binder_refuses` and the contract-verdict path. | Nothing |
| R1-F2.12 — Partly known contracts | Conditional literal/computed and nullable contract arms | verified fixed | `computed_contract::partly_literal_contracts_still_judge_applicability` checks all three recorded shapes. | Nothing |
| R1-F2.13 — Unknown row-shape control | Computed contract, headerless CSV/TSV | verified fixed | `computed_contract::headerless_csv_is_not_judged`, editor clean control, and public execution verify deferred classification followed by the expected object-row conflict. | Nothing |

Neither parent finding is partially repaired or regressed. The extra ledger
instances were addressed during repair; they are not newly discovered
siblings in this review. No earlier open findings remain.

## Unblocked Findings

None.

## Blocked Findings

None. No decision requires stopping for a human.

## Spec coverage and verification level

Every requirement in this feature calls for **Level 1** verification. The
feature changes parsing, composition, diagnostics, reuse, and editor protocol
results; it adds no terminal rendering or physical keyboard behavior. The
spec explicitly requires no real-terminal or browser tests.

| User-facing requirement | Evidence inspected and executed | Strongest level needed / present |
| --- | --- | --- |
| Headerless CSV/TSV includes the first record; one-record, empty, blank, ragged, and quoted inputs retain strict behavior | `load_data_headerless` public composition tests | L1 / L1 |
| Contract errors preserve first/later row pointers and source positions through multiline, multibyte, CRLF, BOM, and missing-cell cases | Headerless public diagnostic tests and logical-row locator tests | L1 / L1 |
| Custom semicolon/pipe delimiters, supported-extension override, TSV defaults and restrictions, exact format names, aliases | `load_data_options` and passive tests | L1 / L1 |
| Defaults, null fields/files, shared coercion, computed options/contracts, passive errors in dead branches, imported contracts | Public passive and composition tests, including every previous instance above | L1 / L1 |
| Raw reads, parse and bind separation, default equivalence, ordering, failure reuse, concurrent callers, invalidation, unchanged read sets | Runtime tests plus public options, slots, and execution tests; inspected all three memo keys and generation propagation | L1 / L1 |
| Extensionless remote input, denied and warmed hosts, redirect policy, deferred reference checks | `load_data_options_remote` local HTTP fixtures; inspected authorization before memo reuse | L1 / L1 |
| Both arities retain result types/nullability; option completion and hover derive from the catalog; diagnostics match composition | `option_arguments` editor protocol tests and `call_check_agreement` | L1 / L1 |
| Normal CLI headerless success and option failure preserve exit status, empty stderr, located diagnostics, read sets, and JSON envelope | `load_data::compose_headerless_rows_json` and `compose_invalid_option_json_envelope`, both executed by the load-data selection | L1 / L1 |
| Catalog examples and documentation describe the delivered behavior | Catalog selection; inspected generated `docs/functions/load-data.md`, Loading Data, Caching, CLI data contracts, editor diagnostics, and skill guidance | L1 / L1 |

The accepted difference in error precedence remains: an invalid computed
contract beside independently invalid literal options can be rejected
passively for the options before execution would discover the contract
error. This fulfills the explicit requirement to judge literal options beside
computed arguments. It requires no new public API or scope decision and is
not a finding.

## Input robustness

The options object is expression syntax, so the configuration-file matrix
does not apply to its grammar. Its field outcomes are nevertheless explicit
and tested through the library's public results: omitted options, `null`,
`{}`, and null members mean defaults; unbindable values and unknown keys are
binder errors; semantic delimiter and applicability failures are data errors.
Valid scalar conversions are preserved. The repaired partial-object path
uses that same validator, rather than introducing a native-type restriction.

The data-file changes select an existing parser and project its immutable
table through a shared logical-row view; they introduce no new document
loader. Existing public execution tests cover format-specific document
shapes, including trailing content, YAML duplicate keys/multiple documents,
TOML duplicate keys, JSON5 comments/trailing commas, JSONL blank lines,
malformed quoting, and UTF-8 failures. Headerless tests verify the changed
row interpretation and positions. Defaults and null equivalence here are
specified behavior, not missing-field recovery.

Searched the affected data modules for `serde(default)`, `filter_map`,
`unwrap_or_default()`, and `.ok()`. No new permissive parse fallback was
found. The table's `filter_map` ranges over its own exact `row_count`, with
`row` using the same header setting; it does not discard invalid input cells.
The remaining default fallbacks handle optional locations or observation
captures rather than data values.

## Checks performed

Executed from `darkmatter/` using the repository's nextest recipes:

- `just test load_data`: **485 passed**.
- `just test option_arguments`: **14 passed**.
- `just test call_check`: **110 passed**.
- `just test catalog`: **265 passed**.
- `just lint`: **passed**, including the Zed extension compile check.
- `just check-tier-coverage darkmatter` from the repository root: **passed**, no stranded tests.

These are overlapping selections, not a count of distinct tests. No selected
test reached the five-second L1 limit. These are this review's own results;
the repair log's earlier full-suite and lint claims were not substituted for
them. No cross-OS execution evidence is claimed or required for readiness.

Checked the declared `l1` targets in the library, CLI, and editor manifests
and their module declarations. The added tests compile and actually ran in
the selections above; their names contain no higher-tier marker. Test
fixtures use temporary directories and headless editor sessions. The CLI
envelope schema is read with `include_str!` inside each added test, retaining
CI's repository-input tracking.

## Recurrence

None. Both previously reported defect classes are repaired; this review
reports no unchanged instance, repair regression, or missed sibling.
