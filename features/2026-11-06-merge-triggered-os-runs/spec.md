---
kind: feature
name: merge-triggered-os-runs
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-11-06-any-host-evidence
  - 2026-11-06-windows-wsl-test-failures
next_spec: 2026-11-06-os-failure-reports
reviewed: false
implemented: false
---

# Prove Windows and WSL2 right after merge

## Why

Windows and WSL2 currently wait for a nightly, so a merged defect can sit for up
to a day. The hours of testing are fine if each result is only ever produced
once.

## Outcome

A merge to `main` immediately starts Windows and WSL2 runs for whatever those OSes
have no evidence for. Linux and macOS are not retested.

- Each run uses `--no-fail-fast`, already the case for L1, so a failing run still
  catalogs every failure.
- Passing and failing results are both recorded.
- Every commit on `main` has an answer per OS: passed, failed, covered by
  unchanged inputs, or superseded.

## Scope

- Add `push` to these environments' `events` in `.github/ci/environments.json`.
- One run per OS at a time, with a newer merge replacing a queued run. Inputs that
  were skipped this way get covered by the next run.
- Decide the nightly's fate: remove it, or keep it purely as a catch-up for runs
  lost to infrastructure.
- Include compile failures in the record, not only test failures.
- Update the event tables and duration figures in `CLAUDE.md` and
  `docs/topics/ci-cd.md`.

## Spike needed

Model the load: about 1.7 merges per merge day against runs of 3 hours or more.
Check for starvation of `main` and queue depth, using recent merge history.

## Note on language

This is mainly configuration. Fixing the existing Python planner is allowed until
its replacement lands (decided 2026-11-07); keep any scheduling change it needs
minimal, since `2026-11-06-rust-ci-planner` follows.
