---
$schema: feature-review.yaml
ready: false
findings:
    - "R10-F1 (high): Matching triggers reject a list whose item type is narrower"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: "codex/gpt-6.2-sol"
recurrence: true
created: "2026-11-23T21:25:55+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: true
implemented-by: "codex/gpt-6"
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/log.md"
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: 2026-11-22-host-registration-v2/review-10.md
previous: 2026-11-22-host-registration-v2/review-9.md
next: 2026-11-22-host-registration-v2/review-11.md
---

# Host registration v2 review, iteration 10

**Production readiness is false.** The review-9 recursive declarations now merge as intended. A new strict-narrowing case fails when one trigger restricts a list of model names to an enum. The conflict prevents schema preparation in the CLI and editor.

## Previous Finding Verification

The eight required instances come from the [iteration 9 Finding Ledger](log.md#implementation-of-review-findings-9). No earlier finding remained open. The author's ruling in review-5.md, item 1 still governs reference refusals. SimplifiedSchema carries the writing declaration and Darkmatter reports it when known. The Darkmatter Level 1 run passed the installed and embedded declaration-location tests and the CLI and editor projections. [Previous verification](review-9.md#previous-finding-verification), [author's ruling](review-5.md#item-1), [registration location tests](../../lib/tests/l1/host_registration_locations.rs).

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R9-F1 — Equivalent recursive declarations conflict when one is written inline | R9-F1.1 — Local and imported tree, both file orders, object and both root-union placements, value checking and origin | **verified fixed** | The public library test passed the six combinations for each spelling, rejected a bad descendant, and checked the first-path origin for two object payloads. For a union payload the same winner loop prunes every losing arm before origins are collected. A temporary installed Claudine bundle also validated the original local pair. [Library test](../../lib/tests/l1/host_trigger_merge.rs#L754), [winner and pruning](../../lib/src/config/registration/merge.rs#L171), [origins](../../lib/src/markdown/schemas/mod.rs#L1293). | None. |
| R9-F1 — Same finding | R9-F1.2 — Narrowing and alternative pairing | **verified fixed** | The test passed in both orders and all three payload placements. It compares the retained schema with the narrower trigger alone, rejects the dropped string and a bad tree, and coalesces the two equivalent property unions. [Narrowing test](../../lib/tests/l1/host_trigger_merge.rs#L782). | None. R10-F1 exercises a narrower list **item type**, a different case from dropping the string alternative of the whole property. |
| R9-F1 — Same finding | R9-F1.3 — Root-union ownership | **verified fixed** | Both file orders keep the recursive key shared, accept either the prompt or sections arm, and reject a bad child or a document with neither arm's required field. A direct installed probe with a mutually recursive pair also kept the mixed-spelling key shared. [Ownership test](../../lib/tests/l1/host_trigger_merge.rs#L825). | None. |
| R9-F1 — Same finding | R9-F1.4 — Object field, dictionary value, tuple element, tuple spread, and list item | **verified fixed** | The library matrix passed at all five nested positions, including the bad descendant in each one. Direct installed probes also coalesced a recursive union and a tree written out to two levels. [Nested declarations test](../../lib/tests/l1/host_trigger_merge.rs#L848). | None. |
| R9-F1 — Same finding | R9-F1.5 — Static cache, validate, compose, triggers, clean, and DMLS | **verified fixed** | The cache test confirms reuse for the matched set. The CLI regression passed all four routes and distinguishes bad child values from merge failure. The editor regression observes successful preparation and still reports bad child values. [Cache test](../../lib/tests/l1/host_trigger_merge.rs#L888), [CLI test](../../cli/tests/l1/host_registration.rs#L820), [DMLS test](../../dmls/tests/l1/registered_trigger_conflict.rs#L183). | None. |
| R9-F1 — Same finding | R9-F1.6 — Deferred template, listed import, both orders and all three payload forms | **verified fixed** | The public library test observes the pending trigger with no conflict in passive inspection. A session with the listed import granted resolves the template and accepts or rejects the tree by its child value. [Deferred test](../../lib/tests/l1/host_trigger_merge.rs#L901). | None. |
| R9-F1 — Same finding | R9-F1.7 — Same and different recursive bodies, imported aliases, mixed lists, raw declarations, and function values | **verified fixed** | The equivalent named pair and shared-library comparison pass. The changed child rule conflicts in both orders and in the two-named-type control. The mixed-list refusal passes its test. Raw payloads compare the complete property schema and requiredness, and the converter refuses function values before they reach the merge. [Equality control](../../lib/tests/l1/host_trigger_merge.rs#L186), [deep rule control](../../lib/tests/l1/host_trigger_merge.rs#L871), [mixed-list control](../../lib/tests/l1/host_trigger_merge.rs#L730), [raw comparison](../../lib/src/config/registration/merge.rs#L292), [function admission](../../../simplified-schema/lib/src/simplified/convert.rs#L1160). | None. |
| R9-F1 — Same finding | R9-F1.8 — Description inherited by a recursive list | **verified fixed** | In an installed bundle, `Node[]` with a described `Node` coalesced with the inline list carrying the same trailing description. Removing only the inline description produced the expected conflict. With both lists undescribed, both file orders coalesced. [Record correction](log.md#review-10-record-correction--2026-11-23). | None. |

## Unblocked Findings

### R10-F1 (high) — Matching triggers reject a list whose item type is narrower

**Authority.** Spec section 6 says to “Use its full declaration when all other rules and metadata agree” if exactly one type is narrower. Its acceptance criteria explicitly require strict narrowing. The defect class is deciding the direction of a list comparison at a different level from the item comparison that checks its rules. Darkmatter first asks SimplifiedSchema's property subtype comparison for a direction. SimplifiedSchema splits a list of a union into apparent whole-list alternatives and compares list atoms only by identity. For `enum(claude, codex)[]` against `(string | number)[]`, this returns false in both directions. Darkmatter exits before its own list item pairing can see that the enum is covered by the string item. [Spec section 6](spec.md#6-when-two-of-a-hosts-triggers-declare-the-same-property), [acceptance criteria](spec.md#acceptance-criteria), [Darkmatter comparison](../../lib/src/config/registration/merge.rs#L292), [SimplifiedSchema subtype](../../../simplified-schema/lib/src/simplified/subtype.rs#L142), [property alternative expansion](../../../simplified-schema/lib/src/simplified/subtype.rs#L212), [item pairing](../../lib/src/config/registration/merge.rs#L512).

Publish Claudine in a temporary home and register the resulting manifest in that home's `.darkmatter.yaml`. In the published `schemas/composition.yaml` and `schemas/inline-composition.yaml`, use `prompt: string(required)` as the match. Set the first payload's `review-model` to `"(string | number)[]"` and the second to `"enum(claude, codex)[]"`. Refresh the two manifest digests with biscuit-hash. In a temporary Git repository, validate the following document.

```markdown
---
prompt: Say hello
review-model: [claude]
---

# Review probe
```

Either trigger alone accepts the document. The wider one also accepts `[gemini]` and `[claude, 3]`. The narrower one rejects those values as item errors. With both triggers matched, the CLI and DMLS report a preparation conflict on `review-model` with the reason “their types are incomparable.” Restricting a list of model names to those supported by a particular workflow is an ordinary use of a matched trigger. It should keep the more specific declaration and judge the document against it.

The type probes used both filename orders and three payload placements, with two objects or with either declaration repeated in every arm of one root union. The union kept separate required keys for its two alternatives.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R10-F1.1 | Whole-property direction for two plain lists or list items with alternatives | `string[]` to `enum(claude, codex)[]`; `(string \| number)[]` to the same enum list; `(string \| number \| boolean)[]` to `(enum(claude, codex) \| number)[]` | All report incomparable types in both orders and all three placements | Keep the narrower list and its origin. A value outside its remaining item types fails validation |
| R10-F1.2 | Resolved item types and named list bodies | `ModelIds[]` with `ModelIds: [string, number]`, narrowed to the enum list. Separately name each entire list and reuse each type under a distinct spare key to exercise stored definitions | The inline, named-item, and stored-list paths report incomparable types | Make the same decision after resolving list and item names |
| R10-F1.3 | Property union of lists and the boundary between list and item alternatives | `string[] \| number[]` to the enum list. Also compare `(string(no-coerce) \| number)[]` with `string(no-coerce)[] \| number[]` | The first pair reports incomparable. The second treats the lists as equivalent at the direction check, then refuses to coalesce them. Its explanation changes with file order. Tested separately, only the mixed-list declaration accepts `[claude, 3]` | The enum list fits the string-list alternative. The union of homogeneous lists is strictly narrower than the mixed list. Retain the separate alternatives so that `[claude, 3]` fails |
| R10-F1.4 | Rules preserved or changed during item narrowing | `(string(min(2)) \| number)[](min(1)) -> model choices` and `alphanumeric(min(2))[](min(1)) -> model choices`. Separately change the item minimum to 3 or remove a list minimum in the enum-list probe | The agreeing and changed-rule pairs all fail earlier with incomparable types | Choose the alphanumeric list when its rules agree. Continue refusing changes to the item or list rules |
| R10-F1.5 | Shared root-union key, representative selection and static consumers | Repeat the enum list in both arms and compare it with the wider object declaration, then swap which declaration is in the union | Both placements identify the key as shared and reach the type conflict. Validation, composition, trigger inspection and cleanup exit 1. DMLS over stdio publishes `dm.host.trigger_conflict`. The static cache saves the result of this same check, and embedded profiles call it too. A document schema overriding the same key still reports the conflict | Keep the key shared and select the narrower declaration. Preserve the union's other fields and use the successful layer on all routes |
| R10-F1.6 | Deferred application and passive inspection | Return either the wider object declaration or the narrower root-union declaration from a constant template | Validation and composition fail after resolution. Trigger inspection exits 0 with the template pending and no conflict | Apply the same narrowing after resolution. Preserve passive inspection |
| R10-F1.7 | Equivalent declarations, scalar and item-removal controls, nested identity, and a genuinely incompatible mixed list | Reordered but identical grouped item alternatives, the same reordered list inside an object, scalar enum narrowing, and the earlier `(string \| number)[]` to `string[]` case. Also try a mixed list against `string[] \| number[] \| boolean` | The equal, scalar and earlier list cases all prepare. The mixed-list case still conflicts because no one wider alternative covers it. Raw schemas have no item-type comparison and qualify for coalescing only by exact equality and requiredness | Preserve all of these results |

Use the same list-aware coverage rule when choosing the type direction and when comparing the retained rules. A complete list covers a different list when its item alternatives cover each of the other list's item alternatives. At the property level, each alternative of the narrower declaration still needs one covering alternative. This retains the difference between a mixed list and several homogeneous options. The scalar matches can keep using SimplifiedSchema's subtype check. Apply the existing rule comparison once a direction is established, and keep the winner's unmodified declaration and origin. A small set of public layer tests can verify the new directional cases and their rule controls. Use one enum-list case across the existing CLI and DMLS fixtures. [Pairwise winner selection](../../lib/src/config/registration/merge.rs#L244), [type coverage and rule pairing](../../lib/src/config/registration/merge.rs#L499), [existing list regressions](../../lib/tests/l1/host_trigger_merge.rs#L362).

## Blocked Findings

None.

## Recurrence

R10-F1 is a newly found sibling of R7-F1, “Valid list narrowing fails when items are a union,” and of the alternative-level comparison in R6-F1, “Trigger narrowing moves rules between union arms.” Review 7 exercised dropping complete item alternatives with identical retained types, such as keeping `string` from `string | number`. The repair compared list items after the initial type check, leaving the initial check unable to narrow an item itself. That sweep should have covered both the decision about which type is narrower and the later rule comparison, including whole-list alternatives and resolved names. R8-F1 and R9-F1 repaired storage and recursive equality cases and their controls still pass. This recurrence illustrates how separate comparison paths produce different answers about the same declarations. It remains high because the spec expressly requires strict narrowing and the enum-list case uses ordinary schema input. [Review 6](review-6.md#unblocked-findings), [review 7](review-7.md#unblocked-findings), [review 8](review-8.md#unblocked-findings), [review 9](review-9.md#unblocked-findings).

## Input Robustness Matrix

An invalid bundle input produces a located refusal when its source is known and contributes no host profile, execution, or publication. Independent documents can still be analyzed under Darkmatter's own rules. The list comparison finding uses valid bundle inputs and occurs after these checks.

| Loader or validator | Public evidence and accepted shapes |
| --- | --- |
| Manifest YAML, member paths, function pairs, digest map | Snapshot tests cover required omissions, null or wrong field shapes, invalid list elements, duplicate keys and paths, read retries, bad version and digest forms, and escaping paths. Empty `files` and `functions` are valid. The lifecycle artifact and its digest remain required. Format 1 produces the upgrade warning. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs). |
| Member YAML and closed kind envelopes | The shared loader and public registration tests cover duplicate keys, trailing content, extra YAML documents, comments and BOM, and malformed field carriers. Missing optional fields are valid where the envelope permits them. YAML-kind `types` also admits null. A trigger with no payload is a matcher only. [Envelope tests](../../../simplified-schema/lib/tests/l1/member_envelope.rs), [location tests](../../lib/tests/l1/host_registration_locations.rs). |
| Lifecycle JSON and function facts | The strict JSON loader rejects duplicate keys and trailing content. Required field readers distinguish absent, null and wrong-shaped fields. Valid empty owned collections are accepted. Registration and lifecycle tests cover stale artifacts, undeclared bindings and located refusals. [JSON reader](../../lib/src/markdown/compose/expression/host/artifact.rs#L485), [lifecycle tests](../../lib/src/lifecycle/host_profile/tests.rs), [registration tests](../../lib/tests/l1/host_registration.rs#L410). |
| User configuration and setup | Omission, explicit null and an empty list select no registrations. A malformed whole list is reported. Malformed members are reported individually and valid siblings remain usable. Setup tests cover migration, preservation after publication failure and permissions conflicts. [Configuration tests](../../lib/tests/l1/config_fields.rs#L319), [setup tests](../../../claudine/cli/tests/l1/setup_command.rs). |
| Named references and member roles | Public tests distinguish missing, unlisted, remote or escaping targets from a listed file refused for content or role. They verify the writing declaration when the resolver supplies provenance. [Reference tests](../../../simplified-schema/lib/tests/l1/reference_error_provenance.rs), [registration location tests](../../lib/tests/l1/host_registration_locations.rs#L1179). |

The parser audit covered serde defaults, nullable fields, filtered list entries and ignored parse failures. The user config reports malformed list indices before the string projection filters them. A lenient name read provides a display label for a diagnostic and has no effect on member admission. The comparison repair introduced no loader or deserializer. [Configuration projection](../../lib/src/config/load.rs#L749), [member label reader](../../../simplified-schema/lib/src/member_envelope.rs#L224).

## Verification

| Acceptance requirement | Verification level and evidence |
| --- | --- |
| Portable bundle, stable revision, and located refusals | Level 1 snapshot, installed and embedded tests exercise digest verification, path and import boundaries, reordering and copying, catalog freshness, and positions. The direct CLI probes published Claudine outside the checkout. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs), [location tests](../../lib/tests/l1/host_registration_locations.rs). |
| Markdown and YAML host selection, precedence, and trigger merge | Level 1 library, CLI and DMLS tests exercise authored matches, shared events, literal mismatches, ambiguity, kind precedence and collisions, higher-layer overrides, and prompt-or-sections validation. Direct CLI and LSP checks expose the remaining list narrowing gap. [Markdown selection tests](../../lib/tests/l1/host_selection.rs), [YAML CLI tests](../../cli/tests/l1/host_registration.rs), [editor agreement tests](../../dmls/tests/l1/registered_host_editor_agreement.rs). |
| Interrupted publication, reader and publisher overlap, retry, and unchanged timestamps | Level 1 publisher tests exercise controlled write boundaries and compare the verified manifest and inventory. [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs#L168). |
| Editor refresh, supersession and recovery | Level 1 controlled-clock and queue tests exercise different notification batches, an in-flight load overtaken by a newer change, and repair reanalysis. The protocol tests observe the updated Markdown and YAML views. [Reload tests](../../dmls/tests/l1/registration_reload_scheduling.rs), [queue tests](../../dmls/src/overlay/hosts.rs#L1052). |
| Inventory, setup, `md init` and ordinary Claudine entry points | Level 1 tests compare inventory details with published bytes, observe installed bundle access through normal startup, preserve unrelated config and permissions, and compare installed Markdown and YAML judgment with Claudine's embedded definitions. [Inventory and startup tests](../../../claudine/cli/tests/l1/registrations_command.rs), [setup tests](../../../claudine/cli/tests/l1/setup_command.rs), [init tests](../../cli/tests/l1/init_command.rs#L85), [installed agreement tests](../../../claudine/lib/tests/l1/installed_registration_agreement.rs). |

These requirements concern schema results, file state, command exit status, and protocol messages. No requirement depends on terminal rendering or OS keyboard input. On Linux, `just test` passed 17,842 Level 1 tests in Darkmatter and 12,725 in Claudine, with 10 and 15 skipped respectively. Neither run recorded a test at or above five seconds. The direct published-bundle probes covered validation, composition, trigger inspection, cleanup, and DMLS over stdio. The [implementation record](implementation-log.md#review-10-record-correction--2026-11-23) and [repair log](log.md#review-10-record-correction--2026-11-23) include dated corrections verified against those results.
