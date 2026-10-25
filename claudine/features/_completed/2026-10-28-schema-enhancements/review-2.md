---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: true
human_review_items:
  - |-
    Inspect the additions to `behavior-ledger.md`: the rows for `trim`,
    `keys`, and `values`, and the final reconciliation table. Confirm that
    they describe the implemented behavior and record the inspection in the
    ledger before merge, as acceptance criterion 13 requires. The earlier
    rows were reviewed on 2026-10-22. No settled design decision needs
    another approval.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T01:27:00-07:00"
spec: 2026-10-28-schema-enhancements/spec.md
implemented: false
description: "A **feature** review of `2026-10-28-schema-enhancements/spec.md`"
feature: 2026-10-28-schema-enhancements/review-2.md
previous: 2026-10-28-schema-enhancements/review-1.md
---

# Review 2

The feature is production ready under this review's technical readiness rules.
Both previous findings are verified fixed, including every child instance in
the repair's Finding Ledger. No new findings or observations remain. The
specification's human inspection of the behavior-change ledger remains a
before-merge requirement; it does not block technical readiness.

## Previous Finding Verification

The verification below checks the implementation and reruns its tests; the
completion flags and repair summaries were not treated as proof. Review 1 is
the only earlier review in this directory.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Commit results do not use the promised shared named type | R1-F1.1–R1-F1.6: variable declaration, function return, common definition, descriptions, embedded resolution and guards, rejection of imported function types | **verified fixed** | Shipped declarations, three passing ownership guards, embedded resolver inspection, and two independent CLI import probes; instance results below | Nothing |
| R1-F2: Call classification loses preserved text representations | R1-F2.1–R1-F2.15: scalar matrix, content formats, exclusions, runtime binding, recovery, composition, editor diagnostics and controls, literals, refined text, optional/union/list sources, downstream decisions, agreement | **verified fixed** | Public classifier matrix, runtime handler counters, composition tests and CLI probe, editor sessions, and consumer inspection; instance results below | Nothing; the YAML classification corrects an expectation in Review 1 |

### Shared commit-result type: instance evidence

The Darkmatter declarations now reference one named definition. Tests in
[recent_commits_shared_type.rs](../../../darkmatter/lib/tests/l1/recent_commits_shared_type.rs)
check ownership before resolution as well as the resolved public catalog.

| Child ID | Checked site and input | Verification result |
|---|---|---|
| R1-F1.1 | Shipped `ctx.recent_commits` declaration | References `recent-commits@darkmatter:functions`, retaining `generated` and `required`; authored-reference guard passes |
| R1-F1.2 | Shipped `recent_commits(count)` descriptor | Return references local `recent-commits`; descriptor guard passes |
| R1-F1.3 | Catalog helper and resolved context member | One `recent-commits: string[]` definition supplies both consumers; resolved member is a string list with the presence markers on the list |
| R1-F1.4 | Both descriptions | Each still names the other consumer and explains capture versus call timing; assertions pass |
| R1-F1.5 | Embedded schema table, import resolution, and test registration | `functions.yaml` is embedded; reserved imports resolve from embedded documents without disk fallback. All three guards are registered in the declared L1 target. Shipped-schema corpus tests pass |
| R1-F1.6 | CLI document declaring `commits: recent-commits@darkmatter:functions`, then `value: min@darkmatter:functions` in a separate document | Helper import composes `[one, two]` and prints its length `2`; function import exits 1 with the document path, property `value`, and the description-only function-type error |

The common definition is in
[functions.yaml](../../../darkmatter/schemas/partials/functions.yaml), its
context consumer is in
[context-variables.yaml](../../../darkmatter/schemas/partials/context-variables.yaml),
and embedded resolution is in
[embedded.rs](../../../darkmatter/lib/src/markdown/schemas/embedded.rs).
No independent element-type declaration remains at either consumer.

### Preserved text: instance evidence

Darkmatter's [call classifier](../../../darkmatter/lib/src/markdown/compose/expression/call_check.rs)
now considers the native scalar and retained text separately. Its source-kind
check also retains both possibilities before applying registered exclusions.
The public tests are in
[design_time_call_check.rs](../../../darkmatter/lib/tests/l1/design_time_call_check.rs).

| Child ID | Checked site and input | Verification result |
|---|---|---|
| R1-F2.1 | Four cross-family numberlike/boolish pairs | All are category 3 rather than category 5; per-cell tests pass |
| R1-F2.2 | Both preservation types passed to `json` | Category 3; runtime binding confirms retained `+4` and `yes` are not JSON |
| R1-F2.3 | `length(src)` for boolish and Boolean declarations | Boolish is category 3; native-only Boolean remains category 5 |
| R1-F2.4 | Runtime `min('1', 5)` and `length('yes')`, both literal and lookup forms | Results are `1` and `3`; each call enters its handler exactly once |
| R1-F2.5 | `number(src, 7)` with boolish declaration | Category 1; the valid fallback still recovers binding failure |
| R1-F2.6 | Composition of required boolish holding `"1"`, with numeric and literal controls | Library composition succeeds; independent `md compose` probe exits 0 and prints `1`. Native Boolean held by boolish still fails at runtime, while a Boolean declaration is rejected by the passive check |
| R1-F2.7 | Published DMLS diagnostics for boolish passed to `min`, `max`, `abs`, `length` | Four correctly ranged category 3 warnings; no incompatible-call error |
| R1-F2.8 | DMLS numberlike-to-`min`, recovered-call, and literal-text controls | No call diagnostics |
| R1-F2.9 | Every cell of the original five-source, seven-target matrix | All 35 cells asserted and passing; YAML expectations corrected as described below |
| R1-F2.10 | Retained text passed to `yaml` | Category 3 is justified: admitted tab-prefixed numeric/Boolean text fails YAML binding; ordinary `+4` remains valid YAML |
| R1-F2.11 | numberlike-to-`literal(true)` and boolish-to-`literal(1)` | Category 3; native number-to-Boolean-literal control remains category 5 |
| R1-F2.12 | Both preservation types passed to `alphanumeric` | Category 3; native number control remains category 5 |
| R1-F2.13 | Optional boolish and boolish/object source union passed to number | Category 3; Boolean/object union control remains category 5 |
| R1-F2.14 | List recursion, overload/recovery folds, guard narrowing, editor severity, runtime exclusions | List-to-number and list-to-Boolean controls pass. Overload and recovery consumers use the corrected per-argument verdicts; guard family filtering conservatively retains preservation types; editor maps the shared category; runtime exclusion examines the actual value kind |
| R1-F2.15 | Composition/editor agreement corpus for retained text | All four cases agree, including the native Boolean rejection control |

The downstream checks were reviewed in
[composition precheck](../../../darkmatter/lib/src/markdown/compose/call_precheck.rs),
[editor diagnostics](../../../darkmatter/dmls/src/diagnostics/call_check.rs),
[scope narrowing](../../../darkmatter/lib/src/markdown/compose/expression/scope/shape.rs),
and [runtime binding](../../../darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs).
Composition/editor agreement was also exercised through LSP sessions, rather
than inferred solely from their shared implementation.

Review 1 incorrectly expected guaranteed YAML compatibility for both preserved
text types. The repair's tab-prefixed examples demonstrate that some admitted
text cannot parse as YAML. Category 3 follows the spec's existing distinction
between native serialization and text-content validation. This is a corrected
review expectation, not a new language policy or an unresolved design choice;
it needs no additional author approval.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None. Both earlier defect classes are closed. The repair's additional literal,
refined-text, optional, union, and YAML instances were checked as part of the
existing findings; this review found no remaining sibling or repair regression.

## Input Robustness Matrix

The repairs introduce no new configuration fields or document loaders.
The bounded reader review covered the existing catalog/schema loaders,
frontmatter admission, raw JSON/YAML schema handling, and DMLS expression
settings. Default behavior for invalid shapes remains a schema/configuration
error; configuration recovery uses the established fallback rather than
running with a partially applied invalid layer.

The deliberate exceptions remain unchanged:

- General standalone schemas can decline an unrecognized envelope; the
  expression catalog requires its tagged envelope and rejects null entries.
- Missing expression settings default to `false`; an empty section sets
  nothing. Invalid editor settings fall back to the file layer; whole-settings
  null clears the editor overlay. Unknown keys retain the existing ignore rule.
- The settled positional section form, such as `expressions: [true]`, retains
  the shared configuration deserializer's behavior.
- Frontmatter presence/default rules surround conversion; preserved text types
  retain their values, and raw schemas still validate unmodeled assertions.

Passing tests cover the public outcomes for field absence, null, wrong whole
types, malformed collection members, empty values, duplicate keys, extra YAML
documents, invalid trailing content, numeric exactness, configuration reload,
and overlay recovery. The schema matrices use frozen producer fixtures with
positive controls. The parser-smell scan found the same guarded cases as
Review 1: defaults implement documented absence behavior; dropped raw-schema
shape information clears exactness so full validation remains authoritative;
catalog text extraction only controls display spelling after validated parsing.
No new silent default or lost validation was found.

## Observations

None.

## Verification and Limits

This review independently ran **590 tests: 590 passed, 0 failed** through the
package-area `just test` recipes and nextest:

| Requirement group | Verification run and inspected | Appropriate level |
|---|---|---|
| Shared named type; five categories; numeric admission; grammar; frontmatter conversion; composition ordering; editor ranges, settings, and agreement | First Darkmatter selection: 168 tests | Level 1 |
| Reader robustness; conversion core and translator agreement; catalog identity and executable examples; configuration recovery; embedded resolution; CLI composition | Second Darkmatter selection: 222 tests | Level 1 |
| Every registered function's binding fixtures; rejected-call handler counters; container/union/overload rules; return-contract failure; passive shipped schemas | Third Darkmatter selection: 192 tests | Level 1 |
| Claudine expression catalog output, completeness, grouping, and snapshot | Claudine selection: 8 tests | Level 1 for the catalog-output contract |
| Behavior-change ledger inspection | Earlier inspection recorded; closing additions still marked pending | Human inspection explicitly required before merge |

Four additional isolated CLI probes checked the helper import, forbidden
function import, repaired boolish call, and numeric control. No probe files or
test changes remain. The selected tests' longest reported duration was 2.675
seconds. The first two attempted compound-filter commands failed in recipe
argument interpolation before tests started; positional filters then ran all
three successful Darkmatter selections.

The new test modules are declared by the consolidated L1 targets, their names
are selected by L1, and the required instrumentation features are enabled by
both local recipes and package CI metadata. No new keyboard, terminal encoding,
visual styling, scrolling, or browser-input requirement needs Level 2 or 3.
Claudine's catalog-output change is verified as text and structure; the spec
does not add a terminal-rendering requirement.

This review did not repeat the complete area suites, lint, or cross-OS gates.
Their passing repair-log results remain implementation claims, distinct from
the independent selected runs above. Cross-OS proof is delegated to CI as the
review instructions require. No source code was changed by this review.
