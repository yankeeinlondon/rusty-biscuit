---
kind: feature
name: legible-ci-names
date: 2026-11-06
status: draft-spec
next_spec: 2026-11-01-rust-pre-push
reviewed: false
implemented: false
---

# CI names a person can read

## Why

The Actions tab should explain what is happening. Today it shows:

```text
area-ci (research) / package-ci / test
area-ci (schematic) / package-ci / wsl2 (schematic-gen, L1, wsl2-ubuntu, windows-latest) / test (wsl2-ubuntu)
```

These describe how the workflows are wired, not what the job is doing, and
terms like `L1`, `area-ci`, and `cell` need insider knowledge.

## Outcome

A newcomer can read the job list and answer: which package, doing what, on which
OS, and why it failed. For example:

```text
research: run tests on Linux
schematic-gen: run tests on WSL2
messenger: compile dependents on Linux
```

## Scope

- A naming convention, covering workflows, jobs, and steps.
- Rename to match it, and state it in `docs/topics/ci-cd.md`.
- Name skipped work so that it reads as intentional, or stop showing it.
- Keep stored identifiers keyed on `{package, environment, gate}`. This renames
  what people see, not artifact names or evidence keys.

## Spike needed

GitHub always prefixes reusable-workflow jobs with the caller's job name. Test
how short that prefix can get, and whether readable names require flattening
`_area-ci.yml` and `_package-ci.yml`.

## Open question

`ci-gate` is the required status context in the `protect-your-bacon` ruleset.
Renaming it means changing the ruleset in step. Keep the name unless there is a
clearly better one.
