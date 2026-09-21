# WSL2

WSL2 (`wsl2-ubuntu`) is a distinct supported environment, not "Linux again".
The CI contract that makes it different is in `.github/workflows/_wsl-ci.yml`
and `fixes/2026-07-27-refactor/wsl-archive-requirements.md`; this file records
what that contract means for a test author and how to reproduce its failures.

## The archive-mode contract

CI builds `x86_64-unknown-linux-gnu` test binaries once on `ubuntu-latest` with
`cargo nextest archive` and only *runs* them inside the WSL2 guest. The guest
has no rustup, no cargo, and no toolchain. Consequences:

- Anything resolved at **compile time** to a builder path does not exist in
  the guest. `env!("CARGO_BIN_EXE_<name>")` names the builder's target
  directory; the test dies with `Os { code: 2, kind: NotFound }` before it can
  observe anything. Use `biscuit_test_harness::bin_exe!("<name>")`, which
  prefers nextest's run-time republication (`NEXTEST_BIN_EXE_<name>`, with
  hyphens rewritten) and falls back to the compile-time value.
- Anything that shells out to `cargo` or `rustc` at test time fails in the
  guest. That includes recipes: the canonical `just` test chain has a
  passthrough (`BISCUIT_NEXTEST_BIN`, `BISCUIT_JUNIT_TARGET_DIR`,
  `BISCUIT_JUNIT_WORKSPACE_ROOT`) that the workflow verifies is still wired.
  A suite that cannot avoid the shell-out — one that tests the repository's
  own tooling, say — declares `requires-toolchain = true` under
  `[package.metadata.ci.tests]`. The planner then renders that package's cell
  on any environment lacking the `cargo_toolchain` capability as a governed
  `ACCEPTED GAP` instead of scheduling a run that cannot pass. Do not reach
  for this to quiet an ordinary failure; it declares absent coverage, and the
  gap carries an owner and an expiry.
- The guest's `/bin/sh` is **dash**, not bash. A recipe or script using a bash
  builtin dies with `sh: 1: [[: not found` — which surfaces as the *recipe*
  failing, not the test, so the traceback names the wrong thing.
- The archive does include the package's non-test binaries and the linked
  paths nextest knows about, so a correctly resolved `bin_exe!` works.

The guest follows **Linux** code paths on a `windows-latest` host. Its build
phase is only the archive download and extraction, and its wall clock is
dominated by slow test execution, so it is never evidence for
native Windows behavior and is never compared with the `windows-latest` leg
as one environment ([ci-runners.md](ci-runners.md)).

Since `fixes/2026-09-12-single-os-compile`, `_wsl-ci.yml` owns **no producer
job**. It downloads the same `build-<package>-ubuntu-latest-<key>` artifact,
checksum, and realized digest native Linux consumes, and publishes its own
distinct `{package, wsl2-ubuntu, L1}` cell. Three things follow that a test
author hits:

- **Verification runs in the guest, not on its Windows host.** The predicates
  that matter are the guest's — architecture, ABI, libc, and the dynamic
  libraries the archived binaries resolve against. `ci-build`'s `host_runtime()`
  reads `cfg!`, so a host-side run would report `msvc` and prove nothing. The
  verifier travels inside the artifact; the guest could not build one.
- **The guest clones to its own `GUEST_ROOT`** (`/home/biscuit/checkout`), a
  path chosen to coincide with no producer's. It used to recreate the manifest's
  `producer_workspace` because ~160 targets read the compile-time
  `env!("CARGO_MANIFEST_DIR")` whatever `--workspace-remap` said; those are
  migrated to `biscuit_test_harness::manifest_dir!()`, and
  `tools/test-toolkit/tests/archive_path_guard.rs` fails the run if one returns.
  Do not reintroduce the derivation — it would hide the next baked path rather
  than surface it.
- **The guest cannot write `$GITHUB_OUTPUT`** (it is a Windows path). Anything
  the host needs from the guest crosses the 9p workspace as a file: the
  manifest is read on the host, and the guest leaves
  `wsl-timing/verify.seconds`, `wsl-timing/l1.seconds`, and a copy of the
  verdict for the status step to read. A guest that dies leaves none, and the
  stages report as absent rather than zero.

## Faithful reproduction on the WSL host

First identify the failing step and read the test summary. A red WSL job can
mean provisioning, test execution, or artifact publication failed. Passing
tests followed by an upload/finalization error are not evidence of a test
regression; do not rerun the suite just to diagnose that upload failure.

For an actual WSL-only test failure, archive-mode path and toolchain assumptions
are useful first checks. Reproduce it on the guest declared by `BUILD_WSL`
within the user's authorized test scope. An instruction not to rerun WSL
applies to cross-check commands and CI jobs triggered by pushes alike. If the
host is unset or the evidence is insufficient, explain the gap before launching
another run. When reproduction is authorized:

```bash
just cross-check <package> --os wsl <test-name-substring>
```

That recipe syncs the standing clone to your local tree, builds a nextest
archive, hides the builder's target directory, and runs the archive with the
workspace remapped, which is the same shape as the CI leg. Proven
non-vacuous on 2026-09-09: shipping the broken `CARGO_BIN_EXE` form of
`claudine-gen::steering_check` through it fails with CI's exact panic, and the
`bin_exe!` form passes 162/162.

Running the archive on the same machine that built it does **not** reproduce
the failure on its own, because the baked builder path still resolves; hiding
the target directory is the load-bearing step. The manual equivalent, for
when you need to vary the procedure:

```bash
ssh -o BatchMode=yes "$BUILD_WSL" 'bash -lc "
  cd ~/ci-verification/rusty-biscuit
  cargo nextest archive -p <pkg> --archive-file /tmp/a.tar.zst
  mv target target.hold
  rm -rf /tmp/x && mkdir -p /tmp/x
  cargo nextest run --archive-file /tmp/a.tar.zst \
    --workspace-remap ~/ci-verification/rusty-biscuit --extract-to /tmp/x \
    --no-fail-fast <test-name-substring>
  mv target.hold target
"'
```

- `--test <name>` is rejected together with `--archive-file`; filter by a
  name substring or a filterset.
- The `--extract-to` directory must already exist.
- Confirm the "before" failure matches CI's panic line, then apply the fix
  (`scp` the file into the clone), rebuild the archive, and rerun both the
  hidden-target archived run and a native `cargo nextest run -p <pkg>`.
- A cheaper approximation for toolchain-dependence bugs on any host: run the
  built test binary directly with `cargo` and `rustc` stripped from `PATH`.
  That is the guest's world.

## Reading the CI failure

The WSL job's log rarely shows the panic. Download the `junit-<pkg>-L1-
wsl2-ubuntu` artifact from the run (`gh run download <run-id> -n <artifact>`)
and read the `<failure>` element in `L1/<pkg>.xml`; it carries the panic
message and the source line.

The leg's label is `area-ci (<area>) / package-ci / wsl2 (<pkg>, L1,
wsl2-ubuntu, windows-latest) / test (wsl2-ubuntu)`: the cell rides on the
delegating `wsl2 (…)` segment because the guest job's own name is static.
Which step went red tells you what failed:

- **"Resolve this cell's execution contract"** — a `cell-contract-*` refusal on
  the Windows host. The row does not match the plan (unknown, duplicated,
  non-executing, or a dangling build record). No guest was provisioned; this is
  a planner or workflow defect, never a test failure.
- **"List this cell's expected tests"** — the listing runs each archived
  binary, so it fails for the same reasons the gate would (a missing native
  library, a sidecar, a baked builder path), minutes earlier.
- **"Certify this cell's completeness"** with the tests green — `completion.py`
  refused the cell, and stderr names each `completion-*` reason. The usual one
  is `completion-test-missing`: an expected test left no report. A skip
  approval in `ci-baseline.toml` cannot excuse that; only an observed
  `<skipped/>` can be approved. The same JUnit artifact also carries
  `expected-L1.json`, the listing the reports were compared with.

## Guest provisioning 403 (fixed, do not re-diagnose)

The guest once produced no test report at all, intermittently, with a `403`
while installing `just` from `just.systems/install.sh`. The script resolves
"latest" through `api.github.com` **anonymously** (60 requests per hour per
IP, shared across GitHub's runner fleet). Every other environment installs
`just` through a GitHub Action with the job token. A GitHub Action cannot run
inside the guest, so the installer is called with an explicit `--tag` and a
token instead. The nearby log text "the WSL2 guest failed to provision" is a
later diagnostic step, not the cause.

## Lost runner during provisioning (open; instrumented, not fixed)

The other way a guest leg dies with no report: GitHub's annotation reads "The
hosted runner lost communication with the server", the job is killed about
45 minutes after it started, and no log is retained. Every occurrence so far
was inside guest provisioning, before any repository code ran: three on
2026-08-27/28 and one on 2026-09-18 (`sniff-cli`, run 35308326156, while the
`sniff` and `biscuit-terminal` legs of the same run provisioned from the same
cached image within seconds of it). Over the twelve runs with WSL2 legs
between 2026-09-12 and 2026-09-18 that is 1 loss in 62 legs. Other projects
report the same shape on `windows-2025` in September 2026 —
[astral-sh/uv-dev#1804](https://github.com/astral-sh/uv-dev/issues/1804) and
[GemTalk/Jasper#580](https://github.com/GemTalk/Jasper/issues/580), the latter
measuring 17 of 3735 legs dying in `Vampire/setup-wsl` — and none has
established a cause; the step-level `timeout-minutes` does not fire, so it is
the runner agent that stops, not `wsl.exe`.

What this means when reading a red WSL leg:

- Treat it as a failure with an unknown cause, not as noise. Do not amend an
  acceptance criterion or rerun by hand to make it go away.
- `ci-infra-retry.yml` reruns the lost job once, automatically, when every
  failure in the run was a lost runner. A real failure anywhere else in the
  run vetoes that retry on purpose, so fix the real failure first; the retry
  then covers the loss on the next run.
- The provisioning phase is now recorded. `_wsl-ci.yml` asks the action only
  to register the distribution (`--no-launch`, no VM); "Boot the guest and
  verify it is WSL2" is the first VM start and "Install guest packages" the
  first apt run. The job's step list (`gh api .../actions/jobs/<id>`, which
  survives a lost runner) names the phase, which is the one measurement the
  loss leaves behind. When the next loss lands, record its step here.

## What the hosted guest is provisioned with, and why each entry is there

`_wsl-ci.yml`'s "Install guest packages" step is the whole apt set, and it is
deliberately thin — the guest RUNS prebuilt archived binaries and compiles
nothing, so it installs no rustup and no toolchain:

```
ca-certificates curl git xz-utils jq python3
```

Two of those are load-bearing in ways a reader would not guess:

- **`jq`** parses the scope-computed native-prerequisite list, in the step
  BEFORE any recipe that could bootstrap it. Without it the guest cannot
  install the libraries its archived binaries dynamically link.
- **`python3`** runs `scripts/ci/completion.py`, the producer-completeness
  validator, INSIDE the guest (added 2026-09-20 by
  `2026-09-19-direct-cell-execution`, ruling R2). It runs there rather than on
  the Windows host because the reports, the expected-test listing, and the plan
  are all on guest ext4, which the host cannot read. The validator is stdlib
  Python precisely because there is no compiler here to build an alternative
  with.

Both are proved reachable (`jq --version`, `python3 --version`) in the SAME
step that installs them. A package apt installed but cannot execute is a
provisioning failure; discovering it forty minutes later, after the suite ran,
reads as a validator defect instead.

The guest also receives one **dispatch row** — `{package, gate, environment,
runner}` — instead of a package plus environment lists, and resolves the rest
(test arguments, the slow-test contract, native prerequisites, the build
record it downloads) from the run's `ci-resolved-plan` through
`scripts/ci/cell_contract.py`. That resolution happens on the WINDOWS HOST,
before provisioning: a row the plan does not schedule must be refused before a
runner spends ten minutes building a guest for it.

`nextest list` runs before the gate, in the guest, as the unprivileged
`biscuit` user. Two reasons it is not root: the staging directory it creates
is the one the gate writes into, and root-owned would make the unprivileged
suite unable to stage anything. The two extractions this implies are
sequential, not concurrent, so peak VHDX growth is unchanged — which matters,
because peak is what exhausted the Windows host's disk in run 30605643702.

## Level 2 on WSL

There is no Level 2 terminal backend in the WSL2 CI environment (recorded as a
policy gap with owner and expiry in `.github/ci/environments.json`). This is
temporary. Do not write it into a spec as an authorized exclusion; record the
criterion as unmet and name provisioning as the required change. Locally,
the guest can run tmux-backed L2 suites when tmux is installed — and it does:
on 2026-09-10 the claudine partial-file family (tmux capture, the `expectrl`
PTY suite, and its L1 neighbors) passed 17/17 inside the guest in archive
mode with `--features terminal-tests`. The recipe is
`just cross-check claudine-cli --os wsl --features terminal-tests <filters>`;
add `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` when running by hand so a skip
cannot print as a pass (a WezTerm test skips in the guest in ~0.03 s and
nextest still says PASS).

## The guest's `~/.config` is a network share

`~/.config` in the guest is a CIFS mount of `//192.168.100.97/config`, a
share on the Synology NAS — the same share the Windows side's `wezterm.lua`
reaches for. When the NAS is down (`ping -c1 -W2 192.168.100.97` from the
guest), two things fail before any test runs:

- **Every `git` command** dies with `fatal: unable to access
  '/home/ken/.config/git/config': Host is down`, so `cross-check --os wsl`
  fails at its first `git fetch`. `GIT_CONFIG_GLOBAL=/dev/null` bypasses it;
  fetch/reset/clean/checkout/apply need no identity.
- **`cargo nextest archive` (any cargo build)** dies with `failed to determine
  package fingerprint for build script for playa` → `Could not read
  repository exclude` → `Host is down`. Cargo lists package files through
  gitoxide, which honors the global excludes at `$XDG_CONFIG_HOME/git/ignore`
  — on the dead mount — and `GIT_CONFIG_GLOBAL` does not reach it. Point
  `XDG_CONFIG_HOME` at an empty local directory for the build.

Nothing the tests read lives on that share, so both bypasses are safe for a
run, and since 2026-09-11 `scripts/cross-check.sh` sets both in its Unix
preamble — the recipe survives the NAS being down. A login shell in the
guest is still slow while the share is down (profile tooling stats paths
under `~/.config`), which shows up as a 20-second-plus first tmux capture;
that is the host, not the test.

`cross-check` also forwards `BISCUIT_TEST_REQUIRED_BACKENDS` from your shell
to every remote run, so `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just cross-check
claudine-cli --os wsl --features terminal-tests level2_` cannot pass by
skipping.
