---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F11 (high): Bundle refusals lose the failed field's source location"
  - "R2-F1 (high): Unused type declarations can bypass bundle import admission"
  - "R2-F2 (high): Type resolution errors point to the wrong declaration"
  - "R2-F3 (high): Missing required member fields have no source position"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-23T11:11:15+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: "2026-11-22-host-registration-v2/review-2.md"
previous: "2026-11-22-host-registration-v2/review-1.md"
next: "2026-11-22-host-registration-v2/review-3.md"
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-host-registration-v2/log.md"
implemented-by: "claude/opus"
---

# Host registration v2 review, iteration 2

**Production readiness is false.** Twelve previous findings are verified fixed. The location repair is partial, and the review found three additional defects. Each has a repair within the specification.

## Previous Finding Verification

The instance ranges below refer to every child row in the [repair ledger](log.md#finding-ledger), including the additional rows from the repair. Verification combines the current code, focused Level 1 test runs, and direct CLI runs against a published bundle in a temporary home.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Resolved trigger templates can import unverified files | R1-F1.1–.6 | **verified fixed** | The deferred resolver captures and restores the bundle scope, and the slot cache uses its root and revision. `host_template_bundle_scope` covers listed bytes, unlisted and escaping references, republishing, embedded payloads, and the incomplete YAML and namespace controls. `bundle_resolution` and the passive-trace cases passed. | The new unused-declaration path is R2-F1. |
| R1-F2 — Malformed Markdown sequence sources pass Darkmatter validation | R1-F2.1–.16 | **verified fixed** | Compared the no-coercion marks in Claudine's bundle with the runtime readers. Installed and embedded validation, composition, DMLS, formal YAML, the text fields and task reference cases passed. The three split metadata tests took 0.789 s, 1.926 s, and 2.017 s in the follow-up Linux run. The ledger records the native Windows remeasurements. | None. |
| R1-F3 — Lifecycle artifacts accept undeclared namespace bindings | R1-F3.1–.9 | **verified fixed** | The registration and generator both check declared roots before using the artifact. `host_namespaces`, the Claudine generator checks, the CLI and DMLS agreement tests, and function-facts cases passed. Darkmatter's built-in bindings and the case with the catalog detached are covered. | None. |
| R1-F4 — An invalid previous manifest can authorize member deletion | R1-F4.1–.8 | **verified fixed** | The publisher now calls the reader's manifest admission before considering retirement, then checks host and publisher path ownership. Both the malformed-manifest matrix and the valid retirement control with mismatched old bytes passed, including preservation of the stray file. The setup, cleanup and listing routes use the same publisher or read independent inputs. | None. |
| R1-F5 — Registered root unions drop higher schema rules | R1-F5.1–.7 | **verified fixed** | The registered union is merged below each higher layer, with host arms first in the two-union cross product. Closed document and trigger controls, higher-arm redeclarations, normal object-layer behavior and phase projection passed in the library and through CLI, DMLS and Claudine consumers. | None. |
| R1-F6 — Trigger inspection reports success for conflicting matches | R1-F6.1–.7 | **verified fixed** | Inspected the common checked merge and the CLI outcome projection. Trace tests passed for a static conflict and compatible matches, with pending slots kept passive. The formatted and data command checks, validation, composition and the DMLS conflict fixture passed. Detect and about never prepare registered matches. | None. |
| R1-F7 — Registered root unions lose their editor schema | R1-F7.1–.10 | **verified fixed** | Read the arm projection through analysis, typed authoring, library completion and CLI assignment. The restored inline-prompt tests and the sections and ordinary object controls passed. The tests exercise hover, key and value completion, expression names and source navigation. The authored-only consumers retain their prior input. | None. |
| R1-F8 — The editor installs superseded first-load results | R1-F8.1–.6 | **verified fixed** | The controlled-clock cold, warm and out-of-order installation cases passed. The cold result keeps watches while withholding profiles and warnings, and Markdown and YAML see the pre-load view. Reviewed adjacent registry, YAML contract, slot and configuration reload handling for the cold-load exemption. | None. |
| R1-F9 — The format 1 upgrade warning points to an unavailable command | R1-F9.1–.13 | **verified fixed** | Ran the shipped `claudine setup` in a clean temporary home. Setup and format 1 fixtures passed for repeat runs, preserving existing settings, permissions conflicts, repository entries and publish-only behavior. The completion menu, callers, docs and caller-skill source name the working setup route. | None. |
| R1-F10 — Namespace and member validation differ across entry points | R1-F10.1–.13 | **verified fixed** | The build and registration share structural envelope admission, and the whole-YAML CLI and editor use the namespace payload checker. The envelope matrix, malformed and unedited namespace cases, target exclusion, trigger and YAML-kind controls, and inventory checks passed. Kept the existing optional null metadata decision recorded in the ledger. | New findings cover unused imports and diagnostic positions. The envelope acceptance cases themselves are repaired. |
| R1-F11 — Bundle refusals lose the failed field's source location | R1-F11.1–.18 | **partially fixed** | The manifest, shared YAML loader, typed artifact locations, duplicate labels, imported grammar error, formatted output and DMLS projection are covered by passing cases. The unknown-kind, unreadable, digest-mismatched and non-text controls preserve refusal and avoid a cascading lifecycle warning. A direct run with a match typo identifies the wrong line, despite the repaired label and static import cases. | R1-F11.3 still needs match condition positions. R2-F2 and R2-F3 enumerate newly found location siblings. |
| R1-F12 — Startup tests can miss reads of the installed bundle | R1-F12.1–.5 | **verified fixed** | With the CLI test feature enabled, the ordinary configured startup tests required an empty access log and unchanged files. Setup and publish produced observable reads. The fallback source inventory passed, and the only normal-command composition path inspected constructs the host from embedded bytes. | None. |
| R1-F13 — The lint timer can round a completed command to zero | R1-F13.1–.7 | **verified fixed** | Fake-clock, real-clock and no-Python tests passed through the lint step, including preserved failures. The package and WSL status-fold tests, measured-zero report cases, and unset-stage guard passed. The remaining companion readers interpret presence independently of the numeric value. | None. |

## Unblocked Findings

### R1-F11 (high) — Bundle refusals lose the failed field's source location

**Authority — spec §3 and the first acceptance criterion.** “Report the failed field and source location.” Invalid trigger matches are named explicitly. The remaining defect class is losing the match condition's path before turning a parsing or constraint error into a registration warning.

Publish and register Claudine into an empty temporary home with `claudine setup`. In the published `schemas/inline-composition.yaml`, change the first `literal(inline-compose; required)` to `literal(inline-compose; requird)` and update the manifest's BLAKE3 digest for that file. On a Markdown file with `prompt: hello`, `md schema validate --output json` omits the registered layer and emits a registration warning for line 44, column 5. The spelling that caused it is at line 25, column 23. The trigger parser returns a property-level error, and the registration locator tries `$schema` before `match`. The locator also searches only the immediate `match` mapping for forbidden constraints. [Registration loader](../../lib/src/config/registration.rs#L1360), [error locator](../../lib/src/config/registration/locate.rs#L79), [trigger location projection](../../../simplified-schema/lib/src/triggers/envelope.rs#L356).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F11.3 | Registered trigger envelope and match validator | Flat `prompt: string(requird)` and published `all` / `group` literal with the same typo | Flat error points to the payload at 25:5, grouped case at 44:5 | Point to the respective match value at 23:13 or 25:23 |
| R1-F11.3a | Forbidden match constraints | Same file, put `prompt(required)@./document.yaml` in the flat or grouped condition | Flat case points to the value at 23:13. Grouped case points to the entire match block at 23:5 rather than its condition on line 31 | Preserve the flat location and locate the grouped condition |
| R1-F11.3b | Match list and other envelope checks | `[7]`, mixed valid condition and number, `{}`, null, scalar, malformed label, and a valid match | Invalid sequence elements point to their index. Empty, null and scalar conditions point to the match value. Bad labels point to the label. The unedited trigger loads | Preserve those results. Missing `match` is covered by R2-F3 |
| R1-F11.3c | Direct YAML checks and ordinary trigger discovery | Same flat and grouped grammar typos and forbidden imported type | Direct YAML validation already pinpoints grammar typos at `/match/prompt` and `/match/all/0/group/0/kind`. Its semantic check on the grouped import reduces the location to `/match`. Ordinary discovery calls the same trigger parser for an error | Preserve exact grammar positions and use the nested condition path for forbidden matches wherever that parser supplies a diagnostic |
| R1-F11.8 | CLI and DMLS delivery | The same broken registration, plus the existing malformed imported-member control | CLI and DMLS both project the registration's coordinates. The imported control is correctly located. Neither projection can recover a position discarded at admission | Fix the location at parsing or admission and keep the existing projections |

Carry the match condition's structural path or span when it fails, including the indices in a nested condition list. Keep the error at the nearest match container if the parser has no more precise location. Add one registered flat case and one grouped case with the same property name also present in the payload.

### R2-F1 (high) — Unused type declarations can bypass bundle import admission

**Authority — spec §§1 and 3 and the first acceptance criterion.** “Missing or unlisted imports … refuse the host with source locations.” The same bundle boundary applies to indirect imports and deferred declarations when resolved. The defect class is admitting an entire member after checking only the type declarations reachable from its current payload.

Append an otherwise unused `optional-host-settings: Options@./options.yaml` to a copy of the published member's `types` table and update its digest. Leave `options.yaml` unlisted. In every member kind below, the modified bundle still supplies the inline trigger layer to `md schema validate --output json` with no registration warning. For the namespace, also update the generated lifecycle catalog hash and its manifest digest. The same digest calculation reproduced the unmodified artifact's original hash first. The type library path parses the declarations without resolving all of them. The other member paths lower the currently exported payload. SimplifiedSchema already has a named-type check that visits declarations a standalone export does not use. [Member preparation](../../lib/src/config/registration.rs#L981), [types-only check](../../lib/src/config/registration.rs#L1200), [standalone named-type check](../../../simplified-schema/lib/src/resolve.rs#L875).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Installed types-only library and its importers | Append unused import to published `shared.yaml`. Repeat with an `https` reference | Both edits load the host. Direct whole-YAML library validation refuses both. When an existing shared declaration actually uses the unlisted path, both registered importers refuse the host | Refuse the unused and used declarations at the authored import |
| R2-F1.2 | Installed trigger with private types | Append the same unused import to `inline-composition.yaml`. Control puts it in the first arm's used prompt type | Unused import is accepted. Used import refuses at the reference | Refuse both |
| R2-F1.3 | Registered YAML kind | Append the same unused import to `formal-sequence-file.yaml`. Control puts it in the kind payload's sequence declaration | Definition is admitted when the import is unused. The used import causes the registration to be refused | Refuse before publishing any registered YAML kind from that bundle |
| R2-F1.4 | Namespace catalog and lifecycle preparation | Append the same unused import to `loop.yaml` with current catalog hash. Control puts it in the namespace payload | Unused import is accepted with the current artifact. Used import refuses at line 8, without a misleading lifecycle hash warning | Refuse before using the catalog or lifecycle artifact |
| R2-F1.5 | Embedded registration and Claudine generation | Same four member branches reached through `load_embedded`. Prepared trigger shortcut can skip resolution | The embedded source provider constrains reads that take place, and `prepare_members` has no all-types check on either route. The build check is structural | Run the same admission before accepting a generated or embedded profile. Leave build admission structural |
| R2-F1.6 | Effects and consumers | The documented static and pending controls and the separate function/lifecycle artifacts | Listed imports resolve, active missing imports fail, and pending-slot resolution has verified scope. Function facts and lifecycle bindings use dedicated artifact readers. CLI, DMLS and Claudine consume already admitted profiles | Keep those controls. Refuse the entire bundle when a member's static import fails admission |
| R2-F1.7 | File-backed examples inside the same declarations | Add `review-note: string(example(./options.yaml))` to an unused library or trigger type. Controls put it in a used declaration | Both unused examples leave the host active. Both used examples refuse the bundle | Check example references as well as named-type imports under the same boundary |

Run a shared static declaration check under the verified byte scope, including standalone libraries and private `types`. Reuse the existing named-type traversal where possible. Keep deferred evaluation and I/O grants on their existing path, and preserve the source location of the failing import. Cover the four copied inventory members and the embedded route through public results.

### R2-F2 (high) — Type resolution errors point to the wrong declaration

**Authority — spec §§3–4 and the first acceptance criterion.** “Report the failed field and source location.” Namespace payload resolution also keeps its defining file as its diagnostic origin. This is a sibling of R1-F11. The defect class is dropping the origin or structural path of a reference before projecting the error into an authored declaration.

Using the same publish and digest procedure, replace the first arm's type in `inline-composition.yaml` with `Missing(required)`. The warning names `[0].prompt` with no position. In a separate edit, replace `required` with `requird` on the same declaration, and it points to the preceding `$extends` line. The locator stops at the first union-arm segment, and its fallback can search an unrelated section with the same property name. A boundary refusal from an imported type can also lose the importer declaring the reference. [Registration error locator](../../lib/src/config/registration/locate.rs#L37), [whole-YAML semantic projection](../../../simplified-schema/lib/src/yaml_documents/semantic.rs#L206).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Registered trigger payload | Unknown named type or constraint typo in the first published root-union arm, line 45 | Unknown type has no coordinates. Grammar typo reports 44:5. Object trigger control with a misspelled `$extends` type reports the declaration at 94:15 | Report the first arm's actual declaration and keep the object control |
| R2-F2.2 | Namespace payload | `Missing` in the second arm of a property union. Also a payload `count: Missing` beside a valid local type named `count` | Second arm points to the first arm at 9:9. Repeated name points to the valid local type at 15:12. Plain and nested object controls point to the actual value | Keep the correct carrier and arm index |
| R2-F2.3 | Library resolution by registered importers | Change the second `shared.yaml` agent arm to `Missing[]`. Separately replace the existing `hash` type with `Options@./options.yaml` | Unknown union type points to the first arm at 23:13. The unlisted transitive import is reported without coordinates on both importing triggers, although the offending reference is in `shared.yaml` at line 30. A file-backed example in the same shared type also loses the source, while the direct trigger example points at its reference. Grammar typo control in the library points to the exact library declaration | Locate the bad arm or unlisted reference in the library. Preserve importer context |
| R2-F2.4 | Registered YAML kind definition | Replace the formal sequence declaration's type with `Missing[](required)` or append a property with `string(requird)` inside a nested mapping | Registration points to the correct value in both cases. The malformed imported-library regression case also passed | Preserve the precise positions while reusing the locator |
| R2-F2.5 | Direct whole-YAML CLI and editor semantic checks | Same unresolved namespace type, YAML kind type and library union arm | Direct CLI locates the carrier's first node at `/$schema` or `/types`. In a multi-line payload the located line can be a valid sibling. The editor uses the same semantic result | Use the resolver's declaration path when available. Standalone trigger checks that only check grammar should locate a grammar failure inside its arm |
| R2-F2.6 | Artifact readers, manifest admission and editor warning projection | Same registration load with stale artifacts, malformed member bytes, and the known located library grammar control | These paths have independent field/span metadata, and the existing CLI and DMLS tests preserve it. A simple unlisted import directly in the registered trigger correctly points to the reference | Preserve these results |

Keep the resolver's source and carrier with the reference path and follow sequence indices when locating authored YAML nodes. For boundary errors, preserve the member that made the failing reference before the error reaches registration. Reuse parser spans for grammar failures and fall back to the corresponding carrier only if no narrower source is available.

### R2-F3 (high) — Missing required member fields have no source position

**Authority — spec §3 and the plan's input robustness matrix.** The manifest reader must “report the failed field and source location,” and the matrix requires a located refusal for invalid member envelopes. This is a sibling of R1-F11. The defect class is trying to locate a value that was omitted and discarding the containing mapping's position.

Delete `match` from the published inline-composition trigger and refresh its digest. The user gets a warning naming the file and missing field, with null line and column. The same trigger checked directly as YAML locates the containing mapping at 19:1. The registration envelope locator only tries the rejected field itself. [Member kind admission](../../lib/src/config/registration.rs#L1170), [structural validator](../../../simplified-schema/lib/src/member_envelope.rs#L123).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F3.1 | Registered trigger and match validator | Omit `match` from the published file | Refused without coordinates. Present null, numeric, empty and invalid sequence elements each have a position | Point to the trigger mapping for absent `match` |
| R2-F3.2 | Registered YAML kind | Omit `$schema` from `formal-sequence-file.yaml` | Refused without coordinates. The direct check locates its mapping at 9:1 | Locate the mapping that requires `$schema` |
| R2-F3.3 | Namespace member | Omit `namespace` or `$schema` from `loop.yaml` | Both refused without coordinates. The direct route locates the mapping at 4:1. An empty mapping payload is already a valid control | Locate the containing mapping and keep the empty control |
| R2-F3.4 | Types-only library and importers | Remove `types` from published `shared.yaml` | Library's structural warning has no coordinates. Both dependent triggers already point to its mapping at 19:1 | Locate the library's own refusal at the same mapping |
| R2-F3.5 | Missing member `kind`, manifest fields and build check | Remove member `kind`, or omit a required manifest field | Missing member `kind` has the same structural fallback gap. Manifest absence already points to the holding mapping. The build names the refused member in its inventory error | Locate the member mapping for absent `kind`. Preserve the manifest and build controls |

Have the structural refusal distinguish a missing key from one that exists with an invalid value. Use the containing mapping for omissions; use the offending token for a present invalid value. The Level 1 assertion should check the public warning's file and position, as well as refusal of the entire host.

## Blocked Findings

None.

## Recurrence

- **Review 1, R1-F11 — Bundle refusals lose the failed field's source location.** Its match-condition instance remains open. The previous repair verified a flat match with a fixture that had no payload. It did not cover published nested match lists or a name shared with the payload. R2-F2 and R2-F3 are additional siblings. The preceding sweep missed resolver failures inside union arms and across files, and required envelope fields with no value span.
- **Review 1, R1-F1 — Resolved trigger templates can import unverified files.** R2-F1 is a sibling of the verified-bundle admission work. The prior sweep checked initial preparation of a used type and preparation after a slot resolved. It did not check static types that no current payload uses. The earlier deferred-resolution repair remains effective.

## Verification and input robustness

| Spec behavior | Strongest evidence checked |
| --- | --- |
| Verified snapshots, import and namespace admission, artifact freshness, order independence and source locations | Level 1 public loader and published CLI tests. Rehashed copied production files for the failing cases. See the findings for the unused imports and missing or misplaced locations |
| Markdown host choice, shared events, malformed host-specific values, ambiguity, and registered YAML precedence and collisions | Level 1 installed and embedded library cases, shipped CLI and DMLS protocol cases. The unmodified published Markdown control included the inline trigger layer |
| Merge equivalence, narrowing, pairwise overlap, trigger inspection, inline alternatives and repository overrides | Level 1 checked merge, composition, CLI and DMLS tests. The prompt, sections, both and neither cases exercise the inline schema |
| Publication interruption, concurrent reader and writer, retry, no-op timestamps and editor reload | Level 1 publisher fixtures and verified reader tests, plus controlled-clock editor tests for cold and warm in-flight updates, quiet windows and repairs |
| Inventory list/detail/hash, repeatable setup, and passivity of ordinary commands | Level 1 shipped commands using temporary homes, including positive and negative file-access controls |

The focused nextest runs passed 702 cases and 114 follow-up cases. None reached five seconds. The combined build hit a disk quota, so I reused already-linked test binaries through nextest metadata for the focused runs. The review used the repair's recorded full-suite and cross-platform evidence for broader coverage. This feature's contracts concern files, command results, and language-server messages, which Level 1 can observe directly.

The default is a located refusal with no activation of the invalid bundle and no effects. The manifest and shared member loaders refuse duplicate keys, trailing content and a second document. Ordinary comments and quoted strings are accepted in the published controls. The shared JSON reader rejects duplicate fields and trailing values, while the recorded lifecycle and facts controls accept ordinary whitespace. An absent or empty user registration list can receive the new entry, and explicit null retains the documented empty-list behavior. Empty namespace payloads and private type tables are valid. An omitted or null trigger payload is matcher-only, while an explicit null matcher is invalid. The earlier optional null metadata decision for ordinary schema documents remains in force. The focused runs included the shared envelope, manifest, lifecycle, JSON facts and YAML-kind cases. The direct matcher matrix covered omission, bare null, wrong container, a mixed list, an all-invalid list and explicit empty. The findings record the remaining acceptance and position discrepancies.

No author ruling is required. The repair log now records which coordinate claims were incomplete and how the published-bundle probes demonstrated that.
