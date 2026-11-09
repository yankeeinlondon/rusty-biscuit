---
kind: feature
name: legible-ci-names
date: 2026-11-06
status: finalized-spec
next_spec: 2026-11-01-rust-pre-push
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-08
review_iterations: 0
implemented: false
rulings:
  - id: static-matrix-identity
    decided_by: Ken Snyder
    decided_on: 2026-11-08
    decision: >-
      Permit one additive, display-only `label` field in planner matrix
      entries so executing package jobs read `<package>: <action> on
      <environment>` under a static job name; keep job identifiers, guards,
      and reusable-workflow layers unchanged. If the bounded display check
      shows the label cannot render legibly, fall back to automatic matrix
      labels with stored wording in the suffix and record why.
  - id: display-name-reader-scope
    decided_by: Ken Snyder
    decided_on: 2026-11-08
    decision: >-
      Grandfather the two existing display-name readers (runner-loss
      attribution in scripts/ci/runner_loss.py and audit discovery via
      scripts/ci/reuse_validation.py::AUDIT_JOB). They parse only stored
      matrix values, never the display-only label, which may not contain
      `,`, `(`, `)`, or ` / `. No new display-name-dependent reader is
      permitted. Replacing these readers belongs to their Rust port.
needs_rulings: false
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

A newcomer can identify the package, the kind of work, and the environment from
an executing job's name. Opening its failed step explains which operation failed;
a name alone cannot explain a failure that has not happened yet. Desired work
labels include:

```text
research: run Level 1 tests on Linux
schematic-gen: run Level 1 tests on WSL2
messenger: check compilation on Linux
```

These are work labels, not promises about the complete name GitHub renders.
Reusable workflows may add caller prefixes; the feasibility check below records
the complete rendered names before implementation chooses a template.

## Scope and boundaries

Apply the convention to `ci.yml`, `_area-ci.yml`, `_package-ci.yml`, and
`_wsl-ci.yml`: workflow titles, job display names, named steps, and the names of
accepted-gap checks published for this pipeline. Other workflows are out of
scope. Document the convention in `docs/topics/ci-cd.md`.

This is a presentation change. Preserve job and step identifiers, workflow file
paths, dependencies, guards, matrix identity values, runner selection, permissions,
timeouts, failure propagation, and evidence reuse. Keep the existing ten
top-level job identifiers in `ci.yml`. Stored artifacts, reports, completion
records, and evidence retain their existing identifiers. Package work stays keyed
on `{package, environment, gate}`; an area only groups packages. No result reader
may recover identity or policy by parsing a display name, except the two
existing readers named under "Existing name readers" below. The one permitted
matrix change is the additive display-only `label` field described under Open
Questions.

Keep `ci-gate` as both its job identifier and displayed required check name.
It remains the required check in `protect-your-bacon`; this feature neither
changes the ruleset nor changes how the gate decides whether merging is allowed.

> Reader's note: shorter names do not justify flattening reusable workflows in
> this change. Those boundaries also control permissions and failure propagation.
> If useful names cannot fit within the current structure, the author must decide
> whether that larger change is worthwhile; see Open Questions.

## Naming rules

Names describe work in plain language. Use package names exactly as declared;
use an area's repository-relative directory for area-wide work so nested areas
remain distinguishable. Do not substitute an area name for a package name.

| Stored value | Reader-facing wording |
| --- | --- |
| `ubuntu-latest` | Linux |
| `macos-latest` | macOS |
| `windows-latest` | Windows |
| `wsl2-ubuntu` | WSL2 |
| `L1` | run Level 1 tests |
| `L2` | run Level 2 tests |
| `browser` | run browser tests |
| `lint` | lint source |
| `check` | check compilation |

Define Level 1 and Level 2 in the documented convention using the repository's
current testing definitions. Retaining the level number distinguishes suites
without claiming that Level 1 contains only unit tests. Describe WSL2 there as
Windows Subsystem for Linux 2. Label work by its execution environment: a WSL2
test runs on WSL2 even though Windows hosts its runner. A Linux build preparing
an archive for WSL2 remains a Linux build.

Executing package work should convey `<package>: <action> on <environment>`
once across the complete caller/child name. Avoid repeating runner labels,
package names, or test levels in successive prefixes. Do not use empty names,
invisible characters, or punctuation-only names to conceal workflow structure.

A compilation check can compile both the changed package's declared targets and
its unchanged direct dependents. Keep the broad label “check compilation”; name
the dependent-compilation step explicitly and retain the existing report naming
those dependents. Do not present them as separately tested packages.

Infrastructure jobs describe their purpose, such as “Choose required work,”
“Prepare test builds,” or “Check required results for <area>.” Keep the reporting
job visibly advisory. Accepted-gap publishing and nightly evidence publication
must remain distinguishable from running tests. Give named steps concise verbs
and objects, such as “Download test results” or “Verify all expected tests reported.”
Rename steps referenced by workflow contract tests together with those assertions;
do not change step identifiers or commands to obtain a nicer name.

## Skipped work and truthful reporting

A skipped job is not evidence that tests passed. Preserve the planner's decisions
and the area's coverage audit, which checks that every required result exists.
Use the existing run report and area summaries to explain these distinct cases:

- No work selected: state that this change required no package tests.
- Passing evidence reused: say the work was already proved and retain its evidence
  link; do not create an extra runner merely to show a successful label.
- Environment deferred by event policy: say it was not scheduled for this event,
  rather than calling it tested or a capability gap.
- Accepted capability gap: retain the `ACCEPTED GAP` marker, neutral conclusion,
  reason, owner, and expiry. Its display name identifies package, work, and
  environment; publishing a gap is not executing that work.
- Required result missing, failed, or canceled: preserve the existing blocking
  behavior. Do not describe this as intentional skipping.

Do not add jobs or change guards to hide skipped placeholders. Jobs whose matrix
may be skipped before expansion must have static, useful display names; do not
put matrix expressions in those names, because skipped jobs can expose the
unevaluated expression. For those jobs, the action can be static while GitHub's
executing matrix suffix supplies identity. The feasibility check must establish
whether that suffix is legible enough. A static name may use an already-available
workflow input only after confirming it renders when the job is skipped.

## Bounded feasibility check

Before changing templates, inspect the existing workflow contracts and available
run records. If they do not settle how names render, use one small disposable
workflow outside production CI with representative static, matrix, nested-call,
and skipped jobs. Do not trigger a full repository run to learn naming behavior.
This is a display check, not a performance benchmark; it needs no host matrix or
repeated measurements.

Record the chosen templates and complete observed names in the implementation
log, including the Actions job list and pull-request checks list. Include native
and WSL2-shaped nesting, two test levels for one package, a nested area, and an
empty selection. These are naming examples, not requests to run those test suites.
Do not promise that caller prefixes can be removed without evidence. If the
current structure cannot meet the outcome, record the limitation and leave the
structural choice below open rather than silently expanding implementation scope.

## Existing name readers

**Ruled 2026-11-08.** Two existing readers derive identity from display names
because, for a runner that is lost, the job name in the Actions API is the only
identity left: the job uploads no status, report, or log, and GitHub's job API
exposes no matrix values or custom metadata.

| Reader | What it reads the name for |
| --- | --- |
| `scripts/ci/runner_loss.py` (`parse_job_name`, `row_cell`, `parse_build_job_name`, `is_non_producer`) | Which cell a lost runner owned, so attribution, the rollup, and `ci-infra-retry.yml` can classify the failure as infrastructure and retry it |
| `scripts/ci/reuse_validation.py::AUDIT_JOB` | Finding successful coverage audits that anchor nightly evidence reuse and certification |

Both are permitted exceptions to the display-name prohibition, under these
rules:

- Update each reader together with the renames, and keep recognizing every
  historical name form it accepts today.
- Readers parse only the stored matrix values (package, gate, environment,
  runner), located from the end of the generated parenthetical, and never the
  display-only `label`. A labeled row such as
  `(schematic-gen: run Level 1 tests on WSL2, schematic-gen, L1, wsl2-ubuntu, windows-latest)`
  yields the same cell as today's
  `(schematic-gen, L1, wsl2-ubuntu, windows-latest)`.
- A `label` must not contain `,`, `(`, `)`, or ` / `; the planner rejects one
  that does.
- No new reader may derive identity or policy from a display name.

Replacing these readers with an identity source independent of display names
is out of scope; it belongs to their Rust port.

## Implementation and acceptance

1. Inventory existing display names and readers that compare job names, including
   workflow contract tests, accepted-gap publication, and nightly audit discovery.
   Preserve machine lookup behavior; where a reader uses the old display name,
   update it with the rename and verify that it still finds the same jobs.
2. Apply the feasible naming templates within the boundaries above. Update
   adjacent comments that describe the old rendered names.
3. Document the wording, test levels, environment mapping, skipped states, and
   any unavoidable GitHub prefixes in `docs/topics/ci-cd.md`.
4. Update focused workflow contract tests in `test-toolkit` and any affected
   publisher tests in `repo-deps`. Verify the rendered-name examples, static names
   for empty matrices, distinct test levels, and the unchanged `ci-gate` context.
   Follow the repository's testing recipes; no new full-scope CI run is required.
5. Cover the grandfathered readers: `row_cell` and `parse_job_name` extract the
   same cell from today's names, historical names, and labeled names;
   `AUDIT_JOB` finds the renamed audit job and historical audit names; and a
   `label` containing `,`, `(`, `)`, or ` / ` fails the planner's contract test.

Implementation is ready for review when names distinguish package, work, and
execution environment; skipped names contain no raw expressions; reports still
explain reuse and gaps accurately; and the diff preserves scheduling, stored
identities, permissions, and merge behavior. Existing contract tests that prohibit
matrix-dependent display names remain authoritative unless a separately reviewed
structural change makes that restriction unnecessary. This naming-only feature
requires no schema-version bump.

## Open Questions

### What if reusable-workflow prefixes prevent readable complete names?

**Ruled 2026-11-08: add a display-only matrix label; fall back to automatic
labels if it cannot render legibly. No workflow restructure.**

GitHub renders an unnamed matrix job as `job-id (value, value, ...)`, which
keeps identity but shows stored values such as `L1` and `wsl2-ubuntu`. A name
containing matrix expressions renders the raw expression when the job is
skipped before expansion, so job names stay static.

The planner (`scripts/ci/affected_scope.py`) adds one field, `label`, to each
executing package matrix entry, carrying the reader-facing work label from the
naming rules, for example `schematic-gen: run Level 1 tests on WSL2`.

- `label` is presentation only. No reader parses it for identity or policy;
  stored identities, artifacts, completion records, and evidence remain keyed
  on `{package, environment, gate}`, and every existing matrix value keeps its
  name and meaning. Adding it does not change scheduling or the plan's
  identity, and needs no schema-version bump unless a reader validates matrix
  keys strictly, in which case update that reader and record why.
- Place `label` where it leads GitHub's generated suffix. The bounded display
  check determines the rendered form: either a static name with the full value
  list, for example
  `Package work for schematic (schematic-gen: run Level 1 tests on WSL2, schematic-gen, L1, wsl2-ubuntu, windows-latest)`,
  or, where GitHub's rendering allows it without losing identity,
  `Package work (schematic-gen: run Level 1 tests on WSL2)`.
  Record the observed names in the implementation log.
- Keep job identifiers, guards, permissions, and reusable-workflow layers
  unchanged. Static job names remain required; the existing contract that
  prohibits matrix-dependent display names stays authoritative.

**Fallback.** If the display check shows that the label cannot render
legibly, or that adding it loses identity in any rendered name, do not add it.
Keep automatic matrix labels, permit their stored gate and environment wording
and repeated runner values in the suffix, apply descriptive names to prefixes,
infrastructure jobs, and steps, and record the observed limitation in the
implementation log.

Rejected alternatives: removing a reusable-workflow layer (requires
re-verifying permissions, failure propagation, and job discovery; a separate
change if ever wanted) and leaving job names mostly unchanged with improved
summaries only (does not make the job list readable).
