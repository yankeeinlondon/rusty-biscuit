---
$schema: feature-review.yaml
ready: false
findings:
  - "R5-F2 (high): Registration diagnostics omit declared display names"
  - "R5-F1 (medium): Example refusals point to successful imports or the importing trigger"
  - "R5-F3 (low): Same-host conflict warnings misstate YAML selection"
  - "R5-F4 (low): The source position helper overstates its column convention"
human-review: false
human-in-the-loop:
  - "R5-F1: repair the recurring source-location class with resolver provenance instead of another text-search fallback in the locator"
human-resolved: true
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-23T15:20:40+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: 2026-11-22-host-registration-v2/review-5.md
previous: 2026-11-22-host-registration-v2/review-4.md
---

# Host registration v2 review, iteration 5

**Production readiness is false.** Both iteration 4 repairs pass their required cases. Four new findings remain. They concern the location and labels in registration diagnostics, the warning for two competing installations, and the documentation of source column units.

## Previous Finding Verification

The instance numbers follow the [iteration 4 repair ledger](log.md#implementation-of-review-findings-4). Earlier reviews closed their carried findings. There are no human rulings in reviews 1–4.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R4-F2 — DMLS uses character counts as protocol columns for registration warnings | .1 Unicode in open and unopened files, .2 every warning source and publication path, .3 CLI and library columns, .4 surrounding editor diagnostics, .5 parser positions, .6 slot related information, .7 other character-column outputs, .8 validation's documented byte columns | **verified fixed** | The protocol test passed for ASCII, `café`, and `🧪 café` in UTF-8 and UTF-16 before and after opening the source. The config entry, manifest field, member, artifact and conflicting-manifest paths supply their source prefix to the same DMLS range converter. Refresh, open and close reuse the session's encoded range. The published Unicode fixture reported 22:66 in formatted and JSON CLI output and in the embedded loader, and 21:69 in a UTF-8 editor session and 21:66 in UTF-16. Document-only ambiguity keeps its file-level anchor. Host event keys, YAML kind selection and schema-document diagnostics use their source spans. The lifecycle JSON parser and YAML error paths preserve character positions. The slot protocol, slot output, YAML-load and YAML-edit tests passed. A direct `md compose` run confirmed the documented byte-column exception. | R5-F4 records a newly introduced comment that overstates the character-column rule. |
| R4-F1 — Registration warnings can point into a property description | .1–.4 unused declarations in all member kinds, .5 used trigger/YAML/namespace payloads, .6 shared declarations and importers, .7 embedded bundles, .8 bare carrier and prose controls, .9 object composition and other reference errors, .10 output routes, .11 comments and guide | **verified fixed** | The public loader tests passed for neighboring prose in all four member kinds, used and unused declarations, a root union arm, shared importers, the embedded loader, bare carriers, top-level and same-declaration prose, object composition, missing and remote references, and refused profiles. Formatted and data output and the language-server protocol preserved the corrected positions. The locator masks descriptions before examining declarations, including the shared carrier choice for a named type. The comments and guide explain this behavior and the carrier fallback. | R5-F1 is a newly found sibling where the earlier spelling is a successful import or the failed example has lost its originating file. |

## Unblocked Findings

### R5-F2 (high) — Registration diagnostics omit declared display names

**Authority — spec §1.** “Diagnostics, `md schema triggers`, and the `claudine registrations` listing show [the name] beside the path.” After changing `required` to `requird` in the published inline-composition trigger and refreshing its digest, the CLI and DMLS correctly report the match condition at 25:23. They omit its declared name, `inline-compose-schema`. The defect class is losing optional display metadata when a schema member becomes a diagnostic subject. [Registered member preparation](../../lib/src/config/registration.rs#L1008), [registration warning conversion](../../lib/src/config/registration.rs#L656).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R5-F2.1 | Registered trigger envelope, identification and payload preparation | The shipped named trigger with a match typo, a match using only the shared `title` key, or an invalid file-backed example | The refusals preserve the field or source path and omit `inline-compose-schema` | Show the valid name with the trigger path on each refusal |
| R5-F2.2 | Registered library preparation and imported-library refusals | Give shipped `shared.yaml` a valid top-level `name: review-shared`, then add an unlisted import to `types`. Inspect the adjacent path for errors reached through importers | The direct installed and embedded warnings give the source and manifest entry only. Importer text is built from the manifest field and relative path, so it has no place for either valid label | Preserve the library label and identify any importer by its own valid label and path, wherever the error carries importer context |
| R5-F2.3 | Checked trigger conflicts in validation, composition, inspection and DMLS | Change the named `composition.yaml` trigger to match `prompt: string(required)` and declare `agent: number`. The existing named inline trigger also matches and supplies an incompatible `agent` type | Both trigger paths survive and preparation fails. Formatted inspection lists the names in its normal trigger rows, while the conflict itself names paths only. The shared error used by validation, compose and DMLS also contains paths only | Attach both display names to the conflict's paths and preserve the code, file links and failed preparation |
| R5-F2.4 | Embedded and installed registration output | Use the same named malformed trigger and library with the public embedded loader, JSON and formatted CLI output, and DMLS publications | All carry the refusal text without the label. The editor keeps the source range | Render the same valid label and path in every warning across these entry points |
| R5-F2.5 | Ordinary trigger discovery, successful registered inspection, inventory and duplicate-name checks | Compare a locally skipped named trigger, the unedited installed inline trigger, list and detail output, and the duplicate-name tests | Ordinary skipped-trigger discovery already keeps the valid name. The successful registered trace prints `inline-compose-schema (…/inline-composition.yaml)`. Inventory carries the optional `name` with the path. Duplicate warnings identify the name on both declarations | Preserve these results. Use path only when no valid label is available |
| R5-F2.6 | Manifest, YAML-kind and namespace errors and configuration warnings | Inspect the category of the error before adding display metadata | Those envelopes have no trigger/library display name. Failure before the bundle verifies cannot trust the member text | Preserve the existing entry or file reference for these errors |
| R5-F2.7 | Claudine build inventory rejection | Run the actual build script against temporary copies of `inline-composition.yaml` and `sequence.yaml`, each with an unexpected `review-typo` key and its existing valid name | The shared inventory checker refuses both and prints only the filenames in the build failure | Report `inline-compose-schema` and `sequence-schema` beside the respective file paths |

Carry a valid display label from the already admitted member text when building a warning, and from the matched triggers when building a conflict. Reuse the ordinary trigger label rules for invalid triggers with a valid `name` field. A bad `name` should still be the problem the reader sees. Preserve the paths used by imports and editor navigation. [Ordinary trigger label extraction](../../../simplified-schema/lib/src/triggers/discovery.rs#L544), [registered conflict input](../../lib/src/config/registration/merge.rs#L77), [editor conflict projection](../../dmls/src/diagnostics/frontmatter.rs#L409).

### R5-F1 (medium) — Example refusals point to successful imports or the importing trigger

**Authority — spec §3.** Registration must “Report the failed field and source location.” A schema author can use a bundled type library successfully and accidentally choose that same file as an example. Darkmatter then points at the working import. An example inherited from a shared declaration can instead point at the importing trigger's payload, which contains neither the example nor the path in the error. The defect class is recovering a failed file reference by filename after discarding the operation or declaration that failed. [Darkmatter's location recovery](../../lib/src/config/registration/locate.rs#L65), [SimplifiedSchema's example resolution](../../../simplified-schema/lib/src/resolve.rs#L5139).

Publish Claudine to a temporary home. In the first payload arm of `schemas/inline-composition.yaml`, add `review-entry: "string(example(./shared.yaml))"` after the existing `$extends` and `prompt` lines, then refresh the member's BLAKE3 digest in the manifest. `md schema validate` correctly refuses to use the modified registration. Its warning points at `./shared.yaml` in the valid `$extends` on 44:35. The declaration to fix is at 46:37.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R5-F1.1 | Trigger payload, including a root union and object composition | Add the bad example after the valid `$extends` in the shipped inline trigger. The ordinary trigger payload and the union arm both resolve example annotations after preparing the type | Installed and embedded loaders choose the successful `$extends` at 44:35 | Locate the example in its own arm at 46:37 |
| R5-F1.2 | Registered YAML-kind payload | Append `review-entry: "string(example(./sequence.yaml))"` to the shipped formal sequence definition's payload, below its working `step[]` import | Installed and embedded warnings point at 21:33 | Point at 22:35 |
| R5-F1.3 | Registered namespace payload | Add a valid `label: hash@./document.yaml` to `loop.yaml`, then add the bad example with the same path | Installed and embedded warnings point at 13:17. The host is refused once | Point at 14:35 and preserve the single refusal |
| R5-F1.4 | A used shared-library type and both importing triggers | Change the shipped `last_updated` under `common-attributes` to `"string(example(./document.yaml))"`, keeping the good `hash` import just above it | Installed and embedded warnings point at `composition.yaml` 94:5 and `inline-composition.yaml` 44:5. The error carries the example's normalized absolute path and loses the library that wrote it | Point at `shared.yaml` 31:39 and name the importing trigger in each warning |
| R5-F1.5 | CLI and DMLS projection and downstream host decisions | Run formatted and JSON validation and open Markdown in a UTF-8 DMLS session with each change | The CLI prints the wrong source. The editor publishes the corresponding wrong range, for example 43:34 for .1. Failed registrations contribute no trigger or YAML-kind definitions | Carry the corrected source through both CLI forms and DMLS. Preserve refusal and fallback selection |
| R5-F1.6 | Single reference, unused declarations and boundary failures | Add just the example to `loop.yaml`, leaving no working import of that path. Separately add an unused example of a listed file to `shared.yaml`. Check the unlisted, remote, escaping and bare-payload tests | A sole bad example points at its example argument. An unused listed example passes boundary admission without loading its contents. The public loader checks unlisted and escaping references and the other carriers independently | Preserve these results. Manifest paths, digest failures, typed named-type errors and artifact fields have separate locators |

Prefer the failed example's authored declaration and source file from the resolver. Where the error already identifies an invalid example in its own member, exclude successful imports from the fallback search. When resolving a shared example, retain the declaration's source before normalizing its path. Pointing at a known declaration is sufficient if the exact argument span is unavailable. Extending a second grammar inside the locator to guess from filenames would cost more and leave the ambiguity in place. Update the guide's first-written rule to reflect what can be located with the information the resolver retains. [Registration diagnostic guide](../../docs/topics/host-registration.md#where-a-refusal-points).

### R5-F3 (low) — Same-host conflict warnings misstate YAML selection

**Authority — defect.** The Darkmatter library's [same-host conflict warning](../../lib/src/config/registration.rs#L590) says both registrations are unused. That is accurate for Markdown. The [registered YAML index](../../lib/src/config/registration/selection.rs#L111) keeps both installations' definitions, so each installation can still be used for a kind only it claims.

Copy a published Claudine installation to a second directory and register both manifests. Equal copies select one `sequence` definition without warnings. In the second installation, change the formal kind literal to `other-sequence` and refresh its digest. The CLI now emits two “neither is used” warnings. It still validates `kind: sequence` through the first installation and `kind: other-sequence` through the second. A document with `kind: other-sequence` and no `sequence` key fails with the second definition's required-property error. DMLS also publishes the warning and uses the second installation's schema for the required-property diagnostic. An inline Markdown prompt falls back to Darkmatter with no registered trigger layer, as intended.

Keep the recorded decision to index both installations' YAML kinds and reject duplicate kind claims. Give the conflict warning its actual scope and explain the same distinction in the guide. Both the CLI and DMLS publish this common warning. [Recorded implementation decisions](implementation-log.md), [host agreement guide](../../docs/topics/host-registration.md#two-manifests-for-one-host).

### R5-F4 (low) — The source position helper overstates its column convention

**Authority — defect.** The Darkmatter library's public [`line_column` helper](../../lib/src/config/load.rs#L1277) converts a byte offset to one-based character coordinates for registration and other authored diagnostics. Its comment says this convention applies to every published `line` and `column`. A developer using that description to interpret a Markdown frontmatter validation record gets the wrong offset after multibyte text.

With `{title: "😀", $schema: {count: "number(required)"}, count: nope}` on the second line of a Markdown document, `md compose --output json` reports `dm.schema.type_mismatch` at 2:62. That is the byte column of `nope`. Its character column is 59. Keep the documented validation result and scope the helper comment to what the helper returns. [CLI diagnostic coordinate contract](../../docs/cli/data-contracts.md#diagnostics).

## Blocked Findings

None.

## Recurrence

R5-F1 is a new sibling of **R4-F1** in review 4, **R3-F1** in review 3, **R2-F2** in review 2, and **R1-F11** in review 1. Those repairs located failed imports and examples where the same spelling appeared in a description and located several failures inside imported libraries. Their sweep missed cases where an earlier use of the path succeeded, and where resolving an inherited example normalized its path before recording who wrote it. The repairs have expanded string-based location recovery each time. A known declaration and source file bound the repair without requiring token-level reconstruction of every spelling. R5-F2 is a display-label omission. Earlier findings about label validation and duplicate-name positions do not cover it. R5-F4 is new documentation drift from the R4-F2 repair. The documented validation exception checked under R4-F2.8 still applies.

## Input Robustness Matrix

Invalid registration input produces a located refusal. It publishes no host profile and runs no host code. Validation can continue on an independent document with the default Darkmatter rules. The reference finding above affects which source that refusal reports.

| Loader or validator | Evidence and exceptions |
| --- | --- |
| Manifest envelope, member list, function pairs and digests | The snapshot suite checks missing fields, nulls, wrong containers and elements, malformed versions, missing and stray digests, duplicate paths and mismatched bytes. An explicit empty `files` or `functions` list is valid. Distinct list and digest spellings are checked before indexing and reading. |
| Shared YAML document loader | SimplifiedSchema's public loader cases exercise duplicate keys, malformed trailing input, an extra YAML document, comments, and the BOM control. A fresh syntax error after `🧪 café` in each of the four published member kinds kept the parser's character location. Manifest, member and standalone YAML routes use the same loader for these diagnostics. |
| Member envelope, named declarations and match conditions | The shared envelope cases cover absent, null and invalid required fields, closed keys and collection shapes. A YAML-kind `types` may be absent or null. An explicit empty YAML-kind table follows the standalone loader's rejection rule. Trigger and namespace tables can be absent or empty and a library requires a populated table. A matcher-only trigger can omit its payload or declare it null. Trigger match tests cover optional alternatives, invalid and mixed match shapes, forbidden constraints and actual literal matching. |
| Lifecycle and function artifacts | The strict JSON readers reject duplicate keys, trailing data and malformed required fields. The lifecycle `effects` cases exercise missing, null, wrong container, mixed and invalid elements and the allowed empty list through the same reader. The function catalog checks cover malformed YAML signatures and disagreement with facts. |
| Configuration and setup | Tests cover missing configuration, absent and empty registration lists, duplicate entries, malformed configuration, preexisting permissions, conflicting permissions and migration. Existing config treats null `register` as contributing no entries. A successful publish still registers the new manifest after a permissions conflict that preserves existing permissions. |

I checked the readers for permissive defaults on required fields, silent filtering of malformed elements and swallowed parse errors. The defaults above are exercised through the relevant library result. The installed CLI cases and DMLS publications verify the output paths.

## Verification

| Spec behavior | Appropriate level and evidence |
| --- | --- |
| Isolated bundle, revisions and refusals | Level 1 public installed and embedded loaders, including a real Claudine installation in a temporary home. Tests cover digests, ordering, bundle boundaries, artifact freshness, namespace names and source locations. R5-F1 and R5-F2 qualify the reported diagnostics. |
| Host selection, checked merge and YAML-kind precedence | Level 1 CLI, library and DMLS protocol cases cover literal matching, shared events, malformed host-specific values, competing hosts, inline alternatives, checked conflicts and registered/local/shipped YAML-kind ordering. R5-F3 qualifies the wording of a same-host conflict. |
| Publication, editor refresh and setup | Level 1 publisher and snapshot tests cover interruption, competing publishers, retry, unchanged timestamps and a reader spanning publication. DMLS controlled-clock and protocol tests cover supersession and warning removal. Setup and inventory tests cover the published bytes, permissions, migration and ordinary startup with zero registration reads. The earlier repair's recorded publish timing supplies the settle-window measurement. |

Two focused Nextest runs passed 279 and 93 tests across the three areas, with a slowest test of 2.472 seconds. The feature-enabled Claudine CLI run passed all 13 registration command tests, including the ordinary-command read guards with setup and publication as controls. SimplifiedSchema's `just test` passed 2,563 tests with the recipe's feature set and 2,158 without default features. I checked the previous repair's full Darkmatter and Claudine Linux reports. They list 17,770 and 12,724 passes respectively and zero tests at or above five seconds. Each user-facing requirement here is observable through a library result, a CLI subprocess, or the editor protocol. None requires terminal rendering or operating system keyboard injection.

The implementation log's character-column statement now reflects the documented validation exception. [Record correction](log.md#review-5-record-correction--2026-11-23).

## Human rulings

### Item 1

- `Ruling:` Repair R5-F1 by carrying provenance from SimplifiedSchema's resolver, not by extending the text search in Darkmatter's registration locator. Every SchemaError variant that today is located by searching the member text for its `reference` string (`InvalidExample`, `BundleBoundary`, `NoMatch`, `Unresolved`, `RemoteUnsupported`, `BareNameSiblingExists`, `TriggerSchemaReferenced`) carries the source that wrote the reference and the structural path of the declaration holding it. Record that provenance where the reference is first read, before its path is normalized or the declaration is inherited through `$extends`, a spread, or a named-type import. The existing `SchemaReferenceSite` (source plus declaration path), which the named-type errors already carry, is the model, and reusing it is preferred over a new type. The locator then points at that declaration in that file. When the exact argument span is unknown, the declaration is the answer: no fallback searches the text for the reference by filename. When an error genuinely has no provenance, it stays file-level (unlocated) rather than guessed.
- `Why:` This is the fifth instance of the same class (R1-F11, R2-F2, R3-F1, R4-F1, R5-F1). Each repair widened the string-based recovery in `darkmatter/lib/src/config/registration/locate.rs`, and each review found a new spelling it misattributes. Reviews 4 and 5 both recommended stopping at a known declaration. Another heuristic would create R6's finding.
- `Changes:`
    - SimplifiedSchema: add the reference site to the variants listed above and populate it at every resolver site that raises them, including `example(...)` resolution in `resolve.rs` and resolution of shared declarations reached through importers. Update the error type's docs and the SimplifiedSchema skill/docs where they describe these variants.
    - Darkmatter: route these variants through `reference_site` (or its equivalent) in `locate.rs`, and delete the `file_reference` text-search path and its helpers (`reference_in`, `in_example`) once nothing uses them. Do not keep them as a fallback. Where an error reached through an importer (R5-F1.4), point at the declaring library and name the importing trigger in the warning text.
    - Tests: R5-F1.1–.6 as written in this review, plus one regression test per earlier sibling instance (R1-F11, R2-F2, R3-F1, R4-F1) that still passes through the new path, so removing the text search loses no coverage.
    - Docs: rewrite the "where a refusal points" section of `darkmatter/docs/topics/host-registration.md` to state the rule plainly: a refusal points at the declaration that wrote the failing reference, in the file that wrote it, and is file-level when that is unknown.
    - Record the change of approach in the implementation log as a departure from the earlier repairs.
