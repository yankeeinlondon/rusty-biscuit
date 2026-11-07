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
reviewed: false
implemented: false
---

# Make the existing release path work once

## Why

The last 200 Release-plz runs were 177 skipped, 23 failed, and none successful.
No pull request has ever carried the `release` label, and the last GitHub
Release was on July 12. Binaries cannot ship until tagging works.

This repairs the current path as the interim release mechanism. The long-term
process is `2026-10-27-release-process`.

## Blockers, in the order they appear

1. `main` goes red on merge: fixed by `2026-11-06-proven-push-planner-cell`.
2. The calculation dies checking out `9ba25e463`, reporting local changes to
   `scripts/init.ps1`. The `*.ps1 eol=crlf` rule is the suspect, but a macOS
   replay did not reproduce it, so the cause is unproven.
3. The repository setting blocks `GITHUB_TOKEN` from opening pull requests.
4. A pull request opened with `GITHUB_TOKEN` starts no CI, and `ci.yml` does not
   listen for `ready_for_review`, so required `ci-gate` never reports.
5. Tags and releases made with `GITHUB_TOKEN` start no other workflow, so nothing
   would start a binary build.

## Outcome

One real release happens end to end: a release pull request is opened, CI runs
on it, merging it creates a tag and a GitHub Release, and that release can start
a build workflow.

## Scope

- Run `Release Dry Run` on Linux to find the true cause of blocker 2, then fix it.
- Authenticate release-plz with a GitHub App token, which addresses blockers 3–5.
- Start the release calculation only after `push` runs of `ci`, not `schedule`.
- Publish only for a pull request from the release-plz branch that also has the
  label, and check out its merge commit rather than `main`.
- Filter `build-integrations.yml` so that it does not build on every package's
  release.
- Update `docs/releases.md`.

## Needs from Ken

Create the GitHub App and install it on the repository.

## Not in scope

Evidence-driven triggering, AI release notes, and `just release`, which belong
to `2026-10-27-release-process`.
