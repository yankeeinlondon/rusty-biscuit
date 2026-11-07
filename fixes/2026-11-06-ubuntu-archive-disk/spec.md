---
kind: fix
name: ubuntu-archive-disk
date: 2026-11-06
status: draft-spec
next_spec: 2026-11-06-windows-wsl-test-failures
reviewed: false
implemented: false
---

# The Ubuntu archive build must finish, so WSL2 can run

## Why

WSL2 cells run archives built by `build (ubuntu-latest)`. On the Nov 5 and Nov 6
nightlies that job died after about 2.5 hours with `No space left on device`
(Nov 4 lost its runner, possibly the same cause). All 66 WSL2 cells then failed
with `Artifact not found`, so **WSL2 tests did not run at all**.

The run looks like a wall of WSL2 failures, but none of them is a test result.

## Outcome

- A full-workspace Ubuntu archive build completes within its timeout, with disk
  headroom to spare, and that headroom is measured in the job log.
- WSL2 cells produce real test results.
- When a producer dies, its consumers report "not run: producer failed" rather
  than each failing on a missing artifact.

## Scope

Choose from the measurements; the likely options are:

- free runner disk before the build and clear each key's output after archiving;
- split the one owner job, which builds up to 26 keys serially;
- reduce what each archive and its debug info hold.

## Spike needed

Record disk use per build key on one real run before choosing. Do not pick a
remedy from a guess at the largest consumer.

## Not in scope

Fixing whatever WSL2 tests fail once they run; that is
`2026-11-06-windows-wsl-test-failures`.
