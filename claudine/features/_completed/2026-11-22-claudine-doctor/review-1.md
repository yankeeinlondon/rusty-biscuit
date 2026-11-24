---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Permissions repair follows linked configuration directories"
  - "R1-F2 (high): Doctor accepts changed registrations that Darkmatter refuses"
  - "R1-F3 (high): Flat configuration entries hide genuine unknown keys"
  - "R1-F4 (high): PowerShell startup writes during inspection"
  - "R1-F5 (high): Failed cleanup can hide files already removed"
  - "R1-F6 (high): CLI validation and repair-format tests miss required boundaries"
  - "R1-F7 (medium): Case-only aliases can be deleted as registration extras"
  - "R1-F8 (low): Configuration remedies promise behavior from commands that bypass configuration"
human-review: true
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: false
created: "2026-11-24T02:05:39+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-1.md
next: 2026-11-22-claudine-doctor/review-2.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/log.md
implemented-by: claude/opus
human-review-items:
  - "R1-F1: only doctor refuses a linked ~/.claudine; claudine setup still writes through it, so dotfiles users who link the folder are not broken"
  - "R1-F4: the PowerShell probe now writes its startup cache to a private OS-temp folder (USERPROFILE/LOCALAPPDATA/APPDATA redirected) and removes it, so 'no side effects' means nothing under the profile changes"
---

# Claudine Doctor review 1

**Production readiness was declined.** A permissions repair can write outside
the intended directory and replace a working inline policy with a broken
reference. Doctor can also exit successfully when Darkmatter refuses the
installed bundle. All findings have repairs within the agreed scope.

## Findings

### R1-F1 (high) Permissions repair follows linked configuration directories

**Authority (spec).** The permissions handoff must apply setup's handoff
“where safe” and “validate both candidates” before publishing them. The
repair plan requires the shared no-follow guard for both writer targets and
sidecars. When I linked the repository's `.claudine` directory to a shared
directory outside the repository, `doctor --fix` edited the shared config
and replaced the inline block with a reference that Darkmatter then refused.
In user scope, the same layout edited the shared file and reported that the
repair succeeded. [Permissions handoff](spec.md#darkmatter-permissions-handoff) · [Repair plan](plan.md)

**Class and invariant.** Checking only a destination's final component
allows a write through a linked ancestor. Changing an ordinary directory
into a link to an identical file must change repair eligibility from
automatic to manual before either config or sidecar is touched. Equivalent
spellings of an already valid reference should continue to pass inspection.

Claudine's [config_target_blockers](../../lib/src/composition/registration/inspect.rs#L243)
checks the config and sidecar as individual paths.
[obstruction](../../lib/src/doctor/permissions.rs#L255) uses that result to
decide whether to advertise and attempt the handoff.
[commit_permissions_handoff](../../lib/src/config/setup.rs#L581) validates
the block's contents, but the repository reference can fail to resolve
within repository scope after both edits. The publisher already checks
directory components below the home for its managed tree.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Permissions user scope | Separate user configs with an ordinary `.claudine`, then a linked `.claudine` with the same bytes | Both changed the Claudine file and the Darkmatter reference. The linked run edited the shared target. | Ordinary directory repairs. Linked directory is named and both files stay as found. |
| R1-F1.2 | Permissions repository scope | Same control and linked-parent edit from a repository | Ordinary repair passed. Linked repair wrote both files, then exited 1 because the new reference was broken. | Refuse before either write and keep the original inline policy. |
| Clean | Permissions file and lock checks | A direct link at `config.yaml` or its sidecar in each scope | CLI reported a manual finding, and the target bytes stayed unchanged. Darkmatter's destination and sidecar use the same check. | Manual finding and no write through the link. |
| Clean | Registration file tree and legacy tree | Same linked user `.claudine` directory | Registration reported the linked component and offered no automatic repair. The publisher walk checks parents before descendants, and legacy planning checks `.claudine` before its children. | Refuse the obstructed repair. |

Walk the config target and sidecar below their trusted scope root with the
shared link and reparse-point check. Do that when deciding eligibility and
again before the writer opens them. Check the proposed reference's scope
against the planned target before replacing the inline block. Cover the
linked-parent case with a no-write assertion for both configs and sidecars.

### R1-F2 (high) Doctor accepts changed registrations that Darkmatter refuses

**Authority (spec).** “A changed manifest that Darkmatter cannot load as
Claudine's registration is an error.” A bundle with an invalid member and an
up-to-date digest exits 0 in Doctor even though Darkmatter reports that it
did not load the registration. A valid byte-only change can also make Doctor
say both that the registration loads and that it will be refused until
republished. [Darkmatter registration](spec.md#darkmatter-registration)

**Class and invariant.** Byte integrity is being used as proof that the
bundle can be loaded. A comment or whitespace edit that keeps a consistent,
loadable bundle should warn. A single change to a member that makes
Darkmatter refuse the bundle must make the changed-manifest row an error,
even when the updated digest is correct.

I started with a bundle from `claudine registrations publish`, updated
member digests using Biscuit Hash, and inspected it with `claudine doctor
--json` and `md schema validate` on a Claudine document in the same home.
Darkmatter's diagnostic that the host “is not loaded” establishes the
refusal. The document validator's own status is separate from that warning.
Claudine's [manifest_load](../../lib/src/composition/registration/inspect.rs#L383)
stops at admission and digest checking. The report's
[disk_findings](../../lib/src/doctor/registration.rs#L90) unconditionally
adds a digest-refusal explanation to every changed member. Darkmatter's
[load_entries](../../../darkmatter/lib/src/config/registration.rs#L486)
also prepares the members and checks that the artifacts agree.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| Clean | Current inventory and changed manifest admission | Untouched bundle, manifest comment, then change host to `other` | Doctor returned `ok`, `warn`, and `error` respectively. | Same outcomes. |
| R1-F2.1 | Changed manifest with schema library | `schemas/action.yaml` with invalid YAML and updated digest | Doctor `warn`, exit 0. Darkmatter refused the bundle. | `error`, exit 1. |
| R1-F2.2 | Changed manifest with schema trigger | Same edit to `schemas/composition.yaml` | Doctor `warn`, exit 0. Darkmatter refused the bundle. | `error`, exit 1. |
| R1-F2.3 | Changed manifest with standalone YAML schema | Same edit to `schemas/formal-sequence-file.yaml` | Doctor `warn`, exit 0. Darkmatter refused the bundle. | `error`, exit 1. |
| R1-F2.4 | Changed manifest with namespace schema and dependent lifecycle artifact | Comment added to `schemas/group.yaml` and its digest updated | Doctor `warn`, exit 0. Darkmatter refused the lifecycle artifact because it was built against the original catalog. Invalid YAML in the same member also produced a false warning. | `error` in both cases. A byte edit is equivalent only if the entire bundle still loads. |
| R1-F2.5 | Changed manifest with lifecycle artifact | `globals.lifecycle.json` replaced with `{}` and digest updated | Doctor `warn`, exit 0. Darkmatter refused the lifecycle artifact. | `error`, exit 1. The same verification applies to any other declared artifact. |
| R1-F2.6 | Member warning in the report | Comment in the library, trigger, or standalone schema, or whitespace in the lifecycle artifact, with digest updated | Doctor warned and said Darkmatter both accepts and refuses the bundle. Darkmatter accepted the edited bundle. | Warn about the deviation from the shipped bytes without alleging a digest mismatch. |
| Clean | Unchanged manifest with mismatched or missing member | Existing stale- and missing-member tests | Warning for changed bytes with the old digest and error for a missing member. | Keep the specified warning and error. |

Give Doctor a read-only Darkmatter verification path for the one managed
manifest, with the same preparation and profile verdict as runtime.
Keep the no-follow entrance checks and the regular startup restriction on
loading Claudine's installed bundle. Make the source guard allow only the
explicit diagnostic call or use a shared verifier over already checked
bytes. A valid changed bundle should get a consistent warning. Cover a
digest-correct refusal using actual published bytes in the public result
and retain a comment-only acceptance control. Document the member warning
according to whether the digest actually disagrees.

### R1-F3 (high) Flat configuration entries hide genuine unknown keys

**Authority (spec and plan).** Doctor must use the validator's unknown-key
findings “including known user settings in the wrong scope” and “the same
validator for nested keys.” The plan explicitly keeps the flat-form
migration diagnostic in `config` and genuine unknown keys in
`stray-config-keys`. Upgrading an old hook or messenger configuration is
precisely when having the complete list saves another edit and run.
[Configuration files and unknown keys](spec.md#configuration-files-and-unknown-keys) · [Validation plan](plan.md)

**Class and invariant.** A migration diagnostic suppresses unrelated
unknown-property evidence at the same location. For recognized legacy
forms, changing between equivalent keyed and flat spellings should change
the migration diagnosis but preserve the verdict on a genuine typo. Adding
a typo must add a located unknown-key finding while keeping the migration
diagnosis. In Claudine, [project](../../lib/src/config/load.rs#L209)
drops every validation problem below the detected flat entry.
[visit_entries](../../lib/src/config/wire.rs#L210) finds the two affected
configuration families.

| Instance | Site | Shape tested with `doctor --json` | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | User `actions` | Keyed `bash: {command: echo hello}` and legacy `type: bash` plus `command`, with and without `timout` | Keyed typo was located. Flat control and flat typo both produced a config error and no stray keys. | Flat form keeps its migration finding. Flat typo also names `timout` at its authored location. |
| R1-F3.2 | Repository `actions` | Same four forms with a user config available | Same omission in the repository. | Same verdict and repository file location. |
| R1-F3.3 | User `messenger.configurations` | Keyed Slack route and legacy `provider: slack` route with the same channel, each with and without `timout` | Keyed typo was located. Flat typo was absent from the stray row. | Keep migration diagnosis and locate the flat typo. |
| Clean | Repository `messenger` | Same keyed and flat forms placed in the repository | All forms reported the forbidden top-level `messenger`. | The field belongs to user scope, so there is no applicable repository route validator. |
| Clean | Other nested validators feeding the stray row | Existing TTS and imported permissions controls plus `pitch` and `sockets` | The Level 1 results located both extra keys. No legacy migration branch processes either field. | Keep both locations and accepted valid controls. |

Reuse Claudine's existing keyed conversion as a validation view for known
flat entries. Preserve the original authored key positions when returning
the additional diagnostics. Keep the migration finding and distinguish it
from genuine extra keys. Unknown provider/action tags should continue
through the ordinary schema path. Wrong whole action types, mixed or
all-invalid action lists, null entries, empty lists, and a valid keyed
control already have config verdicts in the shared action-matrix tests.
The typo loss is specific to recognized flat mappings.

### R1-F4 (high) PowerShell startup writes during inspection

**Authority (spec).** “The default run does no writes,” and Windows probes
must run without startup files so each “has no side effects.” Phase 4's
native Windows run saw `AppData/Local/Microsoft/PowerShell/StartupProfileData-NonInteractive`
appear under the fixture home. The second run passed after most cases
disabled the probes and the remaining case excluded `AppData`. The
report's file-preservation promise includes a child process it starts.
[Command and report](spec.md#command-and-report) · [Executable path](spec.md#executable-path) · [Windows run](implementation-log.md#windows-cross-check-two-attempts)

**Class and invariant.** A read-only command starts a child with
write-capable startup behavior. Adding an installed shell may change the
`path` finding. It must leave the tracked startup and profile directories
unchanged. Sniff's [probe_command](../../../sniff/lib/src/programs/windows_shells.rs#L152)
is the shared subprocess builder, and Claudine's
[native_shell_lookups](../../lib/src/doctor/path.rs#L35) starts each
detected shell. PowerShell's official tests and documentation confirm the
cache location and its recreation at startup. [PowerShell path tests](https://github.com/PowerShell/PowerShell/blob/master/test/powershell/Host/Base-Directory.Tests.ps1) · [PowerShell startup documentation](https://github.com/MicrosoftDocs/PowerShell-Docs/blob/main/reference/docs-conceptual/dev-cross-plat/performance/startup-performance.md)

| Instance | Site | Shape checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F4.1 | PowerShell 7 and Windows PowerShell | Native fixture with fresh local app data, including fallback command construction | The recorded native run created the cache. Both PowerShell variants take the same branch with only `-NoProfile` and `-NonInteractive` to control startup. The record does not identify a separate 5.1 run. | Preserve startup directories for each variant while still issuing its bounded lookup. |
| Clean for this cache mechanism | Git Bash, MSYS2, Cygwin | Shared Bash command with `--noprofile --norc -p`, and `BASH_ENV` and `ENV` removed | The Sniff argument tests attest to that branch. Phase 4 attributed the observed write to PowerShell. | Keep startup files disabled. |
| Clean | Process PATH lookup and absent-shell branch | Focused PATH fixtures and Linux CLI | Lookup scans the executable index without launching `claudine`. An absent shell yields no child. | No startup effects. |

Prevent cache writes in the PowerShell probe's child environment while
retaining the inherited executable search path. Verify the chosen
mechanism against both variants. Keep a Windows proof that runs an actual
PowerShell probe against fresh profile and cache state, checks the lookup
result, and compares the directories without exclusions. The subprocess
fixtures can still cover the other outcome branches without running a real
shell for every case.

### R1-F5 (high) Failed cleanup can hide files already removed

**Authority (spec).** “A failed repair is an `error`, with the reason and
any changes already made.” I ran `doctor --fix --json` as the owner of
the fixture, with a writable format 1 revisions tree beneath a read-only
`.claudine` directory. Doctor deleted the tree's child files and folders,
failed to remove its root, and printed “No change was made before the
failure.” [Command and report](spec.md#command-and-report)

**Class and invariant.** A fallible batch returns its completed operations
only on full success, so the caller discards evidence of partial mutation.
With the normal permissions control all removals should be reported as
success. If a later removal fails, earlier removals must still appear in
the failure detail and a retry must handle only remaining paths.

| Instance | Site | Shape checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F5.1 | Retired tree and manifest cleanup | Same published fixture first with a read-only parent, then made writable | First run removed descendants but reported no changes. A subsequent writable run removed both remaining legacy paths and reported them. The legacy function also discards its local result if removing the manifest fails after the tree is gone. | Carry removals across either failure and report only remaining work on retry. |
| R1-F5.2 | Publisher's leftover temporary-file cleanup | Successful earlier removal with a later directory-entry or removal error in the same batch | Source inspection shows `remove_temps` returns the batch vector only on success, and its caller extends the journal only after `?`. The same loss follows from that return contract. | Record each successful deletion in the repair's journal as it happens. |
| R1-F5.3 | Publisher retirement of old member directories | Prior manifest names a retired member whose parent becomes empty | A later legacy cleanup failure reported the retired file and manifest replacement. The two parent folders were gone and did not appear in the journal. The later extras scan cannot record folders already removed. | Include the removed parent directories if a later operation fails. |
| Clean | Publisher member replacement | Injected stop after the first member write in the existing repair tests | The journal is updated before the observer can stop the publish. The first write reaches the public repair status. | Keep the recorded write when a later operation fails. |
| Clean | Doctor extras and permissions transaction | `remove_extras` appends immediately after each successful removal. The existing permissions interruption test fails after the first replacement. | Extras preserve earlier journal entries by construction. Permissions reports the first replacement and leaves the inline block in effect. | Retain earlier changes under either failure. |

Use the existing operation journal for removals before any later error
can return, including legacy cleanup. Keep successful reporting of the two
legacy roots if desired. A failure that leaves a partly removed directory
needs the completed removals or a clear report that the tree was partially
removed. [Legacy cleanup in Claudine](../../lib/src/composition/registration/legacy.rs#L70) · [Publisher temp cleanup](../../lib/src/composition/registration.rs#L506) · [Doctor repair journal](../../lib/src/composition/registration/repair.rs#L132)

### R1-F6 (high) CLI validation and repair-format tests miss required boundaries

**Authority (spec and plan).** The spec requires Level 1 proof using the
Claudine process fixture, including unknown-key locations and both formats
for warnings, errors, successful repairs, and failed repairs. The plan's
process-proof section calls for an unmodified control, duplicate and nested
key cases with coordinates, and one changed v2 manifest refusal using a
published fixture. These requirements need the shipped command because
the process can report a different status or text from the library.
[Tests](spec.md#tests) · [Input robustness matrix](plan.md#input-robustness-matrix)

**Class and invariant.** Some acceptance cases stop testing just before
the process boundary or test only one of the two output formats. A fixture
edit that changes validity or the outcome of repair must appear in the
report with the same status, location when available, and disk effects
regardless of format. The source inventory below checks the declared
Level 1 integration target. Focused results confirmed that the target and
doctor module run. [CLI process tests](../../cli/tests/l1/doctor.rs) · [Level 1 module list](../../cli/tests/l1/main.rs)

| Instance | Site | Shape and strongest existing proof | Observed coverage | Expected |
| --- | --- | --- | --- | --- |
| R1-F6.1 | YAML loader through Doctor's CLI | Valid home, duplicate key, nested stray key, malformed YAML | Healthy and malformed cases spawn the CLI. Duplicate and nested locations are pinned in the library only. The CLI invalid-value redaction case checks secrecy and severity, without pinning coordinates. | Spawn the duplicate and nested cases, asserting filename, line and column, and preserved files. |
| R1-F6.2 | Registration refusal through Doctor's CLI | Shipped inventory as control, then changed manifest or a digest-correct invalid member | The CLI publishes healthy bytes and proves repair from an absent tree. Tests over changed manifests stay in the library, with no process assertion on refusal or exit status. The direct CLI probe in R1-F2 reproduces why this boundary matters. | One live changed-bundle refusal case with unchanged files and exit 1. Reuse the R1-F2 fixture. |
| R1-F6.3 | Human reports for unresolved, repaired, and failed states | Unresolved errors, both repairs successful, held-lock failure | The integration tests assert these states in JSON. Renderer unit tests use synthetic rows, and the spawned successful human report case covers only warnings. | Assert the corresponding human row text and exit status through the CLI. For failed repair, keep a recovery scenario with a bounded wait. |
| R1-F6.4 | Shared Darkmatter permission-rule array validator | Omitted, null, empty, valid rules, wrong whole type, mixed elements, all-invalid elements | Existing Darkmatter and Doctor tests exercise the default and empty cases, valid and mixed rules, and wrong whole type. I found no automated public-result assertion for an all-invalid array through this validator. A direct CLI run on `[3, true]` located both invalid elements, so this is a test gap. | Prove the all-invalid array yields a located error instead of an empty command grant. Keep this at the shared in-process boundary. |
| Clean | Warnings and report routing | Format 1 leftover, forced-color redirect, plain mode, and stdout write refusal | Both output formats are checked for the warning-only run. The CLI also checks stdout write failure on Linux. | Keep current evidence. |

Use the existing process fixture and add only the missing representative
wiring cases. Leave exhaustive shapes in the library. A plain report
assertion can check stable text such as `Repaired:` and `Repair failed:`
without pinning wrap positions or platform path separators.

### R1-F7 (medium) Case-only aliases can be deleted as registration extras

**Authority (spec).** Doctor must compare “every published artifact
against the inventory” and may “delete extras” inside the managed
directory. A backup restore or manual rename can change only the case of
a managed filename. On a case-insensitive volume, the restored file
still satisfies the path named by the manifest. Its directory entry
must not be removed as an extra. [Darkmatter registration](spec.md#darkmatter-registration)

**Class and invariant.** The same disk entry is identified by filesystem
lookup in one decision and by case-sensitive string comparison in the
next. If the manifest's spelling and the scanned spelling resolve to
the same entry, a case-only change with identical bytes must never
create a deletion candidate. On a case-sensitive volume the alternate
entry is distinct, so repair can install the missing expected file and
remove the alternate one.

| Instance | Site | Shape checked | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F7.1 | Expected file vs. extra-file scan | `schemas/action.yaml` renamed to `schemas/Action.yaml` | Linux control on a case-sensitive volume repaired the missing expected file and removed the extra. On an insensitive volume the expected path still reads as current, but `scan` rejects the actual spelling as an extra, and `remove_extras` deletes that entry before verification. This outcome follows from the two comparisons in the source. | Resolve both spellings to the same directory entry on an insensitive volume and keep it. Preserve the Linux control. |
| R1-F7.2 | Expected folder vs. recursive extras | Same case-only change to `schemas/` | On Linux the repair recreated the expected directory and removed the alternate one. The expected-path walk still opens the folder on an insensitive volume. `scan` uses the differently cased prefix for all children, so it proposes deleting the folder and managed children. | Treat the corresponding subtree as expected. |
| R1-F7.3 | Publisher retirement of old member paths | Previous manifest and its digest map use the alternate case for the same member | Linux installed the expected member and removed the distinct retired path. The retirement planner tests the old spelling against a case-sensitive set before opening it. On an insensitive volume it can retire the still-current member before the extras scan runs. | Use the same on-disk membership decision for the retirement set. |
| Clean | Publisher replacement and final verification | Lookup by expected path | Both use the actual filesystem result. The publisher skips a current file, and final verification will detect the missing file if the extras step removed it. Neither chooses the deletion candidate. | Keep filesystem lookup and prevent the wrong deletion upstream. |

Make managed membership and extra classification agree about the
directory entry reached by each expected path. Preserve distinct names
on case-sensitive volumes and check for links before any identity lookup
that could follow them. Add a case-only rename test on a case-insensitive
volume and keep the distinct-name control. [Expected files and extra scan in Claudine](../../lib/src/composition/registration/inspect.rs#L141) · [Removal decision](../../lib/src/composition/registration/repair.rs#L234)

### R1-F8 (low) Configuration remedies promise behavior from commands that bypass configuration

**Authority (defect).** The Claudine configuration check advises “run any
other `claudine` command” to convert legacy JSON, and promises that “any
other” command prints full validation errors. In temporary homes,
`claudine registrations` returned 0 without converting the legacy file
or printing the error in an invalid YAML file. Help and completion also
bypass configuration by design. Give a specific command path confirmed
to load user configuration, and keep `claudine setup` as the supported
remedy. Update the command reference and any matching guidance.
[Config finding wording](../../lib/src/doctor/config.rs#L365) · [Early dispatch](../../cli/src/main.rs#L535) · [Doctor reference](../../docs/cli/doctor.md#config-configuration-files)

## Verification and input robustness

The existing Level 1 tests suffice for this command's non-interactive
interfaces. It does not depend on terminal keyboard encoding or on a
real terminal to verify its status and text. The library and CLI tests
plus direct Linux CLI probes covered the following boundaries. Native
Windows cache evidence comes from the implementation run.

| Requirement | Verification level and evidence |
| --- | --- |
| Grouped help, completion, JSON layout, exit statuses and stdout routing | Level 1 process coverage. Healthy, warning, invalid usage, unresolved errors, successful repairs, and held lock run through the binary. Human repair and loader wiring gaps are listed in R1-F6. |
| Passive startup and unrelated checks | Level 1 early-dispatch checks for missing and broken homes, legacy preservation, unchanged registration, and no audio spool. Native PowerShell write is R1-F4. |
| Config scope, effective settings, and located/redacted errors | Level 1 library matrices and CLI redaction. Files can be optional, aliases are observed once, and a valid merged pair can supersede a user-only problem. R1-F3 covers the missing nested inference. |
| Registration and permissions | Level 1 temporary-file and simulated interruption tests plus direct process probes. The failures are listed in R1-F1, R1-F2, R1-F5, and R1-F7. |
| Speech and executable discovery | Level 1 injected provider roster and executable index tests. Disabled, automatic, pinned, and unknown-provider choices and severity combinations are represented. Sniff's fixtures cover shell family discovery, missing, crash/spawn error, and timeout. The CLI's own PATH result has present and absent controls. R1-F4 covers the startup write. |

The focused Linux runs passed 101 Doctor library tests, 13 Doctor CLI tests,
9 registration repair tests, 1 spawned completion test, 25 Sniff Windows
shell fixture tests, and 10 union-diagnostic tests. The SimplifiedSchema
recipe also ran its 2,172-test minimal-feature suite. None of these tests
reached five seconds.

The config default is a located error for an inadmissible value or
document; inspection does not launch work or publish files. Exceptions
include optional absent repository files, nullable speech and override
settings, default permissions, and absent/null/empty registration lists.
Claudine and Darkmatter use the shared YAML document reader with their own
scoped validation. Library tests exercise duplicate keys, bad YAML,
multiple documents, non-string keys, and accepted BOM, comments, and
explicit null. Legacy JSON is existence-only in Doctor. The registration
manifest has its own admission path. R1-F2 covers the member validation
Doctor omits after admission.

| Load-bearing projection | Existing controls and invalid cells |
| --- | --- |
| Claudine action and route entries | Valid keyed controls, null event handling, empty event list rejection, mixed and all-invalid action element types, and scope rejection for repository messenger settings. The nested unknown-key sweep in R1-F3 exercises both supported action scopes and the one supported route scope. |
| Claudine TTS and repository overrides | Absent, null, true, false, empty mapping, scalar and array type errors. TTS provider separately has absent/null automatic, recognized/unrecognized strings, empty string and wrong-type cases. Repository override tests distinguish inherited null and invalid empty mappings. |
| Darkmatter permissions | Omitted/null/default and empty mapping controls, invalid numbers and blank references, mixed shell rule elements, valid empty rules, scope-sensitive file globs, and valid and invalid network settings. The referenced target cases cover missing, null, and nonmapping blocks. An empty `file-read` list follows the existing scope schema. R1-F6.4 covers the missing all-invalid rules test. |
| Darkmatter register entries | Omitted/null/empty and other-host controls; wrong field type, mixed/all-invalid elements, and local-reference syntax errors. Invalid entries prevent registration repair. Equivalent spellings are accepted by the identity check. |

I checked the new validation paths for `#[serde(default)]`, nullable
projection, `filter_map`, `unwrap_or_default()`, and `.ok()`. The shell
probe's `.ok()?` can only discard a rejected program-name argument
and Doctor passes the fixed valid name. Failed launches and timeouts are
explicit outcomes. The changed-manifest inspection's early admission
verdict and the flat-config filter are accounted for in the findings.

## Implementation decisions (informational)

I compared two independent inventories of the plan, spec, and implementation
log. The plan allowed unattended implementation choices within the
specified behavior. The spec fixes the two repair scopes, the ordering
and verification of writes, and the passive default. The decisions below
need no author ruling.

- **Startup and registry.** The implementer wired completed callbacks as they
  became available and temporarily stubbed the rest. Later phases removed
  all stubs. The request shares one resolution context, uses a generic
  context failure message, rereads fresh observations for each repair, and
  rechecks affected rows. Library file-test registries avoid host probes.
- **Configuration.** A missing user Darkmatter file is an error with a setup
  remedy. A present repository file is reported as present while its
  validation errors belong to `config`. The passive Darkmatter API reuses
  the read-error category for a missing home and retains the I/O error where
  setup needs it. Merged validation attributes a failure to the responsible
  file or pair. The implementation keeps the actual scoped
  `file-read: []` behavior. Darkmatter's existing unknown-key location
  convention is kept for its own files. Claudine locates its unknown keys
  at their key tokens and orders them by source location.
- **Schema diagnostics.** A union reports a key only if every applicable
  object arm rejects it. This exposes the key under the editor's existing
  unknown-key policy, which warns in default mode and errors in strict
  mode. I confirmed that `md schema validate` still exits 1 for the
  unknown lifecycle verb. `md set` now rejects a write at a field the
  object union does not declare. A comment-preservation fixture switched
  to a declared field. The unresolved legacy-form suppression belongs
  to R1-F3.
- **Repair boundaries.** The registration repair refuses to edit any
  user file whose `register` entries are structurally invalid, including
  malformed entries for another host. The writer preserves valid entries
  for other hosts. Both repairs refuse linked files and lock sidecars.
  The linked-parent gap is R1-F1. The publisher lock spans setup's publish
  and finalization, while the file transaction checks observed bytes.
  Strategy differs by mode for null and scope inference to preserve
  existing setup behavior. A concurrently absent legacy entry is treated
  as already removed. Separate library tests with a combined CLI repair
  proof keep the test fixtures focused. The partial manifest check and its
  guard rationale are addressed in R1-F2.
- **Windows shell lookup and speech.** Sniff accepts a validated plain
  executable name and the CLI passes `claudine`. Discovery chooses one
  preferred executable per family, using the existing index and the
  Git Bash install marker. Successful lookups appear in details, ordered
  by executable path. Linux, macOS, and WSL2 do no alternate-shell
  probing. Tests normalize paths to native separators and inject shell
  outcomes without requiring installed software. Most CLI fixtures skip
  ambient shells for isolation and speed. The exclusion from the one
  real Windows fixture is addressed in R1-F4. Speech ties use runtime
  order, and pinned providers receive targeted advice and the local
  prerequisites caveat.
- **Report and help.** The renderer uses severity text, a bulleted report
  and tally, plain output on redirects even with `FORCE_COLOR`, and a
  direct stdout write so failures propagate. Control characters are
  normalized, Prose markup is escaped, raw validator messages are
  withheld, and user-selected diagnostic path segments are masked.
  Standard wrapping can break at path hyphens. JSON preserves the text
  for scripts. Administration help places the command after setup.
  The configuration remedy needs the correction in R1-F8.
- **Verification and documentation.** The Windows-specific changes
  prompted a focused native cross-check beyond the plan's local
  verification. The held-lock CLI test uses the production deadline,
  skips reading the held sidecar, and the Linux stdout test uses
  `/dev/full`. The focused nextest filters and ordinary CI supply
  remaining OS evidence. Claudine's caller skill summarizes the
  command reference, and Phase 5 records its publication.
- **Deferred work.** CLI wiring and process proof, combined repair
  testing, Prose escaping, documentation, and caller skill publication
  moved to later phases and landed there. Full manifest preparation,
  unknown-key validation for flat forms, and startup cache preservation
  were left unresolved and are findings above. The implementer retained
  standard path wrapping and setup's existing scope behavior. The
  unrelated live Kache test is outside the Level 1 run and the reviewed
  change.

The implementation log now describes the scope of the partial manifest
check, the linked-parent limitation and the native Windows fixture
exclusion. It also records the direct check of the configuration remedy.
[Implementation log](implementation-log.md)
