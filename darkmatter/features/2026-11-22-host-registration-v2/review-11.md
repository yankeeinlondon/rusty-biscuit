---
$schema: feature-review.yaml
ready: true
findings: []
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: "codex/gpt-6.2-sol"
recurrence: false
created: "2026-11-23T22:25:22+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: 2026-11-22-host-registration-v2/review-11.md
previous: 2026-11-22-host-registration-v2/review-10.md
---

# Host registration v2 review, iteration 11

**Production ready.** The repair preserves the narrower list declaration and its source when both triggers agree on the applicable rules. All seven instances from review 10 are verified fixed, with no new findings.

## Previous Finding Verification

The required instances come from the iteration 10 repair ledger. Earlier reviews left no open findings. The author's ruling in review-5.md, item 1 remains in force. SimplifiedSchema records the declaration that wrote a refused reference, and Darkmatter reports that declaration or uses a file-level diagnostic when its position is unknown. The reference-provenance and registration-location suites cover the rule. [Repair ledger](log.md#implementation-of-review-findings-10), [prior verification](review-10.md#previous-finding-verification), [author's ruling](review-5.md#item-1), [SimplifiedSchema provenance tests](../../../simplified-schema/lib/tests/l1/reference_error_provenance.rs), [Darkmatter location tests](../../lib/tests/l1/host_registration_locations.rs).

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R10-F1 — Matching triggers reject a list whose item type is narrower | R10-F1.1 — Plain lists and grouped item alternatives, both file orders and all three payload placements | **verified fixed** | The library test compared the kept schema and origin with the narrower trigger alone. Plain and grouped lists accepted allowed enum items and rejected removed items. It also checked a grouped list that retained its number alternative while narrowing its text alternative. The installed CLI passed the plain and grouped enum controls. [Library tests](../../lib/tests/l1/host_trigger_merge.rs#L335). | None. |
| R10-F1 — Same finding | R10-F1.2 — Named item union and stored list declarations | **verified fixed** | The same public-API matrix passed for a named item union and two reused list definitions. Both spellings kept the narrow declaration and its origin and rejected an item only the wider declaration admitted. [Named type cases](../../lib/tests/l1/host_trigger_merge.rs#L400). | None. |
| R10-F1 — Same finding | R10-F1.3 — Enum list against a union of whole lists, and homogeneous lists against a mixed list | **verified fixed** | The installed CLI accepted `[claude]` and rejected `[3]` when the enum list narrowed `string[] \| number[]`. The union of homogeneous lists won against a mixed list, accepted `[claude]`, and rejected `[claude, 3]` as a value error. The library tests verify the winner and origin in both file orders and all three placements. [Whole-list tests](../../lib/tests/l1/host_trigger_merge.rs#L385). | None. |
| R10-F1 — Same finding | R10-F1.4 — Agreed and changed rules at the item and list levels | **verified fixed** | Agreement on both minima and the list description accepted the narrower alphanumeric list. Changing the item minimum or dropping the list minimum produced a conflict at the corresponding level. Library controls and direct CLI runs agreed. [Rule comparison tests](../../lib/tests/l1/host_trigger_merge.rs#L425). | None. |
| R10-F1 — Same finding | R10-F1.5 — Root-union ownership and origin, static cache, CLI, DMLS, embedded profiles, and higher-layer precedence | **verified fixed** | Repeating the same list in each root arm keeps it shared in both trigger orders and preserves each arm's other fields. Validate, compose, trigger inspection, and clean succeeded with the installed bundle. The CLI and DMLS tests verify that a bad item is rejected after preparation. Static calls reuse the result for the selected set. Embedded profiles use that same check, and the precedence tests verify that document and repository overrides apply only after the checked layer succeeds. [CLI tests](../../cli/tests/l1/host_registration.rs#L726), [editor tests](../../dmls/tests/l1/registered_trigger_conflict.rs#L207), [cache and union tests](../../lib/tests/l1/host_trigger_merge.rs#L1001), [profile](../../lib/src/config/registration.rs#L203). | None. |
| R10-F1 — Same finding | R10-F1.6 — Either side supplied by a deferred template, with passive inspection | **verified fixed** | Public-API tests resolve each side in turn, including a narrow root-union payload, and check both a good and a bad value. With the installed bundle, validate and compose succeeded on both variants. Trigger inspection reported the template as pending, with no conflict. [Deferred tests](../../lib/tests/l1/host_trigger_merge.rs#L569). | None. |
| R10-F1 — Same finding | R10-F1.7 — Equality, scalar narrowing, item removal, nested identity, incompatible lists, and raw schema controls | **verified fixed** | Reordered items, nested equivalent lists, recursive controls, scalar enum narrowing, and pure item removal prepared successfully. A mixed list still conflicted with `string[] \| number[] \| boolean` in every placement and order. Raw schemas branch before type coverage and still require equal property schemas and requiredness to coalesce. [Merge tests](../../lib/tests/l1/host_trigger_merge.rs), [raw comparison](../../lib/src/config/registration/merge.rs#L281). | None. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No new instance reproduced in the earlier comparison or diagnostic-location classes. Direction selection and rule pairing now use the same coverage rule over resolved alternatives. The comparison keeps a list's item alternatives inside that list, which preserves the difference between mixed and homogeneous lists. Root-union ownership still asks whether declarations in its arms are equal. The probes exercised both comparison orders, object and root-union payloads, named and inline forms, and changes to either level of rules. [Comparison code](../../lib/src/config/registration/merge.rs#L281), [coverage and rule pairing](../../lib/src/config/registration/merge.rs#L498), [root-union ownership](../../lib/src/config/registration/merge.rs#L945).

## Input Robustness Matrix

The feature introduced bundle and artifact readers. Invalid inputs refuse the affected host profile with a source position when known. Independent documents can still use Darkmatter's own schema. The iteration 10 repair changes comparisons of already admitted declarations. The table records the loader and validator evidence and the accepted exceptions.

| Loader or validator | Public verification and exceptions |
| --- | --- |
| Manifest YAML and member specifications | Snapshot tests exercise absent required fields, explicit null, wrong whole-field and element types, duplicate keys, path escapes, digest mismatches, trailing invalid content, and format 1 refusal. The file and function lists may be explicitly empty. The lifecycle entry and its digest remain required. Comments and layout changes preserve the normalized revision. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs). |
| YAML member envelopes and named references | The shared YAML loader refuses duplicate keys, invalid trailing content and extra documents, and accepts ordinary comments and quoted values. The shared validators and installed tests distinguish missing and malformed fields, invalid collection elements and prohibited reference targets. A trigger or namespace may omit or write an empty local type table. A YAML kind may omit `types` or use null. A library needs a populated table. A trigger with omitted or null payload can identify a host without adding a schema. [Envelope tests](../../../simplified-schema/lib/tests/l1/member_envelope.rs), [location tests](../../lib/tests/l1/host_registration_locations.rs). |
| Lifecycle and function-facts JSON | The strict loader rejects duplicate keys and trailing values. Required fields reject null and wrong shapes. Empty owned collections are valid where the artifact contract permits them. Registered profile tests check stale artifacts and undeclared namespace bindings. [JSON reader](../../lib/src/markdown/compose/expression/host/artifact.rs#L660), [host registration tests](../../lib/tests/l1/host_registration.rs), [lifecycle tests](../../lib/src/lifecycle/host_profile/tests.rs). |
| Registration list in configuration | Missing, null, and empty lists supply no registrations. Invalid whole fields are reported. Malformed items are reported at their entries and valid siblings remain usable. The string projection runs after the list diagnostics. The lenient member-name projection supplies a display label only. [Configuration tests](../../lib/tests/l1/config_fields.rs#L319), [configuration reader](../../lib/src/config/load.rs#L749), [display label reader](../../../simplified-schema/lib/src/member_envelope.rs#L224). |

## Verification

All requirements below are covered at Level 1. They depend on filesystem state, schema results, command output and exit status, or editor protocol messages. They do not require a real terminal or OS input.

| Acceptance requirement | Evidence |
| --- | --- |
| Portable bundle, stable revision, and located refusals | Snapshot and registration tests cover copying, file reordering, missing and unlisted imports, duplicates, stale artifacts, invalid matches, and diagnostic locations. Direct CLI probes used a published bundle in a temporary home and repository outside the checkout. [Snapshot tests](../../lib/tests/l1/host_registration_snapshot.rs), [location tests](../../lib/tests/l1/host_registration_locations.rs). |
| Markdown host choice and YAML kind lookup | Library tests cover shared events, required literals, malformed host-specific values, and ambiguity. CLI and editor tests cover the registered kind tier, built-in and local precedence, and collisions. [Selection tests](../../lib/tests/l1/host_selection.rs), [CLI registration tests](../../cli/tests/l1/host_registration.rs), [editor agreement tests](../../dmls/tests/l1/registered_host_editor_agreement.rs). |
| Checked overlaps, inline composition, and upper layers | Public library tests cover equivalence, narrowing, rules and presence conflicts, three-way comparisons, and triggers that never match together. Library, CLI and editor tests exercise the prompt-or-sections union, preserved root rules and higher layers. The installed CLI and DMLS probes confirm the repaired list comparison. [Merge tests](../../lib/tests/l1/host_trigger_merge.rs), [CLI tests](../../cli/tests/l1/host_registration.rs), [editor tests](../../dmls/tests/l1/registered_trigger_conflict.rs). |
| Publication interruption, reader and publisher overlap, retry, and timestamps | Publisher tests stop between file writes and before the final manifest, overlap a reader with publication, serialize two publishers, and compare retry and unchanged-install bytes and timestamps. [Publication tests](../../../claudine/lib/tests/l1/host_publication.rs#L168). |
| Debounced editor reload, supersession, and recovery | Controlled-clock tests cover multiple notification batches, continued use of the previous view until the reload, changes to Markdown and YAML analysis, recovery without reopening, and suppression of unchanged reanalysis. Queue tests control completion order and verify that a superseded result is discarded. [Scheduling tests](../../dmls/tests/l1/registration_reload_scheduling.rs), [queue implementation and tests](../../dmls/src/overlay/hosts.rs#L408). |
| Inventory, setup, initialization, and normal Claudine commands | Tests compare inventory and detail bytes with publications, cover registration migration and permissions conflicts, and observe whether ordinary commands read or refresh the published tree. The `md init` test checks that an existing registration and permissions reference survive. [Inventory and startup tests](../../../claudine/cli/tests/l1/registrations_command.rs), [setup tests](../../../claudine/cli/tests/l1/setup_command.rs), [init tests](../../cli/tests/l1/init_command.rs). |

On Linux, Darkmatter's three Level 1 runs passed 64 merge tests, 223 registration, selection, and editor tests, and 27 initialization, registered YAML, and reload-queue tests. Claudine's focused run passed 85 tests across publication, inventory, setup, and installed and embedded validation. None of these runs recorded a test at or above five seconds. SimplifiedSchema's member-envelope and reference-provenance selection passed 23 tests, followed by the recipe's 2,162-test integration run. Direct CLI and editor probes used the binaries in `target/debug` and temporary installations.

The implementation record's Phase 3 summary named the superseded comparison helper. I corrected it in place and added a dated note with the verification. The recorded iteration 10 full Darkmatter Level 1 run reported 17,847 passed, 10 skipped, and zero tests at or above five seconds. [Implementation record correction](implementation-log.md#review-11-record-correction--2026-11-23), [recorded full run](log.md#implementation-of-review-findings-10).
