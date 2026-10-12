# Per-OS Evidence and Execution Constraints

Load before pushing work whose OS coverage you want to reuse or restrict:
which receipt satisfies which environment's cell, what an execution ban is,
and how `scope-only` behaves. The full evidence and execution contract is the
`rust-devops` skill's [ci-cd.md](../rust-devops/ci-cd.md); read it before
selecting a push mode.

## Reuse, and when you must run

- Reuse qualifying passing evidence **per required cell on every OS**. If no
  qualifying passing evidence exists, execute the required tests.
- A request to avoid rerunning passed tests is **not** an environment ban.
- Only a separately explicit instruction (such as an environment unavailable
  during maintenance) creates an execution constraint. Never infer a blanket
  WSL prohibition.

## Evidence combines per cell, across environments and commits

`local_evidence.py verify --cells` reads every note on every
`refs/notes/ci-local/<environment>` ref from the coverage boundary (the merge
base, included) up to the outgoing head; with an all-zero base it reads the
annotated commits reachable from the head instead. A note is a version-3
collection of observations, so a second package, a retry, or a second host at one
head is added to the note and never overwrites it. So:

- a macOS receipt from this push and a prior `cross-check` WSL receipt suppress
  their own cells together;
- a WSL receipt that covered one package does not hide an earlier one that
  covered another;
- a successful macOS receipt alone does **not** suppress WSL.

An observation's report is cited by digest. A `host-dir` report on the verifying
host is checked byte for byte; one on another host is accepted with its
provenance shown. When two results for one cell cannot be ordered (no retry link,
neither commit descends from the other) and they disagree, the cell executes.
A `cross-check.sh` rig leg (`wsl` or `windows`, each filed under its own
environment) retains its report, expected manifest, and completion record in one
per-run directory under the same evidence directory and cites that directory as
the observation's `report_dir`.

The JUnit reports behind a hook receipt stay on the host that produced it, at
the observation's `producer.host.report_dir`
(`$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/`, root default
`~/.rusty-biscuit/ci-evidence`). Investigating a reused cell means asking that
host.

## Hosted nightly cells publish their own proof

On the nightly, each hosted native test cell and each WSL2 guest cell records
itself as a `hosted` observation (`scripts/ci/hosted_observations.py`) and
uploads `observation-<package>-<gate>-<environment>`; the schedule-only
`publish-observations` job then writes the run's observations to
`refs/notes/ci-local/<environment>` (WSL2 cells under the `wsl2-ubuntu` ref).
So a pass in one environment survives a later unrelated failure or cancellation
and a retry executes only what is unproven.

- The report is **not** on any host: it is the `junit-<package>-<gate>-<environment>`
  artifact of the producer run. A `hosted` pass is credited only while that
  artifact is retrievable and its bytes match, so an expired artifact (the
  repository's artifact retention) re-runs the cell. A completed failure is kept
  and still blocks an older pass.
- Nothing is invented: a cancelled, skipped, or report-less cell, or a passing
  gate with no valid completion record, records nothing.
- A run whose publisher job was lost is recovered by the next nightly: its
  `scope` job records the newest five completed scheduled runs' observations
  locally before verifying (`publish --recent --no-push`), and
  `publish-observations` pushes them (`publish --recent`). Older runs need
  `publish --run-id N` by hand.
- Unverified on a real run: that the ruleset lets `GITHUB_TOKEN` push
  `refs/notes/*`. If it does not, the publisher step is red and the artifacts
  are the only record. The CI/CD topic page has the flow.

## Execution bans

- A separately explicit execution ban also constrains CI triggered by a push.
  Verify every requested exclusion before pushing.
- Record the restriction rather than remembering it:
  `<home>/.rusty-biscuit/ci-constraints/<repository>/`
  (`BISCUIT_CI_CONSTRAINTS_DIR` overrides it; the home is Python's
  `Path.home()`, so `USERPROFILE` on native Windows).
- `just ci-local --plan` and the pre-push hook refuse on those prohibition
  records; CI deliberately never reads them.
- If the plan still schedules a prohibited cell, explain the gap before
  triggering CI. Do not treat automatic jobs as exempt.

## `scope-only` push mode

- Resolves and prints the plan, so a recorded constraint is still enforced and
  the run is still reviewable.
- Runs no gate, publishes no outcomes, and excludes no CI cells.
- Does publish the *scope* receipt on `refs/notes/ci-local/scope`, as every
  mode does; CI takes it on an exact `{base, head, tree}` match.
- `off` is its deprecated alias.

## What the hook reviews

- Every pushed branch's **committed** plan (a temporary worktree unless the
  revision is the clean checkout), under that update's remote branch and
  remote, against the base of each run it triggers:
  - a push to `main`: the remote's `main`;
  - on a GitHub remote, each open pull request's target tip, read through `gh`,
    plus the incoming revision when the same push updates that target too;
  - otherwise a provisional plan against the remote's `main`.
- It publishes HEAD's plan. `just ci-local --plan` previews the working tree
  and differs exactly when the checkout is dirty.
- `git push --no-verify` produces no new evidence and does not invalidate
  already-published matching receipts, which CI still verifies.

## Nightly anchors and legacy history

The nightly anchors each environment to the newest successful scheduled run that
proved *that* environment. A run from before certifications existed is credited
only for environments its own `environments.json` scheduled for `schedule` at
that run's commit, and only if every `coverage-audit` job succeeded, so the
WSL2-only history never counts as native Windows proof (Windows stays on the full
gated workspace). A legacy anchor records no coverage contracts, so it inherits
no cell: every WSL2 cell current policy requires is selected, and seeded or other
qualifying evidence decides which execute. A certified anchor inherits a cell only
under an unchanged coverage contract, so a new backend or test feature inside an
existing gate is selected again, and only while no later qualifying result
contradicts it: a failing `just cross-check <pkg> --os windows|wsl` published
after a green nightly, at its head or on a docs-only descendant, puts that one
cell in the next nightly's `reopened_cells` (the rest stay inherited). A
passing rig retry restores inheritance because `cross-check.sh` names the
failure as its `retry_of`; a rerun published without that link conflicts with
the failure and still executes. The first certified nightly ends the legacy case. When reproducing "why did the nightly run everything", read the scope
job's summary row `coverage anchors`: it names the reason code per environment.
