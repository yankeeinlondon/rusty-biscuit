---
title: Throwaway-branch fixture plan — nested-area labels, reused cells, gap routing, and the merge gate
kind: fixture-plan
created: 2026-09-12
for: fixes/2026-09-11-cicd-cleanup/spec.md
answers: [B5 (branch half), review-6 findings 1 and 7]
status: superseded
superseded_by: hosted-fixture.md
---

# Throwaway-branch fixture plan, 2026-09-12

This historical plan is superseded by [the reduced hosted fixture](hosted-fixture.md).
Do not execute its push sequence: it schedules prohibited WSL work and requires
a feature-branch bootstrap whose own trigger plan can violate the restrictions.
The replacement keeps fixture execution separate from product suites and does
not require lifting or expiring any execution constraint.

This is the plan for the branch half of ruling B5, not its record. B5
authorizes a throwaway branch of `rusty-biscuit`; the session that
implemented review 6 was forbidden to commit or push, and a hosted fixture
is nothing but a commit, a push, and a pull request, so it could not run in
that session. Everything below was taken from the working tree on
2026-09-12 — the planner was run read-only against the file set in section
2 — and none of it is a hosted observation. The record goes to
`fixtures/throwaway-branch-<date>.md` in the shape of section 3, and this
plan is superseded the day that record exists.

Two hosted proofs share this branch and this blocker: the presentation
fixture below (review 6, finding 7) and the producer-normalization
regression that `rollout-2026-09-11.md` describes in its final section
(review 6, finding 1). The latter is cross-referenced, not repeated.

## 1. Observations owed

| # | Observation | Criteria | Where it is read |
|---|---|---|---|
| a | Nested-area labels through the four-level chain | AC2, AC10 | `gh run view <run> --json jobs -q '.jobs[].name'`; the PR's Checks tab |
| b | A reused cell with no producer job and a visible completed result | AC4 | the `ci-resolved-plan` artifact, the area's `rollup` step summary, the `ci-results-<slug>` artifact, the run graph |
| c | Gap and no-gap publisher routing | AC9 | the two areas' `accepted-gaps` jobs; `gh api repos/{owner}/{repo}/commits/<head-sha>/check-runs` |
| d | Mixed reused / executing / failed / unselected presentation and the `ci-gate` result | AC8, AC12 | the run graph; `ci-gate`'s log; `gh pr view <n> --json mergeStateStatus` |
| e | The four producer-normalization cases | AC8, AC11 | `rollout-2026-09-11.md`, final section — record them in the same table |

What each observation must show, in the words the record should use:

- **(a)** `claudine/rendezvous` is a nested area and its labels read area
  first, package under it, cell last:
  `area-ci (claudine/rendezvous) / rendezvous-core / lint (ubuntu-latest)`,
  `… / check (ubuntu-latest)`, `… / test (ubuntu-latest)`,
  `… / test (windows-latest)`, and the fourth level through the WSL
  workflow, `… / rendezvous-core / wsl2 / archive (rendezvous-core for wsl2)`
  and `… / wsl2 / test (wsl2-ubuntu)`. The area's own jobs render as
  `area-ci (claudine/rendezvous) / rollup` and
  `area-ci (claudine/rendezvous) / accepted-gaps` (skipped, static name).
  No label anywhere in the run contains `${{`. Lint's and check's labels
  carry their environment.
- **(b)** In the main run (run 1 below) the plan resolves
  `rendezvous-core/macos-latest/L1` and `worktree-cli/macos-latest/L1` to
  `execution: reuse`. No `test (macos-latest)` job exists under either
  package — the native-environments matrix omits the environment — while
  the area rollup's `### Reused results` table lists each cell with origin
  `local`, its counts and duration, and the evidence
  `refs/notes/ci-local/macos-latest` plus the host identity, and the
  `ci-results-<slug>` artifact carries the cell as `PASS` with
  `origin: local`. The scope job's summary reads `scope source: local
  (refs/notes/ci-local/scope @ …)` and its log shows no `rustup show`.
- **(c)** `area-ci (worktree) / accepted-gaps` runs and its log ends with
  `publish-gaps: published neutral check 'accepted gap: worktree /
  worktree-cli / L2 (windows-latest)' on <head-sha>`;
  `area-ci (claudine/rendezvous) / accepted-gaps` is skipped. The check run
  appears in the PR's checks list with conclusion `neutral`, attributed to
  `github-actions`, on the pull request **head** commit rather than the
  merge commit. The job's artifact download succeeded under `contents:
  read` and `checks: write` alone — no `actions: read`. The worktree
  rollup's `### Accepted policy gaps` table shows the cell `ACCEPTED GAP`
  and the run's conclusion is unchanged by the check.
- **(d)** The run graph has exactly two top-level area entries,
  `area-ci (claudine/rendezvous)` and `area-ci (worktree)`; every
  unselected area is absent, not a skipped placeholder;
  `biscuit-tui-captured-stdout` is skipped under its static name;
  `ci-tooling (rollup + scope tests)` runs (section 2 says why). Inside
  `claudine/rendezvous` the reused macOS cell, the executing cells, and the
  `ubuntu-latest` L1 cell shown `FAIL` with a `baseline-accepted` finding
  sit in one grid. `ci-gate` logs `area-ci: success` and concludes
  `success`; after normalization case 2 it logs `area-ci: failure (blocks)`
  and concludes `failure`. `mergeStateStatus` is recorded beside it but is
  not load-bearing until the ruleset names `ci-gate`.
- **(e)** The four cases and their assertions are in the rollout's final
  section. Two corrections to that recipe, found while planning this
  branch, are in section 2 under "The injected failure".

Incidental, not owed: run 1 is also the first hosted measurement of the
Open Question 1 dependent seam (the `check (ubuntu-latest)` cell compiles
`claudine-cli`, `rendezvous-client`, and `rendezvous-daemon`; the log of
2026-09-12 10:38 deferred its duration to the first real run), and the
second and later pushes show whether an unchanged package's macOS cells
arrive as `prior-local` from the earlier head's note (AC7's input
equivalence). Record both if seen; neither closes a finding.

## 2. The reduction

**Governing principle**: test only what needs testing. The branch changes
three files and selects two areas — the fewest that can show a nested area,
a gap area, a no-gap area, a reused cell, and a baselined failure at once.

### What the branch changes

| File | Why |
|---|---|
| `claudine/rendezvous/core/src/lib.rs` (a unit test in the existing test module) | selects the nested area `claudine/rendezvous` through its leanest package (`rendezvous-core`: lib only, 88 L1 tests, no example or bench target, so no `check` cell of its own) and carries the injected failure |
| `worktree/cli/src/main.rs` (a comment) | selects `worktree`, the smallest area with a governed policy gap: `worktree-cli` declares an L2 tier on `tmux`/`kitty`, and `tmux` is unavailable on `windows-latest` (75 L1 tests, 2 L2 tests) |
| `.github/ci/ci-baseline.toml` | the exact `[[failure]]` entry for `{rendezvous-core, ubuntu-latest, L1}` with `owner`, `reason`, `source_run`, and an unexpired `expiry` |

The planner, run against exactly that file set on 2026-09-12, resolves:

| Area | Package | Cells | Execution |
|---|---|---|---|
| `claudine/rendezvous` | `rendezvous-core` | `lint` (ubuntu), `check` (ubuntu, carrying the three dependents), L1 on ubuntu, windows, macos, wsl2-ubuntu | all execute until the macOS receipt exists |
| `worktree` | `worktree-cli` | `lint` (ubuntu), L1 on ubuntu, windows, macos, wsl2-ubuntu; L2 on ubuntu and macos | all execute until the macOS receipt exists |
| `worktree` | `worktree-cli` | L2 on windows-latest and wsl2-ubuntu | `omit`, state `accepted-gap`, governed by `environments.json` (`tmux`) |

`preflight_os` is all three runners and `flags.ci_tooling` is `true`.

### What cannot be trimmed, and why

- **The two WSL cells run.** There is no per-package switch: the planner
  schedules `wsl2-ubuntu` L1 for every package that has tests
  (`affected_scope.py`, the `"wsl": testing and …` projection), and the
  only ways a WSL cell does not execute are a qualifying WSL receipt or a
  persisted constraint, which blocks the push instead. The rollout's
  "`wsl = false`" wording assumed a manifest key that does not exist. Cost:
  two archive builds on `ubuntu-latest` and two guest runs.
- **`ci-tooling` runs.** `.github/ci/` is a CI-tooling prefix, so the
  baseline edit selects the rollup and scope test suites (bounded by its
  20-minute timeout, in parallel with the areas). It is the price of the
  baseline case and cannot be avoided without dropping that case.
- **The dependent seam compiles `claudine-cli`.** One `cargo check` on
  `ubuntu-latest`; it is the ruled Open Question 1 behavior and the first
  place its hub-crate cost is measured.
- **`preflight` runs on three runners.** Derived from the scope; not
  configurable per branch.

Not run: any other area, the browser tier, a `workflow_dispatch`, and a
full-scope run (the standing PR #74 constraint forbids one; nothing here
triggers one).

### The injected failure

Pin the failure to the **runtime** environment name, not to `cfg`:

```rust
#[test]
fn fixture_fails_only_on_ubuntu_latest() {
    assert_ne!(
        std::env::var("BISCUIT_CI_ENVIRONMENT").as_deref(),
        Ok("ubuntu-latest"),
        "throwaway fixture: deliberate failure on the ubuntu-latest L1 cell"
    );
}
```

`BISCUIT_CI_ENVIRONMENT` is stamped per leg — the `test`, `test-l2`, and
`test-browser` jobs set it from their matrix, `_wsl-ci.yml` sets
`wsl2-ubuntu` in the guest, and the archive job and local runs leave it
unset — so the failure lands on exactly `{rendezvous-core, ubuntu-latest,
L1}`. The rollout's recipe gates it on `#[cfg(target_os = "linux")]`,
which is wrong twice: the WSL guest runs the Linux archive, so the cfg would
also fail `wsl2-ubuntu` (a second red cell needing its own baseline entry),
and case 3's `compile_error!` under that cfg would break the WSL archive
build — a setup failure the normalization deliberately does **not** excuse,
so the producer would go red and block the gate for the wrong reason. For
case 3 use the package's existing `build.rs` instead: a guard that panics
when `BISCUIT_CI_ENVIRONMENT` is `ubuntu-latest` fails the build on that
leg only, before nextest can write a report; the archive build and this
host never see the value.

Case 4 has a second problem: it asks this host to reproduce "the same
failure", but the failure fires only on `ubuntu-latest` and this host is
macOS. Make the local run fail too — condition the assertion on the value
being `ubuntu-latest` **or unset** — and add a second baseline entry for
`{rendezvous-core, macos-latest, L1}`. The hosted `ubuntu-latest` cell and
the reused `macos-latest` cell then show the same shape side by side:
`FAIL`, `baseline-accepted`, origins `ci` and `local`, gate green. The
hook now publishes a complete failing strict run before blocking (ruling
D1; the working tree's `.githooks/pre-push` does this), so the note reaches
the remote; push that commit with `RUSTY_BISCUIT_PRE_PUSH=warn` so the
constraint review still runs and no `--no-verify` is needed.

### The reused cell

No separate step produces it. A strict push from a **clean** checkout, with
a pull request open against `feat/unifi`, makes the hook feed the reviewed
plan (the branch's delta against the PR target's remote tip) to `just
pre-push`, run lint and L1 — and worktree-cli's hostable L2 — on this host
for the two packages, and publish two notes on the outgoing head:

- `refs/notes/ci-local/macos-latest` — the validation receipt carrying
  `rendezvous-core/macos-latest/L1`, `worktree-cli/macos-latest/L1`, and
  `worktree-cli/macos-latest/L2` (the last only if the hook ran the L2
  tier);
- `refs/notes/ci-local/scope` — the scope receipt bound to
  `{base = feat/unifi tip, head, tree}`, which the scope job takes as its
  plan on an exact hit.

The order matters. With no pull request open, the hook plans a
**provisional** run against the remote's `main`, and `feat/unifi` changes
the workflows and `scripts/ci/` — global inputs that widen every gate — so
that plan is the full workspace and the local gates would try to run it.
Hence the sequence below.

### Push sequence

1. Confirm the prerequisites in section 4. Branch from the `feat/unifi`
   tip that carries this cycle's implementation, after that tip is pushed:
   `git switch -c fixture/ci-presentation-<date> feat/unifi`.
2. **Commit A**: the empty record `fixtures/throwaway-branch-<date>.md`
   (section 3's template). Documentation only, so it selects no package.
   `RUSTY_BISCUIT_PRE_PUSH=scope-only git push -u origin
   fixture/ci-presentation-<date>`. The hook reviews the provisional plan
   (it must find no prohibited executing cell), publishes a scope receipt
   CI will not match (harmless), and runs no gate. No run is triggered:
   `pull_request` needs a pull request and `push` fires only on `main`.
3. `gh pr create --draft --base feat/unifi --title "fixture: CI
   presentation (throwaway)"`. This is **run 0** on commit A: scope selects
   nothing, `area-ci` is skipped as a whole under its static name,
   `ci-gate` folds `area-ci: skipped` and passes. Record it as the
   whole-stage half of observation (d); it costs the scope job, one
   preflight leg, and the gate.
4. **Commit B**: the three files above (normalization case 1's tree). Clean
   checkout, no override, `git push`. The hook runs the two packages' gates
   here and publishes both notes. **Run 1** is the main run: observations
   (a)–(d) and case 1. Wait for it to complete before the next push.
5. **Commits C, D, E**: normalization cases 2, 3, and 4 as one commit each,
   waiting for each run to complete; case 4 with `RUSTY_BISCUIT_PRE_PUSH=warn`
   as described above.
6. Fill in section 3's table from the runs, save it as
   `fixtures/throwaway-branch-<date>.md` on `feat/unifi`, close the draft
   pull request, and delete the branch (`git push origin --delete
   fixture/ci-presentation-<date>`). The notes published for its heads
   bind to trees no future head will have and can stay.

Five runs in total. Each is two areas wide; none is a full-scope run.

## 3. Recording template

Save as `fixtures/throwaway-branch-<date>.md` with frontmatter `kind:
fixture-record`, `answers: [B5 (branch half), review-6 findings 1 and 7]`,
`repository: https://github.com/yankeeinlondon/rusty-biscuit`, the branch
name, the pull request number, and `status: complete`.

One row per check that carries an assertion; `check name as displayed` is
the string GitHub shows, copied verbatim.

| run id | PR | commit / case | check name as displayed | conclusion | `ci-gate` | merge box |
|---|---|---|---|---|---|---|
| | | A / run 0 | `area-ci` | skipped | success | |
| | | B / case 1 | `area-ci (claudine/rendezvous) / rendezvous-core / test (ubuntu-latest)` | | success | |
| | | B / case 1 | `area-ci (claudine/rendezvous) / rendezvous-core / wsl2 / test (wsl2-ubuntu)` | | success | |
| | | B / case 1 | `area-ci (claudine/rendezvous) / rollup` | | success | |
| | | B / case 1 | `area-ci (claudine/rendezvous) / accepted-gaps` | skipped | success | |
| | | B / case 1 | `area-ci (worktree) / accepted-gaps` | | success | |
| | | B / case 1 | `accepted gap: worktree / worktree-cli / L2 (windows-latest)` | neutral | success | |
| | | B / case 1 | `ci-gate` | | success | |
| | | C / case 2 | `area-ci (claudine/rendezvous) / rollup` | | failure | |
| | | C / case 2 | `ci-gate` | | failure | |
| | | D / case 3 | `area-ci (claudine/rendezvous) / rollup` | | failure | |
| | | E / case 4 | `area-ci (claudine/rendezvous) / rollup` | | success | |

Then one line per observation, `pass` or `fail`, each naming the run and
the artifact or command it was read from:

- (a) nested-area labels — …
- (b) reused cell, no producer job, visible completed result — …
- (c) gap routing: publisher ran for `worktree` only; neutral check on the
  head commit; artifact download without `actions: read` — …
- (d) mixed presentation; `ci-gate` success on case 1 and failure on case
  2 — …
- (e) normalization cases 1–4 — … (one line each)
- incidental: dependent-seam duration; `prior-local` origin on an
  unchanged package — …

Attach, for the record's reader, `gh run view <run> --json jobs` for run 1
and the `ci-results-claudine--rendezvous.json` cell for
`rendezvous-core/macos-latest/L1` (state, origin, evidence).

## 4. Prerequisites

- **The workflow files as of this working tree**, committed on `feat/unifi`
  and pushed before the branch is cut: `.github/workflows/ci.yml`,
  `.github/workflows/_area-ci.yml`, `.github/workflows/_package-ci.yml`,
  `.github/workflows/_wsl-ci.yml`, and what they call —
  `scripts/ci/affected_scope.py`, `scripts/ci/local_evidence.py`,
  `scripts/ci/publish_gaps.py`, `scripts/ci-rollup.rs`,
  `.github/ci/ci-baseline.toml`, `.github/ci/environments.json` — plus
  `.githooks/pre-push` and `just/ci-local.just` for the local half. The
  fixture proves the shipped shape; a branch cut from an older tip proves
  something else.
- `gh` authenticated on this host: the hook's trigger review runs `gh pr
  list` and fails closed without it.
- `sniff` and `jq` on `PATH` (`just init`); the hook withholds the receipt
  without them.
- A clean checkout for every strict push; a dirty tree replans from the
  working tree and publishes nothing, which loses observation (b).
- `just ci-local --plan` shows `prohibited_cells: []`, and the constraint
  store (`~/.rusty-biscuit/ci-constraints/`, empty on this host on
  2026-09-12) holds no record that a full plan against `main` would
  violate — the branch-creating push is reviewed against that provisional
  plan and is blocked by design if one exists. Do not push past it with
  `--no-verify`; lift or expire the record first.
- The ruleset `protect-your-bacon` still names `ci-verdict`, so the merge
  box reads BLOCKED in every case; the load-bearing column is the `ci-gate`
  check run's conclusion, as the rollout's final section already says.
- Branch name: `fixture/ci-presentation-<date>`, the date of the push.
  Delete it after the record is captured; the run and check-run ids in the
  record are historical from then on.
