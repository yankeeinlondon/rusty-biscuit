---
$schema: feature-review.yaml
ready: false
findings:
  - "R6-F1 (high): Six Darkmatter schema tests exceeded the native Windows L1 time limit"
  - "R6-F2 (high): Darkmatter registration warnings name the wrong list entry"
  - "R6-F3 (low): The registration guides overpromise which comments and entries repair keeps"
  - "R6-F4 (low): Doctor's public library documentation omits the PowerShell scratch directory"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-24T12:10:09+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-6.md
previous: 2026-11-22-claudine-doctor/review-5.md
next: 2026-11-22-claudine-doctor/review-7.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/log.md
implemented-by: claude/opus
---

# Claudine Doctor review 6

**Production readiness declined.** All four review 5 findings passed their
required checks. A recorded Windows package run exceeded the L1 timing
limit, and Darkmatter's ordinary registration warning can direct someone to
the adjacent entry in the same configuration file. Two documentation
findings concern the scope of registration edits and the Windows
PowerShell probe.

## Previous Finding Verification

The table covers every child in the review 5 repair ledger. The earlier
verification tables leave no open findings, and reviews 1–5 contain no
human rulings. [Review 5](review-5.md) ·
[Repair ledger](log.md#finding-ledger-4)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R5-F1 Doctor misses a retired registration entry after its files are gone | R5-F1.1, .2, .c1–.c3, .x1 | **verified fixed** | I published a bundle into a temporary home and tested the retired entry with four equivalent spellings. Doctor reports a located, fixable warning and removes the entry, `md compose` stops warning, and a second fix changes nothing. Flow-list probes with and without the retired file give a manual remedy. L1 tests cover the retired-only case and edit-before-cleanup ordering, setup and a missing-member repair, hidden publication that leaves the list alone, duplicate current entries, the distinct repository file, and the writer's matching rule with the old file present and absent. I also repaired a changed member with a retired entry present and verified that the third-party entry and its comment survived. [Doctor tests](../../lib/src/doctor/registration_tests.rs#L586) · [Setup tests](../../lib/tests/l1/installed_registration.rs) · [Writer tests](../../../darkmatter/lib/tests/l1/config_init.rs) · [CLI probes](/tmp/doctor-r6-probes.log) | None for retirement detection and repair. R6-F2 concerns numbering in Darkmatter's own warning. R6-F3 concerns what the guide promises about the edit. |
| R5-F2 Darkmatter's faster test scans skip tracked symbolic links | R5-F2.1–.3, .c1–.c2, .x1 | **verified fixed** | In an isolated copy of the review 5 repository fixture, the compiled source guard and the two Markdown corpus checks reported the same linked files with the links tracked and untracked. Correcting the target text cleared each result. The L1 prefilter tests also passed for a tracked regular file replaced by a link, a tracked link checked out as a regular file, matching regular files, and a clean tracked regular file. The current-root migration guard keeps its existing no-follow walk and reports only the offending regular file in the combined fixture. [Prefilter and migration guard checks](../../../darkmatter/lib/tests/l1/current_root_migration_guard.rs#L226) · [Markdown collector](../../../darkmatter/lib/tests/l1/shipped_schema_corpus.rs#L1084) | None. |
| R5-F3 Doctor gives the wrong reason for some YAML edit refusals | R5-F3.1–.3 | **verified fixed** | I ran user and repository handoffs with plain, anchored, and tagged policies, with and without an equal copy already in Claudine. The plain controls were eligible, and all eight decorated cases correctly explained the layout obstruction. For current plus retired entries, flow form, anchors, aliases, and tags kept the repair manual, with the marker-aware remedy. When an independent bundle finding called for a repair, the report explained the edit refusal. L1 tests covered missing-entry edits too. The guide names the same obstacles and documents the edits that can replace whole values. [Shared reason and remedy](../../lib/src/doctor/registration.rs#L270) · [Guide](../../docs/cli/doctor.md#files-the-repairs-cannot-edit-in-place) | None. |
| R5-F4 The guide and caller skill promise a completely write-free PowerShell probe | R5-F4.1–.2, .x1–.x3 | **verified fixed** | I compared the guide, caller skill source and installed copy, README, schemas topic, and repo skill with Sniff's temporary-directory allocation. The two primary passages explain the exception next to the inspection promise. The three sibling passages scope their claims to user or registration files. [Guide](../../docs/cli/doctor.md#what-an-inspection-never-does) · [Installed caller skill](/home/build/.claude/skills/use-claudine/cli.md:117) · [Sniff probe](../../../sniff/lib/src/programs/windows_shells.rs#L140) | None in the ledger. R6-F4 is a newly found copy of the old claim in the library documentation. |

## Unblocked Findings

### R6-F1 (high) Six Darkmatter schema tests exceeded the native Windows L1 time limit

**Authority (defect in the test suite).** The review's five-second L1 gate
applies to the package runs recorded by the implementer. The first review 5
native Windows package run recorded six tests from the Darkmatter library's
`base_schema_end_to_end` suite at 5.009–5.872 seconds. The second full run
passed the timing gate. The first run has no contemporaneous host load
measurement, and the recorded 19-test rerun exercised the module together.
A host stall is plausible. The evidence leaves the first run outside the
stated load allowance. [First package run](/tmp/r5-win-dm.log:6595) ·
[Module rerun](/tmp/r5-win-dm-bse.log:95) ·
[Second package run](/tmp/r5-win-dm-2.log:6594)

The defect class is L1 schema tests exceeding the test budget under package
concurrency. The invariant is that all cases keep their assertions and finish
within five seconds in a regular package run. I compared the same cases
under full, filtered, and one-at-a-time execution and inspected the complete timing summaries
for Darkmatter, Claudine, and the link-scan repair. I ran the six cases one
at a time with native Nextest on the recorded Windows checkout. All
finished in 0.262–0.781 s. The 12-logical-processor host reported 5% CPU
load before that command. That measurement applies to the isolated run.
The first package run's load is unknown. The Linux package run
gave the same verdicts within the time limit. The four Claudine-host schema
cases share the fixture that publishes and loads a test bundle. The union
case captures the process directory's repository, and the documentation
case makes two context captures for the same catalog directory. Measuring
those paths separately from process startup would identify which work to
reduce. [Darkmatter schema tests](../../../darkmatter/lib/tests/l1/base_schema_end_to_end.rs) ·
[Serial Windows run](/tmp/doctor-r6-win-serial.log:640)

The timing cells list the first package, filtered module, second package,
and serial runs in that order.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R6-F1.1 | Claudine catalog resolution | Real bundle with shared imports | 5.120 s, 3.353 s, 3.497 s, 0.781 s | Same assertions and at most 5 s |
| R6-F1.2 | Claudine loop controls with lifecycle concerns | Valid combined document | 5.009 s, 3.149 s, 3.578 s, 0.699 s | Valid and at most 5 s |
| R6-F1.3 | Claudine typed lifecycle stack | Valid lifecycle document | 5.211 s, 3.198 s, 4.053 s, 0.680 s | Valid and at most 5 s |
| R6-F1.4 | Claudine malformed lifecycle stack | A malformed stack in the same fixture | 5.345 s, 3.392 s, 4.444 s, 0.759 s | Refused and at most 5 s |
| R6-F1.5 | Union of scalar and mapping types | Valid nested mapping arm | 5.147 s, 2.920 s, 3.701 s, 0.262 s | Valid and at most 5 s |
| R6-F1.6 | Schema documentation and library parity | Actual checked-in catalog and transcluded partial | 5.872 s, 3.643 s, 4.665 s, 0.767 s | Same schema and at most 5 s |
| R6-F1.c1 | Rest of the baseline module, including the pure parser control | Same package and module schedules | Thirteen other tests meet the five-second limit in both package runs. The pure parser control takes 4.789 s in the first run. | Keep the same coverage and stay within budget |
| R6-F1.c2 | Source scan, Markdown schema scan, and current-root guard | Full native Windows suite with the review 5 Git filtering | These cases stay within budget in both package runs. The focused link probes also reach their expected results on Linux. | Preserve both the scan policy and timing |
| R6-F1.c3 | Claudine package and Linux Darkmatter package | Existing L1 package runs | Review 5's Claudine Windows and Linux logs and the new Darkmatter Linux run contain zero tests at or above five seconds. | Preserve the passing cases |

Use a temporary-directory context for the union test and share the one
catalog-directory snapshot between the two readers in the documentation
test. Keep the real catalog paths there because the test checks the
published documentation. Measure the four bundle cases in the package run
and cut shared setup work where the timings show a cost. Keep the bundle and
the validation assertions. Record both package and isolated timings with the
host load measured during the package run.

### R6-F2 (high) Darkmatter registration warnings name the wrong list entry

**Authority (defect).** With a valid current manifest followed by a retired
manifest whose file was deleted, `md compose` reports the retired
reference as “entry 1 of `register`” while its source points to the second
list item. The JSON diagnostic carries the same number and source. Doctor
correctly calls it entry 2. Moving the retired entry to
the first line makes Darkmatter call it “entry 0.” A person following the
entry number can remove the working registration. The user-facing warning
renders the zero-based index in
[`Labeled::into_diagnostic`](../../../darkmatter/lib/src/config/registration.rs#L672)
in the Darkmatter library.

The defect class is displaying an internal zero-based index as a numbered
configuration item. Moving an entry from first to second must increase its
displayed number from 1 to 2. An equivalent path spelling at the same
position must leave that number alone. I started with a bundle published
by Claudine and changed one input at a time. Darkmatter's loading and
snapshot diagnostics converge on the same formatter. The editor's
diagnostics forward that message and build their range separately from the
source coordinates. [Darkmatter registration loader](../../../darkmatter/lib/src/config/registration.rs#L481) ·
[Darkmatter language server projection](../../../darkmatter/dmls/src/diagnostics/frontmatter.rs#L200) ·
[CLI probe results](/tmp/doctor-r6-entry-probe.log)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R6-F2.1 | Darkmatter's entry-resolution warning | Retired missing path in positions 1 and 2, then the same second item rewritten from `~/` to `./` | The warning says 0, 1, and 1. The reported source lines move as expected. | 1, 2, and 2, with the same source coordinates |
| R6-F2.2 | Darkmatter's configuration scope and general local-entry warning | Missing unrelated host in user position 2, and retired path in position 1 of a distinct repository configuration | The warning names the correct file and path but prints 1 and 0 respectively. | 2 in the user list and 1 in the repository list |
| R6-F2.3 | Darkmatter's manifest and member refusal warnings | Present retired manifest in position 2, malformed current manifest in position 1, and valid current manifest with a missing member in position 1 | The warnings print 1, 0, and 0. The manifest and member have their own diagnostic locations. | 2, 1, and 1 for the declaring entries, with those failure locations retained |
| R6-F2.c1 | Claudine Doctor and its repair decision | Same retired entry moved from first to second, equivalent spelling, and unrelated-host negative control | Doctor prints 1, 2, and 2, leaves the unrelated host outside its repair scope, and clears the retired-entry warning after repair. | Preserve the correct ordinals and repair decisions |
| R6-F2.c2 | Structured index, writer edits, and same-host conflict warnings | Inspect the zero-based `Registration.index` used by the writer and the separate host-conflict formatter | The writer uses list offsets for YAML edit paths. The conflict message names the conflicting manifests without numbering the entries. | Keep internal offsets and edit paths unchanged and convert the ordinal at the shared refusal formatter |

Add one only where Darkmatter writes the human-facing entry number. Use
a public registration diagnostic test that moves a refused entry from the
first position to the second. The stable diagnostic code, structured
index, and source location can keep their current meanings.

### R6-F3 (low) The registration guides overpromise which comments and entries repair keeps

**Authority (documentation defect).** The Doctor guide says `--fix`
“keeps every other entry and comment.” I placed two current entries, each
with a distinct inline comment, before a retired entry. Doctor repaired the
file and kept just the first current entry and its comment. When the
retired entry was first, the writer rewrote it to the current reference
and kept its inline comment. The previous current entry and its attached
comment went away, and a third-party entry and its comment survived. The
writer is behaving as designed. [Doctor example](../../docs/cli/doctor.md#registration-darkmatter-host-registration) ·
[Darkmatter replacement planner](../../../darkmatter/lib/src/config/init.rs#L730)

Describe the first matching entry being reused and later matching entries
being removed with their inline comments. Scope the preservation promise
to unrelated entries and comments. Apply the same distinction in the
Doctor layout and repair sections, the Claudine setup and schema topics,
and the Darkmatter host registration topic and writer documentation where they
describe comment preservation. The caller skill's description does not promise that
duplicate entries or their inline comments survive. [Claudine schema topic](../../docs/topics/schemas.md#when-claudine-publishes-and-registers) ·
[Claudine configuration topic](../../docs/topics/configuration.md#darkmatters-permissions) ·
[Darkmatter host registration topic](../../../darkmatter/docs/topics/host-registration.md#registering-a-host) ·
[Darkmatter writer documentation](../../../darkmatter/lib/src/config/init.rs#L561)

### R6-F4 (low) Doctor's public library documentation omits the PowerShell scratch directory

**Authority (documentation defect).** The public `claudine::doctor`
module says “An inspection writes nothing.” On native Windows the API
takes the same path through Sniff as the CLI, creating a private
PowerShell scratch directory before starting the shell. Match the
already-correct command guide by naming the temporary directory
exception and the configuration, registration, and user-profile files
the inspection leaves alone. [Claudine Doctor module](../../lib/src/doctor/mod.rs#L1) ·
[Sniff PowerShell probe](../../../sniff/lib/src/programs/windows_shells.rs#L140)

## Blocked Findings

None. The repairs fit the existing specification and decisions.

## Recurrence

- **R6-F1** is a newly observed sibling of **R4-F3**, which found tests
  exceeding the native Windows L1 budget. The earlier timing repair
  cleared its recorded cases, including the scans that review 5 then
  checked for link coverage. The review 5 repair log exposes six
  different cases in the baseline schema suite. Its whole-package timing
  evidence and the controls are in the R6-F1 table. [Review 4 timing finding](review-4.md#r4-f3-high-recorded-native-windows-l1-tests-exceeded-the-timing-gate)
- **R6-F4** is a missed sibling of **R5-F4** and the documentation side
  of **R1-F4**. The previous sweep corrected the guide, caller skill,
  README, schemas topic, and repo skill. The public library module
  still carries the broader wording. The accepted scratch-directory
  behavior stands. [Review 5 wording finding](review-5.md#r5-f4-low-the-guide-and-caller-skill-promise-a-completely-write-free-powershell-probe) ·
  [Review 1 PowerShell finding](review-1.md#r1-f4-high-powershell-startup-writes-during-inspection)

R6-F2 concerns converting an index for display. The earlier registration
findings concerned recognizing and repairing the right references. R6-F3
concerns the stated scope of an edit that performs the intended
deduplication.

## Verification

| User-facing requirement | Strongest reviewed verification |
| --- | --- |
| Early dispatch, help and shell completion, seven ordered rows, and no initialization or refresh during inspection | L1 CLI subprocess tests cover a missing and broken home, a healthy home, configuration snapshots, help and completion, and the unrelated entry points. The checks exercise actual process dispatch and output. [Claudine Doctor CLI tests](../../cli/tests/l1/doctor.rs) |
| Human and JSON reports, redaction, and exit codes | L1 CLI subprocess tests compare library rows with human and JSON output, including plain and redirected output, warning-only and error exits, successful and failed repair, usage errors, and a failed stdout write. The spec specifies report content and stream behavior. [CLI renderer](../../cli/src/commands/doctor.rs) |
| User and repository configuration, optional repository file, unknown keys, and scope aliases | L1 Doctor results and CLI cases cover presence and validity separately, user and repository defaults, aliased files, merged rules, and located redacted diagnostics. [Configuration tests](../../lib/src/doctor/config_tests.rs) |
| Registration inspection, preservation of other hosts, and safe repair and retry | L1 Doctor results and CLI cases cover changed, missing, and extra paths, layout refusals, link and lock guards, partial failure, ordering, and idempotence. The review 5 retired-entry tests exercise the same public repair and clear the ordinary Darkmatter warning. R6-F2 covers the ordinal in that warning before repair. [Registration tests](../../lib/src/doctor/registration_tests.rs) · [Repair tests](../../lib/src/composition/registration/repair_tests.rs) |
| Permissions handoff and interruption | L1 public results and CLI cases cover user and repository scope, equivalent and third-party references, broken targets, policy conflicts, layout and link refusals, and a second write that fails while the original inline policy remains in force. [Permission tests](../../lib/src/doctor/permissions_tests.rs) |
| Speech and executable lookup | L1 synthetic provider detection covers enabled and disabled speech, pinned and unknown providers, no candidate, and quality versus runtime order. Current PATH tests and Sniff's L1 shell subprocess fixtures cover found, absent, missing, failed, and timed-out lookups. The spec requires no interactive input or terminal-emulator behavior. [Speech tests](../../lib/src/doctor/tts_tests.rs) · [Path tests](../../lib/src/doctor/path_tests.rs) · [Shell probes](../../../sniff/lib/src/programs/windows_shells.rs) |

Linux validation passed 289 focused Doctor tests, 54 registration and setup
tests, and 17,909 tests in the full Darkmatter L1 run. None reached five
seconds in those runs. The 14 shipped-Markdown checks also passed after the review was added.
Both package-area tier checks found zero stranded tests. The suites are selected through the declared L1 targets or library
unit tests, and Claudine CLI's `test-fixtures` feature is enabled in the
local and CI recipes. [Doctor run](/tmp/claudine-r6-nextest.log) ·
[Registration run](/tmp/claudine-r6-registration-nextest.log) ·
[Darkmatter run](/tmp/darkmatter-r6-nextest.log)

### Input robustness

The plan's default is a located error and no operations that depend on an
invalid file. A repair for an independent finding can still proceed. The
shared YAML loader has public Doctor coverage for duplicate keys, trailing
content, malformed syntax, a second document, a non-string key, and BOM
and comments. A bare empty document fails while `{}` succeeds. The
CLI exercises the healthy case, duplicate YAML, a nested unknown key,
and a refused published manifest. The manifest reader's separate L1
tests cover format-specific fields, malformed documents, member lists,
and digest checks. [Configuration matrix](../../lib/src/doctor/config_tests.rs#L675) ·
[Manifest reader tests](../../../darkmatter/lib/tests/l1/host_registration_snapshot.rs) ·
[Plan](plan.md#input-robustness-matrix)

The field exceptions reviewed were:

- Missing and null speech settings use the documented default. An explicit
  empty settings mapping enables automatic selection. A blank or
  wrong-type provider errors, and a nonempty unknown name warns or errors
  when enabled speech has no candidates. Nullable configuration
  overrides inherit. An empty action list and mixed or all-invalid
  action items are refused. The same schema enforces nested and
  scope-specific rules. [Claudine configuration tests](../../lib/src/doctor/config_tests.rs#L735)
- Omitted and null Darkmatter permissions use defaults. An explicit empty
  mapping is valid. The field tests reject bad scalars, blank references,
  malformed collection items, an empty file-read list, and invalid
  network values. A reference to an authored empty mapping resolves.
  Missing, null, non-mapping, chained, and unreadable targets fail.
  [Darkmatter field tests](../../../darkmatter/lib/tests/l1/config_fields.rs) ·
  [Claudine reference tests](../../lib/src/doctor/config_tests.rs#L1204)
- Omitted, null, and empty registration lists all lack a Claudine entry.
  The writer tests distinguish those authored forms. Invalid whole-list
  types, mixed and all-invalid elements, and unusable references produce
  located refusals. I traced the list filtering in Darkmatter through the
  schema refusal and precondition checks. The review 5 matcher handles
  already-validated entries and leaves those checks in force.
  [Darkmatter configuration reader](../../../darkmatter/lib/src/config/load.rs#L1204) ·
  [Registration preconditions](../../lib/src/composition/registration/inspect.rs#L631)

## Implementation record corrections

The repair log now identifies the module rerun as a group run and records
the first full Windows run's timing failures separately from the clean
rerun. [Dated correction](log.md#review-6-verification-2026-11-24)
