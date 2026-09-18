---
kind: fix
name: ci-cadence
date: 2026-09-18
status: implementing
related:
  - 2026-09-12-single-os-compile
---

# CI cadence: schedule environments by event

## Why

The first hosted run of `feat/single-os` (PR #83, run 35309434293) planned 133
executing cells for one pull request: lint, check, L1, L2, and browser on
Ubuntu, L1 and check on Windows, L1 on WSL2, and two producers of 26 build
keys each. Both producers hit the 60-minute job timeout with Ubuntu at 20 of
26 keys and Windows at 13 of 26, so every consumer downstream reported
`Artifact not found`. Locally, the pre-push hook took 45 minutes for a
workflow-only change because a workflow file is a gate input for every cell
and invalidated all prior evidence.

The repository has no users. The bar Ken set on 2026-09-18 is: every pull
request proves Linux; macOS comes from the hook's local evidence; Windows and
WSL2 prove themselves after merge and on a schedule. Five decisions, all
taken by Ken, implemented here.

## Decisions

1. **Environments are scheduled by GitHub event.** `environments.json` gains a
   required `events` list per environment naming the events that schedule it:
   `ubuntu-latest` on every event, `macos-latest` on `pull_request`, `push`,
   and `workflow_dispatch`, `windows-latest` on `push`, `schedule`, and
   `workflow_dispatch`, and `wsl2-ubuntu` on `schedule` and
   `workflow_dispatch` only. The planner takes `--event`; an environment the
   event does not schedule contributes no cell, no build, and no preflight
   OS, and is recorded in the plan's `deferred_environments` with the events
   that will run it. Without `--event` (a developer's `just ci-local --plan`,
   the Python suites) every environment is planned, as before.
2. **WSL2 is nightly and manual only.** `ci.yml` gains a `schedule` trigger
   (`0 8 * * *`, its own concurrency group) that plans the full workspace with
   `--event schedule`. macOS is not on the schedule: the development Mac
   covers it on every push through the hook.
3. **`check` is a single-environment gate**, hosted on `ubuntu-latest` like
   `lint`. Compiling examples and benches once is coverage; compiling them
   per operating system was cost.
4. **The producer budget is 180 minutes.** Enough for one owner to build the
   full workspace warm; the Windows owner no longer runs on pull requests.
5. **CI orchestration files are not local gate inputs.** `ci.yml`,
   `_package-ci.yml`, `_wsl-ci.yml`, `environments.json`,
   `affected_scope.py`, and `.github/actions/` decide what CI runs, never what
   a local gate produces, so a change to them no longer invalidates the hook's
   published evidence. Toolchain, Cargo config, manifests, the lockfile,
   `clippy.toml`, `.config/nextest.toml`, and the Just recipes remain inputs.

Two consequences follow:

- **A push to `main` after a validated pull request runs only the deferred
  environments.** The validation job's `reuse` no longer skips the scope job;
  scope plans with `--proven-event pull_request`, which drops every
  environment the pull request event schedules and records them in
  `proven_environments`. A push whose tree the PR never tested plans every
  `push` environment, as before.
- **A pull request can opt back in** with the `ci:all-os` label, which plans
  every environment (`--all-environments`). The label is read at event time,
  so it takes effect on the next push to the branch.

The plan's `event`, `deferred_environments`, and `proven_environments` are
optional fields, so the resolved plan stays at schema version 4 (R9: the
version moves with the required set). A scope receipt is bound to its event:
`scope-verify --event` refuses a receipt planned for another event with
`scope-event-mismatch`, and the hook plans `push` for `main` and
`pull_request` for every other branch.

## Not done

- Lint evidence from the hook. Lint stages no JUnit report and the receipt
  contract records measured cells only; lint already runs on one environment.
- Producer sharding. It multiplies dependency compiles; revisit if the single
  Ubuntu owner exceeds its budget on ordinary pull requests.
