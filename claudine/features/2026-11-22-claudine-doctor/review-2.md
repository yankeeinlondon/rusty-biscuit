---
$schema: feature-review.yaml
ready: false
findings:
  - "R2-F1 (high): A failed user permissions repair skips the repository repair"
  - "R2-F3 (high): Merged configuration errors can name the wrong file"
  - "R2-F4 (high): The speech report confuses highest quality with runtime selection"
  - "R2-F5 (high): Some remedies cannot act on the reported problem"
  - "R2-F7 (high): Permissions repair is offered for an invalid Darkmatter file"
  - "R2-F8 (high): Darkmatter suggests user initialization for broken repository references"
  - "R2-F2 (medium): Doctor refuses loadable manifest member references"
  - "R2-F6 (medium): Flat entries hide independent configuration diagnostics"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-24T05:16:05+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-2.md
previous: 2026-11-22-claudine-doctor/review-1.md
next: 2026-11-22-claudine-doctor/review-3.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/log.md
implemented-by: claude/opus
---

# Claudine Doctor review 2

**Production readiness declined.** A user file lock can prevent a separate
repository repair. Doctor can also reject a bundle Darkmatter loads, point
to the wrong configuration file, and recommend commands that cannot help
with the reported problem. The original repair cases pass. The open findings
below cover additional cases and have repairs within the existing contract.

## Previous Finding Verification

The child IDs come from the repair's [Finding Ledger](log.md#finding-ledger).
Review 1 has no human rulings. The two informational decisions about setup's
linked-folder behavior and temporary PowerShell cache isolation were kept.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 Permissions repair follows linked configuration directories | R1-F1.1, .2, .c1, .c2 | verified fixed | The focused run passed user and repository parent-link tests, the post-planning replacement test, the wrong-target check, and direct links at both files and sidecars. The registration and retired-tree link control also passed. The permissions writer repeats the route check before opening its transaction. [Claudine permissions tests](../../lib/src/doctor/permissions_tests.rs) · [Claudine writer](../../lib/src/config/setup.rs#L644) | None in the original cases. |
| R1-F2 Doctor accepts changed registrations that Darkmatter refuses | R1-F2.1–.6, .c1, .c2 | verified fixed | All five invalid member or artifact checks passed, including the namespace comment that invalidates the lifecycle artifact. All four loadable-byte controls warn without alleging mismatched digests. Untouched, commented, wrong-host, stale, and missing controls passed. The changed-bundle refusal reaches both CLI formats. Darkmatter's two read-byte parity tests passed. [Claudine registration tests](../../lib/src/doctor/registration_tests.rs) · [Darkmatter snapshot tests](../../../darkmatter/lib/tests/l1/host_registration_snapshot.rs#L248) | New sibling R2-F2 covers an earlier admission step. |
| R1-F3 Flat configuration entries hide genuine unknown keys | R1-F3.1–.3, .c1, .c2 | verified fixed | The focused run exercised keyed and flat user actions, repository actions, and a user messenger route, with and without a typo. The authored typo coordinates survive. The repository-only restriction on messenger settings and nested TTS and permissions key checks passed. [Claudine config tests](../../lib/src/doctor/config_tests.rs) | New sibling R2-F6 covers the independent document-level checks. |
| R1-F4 PowerShell startup writes during inspection | R1-F4.1, .c1, .c2 | verified fixed | Both PowerShell variants use the same temporary-profile and module-cache environment, with no startup scripts. I checked the native-Windows test's hit, miss, and whole-tree assertions against the recorded runs on PowerShell 7 and 5.1. The CLI healthy-home fixture now compares the whole tree. Local Sniff shell fixtures and Claudine PATH checks passed. [Sniff shell probes](../../../sniff/lib/src/programs/windows_shells.rs#L140) · [Native test](../../../sniff/lib/src/programs/windows_shells_tests.rs) · [Windows run record](log.md#progress) | None. Native execution evidence comes from the repair run. |
| R1-F5 Failed cleanup can hide files already removed | R1-F5.1–.3, .c1, .c2 | verified fixed | The focused run passed partial retired-tree and manifest removals, retry, the read-only parent case, temporary-file failure, and empty-parent retirement. Setup's process test shows prior removals. The publisher's stopped-write case and permissions interruption cases retain completed operations. The extras loop records each completed removal before the next fallible operation. [Claudine publisher tests](../../lib/src/composition/registration/repair_tests.rs) · [Claudine repair](../../lib/src/composition/registration/repair.rs) | None in the original cases. |
| R1-F6 CLI validation and repair-format tests miss required boundaries | R1-F6.1–.4, .c1 | verified fixed | The declared CLI target passed duplicate and nested key coordinates, changed-bundle refusal, and unchanged-file checks in both formats. Human output now exercises unresolved findings, a failed held-lock repair, and its successful retry. The Darkmatter public-result test located both all-invalid rule elements. Warning, plain, forced-color redirect, and stdout-failure controls passed. [Claudine CLI tests](../../cli/tests/l1/doctor.rs) · [Darkmatter config tests](../../../darkmatter/lib/tests/l1/config_fields.rs) | None. The output-format tests use a representative repair for each state. |
| R1-F7 Case-only aliases can be deleted as registration extras | R1-F7.1–.3, .c1 | verified fixed | File and folder case-only variants, previous manifest spelling, alternate digest spelling, and legacy lookup passed on the Linux case-sensitive branch. I checked the native-Windows test conditions against the recorded insensitive-volume runs. Extras and retirement compare directory entries, while replacement and verification use filesystem lookup. [Claudine publisher tests](../../lib/src/composition/registration/repair_tests.rs#L441) · [Directory entry comparison](../../lib/src/composition/registration/entries.rs#L62) · [Windows run record](log.md#progress) | None. Native execution evidence comes from the repair run. |
| R1-F8 Configuration remedies promise behavior from commands that bypass configuration | R1-F8.1, .2 | verified fixed | The original user and repository YAML errors reach the named commands, including a repository error before a test provider starts. User `setup` and a prompt-bearing repository `claudine claude` convert the corresponding legacy file after Doctor leaves it in place. Both CLI and library cases passed, including home/repository legacy identity. [Claudine CLI tests](../../cli/tests/l1/doctor.rs#L306) · [Claudine config tests](../../lib/src/doctor/config_tests.rs#L129) | New sibling R2-F5 covers other failure categories and scopes. |

## Unblocked Findings

### R2-F1 (high) A failed user permissions repair skips the repository repair

**Authority (spec).** “Continue independent checks after a failure” and
“unrelated findings do not block a repair.” In a temporary repository with
both handoffs eligible, holding only the user Darkmatter writer lock made
`claudine doctor --fix --json` leave the independent repository policy
inline. The report still called the repository repair available.
[Command and report](spec.md#command-and-report)

The defect class is early termination of an independent repair batch. A
failure must preserve the affected scope's policy and reporting while the
other scope converges. The permissions callback in Claudine returns at its
first failed commit. Claudine's top-level registry already continues across
separate repair callbacks. [Claudine permissions repair](../../lib/src/doctor/permissions.rs#L46) · [Claudine registry runner](../../lib/src/doctor/mod.rs#L243)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F1.1 | User then repository permissions handoff | Two valid inline policies with separate config files. Hold the user writer lock across `--fix`. | Exit 1. User lock failure reported. Both policies stay inline. With the lock released both are repaired. | Exit 1. Keep user policy inline and report its failure. Complete and report the repository repair in the same invocation. |
| Clean | Repository failure after user handoff | Move the lock to the repository writer. | Exit 1. User handoff completes and its two changes are listed. Repository inline policy stays. Retry completes the repository. | Preserve this behavior. |
| Clean | Both scopes available | Remove the lock from the same fixture. | Exit 0, four replacements reported, both references resolve. | Same. |
| Clean | Registration callback and permissions callback | Hold the user Darkmatter lock with registration and repository handoff eligible. Verify alongside the shared registry test and combined success fixture. | Registration fails at the writer lock. The repository handoff still finishes, and its two writes are reported. | Continue across callbacks as well as independent scopes. |
| Clean | Two ordered writes within one handoff or publication | Existing write interruption tests. | A failure stops dependent writes and retains the inline policy or retired files at the documented boundary. | Keep this ordering. |

Accumulate scope-specific failure reasons in the permissions callback,
continue to the other eligible scope, and preserve all completed changes.
Keep one stable report row. Add a short-deadline Level 1 case with two
scope pairs and a failure in the first.

### R2-F2 (medium) Doctor refuses loadable manifest member references

**Authority (spec).** Changed managed bytes are warnings, while “a
changed manifest that Darkmatter cannot load as Claudine's registration is
an error.” In a published bundle, replacing a member's `./schemas/…`
reference and digest key with the equivalent `../darkmatter/schemas/…`
made Doctor exit 1. Darkmatter loaded the same bundle.
[Darkmatter registration](spec.md#darkmatter-registration)

The defect class is disagreement between the diagnostic and runtime admission
rules. An alternate reference that resolves to the exact same in-tree member
and keeps its digest must preserve the load verdict. A reference resolving
outside the tree must be refused. I changed only the specified reference and
its digest key, keeping member bytes and digest values unchanged. I checked
the registration row and Darkmatter's host-registration diagnostic using
`md schema validate` with the same home. Darkmatter's exit status describes
document validation. The host refusal is a separate diagnostic.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Schema library | `action.yaml`, with the canonical reference, then `../darkmatter/` and `../` prefixes | Canonical plus manifest comment warns. Re-entry gives Doctor error and Darkmatter acceptance. Actual escape gives errors in both registration verdicts. | Warning on either in-tree spelling, error on escape. |
| R2-F2.2 | Schema trigger | Same edits to `composition.yaml` | Same false error on re-entry. Both controls agree. | Same. |
| R2-F2.3 | Standalone YAML kind | Same edits to `formal-sequence-file.yaml` | Same false error on re-entry. Both controls agree. | Same. |
| R2-F2.4 | Namespace schema | Same edits to `group.yaml` with its original bytes and lifecycle artifact | Same false error on re-entry. Both controls agree. | Same. |
| R2-F2.5 | Lifecycle artifact and optional function pair | Same edits to `globals.lifecycle.json`. Claudine ships no function pairs. Darkmatter's shared member loop applies the same path resolver to lifecycle, function catalogs, and function facts. | Same false error on the lifecycle member. The optional pair goes through the same early lexical filter before the shared verifier. | Same in-tree and out-of-tree verdict for every member category. |
| R2-F2.6 | Digest-note attribution after admission | Add a comment to the action schema without updating its digest. Compare canonical and re-entry spellings in the edited manifest. | Both Doctor runs fail, but only the canonical spelling reports the actual digest mismatch. Lexical admission prevents the other run from reaching the comparison. | Diagnose the digest mismatch using the same confined member resolution for both spellings. |
| Clean | Config `register` identity and post-publish verification | Existing equivalent config reference tests and repair success controls. | The config entry uses Darkmatter's identity check. Post-publish verification compares against the shipped inventory that was just written. | Keep both decisions. |
| Clean | Retirement | Read the previous inventory for removal using the stricter lexical admission step. | An ambiguous or refused old manifest contributes no retirement list. Doctor's extras pass separately uses the checked managed tree. | Keep the conservative retirement policy. |

Use Darkmatter's installed-member resolution to obtain the confined member
locations before Doctor's regular-file and no-follow walk. Reuse those
locations for digest and profile judgment. A small shared resolution seam
would preserve Doctor's ownership of verified reads. Keep the lexical
restriction for embedded and publisher use. Update the Claudine repo
skill's stale description of `inspect_installation(…, Some(context))`.
Current code calls `Installation::judge_manifest` separately.
[Claudine inspection](../../lib/src/composition/registration/inspect.rs#L467) · [Darkmatter manifest resolution](../../../darkmatter/lib/src/config/registration/manifest.rs#L235) · [Claudine skill](../../../.claude/skills/claudine/SKILL.md#L185)

### R2-F3 (high) Merged configuration errors can name the wrong file

**Authority (plan).** The semantic check must “report a semantic failure
with the responsible file, or name both files for an interaction.” A user
mapper with a bad regular expression and a different bad mapper authored by
the repository produced a diagnosis naming the repository event under the
user file. Editing that file cannot fix the reported event.
[Semantic validation plan](plan.md#necessary-rules)

The defect class is loss of a setting's scope during merge and reporting.
An overridden setting should follow the authored source of its effective
value. A user-only validation failure may still affect a check that uses
the user file alone, even when the merged configuration is valid.
Claudine's `typed` reports a merged failure under the user file whenever
any user-only validation fails. Its speech and handoff findings also refer
readers back to `config` in cases where the plan intentionally keeps that
row clear. [Claudine config projection](../../lib/src/doctor/config.rs#L236) · [Speech check](../../lib/src/doctor/tts.rs#L41) · [Permissions findings](../../lib/src/doctor/permissions.rs#L237)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F3.1 | Effective messenger selection | A valid route named `team`. User selects a missing route. Repo first selects `team`, then a different missing route. | With valid override `config` passes. With bad override it reports only the user file and suggests `actions`, which reads the different user selection. | Keep the valid merged control. For a failing override, identify the repository selection and the user route catalog it needs. |
| R2-F3.2 | Effective action mapper | Bad user `before_tool` mapper, replace it with a valid repository action and add a bad repository `after_tool` mapper. | Doctor reports `after_tool` in the user file, which has no such action. | Name the repository file. |
| R2-F3.5 | Effective exit expressions | User has a bad regex. Repository replaces that rule with a valid pattern, then with a pattern that names an unknown agent in its scope. | Valid replacement passes. Invalid replacement points to the user file, which holds an unrelated error. A valid-user control attributes a bad repository rule to the repository. | Name the file whose effective exit-expression setting failed validation. |
| R2-F3.3 | Speech prerequisite | Same missing user route with a valid repository override. Compare user-only and in-repository reports. | Both speech rows say `config` explains the refusal. Inside the repository `config` says all files are valid. | Keep the speech row informational and explain the user-only reason there when it is absent from `config`. |
| R2-F3.4 | Handoff prerequisite | Same repaired repository selection, plus inline user Darkmatter permissions awaiting handoff. | Handoff correctly refuses to edit the invalid user config but points to a healthy `config` row for the explanation. | Keep the repair refusal and explain which user-only setting blocks it. |
| Clean | Repository scope with an otherwise valid user config | Existing missing repository route test and direct control with a valid `team` selection. | Both files are named for the failing interaction. Valid selection passes. | Keep both results. |
| Clean | User-only semantic settings | The validator checks Protect, default sounds, and the route definitions in user configuration. Repository merge can select a route, but cannot replace these settings or definitions. | The source of these settings stays the user file across merge. | Keep their user attribution. |
| Clean | Schema-invalid files and stray-key projection | Existing invalid repo and nested typo cases. | Scope is assigned before merge and the reports name the authored file. | Keep file identity at this boundary. |

Use the source of the effective setting, or report both files when the
failure needs both. For action mappers the event selects the source,
because the repository replaces that event's whole action list. Keep the
plan's valid-merge behavior. Give the user-only consumers the sanitized
reason they already rely on instead of pointing to a clear row.

### R2-F4 (high) The speech report confuses highest quality with runtime selection

**Authority (spec).** For automatic selection Doctor must “report the
highest-rated available candidate and identify the first candidate in
runtime order if it differs.” With `TTS_PROVIDER=espeak` and a discoverable
Piper executable, Doctor says “hook speech uses Piper” and then says it
tries eSpeak NG first. Changing an environment preference should change
which provider the report identifies as first without changing the quality
ratings. [Text-to-speech](spec.md#text-to-speech)

The defect class is reporting a quality ranking as a runtime choice.
The Claudine speech check enters its good-quality branch for the best
candidate and shares the same positive wording between automatic and
pinned modes. The runtime order is the slice from Biscuit Speaks.
[Claudine speech check](../../lib/src/doctor/tts.rs#L72) · [Biscuit Speaks detection](../../../biscuit-speaks/lib/src/detection.rs#L54)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F4.1 | Enabled automatic, Good or higher | Local executable fixture for Piper and installed eSpeak NG. Change `TTS_PROVIDER` from `piper` to `espeak`. | Both summaries say speech uses Piper. With eSpeak first the detail contradicts that selection claim. | Identify Piper as best available, and eSpeak NG as first when preferred. |
| R2-F4.2 | Unrecognized configured provider | Add an unknown `tts.provider` to the eSpeak-first fixture. | Warning and automatic fallback are correct. Detail repeats the same contradictory Piper selection. | Keep the warning and distinguish best available from first in the fallback detail. |
| Clean | Enabled automatic below Good | Remove Piper, expose Mimic 3, and keep eSpeak first. | Warns that Mimic 3 is the best available and names eSpeak NG first. | Keep both descriptions. |
| Clean | Pinned provider | Pin Piper while `TTS_PROVIDER=espeak`. | Names only Piper. The missing-pin test keeps the error even when other engines exist. | Preserve pinning. |
| Clean | Disabled and no-candidate branches | Disable speech with both candidates, and check the existing absent-candidate tests. | Disabled wording describes availability. No-candidate cases make no provider selection claim. | Keep their severity and availability wording. |

Describe the automatic branch using “best available” and “tries first.”
Update the Level 1 ordered-candidate test that currently requires “uses
Kokoro” when eSpeak is first. These probes invoked Doctor only. The fake
provider executables did not synthesize speech.

### R2-F5 (high) Some remedies cannot act on the reported problem

**Authority (defect).** Doctor tells a user whose hook mapper has a bad
regular expression that `claudine actions` prints the full error. It exits
0 without the error. Two permissions remedies similarly name user-only
commands for repository problems. A remedy that leaves the problem in
place sends the user back through the same repair cycle.
[Claudine config wording](../../lib/src/doctor/config.rs#L318) · [Claudine permissions wording](../../lib/src/doctor/permissions.rs#L130)

The defect class is a remedy claiming behavior outside the target command's
validation or edit scope. A semantically equivalent edit should keep the
remedy effective. Changing the issue category or moving it to repository
scope must select a command that handles that category or scope.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F5.1 | User semantic config remedy | Invalid mapper pattern `(` versus an unknown default sound. | `actions` prints the sound failure and exits 1. For the mapper it exits 0 without the promised diagnostic. | Keep the sound remedy. For a mapper, name the event, action, and setting to correct and omit the unverified command. |
| R2-F5.2 | Darkmatter config problem remedy | `register: 3` versus invalid YAML, invalid inline permissions, and a broken permission reference. | The recommended `md compose` with a shell document prints the latter errors. With invalid `register` it reaches shell approval or runs the explicitly allowed command without printing the registration problem. | For a register finding, give a correction using Doctor's existing location. |
| R2-F5.3 | Repairable repository handoff | Inline user and repository permissions cases. Run the recommended `claudine setup` from the repository. | Setup completes the user handoff. It exits 0 and leaves repository permissions inline. | Keep setup for user scope. Offer Doctor's fix for the repository. |
| R2-F5.4 | Broken repository permissions reference | Run `md init --inline` against the same missing reference in each scope. | User reference replaced. Repository file unchanged, exit 0. The repository remedy also risks an unrelated user edit. | Recommend correcting the repository reference or its target. Any optional initializer guidance should match its full write scope. |
| Clean | Syntax, legacy, and missing-file remedies | The repair's original CLI cases plus the valid control homes. | The named Claudine commands handle these shapes in their intended scope. | Preserve these remedies. |
| Clean | Manual permissions conflicts and mapper errors attributed only to a repository merge | Review the decision sites issuing a direct edit instruction. | Their generic correction guidance does not require an unrelated command. | Keep guidance scoped to what the user can correct. |

Choose the remedy by issue category and scope. Identify Doctor's own
evidence where a suitable scoped command is unavailable. Correct the
Doctor reference's assertion that Doctor reads no configuration and the
matching source comment. The early dispatch bypasses startup loading and
still reads the files it diagnoses.

### R2-F7 (high) Permissions repair is offered for an invalid Darkmatter file

**Authority (spec).** “The repair operates on existing valid files,” and
`fixable` means an automatic repair is available. With inline permissions
and `register: 3`, Doctor advertises a handoff that Darkmatter's writer
rejects on every retry. [Permissions handoff](spec.md#darkmatter-permissions-handoff) · [Report contract](spec.md#command-and-report)

The defect class is a repair preflight accepting a file that its writer
deterministically refuses. Omitted, null, and empty registration settings
can still allow the handoff. Adding a malformed entry must make an affected
handoff manual before the writer opens. Claudine's permission prerequisites
treat a readable Darkmatter observation as ready even when it carries
registration findings. The writer validates the complete candidate, including
`register`. [Claudine permission prerequisites](../../lib/src/doctor/permissions.rs#L150) · [Claudine candidate validation](../../lib/src/config/setup.rs#L667) · [Registration preflight](../../lib/src/composition/registration/inspect.rs#L563)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F7.1 | User handoff projection and writer | Inline user policy, then omitted/null/empty/valid `register` controls, scalar, mixed-validity and all-invalid arrays. | Valid shapes allowed. Each invalid shape already gets a config error, but handoff still advertises `fixable: true` and a deterministic candidate-validation failure follows. Original inline policy stays. | Keep valid controls. Invalid registration content blocks eligibility in that file. |
| R2-F7.2 | Repository handoff projection and writer | Same matrix with a distinct repository pair. | Each valid shape permits the repository handoff. All three invalid shapes are reported as fixable before and after the writer refuses them. | Same verdict as the corrected user case, naming the repository file and leaving eligible user work independent. |
| Clean | Registration repair | User `register: 3` and absent registration entry versus absent or empty lists. | Mis-typed entry blocks the registration repair before writes. Omitted or empty valid lists remain repairable. | Same. |
| Clean | Already referenced permissions | A valid reference with an unrelated config problem. | No handoff edit is planned. Existing valid reference remains valid for permissions inspection. | Apply the preflight to edits while preserving the independent reference verdict. |

Apply the registration-field validation result when a permissions plan
needs to edit that Darkmatter file. Keep verification inside the writer as
well. Mark the finding manual with the existing `config` location rather
than offering a retry that needs a user correction.

### R2-F6 (medium) Flat entries hide independent configuration diagnostics

**Authority (spec and plan).** Doctor reports “violations of the applicable
schema or runtime configuration rules” using the normal validation path.
A migration can expose several independent invalid settings at once. A
mixed-version config containing a legacy hook entry beside both the old
and new preferred-agent keys only reports the legacy entry. Correcting the
hook then exposes the second error. [Configuration checks](spec.md#configuration-files-and-unknown-keys) · [Validation plan](plan.md#necessary-rules)

The defect class is a migration diagnostic suppressing independent
validation evidence. Rewriting an otherwise identical entry between flat
and keyed forms should change its migration diagnosis while preserving
independent document checks. I started with good keyed entries and
injected one second defect at a time. Claudine's `project` validates the
keyed view of a flat entry, then returns before the preferred-agent and
exit-expression rules. [Claudine configuration loader](../../lib/src/config/load.rs#L209)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F6.1 | User actions | Keyed and flat Bash entries, alone and with both agent spellings or an exit-expression entry with both/neither pattern fields. Also try both patterns in the layered `rules` form. | In keyed form the independent path is reported, including `/exit_expressions/rules/0`. In flat form only the migration path appears. | Preserve the additional path and the migration diagnosis. |
| R2-F6.2 | Repository actions | Same hook forms and both/neither pattern cases in the repository. | The keyed form names the repository exit expression. The flat form hides it. Agent spelling is a user-only setting. | Same independent path in either applicable form. |
| R2-F6.3 | User messenger routes | Keyed and flat Slack routes, alone and with the same agent and exit-expression edits. | Same disappearance under flat form. | Preserve both diagnostics. |
| Clean | Nested schema diagnostics and wrong scope | The earlier typo and wrong-type controls, and repository `messenger`. | The revised schema projection preserves nested findings and rejects repository messenger settings at the top level. | Keep that behavior. |
| Clean | Legacy JSON migration | The converter upgrades flat entries and then calls the ordinary config validator. | The published YAML candidate must still pass the independent checks. | Keep the converter's validation and no-write failure. |

Run the two independent document-level checks after validating the keyed
view and before returning the combined error list. Keep the authored
coordinates and the hook or route migration diagnostic. The exit-expression
helper already shares its decision between list form and the `rules` form.

### R2-F8 (high) Darkmatter suggests user initialization for broken repository references

**Authority (defect).** Darkmatter's shell-policy error for a repository
`permissions: ./policy.yaml` with a missing target says to run
`md init --inline`. Running that command from the same repository exits 0
and leaves the repository file unchanged. This predates Doctor's own
remedy. [Darkmatter reference errors](../../../darkmatter/lib/src/config/load/reference.rs#L78) · [Darkmatter init scope](../../../darkmatter/cli/src/commands/init.rs#L73)

The defect class is a repair command whose write scope differs from the
error's scope. If a reference resolves to the same file under an equivalent
spelling, its remedy must stay the same. Switching the declaration to a
repository file must give a correction for that file. I changed one
target shape at a time and ran `md compose` on the same shell document
through the user and repository paths. The declared scope is available in
Darkmatter's reference error source, including for committed policy.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R2-F8.1 | Missing target | Omit `policy.yaml` under the user and repository scopes. | Both errors suggest `md init --inline`. It repairs the user case and leaves the repository case as found. | User remedy can remain. Repository error should name the manual scoped correction. |
| R2-F8.2 | Invalid target YAML | Write an unclosed YAML list. | Both errors repeat the same user-only command. | Same scope distinction. |
| R2-F8.3 | Chained reference | Write `permissions: ./other.yaml` in the target. | Both errors repeat the command after describing the unsupported second reference. | Retain the diagnosis and correct the scope of the remedy. |
| R2-F8.4 | Nonmapping document, wrong block type, or missing block | Write `[]`, then `permissions: 3`, then `{}`. | All three branches suggest the user-only command in both scopes. | Use the same scope-appropriate remedy across these branches. |
| R2-F8.5 | Committed repository policy | The committed-tree reader passes the declaring revision to the same missing-target and target-document branches. | The command in those branches is fixed text, including when the declaration is committed. | Explain that the repository correction must also reach the commit where the broken reference is declared. |
| Clean | Valid mapping | Write `permissions: {}` to the target in each scope. | No initializer remedy is emitted. | Preserve this result. |

Use the already available revision to share one scope-aware reference
remedy. Keep `md init --inline` for user errors. For repository errors,
recommend correcting the named reference or target and explain the commit
boundary when applicable. The documented `--repo` flag also touches user
configuration, so a repository-only remedy should be a direct correction.
[Darkmatter init options](../../../darkmatter/docs/cli/init.md#options)

## Blocked Findings

None.

## Recurrence

- **R2-F2 and review-1.md R1-F2, “Doctor accepts changed registrations that
  Darkmatter refuses.”** This is a new sibling. The earlier repair aligned
  parsing and preparation after admission. The current sweep found an
  admission gate before the shared verifier, including the separate
  digest-note reader. The table covers all member categories, registration
  identity, retirement, and final verification.
- **R2-F5 and review-1.md R1-F8, “Configuration remedies promise behavior
  from commands that bypass configuration.”** These are new siblings. The
  earlier repair proved representative syntax and migration commands and
  extended some of that wording to different validation categories.
  This sweep traces the remedy through the actual command, scope, and
  expected result for mapper, Darkmatter register, and permissions issues.
  R2-F8 covers the same class in Darkmatter’s pre-existing reference loader,
  outside Doctor’s own remedy path.
- **R2-F6 and review-1.md R1-F3, “Flat configuration entries hide genuine
  unknown keys.”** This is a new sibling. The repair restored schema evidence
  for entries in the keyed view. Its early return still bypasses the two
  adjacent document-level validators. The sweep covers both action scopes
  and the user route family.

## Verification and input robustness

| Requirement | Strongest verification and result |
| --- | --- |
| Help, completion, report format, exit status, and passive dispatch | Level 1 CLI fixtures verify missing and broken homes, warning/success/failure, redaction, stable JSON fields, output streams, and noninteractive flow. |
| Config scope, location, aliasing, and merged semantics | Level 1 passive reader results and CLI duplicate-key, nested-key, redaction, and migration cases. Direct CLI controls reproduce R2-F3, R2-F5, and R2-F6. |
| Registration and permissions | Level 1 temporary published bundles, repair interruption tests, locks, retry, and real file comparisons. Direct probes compare Doctor to Darkmatter on the same installation and verify scope-specific lock behavior. Open findings are R2-F1, R2-F2, and R2-F7. |
| Speech and executable discovery | Level 1 injected speech candidates and local CLI executable fixtures, plus Sniff's installed/absent/failing/timed-out shell fixtures. The native Windows repair record covers actual PowerShell startup and profile preservation. R2-F4 covers the conflicting speech wording. |

The command has no keyboard interaction or dependency on an emulator's
input behavior. Level 1 is the relevant tier. The selected Claudine, Sniff,
and Darkmatter tests ran from their canonical Just recipes. This review
ran 276 Claudine cases, 84 Sniff cases, and three Darkmatter cases on Linux. The SimplifiedSchema recipe also ran
10 focused union-diagnostic cases and its 2,172-case minimal-feature suite.
None reached five seconds. I used the recorded Windows run alongside source
and fixture inspection for native branches. I did not run a new Windows,
macOS, or WSL2 test job.

Invalid document shapes use a located error when the loader supplies a
position. Passive review leaves files in place. The Claudine and Darkmatter
working files use the shared YAML document loader with scoped schemas.
Their loader coverage includes duplicate keys, invalid or trailing content,
multiple documents, and accepted comments, BOM, and YAML null. The manifest
uses separate format admission, with published-fixture controls for valid,
duplicate, malformed, stale, missing, and digest-correct invalid content.
R2-F2 covers admission parity between the two readers. Doctor only checks
the presence of legacy JSON, so no JSON decoding boundary was added.

| Load-bearing field or validator | Field-shape evidence and exceptions |
| --- | --- |
| Action lists and messenger routes | Keyed and flat controls; absent/null action overrides are unset; empty action lists and wrong whole, mixed, and all-invalid element types receive the schema verdict. Route names select keyed objects and null routes are invalid. R2-F6 covers diagnostics after the shared schema step. |
| TTS and repository overrides | Absent/null/false disable speech. An empty TTS object enables automatic speech, as do true and absent/null provider in an enabled object. Wrong object, scalar, or sequence types and empty provider names are schema errors. A supported provider name pins selection. A nonempty unknown name warns and uses automatic selection. Null overrides inherit. |
| Permissions rules and their references | Omitted and null follow defaults. Empty maps and arrays use scope rules. Existing tests exercise bad whole-field shapes, mixed invalid rules, missing/invalid referenced mappings, and scope-sensitive globs. Darkmatter's all-invalid rule test now locates both elements through the exposed config result. |
| Registration list | Omitted, null, and empty are supported. Scalar, mixed, and all-invalid arrays are located failures as far as config inspection and the registration writer are concerned. R2-F7 covers the consumer that still offers a permissions edit. |

I checked permissive defaults and optional or filtered parsing in the
changed validators. The Win32 shell probe can only return its outer
argument error for an invalid program name. Doctor passes the fixed name
`claudine`, and timeout and process failures have separate outcomes. The
config projections above name the findings where a diagnostic is lost or
misapplied.

## Implementation record corrections

The dated log notes record the remedies that failed in direct use and the
current PowerShell comparison. Historical examples and prior failed runs
stay labeled as historical evidence.
