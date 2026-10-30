---
status: draft
reviewed: false
reviewed_by: claude/default
reviewed_on: 2026-07-27
clarified: "claude/claude-opus-5[1m]"
---

# Self-Hosted GitHub Actions Runners Specification

## Summary

CI wall-clock time for `rusty-biscuit` is dominated by queue time, not by work.
A representative full run spends four minutes waiting for every minute it spends
executing. The cause is a large per-run job fan-out competing with concurrent
branch runs against an account-wide GitHub-hosted concurrency ceiling, on runner
hardware whose disk cannot hold a Rust `target/` directory.

This specification adds self-hosted runners on always-on homelab hardware,
**macOS first** — the worst-queued platform, on the only hardware whose
specification is already known — then Linux and Windows. It is subject to two
hard constraints:

1. **Only changes authored by a trusted identity reach local hardware.** The
   trusted identity is configured in `.env` and mirrored to a repository
   variable for CI use.
2. **Local runner unavailability degrades to GitHub-hosted compute**, never to a
   queued or failed job.

Routing is scoped to the *building* job classes only — `check` and `test`. See
**Routing Scope** below. `lint`, `classify`, `scope`, `preflight`, and
`Failure-class summary` stay pinned to literal `ubuntu-latest`.

Cost is explicitly *not* a motivation. `rusty-biscuit` is a public repository, so
GitHub-hosted standard runner minutes are already free and unlimited. The entire
return is wall-clock latency and the ability to keep a warm build cache.

## Goals

1. Reduce CI wall-clock time by removing queue starvation and enabling a
   persistent `target/` directory.
2. Restrict self-hosted execution to commits authored by a configured trusted
   identity.
3. Fall back to GitHub-hosted runners automatically when a local runner is
   offline, reports the GitHub API's per-runner `busy` flag, or when the trust
   gate does not pass.
4. Contain a compromised runner so it cannot move laterally into the homelab.
5. Leave **both** `.github/ci/areas.json` and `scripts/ci/affected_scope.py`
   unchanged. The earlier wording — "`affected_scope.py`'s validation schema" —
   was the narrower of two possible readings; the stronger commitment holds.
   There is **no per-area routing exemption** in this design (see Platform
   Coverage Policy), so no mechanism exists that could ever have pressured
   either file. Routing resolves entirely from a flat platform→label map,
   applied uniformly across the area matrix.

## Non-Goals

- Reducing GitHub Actions spend. There is none to reduce.
- Migrating release, publish, or secret-bearing workflows to local hardware.
- Replacing GitHub's Actions control plane (that would require GitHub Enterprise
  Server and is disproportionate).
- Fixing the pre-existing red product failures catalogued in
  `features/2026-07-24-devops/`. Those remain owned by their areas.

## Evidence

Measured from run `30274087816` (2026-07-27), at job level via the Actions API.
Skipped jobs excluded.

**Baseline wall clock: 141 minutes.** Derived as the span from the run's
earliest job start to its latest job completion — `14:16:48Z` → `16:38:11Z`,
i.e. 2h21m23s. This is the authoritative figure and the one the Measurement Milestone measures
against; an earlier draft of this document quoted "2h24m" from the GitHub UI's
rounded display, which is superseded. Recompute the same way for any future
baseline so the number stays reproducible.

| Platform | Jobs | Queued | Executing | Queue : Run | Worst single queue |
|---|---|---|---|---|---|
| Linux | 59 | 1,178 min | 285 min | **4.1×** | 42 min |
| Windows | 23 | 303 min | 173 min | **1.7×** | 23 min |
| macOS | 25 | 1,027 min | 162 min | **6.3×** | **78 min** |
| **Total** | **107** | **2,508 min** | **620 min** | **4.0×** | — |

Peak concurrency observed *within* that run was 12, while five other branch runs
(`main`, `workspace-unify`, `homelab-justfile-load`, `exempt-native-deps`,
`devops-phase-4-orchestration`) competed for the same account-wide pool.

### Terminology: two senses of "busy"

The document uses "busy" in exactly one sense from here on, and names the other
explicitly:

| Term | Meaning |
|---|---|
| **`busy`** (code font) | The per-runner boolean returned by `GET /repos/{owner}/{repo}/actions/runners`. True when that one runner is executing a job. This is the only sense used in the probe and fail-safe rules. |
| **saturated** | The colloquial "the pool has no free slot". A property of a *pool*, not of a runner. Never called `busy`. |

### The serial floor: routing moves the queue, it does not remove it

The queue:run ratios above measure contention against the *account-wide hosted
pool*. Self-hosting replaces that ceiling with a much lower one. With a fixed
slot count per host, total executing minutes divided by slots is a hard serial
floor that no amount of routing can go below:

| Platform | Exec-min (measured) | Slots | Serial floor |
|---|---|---|---|
| Linux | 285 | 3 | **95 min** |
| Windows | 173 | 2 | **86.5 min** |
| macOS | 162 | 3 | **54 min** |

Against a **141-minute wall clock** for the whole run, routing all 59 Linux jobs
onto 3 slots produces a 95-minute serial floor for Linux alone. **At a 59:3
fan-out, run-wide routing MOVES the queue from GitHub's pool onto the homelab
rather than removing it.** That is the reason for the job-class scoping in
Routing Scope below, and the reason macOS — whose floor is comfortably under the
current wall clock — goes first.

Three honest caveats on this table:

- The exec-min figures are whole-platform totals from run `30274087816`,
  including jobs that Routing Scope keeps on hosted compute (`lint`, `scope`,
  `preflight`, `classify`, the summary). The routed subset is smaller, so each
  floor is an **upper bound** on the routed work, not a prediction. The
  Measurement Milestone replaces these estimates with measurement.
- The Linux and Windows slot counts are **provisional** (see Hardware), so those
  two floors move when the VM specifications are known.
- The probe's `busy` check cannot relieve any of this. `busy` is read once, in
  the preflight, before any of this run's own jobs have been dispatched — so the
  probe's *not-`busy`* determination can only hold for the **first** run to
  arrive at an idle host. Every subsequent job in the same run queues behind its
  own siblings, and the probe never sees that. See Residual Risk.

### Second-order benefit: decontention of the hosted pool

Moving a platform's jobs off the account-wide hosted pool leaves more of that
pool for the platforms still on it. Vacating 25 macOS jobs against a hard
5-concurrent macOS cap is worth more than the arithmetic suggests.

State this accurately: **this is a benefit of not using the hosted pool, not a
benefit of local execution.** It would accrue equally from simply running fewer
jobs. It is recorded here because it is real, not because it is attributable to
the runner hardware.

### Hosted runner hardware

| Runner | Cores | RAM | Disk |
|---|---|---|---|
| `macos-latest` (M1) | 3 | 7 GB | 14 GB |
| `ubuntu-latest` (public repo) | 4 | 16 GB | 14 GB |
| `windows-latest` (public repo) | 4 | 16 GB | 14 GB |

The 14 GB disk is the binding constraint. A warm Rust `target/` for this
workspace does not fit, which is why the current design carries `kache` plus
`Swatinem/rust-cache` with per-area shared keys as compensation. On self-hosted
hardware that compensation is not merely unnecessary — it is actively harmful.
Both caches must be switched off there; see **Cache Policy on Self-Hosted
Runners** for the specified mechanism.

The macOS concurrency cap is **5 concurrent jobs on Free, Pro, and Team plans
alike**. Upgrading plan tier does not relieve the worst-queued platform.

## Hardware

### Ratified (Phase 1)

| Host | Platform | Spec | Runner slots |
|---|---|---|---|
| iMac Pro | `macOS/x64` | 10-core Xeon W-2150B (20 threads), 128 GB ECC | 3 |

### Provisional (Phase 2, pending specification)

The Linux and Windows slot counts below are **estimates, not ratified values.**
They are derived from no measured hardware — both VM specifications are still
unknown, which is the Phase 2 blocker recorded in Open Questions. They are
listed to make the serial-floor arithmetic reproducible, not to commit to a
sizing.

| Host | Platform | Spec | Provisional slots |
|---|---|---|---|
| Linux VM | `linux/x64` | homelab VM — cores/RAM/disk unknown | 3 |
| Windows VM | `windows/x64` | homelab VM — cores/RAM/disk unknown | 2 |

### Per-slot environment

Each runner slot exports the same four variables. This is **one mechanism, not
four** — whatever sets `CARGO_BUILD_JOBS` sets all of them.

| Variable | Value | Purpose |
|---|---|---|
| `CARGO_BUILD_JOBS` | `6` | Caps cargo's build parallelism. rustc parallelism flattens well before 20 threads on a single crate graph, so several moderately-wide jobs outperform one very wide job. |
| `NEXTEST_TEST_THREADS` | `6` | Caps nextest's concurrent test processes. `CARGO_BUILD_JOBS` does **not** do this — see Test Parallelism Under Co-Tenancy, where it is a correctness requirement rather than tuning. |
| `CI_SELF_HOSTED` | `true` | The only signal a `just` recipe has that it is running on self-hosted hardware. See Privilege: `_ensure-native-libs` must not sudo. |
| `CARGO_TARGET_DIR` | `/Users/ci/targets/slot-N` | Puts the warm target **outside** the slot's `_work` tree, where `actions/checkout`'s `git clean -ffdx` cannot reach it. Without this the feature does not work at all — see The Warm Target Must Survive Checkout. |

`CI_SELF_HOSTED` exists because a `just` recipe cannot read GitHub Actions'
`runner.environment` context — that value is available only to workflow YAML
expressions. Workflow-level gating uses `runner.environment` (see Cache Policy);
recipe-level gating uses this variable. They are two consumers of the same fact,
in two languages that cannot share one source.

#### `CARGO_TARGET_DIR`: per-slot required, shared still forbidden

An earlier draft said "do not set a shared `CARGO_TARGET_DIR`" and stopped
there, which readers took as "do not set `CARGO_TARGET_DIR`." The distinction is
now load-bearing:

| Form | Status | Reason |
|---|---|---|
| **Shared** across slots — one directory for all three | **Forbidden**, unchanged | Cargo takes an exclusive lock on the target directory. Slots sharing one would serialize and forfeit the entire benefit. |
| **Per-slot** — one directory per slot, e.g. `slot-1`, `slot-2`, `slot-3` | **Required** | Never contended, so the lock argument does not apply. It is the only way the warm target survives `actions/checkout`. |

The lock argument was never an argument against `CARGO_TARGET_DIR` as such — it
was an argument against *contention*. Three uncontended directories have no
lock to contend for.

**Hosted path unchanged.** `CARGO_TARGET_DIR` is set only in the runner slot's
environment, so it is unset on GitHub-hosted runners and on developer laptops.
Cargo falls back to `<workspace>/target` exactly as it does today. This is the
same principle as `runner.environment` cache gating and `CI_SELF_HOSTED`: the
hosted path stays byte-identical.

**Do not "fix" `Swatinem/rust-cache`'s `workspaces: ". -> target"`.** That path
is correct and must stay. On hosted runners `CARGO_TARGET_DIR` is unset, so
`target` resolves as it always has. On self-hosted runners the action is gated
off entirely (Cache Policy), so its workspaces path is never evaluated. There is
no configuration in which the two disagree.

## Network Architecture

### The runner initiates; nothing listens

The Actions runner opens an outbound HTTPS long-poll to GitHub on TCP 443 and
holds it for ~50 seconds before re-establishing. GitHub never initiates a
connection to the runner.

Consequences:

- **No inbound firewall rules are required, on any host.**
- No port forwarding, no reverse proxy, no exposed listener.
- The tailnet needs no change to make runners work.

GitHub's Actions control plane cannot join a tailnet; only GitHub Enterprise
Server would place a control plane inside a private network, which is
disproportionate here. Tailscale Funnel is the wrong direction (it publishes
tailnet services outward), and an exit node would only alter egress IP.

Egress does traverse Microsoft infrastructure — Actions cache and artifacts are
stored in Azure Blob Storage (`*.blob.core.windows.net`). This is inherent to
hosted Actions and is TLS-protected outbound traffic.

### The actual threat model

| Vector | Assessment |
|---|---|
| Inbound from GitHub | Not applicable — no listener exists |
| Egress from runner | Real, but low-value to an attacker until code executes |
| **Lateral movement into the homelab** | **Primary risk** |

The runner VMs sit on the tailnet. Hostile code executing in a workflow becomes
a peer on that network. Containment of that peer — not protection of the GitHub
link — is where the security design belongs.

### Tailscale ACL containment

Runner hosts are tagged and denied all tailnet egress. Tailscale ACLs are
default-deny, so omitting a rule *is* the control.

```jsonc
{
  "tagOwners": { "tag:ci-runner": ["autogroup:admin"] },
  "acls": [
    // admin -> runners, for maintenance only
    { "action": "accept", "src": ["autogroup:admin"], "dst": ["tag:ci-runner:22"] }
    // no rule grants tag:ci-runner access to any peer -> default deny
  ],
  "ssh": []
}
```

Requirements:

- Authenticate runner nodes with a pre-authorized auth key carrying
  `tag:ci-runner`. A tagged node cannot re-tag itself.
- Leave Tailscale SSH disabled on runner hosts so a compromised job cannot use
  it for lateral movement.
- Place the runner VMs on a network segment with no route to the LAN.

With this in place, full compromise of a runner yields a VM with internet access
and no reachable homelab resource.

## Trust Gate

### Requirement

Only commits authored by a configured trusted identity may execute on
self-hosted runners. Everything else runs on GitHub-hosted runners.

### Configuration source

`.env` at the repository root is already **gitignored** (`.gitignore:39`) and
already holds a real secret (`ZAI_API_KEY`). It stays local. Two keys are added:

```sh
CI_TRUSTED_ACTORS=yankeeinlondon
CI_TRUSTED_AUTHORS=ken@ken.net
```

`.env.example` **does not yet exist and must be created and committed** to
document the keys — it is an explicit Phase 1a task, not existing state. Values
are mirrored to
repository **variables** (not secrets — they are not sensitive) by a new recipe:

```
just ci-sync-trust     # reads .env, runs `gh variable set CI_TRUSTED_ACTORS ...`
```

**Why not commit `.env` and read it in the workflow:** a file in the repository
is attacker-controlled in a fork pull request. An attacker could add their own
identity to it in the PR head and self-authorize. Repository variables are
modifiable only by repository admins, and `.env` being gitignored makes the
tamper path structurally impossible. The existing `.gitignore` entry therefore
forces the more secure design.

### Gate conditions

All four must hold for self-hosted dispatch:

1. **Not a fork.** `github.event.pull_request.head.repo.full_name ==
   github.repository`, or the event is not a pull request.
2. **Trusted actor.** `github.actor` ∈ `vars.CI_TRUSTED_ACTORS`.
3. **Trusted commit authors.** Every commit in the push range or pull request is
   authored by an address in `vars.CI_TRUSTED_AUTHORS`.
4. **The event is not `workflow_dispatch`.** See below.

Condition 3 is the literal requirement and is evaluated by the hosted preflight
job using `git log --format=%ae <base>..<head>` against a `fetch-depth: 0`
checkout.

### `workflow_dispatch` always routes to hosted

`ci.yml:8-12` triggers on exactly three events: `pull_request: branches: [main]`,
`push: branches: [main]`, and `workflow_dispatch`.

`git log <base>..<head>` is well-defined for the first two. `ci.yml:52-53` and
`:50-51` supply the PR and push base/head SHAs respectively, and the same values
already drive scope calculation.

`workflow_dispatch` has neither a base nor a head — `ci.yml:57` recognizes this
and falls through to `scope_args=(--all)`. There is no commit range to
authorize, so **the trust gate emits the all-hosted map unconditionally for that
event.** This is consistent with the fail-safe direction: no range means no
evidence of authorship, and absence of evidence routes to hosted.

Two adjacent cases that do **not** arise, recorded so they are not
re-litigated: feature branches do not trigger `ci.yml` on push (the `push`
trigger is filtered to `branches: [main]`), so the first-push-to-a-new-branch
case — where `github.event.before` is all zeros — and the force-push case never
reach this gate through `push`. `ci.yml:68` already handles the all-zeros base
defensively for the scope calculation; the trust gate inherits the same
protection but should never exercise it.

**Merge-commit caveat.** Merge commits created by the GitHub web UI are authored
by `noreply@github.com` or by the merging user. If merge commits are used,
`CI_TRUSTED_AUTHORS` must include those addresses, or the gate will correctly
but inconveniently route the run to hosted compute. This is a fail-safe
direction and is acceptable.

Secret-bearing workflows are additionally excluded by policy — see below.

## Availability Fallback

### The problem

GitHub Actions has **no native failover**. If `runs-on` names a label with no
online runner, the job queues for up to 24 hours and then fails. There is no
way to bound Actions queue time below that ceiling, and `timeout-minutes` covers
execution, not queueing.

Fallback must therefore be resolved *before* dispatch.

### Design: runner-map preflight

A single job on `ubuntu-latest` resolves, per platform, which label to use, and
emits a JSON map consumed by the `runs-on` of the **routed job classes** — not
by every downstream job. See Routing Scope.

It emits **one** output: the resolved map. Because there is no per-area
exemption, every area's routed jobs consume the same map, and `ci.yml`'s shared
`with:` block at `ci.yml:213-225` passes it once for the whole matrix with no
conditional:

```yaml
with:
  runner-map: ${{ needs.preflight.outputs.map }}
```

#### Self-hosted runner labels

Ratified. These are the literal strings the probe matches on and the map emits —
not placeholders:

| Platform | Self-hosted label | Phase |
|---|---|---|
| `macos-latest` | `rb-macos` | Phase 1a |
| `ubuntu-latest` | `rb-linux` | Phase 2 |
| `windows-latest` | `rb-windows` | Phase 2 |

A resolved map with Linux and macOS runners online and trusted, and the Windows
runner offline so its canonical hosted label was preserved:

```json
{
  "ubuntu-latest":  "rb-linux",
  "windows-latest": "windows-latest",
  "macos-latest":   "rb-macos"
}
```

That shows all three platforms for map *shape*. **Phase 1a resolves `rb-macos`
only**; `rb-linux` and `rb-windows` are registered in Phase 2, and until then
those two keys always carry their hosted labels.

`_area-ci.yml` gains a `runner-map` input, and the `runs-on` of the **routed job
classes only** becomes:

```yaml
runs-on: ${{ fromJSON(inputs.runner-map)[matrix.os] }}
```

**This keeps `areas.json` completely unchanged.** Canonical platform names
remain the vocabulary in the policy file, and `affected_scope.py`'s
`validate_area_schema` OS allowlist needs no modification. Label selection is a
runtime concern, resolved once per run.

## Routing Scope

Only the **building** job classes route through the runner-map. Everything else
keeps a literal `ubuntu-latest`.

| Job | Location | Routed? |
|---|---|---|
| `check` | `_area-ci.yml:74-107` | **Yes** |
| `test` | `_area-ci.yml:111-194` | **Yes** |
| `lint` | `_area-ci.yml:196-226` | No — literal `ubuntu-latest` |
| `l2` | `_area-ci.yml:235-283` | No — literal `ubuntu-latest` |
| `browser` | `_area-ci.yml:285-328` | No — literal `ubuntu-latest` |
| `classify` (failure class) | `_area-ci.yml:335-360` | No — literal `ubuntu-latest` |
| `scope` | `ci.yml:22-122` | No — literal `ubuntu-latest` |
| `preflight` | `ci.yml:131-174` | No — hosted matrix `runs-on: ${{ matrix.os }}` |
| `summary` (Failure-class summary) | `ci.yml:388-449` | No — literal `ubuntu-latest` |

### Why `lint` in particular must stay hosted

`_area-ci.yml:118` gates every area's `test` job on `needs: lint`, and
`_area-ci.yml:198` pins `lint` to `ubuntu-latest`. `lint` is therefore the
*gating* stage for the entire test fan-out.

Routing it locally would funnel 21 lint jobs (55 exec-min) through 3 slots — an
~18-minute serialized stage that every downstream `test` job waits on. Keeping
`lint` hosted preserves 20-wide concurrency on exactly the stage where width
matters most, and the 55 minutes it costs are minutes GitHub is already
absorbing for free.

### Why the reporting jobs must stay hosted

`classify` (`_area-ci.yml:335`) carries `timeout-minutes: 5`
(`_area-ci.yml:340`) and does no compilation at all — it reads five job results
and appends one line to `$GITHUB_STEP_SUMMARY`. Run `30274087816` contained 12
such jobs. Together with `summary`, routing them would consume build slots to
run `printf`.

### Consequence: what macOS Phase 1 actually routes

Stated here rather than discovered during implementation.

`full_os` defaults to `["ubuntu-latest", "windows-latest"]` and `check_os` to
`["macos-latest"]` — declared once in `scripts/ci/affected_scope.py:41-42` and
mirrored as the reusable workflow's defaults at `_area-ci.yml:30` and `:34`.
**`sniff` is the only area of 21 that overrides `full_os` to include macOS**
(`.github/ci/areas.json:63`; `full_os` appears exactly once in that file). Every
other area inherits the defaults.

Phase 1's routed macOS workload is therefore:

| Job class | Count | Work |
|---|---|---|
| `check` (macos-latest) | up to 20 areas | `cargo check --all-targets` (`_area-ci.yml:107`) |
| `test` (macos-latest) | 1 — `sniff` only | Full L1 suite via `just test` (`_area-ci.yml:182`) |

**This is a real test workload on self-hosted hardware, not a compile-only
proof.** That is the direct result of un-exempting `sniff` (see Platform
Coverage Policy → `sniff` routes to Intel), and it is the reason the exemption
was reversed: a check-only Phase 1 could not have validated co-tenancy at all.

Consequences:

- `NEXTEST_TEST_THREADS=6` (see Test Parallelism Under Co-Tenancy) is **live in
  Phase 1**, not dormant.
- The Measurement Milestone's third output is answerable — with a stated limit.
- `sniff`'s L1 failure-set diff against the hosted baseline is the single most
  important Phase 1 signal, because it is the only test evidence self-hosted
  hardware produces before Phase 2.
- **That leg gates merges.** See the next subsection — this is a decision, not
  an inherited default.
- The leg starts behind a hosted job: `_area-ci.yml:118` gates `test` on
  `needs: lint`, and `lint` stays on `ubuntu-latest`. The Measurement
  Milestone's timing comparison must measure from **job start**, not job
  creation, or hosted `lint` queue time is misattributed to self-hosted
  execution.
- `rendezvous-tests.yml` (`ci.yml:343`) runs its own
  `os: [macos-latest, ubuntu-latest, windows-latest]` matrix outside
  `_area-ci.yml` and takes no `runner-map` input, so it stays hosted.

### `sniff`'s macOS leg stays merge-gating

`sniff` does not override `soft_os`, so it inherits `["windows-latest"]`
(`scripts/ci/affected_scope.py:44`). `_area-ci.yml:128`'s
`continue-on-error: ${{ contains(fromJSON(inputs.soft-os), matrix.os) }}`
therefore resolves to **`false`** for `macos-latest`: a regression introduced by
the Intel move **blocks merges** rather than reporting softly.

**`soft_os` is not overridden.** This is a decision, and the mechanism that
would have softened it was considered and rejected rather than overlooked.

**The tension, stated first.** `_area-ci.yml:125-127`'s own comment describes
this mechanism's intended use precisely:

> Legs listed in `soft-os` report status but do not gate merges — used to light
> up a new platform (e.g. Windows) and burn down the revealed backlog of latent
> cross-platform failures before promoting it to a required check.

Self-hosted Intel macOS **is** a new platform by any reasonable reading. The
comment describes this situation.

**Why it is still not used.** Tier 1 of the Rollback section already covers
exactly this failure mode, and covers it better:

| | `soft_os` override | Tier 1 kill switch |
|---|---|---|
| Response time | A PR to `areas.json`, reviewed and merged | Seconds — one `gh variable set` |
| What it does | Makes a red leg advisory; the bad result still occurs | Routes the leg back to hosted ARM; the bad result stops occurring |
| Cost when unused | Permanently weakens a required check | None — the switch is inert until thrown |
| Goal 5 | **Violates it** — edits `areas.json` | Untouched |

A `soft_os` override would be a **second escape hatch for a risk the first one
already handles**, bought at the price of the Goal 5 commitment and of a check
that no longer gates. Tier 1 restores a *good* result in seconds; `soft_os` only
makes a *bad* result non-blocking, indefinitely.

**Consequence to accept knowingly.** If `sniff`'s macOS L1 goes red on Intel and
nobody throws the kill switch, merges to `main` are blocked. That is the
intended behavior: it is the same posture every other required check in this
repository has, and it is the reason the `test_apple_silicon_capabilities` fix
is a Phase 1a task, landing before `sniff` first dispatches to Intel.

### Probe mechanism

The preflight queries `GET /repos/{owner}/{repo}/actions/runners` and selects a
self-hosted label only when a runner carrying it reports `status: online` and is
not `busy`.

This endpoint requires `administration: read`, which **the default
`GITHUB_TOKEN` cannot be granted** — `administration` is not among the scopes
available to the `permissions:` key. A fine-grained PAT with that single
read-only permission is required, stored as repository secret
`RUNNER_PROBE_TOKEN`.

### Fail-safe behavior

The preflight emits the all-hosted map whenever anything is uncertain. There are
exactly five triggers:

| # | Trigger |
|---|---|
| 1 | Trust gate does not pass — including any `workflow_dispatch` run (gate condition 4). |
| 2 | `RUNNER_PROBE_TOKEN` is absent, as it is for fork PRs, which receive no secrets. |
| 3 | The API call errors, times out, or is rate-limited. |
| 4 | No matching runner reports `status: online`, or every matching runner reports `busy`. |
| 5 | The kill switch `vars.CI_LOCAL_RUNNERS_ENABLED` is not `true`. |

Every one of these degrades to GitHub-hosted compute. **The guarantee is about
*routing*: uncertainty never produces a job pointed at a label with no runner
behind it.** That is a narrower claim than the earlier wording ("there is no
path in which uncertainty produces a queued or failed job"), and the narrower
claim is the true one — see Residual Risk.

Because all five triggers produce a *green* run, they are also all silent. That
is what the Observability section addresses.

### Residual risk

Two distinct risks, previously conflated.

**1. Runner disappears between probe and dispatch.** A runner may go offline in
the window between the preflight's API read and a downstream job's dispatch,
leaving that job queued against the 24-hour Actions ceiling. The window is
seconds, and `CI_LOCAL_RUNNERS_ENABLED` provides a manual escape (see Rollback,
Tier 1). An optional scheduled watchdog that cancels runs queued beyond a
threshold is deferred as a follow-up rather than built up front.

**2. Self-inflicted queueing behind the run's own jobs.** This is the larger and
more certain of the two, and the earlier draft missed it entirely.

The probe reads `busy` **once**, in the preflight, before any of this run's own
jobs exist. The thing that then makes the runners `busy` *is the run being
dispatched*. A single area fan-out puts far more than 3 jobs onto 3 macOS slots,
so from the second job onward the run queues behind itself. The probe's
*not-`busy`* determination can only hold for the first run to arrive at an idle
host; it is a liveness check, and it is not — and cannot be — a capacity check.

**This is an accepted condition, not a defect.** Queueing behind your own work
at a known slot count is different in kind from queueing behind five unrelated
branch runs against an opaque account-wide ceiling: it is bounded by the serial
floor in the Evidence section, it is predictable, and it is not affected by what
anyone else is doing. It is nonetheless real, and the acceptance number set in
the Measurement Milestone must be measured against it rather than against a contention-free ideal.

It also means the fail-safe guarantee and this risk are separate claims.
Routing is fail-safe. Latency is not guaranteed.

## Cache Policy on Self-Hosted Runners

Both caches are switched **off** on self-hosted runners. This is a required
change, not an incidental one — leaving them on does not merely waste time, it
destroys the warm `target/` that is the entire point of the feature.

### The two problems

**1. Restore overwrites the warm target.** `Swatinem/rust-cache`'s keys carry no
runner identity. `_area-ci.yml:153` is representative:

```yaml
shared-key: area-ci-${{ inputs.area }}-test-${{ matrix.os }}
```

`matrix.os` is the canonical platform name, which under the runner-map is
exactly the key that no longer identifies where the job ran. A self-hosted job
would restore an archive built on a 4-core hosted runner directly over a warm
local `target/`.

**2. The post step prunes and uploads the target.** `Swatinem/rust-cache`'s post
step prunes `target/` to a cacheable subset — which removes precisely the
workspace-crate artifacts that make a local target warm — and then uploads the
remainder. In run `30274087816` that post step ran **81 times for 28 exec-min**.
On self-hosted hardware every one of those crosses the homelab uplink, every
job, to upload artifacts that will be discarded on restore.

### Mechanism: gate on `runner.environment`

`runner.environment` is `github-hosted` on GitHub's runners and `self-hosted` on
ours. It is evaluated per job at runtime, needs no new input threading, and —
decisively — **it keeps the hosted fallback path byte-identical to today.**

That property is the reason for choosing it over any alternative. All five
fail-safe triggers route to hosted. Hosted is not a rare backup path in this
design; it runs whenever anything is uncertain, including every fork PR and
every `workflow_dispatch`. It must not regress, and a condition that is false on
hosted runners cannot regress it.

**`Swatinem/rust-cache@v2` — add `if:` at all eight sites.** None currently
carries an `if:`.

| File | Line |
|---|---|
| `_area-ci.yml` | 102, 150, 221, 269, 314 |
| `ci.yml` | 256, 297, 325 |

```yaml
- uses: Swatinem/rust-cache@v2
  if: runner.environment == 'github-hosted'
  with:
    ...
```

**`enable-kache` — gate through the `enabled:` input, NOT `if:`.** The
composite's own header comment records why (`.github/actions/enable-kache/action.yml:8-13`):

> Gating lives on the `enabled` input rather than on the caller's `uses:` step,
> because a step that combines `if: ${{ inputs.kache }}` with a LOCAL composite
> `uses:` fails to load ("Unrecognized named-value: 'inputs'").

Adding an `if:` to these call sites would reintroduce that load failure. The
five `enabled:` expressions at `_area-ci.yml:101, 149, 220, 268, 313` gain a
third conjunct:

```yaml
enabled: ${{ inputs.kache && runner.os != 'Windows' && runner.environment == 'github-hosted' }}
```

### Note on the currently-unrouted sites

Only two of these eight `Swatinem` sites sit on routed jobs — `check`
(`_area-ci.yml:102`) and `test` (`:150`). The other six (`lint` `:221`, `l2`
`:269`, `browser` `:314`, and the three specialized `ci.yml` jobs at `:256`,
`:297`, `:325`) never route locally, so the new condition is always true there.
Likewise three of the five `enable-kache` sites (`lint`, `l2`, `browser`) are
no-ops today.

The gating is applied uniformly anyway: a later change that adds a job class to
the routed set must not silently reintroduce the cache-overwrite bug at a site
someone forgot.

### Disk budget

Persistent runners with disabled caches accumulate `target/` without bound.

**The per-slot disk budget and the eviction threshold are OUTPUTS of the Measurement
Milestone, not guesses.** No number is asserted here. That milestone
measures actual per-slot growth across 5 full-suite runs and the budget is set
from that data.

The prior draft's "a `cargo-sweep` timer is required" is retained as a
requirement but is not a substitute for a threshold: a sweep timer with no
measured budget behind it is an unsized control.

**What the timer sweeps.** The per-slot `CARGO_TARGET_DIR` paths — e.g.
`/Users/ci/targets/slot-{1,2,3}` — **not** `_work`. `_work` is scrubbed by
`actions/checkout` on every job (see The Warm Target Must Survive Checkout) and
holds nothing worth sweeping. Pointing a sweep at `_work` would be a no-op
against the disk problem while doing nothing about the directories that actually
grow.

Relevant history: this repository has documented `target/` reaching **~1.4 TB
across ~957k files at 1× scale**. This design multiplies that by slot count —
three independent target directories on the iMac Pro, deliberately not shared
(see Hardware → `CARGO_TARGET_DIR`). A 3-slot host is a 3× multiplier on a
problem that has already been severe once.

### Resolved: dirty `_work` between jobs

A prior draft left this open. It is closed by the per-slot `CARGO_TARGET_DIR`
decision, though **only the filesystem half of it** — the distinction matters
and is drawn below.

**What is resolved.** `actions/checkout@v4` defaults to `clean: true`, which
runs `git clean -ffdx && git reset --hard HEAD` before fetching. That scrubs
filesystem drift inside the checkout on every job, which is exactly the
guarantee a persistent runner otherwise lacks. It is now **safe to let it run**,
because the warm target no longer lives inside `_work`. No pre-job hook and no
periodic reaper are required for the repo tree.

**What is not resolved, stated precisely.** `git clean -ffdx` removes files. It
does **not**:

- kill leaked child processes,
- clean `/tmp` or `$TMPDIR`,
- reset a running tmux server.

Those were the other half of the original concern and they remain real. Scoping
them honestly: `sniff`'s L1 leg — the only test workload routed in Phase 1 —
drives no tmux, no browser, and no audio, and the L2 and browser tiers are
`ubuntu-latest`-pinned and unrouted (Routing Scope). **Process residue is
therefore a Phase 2 condition**, tracked alongside the cross-slot
resource-conflict task in Test Parallelism rather than as a separate open
question.

Phase 1a is unblocked. The general problem is not fully solved.

### Phase 2 decision: repoint `kache` at a local backend

Previously listed as Open Question 3; it is now a **Phase 2 decision** with a
named consequence either way.

`kache-action@v1` rejects `win32-x64` ("Unsupported platform"), which is what
forces the Windows carve-out repeated at `_area-ci.yml:101, 149, 220, 268, 313`.
A local kache backend shared across runner slots is **the only thing identified
that would remove that carve-out**.

If the answer is **no**, that is a further argument against self-hosting Windows
at all: Windows already has the lowest queue:run ratio of the three platforms
(1.7×, the least to gain), the highest serial floor per slot (86.5 min on 2
slots), and would remain the one platform with no build-acceleration path.

## The Warm Target Must Survive Checkout

**This is the same class of defect as the cache problem above**, and it is worth
naming the class explicitly: *a step that is a harmless no-op on ephemeral
hosted infrastructure and destructive on persistent hardware.* Cache Policy
caught one instance of that class. This section caught a second. **A future
reader should assume there is a third and go looking for it** — the pattern to
search for is any step whose correctness argument silently depends on the
machine being thrown away afterwards.

### The defect

`actions/checkout@v4` defaults to `clean: true`, which runs
`git clean -ffdx && git reset --hard HEAD` before fetching.

All five checkout sites in `_area-ci.yml` — lines **83, 131, 201, 243, 295** —
are bare `uses: actions/checkout@v4` with **no `with:` block**, so the default
applies at every one.

`.gitignore:62` is `**/target/*`, so `target/`'s contents are git-ignored.
`git clean`'s `-x` removes ignored files and `-d` removes the resulting
untracked directory. **The warm `target/` is deleted before every job.**

On a hosted runner this costs nothing: the VM is fresh and `target/` never
existed. On a persistent self-hosted runner it destroys the entire premise of
the feature.

**Had this gone undetected, the Measurement Milestone would have measured five
cold builds and concluded that self-hosting does not help.** The defect was
feature-killing, not a performance detail — and it would have produced a
confident, well-measured, wrong answer.

### The fix: move the target, not the checkout

Each runner slot exports `CARGO_TARGET_DIR` pointing **outside** its `_work`
tree — e.g. `/Users/ci/targets/slot-1`. See Hardware → Per-slot environment.

`actions/checkout` keeps `clean: true`. `_work` is scrubbed every job, which is
desirable (see Cache Policy → Resolved: dirty `_work` between jobs), while the
warm target lives where `git clean` cannot reach it.

**The five checkout steps are not edited.** No `clean: false`, no `with:` block,
no workflow change at those sites. Setting `clean: false` would have been the
obvious fix and the wrong one: it would preserve the target by *also* preserving
every other piece of filesystem drift, converting a solved problem into an open
one.

### Consequence: paths that assume `target/` at the workspace root

Moving the target breaks anything that constructs a path to it by hand rather
than asking cargo. An audit of the repository found the following.

**Safe — resolution is `CARGO_TARGET_DIR`-aware:**

| Pattern | Count | Why it is safe |
|---|---|---|
| `env!("CARGO_BIN_EXE_<name>")` | 55 sites | Cargo injects the real built path at compile time, honoring `CARGO_TARGET_DIR`. Includes `darkmatter/cli/tests/common/level2.rs:41`'s `MD_BIN`, and therefore `md_shim()`. |
| `std::env::current_exe()` | 33 sites | Resolves relative to the running test binary, which lives inside the target directory wherever that is. |
| Root `justfile` and `just/` recipes | 0 hardcoded sites | No recipe constructs a root-relative `target/debug` or `target/release` path; all binary invocation goes through `cargo`. |

**Breaks — hardcoded root-relative paths.** Each is a Phase task, not a
discovery for the implementer:

| Site | Code | Failure |
|---|---|---|
| `biscuit-icon/cli/tests/level2_terminal.rs:24-29` | `CARGO_MANIFEST_DIR` + `../../target/debug/icon`, then `.canonicalize().expect(...)` | Hard panic |
| `claudine/cli/tests/inline_compose_hash.rs:38` | `workspace_root.join("target/debug/md")` behind `assert!(path.exists(), ...)` | Hard assertion failure |
| `_area-ci.yml:193, 282, 327` | JUnit upload `path: target/nextest/ci/test-results.xml` | **Silent.** `if-no-files-found: ignore` means the artifact simply never appears. |

**Low severity:** `tree-hugger/lib/src/corpus/redaction.rs:39-40` replaces the
literal substrings `target/debug/` and `target/release/` with `<BUILD>/` for
corpus normalization. A relocated target stops being redacted. Cosmetic, but it
will surface as corpus diff noise.

#### Phase placement of these breakages

Only one is live in Phase 1, because Phase 1 routes 20 `check` jobs plus
`sniff`'s L1 leg and nothing else:

| Site | Phase | Reason |
|---|---|---|
| `_area-ci.yml:193, 282, 327` JUnit path | **Phase 1a** | `sniff`'s routed `test` leg hits `_area-ci.yml:193` on every run. It fails silently, so it must be fixed before the milestone or five runs will produce no JUnit evidence. |
| `biscuit-icon/cli/tests/level2_terminal.rs` | Phase 2 | An L2 test; `l2` is `ubuntu-latest`-pinned and unrouted. |
| `claudine/cli/tests/inline_compose_hash.rs` | Phase 2 | `claudine` does not declare macOS in `full_os`, so its `test` leg is not routed in Phase 1. |
| `tree-hugger` redaction | Phase 2 | Same reason. |

**Note the contradiction this resolves.** The fix was described as requiring no
workflow edits. That is true of the five *checkout* steps, and false of the
three *JUnit upload* steps, which name `target/` explicitly. The upload path
must become `CARGO_TARGET_DIR`-aware.

## Test Parallelism Under Co-Tenancy

This is a **correctness** requirement, not performance tuning. Getting it wrong
produces red CI, not slow CI.

### The arithmetic

| Control | What it caps | Value |
|---|---|---|
| `CARGO_BUILD_JOBS=6` | cargo's codegen/build parallelism | set per slot |
| `[profile.ci]` `test-threads` | nextest's concurrent test processes | **not set** |

`.config/nextest.toml`'s `[profile.ci]` (line 217) sets `retries`,
`slow-timeout`, `leak-timeout`, and `junit` — but no `test-threads`. Nextest
therefore defaults to `num_cpus`.

On a 20-thread iMac Pro running 3 slots, that is **3 × 20 ≈ 60 concurrent test
processes on 20 hardware threads**, on top of three concurrent 6-way cargo
builds. `CARGO_BUILD_JOBS` does not constrain this; it caps cargo, not nextest.

### Why oversubscription is a correctness problem here

`.config/nextest.toml` carries 18 `[[profile.ci.overrides]]` blocks (and 19 in
`[profile.default]`), and its comments state plainly that they are calibrated
against contention on **4-core hosted runners**:

- line 55 — "can approach the default 30s termination window **under full-suite
  contention on macOS debug builds**"
- lines 31–33 — tests that "finish in 5–10s isolated but get **squeezed past 15s
  under heavy parallel load**"
- lines 134–138 — `preflight_graph_*` tests that "spawn no subprocess at all"
  yet draw a **spurious LEAK-FAIL** because "the pipe read lags the 100ms
  window"

Every one of those thresholds is an empirically-tuned margin against a known
contention level. Tripling the contention invalidates the calibration.

And `[profile.ci]` sets `retries = 0` (line 222), deliberately — the comment
reads "Deterministic L1 failures must run exactly ONCE (D6)." **A
contention-induced LEAK-FAIL is therefore a hard red with no second attempt.**

### Decision

Set `NEXTEST_TEST_THREADS=6` in each runner slot's environment, alongside the
existing `CARGO_BUILD_JOBS=6`.

**Do not put `test-threads` in `[profile.ci]`.** That profile also runs on
4-core hosted runners — which, per Cache Policy, is every fork PR, every
`workflow_dispatch`, and every fail-safe fallback. A value of 6 there would
*increase* oversubscription on the majority path. Per-runner environment is the
only place the value can be correct.

`retries = 0` is **preserved**. See the rejected alternatives.

### Rejected alternatives, and why

**A `[profile.ci-selfhosted]` with scoped retries.** Considered and deliberately
not adopted first. Adding retries before measuring buys *silence* rather than
*data*: a retried flake reports green, and this repository's entire CI
methodology depends on failure-set stability (see Verification). Introducing a
retry at the same moment as a new execution substrate would make it impossible
to attribute a change in the failure set.

It remains available as a follow-up **if measurement shows the tuned thresholds
do not hold** — first from the Measurement Milestone's single-test-leg co-tenancy, and again from
Phase 2's full 3-way case. The sequence matters: measure, then decide whether to
mask.

**nextest `test-groups`.** These cannot solve this problem and should not be
re-proposed. A `test-group` such as `tts-audio` (`.config/nextest.toml:25`,
`max-threads = 4`) is scoped to a **single nextest process**. Three runner slots
are three separate nextest processes, each of which will independently admit up
to its group cap. Cross-slot serialization requires a **host-level lock** —
e.g. `flock` in the runner's job hook — which is outside nextest's model
entirely.

### Phase task: cross-slot resource conflicts on one macOS host

Three host-global resources are shared by all slots on the iMac Pro. None is
addressed by thread caps, because thread caps are per-process and these are
per-host.

| Resource | Evidence | Hazard |
|---|---|---|
| **Audio device** | `.config/nextest.toml:22-25` — the `tts-audio` group caps `max-threads = 4` because "running a dozen of those concurrently saturates the shared audio subsystem" | The device is singular. Three slots each admitting 4 audio children means 12 concurrent — precisely the condition the cap was written to prevent. |
| **tmux server namespace** | `_area-ci.yml:255-261` provisions and verifies tmux; L2 tests spawn sessions | Slots share a user and therefore a tmux server. Session names collide across slots. |
| **Headless Chrome** | `.config/nextest.toml:120-129` — browser teardown spawns "helper + crashpad processes"; the tier runs `-j 1` so "only one Chrome tears down at a time" | `-j 1` is per-process. Three slots means three concurrent Chromes despite the `-j 1` intent. |

Two further hazards specific to a *persistent* runner:

- The L2 harness's documented **"which terminal am I running inside" gotcha**
  (`.claude/skills/biscuit-test-harness/SKILL.md:90`) — backend `available()`
  gates read ambient environment, which on a persistent runner is whatever the
  previous job left behind.
- **`NO_COLOR` A/B tests require a cold tmux server.** A persistent runner never
  has one after its first L2 job. `ci.yml:306-329`'s `darkmatter-no-color` job
  is the closest current analogue and is `ubuntu-latest`-pinned.

The affected legs are `_area-ci.yml:235-283` (L2) and `:285-328` (browser).
**Both are `ubuntu-latest`-pinned and unrouted under Routing Scope**, so none of
these three conflicts is live in Phase 1.

Phase 1 does dispatch a test workload — `sniff`'s L1 leg — but exactly one, so
it produces **one nextest process co-resident with concurrent `cargo check`
jobs**, not three concurrent full suites. `sniff`'s L1 suite drives no audio, no
tmux, and no browser. Full 3-way test co-tenancy, and with it every hazard in
the table above, is a **Phase 2 condition**. The analysis is recorded here so it
is not redone then.

## Platform Coverage Policy

The iMac Pro is Intel; `macos-latest` is ARM64. This changes the macOS target
triple from `aarch64-apple-darwin` to `x86_64-apple-darwin`. **Every area
routes, including `sniff`.** There is no per-area exemption.

### Evidence

- Exactly **4 `target_arch` cfg sites exist across all 72 workspace members**,
  all in `sniff/lib/src/hardware/cpu.rs`.
- No inline assembly anywhere.
- The only `std::arch` usage is CPU feature detection in that same file.
- All other macOS-specific code is gated on `target_os`, which is
  arch-invariant.
- 20 of 21 areas use macOS only for `cargo check --all-targets`.

Additionally, the primary development host is `aarch64-apple-darwin`, so ARM
macOS coverage is already continuous through local `just test` and `just lint`.
Moving CI to Intel adds coverage of an architecture nothing else exercises.

### `sniff` routes to Intel

An earlier draft of this document exempted `sniff` from macOS remapping. **That
exemption is reversed.** The reasoning that produced it inverted cause and
effect: it treated a defective test as a property of the hardware rather than as
a defect to fix.

`sniff` is the one area where CPU architecture is the domain, and the only area
of 21 with macOS in `full_os` (full L1, not merely compile-check). Exempting it
therefore removed the *only* test workload that would ever reach the iMac Pro in
Phase 1, leaving a compile-only phase that could not validate co-tenancy — the
question the Measurement Milestone exists to answer. The exemption bought nominal ARM coverage at
the price of the phase's entire purpose.

#### What is affected, and what is not

| Code | Status under Intel routing |
|---|---|
| `SimdCapabilities::detect()` | **Unaffected.** `sniff/lib/src/hardware/cpu.rs:194-207`'s `test_detect_simd_returns_valid_capabilities` is genuinely cfg-gated per architecture — `#[cfg(target_arch = "x86_64")]` asserts `caps.sse`/`caps.sse2` (`:197-201`), `#[cfg(target_arch = "aarch64")]` asserts `caps.neon` (`:203-206`). Each arm asserts a real property on the arch that runs it. Moving to x86_64 runs the x86_64 arm, as designed. |
| `gpu.rs` Apple-Silicon behavior | **Affected.** `is_apple_silicon`, `unified_memory`, and `metal_family: "apple9"` are no longer exercised by CI. The iMac Pro's GPU is a Vega, not an Apple GPU. |

#### Required Phase 1a fix: `test_apple_silicon_capabilities` is vacuous

Verified at `sniff/lib/src/hardware/gpu.rs:390`:

```rust
#[cfg(target_os = "macos")]
#[test]
fn test_apple_silicon_capabilities() {
    let gpus = detect_gpus();
    // Find Apple Silicon GPU if present
    if let Some(apple_gpu) = gpus.iter().find(|g| g.vendor.as_deref() == Some("Apple")) {
        // Apple Silicon has unified memory
        assert!(apple_gpu.capabilities.unified_memory, "Apple Silicon should have unified memory");
    }
}
```

The test is gated on `target_os = "macos"`, which is arch-invariant, so it
*compiles and runs* on the Intel iMac Pro. The `if let` guard then matches
nothing (Vega GPU, vendor is not `"Apple"`), the body never executes, and the
test **passes without asserting anything**.

**The guard silently tolerating absence is the defect** — and it is a defect
today, on ARM, not one introduced by this change. On the current hosted
`macos-latest` runner a regression that stopped `detect_gpus()` from reporting
an Apple vendor would make this test vacuous rather than red.

Required property — the test must become **arch-honest**:

- On `aarch64`: **fail loudly** if no Apple GPU is found. Absence is a detection
  failure, not a skip condition.
- On `x86_64`: **assert the negative explicitly** — the expected outcome on
  Intel is a stated, checked assertion, not a silently-skipped body.

The implementation may choose whatever cfg shape it prefers. This document
specifies the property, not the code.

#### The residual trade-off, stated plainly

Routing `sniff` to Intel means **CI no longer exercises `gpu.rs`'s
Apple-Silicon behavior on ARM.** That is a real loss and it is not hand-waved
away here.

The mitigation is the one already stated in Evidence: the primary development
host is `aarch64-apple-darwin`, so ARM macOS coverage — including that GPU
path — remains continuous through local `just test` and `just lint`. The RC
dual-arch gate (Phase 3) independently requires an ARM build.

**This is a deliberate trade of CI ARM-GPU coverage for co-tenancy validation,
not an oversight.** It was chosen knowing what it costs.

#### Simplification: no per-area routing mechanism

The exemption would have required a mechanism, because of a shape mismatch:
the runner-map is a flat platform→label object with no area dimension, while
`ci.yml:213-225` calls `_area-ci.yml` from a single `with:` block shared by the
entire area matrix.

Un-exempting `sniff` **deletes that problem rather than solving it.** The shared
`with:` block is sufficient as written — one `runner-map:` value, one preflight
output, no `matrix.area` conditional, no second `hosted_map` output, no matching
change at the canary call site (`ci.yml:187-199`).

That is a simplification, not a compromise. It also makes the Goal 5 commitment
*cleaner*: `.github/ci/areas.json` and `scripts/ci/affected_scope.py` remain
untouched, and now there is no mechanism that could ever have pressured them
toward carrying a routing concern.

### Release candidates

`darkmatter/dmls`'s packaging contract ships `macos-universal.tar.gz`. A
universal binary requires both `x86_64-apple-darwin` and `aarch64-apple-darwin`
at build time, so the release path already cannot be single-architecture. The
release/RC gate must build both; this is an existing constraint made explicit,
not a new policy.

## Security Requirements

### Persistent, not ephemeral

Runners run persistent rather than `--ephemeral`. Ephemeral is the stronger
hygiene posture but wipes the target directory every job, which removes the
primary benefit. Given the trust gate and the tailnet containment, persistent is
the correct trade.

Persistence has three costs, each specified elsewhere rather than waved at:

| Cost | Status | Where |
|---|---|---|
| Unbounded target growth | Sized by measurement. A `cargo-sweep` timer is required on each runner host, but a sweep with no measured budget behind it is an unsized control — the per-slot budget and eviction threshold are outputs of the Measurement Milestone. | Cache Policy → Disk budget |
| Dirty `_work` between jobs | **Resolved.** `actions/checkout`'s `clean: true` scrubs it every job, and it is now safe to let it, because the warm target lives outside `_work`. | Cache Policy → Resolved: dirty `_work` between jobs |
| Process residue — leaked children, `/tmp`, a live tmux server | **Phase 2 condition.** `git clean` does not kill processes. Not live in Phase 1: `sniff`'s L1 leg drives no tmux, browser, or audio. | Test Parallelism → cross-slot resource conflicts |

The target directory surviving between jobs is not incidental to persistence —
it is the entire point, and it required a specific decision to achieve. See
**The Warm Target Must Survive Checkout**.

### Privilege: `_ensure-native-libs` must not sudo

`justfile:430` `_ensure-native-libs` currently runs `sudo apt-get install`. On a
persistent runner this would require passwordless sudo for arbitrary package
installation — root-equivalent — making any "unprivileged runner user"
containment fictional.

**Required change:** pre-bake declared native dependencies into the runner host
image, and make the recipe **verify-only** when running on a self-hosted runner:
assert each declared library is present and fail loudly if not. `areas.json`'s
`native` map remains the single source of truth exactly as the existing D9
design intends; it becomes an assertion rather than an installation. This also
removes an `apt-get update` from every job.

#### How the recipe learns where it is running

`justfile:430` branches on `uname -s` alone. It has no way to distinguish a
self-hosted Linux runner from a hosted one, and no access to GitHub Actions'
`runner.environment` context — that context exists only in workflow YAML
expressions, not in a `just` recipe's shell.

The signal is `CI_SELF_HOSTED`, exported by the runner's per-slot environment
alongside `CARGO_BUILD_JOBS` and `NEXTEST_TEST_THREADS` (see Hardware →
Per-slot environment). One mechanism provisions all three; there is no separate
configuration surface for this.

Required behavior:

| `CI_SELF_HOSTED` | Recipe behavior |
|---|---|
| `true` | Verify-only. Assert each declared library is present; fail loudly if any is missing. Never invoke a package manager, never invoke `sudo`. |
| unset or any other value | **Today's behavior, unchanged.** Install as it does now. |

**Defaulting to install when unset is the load-bearing part.** Local developer
hosts and GitHub-hosted CI runners never set the variable, so neither changes at
all — `just init` on a laptop and `_ensure-native-libs` on `ubuntu-latest`
behave exactly as they do today. This mirrors the Cache Policy principle: the
hosted path stays byte-identical.

**Phase placement.** The `sudo apt-get` path is Linux-only, so verify-only mode
is **not exercised until Phase 2**. The `CI_SELF_HOSTED` variable itself is
established in Phase 1 regardless, because the per-slot environment must exist
for `NEXTEST_TEST_THREADS` anyway. Phase 1 provisions the signal; Phase 2
consumes it.

### Workflow exclusions

The following never dispatch to self-hosted runners:

- `release-plz.yml` — publish-capable.
- `bench-nightly.yml` — carries `secrets.BENCHER_API_KEY`.
- `build-integrations.yml` — release-triggered artifact build.

`GITHUB_TOKEN` and `BENCHER_API_KEY` are the only secrets referenced by any
workflow. No workflow uses `pull_request_target`, which is the most common
self-hosted privilege-escalation vector; this must remain true.

### Repository settings

Actions → Fork pull request workflows → **Require approval for all external
contributors.**

### Priority order

1. Tailscale ACL isolation (`tag:ci-runner`, deny-all peer access)
2. Trust gate in the runner-map preflight
3. Repository fork-approval setting
4. Secret-bearing and publish workflows excluded from self-hosted
5. Passwordless sudo removed via pre-baked native dependencies
6. Runner VMs on a segment with no LAN route

Egress allowlisting is deliberately deferred. GitHub's required domain set
sprawls through CNAMEs that GitHub explicitly warns may change, making it
high-maintenance for marginal gain once 1–6 hold.

## Observability

### The failure mode this addresses

Every one of the five fail-safe triggers produces a **green run on hosted
compute**. That is correct behavior and terrible signal. Concretely:

| Silent-death scenario | Result |
|---|---|
| `RUNNER_PROBE_TOKEN` expires | Trigger 2 fires. Map goes all-hosted. CI green. Feature dead, indefinitely, with zero indication. |
| `vars.CI_LOCAL_RUNNERS_ENABLED` flipped to `false` during an incident and never flipped back | Trigger 5 fires. CI green. Feature dead. |
| Trust gate stops matching — username change, a new committer address, a merge-commit author | Trigger 1 fires. CI green. Feature dead. |
| Runner service stops and nobody notices | Trigger 4 fires. CI green. Feature dead. |

The PAT case is not hypothetical. **A fine-grained PAT has a maximum lifetime of
one year.** `RUNNER_PROBE_TOKEN` will expire, on a schedule, by design.

Success criteria 1–5 are one-time demonstrations. Once signed off they would
never fail again, because nothing re-runs them. **This section is the mechanism
that makes degradation continuously visible**, and it is therefore load-bearing
rather than a nicety.

### Requirements

**1. Step summary.** The preflight writes both the resolved runner-map **and the
per-platform reason for each hosted fallback** into `$GITHUB_STEP_SUMMARY`.

`ci.yml` already establishes this pattern twice — the scope table at
`ci.yml:107-122` and the failure-class block at `ci.yml:425-449`. Follow it:
a markdown table, appended, no new tooling.

| platform | label | reason |
|---|---|---|
| `macos-latest` | `rb-macos` | routed |
| `ubuntu-latest` | `ubuntu-latest` | trigger 4 — no online runner carrying `rb-linux` |
| `windows-latest` | `windows-latest` | trigger 5 — `CI_LOCAL_RUNNERS_ENABLED` is not `true` |

**2. Warning annotation.** Emit a `::warning::` annotation whenever a run that
**passes the trust gate** nonetheless falls back to hosted for any platform.

The trust-gate qualifier is what makes this signal rather than noise: fork PRs
and `workflow_dispatch` runs fall back *by design* and every one of them would
otherwise produce a warning. A trusted run falling back is the anomalous case —
it means the hardware, the token, or the kill switch is not in the state the
owner believes it is in.

## Rollback

Two tiers. Tier 1 is the incident response; Tier 2 is the retreat.

### Tier 1 — kill switch (seconds, no merge)

```sh
gh variable set CI_LOCAL_RUNNERS_ENABLED --body false
```

Fail-safe trigger 5 fires on the next run: the preflight emits the all-hosted
map, and every job dispatches to GitHub-hosted compute.

**The caches re-enable automatically**, because the cache gating keys off
`runner.environment` (see Cache Policy) rather than off the kill switch. A job
that lands on a hosted runner sees `runner.environment == 'github-hosted'` and
restores its cache exactly as it does today.

**This is a deliberate property of the Cache Policy design, not an accident.**
Gating the caches on `CI_LOCAL_RUNNERS_ENABLED` instead would have been the
obvious shortcut and would have coupled two independent concerns, leaving a
rollback path that disabled local execution *and* the hosted caches at the same
moment — the worst possible state to be in during an incident.

#### Tier 1 is load-bearing, not a convenience

This switch carries a design decision made elsewhere in the document. Because
`sniff`'s macOS L1 leg is **merge-gating** (see Routing Scope → `sniff`'s macOS
leg stays merge-gating), a red result on Intel blocks merges to `main`. The
proposal to soften that with a `soft_os` override was rejected specifically
*because* Tier 1 already resolves it — in seconds, without a PR, and without
editing `.github/ci/areas.json`.

That rejection is only sound while this switch works. Two obligations follow:

- `CI_LOCAL_RUNNERS_ENABLED` must be provisioned and **verified functional in
  Phase 1a**, before `sniff`'s leg first dispatches to Intel. An untested kill
  switch is not a mitigation.
- The Observability warning annotation must stay live, because a kill switch
  thrown during an incident and never restored is one of the silent-death
  scenarios that section exists to surface.

### Tier 2 — revert the plumbing (a PR)

Full revert of the runner-map plumbing: remove the `runner-map` input from
`_area-ci.yml`, restore literal `runs-on` values on the routed job classes,
remove the preflight job and its `map` output, drop the `runner-map:` line from
`ci.yml`'s shared `with:` block, and drop the `runner.environment` conditions.

The `test_apple_silicon_capabilities` fix is **not** part of this revert. It
corrects a defect that predates the feature (the test is vacuous on ARM today
whenever GPU detection fails) and should survive a rollback.

### Why both are documented

Tier 1 alone leaves **dead plumbing** in the workflows: a preflight job that
runs on every CI invocation to compute a map that is always all-hosted, a
`runner-map` input threaded through a reusable workflow, and eight cache
conditions that are always true. That is live surface area with no
counterbalancing benefit, and it decays — the next person to touch `_area-ci.yml`
has to reason about a routing mechanism that is switched off.

**Tier 2 should be exercised at least once** — on a branch, not merged — so the
revert is known to be clean before it is needed under pressure.

## Implementation Phases

The phase order was inverted from the prior draft. **macOS goes first**, on this
evidence:

| | macOS | Linux | Windows |
|---|---|---|---|
| Queue : run ratio | **6.3×** (worst) | 4.1× | 1.7× (least to gain) |
| Worst single queue | **78 min** | 42 min | 23 min |
| Concurrency ceiling | **hard cap of 5, on every plan tier** | account-wide, plan-sensitive | account-wide, plan-sensitive |
| Hardware | **known and owned** — 10-core Xeon W-2150B, 128 GB ECC | spec TBD | spec TBD |
| Build acceleration | kache supported | kache supported | `kache-action@v1` rejects `win32-x64` |

macOS is the worst-queued platform, the only one whose ceiling **no plan
upgrade relieves**, and the only one running on hardware whose specification is
already known. Linux and Windows move to Phase 2, blocked on VM specifications.

### Ordering: the measurement needs the plumbing

An earlier draft placed the measurement first and said it "runs before any
workflow is switched." **That was not implementable.** Routing a job to
`rb-macos` requires the runner-map, the `runner-map` input, and the resolved
`runs-on` — so a measurement that dispatches to the iMac Pro cannot precede the
plumbing that dispatches to it.

The alternative — measuring outside CI, by running `just test` on the iMac Pro
by hand — was rejected. It measures the wrong layer. Per the Verification
section, **CI-only bugs are the norm in this repository**: four prior bugs
passed local `actionlint` and text contract tests and still failed on a real
runner. A hand-run measurement skips the dispatch path, the `actions/checkout`,
the cache gating, and the per-slot environment as the runner service actually
exports it — which is precisely where this repository's bugs live.

Resolved ordering:

| Stage | What lands | Where |
|---|---|---|
| **Phase 1a** | Plumbing + observability — everything needed to dispatch and to prove where a run executed | Branch `ci/measure-selfhosted`, unmerged |
| **Measurement Milestone** | 5 measured runs; produces outputs (a), (b), (c) | Draft PR from that branch to `main` |
| **Phase 1b** | Acceptance decision, remaining tasks, merge | `main` |
| **Phase 2** | Linux and Windows | later |
| **Phase 3** | RC dual-arch gate | later |

`main` is never touched until the acceptance bar is met.

### Phase 1a — macOS plumbing (measurement branch)

Everything the measurement needs in order to dispatch, landed on
`ci/measure-selfhosted` and **not merged**.

1. Tailscale tagging and ACL containment for the iMac Pro.
2. Runner installation, dedicated unprivileged user, 3 slots registered under
   the label `rb-macos`, and the full per-slot environment:
   `CARGO_BUILD_JOBS=6`, `NEXTEST_TEST_THREADS=6`, `CI_SELF_HOSTED=true`, and
   `CARGO_TARGET_DIR` pointing outside `_work` (one directory per slot).
3. Fix `test_apple_silicon_capabilities` (`sniff/lib/src/hardware/gpu.rs:390`)
   to the arch-honest property specified in Platform Coverage Policy. This must
   precede `sniff`'s first Intel dispatch, not follow it.
4. Make the JUnit upload paths `CARGO_TARGET_DIR`-aware at `_area-ci.yml:193`,
   `:282` and `:327`. `sniff`'s routed `test` leg hits `:193` on every measured
   run, and `if-no-files-found: ignore` means the failure is **silent** — five
   runs would produce no JUnit evidence and no error. See The Warm Target Must
   Survive Checkout → Consequence.
5. `.env` keys; **create and commit `.env.example`** (it does not exist today);
   `just ci-sync-trust`; repository variables.
6. `RUNNER_PROBE_TOKEN` fine-grained PAT, with its expiry recorded.
7. Runner-map preflight: trust gate (including the `workflow_dispatch`
   condition), five fail-safe triggers, and **one** output (`map`).
8. `_area-ci.yml` `runner-map` input, applied to the `check` and `test` job
   classes only. `lint`, `l2`, `browser`, `classify` keep literal
   `ubuntu-latest`.
9. `ci.yml` passes `runner-map` from the existing shared `with:` block at
   `ci.yml:213-225` — no per-leg conditional, no change to the canary call site.
10. Cache gating: `if: runner.environment == 'github-hosted'` on all eight
   `Swatinem/rust-cache@v2` sites; the third `enabled:` conjunct at the five
   `enable-kache` sites.
11. `CI_LOCAL_RUNNERS_ENABLED` provisioned **and verified functional** — throw
    it once, confirm the map goes all-hosted, restore it. Rollback → Tier 1 is
    load-bearing depends on this working.
12. Observability: preflight step summary + `::warning::` on trusted-run
    fallback. **This is Phase 1a, not 1b**, because the milestone's procedure
    (step 5) uses the absence of that annotation to prove each measured run
    actually executed on self-hosted hardware. Without it, a run that silently
    fell back to hosted would be indistinguishable from a successful one and the
    whole measurement would be worthless.

Deliberately **not** in Phase 1a: `_ensure-native-libs`, the contract tests, and
the Tier 2 rollback exercise. None is required to dispatch, and `_ensure-native-libs`
is a no-op on macOS regardless — `playa` is the only area declaring a `native`
map and it declares only `ubuntu-latest` (`.github/ci/areas.json:43-45`), so the
per-area lookup yields nothing on `macos-latest`.

### Measurement Milestone

Runs on the Phase 1a branch, through the real dispatch path. Produces the
numbers the rest of the design depends on.

#### Dispatch path: a draft PR to `main` is the only option

Two facts constrain this, both verified against `ci.yml:8-12`:

| Trigger | Viable for the measurement? |
|---|---|
| `push: branches: [main]` | **No.** A push to `ci/measure-selfhosted` matches no trigger — `ci.yml` does not run at all on feature-branch pushes. |
| `workflow_dispatch` | **No.** Gate condition 4 makes it emit the all-hosted map unconditionally, so it cannot measure self-hosted execution by construction. |
| `pull_request: branches: [main]` | **Yes.** The only path that both fires `ci.yml` and can resolve a self-hosted map. |

A **draft** PR is sufficient: `ci.yml:8-9` declares no `types:` filter, so the
default `opened` / `synchronize` / `reopened` set applies, and draft PRs emit
all three. Draft status keeps the branch un-mergeable while it is measured.

**The trust gate passes on this path.** Checked against all four conditions:

| # | Condition | Result |
|---|---|---|
| 1 | Not a fork | ✅ `head.repo.full_name == github.repository` for a same-repo branch |
| 2 | Trusted actor | ✅ `github.actor` is the trusted identity opening the PR |
| 3 | Trusted commit authors | ✅ commits authored by the address in `CI_TRUSTED_AUTHORS` |
| 4 | Not `workflow_dispatch` | ✅ the event is `pull_request` |

If any of these did not hold the measurement would silently run on hosted
compute and produce a meaningless result — which is exactly what the
Observability warning annotation exists to make visible. Confirm the annotation
is absent before trusting any measured run.

#### Procedure

1. Register the runner with 3 slots and the full per-slot environment.
2. Land Phase 1a on `ci/measure-selfhosted`.
3. Open a **draft PR to `main`**.
4. Take **5 full-scope runs**. Force full scope so every area's `check` plus
   `sniff`'s L1 leg dispatch on each run.
5. For each run, confirm no hosted-fallback warning annotation was emitted.
6. Compare against baseline run `30274087816` (141 min, derived per Evidence):
   - **Failure sets** — which jobs and which tests fail, across all 5 runs.
   - **Timings** — measured from **job start, not job creation**, so hosted
     `lint` queue time is not misattributed to self-hosted execution.
   - **Per-slot disk growth** across the 5 runs.
7. Set the acceptance number from the data.
8. Delete `ci/measure-selfhosted`.

#### Required outputs

Phase 1b does not begin until all three exist:

| # | Output | Feeds |
|---|---|---|
| a | The wall-clock acceptance number | Replaces success criterion 4 |
| b | Per-slot disk-growth budget and eviction threshold | Cache Policy → Disk budget; sizes the `cargo-sweep` timer |
| c | Whether the tuned `.config/nextest.toml` thresholds hold under co-tenancy | Test Parallelism → decides whether a scoped-retry profile is needed |

**The acceptance number is set FROM this data.** It is not asserted up front,
and nothing in this document asserts one.

#### The precise scope of output (c)

Routing `sniff` (see Platform Coverage Policy) makes (c) answerable, which it
would not have been under the reversed exemption. But be exact about what is
measured: `sniff` is **one** test leg, so this milestone exercises **a single
nextest process co-resident with concurrent `cargo check` jobs** — not three
concurrent full test suites.

That is a genuine co-tenancy signal and it is enough to detect the failure mode
that matters most: whether `.config/nextest.toml`'s hosted-calibrated
`slow-timeout` and `leak-timeout` margins survive when the host is
simultaneously running other jobs. It is **not** a measurement of 3-way test
co-tenancy, and it says nothing about the shared audio device, the tmux server
namespace, or headless Chrome. Those remain Phase 2 conditions.

### Phase 1b — acceptance and merge

**Gate.** Phase 1b begins only if the Measurement Milestone produced outputs
(a), (b) and (c), and the failure set matched the hosted baseline. **A dirty
failure-set diff stops the feature here** — it is not a defect to be worked
around, because a non-deterministic failure set removes this repository's only
method of judging any CI change (see Verification).

Taken together, Phase 1a plus this phase deliver a **warm-target proof over 20
areas' `check` jobs plus one full test leg** (`sniff`'s L1). It is not
compile-only. See Routing Scope → Consequence.

Blocking items:

- **Arch policy (Intel vs ARM).** The `x86_64-apple-darwin` /
  `aarch64-apple-darwin` analysis in Platform Coverage Policy is Phase-1
  blocking, not Phase-2 background.
- **The acceptance number** from output (a), written into success criterion 4.
- **The disk budget and eviction threshold** from output (b), sizing the
  `cargo-sweep` timer.

Tasks:

1. Set the acceptance number and the disk budget from the measured data.
2. If output (c) shows the tuned `.config/nextest.toml` thresholds do not hold,
   decide on `[profile.ci-selfhosted]` (see Test Parallelism → Rejected
   alternatives) **before** merging, not after.
3. Pre-baked native dependencies; `_ensure-native-libs` (`justfile:430`) gains
   its `CI_SELF_HOSTED` branch, defaulting to today's install behavior when
   unset. Safe to defer past the measurement: the verify-only path is Linux-only
   and unexercised until Phase 2, and the recipe is already a no-op on
   `macos-latest` (see Phase 1a's closing note).
4. Contract tests (see below).
5. Document the Tier 2 rollback and exercise it once on a branch.
6. Demonstrate fallback by taking the runner offline mid-run and observing
   hosted dispatch.
7. Merge to `main`; delete `ci/measure-selfhosted`.

**The single most important signal across Phase 1a + Phase 1b** is `sniff`'s L1
failure-set diff against the hosted baseline. It is the first and only test
workload to run on self-hosted hardware before Phase 2, so it carries the entire
empirical case that self-hosted execution does not perturb results. A clean diff
here is what Phase 2 is authorized by.

#### Contract tests affected across Phase 1a and Phase 1b

These are named here so they are handled deliberately rather than discovered
when CI turns red. All live in
`tools/test-toolkit/tests/ci_workflow_contracts.rs`.

| Line | Test / assertion | Action |
|---|---|---|
| `:343-346` | `provisioning_jobs == 6` — exact equality | The `_ensure-native-libs` verify-only change **must preserve this count**. Verify-only changes what the recipe does, not which jobs call it. |
| `:236` | Literal `"full_os": ["ubuntu-latest", "windows-latest", "macos-latest"]` in `areas.json` | Must stay green. `sniff` keeps macOS in `full_os` — routing changes *which machine* that leg runs on, never the policy declaring it. This test is the guard proving `areas.json` was not touched. |
| `:379` | Literal `"os: [macos-latest, ubuntu-latest, windows-latest]"` in `rendezvous-tests.yml` | Must stay green. `rendezvous-tests.yml` takes no `runner-map` input and is unrouted. |
| `:177` | `area_ci_selects_the_ci_nextest_profile_explicitly` — asserts `_area-ci.yml` contains `NEXTEST_PROFILE: ci` | **Confirmed not affected.** `NEXTEST_TEST_THREADS` is set in the runner slot's environment, not in the workflow, and is a different variable from `NEXTEST_PROFILE`. `_area-ci.yml:69` is untouched. |

A new contract test should assert that `lint`, `classify`, and `summary` retain
a **literal** `ubuntu-latest`, so a later change cannot route the gating stage by
accident.

### Phase 2 — Linux and Windows

Blocked on the Linux and Windows VM specifications, which are still unknown (see
Hardware → Provisional). Together these cover 82 of 107
jobs per run, and Linux alone accounts for 1,178 queued minutes — but see the
serial-floor table: at 59 Linux jobs against 3 slots the floor is 95 minutes,
which is most of the current 141-minute wall clock.

1. VM specification, then ratified slot counts replacing the provisional 3 + 2.
2. Tailscale tagging and ACL containment for both VMs.
3. Runner installation, dedicated unprivileged user.
4. Pre-baked native dependencies on both images.
5. Extend the runner-map to `ubuntu-latest` and `windows-latest`.
6. **Decide the local-kache question** (formerly Open Question 3). It is the
   only identified route to removing the `win32-x64` Windows carve-out. If the
   answer is "no", re-evaluate whether Windows should be self-hosted at all.
7. **Process residue**, not filesystem residue. The `_work` tree is handled —
   `actions/checkout`'s `clean: true` scrubs it every job. What remains is
   leaked child processes, `/tmp` accumulation, and a persistent tmux server;
   `git clean` addresses none of these. Specify a reaper or a pre-job hook
   before a Linux `test` leg routes.
8. Cross-slot resource-conflict work for L2 / browser / audio, if those tiers
   are ever added to the routed set.
9. Fix the remaining hardcoded root-relative target paths, which become live
   once Linux `test` legs route: `biscuit-icon/cli/tests/level2_terminal.rs:24-29`,
   `claudine/cli/tests/inline_compose_hash.rs:38`, and
   `tree-hugger/lib/src/corpus/redaction.rs:39-40`. See The Warm Target Must
   Survive Checkout → Consequence.

Contract test relevant only if step 6 is taken:
`ci_workflow_contracts.rs:74` `kache_has_a_single_version_authority` — a local
backend must not introduce a second version literal outside
`.github/kache-version`.

### Phase 3 — Release-candidate dual-arch gate

Unchanged from the prior draft. Ensure the RC/release path builds both darwin
architectures for `macos-universal`.

## Verification

- `actionlint -shellcheck=` (the shellcheck integration hangs on `ci.yml`); run
  `shellcheck` separately on `run:` blocks.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`
- `python3 scripts/ci/test_affected_scope.py`
- A branch PR run. Per `features/2026-07-24-devops/handoff-remaining-work.md`,
  **CI-only bugs are the norm in this repository** — four prior bugs passed local
  `actionlint` and text contract tests yet failed on a real runner. Every
  workflow change in this feature must be verified on a real branch PR run.

That last point is why the Measurement Milestone runs as a draft PR to `main`
rather than as a hand-run suite on the iMac Pro (see Implementation Phases →
Ordering). A measurement that skips the dispatch path measures the layer where
this repository's bugs are *not*.

**The checkout defect is the worked example.** `actions/checkout@v4`'s
`clean: true` default deleting the warm target (see The Warm Target Must Survive
Checkout) is invisible to every tool in the list above:

- `actionlint` validates workflow *syntax*. The five checkout steps are
  syntactically perfect.
- The text contract tests assert on strings present in the workflow files. The
  destructive behavior comes from a **default that is not written anywhere in
  the repository** — there is no string to assert on.
- `test_affected_scope.py` covers scope calculation, not runtime file system
  effects.

It is only observable by running a second job on a machine that ran a first one.
That is precisely the configuration the milestone creates and that no local
check can simulate.

### Failure-set stability is a precondition, not a nice-to-have

This repository does not judge a CI branch by asking "is it green". It cannot —
roughly **34 CI jobs are already red for pre-existing product reasons**
catalogued in `features/2026-07-24-devops/`. The established technique is to
**diff the failure set** against that known-red baseline and require the diff to
be empty.

That technique has a hard prerequisite: **the failure set must be
deterministic.** A flaky self-hosted run cannot be evaluated by it at all. If
co-tenancy converts a stable red-set of ~34 into a fluctuating red-set of
34 ± n, the repository loses its only method of judging any CI change — not just
this one.

This is why the Measurement Milestone compares failure sets before it compares
timings, why it takes 5 runs rather than 1, and why `retries = 0` is preserved
rather than relaxed (see Test Parallelism → Rejected alternatives). Adding
retries would restore apparent stability by hiding the measurement.

It is also why routing `sniff` matters beyond queue relief. `sniff`'s L1 leg is
the only test workload self-hosted hardware executes before Phase 2, which makes
its failure-set diff the sole evidence that self-hosted execution is
result-neutral. A check-only Phase 1 would have produced no such evidence at
all — the argument that decided the exemption reversal.

**This is the gate on Phase 1b.** A dirty failure-set diff stops the feature; it
is not a defect to route around, because a non-deterministic failure set removes
the technique above for *every* future CI change, not just this one.

### Success criteria

| # | Criterion | Type | Verified in |
|---|---|---|---|
| 1 | A trusted push dispatches to self-hosted runners on every eligible platform. | One-time demonstration | Measurement Milestone (the draft PR is the trusted run) |
| 2 | A fork PR dispatches entirely to GitHub-hosted runners. | One-time demonstration | Phase 1b |
| 3 | Taking a runner offline routes that platform to hosted compute with no queued or failed job. | One-time demonstration | Phase 1b, task 6 |
| 4 | **Wall clock meets the acceptance number produced by Measurement Milestone output (a).** | Measured, deferred | Phase 1b, task 1 sets it |
| 5 | A runner host cannot reach any tailnet peer. | One-time demonstration | Phase 1a, task 1 |
| 6 | Across the Measurement Milestone's 5 runs, the failure set is identical to the hosted baseline's. | Measured | Measurement Milestone — **gates Phase 1b** |

**On criterion 4.** The prior wording — "completes materially faster than the
current 5–7 minute hosted cold check" — is not testable. "Materially" names no
threshold, and no experiment can return a verdict against it. It is replaced by
a number the Measurement Milestone produces. Until it runs, **this
document deliberately states no acceptance threshold**, because asserting one
before measuring would be inventing the answer.

**On criteria 1, 2, 3 and 5.** These are one-time demonstrations. Each is
verified once at sign-off and would never fail again afterwards, because nothing
re-runs them. They do not detect the silent-death scenarios in Observability;
the step summary and warning annotation are what does.

## Open Questions

Genuinely open — not deferred decisions, not tasks:

1. **Whether merge commits are used on `main`.** Determines whether
   `CI_TRUSTED_AUTHORS` needs `noreply@github.com`. Still open; affects gate
   condition 3 only, and the failure direction is fail-safe.
2. **Whether the deferred queue watchdog is wanted earlier.** A scheduled job
   that cancels runs queued beyond a threshold would bound Residual Risk 1.
   Deferred as a follow-up, not rejected.
3. **The concrete per-slot environment mechanism.** Hardware → Per-slot
   environment specifies *what* four variables each slot exports — including
   `CARGO_TARGET_DIR`, whose per-slot uniqueness the mechanism must guarantee —
   and *why*, but not *how*. On macOS the candidates are a `launchd` plist per
   slot, a `.env` file in each runner directory, or the runner's own `.env`
   support. Not chosen; deliberately left to Phase 1a implementation.

**None of these three blocks Phase 1a.** Item 1 fails safe (a non-matching
author routes to hosted). Item 2 is a follow-up by definition. Item 3 is an
implementation choice within Phase 1a rather than a prerequisite to it.

### Resolved since the prior drafts

| Was | Now |
|---|---|
| Linux/Windows VM specifications | Moved to **Phase 2**, which is blocked on it. Slot counts marked provisional in Hardware. |
| Repoint `kache` at a local backend | Promoted to a **Phase 2 decision** with a named consequence either way (Cache Policy → Phase 2 decision). |
| How measurement output (c) is obtained, given a check-only Phase 1 | **Resolved by routing `sniff`.** Phase 1 now dispatches a real test leg, so (c) is measurable — subject to the stated limit that it is one nextest process, not three. See Measurement Milestone. |
| Whether `_ensure-native-libs` can detect a self-hosted runner | **Resolved.** `CI_SELF_HOSTED`, exported by the per-slot environment; defaults to today's install behavior when unset. See Privilege → How the recipe learns where it is running. |
| The baseline wall clock (2h24m vs 141 min) | **Resolved.** 141 min, derived `14:16:48Z` → `16:38:11Z`. See Evidence. |
| Whether `sniff` is exempt from macOS remapping | **Reversed — it is not exempt.** The exemption would have removed the only test workload reaching the iMac Pro. See Platform Coverage Policy → `sniff` routes to Intel. |
| Whether `sniff`'s macOS leg should be softened via `soft_os` | **Resolved — no override.** Tier 1 already covers the failure mode, in seconds and without editing `areas.json`. See Routing Scope → `sniff`'s macOS leg stays merge-gating. |
| How the measurement dispatches, given it needs the plumbing it informs | **Resolved.** Phase 1a lands the plumbing on `ci/measure-selfhosted`; the milestone runs as a **draft PR to `main`**. `push` on a feature branch fires nothing and `workflow_dispatch` is all-hosted by construction, so this is the only viable path. See Ordering. |
| What cleans the persistent `_work` tree, and when | **Resolved by moving the target out of `_work`.** `actions/checkout`'s `clean: true` scrubs the repo tree every job and it is now safe to let it. Process residue — leaked children, `/tmp`, tmux — is *not* covered and is a Phase 2 condition. See Cache Policy → Resolved: dirty `_work` between jobs. |
| The self-hosted runner label strings | **Ratified:** `rb-macos`, `rb-linux`, `rb-windows`. Phase 1a registers `rb-macos` only. See Availability Fallback → Self-hosted runner labels. |
