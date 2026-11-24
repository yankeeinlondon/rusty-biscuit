---
$schema: feature-review.yaml
ready: false
findings:
  - "R4-F1 (high): The registration writer validates an aliased user file as repository policy"
  - "R4-F2 (high): Doctor offers repairs that the YAML editor refuses"
  - "R4-F3 (high): Recorded native Windows L1 tests exceeded the timing gate"
  - "R4-F4 (low): The guide says Doctor reports a publisher lock that it can leave unreported"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.2-sol
recurrence: true
created: "2026-11-24T08:34:02+00:00"
spec: 2026-11-22-claudine-doctor/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-claudine-doctor/spec.md`"
feature: 2026-11-22-claudine-doctor/review-4.md
previous: 2026-11-22-claudine-doctor/review-3.md
next: 2026-11-22-claudine-doctor/review-5.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/log.md
implemented-by: claude/opus
---

# Claudine Doctor review 4

**Production readiness declined.** The three review 3 repairs passed their
required cases. The new findings cover a registration edit judged in the
wrong scope, unsupported YAML edits advertised as repairable, recorded
Windows test overruns, and one inaccurate sentence in the command guide.

## Previous Finding Verification

I used the child rows in the repair ledger as the verification inventory.
Reviews 1 and 2 have no open findings carried by review 3. No earlier
review contains a human ruling. [Review 3](review-3.md) ·
[Review 3 repair ledger](log.md#finding-ledger-2)

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R3-F1 Doctor offers permissions repairs whose reference the writer refuses | .1–.4, .c1–.c3 | **verified fixed** | The library tests proved both scopes and both link positions, with omitted, null, and equal inline settings. An in-scope reference-only repair succeeded. The external target stayed manual before and after `--fix` and the independent scope could still repair. Linked Claudine write targets remained manual, previously authored references still followed Darkmatter's rules, and registration's tested path, sidecar, entry identity, and invalid-register controls passed. [Permissions tests](../../lib/src/doctor/permissions_tests.rs) · [Registration tests](../../lib/src/doctor/registration_tests.rs) | None in these instances. R4-F1 and R4-F2 are additional writer checks that the earlier sweep missed. |
| R3-F2 Doctor misses broken Darkmatter references when a Claudine file is unavailable | .1–.3, .c1–.c3, .x1–.x4 | **verified fixed** | The library matrix exercised absent, malformed, and aliased counterparts, a legacy-only home, and both spellings of a valid target. Missing, malformed, chained, null, scalar, absent-block, nonmapping, and external repository targets all produced the expected diagnostics. Invalid declaration and invalid register controls remained visible, a shared Darkmatter file was checked once, and the handoff still skipped unavailable pairs. The CLI test proved location, exit 1, and preservation with both report formats. The registration writer still accepts a valid declaration for a broken target without editing the reference itself. The repo presence, stray key, and speech callbacks have no Darkmatter policy input. [Configuration tests](../../lib/src/doctor/config_tests.rs) · [CLI proof](../../cli/tests/l1/doctor.rs) · [Registration precondition](../../lib/src/composition/registration/inspect.rs#L576) | None. |
| R3-F3 Setup guidance overlooks the publication that must run first | .1–.4, .c1–.c3, .x1 | **verified fixed** | The tests reproduced setup refusal for linked registration paths, shared parents, the publisher lock, and a retired member. The suggested `doctor --fix` completed each handoff, including the case where registration was already healthy. Setup and registration controls, user and repository migration remedies, and both scope handoff tests passed. The missing-user-file test verified that the remedy explains publication first. The remaining remedy branches still name their own correction or diagnostic command. [Permissions tests](../../lib/src/doctor/permissions_tests.rs) · [Configuration tests](../../lib/src/doctor/config_tests.rs) · [CLI proofs](../../cli/tests/l1/doctor.rs) | None in the required cases. R4-F4 concerns the guide's description of the passing publisher-lock case. |

## Unblocked Findings

### R4-F1 (high) The registration writer validates an aliased user file as repository policy

**Authority (spec).** “When a repository file resolves to the same file as
the user configuration, validate it once as a user file.” Registration
repair must update the user entry and preserve other hosts' entries. A user
who manages their home as a Git repository, or links a repository's
`.darkmatter.yaml` to the home file, can get a failed registration repair
for valid user permissions. [Configuration scope](spec.md#configuration-files-and-unknown-keys) ·
[Registration repair](spec.md#darkmatter-registration)

The defect class is scope re-inference during an explicitly scoped write.
The scope used to read, validate, write, and verify a selected target must
stay the same. I published a temporary home, gave Darkmatter a valid user
permission for `~/notes/*.md`, and put another host's entry and a comment in
its block-style registration list. I gave Claudine a different inline
policy so the independent handoff would leave that policy alone. Doctor
read the user file successfully. Launching inside a separate repository
with a link to that file flipped the registration repair from success to
failure. Changing the policy to one both scopes accept restored success
with the link in place. Setup's detailed warning identified the repository
restriction. [Doctor's file observation in Claudine](../../lib/src/doctor/config.rs#L237) ·
[Transaction validation in Darkmatter](../../../darkmatter/lib/src/config/init.rs#L370)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R4-F1.1 | Doctor observation, registration eligibility, repair, and verification | Separate repository links its Darkmatter file to the valid user file. Repeat with and without link, and with a policy allowed in both scopes. | The linked file is read once as user. With the user-only grant, both repair attempts leave the original list intact and report the writer's refusal. The first run correctly lists the changed bundle member it already restored. The other controls append the entry and preserve the comment and other host. | Complete and verify the user edit regardless of the repository alias. |
| R4-F1.2 | Same decisions when the home is the Git root | Initialize a repository in the same home, then run there or outside it with the same files. | Inside the home repository the refusal reproduces. Outside it the update succeeds. | Keep the user's scope in either invocation. |
| R4-F1.3 | Setup's shared registration writer | Same fixtures and policy conflict, using `claudine setup`. | Setup warns separately about the conflict, then the aliased cases also warn that the home grant is forbidden for a repository. Distinct paths and the shared-policy control add the entry. | Register the user manifest even when the independent handoff reports a conflict. |
| R4-F1.c1 | Doctor permissions handoff and verification | Use matching user blocks with the same grant, separate and aliased paths, and the home as Git root. | All user handoffs completed. A repository alias stayed skipped. The handoff transaction explicitly validates the selected scope and verifies the resulting reference. | Preserve these results. |
| R4-F1.c2 | Other ways to publish or check the entry | Hidden publication command, passive entry lookup, and an already-registered bundle. | Publication changes only the bundle. Lookup reads the user file and compares manifest identity. An unchanged registration entry needs no configuration write. | Keep those paths independent of scope inference in the writer. |

Carry the caller's explicit scope into the scoped registration
transactions in Darkmatter, as Claudine's Doctor handoff already does.
Cover both Doctor and setup with the same fixture and keep verifying the
entry and legacy cleanup ordering. Darkmatter's separate
`register_manifest` API also receives an explicit scope and uses the same
scope-inference fallback when it stages a write. It has no production
caller in the checkout. Apply the same scope rule there if changing the
shared registration transaction. [Doctor registration writer](../../lib/src/composition/registration/repair.rs#L177) ·
[Setup registration writer](../../lib/src/config/setup.rs#L199) ·
[Explicit scope in the handoff writer](../../lib/src/config/setup.rs#L675)

### R4-F2 (high) Doctor offers repairs that the YAML editor refuses

**Authority (spec).** “Fixable means that an available automatic repair
applies to a finding in that row.” The registration repair preserves other
hosts, and the permissions handoff preserves unrelated settings and
comments. An inline YAML list such as
`register: [~/.other/manifest.yaml]` is an ordinary valid representation
which Doctor reads and marks fixable. The writer refuses to append an
entry inside the list. [Report contract](spec.md#command-and-report) ·
[Permissions handoff](spec.md#darkmatter-permissions-handoff)

The defect class is repair eligibility inferred from schema validity when
the format-preserving editor also has layout prerequisites. Moving the
same values between inline and block YAML preserves their meaning. The
report must either offer an edit the writer can perform or explain what
the person needs to change before retrying. I used a published home and
ran `doctor --json` and `doctor --fix --json` for every shape below, using
separate repositories for the repository handoff. In the refused cases
the row advertised the repair before and after the attempt. [Registration
assessment in Claudine](../../lib/src/composition/registration/inspect.rs#L576) ·
[Handoff assessment in Claudine](../../lib/src/doctor/permissions.rs#L85) ·
[Edit staging in Darkmatter](../../../darkmatter/lib/src/config/write.rs#L548)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R4-F2.1 | Registration insertion, replacement, and duplicate removal | A nonempty inline list with another host, with a retired Claudine entry, or with both current and retired entries. Use a legacy file in the last two cases. Repeat with equivalent block sequences. | All three inline cases fail the configuration edit. All three block sequences repair and preserve the other host. Setup gives the same inline append refusal. | Use an edit supported by the existing writer or give a file- and setting-specific manual remedy before publication. Keep the old layout until the entry verifies. |
| R4-F2.2 | Registration in an inline root mapping | Put a valid reference and empty `register` in an inline root mapping. Rewrite the same keys as a block mapping. | Inline root refuses the edit even with an empty list. The block root accepts it. | Report the root layout prerequisite before attempting an edit. |
| R4-F2.3 | Handoff writing Claudine's file, both scopes | A valid `{canonical_provider: claude}` in Claudine's file, with an inline Darkmatter policy to transfer. Change only Claudine's root to block form. | Both scopes refuse the inline root. Block form saves both files. | Identify Claudine's root layout before offering the transfer. Leave the original policy in force until a supported edit succeeds. |
| R4-F2.4 | Handoff writing Darkmatter's file, both scopes | Put the same inline policy in a nonempty inline Darkmatter root. Try with an equal Claudine block and with no Claudine block. | The reference-only and two-file transactions both refuse the inline root. The two-file cases leave Claudine's file unchanged too. | Identify Darkmatter's root layout. Preserve the writer's all-candidate check and allow the independent scope to proceed. |
| R4-F2.c1 | Registration field validator and unchanged entry | Omitted, explicit null, empty `[]`, and a valid inline list with the current and other host entries already in place. Exercise cleanup in the last case. | The first three insert successfully in a block root. Cleanup succeeds with the inline list byte-for-byte unchanged when its entry needs no edit. Whole-field, mixed-item, all-invalid, and invalid-reference controls already refuse invalid fields before writing. | Keep the applicable edit decision. Inline layout is only relevant when the planned edit reaches it. |
| R4-F2.c2 | Handoff field validators and unchanged counterpart | Block-root `permissions: {network: deny}`, empty Claudine root `{}`, and an existing equal policy inside a nonempty inline Claudine root. | The transfer from an inline policy value works, the empty root accepts its first key, and the reference-only edit succeeds when it leaves Claudine's inline root alone. Missing and null controls retain their defaults. Broken and incompatible policies remain manual. | Apply the layout check to planned edits and the actual editor behavior. Avoid refusing whole documents based only on the presence of an inline collection. |
| R4-F2.c3 | Setup registrations and shared handoff plan | Block-list registration control and inline-list reproduction in setup. Compare the shared setters that prepare the permissions edit. | Setup's registration failure names the inline collection. Its handoff still uses the same edit staging, with its own warning path rather than Doctor's `fixable` decision. | Preserve setup's diagnostics and check the same shared edit rules if support is extended there. |

Reuse the writer's existing edit feasibility rules for the planned
operations. If the existing editor cannot preserve a particular layout,
name the affected file and tell the user to rewrite the affected root or
list in block form before retrying. That is smaller than extending the
whole YAML editor. Preserve the supported empty and unchanged cases and
check known obstructions before registration makes changes. [YAML edit
implementation in Darkmatter](../../../darkmatter/lib/src/yaml_edit.rs#L154) ·
[Repair ordering in Claudine](../../lib/src/composition/registration/repair.rs)

### R4-F3 (high) Recorded native Windows L1 tests exceeded the timing gate

**Authority (review acceptance rule, “A Slow Test Is a Failing Test”).**
An L1 test over five seconds in the package run requires attention even
when nextest exits zero. A test at or above ten seconds fails regardless
of load. The latest Claudine repair run recorded a 5.003-second case, and
the previous repair log records nine Darkmatter cases, including a
10.6-second case. [Latest repair verification](log.md#completion-report-2) ·
[Earlier repair verification](log.md#progress-1)

The defect class is tests that exceed their L1 budget when the package runs
in parallel. Each case must meet the same assertions within five seconds
in the package run, or meet the review's documented loaded-host exception.
I reran each case alone on the existing Windows rig binaries. The rig had
12 logical processors. CPU use before and after the isolated groups was
4–15%. The original package-run evidence does not record a load
measurement that qualifies for the exception. The fast isolated results
point to contention under the package run.

| Instance | Site | Shape tested | Observed package run / isolated | Expected |
| --- | --- | --- | --- | --- |
| R4-F3.1 | Claudine approval persistence, [saved choice test](../../lib/src/composition/preflight/tests.rs#L1338) | Three saved approval choices, each reread twice without prompting | 5.003 s / 2.83 s | Complete within 5 s in the Windows package run. |
| R4-F3.2 | Darkmatter [glob implementation guard](../../../darkmatter/lib/tests/l1/glob_implementation_guard.rs#L217) | Repository source guard for the shared glob implementation | 10.6 s / 2.23 s | Same budget. The recorded duration exceeds even the loaded-host ceiling. |
| R4-F3.3 | Darkmatter [body shell trigger](../../../darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs#L743) | Denied child-only trigger with a preapproved body shell | 9.5–9.6 s for the pair / 1.89 s for this test | Same budget. |
| R4-F3.4 | Darkmatter [frontmatter shell trigger](../../../darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs#L748) | Denied child-only trigger with a preapproved frontmatter shell | 9.5–9.6 s for the pair / 1.87 s for this test | Same budget. |
| R4-F3.5 | Darkmatter [nested trigger gate](../../../darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs#L776) | Denied child-only trigger before a nested gate | 7.5 s / 1.23 s | Same budget. |
| R4-F3.6 | Darkmatter [environment anchor](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L968) | Unsafe link categories across an environment-based anchor | 6.0–7.8 s for the group / 2.26 s for this test | Same budget. |
| R4-F3.7 | Darkmatter [home anchor](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L985) | Same categories across a home-based anchor | 6.0–7.8 s for the group / 2.21 s for this test | Same budget. |
| R4-F3.8 | Darkmatter [repository anchor](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs#L922) | Same categories across a repository-based anchor | 6.0–7.8 s for the group / 2.23 s for this test | Same budget. |
| R4-F3.9 | Darkmatter [recursive trigger merge](../../../darkmatter/lib/tests/l1/host_trigger_merge.rs#L912) | A reference and its equivalent inline form at multiple nesting positions | 6.7 s / 1.80 s | Same budget. |
| R4-F3.10 | Darkmatter [repository boundary test](../../../darkmatter/lib/tests/l1/file_tree_roots.rs#L180) | Relative path leaving its repository | 5.6 s / 2.09 s | Same budget. |
| R4-F3.c1 | Focused Linux check of the Claudine approval test and Doctor scope | Same saved approval choices and the Doctor library and CLI slices | 1.008 s for approval persistence. No selected Doctor test reached 5 s. | Keep the assertions and the L1 selection. |

Start with repeated fixture and discovery work. Claudine's approval test
builds a new request for each of its nine approval passes and opens the
saved permissions separately for verification. One request context per
fixture and independent tests per approval choice retain the proof.
For the Darkmatter cases, measure fixture discovery and file scanning in
the package run and reduce repeated setup or directory traversal where
the measurements show it. Verify package-run timings on the Windows rig
after the changes. [Claudine test helper](../../lib/src/composition/preflight/tests.rs#L1248) ·
[Test speed guidance](../../../.claude/skills/rust-testing/test-speed.md)

### R4-F4 (low) The guide says Doctor reports a publisher lock that it can leave unreported

**Authority (defect).** The command guide includes a linked publisher lock
among the setup obstructions, then says the registration row keeps
reporting the link after the independent handoff. With a healthy installed
bundle, a linked lock blocks setup, but the Doctor registration row stays
`ok` before and after the handoff. The passing regression test covers this
case. Narrow that sentence to obstructions that Doctor's registration row
actually reports and retain the healthy-lock example. [Command guide](../../docs/cli/doctor.md#darkmatter-permissions) ·
[Publisher lock regression test](../../lib/src/doctor/permissions_tests.rs#L497)

## Blocked Findings

None. All four findings have a repair within the existing contract.

## Recurrence

- **R4-F1** and **R4-F2** are missed siblings of **R2-F7** in
  [review 2](review-2.md#r2-f7-high-permissions-repair-is-offered-for-an-invalid-darkmatter-file)
  and **R3-F1** in
  [review 3](review-3.md#r3-f1-high-doctor-offers-permissions-repairs-whose-reference-the-writer-refuses).
  The earlier fixes reconciled whole-file validity and the permissions
  reference's containment. The sweep did not follow the observed user
  scope through the registration transaction's validation or exercise the
  actual editable layout across the registration and both handoff targets.
  The tables above cover those decisions. The recurring boundary is the
  available repair promised by the spec. Reusing explicit scope and the
  existing editor's feasibility check addresses it without adding another
  file format or a separate YAML policy.
- **R4-F4** is a documentation sibling introduced by the **R3-F3** repair.
  The executable guidance works for the publisher lock. The guide's claim
  about the registration row conflicts with the regression test added in
  that same repair. [Review 3 setup guidance](review-3.md#r3-f3-high-setup-guidance-overlooks-the-publication-that-must-run-first)

## Verification and input robustness

| Requirement | Strongest evidence reviewed |
| --- | --- |
| Early dispatch, help and completion, seven rows, report formats and exit statuses | L1 CLI and completion tests. Real subprocesses in temporary homes verify redirected and plain output, usage status, no setup or conversion during inspection, and success and failure after repair. The requirements do not depend on emulator rendering or keyboard input. |
| Configuration scope, defaults, semantic and unknown-key findings, and independent broken-reference diagnosis | L1 library and CLI. The shared YAML reader's duplicate key, trailing invalid content, second document, malformed YAML, non-string keys, BOM, explicit null, empty-document, and empty-mapping cases are covered. CLI proofs cover a valid control, a duplicate, and a nested unknown key. Broken targets are located at the declaration. |
| Darkmatter registration manifests and registration repair | L1 library and CLI over published fixtures cover installed admission, wrong digest, manifest refusal, legacy cleanup, failures after partial publication, no-follow checks, and idempotence. Scope and edit layout remain open in R4-F1 and R4-F2. |
| Permissions handoff and scoped policies | L1 library and CLI cover omitted, null, valid mapping, equal and conflicting inline blocks, wrong whole type, invalid entries, valid and broken referenced targets, partial transaction failure, and retry. Scoped `file-read: []` follows the existing shipped validators, as recorded during implementation. Layout remains open in R4-F2. |
| Speech and executable discovery | L1 controlled candidate and executable cases verify disabled, automatic, pinned, unknown-provider, quality and ordering, and current-process presence and absence. Sniff's subprocess fixtures verify installed and absent discovery, bounded timeout, spawn and exit failure, and native invocation arguments. Earlier repair evidence exercised both PowerShell variants against a fresh profile. These requirements need no keyboard or visible terminal. |
| Documentation and caller guidance | Reviewed the README and CLI index links, command guide, configuration and registration topics, and caller skill. The guide discrepancy is R4-F4. |

The file checks use the shared YAML reader before applying their schema
rules. Invalid documents and fields report the file and available
location, with eligible independent repairs still allowed under `--fix`.
A legacy JSON file is checked for existence. Null and omitted settings use
the defaults defined by each schema, with separate edit coverage for
omitted, null, and explicitly empty registration lists. I reviewed the
load-bearing conversions in the Claudine wire reader, Darkmatter's
`permissions` and `register` validators, manifest admission, and Doctor's
projection for unchecked defaults and discarded errors. Wrong types and
invalid elements produce located findings in the focused reader tests.
R4-F2 concerns valid YAML reaching a different boundary, the
format-preserving writer.

On Linux, the focused Claudine library and fixture run passed 288 tests,
the Doctor CLI slice passed 25, the setup and registration CLI slice
passed 57, and the Darkmatter and Sniff reader and probe slice passed 133.
The tier coverage check found no stranded tests in the three areas. The
Windows reruns in R4-F3 were isolated invocations of existing test
binaries. The package timings in that finding come from the repair
records, and no new full package run was used to replace them.

## Implementation record corrections

The repair log now identifies the Windows timing result as over budget
and distinguishes the linked publisher lock from obstructions the
registration row reports. The dated note records the package count,
isolated timing, and regression test used to verify those corrections.
[Repair log](log.md#review-4-verification-2026-11-24)
