---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F10 (high): Namespace and member validation differ across entry points"
  - "R3-F1 (high): Registration warnings lose locations for resolved kind claims and file references"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-23T12:39:41+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: true
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/log.md"
implemented-by: claude/opus
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: "2026-11-22-host-registration-v2/review-3.md"
previous: "2026-11-22-host-registration-v2/review-2.md"
next: "2026-11-22-host-registration-v2/review-4.md"
---

# Host registration v2 review, iteration 3

**Production readiness is false.** The four findings open in review 2 passed verification for their recorded instances. An earlier envelope case regressed, and newly found cases share a defect in registration warning locations. Both findings have repairs within the specification.

## Previous Finding Verification

The ranges identify child rows from the [repair ledgers](log.md#finding-ledger). Review 2 verified the other closed findings from review 1. The table includes the earlier envelope finding that reopened in this iteration.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F11 — Bundle refusals lose the failed field's source location | R1-F11.1–.18, including .3a–.3c, and .19 | **verified fixed** | The published Claudine trigger with `requird` in its grouped match now warns at 25:23. Focused tests exercised flat and grouped conditions, forbidden constraints, malformed and valid match shapes, ordinary discovery, embedded preparation, CLI and DMLS output, and the editor route where kind selection fails. Checked the previous manifest, library, namespace, artifact and malformed-member controls against the implementation and full L1 reports. | R3-F1 records newly found siblings. |
| R2-F1 — Unused type declarations can bypass bundle import admission | R2-F1.1–.7 | **verified fixed** | Appending an unused unlisted import to the published shared library warned at the import and withheld the host. Focused tests exercised all four kinds, listed and used controls, remote imports, file-backed examples and the embedded prepared-trigger case, including Claudine's actual inventory. Full L1 reports cover pending slots and function and lifecycle controls. | R1-F10 covers a separate regression in the new check. R3-F1 covers new location cases after a correct refusal. |
| R2-F2 — Type resolution errors point to the wrong declaration | R2-F2.1–.6 | **verified fixed** | Focused tests exercised root-union grammar and unknown types, object inheritance, namespace arms and carrier selection, imported library arms, transitive references and examples with importer context, and registered YAML controls. Standalone semantic, CLI arm and DMLS arm cases passed. Manifest, artifact and warning projections retain their controls. | R3-F1 records newly found siblings. |
| R2-F3 — Missing required member fields have no source position | R2-F3.1–.5 | **verified fixed** | Focused tests exercised omitted `match`, `$schema`, `namespace`, `types` and `kind`, including library importers, embedded, ordinary discovery, and CLI output. The published namespace without its name warned at 4:1. Invalid present matches, valid empty namespace payload, manifest omission and build controls retain their behavior. | None. |
| R1-F10 — Namespace and member validation differ across entry points | R1-F10.1–.13 | **regressed** | The namespace, build envelope and selection cases retain their tests. The standalone CLI still accepts null optional `types` on a YAML-kind definition. The same shape now removes the entire installed host. The second declaration check calls a type-table parser after the envelope and the standalone preparation have admitted the member. | Restore the nullable YAML-kind case in installed and embedded preparation. |

## Unblocked Findings

### R1-F10 (high) — Namespace and member validation differ across entry points

**Authority — regression and spec §5.** YAML lookup must “Reuse today's rules to determine which kind or kinds a definition claims.” The review 1 repair's R1-F10.13 decision preserved null optional YAML-kind metadata as equivalent to omission where the envelope already accepts it. A blank YAML key is realistic when editing or generating a schema. The defect class is a second field validator rejecting a shape that the owning validator already accepted.

Run `claudine setup` in a temporary home outside the checkout. Append `types:` to the published `schemas/formal-sequence-file.yaml` and refresh its digest in the manifest. A standalone copy still passes `md schema validate --output json`. With the edited member installed, the same command on Markdown containing `prompt: hello` emits a 22:7 registration warning and supplies no Claudine trigger. The warning cites a null `$schema` shape although the edited field is `types`. SimplifiedSchema's `check_declared_types`, which Darkmatter runs after member preparation, skips an absent key and passes a present null to the trigger table parser. [Member envelope](../../../simplified-schema/lib/src/member_envelope.rs#L225), [declaration check](../../../simplified-schema/lib/src/resolve.rs#L947), [Darkmatter preparation](../../lib/src/config/registration.rs#L1046).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F10.13 | Installed registration and registered kind lookup | Absent, blank, numeric, empty mapping and populated `types` in the published YAML-kind definition | Absent and populated supply the host. Numeric and empty mapping refuse at `types`. Blank passes the envelope and then aborts the profile before publishing its kind or trigger | Admit blank as a definition without named types. Keep all four controls |
| R1-F10.14 | Standalone YAML, DMLS definition checks, local kind lookup and Claudine structural inventory | Same five shapes | Direct CLI accepts absent, blank and populated and refuses numeric and empty. The editor uses that contract, the local tier uses standalone preparation, and the build uses the shared structural envelope with the null exception | Preserve those rules |
| R1-F10.15 | Embedded registration and Claudine's generated profile | Same definition passed to embedded profile loading | Both profile routes run the affected `prepare_members` path. The structural build check alone does not exercise that decision | Admit the blank field through the shared check and verify a working embedded kind |
| R1-F10.16 | Trigger, namespace and library declaration tables | Blank `types:` in the three published kinds, with the envelope's absent, populated, empty and numeric controls | All three blank fields refuse before the second check. Trigger and namespace may omit the table or write an empty one. Libraries require a nonempty table. Numeric values refuse in all three | Preserve these per-kind rules |

Use the envelope's representation of an absent optional type table, or skip a null table after the owning envelope admits it. Verify the installed kind and embedded profile using the same shape.

### R3-F1 (high) — Registration warnings lose locations for resolved kind claims and file references

**Authority — spec §3 and the first acceptance criterion.** “Report the failed field and source location.” The criterion explicitly requires located refusals for unlisted imports. The defect class is discarding a declaration's structural source before emitting its warning.

In a fresh temporary publication, change the formal sequence kind declaration to `kind: kind-token(required)`, add `types: { kind-token: string }`, and refresh the member digest. This represents extracting a named type for a kind discriminator and making that type too broad. The standalone CLI reports `/$schema/kind` at 12:11. The installed warning has null coordinates because the resolved-claim conversion discarded the path it computed. In a separate edit, append `optional-host-settings: string(example(./options.yaml))` to a member's type table and mention `./options.yaml` in its description. The missing file correctly refuses the host, but its coordinates point into the description. [Claim projection](../../../simplified-schema/lib/src/yaml_documents/registry.rs#L693), [direct YAML semantic check](../../../simplified-schema/lib/src/yaml_documents/semantic.rs#L224), [reference spelling fallback](../../lib/src/config/registration/locate.rs#L171).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Installed and embedded YAML-kind preparation | Above named string claim, with direct string and shipped literal controls | The installed named-claim warning has null coordinates. Embedded shares that check. Direct string fails earlier with payload coordinates. The literal installs | Locate the named claim at its authored declaration |
| R3-F1.2 | Direct YAML validation, local kind discovery and registered kind selection | Same named claim and literal control | Direct CLI reports 12:11. The editor uses the same whole-YAML semantic result. Local discovery marks a failed resolved claim broken. Registered selection only sees prepared definitions | Preserve refusal and selection, and reuse the direct claim path for registration |
| R3-F1.3 | Trigger, namespace, YAML kind and shared-library declaration checks | Published copy of each with the unused file-backed example and the same path in prose | Trigger 21:19, namespace 6:19, YAML kind 10:18, and library 20:18 all point into the description | Point at the example reference or its declaration |
| R3-F1.4 | Used declarations, transitive imports and embedded registration | Compare the existing used-example and transitive controls with the above description shape. An unused `Name@./options.yaml` follows the same plain path in prose | The named import control points after `@`. Existing examples with one spelling and transitive diagnostics pass. The same unstructured fallback serves used examples and references written inside imported or embedded members | Preserve the writing member and importer context while locating the authored reference |
| R3-F1.5 | Trigger payload file references and nearby admission paths | Replace the published inline trigger payload with `$schema: ./options.yaml` while its description mentions that same path. Compare snapshot, envelope and artifact controls | The warning points to description 21:19 instead of payload 43:10. Snapshot, envelope and artifact controls carry their own field or parser positions | Use the carrier for a bare reference. Preserve independent field and parser positions |
| R3-F1.6 | Formatted and JSON CLI output and DMLS registration warnings | The published refusals and the existing known-position controls | JSON carries the missing or wrong position from admission. The formatted CLI and DMLS tests preserve known admission positions | Fix the location in preparation so every projection can use it |

Keep the resolved claim path until it can be applied to the verified member text. For file references, prefer the declaration or carrier the resolver was reading. Use the nearest known declaration where a more precise reference position is unavailable. A general-purpose source mapper is unnecessary for these cases.

## Recurrence

- **R1-F10** regressed the explicit null case of the review 1 envelope finding. The review 2 repair added a second validation step. The sweep covers all four member kinds, their shared envelope, standalone and local YAML validation, and embedded preparation.
- **R3-F1** contains siblings of **R1-F11** in review 1 and **R2-F2** in review 2. Those repairs covered the already reported declarations and simple reference spellings. The sweep missed the resolved YAML-kind claim's conversion and the reference fallback when prose repeats a path. Rows R3-F1.1–.6 follow those paths through preparation, kind selection, and warning projections.

## Input Robustness Matrix

Invalid shapes produce a located error and publish no profile or partial definition. An unrelated Markdown document can still validate using Darkmatter's default schema. The tests observe the registration warning and the selected layer.

| Validator or loader | Public evidence and exceptions |
| --- | --- |
| Manifest envelope, member paths and digest map | The manifest and snapshot tests cover required fields, null and wrong container types, mixed and wholly invalid elements, unknown keys, missing and extra digests, duplicate resolved paths and digest mismatch. Empty `files` and explicit empty `functions` are valid. Omission of either list fails. |
| Shared YAML loader and member envelopes | Duplicate keys, trailing invalid syntax and a second YAML document refuse loading. Comments and quoted text remain valid. The member envelope tests vary absence, null, wrong type and empty values, including invalid categories and match list elements. A types-only library needs a nonempty table. Triggers and namespaces permit empty type tables, and a matcher-only trigger can omit its payload or specify null. R1-F10 records the YAML-kind optional-null regression and its controls. |
| Match conditions | The direct and registration tests cover absent, null, numeric, empty, incorrect elements, flat and grouped grammar, forbidden references and valid controls. Bad conditions supply neither a selection nor a payload. |
| Lifecycle and function facts JSON, and function catalog YAML | The JSON readers share strict duplicate-key and trailing-content parsing. The public lifecycle `effects` cases cover absence, null, scalar, mixed and invalid elements. Empty effects is valid. Separate registration tests cover stale hashes, required data, unknown namespace bindings, function facts and YAML catalog signatures. |

Reviewed the loaders for defaulted values, optional fields, silently dropped elements and swallowed parser errors. The optional-null mismatch occurs after the owning envelope's decision. R3-F1 concerns positions attached to later preparation errors.

## Verification

| Specification behavior | Level 1 evidence |
| --- | --- |
| Complete isolated bundle, located refusals and stable revision | Published Claudine outside the checkout and exercised the CLI with controlled member edits. The verified-snapshot, manifest, embedded and namespace tests cover reordering, equivalent installations in different roots, digests, duplicates, imports and invalid matches. The two open findings qualify refusal and admission behavior. |
| Host choice and kind precedence | Claudine inventory and installed agreement tests cover shared events, wrong literal kind and malformed host fields. Darkmatter host-selection, CLI and DMLS tests cover ambiguity, precedence and kind collisions. |
| Matched declaration merging and inline alternatives | Darkmatter tests cover equivalence, narrowing, presence and constraint conflicts, three-way overlap and incompatible triggers that never match together. Claudine tests cover the two inline alternatives. CLI and DMLS tests exercise conflict delivery, higher layers and editor views. |
| Publication and editor reload | Publication tests interrupt member writes and the manifest commit and exercise reader and competing-publisher overlap, convergence and unchanged timestamps. Controlled-clock and protocol tests cover batching, supersession, unchanged reloads, and warning removal after repair. |
| Inventory, setup and normal startup | Claudine library and CLI tests compare paths, hashes and contents. Temporary-home tests cover migration and permissions. Configured ordinary commands assert an empty read-access log, with setup and publish as positive controls. |

Level 1 is appropriate for the CLI, library and language-server results. The specification has no real-terminal rendering or input requirement.

The implementation's raw Linux Nextest reports show 17,740 Darkmatter tests, 12,723 Claudine tests, and 2,559 SimplifiedSchema tests plus 2,155 without default features passing. The three canonical reports recorded zero slow tests. The review's combined Nextest rerun stopped during compilation when the filesystem ran out of space. As a fallback, I ran 95 focused cases from the available test binaries in separate processes. All 95 passed, with wall-clock times under one second per case. The temporary-home CLI probes independently reproduced both open findings.
