---
kind: feature
name: os-failure-reports
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-11-06-merge-triggered-os-runs
next_spec: 2026-10-27-release-process
reviewed: false
implemented: false
---

# Turn OS failures into a fix pull request

## Why

Merge-triggered runs find Windows and WSL2 problems after merge, and those need
to be fixed forward. Today someone has to go and read the logs. A failing run
should instead produce a to-do list with a place to do the work.

## Outcome

When an OS fails or has slow tests on `main`, a single rolling draft pull request
for that OS appears, holding a generated `fixes/{date}-{os}-failures/spec.md`:

| Column | Content |
|---|---|
| Package and test | exact identity |
| Message | the first useful failure lines |
| First failed at | the first commit showing it, from the store |
| Duration | for slow tests, against a stated threshold |

- Fixes land on that same branch, and merging it closes the loop.
- A later run updates the existing pull request rather than opening another.
- Once the OS is green with nothing left to fix, the automation says so on the
  pull request.

## Scope

- Generating the report from the store rather than from logs.
- Reusing `scripts/slow-tests.rs` for slow-test detection where it fits.
- The rule for handling a pull request a human has already started working on.

## Open questions

- What counts as slow: an absolute limit, or relative to that test's history?
- Should the generated spec be replaced on each run, or keep failures that are no
  longer happening, marked as resolved?
