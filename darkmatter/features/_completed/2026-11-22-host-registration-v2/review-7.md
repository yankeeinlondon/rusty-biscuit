---
$schema: feature-review.yaml
ready: false
findings:
    - "R7-F1 (high): Valid list narrowing fails when items are a union"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: "codex/gpt-6.2-sol"
recurrence: true
created: "2026-11-23T19:41:27+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: true
implemented-by: "claude/opus"
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/log.md
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: 2026-11-22-host-registration-v2/review-7.md
previous: 2026-11-22-host-registration-v2/review-6.md
next: 2026-11-22-host-registration-v2/review-8.md
---

# Host registration v2 review, iteration 7

**Production readiness is false.** The two findings from iteration 6 and the remaining part of the author's source-location ruling are verified fixed. One new finding blocks readiness. A list with a narrower choice of item types can be rejected as a trigger conflict, preventing Darkmatter from processing a valid host document.

## Previous Finding Verification

The required instances come from the [iteration 6 Finding Ledger](log.md#implementation-of-review-findings-6) and the [iteration 5 Finding Ledger](log.md#implementation-of-review-findings-5). The earlier review tables account for the findings already closed.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R6-F1 — Trigger narrowing moves rules between union arms | .1 object payloads, .2 root-union payload and moved element rule, .3 validate, compose, trigger inspection and DMLS, .4 deferred assembly and passive inspection, .5 rule and type controls | **verified fixed** | The 34 checked-merge tests passed, including swapped descriptions in both payload forms, moved element and list rules, element versus property defaults, named alternatives, nested rules and the arm-preserving control. The command tests assert the coded conflict on all three routes. DMLS asserts it is the only diagnostic. The public library test checks that resolving the template reports the conflict and that passive inspection leaves the template pending. Equivalent declarations, scalar narrowing, competing root alternatives and unmatched triggers retain their expected results. [Merge tests](../../lib/tests/l1/host_trigger_merge.rs), [CLI tests](../../cli/tests/l1/host_registration.rs), [DMLS tests](../../dmls/tests/l1/registered_trigger_conflict.rs). | All ledger instances pass. R7-F1 is a newly found sibling involving alternatives inside a list. |
| R6-F2 — Refused schema targets lose the declaration that names them | .1 self and cross-trigger scalar or arm, plus extension operand, .2 library, YAML kind and namespace targets, .3 named imports in kind, namespace and shared library, .4 Markdown and YAML document guards, .5 installed, embedded, formatted, JSON and DMLS, .6 boundary, envelope, parse and admission controls | **verified fixed** | The loader tests assert coordinates for both trigger targets and forms, each of the other member roles, and the imports in a kind, namespace and shared library, including both upstream triggers. Both document guards return a typed error with the writing site and the trigger-specific suggestion. The embedded loader, CLI output families and DMLS assertions passed. The resolution tests also cover delegating files, cycles, unsupported JSON dialect and both object-composition operand forms. The neighboring controls keep parse locations at the broken target, reject boundary escapes and keep refused hosts out of selection. [Installed tests](../../lib/tests/l1/host_registration_locations.rs), [embedded tests](../../lib/tests/l1/embedded_registration.rs), [DMLS tests](../../dmls/tests/l1/registered_host_editor_agreement.rs), [resolution tests](../../../simplified-schema/lib/tests/l1/reference_error_provenance.rs). | None. |
| RULING-1 — Carry source and declaration through reference resolution | .a the seven error variants and first-reader provenance, .b structural registration location, .c earlier regression cases, .d guide and departure record | **verified fixed** | Review-5.md, item 1 requires the resolver to carry the writer and its declaration. The original selection and example cases pass in the current tree. The remaining `TriggerSchemaReferenced` cases now retain the site at both document guards and through trigger-refusal conversion. The registration locator follows the path and keeps a genuinely unknown declaration file-level. The host guide and departure record describe the structural approach. [Author's ruling](review-5.md#item-1), [error provenance](../../../simplified-schema/lib/src/errors.rs#L1300), [registration locator](../../lib/src/config/registration/locate.rs#L109), [host guide](../../docs/topics/host-registration.md). | None. |

## Unblocked Findings

### R7-F1 (high) — Valid list narrowing fails when items are a union

**Authority — [spec §6](spec.md#6-when-two-of-a-hosts-triggers-declare-the-same-property) and the [acceptance criteria](spec.md#acceptance-criteria).** “Use its full declaration when all other rules and metadata agree.” The spec identifies SimplifiedSchema's property subtype check as the type comparison and explicitly requires strict narrowing to be tested. The defect class is comparing union alternatives at inconsistent levels. The property subtype check accounts for the item alternatives of a list of a union. Darkmatter's rule comparison keeps that list whole and then asks the atom subtype check for a matching alternative. That check only compares array atoms by identity, so the valid narrowing fails before Darkmatter compares the rules. [SimplifiedSchema's property subtype check](../../../simplified-schema/lib/src/simplified/subtype.rs#L304), [Darkmatter's checked trigger merge](../../lib/src/config/registration/merge.rs#L315).

Publish Claudine into a temporary home and register its manifest there. Give the installed `composition.yaml` a match of `prompt: string(required)` and a payload of `review-model: "(string | number)[]"`. Add `review-model: "string[]"` to both arms of the installed `inline-composition.yaml` payload. Refresh both manifest digests with biscuit-hash and use this document in a temporary repository.

```markdown
---
prompt: Say hello
review-model: [claude]
---

# Review probe
```

Both triggers match. `md schema validate`, `md compose` and `md schema triggers` exit 1 with `dm.host.trigger_conflict`. DMLS reports the same conflict over stdio. The reason says “alternative 1 narrows no single alternative of the other declaration.” A list of strings fits a list whose items may be strings or numbers. The merge should keep the complete string-list declaration and let it judge the document.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R7-F1.1 | Two object payloads, either declaration order | `(string \| number)[]` and `string[]`, then exchange the declarations between the matched triggers | Both orders fail preparation with the same reason | Choose `string[]` in both orders |
| R7-F1.2 | A key shared by every arm of a root-union payload | Same pairs, retaining the shipped inline-composition union with the same declaration in both arms | Both orders fail on the shared key | Use `string[]` in each arm and preserve the union's other fields |
| R7-F1.3 | Grouped and resolved named item choices | `(string \| number \| boolean)[]` narrowed to `(string \| number)[]`, and `ModelIds[]` with private `ModelIds: [string, number]` narrowed to `string[]` | Both variants fail with the same reason in object and root-union payloads | Compare the resolved item alternatives and admit the narrower declaration |
| R7-F1.4 | List constraints and item constraints | Keep `min(1)` on both lists. Separately keep `no-coerce` on the string item in both lists | Both compatible pairs fail with the missing-alternative reason | Admit both pairs and retain the settings at their respective levels |
| R7-F1.5 | List versus item rule disagreement | Move `min(1)` from the list to the string items, or remove `no-coerce` from the retained string item | Both pairs conflict. The current check fails before comparing the differing rules | Keep refusing both because a retained item rule or list rule changed |
| R7-F1.6 | Static cache and public consumers | Reproduce .2 through validation, composition, trigger inspection and DMLS | All three CLI commands exit 1 and DMLS publishes the conflict. The unchanged-list editor controls publish an empty diagnostic list | All routes use the successful layer and retain schema features |
| R7-F1.7 | Deferred trigger assembly | Supply either side through a constant template returning the same type text, with object and root-union payloads | Validate and compose still conflict after resolution. Passive inspection leaves the template pending and reports no conflict | The same narrowing succeeds after resolution. Preserve passive inspection |
| R7-F1.8 | Comparison controls | Plain identical lists, identical grouped lists, a property union of separate `string[]` and `number[]` alternatives narrowed to `string[]`, and a scalar union narrowed to an enum | Each prepares successfully. With the scalar enum, validation rejects the probe's array as a value error after preparation. Raw payloads require whole-schema and requiredness equality by construction | Preserve these results |
| R7-F1.9 | Boundary between a list of a union and a union of whole lists | Narrow a declaration admitting separate string lists, number lists and a boolean to `(string \| number)[]` | Preparation refuses it in both payload forms | Keep the refusal. The proposed declaration admits a mixed list that neither separate list accepts |

Extend the checked comparison where two list declarations are paired. Compare their container rules once, then pair compatible item alternatives and compare the surviving item rules there. Keep the completeness check for each matched payload and the winner's original declaration. The .9 control prevents a repair from treating a mixed list as if it were one of several homogeneous lists. A focused library regression can also validate a mixed list against the wider trigger alone, then check that the narrowed layer rejects the mixed value after preparing successfully. [Rule comparison](../../lib/src/config/registration/merge.rs#L345), [SimplifiedSchema list syntax and behavior](../../../simplified-schema/docs/topics/language/unions.md).

## Blocked Findings

None.

## Recurrence

R7-F1 is a newly found sibling of R6-F1, “Trigger narrowing moves rules between union arms.” The iteration 6 repair correctly paired scalar and property-union alternatives and separated element rules from list rules. It stopped the pairing at a grouped list even when the underlying subtype check had compared that list's item alternatives. In the previous source, the rule check reached whole-property equality for the unconstrained example. The added covering-alternative check now rejects it. The iteration 6 sweep should have included authored and resolved named item unions, both sides of the pairing, and the static and deferred paths that consume it. The instance table covers those cases. Review 1's root-union findings concerned higher-layer precedence and editor schema projection after preparation. R7-F1 affects whether the checked layer is prepared at all.

## Input Robustness Matrix

Invalid registration input produces a located refusal where the source is known. A refused bundle contributes no host profile and runs no host code. Independent validation can continue using Darkmatter's rules. The grouped-list finding uses valid schema text and concerns merging after admission.

| Loader or validator | Level 1 evidence and defined exceptions |
| --- | --- |
| Manifest fields, path lists and digest map | The snapshot module checks required-field absence, null and wrong shapes, duplicate paths, missing and stray digests, invalid versions, path escapes and bytes that disagree with the manifest. The explicit empty list is accepted where the manifest allows no files or functions. |
| Member YAML loader and closed envelopes | The loader tests reject duplicate keys, malformed trailing content and second documents and accept comments and a leading BOM. Envelope tests distinguish invalid field shapes from the nullable YAML-kind `types` convention and the omitted payload of a matcher-only trigger. The role and parse controls check that a valid file used in the wrong role and a file with invalid content receive distinct diagnostic locations. |
| Lifecycle and function artifacts | Prior full suites and reader tests cover strict JSON loading and the required effects fields. Empty effects are explicit. The catalog/facts checks refuse a stale artifact and undeclared bindings. |
| User config and setup | The config and setup tests use the established absent, empty and nullable registration conventions, check malformed input and duplicate entries, and preserve other settings when a permissions migration conflicts. |
| Schema references | The focused runs cover local and delegated references, unlisted, escaping and remote references, named operands, file examples, wrong-role targets, and targets refused for their own content. A reference with an unknown writer is reported at file level, as required by the ruling. |

I checked required-field defaults, nullable reads, collection filtering and dropped parse errors on the registration input paths. The focused diagnostic cases exercise the public loaders and output, with the earlier full-suite evidence covering unchanged readers.

## Verification

| Spec behavior | Verification level and evidence |
| --- | --- |
| Bundle isolation, revision, source locations and catalog admission | Level 1 snapshot and registration tests check manifest and member admission, identical identity across file order and install locations, and no partial host. Claudine's inventory test and the direct publication ran outside the checkout. |
| Markdown selection, registered YAML precedence and trigger layers | Level 1 library, CLI and DMLS cases cover shared events, literal mismatches, malformed values, ambiguity, YAML kind collisions, root-union precedence and exact-one inline composition. The grouped-list probes identify the remaining strict-narrowing defect. |
| Publication interruption, overlap, retry and unchanged timestamps | Level 1 publisher and reader tests use controlled publication points for interruption and overlap. The Claudine command checks published bytes against the inventory and confirms that an identical publish leaves timestamps alone. |
| Editor updates and repair | Level 1 controlled-clock and load-order tests exercise cross-batch settling, superseding an in-flight load and clearing repaired warnings. Protocol tests and the direct DMLS run check conflict diagnostics. |
| Inventory, setup and ordinary Claudine commands | Level 1 command tests cover listing and detail bytes, failure reporting, registration migration and preservation of permissions. The focused Claudine run includes read guards for ordinary startup. |

On Linux, focused Darkmatter L1 tests passed 276 cases. SimplifiedSchema passed 13 focused provenance cases and 2,162 minimal-feature cases. The focused Claudine run passed 90 cases. The longest reported test took 2.546 seconds. The previous full-suite results remain recorded in the [iteration 6 log](log.md#implementation-of-review-findings-6). These contracts are visible through library calls, command results and the language-server protocol. They require no terminal rendering or keyboard input.

The [repair log](log.md#review-7-record-correction--2026-11-23) and [implementation record](implementation-log.md#review-7-record-correction--2026-11-23) now reflect the grouped-list limit and the current trigger-reference constructor.
