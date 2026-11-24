---
$schema: feature-review.yaml
ready: false
findings:
  - "R7-F1 (high): A Windows shell warning hides the invoking process’s successful PATH lookup"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: false
created: "2026-11-24T13:37:22+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: true
implemented-by: claude/opus
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/log.md
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-7.md
previous: 2026-11-22-claudine-doctor/review-6.md
next: 2026-11-22-claudine-doctor/review-8.md
---

# Claudine Doctor review 7

**Production readiness declined.** All four review 6 findings passed verification.
One new finding blocks readiness. On native Windows, a warning from another shell
can hide where the invoking process found Claudine in both report formats.

## Previous Finding Verification

The required instances come from the review 6 repair ledger. The earlier
verification tables close all findings from reviews 1–5. No earlier review
contains a human ruling. [Review 6](review-6.md) · [Repair ledger](log.md#finding-ledger-5)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R6-F1 Six Darkmatter schema tests exceeded the native Windows L1 time limit | R6-F1.1–.6, .c1–.c3, .x1 | **verified fixed** | The final native Windows package run records 3.029, 2.711, 2.871, 3.255, 3.035, and 2.620 s for the six named cases. The remaining baseline cases, the source and Markdown scans, and the current-root guard meet the budget in the same run. The Claudine Linux and Windows runs and the Darkmatter Linux run record zero tests at or above five seconds. I verified that the four bundle assertions remain, the union test owns a temporary context, and the two documentation readers share one catalog context. The declaration-check memo streams the JSON to the hasher and still caches only accepted declarations. My focused Linux runs passed those tests and the scan controls. [Windows timings](/tmp/r6-win-dm.log:6593) · [Baseline tests](../../../darkmatter/lib/tests/l1/base_schema_end_to_end.rs) · [Declaration memo](../../../simplified-schema/lib/src/defaults.rs#L958) · [Linux baseline run](/tmp/r7-dm-focused.log) | None. |
| R6-F2 Darkmatter registration warnings name the wrong list entry | R6-F2.1–.3, .c1–.c2, .x1 | **verified fixed** | I moved a missing retired reference across both positions and rewrote the second spelling. Darkmatter's text and JSON report 1, 2, 2 with the correct source lines. The unrelated host warning and distinct repository warning report 2 and 1. A retired manifest, malformed current manifest, and missing member report 2, 1, 1 and retain their manifest or member locations. Doctor keeps its own correct ordinals and ignores the unrelated host during repair. Structured offsets, writer edit paths, and same-host conflict wording are preserved. The conflict and refusal tests passed. Claudine's mapper test passes with “action 2” and the zero-based path `actions.before_tool[1]`. [Text probes](/tmp/r7-entry-probe.log) · [JSON probes](/tmp/r7-structured-entry.log) · [Refusal tests](../../../darkmatter/lib/tests/l1/host_registration_locations.rs) · [Mapper tests](../../lib/src/doctor/config_tests.rs#L884) | None. |
| R6-F3 The registration guides overpromise which comments and entries repair keeps | R6-F3.1–.3, .c1 | **verified fixed** | In two fresh configuration inputs I reversed the order of the current and retired entries. Repair reused the first entry and kept its inline comment, removed the later entry and its inline comment, and preserved the unrelated entry and the standalone comment. Repeating the repair kept the same bytes. The Doctor guide, Claudine configuration and schema topics, and Darkmatter topic and writer docs describe those edits. The caller skill stays within that promise. [Repair probe](/tmp/r7-comment-probe.log) · [Doctor guide](../../docs/cli/doctor.md#registration-repair) · [Claudine configuration](../../docs/topics/configuration.md#darkmatters-permissions) · [Claudine schemas](../../docs/topics/schemas.md#when-claudine-publishes-and-registers) · [Darkmatter host registration](../../../darkmatter/docs/topics/host-registration.md#registering-a-host) · [Writer documentation](../../../darkmatter/lib/src/config/init.rs#L561) · [Caller skill](../../docs/use-skills/use-claudine/cli.md) | None. |
| R6-F4 Doctor's public library documentation omits the PowerShell scratch directory | R6-F4.1 | **verified fixed** | The public module names the protected configuration, registration, and profile files and describes the temporary folder, matching the command guide. Sniff creates and owns that folder before starting PowerShell. I traced the call from the public Doctor entry point through the same shell probe used by the CLI, and the focused Sniff shell tests passed. [Public module](../../lib/src/doctor/mod.rs#L1) · [Doctor guide](../../docs/cli/doctor.md#what-an-inspection-never-does) · [Sniff probe](../../../sniff/lib/src/programs/windows_shells.rs#L130) | None. |

## Unblocked Findings

### R7-F1 (high) A Windows shell warning hides the invoking process’s successful PATH lookup

**Authority (spec).** The `path` check must “Report whether the current process
can find `claudine` on its executable path.” On the native Windows build host,
I invoked the built CLI by its absolute path from a temporary directory. With a
`claudine.cmd` shim on PATH, the invoking process found it and PowerShell reported
a hit. Git Bash and Cygwin reported misses. Both Doctor report formats showed
the other shells and omitted the invoking process's result, including the path
it found. Giving the same PATH directory a trailing `\.` produced the same report.
In a healthy temporary home the command exited 0 with the shim. Removing it
produced a visible invoking-process error and exit 1.
[Path requirement](spec.md#executable-path) · [Native shim probe](/tmp/r7-windows-shim.log) ·
[Healthy-home control](/tmp/r7-windows-healthy.log)

The defect class is losing a required observation when a more severe finding
replaces the row's summary. An equivalent PATH spelling must preserve the
invoking-process result. Removing the executable must change that result from
a reported hit to a reported miss. In both cases the report must still describe
each installed shell's own outcome. In the Claudine library,
[`path::assess`](../../lib/src/doctor/path.rs#L57) stores a hit only in the
successful summary. [`summary_of`](../../lib/src/doctor/report.rs#L152) chooses
the most severe finding for the row heading. The row's detail comes from the
finding list, which has an entry for a current-process miss and none for a hit.
The human renderer and JSON serializer consume that same row.
[Report construction](../../lib/src/doctor/report.rs#L114) ·
[CLI projection](../../cli/src/commands/doctor.rs#L37)

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R7-F1.1 | Invoking-process hit combined with another shell's miss | Native Windows PATH containing a `claudine.cmd` shim, then the same directory spelled with a trailing `\.`. Git Bash and Cygwin miss. | The row is `warn` and lists both misses. The invoking-process hit and its resolved path are absent from the row. | Keep the two shell warnings and show the invoking-process hit and resolved path. |
| R7-F1.2 | Invoking-process hit combined with another shell's probe failure | Native Windows PATH containing `claudine.exe`. Changing only `TEMP` and `TMP` to a missing directory prevents the PowerShell probe from starting. | The row is `warn` and lists the failed PowerShell probe. The invoking-process hit is absent. | Preserve the hit alongside the probe failure. |
| R7-F1.3 | Shared shell warning projection | The L1 fixture drives missing, timed-out, spawn-failed, nonzero-exit, and terminated shell answers with the same current-process hit. The native probes exercise Git Bash, Cygwin, and PowerShell 7. | Each warning variant becomes a finding of the same severity and takes the same summary path. The success is omitted for every variant. Windows PowerShell and standalone MSYS2 use the same projection. | Retain the hit for each warning. Keep the existing severity and distinct remedy for each probe outcome. |
| R7-F1.4 | Human and JSON report | The native shim miss and PowerShell startup failure, through `doctor --json` and `--plain doctor`. | Both formats omit the hit and path. JSON still uses the same seven-row contract. | Both formats carry the hit and warning in the existing path row. |
| R7-F1.c1 | All available lookups succeed, or no alternate shell can be probed | Native Windows PATH containing `claudine.exe`, with and without trailing `\.`, and the Linux CLI control with no alternate probes. | The row is `ok`, and the invoking-process path is visible. | Keep the successful report. |
| R7-F1.c2 | Invoking-process miss with alternate-shell results | Remove the shim, or remove the directory containing `claudine.exe` from PATH while PowerShell still fails. | The row is `error`, says the invoking process missed, and includes the shell findings and remedies. | Preserve the current-process error and the shell results. |
| R7-F1.c3 | Other consumers of the shared summary and detail projection | An isolated Linux CLI fixture with block and flow mappings, then a misplaced repository key. The user has a valid permission handoff and an unrecognized speech provider with an automatic candidate. Registration also ran with clean and changed bundles. | The config, permissions, key, and speech rows retain their independent observations in detail. Repository presence has a single result and stays visible. Registration's clean-bundle claim describes the entire row and ceases to apply when a problem is present. [Mixed-row controls](/tmp/r7-row-controls.log) | Preserve those results. |

Store the current-process hit as a finding available to the row's detail.
Keep the current-process miss at `error` and the other shells' misses and
failures at `warn`. Extend the existing L1 mixed-outcome test to assert that the
current-process hit remains visible for each warning variant. The native CLI
probes show how both output formats consume the result.
[Executable-path tests](../../lib/src/doctor/path_tests.rs) ·
[Native executable probe](/tmp/r7-windows-path.log)

## Blocked Findings

None. The report can preserve the missing observation within the existing
row and severity rules.

## Recurrence

R7-F1 is a new projection defect. Review 1's report finding concerned
end-to-end test coverage for repair states. Review 2's speech finding
concerned the difference between the top-rated provider and runtime
selection. This finding concerns how the report retains independent
observations at different severities.
[Review 1 report boundary](review-1.md#r1-f6-high-cli-validation-and-repair-format-tests-miss-required-boundaries) ·
[Review 2 speech selection](review-2.md#r2-f4-high-the-speech-report-confuses-highest-quality-with-runtime-selection)

## Verification

| User-facing requirement | Strongest verification |
| --- | --- |
| Early dispatch, non-interactive operation, help and completion, seven ordered rows, and passive inspection | L1 CLI process tests exercise missing, broken, and healthy homes, configuration and published-tree snapshots, help, and completion. [CLI tests](../../cli/tests/l1/doctor.rs) |
| Human and JSON reports, redaction, and exit status | L1 process tests cover both formats, redirected output, successful and failed repairs, errors, warnings, usage, and stdout-write failure. Native L1 CLI probes expose the mixed PATH omission in R7-F1. [CLI tests](../../cli/tests/l1/doctor.rs) |
| Working-copy configuration, unknown keys, scope and aliases, and optional repository configuration | L1 Doctor results and CLI cases exercise user and repository scopes, aliases, merged constraints, file presence, and located redacted diagnostics. [Configuration tests](../../lib/src/doctor/config_tests.rs) |
| Registration inventory, entry identity, legacy cleanup, and retry | L1 library and CLI tests cover admitted and refused bundles, changed and missing files, extra and retired paths, link and lock guards, partial failure, no-op repeat, and preservation of other hosts. The direct review probes exercised the shared writer and both Darkmatter warning formats. [Registration tests](../../lib/src/doctor/registration_tests.rs) · [Repair tests](../../lib/src/composition/registration/repair_tests.rs) |
| Policy handoff in both scopes and recovery after interruption | L1 Doctor results and CLI tests cover equivalent and third-party references, broken targets, conflicts, layouts, linked targets, and a failed second write with the original policy still in force. [Permissions tests](../../lib/src/doctor/permissions_tests.rs) |
| Speech availability and current-process and installed-shell lookup | L1 fixtures exercise speech settings, candidate quality and order, PATH and suffix handling, and each shell outcome. Native CLI runs show real shell discovery and lookup. The mixed-outcome fixture omits an assertion for the retained current-process hit, as described in R7-F1. The spec requires no terminal-emulator or keyboard behavior. [Speech tests](../../lib/src/doctor/tts_tests.rs) · [Path tests](../../lib/src/doctor/path_tests.rs) · [Sniff shell tests](../../../sniff/lib/src/programs/windows_shells_tests.rs) |

The local run passed 265 Claudine library tests, 25 Claudine CLI tests,
20 Darkmatter baseline tests, 180 Darkmatter location and corpus tests,
25 Sniff shell tests, and the focused declaration and diagnostic checks.
None of these runs recorded a test at or above five seconds. I checked the
declared L1 targets, their module declarations, and the local and CI
feature selection for the work-counter and CLI fixtures. The proof for
review 6's timing finding comes from the recorded native Windows package
run. My native Windows runs were focused behavioral probes.
The repair's temporary profiling driver took 10.240 s across 40
registration loads. The driver was removed before the package runs, and
the checked-in work-counter test measures one load.
[Claudine library](/tmp/r7-cl-lib.log) · [Claudine CLI](/tmp/r7-cl-cli.log) ·
[Darkmatter baseline](/tmp/r7-dm-focused.log) · [Darkmatter corpus and locations](/tmp/r7-dm-registration.log) ·
[Sniff](/tmp/r7-sniff.log) · [Declaration checks](/tmp/r7-ss.log) ·
[Native Windows package](/tmp/r6-win-dm.log) · [Profiling run](/tmp/r6f1-prof-run.log)

### Input robustness

The plan sets a located error as the default for invalid input. Dependent
operations make no writes and independent eligible repairs can proceed.
The shared YAML loader has L1 controls for a valid mapping, duplicate keys,
invalid trailing content, a second document, non-string keys, and BOM and
comment syntax. The CLI tests cover healthy input, duplicate and nested
key locations, and rejection of a damaged published manifest. The v2
manifest uses its own envelope, member, and digest checks. Legacy JSON
is checked for existence only. [Plan](plan.md#input-robustness-matrix) ·
[YAML cases](../../lib/src/doctor/config_tests.rs) ·
[Manifest cases](../../../darkmatter/lib/tests/l1/host_registration_snapshot.rs)

- Claudine speech settings distinguish disabled forms from the empty
  mapping that enables automatic selection. Missing or null provider
  selects automatic detection, blank and wrong-type names error, and an
  unknown name has its own warning. Action and override cases distinguish
  absent, null inheritance, empty input, wrong collection type, and mixed
  or wholly invalid elements. User-only, nested, and imported permission
  keys are exercised separately. [Claudine configuration tests](../../lib/src/doctor/config_tests.rs) ·
  [Claudine wire tests](../../lib/src/config/wire_tests.rs)
- Darkmatter permissions use scoped defaults for omission and null, allow
  an authored empty inline mapping, and require an authored mapping in a
  referenced file. The tests locate malformed scalars, blank references,
  wrong or invalid collection contents, and invalid scope references.
  Missing, null, nonmapping, and chained targets fail. [Darkmatter field tests](../../../darkmatter/lib/tests/l1/config_fields.rs) ·
  [Doctor reference tests](../../lib/src/doctor/config_tests.rs#L1204)
- Absent, null, and empty `register` lists all lack a Claudine reference,
  with distinct source shapes for editing. Wrong whole-list types, mixed
  and wholly invalid items, and invalid reference syntax get a located
  configuration finding. The matcher sees validated entries, so filtering
  at the writer does not silently admit a malformed item. The ordinal
  repair converts the number at display and retains the source offset.
  [Darkmatter configuration loader](../../../darkmatter/lib/src/config/load.rs) ·
  [Writer tests](../../../darkmatter/lib/tests/l1/config_init.rs) ·
  [Registration preconditions](../../lib/src/composition/registration/inspect.rs)

I checked defaulting, nullable fields, list filtering, and discarded parse
results in the Doctor readers and shared registration entry path. Review
6 changes how existing results are numbered and how accepted schema
checks are memoized. It adds no new file shapes.

## Implementation record corrections

I corrected review 6's ledger and progress notes where they attributed
repository discovery to the union test, attributed fixture initialization
counts to one registration load, and misstated the sampled Darkmatter
Windows CPU range. The original union context read the process directory
without repository discovery. The one-time fixture setup accounts for the
extra counted work. The CPU log includes a 39% sample while tests were
running. [Dated correction](log.md#review-7-verification-2026-11-24)
