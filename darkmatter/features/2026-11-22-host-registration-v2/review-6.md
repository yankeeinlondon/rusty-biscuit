---
$schema: feature-review.yaml
ready: false
findings:
    - "R6-F1 (high): Trigger narrowing moves rules between union arms"
    - "R6-F2 (high): Refused schema targets lose the declaration that names them"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-23T18:27:47+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: 2026-11-22-host-registration-v2/review-6.md
previous: 2026-11-22-host-registration-v2/review-5.md
next: 2026-11-22-host-registration-v2/review-7.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/log.md
implemented-by: claude/opus
---

# Host registration v2 review, iteration 6

**Production readiness is false.** Two high-priority findings remain. Both can be repaired under the existing spec and the author's ruling.

## Previous Finding Verification

The required instances come from the [iteration 5 finding ledger](log.md#implementation-of-review-findings-5). Reviews 2–5 closed the carried findings from earlier iterations. Review 5, item 1, is the only human ruling in this series.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R5-F1 — Example refusals point to successful imports or the importing trigger | .1 trigger object and root union, .2 YAML kind, .3 namespace, .4 shared library and both importers, .5 CLI and DMLS, .6 independent reference controls | **verified fixed** | The installed and embedded loader tests passed. On a fresh Claudine publication, the trigger, YAML kind and namespace cases pointed at their edited declarations at 46:21, 22:19 and 13:19 respectively. Both shared-type refusals pointed at `shared.yaml` 31:23 and named their respective trigger importers. The formatted and JSON CLI checks, DMLS stdio cases and sole, unused, unlisted, remote, escaping and bare-payload controls passed. The ruling permits the declaration's value as the reported position. | The example repairs pass. R6-F2 records a sibling involving a different kind of refused target. |
| RULING-1 — Carry source and declaration through reference resolution | .a seven error variants and first-reader provenance, .b registration location and removal of text search, .c earlier regression cases, .d docs and departure record | **partially fixed** | The seven variants have a site field. The new example tests and the earlier tests for unlisted imports, shared declarations and references alongside descriptions pass through the structural locator. The filename search is gone, and the guide and implementation departure record describe the structural approach. The `TriggerSchemaReferenced` constructor always sets the site to `None`. Reclassifying a target refusal as a trigger-payload cycle also loses the source. | R6-F2 gives the remaining RULING-1.a cases under review-5.md, item 1. |
| R5-F2 — Registration diagnostics omit declared display names | .1 trigger envelope, matching and payload, .2 libraries and importers, .3 conflict consumers, .4 embedded/CLI/DMLS, .5 discovery/trace/inventory and duplicate controls, .6 envelopes without labels and unverified input, .7 build inventory | **verified fixed** | The label and control tests passed in both loaders, the CLI, DMLS and inventory. A published trigger with a match typo showed `inline-compose-schema (schemas/inline-composition.yaml)` beside the entry. A known type conflict showed both names in validation, composition, inspection and DMLS while retaining both target paths. The compiled Claudine build script, run on temporary copies with an extra key, identified both `inline-compose-schema (inline-composition.yaml)` and `sequence-schema (sequence.yaml)`. | None for the required instances. |
| R5-F3 — Same-host conflict warnings misstate YAML selection | .1 warning and selection behavior, .2 guide | **verified fixed** | The same-host regression test checked both definitions and the two warnings. The common warning forwarded by CLI and DMLS scopes trigger exclusion to Markdown and describes YAML availability and collisions. The guide explains the split with a second kind. | None. |
| R5-F4 — The source position helper overstates its column convention | .1 helper and other column descriptions | **verified fixed** | Darkmatter's `line_column` documents its character-column return and names the Markdown validation byte-column exception. The surrounding authored-position comments and the data contract describe their own records. | None. |

## Unblocked Findings

### R6-F1 (high) — Trigger narrowing moves rules between union arms

**Authority — spec §6.** “Use its full declaration when all other rules and metadata agree.” The checked merge compares the set of descriptions and the set of constraints across all the arms of a property's union. The resulting sets can agree even when the rules moved to a different arm. This is a loss of association between metadata and the type it governs. [Darkmatter's checked comparison](../../lib/src/config/registration/merge.rs#L274).

Publish Claudine into a temporary home. Give the published `composition.yaml` trigger a match of `prompt: string(required)` and the following payload property:

```yaml
$schema:
    review-model:
        - "string -> single model"
        - "string[] -> model choices"
```

In both payload arms of the published inline-composition trigger, declare the same property with `enum(claude, codex) -> model choices` and `string[] -> single model`. Refresh both digests. On a Markdown document with `prompt: Say hello`, both triggers match. Validation, composition and trigger inspection exit 0, and DMLS publishes no conflict. The same inputs also succeed with `no-coerce` moved from the scalar string to the array's elements. With that change, the array uses different conversion rules even though the array's underlying type is unchanged.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R6-F1.1 | Strict narrowing between two object payloads | Published triggers with the property above and inline-composition's payload replaced by an object holding the narrowed property | Validation and inspection succeed and the trace shows both matches | Refuse the conflict on `review-model` |
| R6-F1.2 | Strict narrowing against a key shared by all arms of a root union | Same descriptions swapped, then the same mutation with `no-coerce` moved, keeping inline-composition's root union | Both variants produce a checked layer | Refuse both overlaps. Each arm must retain its own rules |
| R6-F1.3 | Consumers of the checked layer | Run `md schema validate`, `md compose`, and `md schema triggers` from a temporary repository. Open the document through DMLS stdio | All three commands succeed and DMLS publishes no conflict for either edit in .2. An incomparable-type control gives the coded error, with both trigger names and paths, on every route | Return `dm.host.trigger_conflict` through the same paths and suppress dependent schema features |
| R6-F1.4 | Static and deferred assembly | The static cases above use the registration's matched-set cache. Source inspection shows the deferred branch passes the resolved payloads to the same `merge::check` and bypasses that cache | The deferred branch has no later arm-by-arm validation to recover the constraint assignment. Passive inspection correctly leaves unresolved templates pending | Apply the same rule comparison after resolving a template and keep passive inspection behavior |
| R6-F1.5 | Equivalent, scalar and incompatible controls | Keep descriptions on their original arms while narrowing. Check equivalent declarations, scalar narrowing and disagreement, nested equivalent shapes, competing root alternatives, and unmatched triggers | The unmodified association narrows successfully. Existing tests pass for identical declarations, changed equivalent-type metadata, changed nested constraints, competing root alternatives and unrelated matches. Raw declarations use equality of the entire JSON schema and requiredness | Preserve these decisions |

Keep the type comparison and compare each comparable union arm's description, default and constraints at that arm. Check element rules and collection rules at their own level. A focused regression can reuse the current narrowing fixture with scalar and array alternatives and the existing conflict fixture for public output. [Registration layer assembly](../../lib/src/markdown/schemas/mod.rs#L1574), [checked merge tests](../../lib/tests/l1/host_trigger_merge.rs).

### R6-F2 (high) — Refused schema targets lose the declaration that names them

**Authority — review-5.md, item 1, and spec §3.** The ruling requires the writing source and declaration for `TriggerSchemaReferenced` and a structural location for file-reference refusals. The spec requires registration to “Report the failed field and source location.” Target selection knows which declaration asked to open the file. When the file exists but cannot serve as a schema or library, the error retains the target path or is rewritten as a trigger-payload cycle. The defect class is dropping the reference's source when classifying a target refusal. [Author's ruling](review-5.md#item-1), [SimplifiedSchema reference resolution](../../../simplified-schema/lib/src/resolve.rs#L1351), [trigger refusal conversion](../../../simplified-schema/lib/src/triggers/assemble.rs#L1159).

Publish Claudine and replace the inline-composition trigger's union with `$schema: ./composition.yaml`. Refresh that digest and validate a Markdown prompt. The installed bundle is refused and validation falls back to Darkmatter. The CLI's warning names `inline-composition.yaml` with null line and column even though the reference is at 43:10. DMLS highlights a zero-length range at the start of the trigger file. Use the same edit with a second union arm to see that its position is also lost.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R6-F2.1 | Registered trigger's root reference and file arm | Reference the published trigger itself or `./composition.yaml`, directly or as a second root-union arm | Both targets and forms refuse with a trigger-payload-cycle message and no line or column | Anchor at the scalar or arm that names the trigger and retain the refusal reason |
| R6-F2.2 | Other ineligible whole-schema targets | In the same scalar and arm, name `./shared.yaml`, `./formal-sequence-file.yaml` or `./loop.yaml` | Each refusal points at the target file alone. All three targets are valid at their intended roles | Anchor at the declaration asking to use the target as a document schema. Keep the target and its role in the message |
| R6-F2.3 | Named-type selection in registered payloads and private declarations | Point the formal sequence's `step[]` import, the loop namespace's `count`, or the shared library's `last_updated` at `./inline-composition.yaml` | The kind, namespace and direct library check name the target trigger file with no line or column. The two triggers using the shared library turn that same failure into a cycle warning, also without a position | Point at the import in the file that wrote it, including the shared library for both importers. Keep the importing trigger in each warning |
| R6-F2.4 | Markdown and YAML document `$schema` refusals | Reference a local copy of the shipped trigger from a Markdown file and from a whole YAML file | Both CLI calls reject with the trigger-specific message. The public `TriggerSchemaReferenced` constructor accepts only the target and unconditionally sets `site: None`. Both document consumers use it | Carry the known writing declaration on the error as required by the ruling. Retain the rejection and suggestion |
| R6-F2.5 | Installed and embedded registration and public warning output | Use the cross-trigger case in .1. Check formatted and JSON output and DMLS, then load the same verified bytes through Darkmatter's public embedded API | Formatted output omits coordinates, JSON reports null coordinates, and DMLS publishes 0:0. The embedded loader gives the equivalent virtual trigger path with no line or column | Report the same declaration in each loader and project it in the relevant coordinate convention |
| R6-F2.6 | Neighboring admission controls | Keep the original `$extends` library imports and validate the publication. Use an unlisted file, a remote reference and an escaping reference. Try a scalar `$schema` on a registered YAML-kind definition | Original imports work. The focused tests locate the unlisted, remote and escaping references. A YAML-kind scalar is refused by its envelope at the authored field, ahead of target resolution. A namespace also requires a mapping | Preserve those results, any actual target parse location, and all-or-nothing registration |

Pass the already known site through the two successful-selection boundaries, where loading a schema target or loading a target's named types can fail. Preserve that site when converting the refusal into a trigger-specific error. A valid file used in the wrong role needs the caller's declaration. A file that fails to parse can keep its own parser position. Use a file-level warning where there truly is no site. This follows the author's existing ruling and requires no filename search. [Named-type loader](../../../simplified-schema/lib/src/resolve.rs#L4276), [document trigger guard](../../../simplified-schema/lib/src/triggers/assemble.rs#L1211), [registration location projection](../../lib/src/config/registration/locate.rs#L62).

## Blocked Findings

None.

## Recurrence

R6-F2 is a sibling of the source-location class in R1-F11, R2-F2, R3-F1, R4-F1 and R5-F1. In particular, the review 5 repair covered file selection failures and example refusals, but should also have followed the writing site after a file was selected and when that target failure was converted to a trigger-specific error. The ineligible-target cases in the table expose the same gap through direct, union, named-type and shared-library paths. Review-5.md, item 1 expressly names `TriggerSchemaReferenced` and specifies where to retain its source. The severity follows that ruling and the spec's location requirement. R6-F1 concerns which declarations the common checked merge accepts. Review 1's inspection finding concerned a consumer skipping the check entirely, and its root-union finding concerned precedence after the layer was built.

## Input Robustness Matrix

Invalid registration input produces a refusal with its source, publishes no host profile and runs no host code. Independent validation may continue under Darkmatter's default rules. R6-F2 qualifies the source when a listed target has the wrong role. The iteration 5 changes add no new on-disk format.

| Loader or validator | Level 1 evidence and exceptions |
| --- | --- |
| Manifest, paths and digests | The snapshot cases check missing and null fields, wrong containers and members, duplicate paths, stray and missing digests, malformed versions and mismatched bytes. Empty `files` and `functions` lists are admitted when the rest of the manifest is complete. |
| Shared YAML and member envelopes | Loader cases cover duplicate keys, trailing malformed content, second documents, comments and BOM. Closed envelope checks reject unexpected keys and bad required-value shapes. The existing nullable YAML-kind `types` convention and matcher-only trigger payload convention remain. A valid display name can identify a member whose other fields fail. |
| Lifecycle and function artifacts | Strict JSON loading rejects duplicate keys, trailing content and invalid required fields. The lifecycle `effects` validator rejects absent, null and wrong containers and elements. It accepts explicit empty effects. Function signatures are checked against the facts catalog. |
| Configuration and setup | Tests distinguish missing configuration, absent and empty registration lists, explicit null under the existing config convention, malformed configuration and duplicate entries. A successful publish followed by a permissions conflict preserves the old permissions and still registers the manifest. |
| Resolver target and reference checks | Unlisted, remote and escaping references refuse before reaching a target. Existing and temporary publication controls exercised successful relative imports, a failed example alongside a working import, the wrong-role cases in R6-F2, and refusal without a host layer. |

I checked the readers for defaults on required fields, filtering that could discard invalid collection members, and parse errors dropped before admission. The review's CLI cases establish that the same decisions reach the normal commands.

## Verification

| Spec behavior | Appropriate level and evidence |
| --- | --- |
| Isolated bundle, revision identity, all-or-nothing admission, artifacts and diagnostic locations | Level 1 installed and embedded loader cases, plus real Claudine publications outside the checkout. Earlier revision-order, boundary, stale-artifact and namespace controls apply. R6-F2 identifies the remaining diagnostic gap. |
| Authored host selection, YAML kind precedence, trigger layering and inline alternatives | Level 1 public library, CLI and DMLS cases exercise shared-event and literal controls, malformed values, ambiguity and collisions, applicable-only conflicts, precedence and the exact-one inline contract. The new narrowing probes expose R6-F1. |
| Publication, retry and unchanged timestamps | Level 1 publisher and reader cases exercise interruption, contention, repaired and unchanged publishes, and the verified snapshot. The Claudine CLI inventory case compares installed bytes and unchanged timestamps. |
| Editor updates | Level 1 controlled-clock server tests and explicit load completion ordering cover bursts, stale completion, moved and missing files, old view retention and warning removal. Diagnostic probes used DMLS stdio. |
| Inventory, setup and ordinary commands | Level 1 registration command and setup cases check payload bytes and names, detail and failure behavior, locked config migration and preserved permissions. The feature-enabled CLI run included the ordinary-command read guards. An actual build script run exercised both named inventory refusals. |

The two focused Linux Nextest runs passed 272 and 13 tests respectively. The slowest test took 1.169 seconds. The recorded Darkmatter and SimplifiedSchema L1 summaries also show zero tests at or above five seconds. This iteration used focused reruns and the implementation's broader historical runs for unchanged cases. These behaviors are exposed through library results, subprocess output or the language-server protocol. They do not depend on terminal rendering or operating-system input.

The iteration 5 implementation records now distinguish the populated reference sites from the cases that still lose one. The Phase 3 description also records the difference between comparing the complete declaration and pooling its union-arm metadata. [Repair record](log.md#review-6-record-correction--2026-11-23), [implementation record](implementation-log.md#review-6-record-correction--2026-11-23).
