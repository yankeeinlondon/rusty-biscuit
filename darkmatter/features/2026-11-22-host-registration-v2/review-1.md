---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Resolved trigger templates can import unverified files"
  - "R1-F2 (high): Malformed Markdown sequence sources pass Darkmatter validation"
  - "R1-F3 (high): Lifecycle artifacts accept undeclared namespace bindings"
  - "R1-F4 (high): An invalid previous manifest can authorize member deletion"
  - "R1-F5 (high): Registered root unions drop higher schema rules"
  - "R1-F6 (high): Trigger inspection reports success for conflicting matches"
  - "R1-F7 (high): Registered root unions lose their editor schema"
  - "R1-F8 (high): The editor installs superseded first-load results"
  - "R1-F9 (high): The format 1 upgrade warning points to an unavailable command"
  - "R1-F10 (high): Namespace and member validation differ across entry points"
  - "R1-F11 (high): Bundle refusals lose the failed field's source location"
  - "R1-F12 (high): Startup tests can miss reads of the installed bundle"
  - "R1-F13 (medium): The lint timer can round a completed command to zero"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: false
created: "2026-11-23T06:43:40+00:00"
spec: 2026-11-22-host-registration-v2/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-host-registration-v2/spec.md`"
feature: "2026-11-22-host-registration-v2/review-1.md"
---

# Host registration v2 review

The feature needs another repair cycle before it is production ready. There are 13 findings and each can be repaired within the existing decisions. This is the first implementation review.

## Findings

### R1-F1 (high) — Resolved trigger templates can import unverified files

**Authority — spec §3.** “Validate the resulting member references against the same bundle boundary.” The defect class is loss of the verified bundle scope when a deferred schema becomes concrete.

I published Claudine's bundle into a temporary home, registered it, and changed the published inline composition trigger's prompt type to a slot that returns a named type reference. I updated the trigger's digest in the manifest. With the same manifest, changing the unlisted target's declaration from a required string to a required number changed `md schema validate` from success to a type error for the same document. The target therefore participates in the schema without participating in the registration's revision. Darkmatter's initial trigger loader uses the verified byte table. Its later template path calls ordinary resolution. [Initial preparation](../../lib/src/config/registration.rs#L1009), [deferred preparation](../../lib/src/markdown/schemas/mod.rs#L1558), [slot resolution](../../lib/src/markdown/schemas/slots/session.rs#L574).

| Instance | Site | Input or invariant checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Installed trigger, initial preparation | Existing static named type changed to an unlisted member | Existing public loader test refuses and tracks the path | Refuse |
| R1-F1.2 | Installed trigger, explicit slot resolution in validation and composition | Slot returns a type from an unlisted sibling or a path outside the bundle | CLI validation uses disk bytes in both cases. Composition calls the same effective-schema builder | Refuse either reference with the slot or schema location |
| R1-F1.3 | Installed YAML kinds and namespace schemas | A template appears where a complete definition is required | These prepare before the bundle scope closes and refuse unresolved definitions during registration | Refuse the incomplete definition |
| R1-F1.4 | Embedded registered triggers | Pending payload with embedded origin | The concrete resolver requires a file origin before entering the slot session, so this route cannot fall back to disk | Preserve the embedded boundary |
| R1-F1.5 | Editor preparation and pinned editor requests | Pending registered trigger is resolved for analysis | Editor preparation uses the same slot resolver. A pinned result carries the outcome of that resolution | Pin only a schema prepared from verified members |
| R1-F1.6 | Passive trigger inspection and slot discovery | Template has not been explicitly resolved | Trace reports pending status. Passive discovery enumerates slots without evaluating them | Remain passive |

Keep evaluation of slot data under the existing grant checks. Reenter the registration's verified byte resolver for the subsequent schema preparation, and include the verified revision in any reusable result whose imports depend on that snapshot. Exercise one successful listed import and refused unlisted and escaping imports through the public result.

### R1-F2 (high) — Malformed Markdown sequence sources pass Darkmatter validation

**Authority — spec §5 and acceptance criteria.** “This keeps malformed values such as `sequence: 42` in Claudine's validation.” The defect class is coercion of a malformed authored source into an apparently valid reference after correctly selecting its host.

The published sequence trigger matches native number and boolean values so users get Claudine's diagnostics. Its top-level list-or-string type allows Darkmatter's scalar-to-string conversion to accept both values as potential references. A typo here looks valid to a user in the editor until Claudine tries to run it. [Sequence trigger](../../../claudine/registration/sequence-document.yaml), [shared sequence types](../../../claudine/registration/sequence.yaml#L23).

| Instance | Site | Input tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Markdown validation | `sequence: 42`, `sequence: true` | Exit 0, both Claudine triggers applied | Type error at `sequence` |
| R1-F2.2 | Markdown composition | `sequence: 42` | Successful compose with Claudine selected | Same type error before writing composed output |
| R1-F2.3 | DMLS Markdown diagnostics | `sequence: 42` | Empty diagnostics | Same type error |
| R1-F2.4 | Claudine sequence launch | `sequence: 42` | Runtime parser refuses the numeric input | Refuse |
| R1-F2.5 | Registered whole-YAML kind | `kind: sequence` with `sequence: 42` | Refused at the authored value | Refuse |
| R1-F2.6 | Markdown controls | `sequence: []`, a reference string, `sequence: {}`, and `sequence: null` with no other Claudine keys | List and reference accepted, object refused under Claudine, null alone selects only Darkmatter | Preserve those outcomes |

Make the shared Markdown sequence declaration check the authored type before it can be coerced to a reference. Preserve all supported string source forms and the list shorthand. Keep the scalar rejection test in the installed-schema path as well as the runtime path.

### R1-F3 (high) — Lifecycle artifacts accept undeclared namespace bindings

**Authority — spec §4.** “Refuse … event bindings that name an undeclared host namespace. Retain the existing rules for Darkmatter's own bindings.” The defect class is checking root syntax while skipping membership in the prepared namespace catalog.

In the real published Claudine artifact, replacing an event binding root with `typo_root` and refreshing only the artifact's manifest digest left the registration active with no warning. The event registry checks syntax, reserved names, and duplicate names. A syntactically valid typo passes those checks. [Artifact reader](../../lib/src/lifecycle/host_profile.rs#L270), [binding builder](../../lib/src/markdown/compose/expression/binding.rs#L367).

| Instance | Site | Input tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Core event binding view | Replace a root on `initialize` with `typo_root` | Claudine still selected | Refuse the registration at the binding |
| R1-F3.2 | Host event binding view | Same edit on `success` | Claudine still selected | Refuse at the binding |
| R1-F3.3 | Declared host globals | Replace a name with `typo_root`, with `global_catalog: true` | Registration refused | Refuse |
| R1-F3.4 | Declared host globals without attached catalog | Same one-field edit in an accepted control with `global_catalog: false` | Claudine still selected | Enforce namespace membership when admitting v2 |
| R1-F3.5 | Downstream host analysis for validation and DMLS | Register the modified artifact | Both derive a view from the same admitted profile, and no later check withdraws the root | Reject before either consumer receives the profile |
| R1-F3.6 | Darkmatter's built-in event bindings | Unedited generated artifact with `err` and `timing` | Registration loads normally | Preserve those built-in bindings |

Check the decoded declaration roots once at the registered artifact boundary using the supplied namespace catalog, with the existing exceptions for Darkmatter-owned names. Keep the explicit artifact flag's existing meaning for whether a catalog is attached. Existing shared JSON shape checks cover absent versus null event metadata, wrong whole-field and element types, empty collections where valid, and rejection of duplicate keys and trailing input. The failing cells here are syntactically valid strings with undeclared names.

### R1-F4 (high) — An invalid previous manifest can authorize member deletion

**Authority — plan's fixed-path publication rule.** “Remove retired entries taken from the previous valid v2 manifest.” Spec §8 treats other files as strays for doctor to report. The defect class is divergent manifest admission before deciding whether a file can be deleted.

I renamed the published Claudine config type library to `schemas/retired-config.yaml` and changed its two manifest path occurrences, preserving its digest. The normal reader accepted this control and the next publish retired the renamed member. For each malformed variant I recreated that control and made one edit. The normal reader refused every variant below, while Claudine's separate retirement reader authorized deletion for several. [Manifest admission](../../lib/src/config/registration/manifest.rs#L174), [retirement reader](../../../claudine/lib/src/composition/registration.rs#L659).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F4.1 | Darkmatter manifest envelope and path checks | The malformed versions, digest entries, duplicate key, and bad path spellings below | Profile refused | Refuse |
| R1-F4.2 | Claudine previous-manifest envelope | Omitted version versus empty, null, and numeric version | Omission kept the retired file. Empty, null, and numeric version allowed it to be deleted | Keep the file for every malformed version |
| R1-F4.3 | Claudine previous-manifest digest map | Null map or absent digest key versus null, numeric, and nonhex digest values | First two left the file. The bad values allowed deletion | Keep the file for every malformed digest map |
| R1-F4.4 | Claudine previous-manifest YAML loader | Duplicate digest key versus trailing garbage and a second document | Duplicate key allowed deletion. Trailing garbage and second document kept the file | Keep the file for every malformed document |
| R1-F4.5 | Claudine's path projection for retirement | One other member uses an escaping or unprefixed path | Invalid entry was filtered. The renamed library was deleted | Keep every candidate when the prior manifest is invalid |
| R1-F4.6 | Publication after valid retirement admission | Valid retirable name alongside an unrelated stray | Retires the named file and preserves the stray by construction | Continue to do so |

Reuse the format 2 structural and path admission for the retirement decision, separately from checking whether old member bytes still match their digests. This preserves repair after an interrupted publish without granting a malformed manifest authority over any deletion. Keep the valid control, a mixed valid/invalid member test, and one regression per shared validator. The same validators cover omitted, null and wrong-shaped fields, empty or ill-formed digest values, mixed or wholly invalid map entries, path aliases, document boundaries and duplicate keys. A nonempty valid manifest with no retired members should remain a no-op for retirement.

### R1-F5 (high) — Registered root unions drop higher schema rules

**Authority — spec §6.** “Ordinary schema-root triggers override it, and the document's own `$schema` overrides both.” The defect class is loss of higher-layer root semantics during union lowering.

With Claudine's shipped registration, I validated Markdown containing `prompt: hi`, `extra: hi`, and an authored `$schema` that declares `prompt` and sets `$constraints: { closed: true }`. The check returned valid. Removing the prompt value takes the document off the registered union path and the same schema then rejects `extra`. The union merger puts the higher object below the union after removing its redeclared keys from the arms. It also expressly rejects a document union above a host union. [Union merge](../../lib/src/markdown/schemas/mod.rs#L3281).

| Instance | Site | Shape checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F5.1 | Higher authored object schema | Closed authored schema and undeclared `extra` on a published inline prompt | CLI accepts `extra` | Refuse the extra property |
| R1-F5.2 | Higher schema-root trigger | Closed root on a matching ordinary trigger | CLI accepts the extra field with the union and refuses it with the same trigger in the nonunion control | Retain the trigger's root constraint |
| R1-F5.3 | Higher authored union schema | Two valid document schema arms above a host union | CLI refuses preparation with `SchemaError::Baseline` | Apply the authored schema over the host layer |
| R1-F5.4 | Ordinary object layering and property overrides | Remove the host union, or refine a shared property using a higher trigger | Closed control rejects the extra property, and the existing higher-property test passes | Preserve those outcomes |
| R1-F5.5 | Validation, composition, and DMLS projections | Assemble an effective schema for either higher rule | Share the union lowering. Runtime phase projection sees the already lowered result | Preserve the same root rules through each consumer |

Project applicable layers into each arm with precedence intact. Keep the existing one-union limit **among the matched host triggers**. Test the closed rule from both higher sources, a document union over a host union, and the built-in exactly-one prompt/sections check. Prefer the existing arm projection primitives over extra syntax.

### R1-F6 (high) — Trigger inspection reports success for conflicting matches

**Authority — spec §6 and the plan's checked-merge rule.** “Use the same checked merge for every consumer.” The plan explicitly includes the listing/trace path. The defect class is treating a successful condition match as a successful schema preparation.

In a temporary repository I edited the published `composition.yaml` to match a required string `prompt` and declare that property. I refreshed its digest and checked a normal inline prompt. The shipped inline trigger owns this alternative, so the additional declaration must conflict. [Registered trace](../../lib/src/markdown/schemas/mod.rs#L635), [CLI trace command](../../cli/src/commands/schema/triggers.rs#L59).

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F6.1 | `md schema triggers`, formatted and data projections | Two matches declaring `prompt` | JSON exit 0, both matches reported, no conflict. Formatted output uses the same trace | Show the matches and the conflict, with both paths |
| R1-F6.2 | `md schema validate` and `md compose` | Same document and bundle | Both exit 1 with `dm.host.trigger_conflict` | Refuse preparation |
| R1-F6.3 | DMLS validation and schema features | Two conflicting matched payloads in the editor fixture | Existing L1 conflict test expects the coded error and suppresses features that need the failed schema | Preserve refusal |
| R1-F6.4 | Template and slot discovery | Conflicting shapes whose inputs are still pending | Passive discovery lists inputs. The concrete builder compares the whole resolved set | Keep discovery passive and check once the declarations are concrete |

Use the matched indices from the shared selection and report the static check's result through the existing trace diagnostic output. A pending template can remain pending in a passive trace. Add a CLI control demonstrating that two compatible static matches still report success.

### R1-F7 (high) — Registered root unions lose their editor schema

**Authority — regression.** The existing Claudine editor fixtures opened an inline prompt such as `prompt: hi` and used the host schema for editor features. Six fixtures were changed to `agent: claude` to bypass the new root-union path. The defect class is dropping the schema shape at the editor projection boundary.

The same valid `agent: claude` value gets the hover text “the agent (provider) to run” in an agent-only Markdown file and yields no hover when I add a valid `prompt: hi` to select the shipped inline schema. Both documents validate. [Checked payload shape](../../lib/src/config/registration/merge.rs#L47), [trigger layer construction](../../lib/src/markdown/schemas/mod.rs#L1595).

| Instance | Site | Input or invariant checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F7.1 | DMLS schema hover | Open the installed prompt example, hover `agent` | No hover, while the agent-only control provides the provider type and description | Same declared shared field available |
| R1-F7.2 | DMLS key and value completion | Registered root union has `shape: None` when passed to the editor's schema view | Shared fields and branch fields have no declarations from this layer | Offer the shared fields and appropriate branch declarations |
| R1-F7.3 | DMLS declared expression names and schema-backed navigation | The editor views read the same missing declaration shape | No declarations or their origins from this layer | Retain declarations and provenance for those features |
| R1-F7.4 | Markdown validation and ordinary object editor shape | Same prompt validates, and agent-only control gets schema hover | Both clean | Preserve validation and ordinary object behavior |

Project the prepared union arms into an editor view, keeping the validator's arm relationships intact. Start by making shared declarations available with their original definitions and source locations. Restore prompt-based L1 editor coverage and include the sections branch.

### R1-F8 (high) — The editor installs superseded first-load results

**Authority — spec §7.** “A newer queued load supersedes results from an older in-flight load.” The defect class is applying the generation check only after the first view exists.

The controlled L1 test for a change during first preparation expressly expects the touched result to become visible before a second reload. A publish overlapping editor startup can therefore briefly expose an obsolete registration or refusal while the newer load waits for quiet. [First-load installation](../../dmls/src/overlay/hosts.rs#L480), [controlled first-load test](../../dmls/src/overlay/hosts.rs#L963).

| Instance | Site | Change ordering checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F8.1 | First load, no installed dependency set | Set a watched input change between start and installation | Controlled L1 test expects a view to be installed even though `install` returns that it is stale | Keep the new watch information and withhold the stale profile and warnings |
| R1-F8.2 | Warm load, known watched path | Report another change before the load completes | Generation mismatch discards the result and keeps the existing view | Preserve the warm view until the current load finishes |
| R1-F8.3 | Warm load, newly discovered input | Change a path only present in the candidate dependencies | Completion compares recorded changes with new dependencies and rearms without installing | Preserve this rejection |
| R1-F8.4 | Results complete out of sequence | Finish an older request after the newer view was installed | Lower sequence number is discarded | Preserve this rejection |
| R1-F8.5 | Markdown and YAML editor consumers | Select a host or prepare a kind contract after first result installation | Both see the installed view through the same cache. YAML can compile a contract with its version before the replacement load finishes | Use a neutral view or wait for the current result |

Schedule the replacement using the first result's dependency information and hold back its host profile and warnings. The existing controlled-clock tests can prove the behavior without a wall-clock sleep or a new terminal tier.

### R1-F9 (high) — The format 1 upgrade warning points to an unavailable command

**Authority — spec §§3 and 9.** “Reinstalling Claudine and rerunning setup is the supported way to refresh the user entry.” Setup also must be safe to rerun after interruption. The defect class is an upgrade instruction whose advertised entry point cannot reach the existing migration.

The shipped format 1 warning gives `claudine setup` as the repair. Running it with an existing Claudine configuration in a disposable home exits 2 with “unrecognized subcommand 'setup'.” The library already has the migration, including the required order of publish, permissions handoff, and registration. Configured hosts need a working route to it. [Upgrade warning](../../lib/src/config/registration/manifest.rs#L207), [documented repair](../../docs/topics/host-registration.md#L168), [Claudine setup](../../../claudine/lib/src/config/setup.rs#L161).

| Instance | Site | Upgrade path checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F9.1 | `md` and DMLS format 1 notices | Existing manifest with format version 1 | CLI produces a warning pointing at the version and the unavailable command. DMLS gets the same message from the library | Give a runnable host recovery step |
| R1-F9.2 | Claudine CLI and setup guide | Existing Claudine configuration plus old registration | `claudine setup` is unrecognized. The guide claims that it migrates the entry | Expose a repeatable migration for an existing configuration and document its real invocation |
| R1-F9.3 | First-run initialization and setup library | First-run or direct library invocation | Reaches publication and the locked config-migration code. Existing library tests cover repeated writes | Reuse that behavior |
| R1-F9.4 | `claudine registrations publish` | Existing old configuration entry | The publish route does not edit configuration, as required by spec §8 | Keep publish and config migration separate |
| R1-F9.5 | Obsolete repository entry | v1 manifest named from repository configuration | The passive reader applies the same version warning. Setup owns only the user entry | Keep the repository entry intact and report where its owner needs to edit it |

Add the repeatable Claudine entry point to the existing setup transaction and point the warning and guides to it. Preserve existing settings and the permission-conflict behavior. Test a configured temporary home through the CLI, including an old missing manifest reference and a second run after repair.

### R1-F10 (high) — Namespace and member validation differ across entry points

**Authority — spec §4 and the plan's member-envelope and build checks.** “Ship its kind definition with SimplifiedSchema so the CLI and DMLS can check its envelope and payload.” The plan requires a closed member contract and build rejection of ill-formed envelopes. The defect class is partial admission through a reader that implements only a subset of a member's rules.

A namespace document with `description:` validates in both the CLI and DMLS, even though explicit null is rejected when the same file is registered. With a malformed declaration in the namespace mapping, the CLI returned a `namespace-schema` cannot-be-a-schema-target error. It gave the same error for a direct check of an unedited copy of the published `loop.yaml`. That restriction is for referencing the namespace from another document. The compiled Claudine build script also generated an embed table for a copied type library with an extra top-level key. [Namespace semantic dispatch](../../../simplified-schema/lib/src/yaml_documents/semantic.rs#L41), [registered namespace admission](../../lib/src/config/registration/namespaces.rs#L177), [build member checks](../../../claudine/lib/src/composition/inventory/walk.rs#L303).

| Instance | Site | Shape tested or contract checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F10.1 | Namespace member loader | Optional description explicitly null, versus missing, and required namespace name numeric | Library tests reject null optional fields and admit omissions. CLI probe of numeric name in a registered file reports the name's position | Preserve admission and locations |
| R1-F10.2 | CLI and DMLS namespace document checks | Optional description explicitly null with an empty valid payload | Accepted by both | Report the invalid field |
| R1-F10.3 | CLI and DMLS namespace payload checks | Namespace mapping declares an invalid type | CLI probe reports the unrelated reference restriction. Shared semantic dispatch has no namespace payload branch | Check the definition itself and report the declaration problem |
| R1-F10.4 | Whole-YAML trigger and type library checks | Optional `name: null` | Both refused at `name` in CLI controls | Preserve those checks |
| R1-F10.5 | Claudine build path | Omitted trigger `match`, omitted YAML `$schema`, null namespace, and extra top-level field in a types-only library, each on a copied source member | First three prevent generation. The extra library field is admitted and gets an embed table | No table for any ill-formed envelope |
| R1-F10.6 | Claudine build and generator member checks | A present but null or wrongly shaped `match`/optional type table, or an unknown key in another member kind | The build checks only presence of `match` and omits optional type and unknown-key checking. The generator's full preparation applies the runtime envelope | Share structural member admission before code generation |
| R1-F10.7 | Built-in/local YAML selection and registered consumers | Unmodified published namespace and whole-YAML sequence selected by the registered definition | Registration and sequence selection work. Direct CLI validation of the unedited nonempty namespace fails with the schema-target restriction | Validate the namespace itself and preserve its exclusion from general `$schema` targets |

For the envelope matrix, required absent, null and wrong-type namespace names already fail. For optional scalars, omission is valid and explicit null must fail. For mapping fields, an empty namespace payload or local type table is valid, and a types-only library must supply a nonempty type table. A missing trigger payload is a valid matcher-only trigger. The shared YAML readers already refuse duplicate mapping keys, trailing garbage and second documents, and recognize bare YAML null. The healthy control is the unedited published inventory. Add a shared check for the remaining shape differences. Use existing envelope and payload preparation for CLI and editor validation, and keep compiled build admission structural so it need not resolve an entire host profile.

### R1-F11 (high) — Bundle refusals lose the failed field's source location

**Authority — spec §§3–4 and the first acceptance criterion.** “Report the failed field and source location.” The acceptance criterion explicitly requires locations for missing or unlisted imports, invalid trigger matches, and stale artifacts. The defect class is discarding typed locations while converting admission errors to registration warnings.

With the published Claudine bundle, I changed one field at a time and refreshed that member's digest. An invalid trigger name produced a `dm.host.registration` record with the member file but null line and column. The default formatted warning named the manifest entry and gave no member location. An invalid imported library label produced a warning pointing at the importing file while naming the real source only in the message. [Registration error projections](../../lib/src/config/registration.rs#L976), [formatted run notices](../../cli/src/output/mod.rs#L187).

| Instance | Site | Shape tested or invariant checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F11.1 | Manifest admission and digest verification | Omitted or null `functions`, duplicate key, second YAML document, and wrong member digest | Manifest CLI probes and L1 digest test supply a line and column | Preserve the located refusal |
| R1-F11.2 | Verified member kind and YAML loader | Duplicate `kind` in the registered formal sequence schema | The parser's line is embedded in the message. The diagnostic record has no position | Carry the parser's location |
| R1-F11.3 | Trigger envelope and payload | Bad trigger label or invalid match syntax, and static unlisted payload import | The refusal closures emit no coordinates. An overbroad but syntactically valid match follows a separate clean path that locates `match` | Locate the invalid label, match, or reference respectively |
| R1-F11.4 | Types-only libraries and their importers | Invalid kebab-case library name in the registered sequence library | Library and dependent payloads are refused. Importer warning points at the importing file with no coordinates | Preserve the actual failing file and declaration position |
| R1-F11.5 | Registered YAML kind definition | Definition importing the malformed sequence library | Preparation refusal points at the YAML kind file without a line | Locate the relevant declaration in the library or at the import |
| R1-F11.6 | Namespace definition and its imports | Numeric namespace name and a namespace payload that fails resolution | Numeric name was located in the CLI. Payload failure retains its own `$schema` position, but any more precise imported position is flattened | Preserve the most specific available location |
| R1-F11.7 | Lifecycle and function artifacts | Mismatched artifact host, stale hash, or bad function facts | Host mismatch records manifest `host` with no line. Both external artifact loaders create positionless refusals after verified reads | Locate the manifest field or the failed JSON/YAML field as applicable |
| R1-F11.8 | CLI and DMLS delivery | Same invalid trigger label or import in the installed bundle | JSON preserves available coordinates. Text warning prints just the message. DMLS can only project coordinates that survive admission | Carry the member location into the formatted and editor diagnostics |

Keep parser spans or locate the failed field before converting the error to a report. A missing field can point to its containing mapping, and an unreadable member can point to its manifest entry. Show member coordinates in formatted registration warnings when available. For the validator matrix, compare absent, explicit null, wrong whole type, mixed and wholly invalid element types on collections, and explicit empty controls through the same location projection. Document syntax controls are listed above and in the verification section. Test one malformed member with an exact source span through both CLI forms, and one imported error in the editor.

### R1-F12 (high) — Startup tests can miss reads of the installed bundle

**Authority — final acceptance criterion and the plan's registration-command proof.** Ordinary commands “neither read nor refresh the published bundle.” The plan asks for observable read and write instrumentation. The defect class is an assertion that can establish unchanged content while allowing a silent read.

The Claudine command suite snapshots names, bytes, modification times, and stderr. A startup check could quietly read the manifest and leave all of those assertions true. I watched open/access events on all 17 files of a published bundle in a configured temporary home. `claudine errors` produced none. As a positive control, `md schema validate` produced read events when it used the same registration. That is direct L1 evidence for this Linux run. The checked-in behavioral tests still lack a read observation and would miss a later silent startup read. [Startup tests](../../../claudine/cli/tests/l1/registrations_command.rs#L225), [CLI startup and publication dispatch](../../../claudine/cli/src/main.rs#L529).

| Instance | Site | Shape checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F12.1 | Configured ordinary startup with installed inventory | Valid publication and registered home | Direct file monitoring sees zero reads for `errors`. Automated tests compare only snapshots and output | Pin read passivity at the production entry point |
| R1-F12.2 | Obstructed, missing and stale paths | Current ordinary-command fixtures | Exit succeeds, stderr stays clear of registration messages, and saved files remain untouched. Quiet reads are unobserved | Preserve the existing write assertions and make a read attempt visible |
| R1-F12.3 | Intended reader and explicit publishing route | Same publication used by `md`, and an explicit Claudine publish | Positive read monitor fires for `md`. Existing CLI tests prove publication and an unchanged second publish | Use both as controls for the observation |

Add a small L1 observable read check around the representative configured startup command with one positive control. It does not require a terminal or a provider launch.

### R1-F13 (medium) — The lint timer can round a completed command to zero

**Authority — defect.** The package CI lint command measures elapsed time and its Test Toolkit test requires a positive value when the command ran. The defect class is rounding a short positive duration to the same value used for a missing measurement.

The implementation log records the Test Toolkit failure with `duration_s=0.0` for the test's real short-lived shell stub, including a failure in isolation. The single isolated review run passed, consistent with a timing-dependent failure. The workflow rounds seconds to three decimal places before formatting. The short stub runs below half a millisecond on some runs. [Workflow measurement](../../../.github/workflows/_package-ci.yml#L1341), [test](../../../tools/test-toolkit/tests/ci_workflow_contracts.rs#L3695).

| Instance | Site | Shape checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F13.1 | Package lint elapsed-time publication | Existing immediate-return `just` stub | A positive measured duration below 0.5 ms becomes 0.0 | Preserve the positive measurement |
| R1-F13.2 | Test Toolkit assertion and report distinction | Same command exits normally or returns an error | Test accepts both command exit statuses when duration is positive. A 0.0 duration is treated like no measurement by the report | Keep elapsed time and exit status independent |
| R1-F13.3 | No-Python fallback in the same step | Timer unavailable | Step emits empty duration and still returns the lint result | Preserve the unavailable case |

Keep sufficient precision in the published duration, ideally from a monotonic integer clock, and retain the existing status checks. This is a pre-existing issue encountered during review.

## Verification

I used the current worktree's Linux binaries in temporary homes and, where trigger discovery required it, a temporary Git repository. Each bundle probe started from `claudine registrations publish`. I refreshed only the changed member's manifest digest where the test needed to reach semantic validation. For editor results I used DMLS's standard input/output protocol. These are Level 1 behaviors. None requires real-terminal rendering, OS input, a provider launch, or a real editor window.

| User-visible requirement | Strongest evidence reviewed | Result |
| --- | --- | --- |
| Load a complete portable snapshot, verify exact bytes, keep watches, and normalize identity independently of file order | Level 1 public snapshot and registration tests, plus a real publication outside the checkout and CLI probes | Valid bundle, digest refusal and reordering covered. Deferred imports and locations have gaps in findings R1-F1 and R1-F11 |
| Choose a Markdown host from authored values and choose a registered whole-YAML kind at the intended precedence | Level 1 shared-event, literal, ambiguity, precedence and collision cases in CLI and DMLS. Direct probe of numeric Markdown and YAML sources | Markdown accepts malformed scalar sequence inputs. See R1-F2 |
| Admit namespace and lifecycle definitions and serve host analysis | Level 1 namespace tests, public artifact admission probes, and stdio editor checks | Binding and envelope defects in R1-F3, R1-F10, and R1-F11 |
| Compare every matched trigger and keep higher layers authoritative | Level 1 effective schema tests for equivalent and narrower types, constraints, presence, recursive declarations, three-way conflict, unmatched triggers, and a successful local property override. CLI conflict probes | Closed and union precedence, passive trace, and editor declaration gaps in R1-F5 through R1-F7 |
| Validate exactly one inline alternative, with explicit null allowed for the other | Level 1 installed and embedded fixtures exercise prompt, sections, both, neither, and a competing trigger. The overlap probe produced the coded refusal | Core enforcement covered. Preserve it while repairing R1-F5 and R1-F7 |
| Publish safely through interruption and contention, converge on retry, and preserve unchanged timestamps | Level 1 those scenarios in the publisher suite, including verified reader views and no-op timestamps, plus actual CLI retirement probes | Covered for a valid prior manifest. See R1-F4 for malformed retirement input |
| Reload only after quiet, follow moved or missing files, retain the last view, and clear warnings after repair | Level 1 controlled-clock DMLS protocol tests for batching, unchanged revision, move, ordinary edit, save fallback, and repair. Controlled install ordering checks both load states | First-load gap in R1-F8 |
| List, inspect, hash, and publish from the embedded inventory | Level 1 command suite compares the embedded inventory and exact bytes, including dry listing, detail, unknown path, publish failure, and identical republish | Covered |
| Preserve settings through setup and keep ordinary startup passive after configuration | Level 1 setup transaction tests, CLI initialization tests, configured startup tests, and direct Linux read observation | Repeatable CLI recovery and durable read observation in R1-F9 and R1-F12 |

The focused Linux runs passed 313 Darkmatter `host_` cases, five editor registration reload scheduling cases, eight Claudine registration-command cases, 18 Claudine publication cases, ten registered YAML kind cases, and one DMLS trigger-conflict case. Each run recorded zero tests at or above five seconds. A separate 47-case registration run overlaps the 313-case run and had the same result. The isolated lint-timing test passed once during this review. Its recorded failure remains in R1-F13. I used the implementation log for the wider suite history and platform evidence.

For the input matrix, the shared YAML reader refuses duplicate keys (including nested keys), trailing invalid content, and second documents. Normal comments and quoted scalars work in the published control. A bare `functions:` is an explicit null and is refused. The typed reader requires manifest fields and rejects wrong types, while empty member lists are valid with the required lifecycle and digest still present. The shared strict lifecycle and function-facts JSON reader rejects duplicate keys and trailing values, and accepts ordinary whitespace. The registration fixtures separately cover a valid artifact and a stale or malformed artifact. Config setup recognizes an absent or empty registration list and the existing explicit-null convention. It distinguishes a permissions conflict from invalid configuration. The findings table rows record where an otherwise rejected shape is admitted or loses its position. The test suites declare their Level 1 targets and include the registration test modules from their declared roots.

## Implementation decisions (informational)

Two independent readers reviewed the implementation record and specification before this comparison. The plan authorizes unattended verification and bounds the timing choice. The specification leaves doctor and update integration, lifecycle availability syntax, replacement of function catalog pairs, and sections execution to their own work. None of the decisions below calls for an author ruling.

- **Initial cutover.** The format 1 reader, a restored live sequence fixture, and the single-input lifecycle hash were temporary bridges. The work kept the old fixture because the benchmark version had a different grammar and imports. Claudine's interim format 1 publication tests were expected to fail against the format 2 reader. The final bundle replaced those bridges, wired registered YAML selection, and replaced the temporary inline trigger with the checked embedded union.
- **Manifest syntax and identity.** The reader chose portable ASCII filename characters, including a leading dot, and distinguished a format 1 upgrade from an unknown legacy key on a true format 2 manifest. It boxed snapshot refusals to keep Rust error values small and sorts normalized paths so manifest ordering does not change results.
- **Scope and examples.** The resolver uses a thread-local verified byte table and a virtual embedded root. Slot data requires the normal grant checks. The four permitted member kinds exclude standalone example files. The implementation compares example metadata as constraints, so it omitted an overlap test for a standalone registered example. Restoring the verified scope after a slot resolves is an ordinary repair.
- **Envelope labels and YAML selection.** The code reuses trigger diagnostics for duplicate trigger/library labels, including declarations shadowed by another schema root. YAML kind diagnostics reuse the duplicate cause with host-then-path ordering. Equal revision-and-path copies collapse and refused definitions are excluded. Conflicting versions of the same host keep their YAML claims available for collision reporting. The selected definition carries its host through its source record.
- **Namespace and diagnostic choices.** An empty namespace payload is valid. The implementation retired the two diagnostics specific to legacy schema-root catalogs. It relied on the event registry to validate binding roots and on admission parsers to reject null labels and metadata. The direct probes identify where to repair those assumptions. The location omissions for invalid names and formatted warnings require the ordinary diagnostic repair above.
- **Checked merging.** Inline root unions are admitted for checked registrations. Existing ordinary trigger assembly keeps its prior object contract. Per-key origins and default attribution extend to union arms. Pending registered payloads resolve whole before overrides, and their results skip the static cache. The static matched-set cache retains successes and conflicts for the life of the profile. A boxed conflict keeps the shared error type small.
- **Higher layers and editor views.** The implementation places higher layers below pruned union arms, rejects an authored union over a host union, and omits the higher-layer closed rule. Root unions currently provide no editor shape, so six tests switched from inline prompts to agent-only inputs. Those decisions need repairs within the existing precedence and editor contract. Conflict JSON gained the code and related trigger paths. DMLS relates the two files at their beginning because the merged declarations carry no more precise conflict span. YAML preparation waits for host loading and can republish the same document version.
- **Claudine's source bundle.** Formal-sequence schema validation follows the runtime parser when it accepts an inline list, preserving its more precise errors and accepted reference and dynamic source forms. The config types library is part of the published bundle. Same-name disk triggers apply as higher ordinary layers, and a property is Claudine-owned only when the registered layer supplies the winning declaration. A bad member refuses the whole embedded profile, matching the installed route.
- **Build and preparation costs.** The build does structural checks and the generator and tests do full preparation. The incomplete structural checks are addressed in R1-F10. Prepared payloads grew the shipped artifact from approximately 3.1 MB to 5.7 MB and shortened reported debug cold preparation from about 0.5 s to 0.2 s. Shared default-plan reuse reduced a measured 10.1 s test to 1.4 s in isolation before the later passing suite. The publication and legacy cleanup modules were split to fit the repository's size check.
- **Publisher.** The lock waits up to ten seconds for cooperating publishers. The implementation compiled a hidden test hook and used a separate parser to decide retirement when old member bytes no longer match. A valid envelope can carry the removal decision in that state. The repair in R1-F4 supplies equivalent admission before trusting it. Listing runs after request capture and before configuration loading. The install helper reports a file count and was exercised with an isolated publish recipe and a dry-run install to avoid changing the review host's installation.
- **Setup.** The migration canonicalizes equivalent entries at their original position and uses the first resolution candidate for a missing old path. Legacy cleanup checks the whole owned tree before deleting anything and stops on an obstruction. The passive reader reports obsolete repository entries while setup leaves them intact. The docs direct users to hand-edit a missing registration and use explicit publication for damaged members. A repeatable setup route for migration is an ordinary repair within scope.
- **Reloads.** The server uses path-only notification checks and hashes on the background worker. An unchanged same-byte write can trigger a reload without reanalysis. Warning-only changes keep the analysis version. The production settle window remains 300 ms after the measured publish. In-process and shared test defaults use zero delay, and the explicit timing suite uses a controlled clock. Watch-limited saves debounce while explicit full invalidations and the existing server-rescan path stay immediate.
- **Reload verification.** The first touched cold result was deliberately installed, while touched or superseded warm results were dropped. The implementer used controlled in-process ordering for the in-flight case because the stdio worker gate also blocked document/workspace jobs. The controlled Level 1 boundary can verify the repair to the cold path.
- **Recorded deferrals.** Future doctor/update commands will reuse the inventory and setup interfaces, and inline sections are checked as schema data while their execution is deferred. The macOS verification rig compiled and archived before its login shell failed to find `just`. The log records reused Windows and passing WSL2 evidence. These conditions do not require a human ruling or change the readiness result.

The implementation log now records the verified differences in whole-YAML label admission, lifecycle binding checks, publisher retirement, and the unavailable setup command.
