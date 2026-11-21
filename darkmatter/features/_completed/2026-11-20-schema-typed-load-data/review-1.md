---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Unused branches prevent editor observation of concrete results"
    - "R1-F2 (high): Headerless editor handling follows syntax instead of executed read options"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-21T01:51:38+00:00
spec: 2026-11-20-schema-typed-load-data/spec.md
implemented: true
description: "A **feature** review of `2026-11-20-schema-typed-load-data/spec.md`"
feature: 2026-11-20-schema-typed-load-data/review-1.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-20-schema-typed-load-data/log.md
implemented_by: claude/opus
next: 2026-11-20-schema-typed-load-data/review-2.md
---

# Review 1: Schema-Typed Load Data

The feature is not production ready. Two editor behaviors violate the spec:
unused branches suppress checks of values the editor can evaluate, and the
headerless exception changes according to how equivalent options are written.
Both have a repair within the existing decisions; neither needs an author ruling.

## Findings

### R1-F1 (high): Unused branches prevent editor observation of concrete results

**Authority: spec**, “Evaluation follows ordinary branch selection,
short-circuiting, and `try` recovery.” The implementation instead rejects a
whole expression when an unused branch contains an unavailable binding or
unsupported function. This is the defect class of deciding execution
availability from syntactic dependencies that evaluation never reaches.

In the Darkmatter library,
[`property_reads`](../../lib/src/markdown/schemas/slots/admission.rs:242)
walks both branches and callback bodies. Then
[`Pass::evaluate`](../../lib/src/markdown/schemas/slots/observe.rs:327)
refuses blocked operations and binds every collected property before running
the expression. Either step can prevent an authorized, selected load from
being observed. In DMLS this removes its destination errors and inferred
members, even though compose reports the destination error.

Reproduction: use the spec's people data with Grace's age changed to `old`,
approve `./data.yaml`, declare `rows` as
`{ name: string(required), age: number }[]`, and replace its whole-value
expression with each expression below. The public observation API was checked
with the valid people fixture; the DMLS protocol and compose comparison used
the bad-age fixture. No unused operation needs to run.

| Child ID | Site and expression shape | Observed result | Expected result |
|---|---|---|---|
| R1-F1.1 | Ternary admission: `true ? load_data('data.yaml') : ctx.runtime`; also reversed arms with a false condition | Public result is pending for `ctx`; DMLS gives no destination error | Observe the selected load; report its bad age |
| R1-F1.2 | Logical fallback admission: `load_data('data.yaml') \|\| ctx.runtime` | Pending; zero editor destination errors, one compose error | Use the successful load without requiring the fallback |
| R1-F1.3 | Recovery argument scan: `try(load_data('data.yaml'), ctx.runtime)` | Pending; zero editor destination errors, one compose error | Use the successful load without evaluating recovery |
| R1-F1.4 | Function admission in an unused arm: `true ? load_data('data.yaml') : find_files('*.yaml')` | Pending for unsupported `find_files()`; zero editor destination errors, one compose error | Observe the load without executing a listing |
| R1-F1.5 | Eager property binding: `true ? load_data('data.yaml') : missing_property` | Pending for the absent property; zero editor destination errors, one compose error | Do not bind the unused property; observe the load |
| R1-F1.6 | Callback-body admission: `map([], i => ctx.runtime) \|\| load_data('data.yaml')` | Public result is pending for `ctx`, although the callback cannot run | Evaluate the empty map and ordinary fallback without requiring `ctx` |
| R1-F1.7 | Control: `true ? load_data('data.yaml') : []` | Observed; DMLS and compose each report one bad-age error | Same; clean |
| R1-F1.8 | Guarded local read: existing `recovery_turns_only_a_proven_miss_into_the_fallback` test, `false ? load_data('gone.yaml') : 'other'` | Observes `other` with no reads | Same; clean |
| R1-F1.9 | DMLS verdict and member consumers | A pending outcome cannot enter `replacements` or `shapes` in `slot_jobs::observe`; diagnostics publish nothing for pending outcomes | These consumers correctly withhold genuinely pending results; fix the upstream false deferral |

The sweep covers ternaries, logical fallback, recovery, callback bodies,
function admission, and dependency binding. Ordinary evaluation already
selects branches correctly; the additional editor preparation is the source
of the mismatch. The existing DMLS test
`unsupported_runtime_only_and_cyclic_properties_are_pending` also enshrines
the wrong behavior for a successful load followed by unused `as_markdown`.

**Recommended repair:** authorize functions and resolve bindings when
evaluation reaches them. Keep the passive authored-call rejection separate:
provably invalid calls must still fail before execution, as the spec requires.
An executed unsupported operation must remain pending before it performs any
effect. Add public observation and DMLS regressions for the failing rows,
including the unused recovery argument and empty callback case. Update the
comments and guides that describe whole-expression admission as intended.

### R1-F2 (high): Headerless editor handling follows syntax instead of executed read options

**Authority: spec**, “For `header: false` without `as`, publish
`unknown[][]`; DMLS skips cell-by-cell schema checking through that
uncontracted read,” together with ordinary branch selection and normal
argument binding. Equivalent executed reads currently receive different
verdicts. This is the defect class of deriving a semantic read mode from an
incomplete scan of authored syntax.

The Darkmatter library's
[`reads_headerless_uncontracted`](../../lib/src/markdown/schemas/slots/observe.rs:190)
recognizes only a direct object field containing a literal boolean and an
unparenthesized literal null contract. It also searches every branch rather
than the calls that ran. DMLS uses it both to exclude whole properties from
[`observe`](../../dmls/src/slot_jobs.rs:828) and to exclude call shapes in
[`observed_call_shapes`](../../dmls/src/slot_jobs.rs:874).

Reproduction: approve `./table.csv`, write `1,2\n3,x\n`, and declare
`cells: "tuple([number, number])[]"`. Run the same property through the
normal DMLS open/background-observation path, changing only the third argument
of `load_data('table.csv', null, options)`. The first row below is the
positive control for the required deferral, rather than evidence that the bad
cell is valid.

| Child ID | Site and shape tested | Observed result | Expected result |
|---|---|---|---|
| R1-F2.1 | Property classifier, literal `{header: false}` | No editor cell error | Same; clean |
| R1-F2.2 | Property classifier, `{header: 'false'}` | `dm.schema.type_mismatch` at `/cells/1/1` | Binding converts this to false; no editor cell verdict |
| R1-F2.3 | Property classifier, `opts` with authored `opts: {header: false}` | Same cell error | Same headerless deferral as the literal control |
| R1-F2.4 | Property classifier, `{...{header: false}}` | Same cell error | Same headerless deferral after object spread |
| R1-F2.5 | Property classifier, selected headed call with an unused headerless arm: `true ? load_data('table.csv') : load_data('table.csv', null, {header: false})`, under `rows: "{ a: number, b: number }[]"` | No destination error | Judge the selected headed result and report its schema mismatch |
| R1-F2.6 | Headed control, `load_data('table.csv')` under the same object-row declaration | Existing protocol test reports a destination error | Same; clean |
| R1-F2.7 | Per-call shape classifier in `observed_call_shapes` | The same syntax predicate admits variable/spread/string-false calls into `Shape::observed`; it excludes the direct literal-false control | Preserve provisional cell shapes for every executed uncontracted headerless read |
| R1-F2.8 | Passive return typing, `uncontracted_type` in `call_check.rs` | Direct literal false produces unknown nested arrays; other unprovable options stay a general container | General types are a safe passive fallback; once binding is known, use the actual header mode consistently |

Rows 1–6 were reproduced through the DMLS protocol. Rows 7–8 follow the
classification into every shape decision: the per-call consumer directly
reuses the defective predicate, while the passive checker safely defers
unknown options. The normal loader already binds all four false-option forms
to positional rows. The missing semantic information is in the editor's
classification, not the CSV parser or schema engine.

The same detector also misses a parenthesized or computed null contract and
does not account for a later spread overriding an earlier `header` field.
Include those forms in the repair's bounded tests rather than adding more
special cases to the syntax scan.

**Recommended repair:** retain whether an executed, bound load was
uncontracted and headerless with its observation and call facts. Use those
facts for destination checks and inferred call/member shapes. Only executed
reads should trigger this exception. This reuses information the loader
already has and avoids a second option interpreter. Add the equivalent-option
and unused-branch protocol tests, plus a callback completion/hover check proving
that headerless cells remain provisional.

## Implementation decisions (informational)

Two agents independently read the complete spec and implementation log,
including `decisions_for_review`. Neither found a separate requirement for an
author ruling or a general autonomy clause. The following inventory includes
the choices and deferred work they identified. None requires a human stop.

| Decision or deferred work | Review disposition |
|---|---|
| Adapt shared expression consumers in Phase 1, temporarily using general object typing and generic spread errors | Reasonable sequencing; temporary behavior was replaced in Phase 2 |
| Cache-only remote registration creates neither tasks nor slots; callers use the cached-text API | Keep; it preserves pending results and prevents network activity |
| Runtime invalid spread is fatal, including body interpolation, and cannot be recovered with `try` | Keep; consistent handling of an object that cannot be constructed |
| Carry passive spread errors through call-check records named `...` | Keep; internal reuse avoids duplicate reporting paths |
| Passively reject only operands whose every alternative excludes object and null; report JSON type names | Keep; uncertain values are checked during evaluation |
| Use a scoped thread-local last-value origin instead of per-node provenance records | Keep; direct loads, parentheses, and selected conditional arms retain their origin; derived values clear it |
| Preserve existing malformed-data recovery through `try`, despite contradictory plan wording | Keep; the spec explicitly preserves current failure behavior |
| Map wrapped scalar failures to the source-root pointer, including nonexistent tuple positions | Keep; the original source has no array element to locate |
| Locate unexpected members at their values instead of keys | Keep; consistent with existing contract diagnostics |
| Show an additional linked data-location line and use the authored expression as primary | Keep; satisfies both-location reporting without changing error identity |
| Narrow passive tabular return types only from literal format/header values | Safe provisional fallback; observed header-mode handling still needs R1-F2 |
| Admit the entire editor expression before selecting branches | Requires repair under R1-F1; the spec supplies the answer |
| Keep templated `fm.X` pending while bare property dependencies are evaluated | Accept the conservative authored-seed treatment; do not invent a result from template text |
| Leave parse-failed observations pending because passive checks report syntax failures | Keep; avoids duplicate errors |
| Detect file-dependent declarations from compiled JSON Schema; defer unresolved references or depth beyond 32 | Keep; supports raw JSON Schema without unauthorized file checks |
| Defer linked root declarations when sibling templates/file rules remain; attribute unowned root failures to all observed properties | Keep the conservative deferral allowed by the spec; finer dependency analysis is unnecessary here |
| Use observed shapes for hover/completion while member diagnostics retain declared types; required-member failures are always errors | Keep; avoids treating absent optional members as undeclared |
| Match observed calls by written expression; merge up to eight distinct values, then leave provisional | Keep; bounded internal inference with safe loss of precision |
| Reject stale read results for all slot jobs, extending existing schema jobs too | Keep; consistent current-result installation |
| Add a dev-enabled `cache-test-hooks` feature rather than expose cache internals | Keep; a narrow test seam |
| Use `darkmatter.compose` for missing/evaluation failures, zero-width related data ranges, and raw headerless previews | Keep; schema failures retain their own schema source |
| Inline the unresolved type in the unrelated terminal research document, dropping field descriptions | Accept the small corpus repair; no public contract or new schema file is needed |
| Clarify generated header-mode wording so an explicit contract is optional | Keep; removes a misleading implication |
| Summarize/link the loading guide in the caller skill rather than transclude it | Accept; the summary is small and points to the canonical guide |
| Leave serialization of open inferred pattern keys unchanged, testing openness through member behavior | Accept; internal display precision does not undermine the required open-member behavior |
| Defer header detection and array spread | Already outside the spec's scope |
| Leave more precise root-error attribution, deeper unresolved declarations, and inference beyond eight alternatives provisional | Accept bounded deferral; no unjustified schema verdict is published |
| Leave cross-OS evidence to CI and run locale-sensitive gates with UTF-8 | Repository policy already decides this; no author action |
| Record unrelated test-toolkit duration-measurement and real-resource test issues | No new verification of those unrelated tests was performed in this review |

The early delays of typed spread errors, ordered typing, spread documentation,
and the shipped-document corpus repair were completed in later phases. They
are not remaining deferred work. No factual implementation-record correction
was needed for the behavior verified here; the disputed whole-expression
choice is accurately recorded and is an ordinary finding.

## Verification and input robustness

All requirements here concern parsing, evaluation, validation, diagnostics,
and language-server responses. **Level 1 is appropriate**: actual terminal
rendering, keyboard encoding, and browser interaction are not acceptance
requirements for this feature.

| Requirement | Strongest relevant verification reviewed |
|---|---|
| Worked example, isolated compose failures, direct/parenthesized/conditional source locations | L1 public compose API and CLI data/report tests in `load_data_destination`, CLI `load_data`, and DMLS `typed_load_data_editor` |
| Enum membership, conversion, defaults, closed objects, imported and trigger declarations | L1 public effective-schema validation, compose, and DMLS parity cases |
| Optional/null contracts, null files, scalar/null normalization, memo reuse | L1 public compose results and runtime work counters in `load_data_uncontracted` |
| Header modes, normalized keys, explicit names, width and cell positions | L1 public compose/CLI tests; editor semantic gap is R1-F2 |
| Missing/recovered/guarded reads, grants, dependent buffers, stale jobs, remote cache-only policy | L1 public observation and in-process LSP protocol tests; branch gap is R1-F1 |
| Spread order, duplicates, null and invalid operands, interpolation closing, scanning overwritten operands | L1 parser/public evaluation, compose, DMLS, and recorded Claudine consumer tests |
| Shipped definitions and normal invocation | Passive shipped-schema corpus and shipped-corpus invocation tests |

For file-format inputs, invalid syntax/options normally return a located
failure and no value. Absence and null are intentionally equivalent for
optional `as` and option members; a null file short-circuits. Absent/null
`header` selects the default; an empty explicit name list is invalid. Loaded
null roots become empty arrays, non-null scalar roots are wrapped, and data
null members remain data for schema judgment. Those are defined exceptions,
not accidental defaults.

The existing shared option matrix exercises absent/null, booleans, lists,
wrong whole-field types, mixed/all-invalid elements, empty lists/names, and
duplicate names through the binding/compose boundary. Each loader retains its
strict syntax checks: YAML duplicate keys and multiple documents, JSON trailing
content, JSON5 syntax, TOML duplicates, JSONL blank lines, and delimited
quoting/width. The added work does not introduce a new format parser. Inspected
the changed data-loading/options code for `serde(default)`, `Option<T>`,
`filter_map`, `unwrap_or_default`, and discarded parse errors; no additional
silent-invalid-input finding was established. R1-F2 is the downstream
classification defect after valid options have been bound.

Temporary tests extended the existing declared L1 targets, exercised public
observation and the in-memory LSP server, and printed the actual outcomes.
Their passing status means the reproductions ran; it does not mean the
observed wrong behavior meets the spec. The final reproduction run reported
16 selected tests, none at or above five seconds; the three temporary sweep
tests took 0.031, 0.223, and 0.226 seconds. They were removed after evidence
collection. No production code was changed by the review.

The focused existing-suite run used the canonical Darkmatter `just test`
recipe with `BISCUIT_L1_INCLUDE_SLOW=1` and `BISCUIT_TEST_NARROW` selecting
load-data, spread, observations, schema-slot editor, and shipped-corpus tests.
Final run results are recorded below.

- Final existing-suite run: **672 passed**, 16,732 skipped by the explicit
  filter; no test at or above five seconds. Test execution took 3.933 seconds;
  the canonical recipe took 26 seconds including compilation and recording.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. The added
  implementation test modules are declared by their consolidated L1 targets.
- `md schema validate` of this review with JSON output: **valid**, no schema
  problems. The CLI also reported the host's missing optional user config.
- `git diff --check`: clean. Both temporary-test files were restored byte for
  byte; this review leaves only the review document and the requested spec
  review-iteration update as its changes.

The final test invocation, from `darkmatter/`, was:

```sh
BISCUIT_TEST_NARROW='test(load_data) | test(object_spread) | test(editor_observation) | test(observed_validation) | test(schema_slots_editor) | test(shipped_schema_corpus) | test(shipped_corpus_invocation)' \
BISCUIT_L1_INCLUDE_SLOW=1 LANG=C.UTF-8 just test
```

This is the first implementation review, so there are no earlier finding
outcomes or recurrence to report. Cross-OS proof remains a CI responsibility
and does not affect this readiness decision.
