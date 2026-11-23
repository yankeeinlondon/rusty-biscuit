---
$schema: feature-review.yaml
ready: false
findings:
    - "R8-F1 (high): Storing a named type changes whether matching triggers can agree"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: "codex/gpt-6.2-sol"
recurrence: true
created: "2026-11-23T20:19:48+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: 2026-11-22-host-registration-v2/review-8.md
previous: 2026-11-22-host-registration-v2/review-7.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/log.md
implemented-by: claude/opus
---

# Host registration v2 review, iteration 8

**Production readiness is false.** The iteration 7 repair passes every case in its ledger. A newly found sibling still blocks readiness. Reusing a named schema in a second field can change whether Darkmatter accepts the same pair of matched triggers. The second field can be optional and absent from the document.

## Previous Finding Verification

The required instances are from the [iteration 7 Finding Ledger](log.md#implementation-of-review-findings-7). The earlier reviews closed their remaining findings, including the author's ruling in review-5.md, item 1 about locating reference refusals at the declaration that wrote them. The source-location tests ran again in this iteration. [Prior verification](review-7.md#previous-finding-verification), [author's ruling](review-5.md#item-1).

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R7-F1 — Valid list narrowing fails when items are a union | .1 both object orders, .2 shared key across root arms, .3 grouped and named items, .4 retained rules, .5 moved or removed rules | **verified fixed** | The library regressions compare the kept schema to the narrower trigger alone in both orders and both root forms. They also verify the unchanged union fields and the actual value refusals from retained `min(1)` and `no-coerce`. Moving or dropping a rule fails with a rule-specific reason. [Checked-layer regressions](../../lib/tests/l1/host_trigger_merge.rs#L362). | A use-site description or rule on a stored list is a newly found sibling covered by R8-F1. |
| R7-F1 — Valid list narrowing fails when items are a union | .6 static CLI and DMLS, .7 either template resolved with passive inspection preserved, .8 equality and scalar controls, .9 mixed-list refusal | **verified fixed** | Validation, composition and inspection accept the repaired cases. The DMLS test observes no conflict and still judges a bad item. The template test resolves either declaration and checks that passive inspection never resolves it. Identical lists, ordinary property alternatives and scalar enum narrowing pass, and the mixed-list control still refuses the incompatible pairing. [Library controls](../../lib/tests/l1/host_trigger_merge.rs#L428), [CLI tests](../../cli/tests/l1/host_registration.rs), [DMLS tests](../../dmls/tests/l1/registered_trigger_conflict.rs). | None for these instances. |
| R7-F1 — Valid list narrowing fails when items are a union | .10 root-union private types, .11 matching private definitions stored under different arm keys, .12 plain stored list aliases, .13 marker placement, .14 neighboring item types, grammar and defaults | **verified fixed** | The registered payload test resolves private types in root arms. The public layer test uses two occurrences of a type to exercise a stored reference and checks object items and plain list aliases. The generated-marker and default tests retain the intended boundaries. The grammar rejects nested list syntax and the grouped-union form accepts no type constraint list at the group itself. [Private type test](../../../simplified-schema/lib/src/triggers/assemble.rs), [checked-layer tests](../../lib/tests/l1/host_trigger_merge.rs#L455). | The ledger's tested cases are fixed. R8-F1 also covers mixed inline and stored spellings, which differ from .11's two equivalent stored definitions. |

## Unblocked Findings

### R8-F1 (high) — Storing a named type changes whether matching triggers can agree

**Authority — [spec §6](spec.md#6-when-two-of-a-hosts-triggers-declare-the-same-property), the [acceptance criteria](spec.md#acceptance-criteria), and [plan Rule 8](plan.md#necessary-rules).** The spec says to resolve named types before comparison, coalesce identical resolved declarations, take the complete narrower declaration when its other rules agree, and reserve only differing or missing arm keys for the root union. The plan requires comparison independent of storage IDs. The defect class is treating a schema's internal storage representation as part of its declaration. Darkmatter's strict-narrowing check opens a stored list only when the use itself has no description or ordinary constraints. The equivalent-declaration check uses a different traversal that compares a reference to an inline type only through a plain scalar body. The root-union ownership check uses that same equality test. [Strict narrowing and equivalence in Darkmatter](../../lib/src/config/registration/merge.rs#L280), [stored list opening](../../lib/src/config/registration/merge.rs#L473), [structural equality](../../lib/src/config/registration/merge.rs#L632), [root-union ownership](../../lib/src/config/registration/merge.rs#L750).

Publish Claudine to a temporary home and add its manifest to that home's Darkmatter config. In the installed `schemas/composition.yaml`, use a match of `prompt: string(required)` and this payload. Keep the installed inline trigger's match and place `review-model: "{ id: string }[] -> Models reviewed"` in both of its payload arms. Refresh the two edited members' digests with biscuit-hash. Validate a Markdown document with `prompt: Say hello` and `review-model: [{ id: claude }]`.

```yaml
types:
    Models: "(string | { id: string })[]"
$schema:
    review-model: "Models -> Models reviewed"
    spare: Models
```

Both triggers match. Validation, composition, cleanup in JSON mode, and trigger inspection exit 1. Trigger inspection and composition show `dm.host.trigger_conflict` on `review-model` and say no single wider alternative covers it. DMLS publishes the same conflict over stdio. The bundle loads, either declaration can judge the document by itself, and replacing the wider use with its inline body and the same description makes the overlap succeed. Reusing a named field type is a routine schema edit, so this can prevent otherwise valid host documents from being processed.

All comparison variants below were checked in the published bundle in two object payloads and with either declaration in both arms of one root-union payload, except where the site requires the union itself. The public command and protocol rows list their particular checks.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R8-F1.1 | Strict narrowing between matched payloads | Reused wide alias with a description and equally described inline narrow list, then the reverse file order. Also use a described alias for the narrow list and write the wide list inline | The layer reports no covering alternative. Writing both lists inline with the same description succeeds | Open either stored list for comparison and retain the common description on the complete narrower declaration |
| R8-F1.2 | Container-rule comparison and its value-validation control | Use `Models(min(1))` where the narrower list also has `min(1)`. Move the minimum into the alias definition as a control. Remove it from the narrow declaration as a disagreement control | An equal minimum at the alias use causes a conflict. The equal minimum in the alias definition succeeds. A missing minimum on the narrow list conflicts. Each minimum by itself rejects `[]` as a list rule | The equal minimum coalesces with the narrowing regardless of where the alias obtains it. Preserve the disagreement and empty-list refusals |
| R8-F1.3 | Equivalent top-level declarations and selection of the representative | Reused `Models` against its exact inline list, including reversed file order. Separately remove the extra use or spell both declarations with stored aliases | Mixed stored and inline declarations fail as unequal nested declarations. The single-use and two-stored-alias controls succeed | Coalesce all three equal pairs and take the first bundle path as the property's origin |
| R8-F1.4 | Equivalent scalar union and list of a named union | Reuse `Models: [string, "{ id: string }"]`. Compare `Models` with the written property union, and `Models[]` with a grouped list containing the same choices | Both equal pairs fail. Narrowing each named union to its object alternative, at the matching level, succeeds | Coalesce the equal pairs and preserve both narrowing controls |
| R8-F1.5 | Nested equivalence and reference controls | Match `{ models: Models }` against the same object with the list body inline, with another use keeping `Models` stored. Also try a reference chain to the wide list and compare `Model[]` with an equivalent inline single-object item list | The nested equal list fails. The plain chained alias narrows and the equivalent single-object item list succeeds | Compare nested declarations independent of storage and preserve the successful controls |
| R8-F1.6 | Root-union arm ownership before comparing with a competing trigger | Put the exact same list inline in one root arm and through a stored alias in the other. Give the object trigger the same inline declaration. Repeat with the two trigger files exchanged | The union incorrectly owns the property as an alternative-defining key. Replacing the alias with the inline body in the second arm makes the other trigger legal | Treat the equal declaration as shared. Reserve ownership for a declaration that differs or is absent in an arm |
| R8-F1.7 | Static matched-set cache, CLI and DMLS | Run the described-alias and equivalent-list probes through validation, composition, and inspection, publish both diagnostics through DMLS, and run cleanup against the reversed cases | Validation, composition, inspection and cleanup fail. DMLS publishes the conflict on the document for both cases | Prepare one successful layer for all routes and retain value checking, source origin and editor support |
| R8-F1.8 | Deferred assembly and passive inspection | Supply the narrower described list through a constant template and keep the stored wider list. Test objects and both placements of the root union | Validation and composition fail after resolving the template. Passive inspection succeeds without evaluating it | Prepare successfully once resolved and preserve passive inspection |
| R8-F1.9 | Comparison boundary and admission controls | Compare a grouped list with separate homogeneous list alternatives. Check the existing moved-rule, recursive equality and incompatible raw-declaration rules | The mixed-list probe continues to fail, the checked-layer controls pass, and the raw branch requires matching property schema and requiredness without resolving a named declaration | Preserve the distinction between one list whose items vary and separate homogeneous lists. Keep real rule disagreements and the existing raw and recursive handling |

Use one resolved view of the compared declaration for both checks, retaining the use-site and definition rules at the levels where they apply. Reuse SimplifiedSchema's existing reference expansion and cycle protection where possible. The equivalence check needs that view both between payloads and inside root-union arms. Compare the declarations without rewriting the winner or its source. Keep the repair bounded to representations the resolver actually produces. A public fixture that forces a definition to be stored by using it twice can cover these forms, with the current grouping and rule-disagreement cases as controls. The guide's promise that named types are resolved before comparisons is the intended contract and should match the repaired behavior. [Darkmatter guide](../../docs/topics/host-registration.md#when-two-triggers-declare-one-property).

## Blocked Findings

None.

## Recurrence

R8-F1 is a newly found sibling of review 7's R7-F1, “Valid list narrowing fails when items are a union,” and review 6's R6-F1, “Trigger narrowing moves rules between union arms.” The previous repairs made direct item pairing and plain stored aliases work. The new traversal still stops at description-bearing or constrained aliases, and the equality traversal classifies stored and inline declarations differently. The prior sweep should have followed both representations into strict narrowing, equivalence, nested comparison and root-union ownership. The original cases continue to pass. Incrementally opening another spelling in each traversal has repeatedly left adjacent representations behind. The acceptance criteria explicitly require equivalent declarations and strict narrowing, so the repair should compare the resolved forms in those decisions without adding separate treatment for each spelling. [Review 7](review-7.md#recurrence), [review 6](review-6.md#unblocked-findings).

## Input Robustness Matrix

The default for invalid registration input is a located refusal where the source is known, with no usable host profile or execution from the refused bundle. An independent document may continue under Darkmatter's own rules. The new finding uses valid member files and arises after the bundle has loaded.

| Loader or validator | Public evidence and exceptions |
| --- | --- |
| Manifest YAML, member lists, function pairs and digest map | The snapshot suite exercises required fields, wrong shapes and values, duplicate and escaping members, digest coverage and mismatches, and explicit empty lists. The lifecycle member remains mandatory. Format 1 produces the dedicated upgrade warning. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs). |
| Member YAML loader and envelopes | The location and envelope suites check duplicate keys, trailing content and additional YAML documents, comments and a leading BOM, and the closed envelopes. Omitted optional fields remain legal. YAML-kind `types` has its documented nullable form, and an omitted trigger payload is matcher-only. [Location tests](../../lib/tests/l1/host_registration_locations.rs), [envelope tests](../../../simplified-schema/lib/tests/l1/member_envelope.rs). |
| Lifecycle JSON and artifact fields | The required-value checks distinguish null and absent where the artifact contract does. The strict reader rejects duplicate keys and trailing content. Registration tests check stale hashes and undeclared bindings, and installed fixtures verify the valid artifact. Empty effects and bindings are explicit. [Artifact reader](../../lib/src/markdown/compose/expression/host/artifact.rs), [registration tests](../../lib/tests/l1/host_registration.rs). |
| User configuration and setup | The configuration accepts the existing omitted, empty, and explicit-null registration-list forms. Setup tests confirm migration, preservation of permissions on conflict, and an unchanged config when publication fails. [Setup tests](../../../claudine/cli/tests/l1/setup_command.rs). |
| Named references and member roles | Public loader tests distinguish missing, unlisted, remote and escaping references from a listed file refused for its role or content. The writing declaration is retained where known, following the author's ruling. [Reference tests](../../../simplified-schema/lib/tests/l1/reference_error_provenance.rs), [location tests](../../lib/tests/l1/host_registration_locations.rs). |

I checked the manifest, member and artifact paths for deserialization defaults, nullable reads, and parse failures that could disappear through collection filtering. The reported overlap fails after each trigger is admitted, so these loader cases retain their existing outcomes.

## Verification

| Acceptance requirement | Verification level and evidence |
| --- | --- |
| Isolated portable bundle, revision and located refusals | Level 1 snapshot, installed and embedded registration tests exercise admission, copying and reordering, catalog freshness and source locations. The direct probes published Claudine into a temporary home outside the checkout. |
| Markdown and YAML selection, precedence and conflict | Level 1 library, CLI and DMLS tests cover authored values, shared events, literal mismatches, ambiguity, kind collisions, and local and document precedence. The checked-layer suite covers pairwise comparisons and exact-one inline composition. The named-type probe exposes the remaining equivalent-declaration and strict-narrowing gap. |
| Publication interruption, overlap, retry and unchanged timestamps | Level 1 publisher tests use controlled write points for overlapping reader and publishers and interruption. The inventory command checks published bytes and retains timestamps on an unchanged publish. [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs). |
| Editor updates and recovery | Level 1 controlled-clock tests cover settling across batches, an in-flight load superseded by a newer event, keeping an unchanged view and clearing repaired warnings. Protocol probes directly reproduce the new conflict. [Reload tests](../../dmls/tests/l1/registration_reload_scheduling.rs). |
| Inventory, setup and ordinary Claudine entry points | Level 1 CLI tests check inventory paths, bytes and hashes, setup migration and permission preservation, and ordinary startup with observed bundle access. The installed agreement tests compare shipped Claudine behavior with Darkmatter on Markdown and YAML. [Inventory tests](../../../claudine/cli/tests/l1/registrations_command.rs), [installed agreement](../../../claudine/lib/tests/l1/installed_registration_agreement.rs). |

The focused Linux nextest run passed 61 cases from the previous repair and source-location regressions. The broader Darkmatter, CLI, DMLS and SimplifiedSchema run passed 445 cases, including those 61. The Claudine publication, inventory, configuration and agreement run passed 91 cases. The longest test in those runs took 2.587 seconds. Level 1 reaches the relevant boundaries through public library results, shipped command processes and the language-server protocol. No requirement depends on terminal rendering or OS keyboard input. The previous full-area suites are recorded in the [iteration 7 repair log](log.md#implementation-of-review-findings-7).

The [repair log](log.md#review-8-record-correction--2026-11-23) and [implementation record](implementation-log.md#review-8-record-correction--2026-11-23) now describe the repaired list comparisons and the limits reproduced here.
