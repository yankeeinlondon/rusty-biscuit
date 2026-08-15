---
status: draft
created: 2026-08-15
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-15
supersedes: ../2026-07-27-local-runners/spec.md
related:
    - ../2026-08-15-remote-kache/spec.md
    - ../../fixes/2026-08-06-cicd/spec.md
---

# CI/CD on Self-Hosted Remote Runners

## Summary

CI wall-clock time remains dominated by waiting rather than work. The
2026-07-27 local-runners specification diagnosed this precisely — a 141-minute
run spending 2,508 queued minutes against 620 executing minutes, a 4.0×
queue:run ratio against GitHub's account-wide concurrency ceiling — and
designed a complete solution around an iMac Pro. **That specification was
never implemented**, and the CI it targeted no longer exists: the area-based
fan-out (`_area-ci.yml`, 21 areas) was replaced by the per-package fan-out
(`_package-ci.yml`, `ci.yml` calls one reusable workflow per gating impacted
package; `fixes/2026-08-06-cicd/spec.md`). Its queue measurements are therefore
stale in both numerator and denominator.

This specification supersedes it, carrying forward every design decision that
survives the CI restructuring and re-basing them on the hardware that now
exists: the Venice homelab's Monster Proxmox host and its Linux/Windows VMs,
plus `bolt`. The economics that motivated the original remain and have
improved — GitHub's free hosted minutes for this public repository are
unlimited but slow to obtain; homelab cores are fast, capacious, and idle.

Two requirements are unchanged and non-negotiable:

1. **Only work published by a trusted identity reaches homelab hardware.**
   The trusted publisher is Ken Snyder's GitHub account; commit-author email is
   supporting policy, not identity proof. Everything else — fork PRs, outside
   contributors, and manual dispatches without a verifiable commit range —
   runs on GitHub-hosted runners.
2. **Runner unavailability must recover onto GitHub-hosted compute.** A
   pre-dispatch probe handles known unavailability. The race after the probe
   needs a separately specified recovery path because GitHub does not retarget
   an already queued self-hosted job.

> **Reader's note — corrections made during review.** The draft treated its
> in-workflow trust expression as a security boundary, promised native
> post-dispatch failover that GitHub Actions does not provide, left the current
> hosted bootstrap preflight on the critical path, and proposed Unix `flock`
> hooks as a cross-platform coordination mechanism. It also moved JUnit staging
> into a persistent target without addressing stale evidence. This review
> retains the routing goal but makes the public-repository boundary and
> post-probe availability recovery explicit blockers, moves transient evidence
> outside the warm target, and uses runner labels for cross-platform exclusive
> capacity. See Open Questions 1, 2, and 8 for the decisions that must be
> ratified before implementation.

## Goals

1. Reduce CI wall-clock time by moving trusted builds onto homelab hardware
   with persistent, warm `target/` directories.
2. Restrict self-hosted execution to runs published by the trusted GitHub
   identity and enforce that restriction outside PR-modifiable workflow code.
3. Route to hosted runners before dispatch when a runner is offline or busy,
   the trust gate fails, or a kill switch is thrown; recover automatically from
   the remaining probe-to-dispatch race within a ratified bound.
4. Contain a compromised runner so it cannot move laterally into the homelab.
5. Leave result identity untouched: every artifact, JUnit manifest record, and
   rollup cell stays keyed by `{package, environment, tier}`. Routing changes
   where a job *runs*, never what it *is*.

## Non-Goals

- Reducing GitHub Actions spend. Public repository, free minutes; there is
   none to reduce.
- Migrating release, publish, or other secret-bearing workflows to homelab
   hardware.
- Fixing pre-existing product failures. Those belong to their areas.
- Re-litigating the CI's package-based structure. This feature routes the
  existing jobs; it does not reshape them.
- Treating persistent self-hosted machines as a release provenance boundary.
  Release and publish jobs remain on clean GitHub-hosted runners.

## Decisions carried forward from the 2026-07-27 spec

The original specification's analysis survives intact where it concerns
properties of GitHub Actions rather than properties of the old CI. These are
incorporated by reference rather than restated in full:

- **Runners initiate; Actions needs no inbound listener.** The runner holds an
  outbound HTTPS long-poll to GitHub on 443. The narrow SSH administration
  path specified below is separate from Actions transport.
- **No native failover.** GitHub's
  [self-hosted runner routing contract](https://docs.github.com/en/actions/reference/runners/self-hosted-runners#routing-precedence-for-self-hosted-runners)
  leaves a `runs-on` label with no online runner queued for
  up to 24 hours. Routing known state must be resolved *before* dispatch, by a
  hosted routing job that emits an execution-class→platform→label map. A
  runner can still disappear after that decision; the map alone cannot satisfy
  the availability requirement.
- **The serial floor.** Routing moves queues; it does not delete them. Total
  executing minutes divided by slot count is a hard floor no routing goes
  below. Slot sizing must be checked against the re-measured baseline before
  run-wide routing is enabled.
- **The probe's `busy` check is liveness, not capacity.** A run's own sibling
  jobs queue behind each other by construction. Queueing behind your own work
  at a known slot count is an accepted condition, bounded by the serial floor.
- **Per-slot environment**: `CARGO_BUILD_JOBS`, `NEXTEST_TEST_THREADS` caps;
  `CI_SELF_HOSTED` for recipe-level gating; per-slot `CARGO_TARGET_DIR`
  **outside** the `_work` tree so `actions/checkout`'s default
  `git clean -ffdx` cannot delete the warm target. Shared target directories
  across slots remain forbidden (cargo's exclusive target-dir lock would
  serialize the slots).
- **Cache policy**: `Swatinem/rust-cache` and any kache wiring are switched
  off on self-hosted runners, gated on `runner.environment == 'github-hosted'`
  at the `uses:` sites and through the composite action's `enabled:` input
  respectively — the latter because a step-level `if:` on a local composite
  `uses:` fails to load.
- **Test parallelism under co-tenancy**: thread caps are per-process;
  `.config/nextest.toml`'s timeout margins are calibrated against 4-core
  hosted contention and `[profile.ci]` sets `retries = 0` deliberately, so
  oversubscription is a *correctness* hazard, not a tuning one. Cross-slot
  host-global resources (audio device, tmux server namespace, headless
  Chrome) need host-level coordination once more than one slot runs test
  tiers. Coordination must not depend on a host-only utility such as `flock`.

Where the rest of this document specifies something differently, the
difference is deliberate and argued.

## Hardware

### Known hosts

| Host | Access | Platform | Spec | Initial slots |
|---|---|---|---|---|
| Monster VM `build-linux` | `ssh build-linux` / 192.168.100.143 | Linux x64 | VM on Monster (see below) | 3 (provisional) |
| Monster VM Windows | `ssh build-win-native` / 192.168.100.64 | Windows x64 (+ WSL2 guest via `ssh build-win`) | VM on Monster (see below) | 2 (provisional) |

Monster itself (`192.168.100.14`) is a Proxmox host: 16-core AMD EPYC 9175F,
384 GB RAM. It hosts the two build VMs above. This is a dramatic upgrade over
both the hosted runners (4 cores / 16 GB / 14 GB disk) and the iMac Pro the
original spec was built around (10-core / 128 GB), and it is the first time
disk is not the binding constraint — a warm per-slot `target/` for this
workspace (~71 G clean full build; observed up to 135–222 G on runaway
worktrees per `docs/kache-strategy.md`) fits comfortably with headroom.

**Per-VM allocation (cores / RAM / disk) is a Phase 0 output, not a guess.**
Monster's 384 GB must also serve its other tenants and Proxmox overhead. The
slot counts above are planning inputs for the serial-floor arithmetic; they are
not ratified capacity.

### Pending specification

| Host | Access | Platform | Spec |
|---|---|---|---|
| `bolt` | `ssh bolt` / 192.168.10.157 | presumed macOS (the only platform not otherwise covered) | unknown |

Two blockers, stated plainly:

1. `bolt`'s hardware specification is unknown. Until ratified it cannot be
   sized, and its runner is not registered.
2. **`bolt` is not on the 192.168.100.0/24 Venice homelab subnet** — it is at
   192.168.10.157. Whether it is reachable from, or routable to, the homelab
   segment (and what containment applies there) is an open question. The
   network-containment design below assumes the 192.168.100.0/24 segment;
   `bolt` needs its own answer.

macOS routing is therefore **Phase 3**, not Phase 1 — the reverse of the
original spec's ordering, which was driven by macOS having the worst queue
ratio (6.3×) and the only ratified hardware. Today Linux has the ratified
hardware candidate, and macOS is the platform whose homelab story is least
defined.

### Hosted runner hardware, for comparison

| Runner | Cores | RAM | Disk |
|---|---|---|---|
| `ubuntu-latest` | 4 | 16 GB | 14 GB |
| `windows-latest` | 4 | 16 GB | 14 GB |
| `macos-latest` (ARM) | 3 (M1) | 7 GB | 14 GB |

The 14 GB disk remains the hosted pathology: no warm `target/` survives, which
is why the hosted path leans on `Swatinem/rust-cache` and why compiling test
binaries is ~85% of every hosted test job. Self-hosted slots with persistent
targets attack exactly that 85%.

## Baseline Re-measurement (Phase 0)

Every number in the 2026-07-27 spec came from run `30274087816` under the
area-based CI. The current per-package CI fans out differently (a
package-per-job matrix whose breadth depends on the change's dependency
closure; `fixes/2026-08-06-cicd/measurement.md` records a narrow run at 71
runner-minutes against the old system's ~725). Before any routing is enabled:

1. Capture at least five representative full-scope runs on the current CI at
   the job level via the Actions API. Record both end-to-end latency (workflow
   `created_at` → verdict completion) and execution span (first eligible job
   start → verdict completion); do not call the latter wall-clock time.
2. Recompute queue:run ratios and per-platform serial floors against the
   proposed slot counts. Queue time must be measured from job eligibility, not
   from workflow creation, so dependency wait is not mislabeled as runner
   scarcity.
3. Record the numbers in `measurement.md` here, the same way the original
   spec recorded its baseline, including run IDs, timestamps, job-to-platform
   classification, exclusions, and formulas, so every later claim is
   reproducible.

The design below proceeds on the *structure* of the problem (pre-dispatch
routing, trust, containment), which the re-measurement cannot invalidate. The
*go/no-go on run-wide routing per platform* is decided by the measured floors:
if a platform's routed serial floor exceeds its current wall-clock
contribution, that platform's routing stays scoped to a subset of job classes
until slots grow — exactly the arithmetic the original spec performed for
Linux at a 59:3 fan-out.

## Trust Gate

The requirement, as stated by the owner: only jobs published by Ken Snyder's
GitHub account reach the external runners, reducing the risk of outside parties
exploiting the compute. An author email is mutable commit metadata and must not
be described as authentication.

### Security boundary prerequisite

The repository currently lives under the personal account
`yankeeinlondon/rusty-biscuit`. A repository-level self-hosted runner in a
public repository is addressable by workflow YAML from that repository. A fork
can modify a workflow to use `runs-on: rb-linux` directly, bypassing any trust
expression in `ci.yml`. GitHub's
[secure-use reference](https://docs.github.com/en/actions/reference/security/secure-use#hardening-for-self-hosted-runners)
explicitly warns that persistent self-hosted runners should almost never be
used for public repositories for this reason.

Therefore, **no runner may be registered to this public personal repository
until Open Question 1 is resolved with an enforcement boundary outside
PR-controlled YAML**. The recommended boundary is an
[organization runner group](https://docs.github.com/en/actions/hosting-your-own-runners/managing-self-hosted-runners/managing-access-to-self-hosted-runners-using-groups)
restricted to this repository and to a new trusted execution workflow pinned
to `refs/heads/main`. Only jobs defined directly in that pinned workflow
receive runner-group access. The caller may remain PR-controlled because
bypass jobs cannot reach the group.

The pinned workflow must perform its own scope, trust, and routing checks once
and directly define the routed package matrices. It must not delegate routed
jobs to a generally callable workflow that accepts an authoritative map: a fork
could call that workflow directly. It must not trust a caller-supplied
`runner-map`, trust verdict, checkout SHA,
package command fragment, or author list without deriving or validating that
value against the event and repository. This intentionally changes the current
one-call-per-package reusable-workflow orchestration while preserving package
as the unit of selection and result identity. Candidate changes to the trusted
workflow are validated on hosted runners and do not become the self-hosted
execution authority until merged to `main`.

### Configuration

Repository **variables** (not secrets — they are not sensitive), set by a new
`just ci-sync-trust` recipe reading the gitignored `.env`:

```
CI_TRUSTED_ACTOR_IDS=<numeric-github-user-id>  # immutable github.actor_id values
CI_TRUSTED_AUTHORS=ken@ken.net         # policy metadata, not authentication
CI_LOCAL_RUNNERS_ENABLED=true          # kill switch (Tier 1 rollback)
CI_RUNNER_GROUP_ID=<numeric-group-id>   # organization runner-group authority
```

The `.env`-to-variable flow is specified in the original spec and survives
unchanged: a file in the repository is attacker-controlled in a fork PR's head,
repository variables are admin-modifiable only, and `.gitignore` already
excludes `.env`. `.env.example` must be created and committed documenting the
keys (Phase 0 task — it does not exist yet).
The expected repository and trusted workflow ref are constants in the pinned
routing script, not caller inputs or variables; changing them requires review
of that script on `main`.

### Gate conditions

All must hold for self-hosted dispatch, and the trusted execution workflow must
evaluate them rather than accepting a caller's verdict:

1. **Not a fork.** `github.event.pull_request.head.repo.full_name ==
   github.repository`, or the event is not a pull request.
2. **Trusted publisher.** `github.actor_id` is in the decimal-ID allowlist
   `vars.CI_TRUSTED_ACTOR_IDS`. Log `github.actor` for readability, but do not
   authorize by a renameable login.
3. **Trusted commit authors.** Every commit in the range is authored by an
   address in `vars.CI_TRUSTED_AUTHORS`, evaluated by `runner-routing`
   with `git log --format=%ae <base>..<head>` on a `fetch-depth: 0` checkout.
   Parse the variable as a comma-separated list with surrounding ASCII
   whitespace removed and compare normalized lowercase addresses. This is a
   content-policy check only; the trusted actor ID is the identity proof.
4. **Verifiable range.** Ordinary `workflow_dispatch` has no range and routes
   hosted. A measurement-only dispatch may provide `measurement_base_sha` and
   `measurement_head_sha`; the trusted workflow fetches both full 40-character
   object IDs, verifies that head equals the dispatched ref's commit, verifies
   that base is an ancestor of head, and applies the same non-empty author
   check. Any failure routes hosted. `force_hosted: true` always routes hosted
   and is the paired-control mechanism; it can never widen access.

Adaptation for the current `ci.yml`, which differs from the old one in one
relevant way: `pull_request` now carries **no** `branches:` filter
(`ci.yml:8-18`, so stacked PRs are validated). That change widens the event
population the gate sees but not the gate's logic — base/head SHAs are
supplied by the event either way (`ci.yml:60-63`). For `pull_request`, use
those event base and head SHAs, not the synthetic merge commit. For `push`, use
`github.event.before..github.sha`; an all-zero or missing `before`, a
force-push whose base cannot be fetched, or an empty range routes hosted.

The same gate, the same variables, and the same recipe are shared with the
remote-kache spec's bucket write-access control. One trust definition for both
features; duplicating it would be a drift bug waiting to happen. The current
remote-kache draft still names `CI_TRUSTED_ACTORS`; its implementation must
adopt `CI_TRUSTED_ACTOR_IDS` and the server-enforced boundary from this spec, or
declare a dependency on the completed trust work. This is a coordinated
contract change, not two compatible variable names.

## Routing Design

### `runner-routing` job

Do not overload the existing `preflight` name: that job is the cross-platform
bootstrap gate. A new hosted `runner-routing` job resolves which runner label
to use and emits a JSON map consumed by routed jobs' `runs-on`.

Under the recommended organization runner-group design, the probe queries
`GET /orgs/{org}/actions/runner-groups/{runner_group_id}` and
`GET /orgs/{org}/actions/runner-groups/{runner_group_id}/repositories`, then
`GET /orgs/{org}/actions/runner-groups/{runner_group_id}/runners?per_page=100`
with an explicit GitHub API version. The credential has only organization
`Self-hosted runners: read` and is stored as `RUNNER_PROBE_TOKEN` unless Open
Question 6 selects a GitHub App. The group ID is a repository variable. The
probe rejects a group whose public-repository access, selected repository, or
pinned-workflow restriction differs from the Phase 0 record. A route selects a
self-hosted label only when a matching runner has the expected
OS and architecture, reports `status: online`, and is not `busy`. The request
has a bounded connection and overall timeout, validates the response shape,
and treats pagination, malformed JSON, or an unexpected runner identity as an
error rather than as an empty list.

Global fail-safe triggers emit the all-hosted map:

| # | Trigger |
|---|---|
| 1 | Trust gate does not pass, including a dispatch without a verified range |
| 2 | `RUNNER_PROBE_TOKEN` absent (fork PRs receive no secrets) |
| 3 | API error, timeout, or rate limit |
| 4 | `vars.CI_LOCAL_RUNNERS_ENABLED` is not exactly `true` |
| 5 | Measurement input `force_hosted` is `true` |

No matching runner, an offline runner, or all matching runners being `busy` is
a **per-route** fallback: that execution class and platform stay hosted while
other healthy routes may remain self-hosted. Every fallback preserves result
identity and is reported in the job summary. It does not make the run literally
byte-identical because the routing summary differs.

The probe is a point-in-time observation, not a lease. GitHub assigns a job
only after evaluating `runs-on`; if the selected runner disappears in between,
the job queues and GitHub does not retarget it. Open Question 2 must ratify the
bounded recovery behavior before the availability goal can be accepted.

### Labels

| Canonical environment | Standard label | Exclusive label | Phase |
|---|---|---|---|
| `ubuntu-latest` | `rb-linux` | `rb-linux-exclusive` | 1 / 2 |
| `windows-latest` | `rb-windows` | `rb-windows-exclusive` | 2 |
| `macos-latest` | `rb-macos` | `rb-macos-exclusive` | 3 (pending `bolt` ratification) |

Every slot on a host receives its standard label. Exactly one slot on that host
also receives the exclusive label. Browser, WSL guest-execution, and any tier
that declares a genuinely host-global resource use exclusive capacity;
compile, lint, L1, WSL archive, and tmux-only L2 jobs use standard capacity.
Each Unix slot has a distinct `TMUX_TMPDIR`/server namespace, and tmux tests
already use uniquely named sessions or a slot-local broker, so tmux alone is
not a host-global lock. This serializes only real host-global resources through
GitHub's cross-platform scheduler without `flock`, PowerShell mutexes, or a
second lock service. An exclusive slot may still accept standard work when no
exclusive job is waiting.

Resolved map example — standard Linux and Windows local, Linux exclusive and
all macOS work fallen back:

```json
{
  "standard": {
    "ubuntu-latest": "rb-linux",
    "windows-latest": "rb-windows",
    "macos-latest": "macos-latest"
  },
  "exclusive": {
    "ubuntu-latest": "ubuntu-latest",
    "windows-latest": "rb-windows-exclusive",
    "macos-latest": "macos-latest"
  }
}
```

### What routes, in the current CI

The per-package workflow's job classes, and their routing:

| Job | Location (`_package-ci.yml`) | Routed? |
|---|---|---|
| `check` | 125–194 | **Yes** |
| `test` (L1) | 200–442 | **Yes** |
| `lint` | 444–546 | **Yes** — see below |
| `l2` | 563–753 | Phase 2; standard for tmux-only packages, exclusive only for a declared host-global backend/resource |
| `browser` | 755–830 | Phase 2, through the exclusive route |
| `wsl` | 838–855 → `_wsl-ci.yml` | Phase 2, adapted (below) |
| `ci.yml` `scope` / `ci-tooling` / `ci-verdict` / `summary` | — | No — literal hosted labels |
| `ci.yml` bootstrap `preflight` | 157–207 | Open Question 8; it cannot silently remain the hosted critical-path bottleneck |

The original spec pinned `lint` to hosted because it was the gating stage for
the whole test fan-out. In the current CI, `test` deliberately depends on
nothing (`_package-ci.yml:201-208` — "Lint is a parallel gate, not a
prerequisite"), so that argument is void. `lint` still compiles the package's
full dependency graph under clippy and benefits from a warm target; it routes
from Phase 1.

`l2` and `browser` are Phase 2 because their co-tenancy contracts need explicit
proof. The repository harness already makes tmux parallel-safe through unique
sessions and slot-local brokers, so a tmux-only L2 job does not pay global
serialization. Browser recipes run `-j 1` only within one nextest process;
multiple runner slots would still launch multiple Chrome trees on one VM, so
browser begins on the exclusive route. Phase 4 may widen it only after resource
and teardown measurements show that cross-slot Chrome is safe.

The reporting jobs stay hosted for the same reason as before: `ci-verdict`
builds one small `--no-default-features` binary (~8 s, `ci.yml:353-360`) and
reads artifacts; routing them burns build slots to run `printf`.

The current bootstrap `preflight` is not a reporting job: every package call
depends on the completion of its entire hosted OS matrix. On a full-scope run,
a queued hosted macOS preflight can therefore delay Linux self-hosted fan-out.
That interaction must be resolved explicitly rather than inherited by accident
(Open Question 8).

### The environment-identity invariant

`_package-ci.yml`'s `test` matrix is sound today **only because every native
environment name is also its own runner label** (`_package-ci.yml:210-216`).
The runner-map preserves exactly that property:

```yaml
runs-on: ${{ fromJSON(needs.runner-routing.outputs.runner_map).standard[matrix.environment] }}
```

`matrix.environment` remains `ubuntu-latest` / `windows-latest` /
`macos-latest` — the value stamped into every status artifact, JUnit manifest
record (`BISCUIT_CI_ENVIRONMENT`, `_package-ci.yml:224`), and rollup cell.
The trusted execution workflow reads the `runner-map` output from its own
`runner-routing` job. It never accepts the map from its caller. Standard jobs
read `.standard[...]`; exclusive jobs read `.exclusive[...]`.
**`affected_scope.py`, `environments.json`, and the rollup remain untouched.**
The current `_package-ci.yml` job definitions may be moved or generated into
trusted matrices as required by the security boundary, but package remains the
selection and result-identity unit. A self-hosted leg is indistinguishable from
a hosted one in result identity, while the routing summary retains the physical
runner name for operations.

### The WSL leg

`_wsl-ci.yml` runs on `windows-latest`, provisions a fresh WSL2 distro per run
via `Vampire/setup-wsl`, builds the nextest archive on an `ubuntu-latest` job,
and executes it in the guest. Phase 2 makes these adaptations:

1. The archive-build job uses the standard Linux route only after it emits a
   small layout artifact containing its actual `GITHUB_WORKSPACE`. The current
   workflow assumes the hosted path
   `/home/runner/work/<repo>/<repo>` because `env!("CARGO_MANIFEST_DIR")` is
   embedded in test binaries. A self-hosted slot has a different path, so
   routing the archive without transporting that path would deterministically
   break fixture lookup.
2. The guest downloads and validates the layout artifact, creates its ext4
   checkout at that exact absolute Linux path, and derives
   `BISCUIT_JUNIT_WORKSPACE_ROOT` from it. No hardcoded hosted workspace remains
   in the routed path. A malformed, relative, non-absolute, or non-allowlisted
   layout path fails before clone rather than writing elsewhere in the guest.
3. The guest-execution job uses the exclusive Windows route. It continues to
   provision a fresh per-run distro; the resident `build-win` distro is an
   administration environment, not a CI fixture. This preserves the leg's
   isolation and avoids making test results depend on long-lived guest drift.

The fresh-distro choice is ratified, not an open question. If provisioning is
too slow, optimize or snapshot the disposable guest in a later specification;
do not silently substitute the resident distro.

## Cache Policy on Self-Hosted Runners

Unchanged from the original spec in substance; the sites have moved.

**`Swatinem/rust-cache@v2`** — add `if: runner.environment == 'github-hosted'`
at every site in the current workflows:

- `_package-ci.yml`: `check` (line 164), `test` (255), `lint` (479), `l2`
  (689), `browser` (794)
- `_wsl-ci.yml`: the archive-build site (and the Windows-side cache, if any,
  in that file)
- `ci.yml`: none remain on routed jobs (`preflight` builds nothing)

Two failure modes motivate the gating, both from the original analysis:
restore keys carry `matrix.os`/environment names that no longer identify the
executing host, and the action's post step prunes and re-uploads the target —
destroying precisely the warm workspace-crate artifacts self-hosting exists to
keep, 81 post-step executions for 28 minutes in the measured run.

**kache** — off on self-hosted slots. The remote-kache spec's S3 bucket is
hosted-leg infrastructure; a self-hosted slot may warm from it by hand at
bootstrap (new slot, post-eviction) but CI wiring does not touch it.

**Warm target**: per-slot `CARGO_TARGET_DIR` outside `_work`
(e.g. `/home/ci/targets/slot-N` on `build-linux`, an equivalent path on the
Windows VM), never shared across slots (cargo's exclusive lock).

JUnit staging is **transient evidence, not a build cache**. Do not move it under
the persistent target: old manifests and XML could then be uploaded by a later
package job. Each routed job sets `BISCUIT_JUNIT_STAGE_DIR` to a job-local path
under `${{ runner.temp }}` keyed by run ID, run attempt, package, environment,
and tier. A setup step requires that path to be absent before creating it; it
does not trust runner-temp cleanup. The upload uses that same path and
`if-no-files-found: error`. Cargo metadata already resolves the actual
`CARGO_TARGET_DIR` for nextest's source report; only the staging destination
needs this explicit override. The hosted path may adopt the same layout so the
workflow has one evidence contract.

The current hardcoded root-relative audit has one remaining product-test site:
`biscuit-icon/cli/tests/level2_terminal.rs` constructs
`../../target/debug/icon`. Replace it with the repository's executable
resolution helper before routing that L2 tier. The older
`claudine/cli/tests/inline_compose_hash.rs` site named by the superseded spec no
longer exists and must not be carried forward as a task.

**Disk budget**: the per-slot budget and eviction threshold are Measurement
Milestone outputs. The sweep infrastructure already exists and is
version-controlled (`scripts/sweep.sh`, the Linux/Windows scheduled sweeps,
`docs/kache-strategy.md` → Sweep script); wire the slot target dirs into the
sweep roots rather than inventing a new mechanism. A sweep never runs while
its slot's runner service is executing a job; eviction and build are mutually
exclusive per slot.

## Runner Provisioning and Lifecycle

Each advertised slot is a separate GitHub Actions runner installation and
service with a unique runner name, runner application directory, `_work`
directory, temporary directory, tool cache, and `CARGO_TARGET_DIR`. Sharing a
VM does not mean sharing mutable runner directories. Each slot runs as its own
unprivileged OS service account so one concurrent job cannot read or rewrite
another slot's directories. These accounts have no interactive login,
passwordless sudo, Docker socket, hypervisor API, or homelab credential; the
administration account is separate.

Native packages and required tools are baked into a versioned VM-image or
provisioning manifest. On `CI_SELF_HOSTED=true`, `_ensure-native-libs` is
verify-only and fails with the missing dependency list; it never installs or
elevates. Hosted runners retain today's install behavior. A Phase 0 smoke test
must exercise every declared native package and runner-tool vocabulary entry
on each image before the label is advertised.

Runner software updates are an operational contract, not ambient state. GitHub
can stop assigning work when a disabled-update runner is more than 30 days
behind or misses a required security update. Choose
one of these two supported modes per image and record it in `measurement.md`:

- automatic runner updates, with a health check alerting on version drift; or
- `--disableupdate`, with a scheduled image update within 14 days of an
  upstream release (comfortably inside GitHub's 30-day enforcement window) and
  immediate handling of security-required updates.

All third-party actions reachable from routed jobs must be pinned to reviewed
full commit SHAs before Phase 1. GitHub identifies a full SHA as the only
immutable action release reference; tags such as `@v2` and `@v4` are mutable
and are not an acceptable supply-chain boundary on a persistent runner.
Dependabot or the maintenance audit remains responsible for proposing pin
updates.

## Test Parallelism Under Co-Tenancy

Inherited analysis, re-anchored to the current files: `.config/nextest.toml`'s
`[profile.ci]` timeout margins are calibrated against 4-core hosted
contention, `retries = 0` is deliberate ("Deterministic L1 failures must run
exactly ONCE"), and thread caps are per-process. Each slot therefore exports:

| Variable | Value |
|---|---|
| `CARGO_BUILD_JOBS` | `max(1, floor(usable VM vCPUs / simultaneously active slots))` |
| `NEXTEST_TEST_THREADS` | the same initial cap, tuned downward when tests spawn subprocesses |
| `CI_SELF_HOSTED` | `true` (recipe-level gating; `_ensure-native-libs` must not sudo) |
| `CARGO_TARGET_DIR` | per-slot path outside `_work` |
| `BISCUIT_JUNIT_STAGE_DIR` | job-local path under `runner.temp`, set by the workflow |
| `TMUX_TMPDIR` | unique per Unix slot so tmux server state is not shared across runner services |

`test-threads` deliberately does **not** go into `[profile.ci]` — that profile
still runs on every hosted fallback leg, where a host-derived value could increase
oversubscription. Retries stay 0; measure first, then decide whether to mask
(original spec → Rejected alternatives, unchanged).

The draft's provisional `6 × 3` Linux setting requested 18 build threads from a
16-core physical host before accounting for the Windows VM, other tenants, or
Proxmox. That is not a comfortable starting point. Caps are derived from the
VM's ratified vCPU allocation and the number of simultaneously active slots,
then verified from observed CPU saturation, memory high-water marks, and
timeout behavior. Browser, WSL provisioning, and any explicitly declared
host-global resource are protected by the exclusive runner label; tmux-only L2
remains parallel by the repository harness contract.

## Network Containment

The original spec contained runners with Tailscale tags on a tailnet. The
current homelab is the 192.168.100.0/24 segment behind the Venice gateway,
with the runner VMs as Proxmox guests on Monster (192.168.100.14). Actions
requires no inbound listener because runners long-poll out, but the VMs do have
an intentionally narrow SSH administration surface. The primary risk is
**lateral movement from a compromised runner into the homelab**, and the
control must be rebuilt for this topology:

1. **Separate segment and/or Proxmox firewall rules** deny runner-initiated
   traffic to every RFC1918, link-local, metadata-service, and Proxmox
   management address. Permit established return traffic, outbound Internet
   TCP 443, DNS only to the designated resolver, and NTP only to the designated
   time source. Package registries, action downloads, GitHub artifact storage,
   rustup, and OS updates must work through that policy; do not describe
   "gateway 443" as sufficient without testing those endpoints. Phase 0
   records the applied rules and a deny/allow probe transcript.
2. **No credentials on the runners beyond job scope.** The runner's local
   service account holds nothing homelab-facing: no SSH keys to other hosts,
   no Proxmox tokens, no NFS/CIFS mounts. The build-linux `~/.config` CIFS
   mount documented in `docs/kache-strategy.md` is a developer-host
   arrangement and must not be replicated on the runner VM — which also means
   the systemd-vs-cron sweep scheduling quirk it causes does not apply.
3. **Admin access inbound only.** SSH is allowed from Ken's fixed
   administration source to the runner VM management address, with key-only
   authentication. Runner VMs cannot initiate new connections to the admin
   workstation or Proxmox management plane. The firewall is stateful so reply
   traffic for the authorized SSH session remains possible.
4. **No cloud metadata path.** Proxmox guests do not receive a metadata-service
   credential. If cloud-init is used for provisioning, its seed is removed or
   made non-secret before the runner label is enabled.

`bolt` (192.168.10.157, outside the segment) needs its own containment answer
before Phase 3; see Open Questions.

## Observability

Global and per-route fail-safe triggers intentionally produce green hosted
runs. Their routing decision must not be silent:

1. `runner-routing` appends one summary line per execution class and platform —
   routed vs hosted and which trigger fired — to `$GITHUB_STEP_SUMMARY`.
2. The routing job uploads a machine-readable decision artifact containing the
   trust outcome, selected execution class, runner IDs observed (not tokens),
   API failure class, and timestamp. Retain it for the same three days as the
   scope artifact.
3. The recovery mechanism selected in Open Question 2 records stalled job IDs,
   queue age, cancellation, and replacement run ID. It is part of Phase 1 if
   the availability requirement remains non-negotiable, not an optional later
   watchdog.

Additionally: `kache`-style discipline for the runners themselves — a
`just runner-health` recipe (wrapping
`gh api orgs/{org}/actions/runner-groups/{runner_group_id}/runners` for label,
version, status, and `busy`) so "is it up?" is one command, not a browser tab. The recipe is
read-only, accepts its credential through the normal `gh` environment, and
never starts an interactive login.

## Rollback

| Tier | Action | Speed |
|---|---|---|
| 1 | `gh variable set CI_LOCAL_RUNNERS_ENABLED --body false` — the next routing job selects hosted labels | Seconds |
| 2 | Stop accepting work: wait for the active job to finish, stop the runner service, then remove its registration with an operator-issued removal token | Minutes |
| 3 | Delete the runner registrations and revert the trusted routing workflow | One PR |

Tier 1 prevents newly routed work from selecting self-hosted labels. It does
not retarget a run that is already queued; Open Question 2's recovery path owns
that case. Neither path softens a check (the original spec's argument against a
`soft_os` override is carried forward), and no per-area/per-package softening
mechanism is introduced.

## Verification and Contract Updates

Before the first runner label is enabled:

1. Extend the existing `tools/test-toolkit` L1 workflow-contract suite. Cover
   trusted and untrusted actor IDs, fork/same-repository events, malformed and
   empty author lists, zero/missing push bases, verified and forged measurement
   ranges, kill switch values, missing probe credentials, malformed/paginated
   runner responses, per-route busy fallback, and exact standard/exclusive map
   shape. Tests invoke the same checked-in script the workflow calls; copying
   the logic into test-only code is not evidence.
2. Add contracts proving every routed `runs-on` reads only
   `needs.runner-routing.outputs.runner_map`, result identities still use the
   canonical environment, self-hosted cache actions are gated, transient JUnit
   uploads use `runner.temp` with `if-no-files-found: error`, and no untrusted
   caller input controls a runner label.
3. Run `actionlint`, `just check-canonical`, the affected-scope tests, and the
   package area's canonical `just test`/`just lint` gates. Repository tests run
   through nextest; do not introduce a `cargo test` exception.
4. Exercise the real runner path with one trusted route, one fork or synthetic
   untrusted route, one busy/offline per-route fallback, the global kill switch,
   and the post-probe recovery drill. Capture run and job IDs in
   `measurement.md`.
5. Run L2 only through the canonical harness. The routed tmux proof must stay
   headless, use the slot-local namespace, and produce backend-execution
   evidence. Browser verification remains headless and must not activate or
   focus a host window. No L2 or browser test may use OS input injection.
6. Prove stale-evidence isolation by running two different package/tier jobs
   successively on the same slot and showing the second JUnit artifact contains
   only its own expected manifest and reports.

Update the contracts that this feature intentionally changes in the same
implementation:

- `docs/topics/ci-cd.md`: trusted execution workflow, routing map, bootstrap
  preflight relationship, cache gating, rollback, and runner security boundary;
- `docs/testing-strategy.md`: job-local JUnit staging and slot-local tmux
  namespace;
- `docs/kache-strategy.md`: per-slot external targets, sweep mutual exclusion,
  and the fact that self-hosted CI remains kache-off;
- `.claude/skills/rust-testing/SKILL.md`: replace the current statement that
  `package-ci` is always gated by hosted `preflight` if Open Question 8 changes
  that relationship; and
- `.env.example`: variable names and comments without real IDs or credentials.

## Phasing

| Phase | Content | Exit |
|---|---|---|
| 0 | Resolve Open Questions 1, 2, and 8; baseline re-measurement; VM allocation; network containment; pinned third-party actions; versioned runner images; `.env.example` + `just ci-sync-trust`; probe credential; job-local JUnit staging | Security boundary exists outside PR YAML; baseline and rule/provisioning evidence are in `measurement.md`; no runner is registered before this exit |
| 1 | Register Linux slots; trusted `runner-routing` map; route `check`, `test`, and `lint`; per-slot environments; mandatory availability recovery selected by Open Question 2 | Linux Measurement Milestone and recovery drill |
| 2 | Windows slots; native legs; dynamic WSL archive layout + fresh guest; exclusive labels; `l2`/`browser` routing | Windows milestone; no cross-slot resource overlap or stale evidence |
| 3 | `bolt` ratification + macOS `rb-macos` | Same milestone on macOS |
| 4 | Slot-count and thread-cap tuning from measured floors | Updated capacity record; no correctness regression |

### Measurement Milestone (each phase)

Run five paired full-scope trials at the same commit: one trusted routed run and
one forced-hosted control. Do not move the branch between the pair. Record:

1. End-to-end latency (workflow creation → verdict) and execution span (first
   eligible job start → verdict), reported separately.
2. Per-platform queue:run ratio and critical-path contribution, recomputed with
   the Phase 0 formulas.
3. **Normalized result diff: zero.** Compare
   `{package, environment, tier, result, failing-test identities}` after
   removing timestamps, durations, physical runner names, and artifact IDs.
   Raw artifacts are not expected to be byte-identical. A new red or missing
   evidence on self-hosted hardware is a phase blocker, not a baseline entry.
4. Per-slot disk growth across the five runs → the eviction threshold,
   wired into the existing sweep.
5. CPU, memory, and I/O high-water marks per VM and per slot, plus observed
   runner version and image revision.

The phase is successful only when normalized results are equal, no job exceeds
its existing timeout, the routed median end-to-end latency is lower than its
paired hosted median, and the recovery drill meets the bound ratified in Open
Question 2. If performance is neutral or worse, keep that job class hosted;
warm hardware is not itself acceptance evidence.

## Open Questions

### 1. What enforces trust for a public personal repository? — Phase 0 blocker

The in-workflow gate cannot prevent a fork from replacing the workflow and
targeting a repository runner label directly.

- **Transfer the repository to an organization and use a selected-workflow
  runner group.** Pros: GitHub enforces access before scheduling; jobs remain in
  the original run, so artifacts and required checks retain their current
  shape; the trusted reusable workflow can be pinned to `refs/heads/main`.
  Cons: repository ownership, permissions, URLs, apps, package ownership, and
  local remotes need a migration audit; the selected-workflow capability must
  be confirmed for the organization's GitHub plan; changes to the trusted
  workflow are hosted-only until merged.
- **Keep the public repository where it is and execute through a private broker
  repository.** Pros: strongest separation without transferring the public
  repository; the private repository alone owns the runners and credentials.
  Cons: requires a GitHub App/controller to validate the source event, check out
  the exact public commit, publish check runs, and transport artifacts; failure
  and cancellation semantics span two repositories.
- **Register repository runners and rely on the YAML trust condition.** Pros:
  smallest implementation. Cons: bypassable by changing `runs-on`; directly
  violates the primary security requirement and GitHub's public-repository
  guidance.

**Recommendation:** transfer to an organization and restrict a dedicated
runner group to a new
`{org}/rusty-biscuit/.github/workflows/_trusted-ci.yml@refs/heads/main` that
derives scope/routing and directly defines the routed package matrices.
If transfer is unacceptable, use the
private broker. The repository-runner option is rejected.

### 2. What is the post-probe availability contract? — Phase 0 blocker

GitHub requeues an assigned job if a runner does not accept it, but it does not
replace a custom self-hosted label with a hosted label; the job may remain
queued for up to 24 hours.

- **Best-effort routing plus alerting.** Pros: minimum machinery; no duplicate
  runs. Cons: contradicts the non-negotiable automatic-recovery requirement and
  leaves merges blocked during the probe-to-dispatch race.
- **A recovery controller dispatches an exact-SHA hosted replacement.** Pros:
  provides a measurable bound; reuses `force_hosted`; preserves the original
  source revision. Cons: the original run must be canceled, replacement and
  required-check semantics need proof, and the controller needs idempotency and
  narrowly scoped dispatch/cancel authority.
- **Just-in-time autoscaled runners.** Pros: capacity is coupled to queued work
  and clean instances reduce persistence risk. Cons: startup can still fail,
  GitHub still does not retarget labels, and clean instances remove the warm
  target that supplies most of this feature's expected gain.

**Recommendation:** build the recovery controller with a 10-minute queue-age
bound, idempotency keyed by the original run ID, exact base/head validation,
and no mutation of the global kill switch. Before Phase 1, prove in a disposable
branch that canceling the stalled run and dispatching the hosted replacement
leaves the commit's required `ci-verdict` satisfiable. If GitHub's check semantics
cannot support that proof, revise the requirement to explicit bounded alerting
or adopt the private broker from Question 1; do not claim automatic fallback.

### 3. How is Monster capacity allocated?

- **Static, non-overcommitted reservations.** Pros: predictable serial floors,
  memory pressure, and timeout behavior; simplest attribution. Cons: idle
  capacity cannot be borrowed by another VM.
- **Measured CPU overcommit with fixed memory/disk reservations.** Pros: better
  utilization when Linux and Windows peaks do not coincide. Cons: simultaneous
  full-scope runs can invalidate timing assumptions and produce hard-to-replay
  timeouts.
- **Dynamic ballooning/hot-plug allocation.** Pros: highest theoretical
  utilization. Cons: operational complexity and run-to-run hardware drift make
  the Phase 0 baseline difficult to interpret.

**Recommendation:** start with static reservations and no CPU overcommit. Size
slot counts from those reservations, then consider bounded overcommit only in
Phase 4 with measured high-water evidence. On Windows, keep the OS and `_work`
on NTFS and place only `CARGO_TARGET_DIR` on a ReFS Dev Drive after the complete
runner-tool smoke passes; fall back to NTFS if any tool lacks ReFS support.

### 4. Which containment boundary does Venice use?

- **Proxmox VM-NIC firewall only.** Pros: available at the hypervisor and easy
  to bind to each VM. Cons: a rule-management error shares the production LAN's
  broadcast domain and leaves less defense in depth.
- **Dedicated bridge/VLAN only.** Pros: clean network boundary and simpler
  route-level reasoning. Cons: depends on gateway/switch VLAN support and still
  needs guest-specific ingress policy.
- **Dedicated VLAN plus Proxmox VM-NIC rules.** Pros: separate failure domains
  and defense in depth; the VLAN denies lateral routing while the VM rules
  restrict DNS, NTP, Internet egress, and admin SSH. Cons: most setup and two
  policy surfaces must remain synchronized.

**Recommendation:** use both layers. If the Venice network cannot support a
dedicated VLAN, Proxmox rules are an acceptable Phase 1 fallback only after the
deny/allow transcript proves RFC1918 and management-plane isolation.

### 5. When may `bolt` replace `macos-latest`?

- **Apple Silicon `bolt` on an isolated runner network.** Pros: architecture
  matches the current ARM `macos-latest` identity and preserves coverage. Cons:
  requires hardware discovery, capacity sizing, and a containment path for the
  192.168.10.0/24 location.
- **Intel `bolt` mapped directly to `macos-latest`.** Pros: gains warm local
  macOS builds. Cons: silently replaces ARM coverage with x64 while keeping the
  same environment identity, making the rollup misleading.
- **Keep canonical macOS hosted and add Intel only as a distinct advisory
  environment.** Pros: honest architecture coverage and no blocked Phase 3.
  Cons: does not reduce canonical macOS queue time; an additional environment
  increases work if enabled.

**Recommendation:** map `rb-macos` to `macos-latest` only if `sniff` confirms
Apple Silicon and containment meets Question 4. If `bolt` is Intel or cannot be
isolated, keep `macos-latest` hosted. Any Intel coverage must use a distinct
environment identity and a separate specification.

### 6. What credential probes runner state?

- **Fine-grained PAT (`Self-hosted runners: read` at the organization).** Pros:
  simplest API call and already supported by the draft. Cons: long-lived
  personal credential with a human lifecycle.
- **GitHub App installation token.** Pros: short-lived token, repository-scoped
  installation, auditable independent identity, and no coupling to a personal
  PAT. Cons: App setup and private-key/token minting add operational steps.
- **External heartbeat updates repository variables.** Pros: no runner-admin
  credential in the workflow. Cons: stale heartbeat races, a new daemon, and no
  authoritative `busy` state at dispatch.

**Recommendation:** use a GitHub App installation token minted only inside the
trusted workflow or private broker selected in Question 1. Keep a fine-grained
PAT as a time-boxed bootstrap path, not the steady state. Whichever path is
chosen must call the documented runner-group endpoint with read-only
permission. The repository-level endpoint is not the authority after adopting
the organization runner-group boundary.

### 7. How much persistence is acceptable?

- **Persistent runner services and targets.** Pros: maximum warm-build benefit
  and simplest operations. Cons: a compromised job can persist in the runner
  application, tool cache, work directory, or target until detected.
- **Disposable VM snapshot per job.** Pros: strongest cleanup and easiest
  recovery. Cons: loses warm targets and may make startup time dominate.
- **Disposable runner OS with a separately mounted build cache.** Pros: resets
  the execution plane while retaining some compile artifacts. Cons: a shared
  writable Cargo target is unsafe across concurrent slots; a content-addressed
  cache is required and shifts the design toward the remote-kache spec.

**Recommendation:** retain persistent slots only behind the server-enforced
boundary from Question 1, the network controls from Question 4, pinned actions,
and an image-rebuild runbook. Rebuild a slot from its versioned image after any
suspected compromise; never "clean" it in place. If those controls cannot be
provided, choose disposable VMs and accept that this feature's performance case
must be re-measured.

### 8. What happens to the existing hosted bootstrap preflight? — Phase 0 blocker

Every package call currently waits for the entire selected hosted OS preflight
matrix. Leaving it untouched can preserve the scarce-host queue on the critical
path before self-hosted work begins.

- **Keep it hosted and blocking.** Pros: preserves today's clean-hosted
  bootstrap proof and one-failure-per-OS behavior. Cons: can erase much of the
  routing win, especially when macOS is the slowest prerequisite.
- **Route it through the same runner map.** Pros: removes hosted queueing and
  validates the selected execution environment. Cons: a persistent image is
  not a clean-host fallback proof, and the all-platform matrix still creates a
  cross-platform barrier.
- **Run it hosted in parallel, but remove it as a prerequisite of
  `package-ci`; keep it verdict-gating.** Pros: retains the clean fallback proof
  and overlaps its queue with package fan-out; smallest change to the current
  contract. Cons: a long preflight can still set final verdict latency, and a
  bootstrap failure no longer suppresses redundant package failures.

**Recommendation:** use the parallel verdict-gating design for Phase 1 and
measure its critical-path contribution explicitly. If it still determines the
median verdict time, follow with a separate specification that removes the
cross-platform barrier (one platform preflight gates only the jobs for that
platform) while preserving a scheduled forced-hosted bootstrap smoke.
