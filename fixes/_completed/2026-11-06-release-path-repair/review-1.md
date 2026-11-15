---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Release calculation remains unable to produce the approved proposal'
human_review: true
human_review_items:
  - Accept the isolated Linux diagnosis from the reachable build-linux host despite missing host declarations; no implementation depends on that host choice.
  - Accept applying the package restriction and adding tests earlier than the plan scheduled them; the delivered scope is unchanged.
  - Accept the commit-pinned Rust setup action and the plan's tag-based App-token action; no additional action-pinning change is requested.
  - Review the replacement of the nonworking release-plz pin and the bounded checkout repair after R1-F1 is fixed; the spec permits choosing a working version.
  - Accept treating merged, release-labeled proposals on release-plz branches as plausible proposals whose identity failures produce visible errors.
  - Accept comparing proposal versions against their merge base so later main changes cannot invent release candidates.
  - Accept keeping the Git credential helper entirely in the release process environment instead of creating a temporary helper file.
  - Accept the initial read-only inventory followed by the App-authenticated check before mutation, including refusal beyond 1000 Releases.
  - Accept omitting changelog-generation templates from publication configuration because publication reads the completed changelog.
  - Accept grouping Python cases into separate tests to meet the five-second budget, with a guard ensuring every case runs.
  - Accept interpreting empty successful release-pr output as no proposal and using release-plz configuration to restrict packages before the output is checked.
  - Accept fixing error locations and missing bot-login handling during the testing phase.
  - Accept source inspection plus a pinned-version structural guard for recovery; changing the version requires repeating that inspection.
  - Accept leaving malformed inherited Git configuration counts outside the event-input checks; this is runner configuration, not a realistic release event.
  - Accept the documented pending hosted dry run, App installation check, and prerequisite-on-main check; these remain explicit prerequisites to live release.
  - Complete the separately authorized proposal review, merge, and live release verification after repair; do not publish merely to satisfy this review.
  - Accept retaining focused integration routing coverage without releasing an extra package, and restoring workspace eligibility only in a later approved change.
  - Accept leaving general binary distribution, unchanged skill references, and additional HTTP composition tests outside this repair.
  - 'RULING NEEDED (R1-F1.7): sniff-detection-cli-v0.1.0, git-wt-cli-v0.1.0 and darkmatter-lsp-v0.1.0 point at 907cd6e, which predates the renames, and stop every git_only calculation. Choose between re-pointing them to 82e34966b (a spec amendment, because the spec forbids moving tags), an upstream release-plz change, or another approach.'
  - Confirm the pin of release-plz 0.4.3 (newest stable) instead of the 0.4.2 used in the Phase 1 replay.
  - Confirm that the process-scoped attributes file carries only the PowerShell rule, not a broader text=auto default that would change other historical checkouts.
  - Confirm that publication runs without the calculation attributes environment, because it checks out only the proposal's final commit.
  - Confirm that the dry run calls `release_path.py update`, so it shares the calculation environment with `release-pr` through code rather than duplicated workflow YAML.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: '2026-11-08T19:41:06-08:00'
spec: 2026-11-06-release-path-repair/spec.md
implemented: false
description: 'A **fix** review of `2026-11-06-release-path-repair/spec.md`'
fix: 2026-11-06-release-path-repair/review-1.md
next: 2026-11-06-release-path-repair/review-2.md
log: /Volumes/coding/rusty-biscuit/fixes/2026-11-06-release-path-repair/log.md
implemented_by: claude/opus
---

# Release path repair review, iteration 1

The repair is not production ready. The workflow safeguards and focused tests pass, but the central operation—calculating the two-package proposal—still fails. No human decision is needed to repair this: the spec requires one explicit, working version, rather than the particular version chosen by the implementation plan.

## Findings

### R1-F1 (high): Release calculation remains unable to produce the approved proposal

**Authority: spec.** Acceptance item 1 requires: “The Linux dry run succeeds and the historical checkout failure has a documented cause and focused regression check.” The first-release ruling additionally requires confirming that the proposal contains exactly `sniff-detection-cli` and `git-wt-cli`, without an unreleased dependency bump. Neither result exists. The implementation log explicitly records the repaired calculation as failed.

The defect class is a shared release-tool setup that cannot calculate the repository it is configured to release.

The [shared setup action](../../.github/actions/setup-release-plz/action.yml) installs release-plz 0.3.165 for all three jobs. The recorded Linux replay with the repair configuration fails when that version packages the historical workspace: `homelab/lib`'s dependency on `schematic-schema` has no version requirement. Restricting eligibility to two packages does not prevent this workspace-wide packaging. Consequently the dry run and proposal preparation cannot reach their required results.

The historical checkout cause is now explained, but remains unrepaired. The log's newer-tool replay reaches the original CRLF failure, and the passing regression intentionally asserts that Git aborts. Neither [the dry-run invocation](../../.github/workflows/release-plz.yml) nor [the preparation environment](../../scripts/ci/release_path.py) supplies the demonstrated process-scoped attribute workaround. Merely changing the tool pin would therefore leave the diagnosed failure in place.

The sweep covers every consumer of the shared setup and the relevant decision that accepts its result. Existing Linux calculation evidence was reused rather than repeating the plan's bounded live experiment; the hermetic history-walk regression was rerun in this review.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | Release Dry Run and shared setup | Actual repair configuration with the approved two-package set, Linux replay recorded in Phase 1 | 0.3.165 exits during historical workspace packaging; no proposal edits | Successful calculation showing exactly the approved package/version changes and generated files |
| R1-F1.2 | Create Release PR and its process environment | Same configuration/tool; recorded historical `release-pr` replay and current environment inspection | Shares the incompatible pin; newer-tool historical replay also aborts on CRLF checkout | Calculation can complete without suppressing clean-tree checks or changing user checkouts |
| R1-F1.3 | Historical-checkout regression | Unchanged blob across removal of its CRLF attribute rule, with stale index metadata | Regression passes by confirming checkout failure; no repaired-path success case | Preserve the failure reproduction and add a focused successful reproduction using the actual repair |
| R1-F1.4 | Publish Release | Shared pin plus hermetic publication/recovery cases | Clean for pin consistency and orchestration; real release-plz is replaced by a recording fake | Keep exact-head publication and tag-only recovery valid under whichever working version replaces the pin |
| R1-F1.5 | Calculation report and proposal candidate checks | Existing fixture changes and exact-package-set checks | Clean: reports changes and refuses unexpected packages; publication requires both approved packages | Retain these checks and provide an actual successful initial calculation as acceptance evidence |

**Recommended repair:** choose a release-plz version that avoids the demonstrated historical workspace-packaging failure, then apply the smallest process-scoped fix for the demonstrated CRLF history walk. The log's newer-version candidate and process-local `core.attributesFile` are a reasonable starting point, not proof of a complete repair. Apply the same calculation environment to dry run and preparation; recheck publication and missing-Release recovery against the chosen version's source, update their pin guard and documentation, and obtain one bounded Linux success showing the exact approved proposal. Do not edit immutable historical tags or rewrite unrelated manifests to satisfy the old tool. Keep the original CRLF rule and normal clean-tree checks.

The plan's chosen pin and single-spike limit explain why the implementer stopped, but do not satisfy the spec's outcome. A repair iteration can address the now-demonstrated causes without asking the author to choose an implementation detail. Credential creation, pushing changes, merging, and publication still require their existing separate authorization.

## Verification and limits

- Re-ran the focused Nextest selection: **34 passed**, no selected test skipped, longest test under two seconds. The 173 filtered tests were outside this selection.
- Ran actionlint 1.7.12 on both changed workflows: passed.
- Confirmed the test target is compiled by Cargo's automatic integration-test discovery and these ordinary test names remain in Level 1. The case-group guard verifies that every Python driver case belongs to exactly one group.
- Reviewed selection, topology, candidate discovery, credentials, remote-state classification, per-package configuration, recovery, integration selection/build/upload, and their caller wiring. No additional actionable defect was established.

The appropriate verification level here is Level 1 for event/configuration decisions, isolated Git operations, and HTTP response handling. Those boundaries have executable fixtures. Workflow text assertions establish permissions and wiring; they do not establish that release-plz itself can calculate or publish. The Linux calculation is the required real-tool check and is failing. Live App authentication, proposal CI, merge, publication, tag targets, and release-event delivery remain pending as the spec explicitly permits before normal review and merge. They are not additional findings or demands for duplicate cross-OS evidence.

### Input robustness

Reviewed the JSON event/API loader and TOML configuration/manifest readers. Default invalid-field behavior is a located error; unrelated supported events deliberately select no work. Shared validators distinguish absent and null, reject wrong whole-field types and malformed collection members, and preserve explicitly valid empty label lists. Optional configuration settings retain release-plz defaults when absent.

The passing public-function/command cases cover shared scalar and label validators, selector wiring, JSON duplicate keys/trailing content, TOML duplicate keys/invalid content/equivalent syntax, API failures versus absence, and rendered selection errors before publication. Manifest-version fixtures exercise real Git revisions and candidate decisions. Smell searches found none of the listed Rust coercion patterns in the changed production scripts or Python driver. This review does not turn synthetic runner environment corruption or hypothetical configuration extensions into release requirements.

## Implementation decisions (informational)

Two agents independently read the implementation log and spec before comparison. All items below can proceed with a recommendation within the spec; none passes the test for stopping for a human. The frontmatter lists them for optional author review after repair.

- **Linux host:** accept the isolated diagnosis from the reachable host. It produced useful failure evidence and changed no host configuration; successful acceptance evidence is still required by R1-F1.
- **Phase ordering:** accept moving the allowlist and initial tests earlier so the experiment used the intended configuration. Accept tests added alongside Phase 2 implementation.
- **Action references:** accept the commit-pinned Rust action, the plan's App-token action tag, and unchanged integration toolchain reference. Broader pinning policy is not required by this spec.
- **Nonworking tool pin and proposed attributes repair:** replace the pin and implement the demonstrated repair under R1-F1. The spec did not dictate 0.3.165, and the author did not rule that this version must remain.
- **Proposal identity:** accept the defined boundary between unrelated pull requests and plausible release proposals with invalid identities. It preserves the required identity checks without failing unrelated labeled work.
- **Candidate comparison:** accept using the proposal's merge base to exclude unrelated changes on main. Exact merge topology and final-head tag checks remain enforced.
- **Credentials:** accept the process-local helper instead of a temporary helper file. It resets inherited helpers and avoids a credential-bearing disk artifact.
- **Inventory:** accept the initial read-only report and the repeated App-authenticated check before mutation. Accept the conservative 1000-Release scan bound; it fails rather than treating unknown state as absent.
- **Generated configuration:** accept omitting the changelog-generation table because publication consumes existing notes. Retain the package changelog paths and release settings.
- **Test grouping:** accept multiple driver launches to stay below five seconds, with the complete-case guard. Test count alone is not used as evidence of real-tool compatibility.
- **Proposal output:** accept empty successful output as a no-op, and post-operation output validation backed by the release-plz allowlist. Reconfirm actual output behavior when changing the pin.
- **Testing-phase fixes:** accept the located configuration/API errors and early missing-login error. They fix defects within the authorized scope.
- **Upstream compatibility evidence:** accept documented source inspection and a pin guard instead of a vendored upstream excerpt. R1-F1 must refresh that inspection when the tool changes.
- **Runner-only gaps:** accept leaving malformed inherited `GIT_CONFIG_COUNT` outside this repair. Accept test isolation of ambient API/summary variables and omission of another full publish-over-HTTP composition test; constituent behavior is exercised.
- **Unavailable external checks:** keep hosted workflow dispatch, the prerequisite's passing run on main, and App installation verification explicitly pending. Missing credentials are not evidence that setup works, and the log correctly says so.
- **Live acceptance:** leave the real proposal, passing check, merge, publication, tag-target inspection, and release-event delivery to the separately authorized post-review release. No extra integration release is warranted.
- **Explicit later scope:** retain the two-package restriction. Workspace-wide restoration is a separate approved follow-up; general binary distribution belongs to later work. Leaving an unchanged dated-spec reference outside the edited skill section does not alter current behavior.

There are no previous implementation-review findings to carry forward in iteration 1.
