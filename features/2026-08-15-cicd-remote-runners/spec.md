---
status: draft
created: 2026-08-15
reviewed: false
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

Two constraints are unchanged and non-negotiable:

1. **Only work published by a trusted identity reaches homelab hardware.**
   The trusted author is Ken Snyder (`ken@ken.net`). Everything else — fork
   PRs, outside contributors, `workflow_dispatch` from an untrusted actor —
   runs on GitHub-hosted runners, exactly as today.
2. **Local runner unavailability degrades to GitHub-hosted compute**, never to
   a queued or failed job.

## Goals

1. Reduce CI wall-clock time by moving trusted builds onto homelab hardware
   with persistent, warm `target/` directories.
2. Restrict self-hosted execution to runs authored and triggered by the
   trusted identity, verified before dispatch.
3. Fall back to hosted runners automatically when a runner is offline, busy,
   the trust gate fails, or a kill switch is thrown.
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

## What is inherited verbatim from the 2026-07-27 spec

The original specification's analysis survives intact where it concerns
properties of GitHub Actions rather than properties of the old CI. These are
incorporated by reference rather than restated in full:

- **Runners initiate; nothing listens.** The runner holds an outbound HTTPS
  long-poll to GitHub on 443. No inbound firewall rules, no port forwarding.
- **No native failover.** A `runs-on` label with no online runner queues for
  up to 24 hours. Routing must be resolved *before* dispatch, by a hosted
  preflight that emits a platform→label map.
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
  tiers.

Where the rest of this document specifies something differently, the
difference is deliberate and argued.

## Hardware

### Ratified

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
slot counts above make the serial-floor arithmetic reproducible; they commit
to nothing.

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
hardware, and macOS is the platform whose homelab story is least defined.

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

1. Capture 3–5 representative full-scope runs on the current CI at the job
   level via the Actions API (earliest job start → latest job completion;
   queued vs executing minutes per platform).
2. Recompute queue:run ratios and per-platform serial floors against the
   ratified slot counts.
3. Record the numbers in `measurement.md` here, the same way the original
   spec recorded its baseline, so every later claim is reproducible.

The design below proceeds on the *structure* of the problem (pre-dispatch
routing, trust, containment), which the re-measurement cannot invalidate. The
*go/no-go on run-wide routing per platform* is decided by the measured floors:
if a platform's routed serial floor exceeds its current wall-clock
contribution, that platform's routing stays scoped to a subset of job classes
until slots grow — exactly the arithmetic the original spec performed for
Linux at a 59:3 fan-out.

## Trust Gate

The requirement, as stated by the owner: only jobs published by Ken Snyder
(`ken@ken.net`) reach the external runners, reducing the risk of outside
parties exploiting the compute.

### Configuration

Repository **variables** (not secrets — they are not sensitive), set by a new
`just ci-sync-trust` recipe reading the gitignored `.env`:

```
CI_TRUSTED_ACTORS=ken-snyder-actor      # github.actor values
CI_TRUSTED_AUTHORS=ken@ken.net          # commit author emails
CI_LOCAL_RUNNERS_ENABLED=true           # kill switch (Tier 1 rollback)
```

The `.env`-to-variable flow is specified in the original spec and survives
unchanged: a file in the repository is attacker-controlled in a fork PR's head,
repository variables are admin-modifiable only, and `.gitignore` already
excludes `.env`. `.env.example` must be created and committed documenting the
keys (Phase 0 task — it does not exist yet).

### Gate conditions

All must hold for self-hosted dispatch:

1. **Not a fork.** `github.event.pull_request.head.repo.full_name ==
   github.repository`, or the event is not a pull request.
2. **Trusted actor.** `github.actor` ∈ `vars.CI_TRUSTED_ACTORS`.
3. **Trusted commit authors.** Every commit in the range is authored by an
   address in `vars.CI_TRUSTED_AUTHORS`, evaluated by the hosted preflight
   with `git log --format=%ae <base>..<head>` on a `fetch-depth: 0` checkout.
4. **Not `workflow_dispatch`.** No range means no authorship evidence; absence
   of evidence routes to hosted. (Fail-safe direction, unchanged.)

Adaptation for the current `ci.yml`, which differs from the old one in one
relevant way: `pull_request` now carries **no** `branches:` filter
(`ci.yml:8-18`, so stacked PRs are validated). That change widens the event
population the gate sees but not the gate's logic — base/head SHAs are
supplied by the event either way (`ci.yml:60-63`). The merge-commit caveat
also survives: GitHub-authored merge commits must be in
`CI_TRUSTED_AUTHORS` or those runs route to hosted — fail-safe, acceptable.

The same gate, the same variables, and the same recipe are shared with the
remote-kache spec's bucket write-access control. One trust definition for both
features; duplicating it would be a drift bug waiting to happen.

## Routing Design

### Runner-map preflight

A hosted job resolves, per platform, which label to use, and emits a JSON map
consumed by routed jobs' `runs-on`. The probe queries
`GET /repos/{owner}/{repo}/actions/runers` (fine-grained PAT with
`administration: read`, stored as secret `RUNNER_PROBE_TOKEN`) and selects a
self-hosted label only when a runner carrying it is `status: online` and not
`busy`.

Fail-safe triggers, any of which emits the all-hosted map:

| # | Trigger |
|---|---|
| 1 | Trust gate does not pass (including every `workflow_dispatch`) |
| 2 | `RUNNER_PROBE_TOKEN` absent (fork PRs receive no secrets) |
| 3 | API error, timeout, or rate limit |
| 4 | No matching runner online, or all matching runners `busy` |
| 5 | `vars.CI_LOCAL_RUNNERS_ENABLED` is not `true` |

All five produce a green, hosted, byte-identical-to-today run — and are all
silent, which the Observability step addresses.

### Labels

| Canonical environment | Self-hosted label | Phase |
|---|---|---|
| `ubuntu-latest` | `rb-linux` | 1 |
| `windows-latest` | `rb-windows` | 2 |
| `macos-latest` | `rb-macos` | 3 (pending `bolt` ratification) |

Resolved map example — Linux and Windows local, macOS fallen back:

```json
{
  "ubuntu-latest":  "rb-linux",
  "windows-latest": "rb-windows",
  "macos-latest":   "macos-latest"
}
```

### What routes, in the current CI

The per-package workflow's job classes, and their routing:

| Job | Location (`_package-ci.yml`) | Routed? |
|---|---|---|
| `check` | 125–194 | **Yes** |
| `test` (L1) | 200–442 | **Yes** |
| `lint` | 444–546 | **Yes** — see below |
| `l2` | 563–753 | Phase 2, with host-level resource coordination |
| `browser` | 755–830 | Phase 2 |
| `wsl` | 838–855 → `_wsl-ci.yml` | Phase 2, adapted (below) |
| `ci.yml` `scope` / `preflight` / `ci-tooling` / `ci-verdict` / `summary` | — | No — literal hosted labels |

The original spec pinned `lint` to hosted because it was the gating stage for
the whole test fan-out. In the current CI, `test` deliberately depends on
nothing (`_package-ci.yml:201-208` — "Lint is a parallel gate, not a
prerequisite"), so that argument is void. `lint` still compiles the package's
full dependency graph under clippy and benefits from a warm target; it routes
from Phase 1.

`l2` and `browser` are Phase 2 because they are the co-tenancy-heavy tiers:
tmux servers, headless Chrome teardown, `-j 1` browser teardown intent — all
host-global resources the original spec's analysis showed per-process caps
cannot serialize across slots. They need the host-level locks before more
than one slot runs them.

The reporting jobs stay hosted for the same reason as before: `ci-verdict`
builds one small `--no-default-features` binary (~8 s, `ci.yml:353-360`) and
reads artifacts; routing them burns build slots to run `printf`.

### The environment-identity invariant

`_package-ci.yml`'s `test` matrix is sound today **only because every native
environment name is also its own runner label** (`_package-ci.yml:210-216`).
The runner-map preserves exactly that property:

```yaml
runs-on: ${{ fromJSON(inputs.runner-map)[matrix.environment] }}
```

`matrix.environment` remains `ubuntu-latest` / `windows-latest` /
`macos-latest` — the value stamped into every status artifact, JUnit manifest
record (`BISCUIT_CI_ENVIRONMENT`, `_package-ci.yml:224`), and rollup cell.
`_package-ci.yml` gains a `runner-map` input (default: the identity map) fed
once from `ci.yml`'s `package-ci` call. **`affected_scope.py`,
`environments.json`, and the rollup are untouched.** A self-hosted leg is
indistinguishable, in result identity, from a hosted one.

### The WSL leg

`_wsl-ci.yml` runs on `windows-latest`, provisions a fresh WSL2 distro per run
via `Vampire/setup-wsl`, builds the nextest archive on an `ubuntu-latest` job,
and executes it in the guest. Two adaptations for Phase 2:

1. The archive-build job routes through the map like any other Linux compile.
2. The guest-execution job routes to `rb-windows` only after deciding between
   (a) keeping `Vampire/setup-wsl`'s per-run distro provisioning on the
   persistent Windows VM, and (b) using the VM's resident distro — tempting
   because `build-win` already hosts one, but it changes what the leg tests
   (a long-lived distro with drift vs a fresh one) and violates the
   leg's own byte-identical-binaries reasoning. Default: keep per-run
   provisioning; revisit only with measurements showing it is too slow.

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
Windows VM), never shared across slots (cargo's exclusive lock). The
repository's hardcoded root-relative `target/` paths identified by the
original audit — the JUnit upload staging path
(`target/nextest/ci-reports`, `_package-ci.yml:399` and its L2/browser
counterparts) and two L2 test files — must be made `CARGO_TARGET_DIR`-aware
**before** the first routed test leg runs, or the JUnit uploads fail silently
(`if-no-files-found: ignore`) and the Measurement Milestone produces no
evidence. This is a Phase 1 task, not a discovery.

**Disk budget**: the per-slot budget and eviction threshold are Measurement
Milestone outputs. The sweep infrastructure already exists and is
version-controlled (`scripts/sweep.sh`, the Linux/Windows scheduled sweeps,
`docs/kache-strategy.md` → Sweep script); wire the slot target dirs into the
sweep roots rather than inventing a new mechanism.

## Test Parallelism Under Co-Tenancy

Inherited analysis, re-anchored to the current files: `.config/nextest.toml`'s
`[profile.ci]` timeout margins are calibrated against 4-core hosted
contention, `retries = 0` is deliberate ("Deterministic L1 failures must run
exactly ONCE"), and thread caps are per-process. Each slot therefore exports:

| Variable | Value |
|---|---|
| `CARGO_BUILD_JOBS` | sized from the ratified VM allocation — provisional 6 |
| `NEXTEST_TEST_THREADS` | same sizing — provisional 6 |
| `CI_SELF_HOSTED` | `true` (recipe-level gating; `_ensure-native-libs` must not sudo) |
| `CARGO_TARGET_DIR` | per-slot path outside `_work` |

`test-threads` deliberately does **not** go into `[profile.ci]` — that profile
still runs on every hosted fallback leg, where a value of 6 would *increase*
oversubscription. Retries stay 0; measure first, then decide whether to mask
(original spec → Rejected alternatives, unchanged).

With 16 EPYC cores across (provisionally) 3 Linux slots, the co-tenancy math
is comfortable at build time; the hazards are the host-global resources in the
L2/browser tiers, which is why those tiers are Phase 2 behind host-level locks
(`flock` in the runner's job hook for tmux/Chrome serialization).

## Network Containment

The original spec contained runners with Tailscale tags on a tailnet. The
current homelab is the 192.168.100.0/24 segment behind the Venice gateway,
with the runner VMs as Proxmox guests on Monster (192.168.100.14). The
threat model is unchanged — no inbound surface exists (runners long-poll
out), the primary risk is **lateral movement from a compromised runner into
the homelab** — and the control must be rebuilt for this topology:

1. **Separate segment or Proxmox firewall rules** denying the runner VMs
   routed access to other 192.168.100.0/24 hosts, permitting only the gateway
   (outbound 443 to GitHub/Azure) and the Proxmox host's management interface
   from Ken's admin workstation — not from the VMs. Proxmox's built-in
   firewall on the VM NICs is the natural mechanism; a dedicated bridge/VLAN
   is the stronger one. Phase 0 decides which, with the VM allocation.
2. **No credentials on the runners beyond job scope.** The runner's local
   service account holds nothing homelab-facing: no SSH keys to other hosts,
   no Proxmox tokens, no NFS/CIFS mounts. The build-linux `~/.config` CIFS
   mount documented in `docs/kache-strategy.md` is a developer-host
   arrangement and must not be replicated on the runner VM — which also means
   the systemd-vs-cron sweep scheduling quirk it causes does not apply.
3. **Admin access inbound only.** SSH to the VMs from Ken's workstation for
   maintenance; the VMs initiate nothing toward the homelab.

`bolt` (192.168.10.157, outside the segment) needs its own containment answer
before Phase 3; see Open Questions.

## Observability

All five fail-safe triggers are silent green hosted runs. Two mitigations,
both from the original spec and both still right:

1. The preflight appends one summary line per platform — routed vs hosted and
   which trigger fired — to `$GITHUB_STEP_SUMMARY`.
2. A scheduled watchdog (cancels runs queued beyond a threshold, catching the
   probe-to-dispatch window where a runner vanished) is a named follow-up,
   not built up front.

Additionally: `kache`-style discipline for the runners themselves — a
`just runner-health` recipe (wrapping `gh api /actions/runers` for label,
status, and `busy`) so "is it up?" is one command, not a browser tab.

## Rollback

| Tier | Action | Speed |
|---|---|---|
| 1 | `gh variable set CI_LOCAL_RUNNERS_ENABLED false` — next preflight routes everything hosted | Seconds |
| 2 | Take the runner's registration offline (`./config.sh remove` or GitHub UI) — in-flight jobs drain | Minutes |
| 3 | Delete the labels' runner registrations and revert the `runner-map` wiring PR | One PR |

Tier 1 is the answer to "a self-hosted leg is red and blocking merges *now*":
it restores a good hosted result in seconds rather than softening the check
(the original spec's argument against a `soft_os` override, carried forward —
no per-area/per-package softening mechanism is introduced by this feature).

## Phasing

| Phase | Content | Exit |
|---|---|---|
| 0 | Baseline re-measurement; VM allocation on Monster; network containment; `.env.example` + `just ci-sync-trust` + variables; `RUNNER_PROBE_TOKEN`; `CARGO_TARGET_DIR`-aware JUnit paths; register `rb-linux` on `build-linux` | Baseline in `measurement.md`; trust gate demonstrably fails safe on a synthetic untrusted run |
| 1 | Preflight + runner-map wiring; route `check`, `test`, `lint` for Linux; per-slot env on `build-linux` | Measurement Milestone below |
| 2 | Windows VM: `rb-windows`, native legs + WSL adaptation; `l2`/`browser` routing behind host-level locks | Same milestone on Windows; no cross-slot resource reds |
| 3 | `bolt` ratification + macOS `rb-macos` | Same milestone on macOS |
| 4 | (Optional) watchdog, slot-count tuning from measured floors | — |

### Measurement Milestone (each phase)

Five full-scope trusted runs on the routed platform, compared against the
Phase 0 baseline, measuring:

1. Wall-clock (job start → verdict; **job start**, not creation, so hosted
   queue time upstream of dispatch is not misattributed — the original
   spec's rule).
2. Per-platform queue:run ratio, recomputed the same way as the baseline.
3. **Failure-set diff: zero.** The rollup grid on self-hosted legs must be
   byte-equivalent to the hosted baseline's. A new red on self-hosted
   hardware is a Phase-blocker, not a baseline entry.
4. Per-slot disk growth across the five runs → the eviction threshold,
   wired into the existing sweep.

## Open Questions

1. **Monster VM allocation** — cores/RAM/disk per VM, and whether Windows
   gets its target on a ReFS Dev Drive (the long-term Windows layout
   `docs/kache-strategy.md` recommends) or NTFS for Phase 2. Decides slot
   counts and the serial floors.
2. **`bolt`** — hardware spec, macOS confirmation, and the containment story
   for a host outside 192.168.100.0/24. Blocks Phase 3 entirely.
3. **Proxmox firewall vs dedicated bridge/VLAN** for runner containment —
   what does the Venice gateway/switch actually support?
4. **WSL leg on a persistent Windows VM** — per-run distro provisioning
   (default) vs resident distro; measure provisioning cost on real hardware
   before deciding.
5. **macOS architecture coverage** — if `bolt` is Apple Silicon, the hosted
   `macos-latest` (ARM) → `rb-macos` mapping is arch-neutral and nothing is
   lost; if it is Intel, the original spec's Platform Coverage Policy
   (cfg-audit, the `test_apple_silicon_capabilities` arch-honesty fix,
   ARM-coverage-via-dev-host trade) applies in full and must be re-verified
   against today's tree.
6. Whether the GitHub runner busy-probe PAT (`RUNNER_PROBE_TOKEN`) can be
   replaced by a GitHub App token with `administration: read` — one less
   long-lived credential.
