---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Caller-owned validation documentation overstates trigger coverage
    - Alphanumeric suggestion loading retains its separate rule
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-12T20:24:20-07:00"
spec: 2026-10-17-content-policy-base-schema/spec.md
implemented: false
description: "A **feature** review of `2026-10-17-content-policy-base-schema/spec.md`"
feature: 2026-10-17-content-policy-base-schema/review-2.md
previous: 2026-10-17-content-policy-base-schema/review-1.md
---

# Review of Content Policy Base Schema

The feature is **production ready** under the specification's acceptance
criteria. All three blocking findings from review 1 are repaired. No new
high, medium, or low finding remains, and no human decision is needed to
complete this review cycle.

## Prior findings and repair log

Review 1 has a single Findings section containing three unblocked high
findings. Its frontmatter and body contain no blocked findings to reconsider.
The repair log reports three fixes, no disputed findings, and no deferred
review findings. It separately records two limitations found during the
repairs; these are assessed under Observations below.

| Prior finding | Repair verified | Result |
| --- | --- | --- |
| Disabling triggers skips validation of shell-resolved policy values | Default compose validates after shell expansion in both trigger modes; pending values are deferred according to whether a later stage resolves them | Addressed |
| Rule completion drops six shipped suggestions | Suggestion queries and completion judge the shared list against the rule union; embedded and disk-imported schemas offer the exact seven shipped candidates | Addressed |
| Diagnostic projections discard policy instance paths | Compose names nested paths; DMLS diagnostic data retains `instance_path` beside the source range | Addressed |

The first review's documentation observation is addressed for ordinary
`md compose`: the topic pages now describe post-shell checks with and without
triggers. Its unchanged slow reference-parity test did not exceed the
five-second budget in this run.

## Findings

None. The following tables record the repaired classes and their sibling
checks, rather than raising those classes again.

### Pending-value validation

The reviewed class is a final validation decision that depends on an
unrelated schema source, or defers text that no later stage resolves.

| Site | Shape checked | Observed and expected result |
| --- | --- | --- |
| `md compose`, triggers enabled and disabled | Direct `$(echo words)`, JSON text printed by `cat`, typed boolean shell result | Reject invalid policy values in both modes |
| `md compose`, both trigger modes | Template reading shell output, as a list item and as a whole-list expression | Accept `TimeSensitive`; reject an unknown resolved rule |
| Discovery and first interpolation/validation passes | Template over a pending command; command containing an interpolated argument | Defer provisional values and successfully judge their resolved results |
| First validation pass | Shell text nested in a list, or following ordinary text | Reject incompatible text as written; it is never executed |
| Final compose validation | Shell output and deferred templates | Judge concrete output; invalid output cannot escape through textual pending detection |
| Caller-owned validation and typed-result backstop | Unsuffixed stdout and incompatible typed boolean result | Leave unsuffixed output to the caller; reject the incompatible typed result |
| Public `EffectiveSchema::validate_with_options` in Darkmatter | Nested and trailing shell text | Report invalid values with no shell-pending entry |
| Baseline disabled, no other schema | Invalid-looking shell-produced policy | Accept because no effective schema remains |

These results are asserted through the built `md` in
[standalone_baseline.rs](../../../darkmatter/cli/tests/l1/standalone_baseline.rs)
and through composition in
[shell_result_values.rs](../../../darkmatter/lib/tests/l1/shell_result_values.rs).
Claudine's distinct caller-owned trigger limitation is not presented as a
passing route; see Observations.

### Shipped suggestions

The reviewed class is filtering a union's shared suggestions against only
the alternative storing the list.

| Site | Shape checked | Observed and expected result |
| --- | --- | --- |
| Darkmatter suggestion lint and path query | Embedded `content_policy` definition | No false suggestion lints; all seven rule candidates retained |
| Darkmatter definition queries and alternative completion | List-item definition and required long-form `rule` definition | All seven rule candidates retained |
| Disk schema loading and completion | Real Content Policy editor schema | No false load-time suggestion warnings; same candidates as embedded source |
| DMLS in-memory protocol and built stdio server | Blank compact item and blank `rule:` | Seven rule candidates; compact item also offers long-form keys |
| DMLS action and key completion | Blank `action:` and incomplete long-form mapping | Three legal actions and the appropriate remaining keys |
| DMLS hover | Compact and long-form rules | Shipped branch descriptions retained |

The completion tests compare the candidate sets, not merely the presence of
`Evergreen`. The generic union test also verifies that an array-element
alternative cannot validate a suggestion intended for a scalar insertion.

### Diagnostic paths

The reviewed class is losing a validation problem's instance path while
projecting it to a user-facing result.

| Site | Shape checked | Observed and expected result |
| --- | --- | --- |
| Library baseline validation | Shared block/flow declaration corpus | Exact paths, problem kinds, baseline origin, and offending node positions retained |
| `md schema validate`, pretty and JSON | Invalid rule/action, missing/null rule, empty/wrong-shaped list | Item or field paths and positions retained |
| Built `md compose` | All six acceptance rejection shapes | Full nested paths displayed; missing fields append their name to the containing mapping's path |
| DMLS protocol and built stdio server | Declaration corpus and six acceptance rejection shapes | `data.instance_path` retained alongside the expected range |
| `md clean` JSON findings | Block and flow collections | Report-only message names the instance path |
| Claudine composition reports, optional drops, and sequence validation | Nested missing field and nested invalid value | Nested problems remain problems of their containing value; reports and drop reasons retain the path |
| Claudine signals and generator sidecar validation | Invalid sidecar member | Validation messages retain the instance path |

The additional Claudine changes prevent a missing nested field from being
offered as an unrelated top-level property to collect. Their dedicated tests
passed along with the composition suite.

## Verification and coverage

Review scope includes the original feature implementation and the repair
changes recorded in `log.md`. Production code and tests were left unchanged
by this review. Test files are declared by their consolidated `l1/main.rs`
targets; the executed Level 1 recipes select them. The Darkmatter tier
coverage check found no stranded tests.

| Specification requirement | Verification level and evidence | Result |
| --- | --- | --- |
| Schema-free policy and policy-free documents | Level 1 public baseline validation, compose, built `md`, and DMLS protocol tests | Pass; omitted policy stays omitted |
| Rejection shapes, paths, positions, completion, and hover | Level 1 shared corpus, subprocess output, and editor protocol responses | Pass |
| Installed-binary operation and cache reuse | Level 1 isolated `md`/DMLS launches with fixture homes; same-process cached-reference identity | Pass |
| Embedded imports, errors, disk equivalence, and definition isolation | Level 1 test-owned embedded tables, shipped-artifact preparation, disk/embedded comparison, and layered schemas | Pass |
| CLI selection, conflicts, environment spellings, opt-outs, and exit/output compatibility | Level 1 hermetic subprocess tests | Pass |
| Parser agreement, documented differences, passivity, and resolved values | Level 1 declaration corpus, source-byte/frontmatter assertions, effect counters, and shell/template checks | Pass |
| Test placement and package checks | Declared Level 1 targets, tier audit, nextest recipes, and lint | Required package checks pass |

Level 1 is appropriate for every requirement here: the results are schema
data, CLI output, and language-server protocol responses. The specification
expressly requires no new terminal or browser tier. No window was activated.

The YAML robustness tables in the plan cover `content_policy`, `rule`, and
`action`; schema-envelope fields `$schema`, `kind`, and `types`; and every
policy definition/reference that changes validation. Executable mutations
exercise absent, explicit null, wrong whole-field and element types, empty,
duplicate, and invalid/trailing content against positive controls. Disk and
embedded schema sources share that table and assert public validation results.
The declaration corpus exercises block and quoted flow-list presentations.
Whole-list null, impossible dates, overflow, path limitations, and combined
`Evergreen` have explicit parser/schema expectations. No silent filtering,
empty fallback, or new parser-default defect was found.

Executed on macOS:

- Darkmatter `just test`: **9,751 passed, 11 skipped**; no test reported over
  five seconds. `just lint` passed, including the Zed extension compile check.
- Content Policy `just test`: **176 passed, 4 skipped**. `just lint` passed,
  including the dependency boundary checks with default and all features.
- `just check-tier-coverage darkmatter`: no stranded tests.
- Claudine `just lint`: passed with the existing macOS linker warning.
- Additional full Claudine `just test`: stopped after **5,659 passed and two
  messaging-drain failures**, leaving 3,038 tests unrun. One failure reported
  unavailable `osascript` instead of the test's requested stall; the other
  used the ten-second production deadline instead of the test's two-second
  deadline. Those unchanged tests are outside this feature's scope. A later
  isolated `just test-cli 'lifecycle_message_drain::'` passed all **12 tests**;
  the cause of the earlier discrepancy was not established.
- Focused Claudine `just test` runs passed **1,751 composition tests**,
  **three instance-path tests**, and **105 signal tests**, covering the repair
  changes despite the interrupted additional full run.

Cross-OS evidence remains CI's responsibility and does not change readiness.

## Recurrence

No finding recurs. All three classes from review 1 are addressed within the
specification's required surfaces; the tables above include their sibling
checks. The two separately deferred limitations below are observations,
not renewed findings or additional repair requirements.

## Observations

### Caller-owned validation documentation overstates trigger coverage

Darkmatter's [schema-validation page](../../../darkmatter/docs/inline/schema-validation.md)
and [schema definition page](../../../darkmatter/docs/topics/schemas/definition.md)
say compose still judges trigger schemas when a caller takes over post-shell
validation. The repair deliberately retains textual pending deferral in that
mode: never-expanded `$(...)` text can still escape a trigger-schema verdict,
while [Claudine's post-shell check](../../lib/src/composition/schema/mod.rs)
does not load those triggers. The log explicitly records this limitation.
Ordinary `md compose` and the shipped baseline are covered by the fixes;
the spec does not add a Claudine trigger-validation contract. Narrow the
documentation's caller-owned claim to describe this limitation rather than
expanding this cycle into passing trigger schemas between the two packages.

### Alphanumeric suggestion loading retains its separate rule

In Darkmatter, [check_alphanumeric_values](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs)
checks defaults and suggestions against their individual `alphanumeric`
alternative during schema lowering, before the revised union suggestion lint.
A suggestion accepted only by a sibling alternative can therefore still fail
schema loading. This unchanged rule is distinct from advisory suggestion
filtering and is not reached by Content Policy's shipped string-pattern
schema. Document the exception to the generic union suggestion description;
any change to this schema-load rule belongs to separately scoped work.
