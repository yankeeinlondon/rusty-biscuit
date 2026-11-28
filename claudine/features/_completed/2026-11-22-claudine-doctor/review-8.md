---
$schema: feature-review.yaml
ready: true
findings: []
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: false
created: "2026-11-24T13:53:18+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-8.md
previous: 2026-11-22-claudine-doctor/review-7.md
---

# Claudine Doctor review 8

**Production ready.** Doctor retains the invoking process's successful
executable lookup when another shell cannot find Claudine or its probe
fails. Both report formats carry that observation in the existing `path`
row. Review 8 found no new defects.

## Previous Finding Verification

Review 7 had one open finding. Reviews 2–7 verify closure of the preceding
findings, and reviews 1–7 contain no human rulings. The instance list below
comes from the review 7 repair ledger. [Review 7](review-7.md) · [Repair ledger](log.md#finding-ledger-6)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R7-F1 A Windows shell warning hides the invoking process’s successful PATH lookup | R7-F1.1–.4, .c1–.c3 | **verified fixed** | The mixed-outcome tests keep the current-process hit with a shell miss or startup failure (.1–.2). The tests retain the hit for all five warning outcomes across all five shell kinds and separately check distinct messages, severity and remedies (.3). The mixed row serializes with the hit and both warnings in JSON. The CLI human renderer directly lists every line of that same detail string, which its own test checks (.4). A direct CLI probe confirmed that a directory with a space and its equivalent spelling with `/.` both report the resolved executable and exit 0 in both formats. Removing the directory from PATH reports a miss and exits 1. The injected mixed-miss control retains the shell results and remedies (.c1–.c2). I checked the other summary producers against the mixed-row controls and reran their tests. Configuration, permissions, repository presence, unknown keys, and speech keep independent findings in detail. Registration's success summary describes the overall state (.c3). [Path decision](../../lib/src/doctor/path.rs#L65) · [Path tests](../../lib/src/doctor/path_tests.rs) · [CLI renderer](../../cli/src/commands/doctor.rs#L69) · [Direct CLI probe](/tmp/r8-doctor-cli-probe.log) · [Mixed-row controls](/tmp/r7-row-controls.log) | None. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding recurred. The shared reporting code retains a finding's detail
even when a more severe finding supplies the row summary. The repair uses
that existing behavior for the current-process lookup. [Report construction](../../lib/src/doctor/report.rs#L114)

## Verification

| User-facing requirement | Strongest verification |
| --- | --- |
| Help and completion, early dispatch, non-interactive diagnosis, and passive inspection | L1 CLI tests cover missing, broken, and healthy homes, help and completion, and file snapshots before and after inspection. The existing Sniff tests cover startup-free shell command construction and bounded subprocesses. [CLI tests](../../cli/tests/l1/doctor.rs) · [Shell tests](../../../sniff/lib/src/programs/windows_shells_tests.rs) |
| Ordered human and JSON rows, exit status, redaction, and repair results | L1 CLI tests assert row shape and order, warning-only and error exits, successful and failed repairs, and redaction in both formats. They also verify redirected output and stdout-write failure. [CLI tests](../../cli/tests/l1/doctor.rs) |
| Working-copy configuration and unknown keys in user and repository scopes | L1 Doctor results cover optional and aliased files, strict field and key validation, located findings, and effective merged settings. CLI tests verify the reported locations. [Configuration tests](../../lib/src/doctor/config_tests.rs) · [CLI tests](../../cli/tests/l1/doctor.rs) |
| Registration inventory, equivalent references, and scoped repair | L1 Doctor and CLI cases cover stale and missing files, bundle refusal, extras and legacy paths, link and lock obstructions, partial writes, and a byte-preserving repeat. Shared writer evidence covers preservation of unrelated hosts and comments. [Registration tests](../../lib/src/doctor/registration_tests.rs) · [Review 7 repair probe](/tmp/r7-comment-probe.log) |
| Permission handoff and retry | L1 Doctor and CLI tests cover both scopes, equivalent and third-party references, broken targets, conflicts, and the policy remaining in force after an interrupted handoff. [Permissions tests](../../lib/src/doctor/permissions_tests.rs) · [CLI tests](../../cli/tests/l1/doctor.rs) |
| Speech availability, quality, and runtime order | L1 Doctor tests exercise automatic and pinned selection, disabled hook speech, unrecognized providers, and enabled speech with no candidate. [Speech tests](../../lib/src/doctor/tts_tests.rs) |
| Executable discovery and installed-shell results | L1 subprocess and row tests cover executable presence, absence, platform suffixes, and installed-shell outcomes. The current Linux CLI probe verifies the two output formats, equivalent PATH spellings, exit status, and that the located executable never runs. Review 7's native Windows probe establishes the mixed-shell inputs. Review 8 verifies the changed reporting code with injected shell outcomes and the CLI renderer. [Path tests](../../lib/src/doctor/path_tests.rs) · [Native Windows reproduction](/tmp/r7-windows-shim.log) · [Direct CLI probe](/tmp/r8-doctor-cli-probe.log) |

L1 reaches all the behaviors this command promises. The focused Linux
recipes passed 292 Claudine tests and 25 Sniff shell tests, with every test
under five seconds. The CLI cases are in the declared `l1` target, the local
recipe and CI enable their fixture feature, and the tier coverage check
reported zero stranded tests. The recorded full Claudine repair run contains
13,062 passes, 16 skips, and zero tests at or above five seconds. The lint log
shows a clean run. The full-package and native Windows evidence cited here
comes from the earlier runs. [Focused Claudine run](/tmp/r8-claudine-doctor.log) · [Focused Sniff run](/tmp/r8-sniff-shells.log) · [Full repair run](/tmp/r7impl-test.log) · [Lint log](/tmp/r7impl-lint.log)

The README and Administration command index link to the Doctor reference.
The configuration and schema topics describe the shared repair behavior.
The implementation record captures publication of the caller skill, and the
current skill source describes the command and repair scope. [README](../../README.md) · [Command index](../../docs/cli/index.md) · [Configuration](../../docs/topics/configuration.md) · [Schemas](../../docs/topics/schemas.md) · [Caller skill](../../docs/use-skills/use-claudine/cli.md) · [Publication record](implementation-log.md#skill-publication)

### Input robustness

The final repair changes how a lookup result is reported. The YAML readers,
manifest loaders, and field validators are unchanged by that repair. The
plan's default is a located error for invalid input with no dependent write.
Independent eligible repairs can proceed. The YAML tests cover valid and
invalid whole documents, including duplicate keys, trailing content, a
second document, and accepted BOM and comment syntax. The format 2 manifest
tests exercise the separate envelope, member, and digest checks. Legacy JSON
is checked for file presence only. [Plan](plan.md#input-robustness-matrix) ·
[Configuration cases](../../lib/src/doctor/config_tests.rs) · [Manifest cases](../../../darkmatter/lib/tests/l1/host_registration_snapshot.rs)

- Claudine speech treats omission, null, and false as disabled. An empty
  mapping enables automatic selection. A missing or null provider selects
  automatically, while a blank or wrongly typed name is invalid and an
  unrecognized name produces a warning. The action and override validators
  cover empty input, null inheritance, whole-field type errors, and invalid
  elements. [Configuration cases](../../lib/src/doctor/config_tests.rs) · [Wire cases](../../lib/src/config/wire_tests.rs)
- Darkmatter permissions use scoped defaults when omitted or null and accept
  an empty inline mapping. A reference must resolve to an authored mapping.
  Invalid scalars, blank references, invalid collections, and missing, null,
  nonmapping, or chained reference targets get located findings. [Darkmatter fields](../../../darkmatter/lib/tests/l1/config_fields.rs) · [Doctor references](../../lib/src/doctor/config_tests.rs)
- Darkmatter `register` treats omitted, null, and empty lists as having no
  Claudine entry. A wrong field type, invalid element, or malformed reference
  produces a located finding before the writer considers a change. I checked
  that the shared writer's entry filtering receives validated references.
  [Darkmatter loader](../../../darkmatter/lib/src/config/load.rs) · [Registration tests](../../lib/src/doctor/registration_tests.rs)
