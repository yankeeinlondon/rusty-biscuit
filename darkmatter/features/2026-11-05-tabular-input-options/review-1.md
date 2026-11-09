---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Passive option checks ignore shared coercion results"
  - "R1-F2 (high): Computed contracts suppress provable option applicability errors"
human_review: true
human_review_items:
  - "Informational: retain the prerequisite's raw-entry hash and per-call observation model; no action is required unless the author wants to revisit it."
  - "Informational: keep the slot-read gate immediately after resolution; no action is required."
  - "Informational: temporary private-model dead-code scaffolding was removed after wiring; no action is required."
  - "Informational: unreachable post-binder shape failures return an options error rather than panic; no action is required."
  - "Informational: the catalog now describes explicit format selection rather than extension-only selection; no action is required."
  - "Informational: stale memo generations propagate through dependent computations to prevent stale cache fills; no action is required."
  - "Informational: binder errors take precedence over additional option-semantic errors; retain this precedence while repairing the passive checks."
  - "Informational: provable literal mistakes are rejected even beside a null file; no action is required."
  - "Informational: current guidance uses supported tuple-array contracts rather than adding nested-array grammar; no action is required."
  - "Informational: the raw memo helper follows clippy naming and the unused source-key accessor was omitted; no action is required."
  - "Informational: tests reach private gates and invalidation through public behavior instead of exposing internals; no action is required."
  - "Informational: diagnostic parity tests compare actual execution with passive checking; no action is required."
  - "Informational: existing authorized URL prefetch can precede unsupported-format rejection; changing that prefetch rule was deferred and is not required for this review."
  - "Informational: editor providers share expression-location logic; no action is required."
  - "Informational: completion recovery, quoting, and filtering use the implementation's recorded choices; no action is required."
  - "Informational: passive CLI failures retain null span, authored line/column, and an empty read set; no action is required."
  - "Informational: the Options documentation follows CSV and TSV; no action is required."
  - "Informational: headerless position tests were split into independent cases; no action is required."
  - "Informational: the author's accepted computed-option location, effective-TSV, alias-completion, and repeated-observation decisions remain in effect; no further confirmation is requested."
  - "Decision (R1-F2): a provably bad computed contract beside inapplicable literal options now reports dm.data.invalid_options passively, though execution would report the contract error first; confirm or prefer a different precedence."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-08T20:04:58-08:00
spec: 2026-11-05-tabular-input-options/spec.md
implemented: true
description: "A **feature** review of `2026-11-05-tabular-input-options/spec.md`"
feature: 2026-11-05-tabular-input-options/review-1.md
next: 2026-11-05-tabular-input-options/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-tabular-input-options/log.md
implemented_by: claude/opus
---

# Review of Tabular Input Options — iteration 1

The feature is **not production ready**. Two passive-validation gaps let
provably invalid calls pass composition's initial check and the editor's
error check. Execution rejects the same calls when reached. Both repairs
fit the existing spec and require no human ruling.

Reviewed the spec, plan (including the author's November 8 decisions),
implementation log, feature code changes, current documentation, and tests.
Existing working-tree changes were preserved. This review changes only this
file and the spec's review-iteration metadata.

## Findings

### R1-F1 (high): Passive option checks ignore shared coercion results

**Authority: spec**, “Use the shared function binder and coercion engine for
this object, including normal conversions to its declared field types,”
“The delimiter constraint is checked on the bound string,” and “Preserve the
existing rejection of provably invalid calls in dead branches.” The passive
check instead examines the authored JSON types after asking only whether
binding succeeds. A literal `header: "false"` binds to `false`, but its
headerless object-row conflict is missed; a delimiter `10` binds to `"10"`,
but its invalid length is missed.

Defect class: passive semantic validation uses values before coercion while
execution uses values after coercion.

In darkmatter's [passive data check](../../lib/src/markdown/compose/expression/call_check.rs:1165),
`OptionsView::literal` supplies authored values, and the delimiter/header
branches accept only `Value::String`/`Value::Bool` respectively. Even the
fully literal call's successful binder result is not used here. The
[execution loader](../../lib/src/markdown/compose/expression/data/load.rs:103)
receives bound values and correctly rejects these cases.

Reproduction: create `rows.csv` containing `name,port\nweb,8080\n`,
`rows.json` containing `[]`, and a tab-separated equivalent `rows.tsv`.
Put a call below into `{{ false ? CALL : null }}` and run
`md compose doc.md --output json`. The broken cases exit 0 with no data
error; using the same call directly exits 1 with the expected code.
These are small variations of the shipped headerless and options fixtures,
including the string-to-boolean conversion the new tests expressly support.

| Child ID | Site / shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F1.1 | Passive headerless-row decision: CSV object contract, `{header: "false"}` | Dead branch accepted | `dm.data.row_shape_conflict` at `as` |
| R1-F1.2 | Passive delimiter validator: CSV tuple-array contract, `{delimiter: 10}` | Dead branch accepted | `dm.data.invalid_options` at options |
| R1-F1.3 | Passive non-tabular applicability: JSON `string[]`, `{header: "true"}` | Dead branch accepted | `dm.data.invalid_options` at options |
| R1-F1.4 | Passive TSV applicability: tuple-array contract, `{delimiter: 5}` | Dead branch accepted | `dm.data.invalid_options` at options |
| R1-F1.5 | Partial object projection: CSV object contract, `{header: "false", delimiter: sep}`, frontmatter `sep: ","` | Only an uncertain-call warning; no row conflict | Reject the provable headerless object-row conflict |
| R1-F1.6 | DMLS diagnostic projection, the four fully literal cases above | No error diagnostics in an in-process editor session | Same data errors and argument ranges as composition |
| R1-F1.7 | Runtime binding, all five failing shapes above | Correct data errors when executed | Clean; retain behavior |
| R1-F1.8 | Passive native-value controls: `{header: false}` on CSV objects and `{header: true}` on JSON | Correctly rejected | Clean |
| R1-F1.9 | Valid coerced control: CSV tuple-array contract, `{header: "true"}` | Accepted and executes successfully | Clean |
| R1-F1.10 | Format field and its alias/default decisions | Accepted literals are already strings; canonicalization is shared, and the binder rejects unbindable format names | Clean by the declared enum contract and existing alias/default tests |

This sweeps the three fields and the downstream decisions that consume
header/delimiter: delimiter validity, non-tabular applicability, TSV's tab
restriction, and positional-row classification. The partial-object path
also retains raw literals and needs the same correction. Fully computed
values should still defer to execution.

**Recommended repair:** bind each provable literal member through its
catalog-declared field validator before semantic checks, preserving null as
absent and the binder's error precedence. Reuse those bound values in all
four decisions, including partial objects. Add public passive-check and
DMLS regressions plus a dead-branch composition assertion; keep the valid
coercion controls. Do not introduce native-type restrictions.

### R1-F2 (high): Computed contracts suppress provable option applicability errors

**Authority: spec**, “Check literal option values even when another argument
is computed,” and “If the format is not provable, defer format-specific
checks.” Here the format and invalid options are provable; only the contract
is computed. The spec also requires rejecting provably invalid calls in
dead branches.

Defect class: an unrelated unknown argument stops validation of an
independently decidable option constraint.

Darkmatter's [passive data check](../../lib/src/markdown/compose/expression/call_check.rs:1181)
returns `None` when `as` is not a literal carrier, before it reaches
`provable_format` and `check_applicability`. Neither JSON's prohibition on
`header`/`delimiter` nor TSV's tab-only rule depends on the contract.

Reproduction: use frontmatter `contract: "string[]"` and the expression
`{{ false ? load_data("rows.json", contract, {header: true}) : null }}`.
`md compose doc.md --output json` exits 0 without diagnostics. Remove the
dead-branch wrapper and it reports `dm.data.invalid_options`. Computed
contracts are a normal use of the data/type features, rather than malformed
input.

| Child ID | Site / shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F2.1 | Extension-selected JSON, computed contract, `{header: true}` | Dead branch accepted | Invalid options at the third argument |
| R1-F2.2 | Extension-selected JSON, computed contract, `{delimiter: ";"}` | Dead branch accepted | Invalid options at the third argument |
| R1-F2.3 | Extension-selected TSV, computed contract, `{delimiter: ";"}` | Dead branch accepted | Invalid options at the third argument |
| R1-F2.4 | Explicit JSON override of `rows.csv`, computed contract, `{header: true}` | Dead branch accepted | Invalid options at the third argument |
| R1-F2.5 | Explicit `yaml`, `yml`, `json`, `jsonl`, `ndjson`, `toml`, `json5`, computed contract, `{header: true}` | Every dead branch accepted | Invalid options for every non-tabular format |
| R1-F2.6 | DMLS projection of extension-selected JSON header/delimiter and TSV delimiter cases | No error diagnostics | Same options error and third-argument range |
| R1-F2.7 | Runtime applicability for the extension-selected and override reproductions | Correct errors when executed | Clean; retain behavior |
| R1-F2.8 | Format-independent delimiter check, computed contract, `{delimiter: ";;"}` | Correct passive rejection | Clean |
| R1-F2.9 | Computed contract with null header/delimiter; explicit CSV/TSV with `header: true` | Accepted | Clean: null is absent and these tabular options apply |
| R1-F2.10 | Whole computed options or computed format | Existing tests defer format-specific judgment | Clean: the format can actually change |

The sweep covers both ways of proving the format (extension and override),
all non-tabular formats and aliases, both forbidden non-tabular keys, TSV's
restriction, the early delimiter validator, runtime validation, and editor
projection. Contract-dependent row-shape checks correctly remain provisional
when the contract cannot be classified without execution.

**Recommended repair:** collect whatever contract judgment is available
without returning early for an unknown contract. Continue the independent
format and applicability checks, then run row classification only when a
contract is known. Preserve existing precedence when a literal contract is
provably invalid. Add computed-contract cases to the passive, dead-branch
composition, and DMLS tests.

## Verification and input robustness

All requirements in this feature need **Level 1** evidence: file parsing,
public composition results, memo work counts, local HTTP mocks, CLI JSON
output, and headless editor protocol behavior. The spec explicitly requires
no real-terminal or browser test. There is no keyboard or rendered-terminal
requirement that would need a higher level.

| Requirement group | Present verification | Review outcome |
| --- | --- | --- |
| Headerless CSV/TSV, one record, blanks, ragged input, quoting | `load_data_headerless` public composition tests, L1 | Passed |
| First/later row locations, multiline, multibyte, CRLF, BOM, missing cells | `load_data_headerless` and locator tests, L1 | Passed |
| Defaults, aliases, explicit format, supported-extension override, custom delimiter | `load_data_options`, L1 | Passed |
| Literal/computed option errors, no reads, null file, imported contracts | Public option/passive tests, L1 | Existing tests pass; gaps in R1-F1 and R1-F2 |
| Raw/parse/bind reuse, failures, ordering, invalidation, concurrency, slot gates | Runtime and public options/slot tests, L1 | Passed in the load-data selection; inspected three-layer generation tests |
| Remote extensionless input, denied/warmed hosts, redirects, reference-check deferral | Local HTTP mocks, L1 | Passed |
| Editor completion, hover, diagnostics, callback types, nullability | In-process DMLS protocol tests, L1 | Passed existing cases; missing rejection cases reproduced |
| Catalog examples/generated page and normal CLI success/error envelopes | Catalog and CLI tests, L1 | Passed selected load-data tests |

Executed `just test load_data` in `darkmatter/`: **453 passed**, no selected
test reached five seconds. Executed `just test option_arguments`: **12
passed**. A temporary in-process DMLS probe also confirmed all seven tested
invalid calls lacked error diagnostics; the probe was removed afterward.
Direct shipped-CLI reproductions confirmed the instance tables, including
positive controls and the difference between dead and executed calls.
These are targeted review runs; the implementation log separately records
its full `just test` and `just lint` runs. macOS linker warnings about a large
unwind section did not fail these runs. No cross-OS evidence is claimed.

The new options object is expression syntax, so the configuration-file
robustness matrix does not apply to that syntax. Its semantic field shapes
were nevertheless checked against the public-result tests: omission, null,
and empty object mean defaults; invalid object/field shapes are binder
errors; supported scalar conversions are intentional. Existing tests cover
empty/invalid format strings and delimiter strings (empty, multiple
characters, non-ASCII, quote, CR, LF), null/default fields, and invalid
whole-field types. The missing conversion-dependent cells are R1-F1;
computed-contract suppression is R1-F2. No new collection-valued option
needs an element-shape matrix.

The underlying data loaders retain their existing document-validation
branches; this feature changes effective format/delimiter selection and
logical table-row projection. Reviewed loader changes and existing malformed
input/diagnostic tests. The changed option path introduces no serde-default,
filtered-element, or swallowed-parse fallback. `Option` is intentional here:
the spec explicitly defines absent and null options to be equivalent.

## Implementation decisions (informational)

The two requested parallel readers independently inventoried the log and
spec. None of the decisions below requires a human before repairs can
proceed: each is already approved, follows an explicit non-goal, or has a
recommended choice within the spec. The author may revisit these after the
fix cycle; confirmation is not required to continue.

1. **Raw observations:** keep the prerequisite's actual model: a hash on the
   raw entry and one `observe_read` per call, using no hash on read/parse
   failure. This preserves existing behavior and repeated dependency tracking.
2. **Slot gate order:** retain the gate immediately after resolution, before
   read-set recording and format selection. It preserves the prerequisite's
   trust boundary before consulting any memo layer.
3. **Temporary scaffolding:** use `expect(dead_code)` while the private
   options model was unwired, rather than widening the API. The temporary
   attribute is gone in the completed implementation.
4. **Defensive internal errors:** return `InvalidOptions` with key `options`
   for impossible post-binder whole-object/unknown-key failures. Recommend
   retaining the fallible path rather than introducing a panic or extra type.
5. **Catalog wording:** replace extension-only wording with the actual
   override behavior and regenerate the derived pages.
6. **Memo generations:** propagate the lower layer's old generation through
   dependent computations. Recommend retaining the change and its regression
   test; otherwise stale reads could refill fresh maps.
7. **Binder precedence:** when options cannot bind, retain the binder's error
   rather than replacing it with another semantic error. R1-F1 corrects use
   of successful binding results, not this precedence decision.
8. **Literal null files:** reject provable literal option/row mistakes even
   beside a null file. Computed options still honor null-file early return.
9. **Unsupported nested-array examples:** suggest `tuple([…])[]`, since
   `string[][]` is outside the existing grammar. Adding that grammar was
   deferred because the spec excludes grammar changes. The supported tuple
   alternative fulfills headerless positional loading without new syntax.
10. **Internal naming:** use `answered_from_memo` for the raw entry and omit
    an unused source-key accessor. Cheap internal choices; no public API added.
11. **Public-path tests:** use approval handlers, a real shell stage, and
    public preflight/discovery paths instead of exposing private gates,
    invalidation, and collectors. Retain this coverage; no API exposure needed.
12. **Message comparisons:** compare passive checking with actual computed
    execution rather than manually rebuilding a private error message.
13. **Existing URL prefetch:** retain the existing authorized prefetch that
    can download an extensionless URL before unsupported-format rejection.
    Skipping such downloads was deferred as a separate prefetch rule. The
    failed source is not interpreted, and this decision grants no network
    access. The spec does not require zero requests merely for a missing
    extension; no blocking finding is warranted for that behavior.
14. **Shared editor positioning:** extract expression-location logic into
    `expression_text` for both completion providers instead of duplicating
    YAML projection handling. Internal reuse is appropriate.
15. **Completion recovery:** offer nothing after a key awaiting `:`, let
    ordinary completion handle quoted partial keys, insert unquoted enum
    text inside an existing quote and quoted text outside it, and filter by
    inserted text. These are reversible editor choices with protocol tests.
16. **Literal CLI diagnostic shape:** retain `span: null` plus actual
    authored line/column and an empty read set for passive failures. General
    call-check span changes were deferred because argument location already
    exists and preflight rejection does not read the source.
17. **Documentation organization:** put Options after CSV and TSV. No
    behavior or public interface is fixed by this layout choice.
18. **Independent position tests:** split the combined later-row,
    multiline, multibyte, and CRLF check into independently named tests.
19. **Previously approved decisions:** the plan's November 8 author decisions
    explicitly accept whole-call locations for computed-option failures,
    applying TSV's restriction to effective format, offering aliases in
    completion, and consulting raw entries on each call for observations.
    They need no repeated ruling. Per-argument runtime location work remains
    omitted under that accepted decision.

No required implementation phase was declared deferred. The omitted grammar,
general diagnostic-span work, private test APIs, and prefetch optimization
above do not require moving scope to another spec. There are no earlier
implementation-review findings to carry forward in iteration 1.
