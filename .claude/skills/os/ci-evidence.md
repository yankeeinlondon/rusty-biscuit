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
`cross-check.sh` copies its WSL report to the same evidence directory before
citing it.

The JUnit reports behind a hook receipt stay on the host that produced it, at
the observation's `producer.host.report_dir`
(`$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/`, root default
`~/.rusty-biscuit/ci-evidence`). Investigating a reused cell means asking that
host.

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
gated workspace). A legacy anchor proves no list of cells: the planner assumes it
covers whatever the diff does not reselect, and cannot see a gap that expired or a
backend that became required since. The first certified nightly ends that
assumption. When reproducing "why did the nightly run everything", read the scope
job's summary row `coverage anchors`: it names the reason code per environment.
