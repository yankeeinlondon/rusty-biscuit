---
kind: feature
name: spec-dependency-gate
date: 2026-10-22
status: draft-spec
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
  clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
reviewed: false
clarified: false
implemented: false
human_review: false
---

# Refuse to implement a spec before its prerequisites are completed

## Summary

A spec can name the specs it builds on in its `depends-on` frontmatter, by
their `{date}-{name}` directory. Nothing enforces that list today. The
implement prompts start a plan regardless of where its prerequisites are, so an
agent can build on code that has not landed, or on a design that the
prerequisite's review is still changing.

This feature adds one **prerequisite gate**: a repository command that reports
every `depends-on` entry of a spec as met or unmet, and exits non-zero when any
is unmet. The implement prompts run it before the first phase and refuse to
start when it fails.

```text
$ just spec-dependencies claudine/features/2026-10-29-recursive-schema-types/spec.md
met   2026-10-14-union-type-constraints (claudine/features/_completed/2026-10-14-union-type-constraints): completed
UNMET 2026-10-28-schema-enhancements (claudine/features/2026-10-28-schema-enhancements): not yet moved into `_completed`
1 of 2 prerequisite(s) in `depends-on` of claudine/features/2026-10-29-recursive-schema-types/spec.md are not met; refusing to start.
```

## Motivation

- The Claudine/Darkmatter rollout (`claudine/docs/rollout-strategy.md`) is a
  21-spec pipeline that is safe only when it runs "one at a time, in date
  order". Today that ordering is honored by convention alone; running a later
  spec by mistake gives no warning.
- Pipelines are increasingly run unattended (`just/flow.just`,
  `just/review.just`). An unattended run is exactly where a skipped
  prerequisite goes unnoticed until review.
- `implement-plan.md` already reads `depends-on` in the other direction: its
  final-phase "Pipeline Refresh" finds the specs that depend on this one.
  The forward check is the missing half.

## Prior art

An earlier agent prototyped this on the `fix/spec-dependency-gate` branch,
which was never committed. It contained a Python checker
(`scripts/spec_dependencies.py`), unit tests, two `just` recipes, a `start`
step in `implement-plan.md`, and two throwaway probe specs dated `2099-…`.

The checker's rules are sound and are carried into this spec. The prompt edit
is not reusable as-is: `implement-plan.md` has since gained author-decision
gates in `initialize`, a loop `gate`, and the Pipeline Refresh, so the
integration below is specified against the current prompt. The probe specs
are not carried over.

## Definitions

- **Prerequisite**: one entry in a spec's `depends-on` frontmatter. A string or
  a list of strings is accepted; an absent or empty property means none.
- **Met**: the prerequisite's directory lies under a `_completed` lifecycle
  directory. `implemented: true` is **not** enough, because the author moves a
  spec into `_completed` only after its review cycle closes, and review can
  still change what the prerequisite delivers.
- **Spec directory location**: a directory directly under `features/`,
  `fixes/`, or `reviews/`, or under one of their `_completed` or
  `_unscheduled` folders, at the repository root or in any package area.

## Requirements

### R1. Resolving an entry

1. An entry is normally a `{date}-{name}` directory name (the repository
   convention). It matches every spec directory location with that name.
2. An entry containing `/` is a legacy path to a spec file or its directory.
   It is tried relative to the spec's folder, then to the repository root. If
   neither exists, it falls back to its directory name (rule 1), so a
   prerequisite that has since moved into `_completed` is still found.
3. A path that escapes the repository is not resolved as a path.
4. A directory with the matching name that is **not** a spec directory
   location (for example a copy inside a test fixture) is ignored.
5. The candidate directories are those holding a tracked or untracked,
   non-ignored file (`git ls-files --cached --others --exclude-standard`), so
   build output and ignored scratch files never match.

### R2. Verdicts

Each entry gets exactly one verdict:

| Verdict | When |
|---|---|
| met | it resolves to one directory, under `_completed` |
| unmet: not completed | it resolves to one directory that is active or `_unscheduled` |
| unmet: missing | no spec directory location has this name |
| unmet: ambiguous | more than one spec directory location has this name; all are listed |

An ambiguous or missing entry is unmet, not an error: the author must fix the
`depends-on` before the spec can start.

### R3. The command

- `just spec-dependencies <spec>` from the repository root checks one spec
  file. The spec path may be relative or absolute.
- It prints one line per entry (`met` / `UNMET`, the entry, the resolved
  directory when there is one, and the reason) and, when any is unmet, one
  summary line on stderr.
- Exit status: `0` when every entry is met or none is declared; `1` when any
  entry is unmet; `2` for a usage error, an unreadable spec, or a `depends-on`
  value that is neither a string nor a list of strings.
- Frontmatter is read with `md get` (Darkmatter), not a hand-written YAML
  parser.
- It works on macOS, Linux, native Windows, and WSL2: paths are compared as
  repository-relative POSIX paths, and no POSIX-only shell construct is used
  in the recipe.

### R4. The implement prompts refuse to start

- `prompts/_implement/implement-plan.md` runs the gate when a `spec` is known
  and, on failure, stops before any phase work with:
    - a short `message` naming the spec directory and the count of unmet
      prerequisites (chat routes stay terse), and
    - an `error` whose text tells the author what to do: complete the listed
      specs, or correct the `depends-on` entry.
- The gate does not run again on later phases of the same run. A later
  invocation of the prompt (for example resuming after a `human_review`
  pause) runs it again, which is correct and cheap.
- `prompts/_implement/implement-feature.md` applies the same gate in its
  orchestrator path, so neither entry point can start an unready spec. If the
  phase-by-phase path already delegates to `implement-plan.md`, it inherits
  the gate and does not repeat it.
- The review-fix prompts (`implement-suggestions.md`,
  `implement-review*.md`) are **not** gated: they run after the spec was
  implemented, when its prerequisites were already checked.
- Planning (`prompts/plan.md`) is **not** gated: planning ahead of a
  prerequisite is useful, and the final-phase Pipeline Refresh corrects
  downstream plans.

**Placement decision (recommendation).** Claudine refuses `shell` actions in
`initialize` (`claudine/docs/topics/flow-control/lifecycle.md`), but
`initialize` is where the other refuse-to-start gates live. Options:

| Option | Pros | Cons |
|---|---|---|
| A. `shell` step at the top of `start`, guarded so it runs only for the starting phase of a run | one authority (the script) for prompts and `just`; full resolution rules | runs after `initialize` has created the log file; needs shell approval |
| B. pure expression in `initialize` using `find_files()` and `frontmatter()` | sits with the other gates; no shell approval | re-implements resolution in the expression language; ambiguity, legacy paths, and fixture exclusion are hard to express; walks the whole tree |
| C. check in the `just` callers only (`flow.just`, `review.just`) | trivial | a direct `claudine compose` of the prompt bypasses it |

Recommend **A**. The `start` hook already serves as the per-run entry for
things `initialize` cannot do; the empty log file `initialize` creates is
harmless. The planner confirms how to tell the starting phase of a run inside
`start` (for example comparing against the run's starting `phase`) and, if
the lifecycle cannot express it, accepts the per-phase re-check: verdicts can
only move from unmet to met during a run, so a later re-check never fails a
run the first check passed.

### R5. Tests

- Unit tests of entry resolution cover each row of R2 and each rule of R1:
  completed, active, `_unscheduled`, missing, ambiguous, fixture copy,
  path relative to the spec folder, path relative to the root, moved path
  falling back to its name, and a path escaping the repository.
- End-to-end tests run the real command against a temporary Git repository:
  unmet refuses with exit `1`, met passes with `0`, no `depends-on` passes
  silently, a scalar `depends-on` works, and a non-string list item exits `2`.
- One test runs the shipped `implement-plan.md` lifecycle through the normal
  `claudine` invocation against a fixture spec with an unmet prerequisite and
  asserts the run stops before phase work with the R4 message. If
  `claudine/cli/tests/fixtures/shipped_implement_route/` snapshots the prompt,
  its fixture and hashes are refreshed in the same change.
- The checker's test suite is run by an existing repository recipe that CI
  already runs (alongside the `scripts/ci/test_*.py` suites in
  `just/ci-local.just`), not by a recipe nobody calls.

### R6. Documentation

- The topic page that documents the feature/fix lifecycle (or a new
  `docs/` page if none exists) gains a section on `depends-on`: its accepted
  forms, what "met" means, and that the implement prompts enforce it, with
  the example from the Summary.
- `CLAUDE.md` "Features and Fixes" gains one line: `depends-on` names
  prerequisites by `{date}-{name}`, and implementation refuses to start until
  each is in `_completed`.
- `claudine/docs/rollout-strategy.md` "How to run it" replaces "running in
  order always satisfies it" with a statement that the implement prompts now
  enforce it.

## Non-goals

- Checking `related:` or `peers:`; they carry no ordering.
- Detecting dependency cycles or computing a run order. The date prefix
  remains the order.
- Moving specs into `_completed`. That stays the author's act.
- Validating `depends-on` at spec-writing or review time. That could come
  later and would reuse this command.

## Acceptance criteria

1. `just spec-dependencies` reports every R2 verdict correctly and exits with
   the R3 status codes.
2. Running `implement-plan.md` on a spec with an unmet prerequisite stops
   before any phase work, and the output names each unmet prerequisite and
   why.
3. Running it on a spec whose prerequisites are all in `_completed`, or that
   declares none, behaves exactly as today.
4. `implement-feature.md` cannot start an unready spec through either path.
5. The checker's tests run in an existing CI-run recipe and pass on Linux and
   macOS; the command is exercised on native Windows by the nightly.
6. The R6 documentation is updated, and no `docs/` page names this spec.
