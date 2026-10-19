# Per-OS Evidence and Execution Constraints

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

Load before pushing work whose OS coverage you want to reuse or restrict. The
reuse rule and execution-ban store are in the repo root `CLAUDE.md` ("Evidence
Reuse and Execution Constraints"); the full contract (receipts, push modes,
what the hook reviews, `scope-only`, `--no-verify`) is the `rust-devops`
skill's [ci-cd.md](../rust-devops/ci-cd.md). Read it before choosing a push
mode. This page covers only what differs per OS.

## A receipt proves its own environment only

- The hook names its environment with `sniff os --json` (macOS, Linux, native
  Windows, or WSL2) and publishes per-`{package, gate}` observations under
  `refs/notes/ci-local/<environment>`.
- CI reads **every** environment's ref from the merge base to the outgoing
  head and combines them per cell. Observations accumulate; none overwrites
  another. So:
  - a macOS receipt from this push and an earlier `just cross-check --os wsl`
    receipt suppress their own cells together;
  - a WSL2 receipt for one package does not hide an earlier one for another;
  - a macOS receipt alone **never** suppresses WSL2 or Windows.
- A cell from an older head counts only while its gate-input identity (the
  tested package's build closure plus that gate's global inputs) is unchanged
  across the two trees.
- Two disagreeing results for one cell that cannot be ordered (no retry link,
  neither commit descends from the other) make the cell execute.
- `lint`, `check`, browser, and companion-suite work always run in CI.

## Rig legs (`just cross-check --os windows|wsl`)

- Each leg publishes a `rig` observation under its own environment
  (`windows-latest` or `wsl2-ubuntu`), and only when it ran the outgoing head's
  exact tree on a clean remote worktree, unfiltered, in archive mode. Anything
  else prints why nothing was published. Most rig runs ship uncommitted work
  and publish nothing.
- A passing rig retry names the failure it retries (`retry_of`); a rerun
  without that link conflicts with the failure and the cell still executes.
- `just ci-seed` proves the nightly's Windows and WSL2 cells ahead of it.

## Where the reports live

- A hook or rig observation's JUnit reports stay on the host that produced
  them, under `$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/` (default
  root `~/.rusty-biscuit/ci-evidence`). Investigating a reused cell means
  asking that host.
- On that host, `just ci-evidence` maps the SHA directories to branches,
  subjects, results, and durations (`--json` adds paths). Never rename those
  directories: published receipts name them.
- A nightly `hosted` observation's report is the producer run's
  `junit-<package>-<gate>-<environment>` artifact. Its pass counts only while
  that artifact is retrievable and matches; once it expires, the cell runs
  again. A pass in one environment survives a later unrelated failure, so a
  retry runs only what is unproven (`scripts/ci/hosted_observations.py`).

## Execution bans

- A request not to rerun passed tests is not a ban. Only an explicit
  instruction (for example, an environment down for maintenance) is; never
  infer a blanket WSL2 or Windows prohibition.
- Bans live in `<home>/.rusty-biscuit/ci-constraints/<repository>/`;
  `BISCUIT_CI_CONSTRAINTS_DIR` overrides that location.
- The store's `<home>` is Python's `Path.home()`, which is `USERPROFILE` on
  native Windows. A test that relocates it sets both `HOME` and `USERPROFILE`.
- If `just ci-local --plan` still schedules a banned cell, explain the gap
  before pushing; automatic jobs are not exempt.

## Nightly anchors

The nightly anchors each environment separately, to the newest successful
scheduled run that proved *that* environment.

- History from when the nightly ran WSL2 alone is never native Windows proof;
  without a qualifying anchor, Windows runs the full gated workspace.
- A legacy (pre-certification) anchor inherits no cell; a certified one
  inherits a cell only under an unchanged coverage contract, so a new backend
  or test feature in an existing gate is selected again.
- A failing `just cross-check <pkg> --os windows|wsl` published after a green
  nightly reopens that one cell (`reopened_cells`) until a linked passing retry.
- To answer "why did the nightly run everything", read the scope job's
  `coverage anchors` summary row: it gives a reason code per environment.
