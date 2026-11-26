---
$schema: feature-review.yaml
ready: false
findings:
  - "R4-F2 (high): DMLS uses character counts as protocol columns for registration warnings"
  - "R4-F1 (medium): Registration warnings can point into a property description"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-23T13:50:10+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: "2026-11-22-host-registration-v2/review-4.md"
previous: "2026-11-22-host-registration-v2/review-3.md"
next: "2026-11-22-host-registration-v2/review-5.md"
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/log.md
implemented-by: claude/opus
---

# Host registration v2 review, iteration 4

**Production readiness is false.** Both review 3 repairs pass their recorded cases. Two additional cases can send a reader to the wrong part of a file. Both have repairs within the spec.

## Previous Finding Verification

The required instances come from the [iteration 3 repair ledger](log.md#implementation-of-review-findings-3). Earlier findings closed in reviews 2 and 3 remain closed. The earlier reviews contain no human rulings.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F10 — Namespace and member validation differ across entry points | .13 installed shapes, .14 standalone/local/editor/build controls, .15 embedded and Claudine, .16 other member kinds, .17 documentation. Earlier .1–.12 were closed in review 2 | **verified fixed** | Installed and embedded tests covered absent, blank, explicit null, populated, numeric and empty tables. The actual Claudine YAML definition with a blank table kept its kind through the embedded profile and its formal sequence kind and Markdown trigger layer through the shipped CLI. The empty table control withheld the registered kind. Local lookup, direct YAML and the DMLS YAML contract use the admitted shape. The shared envelope tests exercised all four kinds. Trigger, namespace and library controls still refuse blank tables. The two topic pages describe the same per-kind rules as the code. | None for the recorded cases. |
| R3-F1 — Registration warnings lose locations for resolved kind claims and file references | .1 installed/embedded claim, .2 direct/local/registered claim, .3 all four member kinds, .4 used/transitive/embedded references, .5 bare payload and independent controls, .6 CLI and DMLS, .7 local discovery | **verified fixed** | The published named kind claim points to 12:11 in standalone and installed CLI checks. Embedded, local and registered selection tests verified its span and the valid claim control. The public loader tests exercised used and unused references with top-level descriptions, imported library warnings and the bare carrier, alongside the envelope, snapshot and artifact controls. Formatted and JSON CLI tests and DMLS protocol tests preserve those repaired ASCII positions. | R4-F1 covers descriptions inside declarations. R4-F2 covers encoding the position for an editor. |

## Unblocked Findings

### R4-F2 (high) — DMLS uses character counts as protocol columns for registration warnings

**Authority — defect and spec §3.** The spec requires registration to “Report the failed field and source location.” DMLS advertises the position encoding negotiated with each client. A warning after an accented letter or emoji can put the cursor on the wrong token because the editor projection sends a Unicode character count as a byte or UTF-16 offset. The defect class is converting a source column to a protocol position without accounting for its encoding.

Publish Claudine into a temporary home. Append `types: {label: 'string -> 🧪 café', review-entry: 'string(example(./options.yaml))'}` to `schemas/formal-sequence-file.yaml` and refresh that member's digest. Leave `options.yaml` unlisted. Start DMLS clients that request UTF-8 and UTF-16 and open a Markdown document that uses the installation. The source is line 22, column 66 in both CLI output forms. The language server gives both clients zero-based line 21, column 65. The correct positions are column 69 in UTF-8 and column 66 in UTF-16. The YAML file need not be open for this warning to appear. [Darkmatter character columns](../../lib/src/config/load.rs#L1260), [DMLS registration projection](../../dmls/src/diagnostics/frontmatter.rs#L200), [DMLS publication](../../dmls/src/router.rs#L1272).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R4-F2.1 | DMLS registration publications for an unopened member and an open member | Published YAML-kind example after ASCII prose, `café`, and `🧪 café` on the same line | ASCII was correct for both clients. With `café`, both got column 63. With `🧪 café`, both got column 65. Opening the file kept the same range | ASCII 63 for both. With `café`, UTF-8 64 and UTF-16 63. With `🧪 café`, UTF-8 69 and UTF-16 66 |
| R4-F2.2 | DMLS warnings sourced from a registration entry, manifest, member, or artifact, including conflicting manifests | Follow each located result into the projector exercised in .1 | All use one function that subtracts one from the same character count. Refresh, opening and closing reuse its stored range | Convert the originating location using the negotiated encoding on every path |
| R4-F2.3 | Darkmatter CLI formatted and data warnings, and embedded library result | Same published example and ASCII control | The CLI uses the intended one-based character column. The library location also uses one-based character coordinates | Preserve these source coordinates |
| R4-F2.4 | DMLS document host choice, registered YAML kind selection, and ordinary schema-document errors | Inspect sibling projections fed by the host and kind decisions | Ambiguous-host notices have no column. Host-only event warnings and YAML kind selection get positions from document spans through the source map. Ordinary open schema-document errors use the source map as well | Keep the document-level anchor or the encoded document span, as applicable |

Carry enough source information from the already-read configuration, manifest, or verified member to encode the reported position. Reuse the source-map conversion for editor coordinates and preserve the CLI's one-based character columns. Exercise both negotiated encodings on the same warning. The source is available during registration preparation, so the editor publication step can remain free of per-warning file reads.

### R4-F1 (medium) — Registration warnings can point into a property description

**Authority — spec §3 and the first acceptance criterion.** Registration must “Report the failed field and source location,” including refusals for unlisted imports. A schema author can describe where settings come from in one property's documentation and import that file in another. The warning can point at the description even though registration correctly refuses the host. The defect class is recovering a failed reference from the first substring match in a carrier, including text that the schema parser treats as prose.

In a temporary Claudine installation, append the following to the published formal sequence definition and refresh its digest. With `options.yaml` unlisted, the CLI warns at 23:50 in `review-setting`. It should warn at 24:27 in `review-entry`. Changing the second declaration to `string(example(./options.yaml))` should put the warning at 24:34. Both edits currently choose 23:50. Darkmatter's reference locator searches every string under the two schema sections and keeps the earliest match in the first section with a hit. [Registration locator](../../lib/src/config/registration/locate.rs#L180).

```yaml
types:
    review-setting: 'string -> Read options from ./options.yaml.'
    review-entry: Options@./options.yaml
```

Coordinates below are one-based except where DMLS protocol ranges are shown. Each failing installed row refused the host. The unedited published bundle loaded and supplied the inline trigger and formal sequence kind.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R4-F1.1 | Unused trigger types, `inline-composition.yaml` | Adjacent description and import or file-backed example | Both at 51:50 | Import 52:27, example 52:34 |
| R4-F1.2 | Unused YAML-kind types, `formal-sequence-file.yaml` | Same two edits | Both at 23:50 | Import 24:27, example 24:34 |
| R4-F1.3 | Unused namespace types, `loop.yaml` | Same two edits | Both at 14:50. The lifecycle check adds no stale-hash warning | Import 15:27, example 15:34. Preserve the single refusal |
| R4-F1.4 | Unused shared library types, `shared.yaml` | Same two edits | Both at 69:50 | Import 70:27, example 70:34 |
| R4-F1.5 | Used properties in a trigger arm, a YAML-kind payload, and a namespace payload | Same declarations in `$schema` | Trigger 44:52, YAML kind 12:50, namespace 8:50, all on description lines | Respect the failing property. Trigger import 45:29 and example 45:36. YAML kind 13:27 and 13:34. Namespace 9:27 and 9:34 |
| R4-F1.6 | Used shared-library property and dependent triggers | Introduce the same description next to the original `hash` property in `common-attributes`, then give `hash` the bad declaration | Own warning and both importer warnings at `shared.yaml` 30:50. The writing file and importer labels survive | Import 31:23 or example 31:30 for all three. Keep the importer labels |
| R4-F1.7 | Embedded profile preparation | Load the same installed copies as an in-memory bundle through Darkmatter's public embedded loader | The public result has the same misplaced positions for all four member kinds and correctly locates the top-level description control | Match the corrected installed coordinates under the virtual root |
| R4-F1.8 | Bare reference, top-level description and same-declaration controls | Bare trigger carrier, a top-level description naming the same file, and an example followed by prose in its own declaration | Bare carrier points to 43:10. Top-level prose is skipped. A declaration with its example ahead of its own prose locates the example | Preserve the carrier and reference positions |
| R4-F1.9 | Object composition, remaining typed file-reference failures, and downstream decisions | Give a published trigger arm the same leading description and write `$extends: Options@./options.yaml`. Inspect the common reference projection and adjacent controls | The object composition warning lands at 44:52. Bundle boundary, missing, unresolved, remote, sibling-only, invalid-example and referenced-trigger errors share the spelling search when they reach the locator. Manifest digest and path failures and other envelope and artifact errors use their own location rules. Kind and trigger selection only receive a usable profile | Locate the object composition operand at 45:25. Use the failing declaration for the other reference errors. Preserve separate rejection and source rules and withhold the failed profile |
| R4-F1.10 | Formatted CLI, JSON CLI and DMLS output | In the published formal sequence definition, place the prose before the unused import | Both CLI forms report 23:50. DMLS publishes zero-based 22:49 on the warning's file | CLI 24:27 and DMLS zero-based 23:26 for this ASCII case |

Prefer the authored node that produced the error. Where the resolver can supply only the path spelling, use a recognized import or example position and fall back to the nearest known declaration or carrier. A test with an earlier sibling's description plus the existing top-level and bare-carrier controls can prove the repair without requiring exact token offsets for every quoting form. Update the locator's comment and the topic page's “Where a refusal points” entry to match what the repair guarantees. [Registration diagnostic guide](../../docs/topics/host-registration.md#where-a-refusal-points).

## Blocked Findings

None.

## Recurrence

- **R4-F1** is a sibling of **R3-F1** from review 3, **R2-F2** from review 2, and **R1-F11** from review 1. Review 3's sweep followed references across members and importers with a top-level description fixture. It missed descriptions inside the same schema sections the locator searches. The recurring repairs have expanded where the code guesses a position from strings. The repair can stop at a known declaration when precise syntax provenance is unavailable.
- **R4-F2** is a sibling of **R1-F11.8** from review 1. That row verified that a located warning reaches the editor. Its ASCII cases never distinguished a character column from either protocol encoding. This review followed the range into both clients and through the open and unopened file routes. The original ASCII projection still passes.

## Input Robustness Matrix

The default for an invalid registration field or document is a located refusal with no host profile published and no host code run. Darkmatter can continue validating a separate document with its default schema.

| Loader or shared validator | Evidence and exceptions |
| --- | --- |
| Manifest envelope, member list and digests | The snapshot tests exercise omissions, explicit nulls, wrong containers and elements, empty collections, digest coverage, duplicate resolved members and mismatches. Explicit empty `files` and `functions` are valid. The published format `2` works. `2.0`, quoted `"2"`, null and `3` all refuse the installed host at `format_version`. |
| Shared YAML loader | The current reports contain duplicate-key, trailing-content and extra-document refusals and parser position controls. The unedited published YAML with comments loads. A bare YAML key for optional `types` exercises the format's null syntax. |
| Shared member envelope and type declarations | Installed, embedded, standalone and local checks cover absent, blank, explicit null, numeric, empty mapping and populated YAML-kind type tables. Omission or null means no named types for YAML kinds. Trigger and namespace type tables can be absent or empty, and a library requires a nonempty table. An absent or null trigger payload is valid for a matcher-only trigger. Blank or numeric `types` for triggers, namespaces and libraries are refused. |
| Match grammar | Focused trigger and loader tests cover grouped and flat conditions, invalid and mixed shapes, omissions, valid alternative controls and forbidden constraints. A bad condition contributes neither selection nor a payload. |
| Strict lifecycle and function facts JSON, and YAML function catalogs | The raw L1 reports show duplicate-key and trailing-content tests for JSON, required and malformed artifact properties, and rejected YAML catalog signatures. The lifecycle `effects` projection exercises absent, null, wrong container, mixed elements, wholly invalid elements and the valid empty list through the same reader. |
| Existing user configuration and setup | Setup tests cover a missing configuration and absent or empty register lists, obsolete and duplicate entries, malformed configuration, and conflicting permissions. The configuration reader keeps its null-as-no-entries rule. A valid permissions conflict preserves the existing blocks and still registers the successfully published manifest. |

I checked the relevant readers for defaulted fields, conflated absent and null values, silently filtered elements and swallowed parse errors. The findings above concern the positions produced after refusal.

## Verification

| Spec behavior | Appropriate level and evidence |
| --- | --- |
| Isolated bundle, boundary and stable revision | Level 1 temporary-home CLI, in-memory embedded loader, and snapshot tests for reordering, digests, duplicate members, template scope, stale artifacts and valid controls. The findings qualify location accuracy. |
| Host choice and YAML kind precedence | Level 1 installed Claudine commands, Darkmatter's registered kind cases and DMLS protocol tests cover shared event controls, literal kind matching, malformed host-specific values, competing hosts and registered/local/shipped precedence. |
| Checked trigger merge and inline alternatives | Level 1 host merge tests cover equivalent and narrowing types, presence and constraint conflicts, three-way overlaps and disjoint matches. The CLI, DMLS, and Claudine test cases cover inline alternatives, overrides above the host layer and reported conflicts. |
| Publication and reload | Level 1 publisher tests cover interrupted writes, competing publishers, prior manifest admission, convergence and unchanged timestamps. Snapshot tests exercise a reader spanning publication. DMLS protocol and controlled-clock tests cover batching, supersession, warning removal, and Markdown/YAML refresh. The implementation record includes a changed-publish timing sample for the settle window. |
| Inventory, setup and startup passivity | Level 1 shipped commands compare the inventory to the files written, rerun setup with migration and permission controls, and check zero ordinary-command reads with setup and publish as positive controls. |

Direct Nextest runs passed 61 Darkmatter repair cases, 92 Darkmatter acceptance cases, 26 SimplifiedSchema cases, 37 Claudine installation and embedded cases, and 38 publication and CLI cases. The slowest was 2.628 s. The initial focused `just test` invocation failed while forwarding the filter, so I ran the same test targets through Nextest. I also checked the repair's full Linux run reports for Darkmatter, Claudine and SimplifiedSchema, including the latter without default features. Those reports show zero tests at the five-second threshold. The three area tier checks found no stranded tests. All observations here use Level 1 library, CLI, or language-server entry points. The spec requires no real-terminal rendering or keyboard behavior.

The repair log's prose-location claim now describes the top-level case it verified and records the 2026-11-23 reproduction for the sibling. [Record correction](log.md#review-4-record-correction--2026-11-23).
