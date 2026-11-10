---
spike: S4
spec: 2026-11-06-ubuntu-archive-disk
question: How much disk does each Ubuntu archive build key use, against a hosted runner's free space?
answer: the Nov 6 nightly's 66 keys, built the way CI builds them, end at about 88 GiB (81.5 GiB of target/, 9.4 GiB of it written before the first key, plus 6.2 GiB of archives and sidecars). Single keys add 0–5 GiB. GitHub guarantees 14 GB; this runner's real free space is not determined, and the job's history suggests it is close to 88 GiB
confidence: high for the per-key and total figures; low for the hosted runner's free space, which needs the instrumented run
date: 2026-11-07
---

## Answer

**The whole owner job needs about 88 GiB** of disk, counted as real file
sizes. The `ubuntu-latest` owner builds every planned key serially into one
`target/`, and the total grows with the number of keys.

| After | `target/` (GiB) | Archive output dir (GiB) |
|---|---:|---:|
| Producer tool built, schematic fresh-output guard run | 9.4 | 0 |
| 10 of 66 keys | 15.6 | 0.3 |
| 24 of 66 keys (`content-policy-cli`) | 38.0 | 2.1 |
| 33 of 66 keys (`messenger`) | 54.3 | 3.8 |
| 66 of 66 keys | **81.5** | **6.2** |

The archives themselves add up to 5.63 GiB.

**Hosted runner free space.** GitHub's documentation guarantees **14 GB** of
SSD for `ubuntu-latest`, public or private. A runner-images maintainer has said
the same: "we indeed guarantee only 14GB of free space". Third-party
measurements of the x64 image, from 2024–2025, give 19–22 GB free on `/` of a
72 GB disk, with `/mnt` often holding 50 GB or more. By either figure this job
cannot fit.

**But that contradicts this job's history**, so the real free space on the
current runner is **not determined**:

- **Oct 26: the job succeeded.** That nightly (run 40344426066) had 65
  `wsl2-ubuntu` L1 consumers, about the same scope as today's 66 keys. Its
  produce step ran 2 h 07 min.
- **Nov 5 and 6: the job died** with "No space left on device" after 2 h 09 min
  and 2 h 15 min of produce. That is near the end of a full build at that
  speed, not near its start.
- **Nov 7 (run 42383053581) died the same way after 1 h 35 min of produce.**
  Its plan has the same 66 key identities at the same `head`.

A runner with 14–22 GB free would have died within the first 10–15 keys. The
history suggests the runner has much more free space than the guarantee, and
that the build now needs slightly more than it has. That is an inference, not
a measurement.

## Evidence

### How it was reproduced

- **Host and time.** `$BUILD_LINUX` (Debian 13, 16 vCPU, 16 GB RAM, no swap),
  in a fresh scratch clone at `a816f1447`. The Nov 6 nightly's own plan was
  used for `head` and for the build keys (artifact `ci-resolved-plan`, run
  42246114963). It has 66 `ubuntu-latest` keys and 9 `windows-latest` keys.
- **CI's steps, as-is** (`ci.yml` `build:` job, lines 618–809 at
  `a816f1447`), in order:
  1. `cargo build --release -p repo-deps --no-default-features --features
     build-tools --bin ci-build`
  2. The schematic fresh-output guard: `cargo fetch --manifest-path
     schematic/schema/Cargo.toml` and `just -f schematic/justfile
     test-fresh-output`
  3. `bash scripts/ci/produce-owner.sh`, with `PRODUCER=ubuntu-latest`
- **Job environment.** As in the job: `RUSTC_WRAPPER=""`,
  `NEXTEST_PROFILE=ci`, `COLORTERM=truecolor`. Added from
  `Swatinem/rust-cache`: `CARGO_INCREMENTAL=0`, `CARGO_TERM_COLOR=always`.
- **No cache restore.** `gh cache list` shows no `build-owner-*` cache, because
  none has been saved since the job last succeeded, so CI also builds cold.
- **Same linker as CI.** `ci-build` sets `CARGO_ENCODED_RUSTFLAGS` to
  `-Clinker=cc -Clinker-features=-lld`, which overrides the rig's own
  clang+mold setting.
- **Native prerequisites.** The owner's `native` list (`espeak-ng`,
  `libasound2-dev`, `libdbus-1-dev`, `libgtk-3-dev`, `libpulse-dev`,
  `libwebkit2gtk-4.1-dev`) was already installed on the rig. Nothing was
  installed.
- **Temp files on the measured disk.** `TMPDIR` pointed at the measured
  filesystem, because CI's `/tmp` is on the runner's root disk while the
  rig's `/tmp` is tmpfs.
- **Sampling.** A sampler recorded, every 20 s:
  - `df` free space;
  - `du --apparent-size` and allocated size of `target/`;
  - the size of `$RUNNER_TEMP/build`;
  - the archive count.

  The raw data, plan, manifests, and analysis scripts are in this agent's
  scratchpad: `spikes/a2/s4-data/` and `spikes/a2/scripts/`.
- **Apparent sizes are the runner figure.** The rig's filesystem is ZFS with
  compression: `target/` allocated 30.9 GiB for 81.5 GiB of files. An ext4
  runner allocates about the apparent size, so every figure here is an
  apparent size.

### The out-of-memory interruption

At about 17:06 UTC the rig ran out of memory and the kernel killed GNU `ld`
while it linked `darkmatter`'s `html_to_png` example. That was 16 parallel
links on 16 GB with no swap. The host did not answer SSH for about 12 minutes.

- That run had finished keys 1–24. Its processes were stopped, and every
  `cargo`/`rustc` of toolchain 1.98.1 on the rig was killed. Only this spike
  was building there at the time, as far as `pgrep` showed.
- The produce step was then rerun with `CARGO_BUILD_JOBS=6` on the **same
  `target/`** from 17:21 to 17:52. Keys 1–24 re-archived without compiling,
  then keys 25–66 built. The rerun exited 0 for all 66 keys.
- **The disk figures are unaffected**: `target/` contents do not depend on job
  parallelism.
- **Timings are not comparable** with CI's. A hosted runner has 4 vCPU and
  16 GB and did not hit this.

Phase log: tool 21 s (16:38:58); guard 363 s (16:39:19); produce 16:45:22
until the OOM stall; produce rerun 1868 s (17:21:00–17:52:09).

### Per-key growth of `target/`

| # | Key (features) | Archive MiB | `target/` after (GiB) | Δ GiB |
|---:|---|---:|---:|---:|
| – | tool + guard | – | 9.4 | 9.4 |
| 1–6 | arcam-amp-integration, biscuit-contract, biscuit-file (fetch), biscuit-file-cli, biscuit-hash (all), biscuit-hash-cli | 79 total | 11.5 | 2.1 |
| 7–12 | biscuit-icon, biscuit-icon-cli, biscuit-location, biscuit-location-cli, biscuit-speaks, biscuit-speaks-cli | 351 total | 18.4 | 6.9 |
| 13 | biscuit-terminal (image, serde, terminal-tests, browser-tests) | 70 | 20.0 | 1.6 |
| 14 | biscuit-terminal-cli (terminal-tests) | 121 | 21.5 | 1.5 |
| 15–17 | biscuit-tui, biscuit-tui-cli, biscuit-visualized (all) | 111 total | 23.1 | 1.6 |
| 18 | claudine (work-counters) | 246 | 25.8 | 2.7 |
| 19 | claudine-catalog-types | 6 | 27.1 | 1.4 |
| 20 | **claudine-cli** (daemon-tests, terminal-tests, test-fixtures) | **506** | 32.1 | **5.0** |
| 21 | claudine-contract | 29 | 35.2 | 3.0 |
| 22 | claudine-gen (terminal-tests) | 186 | 37.5 | 2.3 |
| 23–24 | content-policy (all), content-policy-cli | 57 total | 38.0 | 0.5 |
| 25 | **darkmatter** (terminal-tests, browser-tests, effects-instrumentation, work-counters) | 311 | 43.1 | **5.1** |
| 26–27 | darkmatter-cli, darkmatter-lsp | 474 total | 45.0 | 1.9 |
| 28–29 | eversolo-integration, **git-wt-cli** (terminal-tests) | 371 total | 48.7 | 3.7 |
| 30–32 | homelab, homelab-cli, homelab-server | 97 total | 51.3 | 2.6 |
| 33–34 | messenger (all), messenger-cli | 632 total | 57.3 | 5.9 |
| 35–45 | model-citizen … rendezvous-daemon (11 keys) | 840 total | 64.1 | 6.8 |
| 46 | research | 121 | 67.2 | 3.0 |
| 47–52 | schematic-define … simplified-schema (6 keys) | 495 total | 71.9 | 4.7 |
| 53–54 | sniff (remote), sniff-detection-cli (test-fixtures) | 280 total | 75.0 | 3.1 |
| 55–66 | sony-receiver-integration … zed-dmls-cli (12 keys) | 385 total | 81.5 | 6.5 |

Samples were 20 s apart, and several small keys finished between two samples.
The table therefore groups keys that share a sample, and a single-key Δ is
accurate to about ±0.5 GiB. The full 66-row table comes from
`spikes/a2/scripts/s4-table.py`.

### Where the 81.5 GiB is

| Part | GiB |
|---|---:|
| `target/x86_64-unknown-linux-gnu/debug/deps` | 66.8 |
| … of which `.rlib` files (4,225 of them) | 33.2 |
| … of which `.rmeta` files (4,222) | 10.0 |
| … of which test and bin executables (347) | 24.4 |
| `target/x86_64-unknown-linux-gnu/debug/examples` | 2.0 |
| `target/x86_64-unknown-linux-gnu/debug/build` | 1.1 |
| `target/schematic-fresh-output` (the guard) | 6.5 |
| `target/debug` (host build scripts and proc-macros) | 3.4 |
| `target/release` (the `ci-build` tool) | 0.7 |

The largest single files are 300–630 MiB debug test binaries of `claudine`,
`md`, `darkmatter`, and the `l1` integration-test binaries, plus a 480 MiB
`liblibduckdb_sys` rlib.

**Most of the rlib space is duplicates.** The 4,225 rlibs are only **982
distinct crates**: 3,243 are extra copies. Each key is a separate `cargo
nextest archive -p <package> <features>`, so Cargo resolves features
separately for each key, and a dependency built with a different feature set
is a new artifact. Examples:

| Crate | Copies | Total |
|---|---:|---:|
| `lopdf` | 59 | |
| `biscuit_file` | 47 | |
| `reqwest` | 40 | |
| `sniff` | 32 | 2.48 GiB |
| `tokio` | 29 | 0.37 GiB |
| `darkmatter` | 16 | 2.37 GiB |

### History of the hosted job

| Nightly | `build (ubuntu-latest)` | Produce step |
|---|---|---|
| Oct 26 | success | 2 h 07 min |
| Oct 27, 28, 31 | cancelled: 3 h job timeout | |
| Oct 29, 30, Nov 3, 5, 6 | `No space left on device`, written by the runner's own diagnostic log | Nov 5: 2 h 09 min; Nov 6: 2 h 15 min |
| Nov 4 | runner lost communication | |
| Nov 7 | `No space left on device` (same plan, same 66 keys) | 1 h 35 min (15:14–16:49 UTC) |

The logs of the failed jobs are gone (`BlobNotFound`). Successful
`ubuntu-latest` jobs in the same runs report `Image: ubuntu-26.04`: image
20261012.163.1 on Oct 26, 20261101.186.1 on Nov 6. The label is moving to
Ubuntu 26 between Oct 19 and Nov 19 (actions/runner-images#14748). That issue
gives no disk figures.

Sources for the hosted-runner disk:
[GitHub-hosted runners reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
[runner-images discussion #9329](https://github.com/actions/runner-images/discussions/9329),
[HastD/free-disk-space](https://github.com/HastD/free-disk-space),
[Carlos Becker, "Increasing GitHub Actions Disk Space"](https://carlosbecker.com/posts/github-actions-disk-space/).

## What this changes in the spec

- **"Spike needed: record disk use per build key on one real run"**: done
  locally, above. The largest consumers are not the archives (5.6 GiB). They
  are the shared `target/` of one serial owner: 33 GiB of rlibs, 77% of them by count
  per-key feature duplicates, 24 GiB of debug test executables, 10 GiB of
  rmeta, and 6.5 GiB from the fresh-output guard. Measured against the spec's
  three options:
  - **"Clear each key's output after archiving."** Deleting each key's test
    executables after it is archived would bound about 24 GiB. Shared rlibs
    would still accumulate.
  - **"Split the owner job."** Disk is roughly additive in the number of keys
    and their distinct feature sets. Each split owner needs the guard's 9 GiB
    only if it builds `schematic-gen`.
  - **"Reduce archive and debug-info size."** This saves little directly,
    because the archives are 5.6 GiB, but debug info is in all 24 GiB of
    executables.
- **The Outcome's "headroom measured in the job log"** cannot be reached by
  logging alone. When the disk fills, the runner process dies and the whole
  job log is lost. Headroom has to be checked during the run and acted on
  before the disk is full.
- The hosted runner's actual free space is the one number this spike could not
  get, and the choice between "reclaim space" and "split" depends on it.

### The instrumented CI run (needed; not triggered)

The local figures answer "how much per key". They cannot answer "against how
much free space", and the job's history contradicts the documented 14 GB. One
instrumented run would settle it. I have not triggered it.

**Recommended shape.** One temporary workflow, `.github/workflows/s4-disk-probe.yml`,
on a throwaway branch, triggered by `push` to that branch only. `ci.yml`'s
`push` filter is `[main]`, and `workflow_dispatch` would plan all four
environments, so neither is used.

1. Job `probe` on `runs-on: ubuntu-latest`, `timeout-minutes: 180`.
2. At the start, record the runner's disk and image:
   - `df -BM / /mnt /tmp "$RUNNER_TEMP" "$GITHUB_WORKSPACE"`
   - `lsblk`
   - `cat /etc/os-release`
   - `du -sh /usr/local/lib/android /usr/share/dotnet /opt/ghc
     /usr/local/.ghcup /usr/local/share/swift /opt/hostedtoolcache`
   - `echo "$ImageOS $ImageVersion"`
3. `actions/checkout@v5` at `ref: a816f14473c1a63737c79aeaf414222bb0a83706`.
4. Download `ci-resolved-plan` from run `42246114963` with
   `actions/download-artifact@v7` (`run-id`, `github-token`). It must not be
   re-planned.
5. Copy the `build:` job's steps unchanged: native prerequisites from that
   run's `ci-scope`, `rustup show`, `taiki-e/install-action@nextest`, the
   producer tool, and the fresh-output guard. Leave out `rust-cache`, since no
   cache exists, and leave out the publish step.
6. Before produce, start a background sampler that appends a line every 30 s
   to `$RUNNER_TEMP/disk.tsv` with:
   - `date +%s`;
   - `df -BM --output=avail /` and `/mnt`;
   - `du -s --apparent-size -BM target "$RUNNER_TEMP/build"`;
   - the archive count.

   Below 3 GiB free on `/`, the sampler kills `ci-build produce`, so the job
   ends normally instead of losing its log.
7. Run `bash scripts/ci/produce-owner.sh` with `PRODUCER=ubuntu-latest` and
   `continue-on-error: true`.
8. Under `if: always()`:
   - final `df` and `du -s --apparent-size -BM target/* target/*/*`;
   - upload `disk.tsv` and the out-dir manifests as artifact `s4-disk-probe`
     (archives excluded).

What it answers:

- the real free space on `/` and `/mnt` for today's image;
- whether the build hits the floor, and at which key;
- whether the local per-key curve matches the runner's.

Cost: one runner for up to 3 h, about the length of one nightly's Ubuntu
build.

## Surprises

- **The owner builds 66 keys, not 26.** The comment in `ci.yml` and the spec
  say "up to 26 keys"; the Nov 6 plan has 66 `ubuntu-latest` keys. Every one
  is consumed only by `wsl2-ubuntu` L1.
- **The job's log disappears when the disk fills**, so today the failure can
  only be diagnosed from annotations. This affects the Outcome's
  "measured in the job log".
- **The 3 h timeout is a second failure mode on the same job** (Oct 27, 28,
  31). Any fix that keeps one serial owner has to address time as well as
  disk.
- **Heavy rig builds can starve the rig.** 16 parallel GNU `ld` links of
  debug test binaries exhausted 16 GB with no swap and froze SSH for about 12
  minutes. `$BUILD_WIN`, on the same Proxmox host, still answered at 17:19 UTC.
  Worth adding to the `os` skill's `build-hosts.md`.
- **ZFS compression on `$BUILD_LINUX` hides disk use about 2.6×.** `df` on the
  rig understates what an ext4 runner would use. Use `du --apparent-size` when
  comparing.
