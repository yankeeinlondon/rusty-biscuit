# CI/CD in Rusty Biscuit

## Overview

Rusty Biscuit's CI/CD runs on **GitHub Actions** and is layered to match the [testing tier
taxonomy](../testing-strategy.md): fast feedback first, then full coverage, then
nightly/advisory work. Releases are automated through [release-plz](https://release-plz.dev) but
**no crate is published to crates.io** — GitHub releases and version tags are the only
distribution channel today.

Package-area gates use the same `just` recipes developers run locally. Scope selection is handled
by `scripts/ci/affected_scope.py`, which emits **one canonical resolved plan**: the selected
package areas with a reason each, the packages contributing to each area, and one
`{package, environment, gate}` **cell** per unit of work carrying its execution, origin, state,
evidence, and governance. A changed source file selects its owning package; an unchanged reverse
dependency is reported by name and selected nowhere.

**Area groups; package identifies.** Every stored name — artifact, JUnit manifest record, baseline
entry, receipt cell — is keyed on `{package, environment, tier}`. Area is *derived* from each
member's manifest directory with the same rule as `sniff repo package-area` (no committed mapping
file; a drift contract compares the two wherever sniff is installed), and the only stored name
carrying it is the per-area result slice `ci-results-<slug>`. Nested areas such as
`claudine/rendezvous` are their own areas, never folded into a parent.

## Pipeline Layers

The pipeline has four conceptual layers. Each layer answers a different question.

| Layer                     | Question it answers                                               | Blocking?                      |
|---------------------------|-------------------------------------------------------------------|--------------------------------|
| **Local pre-push hook**   | Do source-changed packages pass L1/L2 on this host?                | Strict by default              |
| **Dependency-scoped CI**  | Do changed packages compile and test on the remaining runners?    | Yes                            |
| **Affected coverage**     | Did the changed package closure lose exercised behavior?          | Report-only                    |
| **Nightly / advisory**    | Did anything drift since yesterday?                               | No                             |

### Layer 1 — Local pre-push hook

`.githooks/pre-push` runs `just pre-push` before `git push` completes. That is
`just ci-local --l2`: lint and L1 plus every hostable non-focusing L2 suite for source-changed
packages. On a clean checkout the gates run FROM the reviewed plan described next (`just ci-local
--plan-in`, handed over as `BISCUIT_CI_PLAN_IN`): the planner never selects twice, an L1/L2 cell
that qualifying prior passing evidence already covers is skipped, a cell whose newest prior
evidence is a failure is rerun, and lint runs as always; a dirty checkout or an
`RUSTY_BISCUIT_PRE_PUSH_AREAS` override replans from the working tree for wider feedback and
publishes nothing. A docs-only push gates nothing. Before any gate, the hook reviews every branch update
the push carries, in order — each revision's COMMITTED tree (its own planner, manifests, and
policy, in a temporary worktree unless it is the clean checkout), never the working tree — applies
published evidence to it, prints it, and refuses on a recorded execution constraint scoped to that
update's remote branch (either name when the refspec renames) and remote only when a cell in the
prohibited environment would still execute; reused or absent cells satisfy it.
`just ci-local --plan` is the working-tree preview of that review; the two differ exactly when
the checkout is dirty.

Before any gate, and in every mode, the hook publishes a **scope receipt** under
`refs/notes/ci-local/scope`: the resolved plan and `scope.json` projection for the COMMITTED
`base..head` (never unstaged or untracked files), bound to the exact base, head, and tree. CI's
scope job takes a matching receipt as its plan without running the planner or setting up a Rust
toolchain, falls back to its own calculation on any miss (materializing the toolchain only then)
and says which (`scope-missing`, `scope-base-mismatch`, ...), and ignores it on
`workflow_dispatch`. Its summary also lists the validation environments consulted and matched, the
cells reused from a pass and from a failure, and the refusals by code. The base is the one the CI event will compare with: the remote's
current `main` for a push to `main` (`github.event.before`); otherwise — `ci.yml` fires
`pull_request` for every target branch — the current remote tip of each open pull request's target
branch (listed with `gh pr list` on a GitHub remote; a missing, unauthenticated, or failing `gh`
blocks the push and names the command), each planned and checked in turn, or a provisional plan
against the remote's `main` when no pull request is open. A target branch the same push also
updates is reviewed in both states its run can see — the current tip and the incoming revision —
and a target the push deletes leaves the pull request no base, so that update is blocked. A pull
request opened from the web UI is
a trigger no hook reviews, which is why the provisional plan is constrained too. The receipt binds
the first context's base; a target that advanced past the branch point is reviewed but records no
receipt, and CI calculates scope itself.

The hook uses `sniff` to identify macOS, Linux, native Windows, or WSL2 and publishes a
**validation receipt** under `refs/notes/ci-local/<environment>` — schema version 2, carrying the
reviewed base (the scope receipt's: a pull request's target tip, never a merge base with
`origin/main`), head, tree, and the scope receipt's plan identity plus one record per
`{package, gate}` cell that this run executed with its
outcome, exit code, completion, test counts, duration, gate-input identity, backend proof, and
bounded failure detail. Those records come from the JUnit report each canonical tier recipe
already stages, so a receipt reports what the run *measured* rather than what it claimed. The
reports themselves are retained on the producing host under
`$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/` (root default `~/.rusty-biscuit/ci-evidence`)
before the receipt is written; `host.report_dir` names that directory, and a copy that fails
publishes no receipt.

A **complete** run is evidence whether it passed or failed; `strict` and `warn` both publish one.
A gate that staged no report is recorded `partial` and refused for reuse, so a compile failure
cannot pass for a tested cell. An L2 cell is `partial` unless a backend it required actually drove
a test — an absent backend makes the suite *skip*, and nextest prints PASS in about 0.02 s.
`lint` and `check` stage no report and are therefore always CI-origin.

CI reads **every** note on every environment's notes ref between the merge base and the outgoing
head, so a macOS receipt from this push and a prior `cross-check` WSL receipt combine in one
answer, and two receipts on one environment covering different packages combine across commits;
each cell is resolved by the newest note that qualifies for it. A receipt from an
**older head** is accepted per cell when that cell's *gate-input identity* is unchanged — the
`git ls-tree` entries of the tested package's build closure (dev-dependencies and the lockfile
included) plus that gate's global inputs — and the comparison is recomputed over both trees
rather than read out of the receipt. `schema_version: 1` notes are exact-tree, pass-only,
whole-environment, never upgraded in place, and render their measurements as
`not recorded (v1 receipt)`.

An accepted cell has its *execution* omitted and stays a *cell* with local origin, so the rollup
expects a local-origin result for it instead of reporting `MISSING`. Suppressing the matrix entry
while leaving the policy expecting a CI result was the PR #76 seven-cell regression.

The hook is controlled by `RUSTY_BISCUIT_PRE_PUSH`:

- `scope-only` — resolve and print the plan, publish the scope receipt, run no gate, publish no outcomes, exclude no CI cells
- `off` — deprecated alias of `scope-only`
- `warn` — run and report, but never block the push
- `strict` — run and block the push on failure (the default)

`scripts/cross-check.sh` publishes a `wsl2-ubuntu` receipt only when its WSL leg ran the outgoing
head's exact tree on a clean remote worktree with no test filter; every other run prints why it
published nothing. It ships the developer's local tree, uncommitted work included, so most of its
runs test a tree no head names.

### Execution constraints

Reuse qualifying passing evidence per required cell on every OS. If no
qualifying passing evidence exists, execute the required tests. A request to
avoid rerunning passed tests is not an environment ban. Only a separately
explicit instruction (such as an environment unavailable during maintenance)
creates an execution constraint; never infer a blanket WSL prohibition.

A separately explicit execution ban is **recorded, not remembered**. Each record in the
constraint store — `<home>/.rusty-biscuit/ci-constraints/<repository>/` beside the evidence
directory, unless `BISCUIT_CI_CONSTRAINTS_DIR` overrides it — carries an environment, an optional
gate, a reason, an owner, an expiry, and optionally a repository and branch. The hook derives
`<repository>` from the remote being pushed to and `just ci-local` from `origin`; an unknown
repository reads the store root, recursively, so every record binds. `just ci-local --plan` and
the pre-push hook enforce them; **CI never reads them**, so a constraint can only stop a push and
can never make CI silently skip required coverage. An expired record is announced and ignored; a
malformed one blocks, because an instruction that cannot be read is not one that can be ignored.

`just ci-local --plan` is the pre-trigger review surface: it resolves the plan, prints every cell
with its execution, origin, state, evidence, and governance, and exits non-zero when a prohibited
cell has no qualifying evidence. It runs no gate and starts no build. `--plan-out <path>` writes
the same cells as canonical JSON.

Parallel test workers default to `max(1, logical_cores - 2)` locally. CI uses
all logical cores when there are four or fewer, and `logical_cores - 2` on
larger runners. This preserves capacity on developer and larger shared hosts
without crippling small CI runners. It is a concurrency setting, not CPU
affinity or a guarantee that cores stay reserved. Shared-resource L2 suites
remain at one worker; `l2-parallel-self-spawn` opts isolated suites into the
parallel policy, and an explicit `BISCUIT_L2_THREADS` takes precedence.

`_test_threads` in `just/devops.just` calculates the shared worker default.
It recognizes CI through `CI=true`, `GITHUB_ACTIONS=true`, or a nonempty
`BISCUIT_CI_ENVIRONMENT`; selecting the `ci` Nextest profile alone does not
change a local host's budget. L1, sanity, and real-resource recipes export
`NEXTEST_TEST_THREADS` while preserving an explicit value. Direct local Nextest
runs inherit `test-threads = -2` from `.config/nextest.toml`. Cargo build-job
limits are unchanged.

Existing CI-profile test groups remain narrower limits: Claudine L1 allows
four concurrent tests, Claudine CLI L1 allows one, and Sniff L1 on Windows
allows one. These caps still apply when the overall worker budget is larger.

`RUSTY_BISCUIT_PRE_PUSH_AREAS` (package names or area directories) replaces the computed scope with
a fixed selection. Install the hook once with:

```bash
ln -s ../../.githooks/pre-push .git/hooks/pre-push
```

Run the hook's local regression suite with
`bash .githooks/tests/test-pre-push.sh` when changing its contract.

### Layer 2 — Dependency-scoped CI

`ci.yml` runs on pull requests and pushes to `main`. Its first job validates the canonical recipe
surface, obtains the changed file set from the event's exact base and head SHAs, verifies every
published local receipt per cell, and resolves the plan.

The tested half of a run is **one top-level entry per selected package area**. `ci.yml`'s
`area-ci` job fans out over the planner's `scheduled_areas` and calls `_area-ci.yml`, which fans
out over that area's package matrix and calls `_package-ci.yml`, which delegates the `wsl2-ubuntu`
cell to `_wsl-ci.yml`. That is four levels including the caller — GitHub's maximum, with no margin
for another. Both matrices come from the planner, not from workflow `jq`, so the grouping has test
coverage.

A called workflow's jobs render as `<caller job name> / <called job name>`, so a compile cell
reads `area-ci (claudine) / claudine-cli / check (windows-latest)`: area first, package under it,
environment on the leaf. No display name is parsed for *identity* — that comes from each artifact's
`manifest.jsonl` and `status.json` — but the composite label **is** parsed by
`scripts/ci/runner_loss.py` to attribute a dead runner's cell, so renaming a producer job is a
contract change.

A source-changed package receives lint, L1 across its environments, and its declared higher tiers.
Compile-check is no longer blanket: the planner reads each package's declared Cargo targets from
`cargo metadata`, credits the L1 build with the `lib`, `bin`, and `test` kinds, and schedules a
`check` cell on each native environment where `example` or `bench` targets exist, with
explicit `--examples`/`--benches` selectors in place of `--all-targets`. A package with unchanged
direct reverse dependents also owns a `check` cell on `ubuntu-latest`, which compiles them against
its public API as a second step (Open Question 1, Option B): the dependents get no area, job, or
cell of their own, and the cell reports "also compiled N dependent(s)". Every cell states which gate its
compile coverage came from, and an archive-only environment names the runner that built its
archive rather than claiming to have compiled anything. This alone took the full-scope job
estimate from 486 to 432.

Per-package policy — L2/browser tier ownership, native libraries, Cargo features, runner tools,
and companion suites — lives in each package's `[package.metadata.ci]`; environment capabilities
live in `.github/ci/environments.json`. Documentation, manifests, lockfiles, Just recipes, and
workflow configuration select no package jobs. CI tooling runs its own compact contract suites
(the Python scope tests, the rollup and plan renderer bins, and the `ci_workflow_contracts`
suite, which a workflow edit also selects), and `workflow_dispatch` remains the explicit
full-workspace path. See
[testing-strategy.md](../testing-strategy.md).

Only cells not already satisfied by verified evidence get a hosted runner: the environment lists
the area hands each package are the plan's *executing* set. A reused cell is published straight
into its area's summary, where the grid's "Reused results" table names the receipt's evidence ref,
counts, duration, and host — no setup, build, archive, or test step runs for it.

Skippable jobs carry **no `name:`**. GitHub never evaluates the matrix context for a job it skips,
so a declared `name:` containing `${{ matrix.… }}` reaches the Checks tab as raw expression text
(63 such labels in run 34638047631). Omitting `name:` makes the label the job id when skipped and
`job-id (matrix values)` when it runs. `lint` has no matrix, so it keeps a static
`lint (ubuntu-latest)` — its environment has to be visible.

### Each area owns its outcome

`_area-ci.yml`'s `rollup` job runs `if: always()` behind that area's producers and runs
`ci-rollup rollup --area` then `ci-rollup verdict --area`. It applies that area's baseline, its
governed policy gaps, and the missing-cell rule, and nobody else's: another area's red cell cannot
block it and another area's baseline entry cannot excuse it. It also narrows runner-loss
attribution to its own packages, because a job name carries no area.
`ci-rollup summarize --results <slice>…` folds the slices into one view and applies no policy at
all.

An area whose every cell is reused **still fans out**, so its local-origin results are still
reported somewhere. The matrix is built from each package's declared gates rather than its
executing cells, which is what gets this right.

### Governed policy gaps

A tier a package owns tests for that an environment cannot host is governed once, in
`environments.json`, as a capability object carrying `available: false` plus `reason`, `owner`,
`expiry`, and optionally `closes` — the tracked work that ends the gap. Such a cell is an
**`ACCEPTED GAP`**: a distinct machine-readable state, decided by the planner before the run and
never inferred from a GitHub cancellation conclusion. It is neither a pass nor a test failure and
does not block; the rollup renders its owner, expiry, policy entry, `closes` link, and revocation
instructions where a reader sees the cell. An absent, incomplete, or expired acceptance is a
blocking `POLICY GAP` instead, and a real failure outranks an accepted gap.

The gap is also visible **immediately**, before any producer runs: `_area-ci.yml`'s `accepted-gaps`
job waits on nothing, reads the plan the scope job uploaded, and runs `scripts/ci/publish_gaps.py`,
which creates one `neutral` check run per accepted-gap cell on the pull request head (Open
Question 4, ruled 2026-09-12). The check's name identifies the cell; its title and summary carry
the `ACCEPTED GAP` marker, owner, expiry, and reason; its text carries the revoke instructions and
the `closes` work; `details_url` links the policy entry. `neutral` leaves the PR clean and never
alters the run's conclusion — `cancelled` keeps its one meaning, interruption — and the tool
refuses an ungoverned or expired cell rather than publish it as harmless. That job is the only
one holding `checks: write`; `ci.yml`'s `area-ci` carries the grant as a cap and `package-ci` and
`rollup` declare read-only sets of their own.

### The merge gate

`ci.yml`'s `ci-gate` job is the single required check: a **policy-free fold**. It `needs` every
blocking top-level job, runs `if: always()`, and passes only when each `needs.*.result` is
`success` or `skipped` — an unselected area's job is skipped and must not block, while `failure`
and `cancelled` do. It reads no plan, policy, baseline, or artifact; a `MISSING` cell is caught by
its area's own rollup, which is the only place judgement lives. `continue-on-error` hides a failure
from the fold, so exactly one job (`ci.yml`'s advisory summary) carries it. The semantics were
proven in a scratch repository (`fixes/2026-09-11-cicd-cleanup/fixtures/scratch-2026-09-12.md`).
The `protect-your-bacon` ruleset still names `ci-verdict`, the required check until 2026-09-12;
until Ken switches that context to `ci-gate` — after this change's own run is green — every PR
shows `ci-verdict — Expected` and cannot merge.

`ci-results.json` is `schema_version: 3`, versioned independently of the baseline's 2. Identity is
still `{package, environment, tier}`; each cell also carries its derived `area`, its `origin`
(`ci`, `local`, `prior-local`, or `none`), the `evidence` behind a reused result, its measured
`duration_s`, and the `target_kinds` and `compile_coverage_from` the plan assigned it. The
document carries `accepted_evidence`, one entry per reused cell — the same set accepted for
*scheduling*, so the scheduler and the report cannot disagree. A document from an earlier
generation is refused with a migration error rather than partly read.

### Layer 3 — Affected coverage and specialized workflows

On pull requests, `ci.yml` passes the affected package closure to one `cargo llvm-cov` invocation
and uploads one LCOV artifact (`lcov-affected`). It does not perform a package-by-package pass and
then repeat the workspace.

Specialized runtime contracts are **reusable workflows called by `ci.yml`**, not independently
path-triggered ones, so a commit produces one CI run rather than a wall of overlapping ones. Each
is selected from affected scope and gated on preflight:

| Workflow | Selected when | Unique evidence |
|---|---|---|
| `biscuit-tui-windows-captured-stdout.yml` | `biscuit-tui` in scope | attached-console captured-stdout boundary |

Messenger and all three Rendezvous crates are owned by their ordinary
package-keyed L1 cells on Ubuntu, Windows, macOS, and WSL2. Messenger declares
all-feature coverage and the closed `messenger-desktop-stubs` runner tool. The
native workflow builds and verifies all six helpers once before L1 and exports
`MESSENGER_STUB_BIN_DIR`; the WSL2 archive workflow ships Linux helpers as a
sidecar to its toolchain-free guest. JUnit and producer-status artifacts retain
the package/environment/tier identity consumed by their area's rollup.
`sniff-performance.yml`
stays independent because its PR leg is artifact-only and its scheduled leg measures work counts,
not correctness. `build-integrations.yml` stays release-triggered.

### Layer 4 — Nightly and advisory

Each scheduled workflow owns its own name, schedule slot, artifacts, and summary so none can be
mistaken for required validation: fuzz 02:00, sniff-performance 04:00 UTC, maintenance audit
Mondays 07:00.

Coverage and workspace benchmarking left CI on 2026-08-12: coverage is a local tool (`just
coverage` per package), and `bench-nightly`'s Bencher.dev upload had been failing silently for
weeks — performance testing returns as the opt-in, package-owned design in
`features/2026-08-12-perf-opt-in/spec.md`.

The 90-minute budget is provisional: warm scheduled runs measured 14–18 minutes, but every
cold-cache run was truncated by the previous 30-minute ceiling, so the cold duration has never
actually been observed. Tighten the budget — or split the 16 bench targets across parallel jobs —
once a cold run has been recorded.

#### `fuzz-nightly.yml` — 02:00 UTC daily

Two matrix jobs (`biscuit-file`, `darkmatter`) on the **nightly** toolchain, capped at 10,000 runs
and 300 s per target. The interesting policy bits:

- **Replay-first.** Committed crash corpora (`fuzz/artifacts/<target>/`) are replayed with
    `-runs=0` before any new fuzzing. A regression in a previously-fixed crash fails the run
    immediately and loudly.

- **Auto-issue on new crash.** When a *new* crash is found, the workflow opens a GitHub issue
    de-duplicated by target marker + crash signature. Reproduction instructions are embedded in the
    issue body.

- **Advisory.** Fuzz nightly never gates merges; it produces actionable issues instead.

#### `maintenance-audit.yml` — Mondays 07:00 UTC and manual

Reports what has moved upstream for every value the repository pins on purpose — the required Rust
version, the kache version floor, `cargo-nextest`, third-party GitHub Action versions, and the runner image —
and changes nothing. The job always succeeds; a finding is information. Pins advance only through a
reviewed change (see [Advancing a pinned value](#advancing-a-pinned-value)).

#### `build-integrations.yml` — on `release: published`

Cross-compiles the three Unfolded Circle integrations (`arcam-amp-integration`,
`eversolo-integration`, `sony-receiver-integration`) to `aarch64-unknown-linux-musl` using `cross`,
then uploads tarballs to the GitHub release via `gh release upload`. `fail-fast: false` so one
target's failure doesn't strand the others.

## Release Strategy

Releases are automated end-to-end by `release-plz.yml` in the public repository.

### Two-job flow

1. After the `ci` workflow **succeeds** on `main` (a `workflow_run` trigger, not a bare push — release
   automation follows the validation it depends on rather than racing it), **`release-pr`** runs
   `release-plz release-pr`. It opens (or updates) a single **draft**
   release PR labeled `release`, `automated`. The PR contains version bumps and changelog updates
   for every package with relevant commits since the last tag. Concurrency is configured to be
   **non-cancelling** so two concurrent runs cannot race the PR head.

2. When a merged PR labeled `release` closes, **`release-plz-release`** runs `release-plz
   release`. It creates git tags shaped `{{ package }}-v{{ version }}` and publishes GitHub
   releases with the rendered changelog. Ordinary pushes do not start the publishing job.

### What we do *not* do

- **No crates.io publishing.** `publish = false` is set workspace-wide in `release-plz.toml`. If a
    crate ever needs to ship to crates.io, that's an explicit, per-package decision.

- **No version bumping on PRs that don't touch a published package.** `release-plz` is
    conventional-commit aware (`feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `chore(deps)`,
    `chore`) and skips `chore(release)`.

- **No release for excluded packages.** `biscuit-tui`, `biscuit-tui-cli`, `tabby`, and the `ui`
    package opt out.

### Versioning policy

- **SemVer checks are disabled.** `semver_check = false` avoids release-plz regenerating
    intentionally untracked nested-workspace lockfiles; with `publish = false`, there is no
    crates.io consumer requiring that publication gate.

- **Per-package changelogs** at `<area>/CHANGELOG.md` for nine packages (the rest aggregate into the
    workspace root changelog).

- **Conventional commit prefixes** drive both the bump level and the changelog section. See the
    `commit_parsers` array in `release-plz.toml` for the canonical mapping.

## Caching and Performance

Every Rust workflow uses `Swatinem/rust-cache@v2` with `workspaces: ". -> target"` and a workflow-
or matrix-scoped `shared-key`. The package gates key **per package and per job kind**:
`package-ci-<package>-check-<os>`, `package-ci-<package>-lint-ubuntu-latest`, and
`package-ci-<package>-test-<environment>`. The L2, browser, and WSL-archive jobs deliberately
reuse the `test` key for their environment — they compile the same crates as the L1 leg, so one
warm cache serves every tier instead of three cold ones. Other examples: `coverage-affected`,
`coverage`, `bench-nightly-darkmatter`, and `sniff-bench`. Cache keys are intentionally scoped
rather than global — this trades hit rate for protection against a poisoned target directory
taking down the entire pipeline.

The package-scoped key has **not** been measured against a real run yet. The known pressure is
GitHub's 10 GB repository cache quota: roughly 5 keys × 63 packages means one full run saves more
caches than the quota holds and evicts its own predecessors, so only intra-run reuse is reliable
today. Do not diagnose a cold build as a cache-key bug.

Concurrency is configured per-workflow:

- Dependency-scoped CI: `cancel-in-progress: true` per ref, so force-pushing a fix cancels
    the prior run.

- `release-plz` and the fuzz/bench nightlies: **never cancel** — partial state from these is
    always more useful than nothing.

## Required Toolchain on Runners

`rust-toolchain.toml` pins one **exact** Rust version for the whole repository, and required CI
honors that file with `rustup show` rather than overriding it with a floating channel. Local and CI
therefore resolve the same compiler — which also stabilizes rustfmt and Clippy, curing the
`main`↔branch formatting drift documented in `CLAUDE.md`.

Two deliberate overrides exist, both outside required CI:

- **`rust-latest-stable.yml`** sets `RUSTUP_TOOLCHAIN=stable` to test the newest compiler in
  advance. Advisory; it cannot change required-CI behavior.
- **`fuzz-nightly.yml`** uses nightly because `cargo-fuzz` requires it.

Coverage adds `llvm-tools-preview` on top of the pinned toolchain.

### Advancing a pinned value

The maintenance audit reports drift; advancing a pin is a reviewed change:

1. Check the most recent `rust-latest-stable` run (for a toolchain bump) or the upstream release
   notes (for an action, kache, or nextest bump).
2. Update the single authority — `rust-toolchain.toml`, `.github/kache-min-version`, or the `uses:`
   pin — never a second copy.
3. Run `cargo fmt --all --check` (read-only; never write-mode), plus the affected areas' `just
   build`, `just test`, and `just lint`.
4. Review newly enabled compiler and Clippy diagnostics rather than silencing them.
5. Keep action-version upgrades in their own commit, separate from behavior changes.

Roll back by reverting that one authority value; nothing else encodes it.

Shared CLI tools used in CI:

- `python3`, `jq`, and `gh` — scope calculation and GitHub orchestration.
- Node.js, npm, and pnpm — frontend legs declared with the `node` capability.
- `cargo-nextest` — the canonical test runner for L1 tiers.
- `just` — orchestration entry point for every job.
- `cargo-llvm-cov` — coverage.
- `cargo-fuzz` — fuzz targets.
- `cross` — integration cross-compilation.
- `bencher` — nightly benchmark upload.
- `release-plz` — release planning and publication.

The root `just init` recipe has a CI/CD stage that ensures the applicable
local equivalents. Binaries encapsulated entirely inside a third-party action
remain owned by that action.

## Policy Summary

What a reviewer can rely on when approving a PR:

1. **Every affected gating package passed its configured environment matrix** against the exact
   pinned Rust version in `rust-toolchain.toml`. Native L1 runs on Linux,
   Windows, and macOS; `wsl2-ubuntu` is a distinct archive-based L1 cell.
2. **`just check-canonical` confirms the area structure is well-formed** — no `justfile`
   recipe drift snuck in.

3. **An unchanged downstream consumer is reported, not scheduled.** It appears by name in the
   plan's `reverse_dependencies` and receives no area, job, or result cell, so an untested area
   can never present as a green top-level result. The specialized runtime contracts are still
   selected from affected scope by `ci.yml`.

4. **Coverage is reported but not gated.** Treat coverage as a delta to inspect, not a number to
   defend.

5. **Bench and fuzz drift is captured nightly,** not on the PR itself. A regressed fuzz target
   files an auto-issue rather than blocking your merge.

6. **Releases never happen from a PR branch.** They happen from `main` via release-plz's draft
   PR, which is itself reviewed before merge.

What CI explicitly does **not** guarantee:

- **Compile coverage inside the WSL2 guest.** L1 compiles and runs the `lib`, `bin`, and `test`
    kinds on each native environment; the guest only runs the `ubuntu-latest` archive. A `check`
    cell on each native environment compiles `example` and `bench` through explicit
    `--examples`/`--benches` selectors, and it is scheduled for packages that declare those
    kinds; a package with neither gets a check job only on `ubuntu-latest`, and only to compile
    its unchanged direct reverse dependents.

- **Performance regressions blocking merge.** Bench results are tracked in Bencher but not gated.
- **External-resource (L4 `test-real`) tests passing.** Those tiers are explicitly excluded from
    CI; they live on developer machines and the homelab.

## Adding a New Workflow

Before adding a new workflow, check:

1. **Does an existing canonical recipe cover this?** If yes, register the area in the root
   `justfile` and declare the package's CI policy in its `[package.metadata.ci]`
   instead of inventing a new workflow.

2. **Is this a canonical area contract or a specialized contract?** Canonical work belongs in
   `_package-ci.yml` and the package policy; specialized hardware, IPC, or console behavior may justify
   its own file. If it does, make it a **reusable** workflow (`workflow_call` +
   `workflow_dispatch`, no `push`/`pull_request` triggers, no own `concurrency` group) and add a
   scope-gated job to `ci.yml` that calls it. A self-triggering workflow reintroduces the wall of
   parallel runs per commit that the orchestrator exists to prevent.

3. **Should this gate merges or just report?** Mirror the existing pattern — coverage,
   bench, fuzz, sniff-performance, and the maintenance audit are non-gating; everything else is.
   A non-gating workflow needs its own name, schedule slot, artifact names, and summary.

4. **Pick a unique `shared-key`** for the cache so you don't share state with an unrelated job.
5. **Honor `rust-toolchain.toml`** with `rustup show`; never override it with a floating
   `dtolnay/rust-toolchain@stable`, and never rely on the runner's default. Nightly and
   latest-stable overrides are deliberate exceptions, documented where they occur.
6. **Install native prerequisites before building** with `just _ensure-native-libs <area>`, so a
   `-sys` crate cannot fail to compile for a missing system library.

## Pointers

- Workflow definitions: `.github/workflows/`
- Release config: `release-plz.toml`
- Test tier taxonomy: [`testing-strategy.md`](../testing-strategy.md) and
    `.claude/skills/rust-testing/SKILL.md`

- Pre-push hook: `.githooks/pre-push`, tested by `.githooks/tests/`
- Canonical recipe definitions: root `justfile` and `just/*.just`
