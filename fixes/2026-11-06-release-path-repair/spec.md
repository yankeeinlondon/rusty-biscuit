---
kind: fix
name: release-path-repair
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-11-06-proven-push-planner-cell
next_spec: 2026-10-28-publish
related:
  - 2026-10-27-release-process
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-07
review_iterations: 0
implemented: false
$schema:
  status: |-
    enum(
        draft-spec,
        finalized-spec,
        planned,
        implemented,
        review-findings,
        human-in-the-loop,
        completed,
        on-hold,
        abandoned
    ) -> an indicator of progress for this specification
  reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
  reviewed_by: string -> the agent and model used in the spec review
  reviewed_on: date -> the date the spec was reviewed
  review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
  clarified: boolean -> indicates whether the specification was built -- in part -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
---

# Make the existing release path work once

## Why

The investigation recorded on November 6 reported that the last 200
Release-plz runs were 177 skipped, 23 failed, and none successful. It found no
pull request carrying the `release` label and a last GitHub Release dated
July 12. These are observations from that investigation, not live counts.
Binaries cannot ship through the planned release pipeline until tagging works.

This repairs the current path as the interim release mechanism. The long-term
process is `2026-10-27-release-process`.

## Blockers, in the order they appear

1. `main` goes red on merge: repair is specified by the prerequisite
   `2026-11-06-proven-push-planner-cell`; it must be verified before the live
   release, rather than assumed fixed because a spec exists.
2. The calculation dies checking out `9ba25e463`, reporting local changes to
   `scripts/init.ps1`. The `*.ps1 eol=crlf` rule is the suspect, but a macOS
   replay did not reproduce it, so the cause is unproven.
3. The repository setting blocks `GITHUB_TOKEN` from opening pull requests.
4. The default token does not provide unattended pull-request validation.
   Current GitHub documentation describes approval-required runs for some
   pull-request events; other events remain suppressed. An App installation
   token removes the approval step. `ci.yml` already runs on draft pull
   requests (its `pull_request` trigger has no draft filter), so nothing more is
   needed for a draft proposal to receive `ci-gate`.
5. Tags and releases made with `GITHUB_TOKEN` start no other workflow, so nothing
   would start a binary build.

## Outcome

One reviewed release pull request is opened or updated, receives the required
`ci-gate` check, and creates package tags and published GitHub Releases when
merged. Each tag points at the release pull request's final commit: the
revision that was reviewed and passed `ci-gate`. Events created by the
release credentials can start a release consumer without a manual dispatch.

This proves release preparation and publication. General CLI binaries and
installers belong to `2026-10-28-publish`; this repair does not promise them.
The existing integration build is the only binary consumer in this scope.

## Scope

The changes belong to `.github/workflows/release-plz.yml`,
`.github/workflows/build-integrations.yml`, and any configuration or focused
regression coverage required by the diagnosis below.

### Diagnose the calculation before choosing a fix

Run **Release Dry Run**, the manual job inside `release-plz.yml`, against the
repair branch on Linux. Use the same explicit release-plz version in the dry
run and the two publishing jobs, so success tests the tool that will ship.
Use the repository's pinned Rust toolchain in these jobs.

Record the failing command, source revision, release-plz and Git versions,
effective attributes, and tracked-file state before and after the failure.
Inspect `scripts/init.ps1` in the index and working tree and at the historical
checkout named in the error. Retain diagnostics even when calculation fails.
Reproduce in a disposable checkout; never reset or clean the author's tree.

Fix the demonstrated cause. Do not remove the CRLF rule, normalize unrelated
files, disable clean-tree checks, or force historical checkouts merely to
suppress the error. If attributes must change, explain the effect on PowerShell
users and verify that specific behavior on native Windows. A Linux/macOS
discrepancy alone does not justify a new all-OS CI matrix.

The dry run has read-only repository permissions and receives no App token or
private key. It may change its disposable working tree. Report tracked changes
and newly created, non-ignored files, including new changelogs, on success;
report available diagnostics on failure without concealing the failing exit
status. The production calculation must still leave its checkout clean after
committing its proposal. Correct the workflow comments that currently imply
`release-plz update` and `release-pr` should leave the same local diff.

### Use one repository-scoped GitHub App

Create a short-lived installation token separately in preparation and
publication, scoped to this repository. Give the App **Contents: write** and
**Pull requests: write**, plus GitHub's implicit metadata access. It needs no
branch-protection bypass or permission to change workflows. Store its ID as
`RELEASE_APP_ID` in repository variables and its private key as
`RELEASE_APP_PRIVATE_KEY` in repository secrets. Missing configuration must
produce an actionable failure, never a fallback to `GITHUB_TOKEN`.

Supply the installation token to release-plz's API requests and its Git
authentication. In particular, do not leave checkout's persisted default-token
credentials in place for tag pushes. Mint tokens near their use and retain the
token action's normal cleanup. Ordinary CI and the read-only dry run keep their
existing permissions. Do not change human Git identity or signing settings.

**Reader's note:** using the App is an intentional credential change, not a
relaxation of the repository's pull-request creation setting or merge rules.
It also allows release events to reach later workflows. GitHub's own
[token rules](https://docs.github.com/en/actions/concepts/security/github_token)
take precedence over the broader suppression claim in release-plz's
[token guide](https://release-plz.dev/docs/github/token).

### Prepare only the revision that passed CI

Require a completed, successful `ci` workflow whose triggering event was
`push`, whose branch was `main`, and whose source repository is this canonical
repository. A successful nightly, manual CI run, or pull-request run must not
calculate a release. Preserve the existing named-branch checkout with full
history and an upstream: preparation needs that branch, not a detached commit.
Compare its revision with the triggering run's `head_sha` and stand down with
a notice if `main` has advanced. Keep release preparation serialized without
canceling a running calculation. Preserve the existing `ci` run-conclusion
contract; do not add another interpretation of its plan or evidence.

Keep proposals in draft with the configured `release` and `automated` labels.
Do not change `ci.yml`'s pull-request triggers: CI already runs on drafts, and
adding `ready_for_review` would add a full CI run to every pull request in the
repository when it leaves draft, without answering any new question. Do not
add a second test scheduler or grant the App a way around `ci-gate`.

### Publish only the merged release proposal

Require all of these before creating credentials or running publication:

- The pull request was merged into `main` in the canonical repository.
- Its head repository is also the canonical repository, not a fork.
- Its branch starts with the configured release-plz branch prefix
  (`release-plz-` by default), and it has the `release` label.
- Its author is the configured App's bot account. Document that account at
  setup; a branch prefix and label alone do not establish authorship.

Check out `pull_request.merge_commit_sha` with full history and assert that
the checked-out revision matches. Do not fetch the current `main` as the
publication target. Verify the pinned tool's actual tag targets. Its
[release documentation](https://release-plz.dev/docs/usage/release) says that
after a merge commit it checks out and tags the release pull request's final
commit, an ancestor of the merge commit. After a squash or rebase merge it
finds no such commit and tags whatever `main` holds. This repository allows
merge commits and rebase merges; squash is disabled. Require the merge-commit
strategy for release pull requests, and refuse to publish when the merged
proposal's final commit is not an ancestor of `merge_commit_sha`. Every new tag
must resolve to that final commit, never to a later commit on `main`. Never
repair a wrong published tag by moving it.

Serialize publication without cancellation. Preserve `git_only = true`,
`publish = false`, the `{package}-v{version}` tag format, package exclusions,
and release-plz's ownership of versions, changelogs, tags, and GitHub Releases.
Keep ordinary pushes unable to publish. A later advance of `main` must not
change the package versions or source released by an earlier merge event.

On partial failure, report which tags and releases already exist. A rerun of
the same event must safely complete missing work without changing an existing
tag or creating duplicate releases. Fail visibly on an existing tag pointing
at another revision. Keep retry instructions tied to the original merge
revision; a manual calculation dispatch must remain unable to publish.

### Route integration releases to the matching build

For each published release, select only the matching integration package:
`arcam-amp-integration`, `eversolo-integration`, or `sony-receiver-integration`,
using its complete `{package}-v{version}` tag. An unrelated package's release
starts no integration build. Check out that tag's resolved commit and upload
only that package's archive to that release. Preserve the existing
`aarch64-unknown-linux-musl` target; add no target or installer matrix here.

### Keep the current documentation accurate

Update `docs/releases.md` with App setup, draft-to-ready behavior, exact
publication revision, dry-run diagnostics, and partial-failure recovery.
Update the release contract in the `rust-devops` skill to match the stricter
trigger and proposal checks. Describe planned general binary distribution as
planned. Documentation describes current behavior without linking to this
spec or delegating instructions to another dated spec.

## Verification and acceptance

Use focused coverage in the `test-toolkit` package's
`tools/test-toolkit/tests/ci_workflow_contracts.rs`, plus `actionlint`, to check
the workflow changes. Include positive and negative cases for the event
filters, proposal identity, exact revision, dry-run permissions, token wiring,
and integration package selection. Exercise executable selection logic with
small event fixtures rather than relying only on matching strings in YAML.
Run these through the existing Nextest/Just test path. No full-workspace run,
new permanent CI cell, performance spike, or repeated live release is needed.

Before declaring the implementation ready for review:

1. The Linux dry run succeeds and the historical checkout failure has a
   documented cause and focused regression check. Expected version/changelog
   edits are visible, including newly generated files.
2. Focused contracts and workflow lint pass. Cases for a nightly completion,
   stale `main`, an unrelated labeled pull request, and an unrelated package
   release demonstrate that they cannot publish or build unexpectedly.
3. Record the prerequisite's verification and the App setup status. If either
   is unavailable, report live verification as pending, not complete.

After normal review and merge, complete one author-approved real release.
Record the proposal, its passing `ci-gate`, merge revision, publication run,
tags, and GitHub Release URLs in the implementation log. Verify the tag
targets and the delivery of a published-release event. If the approved set
contains an integration, its matching archive build and upload must succeed;
otherwise that consumer remains covered by focused tests until an integration
is genuinely released. Do not release an extra package merely to test routing.
The general binary pipeline remains the next spec's acceptance work.

## Needs from Ken

Create and install the repository-scoped App, provide the variable and secret
above, and identify its bot account. Review the generated package/version set
and merge the release proposal through normal protection. This spec review
does not authorize creating credentials, merging a proposal, or publishing.

## Open Questions

### Which packages should the first real release include?

The workspace-wide calculation may propose many packages after the long gap.
Decide the first release set before merging; a successful calculation is not
approval of every proposed version.

- **Review the complete generated proposal (recommended).** Preserves the
  existing package-selection policy and releases related version changes
  together. The cost is a potentially large review. Recommend this because
  this repair is meant to restore the configured path without introducing
  another package-selection policy. Keep it in draft until review finishes.
- **Temporarily allow only a small named set.** Limits the first publication's
  reach and review burden. It changes release eligibility and requires checking
  dependent versions and restoring the intended configuration afterward. If
  chosen, document the exact packages and restoration step before implementation.

## Not in scope

Evidence-driven triggering, AI release notes, and `just release`, which belong
to `2026-10-27-release-process`.
