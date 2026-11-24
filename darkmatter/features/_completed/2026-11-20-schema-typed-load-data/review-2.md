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
created: 2026-11-21T02:40:32+00:00
spec: 2026-11-20-schema-typed-load-data/spec.md
implemented: false
description: "A **feature** review of `2026-11-20-schema-typed-load-data/spec.md`"
feature: 2026-11-20-schema-typed-load-data/review-2.md
previous: 2026-11-20-schema-typed-load-data/review-1.md
---

# Review 2: Schema-Typed Load Data

The feature is production ready under the specification's contract. Both
previous findings are verified fixed; there are no new findings or pending
author decisions. This conclusion covers the current working-tree
implementation, including the uncommitted repairs.

## Previous Finding Verification

Reviewed `review-1.md` and the repair's `log.md`, including every child in
its Finding Ledger, before assessing new work. There are no earlier review
files or human rulings in this feature directory. Completion claims were
checked against code, test assertions, and fresh test execution.

In the table, public observation tests live in Darkmatter's
[`editor_observation.rs`](../../lib/tests/l1/editor_observation.rs), and editor
protocol tests live in DMLS's
[`typed_load_data_editor.rs`](../../dmls/tests/l1/typed_load_data_editor.rs).
Both modules are declared in their packages' consolidated L1 targets. All
named regression tests below ran in the focused verification runs.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Unused branches prevent editor observation of concrete results | R1-F1.1–R1-F1.14, detailed below | verified fixed | Reached calls enter `admit_editor_call`; `PassLookup` resolves reached bindings. Public observation and DMLS tests agree on selected results and pending operations. | Nothing |
| R1-F1.1: Unused context arm | Both ternary orientations | verified fixed | `an_unselected_ctx_arm_is_not_bound` and its reversed case observe the rows; corresponding DMLS tests report the bad age, matching compose. | Nothing |
| R1-F1.2: Unused logical fallback | Successful load followed by `ctx.runtime` | verified fixed | `an_unused_ctx_fallback_is_not_bound` and `an_unused_ctx_fallback_does_not_withhold_the_verdict` pass. | Nothing |
| R1-F1.3: Unused recovery argument | Successful `try(load_data(...), ctx.runtime)` | verified fixed | `an_unused_recovery_argument_is_not_bound` and the matching DMLS verdict test pass. | Nothing |
| R1-F1.4: Unsupported call in unused arm | Unselected `find_files` | verified fixed | `an_unselected_listing_is_not_admitted` checks zero handler entries and only data-file observations; the DMLS test reports the destination error. | Nothing |
| R1-F1.5: Absent property in unused arm | Unselected absent name | verified fixed | `an_unselected_absent_property_is_not_bound` and the matching DMLS test pass. | Nothing |
| R1-F1.6: Unrun callback body | Empty `map` with runtime-only body, followed by fallback load | verified fixed | `an_empty_map_callback_body_is_not_bound` observes the load; `an_unrun_callback_body_does_not_withhold_the_verdict` reports the bad age. | Nothing |
| R1-F1.7: Ordinary ternary control | Selected load with unused empty array | verified fixed | `an_ordinary_unselected_arm_is_observed` and `an_ordinary_unselected_arm_is_judged` preserve the expected result. | Nothing |
| R1-F1.8: Guarded missing-file control | Unselected missing read | verified fixed | `recovery_turns_only_a_proven_miss_into_the_fallback` observes `other` with no reads; guarded/recovered DMLS cases remain warning-free. | Nothing |
| R1-F1.9: Pending-result consumers | Selected unavailable bindings, unsupported calls, recovery, callback bodies, verdict/member consumers | verified fixed | Public tests assert pending and no effects for reached operations. DMLS selected-context/listing/absent-property tests withhold verdicts. `slot_jobs::observe` only installs observed values; completion's pending control offers no invented members. | Nothing |
| R1-F1.10: Existing test encoded the defect | Reached and unused `as_markdown`, plus cycles | verified fixed | `unsupported_runtime_only_and_cyclic_properties_are_pending` now distinguishes the executed unsupported call from a successful load's unused fallback; the latter is judged. | Nothing |
| R1-F1.11: Admission and passive rejection | No effects from reached unsupported calls; invalid authored calls still rejected in unused branches | verified fixed | Handler-entry assertions pass with instrumentation enabled; `an_invalid_call_in_an_unselected_arm_is_still_rejected_passively` compares the editor and compose rejection. | Nothing |
| R1-F1.12: Admission documentation | Module comments, schema-slot guide, DMLS diagnostics, repo skill | verified fixed | Read `observe.rs`, `editor_reads.rs`, `schema-slots.md` observed-property section, DMLS diagnostics, and the darkmatter DMLS skill. Each describes reached-operation admission; schema-slot admission retains its distinct contract. | Nothing |
| R1-F1.13: Literal frontmatter index regression found during repair | Literal key, templated key, absent key, unused whole namespace, local indexing | verified fixed | `a_literal_fm_index_reads_only_that_key` passes. The evaluator's normal and caller-file index paths use the optional lookup hook; `ScopedLookup` forwards it while preserving local bindings. | Nothing |
| R1-F1.14: Lookup inventory guard | New observation lookup must be listed | verified fixed | Fresh `lookup_parity` run passes the inventory and binding-separation tests; `PassLookup` has explicit observation coverage. | Nothing |
| R1-F2: Headerless editor handling follows syntax instead of executed read options | R1-F2.1–R1-F2.11, detailed below | verified fixed | Loader records the bound mode of successful executed reads. The syntax predicate is removed; property verdicts and call shapes consume the recorded facts. | Nothing |
| R1-F2.1: Literal headerless control | `{header: false}` without a contract | verified fixed | `a_literal_headerless_read_has_no_editor_cell_verdict` proves compose rejects the bad cell while the editor withholds its verdict. | Nothing |
| R1-F2.2: Converted false option | `{header: 'false'}` | verified fixed | `a_string_false_header_has_no_editor_cell_verdict` and `string_false_headerless_cells_stay_provisional` pass. | Nothing |
| R1-F2.3: Property-supplied options | Literal `opts` property with false header | verified fixed | `headerless_options_from_a_property_have_no_editor_cell_verdict` and the callback-hover case pass. | Nothing |
| R1-F2.4: Spread options | `{...{header: false}}` | verified fixed | `spread_headerless_options_have_no_editor_cell_verdict` and `spread_headerless_cells_stay_provisional` pass. | Nothing |
| R1-F2.5: Unused headerless arm | Selected headed arm and reversed selection | verified fixed | Selected headed-arm test reports the same errors as compose; selected headerless-arm test withholds the verdict. | Nothing |
| R1-F2.6: Headed control | Ordinary headed object rows | verified fixed | `a_headed_read_is_judged` preserves editor/compose parity. | Nothing |
| R1-F2.7: Per-call shape consumer | Literal, converted, variable, spread modes; headed control | verified fixed | Four protocol hover tests retain `unknown[]` for callback rows from headerless reads. `headed_call_rows_are_typed_from_the_observation` checks observed hover and member completion. Read `observed_call_shapes` through its union and overflow decisions. | Nothing |
| R1-F2.8: Passive return typing | Provable literal options versus unprovable options/contracts | verified fixed | `load_data_passive::typing::an_uncontracted_csv_read_narrows_only_from_provable_options` asserts narrow literal results and safe general fallbacks. Passive checking performs no data read. | Nothing |
| R1-F2.9: Equivalent contract/override forms | Parenthesized null, property null, computed null, later spread false, later explicit true | verified fixed | Dedicated DMLS cases cover all five forms; false modes defer and later true is judged. Public observation also checks mode flags and returned rows. | Nothing |
| R1-F2.10: Callback editor consumer | Hover for property-supplied headerless options | verified fixed | `headerless_cells_from_property_options_stay_provisional` asserts `Local r` has `unknown[]`, rather than observed cell types. | Nothing |
| R1-F2.11: Public facts and dependencies | Executed calls only; a property reading a headerless property's value | verified fixed | `only_an_executed_headerless_uncontracted_read_is_marked` checks direct calls, unused arms, and the dependent `b` property with no own calls. `Pass::bare` propagates the dependency flag; DMLS uses that flag before inserting replacements or shapes. | Nothing |

The repair-added rows above extend verification of the original findings;
they are not newly open siblings. No required instance remains partially
fixed, regressed, or blocked.

## Unblocked Findings

None.

## Blocked Findings

None. No item requires a human decision before repair or a separate author
review after this cycle.

## Specification and Test Rigor

These requirements concern returned data, passive checks, schema validation,
diagnostic records, and language-server responses. **Level 1 is the
appropriate verification level for all of them.** The spec adds no terminal
rendering or keyboard behavior requiring Level 2 or Level 3 evidence.

| User-facing requirement | Verification level and evidence |
|---|---|
| Valid worked example; each bad-age property fails independently; direct, parenthesized, and selected direct loads retain data positions | L1 public compose tests in `load_data_destination`, plus DMLS worked-example, per-property parity, and related-location tests |
| Effective declarations, enum membership, numeric conversion, defaults, closed objects, imported types, and triggers | L1 public `validate_observed_frontmatter` results in `observed_validation`, compose destination tests, and editor parity tests |
| Optional/null contracts and files; root normalization; supplied contracts applied after normalization; shared read memo | L1 compose tests in `load_data_uncontracted`, execution/runtime suites, and public observation read/parse counters |
| Header modes, compatible/incompatible contracts, camelCase keys and collisions, verbatim explicit names, invalid names before reads, width and cell positions, separate memo entries | L1 `load_data_uncontracted`, options/headerless suites, CLI `load_data`, and repaired DMLS mode/shape cases |
| Passive checking without reading; executed missing-read warning; guarded/recovered reads; approvals; unsaved buffers; creation/config/schema refresh | L1 public observation and in-process DMLS protocol tests |
| Stale and superseded jobs; cache-only remote authorization, misses, and expiry; shared work allowance | L1 DMLS refresh/expiry tests and `slot_jobs::tests`, public observation budget tests, and cache-only implementation inspection |
| Spread order, multiple/null/invalid operands, explicit duplicates, interpolation closing, unknown-key member typing, permission/dependency scans | L1 parser-backed compose, passive checks, public spread tests, DMLS `object_spread_editor`, and scan tests for overwritten operands |
| Shipped definitions, documentation examples, and normal invocation | L1 `shipped_schema_corpus` and `shipped_corpus_invocation`, generated function-page checks, and CLI/LSP entry-point tests |

Inspected the shared parser's ordered field/spread representation, duplicate
key checks, operand spans, and rejection of array spread. The feature's
existing Claudine consumer verification remains relevant: this repair does
not change the AST, and its evaluator admission and index hooks retain the
default compose behavior. No additional Claudine runtime result is claimed
by this review.

The accepted conservative choices from Review 1 remain accepted: unresolved
or file-dependent schema judgments defer, template-bearing `fm.X` reads
remain pending, large inferred shapes have bounded previews, and call-shape
alternatives have a bounded count. The headerless exception withholds a
whole property's verdict when its evaluation depends on an uncontracted
headerless read. These choices need no new ruling.

## Input Robustness

The default outcome for invalid options or invalid document syntax is a
located failure with no composed value. Option and row-shape validation
failures perform no data-content read. Existing `try` recovery of malformed
data remains intentional; the spec explicitly preserves it.

Defined exceptions are unchanged: omitted/null `as` skips the call contract;
a bound null file returns null; omitted/null option members use defaults;
empty options are valid; a null parsed root becomes `[]`; a non-null scalar
root is wrapped; null data members remain data. Explicit empty name lists,
empty names, and duplicate names are invalid. Header-name elements undergo
normal argument conversion before their semantic validation.

Reviewed the shared option binder and `DataOptions::from_bound` rather than
duplicating a matrix for every file extension. The public compose/passive
tests cover absence, null, wrong whole-field types, mixed/all-invalid
elements where conversion cannot accept them, and explicit empty values.
Format, delimiter, and header each have valid controls and invalid cases;
the new explicit-name validator additionally checks empty/duplicate names
and record-width mismatches. Execution cases use property-supplied options
so passive rejection cannot conceal an execution defect.

Reviewed document loading separately: JSON/JSON5, YAML, TOML, JSONL, and
delimited input retain their distinct parser checks. The selected loader
tests exercise duplicate keys where forbidden, trailing/invalid content,
YAML null and multiple documents, JSON5 comments/trailing commas, JSONL
blank lines, and delimited quoting/width errors. This repair introduces no
new document parser.

Searched the loader, option handling, and observation code for
`serde(default)`, `Option<T>`, `filter_map`, `unwrap_or_default`, and `.ok()`.
The option `Option` values encode the specified absence/null equivalence;
the table iterator uses the table's validated row bounds. Optional source
positions do not invent locations. Observation's compatibility `get` can
drop a pending reason, but evaluation uses its fallible `resolve` override,
and reached-pending tests prove recovery cannot manufacture a value. No
additional invalid-input defect was established.

## Verification Results

Fresh runs used Darkmatter's canonical `just test` recipe with explicit
target selection and `BISCUIT_TEST_NARROW`:

```sh
LANG=C.UTF-8 BISCUIT_L1_INCLUDE_SLOW=1 \
BISCUIT_TEST_NARROW='test(load_data) | test(object_spread) | test(editor_observation) | test(observed_validation) | test(schema_slots_editor) | test(shipped_schema_corpus) | test(shipped_corpus_invocation)' \
just test --lib --bins --test l1

LANG=C.UTF-8 \
BISCUIT_TEST_NARROW='test(lookup_parity) | test(slot_jobs::tests) | test(slots::observe_tests)' \
just test --lib --bins --test l1
```

- Main run: **721 passed**, 16,726 excluded by the filter; test execution
  took 3.281 seconds. No individual test reached five seconds.
- Guard/worker run: **19 passed**, 17,428 excluded by the filter; test
  execution took 0.221 seconds. No individual test reached five seconds.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. Reviewed
  consolidated target declarations and instrumentation feature selection.
- `md schema validate` accepts this review with no schema problems. It
  separately warns that the optional user configuration file is absent.
- `git diff --check`: clean.

Two earlier attempts stopped before test execution because the host disk
quota prevented Cargo from creating executable copies. Removed one
rebuildable worktree-local Claudine incremental compiler cache directory;
the focused runs above then completed. These failed build attempts are not
passing test evidence. No production source, test, or shared build-cache
configuration was changed for the review.

Cross-OS evidence remains with CI as requested. There is no terminal or
browser focus interaction in these verification runs.

## Implementation Record Correction

Corrected the outdated present-tense whole-expression admission statement
in `implementation-log.md` in place and added a dated verification note.
It now distinguishes the initial implementation from the repaired behavior:
property observations admit reached operations; schema slots retain their
separate whole-expression admission contract. This is a record correction,
not a finding or an item for the author to approve.

## Recurrence

None. Both earlier defect classes were swept again through their producers
and downstream verdict/shape consumers; no remaining instance was found.
