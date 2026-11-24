---
$schema: feature-review.yaml
ready: false
findings:
  - "R5-F1 (high): Doctor misses a retired registration entry after its files are gone"
  - "R5-F2 (medium): Darkmatter's faster test scans skip tracked symbolic links"
  - "R5-F3 (low): Doctor gives the wrong reason for some YAML edit refusals"
  - "R5-F4 (low): The guide and caller skill promise a completely write-free PowerShell probe"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-24T10:29:00+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-5.md
previous: 2026-11-22-claudine-doctor/review-4.md
---

# Claudine Doctor review 5

**Production readiness declined.** All four review 4 repairs passed their
required cases. Doctor still misses a retired user registration entry in an
otherwise healthy installation. A test-speed optimization also drops
coverage for tracked symbolic links. Two smaller findings concern the
YAML repair message and the documentation for Windows shell probes.

## Previous Finding Verification

The review 4 repair ledger defines the cases below. Reviews 1–3 left no
open findings and contain no human rulings. [Review 4](review-4.md) ·
[Review 4 repair ledger](log.md#finding-ledger-3) · [Review 3](review-3.md)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R4-F1 The registration writer validates an aliased user file as repository policy | R4-F1.1–.3, .c1–.c3, .x1–.x2 | **verified fixed** | I ran Doctor and setup from outside a repository, inside a separate repository with an alias to the user file, and with the home as the repository root. The user-only grant passed. The comment and other host entry survived, and setup still registered the manifest when the separate permissions handoff reported a conflict. The focused Doctor suite passed the scope and alias skips. The recorded L1 tests passed for setup handoff, hidden publication, existing entry lookup, and Darkmatter's direct registration and initialization APIs. I checked that every caller supplies the selected scope to Darkmatter's writer. [Doctor and setup consumers](../../lib/src/composition/registration/repair.rs#L183) · [Darkmatter scope selection](../../../darkmatter/lib/src/config/init.rs#L378) | None. |
| R4-F2 Doctor offers repairs that the YAML editor refuses | R4-F2.1–.4, .c1–.c3, .x1 | **verified fixed** | The CLI probes covered inline and block registration append, replacement, duplicate removal, and the inline root. Block forms repaired, and the refused cases stayed manual. Omitted, null, empty, and unchanged-list controls passed. The permissions suite covered both scopes and both write targets, equal policies, two-file transfers, and a second scope left eligible. Setup still reports the writer refusal and preserves legacy files. The writer and Doctor layout tests passed, and direct anchor, alias, and tag probes withheld repair when the edit reached those nodes. [Registration assertions](../../lib/src/doctor/registration_tests.rs#L606) · [Permissions assertions](../../lib/src/doctor/permissions_tests.rs#L1283) · [Darkmatter planner](../../../darkmatter/lib/src/config/init.rs#L687) | None in the prior eligibility cases. R5-F1 concerns a retired entry the eligibility check never reaches, and R5-F3 concerns the new explanation of a correctly refused edit. |
| R4-F3 Recorded native Windows L1 tests exceeded the timing gate | R4-F3.1–.10, .c1, .x1–.x4 | **verified fixed** | I inspected the per-test nextest lines and timing summaries. Claudine's three saved-choice cases took 1.168–1.170 s, and the three related approval cases took 0.837–1.164 s. In the final Darkmatter package run the glob guard took 1.513 s, shell-trigger cases 0.421–0.435 s, the nested gate 0.218 s, Windows anchor cases 0.113–0.171 s, five recursive merge cases 0.891–1.026 s, and the repository boundary case 0.265 s. The three discovered scan cases ran in 0.446 s and at most 0.979 s and 0.836 s per split test. Both package summaries report zero cases at or above 5 s. I checked the test targets, the matching Windows source trees for the two packages, and the Linux summaries. [Claudine Windows nextest log](/tmp/r4/win-claudine.log:7309) · [Darkmatter Windows nextest log](/tmp/r4/win-darkmatter-2.log:13092) | None of the timing cases remain. R5-F2 concerns coverage lost in the faster scans. |
| R4-F4 The guide says Doctor reports a publisher lock that it can leave unreported | R4-F4 | **verified fixed** | The focused L1 suite passed the linked-lock case with a healthy bundle and the linked registration path controls. The guide now describes when the lock leaves `registration` at `ok` while Doctor can still fix the permissions handoff. [Guide](../../docs/cli/doctor.md#darkmatter-permissions) · [Lock regression test](../../lib/src/doctor/permissions_tests.rs#L497) | None. |

## Unblocked Findings

### R5-F1 (high) Doctor misses a retired registration entry after its files are gone

**Authority (spec).** The registration check reports “legacy registration
leftovers,” validates the user `register` entry using Darkmatter's
manifest identity rules, and updates the user entry safely. A person can
manually remove the old bundle after publishing the new one, yet leave
both entries in `~/.darkmatter.yaml`. Doctor reports `ok` and exits 0,
while `md compose` still warns at the retired entry. [Registration
contract](spec.md#darkmatter-registration)

The defect class is incomplete detection of a retired reference after its
target disappears. The same retired path must be recognized with its
file present or absent, under any equivalent spelling. Changing the
reference to another host's path should keep that entry outside Claudine's
repair. The assessment only asks whether any entry names the current
manifest. The writer already knows how to plan removal of the retired
entry by location even when its file has gone. I used a bundle published
by the CLI, a healthy permissions reference, and the same user
configuration for inspection, repair, and `md compose`. [Doctor's entry
assessment in Claudine](../../lib/src/doctor/registration.rs#L175) ·
[Replacement planning in Darkmatter](../../../darkmatter/lib/src/config/init.rs#L730)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R5-F1.1 | User `register` observation, registration row, repair selection, and final result | Published current manifest plus the retired entry in a block list. Remove the old manifest with the current entry still present. Test the retired entry with `~/`, `./`, and a bare relative path. | Both Doctor invocations exit 0 with `registration: ok` and `fixable: false`. Darkmatter warns at the retired entry in all three spellings. | Locate the retired entry as a warning and offer its removal. Verify it is gone, with the current entry and unrelated entries intact. |
| R5-F1.2 | Layout precheck and manual report for the same entry | Rewrite the same pair as an inline YAML list, with and without the old manifest. | With the old manifest gone, the row stays `ok` and there is no remedy. With the old file present, the retire-and-dedupe operation has a manual layout remedy. | Recognize the same leftover in both cases. Keep the inline layout manual with the existing block-list remedy. |
| R5-F1.c1 | Existing retirement paths and retry | Keep the retired manifest with both list entries. Separately, remove the old manifest and keep only the retired entry. | The first case warns and drops the retired entry before legacy cleanup. The second reports a missing current entry and replaces the retired entry. Both converge. | Preserve both paths and the write-before-cleanup order. |
| R5-F1.c2 | An independently triggered repair, setup, and standalone publisher | Keep both entries and no retired manifest. Add a third-party entry. Try an otherwise identical setup, a Doctor fix triggered by a modified bundle member, and hidden publication alone. | Setup and the Doctor repair both remove the retired entry and preserve the third-party entry. Hidden publishing leaves the list alone. | Reuse the existing replacement decision for a standalone retired-entry finding. Keep hidden publication limited to the bundle. |
| R5-F1.c3 | Manifest admission, healthy entry lookup, and repository boundary | Keep only the current entry, repeat it, or put the retired entry only in a distinct repository file. | Current-only and duplicate-current files have a working registration. Doctor's registration repair leaves the distinct repository list alone. | Keep the user repair restricted to the user file and authorized Claudine locations. |

Share the writer's retired-entry match with the report, including the
existing identity fallback for a missing manifest. Treat a matching
retired entry as a reason to run the existing repair. Reuse the
layout check and verify the two entry conditions after the write. A small
CLI regression case should confirm that the repaired configuration clears
Darkmatter's warning and that a second fix is a no-op. Preserve other
hosts and use a file and entry position in the report to avoid echoing
authored values. [Entry verification in Claudine](../../lib/src/composition/registration/repair.rs#L183)

### R5-F2 (medium) Darkmatter's faster test scans skip tracked symbolic links

**Authority (regression).** The previous Darkmatter source and shipped
Markdown checks read file links reached by their scans. The timing repair
now skips a tracked link when Git has no match, even when reading its
target would find the searched text. File links are a live repository
pattern, including a linked design document under Claudine's shipped
feature tree. A target outside the scanned directories can consequently
escape both coverage and the guard's counts. [Existing linked design
document](../_completed/2026-06-28-real-errors/design.md) ·
[Darkmatter's shared test prefilter](../../../darkmatter/lib/tests/git_prefilter/mod.rs#L26)

The defect class is a scan skipping a path using a negative result from a
search with a different file policy. With the directory entry and target
bytes held constant, staging a link should leave any scan verdict
unchanged. Removing the searched text or fixing the schema should change
that verdict. Git labels a tracked symbolic link `H` in the listing, but
its working-tree grep omits the link. The same omission happens when a
tracked regular file has been replaced by a link in the working tree.

I ran the compiled Darkmatter L1 tests against a temporary repository with
the relevant directories and copied Claudine docs. Each link pointed to
a fixture outside the scanned directories. I staged and unstaged the
same directory entry, keeping target bytes constant. For the named-type
case I used the repository's named-type example and changed its name to
one the existing schema does not define. [Named-type fixture](../../../messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/p15_one_line.md)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R5-F2.1 | Darkmatter's old-baseline source guard | A scanned `.rs` link points to a text file that includes the old baseline path. | The compiled guard passes with the link tracked. The same guard fails and names the link as soon as it becomes untracked. | Both scans report the old path. |
| R5-F2.2 | Shipped Markdown inline-schema parse tests | A linked document declares a schema with an unknown constraint. | The compiled test passes with the link tracked and fails with the linked document's parse error when untracked. All seven groups use the same collection function. | Both scans submit the schema to the parser. |
| R5-F2.3 | Shipped Markdown named-type resolution tests | A linked document imports an unknown type from a real schema beside the link. | The compiled test passes with the link tracked and fails with the unknown named type when untracked. All seven groups use the same collection function. | Both scans resolve the import from the same document path and report the missing name. |
| R5-F2.c1 | Existing Darkmatter current-root migration guard | The same Git listing for a link with an offending target, plus ordinary files. | The helper also offers to skip the link here, but the guard's own no-follow directory walk excludes file links before the optimization and keeps that same scope. Its regular-file self-test passed. | Preserve the original walk policy and regular-file detections. |
| R5-F2.c2 | Shared Git selection controls | Tracked regular file with the needle, tracked regular file after removing the needle, and untracked regular file with the needle. | The helper retains both files with the needle and skips the clean tracked file. A tracked regular file changed into a link reproduces the bad skip. | Preserve skips for regular-file clean cases and read a link when the scan would previously follow it. |

Keep the performance win by applying Git's negative answer only to
regular entries the scan would read as regular files. Both existing
walks can use the directory entry's no-follow file type to keep file
links in their normal read path. Add one fixture exercising the shared
decision for a tracked link and a working-copy link replacing a regular
file. [Source guard in Darkmatter](../../../darkmatter/lib/tests/l1/global_catalog.rs#L386) ·
[Markdown collector in Darkmatter](../../../darkmatter/lib/tests/l1/shipped_schema_corpus.rs#L1078) ·
[Current-root guard in Darkmatter](../../../darkmatter/lib/tests/l1/current_root_migration_guard.rs#L123)

### R5-F3 (low) Doctor gives the wrong reason for some YAML edit refusals

**Authority (defect).** Doctor says a block-style setting “is not written
in block form” when Darkmatter refuses its YAML anchor or tag. I ran
Doctor over an anchored and a tagged block `permissions` mapping for
both user and repository scope, with both equal and uncopied policies.
The report made the same claim for an anchored or aliased registration
entry when another finding also triggered a layout explanation. The
editor correctly leaves these cases manual, and removing the marker
allows the corresponding plain block edit. Make the reason accurate for
both types of obstruction. A shared explanation that the editor cannot
change this layout in place, followed by the existing guidance, is
sufficient. Update the guide's “only the block layout” sentence to match
what the editor accepts. [Shared explanation in Claudine](../../lib/src/doctor/registration.rs#L251) ·
[YAML editor in Darkmatter](../../../darkmatter/lib/src/yaml_edit.rs#L322) ·
[Command guide](../../docs/cli/doctor.md#files-the-repairs-cannot-edit-in-place)

### R5-F4 (low) The guide and caller skill promise a completely write-free PowerShell probe

**Authority (defect in documentation).** The guide says inspection
“writes, creates, converts, or deletes no file,” and the caller skill
says inspection “writes nothing.” The guide then accurately describes
the private temporary directory in which PowerShell can write a startup
cache. Sniff creates that directory before starting the shell. Review 2
already accepted the temporary-profile isolation, with native evidence
for both installed PowerShell variants. Describe the preserved
configuration, registration, and user profile, and keep the temporary
directory exception next to the promise. Use the same wording in the
caller skill and republish it. [Command guide](../../docs/cli/doctor.md#what-an-inspection-never-does) ·
[Caller skill](../../docs/use-skills/use-claudine/cli.md#checking-the-setup-claudine-doctor) ·
[Sniff's PowerShell probe](../../../sniff/lib/src/programs/windows_shells.rs#L140) ·
[Review 2 verification](review-2.md#previous-finding-verification)

## Blocked Findings

None. Each repair fits the spec and the accepted decisions.

## Recurrence

- **R5-F1** is a missed sibling of the registration checks in **R1-F2**
  and the duplicate-removal case in **R4-F2**. Those reviews checked
  admission of the current manifest and the writer's ability to remove
  an additional retired entry while its file still existed. The sweep
  missed the transition from present to missing at the entry assessment
  and repair-selection decisions. The instance table also checks the
  existing writer, verification, and runtime warning. [Review 1
  registration finding](review-1.md#r1-f2-high-doctor-accepts-changed-registrations-that-darkmatter-refuses) ·
  [Review 4 layout finding](review-4.md#r4-f2-high-doctor-offers-repairs-that-the-yaml-editor-refuses)
- **R5-F3** is a reporting sibling introduced by the **R4-F2** repair.
  The blocked edit and its remedy work. The explanation uses one
  description for flow collections and block values decorated with YAML
  markers. A wording change handles the class without adding another
  editor rule. [Review 4 layout finding](review-4.md#r4-f2-high-doctor-offers-repairs-that-the-yaml-editor-refuses)
- **R5-F4** is the documentation sibling of **R1-F4**. The accepted
  startup-cache isolation still holds. The guide and caller skill
  should describe its exception explicitly. [Review 1 PowerShell
  finding](review-1.md#r1-f4-high-powershell-startup-writes-during-inspection)
- **R5-F2** arose in the **R4-F3** timing repair. It is a new loss of scan
  coverage. The measured timing repair itself passed verification.
  [Review 4 timing finding](review-4.md#r4-f3-high-recorded-native-windows-l1-tests-exceeded-the-timing-gate)

## Verification and input robustness

| Requirement | Strongest reviewed verification |
| --- | --- |
| Early dispatch, help and completion, seven ordered rows, final exit statuses, and no initialization during inspection | L1 subprocess tests exercise help with no configuration, both options in completion, usage error 2, warnings-only exit 0, remaining or failed repairs exit 1, successful repair exit 0, and before/after filesystem snapshots. [Claudine CLI tests](../../cli/tests/l1/doctor.rs) |
| Human and JSON reports, redaction, and output errors | L1 subprocess tests cover the same rows in both modes, redirected and plain output, no ANSI under forced color with redirection, a failing stdout writer, and secrets withheld from stdout and stderr. Their contract concerns the emitted text and exit status. [Claudine CLI output](../../cli/src/commands/doctor.rs) |
| Configuration, scope, unknown keys, and optional repository presence | L1 public result and CLI tests cover both scopes and aliases, missing and legacy files, presence separate from validity, schema and merged rules, nested and imported keys, and file and key positions. [Claudine configuration tests](../../lib/src/doctor/config_tests.rs) |
| Registration observation and repair | L1 library and subprocess cases cover manifest admission, missing, changed and extra files, path identity, blocked links and sidecars, partial failure and retry, concurrent repair, preservation of other registrations, and idempotence. R5-F1 covers a retired reference whose files are already gone. [Claudine registration checks](../../lib/src/doctor/registration_tests.rs) · [Publisher checks](../../lib/src/composition/registration/repair_tests.rs) |
| Permissions and handoff | L1 public results and CLI cases cover both scopes, omitted and null settings, equivalent and third-party references, invalid and broken targets, conflict, unchanged bytes for refused edits, and interrupted second writes. The original block remains in force through a failed transfer. [Claudine handoff tests](../../lib/src/doctor/permissions_tests.rs) |
| Speech and executable discovery | L1 tests cover speech disabled, pinned, automatic, and unknown-provider fallback, quality versus runtime order, no candidates, invoking-process path and executable suffixes, and the distinct Windows shell outcomes. Earlier native runs confirm the profile-isolation behavior with PowerShell 5.1 and 7. The spec requires neither a real terminal emulator nor OS key injection. [Speech tests](../../lib/src/doctor/tts_tests.rs) · [Claudine path tests](../../lib/src/doctor/path_tests.rs) · [Sniff shell tests](../../../sniff/lib/src/programs/windows_shells_tests.rs) |

Invalid working files produce a located report and withhold edits that
depend on the invalid document. Independent eligible repairs can still
run. Claudine and Darkmatter share one YAML loader. Its test cases cover
duplicate keys and the repeated key's location, trailing invalid
content, malformed syntax, a second document, non-string keys, BOM,
comments, null, and an empty document versus an empty mapping. The CLI
passes the positive control and locates both a duplicate key and a
nested stray key. Legacy JSON is existence-only in Doctor.

The field validators cover the following defaults and exceptions.

- Claudine's user `tts` treats absent, null, and false as disabled,
  while true and an empty settings mapping select automatic speech.
  An absent or null provider also selects automatic speech. Empty and
  wrong-type names fail, and an unknown nonempty name warns. Actions
  and overrides have tests for null inheritance, authored empty
  mappings and lists, wrong collection types, mixed and all-invalid
  items, and scoped or merged semantic failures. [Claudine field
  checks](../../lib/src/doctor/config_tests.rs#L735) ·
  [Claudine action wire checks](../../lib/src/config/wire_tests.rs)
- Darkmatter's omitted or null `permissions` uses the scope default.
  An authored empty mapping is valid inline policy. Scoped tests cover
  bad scalars, a blank reference, malformed values in rules, an empty
  rule collection, and invalid file grants and network settings. A
  reference to an authored empty mapping passes. Missing, null,
  nonmapping, chained, and unreadable targets fail. [Darkmatter field
  checks](../../../darkmatter/lib/tests/l1/config_fields.rs) ·
  [Claudine reference checks](../../lib/src/doctor/config_tests.rs#L1204)
- Darkmatter's omitted, null, and empty `register` forms are all
  valid, with distinct edit tests for the three forms. Wrong whole type,
  mixed or all-invalid items, and invalid local-reference syntax produce
  located findings. The manifest loader's format, digest and member
  checks have separate public and CLI coverage. R5-F1 uses valid fields
  and exposes a downstream registration decision. [Darkmatter init
  checks](../../../darkmatter/lib/tests/l1/config_init.rs) ·
  [Host bundle checks](../../../darkmatter/lib/tests/l1/host_registration_snapshot.rs)

I checked load-bearing defaults, nullable conversions, item filtering,
and discarded parse errors in the configuration, permissions,
registration, and manifest readers. The defaulting and filtering in the
reviewed cases follow the schema or occur after a located refusal has
been recorded. The layout dry run is a separate write boundary, exercised
with the same values in flow and block form.

On Linux, 246 focused Claudine library tests, 26 Doctor CLI tests, and
25 Sniff shell tests passed. None reached five seconds. The tier checks
for Claudine, Darkmatter, and Sniff found zero stranded tests. I read the existing native Windows package logs for
the timing repair. They record 7,220 Claudine and 12,989 Darkmatter
passes with zero tests at or above five seconds. The earlier Windows
Darkmatter run recorded three timing failures before the scan repairs
and was superseded by the final package run. The targeted symlink
failures in R5-F2 were deliberate regression probes in an isolated
repository. [Native Darkmatter timing log](/tmp/r4/win-darkmatter-2.log:13092)

## Implementation record corrections

The repair log now describes exactly what Git searched and records the
tracked-link exception confirmed in the compiled Darkmatter tests. It
also gives the timings from the final Darkmatter package run and
preserves the first run's timings as historical results. [Dated
corrections](log.md#review-5-verification-2026-11-24)
