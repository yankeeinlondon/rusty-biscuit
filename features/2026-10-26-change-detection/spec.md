---
kind: feature
name: change-detection
date: 2026-10-26
status: draft-spec
depends-on:
  - 2026-11-06-rust-ci-planner
related:
  - 2026-10-26-next-gen-pipeline
reviewed: false
implemented: false
---

# Change detection and blast radius

The detailed thinking is in [`detection-and-blast.md`](./detection-and-blast.md). The
"Better Evidence" text first drafted in this file moved to
`2026-10-26-next-gen-pipeline`, which is the spec for the evidence store.

## Why

Today a change is evaluated per package: touch a package and its whole test suite is in
scope. A precise statement of what changed, down to config keys and symbols, could mean
far less testing and better descriptions of what a change affects.

## Outcome

A `change-detection` library and CLI that wraps `tree-hugger` and `sniff` and describes
a change, by type, and what it reaches. The Rust planner can then use that to require
less evidence, wherever doing so is safe.

## Scope

The change types in `detection-and-blast.md`: configuration files, source code,
dependencies, and test coverage, plus the `kind` and `blast_radius` frontmatter grammar.

## Spikes needed

- **Is it safe?** Skipping tests because a symbol analysis says they are unaffected can
  hide real regressions from macros, traits, build scripts, and files read at run time.
  Find out how often static analysis would have wrongly skipped a failing test, by
  replaying history.
- **Is it worth it?** With evidence reused per package, measure how much test time
  remains that finer detection could actually remove.
- **`tree-hugger` readiness.** What is missing for symbol-level diffs and reference
  mapping across this workspace?

## Why this is last

It is an optimization of a working system and nothing else depends on it. Value exists
without CI too, in the CLI's descriptions of a change. If the "is it worth it" spike comes
back low, this can stop at that CLI.
