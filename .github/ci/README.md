# CI policy — packages and environments

CI selects, executes, and records work by **package** — a Cargo workspace
member. Every result identity — artifact name, JUnit manifest record, rollup
cell — is keyed on `{package, environment, tier}`.

- **Package policy** lives in the package's own manifest, under
  `[package.metadata.ci]`. `scripts/ci/affected_scope.py` reads and validates
  it.
- **Environment capabilities** live in `environments.json` (below). One
  versioned, schema-validated table.
- **Known-red legs** live in `ci-baseline.toml`, keyed by package.

There is no per-directory policy store. **Package is the stored identity** of
every artifact, JUnit record, baseline entry, and receipt cell. **Area is a
derived grouping**: `affected_scope.py` computes it from each member's manifest
directory using the same rule as `sniff repo package-area`, emits it on every
matrix and policy record, and a drift contract in
`scripts/ci/test_resolved_plan.py` compares the two wherever sniff is installed.
No mapping file exists, because a second policy store would be free to drift.
Nested areas such as `claudine/rendezvous` are their own areas, never folded
into a parent. `just test` in a directory still runs that directory's packages
for local use (R8); CI does not read that list.

An unchanged direct reverse dependent is reported in the plan's
`reverse_dependencies` and selected nowhere — no area, no job, no result cell.
It used to receive a compile-check entry, which presented an untested area as a
green top-level result (PR #76). Its seam is compiled instead inside the
changed package's own `ubuntu-latest` check cell (Open Question 1, ruled
Option B 2026-09-12): the changed package's record carries `dependent_seam`
— the sorted dependent names and one explicit `cargo check` argument string —
and a failure lands on the changed package's area.

## Reusing PR validation after a merge

PRs always run dependency-aware CI. A push to `main` first runs a lightweight
verification job (`scripts/ci/reuse_validation.py`). It skips the expensive
grid only when all of these conditions hold:

- GitHub associates the pushed commit with a merged PR targeting this
  repository's `main`, and the PR's merge commit is the pushed commit.
- The latest matching `pull_request` run of this repository's `ci.yml` has
  completed successfully. The commit endpoint supplies the authoritative PR
  association because GitHub may leave a workflow run's `pull_requests` array
  empty. An older green run cannot override a newer failed, canceled, or
  in-progress run.
- That run has an unexpired `ci-validation-v1-<tree>-<base>-<head>` artifact
  matching the pushed Git tree, pre-push `main` commit, and PR head.

The scope job records the receipt from its actual synthetic merge checkout
and verifies both parents against the PR event. The entire Git tree must
match, including workflows, toolchain, lockfiles, package policies, and test
configuration. The base comparison prevents a stale PR validation from
covering a different integration base or an untested batch of pushes. Merge,
squash, and rebase commits can have different commit IDs; reuse depends on
their file content and recorded integration base.

Receipts are retained for seven days. Searches are bounded, and absent,
expired, malformed, or inaccessible evidence schedules normal CI. Direct
pushes without a matching merged PR also run normal CI. `workflow_dispatch`
always runs the full grid and is the way to force fresh validation, including
when investigating changes to external dependencies or hosted runners.

On reuse, `scope`, `preflight`, the whole `area-ci` fan-out (which needs
`scope`), and every rollup are skipped; `ci-gate` folds those skipped results
and passes, and the advisory summary links the original PR run. The `ci`
workflow still completes on `main`, preserving Release-plz's existing
successful-CI trigger. Runs predating receipt publication cannot be reused.

Validate this boundary with `python3 scripts/ci/test_reuse_validation.py`,
`actionlint .github/workflows/ci.yml`, and the `ci_workflow_contracts` nextest
suite. The Python suite also runs in preflight on every selected OS and in
the CI-tooling job, which runs the `ci_workflow_contracts` suite as well.

## `environment` is not `os`

Windows, macOS, and Linux are operating systems. **WSL2 is a distinct supported
Linux environment that a Windows runner hosts.** Policy and every result
identity are keyed by **environment**; only `runs-on` and the native-package
lookup are keyed by runner OS.

| environment | hosted by runner label | notes |
|---|---|---|
| `ubuntu-latest` | `ubuntu-latest` | |
| `windows-latest` | `windows-latest` | |
| `macos-latest` | `macos-latest` | |
| `wsl2-ubuntu` | `windows-latest` | runs through `wsl-bash`; see `.github/workflows/_wsl-ci.yml` |

`affected_scope.py` derives the per-package workflow inputs from the
environment capability table so the reusable workflow can never route
`wsl2-ubuntu` into a `runs-on` matrix: `native_environments` (the environment
names that *are* runner labels), `l2_environments`, `browser_environments`,
`node_environments`, and `wsl` (a boolean).

It also derives `gates` — which of `lint`, `check`, and `test` this run selected
the package for. A package owning a changed source file carries `lint` and
`test` always, and `check` when it declares example or bench targets or has
unchanged direct reverse dependents to compile: the L1 build already compiles
the library, binary, and test targets, so a separate compile job is scheduled
for the kinds no test gate produces and, on `ubuntu-latest` only, for the
dependents' seam. Unchanged reverse dependents, their dependencies, and
transitive reverse dependencies are not selected. Documentation, manifests, lockfiles, Just recipes, workflows,
and other CI configuration select no package jobs; CI tooling has its own
small contract-test leg. Only an explicit `workflow_dispatch` full-scope run
selects every package.

Source classification is path based: `build.rs` and package-owned files with a
known programming or web-source extension select their owning package. A gate
absent from `gates` schedules no job.

## Local environment evidence

Two documents ride on `refs/notes/ci-local/*`, and they make different claims.

The **scope receipt** (`refs/notes/ci-local/scope`, one ref for every host,
schema version 1) is what the planner selected for one committed `base..head`:
the canonical resolved plan plus the legacy `scope.json` projection `ci.yml`
fans out from, bound to the exact `{base, head, tree}`. The hook publishes it
in every mode before any gate runs, from the committed path set and never the
worktree's, so a dirty tree still publishes scope. Its base is the comparison
base of the first run the update triggers: the remote's current `main` for a
push to `main` (`github.event.before`); otherwise the current remote tip of
the target branch of the first open pull request from that head
(`pull_request.base.sha`, listed with `gh pr list` on a GitHub remote), or the
remote's `main` when no pull request is open yet. A target that has advanced
past the branch point is reviewed for constraints but records no receipt —
the receipt requires an ancestor base — and CI calculates scope itself. A
target branch the same push also updates is reviewed in both states it can
be in (its current tip and the incoming revision), and the receipt binds the
current tip; a target the push deletes blocks that update. CI's
scope job runs `local_evidence.py scope-verify`
first; on an exact match it emits the carried documents as the plan and the
policy artifact **without running the planner**, and on any miss —
`scope-missing`, `scope-schema`, `scope-head-mismatch`, `scope-tree-mismatch`,
`scope-base-mismatch`, `scope-malformed` — it calculates scope exactly as
before and names the code in the summary's `scope source` row. The job
materializes the pinned Rust toolchain (`rustup show`) only on that miss,
immediately before the selection run that reads `cargo metadata`; a hit is
Python and jq end to end and sets up no toolchain (R9). Whichever way the
scope was sourced, the summary's `validation environments`, `reused passing
cells`, `reused failing cells`, and `cells retained (evidence incomplete or
rejected)` rows report which `refs/notes/ci-local/<environment>` refs the
verifier consulted and which matched, the cells reused from a pass and from a
failure, and the refusals by code — read from the written plan, so they
describe exactly what the fan-out sees. `workflow_dispatch` never consults it. When a matching receipt is combined
with accepted validation cells, selection still never runs: the accepted set is
applied to the carried plan (`affected_scope.py --apply-to`, which reads no
manifest, policy, or environment table) and `scope.json` is re-projected from
the result. Only a cell's execution, origin, state, and evidence, plus the
fields derived from them (`accepted_evidence`, `evidence_rejections`,
`prohibited_cells`, `job_estimate`, `change_class`, `preflight_os`,
`preflight_reason`), can differ from the receipt; every other execution
attribute reaches the fan-out as the hook resolved it. The same operation
applies evidence on a scope miss, after the one selection run.

The **validation receipt** is what a host measured. The pre-push hook runs
lint plus L1 and hostable L2 for source-changed packages. On a clean outgoing
`HEAD` with no `RUSTY_BISCUIT_PRE_PUSH_AREAS` override it runs them FROM the
reviewed plan above (`just ci-local --plan-in`, handed over as
`BISCUIT_CI_PLAN_IN`): the planner does not run again, an L1/L2 cell that
qualifying prior *passing* evidence already covers is skipped, a cell whose
newest prior evidence is a failure is rerun (ruling D2), and lint runs as
always. A dirty checkout or an override replans from the working tree for
wider feedback and publishes nothing. It uses `sniff os --json` to identify macOS, Linux, native Windows,
or WSL2. For a clean outgoing `HEAD` it publishes a **validation receipt** under
`refs/notes/ci-local/<environment>`: schema version 2, carrying the reviewed
base (the same base as the scope receipt — a pull request's target tip, never
a merge base with `origin/main`), head, tree, the scope receipt's plan
identity, and one record per `{package, gate}` cell that this run executed with its
outcome, exit code, completion, test counts, duration, gate-input identity,
backend proof, and bounded failure detail. Those records come from the JUnit
report each canonical tier recipe already stages, so a receipt reports what the
run measured rather than what it claimed. Those reports are retained on the
producing host under `$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/`
(root default `~/.rusty-biscuit/ci-evidence`), and the receipt's
`host.report_dir` names that directory; the copy happens before the receipt is
written, and a copy that fails publishes no receipt.

A **complete** run is evidence whether it passed or failed. `strict` and `warn`
both record one; a gate that produced no report is recorded `partial` and is
not reusable, so a compile failure cannot pass for a tested cell. An override
(`RUSTY_BISCUIT_PRE_PUSH_AREAS`) or a dirty tree publishes no validation
receipt (the scope receipt still goes out), and so does a run in which every
recordable cell was already covered; every withheld receipt is announced with
its reason on stderr.

L2 uses every available non-focusing backend declared by the package: detached
tmux, background WezTerm, or keep-focus Kitty. Apple Terminal and Level 3 are
excluded because a push hook must never take window focus. An L2 cell is
recorded `partial` — published, and refused for reuse — unless a backend it
required actually drove a test: an absent backend makes a Level 2 suite *skip*,
and nextest prints PASS in about 0.02 s, which is indistinguishable from
evidence unless the proof is read. The receipt covers
L1, L2, and browser only: `lint` and `check` stage no report and are always
CI-origin, and an environment needed by a companion suite is retained even if
that duplicates its Rust L1 run. Manual full-scope runs ignore local receipts.

`local_evidence.py verify --cells` reads **every** note on every environment's
notes ref between the merge base and the outgoing head and returns the accepted
cells from all of them, plus a coded reason per refused candidate. A macOS
receipt from this push and a prior WSL receipt from `cross-check` therefore
combine in one run, and so do two receipts on one environment that covered
different packages on successive commits: each cell goes to the newest note that
qualifies for it, so an older pass never overrides a newer complete failure and
a refused newer note never hides an older one that still qualifies. The scope
calculator applies that set to the plan in hand (`--apply-to` with
`--accepted-cells`); an accepted cell has its *execution* omitted and stays a
*cell* with local origin, so the rollup expects a local-origin result for it
rather than reporting MISSING — the PR #76 regression. A cell records whether
a receipt may ever satisfy it (`reusable`): `check` cells and the L1 host a
companion suite needs are never reused, whatever a receipt claims.

A receipt from an **older head** is accepted only when the cell's *gate-input
identity* is unchanged: the `git ls-tree` entries of the tested package's build
closure (dev-dependencies included) plus that gate's global inputs. Both trees
are present locally, so the comparison is made rather than read out of the
receipt. `schema_version: 1` notes predate per-cell outcomes; they are accepted
on exact tree identity only, pass-only, whole-environment, are never upgraded
in place, and render their measurements as `not recorded (v1 receipt)`.

`scripts/cross-check.sh` publishes a `wsl2-ubuntu` receipt when its WSL leg ran
the outgoing head's exact tree on a clean remote worktree with no test filter.
Any other run prints why it published nothing — it ships the developer's local
tree, uncommitted work included, so most of its runs test a tree no head names.

### Execution constraints

A restriction such as "do not rerun WSL" is recorded in a constraint store, not
remembered: each record names an environment, an optional gate, a reason, an
owner, an expiry, and optionally a repository and branch. `just ci-local --plan`
and the pre-push hook read it; **CI never does**, so a constraint can only stop
a push and can never make CI silently skip required coverage. The hook reviews
every branch update the push carries first — each revision's committed
`base..head` path set, planned by its committed tree's own planner, manifests,
and policy (a temporary worktree unless it is the clean checkout), never the
working tree — under that update's remote branch (both names when the refspec
renames) and the remote being pushed to, and decides from its executions: a
prohibited environment blocks only when a cell there would still execute, so
reused or absent cells satisfy the record. HEAD's reviewed plan is the scope
receipt it then publishes. The repository field is the remote's URL
without scheme, credentials, or `.git` (`github.com/yankeeinlondon/rusty-biscuit`),
so one record covers every spelling and worktree of a clone. An expired record
is announced and ignored; a malformed one blocks, because an instruction that
cannot be read is not one that can be ignored.

The store is `<home>/.rusty-biscuit/ci-constraints/<repository>/`, beside the
evidence directory, where `<repository>` is that same identity as nested
directories (`github.com/yankeeinlondon/rusty-biscuit/`; a port or drive colon
becomes `_`) and `<home>` is Python's `Path.home()` — `HOME` on Unix,
`USERPROFILE` on native Windows. `BISCUIT_CI_CONSTRAINTS_DIR` overrides it. A
push to a remote whose identity is unknown reads the store root, recursively,
so every repository's records bind. Only the hook and `just ci-local` resolve
that default (`scripts/ci/constraints.py directory`); the planner takes the
store as an explicit `--constraints` argument, which CI never passes.

`just ci-local --plan` is the developer's working-tree preview of that review:
it resolves the plan, prints every cell with its execution, origin, state,
evidence, and governance, and exits non-zero when a prohibited cell has no
qualifying evidence. It runs no gate and starts no build. It and the hook's
review differ exactly when the checkout is dirty.

Skipping the local hook with `--no-verify` creates no new evidence but does not
disable previously published matching notes. CI still verifies those notes.

A WSL2 guest *is* Linux, so `_ensure-native-libs` keys off `uname -s` and reads
the package's `ubuntu-latest` list. `native` therefore stays a **runner OS**
map (keyed by `ubuntu-latest`/`macos-latest`/`windows-latest`) and must not
grow a `wsl2-ubuntu` key.

Cargo metadata — not this file — remains the source of truth for package
membership.

## `environments.json`

One versioned, schema-validated capability table. It defines, for each
environment: the `runner` that hosts it, the `native_key` that maps it to a
native-package installer, and a `capabilities` map over a closed vocabulary:

| capability | meaning |
|---|---|
| `tmux` | whether a headless L2 terminal backend can be provisioned here |
| `headless_browser` | whether a headless browser can be hosted here |
| `node_pnpm` | whether Node 22 + pnpm 10 are provisioned here |
| `archive_only` | whether this environment runs from a prebuilt nextest archive (no Cargo) |

A capability value is either a boolean or, for a **governed unavailability**, an
object carrying `available: false` plus `reason`, `owner`, `expiry`, and
optionally `closes` — the tracked work that ends the gap. A plain `false` is an
**ungoverned** absence — the `POLICY GAP` cell it produces is never excused and
blocks. The facts the eight per-area `policy_gaps` records used to restate —
Windows has no tmux, the WSL2 leg is archive-only, and the browser tier is
Linux-hosted — are now declared once, here, with full governance.

A cell whose gap is governed and unexpired is an **`ACCEPTED GAP`** in the
results: a distinct machine-readable state, decided by the planner before the
run and never inferred from a GitHub cancellation conclusion. It is neither a
pass nor a test failure, and the rollup renders its owner, expiry, policy entry,
`closes` link, and revocation instructions where a reader sees the cell. An
absent, incomplete, or expired acceptance is a blocking `POLICY GAP` instead.
The two current L2-backend gaps name
`features/_unscheduled/windows-l2-ci-leg` and
`features/_unscheduled/wsl2-l2-ci-leg` as what closes them.

**Published immediately, as a `neutral` check run per cell.** The rollup's
grid arrives only after the area's producers finish, so `_area-ci.yml`'s
`accepted-gaps` job — which `needs` nothing — reads the plan the scope job
uploaded and runs `scripts/ci/publish_gaps.py`, which creates one check run
per accepted-gap cell on the pull request head (`POST
/repos/{owner}/{repo}/check-runs`; `github.sha` on a `pull_request` event is
the merge commit, and a check there never reaches the PR's checks list). The
check's name identifies the cell (`accepted gap: <area> / <package> / <gate>
(<environment>)`); its title and summary carry the `ACCEPTED GAP` marker with
the environment, package, gate, capability, owner, expiry, and reason; its text
carries the change/revoke instructions and the `closes` work; `details_url`
links the policy entry's line at the tested head. The conclusion is `neutral`
(Open Question 4, ruled 2026-09-12): it leaves the PR clean and never alters
the run's conclusion, and `cancelled` keeps its single meaning of
interruption. The publisher refuses — exit 2, job red — a cell whose record is
ungoverned, incomplete, or expired, because a `neutral` check for a cell the
rollup is about to block on would misrepresent it; an area with no accepted
gap skips the job on the scope job's `gap_areas` output. Presentation only:
nothing reads the check back.

`checks: write` is held by that job alone. A called workflow's token is
capped by its caller's, so `ci.yml`'s `area-ci` job carries the grant as a
cap; `_area-ci.yml` then confines it — `package-ci` declares `contents: read`
(which `_package-ci.yml` and `_wsl-ci.yml` inherit; neither uses the token)
and `rollup` stays at `checks: read`. The workflow-contract suite pins all of
that.

An L2 tier is hostable where ANY of its declared backends is: each backend is
looked up as a capability under its own name (`tmux` today; `wezterm`,
`kitty`, and `apple-terminal` get the same axis if they ever become
CI-hostable), and a backend with no capability entry is hostable nowhere.

Capability only: package policy decides *which* tiers are expected, so an
unsupported required tier becomes an explicit `POLICY GAP` in the grid rather
than disappearing. `affected_scope.py::load_environments` validates the schema
loudly; `affected_scope.py::package_ci_policy` cross-checks every package's
declared native packages against the runner labels the table defines.

## `[package.metadata.ci]`

Package policy lives in the package's own manifest, following the
`[package.metadata.benchmarks]` pattern already used across this workspace:

```toml
[package.metadata.ci]
gates = false
exclusion-class = "promotion-pending"
owner = "@yankeeinlondon"
reason = "…"
expiry = "2027-01-31"

[package.metadata.ci.native]
ubuntu-latest = ["libasound2-dev"]

[package.metadata.ci.tests]
tiers = ["L1", "L2"]
l2-backends = ["tmux", "wezterm"]
features = ["playa"]
all-features = false
l1-include-slow = false
runner-tools = ["ai-provider-stubs", "darkmatter-md-fixture"]
companion-suites = ["homelab-frontend"]
```

A package with no CI metadata defaults to `gates = true` and the L1 tier —
non-gating is never inferred from zero observed tests (AC15), because that
would silently exempt a package and miss its first test.

### Fields

| Field | Type | Default | Meaning |
|---|---|---|---|
| `gates` | bool | `true` | whether the package fans out in CI |
| `exclusion-class` | string | when `gates = false` | `capability`, `promotion-pending`, or `time-bounded` |
| `owner` | string | when `gates = false` | GitHub handle accountable for closing the exclusion |
| `reason` | string | when `gates = false` | why it does not gate, and what would unblock it |
| `expiry` | ISO date | when `gates = false` unless `capability` | a **past** date fails the scope calculation |

`[package.metadata.ci.tests]`:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `tiers` | string[] | `["L1"]` | CI-gating tiers this package owns. Must include `L1`; `L2`/`browser` are opt-ins |
| `l2-backends` | string[] | `[]` | L2 terminal backends this package's tests require; one of `tmux`, `wezterm`, `kitty`, `apple-terminal`. Required when `L2` is declared |
| `features` | string[] | `[]` | forwarded to check, archive, and the canonical recipe consistently. Conflicts with `all-features` |
| `all-features` | bool | `false` | run with `--all-features`. Conflicts with `features` |
| `l1-include-slow` | bool | `false` | keep `slow_` tests inside the L1 selection (darkmatter's contract) |
| `runner-tools` | string[] | `[]` | closed vocabulary: `ai-provider-stubs`, `darkmatter-md-fixture`, `messenger-desktop-stubs`, `node-22`, `pnpm-10`, `l2-parallel-self-spawn`, `neovim`, `zed-extension` |
| `companion-suites` | string[] | `[]` | non-Cargo suites this package owns; closed vocabulary: `homelab-frontend` |

`[package.metadata.ci.native]`: a map of runner OS (`ubuntu-latest`,
`windows-latest`, `macos-latest`) → system packages needed to build/test. The
union of a selected package's `native` requirements across its dependency
closure reaches every job that compiles or runs it — a dependent job that
compiles `playa` needs ALSA even though it is not testing `playa` (R5).

Validation (`affected_scope.py::validate_package_ci`) rejects unknown fields,
invalid tier or tool names, conflicting `features`/`all-features`, expired
exclusions, an L2 tier without backends, l2-backends without L2, and a
companion suite whose canonical recipe does not exist in its owning directory's
justfile.

### `runner-tools` is a closed vocabulary

Implemented by the reusable workflow (`_package-ci.yml`), not an arbitrary
command surface:

- **`ai-provider-stubs`** — inert AI-provider CLI stubs for tests that require
  provider discovery (claudine-cli).
- **`darkmatter-md-fixture`** — builds darkmatter's `md` binary into the
  workspace target dir, preserving Claudine's clean-checkout fixture that a
  direct `_test claudine-cli` would otherwise lose.
- **`messenger-desktop-stubs`** — builds and verifies Messenger's six desktop
  helper fixtures once before each native L1 suite, then exports their directory
  through `MESSENGER_STUB_BIN_DIR`. The WSL2 archive job builds a Linux sidecar;
  the WSL job copies it onto ext4 with executable permissions and unprivileged
  ownership. The guest verifies that Cargo and rustc are absent before running
  the archive, proving helper execution depends only on the delivered sidecar.
- **`node-22` / `pnpm-10`** — the JavaScript toolchain a companion suite runs
  under (homelab-frontend, owned by homelab-server).
- **`l2-parallel-self-spawn`** — run the L2 tier in `_test_l2`'s parallel
  self-spawn mode for suites dominated by self-isolating tests (claudine-cli).
  The default worker count is `max(1, logical_cores - 2)` locally. CI uses all
  logical cores when there are four or fewer, otherwise `logical_cores - 2`.
  This preserves capacity on developer and larger shared hosts without
  crippling small CI runners. It sets test concurrency, not CPU affinity or a
  guaranteed reservation. An explicit `BISCUIT_L2_THREADS` takes precedence;
  shared-resource L2 suites retain one worker.
- **`neovim`** — provisions Neovim for packages whose L2 contract exercises
  the editor backend.
- **`zed-extension`** — provisions the digest-verified official Zed extension
  packager and the `wasm32-wasip2` target for a package-owned companion check.
  It is a lint-producer verification tool, not an L2 backend, and never uses
  terminal backend-proof evidence.

Messenger and the three Rendezvous packages use the ordinary package grid as
their coverage authority. Their native and `wsl2-ubuntu` L1 evidence is keyed
by `{package, environment, tier}` and consumed from JUnit plus producer-status
artifacts by their area's rollup; no specialized workflow or job name stands in
for a package result.

### Companion suites

`companion-suites` names non-Cargo test suites this package owns, from a
closed vocabulary. `homelab-frontend` invokes the existing non-focusing
frontend recipe (`homelab/justfile::test-frontend`) and attributes its
producer status to `homelab-server`/L1. A companion suite must emit
machine-readable evidence or a producer failure: a green Rust JUnit report
must never hide a failed OR SKIPPED companion suite
(the producer-status `failure` downgrades the cell in the rollup, and a
companion outcome other than `success` — or none at all — downgrades it the
same way).

### Exclusions must be owned and time-bounded

`gates = false` requires `reason`, `owner`, and `exclusion-class`, plus
`expiry` unless the class is `capability`. A **past** `expiry` fails the scope
calculation loudly.

- `capability` — excluded because the environment genuinely cannot host it.
  Permanent, so it must **not** carry an `expiry`.
- `promotion-pending` — a real package with real tests, blocked on identified
  work.
- `time-bounded` — nothing to gate yet (zero or near-zero tests). No current
  package uses this class: a package with no tests gates and records
  `NOTHING TO RUN` instead (AC15), which makes its first future test run
  automatic.

A `gates = false` package still appears in the grid as `NOT SCHEDULED` with its
governance metadata — never a pass, never a silent absence, never conflated
with `NOTHING TO RUN` (R10).

## Native libraries

`native` has exactly one installer: the root `justfile`'s
`_ensure-native-libs`. CI runs `just _ensure-native-libs <packages...>` before
every build, test, and lint command so a `-sys` crate never fails to compile
for a missing system library, and `just init` runs the no-argument form (every
workspace package's declarations) to cover a developer host. The dependency
closure union is computed by the scope job and passed as an explicit list; the
WSL guest has no Cargo but receives that same list. Non-Debian Linux hosts need
the apt name mapped to `dnf` / `pacman` / `apk` in that recipe's table.

## The results baseline — `ci-baseline.toml`

`ci-baseline.toml` records known-red legs and the approved skip budget, keyed
by `{package, environment, tier}`. `scripts/ci-rollup.rs` enforces it:

- a failure **not** listed blocks
- a listed entry that is scheduled and **passes** blocks, forcing cleanup
- an entry outside the run's affected scope is **ignored** — never a pass
- a scheduled entry that is cancelled, missing, or emits no result stays
  blocking; it cannot be accepted as a known test failure
- an entry past its `expiry` blocks

Every entry needs `owner`, `reason`, and `source_run`. `expiry` is optional
but strongly encouraged. The file is currently **empty on purpose** — the
area-keyed predecessor recorded no skips, and inventing entries from an
unmeasured guess would defeat the mechanism.

An entry is applied by the area that owns its `package` and by no other:
another area's rollup can neither block on it nor be excused by it. A cell
satisfied by verified local evidence is a normal result cell with
`origin: local`, so a complete local *failure* is matched against this file
exactly as a hosted one is. Policy gaps are **not** baselined here — they are
governed once in `environments.json`, and a baselined entry is only accepted
against a `FAIL` anyway.

## The area fan-out

The tested half of a run is one top-level entry per selected package **area**.
`ci.yml`'s `area-ci` job fans out over `scheduled_areas` and calls
`_area-ci.yml`, which fans out over that area's package matrix and calls
`_package-ci.yml`, which delegates the WSL2 cell to `_wsl-ci.yml`. That is four
levels including the caller — GitHub's maximum, with no margin for another.

A called workflow's jobs render as `<caller job name> / <called job name>`, so
a compile cell reads `area-ci (claudine) / claudine-cli / check
(windows-latest)`: area first, package under it, environment on the leaf. No
display name is parsed anywhere; identity still comes from each artifact's
`manifest.jsonl` and `status.json`.

**Area is a grouping; package is the identity.** Nothing is re-keyed by area.
The one place an area name appears in a stored name is the per-area result
slice `ci-results-<slug>`, where the slug spells `/` as `--` because GitHub
rejects `/` in an artifact name.

**Both matrices come from the planner.** `affected_scope.py` emits
`scheduled_areas`, `area_matrix` (one ready-made `{"include": [...]}` per
area), and `area_slugs`. Grouping in the planner rather than in workflow `jq`
is what gives the shape test coverage.

**Skippable jobs carry no `name:`.** GitHub never evaluates the matrix context
for a job it skips, so a declared `name:` containing `${{ matrix.… }}` reaches
the Checks tab as raw expression text — 63 such labels in run 34638047631. A
job that can be skipped as a whole therefore omits `name:` and lets GitHub fall
back to the job id, which is static when the job is skipped and gains the
matrix values when it runs. `lint` has no matrix, so it keeps a static
`lint (ubuntu-latest)` — its environment has to be visible.

## Each area owns its outcome

`_area-ci.yml`'s `rollup` job runs `if: always()` behind that area's producers
and runs `ci-rollup rollup --area` then `ci-rollup verdict --area`. It applies
that area's baseline, its governed policy gaps, and the missing-cell rule, and
nobody else's: another area's red cell cannot block it and another area's
baseline entry cannot excuse it. It also narrows runner-loss attribution to its
own packages (`runner_loss.py attribute --package`), because a job name carries
no area.

Its step summary is where a reused cell becomes visible: the grid's "Reused
results" table names the receipt's evidence ref, counts, duration, and host. No
setup, build, archive, or test step runs for such a cell — the planner already
removed its execution from the environment lists the area hands each package.

## `ci-gate` — the single required check

`ci.yml`'s `ci-gate` job is the **only** check branch protection should
require. It applies no policy: it `needs` every blocking top-level job
(`validation`, `scope`, `preflight`, `area-ci`, `biscuit-tui-captured-stdout`,
`ci-tooling`), runs `if: always()`, and its one step folds `needs.*.result`,
passing only when every result is `success` or `skipped`. `skipped` is
accepted by design — an unselected area's job is skipped through its `if:`,
and on a reused validation every downstream job is — while `failure` and
`cancelled` block. Because `needs` names static job ids, a cell the plan
scheduled and no job produced (`MISSING`) is invisible to the fold; each
area's own rollup catches it. `continue-on-error` turns a failed job's result
into `success` for the fold, so it is reserved for the advisory summary and no
blocking job may carry it. All of this was measured in a scratch repository:
`fixes/2026-09-11-cicd-cleanup/fixtures/scratch-2026-09-12.md`.

A producer — `check`, `lint`, `test`, `test-l2`, `test-browser`, `wsl2` —
does **not** fail its job when its gate command fails. Every test, lint, and
compile command runs as a step-level `continue-on-error` with an `id:`, and
the job's `Record producer status` step folds the step outcomes into the
cell's `result` (`failure` on an otherwise healthy job; `job.status` unchanged
otherwise) and annotates the green job with `::error`. The job, `area-ci`, and
the fold therefore stay green for a test failure, and the area's rollup is the
only job that can turn it into a block — after applying the area's baseline
(review 6, finding 1: a red producer job reached the fold before the baseline
was consulted, so a baselined hosted failure blocked while the same failure
reused from local evidence did not). Setup, provisioning, the JUnit and status
uploads, and cancellation are not normalized: they fail the producer, the
area, and the gate directly, and no baseline may excuse them. Judgement — the
baseline, the governed gaps, the missing-cell rule, `--scope`, `--policy`, and
`--plan` — lives in each area's rollup and nowhere else, and no producer may be
a required check.

**Required-context transition.** Ruleset `protect-your-bacon` (id 19747338)
still names `ci-verdict`, which was the required check until 2026-09-12.
Until that context is switched to `ci-gate`, every pull request shows
`ci-verdict — Expected` and cannot merge. The switch is Ken's separate
approval (specification Validation and Rollout step 6), taken after this
change's own run is green; the admin `pull_request` bypass actor on that
ruleset is untouched by it. `just ci-diff` reads a run's `ci-results-<slug>`
slices (there is no whole-run `ci-results` artifact any more) and
`.claudine/scripts/ci-watchdog.ts` waits for the `ci-gate` job.

### The result document

`ci-results.json` is `schema_version: 3`, versioned independently of the
baseline's 2. Identity is still `{package, environment, tier}`; each cell also
carries its derived `area`, its `origin` (`ci`, `local`, `prior-local`, or
`none`), the `evidence` behind a reused result, its measured `duration_s`, and
the `target_kinds` and `compile_coverage_from` the plan assigned it. The
document carries `accepted_evidence`, one entry per reused cell — the same set
that was accepted for *scheduling*, so the scheduler and the report cannot
disagree. A document from an earlier generation is refused with a migration
error rather than partly read.

`rollup` and `verdict` both take `--area`, which narrows the document, its
scope, its scheduled set, and its accepted evidence together: an area then
applies its own baseline, gaps, and missing-cell rule and nobody else's.
`ci-rollup summarize --results <slice>…` folds those slices into one view and
applies no policy at all.

### Artifact contract

Two artifact families, both walked by `ci-rollup rollup --artifacts`:

```
junit-<package>-<tier>-<environment>/
    manifest.jsonl            one JSON record per nextest invocation
    <tier>/<package>.xml      that invocation's verbatim JUnit document

status-<package>-<job>[-<environment>]/
    status.json               {"package","job","environment","result"[,"detail"][,"companion"]}
```

Every test job uploads the whole `target/nextest/ci-reports` **staging
directory**, not `target/nextest/ci/test-results.xml` — that single path is
overwritten by each nextest invocation. **The manifest is the identity
source.** Artifact-name parsing was retired with the area model: a staged XML
with no covering manifest record has no trustworthy identity and is dropped.

`result` is the cell's conclusion: `failure` when a gate command failed on an
otherwise healthy job (the gate steps are `continue-on-error`, so `job.status`
alone would read `success`), else GitHub's own `job.status`. The status step
and its upload both carry `if: ${{ always() }}` so a job that failed in setup
or was cancelled still reports itself; a job that reports nothing at all is
`MISSING`, never a pass. A package that
declares a companion suite also records the companion step's `companion`
outcome on every run — not only on failure — because a *skipped* companion
leaves no other evidence, and the rollup downgrades a green cell whose
declared companion produced no success evidence (R12).

### `job` is read as a tier

`ci-rollup` parses `status.json`'s `job` field with the same vocabulary as
`tier`. That is why the test jobs publish `L1` / `L2` / `browser` rather than
their GitHub job names. Publishing `test` would manufacture a phantom
`<package>/<environment>/test` cell beside the real L1 one and count the same
failure twice.

## The cache key — per package

`Swatinem/rust-cache` is keyed per package and per job kind:
`package-ci-<package>-check-<os>`, `package-ci-<package>-lint-ubuntu-latest`,
and `package-ci-<package>-test-<environment>`. The L2, browser, and WSL
archive jobs deliberately REUSE the `test` key for their environment: they
compile the same crates as the L1 leg, so one warm cache serves every tier
instead of three cold ones.

The per-package unit made the old per-directory key wrong, and the choice is
the single biggest influence on whether this work reduces runtime at all:
compilation is ~85% of a test job. **The package-scoped key has not been
measured against a real run yet** — doing so needs an authorized full trigger.
The known pressure is the cache quota: ~5 keys × 63 packages against GitHub's
10 GB repository limit means one full run saves more caches than the quota holds
and evicts its own predecessors, so only intra-run reuse is reliable today (the
L2, browser, and WSL-archive jobs restore the key their own run saved). Do not
diagnose a cold build as a cache-key bug until that measurement exists.

## Compile-check

A compile-check job exists for the target kinds no test gate produces and for
the seam with unchanged direct reverse dependents. The planner reads each
package's declared Cargo targets from `cargo metadata` and records them on its
plan record; the L1 build is credited with `lib`, `bin`, and `test`, so a
check cell is scheduled on every native environment where `example` or `bench`
targets exist. Every cell states which gate its compile coverage came from, and
an archive-only environment names the runner that built its archive rather than
claiming to have compiled anything.

A changed package's unchanged direct reverse dependents (workspace members
that gate something and are not themselves selected) are compiled inside that
package's own check cell on `ubuntu-latest` only (Open Question 1, Option B).
A package with no example or bench kinds therefore still owns one Linux check
cell when it has such dependents; no other environment's check cell is added
for them. The plan record's `dependent_seam` carries the sorted names and one
`cargo check` argument string — one `-p` per dependent plus the explicit
`--lib`/`--bins`/`--tests` selectors their declared kinds need (the kinds that
consume the public API; never their examples or benches, never
`--all-targets`, no feature flags). `_package-ci.yml` runs it as a second
normalized step, `Compile unchanged dependents`, and the cell's status records
the names plus which half failed; the rollup renders "also compiled N
dependent(s): …" on the cell. The local `just ci-local` runs do not execute
check cells, this half included.

The check job runs `cargo check -p <package>` with an explicit selector per
uncovered kind — `--examples`, `--benches`, or both — plus the declared feature
flags, never `--all-targets`; the planner renders that string as `check_args`.
It runs on every native environment (Linux, Windows, macOS): the WSL2 guest
compiles nothing, and its L1 cell names the `ubuntu-latest` archive build as
its compile coverage. There is no per-package canonical check recipe. The job
deliberately does **not** deny warnings; `lint` does, through clippy, where
`just lint` enforces the same bar locally.

## Adding or changing a package's CI

1. Add/adjust the `[package.metadata.ci]` block in the package's own manifest.
2. Tier/backend/native/feature changes require evidence (measured durations,
   real backend/native requirements, the actual tier-test ownership) — not
   guesses.
3. `python3 scripts/ci/test_affected_scope.py` and
   `cargo nextest run -p test-toolkit --test ci_workflow_contracts` must pass.

## Contract schemas

The resolved plan and the validation receipt are defined and validated in
`scripts/ci/schema.py` and documented in [`schemas/README.md`](schemas/README.md).
`schemas/contract.json` is the field contract dumped from that module for Rust
tooling; regenerate it with `python3 scripts/ci/schema.py`.

## CI's own tooling

The merge-gate binary (`scripts/ci-rollup*.rs`), the scope calculator
(`scripts/ci/`), and the policy store (`.github/ci/`) are not Cargo packages,
so a change to them selects nothing. `affected_scope.py` maps those paths to a
`ci_tooling` flag and `ci.yml` runs their own suites (the scope tests and the
rollup's nextest suite) on a dedicated `ci-tooling` leg, classified in the
advisory summary like the specialized workflows. The same leg runs the R11
workflow-contract suite (`cargo nextest run -p test-toolkit --test
ci_workflow_contracts`), and a change to any `.github/workflows/` file or to
that suite's source also sets the flag, because `test-toolkit` is
`gates = false` (promotion-pending, expiry 2026-10-31) and no area job
schedules it. The durable fix remains its promotion to a gating package.
