---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-24T08:08:19-07:00"
spec: 2026-10-30-claudine-recursive-schemas/spec.md
implemented: false
description: "A **feature** review of `2026-10-30-claudine-recursive-schemas/spec.md`"
feature: 2026-10-30-claudine-recursive-schemas/review-3.md
previous: 2026-10-30-claudine-recursive-schemas/review-2.md
---

# Review 3: Claudine recursive schemas

The feature is **production ready**. Both findings from review 2 are verified
fixed, including every additional instance in the repair's finding ledger.
The four findings closed by review 2 remain fixed. This review found no new
finding or observation, and no human decision is required.

## Previous Finding Verification

I read both earlier reviews, the implementation log, and the repair's
`### Finding Ledger` in `log.md` before checking the implementation. The
outcomes below are based on source, test assertions and selection, and the
independent runs recorded later in this report. No remaining defect is being
classified as a partial repair or deferred to another review.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1 — Literal-token arguments are incorrectly deferred after decoding | R2-F1.1–R2-F1.11: scalar, positional, named and structured arguments; acceptance/settlement; conversion and template controls; DMLS; Claudine prevalidation; md CLI; hand-written syntax scanning; raw and phased APIs and Claudine callers | verified fixed | All eleven instances are checked individually below. Darkmatter retains the stored value while binding its decoded text; Claudine passes stored values into the shared validation API. Public result and consumer tests passed. | None. |
| R2-F2 — Expanded schema tests exceed the L1 time budget | R2-F2.1–R2-F2.7: editor invalid corpus; schema matrix; editor completion branches; provider dry runs; string list forms; data formats; audio dry-run spellings | verified fixed | All seven instances are checked individually below. Serial batches became separately selected tests with coverage guards. Every focused selection passed with no test above five seconds. | None. The repair's five additional instances were siblings found and repaired during that repair, not new open findings here. |
| R1-F1 — Passive argument validation omits runtime registration rules | R1-F1.1–R1-F1.11: recovery, source exclusions, overloads, controls, all consumers, event-time binding and authored-value handling | verified fixed | The lifecycle selection passed the registration, overload, ambiguity, recovery, exclusion and event-binding tests; CLI/editor and Claudine parity selections passed. Binding still receives authored operands before schema conversion. | None; the stored-origin sibling reported separately as R2-F1 is also repaired. |
| R1-F2 — Single-action dictionary long forms are rejected | R1-F2.1–R1-F2.13: three event profiles, roots and branches, loop forms, bundles, four container paths, consumers, companion keys, md profile, schema-about layout and conditional-root rejection | verified fixed | The action-only `entry` families remain separate from list conditionals. Schema/parser, container, md-profile and consumer tests passed. The schema-about component repair remains in source; its passing result is recorded in the earlier repair log. | None. Parser-only placement restrictions remain the accepted boundary. |
| R1-F3 — Malformed templates pass validation for string operands | R1-F3.1–R1-F3.12: generated scalar/positional/named and structured operands, mixed text, controls, consumers, parser, 22 hand-written sites and CLI error mapping | verified fixed | Lifecycle tests still report syntax errors independently of destination type and skip stored literal data. Hand-written operand tests, shared consumer fixtures and launch rejection tests passed. | None. |
| R1-F4 — New launch coverage excludes native Windows | R1-F4.1–R1-F4.8: rejection rows, initialization order, deferred success, evaluated wrong types, expression failure, dry run and portable library/editor controls | verified fixed | The required launch tests still use the compiled fake provider, portable file effects and the enabled `test-fixtures` feature. They have no Unix gate. All 51 launch tests passed locally. | None. Cross-OS execution remains CI's responsibility, not an open review finding. |

### Stored-data repair: every required instance

In Darkmatter, [BoundOperands::bind](../../../darkmatter/lib/src/markdown/schemas/bound_operands.rs)
uses the runtime registration to check generated arguments. It now records
which argument leaves came from stored literal tokens before choosing what
may be deferred. The predicate sees the same argument values the binder
walks; this invariant is documented and protected by the public result tests.
It introduces no expression evaluation or second conversion policy.

| Instance | Verification and result |
|---|---|
| R2-F1.1 — Scalar | `a_literal_token_holding_template_text_is_bound_as_that_text` rejects the encoded expression-looking text at `/actions/0/abs`, with a source position. |
| R2-F1.2 — Positional | The same public-options test rejects `/actions/0/min/0`. |
| R2-F1.3 — Named | The same test rejects `/actions/0/min/a`. |
| R2-F1.4 — Structured | The same test rejects `/actions/0/merge_frontmatter/1`, where an object is required. |
| R2-F1.5 — Acceptance and settlement | All four rejection rows assert an invalid public report and an empty deferred list. Stored text can no longer settle a bad call through template deferral. |
| R2-F1.6 — Controls | `a_literal_token_keeps_the_conversion_and_template_controls` accepts stored text for `trim`, rejects stored `three` for `abs`, accepts stored `3`, and reports the ordinary authored template as deferred. |
| R2-F1.7 — Editor | `a_stored_literal_token_argument_is_rejected_at_its_operand` checks DMLS's published diagnostic at `/start/0/abs` and its exact starting position. |
| R2-F1.8 — Claudine prevalidation | `a_stored_literal_token_is_type_checked_at_launch` rejects the stored text through public prevalidation, while accepting an authored template and stored numeric text. |
| R2-F1.9 — md CLI | The stored-token row in `recursive_lifecycle_corpus_gets_the_agreed_verdicts_and_positions` still rejects with its expected path, line and column. |
| R2-F1.10 — Syntax controls | Generated and hand-written literal-token tests still accept malformed template text as data; the syntax scanner reads the stored form and skips encoded leaves. |
| R2-F1.11 — Phases and callers | `the_phase_apis_keep_a_stored_token_apart_from_an_authored_template` checks raw validation, Launch, Completion, and stored validation with and without a phase. Source inspection confirms Claudine's prevalidation, optional-field cleanup and status classification all pass the stored instance into `validate_stored`. |

The tests above are in
[Darkmatter's generated-operand suite](../../../darkmatter/lib/tests/l1/lifecycle_generated_operands.rs),
[the editor parity suite](../../../darkmatter/dmls/tests/l1/claudine_editor_parity.rs),
[the md CLI corpus](../../../darkmatter/cli/tests/l1/claudine_schema_bundle.rs), and
[Claudine's embedded-schema suite](../../lib/tests/l1/claudine_schemas_embedded.rs).
Each passed in this review. Ordinary event stacks, initialization,
cancellation, conditional branches, bundles, loop forms and container events
all reach this same schema walker and registration adapter; none implements
another stored-data policy.

The existing raw/phase APIs and md CLI do not acquire a new promise to decode
every literal token. In particular, the repair deliberately leaves their
older treatment of encoded numeric text unchanged. That was a clean control
in review 2, not an unresolved instance or a reason to reopen the decision.

### Test-duration repair: every required instance

The Rust testing skill's five-second limit was checked against the normal
focused selections, not just isolated individual tests. The splits preserve
the previous assertions. Source inspection also checked their guards against
the tables or inventories from which the cases originate.

| Instance | Verification and result |
|---|---|
| R2-F2.1 — Editor invalid corpus | Twelve separately named fixture tests retain exact paths, positions and diagnostic text. The corpus guard checks distinct invalid rows and explicit treatment of every fixture. The whole CLI/editor selection's slowest test was 2.911 s. |
| R2-F2.2 — Schema matrix | Disk and embedded matrices retain the control and all eight shapes per column and syntax. Guards compare both column coverage and each generated shape list with the tables. All 859 schema tests passed; the slowest was 3.430 s. |
| R2-F2.3 — Editor deep completion | Separate `then` and `else` tests retain the completion and hover assertions. Both passed in the CLI/editor selection. |
| R2-F2.4 — Provider dry runs | One test per provider, with the guard comparing the generated list against the compiled provider inventory. All ten provider rows passed; the 27-test module's slowest test was 2.988 s. The pre-existing Unix requirement for these shell-stub tests is unchanged. |
| R2-F2.5 — String list forms | Separate CSV, TSV, line, space and scalar cases retain their plan assertions. The guard checks actual classification after YAML decoding and exhaustively handles list formats. The corrected line case now uses a YAML newline escape. All passed; the 33-test source module's slowest test was 3.215 s. |
| R2-F2.6 — Data formats | Separate YAML, JSON, JSON5, JSONL and NDJSON cases compare against one shared expected plan. The guard checks the loader's formats and both line-delimited extensions. All passed in the source module selection. |
| R2-F2.7 — Audio dry-run spellings | Four separately selected spelling tests retain success and no-preparation assertions; the guard compares their spellings against the worker's gate. All passed; the 24-test module's slowest test was 2.860 s. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding recurs in this review. The stored-data classification defect from
review 2, which was related to review 1's passive-binding integration, is now
closed across its full required instance list.

## Observations

None.

## Specification coverage and verification level

| User-facing requirement | Appropriate verification and evidence checked |
|---|---|
| Recursive families, embedded imports and payload size | Level 1: schema tests resolve exports without disk schemas, check recursive definitions and absence of numbered unrolls, and enforce the 2,000,000-byte compact JSON limit. Passed. |
| Catalog signatures, effects metadata and regeneration | Level 1: generation drift, collision inventory, typed metadata coverage, overload and variadic parity tests in the lifecycle selection passed. The maintained generator reads typed catalog APIs; checked-in regions match fresh generation. |
| Deep restrictions and parser-only conditional limit | Level 1: schema/parser fixtures cover both branches, depths 1/3/5, valid depth 6, deep misspellings, initialization shells, bundles and loop gates. Claudine and CLI/editor selections passed. |
| Recursive containers | Level 1: all 18 container tests passed, covering the four depth-four container paths, seven events, cancellation restrictions, initialization branches, generated operands and independent conditional-depth counts. |
| Passive diagnostics, completion and hover | Level 1: LSP protocol tests check published positions, deep branches, generated parameters and event filtering. Instrumented no-action tests passed with the recipe's instrumentation feature enabled. No terminal rendering or keyboard behavior is required by the spec. |
| Launch and event-time argument checking | Level 1: compiled fake-provider launch tests and public event-binding tests passed, including no launch on malformed literals, deferred scalar/structured success, wrong evaluated types, no dispatch on binding failure and `no_error` behavior. |
| Corpus, documentation and generator guidance | Level 1 corpus and frozen-prompt cases in the schema selection passed. Current schema guidance describes recursion, argument forms, conversion, stored data and the parser-only limit. The generator's deterministic drift tests passed; historical spike inputs remain separate. |
| Validation-time goal | Reused the accepted repair-log measurement: depth-four group document, release `md schema validate -vv`, 115.40–121.12 ms total, resolve dominant at 74–80 ms. The log records command, fixture, setup and host load. The spec expressly allows reuse and makes this a recorded goal rather than a timing gate. |

No requirement needs Level 2 or Level 3 evidence. The reviewed tests are
declared in their consolidated `l1/main.rs` targets, their names select L1,
and fixture/instrumentation features are enabled by the actual recipes and
CI metadata. No new tier marker, ignore, retry or timeout increase hides a
case. No terminal or browser was focused.

For input robustness, the existing disk/embedded schema matrix preserves its
controls and every previous shape assertion after splitting. The shared
generated-argument validator tests cover absence, explicit null, scalar and
collection type errors, empty calls, optional omission, unknown names and
literal siblings beside deferred values. Expected exceptions remain the
specified numeric conversions, registration recovery, admitted nulls and
valid deferred operands. The source sweep of `schema_projection.rs`,
`operands.rs` and `bound_operands.rs` found no new permissive defaulting:
named-call selection failure leaves structural validation in charge, and a
bad schema regex remains the schema validator's error. The stored-token
decode fallback is explicit API behavior, not silent acceptance of an
invalid argument. This repair adds no document loader requiring another
format matrix.

## Independent runs and limits

These macOS runs used the canonical just recipes and nextest, sequentially:

| Area and command | Result | Slowest test |
|---|---|---|
| Darkmatter: `just test lifecycle_` | 250 passed | 1.819 s |
| Claudine: `just test-library claudine_schema` | 859 passed | 3.430 s |
| Darkmatter: `just test claudine_` | 120 passed | 2.911 s |
| Claudine: `just test-library container_schema_recursion` | 18 passed | 1.891 s |
| Claudine: `just test-cli launch_trigger_schemas` | 51 passed | 2.540 s |
| Claudine: `just test-cli wrap_sequence_composition` | 27 passed | 2.988 s |
| Claudine: `just test-cli sequence_sources_cli` | 33 passed | 3.215 s |
| Claudine: `just test-cli tts_prewarm` | 24 passed | 2.860 s |

Every command exited successfully, with no SLOW or failing test. Filtered-out
tests in these runs are outside the requested selection, not resource skips
of required cases. Some selections overlap; these counts are test executions,
not a count of distinct tests.

The repair log's full suites, lint, tier audits and earlier native-Windows
run were read as supporting records. This review did not repeat those full
gates, remote runs or the release timing sample. Cross-OS receipts and a
pending human review do not determine readiness under the review contract.
Only this report and the requested review/spec metadata are changed by this
review; implementation and tests are left intact.
