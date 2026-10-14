---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - "Existing cleanup safety gate limits the combined sibling-repair example"
    - "Array defaults remain unavailable through the authored grammar"
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-12T09:44:09-07:00
spec: 2026-10-14-union-type-constraints/spec.md
implemented: false
description: "A **feature** review of `2026-10-14-union-type-constraints/spec.md`"
feature: 2026-10-14-union-type-constraints/review-2.md
previous: 2026-10-14-union-type-constraints/review-1.md
---

# Review of Union Type Constraints

The feature is **production ready** under this review's readiness rules. All
four high findings from review #1 are implemented and verified. No new high,
medium, or low findings were identified. The two existing limitations below
remain observations; neither requires another repair cycle.

## Previous Review and Repair Log

Review #1 had four unblocked high findings and no blocked findings. The repair
log records all four as fixed, with none deferred or disputed. There was no
human decision that subsequently unblocked additional work.

| Prior finding | Repair checked | Verification |
|---|---|---|
| Postfixed aliases bypass union operation checks | Darkmatter's resolver recognizes both a direct union definition and a non-array union atom produced by an earlier postfix. Both pass through the same operation checks, retaining inherited markers and descriptions. | L1 public resolution tests reject forbidden value/container constraints, array arms, and scalar defaults through aliases; allowed bounds and markers survive. Hermetic CLI tests verify rejection through `md schema validate`. An independent public-API probe also passes through cross-file and multi-hop aliases. |
| Nested postfixed unions lose coercion and tagged error reporting | Resolution flattens postfixed inner unions into the enclosing alternatives. Reference markers stay on the containing property; descriptions stay per arm without overwriting an arm's own description. | L1 tests compare direct and postfixed nested forms for property/item lowering, coercion, tagged reporting, CLI composition, and DMLS diagnostics. Lazy file arms retain their lazy behavior. |
| Eager before the array suffix remains unsupported | The passive parser retains the lists before and after `[]` separately; the resolver accepts union markers before `[]` and rejects value constraints there by name. Serialization preserves the authored ordering. | L1 tests compare markers on both sides of `[]` for bare, same-file, and cross-file references; round trips, CLI validation, DMLS diagnostics/navigation, and the non-union rejection boundary pass. Independent multi-hop cases pass too. |
| Inherited eager timing does not hoist to union arrays | Phase projection now reads union-arm timing for both property unions and union arrays, consistent with inherited presence queries and Claudine's classifier. | L1 phase tests compare launch/completion verdicts for properties and arrays, inherited marker combinations, all three reference forms, and lazy/eager file arms. Claudine's resolved-union classifier test includes the array case. |

The principal repair sites are Darkmatter's
[resolver](../../../darkmatter/lib/src/markdown/schemas/resolve.rs),
[named-reference parser](../../../darkmatter/lib/src/markdown/schemas/simplified/grammar.rs),
[serializer](../../../darkmatter/lib/src/markdown/schemas/simplified/serialize.rs),
and [phase projection](../../../darkmatter/lib/src/markdown/schemas/phase.rs).
These respectively enforce reference operations, retain authored constraint
placement, preserve round trips, and decide when a required property must be
present.

## Findings

None.

## Verification

All requirements here concern schema processing, CLI data, or editor protocol
results. Level-1 public-API tests, hermetic binary invocations, and DMLS JSON-RPC
sessions are appropriate. The spec explicitly requires no terminal or browser
windows; terminal-emulator rendering and OS keyboard injection are unnecessary.

| Requirement | Verification level and evidence | Result |
|---|---|---|
| Content Policy acceptance cases and flattened array lowering | L1 resolver/validator tests and hermetic `md schema validate` acceptance tests | Passed, including optional missing/null properties, invalid null items, minimum length, required nested rule, and invalid action |
| Reference operations, origins, imports, cycles, markers, descriptions, and boundaries | L1 public resolution tests, authored round trips, CLI schema-error tests, and independent multi-hop probe | Passed; inherited array defaults have the existing grammar limitation below |
| Validation reporting and node lookup through item alternatives | L1 report tests, including `/content_policy/1/rule`, tagged/unknown/conflicting tags, overlapping arms, and paired property/item cases | Passed |
| Cleanup safety | L1 cleanup tests for tagged and untagged invalid union items, plus non-union sibling controls | Union items remain unchanged; combined sibling repair has the existing limitation below |
| Coercion and eager-file normalization | L1 public consumer and normal CLI composition tests; unique versus ambiguous candidates, nested object fields, independent items, and repeated normalization | Passed |
| Completion, hover, descriptor catalog, and editor schema errors | L1 completion tests, `md schema about` tests, and DMLS JSON-RPC parser/completion/hover/diagnostic/navigation tests | Passed |
| Existing-schema byte identity | Independently reran the saved lowering capture and compared its biscuit-hash BLAKE3 manifest with the prerequisite baseline | **347/347 entries identical** |
| Passive validation and normal composition | L1 public APIs and hermetic CLI pipeline tests assert unchanged validation input, composition coercion, and idempotence | Passed |
| Required local gates | `KACHE_DISABLED=1 just test` and `KACHE_DISABLED=1 just lint` in `darkmatter/` | **9,461 passed, 11 skipped; lint passed**, including the Zed WASM compile check |
| Claudine downstream schema readers | Focused nextest run for `claudine-gen` and `claudine`, selecting schema/composition/inline-prompt/sidecar tests | **1,607 passed**; other tests excluded by the review filter |
| Test compilation and tier selection | Consolidated L1 module declarations inspected; root `just check-tier-coverage darkmatter` | No stranded tests |

The sibling review covered direct union definitions, marker/description aliases,
nested postfixed arms, array-union aliases, and bare/same-file/cross-file/multi-hop
references. Consumer checks covered lowering, validation and discriminant
reporting, coercion, normalization, cleanup, completion, phase projection,
CLI assignment, and DMLS projections. The parser's new constraint placement was
also checked against unresolved-reference readers in Darkmatter, DMLS, and
Claudine. No new defect class was found, so no instance table or recurrence
finding is needed.

The existing input robustness matrix tests exercise the imported YAML `kind`,
`types`, and union definition, and the document `$schema` and property
declaration. Each starts from the positive Content Policy fixture and changes
one field, checking the public load/preparation/validation outcome and unchanged
files. The repair changes a text grammar and resolution rules, not a file-format
deserializer; it adds no new load-bearing configuration field requiring a new
matrix.

Temporary review probes used the existing temp-directory fixture and public
resolver. They checked multi-hop nested aliases, both marker orders, container
bounds/uniqueness, forbidden constraints, array-arm rejection, and inherited
scalar-default rejection. The temporary probe and baseline-capture modules were
removed afterward. This review changes only review/spec metadata and this report;
it introduces no production code, permanent tests, dependencies, or CI cells.

## Observations

### Existing cleanup safety gate limits the combined sibling-repair example

As in review #1, the pre-existing cleanup gate requires a quoting repair to
leave the complete document valid. It repairs a non-union sibling beside valid
union items, but does not apply that repair while an invalid union item remains.
Acceptance criterion 4's combined example is therefore not verified literally.
The feature preserves the existing conservative behavior, and the documentation
records it. A broader repair-proof mechanism would be large relative to its
benefit; retaining the documented limitation is preferable in this cycle.

### Array defaults remain unavailable through the authored grammar

As in review #1, the existing grammar reads `default([a])` as a string, so an
authored schema cannot reach compatible inherited array-default lifting. Tests
verify ordinary-array parity and structured rejection of inherited scalar
defaults. The documentation accurately states this limitation. Expanding the
default-value grammar would exceed this feature's union-reference work; keep the
limitation documented rather than treating the lifting branch as authored
end-to-end coverage.
