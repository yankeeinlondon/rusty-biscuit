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
`refs/notes/ci-local/<environment>` ref between the merge base and the outgoing
head. So:

- a macOS receipt from this push and a prior `cross-check` WSL receipt suppress
  their own cells together;
- a WSL receipt that covered one package does not hide an earlier one that
  covered another;
- a successful macOS receipt alone does **not** suppress WSL.

The JUnit reports behind a hook receipt stay on the host that produced it, at
the receipt's `host.report_dir`
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
