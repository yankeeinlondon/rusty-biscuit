---
status: draft
created: 2026-08-15
reviewed: false
related:
    - ../2026-07-27-local-runners/spec.md
    - ../2026-08-15-cicd-remote-runners/spec.md
    - ../../fixes/2026-08-06-cicd/spec.md
evidence:
    - docs/kache-strategy.md
    - fixes/2026-07-30-ci-cd-stabilization/plan.md
---

# Remote (S3/R2) kache Backend for CI

## Summary

kache was removed from CI on 2026-07-30 after a measured trial returned 0–6%
hit rates (0.4–2.3% weighted by compile cost, ~2–15 s saved per leg). The
removal was not a verdict against kache; it was a verdict against the backend
the trial accidentally ran on. `kache-action@v1`, configured without
`s3-bucket`, fell back to the GitHub Actions cache — whose entries are
immutable, branch-scoped, and capped at 10 GB per repository — so a store
shared by all same-platform jobs could never accumulate. `docs/kache-strategy.md`
records the exact re-entry condition:

> Revisit only with an S3/R2 backend and a measured comparison against a
> no-kache control.

This specification is that revisit. It puts kache in CI backed by S3-compatible
object storage, where entries are mutable, shared across branches and machines,
and not bound by the 10 GB quota — the three properties whose absence sank the
first trial.

One number frames the size of the prize. Compilation is ~85% of every test job
(`fixes/2026-08-06-cicd/spec.md` § Sharding — the same measurement that removed
sharding). The test binaries themselves are a large share of that 85%, and
**kache skips user-facing binaries and test harnesses by default**
(`cache-executables: false`). A dependency-only cache therefore attacks well
under half of the compile cost. This spec makes `cache-executables: true` a
first-class measurement arm rather than an afterthought, because it is the
difference between caching the cheap 40% and the expensive 85%.

## Goals

1. Reduce CI compile time on GitHub-hosted legs by sharing kache artifacts
   through S3-compatible object storage across jobs, branches, and machines.
2. Prove the win with a measured A/B comparison against a no-kache control,
   judged by compile-cost-weighted hit rate and job wall time — not by
   `kache doctor`, and not by trusting hosted-runner wall-clock alone (see
   Measurement Discipline).
3. Restore the multi-machine path kache was designed for: CI populates the
   bucket, developer machines and (later) self-hosted runners warm from it.
4. Keep the failure mode benign: a bucket outage, a credential problem, or a
   kache bug degrades to an uncached compile, never to a red run.
5. Keep the repository's no-tracked-wrapper policy intact
   (`docs/kache-strategy.md` — "Wiring: none tracked"): activation stays a
   property of CI job configuration, not of `.cargo/config.toml`.

## Non-Goals

- Activating kache on any developer machine. That remains a per-host decision
  (`docs/kache-strategy.md` → Per-host activation decision). This spec only
  makes a remote store *available* to hosts that opt in.
- kache inside the WSL2 CI guest. The guest executes a prebuilt nextest
  archive and compiles nothing (`_wsl-ci.yml:34-43`;
  `.github/ci/environments.json` marks `wsl2-ubuntu` `archive_only`). The
  archive *build* on `ubuntu-latest` is in scope; the guest is not.
- C/C++ artifact sharing. kache's C/C++ caching is local-only; the remote
  carries Rust artifacts only.
- Replacing `Swatinem/rust-cache` everywhere on day one. It stays as the
  control and as the fallback for untrusted/fork legs, and is removed per leg
  only after that leg's measurements clear the acceptance bar.
- Speeding up self-hosted runners. Those keep warm per-slot `CARGO_TARGET_DIR`
  directories and run kache-off, per the companion remote-runners spec. The
  interaction is spelled out in Interaction with Self-Hosted Runners.

## Why the first trial failed, precisely

`kache-action@v1` with no `s3-bucket` persists the store through the GitHub
Actions cache. Three properties of that backend are fatal for this repository's
job shape:

| Property | Effect here |
|---|---|
| Entries are immutable | A store populated by 20 same-platform package jobs writes 20 disjoint stores; a hit requires a byte-identical prior invocation, which per-package keying almost never produces twice |
| Branch scoping | `main`'s store is invisible to PR legs, where most runs happen |
| 10 GB repo quota | Even a partial workspace store (a clean full build+test is ~71 G) cannot persist |

An S3 backend removes all three: objects are overwrite-in-place, one bucket is
visible to every branch, and capacity is a lifecycle policy instead of a quota.

## Backend Decision

### Candidates

| | Cloudflare R2 | MinIO on Monster |
|---|---|---|
| New infrastructure | None — managed | An LXC/VM on `192.168.100.14` (Proxmox, 16-core EPYC 9175F, 384 GB RAM) |
| Egress cost | Zero (R2 has no egress fees) | Zero (LAN) |
| Reachable from hosted CI | Yes, over the internet | No — GitHub-hosted runners cannot reach a private 192.168.100.0/24 address |
| Reachable from homelab | Over the home uplink | At LAN speed |
| Ops surface | None | Versioning, backups, capacity, certs for the endpoint |

### Decision: R2 first, MinIO as a measured follow-up

GitHub-hosted runners — the population this spec initially serves — cannot
reach a homelab MinIO endpoint at all, which decides the question for Phase 1.
R2 is configured exactly like S3 with `s3-endpoint` set to the account's
`r2.cloudflarestorage.com` URL.

MinIO on Monster becomes the answer only if the companion remote-runners spec
lands AND measurements show the home uplink is the bottleneck for warm prefetch
from self-hosted runners. At that point the reader population is entirely
inside 192.168.100.0/24 and a LAN-side bucket is strictly better. That migration
is a follow-up feature, not a phase here; kache's content-addressed keys are
backend-independent, so the bucket can be repopulated rather than migrated.

### Bucket layout and credentials

- One bucket (e.g. `rusty-biscuit-kache`), `s3-prefix` scoped per repository
  (default `artifacts`).
- **Scoped credentials**: an R2 token limited to this bucket and prefix — the
  cache is a build-artifact store, not a place to grant broad object-store
  access.
- Two token classes, because a shared cache is a supply-chain surface — anyone
  who can write to the bucket can serve artifacts to everyone who reads it:
  - `KACHE_S3_ACCESS_KEY_ID` / `KACHE_S3_SECRET_ACCESS_KEY` (read-write) —
    used only by trusted runs (see Trust and Write Access).
  - A read-only token for developer-machine warming (Phase 4). Read-mostly for
    humans, write only for CI.
- Lifecycle policy: expire objects untouched for 30 days. The store's working
  set turns over with `Cargo.lock`; a month of staleness is well past any
  useful entry. Cost is expected to be single-digit dollars per month at R2's
  storage pricing for a multi-GiB store, but the lifecycle rule is the guard,
  not the price.

## Trust and Write Access

This repository is public. A pull request from a fork receives no secrets, so
fork legs cannot reach the bucket at all — they keep today's
`Swatinem/rust-cache` path unchanged. That is the structural part of the
containment and it costs nothing.

Within same-repo runs, only runs whose actor and commit authors are trusted may
**write** to the bucket. The trust definition, the repository variables that
carry it (`CI_TRUSTED_ACTORS`, `CI_TRUSTED_AUTHORS`), and the gate that
evaluates them are specified once — in the companion remote-runners spec — and
reused here. If the remote-runners spec is not yet landed, this spec stands up
the same two variables and the same gate logic on its own; they are designed to
be shared, not duplicated forever.

Read access for untrusted same-repo runs is deliberately **not** granted in
Phase 1: it would hand the read token to any same-repo actor, and the marginal
win (warming runs that are rare in practice) does not justify a second
credential path to audit. Revisit after Phase 2 measurements.

## Integration Points

### Where the wrapper is currently neutralized

`RUSTC_WRAPPER: ""` is set as workflow-level env in three places, exactly so a
stray value cannot intercept builds:

- `_package-ci.yml:97` (all package jobs)
- `_wsl-ci.yml:111-112` (archive build and guest)
- `ci.yml:199-201` (preflight's explicit no-wrapper probe)

The kache step must override this per job (`RUSTC_WRAPPER: kache` in the step
env or via the action's own wiring), and the preflight probe stays
wrapper-free — it exists to prove the clean-checkout path, which remains true.

### Activation mechanism

Reuse `.github/actions/enable-kache`? It does not exist anymore — it was
removed with kache itself. Create a new local composite action
`.github/actions/kache-s3` wrapping `kunobi-ninja/kache-action@v1`, owning:

1. Version pinning: a setup step reads `.github/kache-version` (today
   **0.12.0**, the single authority shared with `just install-kache` at
   `justfile:48-51`) into the action's `version` input. Never "latest".
2. Backend wiring: `s3-bucket`, `s3-region: auto`, `s3-endpoint`, `s3-prefix`,
   credentials from repo secrets.
3. An `enabled:` input gating activation — the same pattern the retired action
   used, because a step-level `if:` on a local composite `uses:` fails to load
   ("Unrecognized named-value: 'inputs'", recorded in the old action's header).
   Trust-gate failures, fork legs, and the measurement control all pass
   `enabled: false` and take today's path.

### Job classes in scope

| Job | Kache? | Notes |
|---|---|---|
| `_package-ci.yml` `check` | Phase 2 | Pure compile — cleanest hit-rate signal |
| `_package-ci.yml` `test` | Phase 2 | The 85%-compile population; `cache-executables` matters most here |
| `_package-ci.yml` `lint` | Phase 2 | Clippy builds the same dep graph |
| `_package-ci.yml` `l2`, `browser` | Phase 2 | Shares `shared-key` with `test` today; shares the bucket instead |
| `_wsl-ci.yml` archive build | Phase 3 | Builds `x86_64-unknown-linux-gnu` test binaries on `ubuntu-latest` — the single compile the WSL path performs |
| `_wsl-ci.yml` guest | Never | Compiles nothing |
| `ci.yml` `scope`, `preflight`, `ci-verdict`, `summary` | Never | Compile nothing of the workspace (`ci-verdict` builds one small `--no-default-features` binary, ~8 s) |
| Windows legs | Phase 3, Option B | See Windows below |

### Interplay with `Swatinem/rust-cache`

During measurement both run; kache does not read `target/` state that
`rust-cache` restored, it reads its own store, so they compose without
corruption — but `rust-cache`'s post-step prune/upload is pure overhead once
kache's hit rate is real, and its restored `target/` can *mask* kache misses in
the measurements. **The A/B design therefore compares three arms, not two:**

1. Control — today's `rust-cache`-only path (the status quo).
2. kache-only — kache S3 with `rust-cache` removed on the pilot legs.
3. kache + `cache-executables: true`, kache-only.

Arm 2 vs 1 answers "does the S3 backend beat the GitHub cache it replaces".
Arm 3 vs 2 answers "is the test-binary share of the 85% worth the extra store
bytes". `rust-cache` is removed per leg only when that leg's arm-2-or-3 numbers
clear the acceptance bar; every leg that keeps `rust-cache` keeps it keyed
exactly as today (`shared-key: package-ci-<pkg>-<job>-<env>`).

## Windows

`kache-action@v1` rejects `win32-x64` — the reason the old CI carried a Windows
carve-out at every `enable-kache` site. kache itself ships Windows prebuilt
binaries; only the action is the blocker. `docs/kache-strategy.md` → Future
ambitions already names the path as **Option B**:

1. Install `kache.exe` in the composite action via `cargo binstall` at the
   `.github/kache-version` pin (mirroring `just install-kache`).
2. Set `RUSTC_WRAPPER` and the S3 config through the daemon or environment.
3. Persist nothing locally — hosted Windows runners are ephemeral, so every
   run warms from S3 and pushes new entries.

Watch-points, inherited from the strategy doc and still true:

- NTFS restores by copy. A hit costs a real file copy per artifact; measure
  restore time against the compile time it replaces, per crate size class.
- The daemon is the least-proven part of kache on Windows; prefer explicit
  `kache sync` steps over daemon-managed sync if the daemon proves flaky.
- If `kache-action` ships win32 support, this collapses to deleting the
  carve-out.

Windows is Phase 3 because it is the only platform needing hand-rolled plumbing,
and because the companion remote-runners spec may move Windows legs onto
self-hosted hardware with warm targets first, which would shrink the hosted
Windows population this serves.

## Measurement Discipline

Two repo-specific rules govern the numbers, both earned the hard way:

1. **Judge by `kache stats` / `kache report`, never by `kache doctor`.**
   `doctor` proves wiring; `stats` proves value. A green doctor with a 13% hit
   rate is a failing cache (`docs/kache-strategy.md` → Health).
2. **Do not trust hosted wall-clock alone.** `sniff-performance` measured +330%
   wall time on a hosted runner for a case whose work counters were
   byte-identical (`features/2026-08-12-perf-opt-in/spec.md` cites it as the
   metric problem in the repo). Wall time is evidence only alongside
   runner-minutes and the cache's own hit/miss counters, and only over enough
   runs to see the hosted noise floor.

### Acceptance bar (Phase 2 → rollout gate)

Over the pilot's runs, on Linux or macOS pilot legs:

- Compile-cost-weighted hit rate ≥ **60%** on repeat-configuration runs (the
  same package, toolchain, and lockfile as a prior run — the case the backend
  exists for).
- Median warm-run wall time reduced ≥ **20%** vs the control arm, with the
  distribution (not just the median) clear of the hosted noise floor.
- Zero red runs attributable to kache across the pilot, including one
  deliberate bucket-outage rehearsal (see Fail-Safe).

Below the bar, the feature stops at the pilot and the bucket is deleted. That
is a legitimate outcome, recorded either way.

### What would sink it, stated up front

- **Keying vs `RUSTFLAGS` variance.** `lint` sets `RUSTFLAGS: -D warnings`
  (`_package-ci.yml:452`); `test` does not. Flags are in the blake3 key, so
  lint and test never share entries — expected, not a defect, but it halves
  the apparent hit rate if arms are compared carelessly.
- **Store-size thrash.** `local_max_size` on ephemeral runners bounds the
  per-job store; the action's `max-size` default (50 GiB) is adequate for a
  per-job working set but must be re-checked with `cache-executables: true`,
  which multiplies stored bytes.
- **Uplink economics.** Hosted runners egress to R2 over the internet; a
  multi-GiB warm prefetch must save more compile minutes than it spends in
  transfer. `min-compile-ms` (default 1000) is the tuning knob; raise it if
  cheap crates cost more to fetch than to build.

## Fail-Safe

The wrapper must fail open. If the bucket is unreachable, credentials are
rejected, or the store corrupts, the build proceeds as an ordinary uncached
compile and the run goes green slower. The pilot includes one deliberate
outage rehearsal (bad credentials on a scratch branch) to prove this. A kache
that can red a run is worse than no kache, and the kill switch is structural:
`enabled: false` on the composite action's call sites, plus rotating the R2
token, plus (last resort) the bucket lifecycle expiring everything inside 30
days.

## Interaction with Self-Hosted Runners

The companion spec (`features/2026-08-15-cicd-remote-runners/`) routes trusted
builds onto homelab hardware with warm per-slot targets, where both
`rust-cache` and kache are switched **off** — a restore would overwrite the
warm target that is the entire point of self-hosting.

The S3 store still earns its keep in that world:

- Hosted legs remain for every untrusted run, fork PR, `workflow_dispatch`,
  and fail-safe fallback — that population keeps the bucket warm and keeps
  benefiting from it.
- A fresh self-hosted slot (new host, evicted target, post-upgrade rebuild)
  can warm from the bucket once instead of compiling cold. That is an
  opt-in, run-by-hand step in the runner bootstrap, not CI wiring.
- Developer machines (Phase 4) pull read-only.

If remote-runners lands first and shrinks the hosted population to
untrusted-only runs, the Phase 2 rollout decision should be re-taken against
whatever hosted volume remains; the pilot-to-rollout gate applies to the
population that actually runs.

## Phasing

| Phase | Content | Exit |
|---|---|---|
| 0 | Bucket + scoped tokens + lifecycle; secrets `KACHE_S3_*`; composite action with `enabled` gate and version pin | A scratch `workflow_dispatch` run warms, hits, and reports via `kache report` |
| 1 | Pilot: one package's `check`+`test` on Linux and macOS, three-arm A/B, 5+ repeat runs per arm | Acceptance bar evaluated; numbers recorded in `measurement.md` here |
| 2 | Rollout to Linux/macOS legs that clear the bar; `rust-cache` removed per cleared leg | Repeat-run wall time down ≥ 20% on cleared legs; no kache-attributed reds |
| 3 | Windows via Option B; `_wsl-ci.yml` archive build | Same bar, Windows-tolerant (copy-mode restores) |
| 4 | Read-only dev-machine warming (`just` recipe wrapping `kache sync --pull`, documented in `docs/kache-strategy.md`) | Opt-in per host, per existing policy |

## Open Questions

1. Does `kache-action@v1` honor a `version` input precisely enough to pin
   0.12.0, or does the composite need to install the pinned binary itself
   (as Windows Option B already must)? Resolve in Phase 0.
2. R2's `s3-region` semantics with `s3-endpoint` set (`auto` vs a specific
   region string) — confirm against the action's S3 client in Phase 0.
3. Should untrusted same-repo runs get read access in Phase 2, or does the
   single-credential-path simplicity keep winning? Decide from the pilot's
   data on how often untrusted same-repo runs repeat configurations.
4. Is `cache-executables: true` worth its store bytes on legs whose test
   binaries dominate (arm 3)? This is the highest-variance unknown in the
   spec.
5. If `kache-action` ships win32 support, Phase 3's hand-rolled plumbing is
   dead on arrival — check before building it.
