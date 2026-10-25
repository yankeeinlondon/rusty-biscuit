---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Commit results do not use the promised shared named type"
  - "R1-F2 (high): Call classification loses preserved text representations"
observations: []
human_in_the_loop: null
human_review: true
human_review_items:
  - |-
    After the technical repairs, inspect the additions to `behavior-ledger.md`:
    the rows for `trim`, `keys`, and `values`, and the final reconciliation
    table. Confirm that they accurately describe the implemented behavior and
    record that inspection in the ledger, as acceptance criterion 13 requires.
    The earlier rows were reviewed on 2026-10-22. This is an inspection of the
    completed changes; no settled design choice needs another approval.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T00:43:10-07:00"
spec: 2026-10-28-schema-enhancements/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-28-schema-enhancements/log.md
implemented_by: claude/opus
description: "A **feature** review of `2026-10-28-schema-enhancements/spec.md`"
feature: 2026-10-28-schema-enhancements/review-1.md
next: 2026-10-28-schema-enhancements/review-2.md
---

# Review 1

The feature is not production ready. Two high findings remain: one requested
shared declaration was omitted, and design-time checking rejects valid calls
whose declared input type preserves text. The second defect reaches both
composition and the editor.

## Course of action and decisions

Two independent reviewers inspected the specification and implementation log,
including `decisions_for_author: []`. Neither found an unresolved design choice
meeting the human-decision bar. Historical requests for decisions were settled
by the author's recorded approval on 2026-10-22. The technical review therefore
continued; `human_in_the_loop` is null.

The implementation's main settled choices are:

- Preserve map insertion order and explicitly sort outputs with established
  stable-order contracts.
- Check numeric admission when loading authored numbers and preserve exact
  whole-number presentation.
- Use one conversion engine for frontmatter and calls, bind before entering
  handlers, preserve lazy evaluation, and check returns without conversion.
- Preserve full raw-schema validation and commit successful frontmatter
  conversions per top-level property.
- Derive call argument types from schema declarations, share classification
  between composition and DMLS, and inspect every branch without evaluation.
- Use DMLS's existing settings notification channel and configuration fallback
  behavior, rather than introduce a second settings transport.

These choices do not require reaffirmation. The interpretation of `[true]`
against a list of string-or-number values is also settled: compiled validation
rejects it, while composition may convert the Boolean to text under the shared
conversion rules.

Review decisions, alternatives, reasoning, and downsides are recorded in the
implementation log under **Phase 6 → Review iteration 1: design decisions**.
That record also explains why the author-approved network and identifier
signatures are retained. The pending human ledger inspection is separate from
technical readiness and blocks neither finding's repair.

## Findings

### R1-F1 (high): Commit results do not use the promised shared named type

**Authority — spec:** Function Catalog Migration, item 9: “the function's return
and the variable use one shared named type”; design decision 47 repeats this
requirement. The approved behavior ledger's `recent_commits` row also promises
the shared type. The implementation log's Phase 4 decision 7 explicitly omits
the variable's shared reference because introducing it required fixture updates.
This is narrowed requested scope, which the review instructions classify high.

**Defect class:** two declarations independently own a data contract that the
spec requires to have one named definition.

The function returns `recent-commits`, but the context variable independently
declares `string[]`. Their present expanded shapes agree; that does not deliver
the requested common ownership, and a later change to the named type will not
update the variable.

The bounded sweep covers both declarations, their shared candidate definition,
and the descriptions connecting them:

| Child ID | Site | Shape checked | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | [Context variable declaration](../../../darkmatter/schemas/partials/context-variables.yaml:58) | Shipped `ctx.recent_commits` declaration | Independent `string[](generated; required)` | Reference the shared named type, retaining the variable's annotations |
| R1-F1.2 | [Function return declaration](../../../darkmatter/schemas/partials/functions.yaml:911) | Shipped `recent_commits(count)` signature | `returns(recent-commits)`; clean | Reference the shared type |
| R1-F1.3 | [Named definition](../../../darkmatter/schemas/partials/functions.yaml:66) | Shipped `recent-commits` helper | One `string[]` helper exists; clean | One definition available to both consumers |
| R1-F1.4 | Both descriptions at the sites above | Cross-references between function and variable | Each describes the other and their capture timing; clean | Preserve those cross-references |

This is a declaration-ownership requirement, so its decisive reproduction is
inspection of the shipped declarations, rather than comparing their currently
identical expanded values. The existing catalog and registry tests pass without
asserting the context variable's shared reference.

**Repair:** make both consumers resolve the same named definition, update the
embedded/on-disk schema dependencies and fixture copies together, and add a
guard proving that neither consumer independently owns the element type. This
does not require changing the values callers receive today.

### R1-F2 (high): Call classification loses preserved text representations

**Authority — spec:** “Argument-Type Categories” permits category 5 only when
the input is known to be wrong; a union with some fitting and some nonfitting
values is category 3. “`numberlike` and `boolish`” requires delivery exactly as
sent, including text. Acceptance criteria 36 and 40 require those classifications
and the corresponding editor verdicts. Valid text must not become a proven
error merely because its declared type also admits a native scalar.

**Defect class:** static classification collapses a type with multiple preserved
runtime representations to one primitive family, producing false rejection and
false certainty.

In Darkmatter, [the call classifier](../../../darkmatter/lib/src/markdown/compose/expression/call_check.rs:303)
reports only `Number` for a numberlike source and only `Boolean` for a boolish
source. Its family classification makes the same collapse. Both ignore the
shape's `as_sent` behavior. Consequently, this valid document fails composition:

```markdown
---
$schema:
  src: boolish(required)
src: "1"
---
{{ min(src, 5) }}
```

`md compose` exits 1 with “category 5: no value of that type can bind”. Normal
expression dispatch of `min("1", 5)` returns `1`. The appropriate declaration-only
verdict is category 3: the string `"1"` binds, while a native Boolean does not.
Likewise, `length(src)` rejects a boolish declaration even though `"yes"` is a
valid boolish value whose runtime length is 3.

The same lost representation hides possible content errors: numberlike `"+4"`
and boolish `"yes"` are retained text but are invalid JSON. Those declared sources
are currently classified category 1 against a `json` parameter.

The sweep exercised the public checker with the existing test fixture's one
source declaration and one test-registered parameter, changing one type at a
time. All 35 pairs below were checked. Entries show **observed → expected**
where they differ; a single number means the control was clean. Sources were
required, so these rows isolate representation from optional null handling.

| Source | number | boolean | numberlike | boolish | string | json | yaml |
|---|---|---|---|---|---|---|---|
| number | 1 | 5 | 1 | 5 | 1 | 1 | 1 |
| boolean | 5 | 1 | 5 | 1 | 1 | 1 | 1 |
| string | 3 | 3 | 3 | 3 | 1 | 3 | 3 |
| numberlike | 1 | **5 → 3** | 1 | **5 → 3** | 1 | **1 → 3** | 1 |
| boolish | **5 → 3** | 1 | **5 → 3** | 1 | 1 | **1 → 3** | 1 |

The downstream decision sweep follows that classification through admission,
recovery, composition, and publication to the editor:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | `fit` / `family` in Darkmatter's call classifier | Cross-family numberlike/boolish pairs above | Four category 5 results | Category 3; retained `"1"` can bind |
| R1-F2.2 | Same classifier, content targets | Both preserved-text types → `json` | Category 1 | Category 3; some admitted text is invalid JSON |
| R1-F2.3 | `source_kinds` / `arm_fit` in the classifier | `length(src)` with required boolish source | Category 5 through Boolean exclusion | Category 3; exclude native Boolean, permit retained text |
| R1-F2.4 | Public runtime `evaluate(parse(...))` | `min("1",5)`; `length("yes")` | `1` and `3`, one handler entry each; clean | Successful binding and computation |
| R1-F2.5 | Whole-call recovery | `number(src,7)` with boolish source | Category 1; clean | Valid fallback prevents rejection |
| R1-F2.6 | Composition precheck, shipped `md compose` | Document above, versus a numeric declaration and a literal call | Boolish document exits 1; both controls exit 0 and print `1` | Boolish document also composes; uncertain is not rejected |
| R1-F2.7 | DMLS published diagnostics over an LSP session | Boolish source passed to `min`, `max`, `abs`, and `length` | Four `dm.expression.incompatible_call` errors | Four category 3 warnings |
| R1-F2.8 | Same DMLS session | Numberlike source → `min`; recovered call; literal numeric text → `min` | No call diagnostics; clean | Preserve these successful controls |
| R1-F2.9 | Public classifier controls | Ordinary number/Boolean/string rows and remaining pairs above | Expected categories; clean | Retain the existing guaranteed fits and incompatible native-only pairs |

All production category decisions come from this shared checker; composition's
rejection and DMLS's severity mapping consume its answer. Their common false
answer explains why the existing agreement test passes while valid documents
still fail. The existing tests cover the guaranteed same-family conversions,
but omit these cross-family preserved-text cases.

**Repair:** retain both possible representations when checking conversions and
source exclusions. Preserve the spec's proven conversions, such as numberlike
to number, rather than making every use unknown or changing runtime conversion.
Add focused public checker cases and composition/editor controls for the class.
The repair can stay in the existing classifier; no new input-protection layer
or document syntax is needed.

## Observations

None.

## Verification and limits

The review ran `just test` in `darkmatter/`, narrowed to the schema grammar,
numeric admission, reader matrices, coercion core and translators, call binder
and registry, catalog, composition precheck, expression settings, editor call
diagnostics, and composition/editor agreement modules: **481 passed, 0 failed**.
The longest selected test took 2.652 seconds.

Two temporary review probes also ran through nextest: the public classifier and
runtime matrix, and a DMLS session publishing the false errors. Both reproduced
the reported behavior. They asserted the observed defect, not the desired
repair. They were removed before the existing-suite run; no production code or
test changes remain from these probes. Separate temporary Markdown files proved
the composition failure and successful controls through the shipped CLI.

| Requirement group | Verification reviewed | Appropriate level |
|---|---|---|
| Grammar, catalog identity/examples, numeric admission | In-process public APIs, loader matrices, executable examples | Level 1 |
| Conversion, binding, overloads, recovery, laziness, returns | Public API tests and handler-entry counters | Level 1 |
| Composition rejection, effect ordering, authored surfaces, child documents | Composition API and hermetic CLI tests | Level 1 |
| Editor categories, locations, settings, reload, agreement | In-process checks and LSP sessions | Level 1 |
| Shared commit-result definition | Declaration inspection; missing ownership guard is part of R1-F1 | Level 1 guard needed |
| Behavior-change ledger inspection | Author's earlier inspection recorded; closing additions pending | Human inspection required by criterion 13 |

The declared `l1` targets compile the reviewed test modules despite
`autotests = false`; the package's local and CI features include the required
instrumentation. The configuration and schema-reader matrices distinguish
absence, null, wrong types, and empty values, and test duplicate keys and invalid
document content through their loaders. The deliberate positional editor-section
form and whole-overlay fallback are recorded, settled behavior. No additional
reader finding arose from the requested parse-smell sweep.

This feature does not introduce a keyboard, terminal-emulator, or browser-input
contract requiring Level 2 or Level 3. This review did not rerun the entire
workspace, lint, or cross-OS gates; cross-OS evidence is not a readiness finding.
There are no previous review findings to carry forward in iteration 1.
