---
$schema: feature-review.yaml
ready: false
findings:
  - "R3-F1 (high): Doctor offers permissions repairs whose reference the writer refuses"
  - "R3-F2 (high): Doctor misses broken Darkmatter references when a Claudine file is unavailable"
  - "R3-F3 (high): Setup guidance overlooks the publication that must run first"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-24T07:18:00+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-3.md
previous: 2026-11-22-claudine-doctor/review-2.md
next: 2026-11-22-claudine-doctor/review-4.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/log.md
implemented-by: claude/opus
---

# Claudine Doctor review 3

**Production readiness declined.** Review 2’s cases pass. Three additional
cases remain in permissions diagnosis and repair guidance. Each can be
repaired within the existing contract, without a human ruling.

## Previous Finding Verification

I used the child rows in the review 2 repair ledger as the verification
inventory. The test counts and scope are recorded below. Review 1 had no
open items after review 2, and this review found no regressions in its
original cases. Neither earlier review has a Human rulings section.
[Review 2](review-2.md) · [Repair ledger](log.md#finding-ledger-1)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1 A failed user permissions repair skips the repository repair | .1, .c1–.c4 | verified fixed | The Claudine library tests held each scope's lock in turn, checked completed changes and retry, and verified both-scope success, callback independence, and ordered interruption inside a transaction. [Permissions tests](../../lib/src/doctor/permissions_tests.rs) · [Runner tests](../../lib/src/doctor/tests.rs) | None. |
| R2-F2 Doctor refuses loadable manifest member references | .1–.6, .c1–.c2, .d1 | verified fixed | Canonical, re-entry, and escape cases passed for the four schema categories and the lifecycle artifact. Darkmatter's installed-admission test exercised the catalog and facts pair too. Both digest spellings produced the same mismatch attribution. Registration identity, exact-byte repair verification, and stricter retirement admission passed. I checked the skill text against the call from inspection to manifest judgment. [Claudine registration tests](../../lib/src/doctor/registration_tests.rs) · [Darkmatter snapshot tests](../../../darkmatter/lib/tests/l1/host_registration_snapshot.rs) · [Claudine repo skill](../../../.claude/skills/claudine/SKILL.md) | None. |
| R2-F3 Merged configuration errors can name the wrong file | .1–.5, .c1–.c3 | verified fixed | The effective route, mapper, and exit-expression tests located the authoring scope. The route test also named the user catalog. The user-only TTS and handoff tests explained their prerequisite despite the merged result. User-only settings and schema and stray-key controls passed. [Config tests](../../lib/src/doctor/config_tests.rs) · [Speech tests](../../lib/src/doctor/tts_tests.rs) · [Permissions tests](../../lib/src/doctor/permissions_tests.rs) | None. |
| R2-F4 The speech report confuses highest quality with runtime selection | .1–.2, .c1–.c3 | verified fixed | The candidate-order tests separated highest quality from first tried for automatic and unknown-provider fallback. Below-Good, pinned, disabled, and no-candidate controls passed. Pin filtering also matches biscuit-speaks' runtime selection. [Claudine speech tests](../../lib/src/doctor/tts_tests.rs) · [Runtime candidate selection](../../../biscuit-speaks/lib/src/detection.rs#L282) | None. |
| R2-F5 Some remedies cannot act on the reported problem | .1–.4, .c1–.c2, .d1, .x1 | verified fixed | The mapper and sound CLI comparison, invalid register diagnostic, user/repository handoff controls, and scoped reference remedies passed. Syntax, legacy conversion, missing-file, conflict, and manual-mapping controls passed. The source comment and docs describe Doctor’s early dispatch and inspection. All five setup-null integration cases passed and verified Darkmatter reads the resulting policy. [Claudine CLI tests](../../cli/tests/l1/doctor.rs) · [Configuration tests](../../lib/src/doctor/config_tests.rs) · [Permissions tests](../../lib/src/doctor/permissions_tests.rs) · [Setup tests](../../lib/tests/l1/darkmatter_setup.rs) | None in the required cases. R3-F3 is an additional user setup prerequisite that the earlier sweep missed. |
| R2-F6 Flat entries hide independent configuration diagnostics | .1–.3, .c1–.c2 | verified fixed | All 20 keyed/flat document-rule cases passed, including agent spellings, both/neither exit-expression fields, both list forms, and authored positions. Nested typo, wrong-scope, and legacy conversion refusal controls passed. [Config tests](../../lib/src/doctor/config_tests.rs) · [Conversion tests](../../lib/tests/l1/config_json_conversion.rs) | None. |
| R2-F7 Permissions repair is offered for an invalid Darkmatter file | .1–.2, .c1–.c2 | verified fixed | Both scopes passed the omitted/null/empty/valid and scalar/mixed/all-invalid register tests. Invalid inputs blocked the affected edit, and the independent scope still repaired. The registration repair precondition and already-referenced permissions controls passed. [Permissions tests](../../lib/src/doctor/permissions_tests.rs) · [Registration tests](../../lib/src/doctor/registration_tests.rs) | None for invalid register. R3-F1 is an additional eligibility check. |
| R2-F8 Darkmatter suggests user initialization for broken repository references | .1–.5, .c1 | verified fixed | Darkmatter's 18 scoped remedy tests exercised the six target failures under user, working-copy, and committed policy. Equivalent spelling, valid target, and CLI error-location tests passed. The CLI probe with no Claudine repository file likewise showed the corrected Darkmatter remedy. [Darkmatter library tests](../../../darkmatter/lib/tests/l1/config_permissions.rs) · [Darkmatter CLI tests](../../../darkmatter/cli/tests/l1/config_notices.rs) | None for Darkmatter's remedy. R3-F2 concerns whether Doctor reports the failure at all. |

## Unblocked Findings

### R3-F1 (high) Doctor offers permissions repairs whose reference the writer refuses

**Authority (spec).** “Fixable means that an available automatic repair
applies to a finding in that row.” A user can link a configuration file or
its containing folder into a dotfiles checkout. When the linked file already
holds the desired policy, Doctor calls the reference-only edit fixable.
Its writer rejects the new reference before saving because the target lies
outside the scope. This is a new sibling of R2-F7, on the linked targets
covered by R1-F1. [Permissions assessment in Claudine](../../lib/src/doctor/permissions.rs#L83) · [Reference check and writer in Claudine](../../lib/src/config/setup.rs#L357)

The defect class is repair eligibility that omits a deterministic writer
precondition. The report and writer must apply the same rule to the proposed
reference. I used the published-home fixture and ran `claudine doctor
--json` and `claudine doctor --fix --json` after each change. Replacing an
ordinary file with an in-scope link to equivalent bytes kept the
reference-only edit possible. Moving that same link target outside the
scope made the writer refuse. The permissions row gave the same initial
answer in both cases. Its final report repeated the recommendation even
after the refusal.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R3-F1.1 | User handoff assessment and writer | Equal inline policies or omitted Darkmatter setting. Link `config.yaml` to the same bytes inside, then outside home. | Inside repairs. Outside reports `fixable: true` before and after a deterministic refusal. The writer reports no write. | Apply the same scope check in assessment. Name the user target and give a manual remedy when the writer cannot save. |
| R3-F1.2 | User handoff via linked parent | Same controls with a linked `.claudine` folder. | Same inside/outside split and false eligibility. | Same verdict as a direct file link. |
| R3-F1.3 | Repository handoff assessment and writer | Same equal/omitted controls and inside/outside file link in a separate repository. | Inside repairs. Outside remains `fixable: true` after the writer refuses it. | Name the repository target and give a manual correction before attempting a write. |
| R3-F1.4 | Repository handoff via linked parent | Same controls with a linked `.claudine` folder. | Same inside/outside split and false eligibility. | Same verdict as a direct file link. |
| R3-F1.c1 | Default or transfer requiring a Claudine write, both scopes | Unlinked Claudine file with no mapping, then links to the same file. | Unlinked repair writes both files. Linked repair is already manual before any edit, whether the target is inside or outside. | Preserve the write-destination guard and independent scope behavior. |
| R3-F1.c2 | Existing references, both scopes | Keep an already-authored reference and toggle the same paths. | In-scope references pass. A user reference to an external linked target passes under Darkmatter’s user policy. The equivalent repository reference fails under Darkmatter’s repository policy. None advertises a repair. | Keep existing-reference validation independent of the proposed-reference check. |
| R3-F1.c3 | Registration publication and entry writer | Existing publisher and sidecar obstruction tests, equivalent manifest identity, and invalid register writer controls. | The path and file preconditions guard their respective writes. The registration writer proposes a manifest entry through its own publisher, with no permissions-reference target. | Preserve those checks. |

Reuse the writer’s scope check when assessing a proposed permissions
reference and keep the writer’s second check immediately before the
transaction. The assessment should give the user an actionable correction
at the affected scope and preserve the other scope’s eligibility. No new
path policy or transaction mechanism is needed. The omitted and explicit
null forms use the same planner branch. [Handoff planner in Claudine](../../lib/src/config/setup.rs#L534)

### R3-F2 (high) Doctor misses broken Darkmatter references when a Claudine file is unavailable

**Authority (spec).** Doctor must check existing repository `.darkmatter.yaml`
files under their own scope, reuse normal configuration validation, and
report that “a broken or invalid reference is an error and requires manual
correction.” The optional Claudine override is only a prerequisite of the
handoff. With a healthy user installation and a repository containing only
`.darkmatter.yaml` with `permissions: ./policy.yaml`, Doctor exits 0 when
the policy file is missing. In the same fixture, `md compose doc.md --output
json` on a document with one shell directive fails with a located
`dm.config.reference` error. With a valid policy file it proceeds to shell
approval. [Configuration observation in Claudine](../../lib/src/doctor/config.rs#L207) · [Handoff prerequisites in Claudine](../../lib/src/doctor/permissions.rs#L176) · [Darkmatter working-copy reader](../../../darkmatter/lib/src/config/load.rs#L969)

The defect class is an unrelated prerequisite suppressing an independent
validation result. A valid third-party reference must stay healthy under
an equivalent spelling. Removing or breaking its target must produce a
located error whichever Claudine file is present. I used `./policy.yaml`
and `././policy.yaml` as valid controls, then tested a missing file, invalid
YAML, a chained reference, a null or scalar block, a document without a
block, a nonmapping document, and a link to a valid file outside the
scope. Darkmatter allows the external user target and refuses it for a
repository. An inline scalar and a blank reference exercised the controls where the
declaration itself fails. The publish and executable rows were healthy.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R3-F2.1 | Repository configuration and handoff | No repository Claudine file. Use the two valid spellings, then each broken target. | Valid controls exit 0. Each broken target also exits 0 with `config: ok` and an informational handoff skip. Both CLI formats and `--fix` reproduced the missing-target case. | Keep the skip for handoff, but report the broken repository policy in `config` and exit 1. |
| R3-F2.2 | Repository configuration with an aliased or invalid Claudine counterpart | Point the optional override at the user Claudine file, then replace it with malformed YAML. Repeat the target changes. | The alias produces the same false success. In the malformed case, the Claudine YAML makes the command fail but the repository policy error is absent. | Validate the distinct repository Darkmatter declaration in both cases. Report the two independent failures together when applicable. |
| R3-F2.3 | User configuration with an absent or invalid Claudine counterpart | Remove the user Claudine file, then give it malformed YAML. Repeat the target changes for a separate user policy file. | The required Claudine file error keeps exit 1. `config` calls the user Darkmatter file valid and the handoff skips, without reporting the broken target. | Keep the Claudine error and the handoff skip. Report the independent Darkmatter error too. |
| R3-F2.c1 | Separate valid Claudine pair, both scopes | Repeat every spelling and target change with a valid counterpart. | Valid third-party policy is informational. Every broken target is already an error in the permissions row. | Preserve that status and scope. Avoid duplicating the full diagnostic across both rows. |
| R3-F2.c2 | Validation of the Darkmatter declaration, both scopes | Blank reference and inline scalar with the counterpart present, absent, invalid, or aliased where applicable. | `config` already reports the invalid declaration; only errors obtained by following a syntactically valid reference are lost. | Keep declaration errors and validate a target through the same scope-aware resolver. |
| R3-F2.c3 | Darkmatter runtime and repair consumers | The shell document, the scoped-reference tests, and a broken Doctor target with `--fix`. | Darkmatter reports a located failure through its loader. Doctor makes no target repair. An aliased Darkmatter declaration is intentionally checked once as a user file. | Reuse Darkmatter’s working-copy resolver. Keep broken references manual, the Darkmatter alias rule intact, and committed policy out of Doctor. |

Make working-copy reference validation independent of the availability of
the Claudine counterpart. Reuse Darkmatter’s passive reference result and
show the declaring file and available location without printing its authored
value. The existing handoff assessment can still skip an incomplete pair.
Test the public result with a broken and valid repository target and a
missing optional override. [Darkmatter reference resolution](../../../darkmatter/lib/src/config/load/reference.rs#L61)

### R3-F3 (high) Setup guidance overlooks the publication that must run first

**Authority (defect).** With a repairable user handoff and a linked registration member, Doctor
offers `claudine setup`. The command stops at registration publication
and leaves the inline permissions untouched. `claudine doctor --fix`
completes the permissions handoff in that same fixture and still reports
the registration obstruction. This is a new sibling of R2-F5.
[Remedy selection in Claudine](../../lib/src/doctor/permissions.rs#L165) · [Setup sequence in Claudine](../../lib/src/config/setup.rs#L177)

The defect class is a suggested command whose mandatory earlier step
prevents it from acting on the named problem. I started with matching user
policies and a published bundle. The control setup wrote the reference.
Changing the location of the same bytes with a link at different publisher
paths left Doctor’s handoff recommendation unchanged, even when setup now
stopped before reaching the policy files.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R3-F3.1 | User handoff remedy, registration path | Move the registration folder, schema folder, manifest, or a schema member and link back to the same bytes. | Doctor offers setup. Setup exits 1 and leaves the policy inline. Doctor’s own fix completes the handoff with the registration error still visible. | Offer the independently available Doctor repair. |
| R3-F3.2 | User handoff remedy, shared parent | Replace `~/.claudine` or its `registrations` folder with a link to an in-home folder holding the same files. | Doctor needs to edit only Darkmatter’s file and marks it repairable. The suggested setup exits 1 on the publisher path before the handoff. | Offer the independently available Doctor repair. |
| R3-F3.3 | User handoff remedy, publisher lock | Replace the publisher lock entry with a link in an otherwise current bundle. | The registration row is `ok` because no registration edit is needed. Setup still tries to acquire the publisher lock and stops. | The handoff remedy must work even when a separate setup prerequisite is hidden by a healthy registration row. |
| R3-F3.4 | User handoff remedy, retirement during publication | Add a former member and its digest to the installed manifest, then link the member to the same bytes. | The publisher stops at the former member before setup reaches the handoff. Doctor still offers setup. | Recommend the available Doctor repair. |
| R3-F3.c1 | User handoff with a healthy publisher or a later cleanup obstruction | Ordinary paths, linked legacy manifest, link only at Claudine’s policy file, linked temporary file, and unlisted extra link. | Setup completes the handoff in each. It leaves the two unlisted links in place. The legacy obstruction produces a later warning. | Preserve setup behavior. |
| R3-F3.c2 | Other setup suggestions and handoff scopes | Missing `~/.darkmatter.yaml` with and without the linked publisher path. Ordinary user missing/legacy files and repository handoff controls. | Setup creates the missing Darkmatter file during command initialization even when publication later stops. The original missing/legacy controls pass. Repository handoff guidance already offers Doctor’s fix alone. | Preserve the scoped missing-file and conversion guidance. Keep the handoff recommendation tied to the repair it describes. |
| R3-F3.c3 | Other remedy decision sites | User sound/mapper, repository configuration, Darkmatter register and reference, and direct PATH/speech corrections. | The R2-F5 and R2-F8 tests and probes show the original category and scope distinctions. None of these message-only lookups depends on a successful setup handoff. | Preserve those distinctions. |

Use `claudine doctor --fix` as the handoff remedy in either scope. That
avoids a second publisher preflight solely to recommend an optional
command. Update the remedy test and its description in the command guide.
The setup command’s ordering and supported behavior can stay as they are.

## Blocked Findings

None. No human decision is required.

## Recurrence

- **R3-F1** is a missed sibling of **R2-F7, permissions repair is offered
  for an invalid Darkmatter file**. That repair checked whole-file
  validity but omitted the writer’s proposed-reference check. It also
  follows **R1-F1, permissions repair follows linked configuration
  directories**. The original linked-write cases are fixed. The remaining
  case is a write to a regular Darkmatter file whose proposed target is
  linked outside scope. The table checks both scopes, both link locations,
  the reference-only and transfer paths, and the registration control.
- **R3-F2** is a missed sibling of **R2-F6, flat entries hide independent
  configuration diagnostics**. The repaired Claudine parser collects
  independent document findings. This instance is in Doctor’s projection
  of a Darkmatter result, where requiring the handoff pair still masks a
  policy error. The sweep covers both scopes, both report paths, and the
  target and declaration controls. **R2-F8** verifies the separate
  Darkmatter reporting path.
- **R3-F3** is a missed sibling of **R2-F5, some remedies cannot act on
  the reported problem**, and **R1-F8, configuration remedies promise
  behavior from commands that bypass configuration**. The earlier sweep
  distinguished user and repository setup, but exercised the user handoff
  only after successful publication. The table covers the publisher entry
  points and the cleanup that runs after the handoff. The simpler remedy
  avoids extending setup eligibility checks into the report.

## Verification and input robustness

The specification calls for Level 1 evidence for these non-interactive
behaviors. Grouped help, completion, report formatting, file inspection
and edits, and subprocess exit and timeout behavior can all be exercised
without opening a terminal or generating keyboard events.

| Requirement | Strongest verification and result |
| --- | --- |
| Help, completion, early dispatch, and passive inspection | Level 1 shipped CLI fixtures cover help, usage, missing and broken files, no conversions, and unchanged fixture trees. Direct completion of `doc` returned `doctor` without configuration dispatch. Read-side speech and path methods select local providers and look up executables. |
| Configuration scope, locations, aliases, and semantics | Level 1 public configuration results, Doctor rows, and CLI tests cover both scopes, merged validation, optional files, legacy and alias cases, syntax and stray-key locations, and redaction. R3-F2 covers the missing target diagnosis. |
| Registration inventory, repair order, reporting and safety | Level 1 installed-bundle and publisher fixtures cover stale and missing members, admission parity, other-host preservation, partial operations, links, lock ordering, and retry. |
| Permissions handoff | Level 1 public-result fixtures and CLI probes cover equivalent and third-party references, conflicts, ordered partial writes, no-op retries, and independent scopes. R3-F1 through R3-F3 identify gaps in eligibility, reference diagnosis, and guidance. |
| Exit and both output formats | Level 1 CLI verifies full row order and the JSON contract, warning-only exit, unresolved errors, successful repair, failed repair, redirected and plain output, redaction, and stdout write failure. |
| Speech and executable detection | Level 1 controlled candidate order and executable/process fixtures cover automatic and pinned speech, disabled hook speech, unavailable or unknown providers, PATH hit and miss, absent shells, timing out, and failed probes. Native-Windows profile isolation and executable suffix evidence is recorded in the repair log. |

Claudine and Darkmatter share the admitted YAML document loader. Invalid
document shapes produce located errors and inspection leaves files alone.
I checked the duplicate key, invalid or trailing input, second document,
comments, and YAML null coverage. The host registration manifest adds
its own field admission over decoded YAML. The installed snapshot tests cover
required fields, invalid formats, empty allowed lists, mismatched digests,
and duplicate member paths. Doctor detects the presence of legacy JSON.
The separate conversion path upgrades keyed forms and validates the staged
result before publishing. [Claudine configuration tests](../../lib/src/doctor/config_tests.rs) · [Darkmatter configuration tests](../../../darkmatter/lib/tests/l1/config_fields.rs) · [Manifest tests](../../../darkmatter/lib/tests/l1/host_registration_snapshot.rs)

| Load-bearing validator | Control outcomes and exceptions to invalid-shape rejection |
| --- | --- |
| Actions and messenger routes | Keyed entries pass, and flat entries keep their migration and independent diagnostics. Absent/null overrides inherit. Empty actions and wrong whole, mixed, and all-invalid action element types receive the schema verdict. Named keyed route objects select providers, and null route values are rejected. |
| Speech and repository overrides | Omitted/null/false speech disables hooks. True and an empty object enable automatic selection. A populated object can pin a provider. Empty provider strings and bad scalar, object, or sequence shapes produce located errors. A nonempty unknown provider falls back with a warning. |
| Darkmatter permissions and references | Omitted/null top-level permissions use defaults. An empty mapping and allowed empty lists follow the scope rules. Invalid whole, mixed, and all-invalid rule shapes are covered by located public-result tests. A reference target requires a mapping. Missing, null, chained, malformed, and wrong-shaped targets fail Darkmatter's loader. R3-F2 covers Doctor’s missing projection when the Claudine counterpart is unavailable. |
| Registration lists and their consumers | Omitted/null/empty lists are accepted. Wrong whole, mixed, and all-invalid arrays are reported with the relevant locations. Invalid settings prevent edits to their file. The registration tests cover valid equivalent identities and block invalid entries, and the R2-F7 cases verify that the unaffected scope stays eligible. |

I checked the changed readers for optional and defaulted or filtered parse
paths. Darkmatter records invalid list entries before projecting valid
strings. Claudine’s flat-entry normalization preserves the independent
schema and document rules. Doctor never treats a failed reference target
as an empty policy. The R3-F2 table records the missing diagnosis at the
consumer.

| Level 1 run on Linux | Result |
| --- | --- |
| Claudine library through the shared `just _test` recipe | 5,882 passed. |
| Claudine library integration filtered to setup, conversion, and installed registration | 79 passed. |
| Claudine CLI Doctor and setup through `just test-cli` | 28 passed. |
| Darkmatter config, permissions, and registration through the shared `just _test` recipe | 99 passed in the library and 10 passed in the CLI. |
| Sniff Windows shell fixtures through the shared `just _test` recipe | 25 passed. |

The runs recorded zero tests at or above 5 seconds. The test paths enter
declared library, binary, and consolidated L1 targets.
`just check-tier-coverage` found no stranded tests in Claudine, Darkmatter,
or Sniff. The Claudine and
Darkmatter CLI runs enabled their configured test fixtures. The review
reran Linux tests and used the earlier repair record for native-Windows
observations.

## Implementation record corrections

I narrowed the repair log’s remedy sweep to the verified cases and added
the setup counterexample. I also clarified the implementation record’s
coverage entries for broken references and permissions planning. The
dated notes describe the inputs, commands, and results. The original
implementation test counts remain as recorded.
[Repair log](log.md) · [Implementation record](implementation-log.md)
