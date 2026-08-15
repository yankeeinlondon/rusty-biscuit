---
status: draft
created: 2026-08-15
reviewed: true
reviewed_by: "codex/default"
reviewed_on: "2026-08-15"
related:
    - ../2026-07-27-local-runners/spec.md
    - ../2026-08-15-cicd-remote-runners/spec.md
    - ../../fixes/2026-08-06-cicd/spec.md
evidence:
    - docs/kache-strategy.md
    - fixes/2026-07-30-ci-cd-stabilization/plan.md
    - https://github.com/kunobi-ninja/kache-action/tree/a257c055543c2840700a9bbca8f9c3094a421b1b
    - https://developers.cloudflare.com/r2/api/tokens/
    - https://developers.cloudflare.com/r2/buckets/object-lifecycles/
---

# Remote (S3/R2) kache Backend for CI

## Summary

kache was removed from CI on 2026-07-30 after a measured trial returned 0–6%
hit rates (0.4–2.3% weighted by compile cost, ~2–15 s saved per leg). The
removal was not a verdict against kache; it was a verdict against the backend
the trial accidentally ran on. `kache-action@v1`, configured without
`s3-bucket`, fell back to the GitHub Actions cache. All same-platform jobs at
the same kache version and `Cargo.lock` competed for one immutable entry, so
the first successful save won and later jobs could not add their disjoint
stores. Branch scoping and the 10 GB repository quota further constrained the
backend, although pull-request runs can restore a default-branch cache and the
old WSL cache error was never proven to be quota-caused.
`docs/kache-strategy.md` records the exact re-entry condition:

> Revisit only with an S3/R2 backend and a measured comparison against a
> no-kache control.

This specification is that revisit. It puts kache in CI backed by S3-compatible
object storage, where independently addressed artifacts can accumulate across
jobs, branches, and machines without a GitHub cache entry's immutability or
repository quota. Mutable build manifests and shard indexes select which
content-addressed artifacts to prefetch; artifact blobs remain keyed by their
content and compiler inputs.

One number frames the size of the prize. Compilation was ~85% of the measured
test-shard jobs (`fixes/2026-08-06-cicd/spec.md` § Sharding — the same evidence
that removed sharding). **kache skips user-facing binaries and test harnesses
by default** (`cache-executables: false`), but the repository has not measured
what fraction of that compile time belongs to those outputs. This spec makes
`cache-executables: true` a first-class measurement arm so the decision is
based on that missing evidence rather than an assumed dependency/test-binary
split.

## Goals

1. Reduce CI compile time on GitHub-hosted legs by sharing kache artifacts
   through S3-compatible object storage across jobs, branches, and machines.
2. Prove the win with a measured A/B comparison against a no-kache control,
   judged by compile-cost-weighted hit rate and job wall time — not by
   `kache doctor`, and not by trusting hosted-runner wall-clock alone (see
   Measurement Discipline).
3. Restore the multi-machine path kache was designed for across hosted CI;
   developer and self-host bootstrap reads remain a gated Phase 4 extension.
4. Keep the failure mode benign: a bucket outage, a credential problem, or a
   kache bug degrades to an uncached compile, never to a red run.
5. Keep the repository's no-tracked-wrapper policy intact
   (`docs/kache-strategy.md` — "Wiring: none tracked"): activation stays a
   property of CI job configuration, not of `.cargo/config.toml`.
6. Bound the operational cost: the pilot records stored bytes and R2 Class A/B
   operations, and rollout requires a projected bill of no more than $10/month
   at the observed CI volume.

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

The action revision used by the old trial, with no `s3-bucket`, persisted the
store through the GitHub Actions cache. Its exact key contained kache version,
OS, architecture, and the `Cargo.lock` hash, but not package or job kind. Three
properties made that backend a poor fit for this repository's job shape:

| Property | Effect here |
|---|---|
| Exact entries are immutable | The first same-platform job to save the shared key wins; later jobs cannot merge their disjoint stores into it |
| Branch scoping | PR runs may restore from their base/default branch, but caches created by a PR remain scoped to that PR merge ref and do not become a repository-wide accumulating store |
| 10 GB repo quota | The repository's clean full build+test is ~71 G, so quota pressure and eviction are expected; the old WSL 400 response was consistent with pressure but did not prove it |

An S3 backend removes the immutable aggregate-entry collision and repository
quota. One bucket can be visible to every authorized branch, while lifecycle
and cost policy replace opportunistic GitHub cache eviction.

## Backend Decision

### Candidates

| | Cloudflare R2 | MinIO on Monster |
|---|---|---|
| New infrastructure | None — managed | An LXC/VM on `192.168.100.14` (Proxmox, 16-core EPYC 9175F, 384 GB RAM) |
| Egress cost | Zero (R2 has no egress fees) | Zero (LAN) |
| Reachable from hosted CI | Yes, over the internet | No — GitHub-hosted runners cannot reach a private 192.168.100.0/24 address |
| Reachable from homelab | Over the home uplink | At LAN speed |
| Ops surface | Managed bucket, credentials, lifecycle, and cost monitoring | Service upgrades, capacity, TLS, and secure hosted-runner reachability |

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

- One dedicated bucket (e.g. `rusty-biscuit-kache`). Do not share it with
  source, release, backup, or another repository's artifacts. R2 long-lived API
  tokens can be bucket-scoped, but not prefix-scoped; the bucket is therefore
  the credential boundary.
- `s3-prefix` begins with a generation owned by repository variable
  `KACHE_CACHE_GENERATION` (initially `v1`). Incrementing the generation is the
  instant, non-destructive invalidation path for a poisoned or incompatible
  remote. The retired generation is deleted only after healthy replacement
  runs exist.
- **Scoped credentials**: an R2 token limited to Object Read & Write on this
  bucket — the cache is a build-artifact store, not a place to grant account or
  bucket-administration access. Lifecycle configuration is performed out of
  band with an administrator credential and never exposed to CI.
- Two token classes, because a shared cache is a supply-chain surface — anyone
  who can write to the bucket can serve artifacts to everyone who reads it:
  - `KACHE_S3_ACCESS_KEY_ID` / `KACHE_S3_SECRET_ACCESS_KEY` (read-write) —
    used only by trusted runs (see Trust and Write Access).
  - A read-only token for developer-machine warming (Phase 4). Read-mostly for
    humans, write only for CI.
- Non-secret repository variables hold bucket name, the full account endpoint
  (`https://<ACCOUNT_ID>.r2.cloudflarestorage.com` or its jurisdiction-specific
  form), and `s3-region: auto`. Secrets hold only the access-key ID and secret.
- Lifecycle policy: expire artifact, manifest, and shard objects 30 days after
  upload. R2 lifecycle age is not reset by a read, so this is deliberate
  periodic rotation, not "30 days since last access." Keep R2's incomplete
  multipart-upload cleanup enabled. The pilot records actual stored bytes and
  operation counts; the lifecycle rule is a capacity guard, not a substitute
  for the rollout cost gate.

## Trust and Write Access

This repository is public. A pull request from a fork receives no secrets, so
fork legs cannot reach the bucket at all — they keep today's
`Swatinem/rust-cache` path unchanged. That is the structural part of the
containment and it costs nothing.

Within same-repo runs, only runs whose actor and commit authors are trusted may
**write** to or read from the bucket. The trust definition, the repository
variables that carry it (`CI_TRUSTED_ACTORS`, `CI_TRUSTED_AUTHORS`), and the
hosted preflight gate that evaluates them are shared with the companion
remote-runners spec. Whichever feature lands first creates one
`trusted_ci_run` output; the second consumes it. There must not be two gate
implementations. The gate fails closed when the event has no auditable commit
range, any author is outside the allowlist, the actor is outside the allowlist,
or the pull request comes from a fork.

`CI_KACHE_ENABLED` is a separate repository-variable kill switch and defaults
to `false`. Kache activates only when both `CI_KACHE_ENABLED == 'true'` and
`trusted_ci_run == 'true'`. This keeps compute routing and remote-cache rollout
independently reversible even though they share a trust decision.

Read access for untrusted same-repo runs is deliberately **not** granted. It
would expose a reusable credential to code controlled by that actor. Fork and
other untrusted legs keep the existing `Swatinem/rust-cache` path. Revisiting
this requires short-lived, run-bound R2 credentials or a read proxy; pilot hit
rate alone is not sufficient to weaken the trust boundary.

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
`.github/actions/kache-s3`. It wraps the reviewed upstream action revision
`kunobi-ninja/kache-action@a257c055543c2840700a9bbca8f9c3094a421b1b`, not the
mutable `@v1` tag. This revision supports the required version, S3,
manifest/namespace, reporting, size, and Windows inputs. The composite owns:

1. Version pinning: a setup step reads `.github/kache-version` (today
   **0.12.0**, the single authority shared with `just install-kache` at
   `justfile:48-51`) into the action's `version` input. The upstream action's
   `version` input resolves that exact release and verifies the downloaded
   archive against its published SHA-256 file. Never use "latest."
2. Backend wiring: `s3-bucket`, `s3-region: auto`, full `s3-endpoint`,
   generated `s3-prefix`, manifest key, namespace, and credentials from
   repository variables/secrets. Pass `github-cache: false` explicitly so a
   missing S3 value cannot silently reproduce the old backend. Keep
   `sync: false` and `warm: true`; selective manifest/shard prefetch is the
   default experiment, while pulling the entire remote is a separately measured
   diagnostic only.
3. Pre-activation validation: when enabled, all bucket, endpoint, generation,
   and credential inputs must be non-empty. Incomplete configuration skips
   kache, leaves `RUSTC_WRAPPER` empty, retains `rust-cache`, and writes the
   reason to the job summary. It never invokes the upstream action in its
   GitHub-cache fallback shape.
4. An `enabled:` input gating activation — the same pattern the retired action
   used, because a step-level `if:` on a local composite `uses:` fails to load
   ("Unrecognized named-value: 'inputs'", recorded in the old action's header).
   Trust-gate failures, fork legs, and the measurement control all pass
   `enabled: false` and take today's path.
5. A non-failing post-install verification of the exact pinned kache version.
   The action exposes `active` and `inactive-reason` outputs. A missing or
   mismatched binary clears `RUSTC_WRAPPER`, reports inactive, and lets the
   ordinary build proceed. Consumers gate kache reports and removal of
   `rust-cache` on `active == 'true'`, not merely on the requested input.
6. `pr-comment: false`. Dozens of matrix jobs must not race to update sticky PR
   comments; per-job summaries and uploaded machine-readable reports are the
   evidence surface.

The upstream action SHA and kache version are separate authorities: the SHA
pins installer behavior, while `.github/kache-version` pins the installed
binary. `ci_workflow_contracts.rs` enforces both and rejects a floating
`kunobi-ninja/kache-action@v1` reference.

### Remote identity and prefetch isolation

The remote blob population is shared, but its mutable prefetch metadata must
not be. Without explicit keys, every build for a host target triple overwrites
the same default manifest, reproducing near-zero prefetch in a different form.
Use these identities:

| Value | Identity | Purpose |
|---|---|---|
| `s3-prefix` | `artifacts/<generation>/<experiment-arm-or-rollout>` | Instant generation rollback and strict A/B isolation; never includes branch or package during rollout |
| `manifest-key` | `<package>/<build-family>/<environment>` | Exact prior build intent; `test`, `l2`, and `browser` deliberately use family `test`, while `check`, `lint`, and `wsl-archive` are distinct |
| `namespace` | `<build-family>/<runner-os>/<runner-arch>` | Shares dependency shards across packages with compatible invocation shapes without allowing `lint`, `check`, and `test` manifests to clobber one another |

Compiler version, target, features, and normalized flags remain part of
kache's artifact key. Do not add branch names to these identities: cross-branch
reuse is a goal. During the pilot the two kache arms use separate prefixes, so
the executable arm cannot warm or change the dependency-only arm.

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
| Windows legs | Phase 3 | Same pinned action, separate copy-mode acceptance result; see Windows below |

### Interplay with `Swatinem/rust-cache`

kache does not read the `target/` state restored by `rust-cache`, so the two can
coexist without corrupting one another. They must not coexist in an experiment
arm, however: a restored `target/` can prevent rustc from running and therefore
mask kache misses, while both tools' setup/post costs distort wall time. **The
A/B design therefore compares three mutually exclusive arms:**

1. Control — today's `rust-cache`-only path (the status quo).
2. kache-only — kache S3 with `rust-cache` removed on the pilot legs.
3. kache + `cache-executables: true`, kache-only.

Arm 2 vs 1 answers "does the S3 backend beat the GitHub cache it replaces".
Arm 3 vs 2 answers "is the test-binary share of the 85% worth the extra store
bytes". `rust-cache` is removed only when the local action reports active and
that leg's arm-2-or-3 numbers clear the acceptance bar. A
requested-but-inactive kache step must retain the control path for that job.
Every leg that keeps `rust-cache` keeps it keyed exactly as today
(`shared-key: package-ci-<pkg>-<job>-<env>`).

## Windows

The mutable `@v1` revision used by the old trial rejected `win32-x64`; that was
a property of that action revision, not kache or permanent architecture. The
reviewed action SHA selected above supports Windows x64/arm64, uses `.zip`
release assets, and kache 0.12.0 publishes the matching Windows archives.
Windows therefore uses the same composite and version authority as Linux and
macOS. Manual `cargo binstall` plumbing is the fallback only if the pinned
action fails its Phase 0 Windows smoke test.

Hosted Windows persists nothing locally: every run selectively warms from S3
and pushes new entries. Watch-points inherited from the strategy doc remain:

- NTFS restores by copy. A hit costs a real file copy per artifact; measure
  restore time against the compile time it replaces, per crate size class.
- The daemon is the least-proven part of kache on Windows. If selective warm
  prefetch is unreliable, measure explicit `kache sync --pull` rather than
  silently changing the Windows path; a full pull may cost more than compiling.
- A Windows cache hit is valuable only when copy time plus download time is
  lower than the compile time it replaces. Windows has its own acceptance
  result and cannot inherit the Linux/macOS decision.

Windows is Phase 3 because its copy-mode economics and daemon behavior need
separate evidence, and because the companion remote-runners spec may move
Windows legs onto self-hosted hardware with warm targets first, shrinking the
hosted Windows population this serves.

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

### Pilot protocol

The pilot runs through a temporary, push-triggered experiment workflow on one
trusted pilot branch. Do not use `workflow_dispatch`: the shared trust contract
deliberately rejects events with no auditable commit range. Select the pilot
package from the slowest representative `test` compile in the current baseline
and record the choice before seeing kache results.

One push schedules all three arms against the same commit, pinned toolchain,
package inputs, commands, and runner OSes. The two kache arms use separate
`s3-prefix` values; neither runs `rust-cache`. Prime each kache prefix once,
exclude that cold run, then re-run the same workflow run at least five times so
every measured sample has byte-identical source and configuration. Arms may run
in parallel, but no arm may consume another arm's local target or remote
prefix. Record hosted runner image identifiers so image drift can be identified
rather than mistaken for a cache effect.

The experiment must exercise the canonical `_package-ci.yml` setup and commands,
not a shortened benchmark that omits native prerequisites, fixtures, or
`cargo-nextest`. An experiment-only input may select the cache arm, but its
default is the current `rust-cache` behavior and production callers remain
unchanged until rollout.

### Evidence contract

Every measured kache job runs an `if: always()` evidence step before action post
cleanup and uploads a uniquely named artifact containing:

- `kache report --format json --since 24h` and the Markdown report;
- requested and effective cache mode, `cache-executables`, prefix generation,
  manifest key, namespace, kache version, and upstream action SHA;
- local/remote hits, misses, errors, compile-cost-weighted hit rate, time saved,
  bytes pulled/pushed, and setup/build/post durations when reported;
- commit SHA, `Cargo.lock` hash, Rust toolchain, runner OS/architecture/image,
  package, job family, exact command, run ID, and run attempt.

Artifact names include arm, package, job family, OS, run ID, and
`${{ github.run_attempt }}` so repeated attempts cannot collide.

The control records the same identity and timing fields without a kache report.
`measurement.md` links every raw run/artifact, states exclusions before
aggregation, shows every sample (not only medians), and records R2 stored bytes,
Class A/B operations, and the projected monthly bill. Missing or unparsable
evidence invalidates that sample; it never becomes a zero or a hit.

### Acceptance bar (Phase 2 → rollout gate)

Over the pilot's runs, on Linux or macOS pilot legs:

- Compile-cost-weighted hit rate ≥ **60%** on repeat-configuration runs (the
  same package, toolchain, and lockfile as a prior run — the case the backend
  exists for).
- Median warm-run wall time reduced ≥ **20%** vs the control arm, with the
   distribution (not just the median) clear of the hosted noise floor.
- Zero red runs attributable to kache across the pilot, including one
   deliberate bucket-outage rehearsal (see Fail-Safe).
- No setup silently selected the GitHub Actions cache backend, every measured
  sample produced its required report, and the projected R2 bill is
  ≤ **$10/month** at observed CI volume.

Choose `cache-executables: true` for a cleared job family only when it improves
median wall time by at least another 10% over dependency-only kache and still
meets the cost cap. Otherwise choose the simpler default (`false`). Acceptance
is per OS and job family; a Linux `test` win does not authorize macOS `lint` or
Windows rollout.

Phase 2 initially enables only the `check` and `test` families proven by the
pilot. `lint`, `l2`, and `browser` each require the same mutually exclusive
control comparison and per-family acceptance result before activation; sharing
dependencies or today's `rust-cache` key is not evidence that their economics
match.

Below the bar, the feature stops at the pilot and the bucket is deleted. That
is a legitimate outcome, recorded either way.

### What would sink it, stated up front

- **Keying vs `RUSTFLAGS` variance.** `lint` sets `RUSTFLAGS: -D warnings`
  (`_package-ci.yml:452`); `test` does not. Flags are in the blake3 key, so
  lint and test never share entries — expected, not a defect, but it halves
  the apparent hit rate if arms are compared carelessly.
- **Store-size thrash.** `max-size` on ephemeral runners bounds the per-job
  store. Do not assume the action's 50 GiB default is adequate; record peak
  local usage in the prime run and choose the smallest cap with at least 20%
  headroom. Re-measure with `cache-executables: true`, which can multiply
  stored bytes.
- **Uplink economics.** Hosted runners egress to R2 over the internet; a
  multi-GiB warm prefetch must save more compile minutes than it spends in
  transfer. `min-compile-ms` (default 1000) is the tuning knob; raise it if
  cheap crates cost more to fetch than to build.

## Fail-Safe

The wrapper must fail open. Missing configuration skips activation and retains
`rust-cache`; an unreachable remote after activation degrades to kache's local
miss/pass-through path and a slower non-incremental compile. No cache failure
may turn a correct build red or serve an unchecked artifact.

The pilot proves three isolated cases: a missing credential, an unreachable
endpoint with valid-looking credentials, and a deliberately corrupted object
in the pilot-only generation. It records the effective cache path and build
result for each. If corruption is not rejected as a miss/error or any case reds
the build, rollout stops.

Rollback has three distinct controls:

1. Set `CI_KACHE_ENABLED=false` to disable new activations immediately.
2. Increment `KACHE_CACHE_GENERATION` to abandon remote contents while retaining
   them for diagnosis; this is the corruption/incompatibility recovery path.
3. Rotate/revoke the R2 token for credential compromise. Lifecycle expiration
   is retention policy, not a kill switch.

## Interaction with Self-Hosted Runners

The companion spec (`features/2026-08-15-cicd-remote-runners/`) routes trusted
builds onto homelab hardware with warm per-slot targets, where both
`rust-cache` and kache are switched **off** — a restore would overwrite the
warm target that is the entire point of self-hosting.

The S3 store still earns its keep in that world:

- Trusted runs that fall back to hosted runners because no self-hosted slot is
  available can use the bucket. Untrusted, fork, and `workflow_dispatch` runs
  cannot read it and retain `rust-cache`; they neither warm nor benefit from R2.
- A fresh self-hosted slot (new host, evicted target, post-upgrade rebuild)
  can warm from the bucket once instead of compiling cold. That is an
  opt-in, run-by-hand step in the runner bootstrap, not CI wiring.
- Developer machines (Phase 4) pull read-only.

If remote-runners lands first and leaves no material trusted hosted population,
routine CI no longer populates this bucket. Phase 2 then stops unless measured
hosted fallback plus developer/bootstrap demand still justifies the cost and
staleness. The pilot-to-rollout gate applies to the population that actually
runs, not to the pre-routing baseline.

## Phasing

| Phase | Content | Exit |
|---|---|---|
| 0 | Bucket + scoped tokens + lifecycle; trust output; `CI_KACHE_ENABLED=false`; generation; secrets `KACHE_S3_*`; pinned composite action; contract tests | Trusted push smoke tests on Linux, macOS, and Windows install exactly 0.12.0; incomplete config selects `rust-cache`; no job uses GitHub cache through kache |
| 1 | Pilot: one pre-recorded package's `check`+`test` on Linux and macOS, isolated three-arm A/B, one prime + 5+ identical-commit repeats per arm | Acceptance bar evaluated; raw artifacts and decision recorded in `measurement.md` here |
| 2 | Rollout to Linux/macOS legs that clear the bar; `rust-cache` removed per cleared leg | Repeat-run wall time down ≥ 20% on cleared legs; no kache-attributed reds |
| 3 | Windows through the same pinned action; `_wsl-ci.yml` archive build | Same bar evaluated separately for Windows copy-mode restores and the Linux archive builder |
| 4 | Read-only dev-machine warming (`just` recipe wrapping `kache sync --pull`, documented in `docs/kache-strategy.md`) | Open Question 1 resolved; opt-in per host, per existing policy |

## Repository Contract and Documentation Updates

The old `ci_does_not_wire_the_kache_wrapper` test expresses the deliberately
temporary K2 policy. This feature intentionally supersedes that policy; leaving
the assertion in place would make correct implementation fail. Replace it with
contracts that prove:

- no tracked Cargo config or general initialization activates kache;
- CI reaches kache only through `.github/actions/kache-s3` at the reviewed
  upstream SHA and `.github/kache-version` remains the binary authority;
- the composite passes `github-cache: false`, validates all S3 inputs, disables
  PR comments, and exposes effective activation;
- call sites require both the shared trust output and `CI_KACHE_ENABLED`, keep
  `rust-cache` for inactive/untrusted legs, and never activate in the WSL guest;
- manifest/namespace identities are explicit and the experiment prefixes are
  isolated; and
- enabled jobs emit uniquely named machine-readable reports.

Update `docs/kache-strategy.md`, `.github/ci/README.md`, the relevant workflow
header comments, and `.claude/skills/kache/remote-cache.md` in the rollout
change. The old action/Windows caveat must be rewritten as historical evidence,
not left as current behavior. Add the upstream action SHA to the maintenance
audit so upgrades are deliberate and reviewed separately from kache binary
upgrades. Workflow permissions remain `contents: read`; `pr-comment: false`
means this feature does not add `pull-requests: write`.

## Open Questions

1. **What provenance boundary is sufficient for executable artifacts shared
   with developer machines?** Content-derived identities and checksum
   validation can detect accidental corruption when verified, but they do not
   prove that an actor holding the authorized write credential produced a
   benign artifact.

   - **A — Keep the trusted-writer bucket model for CI and Phase 4.**
     - Pros: supported by kache today; one shared population; no additional
       service; bucket-scoped credentials and generation rollback limit blast
       radius.
     - Cons: compromise of the long-lived CI write token can poison artifacts
       consumed by trusted CI and read-only developer clients; there is no
       independent build provenance.
   - **B — Keep R2 for trusted CI but remove Phase 4 developer reads.**
     - Pros: contains the consumer population to ephemeral CI; no developer
       credential distribution; simplest conservative boundary.
     - Cons: gives up a stated multi-machine benefit and makes the bucket less
       valuable if self-hosted routing removes most trusted hosted jobs.
   - **C — Require short-lived credentials plus signed manifests/artifacts.**
     - Pros: strongest provenance and revocation story; a stolen per-run token
       has a narrow lifetime and signatures can separate artifact authority
       from object-store authority.
     - Cons: kache does not currently define this verification contract; it
       requires an OIDC/credential broker and upstream or local kache work,
       materially expanding the feature.

   **Recommendation: A for the bounded CI pilot and rollout, B for Phase 4
   until a separate security review accepts A for persistent developer
   consumption or scopes C as its own feature.** This preserves the measurable
   CI goal without treating possession of a read-only token as proof of
   artifact provenance. The decision must be recorded before Phase 4; pilot
   performance data cannot answer it.
